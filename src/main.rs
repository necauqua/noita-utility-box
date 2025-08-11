#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{Context, Result, anyhow};
use app::NoitaUtilityBox;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, format::FmtSpan},
    prelude::*,
};

mod app;
mod orb_searcher;
mod tools;
mod update_check;
mod util;
mod widgets;

fn setup_logging() -> Result<WorkerGuard> {
    // attempt to attach to parent console, so that we have panics/logs when
    // started from cmd.exe regardless of windows_subsystem = "windows"
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Console::*;
        _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }

    let storage_dir = eframe::storage_dir(env!("CARGO_PKG_NAME")).context("No storage dir")?;
    std::fs::create_dir_all(&storage_dir).context("Failed to create storage dir")?;
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(storage_dir.join("latest.log"))?;

    let (file_writer, guard) = tracing_appender::non_blocking(log_file);
    tracing::subscriber::set_global_default(
        fmt::Subscriber::builder()
            .with_env_filter(
                EnvFilter::builder().parse(
                    std::env::var(EnvFilter::DEFAULT_ENV)
                        .as_deref()
                        .unwrap_or("info,wgpu_core=warn,wgpu_hal=warn,zbus=warn"),
                )?,
            )
            .with_span_events(FmtSpan::CLOSE)
            .finish()
            .with(
                fmt::Layer::default()
                    .with_ansi(false)
                    .with_writer(file_writer),
            ),
    )?;
    Ok(guard)
}

fn main() -> Result<()> {
    color_eyre::install().unwrap();

    let _guard = setup_logging()?;

    NoitaUtilityBox::run().map_err(|e| anyhow!("{e:#}"))?;

    Ok(())
}
