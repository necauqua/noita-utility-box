use anyhow::Result;
use eframe::egui::{Align, Context, Frame, Layout, OpenUrl, ScrollArea};
use egui_modal::Modal;
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
            body: "This is a test update notice, since you're running a debug build with github env vars set".into(),
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

fn show_update_modal(ctx: &Context, update_info: &UpdateInfo, state: &mut AppState) -> bool {
    if !state.settings.notify_when_outdated {
        return false;
    }

    let modal = Modal::new(ctx, "update").with_close_on_outside_click(true);
    modal.open();
    modal.show(|ui| {
        let pre_title_rest = ui.available_height();
        modal.title(ui, "An update is available");
        let title_height = pre_title_rest - ui.available_height();

        let max_height = ui.ctx().input(|i| i.screen_rect.height()) - title_height * 6.0; // idk lol

        // modal.frame(ui, |ui| {
        ui.with_layout(Layout::top_down(Align::Min), |ui| {
            Frame::none().inner_margin(5.0).show(ui, |ui| {
                ui.label(format!(
                    "Version {} was released\n\nChangelog:",
                    update_info.tag_name
                ));

                ScrollArea::vertical()
                    .max_height(max_height)
                    .show(ui, |ui| {
                        ui.label(&update_info.body);
                    });
            });
        });
        // });

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
                    modal.close();
                }
                if ui.button("Dismiss").clicked() {
                    modal.close();
                }
            })
        })
    });
    modal.is_open()
}

#[derive(Debug, Default)]
pub struct UpdateChecker {
    update_task: Promise<Option<UpdateInfo>>,
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
