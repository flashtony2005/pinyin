//! Pure pinyin composition logic. No UI framework dependencies.

use crate::data::pinyin_data::{FINALS, INITIALS, MATRIX, SYLLABLES};

/// Result of combining an initial and a final.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposeResult {
    /// A legal syllable, e.g. `b` + `a` -> `ba`.
    Valid(String),
    /// The pair is not a Mandarin syllable, e.g. `b` + `ong`.
    Invalid,
}

/// Spell an initial and final into written form, applying ü rules for j/q/x/y.
fn spell(initial: &str, final_: &str) -> String {
    if matches!(initial, "j" | "q" | "x" | "y") && matches!(final_, "ü" | "ün" | "üe") {
        // In writing, j/q/x/y + ü is spelled with u (ju, jun, jue).
        let mut s = String::with_capacity(initial.len() + final_.len());
        s.push_str(initial);
        s.push_str(&final_.replace('ü', "u"));
        return s;
    }
    format!("{initial}{final_}")
}

/// Combine an initial and final. Returns `Invalid` unless the pair is legal.
pub fn compose(initial: &str, final_: &str) -> ComposeResult {
    // y/w initials form syllables by simple concatenation like other initials.
    let legal = MATRIX
        .iter()
        .find(|(i, _)| *i == initial)
        .map(|(_, finals)| finals.iter().any(|f| *f == final_))
        .unwrap_or(false);

    if legal {
        ComposeResult::Valid(spell(initial, final_))
    } else {
        ComposeResult::Invalid
    }
}

/// Finals that can legally follow the given initial.
pub fn finals_for(initial: &str) -> &'static [&'static str] {
    MATRIX
        .iter()
        .find(|(i, _)| *i == initial)
        .map(|(_, f)| *f)
        .unwrap_or(&[])
}

/// Whether a written syllable exists in the standard table.
pub fn is_known_syllable(s: &str) -> bool {
    SYLLABLES.iter().any(|x| *x == s)
}

pub fn all_initials() -> &'static [&'static str] {
    INITIALS
}

pub fn all_finals() -> &'static [&'static str] {
    FINALS
}

/// 把书写形式的音节拆成声母和韵母。
/// 处理 j/q/x/y + ü 写成 u 的情况（ju -> j + ü），也处理 zh/ch/sh 双字母声母。
pub fn split_syllable(s: &str) -> Option<(&'static str, &'static str)> {
    let mut initials: Vec<&str> = INITIALS.iter().copied().collect();
    initials.sort_by_key(|i| std::cmp::Reverse(i.len()));

    for initial in initials {
        if !s.starts_with(initial) {
            continue;
        }
        let rest = &s[initial.len()..];
        if rest.is_empty() {
            continue;
        }
        // j/q/x/y 后的 u 实际表示 ü，先尝试还原，再按字面找韵母。
        if matches!(initial, "j" | "q" | "x" | "y") && rest.starts_with('u') {
            let umlaut = format!("ü{}", &rest[1..]);
            if let Some(&final_) = FINALS.iter().find(|f| **f == umlaut) {
                return Some((initial, final_));
            }
        }
        if let Some(&final_) = FINALS.iter().find(|f| **f == rest) {
            return Some((initial, final_));
        }
    }
    None
}

