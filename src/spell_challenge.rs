//! 精确拼读闯关的纯逻辑：给一个目标音节，孩子选声母 + 韵母，
//! 用 `compose()` 做符号级精确判定——拼出来等于目标就过，否则重来。
//!
//! 设计要点（和旧的「跟读闯关」声学评分彻底分开）：
//! - 判定对象是**符号**（选了哪张卡），不是声音相似度，所以 100% 精确、零阈值；
//! - 干扰项从音近/形近分组里取，练的正是 b/p、z/zh、an/ang 这些易混点；
//! - 不依赖麦克风、不依赖 MFCC/GOP，完全离线且不受童声音色影响。
//!
//! 本模块不含 UI，便于单元测试；UI 在 `lib.rs::recognition_challenge` 接入。

use crate::data::pinyin_data::{FINALS, INITIALS};
use crate::pinyin::{ComposeResult, compose, split_syllable};

/// 一道拼读题：目标音节 + 正确拆分 + 声母/韵母候选（含干扰项）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpellQuestion {
    pub syllable: &'static str,
    pub initial: &'static str,
    pub final_: &'static str,
    pub initial_options: Vec<&'static str>,
    pub final_options: Vec<&'static str>,
}

/// 闯关目标音节：覆盖常用声母、韵母和全部易混对。
/// 非法的（split/compose 对不上，例如整体认读）在 `build_questions` 里自动跳过。
const TARGETS: &[&str] = &[
    "ba", "bo", "bi", "bu", "ban", "bang", "pa", "po", "pi", "pu", "pan", "pang", "ma", "mo",
    "mi", "fa", "fo", "fu", "da", "ta", "na", "la", "dan", "tang", "ge", "ke", "he", "gan",
    "kang", "gong", "ji", "qi", "xi", "jin", "jing", "zhi", "chi", "shi", "ri", "zhan", "zhang",
    "zi", "ci", "si", "zong", "zhong", "chong", "yi", "wu", "ya", "wa", "yan", "yang", "fen",
    "feng", "zhen", "zheng", "xin", "xing",
];

/// 易混声母分组（取同组做干扰项）。
const CONFUSE_INITIALS: &[&[&'static str]] = &[
    &["b", "p", "m", "f"],
    &["d", "t", "n", "l"],
    &["g", "k", "h"],
    &["j", "q", "x"],
    &["zh", "ch", "sh", "r"],
    &["z", "c", "s"],
    &["y", "w"],
];

/// 易混韵母分组，重点覆盖前后鼻音。
const CONFUSE_FINALS: &[&[&'static str]] = &[
    &["a", "o", "e"],
    &["i", "u", "ü"],
    &["an", "en", "in", "un", "ün"],
    &["ang", "eng", "ing", "ong"],
    &["an", "ang"],
    &["en", "eng"],
    &["in", "ing"],
];

/// 稳定的字符串散列，用来按音节确定性地打乱选项顺序（不引入 rand 依赖）。
fn order_seed(s: &str) -> usize {
    let mut h = 2166136261u32;
    for b in s.bytes() {
        h = h.wrapping_mul(16777619) ^ b as u32;
    }
    h as usize
}

/// 构造候选：正确答案 + 同组易混项 + 全量补齐，再按 seed 旋转，避免答案总在首位。
fn options(
    correct: &'static str,
    groups: &[&[&'static str]],
    pool: &[&'static str],
    count: usize,
    seed: usize,
) -> Vec<&'static str> {
    let mut out = vec![correct];
    'groups: for group in groups {
        if group.contains(&correct) {
            for &v in group.iter() {
                if v != correct && !out.contains(&v) {
                    out.push(v);
                    if out.len() >= count {
                        break 'groups;
                    }
                }
            }
        }
    }
    for &v in pool {
        if !out.contains(&v) {
            out.push(v);
            if out.len() >= count {
                break;
            }
        }
    }
    out.truncate(count);
    let k = seed % out.len();
    out.rotate_left(k);
    out
}

/// 生成全部合法题目。非法目标（整体认读、三拼）自动跳过。
pub fn build_questions() -> Vec<SpellQuestion> {
    TARGETS
        .iter()
        .filter_map(|&s| {
            let (initial, final_) = split_syllable(s)?;
            if !matches!(compose(initial, final_), ComposeResult::Valid(ref written) if written == s)
            {
                return None;
            }
            let seed = order_seed(s);
            Some(SpellQuestion {
                syllable: s,
                initial,
                final_,
                initial_options: options(initial, CONFUSE_INITIALS, INITIALS, 4, seed),
                final_options: options(final_, CONFUSE_FINALS, FINALS, 4, seed / 7 + 1),
            })
        })
        .collect()
}

/// 精确判定：把孩子选的声母+韵母拼出来，必须**等于**目标音节。
/// b 读成 w（选 w + a → "wa"）会被严格判错，不存在"差不多就过"。
pub fn judge(q: &SpellQuestion, initial: &str, final_: &str) -> bool {
    matches!(compose(initial, final_), ComposeResult::Valid(ref written) if written == q.syllable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn questions_are_all_valid() {
        let qs = build_questions();
        assert!(qs.len() >= 40, "expected enough questions, got {}", qs.len());
        for q in &qs {
            assert!(q.initial_options.contains(&q.initial), "{} initial missing", q.syllable);
            assert!(q.final_options.contains(&q.final_), "{} final missing", q.syllable);
            assert_eq!(q.initial_options.len(), 4, "{} initial options", q.syllable);
            assert_eq!(q.final_options.len(), 4, "{} final options", q.syllable);
        }
    }

    #[test]
    fn correct_combination_passes() {
        for q in build_questions() {
            assert!(judge(&q, q.initial, q.final_), "correct pair must pass: {}", q.syllable);
        }
    }

    #[test]
    fn wrong_combination_fails() {
        let qs = build_questions();
        let ba = qs.iter().find(|q| q.syllable == "ba").expect("ba question");
        assert!(!judge(ba, "p", "a"), "b→p must fail");
        assert!(!judge(ba, "w", "a"), "b→w must fail");
        assert!(!judge(ba, "b", "o"), "a→o must fail");
    }

    #[test]
    fn confusion_pairs_are_separated() {
        let qs = build_questions();
        let find = |s: &str| qs.iter().find(|q| q.syllable == s).cloned();
        if let (Some(ban), Some(bang)) = (find("ban"), find("bang")) {
            assert!(judge(&ban, "b", "an"));
            assert!(!judge(&ban, "b", "ang"), "an→ang must fail");
            assert!(judge(&bang, "b", "ang"));
            assert!(!judge(&bang, "b", "an"), "ang→an must fail");
        }
        if let (Some(zhi), Some(zi)) = (find("zhi"), find("zi")) {
            assert!(judge(&zhi, "zh", "i"));
            assert!(!judge(&zhi, "z", "i"), "zh→z must fail");
            assert!(judge(&zi, "z", "i"));
            assert!(!judge(&zi, "zh", "i"), "z→zh must fail");
        }
    }
}
