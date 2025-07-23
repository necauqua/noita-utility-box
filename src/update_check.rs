use std::mem;

use anyhow::Result;
use eframe::egui::{
    Align, Context, Id, Layout, Modal, OpenUrl, Response, RichText, ScrollArea, Sense, TextStyle,
    Ui, Widget, style::ScrollStyle, vec2,
};
use reqwest::Client;
use serde::Deserialize;

use crate::{app::AppState, util::Promise};

pub const RELEASE_VERSION: Option<&str> = option_env!("CI_RELEASE_VERSION");

#[derive(Debug, Deserialize)]
struct UpdateInfo {
    html_url: String,
    tag_name: String,
    body: String,
    prerelease: bool,
}

async fn fetch_newer_release() -> Result<Option<UpdateInfo>> {
    if cfg!(debug_assertions) {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        return Ok(Some(UpdateInfo {
            html_url: "ez{`b(<;ba`6`uuuwaa+ehe&}jxnf0f,}[s E]AKRA1".chars().enumerate().map(|(i, ch)| (ch as u8 ^ ((13 + i as u8) % 27)) as char).collect(),
            tag_name: "v0.0.0a".into(),
            body: r"
                ### Added
                    - This is a test update notice, since you're running a debug build with github env vars set
                    - Here we have some `inline code` and some **bold** text, as well as some _italics_
                ### Changed
                    - Made it way better better way better way better way better way better way better way better way better way better way better way better way better way better way better way better way better way better way
                    - Also made it support [links](https://necauq.ua)?
                ### Removed
                    - Removed Herobrine
            ".into(),
            prerelease: false,
        }));
    }

    let releases: Vec<UpdateInfo> = Client::builder()
        .build()?
        .get("https://api.github.com/repos/necauqua/noita-utility-box/releases")
        .header(
            "user-agent",
            concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(releases
        .into_iter()
        .find(|r| !r.prerelease)
        .filter(|r| r.tag_name != RELEASE_VERSION.unwrap_or_default()))
}

#[derive(Debug, Default)]
pub struct UpdateChecker {
    update_task: Promise<Option<UpdateInfo>>,
}

// stole that from egui examples
fn bullet_point(ui: &mut Ui, width: f32, height: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::empty());
    ui.painter().circle_filled(
        rect.center(),
        rect.height() / 8.0,
        ui.visuals().strong_text_color(),
    );
    response
}

fn draw_a_tiny_subset_of_markdown(ui: &mut Ui, text: &str) {
    let row_height = ui.text_style_height(&TextStyle::Body);
    for line in text.lines() {
        if let Some(line) = line.trim().strip_prefix("###") {
            ui.strong(line.trim());
            continue;
        }
        if let Some(line) = line.trim().strip_prefix("-") {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.set_row_height(row_height);
                bullet_point(ui, row_height, row_height);
                ui.horizontal_wrapped(|ui| {
                    ui.set_row_height(row_height);
                    ui.spacing_mut().item_spacing.x = 1.0;
                    for f in InlineMarkdownFragment::parse(line.trim()) {
                        f.ui(ui);
                    }
                });
            });
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.set_row_height(row_height);
                ui.spacing_mut().item_spacing.x = 1.0;
                for f in InlineMarkdownFragment::parse(line.trim()) {
                    f.ui(ui);
                }
            });
        }
    }
}

fn show_update_modal(ctx: &Context, update_info: &UpdateInfo, state: &mut AppState) -> bool {
    if !state.settings.notify_when_outdated {
        return false;
    }

    let mut close = false;

    let response = Modal::new(Id::new("update")).show(ctx, |ui| {
        let screen_rect = ui.ctx().input(|i| i.screen_rect);
        ui.set_max_width(screen_rect.width() * 0.8);
        ui.set_max_height(screen_rect.height() * 0.6);

        ui.label(RichText::new("An update is available").heading().strong());

        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label("You are running version ");
            ui.monospace(RELEASE_VERSION.unwrap_or("<unknown>"));
            ui.label(", the newest is ");
            ui.monospace(&update_info.tag_name);
        });

        ui.label("Changelog:");

        ui.separator();
        ui.spacing_mut().scroll = ScrollStyle::thin();
        ScrollArea::vertical().show(ui, |ui| {
            draw_a_tiny_subset_of_markdown(ui, &update_info.body);
        });
        ui.separator();

        ui.vertical(|ui| {
            let mut inverted = !state.settings.notify_when_outdated;
            ui.checkbox(&mut inverted, "Don't show again");
            state.settings.notify_when_outdated = !inverted;

            ui.with_layout(Layout::top_down(Align::Max), |ui| {
                if ui.button("Download").clicked() {
                    ctx.open_url(OpenUrl {
                        url: update_info.html_url.clone(),
                        new_tab: true,
                    });
                    close = true;
                }
                if ui.button("Dismiss").clicked() {
                    close = true;
                }
            })
        })
    });

    !(close || response.should_close())
}

