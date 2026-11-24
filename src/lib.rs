//! 拼音拼读：小朋友用声母 + 韵母拼出音节并听发音。
//!
//! 工程约束见 `RUST-CONVENTIONS.md`。下面两条只作用于非测试构建：测试里
//! 用 `unwrap`/`expect` 表达"夹具必须成立"是合理的。
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod audio_tone;
pub mod data;
pub mod dtw;
pub mod mic;
pub mod pinyin;
pub mod quiz;

use std::rc::Rc;

use nsh_core::{
    audio::PlayerSettings,
    canvas::{LineCap, LineJoin, svg_path},
    element::{ImageAsset, Timing, asset},
    prelude::*,
    storage::Storage,
};

use crate::data::mnemonics::{RULES, mnemonic};
use crate::data::pinyin_data::{FINALS, INITIALS, SYLLABLES};
use crate::pinyin::{
    ComposeResult, compose, compose_three, finals_for, finals_for_three, medials_for,
};
use crate::quiz::{Question, QuestionKind, WrongBook, new_question};

const PACKAGE: &str = env!("CARGO_PKG_NAME");

// 界面资源：渐变背景 + 图标（随 assets!() 一起打包）
const BG: ImageAsset = asset!("bg.png");
const WAVE: ImageAsset = asset!("wave.png");
const SPEAKER: ImageAsset = asset!("speaker.png");
const CLEAR: ImageAsset = asset!("clear.png");

/// 声调名称（含轻声），与下方配色一一对应。
const TONE_NAMES: [&str; 5] = ["一声", "二声", "三声", "四声", "轻声"];
/// 声调调号字形（含轻声）。
const TONE_MARKS: [&str; 5] = ["\u{2500}", "\u{2571}", "\u{2228}", "\u{2572}", "\u{00B7}"];

mod baidu {
    include!(concat!(env!("OUT_DIR"), "/baidu_tones.rs"));
}

mod examples {
    include!(concat!(env!("OUT_DIR"), "/examples.rs"));
}

mod tone_examples {
    include!(concat!(env!("OUT_DIR"), "/tone_examples.rs"));
}

const WRONG_BOOK_KEY: &str = "pinyin_pal_wrong_book_v1";

/// 认读层掌握度存储键。每个字母 0=生 1=熟 2=掌握。
const RECOG_KEY: &str = "pinyin_pal_recognition_v1";
/// 6 组易混对，认读层最小版：(左, 右)。
const CONFUSABLE_PAIRS: [(&str, &str); 6] = [
    ("b", "d"),
    ("p", "q"),
    ("f", "t"),
    ("n", "h"),
    ("ie", "ei"),
    ("iu", "ui"),
];

/// 从设备存储读取 12 个字母的掌握度；无记录时全为 0（生）。
fn load_recog_points() -> [u8; 12] {
    let mut pts = [0u8; 12];
    if let Some(text) = Storage::get_item(RECOG_KEY) {
        for (i, part) in text.split(',').enumerate().take(12) {
            pts[i] = part.trim().parse::<u8>().unwrap_or(0).min(2);
        }
    }
    pts
}

/// 掌握度写回设备存储，用逗号分隔的 12 个数字。
fn save_recog_points(pts: &[u8; 12]) {
    let text = pts.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",");
    let _ = Storage::set_item(RECOG_KEY, &text);
}

/// 认读卡的口诀：声母用现成顺口溜，韵母用第五幕的方位口诀。
fn recog_note(letter: &str) -> &'static str {
    let m = mnemonic(letter);
    if !m.is_empty() {
        return m;
    }
    match letter {
        "ie" => "i 在前，姐姐的 ie",
        "ei" => "e 在前，妹妹的 ei",
        "iu" => "i 在前，加油的 iu",
        "ui" => "u 在前，威风的 ui",
        _ => "",
    }
}

/// 从 nsh 的设备存储读取错题库；没有记录时返回空。
fn load_wrong_book() -> WrongBook {
    Storage::get_item(WRONG_BOOK_KEY)
        .map(|text| WrongBook::decode(&text))
        .unwrap_or_default()
}

/// 错题库写回设备存储；容量超限时不崩溃，仅保留本次会话状态。
fn save_wrong_book(book: &WrongBook) {
    let _ = Storage::set_item(WRONG_BOOK_KEY, book.encode());
}

/// 例字映射是「第一声同音字」（ba→芭、ma→妈），用来帮孩子把音和字对上。
fn example_for(syllable: &str) -> Option<&'static str> {
    examples::example_char(syllable)
}

/// 提示文字：分三种情形，复用结果卡同一行，不增加布局高度。
/// - 只选了声母（还没拼出音节）：显示该声母的顺口溜助记
/// - 拼出合法音节：按情况给出 ü 书写规则或标调规则
/// - 其它：空
fn rule_hint(initial: &str, medial: &str, final_: &str, spoken: &str) -> &'static str {
    if spoken.is_empty() || spoken == "×" {
        return if initial.is_empty() {
            ""
        } else {
            mnemonic(initial)
        };
    }
    // 三拼：强调"声轻介快韵母响"的读法。
    if !medial.is_empty() {
        return "三拼：声轻介快韵母响，一口气连起来";
    }
    // j/q/x/y + ü 在书写上省略两点（ju/que/xun/yuan）。
    if matches!(initial, "j" | "q" | "x" | "y") && matches!(final_, "ü" | "ün" | "üe") {
        return "ü 遇到 j q x y，写成 u 仍读 ü";
    }
    "标调：有 a 找 a，没 a 找 o e，i u 并列标在后"
}

/// 播放一个拼音的发音。
///
/// 优先用百度真人的分声调 mp3（`assets/audio/bd/<name><tone>.mp3`）；
/// 该音节/声调不存在时回退到本地 TTS 合成的 `<name>.m4a`。
/// 用 asset URI 流式播放，避免 Sound 的 64 个上限。
/// 声调配色：(浅底, 主色)。轻声 5 用灰紫。
fn tone_colors(t: u8) -> (u32, u32) {
    match t {
        1 => (0xE8F1FE, 0x3B7DE0),
        2 => (0xE4F6EC, 0x1E9E63),
        3 => (0xFDF1DC, 0xE09112),
        4 => (0xFDE9EE, 0xD83A5B),
        _ => (0xEDEBF6, 0x8E86A8),
    }
}

fn speak(name: &str, tone: u8, rate: f32) {
    let mask = baidu::baidu_mask(name);
    // 轻声(5) 没有专门音频，回退到第一声；tone=0 同样回退。
    let wanted = if tone == 0 || tone == 5 { 1 } else { tone };
    let uri = if mask & (1 << (wanted - 1)) != 0 {
        format!("asset:/@{PACKAGE}/audio/bd/{name}{wanted}.mp3")
    } else {
        format!("asset:/@{PACKAGE}/audio/{name}.m4a")
    };
    let mut player = audio_player(&uri, PlayerSettings::default());
    let mut settings = PlayerSettings::default();
    settings.rate = rate;
    player.set_settings(settings);
    spawn(async move {
        player.play();
        // 保持 player 存活到播放结束，drop 会停止播放。
        while let Some(event) = player.next().await {
            if matches!(event, nsh_core::audio::AudioEvent::Ended) {
                break;
            }
        }
    });
}

/// 依次读两个音（三拼没有整音素材时：j → i → a）。
///
/// 延迟用 `set_timeout`，它的 `Spawned` 一旦 drop 会取消任务，这里显式
/// `forget`，代价是每次点击泄漏一个极小的句柄。
fn speak_after(name: &'static str, tone: u8, ms: u64) {
    std::mem::forget(set_timeout(
        std::time::Duration::from_millis(ms),
        move || speak(name, tone, 1.0),
    ));
}

/// 三拼分解读法：先读声母，再读介母，最后读韵母。
fn speak_three(first: &'static str, medial: &'static str, final_: &'static str) {
    speak(first, 0, 1.0);
    speak_after(medial, 0, 380);
    speak_after(final_, 0, 760);
}

/// 播放当前拼读结果。三拼优先读整音（百度真人录音），缺该声调时回退分解连读；
/// 两拼直接读整音。
fn speak_current(
    initial: &'static str,
    medial: &'static str,
    final_: &'static str,
    spoken: &str,
    tone: u8,
    rate: f32,
) {
    if !medial.is_empty() {
        let wanted = if tone == 0 || tone == 5 { 1 } else { tone };
        let has_whole = baidu::baidu_mask(spoken) & (1 << (wanted - 1)) != 0;
        if has_whole {
            speak(spoken, tone, rate);
        } else {
            speak_three(initial, medial, final_);
        }
    } else if !spoken.is_empty() && spoken != "×" {
        speak(spoken, tone, rate);
    }
}

#[native_module("pinyin-pal/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>().assets(assets!());
}

/// 拼音卡片配色：用浅蓝表示声母、浅绿表示韵母，选中色保持同色系。
#[derive(Clone, Copy)]
struct TileColors {
    normal_bg: u32,
    normal_fg: u32,
    selected_bg: u32,
    selected_fg: u32,
    disabled_bg: u32,
    disabled_fg: u32,
}

const INITIAL_COLORS: TileColors = TileColors {
    normal_bg: 0xEEF5FF,
    normal_fg: 0x4C709F,
    selected_bg: 0xD4E7FF,
    selected_fg: 0x285F9E,
    disabled_bg: 0xE7EEF7,
    disabled_fg: 0xA6B7CA,
};

const FINAL_COLORS: TileColors = TileColors {
    normal_bg: 0xEDF9F3,
    normal_fg: 0x43866B,
    selected_bg: 0xD4F1E3,
    selected_fg: 0x247A55,
    disabled_bg: 0xE6F0EA,
    disabled_fg: 0xA7BEb1,
};

/// 一个可点击的拼音卡片。
fn tile(
    label: &'static str,
    colors: TileColors,
    selected: impl Fn() -> bool + 'static,
    disabled: impl Fn() -> bool + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let id = format!("tile-{label}");
    let selected: Rc<dyn Fn() -> bool> = Rc::new(selected);
    let disabled: Rc<dyn Fn() -> bool> = Rc::new(disabled);
    let bg_selected = rgb(colors.selected_bg);
    let bg_normal = rgb(colors.normal_bg);
    let bg_disabled = rgb(colors.disabled_bg);
    let fg_selected = rgb(colors.selected_fg);
    let fg_normal = rgb(colors.normal_fg);
    let fg_disabled = rgb(colors.disabled_fg);
    let sel = selected.clone();
    let dis = disabled.clone();
    let bg = move || {
        if sel() {
            bg_selected
        } else if dis() {
            bg_disabled
        } else {
            bg_normal
        }
    };
    let sel = selected.clone();
    let dis = disabled.clone();
    let fg = move || {
        if sel() {
            fg_selected
        } else if dis() {
            fg_disabled
        } else {
            fg_normal
        }
    };
    let dis_op = disabled.clone();
    let dis_press = disabled.clone();
    view()
        .test_id(&id)
        .flex_grow(1.0)
        .h_full()
        .items_center()
        .justify_center()
        .rounded(px(14.0))
        .shadow_sm()
        .bg(bg)
        .text_color(fg)
        .opacity(move || if dis_op() { 0.45_f32 } else { 1.0_f32 })
        .active(|s| s.opacity(0.7))
        .on_press(move || {
            if !dis_press() {
                on_press();
            }
        })
        .child(text(label).text_size(px(19.0)))
}

