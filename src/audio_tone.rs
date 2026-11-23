//! 声调评测核心算法（纯 Rust，零外部依赖，端侧可跑）。
//!
//! 流水线：PCM → 分帧 → 归一化自相关（NSDF）提 F0 → 去静音/中值平滑
//!       → 转音分 → 前/中/后三段调型判定 → 四声。
//!
//! 这一层不碰麦克风、不解码文件，输入就是 16 kHz 的 f32 PCM，因此可以在
//! 桌面 `cargo test` 里用合成信号验证，等真机录音链路（JNI）就绪后直接接入。
//!
//! 判据用「相对音分」而非绝对 Hz：不同孩子嗓音基频差异大，只有**轮廓**
//! （平/升/降/先降后升）才是声调的本质，这样阈值对每个孩子都成立。

/// 四声（一声阴平、二声阳平、三声上声、四声去声）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    One,
    Two,
    Three,
    Four,
}

impl Tone {
    pub fn name(self) -> &'static str {
        match self {
            Tone::One => "一声",
            Tone::Two => "二声",
            Tone::Three => "三声",
            Tone::Four => "四声",
        }
    }
}

/// 一次分析的结果。
#[derive(Debug, Clone)]
pub struct ToneAnalysis {
    /// 判定的声调。
    pub tone: Tone,
    /// 置信度 0..1：调型特征越明显越高（供 UI 决定鼓励还是重来）。
    pub confidence: f32,
    /// 每帧基频（Hz）；无声帧为 0。前 3 帧因自相关窗口未填满通常也是 0。
    pub f0_hz: Vec<f32>,
    /// 前 / 中 / 后 三段的平均音分（相对全句中位基频）。
    pub contour: [f32; 3],
}

const FRAME_SIZE: usize = 512; // @16k = 32 ms，足够容纳低至 70 Hz 的两个周期
const HOP: usize = 160; // 10 ms 帧移
const MIN_HZ: f32 = 70.0; // 覆盖儿童低音
const MAX_HZ: f32 = 450.0; // 覆盖儿童高音
const RMS_GATE: f32 = 0.012; // 低于此能量的帧视为静音

/// 分析一段 16 kHz f32 PCM 的声调。
pub fn analyze_tone(pcm: &[f32], sample_rate: u32) -> ToneAnalysis {
    let sr = sample_rate.max(1);
    let frame = (FRAME_SIZE as f32 * sr as f32 / 16_000.0).round() as usize;
    let hop = (HOP as f32 * sr as f32 / 16_000.0).round() as usize;
    let min_lag = (sr as f32 / MAX_HZ).floor() as usize;
    let max_lag = (sr as f32 / MIN_HZ).ceil() as usize;

    let mut f0 = Vec::new();
    if pcm.len() >= frame && max_lag < frame {
        let mut start = 0;
        while start + frame <= pcm.len() {
            let hz = frame_f0(&pcm[start..start + frame], sr, min_lag, max_lag);
            f0.push(hz.unwrap_or(0.0));
            start += hop;
        }
    }

    // 中值滤波去孤立野点（自相关偶尔跳八度）。
    let f0 = median_filter(&f0, 3);
    // 取有声帧的中位基频作为参考，转成相对音分。
    let voiced: Vec<f32> = f0.iter().copied().filter(|&v| v > 0.0).collect();
    if voiced.is_empty() {
        return ToneAnalysis { tone: Tone::One, confidence: 0.0, f0_hz: f0, contour: [0.0; 3] };
    }
    let mut sorted = voiced.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let base = sorted[sorted.len() / 2];

    let cents: Vec<f32> = f0
        .iter()
        .map(|&v| if v > 0.0 { 1200.0 * (v / base).log2() } else { f32::NAN })
        .collect();
    let contour = three_point_contour(&cents);
    let (tone, confidence) = judge(&contour);
    ToneAnalysis { tone, confidence, f0_hz: f0, contour }
}

/// 单帧基频：去直流 + 汉明窗 + NSDF 自相关 + 抛物线插值。
fn frame_f0(frame: &[f32], sr: u32, min_lag: usize, max_lag: usize) -> Option<f32> {
    let mean = frame.iter().sum::<f32>() / frame.len() as f32;
    let energy = frame.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / frame.len() as f32;
    if energy.sqrt() < RMS_GATE {
        return None;
    }
    let mut buf = Vec::with_capacity(frame.len());
    for (i, &x) in frame.iter().enumerate() {
        let w = 0.54 - 0.46 * (2.0 * std::f32::consts::PI * i as f32 / (frame.len() - 1) as f32).cos();
        buf.push((x - mean) * w);
    }
    let denom0: f32 = buf.iter().map(|&x| x * x).sum();
    if denom0 < 1e-9 {
        return None;
    }

    // NSDF(lag) = Σ x[i]·x[i+lag] / √(Σx[i]² · Σx[i+lag]²)
    let mut nsdf = vec![0.0f32; max_lag + 1];
    for lag in min_lag..=max_lag {
        let mut num = 0.0f32;
        let mut later = 0.0f32;
        for i in 0..(buf.len() - lag) {
            num += buf[i] * buf[i + lag];
            later += buf[i + lag] * buf[i + lag];
        }
        nsdf[lag] = num / (denom0 * later).sqrt().max(1e-9);
    }
    let mut best = min_lag;
    for lag in (min_lag + 1)..=max_lag {
        if nsdf[lag] > nsdf[best] {
            best = lag;
        }
    }
    if nsdf[best] < 0.55 {
        return None;
    }
    // 抛物线插值细化峰位置，精度远高于整 lag。
    let (y1, y2, y3) = (
        nsdf[best.saturating_sub(1)],
        nsdf[best],
        nsdf[(best + 1).min(max_lag)],
    );
    let denom = y1 - 2.0 * y2 + y3;
    let delta = if denom.abs() > 1e-9 { 0.5 * (y1 - y3) / denom } else { 0.0 };
    let lag_f = best as f32 + delta.clamp(-1.0, 1.0);
    if lag_f <= 0.0 {
        return None;
    }
    Some(sr as f32 / lag_f)
}

