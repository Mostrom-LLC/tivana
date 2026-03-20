//! Zero-config CAPTCHA detection and solving
//!
//! Automatically detects and solves CAPTCHAs without API keys or paid services.
//! Supports reCAPTCHA v2/v3, hCaptcha, and Cloudflare Turnstile.
//!
//! Strategy:
//! 1. Stealth solve — click the checkbox (works when fingerprint is trusted)
//! 2. Audio solve — transcribe audio challenge via Whisper (fallback for reCAPTCHA v2)

use std::time::Instant;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::browser::PageHandle;
use crate::error::ProtocolError;

/// Types of CAPTCHA that can be detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptchaType {
    RecaptchaV2,
    RecaptchaV3,
    #[serde(rename = "hcaptcha")]
    HCaptcha,
    Turnstile,
    None,
}

/// Information about a detected CAPTCHA
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptchaInfo {
    pub captcha_type: CaptchaType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sitekey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iframe_selector: Option<String>,
    pub is_visible: bool,
}

/// Result of a CAPTCHA solve attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptchaSolveResult {
    pub solved: bool,
    pub method: String,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Zero-config CAPTCHA solver
pub struct CaptchaSolver;

impl CaptchaSolver {
    /// Detect CAPTCHA presence on the current page
    pub async fn detect(page: &PageHandle) -> Result<CaptchaInfo, ProtocolError> {
        let result: serde_json::Value = page
            .evaluate(DETECT_CAPTCHA_JS)
            .await
            .map_err(|e| ProtocolError::internal(format!("CAPTCHA detection failed: {}", e)))?;

        let captcha_type = match result.get("type").and_then(|v| v.as_str()) {
            Some("recaptcha_v2") => CaptchaType::RecaptchaV2,
            Some("recaptcha_v3") => CaptchaType::RecaptchaV3,
            Some("hcaptcha") => CaptchaType::HCaptcha,
            Some("turnstile") => CaptchaType::Turnstile,
            _ => CaptchaType::None,
        };

        Ok(CaptchaInfo {
            captcha_type,
            sitekey: result
                .get("sitekey")
                .and_then(|v| v.as_str())
                .map(String::from),
            iframe_selector: result
                .get("iframeSelector")
                .and_then(|v| v.as_str())
                .map(String::from),
            is_visible: result
                .get("isVisible")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        })
    }

