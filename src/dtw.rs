//! 轻量声学相似度：MFCC + DTW，纯 Rust，零外部依赖。
//!
//! 用途：给孩子读的音节和参考模板打分（单音节"读得像不像"）。这条链路
//! 不认字、不猜词，只比"声音形状"，所以短音节也能用，且给出连续分数，
//! 能看到"这次比上次准"的进步曲线。
//!
//! 局限（写在这里，免得误用）：对说话人音色敏感。模板若是成人录音，
//! 儿童音高与共振峰差异会让距离整体偏大；正确做法是用同龄儿童录音
//! 重建模板（见 `tools/` 的采集脚本），再把阈值按批次标定。
//!
//! 两条特征链路：
//! - `mfcc` —— 13 维含 c0、仅 CMN，供历史单测与 FFT 验证。
//! - `enhanced` —— 24 维（c1..c12 + 一阶差分）、逐句 CMN + 全局方差白化，
//!   供跟读判定。参数与 `tools/build_letter_templates.py` 严格镜像。
//!
//! 流水线：PCM → 预加重 → 分帧 + 汉明窗 → 512 点 FFT → 梅尔滤波器组
//!        → 取对数 → DCT-II → 13 维 MFCC → 去 c0 + delta + CMN + 白化。

/// MFCC 参数。默认值对应 16 kHz、25 ms 帧长、10 ms 帧移。
#[derive(Debug, Clone, Copy)]
pub struct MfccConfig {
    pub sample_rate: u32,
    /// 梅尔滤波器个数。
    pub n_mels: usize,
    /// 保留的倒谱系数个数（含 c0）。
    pub n_ceps: usize,
    /// 每帧采样点数（会补零到 2 的幂做 FFT）。
    pub frame: usize,
    /// 帧移采样点数。
    pub hop: usize,
}

impl Default for MfccConfig {
    fn default() -> Self {
        Self { sample_rate: 16_000, n_mels: 26, n_ceps: 13, frame: 400, hop: 160 }
    }
}

const PRE_EMPHASIS: f32 = 0.97;
const FFT_SIZE: usize = 512;
/// 增强特征维度：MFCC c1..c12（12）+ 其一阶差分（12）= 24。
pub const ENHANCED_DIM: usize = 24;

/// 13 维 MFCC（含 c0）+ 逐句 CMN。保留给历史单测与 FFT 验证。
pub fn mfcc(pcm: &[f32], cfg: &MfccConfig) -> Vec<Vec<f32>> {
    let mut out = raw_ceps(pcm, cfg);
    cepstral_mean_normalize(&mut out);
    out
}

/// 不加 CMN 的 13 维 MFCC（含 c0），增强特征的中间产物。
fn raw_ceps(pcm: &[f32], cfg: &MfccConfig) -> Vec<Vec<f32>> {
    if pcm.len() < cfg.frame || cfg.frame > FFT_SIZE {
        return Vec::new();
    }
    let window = hamming(cfg.frame);
    let filters = mel_filterbank(cfg.sample_rate, FFT_SIZE, cfg.n_mels);
    let dct = dct_matrix(cfg.n_mels, cfg.n_ceps);

    // 预加重
    let mut pre = Vec::with_capacity(pcm.len());
    pre.push(pcm[0]);
    for i in 1..pcm.len() {
        pre.push(pcm[i] - PRE_EMPHASIS * pcm[i - 1]);
    }

    let mut out = Vec::new();
    let mut start = 0;
    while start + cfg.frame <= pre.len() {
        let mut re = vec![0f32; FFT_SIZE];
        let mut im = vec![0f32; FFT_SIZE];
        for i in 0..cfg.frame {
            re[i] = pre[start + i] * window[i];
        }
        fft(&mut re, &mut im);

        // 功率谱（只取正频率一半）
        let half = FFT_SIZE / 2 + 1;
        let mut power = vec![0f32; half];
        for k in 0..half {
            power[k] = re[k] * re[k] + im[k] * im[k];
        }

        // 梅尔滤波 → 取对数
        let mut log_mel = vec![0f32; cfg.n_mels];
        for (m, band) in filters.iter().enumerate() {
            let mut sum = 0f32;
            for &(k, w) in band {
                sum += power[k] * w;
            }
            log_mel[m] = (sum.max(1e-10)).ln();
        }

        // DCT-II → 倒谱
        let mut ceps = vec![0f32; cfg.n_ceps];
        for (c, row) in dct.iter().enumerate() {
            let mut sum = 0f32;
            for (m, &coef) in row.iter().enumerate() {
                sum += log_mel[m] * coef;
            }
            ceps[c] = sum;
        }
        out.push(ceps);
        start += cfg.hop;
    }
    out
}

