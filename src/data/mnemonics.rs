//! 声母顺口溜助记口诀（经典教学口诀 + 字形描述）。
//! 点选单个声母、尚未拼出音节时显示，帮助孩子记住字形和读音。

/// (声母, 口诀) 静态表。
pub static MNEMONICS: &[(&str, &str)] = &[
    ("b", "收听广播 b b b，右下半圆 b b b"),
    ("p", "右上半圆 p p p，端盆泼水 p p p"),
    ("m", "两个门洞 m m m"),
    ("f", "一根拐杖 f f f"),
    ("d", "左下半圆 d d d，马蹄印儿 d d d"),
    ("t", "伞柄朝下 t t t"),
    ("n", "一个门洞 n n n"),
    ("l", "一根小棍 l l l"),
    ("g", "9 字加弯 g g g，白鸽飞翔 g g g"),
    ("k", "蝌蚪戏水 k k k"),
    ("h", "一把椅子 h h h"),
    ("j", "竖弯加点 j j j，公鸡蝴蝶 j j j"),
    ("q", "左上半圆 q q q，七个气球 q q q"),
    ("x", "一个大叉 x x x，西瓜切片 x x x"),
    ("zh", "蜘蛛结网 zh zh zh"),
    ("ch", "小刺猬吃果 ch ch ch"),
    ("sh", "狮子舞蹈 sh sh sh"),
    ("r", "日出东方 r r r"),
    ("z", "像个 2 字 z z z"),
    ("c", "半个圆圈 c c c"),
    ("s", "半个 8 字 s s s"),
    ("y", "树杈 y y y"),
    ("w", "两个屋顶 w w w"),
];

/// 返回某个声母的顺口溜，未知声母返回空串。
pub fn mnemonic(initial: &str) -> &'static str {
    MNEMONICS
        .iter()
        .find(|(key, _)| *key == initial)
        .map(|(_, text)| *text)
        .unwrap_or("")
}

/// 样例项：(显示文本, 发音拼音, 声调)。拼音为空表示纯文字样例（无音频，不可点）。
pub type Example = (&'static str, &'static str, u8);

/// 拼音知识条目：(标题, 内容, 样例列表)。
pub static RULES: &[(&str, &str, &[Example])] = &[
    (
        "声调规则",
        "普通话有四个声调：一声平（－）、二声扬（／）、三声拐弯（∨）、四声降（＼）。标调口诀：有 a 别放过，没 a 找 o e，i u 并列标在后。",
        &[
            ("妈 mā", "ma", 1),
            ("麻 má", "ma", 2),
            ("马 mǎ", "ma", 3),
            ("骂 mà", "ma", 4),
        ],
    ),
    (
        "ü 的书写",
        "ü 遇到 j q x y 时，要去掉两点写成 u（ju、qu、xu、yu），但读音仍然是 ü；遇到 n l 时两点要保留（nü、lü）。",
        &[
            ("居 jū", "ju", 1),
            ("区 qū", "qu", 1),
            ("需 xū", "xu", 1),
            ("女 nǚ·保点", "nü", 3),
            ("绿 lǜ·保点", "lü", 4),
        ],
    ),
    (
        "整体认读音节",
        "zhi chi shi ri zi ci si yi wu yu ye yue yuan yin yun ying 这 16 个音节要整体记住，不用拼读。",
        &[
            ("知 zhi", "zhi", 1),
            ("吃 chi", "chi", 1),
            ("狮 shi", "shi", 1),
            ("一 yi", "yi", 1),
            ("屋 wu", "wu", 1),
            ("鱼 yú", "yu", 2),
            ("鹰 ying", "ying", 1),
        ],
    ),
    (
        "三拼音节",
        "由声母 + 介母（i/u/ü）+ 韵母组成，如 j-i-a → jia、g-u-a → gua。拼读时要读得又快又轻，连成一个音。",
        &[
            ("j-i-a → jia（家）", "", 0),
            ("g-u-a → gua（瓜）", "", 0),
            ("x-i-a → xia（虾）", "", 0),
        ],
    ),
    (
        "平翘舌音",
        "z c s 是平舌音，舌尖抵住上齿背；zh ch sh r 是翘舌音，舌尖翘起抵住硬腭。南方小朋友要特别多练。",
        &[
            ("平·字 zi", "zi", 4),
            ("翘·治 zhi", "zhi", 4),
            ("平·次 ci", "ci", 4),
            ("翘·翅 chi", "chi", 4),
            ("平·四 si", "si", 4),
            ("翘·是 shi", "shi", 4),
        ],
    ),
    (
        "前后鼻音",
        "an en in un ün 是前鼻音，声音收在前面；ang eng ing ong 是后鼻音，声音要往后拖。",
        &[
            ("前·安 an", "an", 1),
            ("后·昂 ang", "ang", 2),
            ("前·因 in", "in", 1),
            ("后·英 ing", "ing", 1),
            ("前·恩 en", "en", 1),
            ("后·eng", "eng", 1),
        ],
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_initial_has_a_mnemonic() {
        for &s in crate::data::pinyin_data::INITIALS {
            assert!(!mnemonic(s).is_empty(), "missing mnemonic for {s}");
        }
    }

    #[test]
    fn unknown_returns_empty() {
        assert_eq!(mnemonic("xyz"), "");
    }

    #[test]
    fn every_rule_has_valid_examples() {
        for &(title, body, examples) in RULES {
            assert!(!title.is_empty() && !body.is_empty());
            assert!(!examples.is_empty(), "rule {title} has no examples");
            for &(label, py, t) in examples {
                assert!(!label.is_empty(), "empty example label in {title}");
                if py.is_empty() {
                    assert_eq!(t, 0, "text-only example must use tone 0 in {title}");
                } else {
                    assert!((1..=4).contains(&t), "bad tone {t} in {title}");
                }
            }
        }
    }
}
