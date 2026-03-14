# Protocol Specification

Message formats for communication between the Tivana runtime and agents.

---

## Transport

- **WebSocket** connection between runtime and agent
- **JSON** message format
- **Bidirectional**: runtime responds to requests and pushes events
- **Default port**: 9876
- **Protocol version**: "1.0"

---

## Message Envelope

All messages share a common envelope structure:

### Request Message (Agent → Runtime)

```typescript
{
  id: string;           // Unique message ID for correlation
  type: "request";      // Always "request" for inbound
  method: string;       // Method to invoke (e.g., "session.create")
  sessionId?: string;   // Required for session-scoped methods
  params: object;       // Method-specific parameters
  version?: string;     // Protocol version (default: "1.0")
}
```

### Response Message (Runtime → Agent)

```typescript
{
  id: string;           // Correlated request ID
  type: "response";     // Always "response"
  result?: object;      // Result payload (on success)
  error?: {             // Error details (on failure)
    code: string;
    message: string;
    data?: unknown;
  };
  version: string;      // Protocol version
}
```

### Event Message (Runtime → Agent)

```typescript
{
  id: string;           // Unique event ID
  type: "event";        // Always "event"
  event: string;        // Event name (e.g., "page.mutation")
  sessionId?: string;   // Session this event relates to
  data: object;         // Event payload
  version: string;      // Protocol version
}
```

---

## Session Methods

### session.create

Create a new browser session.

**Request params:**
```typescript
{
  headless?: boolean;   // Run in headless mode (default: true)
}
```

**Response result:**
```typescript
{
  sessionId: string;    // Created session ID
  status: "created" | "launching" | "active";
}
```

### session.close

Close an active session.

**Request params:** (none)

**Response result:** `{}`

### session.list

List all active sessions.

**Request params:** (none)

**Response result:**
```typescript
{
  sessions: Array<{
    sessionId: string;
    status: string;
    url?: string;
  }>;
}
```

---

## Perceive Methods

### perceive.pageState

Get current page state snapshot.

**Response result:**
```typescript
{
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

### perceive.elements

Get interactive elements on the page.

**Response result:** `Element[]` (see Element Model)

### perceive.accessibilitySnapshot

Get full accessibility tree snapshot.

**Response result:**
```typescript
{
  root?: Element;
  interactiveElements: Element[];
  timestampMs: number;
}
```

### perceive.textContent

Get page text content.

**Response result:**
```typescript
{
  text: string;
  wordCount: number;
  charCount: number;
}
```

### perceive.metadata

Get page metadata.

**Response result:**
```typescript
{
  url: string;
  title?: string;
  description?: string;
  favicon?: string;
  ogImage?: string;
  language?: string;
}
```

### perceive.findElements

Find elements matching a CSS selector.

**Request params:**
```typescript
{
  selector: string;
}
```

**Response result:**
```typescript
Array<{
  selector: string;
  tagName: string;
  text?: string;
  attributes: Record<string, string>;
  bounds?: { x, y, width, height };
}>
```

---

## Act Methods

### act.navigate

Navigate to a URL.

**Request params:**
```typescript
{
  url: string;
}
```

**Response result:** `ActionResult`

### act.click

Click on an element.

**Request params:**
```typescript
{
  target: ActionTarget;
  button?: "left" | "right" | "middle";
  clickCount?: number;
  delayMs?: number;
  modifiers?: string[];
}
```

**Response result:** `ActionResult`

### act.type

Type text into an element.

**Request params:**
```typescript
{
  text: string;
  target?: ActionTarget;
  delayMs?: number;
  clearFirst?: boolean;
}
```

**Response result:** `ActionResult`

### act.press

Press a key or key combination.

**Request params:**
```typescript
{
  key: string;
  modifiers?: string[];
}
```

**Response result:** `ActionResult`

### act.scroll

Scroll the page or element.

**Request params:**
```typescript
{
  target?: ActionTarget;
  direction?: "up" | "down" | "left" | "right";
  amount?: number;
  smooth?: boolean;
}
```

**Response result:** `ActionResult`

### act.hover

Hover over an element.

**Request params:**
```typescript
{
  target: ActionTarget;
}
```

**Response result:** `ActionResult`

### act.focus

Focus an element.

**Request params:**
```typescript
{
  target: ActionTarget;
}
```

**Response result:** `ActionResult`

### act.select

Select an option from a dropdown.

**Request params:**
```typescript
{
  target: ActionTarget;
  value: string;
}
```

**Response result:** `ActionResult`

### act.waitFor

Wait for a condition.

**Request params:**
```typescript
{
  condition: WaitCondition;
  timeoutMs?: number;
}
```

**Response result:** `ActionResult`

---

## Common Types

### ActionTarget

Target element for actions:

```typescript
{
  elementId?: string;    // Element ID from perceive.elements (e.g., "e1")
  selector?: string;     // CSS selector
  text?: string;         // Text content to match
  role?: string;         // ARIA role
  label?: string;        // ARIA label
  coordinates?: [x, y];  // Direct coordinates
}
```

Priority: `coordinates` > `elementId` > `selector` > `role+label`

### ActionResult

Result of an action:

```typescript
{
  success: boolean;
  pageState?: PageState;   // Updated page state
  data?: unknown;          // Action-specific data
  durationMs: number;      // Action duration
}
```

### WaitCondition

Condition to wait for:

```typescript
// Wait for element to exist
{ type: "Element"; selector: string }

// Wait for element to be visible
{ type: "Visible"; selector: string }

// Wait for element to be hidden
{ type: "Hidden"; selector: string }

// Wait for navigation to complete
{ type: "Navigation" }

// Wait for network idle
{ type: "NetworkIdle"; idleTimeMs?: number }

// Wait for specific duration
{ type: "Delay"; durationMs: number }
```

---

## Error Codes

| Code | Description |
|------|-------------|
| `invalid_message` | Malformed message |
| `missing_field` | Required field missing |
| `unknown_method` | Unknown method name |
| `session_not_found` | Session ID not found |
| `session_closed` | Session already closed |
| `browser_launch_failed` | Failed to launch browser |
| `browser_crashed` | Browser crashed |
| `target_not_found` | Element target not found |
| `target_ambiguous` | Multiple elements match target |
| `action_failed` | Action execution failed |
| `action_timeout` | Action timed out |
| `internal_error` | Internal server error |

---

## Events

### page.mutation

Emitted when DOM changes occur:

```typescript
{
  event: "page.mutation";
  data: MutationEvent[];
}
```

Where `MutationEvent` is one of:

```typescript
{ type: "Added"; elementId: string; parentId?: string }
{ type: "Removed"; elementId: string }
{ type: "Changed"; elementId: string; attribute: string; oldValue?: string; newValue?: string }
{ type: "TextChanged"; elementId: string; text: string }
```
