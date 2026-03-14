# Tivana TypeScript SDK

TypeScript/JavaScript SDK for Tivana - streaming browser perception protocol for AI agents.

## Installation

```bash
npm install tivana
# or
bun add tivana
```

For local development:
```bash
cd sdk/ts
bun install  # or npm install
```

## Quick Start

```typescript
import { TivanaClient } from "tivana";

// Connect to runtime
const client = new TivanaClient();
await client.connect("ws://localhost:9876");

// Create browser session
const sessionId = await client.createSession();
console.log(`Session: ${sessionId}`);

// Navigate
await client.navigate("https://example.com");

// Get page state
const state = await client.pageState();
console.log(`URL: ${state.url}`);
console.log(`Title: ${state.title}`);

// Get interactive elements
const elements = await client.elements();
for (const el of elements) {
  console.log(`${el.id}: ${el.role} "${el.name}" at (${el.bounds?.x}, ${el.bounds?.y})`);
}

// Take actions
await client.click("e5"); // Click by element ID
await client.click({ role: "button", label: "Submit" }); // Click by selector
await client.type("hello world", "e3"); // Type into element

// Subscribe to mutations
const unsubscribe = client.onMutation((events) => {
  for (const event of events) {
    console.log(`Mutation: ${event.type}`);
  }
});

// Cleanup
await client.closeSession();
client.disconnect();
```

## Convenience API

For simpler use cases:

```typescript
import { connect, observe, act } from "tivana";

// Connect and create session in one step
await connect("ws://localhost:9876");

// Observe page state (called on load and mutations)
observe((state, elements) => {
  console.log(`Now at: ${state.url}`);
  console.log(`Elements: ${elements.length}`);
});

// Take actions
await act.navigate("https://example.com");
await act.click("e3");
await act.type("hello");
await act.scroll("e10");
```

## Running the Smoke Test

```bash
# Start the runtime first
./target/release/tivana start &

# Run the smoke test
cd sdk/ts
bun run smoke-test.ts
# or
npx tsx smoke-test.ts
```

## Requirements

- **Runtime**: Tivana runtime must be running (`./target/release/tivana start`)
- **Node.js**: 18+ (uses native WebSocket or `ws` package)
- **Bun**: 1.0+ (uses native WebSocket)

## API Reference

### TivanaClient

The main client class for connecting to Tivana.

#### Constructor

```typescript
const client = new TivanaClient({
  url: "ws://localhost:9876", // Default URL
  timeout: 30000,             // Request timeout (ms)
  autoReconnect: false,       // Auto-reconnect on disconnect
  reconnectDelay: 1000,       // Delay between reconnect attempts
});
```

#### Connection Methods

- `connect(url?)` - Connect to runtime
- `disconnect()` - Disconnect from runtime
- `isConnected()` - Check connection status

#### Session Methods

- `createSession(params?)` - Create browser session (launches Chromium)
- `closeSession()` - Close current session
- `listSessions()` - List all sessions
- `getSessionId()` - Get current session ID

#### Perception Methods

- `pageState()` - Get current page state (URL, title, scroll, viewport)
- `elements()` - Get interactive elements with visual data
- `accessibilitySnapshot()` - Get full accessibility tree
- `textContent()` - Get page text content
- `metadata()` - Get page metadata (title, description, og:image, etc.)
- `findElements(selector)` - Find elements by CSS selector
- `onMutation(callback)` - Subscribe to DOM mutations (returns unsubscribe fn)

#### Action Methods

- `navigate(url)` - Navigate to URL
- `click(target, options?)` - Click element by ID, selector, or role+label
- `type(text, target?, options?)` - Type text into element
- `press(key, modifiers?)` - Press a key or key combination
- `scroll(target?, direction?, options?)` - Scroll page or element
- `hover(target)` - Hover over element
- `focus(target)` - Focus element
- `select(target, value)` - Select dropdown option
- `waitFor(condition, timeoutMs?)` - Wait for condition

### Types

#### PageState

```typescript
interface PageState {
  url: string;
  title: string | null;
  focusedElementId: string | null;
  scrollX: number;
  scrollY: number;
  viewportWidth: number;
  viewportHeight: number;
  documentWidth: number;
  documentHeight: number;
  timestampMs: number;
}
```

#### Element

```typescript
interface Element {
  id: string;              // Stable ID (e.g., "e1", "e2")
  role: string;            // Accessibility role
  name?: string;           // Accessible name/label
  value?: string;          // Form element value
  description?: string;    // Accessible description

  bounds?: BoundingBox;    // Position and size
  styles?: ElementStyles;  // Computed styles

  focused: boolean;
  enabled: boolean;
  checked?: boolean;
  selected?: boolean;
  expanded?: boolean;
  required?: boolean;

  children?: Element[];
}

interface BoundingBox {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface ElementStyles {
  fontFamily?: string;
  fontSize?: string;
  fontWeight?: string;
  color?: string;
  backgroundColor?: string;
  border?: string;
  display?: string;
  visibility?: string;
}
```

#### ActionResult

```typescript
interface ActionResult {
  success: boolean;
  pageState?: PageState;
  data?: unknown;
  durationMs: number;
}
```

#### Mutations

```typescript
type MutationEvent =
  | { type: "Added"; elementId: string; parentId?: string }
  | { type: "Removed"; elementId: string }
  | { type: "Changed"; elementId: string; attribute: string; oldValue?: string; newValue?: string }
  | { type: "TextChanged"; elementId: string; text: string };
```

### Error Handling

Errors are thrown with structured codes:

```typescript
try {
  await client.click("e999");
} catch (e) {
  // Error: [target_not_found] Element not found: e999
  console.error(e.message);
}
```

Error codes:
- Protocol errors: `invalid_message`, `missing_field`, `unknown_method`
- Session errors: `session_not_found`, `session_closed`
- Browser errors: `browser_launch_failed`, `browser_crashed`
- Action errors: `target_not_found`, `target_ambiguous`, `action_failed`
- Internal errors: `internal_error`

## Runtime

The SDK requires the Tivana runtime to be running. See the main README for runtime installation and startup.

```bash
# Build the runtime
cd tivana
cargo build --release

# Start runtime
./target/release/tivana start --port 9876

# With options
./target/release/tivana start --headless --port 8080
```

## License

MIT