fn section_title(title: &'static str, hint: &'static str) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(text(title).text_size(px(15.0)).text_color(rgb(0x2C2C2A)))
        .child(text(hint).text_size(px(12.0)).text_color(rgb(0x888780)))
}

#[component]
pub fn App() -> impl IntoElement {
    let initial: Signal<&'static str> = signal("");
    let medial: Signal<&'static str> = signal(""); // 三拼介母 i/u/ü，空=两拼
    let final_: Signal<&'static str> = signal("");
    let spoken: Signal<String> = signal(String::new());
    // 模式：0=拼读 1=查表 2=练习 3=知识 4=动画（认读已并入动画页子场景）
    let mode: Signal<u8> = signal(0);
    let slow = signal(false);
    let tone: Signal<u8> = signal(1);
    // 练习状态：种子 + 当前题目 + 是否已答 + 得分/题数 + 错题库/复习模式
    let mut seed = 0x9E3779B97F4A7C15u64;
    let empty_book = WrongBook::default();
    let first_q = new_question(
        &mut seed,
        examples::EXAMPLES,
        tone_examples::TONE_EXAMPLES,
        false,
        &empty_book,
    );
    let quiz_seed: Signal<u64> = signal(seed);
    let quiz_q: Signal<Question> = signal(first_q);
    let quiz_answered = signal(false);
    let quiz_score = signal((0u32, 0u32));
    let quiz_wrong: Signal<WrongBook> = signal(load_wrong_book());
    let quiz_review = signal(false);
    // 跟读评测状态（知识页顶部卡片用）
    let rec_state: Signal<RecTest> = signal(RecTest::Idle);

    view()
        .flex_1()
        .text_color(rgb(0x2E2C3A))
        .child(image(BG).absolute().inset_0())
        .p_safe_area()
        .child(
            view()
                .flex_1()
                .px(px(18.0))
                .py(px(12.0))
                .gap(px(10.0))
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap(px(9.0))
                                .child(image(WAVE).w(px(26.0)).h(px(30.0)))
                                .child(
                                    view()
                                        .gap(px(1.0))
                                        .child(
                                            text("拼音拼读")
                                                .text_size(px(22.0))
                                                .text_color(rgb(0x2E2C3A)),
                                        )
                                        .child(
                                            text("轻松拼读 · 快乐学习")
                                                .text_size(px(11.0))
                                                .text_color(rgb(0x9A97AC)),
                                        ),
                                ),
                        )
                        .child(mode_switcher(mode)),
                )
                .child(when(
                    move || mode.get() == 0,
                    move || {
                        view()
                            .flex_1()
                            .gap(px(10.0))
                            .child(practice_view(
                                initial, medial, final_, spoken, slow, tone,
                            ))
                            .child(tone_card_view(initial, medial, final_, spoken, tone))
                            .child(result_view(
                                initial, medial, final_, spoken, slow, tone,
                            ))
                    },
                )
                .otherwise(move || {
                    when(move || mode.get() == 1, move || syllable_table_view(tone)).otherwise(
                        move || {
                            when(move || mode.get() == 2, move || {
                                quiz_view(
                                    quiz_seed,
                                    quiz_q,
                                    quiz_answered,
                                    quiz_score,
                                    quiz_wrong,
                                    quiz_review,
                                )
                            })
                            .otherwise(move || {
                                when(
                                    move || mode.get() == 3,
                                    move || {
                                        knowledge_view(initial, medial, final_, tone, rec_state)
                                    },
                                )
                                .otherwise(animation_view)
                            })
                        },
                    )
                })),
        )
}

/// 顶部五态切换按钮（胶囊）：拼读 / 查表 / 练习 / 知识 / 动画（认读在动画页内）。
fn mode_switcher(mode: Signal<u8>) -> impl IntoElement {
    let items = [
        ("拼读", 0u8),
        ("查表", 1),
        ("练习", 2),
        ("知识", 3),
        ("动画", 4),
    ];
    view()
        .flex_row()
        .gap(px(2.0))
        .p(px(3.0))
        .rounded(px(18.0))
        .bg(rgb(0xF3F1FB))
        .children(
            items
                .iter()
                .map(|&(label, value)| {
                    let m = mode;
                    view()
                        .px(px(9.0))
                        .py(px(7.0))
                        .rounded(px(15.0))
                        .bg(move || {
                            if m.get() == value {
                                rgb(0x6C4FF5)
                            } else {
                                rgb(0xF3F1FB)
                            }
                        })
                        .text_color(move || {
                            if m.get() == value {
                                rgb(0xFFFFFF)
                            } else {
                                rgb(0x6C4FF5)
                            }
                        })
                        .active(|s| s.opacity(0.75))
                        .on_press(move || m.set(value))
                        .child(text(label).text_size(px(12.0)))
                })
                .collect::<Vec<_>>(),
        )
}

/// 声调独立大卡片：喇叭图标 + 标题/说明 + 清除按钮 + 五个彩色声调按钮。
fn tone_card_view(
    initial: Signal<&'static str>,
    medial: Signal<&'static str>,
    final_: Signal<&'static str>,
    spoken: Signal<String>,
    tone: Signal<u8>,
) -> impl IntoElement {
    let clear_i = initial;
    let clear_m = medial;
    let clear_f = final_;
    let clear_s = spoken;
    view()
        .gap(px(10.0))
        .p(px(14.0))
        .rounded(px(20.0))
        .bg(rgb(0xFFFFFF))
        .shadow_sm()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(9.0))
                .child(image(SPEAKER).w(px(24.0)).h(px(24.0)))
                .child(
                    view()
                        .gap(px(1.0))
                        .child(text("声调").text_size(px(16.0)).text_color(rgb(0x2E2C3A)))
                        .child(
                            text("选择声调，学习更高效")
                                .text_size(px(11.0))
                                .text_color(rgb(0xA6A3B5)),
                        ),
                )
                .child(view().flex_1())
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(5.0))
                        .px(px(12.0))
                        .py(px(7.0))
                        .rounded(px(14.0))
                        .bg(rgb(0xF3F1FB))
                        .active(|s| s.opacity(0.7))
                        .on_press(move || {
                            clear_i.set("");
                            clear_m.set("");
                            clear_f.set("");
                            clear_s.set(String::new());
                        })
                        .child(image(CLEAR).w(px(14.0)).h(px(14.0)))
                        .child(text("清除").text_size(px(12.0)).text_color(rgb(0x8E86A8))),
                ),
        )
        .child(
            view().flex_row().gap(px(7.0)).children(
                [1u8, 2, 3, 4, 5]
                    .iter()
                    .map(|&t| {
                        let set_tone = tone;
                        let sel_t = tone;
                        let pi = initial;
                        let pm = medial;
                        let pf = final_;
                        let ps = spoken;
                        let (bg_c, fg_c) = tone_colors(t);
                        view()
                            .test_id(&format!("tone-{t}"))
                            .flex_grow(1.0)
                            .h(px(62.0))
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .gap(px(1.0))
                            .rounded(px(13.0))
                            .border(move || if sel_t.get() == t { px(2.0) } else { px(0.0) })
                            .border_color(move || {
                                if sel_t.get() == t {
                                    rgb(fg_c)
                                } else {
                                    rgb(bg_c)
                                }
                            })
                            .bg(rgb(bg_c))
                            .active(|s| s.opacity(0.7))
                            .on_press(move || {
                                set_tone.set(t);
                                let s = ps.get();
                                speak_current(pi.get(), pm.get(), pf.get(), &s, t, 1.0_f32);
                            })
                            .child(
                                text(TONE_NAMES[(t - 1) as usize])
                                    .text_size(px(12.0))
                                    .text_color(rgb(fg_c)),
                            )
                            .child(
                                text(TONE_MARKS[(t - 1) as usize])
                                    .text_size(px(24.0))
                                    .text_color(rgb(fg_c)),
                            )
                    })
                    .collect::<Vec<_>>(),
            ),
        )
}

/// 三拼介母卡片配色：介于声母蓝与韵母绿之间的青色。
const MEDIAL_COLORS: TileColors = TileColors {
    normal_bg: 0xECF6F8,
    normal_fg: 0x3B7F8C,
    selected_bg: 0xC9E9EE,
    selected_fg: 0x1F6673,
    disabled_bg: 0xE6EFF1,
    disabled_fg: 0xA6BCBF,
};

fn practice_view(
    initial: Signal<&'static str>,
    medial: Signal<&'static str>,
    final_: Signal<&'static str>,
    spoken: Signal<String>,
    slow: Signal<bool>,
    tone: Signal<u8>,
) -> impl IntoElement {
    let i_sel = initial;
    let i_tile = initial;
    let i_dis = initial;
    let m_tile = medial;
    let m_dis = medial;
    let f_tile = final_;
    let f_sel = final_;
    let f_dis = final_;
    let sp_tile = spoken;

    // 点声母：重置介母与韵母，重新发音。
    let on_initial = move |s: &'static str| {
        i_tile.set(s);
        m_tile.set("");
        f_tile.set("");
        sp_tile.set(String::new());
        speak(s, 1, 1.0_f32);
    };

    // 点介母：切换三拼，重置韵母与结果，读介母音（仅该声母支持时）。
    let on_medial = move |m: &'static str| {
        m_dis.set(m);
        f_dis.set("");
        sp_tile.set(String::new());
        speak(m, 1, 1.0_f32);
    };

    // 点韵母/后韵：有声母时拼读，无声母时单独读韵母。
    let on_final = move |f: &'static str| {
        f_tile.set(f);
        let i = i_dis.get();
        let m = m_dis.get();
        if i.is_empty() {
            speak(f, 1, 1.0_f32);
            sp_tile.set(String::new());
        } else {
            let result = if m.is_empty() {
                compose(i, f)
            } else {
                compose_three(i, m, f)
            };
            match result {
                ComposeResult::Valid(syl) => {
                    sp_tile.set(syl.clone());
                    speak_current(i, m, f, &syl, tone.get(), if slow.get() { 0.75 } else { 1.0 });
                }
                ComposeResult::Invalid => sp_tile.set("×".to_owned()),
            }
        }
    };

    // 韵母区：两拼显示全部 FINALS；三拼只显示该声母+介母可用的后韵。
    // 非法后韵用禁用态（灰掉）而非隐藏，避免切换时网格跳动。
    let finals_view = view().flex_grow(1.0).gap(px(4.0)).child(
        view().flex_grow(1.0).gap(px(6.0)).children(
            FINALS
                .chunks(6)
                .map(|row| {
                    let cells = row
                        .iter()
                        .map(|&f| {
                            let press = on_final;
                            let fi = initial;
                            let fm = medial;
                            tile(
                                f,
                                FINAL_COLORS,
                                move || f_sel.get() == f,
                                move || {
                                    let i = fi.get();
                                    if i.is_empty() {
                                        false
                                    } else {
                                        let m = fm.get();
                                        if m.is_empty() {
                                            !finals_for(i).contains(&f)
                                        } else {
                                            !finals_for_three(i, m).contains(&f)
                                        }
                                    }
                                },
                                move || press(f),
                            )
                        })
                        .collect::<Vec<_>>();
                    view()
                        .flex_row()
                        .flex_grow(1.0)
                        .justify_center()
                        .gap(px(6.0))
                        .children(cells)
                })
                .collect::<Vec<_>>(),
        ),
    );

    view()
        .flex_grow(1.0)
        .gap(px(5.0))
        .child(
            view().flex_grow(1.0).gap(px(4.0)).child(
                view().flex_grow(1.0).gap(px(6.0)).children(
                    INITIALS
                        .chunks(6)
                        .map(|row| {
                            let cells = row
                                .iter()
                                .map(|&s| {
                                    let press = on_initial;
                                    tile(
                                        s,
                                        INITIAL_COLORS,
                                        move || i_sel.get() == s,
                                        || false,
                                        move || press(s),
                                    )
                                })
                                .collect::<Vec<_>>();
                            view()
                                .flex_row()
                                .flex_grow(1.0)
                                .justify_center()
                                .gap(px(6.0))
                                .children(cells)
                        })
                        .collect::<Vec<_>>(),
                ),
            ),
        )
        .child(view().h(px(1.0)).mx(px(8.0)).bg(rgb(0xE6E4DC)))
        // 介母行：无声母时全部禁用；有声母时只亮起该声母支持的介母。
        .child(
            view().flex_row().items_center().gap(px(6.0)).child(
                text("三拼介母").text_size(px(11.0)).text_color(rgb(0x888780)),
            )
            .children(["i", "u", "ü"].iter().map(|&m| {
                let press = on_medial;
                let mi = initial;
                tile(
                    m,
                    MEDIAL_COLORS,
                    move || m_dis.get() == m,
                    move || {
                        let i = mi.get();
                        i.is_empty() || !medials_for(i).contains(&m)
                    },
                    move || press(m),
                )
            })),
        )
        .child(view().h(px(1.0)).mx(px(8.0)).bg(rgb(0xE6E4DC)))
        .child(finals_view)
}

