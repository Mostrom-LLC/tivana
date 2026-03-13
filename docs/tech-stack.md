# Tech Stack

- Growing browser automation ecosystem (chromiumoxide, headless_chrome)
## Browser Control: Raw CDP
- Direct WebSocket connection to browser
- No abstraction layer overhead
- Full control over exactly what we request
- Can optimize for our specific use case (streaming state)
- No dependency on Playwright/Puppeteer release cycles
## Rust Crates
- chromiumoxide — CDP client, browser launching, async
- tokio — async runtime
- tokio-tungstenite — WebSocket server
- serde — JSON serialization
- clap — CLI argument parsing
## Agent SDK: TypeScript
While the runtime is Rust, the agent SDK should still be TypeScript:

- Most AI agents are in Python or TypeScript
- SDK is just a thin WebSocket client — not performance critical
- Type definitions help agent developers
- Can also provide Python SDK later

---

# Architecture Clarification
The agent NEVER writes browser automation code. The runtime handles all browser complexity.

```plain text
Agent sends:     { action: "click", target: "e42" }
                          │
                          ▼
Runtime receives action, internally does:
  - Find element with id "e42" in state
  - Get its coordinates from bounds
  - Send CDP Input.dispatchMouseEvent
  - Wait for result
  - Send mutation events back to agent
                          │
                          ▼
Agent receives:  { type: "page.mutation", ... }
```
Agent just sends/receives JSON. All CDP, coordinates, input synthesis is hidden inside runtime.


---

# Tradeoffs Accepted
## More upfront work
- Raw CDP means implementing input synthesis ourselves
- Rust learning curve if team is primarily TypeScript
- Longer time to MVP
## Worth it because
- No Playwright/Puppeteer bugs or breaking changes affecting us
- Predictable streaming performance (no GC pauses)
- Single ~10MB binary vs 200MB+ node_modules
- Full control over behavior edge cases
- Can optimize specifically for streaming state (not general automation)

---

# Final Stack
- Runtime: Rust + Raw CDP + tokio + chromiumoxide
- Agent SDK: TypeScript (thin WebSocket client)
- Protocol: JSON over WebSocket
- Browser: Chromium (Chrome, Edge, Brave, Arc)
Finalized technology choices for Tivana.


---

# Runtime
## Rust
- Memory safety without garbage collection
- No GC pauses during streaming
- Single binary deployment
- Compile-time bug prevention
## Raw CDP (Chrome DevTools Protocol)
- Direct WebSocket to browser — no abstraction overhead
- Full control over state building
- No dependency on Playwright/Puppeteer releases
- Optimized specifically for streaming perception
## Rust Crates
- chromiumoxide — CDP client, browser launching
- tokio — async runtime
- tokio-tungstenite — WebSocket server
- serde — JSON serialization
- clap — CLI argument parsing

---

# Agent SDK
## TypeScript
- Thin WebSocket client — not performance critical
- Type definitions for PageState, Element, Actions
- Easy integration for TypeScript/JavaScript agents
- Python SDK can be added later

---

# Protocol
- JSON over WebSocket
- Bidirectional: runtime pushes state, agent sends actions

---

# Browser
- Chromium-based (Chrome, Edge, Brave, Arc)
- Launched by runtime with remote debugging
- Visible window — agent actions observable in real-time

---

# Architecture
The agent never writes automation code. It sends simple JSON actions and receives page state.

```plain text
Agent sends:     { action: "click", target: "e42" }
                          |
                          v
Runtime (Rust) handles:
  - Find element in state
  - Get coordinates from bounds
  - Send CDP Input.dispatchMouseEvent
  - Observe mutations
  - Stream updated state
                          |
                          v
Agent receives:  { type: "page.mutation", ... }
```
All CDP complexity is hidden inside the Rust runtime.