/// 一阶差分：中心差分，端点用单侧差分。与 Python `delta()` 镜像。
fn delta(frames: &[Vec<f32>]) -> Vec<Vec<f32>> {
    if frames.is_empty() {
        return Vec::new();
    }
    let dim = frames[0].len();
    let mut out = Vec::with_capacity(frames.len());
    for i in 0..frames.len() {
        let mut d = vec![0f32; dim];
        if frames.len() >= 2 {
            let (a, b) = match i {
                0 => (&frames[0], &frames[1]),
                n if n + 1 == frames.len() => (&frames[n - 1], &frames[n]),
                n => (&frames[n - 1], &frames[n + 1]),
            };
            let scale = if i == 0 || i + 1 == frames.len() { 1.0 } else { 0.5 };
            for j in 0..dim {
                d[j] = (b[j] - a[j]) * scale;
            }
        }
        out.push(d);
    }
    out
}

/// 增强特征：c1..c12 + delta(c1..c12)，逐句 CMN，再按 `std` 全局白化。
/// `std` 长度必须是 24；与生成模板的 `MFCC_STD` 一致。
pub fn enhanced(pcm: &[f32], cfg: &MfccConfig, std: &[f32; ENHANCED_DIM]) -> Vec<Vec<f32>> {
    let ceps = raw_ceps(pcm, cfg);
    if ceps.is_empty() {
        return Vec::new();
    }
    // 去 c0：保留 1..n_ceps（共 12 维）
    let base: Vec<Vec<f32>> = ceps.iter().map(|f| f[1..].to_vec()).collect();
    let d = delta(&base);
    let mut out: Vec<Vec<f32>> = base
        .iter()
        .zip(d.iter())
        .map(|(s, dd)| {
            let mut row = Vec::with_capacity(ENHANCED_DIM);
            row.extend_from_slice(s);
            row.extend_from_slice(dd);
            row
        })
        .collect();
    // 逐句 CMN（减每维均值）
    let mut mean = [0f32; ENHANCED_DIM];
    for row in &out {
        for (i, &v) in row.iter().enumerate() {
            mean[i] += v;
        }
    }
    let inv = 1.0 / out.len() as f32;
    for m in mean.iter_mut() {
        *m *= inv;
    }
    // 白化（分母来自 Python 全局 std，非逐句）
    for row in &mut out {
        for (i, v) in row.iter_mut().enumerate() {
            *v = (*v - mean[i]) / std[i].max(1e-3);
        }
    }
    out
}

/// 谱平坦度（spectral flatness）：幅度谱的几何均值 / 算术均值。
///
/// 纯音（哼声、哨声、正弦）频谱只有一根尖峰 → 接近 0；真实语音有丰富谐波
/// → 0.15~0.5。这是挡"随便发个音"的关键：MFCC 距离分不开哼声和语音
/// （实测乱哼 d=1.5 比读对 d=1.9 还近），但谱平坦度差 4 个数量级。
///
/// 分帧后取各帧平坦度的中位数，避免单帧静音/爆破瞬时拉偏。
pub fn spectral_flatness(pcm: &[f32]) -> f32 {
    const N: usize = 512;
    const HOP: usize = 160;
    if pcm.len() < N {
        return 0.0;
    }
    let win = hamming(N);
    let mut vals = Vec::new();
    let mut start = 0;
    while start + N <= pcm.len() {
        let mut re = vec![0f32; N];
        let mut im = vec![0f32; N];
        for i in 0..N {
            re[i] = pcm[start + i] * win[i];
        }
        fft(&mut re, &mut im);
        // |X[k]|（幅度谱），跳过 DC。与 Python 诊断脚本口径一致。
        let mut sum_log = 0f32;
        let mut sum = 0f32;
        let mut count = 0usize;
        for k in 1..N / 2 {
            let mag = (re[k] * re[k] + im[k] * im[k]).sqrt() + 1e-9;
            sum_log += mag.ln();
            sum += mag;
            count += 1;
        }
        if count > 0 && sum > 1e-9 {
            vals.push((sum_log / count as f32).exp() / (sum / count as f32));
        }
        start += HOP;
    }
    if vals.is_empty() {
        return 0.0;
    }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    vals[vals.len() / 2]
}