fn result_view(
    initial: Signal<&'static str>,
    medial: Signal<&'static str>,
    final_: Signal<&'static str>,
    spoken: Signal<String>,
    slow: Signal<bool>,
    tone: Signal<u8>,
) -> impl IntoElement {
    let play_i = initial;
    let play_m = medial;
    let play_f = final_;
    let play_spoken = spoken;
    let play_slow = slow;
    let play_tone = tone;
    let hint_init = initial;
    let hint_medial = medial;
    let hint_final = final_;
    let hint_spoken = spoken;
    view()
        .gap(px(7.0))
        .p(px(12.0))
        .rounded(px(20.0))
        .bg(rgb(0xFFFFFF))
        .shadow_sm()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    view()
                        .flex_1()
                        .flex_row()
                        .items_center()
                        .gap(px(7.0))
                        .child(
                            text(move || {
                                let i = initial.get();
                                let m = medial.get();
                                let f = final_.get();
                                let i_s = if i.is_empty() { "选声母" } else { i };
                                let f_s = if f.is_empty() { "选韵母" } else { f };
                                if m.is_empty() {
                                    format!("{i_s} + {f_s} =")
                                } else {
                                    format!("{i_s} + {m} + {f_s} =")
                                }
                            })
                            .text_size(px(19.0))
                            .text_color(rgb(0x3A3848)),
                        )
                        .child(
                            text(move || spoken.get())
                                .text_size(px(24.0))
                                .text_color(rgb(0x6C4FF5)),
                        ),
                )
                .child(
                    view()
                        .px(px(22.0))
                        .py(px(13.0))
                        .rounded(px(16.0))
                        .bg(rgb(0x6C4FF5))
                        .active(|s| s.opacity(0.75))
                        .on_press(move || {
                            let s = play_spoken.get();
                            speak_current(
                                play_i.get(),
                                play_m.get(),
                                play_f.get(),
                                &s,
                                play_tone.get(),
                                if play_slow.get() { 0.75_f32 } else { 1.0_f32 },
                            );
                        })
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap(px(7.0))
                                .child(
                                    text("\u{25B6}")
                                        .text_size(px(15.0))
                                        .text_color(rgb(0xFFFFFF)),
                                )
                                .child(text("播放").text_size(px(17.0)).text_color(rgb(0xFFFFFF))),
                        ),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(7.0))
                .child(text("例字").text_size(px(11.0)).text_color(rgb(0xA6A3B5)))
                .child(
                    text(move || {
                        let s = hint_spoken.get();
                        if s.is_empty() || s == "×" {
                            "—".to_owned()
                        } else {
                            example_for(&s).unwrap_or("—").to_owned()
                        }
                    })
                    .text_size(px(17.0))
                    .text_color(rgb(0x3B7DE0)),
                )
                .child(view().w(px(6.0)))
                .child(
                    view().flex_1().child(
                        text(move || {
                            rule_hint(
                                hint_init.get(),
                                hint_medial.get(),
                                hint_final.get(),
                                &hint_spoken.get(),
                            )
                            .to_owned()
                        })
                        .text_size(px(11.0))
                        .text_color(rgb(0xA6A3B5)),
                    ),
                )
                .child(
                    view()
                        .px(px(10.0))
                        .py(px(5.0))
                        .rounded(px(10.0))
                        .bg(move || {
                            if slow.get() {
                                rgb(0x6C4FF5)
                            } else {
                                rgb(0xF3F1FB)
                            }
                        })
                        .text_color(move || {
                            if slow.get() {
                                rgb(0xFFFFFF)
                            } else {
                                rgb(0x8E86A8)
                            }
                        })
                        .active(|s| s.opacity(0.75))
                        .on_press(move || slow.update(|v| *v = !*v))
                        .child(
                            text(move || if slow.get() { "慢速中" } else { "慢速" })
                                .text_size(px(11.0)),
                        ),
                ),
        )
}

/// 多题型练习：听音选字 / 看字选拼音 / 听音选声母 / 听音选韵母 / 听音选声调。
/// 答错进入错题库；可在普通练习与错题复习之间切换。
fn quiz_view(
    seed: Signal<u64>,
    question: Signal<Question>,
    answered: Signal<bool>,
    score: Signal<(u32, u32)>,
    wrong: Signal<WrongBook>,
    review: Signal<bool>,
) -> impl IntoElement {
    // 选中的选项下标，用 u8 的哨兵 255 表示未选。
    let picked: Signal<u8> = signal(255);

    // 换题自动朗读；看字选拼音不自动播放，否则会泄题。
    let auto_play = question;
    effect(move || {
        let q = auto_play.get();
        if q.kind != QuestionKind::ReadChar {
            speak(&q.syllable, q.audio_tone, 1.0_f32);
        }
    });

    let q_play = question;
    let q_toggle = question;
    let wrong_toggle = wrong;
    let review_toggle = review;

    view()
        .flex_1()
        .gap(px(12.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    text(move || {
                        let (right, total) = score.get();
                        let mode = if review.get() {
                            "错题复习"
                        } else {
                            "综合练习"
                        };
                        format!("{mode} · 答对 {right}/{total}")
                    })
                    .flex_1()
                    .text_size(px(13.0))
                    .text_color(rgb(0x888780)),
                )
                .child(
                    view()
                        .test_id("quiz-wrong-toggle")
                        .px(px(12.0))
                        .py(px(9.0))
                        .rounded(px(10.0))
                        .bg(move || {
                            if review_toggle.get() {
                                rgb(0xE4DDFD)
                            } else {
                                rgb(0xF3F1FB)
                            }
                        })
                        .active(|s| s.opacity(0.75))
                        .on_press(move || review_toggle.update(|v| *v = !*v))
                        .child(
                            text(move || {
                                let n = wrong_toggle.get().len();
                                if review_toggle.get() {
                                    format!("错题 {n} · 复习中")
                                } else {
                                    format!("错题 {n}")
                                }
                            })
                            .text_size(px(12.0))
                            .text_color(rgb(0x6750F5)),
                        ),
                )
                .child(
                    view()
                        .test_id("quiz-replay")
                        .px(px(13.0))
                        .py(px(9.0))
                        .rounded(px(10.0))
                        .bg(rgb(0x6750F5))
                        .active(|s| s.opacity(0.75))
                        .on_press(move || {
                            let q = q_play.get();
                            speak(&q.syllable, q.audio_tone, 1.0_f32);
                        })
                        .child(
                            text("再听一次")
                                .text_size(px(12.0))
                                .text_color(rgb(0xFFFFFF)),
                        ),
                ),
        )
        .child(
            text(move || {
                let q = q_toggle.get();
                if q.kind == QuestionKind::ReadChar {
                    let char_ = example_for(&q.syllable).unwrap_or("?");
                    format!("看「{char_}」，选出正确拼音")
                } else {
                    q.kind.prompt().to_owned()
                }
            })
            .text_size(px(16.0))
            .text_color(rgb(0x2C2C2A)),
        )
        .child(
            view().flex_grow(1.0).gap(px(10.0)).children(
                [0u8, 1]
                    .into_iter()
                    .map(|row| {
                        view().flex_row().flex_grow(1.0).gap(px(10.0)).children(
                            [0u8, 1]
                                .into_iter()
                                .map(|col| {
                                    let idx = row * 2 + col;
                                    let q = question;
                                    let ans = answered;
                                    let sc = score;
                                    let pk = picked;
                                    let wb = wrong;
                                    let right_q = question;
                                    let chosen = move || picked.get() == idx;
                                    let is_right = move || idx as usize == right_q.get().answer_idx;
                                    view()
                                        .test_id(&format!("quiz-opt-{idx}"))
                                        .flex_grow(1.0)
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(16.0))
                                        .shadow_sm()
                                        .bg(move || {
                                            if !ans.get() {
                                                rgb(0xFFFFFF)
                                            } else if is_right() {
                                                rgb(0xE4F6EC)
                                            } else if chosen() {
                                                rgb(0xFDE9EE)
                                            } else {
                                                rgb(0xFFFFFF)
                                            }
                                        })
                                        .text_color(move || {
                                            if ans.get() && is_right() {
                                                rgb(0x1E9E63)
                                            } else if ans.get() && chosen() {
                                                rgb(0xD83A5B)
                                            } else {
                                                rgb(0x2C2C2A)
                                            }
                                        })
                                        .active(|s| s.opacity(0.8))
                                        .on_press(move || {
                                            if ans.get() {
                                                return;
                                            }
                                            pk.set(idx);
                                            ans.set(true);
                                            let q2 = q.get();
                                            let ok = idx as usize == q2.answer_idx;
                                            sc.update(|(r, t)| {
                                                *t += 1;
                                                if ok {
                                                    *r += 1;
                                                }
                                            });
                                            wb.update(|book| book.record(&q2, ok));
                                            save_wrong_book(&wb.get());
                                            speak(&q2.syllable, q2.audio_tone, 1.0_f32);
                                        })
                                        .child(
                                            text(move || {
                                                question.get().options[idx as usize].clone()
                                            })
                                            .text_size(move || match question.get().kind {
                                                QuestionKind::ListenChar => px(40.0),
                                                QuestionKind::ListenInitial
                                                | QuestionKind::ListenFinal => px(30.0),
                                                _ => px(24.0),
                                            }),
                                        )
                                })
                                .collect::<Vec<_>>(),
                        )
                    })
                    .collect::<Vec<_>>(),
            ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    text(move || {
                        let q = question.get();
                        if !answered.get() {
                            if review.get() && wrong.get().is_empty() {
                                "错题本为空，先做普通练习".to_owned()
                            } else {
                                q.kind.choose_hint().to_owned()
                            }
                        } else if picked.get() as usize == q.answer_idx {
                            "✓ 答对了".to_owned()
                        } else {
                            format!("✗ 正确答案：{} · 已记入错题本", q.options[q.answer_idx])
                        }
                    })
                    .text_size(px(14.0))
                    .text_color(move || {
                        if !answered.get() {
                            rgb(0x888780)
                        } else if picked.get() as usize == question.get().answer_idx {
                            rgb(0x1E9E63)
                        } else {
                            rgb(0xD83A5B)
                        }
                    }),
                )
                .child(view().flex_1())
                .child(
                    when(
                        move || review.get() && !wrong.get().is_empty(),
                        move || {
                            view()
                                .test_id("quiz-wrong-clear")
                                .px(px(10.0))
                                .py(px(10.0))
                                .rounded(px(10.0))
                                .bg(rgb(0xF3F1FB))
                                .active(|s| s.opacity(0.7))
                                .on_press(move || {
                                    wrong.set(WrongBook::default());
                                    save_wrong_book(&wrong.get());
                                    review.set(false);
                                })
                                .child(text("清空").text_size(px(12.0)).text_color(rgb(0x8E86A8)))
                        },
                    )
                    .otherwise(|| view()),
                )
                .child(
                    view()
                        .test_id("quiz-next")
                        .px(px(16.0))
                        .py(px(10.0))
                        .rounded(px(10.0))
                        .bg(rgb(0x6750F5))
                        .active(|s| s.opacity(0.75))
                        .on_press(move || {
                            picked.set(255);
                            answered.set(false);
                            let mut s = seed.get();
                            let q = new_question(
                                &mut s,
                                examples::EXAMPLES,
                                tone_examples::TONE_EXAMPLES,
                                review.get(),
                                &wrong.get(),
                            );
                            seed.set(s);
                            question.set(q);
                        })
                        .child(
                            text(move || {
                                if answered.get() {
                                    "下一题"
                                } else {
                                    "跳过"
                                }
                            })
                            .text_size(px(13.0))
                            .text_color(rgb(0xFFFFFF)),
                        ),
                ),
        )
}