    /// Attempt to solve any detected CAPTCHA automatically
    pub async fn solve(page: &PageHandle) -> Result<CaptchaSolveResult, ProtocolError> {
        let start = Instant::now();

        let info = Self::detect(page).await?;

        match info.captcha_type {
            CaptchaType::None => Ok(CaptchaSolveResult {
                solved: true,
                method: "none".to_string(),
                duration_ms: start.elapsed().as_millis() as u64,
                error: None,
            }),
            CaptchaType::RecaptchaV2 => Self::solve_recaptcha_v2(page, &info, start).await,
            CaptchaType::RecaptchaV3 => {
                // reCAPTCHA v3 is score-based and invisible — stealth patches handle it
                Ok(CaptchaSolveResult {
                    solved: true,
                    method: "stealth".to_string(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: None,
                })
            }
            CaptchaType::HCaptcha => Self::solve_hcaptcha(page, &info, start).await,
            CaptchaType::Turnstile => Self::solve_turnstile(page, &info, start).await,
        }
    }

    /// Solve reCAPTCHA v2 — stealth click first, audio fallback
    async fn solve_recaptcha_v2(
        page: &PageHandle,
        _info: &CaptchaInfo,
        start: Instant,
    ) -> Result<CaptchaSolveResult, ProtocolError> {
        info!("Attempting reCAPTCHA v2 stealth solve");

        // Step 1: Find the reCAPTCHA anchor iframe and click the checkbox
        let click_result: serde_json::Value = page
            .evaluate(RECAPTCHA_STEALTH_CLICK_JS)
            .await
            .map_err(|e| ProtocolError::internal(format!("reCAPTCHA click failed: {}", e)))?;

        if click_result
            .get("error")
            .and_then(|v| v.as_str())
            .is_some()
        {
            // Could not find the iframe — try coordinate-based click
            let coord_result = Self::click_recaptcha_by_coords(page).await;
            if let Err(e) = coord_result {
                return Ok(CaptchaSolveResult {
                    solved: false,
                    method: "stealth".to_string(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: Some(format!("Could not locate reCAPTCHA checkbox: {}", e)),
                });
            }
        }

        // Wait for the checkbox to settle
        tokio::time::sleep(tokio::time::Duration::from_millis(2500)).await;

        // Check if solved
        let solved: bool = page
            .evaluate(CHECK_RECAPTCHA_SOLVED_JS)
            .await
            .unwrap_or(false);

        if solved {
            info!("reCAPTCHA v2 solved via stealth click");
            return Ok(CaptchaSolveResult {
                solved: true,
                method: "stealth".to_string(),
                duration_ms: start.elapsed().as_millis() as u64,
                error: None,
            });
        }

        // Step 2: Stealth click didn't work — try audio challenge
        info!("Stealth click triggered challenge, attempting audio solve");
        Self::solve_recaptcha_audio(page, start).await
    }

    /// Click reCAPTCHA checkbox by finding the iframe position and dispatching click
    async fn click_recaptcha_by_coords(page: &PageHandle) -> Result<(), ProtocolError> {
        let bounds: serde_json::Value = page
            .evaluate(GET_RECAPTCHA_IFRAME_BOUNDS_JS)
            .await
            .map_err(|e| {
                ProtocolError::internal(format!("Failed to get reCAPTCHA bounds: {}", e))
            })?;

        let x = bounds
            .get("x")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ProtocolError::internal("No x coordinate for reCAPTCHA iframe"))?;
        let y = bounds
            .get("y")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ProtocolError::internal("No y coordinate for reCAPTCHA iframe"))?;

        // The checkbox is roughly at offset (27, 30) inside the anchor iframe
        let click_x = x + 27.0;
        let click_y = y + 30.0;

        page.click_at(click_x, click_y).await.map_err(|e| {
            ProtocolError::internal(format!("reCAPTCHA coordinate click failed: {}", e))
        })?;

        Ok(())
    }

    /// Solve reCAPTCHA v2 via audio challenge + Whisper transcription
    async fn solve_recaptcha_audio(
        page: &PageHandle,
        start: Instant,
    ) -> Result<CaptchaSolveResult, ProtocolError> {
        // Click the audio button in the challenge iframe
        let audio_click: serde_json::Value = page
            .evaluate(RECAPTCHA_CLICK_AUDIO_JS)
            .await
            .map_err(|e| {
                ProtocolError::internal(format!("Failed to click audio button: {}", e))
            })?;

        if let Some(err) = audio_click.get("error").and_then(|v| v.as_str()) {
            return Ok(CaptchaSolveResult {
                solved: false,
                method: "audio".to_string(),
                duration_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("Could not access audio challenge: {}", err)),
            });
        }

        // Wait for audio to load
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

        // Extract the audio URL
        let audio_url: String = page
            .evaluate(GET_RECAPTCHA_AUDIO_URL_JS)
            .await
            .map_err(|e| {
                ProtocolError::internal(format!("Failed to get audio URL: {}", e))
            })?;

        if audio_url.is_empty() {
            return Ok(CaptchaSolveResult {
                solved: false,
                method: "audio".to_string(),
                duration_ms: start.elapsed().as_millis() as u64,
                error: Some("Audio source URL not found".to_string()),
            });
        }

        // Download the audio file
        let temp_dir = std::env::temp_dir();
        let audio_path = temp_dir.join("tivana_captcha_audio.mp3");

        let audio_bytes = reqwest::get(&audio_url)
            .await
            .map_err(|e| ProtocolError::internal(format!("Audio download failed: {}", e)))?
            .bytes()
            .await
            .map_err(|e| ProtocolError::internal(format!("Audio read failed: {}", e)))?;

