//! 多题型练习逻辑：听音选字、看字选拼音、听音选声母/韵母、听音选声调。
//! 音近/形近干扰项按声母、韵母和字形分组选取；错题库可序列化为字符串持久化。

use std::collections::HashSet;

use crate::data::pinyin_data::{FINALS, INITIALS, SYLLABLES};
use crate::pinyin::split_syllable;

const TONE_OPTIONS: [&str; 4] = ["一声", "二声", "三声", "四声"];

/// 易混声母：同一组内互为音近干扰项。
const SIMILAR_INITIALS: &[&[&str]] = &[
    &["b", "p", "m", "f"],
    &["d", "t", "n", "l"],
    &["g", "k", "h"],
    &["j", "q", "x"],
    &["zh", "ch", "sh", "r"],
    &["z", "c", "s"],
    &["y", "w"],
];

/// 字形相近的声母：b/d/p/q、m/n/h、f/t 等。
const INITIAL_SHAPES: &[&[&str]] = &[
    &["b", "d", "p", "q"],
    &["m", "n", "h"],
    &["f", "t"],
    &["z", "zh"],
    &["c", "ch"],
    &["s", "sh"],
];

/// 易混韵母：前后鼻音、i/u/ü 等。
const SIMILAR_FINALS: &[&[&str]] = &[
    &["a", "o", "e"],
    &["i", "u", "ü"],
    &["ai", "ei", "ui"],
    &["ao", "ou", "iu"],
    &["ie", "üe", "er"],
    &["an", "en", "in", "un", "ün"],
    &["ang", "eng", "ing", "ong"],
];

/// 字形相近的韵母，主要补前后鼻音和 u/ü。
const FINAL_SHAPES: &[&[&str]] = &[
    &["u", "ü"],
    &["an", "ang"],
    &["en", "eng"],
    &["in", "ing"],
    &["ai", "ao"],
    &["ei", "ou"],
];

/// 现有例字里可稳定找到的形近字组。
const SIMILAR_CHARS: &[&[&str]] = &[&["日", "曰"], &["刀", "力"], &["杯", "怀"]];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QuestionKind {
    ListenChar,
    ReadChar,
    ListenInitial,
    ListenFinal,
    ListenTone,
}

impl QuestionKind {
    fn code(self) -> u8 {
        match self {
            QuestionKind::ListenChar => 0,
            QuestionKind::ReadChar => 1,
            QuestionKind::ListenInitial => 2,
            QuestionKind::ListenFinal => 3,
            QuestionKind::ListenTone => 4,
        }
    }

    fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(QuestionKind::ListenChar),
            1 => Some(QuestionKind::ReadChar),
            2 => Some(QuestionKind::ListenInitial),
            3 => Some(QuestionKind::ListenFinal),
            4 => Some(QuestionKind::ListenTone),
            _ => None,
        }
    }

    pub fn prompt(self) -> &'static str {
        match self {
            QuestionKind::ListenChar => "听发音，选出正确的字",
            QuestionKind::ReadChar => "看汉字，选出正确拼音",
            QuestionKind::ListenInitial => "听发音，选出声母",
            QuestionKind::ListenFinal => "听发音，选出韵母",
            QuestionKind::ListenTone => "听发音，选出声调",
        }
    }

    pub fn choose_hint(self) -> &'static str {
        match self {
            QuestionKind::ListenChar => "选一个字",
            QuestionKind::ReadChar => "选一个拼音",
            QuestionKind::ListenInitial => "选一个声母",
            QuestionKind::ListenFinal => "选一个韵母",
            QuestionKind::ListenTone => "选一个声调",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Question {
    pub kind: QuestionKind,
    pub syllable: String,
    pub audio_tone: u8,
    pub options: [String; 4],
    pub answer_idx: usize,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct WrongEntry {
    pub syllable: String,
    pub kind: QuestionKind,
    pub count: u32,
}

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct WrongBook {
    pub entries: Vec<WrongEntry>,
}

impl WrongBook {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 答错累加一次；答对减一次，减到 0 表示掌握，移出错题本。
    pub fn record(&mut self, question: &Question, correct: bool) {
        let key = (question.kind, question.syllable.as_str());
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|e| e.kind == key.0 && e.syllable.as_str() == key.1)
        {
            if correct {
                entry.count = entry.count.saturating_sub(1);
            } else {
                entry.count = entry.count.saturating_add(1).min(9);
            }
        } else if !correct {
            self.entries.push(WrongEntry {
                syllable: question.syllable.clone(),
                kind: question.kind,
                count: 1,
            });
        }
        if correct {
            self.entries.retain(|e| e.count > 0);
        }
    }

    /// 优先复习错得最多的题，同权重内用种子做稳定随机。
    pub fn pick(&self, seed: &mut u64) -> Option<&WrongEntry> {
        let max = self.entries.iter().map(|e| e.count).max()?;
        let top: Vec<&WrongEntry> = self.entries.iter().filter(|e| e.count == max).collect();
        let idx = (next_rand(seed) as usize) % top.len();
        Some(top[idx])
    }

    /// 格式：kind,syllable,count;... 音节不含逗号或分号，直接拼接即可。
    pub fn encode(&self) -> String {
        self.entries
            .iter()
            .map(|e| format!("{},{},{}", e.kind.code(), e.syllable, e.count))
            .collect::<Vec<_>>()
            .join(";")
    }

    pub fn decode(text: &str) -> Self {
        let mut entries = Vec::new();
        for part in text.split(';').filter(|p| !p.is_empty()) {
            let mut fields = part.split(',');
            let Some(kind_code) = fields.next().and_then(|v| v.parse::<u8>().ok()) else {
                continue;
            };
            let Some(kind) = QuestionKind::from_code(kind_code) else {
                continue;
            };
            let Some(syllable) = fields.next() else {
                continue;
            };
            let Some(count) = fields.next().and_then(|v| v.parse::<u32>().ok()) else {
                continue;
            };
            if syllable.is_empty() || count == 0 {
                continue;
            }
            entries.push(WrongEntry {
                syllable: syllable.to_owned(),
                kind,
                count: count.min(9),
            });
        }
        Self { entries }
    }
}

/// xorshift64 伪随机，避免引入 rand 依赖。种子不能为 0。
pub fn next_rand(seed: &mut u64) -> u64 {
    let mut x = *seed;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *seed = x;
    x
}

fn shuffle<T>(items: &mut [T], seed: &mut u64) {
    if items.len() < 2 {
        return;
    }
    for i in (1..items.len()).rev() {
        let j = (next_rand(seed) as usize) % (i + 1);
        items.swap(i, j);
    }
}

#[cfg(test)]
fn group_has(groups: &[&[&str]], value: &str) -> bool {
    groups.iter().any(|group| group.contains(&value))
}

fn group_members(groups: &[&[&str]], value: &str) -> Vec<String> {
    let mut out = Vec::new();
    for group in groups {
        if group.contains(&value) {
            for member in group.iter() {
                if *member != value {
                    out.push((*member).to_owned());
                }
            }
        }
    }
    out
}

fn first_group_value(groups: &[&[&str]], value: &str, seed: &mut u64) -> Option<String> {
    let members = group_members(groups, value);
    if members.is_empty() {
        return None;
    }
    let idx = (next_rand(seed) as usize) % members.len();
    Some(members[idx].clone())
}

/// 从音近/形近分组优先取干扰项，不足时用全量候选补齐。
fn pick_distractors(
    groups: &[&[&str]],
    answer: &str,
    fallback: &[String],
    seed: &mut u64,
) -> Vec<String> {
    let mut pool = group_members(groups, answer);
    pool.extend(fallback.iter().cloned());
    let mut seen: HashSet<String> = HashSet::new();
    seen.insert(answer.to_owned());
    let mut out = Vec::new();
    shuffle(&mut pool, seed);
    for item in pool {
        if seen.insert(item.clone()) {
            out.push(item);
            if out.len() == 3 {
                break;
            }
        }
    }
    out
}

fn valid_examples<'a>(pool: &'a [(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    pool.iter()
        .copied()
        .filter(|(syllable, _)| SYLLABLES.iter().any(|s| s == syllable))
        .collect()
}

fn example_char<'a>(pool: &'a [(&'a str, &'a str)], syllable: &str) -> Option<&'a str> {
    pool.iter().find(|(s, _)| *s == syllable).map(|(_, c)| *c)
}

