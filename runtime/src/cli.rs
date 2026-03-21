//! CLI argument parsing with clap

use clap::Parser;
use std::path::PathBuf;

/// Tivana - Streaming browser perception protocol for AI agents
#[derive(Parser, Debug, Clone)]
#[command(name = "tivana")]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Port to run the WebSocket server on
    #[arg(short, long, default_value = "9876")]
    pub port: u16,

    /// Run browser in headless mode
    #[arg(long, default_value = "false")]
    pub headless: bool,

    /// Run browser in headed mode (visible window)
    #[arg(long, default_value = "true", conflicts_with = "headless")]
    pub headed: bool,

    /// Path to Chrome/Chromium executable
    #[arg(long)]
    pub chrome_path: Option<PathBuf>,

    /// Host to bind the WebSocket server to
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Connect to an existing Chrome instance instead of launching a new one.
    /// Provide the debugging port (e.g., --connect 9222) or full WebSocket URL.
    #[arg(long)]
    pub connect: Option<String>,

    /// Chrome user data directory. Defaults to a temporary profile.
    /// Use your real Chrome profile for higher reCAPTCHA trust scores:
    ///   --user-data-dir "$HOME/Library/Application Support/Google/Chrome"
    #[arg(long)]
    pub user_data_dir: Option<PathBuf>,

    /// Use your default browser profile instead of an isolated one.
    /// Quits any running Chrome, relaunches with debugging enabled using your
    /// real profile (cookies, logins, history). Gives highest reCAPTCHA trust.
    #[arg(long)]
    pub use_default_browser: bool,
}

impl Args {
    /// Returns true if browser should run in headless mode
    pub fn is_headless(&self) -> bool {
        self.headless || !self.headed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_args() {
        let args = Args::parse_from(["tivana"]);
        assert_eq!(args.port, 9876);
        assert!(!args.headless);
        assert!(args.headed);
        assert_eq!(args.host, "127.0.0.1");
    }

    #[test]
    fn test_custom_port() {
        let args = Args::parse_from(["tivana", "--port", "8080"]);
        assert_eq!(args.port, 8080);
    }

    #[test]
    fn test_headless_mode() {
        let args = Args::parse_from(["tivana", "--headless"]);
        assert!(args.headless);
        assert!(args.is_headless());
    }
}
