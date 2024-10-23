use std::fmt::Write as _;

use crate::{app::AppState, orb_searcher::OrbSearcher};
use eframe::egui::{
    pos2, vec2, Align, Align2, Color32, FontId, Layout, Rect, Rounding, Stroke, Ui,
};
use serde::{Deserialize, Serialize};

use super::{Result, Tool};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct OrbRadar {
    realtime: bool,
    #[serde(skip)]
    orb_searcher: OrbSearcher,
}

#[typetag::serde]
impl Tool for OrbRadar {
    fn ui(&mut self, ui: &mut Ui, state: &mut AppState) -> Result {
        self.ui(ui, state);
        Ok(())
    }
}

impl OrbRadar {
    pub fn ui(&mut self, ui: &mut Ui, state: &mut AppState) {
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.realtime, "Realtime");
                if ui.button("Reset").clicked() {
                    self.orb_searcher.reset();
                }
            });

            if self.realtime {
                ui.ctx().request_repaint();
            }

            let (_, rect) = ui.allocate_space(ui.available_size());

            let mut painter = ui.painter_at(rect);

            let text_color = ui.style().visuals.text_color();
            let stroke = Stroke::new(2.0, text_color);

            let tracer = Stroke::new(
                1.0 / ui.ctx().pixels_per_point(),
                ui.style().visuals.weak_text_color(),
            );
            let tracer_bright = Stroke::new(
                1.0 / ui.ctx().pixels_per_point(),
                ui.style().visuals.strong_text_color(),
            );

            let rect = rect.shrink(stroke.width);
            painter.rect(
                rect,
                Rounding::same(0.0),
                ui.style().visuals.extreme_bg_color,
                stroke,
            );
            painter.set_clip_rect(rect);

            let pos = state.noita.as_mut().and_then(|n| {
                n.get_player()
                    .map_err(|e| {
                        tracing::warn!(%e, "failed to read player pos");
                        e
                    })
                    .ok()
                    .flatten()
                    .map(|(player, p)| {
                        let pos = player.transform.pos;
                        (pos2(pos.x, pos.y), p)
                    })
            });

            let Some(((pos, p), seed)) = pos.zip(state.seed) else {
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "NO DATA",
                    FontId::monospace(16.0),
                    ui.style().visuals.warn_fg_color,
                );

                return;
            };
            if p {
                painter.text(
                    rect.left_top() + vec2(5.0, 5.0),
                    Align2::LEFT_TOP,
                    "POLYMORPHED LOL",
                    FontId::proportional(16.0),
                    ui.style().visuals.strong_text_color(),
                );
            }

            self.orb_searcher.poll_search(ui.ctx(), seed, pos);

            for (i, orb) in self.orb_searcher.known_orbs().iter().enumerate() {
                let dir = *orb - pos;
                let pos = rect.center() + dir;

                if rect.contains(pos) {
                    painter.circle_stroke(
                        pos,
                        6.0,
                        Stroke::new(1.0, ui.style().visuals.strong_text_color()),
                    );
                    painter.rect(
                        Rect::from_center_size(pos, vec2(2.0, 2.0)),
                        Rounding::same(0.0),
                        ui.style().visuals.strong_text_color(),
                        Stroke::NONE,
                    );
                    break;
                }

                let dist = dir.length();
                let dir = dir.normalized();

                painter.line_segment(
                    [rect.center() + dir * 10.0, pos],
                    if i == 0 { tracer_bright } else { tracer },
                );

                painter.text(
                    rect.center() + dir * (rect.width().min(rect.height()) / 4.0),
                    Align2::CENTER_CENTER,
                    format!("{dist:.1} px"),
                    FontId::monospace(6.0),
                    text_color,
                );
            }

            let c = rect.center();
            let c_from = 2.0;
            let c_to = 5.0;

            let r = |p| painter.round_pos_to_pixels(p);

            painter.line_segment([r(c - vec2(c_from, 0.0)), r(c - vec2(c_to, 0.0))], stroke);
            painter.line_segment([r(c + vec2(c_from, 0.0)), r(c + vec2(c_to, 0.0))], stroke);
            painter.line_segment([r(c - vec2(0.0, c_from)), r(c - vec2(0.0, c_to))], stroke);
            painter.line_segment([r(c + vec2(0.0, c_from)), r(c + vec2(0.0, c_to))], stroke);

            let mut text = format!(
                "pos: x:{:.1} y:{:.1}\nchunks searched: {}\nchunk size: {}\norbs found: {}\n",
                pos.x,
                pos.y,
                self.orb_searcher.searched_chunks(),
                self.orb_searcher.chunk_size(),
                self.orb_searcher.known_orbs().len(),
            );
            for orb in self.orb_searcher.known_orbs() {
                writeln!(&mut text, "  ({: >5.0}, {: >5.0})", orb.x, orb.y).unwrap();
            }

            painter.text(
                rect.right_top() + vec2(-5.0, 5.0),
                Align2::RIGHT_TOP,
                text,
                FontId::monospace(6.0),
                ui.style().visuals.weak_text_color(),
            );

            let diameter = 25.0;
            let offset = 10.0;

            let radius = diameter / 2.0;
            let circle_pos = rect.left_bottom() + vec2(radius + offset, -radius - offset);

            if let Some(orb) = self.orb_searcher.known_orbs().first() {
                if pos.x.round() == orb.x.round() && pos.y.round() == orb.y.round() {
                    painter.circle(circle_pos, radius, Color32::from_rgb(40, 255, 40), stroke);
                } else {
                    painter.circle_stroke(circle_pos, radius, stroke);
                    let dir = *orb - pos;
                    let len = dir.length();
                    let arrow = dir * (diameter - 10.0) / len;
                    painter.arrow(circle_pos - arrow / 2.0, arrow, stroke);

                    painter.text(
                        circle_pos + vec2(radius + offset, 0.0),
                        Align2::LEFT_CENTER,
                        format!("{len:.1} px"),
                        FontId::monospace(8.0),
                        text_color,
                    );
                }
            } else {
                painter.circle_stroke(circle_pos, radius, stroke);
            }
        });
    }
}