        tokio::fs::write(&audio_path, &audio_bytes)
            .await
            .map_err(|e| ProtocolError::internal(format!("Audio save failed: {}", e)))?;

        // Transcribe with Whisper
        let transcription = Self::transcribe_audio(&audio_path).await?;

        if transcription.is_empty() {
            return Ok(CaptchaSolveResult {
                solved: false,
                method: "audio".to_string(),
                duration_ms: start.elapsed().as_millis() as u64,
                error: Some("Whisper transcription returned empty result".to_string()),
            });
        }

        info!(transcription = %transcription, "Audio transcription complete");

        // bframe is cross-origin, so type into the audio response field using coordinates
        let typed = Self::type_in_recaptcha_audio(page, &transcription).await;

        if let Err(e) = typed {
            return Ok(CaptchaSolveResult {
                solved: false,
                method: "audio".to_string(),
                duration_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("Failed to type transcription: {}", e)),
            });
        }

        // Click verify (best-effort, coordinate-based)
        let _ = Self::click_recaptcha_verify(page).await;

        // Wait for verification
        tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;

        // Check if solved
        let solved: bool = page
            .evaluate(CHECK_RECAPTCHA_SOLVED_JS)
            .await
            .unwrap_or(false);

        // Cleanup temp file
        let _ = tokio::fs::remove_file(&audio_path).await;

        Ok(CaptchaSolveResult {
            solved,
            method: "audio".to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
            error: if solved {
                None
            } else {
                Some("Verification failed after audio transcription".to_string())
            },
        })
    }

    /// Type transcription into the reCAPTCHA audio response field using coordinates
    async fn type_in_recaptcha_audio(
        page: &PageHandle,
        text: &str,
    ) -> Result<(), ProtocolError> {
        // Get the bframe position and the audio-response input offset within it
        let bounds: serde_json::Value = page
            .evaluate(GET_RECAPTCHA_BFRAME_BOUNDS_JS)
            .await
            .map_err(|e| {
                ProtocolError::internal(format!("Failed to get bframe bounds: {}", e))
            })?;

        let x = bounds
            .get("inputX")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ProtocolError::internal("No input coordinates in bframe"))?;
        let y = bounds
            .get("inputY")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ProtocolError::internal("No input coordinates in bframe"))?;

        // Click the input field to focus it
        page.click_at(x, y).await.map_err(|e| {
            ProtocolError::internal(format!("Failed to click audio input: {}", e))
        })?;

        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // Type the transcription
        page.type_text(text).await.map_err(|e| {
            ProtocolError::internal(format!("Failed to type transcription: {}", e))
        })?;

        Ok(())
    }

    /// Click the reCAPTCHA verify button using coordinates
    async fn click_recaptcha_verify(page: &PageHandle) -> Result<(), ProtocolError> {
        let bounds: serde_json::Value = page
            .evaluate(GET_RECAPTCHA_VERIFY_BOUNDS_JS)
            .await
            .map_err(|e| {
                ProtocolError::internal(format!("Failed to get verify button bounds: {}", e))
            })?;

        let x = bounds
            .get("x")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ProtocolError::internal("No verify button coordinates"))?;
        let y = bounds
            .get("y")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ProtocolError::internal("No verify button coordinates"))?;

        page.click_at(x, y).await.map_err(|e| {
            ProtocolError::internal(format!("Failed to click verify: {}", e))
        })?;

        Ok(())
    }

    /// Transcribe audio using Whisper CLI
    async fn transcribe_audio(
        audio_path: &std::path::Path,
    ) -> Result<String, ProtocolError> {
        let audio_str = audio_path.to_string_lossy();
        let output_dir = audio_path.parent().unwrap_or(std::path::Path::new("/tmp"));
        let stem = audio_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();

        // Try whisper CLI directly
        let result = tokio::process::Command::new("whisper")
            .args([
                &*audio_str,
                "--model",
                "base",
                "--output_format",
                "txt",
                "--output_dir",
                &*output_dir.to_string_lossy(),
                "--language",
                "en",
            ])
            .output()
            .await;

        if let Ok(output) = result {
            if output.status.success() {
                let txt_path = output_dir.join(format!("{}.txt", stem));
                if let Ok(text) = tokio::fs::read_to_string(&txt_path).await {
                    let _ = tokio::fs::remove_file(&txt_path).await;
                    return Ok(text.trim().to_string());
                }
            }
        }

        // Try python3 -m whisper
        debug!("whisper CLI not found, trying python3 -m whisper");
        let result = tokio::process::Command::new("python3")
            .args([
                "-m",
                "whisper",
                &*audio_str,
                "--model",
                "base",
                "--output_format",
                "txt",
                "--output_dir",
                &*output_dir.to_string_lossy(),
                "--language",
                "en",
            ])
            .output()
            .await;

        if let Ok(output) = result {
            if output.status.success() {
                let txt_path = output_dir.join(format!("{}.txt", stem));
                if let Ok(text) = tokio::fs::read_to_string(&txt_path).await {
                    let _ = tokio::fs::remove_file(&txt_path).await;
                    return Ok(text.trim().to_string());
                }
            }
        }

        // Try installing whisper via pip
        warn!("Whisper not found, attempting pip install");
        let install = tokio::process::Command::new("pip3")
            .args(["install", "-q", "openai-whisper"])
            .output()
            .await;

        if let Ok(output) = install {
            if output.status.success() {
                // Retry with whisper CLI
                let result = tokio::process::Command::new("whisper")
                    .args([
                        &*audio_str,
                        "--model",
                        "base",
                        "--output_format",
                        "txt",
                        "--output_dir",
                        &*output_dir.to_string_lossy(),
                        "--language",
                        "en",
                    ])
                    .output()
                    .await;

                if let Ok(output) = result {
                    if output.status.success() {
                        let txt_path = output_dir.join(format!("{}.txt", stem));
                        if let Ok(text) = tokio::fs::read_to_string(&txt_path).await {
                            let _ = tokio::fs::remove_file(&txt_path).await;
                            return Ok(text.trim().to_string());
                        }
                    }
                }
            }
        }

        Err(ProtocolError::internal(
            "Whisper not available. Install with: pip install openai-whisper",
        ))
    }

    /// Solve hCaptcha — click the checkbox
    async fn solve_hcaptcha(
        page: &PageHandle,
        _info: &CaptchaInfo,
        start: Instant,
    ) -> Result<CaptchaSolveResult, ProtocolError> {
        info!("Attempting hCaptcha stealth solve");

        let bounds: serde_json::Value = page
            .evaluate(GET_HCAPTCHA_BOUNDS_JS)
            .await
            .map_err(|e| {
                ProtocolError::internal(format!("Failed to get hCaptcha bounds: {}", e))
            })?;

        if let Some(err) = bounds.get("error").and_then(|v| v.as_str()) {
            return Ok(CaptchaSolveResult {
                solved: false,
                method: "stealth".to_string(),
                duration_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("hCaptcha iframe not found: {}", err)),
            });
        }

        let x = bounds.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let y = bounds.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);

        // Click the hCaptcha checkbox (offset ~30, 30 inside iframe)
        page.click_at(x + 30.0, y + 30.0).await.map_err(|e| {
            ProtocolError::internal(format!("hCaptcha click failed: {}", e))
        })?;

        tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;

        // Check if solved
        let solved: bool = page
            .evaluate(CHECK_HCAPTCHA_SOLVED_JS)
            .await
            .unwrap_or(false);

        Ok(CaptchaSolveResult {
            solved,
            method: "stealth".to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
            error: if solved {
                None
            } else {
                Some("hCaptcha challenge appeared — manual solving required".to_string())
            },
        })
    }

    /// Solve Cloudflare Turnstile — click the widget
    async fn solve_turnstile(
        page: &PageHandle,
        _info: &CaptchaInfo,
        start: Instant,
    ) -> Result<CaptchaSolveResult, ProtocolError> {
        info!("Attempting Turnstile stealth solve");

        let bounds: serde_json::Value = page
            .evaluate(GET_TURNSTILE_BOUNDS_JS)
            .await
            .map_err(|e| {
                ProtocolError::internal(format!("Failed to get Turnstile bounds: {}", e))
            })?;

        if let Some(err) = bounds.get("error").and_then(|v| v.as_str()) {
            return Ok(CaptchaSolveResult {
                solved: false,
                method: "stealth".to_string(),
                duration_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("Turnstile widget not found: {}", err)),
            });
        }

        let x = bounds.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let y = bounds.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let w = bounds.get("width").and_then(|v| v.as_f64()).unwrap_or(300.0);
        let h = bounds.get("height").and_then(|v| v.as_f64()).unwrap_or(65.0);

        // Click center of the Turnstile widget
        page.click_at(x + w / 2.0, y + h / 2.0)
            .await
            .map_err(|e| {
                ProtocolError::internal(format!("Turnstile click failed: {}", e))
            })?;

        // Turnstile usually resolves within a few seconds
        tokio::time::sleep(tokio::time::Duration::from_millis(4000)).await;

        let solved: bool = page
            .evaluate(CHECK_TURNSTILE_SOLVED_JS)
            .await
            .unwrap_or(false);

        Ok(CaptchaSolveResult {
            solved,
            method: "stealth".to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
            error: if solved {
                None
            } else {
                Some("Turnstile challenge not resolved".to_string())
            },
        })
    }
}