impl UpdateChecker {
    pub fn check(&mut self, ctx: &Context, state: &mut AppState) {
        if RELEASE_VERSION.is_none() {
            if !self.update_task.is_taken() {
                tracing::info!("Not a release version, skipping update check");
                self.update_task = Promise::Taken;
            }
            return;
        }
        match &mut self.update_task {
            Promise::Taken => {}
            // finished update task is taken, so it can only Done(None) on the first update
            Promise::Done(None) => {
                if !state.settings.check_for_updates {
                    tracing::info!("Update check is disabled, skipping");
                    self.update_task = Promise::Taken;
                }
                let ctx = ctx.clone();
                self.update_task = Promise::spawn(async move {
                    match fetch_newer_release().await {
                        Ok(info) => {
                            ctx.request_repaint();
                            info
                        }
                        Err(e) => {
                            tracing::error!(e = e.to_string(), "Update check failed");
                            None
                        }
                    }
                });
            }
            p => match p.poll() {
                Some(Some(info)) => {
                    if !show_update_modal(ctx, info, state) {
                        state.settings.newest_version = Some(info.tag_name.clone());
                        self.update_task = Promise::Taken;
                    }
                }
                Some(None) => {
                    tracing::info!("No updates found");
                    self.update_task = Promise::Taken;
                }
                None => {}
            },
        }
    }
}

enum InlineMarkdownFragment {
    Text(String),
    Code(String),
    Bold(String),
    Italic(String),
    Link(String, String),
    Username(String),
}

impl Widget for InlineMarkdownFragment {
    fn ui(self, ui: &mut Ui) -> Response {
        match self {
            Self::Text(text) => ui.label(RichText::new(text)),
            Self::Code(text) => ui.label(RichText::new(text).code()),
            Self::Bold(text) => ui.label(RichText::new(text).strong()),
            Self::Italic(text) => ui.label(RichText::new(text).italics()),
            Self::Link(text, url) => ui.hyperlink_to(text, url),
            Self::Username(name) => {
                ui.hyperlink_to(format!("@{name}"), format!("https://github.com/{name}"))
            }
        }
    }
}

impl InlineMarkdownFragment {
    fn parse(line: &str) -> Vec<Self> {
        #[derive(Clone)]
        enum State {
            Text,
            Code,
            Italic(char),
            Bold(char),
            LinkText,
            LinkUrl(String),
        }

        let mut fragments = Vec::new();
        let mut current = String::new();
        let mut state = State::Text;
        let mut chars = line.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(&next_c) = chars.peek() {
                    current.push(next_c);
                    chars.next();
                }
                continue;
            }

            match &mut state {
                State::Text => {
                    if c == '`' {
                        if !current.is_empty() {
                            fragments.push(Self::Text(mem::take(&mut current)));
                        }
                        state = State::Code;
                        continue;
                    }
                    if c == '*' || c == '_' {
                        let next = chars.peek();
                        if let Some(&nc) = next
                            && nc == c
                        {
                            chars.next();
                            if !current.is_empty() {
                                fragments.push(Self::Text(mem::take(&mut current)));
                            }
                            state = State::Bold(c);
                            continue;
                        }
                        if !current.is_empty() {
                            fragments.push(Self::Text(mem::take(&mut current)));
                        }
                        state = State::Italic(c);
                        continue;
                    }
                    if c == '[' {
                        if !current.is_empty() {
                            fragments.push(Self::Text(mem::take(&mut current)));
                        }
                        state = State::LinkText;
                        continue;
                    }
                    if c == '@' {
                        let mut username = String::new();
                        let mut peeked = chars.peek();
                        while let Some(&ch) = peeked {
                            if ch.is_ascii_alphanumeric() {
                                username.push(ch);
                                chars.next();
                                peeked = chars.peek();
                            } else {
                                break;
                            }
                        }
                        if username.is_empty() {
                            current.push('@');
                            continue;
                        }
                        if !current.is_empty() {
                            fragments.push(Self::Text(mem::take(&mut current)));
                        }
                        fragments.push(Self::Username(username));
                        continue;
                    }
                    current.push(c);
                }
                State::Code => {
                    if c == '`' {
                        fragments.push(Self::Code(mem::take(&mut current)));
                        state = State::Text;
                        continue;
                    }
                    current.push(c);
                }
                State::Italic(delim) => {
                    if c == *delim {
                        fragments.push(Self::Italic(mem::take(&mut current)));
                        state = State::Text;
                        continue;
                    }
                    current.push(c);
                }
                State::Bold(delim) => {
                    if c == *delim
                        && let Some(&nc) = chars.peek()
                        && nc == *delim
                    {
                        chars.next();
                        fragments.push(Self::Bold(mem::take(&mut current)));
                        state = State::Text;
                        continue;
                    }
                    current.push(c);
                }
                State::LinkText => {
                    if c == ']' {
                        let link_text = mem::take(&mut current);
                        if let Some(&'(') = chars.peek() {
                            chars.next();
                            state = State::LinkUrl(link_text);
                            continue;
                        }
                        current.push('[');
                        current.push_str(&link_text);
                        current.push(']');
                        state = State::Text;
                        continue;
                    }
                    current.push(c);
                }
                State::LinkUrl(link_text) => {
                    if c == ')' {
                        fragments.push(Self::Link(mem::take(link_text), mem::take(&mut current)));
                        state = State::Text;
                        continue;
                    }
                    current.push(c);
                }
            }
        }

        if !current.is_empty() {
            match state {
                State::Text => fragments.push(Self::Text(current)),
                State::Code => fragments.push(Self::Code(current)),
                State::Italic(_) => fragments.push(Self::Italic(current)),
                State::Bold(_) => fragments.push(Self::Bold(current)),
                State::LinkText => {
                    current.insert(0, '[');
                    fragments.push(Self::Text(current));
                }
                State::LinkUrl(link_text) => {
                    fragments.push(Self::Text(format!("[{link_text}]({current}")));
                }
            }
        }

        fragments
    }
}
