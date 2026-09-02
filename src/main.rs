mod about;
mod gif_unfuck;

use std::{path::PathBuf, time::Duration};

use std::{cell::RefCell, rc::Rc, sync::Arc};

use gpui::{
    App, AppContext as _, Application, AsyncApp, Bounds, Context, ExternalPaths,
    InteractiveElement, IntoElement, KeyBinding, Menu, MenuItem, ObjectFit, ParentElement,
    PathPromptOptions, Render, RenderImage, SharedString, StatefulInteractiveElement, Styled,
    StyledImage, TitlebarOptions, WeakEntity, Window, WindowBounds, WindowOptions, actions, div,
    img, prelude::FluentBuilder, px, rgb, rgba, size,
};

use gif_unfuck::Unfucked;

actions!(bsky_gif_unfucker, [Open, Quit, ShowAbout]);

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

enum GifState {
    Empty,
    Loading,
    Loaded(Loaded),
    Error(String),
}

struct Loaded {
    source: PathBuf,
    preview: Arc<RenderImage>,
    original_secs: f64,
    unfucked: Option<Unfucked>,
}

struct GifUnfucker {
    state: GifState,
    spinner_frame: usize,
    retired_previews: Vec<Arc<RenderImage>>,
}

impl GifUnfucker {
    fn new() -> Self {
        Self {
            state: GifState::Empty,
            spinner_frame: 0,
            retired_previews: Vec::new(),
        }
    }

    fn retire_preview(&mut self) {
        if let GifState::Loaded(loaded) = &self.state {
            self.retired_previews.push(loaded.preview.clone());
        }
    }

    fn load_gif(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("gif"))
        {
            self.state = GifState::Error("That's not a .gif file.".into());
            cx.notify();
            return;
        }