fn sound_syllables(syllable: &str, pool: &[(&str, &str)], seed: &mut u64) -> Vec<String> {
    let Some((initial, final_)) = split_syllable(syllable) else {
        return pool.iter().map(|(s, _)| (*s).to_owned()).collect();
    };
    let mut out = Vec::new();
    if let Some(peer) = first_group_value(SIMILAR_INITIALS, initial, seed) {
        for (s, _) in pool {
            if let Some((i, _)) = split_syllable(s) {
                if i == peer && *s != syllable {
                    out.push((*s).to_owned());
                }
            }
        }
    }
    if let Some(peer) = first_group_value(SIMILAR_FINALS, final_, seed) {
        for (s, _) in pool {
            if let Some((_, f)) = split_syllable(s) {
                if f == peer && *s != syllable {
                    out.push((*s).to_owned());
                }
            }
        }
    }
    out.extend(
        pool.iter()
            .map(|(s, _)| (*s).to_owned())
            .filter(|s| s != syllable),
    );
    out
}

fn dedup_take(items: Vec<String>, answer: &str, count: usize) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    seen.insert(answer.to_owned());
    let mut out = Vec::new();
    for item in items {
        if seen.insert(item.clone()) {
            out.push(item);
            if out.len() == count {
                break;
            }
        }
    }
    out
}

fn tone_options(seed: &mut u64, tone: usize) -> Option<([String; 4], usize)> {
    let mut options: Vec<String> = TONE_OPTIONS.iter().map(|s| (*s).to_owned()).collect();
    shuffle(&mut options, seed);
    let answer = TONE_OPTIONS[tone - 1];
    let answer_idx = options.iter().position(|o| o == answer)?;
    let mut iter = options.into_iter();
    Some((
        [iter.next()?, iter.next()?, iter.next()?, iter.next()?],
        answer_idx,
    ))
}

fn make_options(
    seed: &mut u64,
    kind: QuestionKind,
    syllable: &str,
    examples: &[(&str, &str)],
) -> Option<([String; 4], usize)> {
    let char_ = example_char(examples, syllable)?;
    let answer = match kind {
        QuestionKind::ListenChar => char_.to_owned(),
        QuestionKind::ReadChar => syllable.to_owned(),
        QuestionKind::ListenInitial | QuestionKind::ListenFinal => {
            let (initial, final_) = split_syllable(syllable)?;
            if kind == QuestionKind::ListenInitial {
                initial.to_owned()
            } else {
                final_.to_owned()
            }
        }
        QuestionKind::ListenTone => {
            let tone = 1 + (next_rand(seed) as usize % 4);
            return tone_options(seed, tone);
        }
    };

    let pool_chars: Vec<String> = examples.iter().map(|(_, c)| (*c).to_owned()).collect();
    let pool_syllables: Vec<String> = examples.iter().map(|(s, _)| (*s).to_owned()).collect();
    let initials: Vec<String> = INITIALS.iter().map(|s| (*s).to_owned()).collect();
    let finals: Vec<String> = FINALS.iter().map(|s| (*s).to_owned()).collect();

    let mut options = vec![answer.clone()];
    let distractors = match kind {
        QuestionKind::ListenChar => {
            let mut groups: Vec<&[&str]> = Vec::new();
            for group in SIMILAR_CHARS {
                if group.contains(&answer.as_str()) {
                    groups.push(*group);
                }
            }
            let mut out = pick_distractors(&groups, &answer, &pool_chars, seed);
            if out.len() < 3 {
                let related = sound_syllables(syllable, examples, seed);
                for s in related {
                    if let Some(c) = example_char(examples, &s) {
                        if c != answer && !out.iter().any(|x| x == c) {
                            out.push(c.to_owned());
                            if out.len() == 3 {
                                break;
                            }
                        }
                    }
                }
            }
            out
        }
        QuestionKind::ReadChar => {
            let related = sound_syllables(syllable, examples, seed);
            let mut out: Vec<String> = related.into_iter().filter(|s| s != &answer).collect();
            out.extend(pool_syllables.iter().cloned().filter(|s| s != &answer));
            dedup_take(out, &answer, 3)
        }
        QuestionKind::ListenInitial => {
            let mut groups: Vec<&[&str]> = Vec::new();
            for group in SIMILAR_INITIALS.iter().chain(INITIAL_SHAPES.iter()) {
                if group.contains(&answer.as_str()) {
                    groups.push(group);
                }
            }
            pick_distractors(&groups, &answer, &initials, seed)
        }
        QuestionKind::ListenFinal => {
            let mut groups: Vec<&[&str]> = Vec::new();
            for group in SIMILAR_FINALS.iter().chain(FINAL_SHAPES.iter()) {
                if group.contains(&answer.as_str()) {
                    groups.push(group);
                }
            }
            pick_distractors(&groups, &answer, &finals, seed)
        }
        QuestionKind::ListenTone => {
            unreachable!("ListenTone 的选项在 match 的独立分支里构造，不会落到通用干扰项选取")
        }
    };

    options.extend(distractors);
    if options.len() < 4 {
        return None;
    }
    options.truncate(4);
    shuffle(&mut options, seed);
    let answer_idx = options.iter().position(|o| o == &answer)?;
    let mut iter = options.into_iter();
    Some((
        [iter.next()?, iter.next()?, iter.next()?, iter.next()?],
        answer_idx,
    ))
}

