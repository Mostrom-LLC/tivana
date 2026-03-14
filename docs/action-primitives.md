# Action Primitives

Available actions that agents can perform through the Tivana protocol.

---

## Navigation Actions

### navigate

Navigate to a URL.

```typescript
await client.navigate("https://example.com");
```

**Parameters:**
- `url` (required): URL to navigate to (absolute or relative)

**Returns:** `ActionResult` with updated page state

---

## Input Actions

### click

Click an element.

```typescript
// By element ID
await client.click("e5");

// By CSS selector
await client.click("button.submit");

// By role and label
await client.click({ role: "button", label: "Submit" });
```

**Parameters:**
- `target` (required): Element ID, selector, or `{ role, label }`
- `button` (optional): `"left"`, `"right"`, or `"middle"` (default: `"left"`)
- `clickCount` (optional): Number of clicks (default: 1, use 2 for double-click)
- `delayMs` (optional): Delay between clicks in ms
- `modifiers` (optional): `["Control"]`, `["Shift"]`, etc.

### type

Type text into an element or the focused element.

```typescript
// Type into focused element
await client.type("hello world");

// Type into specific element
await client.type("hello", "e3");

// With options
await client.type("hello", "e3", { clearFirst: true, delayMs: 50 });
```

**Parameters:**
- `text` (required): Text to type
- `target` (optional): Element ID or selector to focus first
- `clearFirst` (optional): Clear existing content before typing
- `delayMs` (optional): Delay between keystrokes

### press

Press a key or key combination.

```typescript
// Single key
await client.press("Enter");
await client.press("Tab");
await client.press("Escape");

// Key combination
await client.press("a", ["Control"]);  // Ctrl+A
await client.press("c", ["Control"]);  // Ctrl+C
```

**Parameters:**
- `key` (required): Key name (`Enter`, `Tab`, `Escape`, `ArrowDown`, etc.)
- `modifiers` (optional): `["Control"]`, `["Shift"]`, `["Alt"]`, `["Meta"]`

### hover

Move mouse over an element to trigger hover states.

```typescript
await client.hover("e5");
await client.hover("button.dropdown");
```

**Parameters:**
- `target` (required): Element ID or selector

### focus

Focus an element.

```typescript
await client.focus("e3");
await client.focus("input#email");
```

**Parameters:**
- `target` (required): Element ID or selector

---

## Scroll Actions

### scroll

Scroll the page or an element into view.

```typescript
// Scroll element into view
await client.scroll("e10");

// Scroll page by direction
await client.scroll(undefined, "down", { amount: 300 });

// Scroll with smooth animation
await client.scroll("footer", undefined, { smooth: true });
```

**Parameters:**
- `target` (optional): Element ID or selector to scroll into view
- `direction` (optional): `"up"`, `"down"`, `"left"`, `"right"`
- `amount` (optional): Pixels to scroll (default: 100)
- `smooth` (optional): Use smooth scrolling animation

---

## Form Actions

### select

Select an option from a dropdown.

```typescript
await client.select("select#country", "US");
await client.select("e7", "option-value");
```

**Parameters:**
- `target` (required): Element ID or selector (must be a `<select>`)
- `value` (required): Option value to select

---

## Wait Actions

### waitFor

Wait for a condition before continuing.

```typescript
// Wait for element to exist
await client.waitFor({ type: "Element", selector: ".modal" });

// Wait for element to be visible
await client.waitFor({ type: "Visible", selector: "#content" });

// Wait for element to disappear
await client.waitFor({ type: "Hidden", selector: ".spinner" });

// Wait for navigation
await client.waitFor({ type: "Navigation" });

// Wait for network idle
await client.waitFor({ type: "NetworkIdle" });

// Wait for a delay
await client.waitFor({ type: "Delay", durationMs: 1000 });
```

**Parameters:**
- `condition` (required): Condition to wait for
- `timeoutMs` (optional): Maximum wait time (default: 30000)

**Condition types:**
| Type | Description |
|------|-------------|
| `Element` | Wait for element matching selector to exist |
| `Visible` | Wait for element to be visible |
| `Hidden` | Wait for element to be hidden/removed |
| `Navigation` | Wait for page navigation to complete |
| `NetworkIdle` | Wait for network requests to finish |
| `Delay` | Wait for a fixed duration |

---

## Action Result

All actions return an `ActionResult`:

```typescript
interface ActionResult {
  success: boolean;       // Whether action succeeded
  pageState?: PageState;  // Updated page state after action
  data?: unknown;         // Action-specific result data
  durationMs: number;     // How long the action took
}
```

### Result Data Examples

**Click result:**
```json
{ "clickedAt": { "x": 350, "y": 200 } }
```

**Type result:**
```json
{ "typed": 11 }  // characters typed
```

**Press result:**
```json
{ "key": "Control+a" }
```

---

## Error Handling

Actions can fail for various reasons:

| Error Code | Meaning |
|------------|---------|
| `target_not_found` | Element doesn't exist or stale ID |
| `target_ambiguous` | Multiple elements match selector |
| `action_failed` | Action couldn't complete |
| `action_timeout` | Action timed out |

**Handling stale elements:**

```typescript
const result = await client.click("e5");
if (!result.success) {
  // Re-perceive to get fresh element IDs
  const elements = await client.elements();
  const button = elements.find(e => e.name === "Submit");
  if (button) {
    await client.click(button.id);
  }
}
```

---

## Best Practices

### Use Element IDs

Element IDs (`e1`, `e2`, etc.) are the most reliable targets:

```typescript
// Good: Use element ID from perceive.elements
const elements = await client.elements();
const submitBtn = elements.find(e => e.name === "Submit");
await client.click(submitBtn.id);

// Okay: Use CSS selector as fallback
await client.click("button[type='submit']");
```

### Handle Navigation

After clicking links, wait for navigation:

```typescript
await client.click("e3");  // Click a link
await client.waitFor({ type: "Navigation" });
const newState = await client.pageState();
```

### Check Results

Always check `success` for critical actions:

```typescript
const result = await client.type("user@example.com", "e5");
if (!result.success) {
  console.error("Failed to type into input");
}
```

### Use Updated Page State

Actions return updated page state — use it:

```typescript
const result = await client.navigate("https://example.com");
if (result.pageState) {
  console.log(`Now at: ${result.pageState.url}`);
}
```
