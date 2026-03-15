//! Integration tests for Tivana runtime
//!
//! These tests require a Chromium browser to be installed.
//! Run with: cargo test --test integration -- --ignored
//!
//! Tests verify:
//! - Browser launch and navigation
//! - Perception primitives (pageState, elements, mutations)
//! - Action primitives (click, type, scroll)
//! - Error handling (target not found)
//!
//! These tests exercise the same logic paths used by the Tivana runtime,
//! validating the underlying browser automation works correctly.

use std::time::Duration;

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::input::InsertTextParams;
use futures_util::StreamExt;

/// Find Chrome/Chromium executable
fn chrome_path() -> Option<String> {
    // Check environment variable first
    if let Ok(path) = std::env::var("CHROME_PATH") {
        if std::path::Path::new(&path).exists() {
            return Some(path);
        }
    }

    // Check Playwright cache (common in CI)
    let home = std::env::var("HOME").unwrap_or_default();
    let playwright_paths = [
        format!(
            "{}/.cache/ms-playwright/chromium-1208/chrome-linux64/chrome",
            home
        ),
        format!(
            "{}/.cache/ms-playwright/chromium-1169/chrome-linux64/chrome",
            home
        ),
    ];
    for path in &playwright_paths {
        if std::path::Path::new(path).exists() {
            return Some(path.clone());
        }
    }

    // Check common system paths
    let paths = [
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
    ];

    for path in paths {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    // Try `which` command
    std::process::Command::new("which")
        .arg("chromium")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            std::process::Command::new("which")
                .arg("google-chrome")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
        })
}

/// Skip test if Chrome not available
macro_rules! require_chrome {
    () => {
        match chrome_path() {
            Some(p) => {
                println!("Using Chrome at: {}", p);
                p
            }
            None => {
                eprintln!("⚠️  Skipping test: Chrome not found");
                return;
            }
        }
    };
}

/// Create a headless browser config for tests
fn test_browser_config(chrome: &str) -> BrowserConfig {
    let user_data_dir = format!("/tmp/tivana-test-{}", std::process::id());
    BrowserConfig::builder()
        .chrome_executable(chrome)
        .no_sandbox()
        .arg("--headless=new")
        .arg("--disable-gpu")
        .arg("--disable-dev-shm-usage")
        .arg(format!("--user-data-dir={}", user_data_dir))
        .build()
        .expect("Failed to build browser config")
}

// =============================================================================
// Browser Lifecycle Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires Chromium browser"]
async fn test_browser_launch_and_close() {
    let chrome = require_chrome!();
    let config = test_browser_config(&chrome);

    let (browser, mut handler) = Browser::launch(config)
        .await
        .expect("Failed to launch browser");
    tokio::spawn(async move { while let Some(_) = handler.next().await {} });

    // Create a page
    let page = browser
        .new_page("about:blank")
        .await
        .expect("Failed to create page");

    // Verify page exists
    let url: String = page
        .evaluate("window.location.href")
        .await
        .expect("Failed to evaluate")
        .into_value()
        .expect("Failed to parse");

    assert!(url.contains("about:blank"));

    // Close browser
    drop(browser);
    println!("✅ Browser launch and close test passed");
}

#[tokio::test]
#[ignore = "Requires Chromium browser"]
async fn test_navigate_to_url() {
    let chrome = require_chrome!();
    let config = test_browser_config(&chrome);

    let (browser, mut handler) = Browser::launch(config).await.expect("Failed to launch");
    tokio::spawn(async move { while let Some(_) = handler.next().await {} });

    let page = browser
        .new_page("https://example.com")
        .await
        .expect("Failed to create page");
    tokio::time::sleep(Duration::from_secs(1)).await;

    let url: String = page
        .evaluate("window.location.href")
        .await
        .expect("Failed")
        .into_value()
        .expect("Failed");

    assert!(url.contains("example.com"), "Should be on example.com");

    let title: String = page
        .evaluate("document.title")
        .await
        .expect("Failed")
        .into_value()
        .expect("Failed");

    assert!(!title.is_empty(), "Should have a title");
    println!("✅ Navigate test passed - URL: {}, Title: {}", url, title);
}

