# Observation Guide

How to use Tivana's observation model to maintain continuous awareness of page state.

---

## Overview

Tivana provides two ways to perceive the browser:

| Mode | Mechanism | Data | Use case |
|------|-----------|------|----------|
| **Snapshot** | Request/response | Complete state | Initial load, recovery, full picture |
| **Event** | Server-push | IDs and deltas | Continuous awareness without polling |

Agents should use both together: take a snapshot to establish baseline state, then observe incremental events to stay current.

---

## Quick Start

### 1. Create a session and navigate

```json
{ "id": "1", "type": "request", "method": "session.create", "params": {} }
```

```json
{ "id": "2", "type": "request", "method": "act.navigate",
  "sessionId": "sess-abc", "params": { "url": "https://example.com" } }
```

### 2. Take a full snapshot

Request the initial page state and element tree to establish your baseline:

```json
{ "id": "3", "type": "request", "method": "perceive.pageState", "sessionId": "sess-abc" }
```

```json
{ "id": "4", "type": "request", "method": "perceive.elements", "sessionId": "sess-abc" }
```

Store these results — they are your ground truth until you take another snapshot.

### 3. Start observation

```json
{ "id": "5", "type": "request", "method": "perceive.observe",
  "sessionId": "sess-abc", "params": {} }
```

From this point, events will stream over the WebSocket.

### 4. Handle events

Events arrive as server-push messages. Apply them to your stored state:

```json
← { "type": "event", "event": "page.focus", "sessionId": "sess-abc",
     "data": { "elementId": "e5", "previousElementId": null, "timestampMs": 1711234567890 } }

← { "type": "event", "event": "page.mutation", "sessionId": "sess-abc",
     "data": [
       { "type": "Added", "elementId": "e42", "parentId": "e5" },
       { "type": "TextChanged", "elementId": "e3", "text": "Updated" }
     ] }

← { "type": "event", "event": "page.scroll", "sessionId": "sess-abc",
     "data": { "scrollX": 0, "scrollY": 300, "timestampMs": 1711234568090 } }
```

### 5. Re-snapshot when needed

After navigation, reconnect, or when state feels uncertain:

```json
{ "id": "10", "type": "request", "method": "perceive.pageState", "sessionId": "sess-abc" }
{ "id": "11", "type": "request", "method": "perceive.elements", "sessionId": "sess-abc" }
```

### 6. Stop observation

```json
{ "id": "20", "type": "request", "method": "perceive.unobserve",
  "sessionId": "sess-abc", "params": {} }
```

---

## Event Reference

### page.loaded

Fired when DOMContentLoaded occurs after a full navigation.

```json
{
  "url": "https://example.com/page2",
  "title": "Page Two",
  "timestampMs": 1711234567890
}
```

**Agent action**: Update stored URL and title. Consider requesting a fresh element snapshot since the DOM has fully changed.

### page.navigated

Fired on URL changes from pushState, replaceState, popstate, or hashchange. Unlike `page.loaded`, these are client-side navigations that may not replace the full DOM.

```json
{
  "url": "https://example.com/page2",
  "navigationType": "pushState",
  "timestampMs": 1711234567891
}
```

**Agent action**: Update stored URL. For SPA navigations (`pushState`, `replaceState`), the DOM may change incrementally — mutation events will follow. For `popstate` or `hashchange`, consider a fresh snapshot if the page structure is unknown.

### page.mutation

Fired when the DOM changes. Data is an array (batch) of individual mutations:

| Mutation type | Fields | Meaning |
|---------------|--------|---------|
| `Added` | `elementId`, `parentId?` | New element appeared in the DOM |
| `Removed` | `elementId` | Element was removed from the DOM |
| `Changed` | `elementId`, `attribute`, `oldValue?`, `newValue?` | Element attribute changed |
| `TextChanged` | `elementId`, `text` | Element text content changed |

**Agent action**: Update your element model. For `Added` elements, the event provides the element ID and parent — request `perceive.elements` if you need full element data (bounds, role, etc.).

### page.focus

Fired when the focused element changes.

```json
{
  "elementId": "e5",
  "previousElementId": "e3",
  "timestampMs": 1711234567892
}
```

**Agent action**: Update your tracked focus state. Both fields may be `null` (focus moved to/from a non-tracked element or document body).

### page.scroll

Fired when scroll position changes. **Throttled to 200ms** — you will not receive more than 5 scroll events per second.

```json
{
  "scrollX": 0,
  "scrollY": 450,
  "timestampMs": 1711234567893
}
```

**Agent action**: Update stored scroll position. Element bounds from your last snapshot are still valid (they are in page coordinates), but visibility relative to viewport may have changed.

### page.resize

Fired when the viewport dimensions change.

