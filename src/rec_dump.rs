//! 试运行期的录音落盘：把每次跟读的原始 PCM 存成 WAV，便于事后离线分析/微调。
//!
//! 同一段录音写两份：
//! - `${dataDir}/rec/`（App 内部目录）：`nsh.json` 的 `capabilities.fs.read` 授权，
//!   面板上的「回放」靠它播；
//! - `/storage/emulated/0/Android/data/<包名>/files/rec/`（外部私有目录）：
//!   `adb pull` 直接能取，不需要 root / debuggable。
//!
//! 外部目录里还有 `index.txt`（每次判定）与 `labels.tsv`（人工「对/错」标注）。

use std::io::Write;
use std::path::{Path, PathBuf};

use nsh_core::element::data_dir;

/// Android 包名（与 `nsh.json` 的 `id` 一致）。
const ANDROID_ID: &str = "com.lushaohui.pinyinpal";

/// 目录里最多保留多少个文件（防止长期试运行把手机塞满）。
const MAX_FILES: usize = 600;

fn internal_dir() -> Option<PathBuf> {
    let d = data_dir()?.join("rec");
    std::fs::create_dir_all(&d).ok()?;
    Some(d)
}

/// 共享目录（`adb pull` 能直接取）。按可用性依次尝试：
/// - `Android/media/<包名>/`：App 可自建，shell/文件管理器都能读（首选）；
/// - `Android/data/<包名>/files/`：部分系统要求由框架先创建，建不出来就跳过。
fn external_dir() -> Option<PathBuf> {
    for path in [
        format!("/storage/emulated/0/Android/media/{ANDROID_ID}/rec"),
        format!("/storage/emulated/0/Android/data/{ANDROID_ID}/files/rec"),
    ] {
        let d = PathBuf::from(path);
        if std::fs::create_dir_all(&d).is_ok() {
            return Some(d);
        }
    }
    None
}

/// 把一次录音写成 16k/16bit/单声道 WAV（内部 + 外部各一份）。
///
/// 返回回放用的路径（内部那份，nsh 授权可播）。拿不到可写目录（比如 web）
/// 时返回 `None`，判定照常进行。
pub fn dump(pcm: &[f32], target: &str, note: &str) -> Option<PathBuf> {
    let internal = internal_dir();
    let external = external_dir();
    if internal.is_none() && external.is_none() {
        return None;
    }
    if let Some(ext) = &external {
        let count = std::fs::read_dir(ext).map(|d| d.count()).unwrap_or(0);
        if count > MAX_FILES {
            return None;
        }
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let name = format!("{target}_{stamp}.wav");

    let mut played = None;
    if let Some(d) = &internal {
        let p = d.join(&name);
        if write_wav(&p, pcm, 16_000).is_ok() {
            played = Some(p);
        }
    }
    if let Some(d) = &external {
        let _ = write_wav(&d.join(&name), pcm, 16_000);
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(d.join("index.txt"))
        {
            let _ = writeln!(f, "{target}\t{note}\t{stamp}\t{}", pcm.len());
        }
    }
    played.or_else(|| external.map(|d| d.join(name)))
}

/// 记下人工判定（`对`/`错`），供事后标阈值、微调模型。
///
/// 追加到外部目录的 `labels.tsv`：`对错 \t 目标 \t 文件名 \t 机器判定`。
/// 外部目录不可写时退回到 WAV 所在目录。
pub fn label(wav: &Path, ok: bool, target: &str, machine: &str) {
    let Some(name) = wav.file_name() else {
        return;
    };
    let fname = name.to_string_lossy().to_string();
    let line = format!("{}\t{target}\t{fname}\t{machine}\n", if ok { "对" } else { "错" });
    let mut written = false;
    if let Some(d) = external_dir() {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(d.join("labels.tsv"))
        {
            written = f.write_all(line.as_bytes()).is_ok();
        }
    }
    if !written {
        if let Some(d) = wav.parent() {
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(d.join("labels.tsv"))
            {
                let _ = f.write_all(line.as_bytes());
            }
        }
    }
}

fn write_wav(path: &Path, pcm: &[f32], sr: u32) -> std::io::Result<()> {
    let data_len = pcm.len() * 2;
    let mut b = Vec::with_capacity(44 + data_len);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&1u16.to_le_bytes()); // 单声道
    b.extend_from_slice(&sr.to_le_bytes());
    b.extend_from_slice(&(sr * 2).to_le_bytes()); // 字节率
    b.extend_from_slice(&2u16.to_le_bytes()); // 块对齐
    b.extend_from_slice(&16u16.to_le_bytes()); // 位深
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(data_len as u32).to_le_bytes());
    for &v in pcm {
        let s = (v.clamp(-1.0, 1.0) * 32767.0) as i16;
        b.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_is_wellformed() {
        let dir = std::env::temp_dir().join("pinyin_pal_wav_test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.wav");
        write_wav(&p, &[0.0, 0.5, -0.5], 16_000).unwrap();
        let d = std::fs::read(&p).unwrap();
        assert_eq!(&d[0..4], b"RIFF");
        assert_eq!(&d[8..12], b"WAVE");
        assert_eq!(d.len(), 44 + 3 * 2);
        assert_eq!(u32::from_le_bytes([d[24], d[25], d[26], d[27]]), 16_000);
        assert_eq!(u16::from_le_bytes([d[34], d[35]]), 16);
        let _ = std::fs::remove_file(&p);
    }
}
