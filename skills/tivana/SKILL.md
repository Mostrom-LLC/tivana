# Tivana Skill

Streaming browser perception protocol for AI agents.

## When to Use

Use Tivana when you need to:
- Navigate and interact with web pages programmatically
- Perceive page state including elements, text, and visual styles
- Automate browser tasks (form filling, clicking, navigation)
- Test web applications
- Scrape structured data from websites

Tivana is **not** for:
- Simple HTTP requests (use `fetch` or `curl`)
- Static HTML parsing (use `cheerio` or similar)
- When you already have the data you need

## Prerequisites

### 1. Start the Tivana Runtime

The runtime must be running before you can use the SDK.

```bash
# If not built yet
cd tivana/runtime
cargo build --release

# Start the runtime (headed mode - browser visible)
./target/release/tivana start

# Or headless mode (no browser window)
./target/release/tivana start --headless
```

The runtime listens on `ws://localhost:9876` by default.

### 2. Install the SDK

```bash
# Using local SDK (pre-npm publish)
cd tivana/sdk/ts
bun install
```

Then import directly from the local path or use a symlink.

## Core Methods

### Connection

```typescript
import { TivanaClient } from "tivana";

const client = new TivanaClient();
await client.connect();  // Connects to ws://localhost:9876

// Custom URL
await client.connect("ws://localhost:8080");

// Disconnect when done
client.disconnect();
```

### Session Management

```typescript
// Create a browser session (launches Chromium)
await client.createSession();

// With options
await client.createSession({ headless: true });

// Close session
await client.closeSession();

// List all sessions
const sessions = await client.listSessions();
```

### Navigation

```typescript
// Navigate to URL
await client.navigate("https://example.com");

// Returns when page is loaded
```

### Perception

```typescript
// Get page state (URL, title, scroll position, viewport)
const state = await client.pageState();
// { url, title, scrollX, scrollY, viewportWidth, viewportHeight, timestampMs }

// Get interactive elements with visual data
const elements = await client.elements();
// [{ id, role, name, bounds, styles, focused, enabled, ... }]

// Get page text content
const text = await client.textContent();

// Get metadata (title, description, og:image)
const meta = await client.metadata();
```

### Actions

```typescript
// Click by element ID (from elements() response)
await client.click("e5");

// Click by role and label
await client.click({ role: "button", label: "Submit" });

// Type into element
await client.type("hello world", "e3");

// Type into focused element
await client.type("hello");

// Press key
await client.press("Enter");

// Key combination
await client.press("Control+A");

// Scroll element into view
await client.scroll("e10");

// Scroll page
await client.scroll(null, "down");

// Hover
await client.hover("e5");

// Select dropdown option
await client.select("e7", "option-value");

// Wait for condition
await client.waitFor({ type: "Navigation" });
await client.waitFor({ type: "Element", selector: "#result" });
```

## Example Flows

### Navigate and Read Page

```typescript
const client = new TivanaClient();
await client.connect();
await client.createSession();

await client.navigate("https://news.ycombinator.com");
const state = await client.pageState();
console.log(`Page: ${state.title}`);

const elements = await client.elements();
const links = elements.filter(e => e.role === "link");
console.log(`Found ${links.length} links`);

await client.closeSession();
client.disconnect();
```

### Fill and Submit Form

```typescript
await client.navigate("https://example.com/login");

// Get form elements
const elements = await client.elements();

// Find username field
const usernameField = elements.find(e => 
  e.role === "textbox" && e.name?.toLowerCase().includes("username")
);

// Find password field
const passwordField = elements.find(e =>
  e.role === "textbox" && e.name?.toLowerCase().includes("password")
);

// Find submit button
const submitButton = elements.find(e =>
  e.role === "button" && e.name?.toLowerCase().includes("sign in")
);

// Fill form
if (usernameField) await client.type("myuser", usernameField.id);
if (passwordField) await client.type("mypass", passwordField.id);
if (submitButton) await client.click(submitButton.id);

// Wait for navigation
await client.waitFor({ type: "Navigation" });
```

### Extract Data from Table

```typescript
await client.navigate("https://example.com/data");

const elements = await client.elements();

// Find table cells
const cells = elements.filter(e => e.role === "cell" || e.role === "gridcell");

// Extract text from cells
const data = cells.map(cell => ({
  id: cell.id,
  text: cell.name || cell.value || "",
  bounds: cell.bounds
}));

console.log(JSON.stringify(data, null, 2));
```

### Handle Dynamic Content

```typescript
await client.navigate("https://example.com/spa");

// Click a button that loads content
const loadButton = (await client.elements()).find(e => 
  e.name?.includes("Load More")
);
if (loadButton) {
  await client.click(loadButton.id);
  
  // Wait for new content
  await client.waitFor({ type: "Element", selector: ".new-content" });
  
  // Get updated elements
  const newElements = await client.elements();
  console.log(`Now have ${newElements.length} elements`);
}
```

## Error Handling

```typescript
try {
  await client.click("e999");
} catch (error) {
  // Error codes:
  // - target_not_found: Element doesn't exist
  // - target_ambiguous: Multiple matches
  // - action_failed: Click couldn't complete
  // - session_not_found: No active session
  // - browser_crashed: Browser died
  console.error(`Action failed: ${error.message}`);
}
```

### Common Errors

| Error | Cause | Solution |
|-------|-------|----------|
| `target_not_found` | Element ID doesn't exist | Call `elements()` to get fresh IDs after navigation |
| `session_not_found` | No browser session | Call `createSession()` first |
| `connection_refused` | Runtime not running | Start runtime with `tivana start` |
| `timeout` | Action took too long | Increase timeout or check page state |

## Tips

1. **Always refresh element IDs after navigation** - IDs reset when the page changes
2. **Use headless mode for automation** - Faster and uses less resources
3. **Check element enabled state** - Don't click disabled buttons
4. **Use role+label selectors for stability** - More resilient than IDs across page changes
5. **Handle errors gracefully** - Web pages are unpredictable

## Reference

- [Full API Documentation](../sdk/ts/README.md)
- [Protocol Specification](../docs/protocol-specification.md)
- [Element Model](../docs/element-model.md)
- [Action Primitives](../docs/action-primitives.md)