```json
{
  "viewportWidth": 1024,
  "viewportHeight": 768,
  "timestampMs": 1711234567894
}
```

**Agent action**: Update stored viewport dimensions. Element bounds may have changed due to reflow — consider requesting a fresh element snapshot if layout-sensitive work is in progress.

---

## State Management

### What events carry

Events carry **IDs and deltas**, not full element data. A `page.mutation` event with `{ "type": "Added", "elementId": "e42" }` tells you an element appeared — but not its role, text, bounds, or styles.

### What agents should maintain

Agents should maintain a local state model:

```
AgentState {
  url: string
  title: string
  scrollX: number
  scrollY: number
  viewportWidth: number
  viewportHeight: number
  focusedElementId: string | null
  elements: Map<string, Element>    // from last perceive.elements snapshot
  lastSnapshotMs: number
}
```

Apply incoming events to update this model. When the model becomes uncertain (e.g., many mutations, navigation, reconnect), take a fresh snapshot.

### When to re-snapshot

| Trigger | Why |
|---------|-----|
| After `page.loaded` | The DOM was fully replaced |
| After reconnection | Events were missed during disconnect |
| After many mutations | Accumulated deltas may have drifted from truth |
| Before complex interaction | Need accurate element bounds for clicking |
| After `page.resize` | Element bounds may have changed due to reflow |

---

## Recovery After Reconnect

If the WebSocket connection drops and reconnects:

1. **Observation is no longer active** — events were not buffered during disconnect
2. **Re-request a full snapshot** to establish new baseline:
   ```json
   { "id": "r1", "type": "request", "method": "perceive.pageState", "sessionId": "sess-abc" }
   { "id": "r2", "type": "request", "method": "perceive.elements", "sessionId": "sess-abc" }
   ```
3. **Re-start observation**:
   ```json
   { "id": "r3", "type": "request", "method": "perceive.observe",
     "sessionId": "sess-abc", "params": {} }
   ```
4. **Resume processing events** from this point forward

There is no event replay or sequence numbering. The snapshot is the synchronization point.

---

## Complete Example Flow

```
Agent                         Runtime                        Browser
  │                              │                              │
  │── session.create ──────────►│── launch Chromium ──────────►│
  │◄── { sessionId: "s1" } ─────│                              │
  │                              │                              │
  │── act.navigate ────────────►│── CDP Page.navigate ────────►│
  │◄── { success: true } ───────│                              │
  │                              │                              │
  │── perceive.pageState ──────►│── evaluate JS ──────────────►│
  │◄── { url, title, ... } ─────│◄── page data ────────────────│
  │                              │                              │
  │── perceive.elements ───────►│── evaluate JS ──────────────►│
  │◄── { elements: [...] } ─────│◄── element data ─────────────│
  │                              │                              │
  │   (agent stores snapshot)    │                              │
  │                              │                              │
  │── perceive.observe ────────►│── inject MutationObserver ──►│
  │◄── { observing: true } ─────│── inject event listeners ───►│
  │                              │                              │
  │   ... user types in input ...│                              │
  │                              │                              │
  │◄── page.focus ───────────────│◄── focusin ──────────────────│
  │◄── page.mutation ────────────│◄── MutationObserver ─────────│
  │                              │                              │
  │   (agent applies deltas)     │                              │
  │                              │                              │
  │── act.click("e5") ─────────►│── CDP Input.dispatch ───────►│
  │◄── { success: true } ───────│                              │
  │◄── page.mutation ────────────│◄── DOM changed ──────────────│
  │◄── page.navigated ──────────│◄── pushState ────────────────│
  │                              │                              │
  │   (agent detects navigation) │                              │
  │                              │                              │
  │── perceive.elements ───────►│── evaluate JS ──────────────►│
  │◄── { elements: [...] } ─────│◄── element data ─────────────│
  │                              │                              │
  │   (agent updates snapshot)   │                              │
  │                              │                              │
  │── perceive.unobserve ──────►│── remove listeners ─────────►│
  │◄── { observing: false } ────│                              │
  │                              │                              │
  │── session.close ───────────►│── close Chromium ───────────►│
  │◄── { closed: true } ────────│                              │
```

---

## Tips

- **Don't poll.** Use `perceive.observe` and let events come to you.
- **Snapshot is cheap.** When in doubt, re-snapshot. It's a single request/response.
- **Events are lightweight.** They carry IDs, not full element data — low bandwidth.
- **Element IDs are stable within a page.** After navigation, IDs reset — re-snapshot.
- **Scroll events are throttled.** You'll get at most 5 per second — safe to process synchronously.
- **Mutation batches can be large.** A single `page.mutation` event may contain many individual mutations — process the whole array.
