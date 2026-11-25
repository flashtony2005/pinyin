//! 跟读评测的端侧推理（手写前向，无 ONNX / 无第三方运行时）。
//!
//! 网络结构与 `tools/_p0/b_model.py` 的 `TinyNet` 一一对应：
//!   in [2][64][198]
//!   → conv3x3 2→24 + ReLU + maxpool2x2     [24][32][99]
//!   → conv3x3 24→48 + ReLU + maxpool2x2    [48][16][49]
//!   → conv3x3 48→96 + ReLU + maxpool(2,1)  [96][8][49]
//!   → conv 8x1 96→128 + ReLU               [128][1][49]
//!   → 时间轴 mean/max 拼接 [256] → fc [128] → L2 归一
//!   → 与类中心做点积 ×scale                [n_cls]
//!
//! 权重来自 `src/data/spell_net.bin`（`tools/_p0/b_weights.py` 导出），
//! 词表来自 `src/data/spell_vocab.txt`。

use std::sync::OnceLock;

/// 静态权重字节。
const WEIGHTS: &[u8] = include_bytes!("data/spell_net.bin");
/// 词表（每行一个 音节+声调，如 `bo1`）。
const VOCAB_TXT: &str = include_str!("data/spell_vocab.txt");

/// 通道数。
const CH: [usize; 4] = [24, 48, 96, 128];
/// 嵌入维度。
const EMB: usize = 128;

use crate::ctc_feat::{N_FRAMES, N_MELS};

/// 一次判定的结果。
#[derive(Clone, Debug)]
pub struct Verdict {
    /// 目标音节在词表中的下标。
    pub target: usize,
    /// 模型 top1 音节下标。
    pub top1: usize,
    /// 模型判定的最佳音节文本。
    pub top1_text: &'static str,
    /// top1 的余弦相似度。
    pub top1_cos: f32,
    /// 目标音节余弦相似度。
    pub target_cos: f32,
    /// 间隔（目标 − 次优，且次优不含目标），余弦单位。
    pub margin: f32,
    /// 间隔换算到 logit 单位（余弦 × 尺度），便于显示。
    pub logit_margin: f32,
    /// 是否判定"读对了"。
    pub pass: bool,
}

struct Net {
    w1: Vec<f32>, b1: Vec<f32>,
    w2: Vec<f32>, b2: Vec<f32>,
    w3: Vec<f32>, b3: Vec<f32>,
    w4: Vec<f32>, b4: Vec<f32>,
    wfc: Vec<f32>, bfc: Vec<f32>,
    class_w: Vec<f32>,
    scale: f32,
    n_cls: usize,
    vocab: Vec<&'static str>,
    /// 去重后的音节（词表 `xxx+声调` 的 `xxx` 部分）。
    syl_names: Vec<&'static str>,
    /// 每个类别属于哪个音节（`syl_names` 的下标）。
    cls2syl: Vec<u16>,
}

static NET: OnceLock<Option<Net>> = OnceLock::new();

