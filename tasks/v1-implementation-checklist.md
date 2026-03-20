# Tivana v1 Implementation Checklist

## 0. Freeze v1 scope
- [x] Confirm v1 is **Chromium-only**
- [x] Confirm v1 scope is only:
  - [x] **Perceive** (streaming page state)
  - [x] **Act** (click, type, scroll, navigate)
- [x] Explicitly exclude for v1:
  - [x] memory
  - [x] planning
  - [x] domain logic
  - [x] non-Chromium browsers
  - [x] npm publishing
  - [x] **screenshots** (not even as fallback — streaming perception replaces them entirely)

---

## 1. Repo / workspace setup
### Rust runtime
- [x] Create Rust crate/app entrypoint
- [x] Add dependencies:
  - [x] `chromiumoxide`
  - [x] `tokio`
  - [x] `tokio-tungstenite`
  - [x] `serde`
  - [x] `serde_json`
  - [x] `clap`
  - [x] `tracing`
- [x] Add runtime module structure:
  - [x] `src/main.rs`
  - [x] `src/cli.rs`
  - [x] `src/server.rs`
  - [x] `src/session.rs`
  - [x] `src/protocol.rs`
  - [x] `src/browser.rs`
  - [x] `src/perceive.rs`
  - [x] `src/act.rs`
  - [x] `src/error.rs`

### TypeScript SDK
- [x] Create `sdk/ts` package
- [x] Use Bun as default runtime/tooling
- [x] Add Node fallback via `ws`
- [x] Add:
  - [x] `package.json`
  - [x] `tsconfig.json`
  - [x] `src/client.ts`
  - [x] `src/types.ts`
  - [x] `src/index.ts`

---

## 2. Protocol envelope
- [x] Define JSON message envelope
- [x] Every message has:
  - [x] `id`
  - [x] `type`
  - [x] `method`
  - [x] `sessionId` when applicable
  - [x] `params`
  - [x] `result`
  - [x] `error`
- [x] Define message types:
  - [x] `request`
  - [x] `response`
  - [x] `event`
- [x] Define protocol version field
- [x] Define standard error shape:
  - [x] `code`
  - [x] `message`
  - [x] `data`

> **Note:** Capability negotiation deferred to v2. Keep v1 simple.

---

## 3. Session model
- [x] Implement session registry
- [x] Implement session lifecycle states:
  - [x] `created`
  - [x] `launching`
  - [x] `active`
  - [x] `closed`
- [x] Define session ownership rules
- [x] Route all browser actions through `sessionId`
- [x] Ensure session close cleans up browser/page handles

---

## 4. WebSocket server
- [x] Start local WebSocket server
- [x] Accept one or more clients
- [x] Parse JSON frames
- [x] Validate protocol envelope
- [x] Route commands to session/browser handlers
- [x] Return correlated responses
- [x] Emit structured errors for invalid requests
- [x] Add connection logging with `tracing`

---

## 5. Chromium runtime integration
- [x] Launch Chromium via `chromiumoxide`
- [x] Open a page/tab
- [x] Navigate to URL
- [x] Track active page handle inside session
- [x] Close browser cleanly on session close
- [x] Close browser cleanly on runtime shutdown
- [x] Handle browser launch failures gracefully

---

## 6. Perceive primitives

> **⚠️ NO SCREENSHOTS.** Not as primary, not as fallback. Streaming semantic perception is the entire point. If we hit a wall that "needs" screenshots, we solve it with better perception — not by falling back to pixels.

### Minimum v1 perception
- [x] `perceive.pageState`
  - [x] current URL
  - [x] page title
  - [x] focused element ID
  - [x] scroll position
  - [x] viewport dimensions
  - [x] timestamp
- [x] `perceive.elements` — returns element tree with:
  - [x] AXTree data (role, label, value, focused, enabled)
  - [x] Computed styles (font, colors, borders, padding, margin)
  - [x] Geometry (bounds via getBoundingClientRect)
