//! 跟读闯关的 47 个字母 → 呼读音音节（`spell_net` 词表的 key 前缀）。
//!
//! 与 `LETTERS` 一一对应、顺序一致：声母读呼读音（b 读 bo），单韵母/复韵母按
//! 独立成音节时的写法（i 读 yi、un 读 wen、ong 读 weng …）。
//! `spell_net` 的词表是 `音节+声调`，判定时按音节聚合、不看声调。
//!
//! `yu`（ü）不在词表里（语料缺这个音节），端上会退回旧的模板比对。

/// 与 `data::letter_templates::LETTERS` 同序。
pub const LETTER_SPELL_KEYS: [&str; 47] = [
    "bo",  // b
    "po",  // p
    "mo",  // m
    "fo",  // f
    "de",  // d
    "te",  // t
    "ne",  // n
    "le",  // l
    "ge",  // g
    "ke",  // k
    "he",  // h
    "ji",  // j
    "qi",  // q
    "xi",  // x
    "zhi", // zh
    "chi", // ch
    "shi", // sh
    "ri",  // r
    "zi",  // z
    "ci",  // c
    "si",  // s
    "yi",  // y
    "wu",  // w
    "a",   // a
    "o",   // o
    "e",   // e
    "yi",  // i
    "wu",  // u
    "yu",  // ü —— 词表未覆盖，走旧路径
    "ai",  // ai
    "ei",  // ei
    "ui",  // ui
    "ao",  // ao
    "ou",  // ou
    "iu",  // iu
    "ie",  // ie
    "ve",  // üe
    "er",  // er
    "an",  // an
    "en",  // en
    "yin", // in
    "wen", // un
    "yun", // ün
    "ang", // ang
    "eng", // eng
    "ying", // ing
    "weng", // ong
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::letter_templates::LETTERS;

    #[test]
    fn table_is_aligned_with_letters() {
        assert_eq!(LETTER_SPELL_KEYS.len(), LETTERS.len(), "映射表与字母表长度不一致");
    }

    /// 词表里有的音节必须至少有 1 个声调变体，否则端上会静默退回旧路径。
    #[test]
    fn keys_exist_in_spell_vocab() {
        if crate::spell_net::vocab().is_empty() {
            return; // 权重/词表未生成时跳过
        }
        let missing: Vec<&str> = LETTER_SPELL_KEYS
            .iter()
            .copied()
            .filter(|k| crate::spell_net::syllable_index_of(k).is_none())
            .collect();
        assert_eq!(missing, vec!["yu"], "未覆盖的音节应只有 yu，实际 {missing:?}");
    }
}