// =============================================================================
// JavaScript snippets for CAPTCHA detection and interaction
// =============================================================================

/// Detect CAPTCHA type, sitekey, and visibility
const DETECT_CAPTCHA_JS: &str = r#"(() => {
    const result = { type: 'none', sitekey: null, iframeSelector: null, isVisible: false };

    // reCAPTCHA v2: look for anchor iframe
    const recaptchaIframe = document.querySelector(
        'iframe[src*="google.com/recaptcha/api2/anchor"], iframe[src*="recaptcha/enterprise/anchor"]'
    );
    if (recaptchaIframe) {
        result.type = 'recaptcha_v2';
        const src = recaptchaIframe.src;
        const kMatch = src.match(/[?&]k=([^&]+)/);
        if (kMatch) result.sitekey = kMatch[1];
        result.iframeSelector = 'iframe[src*="recaptcha/api2/anchor"], iframe[src*="recaptcha/enterprise/anchor"]';
        const rect = recaptchaIframe.getBoundingClientRect();
        result.isVisible = rect.width > 0 && rect.height > 0;
        return result;
    }

    // reCAPTCHA v2: look for div.g-recaptcha with data-sitekey
    const gRecaptcha = document.querySelector('.g-recaptcha[data-sitekey]');
    if (gRecaptcha) {
        result.type = 'recaptcha_v2';
        result.sitekey = gRecaptcha.getAttribute('data-sitekey');
        result.iframeSelector = '.g-recaptcha';
        const rect = gRecaptcha.getBoundingClientRect();
        result.isVisible = rect.width > 0 && rect.height > 0;
        return result;
    }

    // reCAPTCHA v3: invisible, script-based
    const recaptchaV3Script = document.querySelector(
        'script[src*="recaptcha/api.js"][src*="render="]'
    );
    if (recaptchaV3Script) {
        result.type = 'recaptcha_v3';
        const renderMatch = recaptchaV3Script.src.match(/render=([^&]+)/);
        if (renderMatch) result.sitekey = renderMatch[1];
        result.isVisible = false;
        return result;
    }

    // hCaptcha
    const hcaptchaIframe = document.querySelector('iframe[src*="hcaptcha.com"]');
    if (hcaptchaIframe) {
        result.type = 'hcaptcha';
        const hcDiv = document.querySelector('.h-captcha[data-sitekey]');
        if (hcDiv) result.sitekey = hcDiv.getAttribute('data-sitekey');
        result.iframeSelector = 'iframe[src*="hcaptcha.com"]';
        const rect = hcaptchaIframe.getBoundingClientRect();
        result.isVisible = rect.width > 0 && rect.height > 0;
        return result;
    }

    // Cloudflare Turnstile
    const turnstileScript = document.querySelector(
        'script[src*="challenges.cloudflare.com/turnstile"]'
    );
    const turnstileIframe = document.querySelector(
        'iframe[src*="challenges.cloudflare.com"]'
    );
    const turnstileDiv = document.querySelector('.cf-turnstile[data-sitekey]');
    if (turnstileScript || turnstileIframe || turnstileDiv) {
        result.type = 'turnstile';
        if (turnstileDiv) result.sitekey = turnstileDiv.getAttribute('data-sitekey');
        if (turnstileIframe) {
            result.iframeSelector = 'iframe[src*="challenges.cloudflare.com"]';
            const rect = turnstileIframe.getBoundingClientRect();
            result.isVisible = rect.width > 0 && rect.height > 0;
        } else if (turnstileDiv) {
            result.iframeSelector = '.cf-turnstile';
            const rect = turnstileDiv.getBoundingClientRect();
            result.isVisible = rect.width > 0 && rect.height > 0;
        }
        return result;
    }

    return result;
})()"#;

