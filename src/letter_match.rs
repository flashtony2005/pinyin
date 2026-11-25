//! 单字母跟读判定：录音的 24 维增强特征 vs 目标字母的多个真人模板。
//!
//! 不是 47 选 1 的闭集识别，而是「读得像不像这一个音」的验证。
//! 三道门（2026-11-25 用独立样本实测重定，见 tools/diag_*.py）：
//!   1. 信号门：VAD 有声段 + 过零率 + **能量包络起伏**排哼声/纯音/宽带噪声；
//!   2. 绝对门：目标距离本身要够小，挡住「无关音碰巧离目标最近」；
//!   3. 幅度门：VAD 的能量下限（在 voice_span 内）。
//!
//! ⚠️ 曾用「竞争门」(d_target <= d_rival * 0.85) 想挡 b 读成 w，实测证明
//! 它无效且有害：读对 ratio 中位数 1.23、读错 1.27，分布几乎完全重叠；
//! 0.85 把 26/28 个正确发音全拒，就是用户说的「过不了」。已移除。
//!
//! ⚠️ 纯 MFCC 距离本身也分不开「读错字母」：读对 d=1.89、读错 d=1.92、
//! 乱哼 d=1.51（比读对还近）。所以挡乱哼靠能量包络起伏 CV（哼声 ≤0.01、
//! 语音 ≥0.297，差 30 倍），不是靠 MFCC 距离。音素级严格区分（b vs w）
//! 需另上 GOP，见文档。
//!
//! 流水线：VAD → 包络 CV 门 → enhanced(24维) → 多模板最小距离 → 绝对阈值。
//! 模板由 tools/build_letter_templates.py 生成，参数与 dtw.rs 严格镜像。

use crate::data::letter_templates::{
    LETTERS, LETTER_F0, LETTER_TEMPLATES, MFCC_STD,
};
use crate::dtw::{self, MfccConfig};

/// 能量 VAD：裁掉录制前后的静音，只留有声段。返回切片区间。
fn voice_span(pcm: &[f32], sample_rate: u32) -> (usize, usize) {
    let win = (sample_rate as usize * 20 / 1000).max(1);
    if pcm.len() < win * 3 {
        return (0, pcm.len());
    }
    let n = pcm.len() / win;
    let mut energies = Vec::with_capacity(n);
    for i in 0..n {
        let seg = &pcm[i * win..(i + 1) * win];
        let sum: f32 = seg.iter().map(|&x| x * x).sum();
        energies.push((sum / seg.len() as f32).sqrt());
    }
    let peak = energies.iter().copied().fold(0.0f32, f32::max);
    const ABS_FLOOR: f32 = 0.012;
    if peak < ABS_FLOOR {
        return (0, 0);
    }
    let thresh = (peak * 0.18).max(ABS_FLOOR * 0.5);
    let mut first = None;
    let mut last = None;
    for (i, &e) in energies.iter().enumerate() {
        if e >= thresh {
            if first.is_none() {
                first = Some(i);
            }
            last = Some(i);
        }
    }
    match (first, last) {
        (Some(a), Some(b)) => {
            let start = a.saturating_sub(1) * win;
            let end = ((b + 2).min(n)) * win;
            (start, end)
        }
        _ => (0, 0),
    }
}

/// 能量包络起伏系数（CV = 标准差 / 均值）。
///
/// 这是挡「随便哼一个音」最稳的判据：纯音/哼声能量恒定，CV ≤ 0.010；
/// 真实语音有音节起伏，独立样本实测最小 0.297、中位 0.419——差 30 倍，
/// 且对哼声频率不敏感（谱平坦度在 330Hz 会与语音重叠，故以本判据为主）。
/// 用 20ms 窗口、10ms 帧移统计有声段内的能量波动。
fn envelope_cv(pcm: &[f32]) -> f32 {
    const WIN: usize = 320; // 20ms @16k
    const HOP: usize = 160; // 10ms
    if pcm.len() < WIN * 2 {
        return 0.0;
    }
    let mut e = Vec::new();
    let mut start = 0;
    while start + WIN <= pcm.len() {
        let sum: f32 = pcm[start..start + WIN].iter().map(|&x| x * x).sum();
        e.push((sum / WIN as f32).sqrt());
        start += HOP;
    }
    if e.len() < 2 {
        return 0.0;
    }
    let mean = e.iter().sum::<f32>() / e.len() as f32;
    if mean <= 1e-9 {
        return 0.0;
    }
    let var = e.iter().map(|&x| (x - mean) * (x - mean)).sum::<f32>() / e.len() as f32;
    var.sqrt() / mean
}

