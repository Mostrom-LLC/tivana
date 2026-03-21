# Tivana

**Zero-config browser automation that just works.** Stealth-first, fast, and built for developers who don't want to fight with CAPTCHAs, fingerprinting, or brittle selectors.

```bash
npm install tivana  # coming soon
```

```typescript
import { TivanaClient } from "tivana";

const client = new TivanaClient();
await client.connect();
await client.createSession();

await client.navigate("https://example.com");
const elements = await client.elements();
await client.click(elements[0].id);

// Extract design tokens in 1ms
const tokens = await client.evaluate(`({
  bg: getComputedStyle(document.body).backgroundColor,
  font: getComputedStyle(document.body).fontFamily,
  props: Object.keys(getComputedStyle(document.body)).length
})`);
```

> **Status:** Pre-release. All features built and tested. npm publish coming soon.

---

## Features

### 🛡️ Stealth & Anti-Detection
Browser fingerprint hardening out of the box. WebGL, Canvas, AudioContext, plugins, and language spoofing. Human-like mouse movement via Bézier curves. Realistic typing cadence with variable per-character delays. Passes **55/56 bot detection tests** with zero configuration.

### 🔓 Zero-Config CAPTCHA Solver
Auto-detects reCAPTCHA v2/v3, hCaptcha, and Cloudflare Turnstile. Stealth-first approach means CAPTCHAs rarely trigger. When they do, audio challenge + local Whisper transcription handles it. **No API keys. No paid services. No config.**

### ⚡ Batch Actions & Speed
Execute multiple actions in a single WebSocket roundtrip. `fillForm()` fills an entire form from a field map in one call. Sub-second form fills, sub-2s batch operations.

```typescript
await client.batch([
  { type: "click", target: "e5" },
  { type: "type", target: "e5", text: "hello@example.com" },
  { type: "click", target: "e12" }
]);

await client.fillForm({
  "e5": "John Doe",
  "e8": "john@example.com",
  "e11": true  // checkbox
}, "e15"); // submit button
```

### 👁️ Page Perception
Discover every interactive element with semantic roles, labels, checked state, and accessibility info. Full text content extraction. Page state including URL, title, viewport, scroll position, and focused element.

```typescript
const elements = await client.elements();
// → [{ id: "e1", role: "textbox", name: "Email", enabled: true, focused: false }, ...]

const state = await client.pageState();
// → { url: "...", title: "...", viewport: { width: 1440, height: 900 } }
```

### 🧠 Arbitrary JavaScript Execution
Run any JS on the page and get structured results back. Extract computed styles, CSS custom properties, DOM data. Run accessibility audits. Access any browser API.

```typescript
const tokens = await client.evaluate(`({
  colors: {
    bg: getComputedStyle(document.body).backgroundColor,
    text: getComputedStyle(document.body).color
  },
  fonts: getComputedStyle(document.body).fontFamily,
  customProps: Array.from(document.styleSheets)
    .flatMap(s => { try { return [...s.cssRules] } catch { return [] } })
    .flatMap(r => (r.cssText.match(/--[\\w-]+/g) || []))
    .length
})`);
```

### 📸 Screenshots
PNG and JPEG capture. Full-page or viewport-only. Clip to specific regions. Quality control for JPEG compression.

```typescript
const shot = await client.screenshot({ format: "png", fullPage: true });
// → { data: "<base64>", format: "png", width: 1440, height: 3200 }
```

### 🌐 Network Monitoring
Capture all fetch and XHR requests automatically. Filter by URL pattern. Inspect method, URL, headers, and timing.

```typescript
await client.enableNetworkCapture();
await client.navigate("https://api.example.com");
const requests = await client.getNetworkRequests("api.example.com");
// → [{ method: "GET", url: "...", status: 200, timing: 142 }]
```

### 🍪 Cookie & Storage Management
Read, set, and clear cookies. Full localStorage and sessionStorage access.

```typescript
const cookies = await client.getCookies();
await client.setCookie("session", "abc123");
await client.setLocalStorage("theme", "dark");
const theme = await client.evaluate("localStorage.getItem('theme')");
```

### 📁 File Upload
Upload files to file input elements via CDP. No physical keyboard or mouse — pure protocol-level.

```typescript
await client.uploadFile("e7", ["/path/to/resume.pdf"]);
```

### 🗂️ Multi-Tab Management
List, open, switch, and close tabs. Tab-aware actions across any open tab.

```typescript
const tabs = await client.tabs();
await client.newTab("https://example.com");
await client.switchTab(tabs[0].targetId);
await client.closeTab(tabs[1].targetId);
```

### ⏳ Smart Wait Conditions
Wait for elements, navigation, or custom JS conditions. No more `sleep()`.

```typescript
await client.waitForSelector("button.submit", 10000);
await client.waitForNavigation(5000);
await client.waitForFunction("document.readyState === 'complete'");
```

### 🔄 Stale Element Recovery
DOM mutations invalidate element references in SPAs. Tivana auto-retries failed actions by re-enumerating elements — transparent to the developer.

### 💾 Session Persistence
Sessions survive runtime restarts. Reattach to existing Chrome tabs on reconnect. Pick up where you left off.

### 🔌 Flexible Connection
Launch a new Chrome instance or connect to an existing one via `--connect`. WebSocket heartbeat with auto-reconnect and exponential backoff.