/// 倒谱均值归一化（CMN）：减掉每条轨道的均值，抵消信道/麦克风差异。
fn cepstral_mean_normalize(frames: &mut [Vec<f32>]) {
    if frames.is_empty() {
        return;
    }
    let n = frames[0].len();
    let mut mean = vec![0f32; n];
    for f in frames.iter() {
        for (i, &v) in f.iter().enumerate() {
            mean[i] += v;
        }
    }
    for m in mean.iter_mut() {
        *m /= frames.len() as f32;
    }
    for f in frames.iter_mut() {
        for (i, v) in f.iter_mut().enumerate() {
            *v -= mean[i];
        }
    }
}

fn hamming(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| 0.54 - 0.46 * (2.0 * std::f32::consts::PI * i as f32 / (n - 1) as f32).cos())
        .collect()
}

fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10f32.powf(mel / 2595.0) - 1.0)
}

/// 三角梅尔滤波器组，返回每个滤波器的 (频率 bin, 权重) 稀疏表示。
fn mel_filterbank(sample_rate: u32, fft_size: usize, n_mels: usize) -> Vec<Vec<(usize, f32)>> {
    let low = hz_to_mel(60.0);
    let high = hz_to_mel(sample_rate as f32 / 2.0);
    let mut points = Vec::with_capacity(n_mels + 2);
    for i in 0..(n_mels + 2) {
        let mel = low + (high - low) * i as f32 / (n_mels + 1) as f32;
        let hz = mel_to_hz(mel);
        let bin = ((fft_size + 1) as f32 * hz / sample_rate as f32).floor() as usize;
        points.push(bin.min(fft_size / 2));
    }

    let mut filters = Vec::with_capacity(n_mels);
    for m in 1..=n_mels {
        let (left, center, right) = (points[m - 1], points[m], points[m + 1]);
        let mut band = Vec::new();
        for k in left..center {
            if center > left {
                band.push((k, (k - left) as f32 / (center - left) as f32));
            }
        }
        for k in center..right {
            if right > center {
                band.push((k, (right - k) as f32 / (right - center) as f32));
            }
        }
        filters.push(band);
    }
    filters
}

/// DCT-II 矩阵：`n_ceps × n_mels`，正交归一化。
fn dct_matrix(n_mels: usize, n_ceps: usize) -> Vec<Vec<f32>> {
    let mut mat = vec![vec![0f32; n_mels]; n_ceps];
    let scale0 = (1.0 / n_mels as f32).sqrt();
    for c in 0..n_ceps {
        let scale = if c == 0 { scale0 } else { (2.0 / n_mels as f32).sqrt() };
        for m in 0..n_mels {
            let angle = std::f32::consts::PI * c as f32 * (m as f32 + 0.5) / n_mels as f32;
            mat[c][m] = scale * angle.cos();
        }
    }
    mat
}

/// 原地 radix-2 Cooley-Tukey FFT（长度须为 2 的幂）。
fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    debug_assert!(n.is_power_of_two());
    // 位反转置换
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * std::f32::consts::PI / len as f32;
        let (wr, wi) = (ang.cos(), ang.sin());
        let mut i = 0;
        while i < n {
            let (mut cr, mut ci) = (1.0f32, 0.0f32);
            for k in 0..len / 2 {
                let (ur, ui) = (re[i + k], im[i + k]);
                let (vr, vi) = (
                    re[i + k + len / 2] * cr - im[i + k + len / 2] * ci,
                    re[i + k + len / 2] * ci + im[i + k + len / 2] * cr,
                );
                re[i + k] = ur + vr;
                im[i + k] = ui + vi;
                re[i + k + len / 2] = ur - vr;
                im[i + k + len / 2] = ui - vi;
                let ncr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = ncr;
            }
            i += len;
        }
        len <<= 1;
    }
}

