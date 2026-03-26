# Tivana TypeScript SDK

TypeScript client for Tivana's browser perception protocol.

Tivana gives agents semantic awareness of web pages: perceive what's on screen, reason about it, act on it. The SDK connects to the Tivana runtime over WebSocket.

## Installation

```bash
npm install @mostrom/tivana
```

## Quick Start

```typescript
import { TivanaClient } from "@mostrom/tivana";

const client = new TivanaClient({ url: "ws://localhost:9876" });
await client.connect();
await client.createSession();

// Perceive
await client.navigate("https://news.ycombinator.com");
const page = await client.pageState();
const elements = await client.elements();

// Reason
const link = elements.find(e => e.role === "a" && e.name?.includes("new"));

// Act
if (link) await client.click(link.id);

await client.closeSession();
client.disconnect();
```

## Agent Loop Pattern

The canonical Tivana integration: perceive → reason → act → repeat.

```typescript
import { TivanaClient } from "@mostrom/tivana";

const client = new TivanaClient({ url: "ws://localhost:9876" });
await client.connect();
await client.createSession();
await client.navigate("https://news.ycombinator.com");

for (let step = 0; step < 20; step++) {
  const [page, elements] = await Promise.all([
    client.pageState(),
    client.elements(),
  ]);

  // Send perception to your model
  const decision = await yourModel.decide({ goal, page, elements });

  if (decision.type === "done") break;
  if (decision.type === "click") await client.click(decision.target);
  if (decision.type === "type") await client.type(decision.text, decision.target);
  if (decision.type === "navigate") await client.navigate(decision.url);
}
```

## API Reference

### Constructor

```typescript
new TivanaClient(options?: { url?: string; timeout?: number })
```

- `url` — WebSocket URL (default: `ws://localhost:9876`)
- `timeout` — Default request timeout in ms (default: `30000`)

### Connection

| Method | Returns | Description |
|--------|---------|-------------|
| `connect(url?)` | `Promise<void>` | Connect to Tivana runtime |
| `disconnect()` | `void` | Close WebSocket connection |
| `createSession(opts?)` | `Promise<{ sessionId: string }>` | Create browser session. Options: `{ headless?: boolean }` |
| `closeSession()` | `Promise<void>` | Close current session and browser |
| `request(method, params)` | `Promise<T>` | Send raw protocol request |

### Perception

| Method | Returns | Description |
|--------|---------|-------------|
| `pageState()` | `Promise<PageState>` | Page URL, title, viewport, scroll position, document size |
| `elements()` | `Promise<Element[]>` | All interactive elements with semantic IDs, roles, labels, bounds, visibility |
| `accessibilitySnapshot()` | `Promise<AccessibilitySnapshot>` | Accessibility tree snapshot |
| `textContent()` | `Promise<string>` | Full visible text content |
| `metadata()` | `Promise<PageMetadata>` | Page metadata (title, description, og tags) |
| `formFields()` | `Promise<FormField[]>` | Form field enumeration with computed labels, options, validation |
| `evaluate(expression)` | `Promise<any>` | Execute JavaScript in page context |

### Actions

| Method | Returns | Description |
|--------|---------|-------------|
| `navigate(url)` | `Promise<ActionResult>` | Navigate to URL |
| `click(target)` | `Promise<ActionResult>` | Click element by ID (e.g. `"e5"`) |
| `type(text, target?)` | `Promise<ActionResult>` | Type text into element or focused element |
| `press(key, modifiers?)` | `Promise<ActionResult>` | Press key. Keys: `Enter`, `Tab`, `Escape`, `Backspace`, `ArrowDown`, etc. Modifiers: `["Shift"]`, `["Control"]`, `["Meta"]` |
| `scroll(target?, direction?)` | `Promise<ActionResult>` | Scroll element or page. Direction: `"up"`, `"down"`, `"left"`, `"right"` |
| `hover(target)` | `Promise<ActionResult>` | Hover over element |
| `focus(target)` | `Promise<ActionResult>` | Focus element |
| `select(target, values)` | `Promise<ActionResult>` | Select dropdown option(s) |

### Observation