```bash
# Launch fresh Chrome
tivana

# Connect to existing Chrome
tivana --connect 9222
```

### 🌍 Proxy & IP Rotation
HTTP, HTTPS, and SOCKS5 proxy support. Proxy pool with round-robin rotation. Authenticated proxy support. Session-level configuration.

```typescript
await client.setProxy({ server: "proxy.example.com:8080", protocol: "http" });
await client.setProxyPool([
  { server: "us.proxy.com:8080", protocol: "socks5" },
  { server: "eu.proxy.com:8080", protocol: "socks5" }
]);
await client.rotateProxy();
```

### 🛡️ Error Recovery
Auto-handle JavaScript dialogs (alerts, confirms, prompts). Navigation resilience with DOMContentLoaded detection. Graceful degradation on page destruction.

### 🏗️ Developer Experience
- **Zero config** — install and go, no setup wizard
- **TypeScript SDK** with full type safety
- **One WebSocket** connection, simple JSON protocol
- **No paid services**, no API keys, no BYOB
- **MIT licensed**

---

## Getting Started

### Prerequisites

- **Rust** 1.75+ ([install](https://rustup.rs))
- **Bun** 1.0+ ([install](https://bun.sh)) or Node.js 18+
- **Chromium** browser (Chrome, Edge, Brave, or Arc)

### 1. Clone and Build

```bash
git clone https://github.com/Mostrom-LLC/tivana.git
cd tivana

cd runtime
cargo build --release
```

### 2. Start the Runtime

```bash
./target/release/tivana

# Options:
#   --headless         Run without browser window
#   --port 8080        Custom port (default: 9876)
#   --connect 9222     Connect to existing Chrome
```

### 3. Install the SDK

```bash
cd tivana/sdk/ts
bun install

# Future: npm install tivana
```

### 4. Connect and Interact

```typescript
import { TivanaClient } from "./sdk/ts/src/index.js";

const client = new TivanaClient();
await client.connect();
await client.createSession();

await client.navigate("https://github.com");
const state = await client.pageState();
const elements = await client.elements();

console.log(`URL: ${state.url}`);
console.log(`Elements: ${elements.length}`);

const signIn = elements.find(e => e.name?.includes("Sign in"));
if (signIn) await client.click(signIn.id);

await client.closeSession();
client.disconnect();
```

---

## SDK API Reference

### Session Management
```typescript
await client.createSession({ headless: true });
await client.closeSession();
const sessions = await client.listSessions();
```

### Perception
```typescript
const state = await client.pageState();
const elements = await client.elements();
const text = await client.textContent();
const result = await client.evaluate("document.title");
const shot = await client.screenshot({ format: "png" });
```

### Actions
```typescript
await client.navigate("https://example.com");
await client.click("e5");
await client.type("hello", "e3");
await client.press("Enter");
await client.scroll("down", 300);
await client.uploadFile("e7", ["/path/to/file.pdf"]);
```

### Batch & Form Fill
```typescript
await client.batch([
  { type: "click", target: "e5" },
  { type: "type", target: "e5", text: "hello" }
]);
await client.fillForm({ "e5": "value", "e8": true }, "e12");
```

### Wait Conditions
```typescript
await client.waitForSelector("button.submit");
await client.waitForNavigation();
await client.waitForFunction("window.loaded === true");
```

### Network & Storage
```typescript
await client.enableNetworkCapture();
const requests = await client.getNetworkRequests();
const cookies = await client.getCookies();
await client.setCookie("key", "value");
await client.setLocalStorage("key", "value");
```

### Tabs
```typescript
const tabs = await client.tabs();
await client.newTab("https://example.com");
await client.switchTab(targetId);
await client.closeTab(targetId);
```

### Proxy
```typescript
await client.setProxy({ server: "host:port", protocol: "socks5" });
await client.setProxyPool(proxies);
await client.rotateProxy();
```

---

## Architecture

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Browser    │────►│   Runtime    │────►│    Agent     │
│  (Chromium)  │◄────│   (Rust)     │◄────│  (TS SDK)    │
└──────────────┘     └──────────────┘     └──────────────┘
     CDP              WebSocket             Your Code
```

- **Runtime**: Rust + CDP (chromiumoxide, tokio) — fast, safe, concurrent
- **SDK**: TypeScript — type-safe, auto-reconnect, event-driven
- **Protocol**: JSON over WebSocket on port 9876

---

## Running Tests

```bash
# Rust unit tests
cd runtime && cargo test

# Integration tests (requires Chromium)
cargo test --test browser_test -- --ignored --nocapture

# Realistic browser tests (uses the-internet.herokuapp.com)
cargo test --test realistic_browser_test -- --ignored --nocapture --test-threads=1
```

---

## Benchmark

```
8/8 capability tests passed in 15.3 seconds:

✅ Bot Detection          3.2s   55/56 tests passed
✅ Design Token Extract   1.9s   113 CSS custom properties extracted
✅ Screenshot             0.16s  160KB PNG captured
✅ Network Capture        2.1s   Request interception working
✅ Cookie & Storage       0.004s Round-trip in 4ms
✅ Wait Conditions        0.42s  Element found via polling
✅ Batch Speed            2.3s   3 actions in single roundtrip
✅ JS Evaluation          0.001s Full page audit in 1ms
```

---

## License

MIT © [Mostrom LLC](https://github.com/Mostrom-LLC) 2025