// ================= 第 5 页：动画课堂（nsh 原生 Animated） =================

/// 延迟 `delay_ms` 后把 `v` 从 `from` 动画到 `to`（毫秒）。
///
/// 延迟用 `Timing::delay_ms` 交给动画图，而不是 `set_timeout`：后者返回的
/// `Spawned` 一旦被丢弃就会取消任务，动画根本不会开始。
fn anim_to(v: AnimatedValue, from: f32, to: f32, ms: f32, delay_ms: f32) {
    v.set(from);
    let _ = v.timing_with(
        to,
        Timing {
            duration_ms: ms,
            easing: Easing::EaseOut,
            delay_ms,
            frame_rate: None,
        },
    );
}

/// 四声调型曲线，画在 100×100 的网格坐标系里（与设计稿 SVG 的 viewBox 一致）；
/// 第二个数是曲线在网格里的长度，虚线偏移用它才能从第一帧就开始描边。
const TONE_PATHS: [(&str, f32); 4] = [
    ("M16 34 L84 34", 68.0),
    ("M16 74 L84 28", 82.0),
    ("M16 40 L46 82 L48 82 L78 34", 110.0),
    ("M16 26 L84 78", 86.0),
];

/// 五度标记网格 + 调型曲线：曲线用虚线偏移「画」出来，偏移量是被平台逐帧
/// 读取的动画值，所以描边动画在 UI 线程跑，不回调我们的代码。
fn tone_curve_canvas(idx: usize, color: u32, v: AnimatedValue) -> impl IntoElement {
    let (d, len) = TONE_PATHS[idx];
    let path = svg_path(d);
    canvas(move || {
        // len = 整条曲线都还是空隙；0 = 全部画完。
        let off = v.interpolate(&[0.0, 1.0], &[len, 0.0]);
        Painter::new()
            .scale(0.44, 0.44)
            // 三条横线：高、中（虚线）、低
            .line_width(1.4)
            .stroke_color(rgb(0xEEF2F6))
            .begin_path()
            .move_to(6.0, 26.0)
            .line_to(94.0, 26.0)
            .stroke()
            .begin_path()
            .move_to(6.0, 50.0)
            .line_to(94.0, 50.0)
            .line_dash(&[Points(4.0), Points(5.0)])
            .stroke()
            .line_dash(&[])
            .begin_path()
            .move_to(6.0, 74.0)
            .line_to(94.0, 74.0)
            .stroke()
            // 调型曲线
            .line_width(6.0)
            .line_cap(LineCap::Round)
            .line_join(LineJoin::Round)
            .stroke_color(rgb(color))
            .line_dash(&[Points(len), Points(len)])
            .line_dash_offset(&off)
            .stroke_path(&path)
    })
    .w(px(44.0))
    .h(px(44.0))
}

/// 一张声调卡：调型曲线 + 字母 + 名称，淡入并上浮。
fn anim_tone_card(
    idx: usize,
    letter: &'static str,
    name: &'static str,
    color: u32,
    v: AnimatedValue,
) -> impl IntoElement {
    let dy = v.interpolate(&[0.0, 1.0], &[20.0, 0.0]);
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap(px(3.0))
        .h(px(196.0))
        .rounded(px(16.0))
        .bg(rgb(0xFFFFFF))
        .shadow_sm()
        .bind_opacity(&v)
        .bind_translate_y(&dy)
        .on_press(move || speak("ma", (idx + 1) as u8, 1.0))
        .child(tone_curve_canvas(idx, color, v))
        .child(text(letter).text_size(px(26.0)).text_color(rgb(color)))
        .child(text(name).text_size(px(11.0)).text_color(rgb(0x888780)))
}

/// 第一幕：四个声调。
fn anim_scene_tones() -> impl IntoElement {
    let v = [
        animated_value(0.0),
        animated_value(0.0),
        animated_value(0.0),
        animated_value(0.0),
    ];
    for (i, value) in v.iter().enumerate() {
        anim_to(*value, 0.0, 1.0, 460.0, i as f32 * 130.0);
    }
    view().flex_row().gap(px(9.0)).children(
        [
            ("\u{0101}", "一声 平", 0xE8703F),
            ("\u{00E1}", "二声 扬", 0xE0A12A),
            ("\u{01CE}", "三声 拐弯", 0x2FA37A),
            ("\u{00E0}", "四声 降", 0x2F7FB5),
        ]
        .into_iter()
        .enumerate()
        .zip(v)
        .map(|((idx, (letter, name, color)), value)| {
            anim_tone_card(idx, letter, name, color, value)
        })
        .collect::<Vec<_>>(),
    )
}

/// 第二幕：b / d / p / q 的左右手四格。
fn anim_hand_cell(
    letter: &'static str,
    hint: &'static str,
    left: bool,
    v: AnimatedValue,
) -> impl IntoElement {
    let sc = v.interpolate(&[0.0, 1.0], &[0.76, 1.0]);
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap(px(2.0))
        .h(px(112.0))
        .rounded(px(18.0))
        .border(px(2.0))
        .border_color(if left { rgb(0xBCDCF0) } else { rgb(0xBFE8D7) })
        .bg(if left { rgb(0xF5FBFF) } else { rgb(0xF4FBF7) })
        .bind_opacity(&v)
        .bind_scale(&sc)
        .on_press(move || speak(letter, 0, 1.0))
        .child(
            text(letter)
                .text_size(px(42.0))
                .text_color(if left { rgb(0x2F7FB5) } else { rgb(0x2FA37A) }),
        )
        .child(text(hint).text_size(px(11.0)).text_color(rgb(0x888780)))
}

fn anim_scene_hands() -> impl IntoElement {
    let v = [
        animated_value(0.0),
        animated_value(0.0),
        animated_value(0.0),
        animated_value(0.0),
    ];
    for (i, value) in v.iter().enumerate() {
        anim_to(*value, 0.0, 1.0, 380.0, i as f32 * 150.0);
    }
    let row = |head: &'static str, a: (&'static str, bool), b: (&'static str, bool), va: AnimatedValue, vb: AnimatedValue| {
        view()
            .gap(px(8.0))
            .child(
                text(head)
                    .text_size(px(12.0))
                    .text_color(rgb(0x9A97AC)),
            )
            .child(
                view()
                    .flex_row()
                    .gap(px(9.0))
                    .child(anim_hand_cell(a.0, if a.1 { "左手" } else { "右手" }, a.1, va))
                    .child(anim_hand_cell(b.0, if b.1 { "左手" } else { "右手" }, b.1, vb)),
            )
    };
    view()
        .flex_1()
        .justify_center()
        .gap(px(10.0))
        .child(row("不向下", ("b", true), ("d", false), v[0], v[1]))
        .child(row("向下", ("p", true), ("q", false), v[2], v[3]))
}

/// ü 演示的一侧：大字 u、上方两点、标题与结论。
fn umlaut_card(
    title: &'static str,
    note: &'static str,
    tint: u32,
    speak_name: &'static str,
    dots: (impl IntoElement, impl IntoElement),
) -> impl IntoElement {
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap(px(4.0))
        .h(px(216.0))
        .rounded(px(18.0))
        .bg(rgb(0xFBFCFE))
        .border(px(2.0))
        .border_color(rgb(0xE6EDF3))
        .on_press(move || speak(speak_name, 1, 1.0))
        .child(text(title).text_size(px(16.0)).text_color(rgb(tint)))
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_center()
                .gap(px(16.0))
                .h(px(18.0))
                .child(dots.0)
                .child(dots.1),
        )
        .child(
            view()
                .h(px(88.0))
                .items_center()
                .justify_center()
                .child(text("u").text_size(px(80.0)).text_color(rgb(0x2E2C3A))),
        )
        .child(text(note).text_size(px(12.0)).text_color(rgb(0x888780)))
}

/// 第三幕：ü 的两点去留——左边 j q x 让两点飞走，右边 n l 让两点留下。
fn anim_scene_umlaut() -> impl IntoElement {
    // fly：0 = 两点在原位，1 = 两点飞走。
    let fly = animated_value(0.0);
    anim_to(fly, 0.0, 1.0, 400.0, 900.0);

    let dxl = fly.interpolate(&[0.0, 1.0], &[0.0, -34.0]);
    let dxr = fly.interpolate(&[0.0, 1.0], &[0.0, 34.0]);
    let dy = fly.interpolate(&[0.0, 1.0], &[0.0, -42.0]);
    let fade = fly.interpolate(&[0.0, 1.0], &[1.0, 0.0]);

    let moving_dot = |dx: &AnimatedNode| {
        view()
            .w(px(13.0))
            .h(px(13.0))
            .rounded(px(7.0))
            .bg(rgb(0x2F7FB5))
            .bind_translate_x(dx)
            .bind_translate_y(&dy)
            .bind_opacity(&fade)
    };
    let steady_dot = || {
        view()
            .w(px(13.0))
            .h(px(13.0))
            .rounded(px(7.0))
            .bg(rgb(0x2FA37A))
    };

    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap(px(10.0))
        .child(
            view()
                .flex_row()
                .w_full()
                .gap(px(12.0))
                .child(umlaut_card(
                    "j q x",
                    "擦掉眼泪 → ju",
                    0x2F7FB5,
                    "ju",
                    (moving_dot(&dxl), moving_dot(&dxr)),
                ))
                .child(umlaut_card(
                    "n l",
                    "两点不能去 → lü",
                    0x2FA37A,
                    "lü",
                    (steady_dot(), steady_dot()),
                )),
        )
        .child(
            text("两点去还是留，看它前面站着哪个声母")
                .text_size(px(12.0))
                .text_color(rgb(0x9A97AC)),
        )
}