- [x] `perceive.mutations` — **event stream** (push via WebSocket + polling fallback)
  - [x] element added
  - [x] element removed
  - [x] element changed (with changed properties)
  - [x] focus changed
  - [x] navigation occurred

### Output discipline
- [x] Keep page-state payload normalized and compact
- [x] Element IDs must be stable enough for follow-up actions
- [x] Document mutation event semantics

---

## 7. Act primitives
### Minimum v1 actions
- [x] `act.navigate` — go to URL
- [x] `act.click` — click element by ID or selector
- [x] `act.type` — type text into focused element or target
- [x] `act.scroll` — scroll element into view

### Action requirements
- [x] Accept element ID as primary target
- [x] Accept role+label selector as fallback target
- [x] Return success/failure in structured form
- [x] Return new page state after action completes
- [x] Fail clearly when target is missing/ambiguous

---

## 8. Element targeting model
- [x] Primary: element ID from AXTree (e.g., `e1`, `e2`)
- [x] Secondary: role + label selector (e.g., `{ role: "button", label: "Submit" }`)
- [x] Define ID stability rules:
  - [x] IDs are stable within a page session
  - [x] IDs may change after navigation or major DOM mutation
- [x] Define stale target behavior:
  - [x] If element ID no longer exists, return `target_not_found` error
  - [x] Agent must re-perceive to get fresh IDs
- [x] Define ambiguous target behavior:
  - [x] If multiple elements match selector, return `target_ambiguous` error with count

---

## 9. Error handling + recovery
- [x] Separate error classes:
  - [x] `protocol_error` — malformed message, missing fields
  - [x] `session_error` — invalid session, session closed
  - [x] `browser_error` — launch failed, crashed, disconnected
  - [x] `action_error` — target not found, action failed
  - [x] `perception_error` — failed to read page state
- [x] Add disconnect handling (client disconnects mid-action)
- [x] Add browser crash handling (detect crash, return structured error)
- [x] Add stale page/session handling
- [x] Ensure runtime never silently hangs on failed browser action

---

## 10. CLI
- [x] Implement `tivana` CLI with `clap`
- [x] Support:
  - [x] `tivana` — start server
  - [x] `--port <port>` — port selection (default: 9876)
  - [x] `--headless` / `--headed` — browser visibility
  - [x] `--chrome-path <path>` — chromium executable override
- [x] Add helpful startup logs (port, mode, version)
- [x] Add graceful shutdown on SIGINT/SIGTERM

---

## 11. TypeScript SDK v1
- [x] Connect to runtime over WebSocket
- [x] Bun WebSocket primary path
- [x] Node `ws` fallback path
- [x] Implement:
  - [x] `connect(url)` — connect to runtime
  - [x] `createSession()` — create browser session
  - [x] `navigate(url)` — navigate to URL
  - [x] `pageState()` — get current page state
  - [x] `elements()` — get element tree
  - [x] `click(target)` — click element
  - [x] `type(text, target?)` — type text
  - [x] `scroll(target)` — scroll to element
  - [x] `onMutation(callback)` — subscribe to mutations
  - [x] `closeSession()` — close session
  - [x] `disconnect()` — disconnect from runtime
- [x] Add typed request/response interfaces
- [x] Add example script using the SDK end-to-end

---

## 12. Local verification
### Runtime verification
- [x] Start runtime locally (`tivana --headless`)
- [x] Connect with TS SDK
- [x] Create session
- [x] Chromium launches (headless verified)
- [x] Navigate to a page (example.com)
- [x] Read page state (URL, title, viewport, timestamp)
- [x] Read element tree with bounds
- [x] Click an element (navigated via "Learn more" link)
- [x] Type into an input (verified with success=true)
- [x] Scroll to element (100px smooth scroll)
- [x] Mutation event push mechanism implemented
- [x] Close session
- [x] Chromium closes
- [x] Shut down runtime cleanly

