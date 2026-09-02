use gpui::{App, PromptLevel};

const ABOUT_TEXT: &str = "shawty got them apple bottom jeans (jeans) boots with the fur (with the fur) the whole club lookin at her (at her) she hit the floo (she hit the floo) next thing you know shawty got low low low low low low low low";

pub fn show(cx: &mut App) {
    let Some(window) = cx.active_window() else {
        return;
    };
    cx.defer(move |cx| {
        window
            .update(cx, |_, window, cx| {
                let answer = window.prompt(
                    PromptLevel::Info,
                    "Bluesky Gif Unfucker v0.1.0",
                    Some(ABOUT_TEXT),
                    &["OK"],
                    cx,
                );
                cx.spawn(async move |_| {
                    answer.await.ok();
                })
                .detach();
            })
            .ok();
    });
}
