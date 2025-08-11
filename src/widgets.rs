use eframe::egui::{self, CollapsingHeader, ScrollArea, Ui, Widget, WidgetText};
use serde_json::Value;

pub struct JsonWidget<'a>(&'a Value);

impl<'a> JsonWidget<'a> {
    pub fn new(value: &'a Value) -> Self {
        Self(value)
    }
}

impl Widget for JsonWidget<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        ScrollArea::vertical()
            .auto_shrink([false, true])
            .show(ui, |ui| draw_key_value(Key::None, self.0, ui));
        ui.response()
    }
}

enum Key<'a> {
    Object(&'a str),
    Array(usize),
    None,
}

impl Key<'_> {
    fn simple(&self, value: impl Into<WidgetText>, ui: &mut Ui) {
        if let Key::Object(k) = self {
            ui.horizontal(|ui| {
                ui.style_mut().spacing.item_spacing.x = 0.0;
                ui.label(format!("{k:?}: "));
                ui.label(value);
            });
        } else {
            ui.label(value);
        }
    }

    fn nested(
        &self,
        ui: &mut Ui,
        count: Option<usize>,
        add_contents: impl FnOnce(&mut Ui) -> egui::Response,
    ) -> egui::Response {
        match self {
            Key::Object(k) => {
                let title = if let Some(count) = count {
                    format!("{k:?} ({count})")
                } else {
                    format!("{k:?}")
                };
                CollapsingHeader::new(title)
                    .id_salt(ui.id().with(k))
                    .show(ui, add_contents)
                    .header_response
            }
            Key::Array(i) => {
                let title = if let Some(count) = count {
                    format!("{i} ({count})")
                } else {
                    i.to_string()
                };
                CollapsingHeader::new(title)
                    .id_salt(ui.id().with(i))
                    .show(ui, add_contents)
                    .header_response
            }
            Key::None => add_contents(ui),
        }
    }
}

fn draw_key_value(key: Key, value: &Value, ui: &mut Ui) {
    match value {
        Value::Null => key.simple("null", ui),
        Value::Bool(b) => key.simple(b.to_string(), ui),
        Value::Number(n) => key.simple(n.to_string(), ui),
        Value::String(s) => key.simple(format!("\"{s}\""), ui),
        Value::Array(arr) => {
            key.nested(ui, Some(arr.len()), |ui| {
                for (i, item) in arr.iter().enumerate() {
                    draw_key_value(Key::Array(i), item, ui);
                }
                ui.response()
            });
        }
        Value::Object(obj) => {
            key.nested(ui, None, |ui| {
                for (k, v) in obj.iter() {
                    draw_key_value(Key::Object(k), v, ui);
                }
                ui.response()
            });
        }
    }
}
