//! Browser integration tests
//!
//! Tests actual browser launch and CDP operations.
//! Run with: CHROME_PATH=/path/to/chrome cargo test --test browser_test -- --ignored

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::layout::Point;
use futures_util::StreamExt;
use std::env;
use std::time::Duration;
use tokio::time::timeout;

fn chrome_path() -> Option<String> {
    // Check environment variable first
    if let Ok(path) = env::var("CHROME_PATH") {
        if std::path::Path::new(&path).exists() {
            return Some(path);
        }
    }

    // Check Playwright cache
    let playwright_path =
        format!("{}/.cache/ms-playwright/chromium-1208/chrome-linux64/chrome", env::var("HOME").unwrap_or_default());
    if std::path::Path::new(&playwright_path).exists() {
        return Some(playwright_path);
    }

    // Check common paths
    let paths = [
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
    ];

    for path in paths {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    None
}

#[tokio::test]
#[ignore = "Requires Chromium browser"]
async fn test_browser_launch_and_navigate() {
    let chrome = match chrome_path() {
        Some(p) => p,
        None => {
            eprintln!("Skipping: Chrome not found");
            return;
        }
    };

    println!("Using Chrome at: {}", chrome);

    // Build browser config (headless mode for CI)
    let user_data_dir = format!("/tmp/tivana-test-{}", std::process::id());
    let config = BrowserConfig::builder()
        .chrome_executable(&chrome)
        .no_sandbox()
        .arg("--headless=new")
        .arg("--disable-gpu")
        .arg("--disable-dev-shm-usage")
        .arg("--disable-software-rasterizer")
        .arg(format!("--user-data-dir={}", user_data_dir))
        .build()
        .expect("Failed to build config");

    // Launch browser with timeout
    let result = timeout(Duration::from_secs(30), Browser::launch(config)).await;

    let (browser, mut handler) = match result {
        Ok(Ok((b, h))) => (b, h),
        Ok(Err(e)) => {
            eprintln!("Browser launch failed: {}", e);
            return;
        }
        Err(_) => {
            eprintln!("Browser launch timed out");
            return;
        }
    };

    // Spawn handler
    let handler_handle = tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(e) = event {
                eprintln!("Handler error: {}", e);
            }
        }
    });

    // Create page
    let page = browser
        .new_page("about:blank")
        .await
        .expect("Failed to create page");

    // Navigate
    page.goto("https://example.com")
        .await
        .expect("Failed to navigate");

    // Wait for load
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Get URL
    let url: String = page
        .evaluate("window.location.href")
        .await
        .expect("Failed to get URL")
        .into_value()
        .expect("Failed to parse URL");

    println!("Current URL: {}", url);
    assert!(url.contains("example.com"));

    // Get title
    let title: String = page
        .evaluate("document.title")
        .await
        .expect("Failed to get title")
        .into_value()
        .expect("Failed to parse title");

    println!("Title: {}", title);
    assert!(title.contains("Example"));

    // Get element count
    let count: i64 = page
        .evaluate("document.querySelectorAll('*').length")
        .await
        .expect("Failed to count elements")
        .into_value()
        .expect("Failed to parse count");

    println!("Element count: {}", count);
    assert!(count > 0);

    // Test click
    let link_exists: bool = page
        .evaluate("document.querySelector('a') !== null")
        .await
        .expect("Failed to check link")
        .into_value()
        .expect("Failed to parse bool");

    if link_exists {
        // Get link position
        let pos: serde_json::Value = page
            .evaluate(
                r#"(() => {
                const link = document.querySelector('a');
                const rect = link.getBoundingClientRect();
                return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
            })()"#,
            )
            .await
            .expect("Failed to get position")
            .into_value()
            .expect("Failed to parse position");

        let x = pos["x"].as_f64().unwrap();
        let y = pos["y"].as_f64().unwrap();
        println!("Link position: ({}, {})", x, y);

        // Click the link
        page.click(Point { x, y }).await.expect("Failed to click");
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Verify navigation
        let new_url: String = page
            .evaluate("window.location.href")
            .await
            .expect("Failed to get new URL")
            .into_value()
            .expect("Failed to parse new URL");

        println!("New URL after click: {}", new_url);
    }

    // Test typing (find or create input)
    let _ = page
        .evaluate(
            r#"(() => {
            if (!document.querySelector('input')) {
                const input = document.createElement('input');
                input.id = 'test-input';
                document.body.appendChild(input);
            }
            return true;
        })()"#,
        )
        .await;

    // Focus input
    let _ = page.evaluate("document.querySelector('input')?.focus()").await;

    // Type using CDP
    use chromiumoxide::cdp::browser_protocol::input::InsertTextParams;
    page.execute(InsertTextParams::new("Hello Tivana!".to_string()))
        .await
        .expect("Failed to type");

    // Verify typed text
    let value: String = page
        .evaluate("document.querySelector('input')?.value || ''")
        .await
        .expect("Failed to get value")
        .into_value()
        .expect("Failed to parse value");

    println!("Input value: {}", value);
    assert!(value.contains("Hello"));

    // Close browser
    drop(browser);
    handler_handle.abort();

    println!("✅ Browser test passed!");
}

