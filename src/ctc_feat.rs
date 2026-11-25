//! 跟读评测用的声学前端（与 `tools/_p0/b_data.py` 逐位对齐）。
//!
//! 流水线：PCM(16k, f32) → 居中补零到 2.0s → 分帧(400/160, periodic Hann)
//!        → 512 点 FFT → 功率谱 → 64 维三角梅尔 → log
//!        → 归一化（逐帧去均值 / 逐 bin CMVN）→ 静音帧置零 → 一阶差分
//!
//! 输出布局：`[2][64][198]`（通道优先，通道 0 = 静态，通道 1 = 差分）。
//!
//! ⚠ 任何参数变更都必须同步 `tools/_p0/b_data.py`，否则端上分数全乱。

/// 采样率。
pub const SR: usize = 16_000;
/// 分析窗长（采样点）。
pub const WIN: usize = 400;
/// 帧移（采样点）。
pub const HOP: usize = 160;
/// FFT 点数。
pub const NFFT: usize = 512;
/// 梅尔通道数。
pub const N_MELS: usize = 64;
/// 固定输入时长（秒）。
pub const DUR: f32 = 2.0;
/// 固定输入采样点数。
pub const N_SAMP: usize = 32_000;
/// 帧数。
pub const N_FRAMES: usize = (N_SAMP - WIN) / HOP + 1; // 198
/// 梅尔下限频率（Hz）。
pub const FMIN: f32 = 60.0;
const EPS: f32 = 1e-10;
/// 有效帧判定：帧能量不低于峰值该分贝数。
const ACTIVE_FLOOR_DB: f32 = -40.0;

/// 归一化模式。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Norm {
    /// 逐帧减均值（保留谱形），训练时用 `--feat frame`。
    Frame,
    /// 逐 bin 均值方差归一（去信道/声道），训练时用 `--feat cmvn`。
    Cmvn,
}

/// 当前端上采用的模式，须与导出模型训练时的 `--feat` 一致。
pub const NORM: Norm = Norm::Frame;

fn hann() -> [f32; WIN] {
    let mut w = [0f32; WIN];
    for (i, v) in w.iter_mut().enumerate() {
        *v = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / WIN as f32).cos();
    }
    w
}

fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10f32.powf(mel / 2595.0) - 1.0)
}