/// Click the reCAPTCHA checkbox by programmatically triggering events on the anchor iframe.
/// Since the iframe is cross-origin, we use postMessage or coordinate-based fallback.
const RECAPTCHA_STEALTH_CLICK_JS: &str = r#"(() => {
    const iframe = document.querySelector(
        'iframe[src*="google.com/recaptcha/api2/anchor"], iframe[src*="recaptcha/enterprise/anchor"]'
    );
    if (!iframe) return { error: 'anchor iframe not found' };

    const rect = iframe.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return { error: 'iframe not visible' };

    return {
        x: rect.left,
        y: rect.top,
        width: rect.width,
        height: rect.height
    };
})()"#;

/// Get reCAPTCHA anchor iframe bounding rect
const GET_RECAPTCHA_IFRAME_BOUNDS_JS: &str = r#"(() => {
    const iframe = document.querySelector(
        'iframe[src*="google.com/recaptcha/api2/anchor"], iframe[src*="recaptcha/enterprise/anchor"]'
    );
    if (!iframe) return { error: 'not found' };
    const rect = iframe.getBoundingClientRect();
    return { x: rect.left, y: rect.top, width: rect.width, height: rect.height };
})()"#;

/// Check if reCAPTCHA is solved by looking for the response textarea having a value
const CHECK_RECAPTCHA_SOLVED_JS: &str = r#"(() => {
    const textarea = document.querySelector('#g-recaptcha-response, [name="g-recaptcha-response"]');
    if (textarea && textarea.value && textarea.value.length > 0) return true;

    // Also check for the recaptcha-checkbox-checked class via the anchor iframe
    const iframe = document.querySelector(
        'iframe[src*="google.com/recaptcha/api2/anchor"], iframe[src*="recaptcha/enterprise/anchor"]'
    );
    if (iframe) {
        try {
            const doc = iframe.contentDocument;
            if (doc) {
                const anchor = doc.querySelector('.recaptcha-checkbox-checked, #recaptcha-anchor[aria-checked="true"]');
                if (anchor) return true;
            }
        } catch(e) { /* cross-origin */ }
    }

    return false;
})()"#;

