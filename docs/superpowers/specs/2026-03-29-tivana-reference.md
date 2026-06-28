# Tivana Reference

**Date:** 2026-03-29
**Status:** Canonical
**Canonical Status:** This is the consolidated Tivana runtime, protocol, SDK, and development reference. It replaces the previously scattered material under `docs/tivana/`.

---

## Overview

Tivana is a browser perception protocol for AI agents.

It is designed for agents that need semantic awareness of live web pages rather
than scripted browser automation. Instead of hardcoded selectors or pure
screenshot reasoning, Tivana gives agents structured page state, stable element
references, and action primitives over Chromium via CDP.

### Positioning

- Not Playwright or Puppeteer-style scripted automation
- Not screenshot-only computer use
- Not site-specific workflows
- A protocol and runtime for perceive -> reason -> act browser control

### Primary Use Cases

- Autonomous browser agents
- Accessibility review and anomaly detection
- Exploratory QA
- Multi-step web flows such as sign-in, checkout, and form completion
- Structured extraction and observation of dynamic pages

---

## Architecture

Tivana has three moving parts:

1. **Browser**
   Chromium-based browser such as Chrome, Edge, Brave, Arc, or system Chromium.
2. **Runtime**
   Rust process that connects to the browser through CDP, builds semantic page
   state, and executes actions.
3. **Agent**
   Any system that can consume structured state and emit actions.

### System Shape

```text
Browser <-> Tivana Runtime <-> Agent
```

### Browser Transports

#### Managed Browser

- Runtime launches Chromium with remote debugging enabled
- Best for isolated sessions and headless workflows

#### Extension-Backed Session

- Chrome extension connects an existing user tab via `chrome.debugger`
- Best when the agent needs the user’s real browser profile, cookies, and login state

Both transports expose the same perception and action API to agents.

---

## Core Model

Tivana is built around two ideas:

1. **Snapshots**
   Request/response calls for complete state such as page state or current
   interactive elements.
2. **Events**
   Streaming deltas such as mutations, focus changes, scroll, resize, and
   navigation.

The usual agent pattern is:

1. Create a session
2. Navigate
3. Request `pageState`
4. Request `elements`
5. Start observation
6. Act by element ID
7. Re-snapshot when state becomes uncertain

---

## Protocol

### Transport

- WebSocket
- JSON messages
- Bidirectional request/response plus server-pushed events
- Default port: `9876`

### Message Envelope

```typescript
interface RequestMessage {
  id: string;
  type: "request";
  method: string;
  sessionId?: string;
  params: object;
  version?: string;
}

interface ResponseMessage {
  id: string;
  type: "response";
  result?: object;
  error?: {
    code: string;
    message: string;
    data?: unknown;
  };
  version: string;
}

interface EventMessage {
  id: string;
  type: "event";
  event: string;
  sessionId?: string;
  data: object;
  version: string;
}
```

### Main Method Families

#### Session

- `session.create`
- `session.close`
- `session.attach`

#### Perception

- `perceive.pageState`
- `perceive.elements`
- `perceive.observe`
- `perceive.unobserve`

#### Actions

- `act.navigate`
- `act.click`
- `act.type`
- `act.fill`
- `act.press`
- `act.scroll`
- `act.select`
- `act.hover`
- `act.screenshot`
- `act.wait`
- `act.uploadFile`
- `act.evaluate`

### Error Categories

- Client/request validation
- Session not found or disconnected
- Browser launch/control failures
- Unsupported target or action
- Runtime/internal failures

Agents should treat errors as recoverable where possible: re-snapshot, retry,
or choose a different target.

---

## Perception Model

### Page State

Page state includes:

- Current URL and title
- Viewport size
- Scroll position
- Document size
- Focused element when available

### Elements

Interactive elements are represented semantically rather than as selectors.

```typescript
interface Element {
  id: string;
  role: string;
  name?: string;
  value?: string;
  description?: string;
  bounds?: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
  styles?: {
    fontFamily?: string;
    fontSize?: string;
    fontWeight?: string;
    color?: string;
    backgroundColor?: string;
    border?: string;
    display?: string;
    visibility?: string;
  };
  focused: boolean;
  enabled: boolean;
  checked?: boolean;
  selected?: boolean;
  expanded?: boolean;
  required?: boolean;
  children?: Element[];
}
```

### Element IDs

- IDs use the `eN` format such as `e1`
- IDs are intended to be stable enough for agents to reference elements between
  observations
- Agents should still re-snapshot after failed actions or major UI changes

### What Tivana Extracts

- Accessibility role and naming
- Current value and state flags
- Bounding boxes
- Subset of computed styles
- Focus and interaction state

### What Tivana Does Not Fully Solve

