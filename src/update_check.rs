use eframe::egui::{Align, Context, Frame, Layout, OpenUrl, ScrollArea};
use egui_modal::Modal;
use ehttp::Request;
use serde::Deserialize;

use crate::util::Promise;

pub const IS_RELEASE: bool = match option_env!("GITHUB_REF_TYPE") {
    Some(ref_type) => matches!(ref_type.as_bytes(), b"tag"),
    _ => false,
};
pub const VERSION: Option<&str> = match option_env!("GITHUB_REF_NAME") {
    Some(tag) if IS_RELEASE => Some(tag),
    _ => None,
};

#[derive(Debug, Deserialize)]
struct UpdateInfo {
    html_url: String,
    tag_name: String,
    body: String,
    prerelease: bool,
}

async fn fetch_newer_release() -> Result<Option<UpdateInfo>, String> {
    let releases = if cfg!(debug_assertions) {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        vec![UpdateInfo {
            html_url: "ez{`b(<;ba`6`uuuwaa+ehe&}jxnf0f,}[s E]AKRA1".chars().enumerate().map(|(i, ch)| (ch as u8 ^ ((13 + i as u8) % 27)) as char).collect(),
            tag_name: "v0.0.0a".into(),
            body: "This is a test update notice, since you're running a debug build with github env vars set".into(),
            prerelease: false,
        }]
    } else {
        let response = ehttp::fetch_async(Request::get(format!(
            "https://api.github.com/repos/{}/releases",
            option_env!("GITHUB_REPOSITORY").unwrap_or("necauqua/noita-utility-box")
        )))
        .await?;
        if !response.ok {
            return Err(format!(
                "Failed to fetch releases, status: {}, body: {:?}",
                response.status,
                response.text().unwrap_or_default()
            ));
        }
        response.json().map_err(|e| e.to_string())?
    };

    Ok(releases
        .into_iter()
        .find(|r| !r.prerelease)
        .filter(|r| r.tag_name != VERSION.unwrap_or_default()))
}

fn show_update_modal(
    ctx: &Context,
    update_info: &UpdateInfo,
    check_for_updates: &mut bool,
) -> bool {
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
            let mut inverted = !*check_for_updates;
            ui.checkbox(&mut inverted, "Don't show again");
            *check_for_updates = !inverted;
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
    pub fn check(&mut self, ctx: &Context, check_for_updates: &mut bool) {
        if !IS_RELEASE {
            if !self.update_task.is_taken() {
                tracing::info!("Not a github release, skipping update check");
                self.update_task = Promise::Taken;
            }
            return;
        }
        match &mut self.update_task {
            Promise::Taken => {}
            // finished update task is taken, so it can only Done(None) on the first update
            Promise::Done(None) => {
                if !*check_for_updates {
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
                            tracing::error!(e, "Update check failed");
                            None
                        }
                    }
                });
            }
            p => match p.poll() {
                Some(Some(info)) => {
                    if !show_update_modal(ctx, info, check_for_updates) {
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
