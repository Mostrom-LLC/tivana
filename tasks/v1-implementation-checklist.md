# Tivana v1 Implementation Checklist

## 0. Freeze v1 scope
- [ ] Confirm v1 is **Chromium-only**
- [ ] Confirm v1 scope is only:
  - [ ] **Perceive** (streaming page state)
  - [ ] **Act** (click, type, scroll, navigate)
- [ ] Explicitly exclude for v1:
  - [ ] memory
  - [ ] planning
  - [ ] domain logic
  - [ ] non-Chromium browsers
  - [ ] npm publishing
  - [ ] **screenshots** (not even as fallback — streaming perception replaces them entirely)

---

## 1. Repo / workspace setup
### Rust runtime
- [ ] Create Rust crate/app entrypoint
- [ ] Add dependencies:
  - [ ] `chromiumoxide`
  - [ ] `tokio`
  - [ ] `tokio-tungstenite`
  - [ ] `serde`
  - [ ] `serde_json`
  - [ ] `clap`
  - [ ] `tracing`
- [ ] Add runtime module structure:
  - [ ] `src/main.rs`
  - [ ] `src/cli.rs`
  - [ ] `src/server.rs`
  - [ ] `src/session.rs`
  - [ ] `src/protocol.rs`
  - [ ] `src/browser.rs`
  - [ ] `src/perceive.rs`
  - [ ] `src/act.rs`
  - [ ] `src/error.rs`

### TypeScript SDK
- [ ] Create `sdk/ts` package
- [ ] Use Bun as default runtime/tooling
- [ ] Add Node fallback via `ws`
- [ ] Add:
  - [ ] `package.json`
  - [ ] `tsconfig.json`
  - [ ] `src/client.ts`
  - [ ] `src/types.ts`
  - [ ] `src/index.ts`

---

## 2. Protocol envelope
- [ ] Define JSON message envelope
- [ ] Every message has:
  - [ ] `id`
  - [ ] `type`
  - [ ] `method`
  - [ ] `sessionId` when applicable
  - [ ] `params`
  - [ ] `result`
  - [ ] `error`
- [ ] Define message types:
  - [ ] `request`
  - [ ] `response`
  - [ ] `event`
  - [ ] `error`
- [ ] Define protocol version field
- [ ] Define standard error shape:
  - [ ] `code`
  - [ ] `message`
  - [ ] `data`

> **Note:** Capability negotiation deferred to v2. Keep v1 simple.

---

## 3. Session model
- [ ] Implement session registry
- [ ] Implement session lifecycle states:
  - [ ] `created`
  - [ ] `launching`
  - [ ] `active`
  - [ ] `closed`
- [ ] Define session ownership rules
- [ ] Route all browser actions through `sessionId`
- [ ] Ensure session close cleans up browser/page handles

---

## 4. WebSocket server
- [ ] Start local WebSocket server
- [ ] Accept one or more clients
- [ ] Parse JSON frames
- [ ] Validate protocol envelope
- [ ] Route commands to session/browser handlers
- [ ] Return correlated responses
- [ ] Emit structured errors for invalid requests
- [ ] Add connection logging with `tracing`

---

## 5. Chromium runtime integration
- [ ] Launch Chromium via `chromiumoxide`
- [ ] Open a page/tab
- [ ] Navigate to URL
- [ ] Track active page handle inside session
- [ ] Close browser cleanly on session close
- [ ] Close browser cleanly on runtime shutdown
- [ ] Handle browser launch failures gracefully

---

## 6. Perceive primitives

> **⚠️ NO SCREENSHOTS.** Not as primary, not as fallback. Streaming semantic perception is the entire point. If we hit a wall that "needs" screenshots, we solve it with better perception — not by falling back to pixels.

### Minimum v1 perception
- [ ] `perceive.pageState`
  - [ ] current URL
  - [ ] page title
  - [ ] focused element ID
  - [ ] scroll position
  - [ ] viewport dimensions
  - [ ] timestamp
- [ ] `perceive.elements` — returns element tree with:
  - [ ] AXTree data (role, label, value, focused, enabled)
  - [ ] Computed styles (font, colors, borders, padding, margin)
  - [ ] Geometry (bounds via getBoundingClientRect)
- [ ] `perceive.mutations` — **event stream** (not polling)
  - [ ] element added
  - [ ] element removed
  - [ ] element changed (with changed properties)
  - [ ] focus changed
  - [ ] navigation occurred

### Output discipline
- [ ] Keep page-state payload normalized and compact
- [ ] Element IDs must be stable enough for follow-up actions
- [ ] Document mutation event semantics

---

## 7. Act primitives
### Minimum v1 actions
- [ ] `act.navigate` — go to URL
- [ ] `act.click` — click element by ID or selector
- [ ] `act.type` — type text into focused element or target
- [ ] `act.scroll` — scroll element into view

### Action requirements
- [ ] Accept element ID as primary target
- [ ] Accept role+label selector as fallback target
- [ ] Return success/failure in structured form
- [ ] Return new page state after action completes
- [ ] Fail clearly when target is missing/ambiguous

---

## 8. Element targeting model
- [ ] Primary: element ID from AXTree (e.g., `e1`, `e2`)
- [ ] Secondary: role + label selector (e.g., `{ role: "button", label: "Submit" }`)
- [ ] Define ID stability rules:
  - [ ] IDs are stable within a page session
  - [ ] IDs may change after navigation or major DOM mutation
- [ ] Define stale target behavior:
  - [ ] If element ID no longer exists, return `target_not_found` error
  - [ ] Agent must re-perceive to get fresh IDs
