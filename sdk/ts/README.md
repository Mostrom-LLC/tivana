# Tivana TypeScript SDK

TypeScript/JavaScript SDK for Tivana - streaming browser perception protocol for AI agents.

## Installation

```bash
npm install tivana
# or
bun add tivana
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
const page = await client.pageState();
console.log(`URL: ${page.url}`);
console.log(`Title: ${page.title}`);

// Get elements with full visual data
const elements = await client.elements();
for (const el of elements) {
  console.log(`${el.id}: ${el.role} "${el.label}" at (${el.bounds.x}, ${el.bounds.y})`);
}

// Take actions
await client.click("e5"); // Click by element ID
await client.click({ role: "button", label: "Submit" }); // Click by selector
await client.type("hello world", "e3"); // Type into element

// Subscribe to mutations
const unsubscribe = client.onMutation((event) => {
  for (const mutation of event.mutations) {
    console.log(`Mutation: ${mutation.type}`);
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
observe((page, elements) => {
  console.log(`Now at: ${page.url}`);
  console.log(`Elements: ${elements.length}`);
});

// Take actions
await act.navigate("https://example.com");
await act.click("e3");
await act.type("hello");
await act.scroll("e10");
```

## Requirements

- **Runtime**: Tivana runtime must be running (`tivana start`)
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
- `elements()` - Get element tree with full visual data
- `onMutation(callback)` - Subscribe to DOM mutations (returns unsubscribe fn)

#### Action Methods

- `navigate(url)` - Navigate to URL
- `click(target, options?)` - Click element by ID or selector
- `type(text, target?)` - Type text (into focused element or target)
- `scroll(target, behavior?)` - Scroll element into view

### Types

#### PageState

```typescript
interface PageState {
  url: string;
  title: string;
  focusedElement: string | null;
  scrollPosition: { x: number; y: number };
  viewport: { width: number; height: number };
  timestamp: number;
}
```

#### Element

```typescript
interface Element {
  id: string;           // Stable ID (e.g., "e1", "e2")
  role: string;         // Accessibility role
  label: string;        // Accessible name
  value?: string;       // Form element value
  text?: string;        // Visible text

  focused: boolean;
  enabled: boolean;
  visible: boolean;
  interactable: boolean;

  bounds: { x, y, width, height };
  font?: { family, size, weight, color };
  background?: string;
  border?: { width, style, color, radius };

  // ... more properties
}
```

#### Mutations

```typescript
type Mutation =
  | { type: "added"; element: Element }
  | { type: "removed"; elementId: string }
  | { type: "changed"; elementId: string; changes: Record<string, unknown> }
  | { type: "focusChanged"; previousElement: string | null; currentElement: string | null }
  | { type: "navigation"; url: string };
```

### Error Handling

Errors are thrown with structured codes:

```typescript
try {
  await client.click("e999");
} catch (e) {
  // Error: [4001] Target element not found: e999
  console.error(e.message);
}
```

Error codes:
- `1xxx` - Protocol errors (invalid message, missing field)
- `2xxx` - Session errors (not found, closed)
- `3xxx` - Browser errors (launch failed, crashed)
- `4xxx` - Action errors (target not found, ambiguous)
- `5xxx` - Perception errors (failed to read state)

## Runtime

The SDK requires the Tivana runtime to be running. See the main README for runtime installation and startup.

```bash
# Start runtime
tivana start --port 9876

# With options
tivana start --headless --port 8080
```

## License

MIT
