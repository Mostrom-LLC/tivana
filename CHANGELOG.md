# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-24

### Added

#### Runtime
- WebSocket server with JSON protocol routing
- CDP-based browser management (Chromium)
- Session lifecycle: create, close, persist, restore
- `perceive.pageState` — URL, title, viewport, scroll, document dimensions
- `perceive.elements` — interactive element discovery with semantic IDs, roles, labels, bounds, `visible`, `interactable` fields
- `perceive.accessibilitySnapshot` — full accessibility tree
- `perceive.textContent` — page text extraction
- `perceive.evaluate` — arbitrary JS execution with 30s timeout
- `perceive.metadata` — page metadata (title, description, og tags)
- `perceive.formFields` — form field enumeration with computed labels
- `perceive.observe` / `perceive.unobserve` — observation lifecycle
- `act.click`, `act.type`, `act.press`, `act.scroll`, `act.hover`, `act.focus`, `act.select`, `act.navigate`
- `act.batch` — execute multiple actions in a single roundtrip
- `act.smartFill` — profile-based form filling
- Page event streaming: `page.loaded`, `page.navigated`, `page.focus`, `page.scroll`, `page.resize`
- DOM mutation event streaming with enriched data (role, name on added elements)
- Element ID stability via `data-tivana-id` attributes
- Auto-scroll element into view before click
- Multi-tab support: `session.tabs`, `session.switchTab`, `session.newTab`, `session.closeTab`
- Session persistence to `~/.tivana/sessions.json`
- WebSocket heartbeat (60s ping, 300s stale detection)
- Browser fingerprint hardening (navigator.webdriver, chrome.runtime, plugins, languages)
- `--connect` mode to attach to existing Chrome via remote debugging port
- `--headless` / `--port` CLI options
- Screenshot capture (`perceive.screenshot`)
- Network interception (`network.capture`, `network.requests`)
- Cookie and storage management
- CAPTCHA detection and solving (stealth-first approach)
- Proxy support with authenticated proxy handling
- MIT License

#### SDK (TypeScript)
- `TivanaClient` with auto-reconnect and exponential backoff
- CJS + ESM dual build with type declarations
- Full API coverage: `pageState()`, `elements()`, `click()`, `type()`, `press()`, `scroll()`, `navigate()`, `evaluate()`
- Observation API: `startObservation()`, `stopObservation()`, `onEvent()`, `onPageEvent()`
- Command queuing during reconnect
- 30s default request timeout

#### Chrome Extension
- Manifest V3 extension using `chrome.debugger` API
- WebSocket bridge to Tivana runtime
- Badge states (ON/…/!/off) for connection status
- Auto-reconnect with exponential backoff
- Stale debugger detection and auto-reattach
- Tab navigation handling with debugger re-enable

#### Documentation
- Protocol specification
- Architecture guide with transport diagrams
- Observation guide (snapshot vs event contracts)
- Protocol reference (all methods and types)
- Integration guide
- 7 working examples with README

### Notes

- Chromium-only in v1 (Firefox/WebKit planned for future)
- Extension-backed sessions are single-tab
- `NetworkIdle` wait condition is stubbed
- Shadow DOM traversal not yet supported