- CAPTCHA solving
- Internal structure of canvas/WebGL
- Text embedded only in images
- Full introspection of hostile cross-origin iframes

---

## Action Model

Agents act semantically through the runtime. The runtime owns CDP details.

### Navigation

- `navigate(url)`

### Pointer and Keyboard Input

- `click(target)`
- `hover(target)`
- `type(text, target?, options?)`
- `press(key, modifiers?)`

### Form Control

- `fill(target, value)`
- `select(target, value)`
- `uploadFile(target, filePaths)`

### Page Control

- `scroll(...)`
- `wait(seconds)`
- `screenshot()`
- `evaluate(script)`

### Action Targeting

Targets can be expressed by:

- Tivana element ID
- CSS selector
- Semantic lookup such as role and label
- Coordinates in lower-level cases

The runtime resolves targets, synthesizes CDP input, and returns action
results plus resulting observation changes.

---

## Observation Model

Use both snapshots and events together.

### Snapshot Use

- Initial load
- Recovery after reconnect
- After navigation
- When incremental state no longer feels trustworthy

### Event Use

- `page.loaded`
- `page.mutation`
- `page.focus`
- `page.scroll`
- `page.resize`
- `page.navigated`

### Recommended Agent Loop

```typescript
const client = new TivanaClient();
await client.connect();
await client.createSession();

await client.navigate("https://example.com");

const [page, elements] = await Promise.all([
  client.pageState(),
  client.elements(),
]);

// reason with model here

await client.click("e5");
```

Keep a local state model, apply deltas from events, and re-snapshot on doubt.

---

## Integration

### TypeScript SDK

The TS SDK is a thin WebSocket client around the protocol.

Typical usage:

```typescript
import { TivanaClient } from "@mostrom/tivana";

const client = new TivanaClient({ url: "ws://localhost:9876" });
await client.connect();
await client.createSession();

const state = await client.pageState();
const elements = await client.elements();

await client.closeSession();
client.disconnect();
```

### Quick Start

```bash
cd runtime && cargo build --release
./target/release/tivana

cd sdk/ts && bun install && bun run build
```

### Local Development Requirements

- Rust 1.75+
- Chromium-based browser
- Bun 1.x or Node 18+

---

## Edge Cases

Tivana must handle or degrade gracefully for:

- Infinite scroll and lazy-loaded content
- SPAs and soft navigation
- Live-updating pages
- Modals, overlays, and stacked elements
- Dropdowns and hover-only menus
- Shadow DOM
- Cross-origin iframes
- CSS transforms and animations
- Large DOMs and rapid mutation streams
- Slow network and partially loaded pages
- OAuth popups and multi-window auth flows
- Bot detection and CAPTCHA boundaries

The right recovery strategy is usually:

1. Wait briefly
2. Re-snapshot
3. Retry with a different target or action
4. Escalate to the user only when genuinely blocked

---

## Success Criteria

### Developer Experience

- Install, start, connect in three steps or less
- Works with any Chromium-based browser
- Agent-agnostic protocol and SDK

### Perception Quality

- Structured element references
- Computed style awareness
- Accessibility-aware state
- Streaming updates for changed state

### Action Reliability

- Target by stable semantic ID rather than brittle selectors
- Visible real-time action execution
- Result reporting and recoverable failures

### Performance

- Small text-first payloads compared with screenshots
- Incremental updates rather than full-page retransmission
- Works on large, dynamic pages without constant polling

---

## Technology Choices

### Runtime

- Rust
- `chromiumoxide`
- `tokio`
- `tokio-tungstenite`
- `serde`
- `clap`

### SDK

- TypeScript
- WebSocket client
- Thin abstraction over the protocol

### Why This Stack

- Direct CDP control
- Predictable performance
- Runtime keeps browser complexity away from the agent
- TS SDK stays easy to adopt

---

## Development Notes

Repo layout:

```text
runtime/   Rust runtime
sdk/ts/    TypeScript SDK
extension/ Chrome extension for extension-backed sessions
examples/  Working demos
docs/      Canonical specs and historical docs
```

Main runtime modules:

- `main.rs`
- `server.rs`
- `session.rs`
- `browser.rs`
- `perceive.rs`
- `act.rs`
- `protocol.rs`
- `error.rs`

The important code boundaries are:

- `perceive.rs` for page and element extraction
- `act.rs` for action execution
- `sdk/ts/src/` for client and types

---

## Relationship To Atlas

Tivana is the browser perception substrate.

Atlas is the Electron browser product that reuses Tivana concepts and ports
relevant perception/action logic into an embedded browser experience.

For Atlas product requirements and MVP scope, use:

- `2026-03-28-tivana-atlas-design.md`

