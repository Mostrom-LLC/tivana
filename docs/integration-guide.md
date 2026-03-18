# Integration Guide

How to connect an AI agent to Tivana.

---

## Quick Start

### 1. Build the Runtime

```bash
cd tivana
cargo build --release
```

The binary will be at `./target/release/tivana`.

### 2. Start the Runtime

```bash
# Headed mode (browser visible)
./target/release/tivana

# Headless mode (no browser window)
./target/release/tivana --headless

# Custom port
./target/release/tivana --port 8080
```

Default port is **9876**.

### 3. Install the SDK

```bash
# Using npm
npm install tivana

# Using bun
bun add tivana
```

Or use the local SDK:

```bash
cd sdk/ts
bun install  # or npm install
```

### 4. Connect Your Agent

```typescript
import { TivanaClient } from "tivana";

const client = new TivanaClient();
await client.connect();

// Create a browser session
const sessionId = await client.createSession();

// Navigate and interact
await client.navigate("https://github.com");

const state = await client.pageState();
console.log(`Now at: ${state.url}`);

const elements = await client.elements();
console.log(`Found ${elements.length} interactive elements`);

// Click something
const signInLink = elements.find(e => e.name?.includes("Sign in"));
if (signInLink) {
  await client.click(signInLink.id);
}

// Clean up
await client.closeSession();
client.disconnect();
```

---

## Local Development Setup

### Prerequisites

- **Rust** 1.70+ (install via [rustup](https://rustup.rs/))
- **Node.js** 18+ or **Bun** 1.0+
- **Chromium-based browser** (Chrome, Edge, Brave, or system Chromium)

### Build from Source

```bash
# Clone the repo
git clone https://github.com/your-org/tivana.git
cd tivana

# Build the Rust runtime
cargo build --release

# Install SDK dependencies
cd sdk/ts
bun install  # or npm install
```

### Run Tests

```bash
# Rust tests
cargo test

# SDK smoke test (requires runtime running)
./target/release/tivana &
cd sdk/ts
bun run smoke-test.ts
```

---

## Working with Page State

```typescript
// Get current page state
const state = await client.pageState();
console.log(`URL: ${state.url}`);
console.log(`Title: ${state.title}`);
console.log(`Scroll: ${state.scrollX}, ${state.scrollY}`);
console.log(`Viewport: ${state.viewportWidth}x${state.viewportHeight}`);

// Get interactive elements
const elements = await client.elements();

// Find all buttons
const buttons = elements.filter(e => e.role === "button");

// Find element by name
const submitBtn = elements.find(
  e => e.role === "button" && e.name === "Submit"
);

// Find visible inputs
const visibleInputs = elements.filter(
  e => e.role === "textbox" && e.enabled
);
```

---

## Performing Actions

```typescript
// Click by element ID
await client.click("e42");

// Click by semantic selector
await client.click({ role: "button", label: "Sign in" });

// Click by CSS selector
await client.click("button.primary");

// Type into focused element
await client.type("hello world");

// Type into specific element
await client.type("user@example.com", "e5");

// Press keys
await client.press("Enter");
await client.press("a", ["Control"]);  // Select all

// Navigate
await client.navigate("https://example.com");

// Scroll element into view
await client.scroll("e10");

// Scroll page
await client.scroll(undefined, "down", { amount: 300 });
```

---

## Full Example: Login Flow

```typescript
import { TivanaClient } from "tivana";

async function login(username: string, password: string) {
  const client = new TivanaClient({ timeout: 60000 });
  await client.connect();
  await client.createSession({ headless: false });

  try {
    // Navigate to login page
    await client.navigate("https://app.example.com/login");

    // Wait for login form
    await client.waitFor({
      type: "Visible",
      selector: "input[name='username']"
    });

    // Get elements
    const elements = await client.elements();

    // Find and fill username
    const usernameInput = elements.find(
      e => e.role === "textbox" && e.name?.toLowerCase().includes("username")
    );
    if (usernameInput) {
      await client.type(username, usernameInput.id, { clearFirst: true });
    }

    // Find and fill password
    const passwordInput = elements.find(
      e => e.role === "textbox" && e.name?.toLowerCase().includes("password")
    );
    if (passwordInput) {
      await client.type(password, passwordInput.id, { clearFirst: true });
    }

    // Click sign in
    const signInBtn = elements.find(
      e => e.role === "button" && e.name?.toLowerCase().includes("sign in")
    );
    if (signInBtn) {
      await client.click(signInBtn.id);
    }

    // Wait for navigation
    await client.waitFor({ type: "Navigation" });

    const state = await client.pageState();
    console.log(`Logged in! Now at: ${state.url}`);

  } finally {
    await client.closeSession();
    client.disconnect();
  }
}
```

---

## Integrating with LLMs

```typescript
import { TivanaClient } from "tivana";
import Anthropic from "@anthropic-ai/sdk";

const anthropic = new Anthropic();
const client = new TivanaClient();

await client.connect();
await client.createSession({ headless: true });

const goal = "Find the pricing page and list all plan names";

// Get current state
const state = await client.pageState();
const elements = await client.elements();

// Ask LLM what to do
const response = await anthropic.messages.create({
  model: "claude-sonnet-4-20250514",
  max_tokens: 1024,
  messages: [{
    role: "user",
    content: `
Goal: ${goal}

Current URL: ${state.url}
Page Title: ${state.title}

Interactive Elements:
${elements.slice(0, 30).map(e =>
  `- ${e.id}: ${e.role} "${e.name || "(no name)}"`
).join("\n")}

What action should I take? Respond with JSON:
{ "action": "click" | "type" | "navigate" | "done", "target": "...", "value": "..." }
`
  }]
});

const action = JSON.parse(response.content[0].text);

// Execute the action
switch (action.action) {
  case "click":
    await client.click(action.target);
    break;
  case "type":
    await client.type(action.value, action.target);
    break;
  case "navigate":
    await client.navigate(action.target);
    break;
  case "done":
    console.log("Goal achieved!");
    break;
}
```

---

## CLI Options

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

---

## Troubleshooting

### Connection Failed

```
Error: WebSocket connection failed
```

**Solutions:**
1. Make sure the runtime is running: `./target/release/tivana`
2. Check the port is correct (default: 9876)
3. Check firewall settings

### Browser Launch Failed

```
Error: browser_launch_failed
```

**Solutions:**
1. Install Chrome/Chromium
2. Specify path: `--chrome-path /path/to/chrome`
3. Check browser permissions

### Element Not Found

```
Error: target_not_found
```

**Solutions:**
1. Element IDs may be stale — call `elements()` again
2. Element may be hidden or removed from DOM
3. Use CSS selector or role+label instead of ID

### Timeout

```
Error: Request timeout: act.click
```

**Solutions:**
1. Increase timeout: `new TivanaClient({ timeout: 60000 })`
2. Check if browser is responsive
3. Simplify the action (break into smaller steps)