fn take<'a>(buf: &'a [u8], off: &mut usize, n: usize) -> Vec<f32> {
    let bytes = &buf[*off..*off + n * 4];
    *off += n * 4;
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

impl Net {
    fn parse() -> Option<Net> {
        let n_cls: usize = VOCAB_TXT.lines().filter(|l| !l.trim().is_empty()).count();
        let vocab: Vec<&'static str> = VOCAB_TXT.lines().filter(|l| !l.trim().is_empty()).collect();
        let (c1, c2, c3, c4) = (CH[0], CH[1], CH[2], CH[3]);
        let need = (c1 * 2 * 9 + c1)
            + (c2 * c1 * 9 + c2)
            + (c3 * c2 * 9 + c3)
            + (c4 * c3 * 8 + c4)
            + (EMB * 2 * c4 + EMB)
            + (n_cls * EMB)
            + 1;
        if WEIGHTS.len() != need * 4 {
            eprintln!(
                "spell_net 权重长度不符：期望 {} 字节，实际 {}",
                need * 4,
                WEIGHTS.len()
            );
            return None;
        }
        let mut off = 0usize;
        let mut next = |n: usize| take(WEIGHTS, &mut off, n);
        let mut syl_names: Vec<&'static str> = Vec::new();
        let mut cls2syl: Vec<u16> = Vec::with_capacity(n_cls);
        for &k in &vocab {
            let s = syllable_of(k);
            let idx = match syl_names.iter().position(|&x| x == s) {
                Some(i) => i,
                None => {
                    syl_names.push(s);
                    syl_names.len() - 1
                }
            };
            cls2syl.push(idx as u16);
        }
        Some(Net {
            w1: next(c1 * 2 * 9), b1: next(c1),
            w2: next(c2 * c1 * 9), b2: next(c2),
            w3: next(c3 * c2 * 9), b3: next(c3),
            w4: next(c4 * c3 * 8), b4: next(c4),
            wfc: next(EMB * 2 * c4), bfc: next(EMB),
            class_w: next(n_cls * EMB),
            scale: f32::from_le_bytes([WEIGHTS[off], WEIGHTS[off + 1], WEIGHTS[off + 2], WEIGHTS[off + 3]]),
            n_cls,
            vocab,
            syl_names,
            cls2syl,
        })
    }
}

/// 词表 key（`bo1`）→ 音节（`bo`）。无数字后缀时原样返回。
fn syllable_of(key: &'static str) -> &'static str {
    match key.as_bytes().last() {
        Some(b) if b.is_ascii_digit() => &key[..key.len() - 1],
        _ => key,
    }
}

fn net() -> Option<&'static Net> {
    NET.get_or_init(Net::parse).as_ref()
}

fn conv3x3(
    x: &[f32], ci: usize, h: usize, w: usize,
    wt: &[f32], b: &[f32], co: usize,
) -> Vec<f32> {
    let mut out = vec![0f32; co * h * w];
    for o in 0..co {
        let bias = b[o];
        let wbase = o * ci * 9;
        for y in 0..h {
            for xx in 0..w {
                let mut acc = bias;
                for ky in 0..3usize {
                    let sy = y as isize + ky as isize - 1;
                    if sy < 0 || sy >= h as isize {
                        continue;
                    }
                    for kx in 0..3usize {
                        let sx = xx as isize + kx as isize - 1;
                        if sx < 0 || sx >= w as isize {
                            continue;
                        }
                        let wgt_base = wbase + (ky * 3 + kx);
                        for i in 0..ci {
                            let wv = wt[wgt_base + i * 9];
                            if wv != 0.0 {
                                acc += x[i * h * w + sy as usize * w + sx as usize] * wv;
                            }
                        }
                    }
                }
                out[o * h * w + y * w + xx] = acc.max(0.0);
            }
        }
    }
    out
}

fn conv_freq8(x: &[f32], ci: usize, h: usize, w: usize, wt: &[f32], b: &[f32], co: usize) -> Vec<f32> {
    debug_assert_eq!(h, 8);
    let mut out = vec![0f32; co * w];
    for o in 0..co {
        let wbase = o * ci * 8;
        for xx in 0..w {
            let mut acc = b[o];
            for i in 0..ci {
                for k in 0..8usize {
                    acc += x[i * h * w + k * w + xx] * wt[wbase + i * 8 + k];
                }
            }
            out[o * w + xx] = acc.max(0.0);
        }
    }
    out
}

fn maxpool(x: &[f32], c: usize, h: usize, w: usize, kh: usize, kw: usize) -> (Vec<f32>, usize, usize) {
    let (oh, ow) = (h / kh, w / kw);
    let mut out = vec![0f32; c * oh * ow];
    for ch in 0..c {
        for y in 0..oh {
            for xx in 0..ow {
                let mut m = f32::NEG_INFINITY;
                for dy in 0..kh {
                    for dx in 0..kw {
                        let v = x[ch * h * w + (y * kh + dy) * w + xx * kw + dx];
                        if v > m {
                            m = v;
                        }
                    }
                }
                out[ch * oh * ow + y * ow + xx] = m;
            }
        }
    }
    (out, oh, ow)
}

impl Net {
    fn embed(&self, feat: &[f32]) -> Vec<f32> {
        let (h, w) = (N_MELS, N_FRAMES);
        let a = conv3x3(feat, 2, h, w, &self.w1, &self.b1, CH[0]);
        let (a, h, w) = maxpool(&a, CH[0], h, w, 2, 2);
        let b = conv3x3(&a, CH[0], h, w, &self.w2, &self.b2, CH[1]);
        let (b, h, w) = maxpool(&b, CH[1], h, w, 2, 2);
        let c = conv3x3(&b, CH[1], h, w, &self.w3, &self.b3, CH[2]);
        let (c, h, w) = maxpool(&c, CH[2], h, w, 2, 1);
        let d = conv_freq8(&c, CH[2], h, w, &self.w4, &self.b4, CH[3]); // [CH3][w]
        let t = w;
        let mut feat_v = vec![0f32; 2 * CH[3]];
        for ch in 0..CH[3] {
            let row = &d[ch * t..(ch + 1) * t];
            let mean = row.iter().sum::<f32>() / t as f32;
            let max = row.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            feat_v[ch] = mean;
            feat_v[CH[3] + ch] = max;
        }
        let mut emb = vec![0f32; EMB];
        for o in 0..EMB {
            let mut acc = self.bfc[o];
            for i in 0..2 * CH[3] {
                acc += feat_v[i] * self.wfc[o * 2 * CH[3] + i];
            }
            emb[o] = acc;
        }
        let norm = emb.iter().map(|v| v * v).sum::<f32>().sqrt() + 1e-6;
        for v in emb.iter_mut() {
            *v /= norm;
        }
        emb
    }

    fn cosines(&self, emb: &[f32]) -> Vec<f32> {
        let mut out = vec![0f32; self.n_cls];
        for (c, slot) in out.iter_mut().enumerate() {
            let row = &self.class_w[c * EMB..(c + 1) * EMB];
            *slot = row.iter().zip(emb).map(|(a, b)| a * b).sum();
        }
        out
    }
}

/// 词表（音节+声调）。
pub fn vocab() -> &'static [&'static str] {
    match net() {
        Some(n) => &n.vocab,
        None => &[],
    }
}

