//! 试运行对比：同一段录音分别过「新模型 `spell_net`」和「旧模板 `letter_match`」，
//! 打印两边的判定，用来判断端上到底该用哪个、差多少。
//!
//! 手工跑（不参与默认测试）：
//! ```text
//! set COMPARE_DIR=F:\project\pinyin-pal\sdcard_rec
//! cargo test --lib compare_dir -- --ignored --nocapture
//! ```
//! 文件名约定：`<音节>_<任意>.wav`（如 `bo_1732.wav`），也接受 `<字母>.wav`。

use std::path::{Path, PathBuf};

use crate::data::letter_spell_keys::LETTER_SPELL_KEYS;
use crate::data::letter_templates::LETTERS;

/// 读 16bit PCM WAV（单声道），返回 (采样, 采样率)。
pub(crate) fn read_wav(path: &Path) -> Option<(Vec<f32>, u32)> {
    let b = std::fs::read(path).ok()?;
    if b.len() < 44 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return None;
    }
    let mut off = 12usize;
    let mut ch = 1u16;
    let mut bits = 16u16;
    let mut sr = 16_000u32;
    while off + 8 <= b.len() {
        let id = &b[off..off + 4];
        let len = u32::from_le_bytes([b[off + 4], b[off + 5], b[off + 6], b[off + 7]]) as usize;
        let body = off + 8;
        match id {
            b"fmt " => {
                ch = u16::from_le_bytes([b[body + 2], b[body + 3]]);
                sr = u32::from_le_bytes([b[body + 4], b[body + 5], b[body + 6], b[body + 7]]);
                bits = u16::from_le_bytes([b[body + 14], b[body + 15]]);
            }
            b"data" => {
                let end = (body + len).min(b.len());
                let mut out = Vec::with_capacity((end - body) / 2);
                let mut i = body;
                while i + 1 < end {
                    let v = i16::from_le_bytes([b[i], b[i + 1]]) as f32 / 32768.0;
                    out.push(v);
                    i += 2;
                }
                if ch > 1 {
                    let c = ch as usize;
                    out = out
                        .chunks(c)
                        .map(|f| f.iter().sum::<f32>() / c as f32)
                        .collect();
                }
                let _ = bits;
                return Some((out, sr));
            }
            _ => {}
        }
        off = body + len + (len & 1);
    }
    None
}

/// 文件名 → (目标音节, 字母下标)
fn target_of(path: &Path) -> Option<(String, usize)> {
    let stem = path.file_stem()?.to_string_lossy().to_string();
    let head = stem.split('_').next()?.to_lowercase();
    // 1) 直接是字母（b / p / zh ...）
    if let Some(i) = LETTERS.iter().position(|&l| l == head) {
        return Some((LETTER_SPELL_KEYS[i].to_string(), i));
    }
    // 2) 是音节（bo / zhi / a ...）→ 反查字母
    LETTER_SPELL_KEYS
        .iter()
        .position(|&k| k == head)
        .map(|i| (head, i))
}

#[test]
#[ignore = "手工对比：需要 COMPARE_DIR 指向录音目录"]
fn compare_dir() {
    let dir = std::env::var("COMPARE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sdcard_rec"));
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "wav"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    if files.is_empty() {
        eprintln!("{dir:?} 里没有 wav");
        return;
    }

    let (mut net_ok, mut dtw_ok, mut both, mut n) = (0, 0, 0, 0);
    println!(
        "{:<22} {:>6} {:>8} {:>7} {:>6} {:>8} {:>8}",
        "文件", "目标", "听到", "间隔", "net", "dtw分", "dtw"
    );
    for f in &files {
        let Some((target, idx)) = target_of(f) else {
            println!("{:<22} 文件名看不懂，跳过", f.file_name().unwrap().to_string_lossy());
            continue;
        };
        let Some((pcm, sr)) = read_wav(f) else {
            println!("{:<22} 读不了", f.file_name().unwrap().to_string_lossy());
            continue;
        };
        // 统一到 16k（App 落盘的已经是 16k；外部素材可能是别的采样率）
        let pcm = if sr == 16_000 {
            pcm
        } else {
            let ratio = 16_000f64 / sr as f64;
            let n_out = (pcm.len() as f64 * ratio) as usize;
            (0..n_out)
                .map(|i| {
                    let x = i as f64 / ratio;
                    let a = x.floor() as usize;
                    let t = (x - a as f64) as f32;
                    let v0 = pcm.get(a).copied().unwrap_or(0.0);
                    let v1 = pcm.get(a + 1).copied().unwrap_or(v0);
                    v0 + (v1 - v0) * t
                })
                .collect()
        };

        let net = crate::spell_net::assess_syllable(&pcm, &target);
        let dtw = crate::letter_match::assess(&pcm, 16_000, idx);
        n += 1;
        let net_pass = net.as_ref().map(|v| v.pass).unwrap_or(false);
        let dtw_pass = matches!(dtw.verdict, crate::letter_match::Verdict::Pass { .. });
        if net_pass {
            net_ok += 1;
        }
        if dtw_pass {
            dtw_ok += 1;
        }
        if net_pass && dtw_pass {
            both += 1;
        }
        println!(
            "{:<22} {:>6} {:>8} {:>7.3} {:>6} {:>8.3} {:>8}",
            f.file_name().unwrap().to_string_lossy(),
            target,
            net.as_ref().map(|v| v.top1_syl).unwrap_or("-"),
            net.as_ref().map(|v| v.margin).unwrap_or(f32::NAN),
            if net_pass { "过" } else { "拒" },
            dtw.overall,
            if dtw_pass { "过" } else { "拒" },
        );
    }
    println!(
        "\n共 {n} 条：新模型过 {net_ok}、旧模板过 {dtw_ok}、都过 {both}"
    );
}