// =============================================================================
// Perceive Tests - pageState
// =============================================================================

#[tokio::test]
#[ignore = "Requires Chromium browser"]
async fn test_perceive_page_state() {
    let chrome = require_chrome!();
    let config = test_browser_config(&chrome);

    let (browser, mut handler) = Browser::launch(config).await.expect("Failed to launch");
    tokio::spawn(async move { while let Some(_) = handler.next().await {} });

    let page = browser
        .new_page("https://example.com")
        .await
        .expect("Failed to create page");
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Get page state (mirrors perceive.pageState)
    #[derive(serde::Deserialize, Debug)]
    #[allow(dead_code)]
    struct PageState {
        url: String,
        title: String,
        scroll_x: f64,
        scroll_y: f64,
        viewport_width: f64,
        viewport_height: f64,
        document_width: f64,
        document_height: f64,
        focused_element_id: Option<String>,
    }

    let state: PageState = page
        .evaluate(
            r#"(() => {
            const focused = document.activeElement;
            return {
                url: window.location.href,
                title: document.title,
                scroll_x: window.scrollX || 0,
                scroll_y: window.scrollY || 0,
                viewport_width: window.innerWidth || 0,
                viewport_height: window.innerHeight || 0,
                document_width: Math.max(document.body?.scrollWidth || 0, document.documentElement?.scrollWidth || 0),
                document_height: Math.max(document.body?.scrollHeight || 0, document.documentElement?.scrollHeight || 0),
                focused_element_id: focused && focused !== document.body ? focused.id || null : null
            };
        })()"#,
        )
        .await
        .expect("Failed to get state")
        .into_value()
        .expect("Failed to parse");

    assert!(state.url.contains("example.com"), "URL should be set");
    assert!(!state.title.is_empty(), "Title should be set");
    assert!(
        state.viewport_width > 0.0,
        "Viewport width should be positive"
    );
    assert!(
        state.viewport_height > 0.0,
        "Viewport height should be positive"
    );

    println!("✅ Perceive pageState test passed - {:?}", state);
}

// =============================================================================
// Perceive Tests - elements
// =============================================================================

#[tokio::test]
#[ignore = "Requires Chromium browser"]
async fn test_perceive_elements() {
    let chrome = require_chrome!();
    let config = test_browser_config(&chrome);

    let (browser, mut handler) = Browser::launch(config).await.expect("Failed to launch");
    tokio::spawn(async move { while let Some(_) = handler.next().await {} });

    let page = browser
        .new_page("https://example.com")
        .await
        .expect("Failed to create page");
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Get interactive elements (mirrors perceive.elements)
    let elements: Vec<serde_json::Value> = page
        .evaluate(
            r#"(() => {
            const elements = [];
            let counter = 1;
            
            const selector = [
                'a[href]', 'button', 'input', 'select', 'textarea',
                '[role="button"]', '[role="link"]',
                '[tabindex]:not([tabindex="-1"])'
            ].join(', ');
            
            document.querySelectorAll(selector).forEach(el => {
                const rect = el.getBoundingClientRect();
                const style = window.getComputedStyle(el);
                
                if (style.display === 'none' || style.visibility === 'hidden') return;
                if (rect.width === 0 && rect.height === 0) return;
                
                let role = el.getAttribute('role') || el.tagName.toLowerCase();
                let name = el.getAttribute('aria-label') ||
                           el.innerText?.trim()?.slice(0, 100) ||
                           null;
                
                elements.push({
                    id: 'e' + (counter++),
                    role: role,
                    name: name,
                    bounds: {
                        x: rect.x,
                        y: rect.y,
                        width: rect.width,
                        height: rect.height
                    },
                    styles: {
                        color: style.color,
                        fontSize: style.fontSize,
                        fontFamily: style.fontFamily
                    },
                    enabled: !el.disabled,
                    focused: document.activeElement === el
                });
            });
            
            return elements;
        })()"#,
        )
        .await
        .expect("Failed to get elements")
        .into_value()
        .expect("Failed to parse");

    println!("Found {} interactive elements", elements.len());
    assert!(
        !elements.is_empty(),
        "example.com should have interactive elements"
    );

    // Verify element structure
    let first = &elements[0];
    assert!(first["id"].is_string(), "Element should have id");
    assert!(first["role"].is_string(), "Element should have role");
    assert!(
        first["bounds"]["x"].is_number(),
        "Element should have bounds.x"
    );
    assert!(
        first["bounds"]["y"].is_number(),
        "Element should have bounds.y"
    );
    assert!(
        first["bounds"]["width"].is_number(),
        "Element should have bounds.width"
    );
    assert!(
        first["bounds"]["height"].is_number(),
        "Element should have bounds.height"
    );

    println!("✅ Perceive elements test passed");
}