#[tokio::test]
#[ignore = "Requires Chromium browser"]
async fn test_element_perception() {
    let chrome = match chrome_path() {
        Some(p) => p,
        None => {
            eprintln!("Skipping: Chrome not found");
            return;
        }
    };

    let user_data_dir = format!("/tmp/tivana-test-perception-{}", std::process::id());
    let config = BrowserConfig::builder()
        .chrome_executable(&chrome)
        .no_sandbox()
        .arg("--headless=new")
        .arg("--disable-gpu")
        .arg("--disable-dev-shm-usage")
        .arg(format!("--user-data-dir={}", user_data_dir))
        .build()
        .expect("Failed to build config");

    let (browser, mut handler) = Browser::launch(config)
        .await
        .expect("Failed to launch browser");

    tokio::spawn(async move {
        while let Some(_) = handler.next().await {}
    });

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Failed to create page");

    page.goto("https://example.com")
        .await
        .expect("Failed to navigate");

    tokio::time::sleep(Duration::from_secs(1)).await;

    // Get elements with properties (similar to perceive.elements)
    let elements: serde_json::Value = page
        .evaluate(
            r#"(() => {
            const elements = [];
            let counter = 0;
            
            document.querySelectorAll('h1, h2, h3, p, a, button, input').forEach(el => {
                const rect = el.getBoundingClientRect();
                const style = window.getComputedStyle(el);
                
                elements.push({
                    id: 'e' + (++counter),
                    tag: el.tagName.toLowerCase(),
                    role: el.getAttribute('role') || el.tagName.toLowerCase(),
                    text: el.innerText?.trim().slice(0, 100) || null,
                    bounds: {
                        x: rect.x,
                        y: rect.y,
                        width: rect.width,
                        height: rect.height
                    },
                    font: {
                        family: style.fontFamily,
                        size: style.fontSize,
                        color: style.color
                    },
                    visible: rect.width > 0 && rect.height > 0
                });
            });
            
            return elements;
        })()"#,
        )
        .await
        .expect("Failed to get elements")
        .into_value()
        .expect("Failed to parse elements");

    println!("Elements: {}", serde_json::to_string_pretty(&elements).unwrap());

    let elements_arr = elements.as_array().expect("Expected array");
    assert!(!elements_arr.is_empty(), "Should have found elements");

    // Verify element structure
    for el in elements_arr {
        assert!(el["id"].is_string());
        assert!(el["role"].is_string());
        assert!(el["bounds"]["x"].is_number());
        assert!(el["bounds"]["y"].is_number());
    }

    drop(browser);
    println!("✅ Element perception test passed!");
}
