//! Tivana - Streaming browser perception protocol for AI agents
//!
//! This is the Rust runtime that launches Chromium, streams page state via WebSocket,
//! and executes agent actions.

#![allow(clippy::result_large_err)]
#![allow(dead_code)]

mod act;
mod browser;
mod captcha;
mod cli;
mod error;
mod network;
mod perceive;
mod persistence;
mod protocol;
mod proxy;
mod server;
mod session;

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
    if args.use_default_browser {
        info!("Browser mode: default browser profile (highest trust)");
    } else if let Some(ref target) = args.connect {
        info!("Browser mode: connect to existing Chrome at {}", target);
    } else {
        info!(
            "Browser mode: {}",
            if args.headless { "headless" } else { "headed" }
        );
    }

    // Start the server
    let server = Server::new(args)?;
    server.run().await?;

    Ok(())
}