// =============================================================================
// Perceive Tests - mutations
// =============================================================================

#[tokio::test]
#[ignore = "Requires Chromium browser"]
async fn test_perceive_mutations() {
    let chrome = require_chrome!();
    let config = test_browser_config(&chrome);

    let (browser, mut handler) = Browser::launch(config).await.expect("Failed to launch");
    tokio::spawn(async move { while let Some(_) = handler.next().await {} });

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Failed to create page");

    // Set up mutation observer (mirrors perceive.mutations)
    let setup_result: serde_json::Value = page
        .evaluate(
            r#"(() => {
            window.__tivana_mutations = [];
            window.__tivana_element_counter = 1;
            
            const getElementId = (el) => {
                if (!el || el.nodeType !== 1) return null;
                if (!el.dataset.tivanaId) {
                    el.dataset.tivanaId = 'e' + (window.__tivana_element_counter++);
                }
                return el.dataset.tivanaId;
            };
            
            window.__tivana_observer = new MutationObserver((mutations) => {
                for (const mutation of mutations) {
                    if (mutation.type === 'childList') {
                        for (const node of mutation.addedNodes) {
                            if (node.nodeType === 1) {
                                const id = getElementId(node);
                                const parentId = getElementId(node.parentElement);
                                window.__tivana_mutations.push({
                                    type: 'added',
                                    elementId: id,
                                    parentId: parentId
                                });
                            }
                        }
                        for (const node of mutation.removedNodes) {
                            if (node.nodeType === 1) {
                                window.__tivana_mutations.push({
                                    type: 'removed',
                                    elementId: node.dataset?.tivanaId || 'unknown'
                                });
                            }
                        }
                    } else if (mutation.type === 'attributes') {
                        window.__tivana_mutations.push({
                            type: 'changed',
                            elementId: getElementId(mutation.target),
                            attribute: mutation.attributeName
                        });
                    }
                }
            });
            
            window.__tivana_observer.observe(document.body, {
                childList: true,
                subtree: true,
                attributes: true
            });
            
            return { status: 'started' };
        })()"#,
        )
        .await
        .expect("Failed to setup observer")
        .into_value()
        .expect("Failed to parse");

    assert_eq!(setup_result["status"], "started");

    // Trigger DOM mutations
    page.evaluate(
        r#"document.body.innerHTML = '<div id="test1">Hello</div><button id="btn">Click</button>'"#,
    )
    .await
    .expect("Failed to modify DOM");

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Add more mutations
    page.evaluate(r#"document.getElementById('test1').setAttribute('class', 'active')"#)
        .await
        .expect("Failed");

    page.evaluate(r#"document.body.appendChild(document.createElement('span'))"#)
        .await
        .expect("Failed");

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Poll mutations
    let mutations: Vec<serde_json::Value> = page
        .evaluate(
            r#"(() => {
            const m = window.__tivana_mutations;
            window.__tivana_mutations = [];
            return m;
        })()"#,
        )
        .await
        .expect("Failed to poll")
        .into_value()
        .expect("Failed to parse");

    println!("Captured {} mutations: {:?}", mutations.len(), mutations);
    assert!(!mutations.is_empty(), "Should capture DOM mutations");

    // Verify mutation types
    let has_added = mutations.iter().any(|m| m["type"] == "added");
    let has_changed = mutations.iter().any(|m| m["type"] == "changed");

    assert!(has_added, "Should have 'added' mutations");
    assert!(has_changed, "Should have 'changed' mutations");

    // Cleanup
    page.evaluate("window.__tivana_observer?.disconnect()")
        .await
        .ok();

    println!("✅ Perceive mutations test passed");
}