/// Click the audio challenge button in the reCAPTCHA bframe.
/// Since it's cross-origin, we return the bframe bounds for coordinate-based clicking.
const RECAPTCHA_CLICK_AUDIO_JS: &str = r#"(() => {
    const bframe = document.querySelector(
        'iframe[src*="recaptcha/api2/bframe"], iframe[src*="recaptcha/enterprise/bframe"]'
    );
    if (!bframe) return { error: 'bframe not found' };
    const rect = bframe.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return { error: 'bframe not visible' };

    // The audio button is typically in the bottom-left area of the challenge popup
    // Approximate position: ~55px from left, ~310px from top in a standard challenge
    return {
        x: rect.left + 55,
        y: rect.top + 310,
        frameX: rect.left,
        frameY: rect.top,
        frameWidth: rect.width,
        frameHeight: rect.height
    };
})()"#;

/// Get the audio source URL from the reCAPTCHA bframe.
/// Since bframe is cross-origin, we look for the audio URL through alternative means.
const GET_RECAPTCHA_AUDIO_URL_JS: &str = r#"(() => {
    // The audio challenge creates a link with class 'rc-audiochallenge-tdownload-link'
    // Since the bframe is cross-origin, we try to find it via network requests
    // Alternatively, look for an audio element in the bframe

    // Check if there's an accessible audio element
    const bframe = document.querySelector(
        'iframe[src*="recaptcha/api2/bframe"], iframe[src*="recaptcha/enterprise/bframe"]'
    );
    if (!bframe) return '';

    try {
        const doc = bframe.contentDocument;
        if (doc) {
            const audio = doc.querySelector('#audio-source, audio source, .rc-audiochallenge-tdownload-link');
            if (audio) return audio.src || audio.href || '';
        }
    } catch(e) { /* cross-origin */ }

    return '';
})()"#;

