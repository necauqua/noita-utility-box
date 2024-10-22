use crate::app::AppState;
use crate::util::to_title_case;
use eframe::egui::{Context, Ui};

macro_rules! tools {
    (_get_title $title:expr ; $t:ident) => {
        $title
    };
    (_get_title ; $t:ident) => {
        to_title_case!(stringify!($t))
    };
    ($($prefix:ident::$t:ident $(: $title:expr)?;)*) => {
        $(pub mod $prefix;)*

        pub static TOOLS: &[&ToolInfo] = &[
            $(
                &$crate::tools::ToolInfo {
                    default_constructor: || Box::new(<$prefix::$t>::default()),
                    title: tools!(_get_title $($title)?; $t),
                },
            )*
        ];
    };
}

tools! {
    process_panel::ProcessPanel : "Noita";
    orb_radar::OrbRadar;
    live_stats::LiveStats;
    material_pipette::MaterialPipette;
    address_maps::AddressMaps;
    settings::Settings;
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ToolInfo {
    pub default_constructor: fn() -> Box<dyn Tool>,
    pub title: &'static str,
}

#[typetag::serde]
pub trait Tool: Send + 'static {
    /// The main egui draw function for the tool
    fn ui(&mut self, ui: &mut Ui, state: &mut AppState);

    /// The background update call
    fn tick(&mut self, _ctx: &Context, _state: &mut AppState) {}
}
