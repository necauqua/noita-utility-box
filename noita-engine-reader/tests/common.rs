use anyhow::{Context, Result};
use noita_engine_reader::{Noita, discovery::KnownBuild};
use sysinfo::ProcessesToUpdate;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

pub fn setup() -> Result<Noita> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env()?,
        )
        .try_init();

    let mut system = sysinfo::System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);

    Ok(Noita::lookup(KnownBuild::last().map())?.context("Noita process not found")?)
}