        self.retire_preview();
        self.state = GifState::Loading;
        self.spinner_frame = 0;
        cx.notify();

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(80))
                    .await;
                let still_loading = this.update(cx, |this, cx| {
                    if matches!(this.state, GifState::Loading) {
                        this.spinner_frame += 1;
                        cx.notify();
                        true
                    } else {
                        false
                    }
                });
                if !still_loading.unwrap_or(false) {
                    break;
                }
            }
        })
        .detach();

        let work = cx.background_executor().spawn({
            let path = path.clone();
            async move {
                let processed = gif_unfuck::process(&path)?;
                let preview_path = processed
                    .unfucked
                    .as_ref()
                    .map_or(path.as_path(), |unfucked| unfucked.path.as_path());
                let preview = gif_unfuck::decode_preview(preview_path)?;
                anyhow::Ok((processed, preview))
            }
        });
        cx.spawn(async move |this, cx| {
            let result = work.await;
            this.update(cx, |this, cx| {
                this.state = match result {
                    Ok((processed, preview)) => GifState::Loaded(Loaded {
                        preview: Arc::new(preview),
                        source: path,
                        original_secs: processed.original_secs,
                        unfucked: processed.unfucked,
                    }),
                    Err(error) => GifState::Error(format!("Couldn't read that gif: {error}")),
                };
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn save_unfucked(&mut self, cx: &mut Context<Self>) {
        let GifState::Loaded(loaded) = &self.state else {
            return;
        };
        let Some(unfucked) = &loaded.unfucked else {
            return;
        };

        let directory = loaded
            .source
            .parent()
            .map_or_else(std::env::temp_dir, PathBuf::from);
        let stem = loaded
            .source
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("gif");
        let answer = cx.prompt_for_new_path(&directory, Some(&format!("{stem}-unfucked.gif")));
        let temp_path = unfucked.path.clone();
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(destination))) = answer.await
                && let Err(error) = std::fs::copy(&temp_path, &destination)
            {
                this.update(cx, |this, cx| {
                    this.state = GifState::Error(format!("Couldn't save the gif: {error}"));
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn drop_zone(&self) -> gpui::Div {
        div()
            .size(px(300.))
            .rounded_xl()
            .border_2()
            .border_dashed()
            .border_color(rgb(0x4a4a5a))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
    }

    fn open_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("open")
            .px_3()
            .py_1()
            .rounded_md()
            .bg(rgb(0x2f2f3d))
            .hover(|style| style.bg(rgb(0x3a3a4a)))
            .cursor_pointer()
            .on_click(cx.listener(|_, _, _, cx| {
                let view = cx.weak_entity();
                prompt_open(view, cx);
            }))
            .child("Open...")
    }

    fn render_empty(&self, error: Option<&str>, cx: &mut Context<Self>) -> gpui::AnyElement {
        self.drop_zone()
            .when_some(error, |zone, error| {
                zone.child(
                    div()
                        .max_w(px(260.))
                        .text_sm()
                        .text_center()
                        .text_color(rgb(0xf28b82))
                        .child(SharedString::from(error.to_string())),
                )
            })
            .child(div().text_sm().child("Drag a .gif here, or"))
            .child(self.open_button(cx))
            .into_any_element()
    }

    fn render_loading(&self) -> gpui::AnyElement {
        self.drop_zone()
            .child(
                div()
                    .text_xl()
                    .text_color(rgb(0x9a9ab0))
                    .child(SPINNER_FRAMES[self.spinner_frame % SPINNER_FRAMES.len()]),
            )
            .into_any_element()
    }

    fn render_loaded(&self, loaded: &Loaded, cx: &mut Context<Self>) -> gpui::AnyElement {
        let duration_label = match &loaded.unfucked {
            Some(unfucked) => format!(
                "{} → {}",
                fmt_secs(loaded.original_secs),
                fmt_secs(unfucked.secs)
            ),
            None => fmt_secs(loaded.original_secs),
        };

        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            .child(
                div()
                    .relative()
                    .size(px(300.))
                    .rounded_xl()
                    .overflow_hidden()
                    .bg(rgb(0x0c0c10))
                    .child(
                        img(loaded.preview.clone())
                            .id("gif-preview")
                            .size_full()
                            .object_fit(ObjectFit::Contain),
                    )
                    .child(
                        div()
                            .id("clear")
                            .absolute()
                            .top_2()
                            .right_2()
                            .size(px(22.))
                            .rounded_full()
                            .bg(rgba(0x000000cc))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_xs()
                            .text_color(rgb(0xffffff))
                            .cursor_pointer()
                            .hover(|style| style.bg(rgba(0x333333cc)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.retire_preview();
                                this.state = GifState::Empty;
                                cx.notify();
                            }))
                            .child("✕"),
                    )
                    .child(
                        div()
                            .absolute()
                            .bottom_2()
                            .right_2()
                            .px_2()
                            .py_1()
                            .rounded_full()
                            .bg(rgb(0x000000))
                            .text_xs()
                            .text_color(rgb(0xffffff))
                            .child(SharedString::from(duration_label)),
                    ),
            )
            .child(if loaded.unfucked.is_some() {
                div()
                    .id("save")
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .bg(rgb(0x2f2f3d))
                    .hover(|style| style.bg(rgb(0x3a3a4a)))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.save_unfucked(cx)))
                    .child("Save unfucked gif")
                    .into_any_element()
            } else {
                div()
                    .max_w(px(300.))
                    .text_sm()
                    .text_center()
                    .text_color(rgb(0x9a9ab0))
                    .child("Your original gif is long enough and doesn't need unfucking.")
                    .into_any_element()
            })
            .into_any_element()
    }
}

impl Render for GifUnfucker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        for preview in self.retired_previews.drain(..) {
            window.drop_image(preview).ok();
        }
        div()
            .id("root")
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .p_4()
            .bg(rgb(0x16161c))
            .text_color(rgb(0xe8e8ef))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                if matches!(this.state, GifState::Loading) {
                    return;
                }
                if let Some(path) = paths.paths().first() {
                    this.load_gif(path.clone(), cx);
                }
            }))
            .drag_over::<ExternalPaths>(|style, _, _, _| style.bg(rgb(0x20202c)))
            .child(match &self.state {
                GifState::Empty => self.render_empty(None, cx),
                GifState::Error(error) => self.render_empty(Some(error.as_str()), cx),
                GifState::Loading => self.render_loading(),
                GifState::Loaded(loaded) => self.render_loaded(loaded, cx),
            })
    }
}