/// 第四幕：介母飞入拼合。
fn anim_blend_row(
    initial: &'static str,
    medial: &'static str,
    final_: &'static str,
    syllable: &'static str,
    example: &'static str,
    m: AnimatedValue,
    r: AnimatedValue,
) -> impl IntoElement {
    let dy = m.interpolate(&[0.0, 1.0], &[-56.0, 0.0]);
    let ro = r.interpolate(&[0.0, 1.0], &[0.22, 1.0]);
    let rs = r.interpolate(&[0.0, 1.0], &[1.0, 1.05]);
    let cell = |label: &'static str, color: u32, bg: u32, border: u32| {
        view()
            .w(px(52.0))
            .h(px(58.0))
            .items_center()
            .justify_center()
            .rounded(px(14.0))
            .border(px(2.0))
            .border_color(rgb(border))
            .bg(rgb(bg))
            .on_press(move || speak(label, 0, 1.0))
            .child(text(label).text_size(px(28.0)).text_color(rgb(color)))
    };
    view()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(cell(initial, 0x2F7FB5, 0xF5FBFF, 0xC7E2F3))
        .child(text("＋").text_size(px(16.0)).text_color(rgb(0x9DABB6)))
        .child(
            view()
                .w(px(52.0))
                .h(px(58.0))
                .items_center()
                .justify_center()
                .rounded(px(14.0))
                .border(px(2.0))
                .border_color(rgb(0xFFD2BC))
                .bg(rgb(0xFFF6F0))
                .bind_opacity(&m)
                .bind_translate_y(&dy)
                .on_press(move || speak(medial, 0, 1.0))
                .child(text(medial).text_size(px(28.0)).text_color(rgb(0xE8703F))),
        )
        .child(text("＋").text_size(px(16.0)).text_color(rgb(0x9DABB6)))
        .child(cell(final_, 0x2FA37A, 0xF4FBF7, 0xC4E9D8))
        .child(text("→").text_size(px(16.0)).text_color(rgb(0x9DABB6)))
        .child(
            view()
                .w(px(96.0))
                .h(px(58.0))
                .items_center()
                .justify_center()
                .gap(px(1.0))
                .rounded(px(14.0))
                .border(px(2.0))
                .border_color(rgb(0xFFD0B9))
                .bg(rgb(0xFFF1E8))
                .bind_opacity(&ro)
                .bind_scale(&rs)
                .on_press(move || speak_three(initial, medial, final_))
                .child(
                    text(syllable)
                        .text_size(px(24.0))
                        .text_color(rgb(0xE8703F)),
                )
                .child(text(example).text_size(px(10.0)).text_color(rgb(0xA86A4A))),
        )
}

fn anim_scene_medial() -> impl IntoElement {
    let m1 = animated_value(0.0);
    let r1 = animated_value(0.0);
    let m2 = animated_value(0.0);
    let r2 = animated_value(0.0);
    anim_to(m1, 0.0, 1.0, 520.0, 120.0);
    anim_to(r1, 0.0, 1.0, 320.0, 620.0);
    anim_to(m2, 0.0, 1.0, 520.0, 920.0);
    anim_to(r2, 0.0, 1.0, 320.0, 1420.0);
    view()
        .flex_1()
        .justify_center()
        .gap(px(16.0))
        .child(anim_blend_row("j", "i", "a", "jia", "家", m1, r1))
        .child(anim_blend_row("g", "u", "o", "guo", "锅", m2, r2))
}

/// 一张复韵母对比卡：韵母 + 两个字母牌（从两侧滑入）+ 口诀 + 例字。
fn pair_card(
    syllable: &'static str,
    first: &'static str,
    second: &'static str,
    note: &'static str,
    examples: &'static str,
    speak_name: &'static str,
    tint: u32,
    v: AnimatedValue,
) -> impl IntoElement {
    let dy = v.interpolate(&[0.0, 1.0], &[16.0, 0.0]);
    let dxl = v.interpolate(&[0.0, 1.0], &[-18.0, 0.0]);
    let dxr = v.interpolate(&[0.0, 1.0], &[18.0, 0.0]);
    let letter = |ch: &'static str, dx: &AnimatedNode| {
        view()
            .w(px(40.0))
            .h(px(40.0))
            .items_center()
            .justify_center()
            .rounded(px(10.0))
            .bg(rgb(0xF5F9FC))
            .border(px(1.5))
            .border_color(rgb(0xD9E7F1))
            .bind_translate_x(dx)
            .child(text(ch).text_size(px(21.0)).text_color(rgb(0x2E2C3A)))
    };
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap(px(5.0))
        .h(px(166.0))
        .rounded(px(16.0))
        .bg(rgb(0xFFFFFF))
        .border(px(2.0))
        .border_color(rgb(0xE6EDF3))
        .bind_opacity(&v)
        .bind_translate_y(&dy)
        .on_press(move || speak(speak_name, 0, 1.0))
        .child(text(syllable).text_size(px(25.0)).text_color(rgb(tint)))
        .child(
            view()
                .flex_row()
                .gap(px(6.0))
                .child(letter(first, &dxl))
                .child(letter(second, &dxr)),
        )
        .child(text(note).text_size(px(11.0)).text_color(rgb(0x888780)))
        .child(text(examples).text_size(px(10.0)).text_color(rgb(tint)))
}

/// 第五幕：ie / ei、iu / ui —— 看谁排在前面。
fn anim_scene_pairs() -> impl IntoElement {
    let v = [
        animated_value(0.0),
        animated_value(0.0),
        animated_value(0.0),
        animated_value(0.0),
    ];
    for (i, value) in v.iter().enumerate() {
        anim_to(*value, 0.0, 1.0, 420.0, i as f32 * 140.0);
    }
    view()
        .flex_1()
        .justify_center()
        .items_stretch()
        .gap(px(10.0))
        .child(
            view()
                .flex_row()
                .w_full()
                .gap(px(10.0))
                .child(pair_card(
                    "ie", "i", "e", "i 在前 · 姐姐", "jie  qie", "jie", 0xE8703F, v[0],
                ))
                .child(pair_card(
                    "ei", "e", "i", "i 在后 · 妹妹", "mei  wei", "mei", 0x2F7FB5, v[1],
                )),
        )
        .child(
            view()
                .flex_row()
                .w_full()
                .gap(px(10.0))
                .child(pair_card(
                    "iu", "i", "u", "i 在前 · 喊加油", "niu  diu", "niu", 0xE8703F, v[2],
                ))
                .child(pair_card(
                    "ui", "u", "i", "u 在前 · 很威风", "hui  dui", "hui", 0x2FA37A, v[3],
                )),
        )
}

/// 拼合演示里的一小块字母。
fn blend_block(label: &'static str, color: u32, bg: u32, border: u32) -> impl IntoElement {
    view()
        .w(px(46.0))
        .h(px(52.0))
        .items_center()
        .justify_center()
        .rounded(px(12.0))
        .border(px(2.0))
        .border_color(rgb(border))
        .bg(rgb(bg))
        .on_press(move || speak(label, 0, 1.0))
        .child(text(label).text_size(px(24.0)).text_color(rgb(color)))
}

/// 一行拼合：a + b (+ c) → 结果。`c` 为空表示两拼。
///
/// 结果卡点击：有整音（`whole`）就播整音；三拼没有整音素材，改为依次
/// 读出声母、介母、韵母。
fn blend_line(
    a: &'static str,
    b: &'static str,
    c: &'static str,
    result: &'static str,
    note: &'static str,
    whole: Option<&'static str>,
    v: AnimatedValue,
) -> impl IntoElement {
    let dx = v.interpolate(&[0.0, 1.0], &[36.0, 0.0]);
    let ro = v.interpolate(&[0.0, 1.0], &[0.28, 1.0]);
    let rs = v.interpolate(&[0.0, 1.0], &[1.0, 1.05]);
    view()
        .flex_row()
        .items_center()
        .gap(px(7.0))
        .child(blend_block(a, 0x2F7FB5, 0xF5FBFF, 0xC7E2F3))
        .child(text("＋").text_size(px(14.0)).text_color(rgb(0x9DABB6)))
        .child(
            view()
                .bind_translate_x(&dx)
                .bind_opacity(&v)
                .child(blend_block(b, 0x2FA37A, 0xF4FBF7, 0xC4E9D8)),
        )
        .child(
            when(move || !c.is_empty(), move || {
                view()
                    .flex_row()
                    .items_center()
                    .gap(px(7.0))
                    .child(text("＋").text_size(px(14.0)).text_color(rgb(0x9DABB6)))
                    .child(blend_block(c, 0x2FA37A, 0xF4FBF7, 0xC4E9D8))
            })
            .otherwise(view),
        )
        .child(text("→").text_size(px(14.0)).text_color(rgb(0x9DABB6)))
        .child(
            view()
                .w(px(84.0))
                .h(px(52.0))
                .items_center()
                .justify_center()
                .rounded(px(12.0))
                .border(px(2.0))
                .border_color(rgb(0xFFD0B9))
                .bg(rgb(0xFFF1E8))
                .bind_opacity(&ro)
                .bind_scale(&rs)
                .on_press(move || match whole {
                    Some(w) => speak(w, 0, 1.0),
                    None => {
                        speak(a, 0, 1.0);
                        speak_after(b, 0, 380);
                        if !c.is_empty() {
                            speak_after(c, 0, 760);
                        }
                    }
                })
                .child(text(result).text_size(px(22.0)).text_color(rgb(0xE8703F))),
        )
        .child(
            view()
                .flex_1()
                .child(text(note).text_size(px(11.0)).text_color(rgb(0x888780))),
        )
}

/// 标调演示的一行：声母 + 韵母 → 带调音节，一个「帽子」从上方落到音节上。
fn tone_rule_row(
    initial: &'static str,
    final_: &'static str,
    result: &'static str,
    note: &'static str,
    speak_name: &'static str,
    speak_tone: u8,
    v: AnimatedValue,
) -> impl IntoElement {
    let hat_dy = v.interpolate(&[0.0, 1.0], &[-22.0, 0.0]);
    let hat_o = v.interpolate(&[0.0, 1.0], &[0.0, 1.0]);
    let ro = v.interpolate(&[0.0, 1.0], &[0.3, 1.0]);
    view()
        .flex_row()
        .items_center()
        .gap(px(7.0))
        .child(blend_block(initial, 0x2F7FB5, 0xF5FBFF, 0xC7E2F3))
        .child(text("＋").text_size(px(14.0)).text_color(rgb(0x9DABB6)))
        .child(blend_block(final_, 0x2FA37A, 0xF4FBF7, 0xC4E9D8))
        .child(text("→").text_size(px(14.0)).text_color(rgb(0x9DABB6)))
        .child(
            view()
                .w(px(84.0))
                .h(px(58.0))
                .items_center()
                .justify_center()
                .gap(px(2.0))
                .rounded(px(12.0))
                .border(px(2.0))
                .border_color(rgb(0xFFD0B9))
                .bg(rgb(0xFFF1E8))
                .bind_opacity(&ro)
                .on_press(move || speak(speak_name, speak_tone, 1.0))
                .child(
                    view()
                        .w(px(18.0))
                        .h(px(4.0))
                        .rounded(px(2.0))
                        .bg(rgb(0xE8703F))
                        .bind_translate_y(&hat_dy)
                        .bind_opacity(&hat_o),
                )
                .child(text(result).text_size(px(22.0)).text_color(rgb(0xE8703F))),
        )
        .child(
            view()
                .flex_1()
                .child(text(note).text_size(px(11.0)).text_color(rgb(0x888780))),
        )
}

