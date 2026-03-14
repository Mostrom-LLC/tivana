# Tivana v1 Implementation Checklist

## 0. Freeze v1 scope
- [ ] Confirm v1 is **Chromium-only**
- [ ] Confirm v1 scope is only:
  - [ ] **Perceive**
  - [ ] **Act**
- [ ] Explicitly exclude for v1:
  - [ ] memory
  - [ ] planning
  - [ ] domain logic
  - [ ] non-Chromium browsers
  - [ ] npm publishing

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
- [ ] Define capability negotiation field
- [ ] Define standard error shape:
  - [ ] `code`
  - [ ] `message`
  - [ ] `data`

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
### Minimum v1 perception
- [ ] `perceive.pageState`
- [ ] Return:
  - [ ] current URL
  - [ ] page title
  - [ ] page HTML or normalized DOM snapshot
  - [ ] focused element if available
- [ ] `perceive.screenshot`
- [ ] `perceive.accessibilityTree` or equivalent structured element view
- [ ] `perceive.mutations` event stream or periodic state refresh event

### Output discipline
- [ ] Keep page-state payload normalized
- [ ] Keep element representation stable enough for follow-up actions
- [ ] Document what “mutation” means in v1

---

## 7. Act primitives
### Minimum v1 actions
- [ ] `act.navigate`
- [ ] `act.click`
- [ ] `act.type`
- [ ] `act.scroll`

### Action requirements
- [ ] Accept target descriptor for actions
- [ ] Return success/failure in structured form
- [ ] Verify action effect where possible
- [ ] Fail clearly when target is missing/ambiguous

---

## 8. Element targeting model
- [ ] Define how elements are identified in v1
- [ ] Support at least one stable targeting approach:
  - [ ] accessibility/text-based selector
  - [ ] DOM path / node id
  - [ ] bounding box fallback if needed
- [ ] Document target ambiguity behavior
- [ ] Document stale target behavior after DOM changes

---

## 9. Error handling + recovery
- [ ] Separate error classes:
  - [ ] protocol validation error
  - [ ] session error
  - [ ] browser error
  - [ ] action failure
  - [ ] perception failure
- [ ] Add disconnect handling
- [ ] Add browser crash handling
- [ ] Add stale page/session handling
- [ ] Ensure runtime never silently hangs on failed browser action

---

## 10. CLI
- [ ] Implement `tivana` CLI with `clap`
- [ ] Support:
  - [ ] start server
  - [ ] port selection
  - [ ] headless/headed mode
  - [ ] chromium executable override if needed
- [ ] Add helpful startup logs
- [ ] Add graceful shutdown behavior

---

## 11. TypeScript SDK v1
- [ ] Connect to runtime over WebSocket
- [ ] Bun WebSocket primary path
- [ ] Node `ws` fallback path
- [ ] Implement:
  - [ ] `connect()`
  - [ ] `handshake()`
  - [ ] `createSession()`
  - [ ] `navigate()`
  - [ ] `pageState()`
  - [ ] `click()`
  - [ ] `type()`
  - [ ] `scroll()`
  - [ ] `closeSession()`
- [ ] Add typed request/response interfaces
- [ ] Add event subscription support
- [ ] Add example script using the SDK end-to-end

---

## 12. Local verification
### Runtime verification
- [ ] Start runtime locally
- [ ] Connect with TS SDK
- [ ] Create session
- [ ] Launch Chromium
- [ ] Navigate to a page
- [ ] Read page state
- [ ] Click an element
- [ ] Type into an input
- [ ] Scroll
- [ ] Close session
- [ ] Shut down runtime cleanly

### Reliability verification
- [ ] Verify bad request returns structured error
- [ ] Verify browser launch failure returns structured error
- [ ] Verify session close actually tears down resources
- [ ] Verify client disconnect does not crash runtime

---

## 13. Test suite
### Rust
- [ ] protocol serialization/deserialization tests
- [ ] session lifecycle tests
- [ ] action routing tests
- [ ] error mapping tests

### SDK
- [ ] client connect/disconnect tests
- [ ] request correlation tests
- [ ] fallback transport tests

### End-to-end
- [ ] one smoke test:
  - [ ] connect
  - [ ] create session
  - [ ] launch Chromium
  - [ ] navigate
  - [ ] perceive
  - [ ] act
  - [ ] close

---

## 14. Docs to finish before calling v1 usable
- [ ] protocol envelope reference
- [ ] supported v1 methods
- [ ] session lifecycle doc
- [ ] Chromium-only scope doc
- [ ] local run instructions
- [ ] TS SDK usage example
- [ ] known limitations

---

# Recommended build order
If you want the shortest critical path:

## Phase 1
- [ ] Rust runtime skeleton
- [ ] WebSocket server
- [ ] protocol envelope
- [ ] session registry

## Phase 2
- [ ] Chromium launch/navigate/close
- [ ] `perceive.pageState`
- [ ] `act.click`
- [ ] `act.type`
- [ ] `act.scroll`

## Phase 3
- [ ] TypeScript SDK
- [ ] local smoke test
- [ ] basic docs
- [ ] hardening/errors

## Definition of done for v1
Tivana v1 is “done” when:
- [ ] a TS client can connect to the Rust runtime
- [ ] create a session
- [ ] launch Chromium
- [ ] navigate to a page
- [ ] perceive page state
- [ ] click/type/scroll successfully
- [ ] receive structured responses/errors
- [ ] close everything cleanly
- [ ] all of that works locally without manual intervention