/// Get the reCAPTCHA bframe bounds and audio response input position
const GET_RECAPTCHA_BFRAME_BOUNDS_JS: &str = r#"(() => {
    const bframe = document.querySelector(
        'iframe[src*="recaptcha/api2/bframe"], iframe[src*="recaptcha/enterprise/bframe"]'
    );
    if (!bframe) return { error: 'bframe not found' };
    const rect = bframe.getBoundingClientRect();

    // The audio-response input is roughly centered horizontally, ~200px from top
    return {
        frameX: rect.left,
        frameY: rect.top,
        inputX: rect.left + rect.width / 2,
        inputY: rect.top + 200,
        verifyX: rect.left + rect.width / 2,
        verifyY: rect.top + 280
    };
})()"#;

/// Get verify button bounds in reCAPTCHA bframe
const GET_RECAPTCHA_VERIFY_BOUNDS_JS: &str = r#"(() => {
    const bframe = document.querySelector(
        'iframe[src*="recaptcha/api2/bframe"], iframe[src*="recaptcha/enterprise/bframe"]'
    );
    if (!bframe) return { error: 'bframe not found' };
    const rect = bframe.getBoundingClientRect();

    // Verify button is at the bottom-right of the challenge
    return {
        x: rect.left + rect.width - 80,
        y: rect.top + rect.height - 35
    };
})()"#;

/// Get hCaptcha iframe bounds
const GET_HCAPTCHA_BOUNDS_JS: &str = r#"(() => {
    const iframe = document.querySelector('iframe[src*="hcaptcha.com/captcha"]');
    if (!iframe) {
        // Try the container div
        const container = document.querySelector('.h-captcha iframe');
        if (!container) return { error: 'hCaptcha iframe not found' };
        const rect = container.getBoundingClientRect();
        return { x: rect.left, y: rect.top, width: rect.width, height: rect.height };
    }
    const rect = iframe.getBoundingClientRect();
    return { x: rect.left, y: rect.top, width: rect.width, height: rect.height };
})()"#;

/// Check if hCaptcha is solved
const CHECK_HCAPTCHA_SOLVED_JS: &str = r#"(() => {
    const textarea = document.querySelector('[name="h-captcha-response"], [name="g-recaptcha-response"]');
    if (textarea && textarea.value && textarea.value.length > 0) return true;

    const iframe = document.querySelector('iframe[src*="hcaptcha.com"]');
    if (iframe) {
        try {
            const doc = iframe.contentDocument;
            if (doc) {
                const checked = doc.querySelector('[aria-checked="true"]');
                if (checked) return true;
            }
        } catch(e) { /* cross-origin */ }
    }

    return false;
})()"#;