/// 第六幕：标调规则——帽子落在谁头上。
fn anim_scene_tone_rule() -> impl IntoElement {
    let v = [
        animated_value(0.0),
        animated_value(0.0),
        animated_value(0.0),
    ];
    for (i, value) in v.iter().enumerate() {
        anim_to(*value, 0.0, 1.0, 420.0, i as f32 * 260.0);
    }
    view()
        .flex_1()
        .justify_center()
        .gap(px(14.0))
        .child(tone_rule_row(
            "m", "a", "mā", "有 a 就标在 a 上", "ma", 1, v[0],
        ))
        .child(tone_rule_row(
            "h", "ui", "huī", "iu 并列，标在后面", "hui", 1, v[1],
        ))
        .child(tone_rule_row(
            "n", "i", "nǐ", "标 i 上，点去掉", "ni", 3, v[2],
        ))
}

/// 第七幕：y / w 整体认读——小字母后退合体。
fn anim_scene_yw() -> impl IntoElement {
    let v = [
        animated_value(0.0),
        animated_value(0.0),
        animated_value(0.0),
    ];
    for (i, value) in v.iter().enumerate() {
        anim_to(*value, 0.0, 1.0, 460.0, i as f32 * 260.0);
    }
    view()
        .flex_1()
        .justify_center()
        .gap(px(14.0))
        .child(blend_line(
            "y",
            "i",
            "",
            "yi",
            "小 i 见 y，往后退",
            Some("yi"),
            v[0],
        ))
        .child(blend_line(
            "w",
            "u",
            "",
            "wu",
            "小 u 见 w 后退",
            Some("wu"),
            v[1],
        ))
        .child(blend_line(
            "y",
            "ü",
            "",
            "yu",
            "小 ü 见 y，摘帽",
            Some("yu"),
            v[2],
        ))
}

/// 第八幕：两拼与三拼——轻短、相碰、介母快速滑过。
fn anim_scene_blend() -> impl IntoElement {
    let v = [
        animated_value(0.0),
        animated_value(0.0),
        animated_value(0.0),
    ];
    for (i, value) in v.iter().enumerate() {
        anim_to(*value, 0.0, 1.0, 460.0, i as f32 * 260.0);
    }
    view()
        .flex_1()
        .justify_center()
        .gap(px(14.0))
        .child(blend_line(
            "b",
            "a",
            "",
            "ba",
            "前音轻短后音重",
            Some("ba"),
            v[0],
        ))
        .child(blend_line(
            "j",
            "i",
            "",
            "ji",
            "两音相碰猛一碰",
            Some("ji"),
            v[1],
        ))
        .child(blend_line(
            "j",
            "i",
            "a",
            "jia",
            "",
            None,
            v[2],
        ))
}

/// 第九幕辅助：一圈从鼻子向外扩散并淡出的气流波纹。
/// v: 0→1 时 scale 0.5→2.2、opacity 0.85→0（同心叠放，靠多个错位 value 造连续感）。
fn nasal_ripple(v: AnimatedValue, color: u32) -> impl IntoElement {
    let scale = v.interpolate(&[0.0, 1.0], &[0.5, 2.2]);
    let opacity = v.interpolate(&[0.0, 1.0], &[0.85, 0.0]);
    view()
        .absolute()
        .inset_0()
        .items_center()
        .justify_center()
        .child(
            view()
                .w(px(34.0))
                .h(px(34.0))
                .rounded(px(18.0))
                .border(px(3.0))
                .border_color(rgb(color))
                .bind_scale(&scale)
                .bind_opacity(&opacity),
        )
}

/// 第九幕辅助：一只从右侧移入「捏住鼻子」的手（用 emoji + 文案）。
/// v: 0→1 时手从右滑入到鼻子上。
fn nasal_hand(v: AnimatedValue, blocked: bool) -> impl IntoElement {
    let dx = v.interpolate(&[0.0, 1.0], &[70.0, 6.0]);
    let op = v.interpolate(&[0.0, 1.0], &[0.0, 1.0]);
    view()
        .absolute()
        .inset_0()
        .items_center()
        .justify_center()
        .bind_opacity(&op)
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(2.0))
                .bind_translate_x(&dx)
                .child(text("🤏").text_size(px(34.0)))
                .child(
                    text(if blocked { "捏住！气没了" } else { "捏住！照样响" })
                        .text_size(px(11.0))
                        .text_color(if blocked {
                            rgb(0xD83A5B)
                        } else {
                            rgb(0x2FA37A)
                        }),
                ),
        )
}

/// 第九幕一张鼻音/边音对比卡。
fn nasal_card(
    letter: &'static str,
    title: &'static str,
    hint: &'static str,
    example: &'static str,
    example_py: &'static str,
    color: u32,
    tint_bg: u32,
    waves: [AnimatedValue; 3],
    hand: AnimatedValue,
    blocked: bool,
    v: AnimatedValue,
) -> impl IntoElement {
    let enter = v.interpolate(&[0.0, 1.0], &[0.75, 1.0]);
    view()
        .flex_1()
        .h(px(228.0))
        .items_center()
        .justify_center()
        .gap(px(3.0))
        .rounded(px(18.0))
        .bg(rgb(tint_bg))
        .border(px(2.0))
        .border_color(rgb(color))
        .bind_opacity(&v)
        .bind_scale(&enter)
        .on_press(move || speak(letter, 0, 1.0))
        .child(text(title).text_size(px(15.0)).text_color(rgb(color)))
        // 鼻子示意图：固定舞台 + 同心波纹 + 手
        .child(
            view()
                .w(px(120.0))
                .h(px(96.0))
                .items_center()
                .justify_center()
                .child(nasal_ripple(waves[0], color))
                .child(nasal_ripple(waves[1], color))
                .child(nasal_ripple(waves[2], color))
                .child(text("👃").text_size(px(46.0)).z_index(2))
                .child(nasal_hand(hand, blocked)),
        )
        .child(
            text(letter)
                .text_size(px(34.0))
                .text_color(rgb(color))
                .on_press(move || speak(letter, 0, 1.0)),
        )
        .child(text(hint).text_size(px(11.0)).text_color(rgb(0x66636F)))
        .child(
            view()
                .on_press(move || speak(example_py, 0, 1.0))
                .child(
                    text(example)
                        .text_size(px(12.0))
                        .text_color(rgb(0x8A8794)),
                ),
        )
}

/// 第九幕：n / l 鼻音边音——「捏鼻子法」。
/// n 是鼻音，捏住鼻子气出不来（波纹被手挡住）；l 是边音，气走舌头两边，
/// 捏住鼻子照样响（波纹持续）。对应教研里 n/l 不分（约占 25%）的方言痛点。
fn anim_scene_nasal() -> impl IntoElement {
    // n 卡：三圈波纹先扩散（气走鼻子），随后手滑入捏住，波纹已散尽 → 发不出。
    let nw = [
        animated_value(0.0),
        animated_value(0.0),
        animated_value(0.0),
    ];
    anim_to(nw[0], 0.0, 1.0, 900.0, 120.0);
    anim_to(nw[1], 0.0, 1.0, 900.0, 460.0);
    anim_to(nw[2], 0.0, 1.0, 900.0, 800.0);
    let nh = animated_value(0.0);
    anim_to(nh, 0.0, 1.0, 500.0, 1700.0);
    let nv = animated_value(0.0);
    anim_to(nv, 0.0, 1.0, 380.0, 0.0);

    // l 卡：手捏住后，三圈波纹再依次扩散 → 照样响。
    let lw = [
        animated_value(0.0),
        animated_value(0.0),
        animated_value(0.0),
    ];
    anim_to(lw[0], 0.0, 1.0, 900.0, 1800.0);
    anim_to(lw[1], 0.0, 1.0, 900.0, 2140.0);
    anim_to(lw[2], 0.0, 1.0, 900.0, 2480.0);
    let lh = animated_value(0.0);
    anim_to(lh, 0.0, 1.0, 500.0, 1150.0);
    let lv = animated_value(0.0);
    anim_to(lv, 0.0, 1.0, 380.0, 160.0);

    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap(px(12.0))
        .child(nasal_card(
            "n",
            "n · 鼻音",
            "捏住鼻子，气出不来 → 发不出",
            "拿 ná · 泥 ní",
            "na",
            0x2F7FB5,
            0xF2F8FF,
            nw,
            nh,
            true,
            nv,
        ))
        .child(nasal_card(
            "l",
            "l · 边音",
            "气走舌头两边，捏鼻子照样响",
            "拉 lā · 梨 lí",
            "la",
            0x2FA37A,
            0xF1FAF5,
            lw,
            lh,
            false,
            lv,
        ))
}