### Reliability verification
- [x] Bad request returns structured error
- [x] Browser launch failure returns structured error
- [x] Session close tears down resources
- [x] Client disconnect does not crash runtime
- [x] Stale element ID returns clear error

---

## 13. Test suite
### Rust
- [x] Protocol serialization/deserialization tests
- [x] Session lifecycle tests
- [x] Action routing tests
- [x] Error mapping tests

### SDK
- [x] Client connect/disconnect tests
- [x] Request correlation tests
- [x] Bun WebSocket tests (primary path)
- [x] Node `ws` fallback tests (connect failure path tested)

### End-to-end
- [x] Smoke test (created sdk/ts/smoke-test.ts):
  - [x] connect
  - [x] create session
  - [x] launch Chromium
  - [x] navigate
  - [x] perceive page state
  - [x] perceive elements
  - [x] click
  - [x] type
  - [x] scroll
  - [x] close session
  - [x] disconnect

---

## 14. Docs to finish before calling v1 usable
- [x] Protocol envelope reference (message shapes)
- [x] Supported v1 methods (perceive.*, act.*)
- [x] Session lifecycle doc
- [x] Element model doc (AXTree + styles + geometry)
- [x] Element targeting doc (IDs, selectors, staleness)
- [x] Chromium-only scope doc
- [x] Local run instructions
- [x] TS SDK usage example
- [x] Known limitations

---

# Recommended build order

## Phase 1 — Foundation ✅
- [x] Rust runtime skeleton (`cargo new`)
- [x] Dependencies in `Cargo.toml`
- [x] Module structure
- [x] Protocol envelope types
- [x] WebSocket server (accept connections, parse JSON)
- [x] Session registry (create, get, close)

## Phase 2 — Browser + Perceive + Act ✅
- [x] Chromium launch/navigate/close via chromiumoxide
- [x] `perceive.pageState`
- [x] `perceive.elements` (AXTree + styles + geometry)
- [x] `perceive.mutations` event stream (push via WebSocket + polling)
- [x] `act.navigate`
- [x] `act.click`
- [x] `act.type`
- [x] `act.scroll`

## Phase 3 — SDK + Polish ✅
- [x] TypeScript SDK (Bun primary, Node fallback)
- [x] Local smoke test script
- [x] Error handling hardening
- [x] Basic docs
- [x] README update with usage

---

# Definition of done for v1

Tivana v1 is "done" when:

- [x] A TS client can connect to the Rust runtime
- [x] Create a session (Chromium launches)
- [x] Navigate to any page
- [x] Perceive page state (URL, title, scroll, viewport)
- [x] Perceive element tree (roles, labels, styles, geometry)
- [x] Receive mutation events when DOM changes (push + polling)
- [x] Click an element by ID
- [x] Type text into an input
- [x] Scroll to an element
- [x] Receive structured errors for failures
- [x] Close session (Chromium closes)
- [x] Disconnect cleanly
- [x] All of that works locally without manual intervention (smoke test: 20/20 pass)
- [x] **No screenshots anywhere in the codebase**

---

# Remaining Gaps (all resolved)

All v1 blockers have been resolved:

1. ✅ **Mutation Event Stream** — Push via WebSocket + polling fallback implemented
2. ✅ **Browser Crash Recovery** — Crash detection returns structured error
3. ✅ **End-to-End Verification** — Smoke test: 20/20 pass (100%)
4. ✅ **SDK Unit Tests** — 13 tests covering client, types, and edge cases
5. ✅ **SDK Build Chain** — CJS + ESM + declarations output correctly
6. ✅ **SDK↔Runtime Method Mismatches** — All method names aligned
7. ✅ **Element ID Stability** — WeakMap-based persistent IDs per page session
8. ✅ **CDP Input** — click/type/press use CDP Input domain (not JS events)
9. ✅ **npm Publish Prep** — LICENSE, prepublishOnly, files field
10. ✅ **Headless Default** — Session inherits server's headless setting
11. ✅ **Cargo.lock** — Committed for reproducible builds
