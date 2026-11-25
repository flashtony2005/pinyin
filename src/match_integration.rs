//! 跟读判定的独立样本集成测试。仅在 cargo test 下编译。
//!
//! ⚠️ 旧版测试用 tools/_asr/*.wav 当探针，但 build_letter_templates.py 恰好
//! 也把 _asr/ 收进模板池——自己评自己，虚报「误放行 0」。本文件改用
//! tools/probes_indep/（没进模板池的同字母发音），才是真机泛化的诚实度量。

use crate::data::letter_templates::LETTERS;
use crate::letter_match::{match_letter, Verdict};

/// 独立样本：每个字母一个没进模板池的同字母发音，必须判 Pass。
/// 这是用户「过不了」的核心回归——旧竞争门下 26/28 被拒。
#[test]
fn independent_samples_pass() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/probes_indep");
    let entries: Vec<_> = std::fs::read_dir(&dir)
        .expect("probes_indep dir exists (run tools/build_indep_probes.py)")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "f32"))
        .collect();

    let mut passed = 0usize;
    let mut failed = Vec::new();
    for (idx, letter) in LETTERS.iter().enumerate() {
        let prefix = format!("{:02}_", idx);
        let Some(e) = entries.iter().find(|e| {
            e.file_name().to_string_lossy().starts_with(&prefix)
        }) else {
            continue;
        };
        let bytes = std::fs::read(e.path()).expect("read probe");
        let pcm: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        match match_letter(&pcm, 16_000, idx) {
            Verdict::Pass { .. } => passed += 1,
            other => failed.push(format!("{letter}: {other:?}")),
        }
    }
    println!("\n[integration] 独立样本通过 {passed}/{}", LETTERS.len());
    if !failed.is_empty() {
        println!("[integration] 未通过: {}", failed.join(", "));
    }
    // 允许个别疑难音（ke/o）因素材问题不过，但绝大多数必须能过。
    assert!(
        passed >= 24,
        "independent pass {passed} too low (competition gate regression?): {}",
        failed.join(", ")
    );
}

/// 乱哼（多个频率）评所有字母，必须全部 Retry——「随便发个音都过」回归。
#[test]
fn arbitrary_hum_rejected_for_all_letters() {
    let mut rejected = 0usize;
    let mut total = 0usize;
    for freq in [180f32, 220.0, 260.0, 330.0, 440.0] {
        let n = 16_000; // 1 秒
        let hum: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / 16_000.0).sin() * 0.5)
            .collect();
        for idx in 0..LETTERS.len() {
            total += 1;
            if matches!(match_letter(&hum, 16_000, idx), Verdict::Retry { .. }) {
                rejected += 1;
            }
        }
    }
    println!("[integration] 乱哼拒绝 {rejected}/{total}");
    assert_eq!(rejected, total, "arbitrary hum must be rejected everywhere");
}
