---
name: tivana
description: "Browser perception protocol for AI agents via Tivana runtime + TypeScript SDK. Use when: (1) perceiving web page structure, elements, and state semantically, (2) building agent loops that perceive → reason → act on web pages, (3) clicking, typing, scrolling, or navigating by semantic element ID, (4) monitoring page mutations and events in real-time, (5) running accessibility reviews or anomaly detection on live pages, (6) extracting design tokens or page metadata, (7) filling forms through perception rather than hardcoded selectors. NOT for: site-specific automation scripts with hardcoded selectors, CAPTCHA solving as primary goal, or tasks that don't involve browser interaction. Requires: Tivana runtime (Rust binary) running on localhost, or Chrome extension for extension-backed sessions."
---

# Tivana — Browser Perception for Agents

Tivana provides semantic browser awareness. Perceive the page, reason about it, act on it.

## Architecture

```
Agent (you) ←→ Tivana SDK (TypeScript) ←→ Tivana Runtime (Rust, WebSocket)
                                              ↕
                                         Browser (CDP)
                                              or
                                         Chrome Extension
```

## Quick Start

```typescript
import { TivanaClient } from "@mostrom/tivana";

const client = new TivanaClient({ url: "ws://localhost:9876" });
await client.connect();
await client.createSession();

// Perceive
const page = await client.pageState();
const elements = await client.elements();

// Reason (your logic / LLM call)
const target = elements.find(e => e.role === "button" && e.name?.includes("Submit"));

// Act
if (target) await client.click(target.id);

await client.closeSession();
client.disconnect();
```

## Core API

### Session

| Method | Description |
|--------|-------------|
| `connect(url?)` | Connect to runtime (default `ws://localhost:9876`) |
| `createSession(opts?)` | Create managed browser session (`{ headless?: boolean }`) |
| `closeSession()` | Close session and browser |
| `request(method, params)` | Send raw protocol request |

### Perceive

| Method | Returns | Description |
|--------|---------|-------------|
| `pageState()` | `PageState` | URL, title, viewport, scroll position, document size |
| `elements()` | `Element[]` | All interactive elements with id, role, name, bounds, visible, interactable |
| `accessibilitySnapshot()` | `AccessibilitySnapshot` | Accessibility tree |
| `textContent()` | `string` | Full page text |
| `evaluate(js)` | `any` | Execute JS in page context |

### Act

| Method | Description |
|--------|-------------|
| `click(elementId)` | Click element by semantic ID |
| `type(text, elementId?)` | Type text into element or focused element |
| `press(key, modifiers?)` | Press key (Enter, Tab, Escape, etc.) |
| `scroll(elementId?, direction?)` | Scroll element or page |
| `navigate(url)` | Navigate to URL |
| `hover(elementId)` | Hover over element |
| `focus(elementId)` | Focus element |
| `select(elementId, values)` | Select dropdown option |

### Observe

| Method | Description |
|--------|-------------|
| `startObservation()` | Begin streaming page events and mutations |
| `stopObservation()` | Stop streaming |
| `onEvent(callback)` | Subscribe to all events |
| `onPageEvent(type, callback)` | Subscribe to specific event type |

Event types: `page.mutation`, `page.loaded`, `page.navigated`, `page.focus`, `page.scroll`, `page.resize`

## Element Properties

Each element from `elements()` includes:

- `id` — stable semantic ID (e.g., `e1`, `e42`)
- `role` — semantic role (`button`, `a`, `text`, `select`, `checkbox`, etc.)
- `name` — accessible label
- `value` — current value
- `visible` — computed visibility (display, opacity, dimensions)
- `interactable` — visible + enabled + hit-test passes
- `enabled` — not disabled
- `focused` — has focus
- `required` — required field
- `checked` — checkbox/radio state
- `bounds` — `{ x, y, width, height }` viewport coordinates

## Extension-Backed Sessions

For real browser tabs (with cookies, auth, extensions):

```typescript
const ext = await client.request("session.fromExtension", {});
// Now all perceive/act commands target the real browser tab
```

Requires the Tivana Chrome extension installed and a tab attached.

## Canonical Agent Loop Pattern

```typescript
for (let step = 0; step < maxSteps; step++) {
  const [page, elements] = await Promise.all([
    client.pageState(),
    client.elements(),
  ]);

  // Send perception to LLM for decision
  const decision = await yourModel.decide({ goal, page, elements });

  if (decision.type === "done") break;
  if (decision.type === "click") await client.click(decision.target);
  if (decision.type === "type") await client.type(decision.text, decision.target);
  if (decision.type === "navigate") await client.navigate(decision.url);
}
```

## Starting the Runtime

```bash
# Default (headed, port 9876)
./tivana

# Headless
./tivana --headless

# Custom port
./tivana --port 3000

# Connect to existing Chrome
./tivana --connect 9222
```

## Examples

See `examples/` in the repo for working demos:
- `01-observe-and-explore.ts` — page structure discovery
- `02-agent-loop.ts` — perceive → reason → act
- `03-accessibility-review.ts` — a11y audit via perception
- `04-anomaly-detection.ts` — visual/structural QA
- `05-form-awareness.ts` — form understanding without selectors
- `06-event-streaming.ts` — real-time observation
- `07-design-token-extraction.ts` — W3C DTCG token extraction

## Key Principle

Tivana provides perception and action primitives. Site-specific logic, field matching, label heuristics, and decision-making belong in the agent, not in Tivana calls. Always perceive first, then reason, then act.