| Method | Returns | Description |
|--------|---------|-------------|
| `startObservation()` | `Promise<void>` | Begin streaming page events and DOM mutations |
| `stopObservation()` | `Promise<void>` | Stop streaming |
| `onEvent(callback)` | `void` | Subscribe to all events |
| `onPageEvent(type, callback)` | `void` | Subscribe to specific event type |

**Event types:**

| Type | Data | Description |
|------|------|-------------|
| `page.mutation` | `MutationEvent[]` | DOM changes (Added, Removed, Changed, TextChanged) |
| `page.loaded` | `{ url, title, timestampMs }` | Page finished loading |
| `page.navigated` | `{ url, previousUrl, timestampMs }` | Navigation occurred |
| `page.focus` | `{ elementId, role, name, timestampMs }` | Focus changed |
| `page.scroll` | `{ scrollX, scrollY, timestampMs }` | Page scrolled (200ms throttle) |
| `page.resize` | `{ viewportWidth, viewportHeight, timestampMs }` | Viewport resized |

### Secondary API

| Method | Description |
|--------|-------------|
| `screenshot(options?)` | Capture page screenshot |
| `enableNetworkCapture()` | Start capturing network requests |
| `getNetworkRequests(pattern?)` | Get captured requests |
| `listTabs()` | List open browser tabs |
| `switchTab(tabId)` | Switch to tab |
| `newTab(url?)` | Open new tab |
| `closeTab(tabId)` | Close tab |

## Types

### PageState

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

### Element

```typescript
interface Element {
  id: string;           // Stable semantic ID (e.g. "e1", "e42")
  role: string;         // Semantic role: "button", "a", "text", "select", "checkbox", etc.
  name?: string;        // Accessible label
  value?: string;       // Current value
  description?: string;
  bounds?: {            // Viewport coordinates
    x: number;
    y: number;
    width: number;
    height: number;
  };
  visible: boolean;     // Computed from display, opacity, dimensions
  interactable: boolean; // visible + enabled + hit-test
  focused: boolean;
  enabled: boolean;
  checked?: boolean;
  selected?: boolean;
  expanded?: boolean;
  required?: boolean;
}
```

### ActionResult

```typescript
interface ActionResult {
  success: boolean;
  pageState?: PageState;
  durationMs: number;
}
```

### PageEvent

```typescript
type PageEvent =
  | { type: "page.mutation"; data: MutationEvent[] }
  | { type: "page.loaded"; data: { url: string; title: string; timestampMs: number } }
  | { type: "page.navigated"; data: { url: string; previousUrl?: string; timestampMs: number } }
  | { type: "page.focus"; data: { elementId?: string; role?: string; name?: string; timestampMs: number } }
  | { type: "page.scroll"; data: { scrollX: number; scrollY: number; timestampMs: number } }
  | { type: "page.resize"; data: { viewportWidth: number; viewportHeight: number; timestampMs: number } };
```

## Extension-Backed Sessions

For real browser tabs with cookies, auth, and extensions:

```typescript
const client = new TivanaClient({ url: "ws://localhost:9876" });
await client.connect();

// Attach to the tab controlled by the Chrome extension
const ext = await client.request("session.fromExtension", {});
```

Requires the Tivana Chrome extension with a tab attached.

## Auto-Reconnect

The SDK automatically reconnects with exponential backoff (1s → 2s → 4s → max 30s) and queues commands during reconnect. No manual handling needed.

## Error Handling

```typescript
try {
  await client.click("e999");
} catch (error) {
  // Error: [element_not_found] Element e999 not found
  console.error(String(error));
}
```

Error codes: `element_not_found`, `session_not_found`, `browser_disconnected`, `timeout`, `invalid_params`, `unknown_method`.

## Runtime

The SDK requires the Tivana runtime. Start it with:

```bash
npx @mostrom/tivana
```

The CLI auto-downloads the prebuilt binary for your platform on first run. Options:

```bash
npx @mostrom/tivana --headless       # Headless mode
npx @mostrom/tivana --port 3000      # Custom port
npx @mostrom/tivana --connect 9222   # Attach to existing Chrome
```

## Examples

See [examples/](../../examples/) for 7 working demos covering exploration, agent loops, accessibility review, anomaly detection, form awareness, event streaming, and design token extraction.

## License

MIT © Mostrom LLC 2025
