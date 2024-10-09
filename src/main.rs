#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{anyhow, Result};
use app::NoitaUtilityBox;
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

mod app;
mod orb_searcher;
mod tools;
mod util;

fn main() -> Result<()> {
    color_eyre::install().unwrap();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder().parse(
                std::env::var(EnvFilter::DEFAULT_ENV)
                    .as_deref()
                    .unwrap_or("info,wgpu_core=warn,wgpu_hal=warn,zbus=warn"),
            )?,
        )
        .with_span_events(FmtSpan::CLOSE)
        .init();

    NoitaUtilityBox::run().map_err(|e| anyhow!("{e:#}"))?;

    Ok(())
}