/// 认读层：一张易混对大卡（左右并排 + 掌握度）。
/// 直接读 signals，切组/答题后自动刷新；点卡片发音并参与听音判定。
fn confusable_card(
    group: Signal<u8>,
    points: Signal<[u8; 12]>,
    target: Signal<u8>,
    answered: Signal<bool>,
    got_it: Signal<bool>,
    feedback: Signal<&'static str>,
) -> impl IntoElement {
    let tints = [0x3B7DE0u32, 0x1E9E63u32];
    view()
        .flex_1()
        .flex_row()
        .gap(px(10.0))
        .children(
            (0..2)
                .map(|i| {
                    let g = group;
                    let t_s = target;
                    let ans = answered;
                    let got = got_it;
                    let tint = tints[i];
                    let is_left = i == 0;
                    let letter = move || {
                        let (a, b) = CONFUSABLE_PAIRS[g.get() as usize];
                        if is_left { a } else { b }
                    };
                    let border = move || {
                        if !ans.get() {
                            return rgb(0xE6EDF3);
                        }
                        let correct_side = t_s.get() == i as u8;
                        if got.get() == correct_side {
                            rgb(0x2FA37A)
                        } else {
                            rgb(0xD83A5B)
                        }
                    };
                    let level_i = i;
                    let dots_g = group;
                    let dots_pts = points;
                    view()
                        .flex_1()
                        .h(px(210.0))
                        .items_center()
                        .justify_center()
                        .gap(px(6.0))
                        .rounded(px(18.0))
                        .bg(rgb(0xFFFFFF))
                        .border(px(2.5))
                        .border_color(border)
                        .active(|s| s.opacity(0.75))
                        .on_press({
                            let pg = group;
                            let pt = target;
                            let pa = answered;
                            let pp = points;
                            let pgo = got_it;
                            let pf = feedback;
                            let repl_t = target;
                            let repl_g = group;
                            move || {
                                let gi = pg.get() as usize;
                                let (a, b) = CONFUSABLE_PAIRS[gi];
                                let l = if is_left { a } else { b };
                                speak(l, 0, 1.0);
                                if pa.get() {
                                    return;
                                }
                                let side = if is_left { 0u8 } else { 1u8 };
                                let tgt = pt.get();
                                let ok = side == tgt;
                                pa.set(true);
                                pgo.set(ok);
                                let idx = gi * 2 + tgt as usize;
                                let mut p = pp.get();
                                let fb = pf;
                                if ok {
                                    p[idx] = (p[idx] + 1).min(2);
                                    fb.set("答对啦！真棒");
                                } else {
                                    p[idx] = 0;
                                    fb.set("再听一次，慢慢找");
                                    std::mem::forget(set_timeout(
                                        std::time::Duration::from_millis(520),
                                        move || {
                                            let (ra, rb) = CONFUSABLE_PAIRS[repl_g.get() as usize];
                                            let rl = if repl_t.get() == 0 { ra } else { rb };
                                            speak(rl, 0, 0.9);
                                        },
                                    ));
                                }
                                pp.set(p);
                                save_recog_points(&p);
                            }
                        })
                        .child(
                            text(move || {
                                let l = letter();
                                l.to_string()
                            })
                            .text_size(px(72.0))
                            .text_color(rgb(tint)),
                        )
                        .child(
                            text(move || {
                                let (a, b) = CONFUSABLE_PAIRS[dots_g.get() as usize];
                                let l = if level_i == 0 { a } else { b };
                                recog_note(l).to_string()
                            })
                            .text_size(px(11.0))
                            .text_color(rgb(0x888780)),
                        )
                        .child(
                            view()
                                .flex_row()
                                .gap(px(3.0))
                                .children(
                                    (0..3)
                                        .map(|d| {
                                            let dp = dots_pts;
                                            let dg = dots_g;
                                            let di = level_i;
                                            view()
                                                .w(px(7.0))
                                                .h(px(7.0))
                                                .rounded(px(4.0))
                                                .bg(move || {
                                                    let gi = dg.get() as usize;
                                                    let lvl = dp.get()[gi * 2 + di];
                                                    if (d as u8) < lvl {
                                                        rgb(tint)
                                                    } else {
                                                        rgb(0xE3E6EC)
                                                    }
                                                })
                                        })
                                        .collect::<Vec<_>>(),
                                ),
                        )
                })
                .collect::<Vec<_>>(),
        )
}

/// 认读页：6 组易混对并排认 + 听音辨字小测 + 掌握度持久化。
/// 顶部组切换胶囊；点卡片发音；「听一听」随机读一个，点对的卡即得分。
fn recognition_view() -> impl IntoElement {
    let group: Signal<u8> = signal(0);
    let points: Signal<[u8; 12]> = signal(load_recog_points());
    // 听音辨字：目标 0=左 1=右；answer 是本次要辨认的字母索引。
    let target: Signal<u8> = signal(0);
    let answered: Signal<bool> = signal(false);
    let got_it: Signal<bool> = signal(false);
    let feedback: Signal<&'static str> = signal("点「听一听」，再选出你听到的那个");

    let group_val = group;

    // 「听一听」：读当前目标字母。
    let play_group = group;
    let play_target = target;
    let play = move || {
        let (a, b) = CONFUSABLE_PAIRS[play_group.get() as usize];
        let letter = if play_target.get() == 0 { a } else { b };
        speak(letter, 0, 0.9);
    };

    // 「下一组」。
    let next_fb = feedback;
    let next_answered = answered;
    let next_target = target;
    let next_group = group;
    let do_next = move || {
        let g = next_group.get();
        let ng = (g + 1) % CONFUSABLE_PAIRS.len() as u8;
        next_group.set(ng);
        next_fb.set("点「听一听」，再选出你听到的那个");
        next_answered.set(false);
        next_target.set(if (ng as u32 * 2654435761) % 2 == 0 { 0 } else { 1 });
    };

    view()
        .flex_1()
        .gap(px(10.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(text("认一认 · 分得清").text_size(px(16.0)).text_color(rgb(0x2C2C2A)))
                .child(
                    text(move || {
                        let n = points.get().iter().filter(|&&v| v >= 2).count();
                        format!("已掌握 {n}/12")
                    })
                    .text_size(px(12.0))
                    .text_color(rgb(0x6C4FF5)),
                ),
        )
        .child(
            view()
                .flex_row()
                .gap(px(5.0))
                .children(
                    CONFUSABLE_PAIRS
                        .iter()
                        .enumerate()
                        .map(|(i, (x, y))| {
                            let gi = i as u8;
                            let s = group_val;
                            let cb_group = group;
                            let cb_target = target;
                            let cb_answered = answered;
                            let cb_feedback = feedback;
                            let cb = move |_: u8| {
                                cb_group.set(gi);
                                cb_target.set(if (gi as u32 * 2654435761) % 2 == 0 { 0 } else { 1 });
                                cb_answered.set(false);
                                cb_feedback.set("点「听一听」，再选出你听到的那个");
                            };
                            view()
                                .flex_1()
                                .py(px(7.0))
                                .rounded(px(10.0))
                                .bg(move || {
                                    if s.get() == gi {
                                        rgb(0x6C4FF5)
                                    } else {
                                        rgb(0xF3F1FB)
                                    }
                                })
                                .text_color(move || {
                                    if s.get() == gi {
                                        rgb(0xFFFFFF)
                                    } else {
                                        rgb(0x6C4FF5)
                                    }
                                })
                                .active(|st| st.opacity(0.75))
                                .on_press(move || cb(gi))
                                .child(
                                    text(format!("{x}/{y}"))
                                        .text_size(px(12.0))
                                        .text_color(move || {
                                            if s.get() == gi {
                                                rgb(0xFFFFFF)
                                            } else {
                                                rgb(0x6C4FF5)
                                            }
                                        }),
                                )
                        })
                        .collect::<Vec<_>>(),
                ),
        )
        .child(confusable_card(group, points, target, answered, got_it, feedback))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    view()
                        .px(px(16.0))
                        .py(px(11.0))
                        .rounded(px(12.0))
                        .bg(rgb(0x6C4FF5))
                        .active(|st| st.opacity(0.75))
                        .on_press(play)
                        .child(text("听一听").text_size(px(14.0)).text_color(rgb(0xFFFFFF))),
                )
                .child(text(move || feedback.get().to_string()).flex_1().text_size(px(12.0)).text_color(rgb(0x888780)))
                .child(
                    when(move || answered.get(), move || {
                        view()
                            .px(px(14.0))
                            .py(px(11.0))
                            .rounded(px(12.0))
                            .bg(rgb(0xE4F6EC))
                            .active(|st| st.opacity(0.75))
                            .on_press(do_next)
                            .child(text("下一组 →").text_size(px(13.0)).text_color(rgb(0x1E9E63)))
                    }),
                ),
        )
}

/// 动画课堂：八幕，nsh 原生 Animated（UI 线程逐帧，不经过引擎重绘）。
fn animation_view() -> impl IntoElement {
    let scene: Signal<u8> = signal(0);
    let verse = [
        "一声 ā，二声 á，三声 ǎ，四声 à",
        "左手 b，右手 d；向下 p，向下 q",
        "小 ü 见 j q x 擦泪，见 n l 两点不去",
        "j q x 见 a 补 i；g k h 见 o 补 u",
        "小 i 在前是 ie，在后是 ei；i 前 iu，u 前 ui",
        "有 a 就标 a，没 a 找 o e；iu 并列标在后",
        "小 i 见大 y 往后退；小 ü 见大 y 摘掉帽",
        "前音轻短后音重；声轻介快韵母响",
        "n 气走鼻子捏住闷；l 走舌头两边照样响",
        "认一认 · 分得清，听音辨字记得牢",
    ];
    let tabs = [
        "四声", "左右手", "ü 点", "介母", "复韵母", "标调", "y w", "拼读", "n / l", "认读",
    ];
    let tab = move |i: usize| {
        let s = scene;
        let value = i as u8;
        let label = tabs[i];
        view()
            .px(px(13.0))
            .py(px(9.0))
            .rounded(px(13.0))
            .bg(move || {
                if s.get() == value {
                    rgb(0x6C4FF5)
                } else {
                    rgb(0xF3F1FB)
                }
            })
            .text_color(move || {
                if s.get() == value {
                    rgb(0xFFFFFF)
                } else {
                    rgb(0x6C4FF5)
                }
            })
            .on_press(move || s.set(value))
            .child(text(label).text_size(px(14.0)))
    };
    view()
        .flex_1()
        .gap(px(10.0))
        .child(
            view()
                .gap(px(2.0))
                .p(px(3.0))
                .rounded(px(14.0))
                .bg(rgb(0xF3F1FB))
                .child(
                    view()
                        .flex_row()
                        .gap(px(2.0))
                        .child(tab(0))
                        .child(tab(1))
                        .child(tab(2))
                        .child(tab(3))
                        .child(tab(4)),
                )
                .child(
                    view()
                        .flex_row()
                        .gap(px(2.0))
                        .child(tab(5))
                        .child(tab(6))
                        .child(tab(7))
                        .child(tab(8))
                        .child(tab(9)),
                ),
        )
        // 第 10 幕「认读」用独立整页布局（自带卡片），不套白色动画容器。
        .child(when(move || scene.get() == 9, recognition_view).otherwise(move || {
            view()
                .flex_1()
                .gap(px(10.0))
                .child(
                    view()
                        .flex_1()
                        .p(px(14.0))
                        .rounded(px(18.0))
                        .bg(rgb(0xFFFFFF))
                        .shadow_sm()
                        .justify_center()
                        .child(
                            when(move || scene.get() == 0, move || anim_scene_tones())
                                .otherwise(move || {
                                    when(move || scene.get() == 1, move || anim_scene_hands())
                                        .otherwise(move || {
                                            when(move || scene.get() == 2, move || {
                                                anim_scene_umlaut()
                                            })
                                            .otherwise(move || {
                                                when(move || scene.get() == 3, move || {
                                                    anim_scene_medial()
                                                })
                                                .otherwise(move || {
                                                    when(move || scene.get() == 4, move || {
                                                        anim_scene_pairs()
                                                    })
                                                    .otherwise(move || {
                                                        when(move || scene.get() == 5, move || {
                                                            anim_scene_tone_rule()
                                                        })
                                                        .otherwise(move || {
                                                            when(move || scene.get() == 6, move || {
                                                                anim_scene_yw()
                                                            })
                                                            .otherwise(move || {
                                                                when(
                                                                    move || scene.get() == 7,
                                                                    move || anim_scene_blend(),
                                                                )
                                                                .otherwise(move || {
                                                                    anim_scene_nasal()
                                                                })
                                                            })
                                                        })
                                                    })
                                                })
                                            })
                                        })
                                }),
                        ),
                )
                .child(
                    text(move || verse[scene.get() as usize])
                        .text_size(px(12.0))
                        .text_color(rgb(0x6750F5)),
                )
        }))
}

