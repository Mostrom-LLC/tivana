# Tivana

**Streaming browser perception protocol for AI agents.**

Tivana gives AI agents human-like awareness of web pages — not scripted automation, but continuous, semantic perception of page state so agents can explore, notice anomalies, and make judgment calls.

## Why Tivana?

Existing browser automation tools are built for testing, not agency:

- **Playwright/Puppeteer** — Execute predefined scripts, blind between steps
- **Screenshots + Vision** — Heavy, lossy, point-in-time, can't reference elements
- **Raw CDP** — Too low-level, requires browser automation expertise

Humans catch bugs that tests miss because we see the whole page, notice things that "feel off," and have continuous awareness. Tivana gives agents the same capability.

## Quick Start

### 1. Build the Runtime

```bash
# Clone the repo
git clone https://github.com/Mostrom-LLC/tivana.git
cd tivana/runtime

# Build with Rust (requires Rust 1.75+)
cargo build --release
```

### 2. Start the Runtime

```bash
# Headed mode (see the browser)
./target/release/tivana start

# Headless mode
./target/release/tivana start --headless

# Custom port
./target/release/tivana start --port 8080
```

### 3. Install the SDK

```bash
# Using npm
npm install tivana

# Using bun
bun add tivana

# Or use local SDK
cd sdk/ts && bun install
```

### 4. Connect and Interact

```typescript
import { TivanaClient } from "tivana";

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
tivana start [OPTIONS]

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

# SDK smoke test (requires runtime)
./target/release/tivana start &
cd sdk/ts
bun run smoke-test.ts
```

## Documentation

See the [docs](./docs) folder:

- [Protocol Specification](./docs/protocol-specification.md) — Message formats
- [Element Model](./docs/element-model.md) — Element structure
- [Action Primitives](./docs/action-primitives.md) — Available actions
- [Integration Guide](./docs/integration-guide.md) — How to connect an agent
- [Architecture](./docs/architecture.md) — Runtime design

## License

MIT