/// 过零率：元音 <0.15，宽带噪声 ~0.5。挡住"环境噪声被误判成发音"。
fn zero_crossing_rate(pcm: &[f32]) -> f32 {
    if pcm.len() < 2 {
        return 0.0;
    }
    let mut zc = 0usize;
    for w in pcm.windows(2) {
        if (w[0] < 0.0) != (w[1] < 0.0) {
            zc += 1;
        }
    }
    zc as f32 / (pcm.len() - 1) as f32
}

/// 判定结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verdict {
    /// 太安静 / 纯噪声 / 乱哼，没听到有效发音。
    TooQuiet,
    /// 读对了：信号门 + 绝对距离都达标。
    Pass { score: f32 },
    /// 读得不像，再试一次。
    Retry { score: f32 },
}

impl Verdict {
    /// UI 展示用的 0..1 分数。
    pub fn score(&self) -> f32 {
        match self {
            Verdict::TooQuiet => 0.0,
            Verdict::Pass { score } | Verdict::Retry { score } => *score,
        }
    }
}

/// DTW 距离 → 0..1 展示分。scale 按实测标定：独立样本读对 d 中位数 1.89，
/// scale=10 让读对显示 ~0.84，避免"明明读对了却只有 70 分"的挫败感。
pub const SCORE_SCALE: f32 = 10.0;

/// 绝对距离门：目标距离超过它就一律 Retry。
/// 独立样本读对 d 的 p90=2.37，取 4.0 留童声音色差异余量。
pub const ABS_MAX_DISTANCE: f32 = 4.0;

/// 能量包络 CV 下限：纯音/哼声 ≤0.010，真实语音 ≥0.297（独立样本实测）。
/// 取 0.08 留 3.7 倍余量。这是挡"随便发个音"的主门——MFCC 距离做不到
/// （乱哼 d 比读对还小），谱平坦度对 330Hz 哼声也会误判，唯有包络起伏稳定。
pub const MIN_ENVELOPE_CV: f32 = 0.08;

/// 单个字母的多个模板里取最小距离（最宽容的那一版真人样本）。
fn letter_min_distance(frames: &[Vec<f32>], templates: &[&[&[f32]]]) -> f32 {
    let mut best = f32::INFINITY;
    for tpl in templates {
        let d = dtw::dtw_distance(frames, *tpl);
        if d < best {
            best = d;
        }
    }
    best
}

/// 和某个字母的标准音比对。
///
/// `index` 是字母索引（对应 `LETTER_TEMPLATES` 顺序）。
pub fn match_letter(pcm: &[f32], sample_rate: u32, index: usize) -> Verdict {
    if index >= LETTERS.len() {
        return Verdict::TooQuiet;
    }
    let (a, b) = voice_span(pcm, sample_rate);
    if b <= a {
        return Verdict::TooQuiet;
    }
    let voiced = &pcm[a..b];
    let zcr = zero_crossing_rate(voiced);
    // 包络起伏：挡"乱哼一个音"——哼声能量恒定，语音有音节起伏。
    let env_cv = envelope_cv(voiced);

    let cfg = MfccConfig::default();
    let frames = dtw::enhanced(voiced, &cfg, &MFCC_STD);
    if frames.is_empty() {
        return Verdict::TooQuiet;
    }
    // 纯噪声判据：过零率过高 → 不是语音。
    if zcr > 0.35 {
        return Verdict::TooQuiet;
    }

    let d_target = letter_min_distance(&frames, LETTER_TEMPLATES[index]);
    let score = (1.0 / (1.0 + d_target / SCORE_SCALE)).clamp(0.0, 1.0);

    #[cfg(target_os = "android")]
    println!(
        "LMATCH idx={} len={} span=({},{}) zcr={:.3} envcv={:.4} frames={} d_target={:.3} score={:.3}",
        index, pcm.len(), a, b, zcr, env_cv, frames.len(), d_target, score
    );

    // 包络门优先：纯音/哼声/哨声能量恒定，一律 Retry，哪怕 MFCC 距离很近。
    if env_cv < MIN_ENVELOPE_CV {
        return Verdict::Retry { score };
    }
    // 绝对门：离目标太远，不算读对。
    if !d_target.is_finite() || d_target > ABS_MAX_DISTANCE {
        return Verdict::Retry { score };
    }
    Verdict::Pass { score }
}