/// 拼音知识：规则要点 + 23 个声母顺口溜。
fn knowledge_view(
    initial: Signal<&'static str>,
    medial: Signal<&'static str>,
    final_: Signal<&'static str>,
    tone: Signal<u8>,
    rec_state: Signal<RecTest>,
) -> impl IntoElement {
    view().flex_1().child(
        scroll()
            .flex_1()
            .gap(px(14.0))
            .pb(px(8.0))
            .child(
                text("拼音小知识")
                    .text_size(px(18.0))
                    .text_color(rgb(0x2C2C2A)),
            )
            .child(rec_test_card(initial, medial, final_, tone, rec_state))
            .children(
                RULES
                    .iter()
                    .map(|&(title, body, examples)| {
                        view()
                            .gap(px(5.0))
                            .p(px(12.0))
                            .rounded(px(12.0))
                            .bg(rgb(0xFFFFFF))
                            .child(text(title).text_size(px(15.0)).text_color(rgb(0x6750F5)))
                            .child(text(body).text_size(px(13.0)).text_color(rgb(0x5F5E5A)))
                            .child(
                                view()
                                    .flex_row()
                                    .flex_wrap()
                                    .items_center()
                                    .gap(px(6.0))
                                    .mt(px(2.0))
                                    .child(text("例").text_size(px(11.0)).text_color(rgb(0x888780)))
                                    .children(
                                        examples
                                            .iter()
                                            .map(|&(label, py, t)| {
                                                view()
                                                    .px(px(9.0))
                                                    .py(px(5.0))
                                                    .rounded(px(6.0))
                                                    .bg(if py.is_empty() {
                                                        rgb(0xF5F5F4)
                                                    } else {
                                                        rgb(0xF1EFFE)
                                                    })
                                                    .text_color(if py.is_empty() {
                                                        rgb(0x5F5E5A)
                                                    } else {
                                                        rgb(0x5B47D6)
                                                    })
                                                    .on_press(move || {
                                                        if !py.is_empty() {
                                                            speak(py, t, 1.0_f32);
                                                        }
                                                    })
                                                    .child(text(label).text_size(px(12.0)))
                                            })
                                            .collect::<Vec<_>>(),
                                    ),
                            )
                    })
                    .collect::<Vec<_>>(),
            )
            .child(
                text("声母顺口溜")
                    .text_size(px(18.0))
                    .text_color(rgb(0x2C2C2A))
                    .mt(px(6.0)),
            )
            .children(
                crate::data::mnemonics::MNEMONICS
                    .iter()
                    .map(|&(key, text_)| {
                        view()
                            .flex_row()
                            .gap(px(10.0))
                            .p(px(10.0))
                            .rounded(px(10.0))
                            .bg(rgb(0xFFFFFF))
                            .child(
                                view()
                                    .w(px(40.0))
                                    .h(px(40.0))
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(rgb(0xEEEDFE))
                                    .child(text(key).text_size(px(16.0)).text_color(rgb(0x6750F5))),
                            )
                            .child(
                                view().flex_1().child(
                                    text(text_).text_size(px(13.0)).text_color(rgb(0x2C2C2A)),
                                ),
                            )
                    })
                    .collect::<Vec<_>>(),
            ),
    )
}

// ---------------------------------------------------------------- 跟读评测
//
// 链路：按一下 → 后台录 2 秒（AAudio）→ UI 定时轮询取 PCM
//      → audio_tone 纯算法判声调 → 和目标声调比对给出鼓励。
// 不联网、不认字、不上大模型，全部端侧。

/// 跟读评测的一次性状态。
#[derive(Clone, PartialEq)]
enum RecTest {
    Idle,
    Recording,
    Done {
        heard: &'static str,
        target: &'static str,
        ok: bool,
        confidence: f32,
        advice: &'static str,
    },
    Failed(String),
}

fn tone_index(t: audio_tone::Tone) -> u8 {
    match t {
        audio_tone::Tone::One => 1,
        audio_tone::Tone::Two => 2,
        audio_tone::Tone::Three => 3,
        audio_tone::Tone::Four => 4,
    }
}

/// 轮询麦克风结果：约 `tries × 150ms` 内取到 PCM 就做声调分析。
fn rec_test_poll(state: Signal<RecTest>, target: u8, tries: u32) {
    if let Some(pcm) = mic::take() {
        let r = audio_tone::analyze_tone(&pcm, mic::SAMPLE_RATE);
        if r.confidence <= 0.0 {
            state.set(RecTest::Failed("没有听到声音，靠近一点再读一次".to_owned()));
            return;
        }
        let heard = tone_index(r.tone);
        let confident = r.confidence >= 0.35;
        let ok = confident && target <= 4 && heard == target;
        let advice = if target > 4 {
            "轻声没有调型，换一、二、三、四声试试"
        } else if ok {
            "读得准！保持这个调子"
        } else if !confident {
            "没听清，再大声一点、读长一点"
        } else {
            "声调不太对，点「听示范」再听一遍"
        };
        state.set(RecTest::Done {
            heard: TONE_NAMES[(heard - 1) as usize],
            target: TONE_NAMES[(target.clamp(1, 4) - 1) as usize],
            ok,
            confidence: r.confidence,
            advice,
        });
        return;
    }
    if let Some(err) = mic::take_error() {
        state.set(RecTest::Failed(err.message().to_owned()));
        return;
    }
    if tries == 0 {
        state.set(RecTest::Failed("录音超时，再试一次".to_owned()));
        return;
    }
    std::mem::forget(set_timeout(
        std::time::Duration::from_millis(150),
        move || rec_test_poll(state, target, tries - 1),
    ));
}

/// 目标音节文本，如 `jia · 三声`。
fn rec_target_label(initial: &str, medial: &str, final_: &str, tone: u8) -> String {
    if initial.is_empty() {
        return "先在上面选声母".to_owned();
    }
    let t = TONE_NAMES[(tone.clamp(1, 5) - 1) as usize];
    if medial.is_empty() {
        format!("{initial} + {final_} · {t}")
    } else {
        format!("{initial} + {medial} + {final_} · {t}")
    }
}

fn rec_test_card(
    initial: Signal<&'static str>,
    medial: Signal<&'static str>,
    final_: Signal<&'static str>,
    tone: Signal<u8>,
    state: Signal<RecTest>,
) -> impl IntoElement {
    view()
        .gap(px(8.0))
        .p(px(12.0))
        .rounded(px(12.0))
        .bg(rgb(0xFFFFFF))
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(text("跟读评测").text_size(px(15.0)).text_color(rgb(0x6750F5)))
                .child(
                    text(move || {
                        rec_target_label(
                            initial.get(),
                            medial.get(),
                            final_.get(),
                            tone.get(),
                        )
                    })
                    .text_size(px(12.0))
                    .text_color(rgb(0x5B47D6)),
                ),
        )
        .child(
            text("点一下，跟着读一遍，看看声调对不对（只用声调轮廓判断，不识别字）。")
                .text_size(px(11.0))
                .text_color(rgb(0x888780)),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    view()
                        .px(px(14.0))
                        .py(px(9.0))
                        .rounded(px(10.0))
                        .bg(move || {
                            if mic::is_recording() {
                                rgb(0xD9D5EC)
                            } else {
                                rgb(0x6750F5)
                            }
                        })
                        .on_press(move || {
                            if initial.get().is_empty() {
                                state.set(RecTest::Failed("先在上面选好声母和韵母".to_owned()));
                                return;
                            }
                            if mic::is_recording() {
                                return;
                            }
                            if mic::start(2000) {
                                state.set(RecTest::Recording);
                                rec_test_poll(state, tone.get(), 24);
                            } else {
                                state.set(RecTest::Failed("麦克风忙，稍后再试".to_owned()));
                            }
                        })
                        .child(
                            text(move || {
                                if mic::is_recording() {
                                    "录音中…"
                                } else {
                                    "录 2 秒"
                                }
                            })
                            .text_size(px(13.0))
                            .text_color(rgb(0xFFFFFF)),
                        ),
                )
                .child(
                    view()
                        .px(px(12.0))
                        .py(px(9.0))
                        .rounded(px(10.0))
                        .bg(rgb(0xF1EFFE))
                        .on_press(move || {
                            let i = initial.get();
                            let m = medial.get();
                            let f = final_.get();
                            if !i.is_empty() && !m.is_empty() {
                                // 三拼：有整音读整音，否则分解连读。
                                let whole = format!("{i}{m}{f}");
                                let wanted = if tone.get() == 0 { 1 } else { tone.get() };
                                if baidu::baidu_mask(&whole) & (1 << (wanted - 1)) != 0 {
                                    speak(&whole, tone.get(), 1.0);
                                } else {
                                    speak_three(i, m, f);
                                }
                            } else if !i.is_empty() {
                                let s = format!("{i}{f}");
                                speak(&s, tone.get(), 1.0);
                            }
                        })
                        .child(text("听示范").text_size(px(13.0)).text_color(rgb(0x5B47D6))),
                ),
        )
        .child(rec_test_result(state))
}

fn rec_test_result(state: Signal<RecTest>) -> impl IntoElement {
    when(
        move || !matches!(state.get(), RecTest::Idle),
        move || {
            view()
                .gap(px(3.0))
                .p(px(10.0))
                .rounded(px(10.0))
                .bg(move || match state.get() {
                    RecTest::Done { ok: true, .. } => rgb(0xE4F6EC),
                    RecTest::Failed(_) => rgb(0xFDE9EE),
                    _ => rgb(0xF3F1FB),
                })
                .child(
                    text(move || match state.get() {
                        RecTest::Idle => String::new(),
                        RecTest::Recording => "正在听…请把整个音节读完".to_owned(),
                        RecTest::Failed(e) => e,
                        RecTest::Done { heard, target, ok, .. } => {
                            if ok {
                                format!("通过！听到「{heard}」，和选的「{target}」一致")
                            } else {
                                format!("再试试：听到「{heard}」，你选的是「{target}」")
                            }
                        }
                    })
                    .text_size(px(13.0))
                    .text_color(move || match state.get() {
                        RecTest::Done { ok: true, .. } => rgb(0x1E9E63),
                        RecTest::Failed(_) => rgb(0xD83A5B),
                        _ => rgb(0x5F5E5A),
                    }),
                )
                .child(
                    when(
                        move || matches!(state.get(), RecTest::Done { .. }),
                        move || {
                            text(move || match state.get() {
                                RecTest::Done { advice, confidence, .. } => {
                                    format!("{advice}（把握 {:.0}%）", confidence * 100.0)
                                }
                                _ => String::new(),
                            })
                            .text_size(px(11.0))
                            .text_color(rgb(0x888780))
                        },
                    )
                    .otherwise(view),
                )
        },
    )
    .otherwise(view)
}

fn syllable_table_view(tone: Signal<u8>) -> impl IntoElement {
    view()
        .flex_1()
        .gap(px(12.0))
        .child(section_title("音节表", "共 343 个合法音节，点按可听"))
        .child(
            scroll().flex_1().child(
                view().flex_row().flex_wrap().gap(px(8.0)).children(
                    SYLLABLES
                        .iter()
                        .map(|&s| {
                            let name = s.to_owned();
                            let tn = tone;
                            view()
                                .px(px(12.0))
                                .py(px(8.0))
                                .rounded(px(8.0))
                                .bg(rgb(0xFFFFFF))
                                .on_press(move || speak(&name, tn.get(), 1.0_f32))
                                .child(text(s).text_size(px(14.0)))
                        })
                        .collect::<Vec<_>>(),
                ),
            ),
        )
}