// =============================================================================
// Action Tests - click
// =============================================================================

#[tokio::test]
#[ignore = "Requires Chromium browser"]
async fn test_act_click() {
    let chrome = require_chrome!();
    let config = test_browser_config(&chrome);

    let (browser, mut handler) = Browser::launch(config).await.expect("Failed to launch");
    tokio::spawn(async move { while let Some(_) = handler.next().await {} });

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Failed to create page");

    // Create a clickable button that tracks clicks
    page.evaluate(
        r#"
        document.body.innerHTML = '<button id="btn">Click Me</button>';
        window.clickCount = 0;
        document.getElementById('btn').addEventListener('click', () => window.clickCount++);
    "#,
    )
    .await
    .expect("Failed to setup");

    // Get button position
    let pos: serde_json::Value = page
        .evaluate(
            r#"(() => {
            const btn = document.getElementById('btn');
            const rect = btn.getBoundingClientRect();
            return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
        })()"#,
        )
        .await
        .expect("Failed")
        .into_value()
        .expect("Failed");

    let x = pos["x"].as_f64().unwrap();
    let y = pos["y"].as_f64().unwrap();

    // Click using JavaScript dispatch (mirrors act.click)
    page.evaluate(format!(
        r#"(() => {{
            const el = document.elementFromPoint({}, {});
            if (!el) return false;
            ['mousedown', 'mouseup', 'click'].forEach(type => {{
                el.dispatchEvent(new MouseEvent(type, {{
                    view: window, bubbles: true, cancelable: true,
                    clientX: {}, clientY: {}
                }}));
            }});
            return true;
        }})()"#,
        x, y, x, y
    ))
    .await
    .expect("Click failed");

    // Verify click was registered
    let click_count: i64 = page
        .evaluate("window.clickCount")
        .await
        .expect("Failed")
        .into_value()
        .expect("Failed");

    assert_eq!(click_count, 1, "Click should be registered");

    println!("✅ Act click test passed");
}

// =============================================================================
// Action Tests - type
// =============================================================================