/// 两段 MFCC 的 DTW 距离（欧氏局部代价，返回按路径长度归一化的平均代价）。
///
/// 泛型到 `AsRef<[f32]>`：既接受运行时的 `&[Vec<f32>]`，也接受编译期静态
/// 模板 `&[&[f32]]`（见 letter_templates），避免每次识别都拷贝模板。
pub fn dtw_distance<A: AsRef<[f32]>, B: AsRef<[f32]>>(a: &[A], b: &[B]) -> f32 {
    if a.is_empty() || b.is_empty() {
        return f32::INFINITY;
    }
    let (n, m) = (a.len(), b.len());
    let mut prev = vec![f32::INFINITY; m + 1];
    let mut cur = vec![f32::INFINITY; m + 1];
    prev[0] = 0.0;
    for i in 1..=n {
        cur[0] = f32::INFINITY;
        for j in 1..=m {
            let cost = euclidean(a[i - 1].as_ref(), b[j - 1].as_ref());
            let best = prev[j].min(cur[j - 1]).min(prev[j - 1]);
            cur[j] = cost + if best.is_finite() { best } else { 0.0 };
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[m] / (n + m) as f32
}

fn euclidean(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len());
    let mut sum = 0f32;
    for i in 0..n {
        let d = a[i] - b[i];
        sum += d * d;
    }
    sum.sqrt()
}

/// 把 DTW 距离折成 0..1 的相似度。`scale` 是"多远算完全不同"，按模板
/// 批次标定（先用 40，实测再调）。
pub fn similarity<A: AsRef<[f32]>, B: AsRef<[f32]>>(a: &[A], b: &[B], scale: f32) -> f32 {
    let d = dtw_distance(a, b);
    if !d.is_finite() {
        return 0.0;
    }
    (1.0 / (1.0 + d / scale.max(1e-3))).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(sr: u32, ms: u32, freq: f32, harmonic: f32) -> Vec<f32> {
        let n = (sr as f64 * ms as f64 / 1000.0) as usize;
        (0..n)
            .map(|i| {
                let t = i as f32 / sr as f32;
                let s = (2.0 * std::f32::consts::PI * freq * t).sin()
                    + harmonic * (4.0 * std::f32::consts::PI * freq * t).sin();
                s * 0.4
            })
            .collect()
    }

    #[test]
    fn identical_signals_are_very_similar() {
        let cfg = MfccConfig::default();
        let a = tone(16_000, 600, 200.0, 0.5);
        let m = mfcc(&a, &cfg);
        assert!(!m.is_empty(), "应提取出 MFCC 帧");
        let sim = similarity(&m, &m, 40.0);
        assert!(sim > 0.95, "自相似度应接近 1，实际 {sim}");
    }

    #[test]
    fn different_pitch_is_less_similar() {
        let cfg = MfccConfig::default();
        let a = mfcc(&tone(16_000, 600, 200.0, 0.5), &cfg);
        let b = mfcc(&tone(16_000, 600, 320.0, 0.1), &cfg);
        let same = similarity(&a, &a, 40.0);
        let diff = similarity(&a, &b, 40.0);
        assert!(diff < same, "不同音应更不相似：{diff} vs {same}");
    }

    #[test]
    fn fft_matches_naive_dft() {
        let n = 16;
        let sig: Vec<f32> = (0..n).map(|i| (i as f32 * 0.7).sin()).collect();
        let mut re = sig.clone();
        let mut im = vec![0f32; n];
        fft(&mut re, &mut im);
        for k in 0..n {
            let (mut nr, mut ni) = (0f32, 0f32);
            for t in 0..n {
                let ang = -2.0 * std::f32::consts::PI * k as f32 * t as f32 / n as f32;
                nr += sig[t] * ang.cos();
                ni += sig[t] * ang.sin();
            }
            assert!((re[k] - nr).abs() < 1e-3, "实部 k={k}: {} vs {nr}", re[k]);
            assert!((im[k] - ni).abs() < 1e-3, "虚部 k={k}: {} vs {ni}", im[k]);
        }
    }

    #[test]
    fn empty_input_is_safe() {
        let cfg = MfccConfig::default();
        assert!(mfcc(&[], &cfg).is_empty());
        let empty: &[Vec<f32>] = &[];
        assert_eq!(dtw_distance(empty, empty), f32::INFINITY);
        assert_eq!(similarity(empty, empty, 40.0), 0.0);
    }
}