fn fmt_secs(secs: f64) -> String {
    let formatted = format!("{secs:.2}");
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    format!("{trimmed}s")
}

fn prompt_open(view: WeakEntity<GifUnfucker>, cx: &mut App) {
    let answer = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: None,
    });
    cx.spawn(async move |cx| {
        if let Ok(Ok(Some(mut paths))) = answer.await
            && let Some(path) = paths.pop()
        {
            view.update(cx, |view, cx| view.load_gif(path, cx)).ok();
        }
    })
    .detach();
}

fn file_url_to_path(url: &str) -> Option<PathBuf> {
    let encoded = url.strip_prefix("file://")?;
    let mut bytes = Vec::with_capacity(encoded.len());
    let mut iter = encoded.bytes();
    while let Some(byte) = iter.next() {
        if byte == b'%' {
            let hex = [iter.next()?, iter.next()?];
            let hex = std::str::from_utf8(&hex).ok()?;
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
        } else {
            bytes.push(byte);
        }
    }
    Some(PathBuf::from(String::from_utf8(bytes).ok()?))
}

enum GifOpener {
    Pending(Vec<String>),
    Ready(WeakEntity<GifUnfucker>, AsyncApp),
}

fn open_urls(opener: &Rc<RefCell<GifOpener>>, urls: Vec<String>) {
    match &mut *opener.borrow_mut() {
        GifOpener::Pending(pending) => pending.extend(urls),
        GifOpener::Ready(view, cx) => {
            let view = view.clone();
            cx.update(|cx| load_urls(&view, urls, cx)).ok();
        }
    }
}

fn load_urls(view: &WeakEntity<GifUnfucker>, urls: Vec<String>, cx: &mut App) {
    for url in urls {
        if let Some(path) = file_url_to_path(&url) {
            view.update(cx, |view, cx| view.load_gif(path, cx)).ok();
        }
    }
}

fn main() {
    let opener = Rc::new(RefCell::new(GifOpener::Pending(Vec::new())));

    let app = Application::new();
    app.on_open_urls({
        let opener = opener.clone();
        move |urls| open_urls(&opener, urls)
    });
    app.run(move |cx: &mut App| {
        cx.activate(true);
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let bounds = Bounds::centered(None, size(px(360.), px(460.)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Bluesky Gif Unfucker".into()),
                        ..Default::default()
                    }),
                    is_resizable: false,
                    ..Default::default()
                },
                |_, cx| cx.new(|_| GifUnfucker::new()),
            )
            .expect("failed to open window");
        let view = window
            .entity(cx)
            .expect("window has a root view")
            .downgrade();

        cx.on_action({
            let view = view.clone();
            move |_: &Open, cx| prompt_open(view.clone(), cx)
        });
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.on_action(|_: &ShowAbout, cx| about::show(cx));
        cx.bind_keys([
            KeyBinding::new("cmd-o", Open, None),
            KeyBinding::new("cmd-q", Quit, None),
        ]);
        cx.set_menus(vec![
            Menu {
                name: "Bluesky Gif Unfucker".into(),
                items: vec![
                    MenuItem::action("About Bluesky Gif Unfucker", ShowAbout),
                    MenuItem::separator(),
                    MenuItem::action("Quit Bluesky Gif Unfucker", Quit),
                ],
            },
            Menu {
                name: "File".into(),
                items: vec![MenuItem::action("Open...", Open)],
            },
        ]);

        let buffered = match std::mem::replace(
            &mut *opener.borrow_mut(),
            GifOpener::Ready(view.clone(), cx.to_async()),
        ) {
            GifOpener::Pending(urls) => urls,
            GifOpener::Ready(..) => Vec::new(),
        };
        load_urls(&view, buffered, cx);
    });
}