fn make_question(
    seed: &mut u64,
    kind: QuestionKind,
    syllable: &str,
    examples: &[(&str, &str)],
) -> Option<Question> {
    let (options, answer_idx) = make_options(seed, kind, syllable, examples)?;
    let audio_tone = if kind == QuestionKind::ListenTone {
        TONE_OPTIONS
            .iter()
            .position(|label| *label == options[answer_idx])
            .map(|i| (i + 1) as u8)?
    } else {
        1
    };
    Some(Question {
        kind,
        syllable: syllable.to_owned(),
        audio_tone,
        options,
        answer_idx,
    })
}

/// 生成一道题。wrong_only=true 时优先从错题库挑，错题本为空则回到普通练习。
pub fn new_question(
    seed: &mut u64,
    examples: &[(&str, &str)],
    tone_examples: &[(&str, &str)],
    wrong_only: bool,
    wrong: &WrongBook,
) -> Question {
    if wrong_only {
        if let Some(entry) = wrong.pick(seed) {
            let pool = if entry.kind == QuestionKind::ListenTone {
                tone_examples
            } else {
                examples
            };
            if let Some(q) = make_question(seed, entry.kind, &entry.syllable, pool) {
                return q;
            }
        }
    }

    let mut kinds = vec![
        QuestionKind::ListenChar,
        QuestionKind::ReadChar,
        QuestionKind::ListenInitial,
        QuestionKind::ListenFinal,
    ];
    if !tone_examples.is_empty() {
        kinds.push(QuestionKind::ListenTone);
    }

    let valid = valid_examples(examples);
    for _ in 0..500 {
        let kind = kinds[(next_rand(seed) as usize) % kinds.len()];
        let pool = if kind == QuestionKind::ListenTone {
            tone_examples
        } else {
            &valid
        };
        if pool.is_empty() {
            continue;
        }
        let pos = (next_rand(seed) as usize) % pool.len();
        let syllable = pool[pos].0;
        if kind != QuestionKind::ListenTone && split_syllable(syllable).is_none() {
            continue;
        }
        if let Some(q) = make_question(seed, kind, syllable, pool) {
            return q;
        }
    }

    let syllable = valid.first().map(|(s, _)| *s).unwrap_or("ba");
    make_question(seed, QuestionKind::ListenChar, syllable, &valid).unwrap_or_else(|| Question {
        kind: QuestionKind::ListenChar,
        syllable: "ba".to_owned(),
        audio_tone: 1,
        options: ["芭", "白", "班", "邦"].map(str::to_owned),
        answer_idx: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool() -> Vec<(&'static str, &'static str)> {
        vec![
            ("ba", "芭"),
            ("bai", "白"),
            ("ban", "斑"),
            ("bang", "邦"),
            ("ma", "妈"),
            ("mai", "埋"),
            ("man", "瞒"),
            ("mang", "芒"),
            ("da", "搭"),
            ("dai", "呆"),
            ("dan", "耽"),
            ("dang", "当"),
            ("zhi", "芝"),
            ("chi", "吃"),
            ("shi", "匙"),
            ("ri", "日"),
            ("yue", "曰"),
            ("dao", "刀"),
            ("li", "力"),
        ]
    }

    fn tones() -> Vec<(&'static str, &'static str)> {
        vec![("ba", "芭"), ("ma", "妈"), ("yu", "迂")]
    }

    #[test]
    fn options_are_four_distinct_with_one_answer() {
        let mut seed = 12345u64;
        let p = pool();
        let t = tones();
        for _ in 0..300 {
            let q = new_question(&mut seed, &p, &t, false, &WrongBook::default());
            let distinct: HashSet<&String> = q.options.iter().collect();
            assert_eq!(distinct.len(), 4, "options must be distinct");
            assert!(q.answer_idx < 4);
            assert!(!q.options[q.answer_idx].is_empty());
        }
    }

    #[test]
    fn initial_question_prefers_similar_initials() {
        let mut seed = 77u64;
        let p = pool();
        let t = tones();
        let mut found = false;
        for _ in 0..200 {
            let q = new_question(&mut seed, &p, &t, false, &WrongBook::default());
            if q.kind != QuestionKind::ListenInitial {
                continue;
            }
            let answer = &q.options[q.answer_idx];
            assert!(
                group_has(SIMILAR_INITIALS, answer.as_str())
                    || group_has(INITIAL_SHAPES, answer.as_str())
            );
            found = true;
        }
        assert!(found);
    }

    #[test]
    fn final_question_prefers_similar_finals() {
        let mut seed = 99u64;
        let p = pool();
        let t = tones();
        let mut found = false;
        for _ in 0..200 {
            let q = new_question(&mut seed, &p, &t, false, &WrongBook::default());
            if q.kind != QuestionKind::ListenFinal {
                continue;
            }
            let answer = &q.options[q.answer_idx];
            assert!(
                group_has(SIMILAR_FINALS, answer.as_str())
                    || group_has(FINAL_SHAPES, answer.as_str())
            );
            found = true;
        }
        assert!(found);
    }

    #[test]
    fn tone_question_uses_real_tone_and_four_labels() {
        let mut seed = 2026u64;
        let q = make_question(&mut seed, QuestionKind::ListenTone, "ba", &tones()).unwrap();
        assert!(q.audio_tone >= 1 && q.audio_tone <= 4);
        assert_eq!(
            TONE_OPTIONS[q.audio_tone as usize - 1],
            q.options[q.answer_idx]
        );
        assert_eq!(q.options.len(), 4);
    }

    #[test]
    fn wrong_book_records_and_clears_mastered_items() {
        let mut seed = 1u64;
        let p = pool();
        let t = tones();
        let q = new_question(&mut seed, &p, &t, false, &WrongBook::default());
        let mut book = WrongBook::default();
        book.record(&q, false);
        book.record(&q, false);
        assert_eq!(book.len(), 1);
        assert_eq!(book.entries[0].count, 2);
        book.record(&q, true);
        assert_eq!(book.entries[0].count, 1);
        book.record(&q, true);
        assert!(book.is_empty());
    }

    #[test]
    fn wrong_only_mode_reuses_wrong_book() {
        let mut seed = 5u64;
        let p = pool();
        let t = tones();
        let q = new_question(&mut seed, &p, &t, false, &WrongBook::default());
        let mut book = WrongBook::default();
        book.record(&q, false);
        let review = new_question(&mut seed, &p, &t, true, &book);
        assert_eq!(review.kind, q.kind);
        assert_eq!(review.syllable, q.syllable);
    }

    #[test]
    fn wrong_book_roundtrip() {
        let mut seed = 42u64;
        let p = pool();
        let t = tones();
        let mut book = WrongBook::default();
        for _ in 0..6 {
            let q = new_question(&mut seed, &p, &t, false, &book);
            book.record(&q, false);
        }
        let text = book.encode();
        assert_eq!(WrongBook::decode(&text), book);
    }

    #[test]
    fn is_deterministic_for_same_seed() {
        let mut s1 = 999u64;
        let mut s2 = 999u64;
        let p = pool();
        let t = tones();
        assert_eq!(
            new_question(&mut s1, &p, &t, false, &WrongBook::default()),
            new_question(&mut s2, &p, &t, false, &WrongBook::default())
        );
    }

    #[test]
    fn seed_advances() {
        let mut seed = 1u64;
        let a = next_rand(&mut seed);
        let b = next_rand(&mut seed);
        assert_ne!(a, b);
    }
}