- [ ] Define ambiguous target behavior:
  - [ ] If multiple elements match selector, return `target_ambiguous` error with count

---

## 9. Error handling + recovery
- [ ] Separate error classes:
  - [ ] `protocol_error` — malformed message, missing fields
  - [ ] `session_error` — invalid session, session closed
  - [ ] `browser_error` — launch failed, crashed, disconnected
  - [ ] `action_error` — target not found, action failed
  - [ ] `perception_error` — failed to read page state
- [ ] Add disconnect handling (client disconnects mid-action)
- [ ] Add browser crash handling (restart session or error out)
- [ ] Add stale page/session handling
- [ ] Ensure runtime never silently hangs on failed browser action

---

## 10. CLI
- [ ] Implement `tivana` CLI with `clap`
- [ ] Support:
  - [ ] `tivana start` — start server
  - [ ] `--port <port>` — port selection (default: 9876)
  - [ ] `--headless` / `--headed` — browser visibility
  - [ ] `--chrome-path <path>` — chromium executable override
- [ ] Add helpful startup logs (port, mode, version)
- [ ] Add graceful shutdown on SIGINT/SIGTERM

---

## 11. TypeScript SDK v1
- [ ] Connect to runtime over WebSocket
- [ ] Bun WebSocket primary path
- [ ] Node `ws` fallback path
- [ ] Implement:
  - [ ] `connect(url)` — connect to runtime
  - [ ] `createSession()` — create browser session
  - [ ] `navigate(url)` — navigate to URL
  - [ ] `pageState()` — get current page state
  - [ ] `elements()` — get element tree
  - [ ] `click(target)` — click element
  - [ ] `type(text, target?)` — type text
  - [ ] `scroll(target)` — scroll to element
  - [ ] `onMutation(callback)` — subscribe to mutations
  - [ ] `closeSession()` — close session
  - [ ] `disconnect()` — disconnect from runtime
- [ ] Add typed request/response interfaces
- [ ] Add example script using the SDK end-to-end

---

## 12. Local verification
### Runtime verification
- [ ] Start runtime locally (`tivana start`)
- [ ] Connect with TS SDK
- [ ] Create session
- [ ] Chromium launches (visible in headed mode)
- [ ] Navigate to a page
- [ ] Read page state
- [ ] Read element tree with styles
- [ ] Click an element
- [ ] Type into an input
- [ ] Scroll to element
- [ ] Receive mutation events
- [ ] Close session
- [ ] Chromium closes
- [ ] Shut down runtime cleanly

### Reliability verification
- [ ] Bad request returns structured error
- [ ] Browser launch failure returns structured error
- [ ] Session close tears down resources
- [ ] Client disconnect does not crash runtime
- [ ] Stale element ID returns clear error

---

## 13. Test suite
### Rust
- [ ] Protocol serialization/deserialization tests
- [ ] Session lifecycle tests
- [ ] Action routing tests
- [ ] Error mapping tests

### SDK
- [ ] Client connect/disconnect tests
- [ ] Request correlation tests
- [ ] Bun WebSocket tests
- [ ] Node `ws` fallback tests

### End-to-end
- [ ] Smoke test:
  - [ ] connect
  - [ ] create session
  - [ ] launch Chromium
  - [ ] navigate
  - [ ] perceive page state
  - [ ] perceive elements
  - [ ] click
  - [ ] type
  - [ ] scroll
  - [ ] close session
  - [ ] disconnect

---

## 14. Docs to finish before calling v1 usable
- [ ] Protocol envelope reference (message shapes)
- [ ] Supported v1 methods (perceive.*, act.*)
- [ ] Session lifecycle doc
- [ ] Element model doc (AXTree + styles + geometry)
- [ ] Element targeting doc (IDs, selectors, staleness)
- [ ] Chromium-only scope doc
- [ ] Local run instructions
- [ ] TS SDK usage example
- [ ] Known limitations

---

# Recommended build order

## Phase 1 — Foundation
- [ ] Rust runtime skeleton (`cargo new`)
- [ ] Dependencies in `Cargo.toml`
- [ ] Module structure
- [ ] Protocol envelope types
- [ ] WebSocket server (accept connections, parse JSON)
- [ ] Session registry (create, get, close)

## Phase 2 — Browser + Perceive + Act
- [ ] Chromium launch/navigate/close via chromiumoxide
- [ ] `perceive.pageState`
- [ ] `perceive.elements` (AXTree + styles + geometry)
- [ ] `perceive.mutations` event stream
- [ ] `act.navigate`
- [ ] `act.click`
- [ ] `act.type`
- [ ] `act.scroll`

## Phase 3 — SDK + Polish
- [ ] TypeScript SDK (Bun primary, Node fallback)
- [ ] Local smoke test script
- [ ] Error handling hardening
- [ ] Basic docs
- [ ] README update with usage

---

# Definition of done for v1

Tivana v1 is "done" when:

- [ ] A TS client can connect to the Rust runtime
- [ ] Create a session (Chromium launches)
- [ ] Navigate to any page
- [ ] Perceive page state (URL, title, scroll, viewport)
- [ ] Perceive element tree (roles, labels, styles, geometry)
- [ ] Receive mutation events when DOM changes
- [ ] Click an element by ID
- [ ] Type text into an input
- [ ] Scroll to an element
- [ ] Receive structured errors for failures
- [ ] Close session (Chromium closes)
- [ ] Disconnect cleanly
- [ ] All of that works locally without manual intervention
- [ ] **No screenshots anywhere in the codebase**
