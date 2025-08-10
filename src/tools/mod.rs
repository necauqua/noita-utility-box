use std::{
    any::TypeId,
    borrow::Cow,
    fmt::{self, Display},
    panic::Location,
};

use crate::app::AppState;
use crate::util::to_title_case;
use anyhow::{Context as _, anyhow};
use eframe::egui::{Context, Ui};
use noita_engine_reader::{
    ComponentStore,
    memory::Pod,
    types::{Entity, components::ComponentName},
};
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
    player_info::PlayerInfo;
    material_list::MaterialList;
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
    #[error("{0}\n    at {1}")]
    Unexpected(UnexpectedError, &'static Location<'static>),
    #[error("{0}")]
    BadState(String),
    #[error("{0}")]
    ImmediateRetry(Cow<'static, str>),
}

impl ToolError {
    pub fn bad_state<R>(reason: impl Into<String>) -> std::result::Result<R, Self> {
        Err(ToolError::BadState(reason.into()))
    }
    pub fn retry<R>(reason: impl Into<Cow<'static, str>>) -> std::result::Result<R, Self> {
        Err(ToolError::ImmediateRetry(reason.into()))
    }
}

impl From<anyhow::Error> for ToolError {
    #[track_caller]
    fn from(e: anyhow::Error) -> Self {
        ToolError::Unexpected(UnexpectedError::Contextual(e), Location::caller())
    }
}

impl From<std::io::Error> for ToolError {
    #[track_caller]
    fn from(e: std::io::Error) -> Self {
        ToolError::Unexpected(UnexpectedError::Io(e), Location::caller())
    }
}

pub type Result<T = ()> = std::result::Result<T, ToolError>;

pub trait ComponentStoreExt<T> {
    fn get_checked(&self, entity: &Entity) -> Result<T>;
}

impl<T> ComponentStoreExt<T> for ComponentStore<T>
where
    T: ComponentName + Pod,
{
    fn get_checked(&self, entity: &Entity) -> Result<T> {
        Ok(self
            .get(entity)
            .map_err(|e| anyhow!(e))
            .and_then(|c| c.context("Component missing"))
            .with_context(|| format!("Reading {} from entity {}", T::NAME, entity.id))?)
    }
}

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