/// 综合评测：音色判定 + 声调轮廓分，给 UI 一次性取走。
///
/// 过关只看音色（包络门 + 绝对门）——童声 F0 容易判偏，不宜做硬门槛。
/// ⚠️ 展示分不再做「音色×0.7 + 声调×0.3」：童声声调常判 0，会把读对的
/// 100 分压成 70 分（用户反馈的「只有 70%」）。综合分 = 纯音色分；
/// 声调分只进 tone 字段，供 UI 出鼓励文案。
pub struct Assessment {
    pub verdict: Verdict,
    /// 音色分 0..1（与 verdict.score 一致，也是展示分）。
    pub timbre: f32,
    /// 声调轮廓相似度 0..1（仅文案用，不进分数）。
    pub tone: f32,
    /// 展示用综合分 = 音色分。
    pub overall: f32,
}

/// 对一段录音做完整评测。`pcm` 为 16 kHz f32。
pub fn assess(pcm: &[f32], sample_rate: u32, index: usize) -> Assessment {
    let verdict = match_letter(pcm, sample_rate, index);
    let timbre = verdict.score();
    let tone = if timbre > 0.0 {
        let contour = crate::audio_tone::contour_20(pcm, sample_rate);
        tone_similarity(&contour, index)
    } else {
        0.0
    };
    Assessment { verdict, timbre, tone, overall: timbre }
}