/// 三拼音节表：(声母, 介母, 后韵)。书写音节由字面拼接得到（ü 遇 j/q/x 去点）。
///
/// 后韵记号说明：`i` 表示 ui（g+u+i→gui），`n` 表示 un（g+u+n→gun），
/// 这样字面拼接即为标准书写，无需特例。介母只有 i / u / ü 三个。
pub static TRIPLETS: &[(&str, &str, &str)] = &[
    // i 介母（j/q/x 系）
    ("j", "i", "a"), ("j", "i", "ao"), ("j", "i", "an"), ("j", "i", "ang"), ("j", "i", "ong"),
    ("q", "i", "a"), ("q", "i", "ao"), ("q", "i", "an"), ("q", "i", "ang"), ("q", "i", "ong"),
    ("x", "i", "a"), ("x", "i", "ao"), ("x", "i", "an"), ("x", "i", "ang"), ("x", "i", "ong"),
    // ü 介母（j/q/x，书写去两点）
    ("j", "ü", "an"), ("q", "ü", "an"), ("x", "ü", "an"),
    // u 介母 — g / k / h
    ("g", "u", "a"), ("g", "u", "o"), ("g", "u", "ai"), ("g", "u", "an"), ("g", "u", "ang"),
    ("g", "u", "i"), ("g", "u", "n"),
    ("k", "u", "a"), ("k", "u", "o"), ("k", "u", "ai"), ("k", "u", "an"), ("k", "u", "ang"),
    ("k", "u", "i"), ("k", "u", "n"),
    ("h", "u", "a"), ("h", "u", "o"), ("h", "u", "ai"), ("h", "u", "an"), ("h", "u", "ang"),
    ("h", "u", "i"), ("h", "u", "n"),
    // u 介母 — zh / ch / sh / r
    ("zh", "u", "a"), ("zh", "u", "o"), ("zh", "u", "ai"), ("zh", "u", "an"), ("zh", "u", "ang"),
    ("zh", "u", "i"), ("zh", "u", "n"),
    ("ch", "u", "a"), ("ch", "u", "o"), ("ch", "u", "ai"), ("ch", "u", "an"), ("ch", "u", "ang"),
    ("ch", "u", "i"), ("ch", "u", "n"),
    ("sh", "u", "a"), ("sh", "u", "o"), ("sh", "u", "ai"), ("sh", "u", "an"), ("sh", "u", "ang"),
    ("sh", "u", "i"), ("sh", "u", "n"),
    ("r", "u", "o"), ("r", "u", "an"), ("r", "u", "i"), ("r", "u", "n"),
    // u 介母 — d / t / n / l
    ("d", "u", "o"), ("d", "u", "an"), ("d", "u", "i"), ("d", "u", "n"),
    ("t", "u", "o"), ("t", "u", "an"), ("t", "u", "i"), ("t", "u", "n"),
    ("n", "u", "o"), ("n", "u", "an"),
    ("l", "u", "o"), ("l", "u", "an"), ("l", "u", "n"),
];

/// 三拼书写：字面拼接，ü 遇 j/q/x 去点（j + ü + an → juan）。
fn spell_three(initial: &str, medial: &str, final_: &str) -> String {
    let m = if matches!(initial, "j" | "q" | "x") && medial == "ü" {
        "u"
    } else {
        medial
    };
    format!("{initial}{m}{final_}")
}

/// 三拼：声母 + 介母 + 后韵。非法组合返回 `Invalid`。
pub fn compose_three(initial: &str, medial: &str, final_: &str) -> ComposeResult {
    if TRIPLETS
        .iter()
        .any(|&(i, m, f)| i == initial && m == medial && f == final_)
    {
        ComposeResult::Valid(spell_three(initial, medial, final_))
    } else {
        ComposeResult::Invalid
    }
}

/// 给定声母 + 介母，列出可拼的后韵（供 UI 过滤韵母卡）。
pub fn finals_for_three(initial: &str, medial: &str) -> Vec<&'static str> {
    TRIPLETS
        .iter()
        .filter(|&&(i, m, _)| i == initial && m == medial)
        .map(|&(_, _, f)| f)
        .collect()
}

