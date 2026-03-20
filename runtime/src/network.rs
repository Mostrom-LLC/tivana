//! Network monitoring for capturing HTTP requests
//!
//! Injects a JavaScript interceptor that logs all fetch() and XMLHttpRequest
//! calls to `window.__tivana_network_log`. Provides methods to enable monitoring,
//! retrieve captured requests, and clear the log.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

use crate::browser::PageHandle;
use crate::error::TivanaError;

/// A captured network request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkRequest {
    /// Request URL
    pub url: String,

    /// HTTP method (GET, POST, etc.)
    pub method: String,

    /// HTTP status code (0 if pending/failed)
    pub status: u16,

    /// Response status text
    pub status_text: String,

    /// Request type (fetch or xhr)
    pub request_type: String,

    /// Timestamp in milliseconds since epoch
    pub timestamp_ms: f64,

    /// Response duration in milliseconds (0 if pending)
    pub duration_ms: f64,
}

/// Network monitoring manager
pub struct NetworkManager;

impl NetworkManager {
    /// Enable network monitoring by injecting the interceptor script.
    /// Safe to call multiple times — will not re-inject if already enabled.
    pub async fn enable(page: &Arc<PageHandle>) -> Result<(), TivanaError> {
        debug!("Enabling network monitoring");

        page.evaluate_void(
            r#"
            (() => {
                if (window.__tivana_network_log) return;
                window.__tivana_network_log = [];

                // Intercept fetch
                const origFetch = window.fetch;
                window.fetch = async function(...args) {
                    const url = typeof args[0] === 'string' ? args[0] : args[0]?.url || '';
                    const method = (args[1]?.method || 'GET').toUpperCase();
                    const start = Date.now();
                    const entry = {
                        url, method,
                        status: 0, statusText: '',
                        requestType: 'fetch',
                        timestampMs: start,
                        durationMs: 0,
                    };
                    window.__tivana_network_log.push(entry);
                    try {
                        const resp = await origFetch.apply(this, args);
                        entry.status = resp.status;
                        entry.statusText = resp.statusText;
                        entry.durationMs = Date.now() - start;
                        return resp;
                    } catch (e) {
                        entry.durationMs = Date.now() - start;
                        throw e;
                    }
                };

                // Intercept XMLHttpRequest
                const origOpen = XMLHttpRequest.prototype.open;
                const origSend = XMLHttpRequest.prototype.send;

                XMLHttpRequest.prototype.open = function(method, url, ...rest) {
                    this.__tivana = { method: method.toUpperCase(), url: String(url) };
                    return origOpen.apply(this, [method, url, ...rest]);
                };

                XMLHttpRequest.prototype.send = function(...args) {
                    const start = Date.now();
                    const entry = {
                        url: this.__tivana?.url || '',
                        method: this.__tivana?.method || 'GET',
                        status: 0, statusText: '',
                        requestType: 'xhr',
                        timestampMs: start,
                        durationMs: 0,
                    };
                    window.__tivana_network_log.push(entry);

                    this.addEventListener('loadend', () => {
                        entry.status = this.status;
                        entry.statusText = this.statusText;
                        entry.durationMs = Date.now() - start;
                    });

                    return origSend.apply(this, args);
                };
            })()
            "#,
        )
        .await?;

        Ok(())
    }

    /// Get captured network requests, optionally filtered by URL pattern.
    pub async fn get_requests(
        page: &Arc<PageHandle>,
        url_pattern: Option<&str>,
    ) -> Result<Vec<NetworkRequest>, TivanaError> {
        debug!(url_pattern, "Getting network requests");

        let requests: Vec<NetworkRequest> = if let Some(pattern) = url_pattern {
            let script = format!(
                r#"(window.__tivana_network_log || []).filter(r => r.url.includes("{}"))"#,
                pattern.replace('"', r#"\""#)
            );
            page.evaluate(&script).await?
        } else {
            page.evaluate("(window.__tivana_network_log || [])").await?
        };

        Ok(requests)
    }

    /// Clear all captured network requests.
    pub async fn clear(page: &Arc<PageHandle>) -> Result<(), TivanaError> {
        debug!("Clearing network requests");
        page.evaluate_void("if (window.__tivana_network_log) window.__tivana_network_log = [];")
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_request_serialization() {
        let req = NetworkRequest {
            url: "https://example.com/api".to_string(),
            method: "GET".to_string(),
            status: 200,
            status_text: "OK".to_string(),
            request_type: "fetch".to_string(),
            timestamp_ms: 1700000000000.0,
            duration_ms: 150.0,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("example.com"));
        assert!(json.contains("\"status\":200"));

        let parsed: NetworkRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.url, "https://example.com/api");
        assert_eq!(parsed.status, 200);
    }
}
