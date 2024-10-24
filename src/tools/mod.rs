use std::{
    any::TypeId,
    fmt::{self, Display},
};

use crate::app::AppState;
use crate::util::to_title_case;
use eframe::egui::{Context, Ui};
use thiserror::Error;

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
                    type_id: {
                        fn deferred() -> TypeId {
                            TypeId::of::<$prefix::$t>()
                        }

                        deferred
                    },
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
    type_id: fn() -> TypeId,
}

impl ToolInfo {
    pub fn is_it(&self, tool: &dyn Tool) -> bool {
        (self.type_id)() == tool.type_id()
    }
}

#[derive(Debug)]
pub enum UnexpectedError {
    Contextual(anyhow::Error),
    Io(std::io::Error),
}

impl Display for UnexpectedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use UnexpectedError as E;
        match self {
            E::Contextual(e) => write!(f, "Error: {e:#}"),
            E::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("{0}")]
    Unexpected(UnexpectedError),
    #[error("Not connected to Noita")]
    NoitaNotConnected,
    #[error("Player entity not found")]
    PlayerNotFound,
}

impl From<anyhow::Error> for ToolError {
    fn from(e: anyhow::Error) -> Self {
        ToolError::Unexpected(UnexpectedError::Contextual(e))
    }
}

impl From<std::io::Error> for ToolError {
    fn from(e: std::io::Error) -> Self {
        ToolError::Unexpected(UnexpectedError::Io(e))
    }
}

pub type Result = std::result::Result<(), ToolError>;

#[typetag::serde]
pub trait Tool: Send + 'static {
    /// The background update call
    fn tick(&mut self, _ctx: &Context, _state: &mut AppState) {}

    /// The main egui draw function for the tool
    fn ui(&mut self, ui: &mut Ui, state: &mut AppState) -> Result;

    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}