/// 声调轮廓对比：录音 F0 轮廓 vs 目标字母标准音模板。
/// 返回 0..1 相似度；轮廓用相对音分，天然抵消童声比成人高八度。
pub fn tone_similarity(f0_cents: &[f32; 20], index: usize) -> f32 {
    if index >= LETTER_F0.len() {
        return 0.0;
    }
    let mut best = 0.0f32;
    for tpl in LETTER_F0[index] {
        let mut sum = 0.0f32;
        let mut n = 0usize;
        for (&a, &b) in f0_cents.iter().zip(tpl.iter()) {
            if a.is_finite() && b.is_finite() {
                // 音分差 → 相似度：差 200 cents（全音）记 0.5
                sum += 1.0 / (1.0 + (a - b).abs() / 200.0);
                n += 1;
            }
        }
        if n > 0 {
            best = best.max(sum / n as f32);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(sr: u32, ms: u32, freq: f32) -> Vec<f32> {
        let n = (sr as f64 * ms as f64 / 1000.0) as usize;
        (0..n)
            .map(|i| {
                let t = i as f32 / sr as f32;
                (2.0 * std::f32::consts::PI * freq * t).sin() * 0.5
            })
            .collect()
    }

    /// 加载真实录音 f32（assets 里的 mp3 已由 Python 解码为 f32 夹具）。
    fn pcm_fixture(name: &str) -> Vec<f32> {
        let path = format!("{}/tools/{}", env!("CARGO_MANIFEST_DIR"), name);
        let bytes = std::fs::read(path).expect("fixture exists");
        bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }

    #[test]
    fn enhanced_matches_python_reference() {
        let pcm = pcm_fixture("test_enhanced.f32");
        let cfg = MfccConfig::default();
        let f = dtw::enhanced(&pcm, &cfg, &MFCC_STD);
        assert_eq!(f.len(), 23, "frame count mismatch");
        // Python build_letter_templates.enhance() on the same PCM+STD,
        // first 3 frames. 容差 0.35（FFT 实现差异），但远小于维度跨度。
        let expected: [[f32; 6]; 3] = [
            [-0.3567, -0.1343, 0.6012, 1.1427, 0.8946, 1.3168],
            [0.1851, 0.9297, 0.2426, -0.1194, 0.5281, 0.8777],
            [0.1966, 0.9602, 0.1674, -0.5815, 0.1648, -0.0881],
        ];
        for (i, row) in expected.iter().enumerate() {
            for (j, &e) in row.iter().enumerate() {
                assert!((f[i][j] - e).abs() < 0.35,
                    "enhanced frame {i} cep {j}: rust={} python={e}", f[i][j]);
            }
        }
    }

    #[test]
    fn enhanced_features_have_expected_dim() {
        let pcm = tone(16_000, 500, 220.0);
        let f = dtw::enhanced(&pcm, &MfccConfig::default(), &MFCC_STD);
        assert!(!f.is_empty());
        assert!(f.iter().all(|r| r.len() == dtw::ENHANCED_DIM));
    }

    #[test]
    fn template_pcm_scores_pass() {
        // 同一段 PCM（bo1，字母 b=索引0）必须判 Pass（自己评自己）。
        let pcm = pcm_fixture("test_enhanced.f32");
        assert!(matches!(
            match_letter(&pcm, 16_000, 0),
            Verdict::Pass { .. }
        ));
    }

    #[test]
    fn broadband_noise_is_rejected() {
        let mut seed = 12345u32;
        let pcm: Vec<f32> = (0..16_000)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((seed >> 8) as f32 / 16_777_216.0 - 0.5) * 0.2
            })
            .collect();
        assert_eq!(match_letter(&pcm, 16_000, 0), Verdict::TooQuiet);
    }

    #[test]
    fn all_silence_is_too_quiet() {
        let v = vec![0.0f32; 16_000];
        assert_eq!(match_letter(&v, 16_000, 0), Verdict::TooQuiet);
    }

    #[test]
    fn bad_index_is_safe() {
        let v = tone(16_000, 500, 220.0);
        assert_eq!(match_letter(&v, 16_000, 999), Verdict::TooQuiet);
    }

    #[test]
    fn voice_span_trims_leading_silence() {
        let mut v = vec![0.0f32; 16_000];
        v.extend(tone(16_000, 400, 220.0));
        let (a, b) = voice_span(&v, 16_000);
        assert!(a >= 15_000, "leading silence should be trimmed, got a={a}");
        assert!(b > a);
    }

    #[test]
    fn tone_similarity_self_is_high() {
        let tpl = &LETTER_F0[0][0];
        let s = tone_similarity(tpl, 0);
        assert!(s > 0.95, "self F0 similarity should be ~1, got {s}");
    }

    /// 能量包络起伏：乱哼必须低（<0.08），真实语音必须高。
    /// 这是「随便发个音都过」的根治判据，且对哼声频率不敏感。
    #[test]
    fn envelope_cv_separates_hum_from_speech() {
        for freq in [180.0, 220.0, 330.0, 440.0] {
            let hum = tone(16_000, 1200, freq);
            let h = envelope_cv(&hum);
            assert!(h < MIN_ENVELOPE_CV, "hum {freq}Hz env_cv should be low, got {h}");
        }
        let speech = pcm_fixture("test_enhanced.f32");
        let s = envelope_cv(&speech);
        assert!(s > MIN_ENVELOPE_CV,
            "real speech env_cv should be high, got {s}");
        assert!(s > 0.20, "speech env_cv ({s}) should be >> hum");
    }

    /// 关键回归：「随便发个音」不能过关。多个频率的哼声评所有字母必须全 Retry。
    #[test]
    fn arbitrary_hum_rejected_for_every_letter() {
        for freq in (180..=420).step_by(40) {
            let hum = tone(16_000, 1200, freq as f32);
            for (i, letter) in LETTERS.iter().enumerate() {
                assert_ne!(
                    match_letter(&hum, 16_000, i),
                    Verdict::TooQuiet,
                    "hum reaches decision gates for {letter}"
                );
                assert!(
                    matches!(match_letter(&hum, 16_000, i), Verdict::Retry { .. }),
                    "arbitrary hum ({freq}Hz) must NOT pass as {letter}"
                );
            }
        }
    }

    /// 独立样本（没进模板池）的正确发音应判 Pass——「过不了」的回归测试。
    #[test]
    fn independent_samples_pass() {
        let cases = [
            ("probe_b.f32", 0usize),   // b
            ("probe_w.f32", 22usize),  // w
            ("probe_z.f32", 18usize),  // z
            ("probe_zh.f32", 14usize), // zh
        ];
        for (name, idx) in cases {
            let pcm = pcm_fixture(name);
            assert!(
                matches!(match_letter(&pcm, 16_000, idx), Verdict::Pass { .. }),
                "independent sample {name} should pass as {}", LETTERS[idx]
            );
        }
    }
}