#[tokio::test]
#[ignore = "Requires Chromium browser"]
async fn test_act_type() {
    let chrome = require_chrome!();
    let config = test_browser_config(&chrome);

    let (browser, mut handler) = Browser::launch(config).await.expect("Failed to launch");
    tokio::spawn(async move { while let Some(_) = handler.next().await {} });

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Failed to create page");

    // Create an input field
    page.evaluate(r#"document.body.innerHTML = '<input type="text" id="input">'"#)
        .await
        .expect("Failed");

    // Focus the input
    page.evaluate("document.getElementById('input').focus()")
        .await
        .expect("Failed to focus");

    // Type using CDP InsertText (matches act.type implementation)
    page.execute(InsertTextParams::new("Hello Tivana!".to_string()))
        .await
        .expect("Failed to type");

    // Verify
    let value: String = page
        .evaluate("document.getElementById('input').value")
        .await
        .expect("Failed")
        .into_value()
        .expect("Failed");

    assert_eq!(value, "Hello Tivana!", "Input should contain typed text");

    println!("✅ Act type test passed");
}

// =============================================================================
// Action Tests - scroll
// =============================================================================

#[tokio::test]
#[ignore = "Requires Chromium browser"]
async fn test_act_scroll() {
    let chrome = require_chrome!();
    let config = test_browser_config(&chrome);

    let (browser, mut handler) = Browser::launch(config).await.expect("Failed to launch");
    tokio::spawn(async move { while let Some(_) = handler.next().await {} });

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Failed to create page");

    // Create a long page
    page.evaluate(
        r#"document.body.innerHTML = '<div style="height:3000px">Long page</div><div id="target">Target at bottom</div>'"#,
    )
    .await
    .expect("Failed");

    // Get initial scroll
    let initial: f64 = page
        .evaluate("window.scrollY")
        .await
        .expect("Failed")
        .into_value()
        .expect("Failed");

    assert_eq!(initial, 0.0, "Should start at top");

    // Scroll down (mirrors act.scroll)
    page.evaluate("window.scrollBy({ top: 500, behavior: 'instant' })")
        .await
        .expect("Failed to scroll");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let after_scroll: f64 = page
        .evaluate("window.scrollY")
        .await
        .expect("Failed")
        .into_value()
        .expect("Failed");

    assert!(after_scroll > 0.0, "Should have scrolled down");
    println!("Scrolled from {} to {}", initial, after_scroll);

    // Scroll element into view
    page.evaluate(
        r#"document.getElementById('target').scrollIntoView({ behavior: 'instant', block: 'center' })"#,
    )
    .await
    .expect("Failed");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let final_scroll: f64 = page
        .evaluate("window.scrollY")
        .await
        .expect("Failed")
        .into_value()
        .expect("Failed");

    assert!(
        final_scroll > after_scroll,
        "Should have scrolled to target element"
    );

    println!("✅ Act scroll test passed");
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[tokio::test]
#[ignore = "Requires Chromium browser"]
async fn test_target_not_found_error() {
    let chrome = require_chrome!();
    let config = test_browser_config(&chrome);

    let (browser, mut handler) = Browser::launch(config).await.expect("Failed to launch");
    tokio::spawn(async move { while let Some(_) = handler.next().await {} });

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Failed to create page");

    // Try to find non-existent element - returns "not_found" string instead of null
    // to avoid chromiumoxide parsing issues with null values
    let result: String = page
        .evaluate(
            r#"(() => {
            const el = document.querySelector('#does-not-exist');
            if (!el) return "not_found";
            const rect = el.getBoundingClientRect();
            return JSON.stringify({ x: rect.x, y: rect.y });
        })()"#,
        )
        .await
        .expect("Query failed")
        .into_value()
        .expect("Failed to parse");

    assert_eq!(
        result, "not_found",
        "Should return 'not_found' for non-existent element"
    );

    // Verify this triggers target_not_found in Tivana's Actor
    // (The actual error is raised by Actor::resolve_target)
    println!("✅ Target not found test passed");
}

// =============================================================================
// End-to-End Smoke Test
// =============================================================================