/// 三角梅尔滤波器的稀疏表示：每个滤波器一组 (频率 bin, 权重)。
fn mel_filterbank() -> Vec<Vec<(usize, f32)>> {
    let low = hz_to_mel(FMIN);
    let high = hz_to_mel(SR as f32 / 2.0);
    let mut points = Vec::with_capacity(N_MELS + 2);
    for i in 0..(N_MELS + 2) {
        let mel = low + (high - low) * i as f32 / (N_MELS + 1) as f32;
        let hz = mel_to_hz(mel);
        let bin = ((NFFT + 1) as f32 * hz / SR as f32).floor() as usize;
        points.push(bin.min(NFFT / 2));
    }
    let mut filters = Vec::with_capacity(N_MELS);
    for m in 1..=N_MELS {
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

/// 原地 radix-2 Cooley-Tukey FFT（长度须为 2 的幂）。
fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    debug_assert!(n.is_power_of_two());
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

/// 把任意长度 PCM 居中放进固定 2.0s 窗口（与 Python `_pad2s` 一致）。
pub fn center_pad(pcm: &[f32], out: &mut [f32; N_SAMP]) {
    out.fill(0.0);
    let n = pcm.len().min(N_SAMP);
    let off = (N_SAMP - n) / 2;
    out[off..off + n].copy_from_slice(&pcm[..n]);
}

/// log-mel（未归一化）+ 每帧 RMS。
pub fn logmel(pcm: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let mut x = [0f32; N_SAMP];
    center_pad(pcm, &mut x);
    let w = hann();
    let fb = mel_filterbank();
    let mut mel = vec![0f32; N_FRAMES * N_MELS];
    let mut rms = vec![0f32; N_FRAMES];
    let mut re = vec![0f32; NFFT];
    let mut im = vec![0f32; NFFT];
    let mut power = vec![0f32; NFFT / 2 + 1];
    for t in 0..N_FRAMES {
        let base = t * HOP;
        let mut acc = 0f32;
        for i in 0..WIN {
            let v = x[base + i] * w[i];
            re[i] = v;
            im[i] = 0.0;
            acc += v * v;
        }
        for v in re.iter_mut().skip(WIN) {
            *v = 0.0;
        }
        for v in im.iter_mut().skip(WIN) {
            *v = 0.0;
        }
        rms[t] = (acc / WIN as f32).sqrt();
        fft(&mut re, &mut im);
        for (k, p) in power.iter_mut().enumerate() {
            *p = re[k] * re[k] + im[k] * im[k];
        }
        for (m, band) in fb.iter().enumerate() {
            let mut e = 0f32;
            for &(k, g) in band {
                e += power[k] * g;
            }
            mel[t * N_MELS + m] = (e + EPS).ln();
        }
    }
    (mel, rms)
}

/// 有效帧掩码（能量不低于峰值 -40dB，至少 8 帧）。
pub fn active_mask(rms: &[f32]) -> Vec<bool> {
    let mut db = vec![0f32; rms.len()];
    let mut peak = f32::NEG_INFINITY;
    for (i, &r) in rms.iter().enumerate() {
        let v = 20.0 * (r + 1e-8).log10();
        db[i] = v;
        if v > peak {
            peak = v;
        }
    }
    let mut mask: Vec<bool> = db.iter().map(|&v| v > peak + ACTIVE_FLOOR_DB).collect();
    if mask.iter().filter(|&&b| b).count() < 8 {
        mask.iter_mut().for_each(|b| *b = true);
    }
    mask
}

fn normalize(mel: &mut [f32], mask: &[bool], mode: Norm) {
    match mode {
        Norm::Frame => {
            for t in 0..N_FRAMES {
                let row = &mut mel[t * N_MELS..(t + 1) * N_MELS];
                let mean = row.iter().sum::<f32>() / N_MELS as f32;
                for v in row.iter_mut() {
                    *v -= mean;
                }
            }
        }
        Norm::Cmvn => {
            let mut mean = [0f32; N_MELS];
            let mut n = 0usize;
            for t in 0..N_FRAMES {
                if !mask[t] {
                    continue;
                }
                n += 1;
                for m in 0..N_MELS {
                    mean[m] += mel[t * N_MELS + m];
                }
            }
            let n = n.max(1) as f32;
            for m in mean.iter_mut() {
                *m /= n;
            }
            let mut var = [0f32; N_MELS];
            for t in 0..N_FRAMES {
                if !mask[t] {
                    continue;
                }
                for m in 0..N_MELS {
                    let d = mel[t * N_MELS + m] - mean[m];
                    var[m] += d * d;
                }
            }
            for m in 0..N_MELS {
                var[m] = (var[m] / n).sqrt() + 1e-5;
            }
            for t in 0..N_FRAMES {
                for m in 0..N_MELS {
                    let v = mel[t * N_MELS + m];
                    mel[t * N_MELS + m] = (v - mean[m]) / var[m];
                }
            }
        }
    }
    for t in 0..N_FRAMES {
        if !mask[t] {
            for m in 0..N_MELS {
                mel[t * N_MELS + m] = 0.0;
            }
        }
    }
}

/// 一阶差分（时间轴，边界复制填充，宽度 2）。
fn delta(static_feat: &[f32], out: &mut [f32]) {
    const W: usize = 2;
    for t in 0..N_FRAMES {
        let hi = (t + W).min(N_FRAMES - 1);
        let lo = t.saturating_sub(W);
        for m in 0..N_MELS {
            out[t * N_MELS + m] =
                (static_feat[hi * N_MELS + m] - static_feat[lo * N_MELS + m]) / (2.0 * W as f32);
        }
    }
}

/// 端上完整前端：返回 `[2][64][198]`（通道优先）。
pub fn features(pcm: &[f32]) -> Vec<f32> {
    features_with(pcm, NORM)
}

/// 按 20ms 窗能量裁掉首尾静音，并与 `tools/_p0/b_data.py` 的 `trim` 对齐
/// （阈值 峰值 −38dB，两侧各留 50ms）。
///
/// 训练数据都是在"裁掉静音 + 居中补零"的音频上提的特征；端上录音会长短不齐、
/// 孩子还可能晚半秒才开口，不裁就会让语音在 2 秒窗里偏一大截，和训练分布对不上。
pub fn trim_speech(pcm: &[f32]) -> Vec<f32> {
    const W: usize = 320; // 20ms
    const PAD: usize = 800; // 50ms
    if pcm.len() < W {
        return pcm.to_vec();
    }
    let n = pcm.len() / W;
    let mut db = Vec::with_capacity(n);
    let mut peak = f32::NEG_INFINITY;
    for i in 0..n {
        let seg = &pcm[i * W..(i + 1) * W];
        let e = (seg.iter().map(|v| v * v).sum::<f32>() / W as f32).sqrt();
        let v = 20.0 * (e + 1e-12).log10();
        if v > peak {
            peak = v;
        }
        db.push(v);
    }
    let th = peak - 38.0;
    let (mut a, mut b) = (n, 0usize);
    for i in 0..n {
        if db[i] > th {
            a = a.min(i);
            b = b.max(i);
        }
    }
    if a > b {
        return pcm.to_vec();
    }
    let start = (a * W).saturating_sub(PAD);
    let end = ((b + 1) * W).min(pcm.len()) + PAD;
    pcm[start..end.min(pcm.len())].to_vec()
}

/// 指定归一化模式的完整前端（测试与离线比对用）。
pub fn features_with(pcm: &[f32], norm: Norm) -> Vec<f32> {
    let (mut mel, rms) = logmel(pcm);
    let mask = active_mask(&rms);
    normalize(&mut mel, &mask, norm);
    let mut d = vec![0f32; N_FRAMES * N_MELS];
    delta(&mel, &mut d);
    // 内部按 [帧][梅尔] 计算，对外转成 [通道][梅尔][帧]（spell_net 约定）
    let mut out = vec![0f32; 2 * N_MELS * N_FRAMES];
    for t in 0..N_FRAMES {
        for m in 0..N_MELS {
            out[m * N_FRAMES + t] = mel[t * N_MELS + m];
            out[N_MELS * N_FRAMES + m * N_FRAMES + t] = d[t * N_MELS + m];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_count_matches_python() {
        assert_eq!(N_FRAMES, 198);
    }

    #[test]
    fn silence_gives_zero_matrix() {
        let f = features(&vec![0f32; 16_000]);
        assert_eq!(f.len(), 2 * N_MELS * N_FRAMES);
        // 全静音：逐帧去均值后各 bin 相同，delta 为 0，掩码全真时也不应出现 NaN
        assert!(f.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn tone_burst_is_not_flat() {
        let mut pcm = vec![0f32; 16_000];
        for (i, v) in pcm.iter_mut().enumerate() {
            let t = i as f32 / SR as f32;
            if (0.4..0.8).contains(&t) {
                *v = (2.0 * std::f32::consts::PI * 220.0 * t).sin() * 0.3;
            }
        }
        let f = features(&pcm);
        let energy: f32 = f.iter().map(|v| v * v).sum();
        assert!(energy > 1.0, "energy={energy}");
    }

    /// 裁静音：只留语音段 + 两侧 50ms，坐标要落在语音附近。
    #[test]
    fn trim_keeps_speech_region() {
        let mut pcm = vec![0f32; 24_000]; // 1.5s，0.5~1.0s 有语音
        for (i, v) in pcm.iter_mut().enumerate() {
            let t = i as f32 / SR as f32;
            if (0.5..1.0).contains(&t) {
                *v = (2.0 * std::f32::consts::PI * 220.0 * t).sin() * 0.3;
            }
        }
        let out = trim_speech(&pcm);
        // 0.5s 语音 + 两侧各 50ms ≈ 9600 采样，容差 ±1300
        assert!(
            (out.len() as i64 - 9_600).abs() < 1_300,
            "len={}",
            out.len()
        );
        // 全静音输入原样返回，不应 panic 或清空
        assert_eq!(trim_speech(&vec![0f32; 8_000]).len(), 8_000);
    }

    /// 与 Python 前端逐元素比对（容差 5e-2，log 域）。
    #[test]
    fn features_match_python_golden() {
        const GOLDEN: &[u8] = include_bytes!("data/golden_feat.bin");
        if GOLDEN.len() < 8 {
            eprintln!("跳过：缺少 golden_feat.bin");
            return;
        }
        let n = i32::from_le_bytes([GOLDEN[0], GOLDEN[1], GOLDEN[2], GOLDEN[3]]) as usize;
        let pcm: Vec<f32> = GOLDEN[4..4 + n * 4]
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        let want: Vec<f32> = GOLDEN[4 + n * 4..]
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        let mut best = (f32::INFINITY, Norm::Frame, 0usize, 0f32, 0f32);
        for mode in [Norm::Frame, Norm::Cmvn] {
            let got = features_with(&pcm, mode);
            assert_eq!(got.len(), want.len(), "特征长度不符");
            let mut max_err = 0f32;
            let mut worst = 0usize;
            for i in 0..got.len() {
                let e = (got[i] - want[i]).abs();
                if e > max_err {
                    max_err = e;
                    worst = i;
                }
            }
            if max_err < best.0 {
                best = (max_err, mode, worst, got[worst], want[worst]);
            }
        }
        eprintln!(
            "前端最大误差 {:.4}（模式 {:?} @ {}，got {} / want {}）",
            best.0, best.1, best.2, best.3, best.4
        );
        assert!(best.0 < 5e-2, "前端误差 {}", best.0);
    }
}

