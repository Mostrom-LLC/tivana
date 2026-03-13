# Architecture

How the runtime connects browsers to agents.


---

# System Overview
```plain text
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Browser    │────►│   Runtime    │────►│    Agent     │
│  (Chromium)  │◄────│  (tivana)    │◄────│   (LLM/AI)   │
└──────────────┘     └──────────────┘     └──────────────┘
       │                    │                    │
       │                    │                    │
   Renders page        Streams state        Reasons about
   Executes actions    Routes actions       what to do
```
# Components
## 1. Browser
- Chromium-based (Chrome, Edge, Brave, Arc)
- Launched by runtime with remote debugging enabled
- Visible window — agent actions are observable in real-time
## 2. Runtime (tivana)
- Launches and manages browser process
- Connects to browser via CDP (Chrome DevTools Protocol)
- Reads accessibility tree + computed styles
- Streams PageState to agents via WebSocket
- Executes agent actions (click, type, navigate)
## 3. Agent
- Any AI system that can receive JSON and output actions
- Receives streaming PageState updates
- Reasons about what to do based on goal + page state
- Sends actions back to runtime

---

# Data Flow
```plain text
Browser                    Runtime                    Agent
   │                          │                          │
   │──── AXTree ─────────────►│                          │
   │──── ComputedStyles ─────►│                          │
   │──── LayoutRects ────────►│                          │
   │                          │                          │
   │                          │──── PageState ──────────►│
   │                          │                          │
   │                          │                          │ (reasons)
   │                          │                          │
   │                          │◄──── action.click ───────│
   │                          │                          │
   │◄──── dispatchEvent ──────│                          │
   │                          │                          │
   │──── MutationEvent ──────►│                          │
   │                          │──── page.mutation ──────►│
   │                          │                          │
```

---

# Why No Extension?
Extensions require separate implementations for Chrome, Firefox, Safari. By using CDP and launching a managed browser, we get:

- Single implementation — Works with any Chromium browser
- No install friction — No extension store, no permissions dialogs
- Full access — CDP provides everything we need (AXTree, styles, input)
- Isolated environment — Agent gets its own browser, no interference
