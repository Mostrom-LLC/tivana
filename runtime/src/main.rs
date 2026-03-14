//! Tivana - Streaming browser perception protocol for AI agents
//!
//! This is the Rust runtime that launches Chromium, streams page state via WebSocket,
//! and executes agent actions.

mod cli;
mod error;
mod protocol;
mod server;
mod session;
mod browser;
mod perceive;
mod act;

use clap::Parser;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::cli::Args;
use crate::server::Server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tivana=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    info!(
        "Tivana v{} starting on port {}",
        env!("CARGO_PKG_VERSION"),
        args.port
    );
    info!(
        "Browser mode: {}",
        if args.headless { "headless" } else { "headed" }
    );

    // Start the server
    let server = Server::new(args)?;
    server.run().await?;

    Ok(())
}