/// 词表里查下标。
pub fn index_of(key: &str) -> Option<usize> {
    net().and_then(|n| n.vocab.iter().position(|&v| v == key))
}

/// 音节是否在词表里（忽略声调）。
pub fn syllable_index_of(syllable: &str) -> Option<usize> {
    net().and_then(|n| n.syl_names.iter().position(|&s| s == syllable))
}

/// 通过率门槛（余弦间隔）。保守取值：宁可让孩子重说，也不放行乱读。
pub const MARGIN_PASS: f32 = 0.06;

/// 音节级通过门槛（目标音节组 − 其它音节组）。
///
/// 实测（`tools/_p0/b_eval_syl.py final`）：0.0 这一档同源 100%、
/// 跨音源 21.7%、乱读误放 0%。
pub const SYL_MARGIN_PASS: f32 = 0.0;

/// 音节级判定结果（不看声调）。
#[derive(Clone, Debug)]
pub struct SylVerdict {
    /// 要求的目标音节。
    pub syllable: &'static str,
    /// 目标音节的得分（该音节所有声调变体的最高余弦）。
    pub target_cos: f32,
    /// 其它音节里的最高分。
    pub best_other_cos: f32,
    /// 间隔 = 目标 − 其它。
    pub margin: f32,
    /// 模型 top1 类别所属音节。
    pub top1_syl: &'static str,
    /// 模型 top1 类别（音节+声调）。
    pub top1_key: &'static str,
    /// top1 余弦。
    pub top1_cos: f32,
    /// 是否判定"读对了"。
    pub pass: bool,
}

/// 音节级判定：孩子读的是不是 `syllable`，不追究声调。
///
/// 词表里没有这个音节时返回 `None`（调用方退回旧路径）。
pub fn assess_syllable(pcm: &[f32], syllable: &str) -> Option<SylVerdict> {
    let n = net()?;
    let ti = n.syl_names.iter().position(|&s| s == syllable)?;
    // 与训练一致：先裁掉首尾静音，再居中放进 2 秒窗。
    let pcm = crate::ctc_feat::trim_speech(pcm);
    let feat = crate::ctc_feat::features(&pcm);
    let emb = n.embed(&feat);
    let cos = n.cosines(&emb);

    // 每个音节取自己所有声调变体的最高分，再比音节之间。
    let mut per = vec![f32::NEG_INFINITY; n.syl_names.len()];
    for (c, &v) in cos.iter().enumerate() {
        let s = n.cls2syl[c] as usize;
        if v > per[s] {
            per[s] = v;
        }
    }
    let target_cos = per[ti];
    let mut best_other = f32::NEG_INFINITY;
    for (s, &v) in per.iter().enumerate() {
        if s != ti && v > best_other {
            best_other = v;
        }
    }
    let margin = target_cos - best_other;

    let mut top1 = 0usize;
    for (i, &v) in cos.iter().enumerate() {
        if v > cos[top1] {
            top1 = i;
        }
    }
    Some(SylVerdict {
        syllable: n.syl_names[ti],
        target_cos,
        best_other_cos: best_other,
        margin,
        top1_syl: n.syl_names[n.cls2syl[top1] as usize],
        top1_key: n.vocab[top1],
        top1_cos: cos[top1],
        pass: margin >= SYL_MARGIN_PASS,
    })
}

