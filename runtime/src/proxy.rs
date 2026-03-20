//! Proxy configuration and IP rotation for browser sessions
//!
//! Supports HTTP, HTTPS, and SOCKS5 proxies with optional authentication.
//! ProxyPool enables rotating through multiple proxies.

use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

/// Proxy protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyProtocol {
    Http,
    Https,
    Socks5,
}

impl Default for ProxyProtocol {
    fn default() -> Self {
        Self::Http
    }
}

impl fmt::Display for ProxyProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyProtocol::Http => write!(f, "http"),
            ProxyProtocol::Https => write!(f, "https"),
            ProxyProtocol::Socks5 => write!(f, "socks5"),
        }
    }
}

/// Proxy configuration for a browser session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Proxy server address (host:port)
    pub server: String,

    /// Proxy protocol
    #[serde(default)]
    pub protocol: ProxyProtocol,

    /// Username for authenticated proxies
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// Password for authenticated proxies
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

impl ProxyConfig {
    /// Build the Chrome --proxy-server flag value
    pub fn to_chrome_arg(&self) -> String {
        format!("--proxy-server={}://{}", self.protocol, self.server)
    }

    /// Whether this proxy requires authentication
    pub fn requires_auth(&self) -> bool {
        self.username.is_some() && self.password.is_some()
    }
}

/// A pool of proxies for rotation
pub struct ProxyPool {
    proxies: Vec<ProxyConfig>,
    current: AtomicUsize,
}

impl ProxyPool {
    /// Create a new proxy pool from a list of proxy configs
    pub fn from_list(proxies: Vec<ProxyConfig>) -> Self {
        Self {
            proxies,
            current: AtomicUsize::new(0),
        }
    }

    /// Get the current proxy without rotating
    pub fn current(&self) -> Option<&ProxyConfig> {
        if self.proxies.is_empty() {
            return None;
        }
        let idx = self.current.load(Ordering::Relaxed) % self.proxies.len();
        Some(&self.proxies[idx])
    }

    /// Rotate to the next proxy and return it
    pub fn next(&self) -> Option<&ProxyConfig> {
        if self.proxies.is_empty() {
            return None;
        }
        let idx = self.current.fetch_add(1, Ordering::Relaxed);
        let next_idx = (idx + 1) % self.proxies.len();
        Some(&self.proxies[next_idx])
    }

    /// Number of proxies in the pool
    pub fn len(&self) -> usize {
        self.proxies.len()
    }

    /// Whether the pool is empty
    pub fn is_empty(&self) -> bool {
        self.proxies.is_empty()
    }
}

// Manual Debug since AtomicUsize doesn't derive Debug in a useful way
impl fmt::Debug for ProxyPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProxyPool")
            .field("proxies", &self.proxies)
            .field("current", &self.current.load(Ordering::Relaxed))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_config_chrome_arg() {
        let config = ProxyConfig {
            server: "proxy.example.com:8080".to_string(),
            protocol: ProxyProtocol::Http,
            username: None,
            password: None,
        };
        assert_eq!(
            config.to_chrome_arg(),
            "--proxy-server=http://proxy.example.com:8080"
        );
    }

    #[test]
    fn test_proxy_config_socks5() {
        let config = ProxyConfig {
            server: "socks.example.com:1080".to_string(),
            protocol: ProxyProtocol::Socks5,
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
        };
        assert_eq!(
            config.to_chrome_arg(),
            "--proxy-server=socks5://socks.example.com:1080"
        );
        assert!(config.requires_auth());
    }

    #[test]
    fn test_proxy_config_no_auth() {
        let config = ProxyConfig {
            server: "proxy.example.com:8080".to_string(),
            protocol: ProxyProtocol::Http,
            username: None,
            password: None,
        };
        assert!(!config.requires_auth());
    }

    #[test]
    fn test_proxy_pool_rotation() {
        let pool = ProxyPool::from_list(vec![
            ProxyConfig {
                server: "proxy1:8080".to_string(),
                protocol: ProxyProtocol::Http,
                username: None,
                password: None,
            },
            ProxyConfig {
                server: "proxy2:8080".to_string(),
                protocol: ProxyProtocol::Http,
                username: None,
                password: None,
            },
            ProxyConfig {
                server: "proxy3:8080".to_string(),
                protocol: ProxyProtocol::Http,
                username: None,
                password: None,
            },
        ]);

        assert_eq!(pool.len(), 3);
        assert!(!pool.is_empty());

        // Current should be proxy1 (index 0)
        assert_eq!(pool.current().unwrap().server, "proxy1:8080");

        // Next rotates to proxy2
        assert_eq!(pool.next().unwrap().server, "proxy2:8080");

        // Next rotates to proxy3
        assert_eq!(pool.next().unwrap().server, "proxy3:8080");

        // Next wraps around to proxy1
        assert_eq!(pool.next().unwrap().server, "proxy1:8080");
    }

    #[test]
    fn test_proxy_pool_empty() {
        let pool = ProxyPool::from_list(vec![]);
        assert!(pool.is_empty());
        assert!(pool.current().is_none());
        assert!(pool.next().is_none());
    }

    #[test]
    fn test_proxy_protocol_default() {
        assert_eq!(ProxyProtocol::default(), ProxyProtocol::Http);
    }

    #[test]
    fn test_proxy_protocol_display() {
        assert_eq!(format!("{}", ProxyProtocol::Http), "http");
        assert_eq!(format!("{}", ProxyProtocol::Https), "https");
        assert_eq!(format!("{}", ProxyProtocol::Socks5), "socks5");
    }

    #[test]
    fn test_proxy_config_serialization() {
        let config = ProxyConfig {
            server: "proxy.example.com:8080".to_string(),
            protocol: ProxyProtocol::Socks5,
            username: Some("user".to_string()),
            password: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("socks5"));
        assert!(json.contains("proxy.example.com:8080"));
        // password is None so should be skipped
        assert!(!json.contains("password"));
    }
}
