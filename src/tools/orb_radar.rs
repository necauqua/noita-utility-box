use std::{collections::HashSet, fmt::Write as _};

use crate::{
    app::AppState,
    orb_searcher::{Orb, OrbSearcher, OrbSource},
};
use eframe::egui::{
    Align, Align2, Color32, FontId, Layout, Rect, Rounding, Stroke, TextStyle, Ui, pos2, vec2,
};
use noita_engine_reader::{PlayerState, Seed};
use serde::{Deserialize, Serialize};

use super::{Result, Tool};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct OrbRadar {
    realtime: bool,
    show_rooms: bool,
    filter_collected_orbs: bool,
    orb_searcher: OrbSearcher,
    #[serde(skip)]
    prev_seed: Option<Seed>,
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
        if state.seed != self.prev_seed {
            self.prev_seed = state.seed;
            self.orb_searcher.reset();
        }

        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.realtime, "Realtime");
                ui.checkbox(&mut self.show_rooms, "Show orb rooms");
                ui.checkbox(&mut self.filter_collected_orbs, "Filter collected orbs");

                if ui
                    .checkbox(
                        &mut self.orb_searcher.look_for_sampo_instead,
                        "Look for sampo instead",
                    )
                    .changed()
                    | ui.button("Reset").clicked()
                {
                    self.orb_searcher.reset();
                };

                if self.orb_searcher.is_searching() {
                    ui.label("Searching..");
                    ui.spinner();
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

            let player = state.noita.as_mut().and_then(|n| {
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

            let heading_font = ui
                .style()
                .text_styles
                .get(&TextStyle::Heading)
                .cloned()
                .unwrap_or(FontId::proportional(16.0));

            let Some(((pos, player_state), seed)) = player.zip(state.seed) else {
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "NO DATA",
                    heading_font,
                    ui.style().visuals.warn_fg_color,
                );

                return;
            };
            let popup = match player_state {
                PlayerState::Normal => "",
                PlayerState::Polymorphed => "POLYMORPHED LOL",
                PlayerState::Cessated => "Cessated",
            };
            if !popup.is_empty() {
                painter.text(
                    rect.left_top() + vec2(5.0, 5.0),
                    Align2::LEFT_TOP,
                    popup,
                    heading_font.clone(),
                    ui.style().visuals.strong_text_color(),
                );
            }

            self.orb_searcher.poll_search(ui.ctx(), seed, pos);

            let known_orbs: Vec<Orb> = if self.show_rooms {
                self.orb_searcher
                    .known_orbs()
                    .iter()
                    .chain(self.orb_searcher.known_rooms())
                    .cloned()
                    .collect()
            } else {
                self.orb_searcher.known_orbs().to_vec()
            };

            let collected_orbs = OrbRadar::collected_orbs(state);
            let mut displayed_orbs = if self.filter_collected_orbs {
                known_orbs
                    .iter()
                    .filter(|orb: &&Orb| !collected_orbs.contains(&orb.id))
                    .cloned()
                    .collect::<Vec<Orb>>()
            } else {
                known_orbs
            };

            displayed_orbs.sort_by_key(|orb| {
                let dir = orb.pos - pos;
                dir.length_sq() as i32
            });

            let Some(first_orb) = displayed_orbs.first() else {
                return;
            };

            let dir_to_first = first_orb.pos - pos;
            let dist_to_first = dir_to_first.length();

            let alpha = ((dist_to_first - 25.0) * 2.0 / (rect.width().min(rect.height()) - 25.0))
                .clamp(0.0, 1.0);

            for (i, orb) in displayed_orbs.iter().enumerate() {
                let dir = orb.pos - pos;
                let pos = rect.center() + dir;
                let orb_color = self.orb_color(ui, orb, state);

                if rect.contains(pos) {
                    let color = if i == 0 {
                        orb_color
                    } else {
                        orb_color.linear_multiply(alpha)
                    };

                    painter.circle_stroke(pos, 6.0, Stroke::new(1.0, color));
                    painter.rect(
                        Rect::from_center_size(pos, vec2(2.0, 2.0)),
                        Rounding::same(0.0),
                        color,
                        Stroke::NONE,
                    );
                } else if self.orb_searcher.look_for_sampo_instead {
                    continue;
                }

                let dist = dir.length();
                let dir = dir.normalized();

                if dist > 25.0 {
                    let mut tracer = if i == 0 { tracer_bright } else { tracer };
                    tracer.color = orb_color.linear_multiply(alpha);
                    painter.line_segment([rect.center() + dir * 10.0, pos], tracer);
                }

                let offset = rect.width().min(rect.height()) / 4.0;
                if offset < dist {
                    painter.text(
                        rect.center() + dir * offset,
                        Align2::CENTER_CENTER,
                        format!("{dist:.1} px"),
                        ui.style()
                            .text_styles
                            .get(&TextStyle::Monospace)
                            .cloned()
                            .unwrap_or(FontId::monospace(6.0)),
                        orb_color.linear_multiply(alpha),
                    );
                }
            }

            // Crosshair
            let r = |p| painter.round_pos_to_pixels(p);
            let c = rect.center();
            let c_from = 2.0;
            let c_to = 5.0;

            painter.line_segment([r(c - vec2(c_from, 0.0)), r(c - vec2(c_to, 0.0))], stroke);
            painter.line_segment([r(c + vec2(c_from, 0.0)), r(c + vec2(c_to, 0.0))], stroke);
            painter.line_segment([r(c - vec2(0.0, c_from)), r(c - vec2(0.0, c_to))], stroke);
            painter.line_segment([r(c + vec2(0.0, c_from)), r(c + vec2(0.0, c_to))], stroke);

            // Player infos
            let player_info_pos = rect.right_top() + vec2(-5.0, 5.0);
            let player_infos_font = ui
                .style()
                .text_styles
                .get(&TextStyle::Monospace)
                .cloned()
                .unwrap_or(FontId::monospace(12.0));

            let mut player_infos = String::new();
            writeln!(&mut player_infos, "pos: ({:.1}, {:.1})", pos.x, pos.y).unwrap();
            writeln!(
                &mut player_infos,
                "chunks searched: {}",
                self.orb_searcher.searched_chunks()
            )
            .unwrap();
            writeln!(
                &mut player_infos,
                "orbs collected: {}",
                collected_orbs.len()
            )
            .unwrap();
            writeln!(
                &mut player_infos,
                "chest orbs found: {}",
                self.orb_searcher.known_orbs().len()
            )
            .unwrap();

            let orbs_to_list =
                (rect.height() / ui.fonts(|f| f.row_height(&player_infos_font))) as usize / 2;

            for orb in displayed_orbs.iter().take(orbs_to_list) {
                // NOTE: EGUI does not support rendering UTF-8 emojis... Sadge
                writeln!(
                    &mut player_infos,
                    "  ({: >5.0}, {: >5.0}) id={}",
                    orb.pos.x, orb.pos.y, orb.id
                )
                .unwrap();
            }
            if displayed_orbs.len() > orbs_to_list {
                writeln!(
                    &mut player_infos,
                    "  ... and {} more orbs",
                    displayed_orbs.len() - orbs_to_list
                )
                .unwrap();
            }

            let color = ui.style().visuals.weak_text_color();
            painter.text(
                player_info_pos,
                Align2::RIGHT_TOP,
                player_infos,
                player_infos_font.clone(),
                color,
            );

            // Bottom Left compass
            let diameter = player_infos_font.size * 2.0; // Diameter relative to the text size
            let radius = diameter / 2.0;

            let padding = 10.0; // Padding from the sides to the circle

            let circle_pos = rect.left_bottom() + vec2(radius + padding, -radius - padding);

            if pos.x.round() == first_orb.pos.x.round() && pos.y.round() == first_orb.pos.y.round()
            {
                painter.circle(circle_pos, radius, Color32::from_rgb(40, 255, 40), stroke);
                return;
            }
            painter.circle_stroke(circle_pos, radius, stroke);
            let arrow = dir_to_first * (diameter - 10.0) / dist_to_first;
            painter.arrow(
                circle_pos - arrow / 2.0,
                arrow,
                Stroke::new(stroke.width, self.orb_color(ui, first_orb, state)),
            );

            painter.text(
                circle_pos + vec2(radius + padding, 0.0),
                Align2::LEFT_CENTER,
                format!("{dist_to_first:.1} px"),
                player_infos_font,
                self.orb_color(ui, first_orb, state),
            );
        });
    }

    fn orb_color(&self, ui: &Ui, orb: &Orb, state: &AppState) -> Color32 {
        if !self.show_rooms {
            return ui.style().visuals.text_color();
        }
        match orb.source {
            OrbSource::Room => state.settings.color_orb_rooms,
            OrbSource::Chest => state.settings.color_orb_chests,
        }
    }

    fn collected_orbs(state: &mut AppState) -> HashSet<i32> {
        let world = state
            .noita
            .as_mut()
            .and_then(|n| n.get_world_state().unwrap_or(None));
        let Some(world) = world else {
            return HashSet::new();
        };

        let Ok(collected_orbs) = world
            .orbs_found_thisrun
            .read_storage(state.noita.as_mut().unwrap().proc())
        else {
            return HashSet::new();
        };
        collected_orbs.iter().copied().collect()
    }
}