/// 长度 3 的中值滤波，抹掉单帧跳八度的野点，不动调型。
fn median_filter(v: &[f32], k: usize) -> Vec<f32> {
    let mut out = v.to_vec();
    let half = k / 2;
    for i in half..v.len().saturating_sub(half) {
        let mut w = vec![v[i - half], v[i], v[i + half]];
        w.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        out[i] = w[half];
    }
    out
}

/// 把有效帧三等分，取各段音分中位数：[前, 中, 后]。
fn three_point_contour(cents: &[f32]) -> [f32; 3] {
    let valid: Vec<f32> = cents.iter().copied().filter(|v| v.is_finite()).collect();
    if valid.is_empty() {
        return [0.0; 3];
    }
    let mut out = [0.0f32; 3];
    for (seg, slot) in out.iter_mut().enumerate() {
        let a = valid.len() * seg / 3;
        let b = (valid.len() * (seg + 1) / 3).max(a + 1).min(valid.len());
        let mut part = valid[a..b].to_vec();
        part.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        *slot = part[part.len() / 2];
    }
    out
}

/// 声调判据（音分）：
/// - 中段明显低于前后 → 三声（先降后升的凹形）
/// - 后段高于前段 → 二声（升）
/// - 后段低于前段 → 四声（降）
/// - 否则 → 一声（平）
/// 置信度取「主特征超出阈值的幅度」，UI 据此决定表扬还是再来一次。
fn judge(c: &[f32; 3]) -> (Tone, f32) {
    let [first, mid, last] = *c;
    let dip = (first + last) * 0.5 - mid; // >0 表示中间凹陷
    let rise = last - first; // >0 升，<0 降

    if dip > 70.0 && rise.abs() < 260.0 {
        return (Tone::Three, (dip / 200.0).clamp(0.0, 1.0));
    }
    if rise > 90.0 {
        return (Tone::Two, (rise / 400.0).clamp(0.0, 1.0));
    }
    if rise < -90.0 {
        return (Tone::Four, ((-rise) / 400.0).clamp(0.0, 1.0));
    }
    let flat = 1.0 - (rise.abs() / 180.0).clamp(0.0, 1.0);
    (Tone::One, flat.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    /// 合成一个带 3 次谐波的嗓音，f0_fn(t) 给出 t∈[0,1] 时刻的基频（Hz）。
    fn synth(sr: u32, dur_ms: u32, f0_fn: impl Fn(f32) -> f32) -> Vec<f32> {
        let n = (sr as f64 * dur_ms as f64 / 1000.0) as usize;
        let mut out = Vec::with_capacity(n);
        let mut phase = 0.0f64;
        for i in 0..n {
            let t = i as f32 / n as f32;
            phase += 2.0 * PI * f0_fn(t) as f64 / sr as f64;
            let s = phase.sin() * 0.6 + (2.0 * phase).sin() * 0.3 + (3.0 * phase).sin() * 0.1;
            out.push((s * 0.5) as f32);
        }
        out
    }

    #[test]
    fn detects_level_tone() {
        let sr = 16_000;
        let pcm = synth(sr, 400, |_| 180.0);
        let r = analyze_tone(&pcm, sr);
        assert_eq!(r.tone, Tone::One, "一声 contour={:?}", r.contour);
    }

    #[test]
    fn detects_rising_tone() {
        let sr = 16_000;
        let pcm = synth(sr, 400, |t| 150.0 + 110.0 * t);
        let r = analyze_tone(&pcm, sr);
        assert_eq!(r.tone, Tone::Two, "二声 contour={:?}", r.contour);
    }

    #[test]
    fn detects_dipping_tone() {
        let sr = 16_000;
        // 先降后升的凹形：180 → 90 → 220。
        let pcm = synth(sr, 400, |t| {
            if t < 0.5 { 180.0 - 90.0 * (t / 0.5) } else { 90.0 + 130.0 * ((t - 0.5) / 0.5) }
        });
        let r = analyze_tone(&pcm, sr);
        assert_eq!(r.tone, Tone::Three, "三声 contour={:?}", r.contour);
    }

    #[test]
    fn detects_falling_tone() {
        let sr = 16_000;
        let pcm = synth(sr, 400, |t| 260.0 - 150.0 * t);
        let r = analyze_tone(&pcm, sr);
        assert_eq!(r.tone, Tone::Four, "四声 contour={:?}", r.contour);
    }

    #[test]
    fn silence_yields_zero_confidence() {
        let r = analyze_tone(&vec![0.0; 16_000], 16_000);
        assert_eq!(r.confidence, 0.0);
    }

    #[test]
    fn f0_tracks_known_pitch() {
        let sr = 16_000;
        let pcm = synth(sr, 400, |_| 220.0);
        let r = analyze_tone(&pcm, sr);
        let voiced: Vec<f32> = r.f0_hz.iter().copied().filter(|&v| v > 0.0).collect();
        assert!(!voiced.is_empty(), "应有有声帧");
        let mean = voiced.iter().sum::<f32>() / voiced.len() as f32;
        assert!((mean - 220.0).abs() < 8.0, "估计基频 {mean} 应接近 220");
    }
}