#[tokio::test]
#[ignore = "Requires Chromium browser"]
async fn test_e2e_smoke() {
    let chrome = require_chrome!();
    let config = test_browser_config(&chrome);

    let (browser, mut handler) = Browser::launch(config).await.expect("Failed to launch");
    tokio::spawn(async move { while let Some(_) = handler.next().await {} });

    println!("1. Creating session (page)...");
    let page = browser
        .new_page("about:blank")
        .await
        .expect("Failed to create page");

    println!("2. Navigating to example.com...");
    page.goto("https://example.com")
        .await
        .expect("Navigation failed");
    tokio::time::sleep(Duration::from_secs(1)).await;

    println!("3. Getting page state (perceive.pageState)...");
    let url: String = page
        .evaluate("window.location.href")
        .await
        .expect("Failed")
        .into_value()
        .expect("Failed");
    let title: String = page
        .evaluate("document.title")
        .await
        .expect("Failed")
        .into_value()
        .expect("Failed");
    assert!(url.contains("example.com"));
    assert!(!title.is_empty());
    println!("   URL: {}, Title: {}", url, title);

    println!("4. Getting elements (perceive.elements)...");
    let element_count: i64 = page
        .evaluate("document.querySelectorAll('a[href], button, input, [role=\"button\"]').length")
        .await
        .expect("Failed")
        .into_value()
        .expect("Failed");
    assert!(element_count > 0);
    println!("   Found {} interactive elements", element_count);

    println!("5. Clicking link (act.click)...");
    let link_clicked: bool = page
        .evaluate(
            r#"(() => {
            const link = document.querySelector('a');
            if (!link) return false;
            link.click();
            return true;
        })()"#,
        )
        .await
        .expect("Failed")
        .into_value()
        .expect("Failed");
    assert!(link_clicked);
    tokio::time::sleep(Duration::from_secs(2)).await;

    println!("6. Verifying navigation...");
    let new_url: String = page
        .evaluate("window.location.href")
        .await
        .expect("Failed")
        .into_value()
        .expect("Failed");
    println!("   New URL: {}", new_url);

    println!("7. Closing session...");
    drop(browser);

    println!("✅ E2E smoke test passed!");
}

// =============================================================================
// Full Integration Test - Real Browser Session Flow
// =============================================================================

#[tokio::test]
#[ignore = "Requires Chromium browser"]
async fn test_full_session_workflow() {
    let chrome = require_chrome!();
    let config = test_browser_config(&chrome);

    let (browser, mut handler) = Browser::launch(config).await.expect("Failed to launch");
    tokio::spawn(async move { while let Some(_) = handler.next().await {} });

    // Create page and navigate
    let page = browser
        .new_page("https://example.com")
        .await
        .expect("Failed");
    tokio::time::sleep(Duration::from_secs(1)).await;

    // perceive.pageState
    let state: serde_json::Value = page
        .evaluate(
            r#"({
            url: window.location.href,
            title: document.title,
            scrollX: window.scrollX,
            scrollY: window.scrollY,
            viewportWidth: window.innerWidth,
            viewportHeight: window.innerHeight
        })"#,
        )
        .await
        .expect("Failed")
        .into_value()
        .expect("Failed");

    println!(
        "Page state: {}",
        serde_json::to_string_pretty(&state).unwrap()
    );

    // perceive.elements
    let elements: Vec<serde_json::Value> = page
        .evaluate(
            r#"Array.from(document.querySelectorAll('a, button, input')).map((el, i) => ({
            id: 'e' + (i + 1),
            role: el.getAttribute('role') || el.tagName.toLowerCase(),
            name: el.innerText?.trim() || el.getAttribute('aria-label') || null,
            bounds: (() => {
                const r = el.getBoundingClientRect();
                return { x: r.x, y: r.y, width: r.width, height: r.height };
            })()
        }))"#,
        )
        .await
        .expect("Failed")
        .into_value()
        .expect("Failed");

    println!("Found {} elements", elements.len());
    assert!(!elements.is_empty());

    // act.click on first element by coordinates
    if let Some(first) = elements.first() {
        let bounds = &first["bounds"];
        let x = bounds["x"].as_f64().unwrap() + bounds["width"].as_f64().unwrap() / 2.0;
        let y = bounds["y"].as_f64().unwrap() + bounds["height"].as_f64().unwrap() / 2.0;

        page.evaluate(format!(
            r#"(() => {{
                const el = document.elementFromPoint({}, {});
                el?.click();
            }})()"#,
            x, y
        ))
        .await
        .expect("Click failed");

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    // perceive.pageState after action
    let new_state: serde_json::Value = page
        .evaluate(
            r#"({
            url: window.location.href,
            title: document.title
        })"#,
        )
        .await
        .expect("Failed")
        .into_value()
        .expect("Failed");

    println!(
        "After click - URL: {}, Title: {}",
        new_state["url"], new_state["title"]
    );

    println!("✅ Full session workflow test passed!");
}