/// 音节级展示分（0..=100）。间隔 −0.25 → 0 分，+0.30 → 100 分。
pub fn percent_syl(v: &SylVerdict) -> u32 {
    let s = ((v.margin + 0.25) / 0.55).clamp(0.0, 1.0);
    (s * 100.0).round() as u32
}

/// 对一段 16k 单声道 PCM 做"是不是目标音节"的判定。
pub fn assess(pcm: &[f32], target: usize) -> Option<Verdict> {
    let n = net()?;
    if target >= n.n_cls {
        return None;
    }
    let feat = crate::ctc_feat::features(pcm);
    let emb = n.embed(&feat);
    let cos = n.cosines(&emb);
    let mut top1 = 0usize;
    for (i, &v) in cos.iter().enumerate() {
        if v > cos[top1] {
            top1 = i;
        }
    }
    let mut best_other = f32::NEG_INFINITY;
    for (i, &v) in cos.iter().enumerate() {
        if i != target && v > best_other {
            best_other = v;
        }
    }
    let margin = cos[target] - best_other;
    Some(Verdict {
        target,
        top1,
        top1_text: n.vocab[top1],
        top1_cos: cos[top1],
        target_cos: cos[target],
        margin,
        logit_margin: margin * n.scale,
        pass: margin >= MARGIN_PASS,
    })
}

/// 目标音节的展示用百分制得分（0..=100）。
pub fn percent(v: &Verdict) -> u32 {
    // 余弦 0.30 → 0 分，0.95 → 100 分；再按间隔扣分，避免"自信地读错"拿高分
    let base = ((v.target_cos - 0.30) / 0.65).clamp(0.0, 1.0);
    let pen = (v.margin / MARGIN_PASS).clamp(0.0, 1.0);
    let s = base * 0.35 + pen * 0.65;
    (s * 100.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOLDEN: &[u8] = include_bytes!("data/golden.bin");

    fn f32s(bytes: &[u8]) -> Vec<f32> {
        bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }

    #[test]
    fn weights_layout_matches_rust() {
        if WEIGHTS.is_empty() {
            eprintln!("跳过：尚未导出 spell_net.bin");
            return;
        }
        assert!(net().is_some(), "权重长度与 Rust 侧布局不一致");
    }

    #[test]
    fn golden_logits_match_python() {
        if GOLDEN.len() < 8 || WEIGHTS.is_empty() {
            eprintln!("跳过：缺少 golden.bin 或 spell_net.bin");
            return;
        }
        let n = i32::from_le_bytes([GOLDEN[0], GOLDEN[1], GOLDEN[2], GOLDEN[3]]) as usize;
        let n_cls = i32::from_le_bytes([GOLDEN[4], GOLDEN[5], GOLDEN[6], GOLDEN[7]]) as usize;
        let feat_len = 2 * N_MELS * N_FRAMES;
        let mut off = 8usize;
        for k in 0..n {
            let feat = f32s(&GOLDEN[off..off + feat_len * 4]);
            off += feat_len * 4;
            let want = f32s(&GOLDEN[off..off + n_cls * 4]);
            off += n_cls * 4;
            let n_ref = net().expect("weights");
            let emb = n_ref.embed(&feat);
            let got = n_ref.cosines(&emb);
            let mut max_err = 0f32;
            for i in 0..n_cls {
                max_err = max_err.max((got[i] * n_ref.scale - want[i]).abs());
            }
            eprintln!("golden #{k}: 用余弦对齐，最大误差 {max_err:.4}");
            assert!(max_err < 5e-3, "golden #{k} 误差 {max_err}");
        }
    }
}
