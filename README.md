# Tivana

**Streaming browser perception protocol for AI agents.**

Tivana gives AI agents human-like awareness of web pages — not scripted automation, but continuous, semantic perception of page state so agents can explore, notice anomalies, and make judgment calls.

> **Note:** Tivana is not yet published to npm. See [Getting Started](#getting-started) for local installation instructions. npm publish coming soon.

## Why Tivana?

Existing browser automation tools are built for testing, not agency:

- **Playwright/Puppeteer** — Execute predefined scripts, blind between steps
- **Screenshots + Vision** — Heavy, lossy, point-in-time, can't reference elements
- **Raw CDP** — Too low-level, requires browser automation expertise

Humans catch bugs that tests miss because we see the whole page, notice things that "feel off," and have continuous awareness. Tivana gives agents the same capability.

## Getting Started

### Prerequisites

- **Rust** 1.75+ ([install](https://rustup.rs))
- **Bun** 1.0+ ([install](https://bun.sh)) or Node.js 18+
- **Chromium** browser (Chrome, Edge, Brave, or Arc)

### 1. Clone and Build

```bash
# Clone the repo
git clone https://github.com/Mostrom-LLC/tivana.git
cd tivana

# Build the runtime
cd runtime
cargo build --release
```

### 2. Start the Runtime

```bash
# From tivana/runtime directory
./target/release/tivana

# Options:
#   --headless    Run without browser window
#   --port 8080   Custom port (default: 9876)
```

### 3. Install the SDK

```bash
# Local installation (pre-npm publish)
cd tivana/sdk/ts
bun install

# Future: npm install tivana (coming soon)
```

### 4. Connect and Interact

```typescript
// For local development (pre-npm)
import { TivanaClient } from "./sdk/ts/src/index.js";

// Future: import { TivanaClient } from "tivana";

const client = new TivanaClient();
await client.connect();

// Create a browser session
await client.createSession();

// Navigate and perceive
await client.navigate("https://github.com");
const state = await client.pageState();
const elements = await client.elements();

console.log(`URL: ${state.url}`);
console.log(`Elements: ${elements.length}`);

// Interact
const signIn = elements.find(e => e.name?.includes("Sign in"));
if (signIn) {
  await client.click(signIn.id);
}

// Clean up
await client.closeSession();
client.disconnect();
```

### 5. Using from Another Project

Until npm publish, you can use Tivana from another project by:

```bash
# Option A: npm link
cd tivana/sdk/ts
bun link
cd your-project
bun link tivana

# Option B: File path in package.json
{
  "dependencies": {
    "tivana": "file:../tivana/sdk/ts"
  }
}

# Option C: Direct import
import { TivanaClient } from "../tivana/sdk/ts/src/index.js";
```

## What the Agent Sees

```typescript
// Page State
{
  url: "https://github.com/login",
  title: "Sign in to GitHub",
  scrollX: 0,
  scrollY: 0,
  viewportWidth: 1280,
  viewportHeight: 720,
  timestampMs: 1710412800000
}

// Elements
[
  {
    id: "e1",
    role: "textbox",
    name: "Username or email address",
    focused: false,
    enabled: true,
    bounds: { x: 200, y: 150, width: 280, height: 40 },
    styles: {
      fontFamily: "Inter, sans-serif",
      fontSize: "16px",
      color: "rgb(36, 41, 47)",
      backgroundColor: "rgb(255, 255, 255)"
    }
  },
  // ... more elements
]
```

## Full Visual Awareness

Unlike accessibility-tree-only approaches, Tivana includes computed styles:

- **Typography** — font family, size, weight, color
- **Colors** — background, foreground, border colors
- **Geometry** — bounds (position, size)
- **State** — focused, enabled, checked, expanded

This enables use cases like visual regression testing, accessibility auditing, and design system validation.

## SDK API

### Session

```typescript
await client.createSession({ headless: true });
await client.closeSession();
const sessions = await client.listSessions();
```

### Perception

```typescript
const state = await client.pageState();      // URL, title, scroll, viewport
const elements = await client.elements();    // Interactive elements
const metadata = await client.metadata();    // Meta tags, favicon, og:image
const text = await client.textContent();     // Page text content
```

### Actions

```typescript
await client.navigate("https://example.com");
await client.click("e5");                    // By element ID
await client.click({ role: "button", label: "Submit" });  // By role+label
await client.type("hello", "e3");            // Type into element
await client.press("Enter");                 // Press key
await client.scroll("e10");                  // Scroll element into view
await client.hover("e5");                    // Hover over element
await client.select("e7", "option-value");   // Select dropdown option
await client.waitFor({ type: "Navigation" }); // Wait for condition
```

## CLI Reference

```bash
tivana [OPTIONS]

Options:
  --port <PORT>        WebSocket server port (default: 9876)
  --headless           Run browser in headless mode
  --headed             Run browser in headed mode (default)
  --chrome-path <PATH> Path to Chrome/Chromium executable
  -h, --help           Print help
  -V, --version        Print version
```

## Requirements

- **Rust** 1.70+ (for building)
- **Node.js** 18+ or **Bun** 1.0+ (for SDK)
- **Chromium-based browser** (Chrome, Edge, Brave)
- macOS, Linux, or Windows

## Architecture

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Browser    │────►│   Runtime    │────►│    Agent     │
│  (Chromium)  │◄────│   (Rust)     │◄────│  (TS SDK)    │
└──────────────┘     └──────────────┘     └──────────────┘
     CDP              WebSocket             Your Code
```

- **Runtime**: Rust + CDP (chromiumoxide, tokio)
- **SDK**: TypeScript (WebSocket client)
- **Protocol**: JSON over WebSocket

## Running Tests

```bash
# Rust unit tests
cargo test

# Browser integration tests (requires Chromium)
# Set CHROME_PATH or have Chromium in PATH
cargo test --test browser_test -- --ignored --nocapture

# Realistic browser integration tests (uses https://the-internet.herokuapp.com)
cargo test --test realistic_browser_test -- --ignored --nocapture --test-threads=1

# SDK smoke test (requires runtime)
./target/release/tivana &
cd sdk/ts
bun run smoke-test.ts
```

### Test Coverage

#### Basic Browser Tests (`browser_test.rs`)
- Browser launch and navigation
- Element perception
- CDP connection

#### Realistic Browser Tests (`realistic_browser_test.rs`)
Tests against [the-internet.herokuapp.com](https://the-internet.herokuapp.com):

| Test | URL | Coverage |
|------|-----|----------|
| `test_login_form_submission` | `/login` | Form fill, submit, success validation |
| `test_login_form_validation_failure` | `/login` | Invalid credentials, error messages |
| `test_dynamic_loading_*` | `/dynamic_loading/*` | Async content, wait-for patterns |
| `test_javascript_alert_*` | `/javascript_alerts` | Alert, confirm, prompt dialogs |
| `test_iframe_interaction` | `/iframe` | Cross-frame content manipulation |
| `test_nested_frames` | `/nested_frames` | Multi-level frame traversal |
| `test_shadow_dom_traversal` | `/shadowdom` | Shadow DOM piercing |
| `test_dropdown_selection` | `/dropdown` | Select element interaction |
| `test_checkboxes` | `/checkboxes` | Checkbox state toggling |
| `test_hover_and_hidden_content` | `/hovers` | Hover-triggered content |
| `test_drag_and_drop` | `/drag_and_drop` | HTML5 drag events |
| `test_file_upload` | `/upload` | File input UI verification |
| `test_key_presses` | `/key_presses` | Keyboard event handling |
| `test_infinite_scroll` | `/infinite_scroll` | Scroll-triggered content loading |

## Documentation

See the [docs](./docs) folder:

- [Protocol Specification](./docs/protocol-specification.md) — Message formats
- [Element Model](./docs/element-model.md) — Element structure
- [Action Primitives](./docs/action-primitives.md) — Available actions
- [Integration Guide](./docs/integration-guide.md) — How to connect an agent
- [Architecture](./docs/architecture.md) — Runtime design

## License

MIT
