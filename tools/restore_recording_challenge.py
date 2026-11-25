"""把 lib.rs 里误改成「选卡拼读」的 recognition_challenge 恢复为录音跟读闯关。

只做这一件事：替换 2337..2634 那段（从「拼读闯关状态」注释到 recognition_view 之前）。
其余代码（含 spell_challenge 模块、其它页面）一律不动。
"""
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
p = ROOT / "src" / "lib.rs"
s = p.read_text(encoding="utf-8")

start_marker = "/// 拼读闯关状态：目标题号 + 已选声母/韵母 + 判定阶段。"
end_marker = "fn recognition_view() -> impl IntoElement {"
a = s.index(start_marker)
b = s.index(end_marker)

NEW = r'''/// 认读页：6 组易混对并排认 + 听音辨字小测 + 掌握度持久化。
/// 顶部组切换胶囊；点卡片发音；「听一听」随机读一个，点对的卡即得分。
/// 跟读闯关状态：目标字母索引 + 录音/判定阶段 + 分数。
#[derive(Clone, Copy, PartialEq)]
enum ChallengePhase {
    Ready,      // 显示字母，等孩子点录音
    Recording,  // 正在录 / 分析中
    Passed,     // 读对了，打勾动画中
    Failed,     // 没读准，可重来
}

/// 跟读闯关轮询：录音线程取 PCM → 和当前字母模板比对 → 判定。
/// `on_pass` 在读对时触发打勾动画 + 自动前进。
fn challenge_poll<F>(
    idx: usize,
    phase: Signal<ChallengePhase>,
    score: Signal<f32>,
    tries: u32,
    on_pass: F,
) where
    F: Fn() + Copy + 'static,
{
    if let Some(pcm) = mic::take() {
        let assessment = letter_match::assess(&pcm, mic::SAMPLE_RATE, idx);
        score.set(assessment.overall);
        match assessment.verdict {
            letter_match::Verdict::Pass { .. } => {
                phase.set(ChallengePhase::Passed);
                on_pass();
            }
            letter_match::Verdict::Retry { .. } => {
                phase.set(ChallengePhase::Failed);
            }
            letter_match::Verdict::TooQuiet => {
                score.set(0.0);
                phase.set(ChallengePhase::Failed);
            }
        }
        return;
    }
    if mic::take_error().is_some() {
        score.set(0.0);
        phase.set(ChallengePhase::Failed);
        return;
    }
    if tries == 0 {
        phase.set(ChallengePhase::Failed);
        return;
    }
    std::mem::forget(set_timeout(
        std::time::Duration::from_millis(120),
        move || challenge_poll(idx, phase, score, tries - 1, on_pass),
    ));
}

/// 全部字母跟读闯关：依次显示 47 个字母（声母+韵母），自动播标准音，
/// 孩子点「我读好了」录音，识别通过就打勾、动画后自动转下一个。
fn recognition_challenge() -> impl IntoElement {
    use crate::data::letter_templates::{
        LETTERS, LETTER_SYLLABLES, LETTER_TONES,
    };

    let idx: Signal<usize> = signal(0);
    let phase: Signal<ChallengePhase> = signal(ChallengePhase::Ready);
    let score: Signal<f32> = signal(0.0);
    let total = LETTERS.len();

    // 打勾动画值：一次性的缩放 + 透明度。
    let check_scale = animated_value(0.4);
    let check_alpha = animated_value(0.0);
    let check_scale_echo = check_scale;
    let check_alpha_echo = check_alpha;

    // 进入某个字母：播标准音 + 复位到 Ready。
    let speak_letter = move |i: usize| {
        speak(LETTER_SYLLABLES[i], LETTER_TONES[i], 1.0);
    };

    // 触发打勾动画，并在动画后自动前进。
    let anim_idx = idx;
    let anim_phase = phase;
    let do_pass = move || {
        anim_to(check_scale, 0.4, 1.15, 260.0, 0.0);
        anim_to(check_alpha, 0.0, 1.0, 200.0, 0.0);
        let ai = anim_idx;
        let ap = anim_phase;
        std::mem::forget(set_timeout(
            std::time::Duration::from_millis(1150),
            move || {
                let next = (ai.get() + 1) % LETTERS.len();
                ai.set(next);
                ap.set(ChallengePhase::Ready);
                speak(LETTER_SYLLABLES[next], LETTER_TONES[next], 1.0);
            },
        ));
    };

    view()
        .flex_1()
        .gap(px(11.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(text("跟读闯关 · 全部字母").text_size(px(16.0)).text_color(rgb(0x2C2C2A)))
                .child(
                    text(move || format!("{}/{}", idx.get() + 1, total))
                        .text_size(px(13.0))
                        .text_color(rgb(0x6C4FF5)),
                ),
        )
        // 进度条：47 个小格，已过关的亮色（响应式 bg，避免动态宽度）。
        .child(
            view()
                .w_full()
                .h(px(9.0))
                .flex_row()
                .gap(px(1.5))
                .children(
                    (0..total).map(|i| {
                        let ip = idx;
                        view()
                            .flex_1()
                            .h(px(9.0))
                            .rounded(px(2.0))
                            .bg(move || {
                                if ip.get() >= i { rgb(0x6C4FF5) } else { rgb(0xEDEAF8) }
                            })
                    })
                    .collect::<Vec<_>>(),
                ),
        )
        // 当前字母大卡 + 打勾动画覆盖
        .child(
            view()
                .flex_1()
                .w_full()
                .rounded(px(18.0))
                .bg(rgb(0xFFFFFF))
                .border(px(2.5))
                .border_color(move || {
                    if phase.get() == ChallengePhase::Passed {
                        rgb(0x2FA37A)
                    } else if phase.get() == ChallengePhase::Failed {
                        rgb(0xF0B6C2)
                    } else {
                        rgb(0xE6EDF3)
                    }
                })
                .items_center()
                .justify_center()
                .gap(px(4.0))
                .child(
                    text(move || LETTERS[idx.get()])
                        .text_size(px(96.0))
                        .text_color(move || {
                            if phase.get() == ChallengePhase::Passed {
                                rgb(0x2FA37A)
                            } else {
                                rgb(0x5B47D6)
                            }
                        }),
                )
                .child(
                    text("听标准音 · 跟读一遍")
                        .text_size(px(12.0))
                        .text_color(rgb(0x888780)),
                )
                // 打勾动画：对勾 ✓ 从 0.4 放大并淡入
                .child(
                    view()
                        .bind_opacity(&check_alpha_echo)
                        .bind_scale(&check_scale_echo)
                        .child(text("✓").text_size(px(72.0)).text_color(rgb(0x2FA37A))),
                ),
        )
        // 操作按钮 + 反馈
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    view()
                        .px(px(13.0))
                        .py(px(11.0))
                        .rounded(px(12.0))
                        .bg(rgb(0xEDEAF8))
                        .active(|s| s.opacity(0.75))
                        .on_press({
                            let sp = speak_letter;
                            move || sp(idx.get())
                        })
                        .child(text("🔊 听标准音").text_size(px(13.0)).text_color(rgb(0x5B47D6))),
                )
                .child(
                    view()
                        .flex_1()
                        .px(px(16.0))
                        .py(px(11.0))
                        .rounded(px(12.0))
                        .bg(move || {
                            if mic::is_recording() {
                                rgb(0xD9D5EC)
                            } else {
                                rgb(0x6C4FF5)
                            }
                        })
                        .active(|s| s.opacity(0.75))
                        .on_press({
                            let pi = idx;
                            let pp = phase;
                            let ps = score;
                            let panim = do_pass;
                            move || {
                                if mic::is_recording() {
                                    return;
                                }
                                let p = pp.get();
                                if p == ChallengePhase::Passed {
                                    return;
                                }
                                let i = pi.get();
                                if mic::start(1600) {
                                    ps.set(0.0);
                                    pp.set(ChallengePhase::Recording);
                                    challenge_poll(i, pp, ps, 40, panim);
                                } else {
                                    pp.set(ChallengePhase::Failed);
                                }
                            }
                        })
                        .child(
                            text(move || match phase.get() {
                                ChallengePhase::Recording => "录音中…",
                                _ => "🎤 我读好了",
                            })
                            .text_size(px(14.0))
                            .text_color(rgb(0xFFFFFF)),
                        ),
                ),
        )
        // 反馈文字
        .child(
            text(move || match phase.get() {
                ChallengePhase::Ready => "先听标准音，再点「我读好了」跟着读一遍".to_owned(),
                ChallengePhase::Recording => "正在听，请把这个字母读出来～".to_owned(),
                ChallengePhase::Passed => {
                    format!("读对啦！({:.0}%) 马上进入下一个 ✓", score.get() * 100.0)
                }
                ChallengePhase::Failed => {
                    if score.get() > 0.0 {
                        format!("还不太像（{:.0}%），再听一遍试试", score.get() * 100.0)
                    } else {
                        "没听清，靠近一点再读一次".to_owned()
                    }
                }
            })
            .text_size(px(12.0))
            .text_color(move || {
                if phase.get() == ChallengePhase::Passed {
                    rgb(0x1E9E63)
                } else if phase.get() == ChallengePhase::Failed {
                    rgb(0xD83A5B)
                } else {
                    rgb(0x888780)
                }
            }),
        )
}
'''

s = s[:a] + NEW + "\n" + s[b:]
p.write_text(s, encoding="utf-8")
print("restored recording challenge; removed", b - a, "chars of card-picking code")