/// 给定声母，列出它支持的三拼介母（供 UI 亮起可用介母卡，顺序 i/u/ü）。
pub fn medials_for(initial: &str) -> Vec<&'static str> {
    let mut out = Vec::new();
    for m in ["i", "u", "ü"] {
        if TRIPLETS.iter().any(|&(i, med, _)| i == initial && med == m) {
            out.push(m);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_are_standard() {
        assert_eq!(INITIALS.len(), 23);
        assert_eq!(FINALS.len(), 24);
    }

    #[test]
    fn basic_composition() {
        assert_eq!(compose("b", "a"), ComposeResult::Valid("ba".into()));
        assert_eq!(compose("zh", "ong"), ComposeResult::Valid("zhong".into()));
        assert_eq!(compose("x", "iang"), ComposeResult::Invalid); // 三拼, not in two-part table
    }

    #[test]
    fn illegal_pair_is_rejected() {
        assert_eq!(compose("b", "ong"), ComposeResult::Invalid);
        assert_eq!(compose("f", "ü"), ComposeResult::Invalid);
        assert_eq!(compose("q", "a"), ComposeResult::Invalid);
    }

    #[test]
    fn u_umlaut_spelling_rule() {
        assert_eq!(compose("j", "ü"), ComposeResult::Valid("ju".into()));
        assert_eq!(compose("q", "üe"), ComposeResult::Valid("que".into()));
        assert_eq!(compose("x", "ün"), ComposeResult::Valid("xun".into()));
        assert_eq!(compose("n", "ü"), ComposeResult::Valid("nü".into()));
        assert_eq!(compose("l", "üe"), ComposeResult::Valid("lüe".into()));
    }

    #[test]
    fn y_w_initials() {
        assert_eq!(compose("y", "u"), ComposeResult::Valid("yu".into()));
        assert_eq!(compose("w", "o"), ComposeResult::Valid("wo".into()));
    }

    #[test]
    fn finals_for_initial() {
        assert!(!finals_for("b").contains(&"ong"));
        assert!(finals_for("b").contains(&"a"));
        assert!(finals_for("not-an-initial").is_empty());
    }

    #[test]
    fn splits_syllables_into_initial_and_final() {
        assert_eq!(split_syllable("ba"), Some(("b", "a")));
        assert_eq!(split_syllable("zhong"), Some(("zh", "ong")));
        assert_eq!(split_syllable("shi"), Some(("sh", "i")));
        assert_eq!(split_syllable("ju"), Some(("j", "ü")));
        assert_eq!(split_syllable("que"), Some(("q", "üe")));
        assert_eq!(split_syllable("yu"), Some(("y", "ü")));
        assert_eq!(split_syllable("nü"), Some(("n", "ü")));
        assert_eq!(split_syllable("a"), None);
    }

    #[test]
    fn three_part_composition() {
        assert_eq!(compose_three("j", "i", "a"), ComposeResult::Valid("jia".into()));
        assert_eq!(compose_three("g", "u", "o"), ComposeResult::Valid("guo".into()));
        assert_eq!(compose_three("zh", "u", "ang"), ComposeResult::Valid("zhuang".into()));
        assert_eq!(compose_three("x", "i", "ao"), ComposeResult::Valid("xiao".into()));
    }

    #[test]
    fn three_part_umlaut_drops_dots() {
        assert_eq!(compose_three("j", "ü", "an"), ComposeResult::Valid("juan".into()));
        assert_eq!(compose_three("q", "ü", "an"), ComposeResult::Valid("quan".into()));
    }

    #[test]
    fn three_part_ui_un_notation() {
        // 后韵 i/n 表示 ui/un，字面拼接即标准书写。
        assert_eq!(compose_three("g", "u", "i"), ComposeResult::Valid("gui".into()));
        assert_eq!(compose_three("zh", "u", "n"), ComposeResult::Valid("zhun".into()));
    }

    #[test]
    fn three_part_illegal() {
        assert_eq!(compose_three("b", "i", "a"), ComposeResult::Invalid);
        assert_eq!(compose_three("j", "u", "a"), ComposeResult::Invalid);
        assert!(finals_for_three("j", "i").contains(&"a"));
        assert!(finals_for_three("g", "u").contains(&"o"));
    }

    #[test]
    fn every_legal_pair_is_in_syllable_table() {
        let mut checked = 0;
        for (initial, finals) in MATRIX.iter() {
            for final_ in finals.iter() {
                let s = spell(initial, final_);
                assert!(
                    is_known_syllable(&s),
                    "composed syllable {} missing from table",
                    s
                );
                checked += 1;
            }
        }
        assert_eq!(checked, 320);
    }
}