/// Get Turnstile widget bounds
const GET_TURNSTILE_BOUNDS_JS: &str = r#"(() => {
    // Try the iframe first
    const iframe = document.querySelector('iframe[src*="challenges.cloudflare.com"]');
    if (iframe) {
        const rect = iframe.getBoundingClientRect();
        if (rect.width > 0 && rect.height > 0) {
            return { x: rect.left, y: rect.top, width: rect.width, height: rect.height };
        }
    }

    // Try the container div
    const container = document.querySelector('.cf-turnstile');
    if (container) {
        const rect = container.getBoundingClientRect();
        return { x: rect.left, y: rect.top, width: rect.width, height: rect.height };
    }

    return { error: 'Turnstile widget not found' };
})()"#;

/// Check if Turnstile is solved
const CHECK_TURNSTILE_SOLVED_JS: &str = r#"(() => {
    // Check for the response input
    const input = document.querySelector('[name="cf-turnstile-response"]');
    if (input && input.value && input.value.length > 0) return true;

    // Check for success state on the container
    const container = document.querySelector('.cf-turnstile');
    if (container && container.getAttribute('data-status') === 'solved') return true;

    // Check for turnstile callback having been invoked
    if (window.turnstile) {
        try {
            const widgets = document.querySelectorAll('.cf-turnstile');
            for (const w of widgets) {
                const widgetId = w.getAttribute('data-widget-id');
                if (widgetId) {
                    const response = window.turnstile.getResponse(widgetId);
                    if (response && response.length > 0) return true;
                }
            }
        } catch(e) { /* ignore */ }
    }

    return false;
})()"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_captcha_type_serialization() {
        let info = CaptchaInfo {
            captcha_type: CaptchaType::RecaptchaV2,
            sitekey: Some("6LcX...".to_string()),
            iframe_selector: Some("iframe[src*=\"recaptcha\"]".to_string()),
            is_visible: true,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("recaptcha_v2"));
        assert!(json.contains("6LcX"));
        assert!(json.contains("isVisible"));
    }

    #[test]
    fn test_captcha_type_none_serialization() {
        let info = CaptchaInfo {
            captcha_type: CaptchaType::None,
            sitekey: None,
            iframe_selector: None,
            is_visible: false,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"captchaType\":\"none\""));
        // sitekey and iframeSelector should be omitted
        assert!(!json.contains("sitekey"));
        assert!(!json.contains("iframeSelector"));
    }

    #[test]
    fn test_solve_result_serialization() {
        let result = CaptchaSolveResult {
            solved: true,
            method: "stealth".to_string(),
            duration_ms: 2500,
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"solved\":true"));
        assert!(json.contains("\"method\":\"stealth\""));
        assert!(json.contains("\"durationMs\":2500"));
        assert!(!json.contains("error"));
    }

    #[test]
    fn test_solve_result_with_error() {
        let result = CaptchaSolveResult {
            solved: false,
            method: "audio".to_string(),
            duration_ms: 5000,
            error: Some("Whisper not available".to_string()),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"solved\":false"));
        assert!(json.contains("Whisper not available"));
    }

    #[test]
    fn test_all_captcha_types() {
        let types = vec![
            (CaptchaType::RecaptchaV2, "recaptcha_v2"),
            (CaptchaType::RecaptchaV3, "recaptcha_v3"),
            (CaptchaType::HCaptcha, "hcaptcha"),
            (CaptchaType::Turnstile, "turnstile"),
            (CaptchaType::None, "none"),
        ];

        for (captcha_type, expected) in types {
            let json = serde_json::to_value(captcha_type).unwrap();
            assert_eq!(json.as_str().unwrap(), expected);
        }
    }

    #[test]
    fn test_captcha_type_deserialization() {
        let v2: CaptchaType = serde_json::from_str("\"recaptcha_v2\"").unwrap();
        assert_eq!(v2, CaptchaType::RecaptchaV2);

        let none: CaptchaType = serde_json::from_str("\"none\"").unwrap();
        assert_eq!(none, CaptchaType::None);
    }
}
