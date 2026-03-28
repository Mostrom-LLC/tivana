# Tivana Atlas — Design Spec

**Date:** 2026-03-28
**Status:** Approved

---

## Overview

Tivana Atlas is an Electron-based AI-controlled browser. A user gives the agent a goal (e.g., "Apply to remote DevOps jobs on Indeed paying $140K+"), and the agent autonomously navigates, fills forms, clicks buttons, and completes the task while the user watches. The user brings their own Gemini API key. There are no permission gates or confirmation prompts — the only safety mechanism is a kill switch.

### Core Principles

1. **Fully autonomous** — the agent runs until it calls `done` or the user kills it
2. **No permission gates** — no "are you sure?" prompts, no standing permissions system, no action confirmations
3. **Kill switch as only safety** — user can instantly halt the agent and optionally resume
4. **BYOK** — user provides their own Gemini API key, stored securely in the OS keychain
5. **Transparency** — user watches the browser in real time and reads the agent's reasoning in the sidebar

---

## Architecture

Three TypeScript layers running in Electron's main process. No Rust runtime — Electron's `webContents.debugger` provides direct CDP access.

```
┌──────────────────────────────────────────────────┐
│  Electron Main Process                           │
│                                                  │
│  ┌────────────┐  ┌──────────┐  ┌─────────────┐  │
│  │ CDP Layer  │  │ Percept. │  │ Agent Loop  │  │
│  │            │←→│ + Action │←→│ (Gemini)    │  │
│  │ debugger   │  │ Engine   │  │             │  │
│  │ attach/cmd │  │          │  │ perceive →  │  │
│  └─────┬──────┘  └──────────┘  │ reason →    │  │
│        │                       │ act →       │  │
│        │ CDP                   │ repeat      │  │
│        ▼                       └─────────────┘  │
│  ┌──────────────┐                                │
│  │ WebContents  │  ← user sees this              │
│  │ View (tabs)  │                                │
│  └──────────────┘                                │
│                                                  │
│  ┌──────────────────────────────────────────┐    │
│  │ Renderer Process (React)                 │    │
│  │ Sidebar chat, tab bar, URL bar, status   │    │
│  └──────────────────────────────────────────┘    │
└──────────────────────────────────────────────────┘
```

### Layer 1: CDP Layer (`cdp.ts`)

Thin typed wrapper around `webContents.debugger`:

- `attach(webContents)` / `detach(webContents)` — manage debugger sessions per tab
- `send(webContents, method, params)` — send CDP commands, return typed responses
- Event subscription for CDP events (Page, DOM, Network, Runtime domains)
- Error handling: listen for `detach` event on debugger. If a tab's debugger detaches (crash, navigation to chrome:// URL), mark that tab as disconnected, notify the agent loop, and show an error in the sidebar. Agent loop skips actions on disconnected tabs and re-perceives.

### Layer 2: Perception + Action Engine

**Perception (`perception.ts`):**

Extracts structured page state by injecting JavaScript into the page via `Runtime.evaluate`. The extraction scripts are ported directly from Tivana's existing `perceive.rs` (which embeds JS strings for element extraction).

Returns:
- `PageState` — URL, title, viewport dimensions, scroll position
- `Element[]` — all interactive elements with: IDs, roles, labels, values, bounding boxes, visibility, interactability

**Element IDs:** String IDs in "eN" format (e.g., "e1", "e2", "e3") assigned by a `WeakMap` + counter stored on `window.__tivana_element_map`, matching the Tivana runtime's existing scheme. IDs are **stable within a session** — the same DOM element keeps its ID across perceive calls via the WeakMap. If a CDP action fails with "element not found," the agent re-perceives to get updated IDs.

**Actions (`actions.ts`):**

Each action sends CDP commands to manipulate the page:

- `navigate(url)` — `Page.navigate`
- `click(elementId)` — resolve element coordinates, `Input.dispatchMouseEvent`
- `type(elementId, text)` — focus element, `Input.dispatchKeyEvent` per character
- `fill(elementId, value)` — focus + set value via JS injection + dispatch input/change events
- `scroll(direction, amount)` — `Input.dispatchMouseEvent` (wheel) or JS scroll
- `select(elementId, value)` — set select value via JS + dispatch change event
- `screenshot()` — `Page.captureScreenshot`, return base64
- `hover(elementId)` — resolve coordinates, `Input.dispatchMouseEvent` (mouseMoved)
- `wait(seconds)` — simple `setTimeout` delay, used by the agent to wait for async page updates (SPAs, loading spinners)

**Tab tools** (`new_tab`, `switch_tab`, `close_tab`) are implemented in `tabs.ts` but dispatched by the same tool executor in `agent.ts`. The agent loop routes tab tools to `tabs.ts` and browser action tools to `actions.ts`.

### Layer 3: Agent Loop (`agent.ts`)

Stateless loop:

```
1. Perceive: get PageState + Element[] from current tab
2. Build prompt: system prompt + page state + elements + conversation history
3. Call Gemini: send prompt with tool definitions, get back tool calls
4. Validate: check tool call parameters (element ID exists in current snapshot, required params present)
   - If invalid: feed error message back to Gemini as a tool result ("Element ID 42 not found. Available IDs: 1-15. Re-perceiving page.")
5. Execute: run each valid tool call via the action engine
6. Collect results: tool call outcomes become the next assistant message
7. Check kill flag: if user hit kill switch, stop
8. Repeat from 1
```

The loop ends when Gemini calls `done(summary)` or the user kills it.

**Error handling in the loop:**
- **Gemini API auth error (401/403):** Stop the loop, show "Invalid API key" in sidebar with a link to settings. Do not retry.
- **Gemini rate limit (429):** Exponential backoff — wait 1s, 2s, 4s, max 30s. Show "Rate limited, retrying..." in sidebar.
- **Network failure:** Retry up to 3 times with 2s delay. If still failing, stop loop and show error.
- **Malformed tool calls:** Feed the error back to Gemini as a tool result so it can self-correct. If 3 consecutive malformed calls, stop loop.
- **CDP action failure:** Feed the error back to Gemini (e.g., "Click failed: element not interactable"). Agent re-perceives and tries a different approach.

---

## Gemini Integration (`gemini.ts`)

**SDK:** `@google/generative-ai`

**Model:** Gemini 2.5 Pro (user-configurable)

**Tool definitions provided to Gemini:**

| Tool | Parameters | Description |
|------|-----------|-------------|
| `navigate` | `url: string` | Go to a URL |
| `click` | `id: string` | Click element by ID (e.g., "e1") |
| `type` | `id: string, text: string` | Type text into element |
| `fill` | `id: string, value: string` | Set field value instantly |
| `scroll` | `direction: "up"\|"down", amount?: number` | Scroll the page |
| `select` | `id: string, value: string` | Select dropdown option |
| `hover` | `id: string` | Hover over element |
| `screenshot` | — | Capture page screenshot (displayed in sidebar only, not sent to model — vision is out of scope for MVP) |
| `wait` | `seconds: number` | Wait for async page updates |
| `new_tab` | `url?: string` | Open a new tab |
| `switch_tab` | `index: number` | Switch to tab by index |
| `close_tab` | `index: number` | Close a tab |
| `done` | `summary: string` | Task complete |
| `ask_user` | `question: string` | Ask user for input |

`ask_user` is the only tool that pauses the loop — and only when the LLM itself decides it needs information, not as a permission gate.

**System prompt:**

```
You are an autonomous browser agent. You execute tasks without asking for
confirmation. Use the provided tools to interact with the page. When you
receive a task, do it. Do not ask "are you sure?" — the user already decided.

You can see the page as a list of interactive elements with IDs, roles, labels,
and values. Use the element IDs to target actions. If an element isn't in the
list, it may not be visible — try scrolling.

When the task is complete, call done() with a summary of what you accomplished.
Only use ask_user() when you genuinely need information you cannot find on the
page (e.g., a password, a preference not stated in the task).
```

**API key storage:** Electron `safeStorage` API (OS keychain-backed encryption). Key is encrypted at rest, decrypted in-memory only when making API calls.

**Cost tracking:** Extract `usageMetadata` (prompt/completion token counts) from each Gemini response. Running totals displayed in the status bar.

**Streaming vs request/response:** MVP uses **request/response** (non-streaming) for Gemini calls. The agent loop calls `generateContent()` and waits for the complete response including tool calls. The sidebar displays the agent's text response once it arrives (not token-by-token). This simplifies tool call parsing and kill switch behavior. Streaming can be added later for better UX.

**Cost tracking formula:** Token counts come from `response.usageMetadata.promptTokenCount` and `candidatesTokenCount`. Cost is calculated using hardcoded per-model rates (e.g., Gemini 2.5 Pro: $1.25/1M input, $10/1M output for >200k context). Rates are stored as a simple lookup table in `gemini.ts` and updated manually with new releases. Display format: `$0.02` in the status bar.

**Provider interface for future expansion:**

```typescript
interface LLMProvider {
  chat(messages: Message[], tools: ToolDefinition[]): Promise<LLMResponse>;
  name: string;
  model: string;
}

interface LLMResponse {
  text: string;
  toolCalls: ToolCall[];
  usage: { inputTokens: number; outputTokens: number };
}
```

MVP only implements `GeminiProvider`. Adding OpenAI, Anthropic, Ollama later is just a new class implementing this interface.

---

## UI / Layout

Atlas-style layout. Browser on the left, chat sidebar on the right.

```
┌──────────────────────────────────────────────────────────────┐
│  [Tab 1] [Tab 2] [+]                             [─][□][×]  │
├──────────────────────────────────────────────────────────────┤
│  [← →]  [ https://indeed.com/jobs                ]  [⟳]    │
├─────────────────────────────────┬────────────────────────────┤
│                                 │  Chat Sidebar              │
│                                 │                            │
│  Browser Pane                   │  Agent: Navigating to      │
│  (WebContentsView)              │  Indeed job search...      │
│                                 │                            │
│                                 │  Agent: Found 12 matching  │
│                                 │  jobs. Opening first...    │
│                                 │                            │
│                                 │  Agent: Filling out        │
│                                 │  application form...       │
│                                 │                            │
│                                 │  ┌──────────────────────┐  │
│                                 │  │   ■ Kill Switch      │  │
│                                 │  └──────────────────────┘  │
│                                 │                            │
│                                 │  [ Type a task...     ⏎ ]  │
├─────────────────────────────────┴────────────────────────────┤
│  Tokens: 12.4k  |  Cost: $0.02  |  Gemini 2.5 Pro  |  ●    │
└──────────────────────────────────────────────────────────────┘
```

**Components:**

- **Tab Bar** — horizontal tabs, each backed by its own `WebContentsView`. New tab button. Close button per tab.
- **URL Bar** — shows current tab URL. Back/forward/refresh for manual navigation. Editable for direct URL entry.
- **Browser Pane** — the `WebContentsView` for the active tab. Fully interactive when agent is stopped.
- **Chat Sidebar** — streaming agent messages showing what it's doing and why. Scrollable history. Collapsible.
- **Kill Switch** — always visible red button. `Cmd+Shift+K` / `Ctrl+Shift+K` keyboard shortcut. Instantly halts the agent loop. Becomes a "Resume" button after activation.
- **Task Input** — text field at the bottom of the sidebar. Type a goal, press Enter, agent starts.
- **Status Bar** — token usage, cost, current model, connection indicator.

**Styling:** Tailwind CSS. Dark theme matching Atlas aesthetic.

---

## `ask_user` Flow

When Gemini calls `ask_user(question)`:

1. Agent loop pauses (same as kill switch, but triggered by the model)
2. The question appears in the sidebar chat as a message from the agent
3. The task input field gets focus with a placeholder: "Agent is waiting for your answer..."
4. User types their response and presses Enter
5. The response is added to conversation history as a user message
6. Agent loop resumes automatically — next cycle sends the answer to Gemini along with the current page state

This is the only model-initiated pause. It is NOT a permission gate — it's for genuinely missing information (passwords, preferences, ambiguous choices).

---

## IPC Contract (`ipc.ts`)

Communication between main process and renderer via Electron's `ipcMain`/`ipcRenderer`.

**Main → Renderer (events pushed to UI):**

| Channel | Payload | Purpose |
|---------|---------|---------|
| `agent:message` | `{ text: string, type: "thinking" \| "action" \| "result" \| "error" }` | Agent chat messages for sidebar |
| `agent:status` | `{ running: boolean, waiting: boolean }` | Agent state changes (running, paused, waiting for user) |
| `agent:usage` | `{ inputTokens: number, outputTokens: number, cost: number }` | Updated token/cost totals |
| `tabs:update` | `Tab[]` | Full tab list (id, title, url, active) |
| `navigation:url` | `{ url: string }` | Active tab URL changed |

**Renderer → Main (user actions):**

| Channel | Payload | Purpose |
|---------|---------|---------|
| `task:start` | `{ goal: string }` | User submitted a new task |
| `task:kill` | — | Kill switch pressed |
| `task:resume` | — | Resume button pressed |
| `user:response` | `{ text: string }` | User answered an `ask_user` question |
| `tab:switch` | `{ id: string }` | User clicked a tab |
| `tab:new` | — | User clicked new tab button |
| `tab:close` | `{ id: string }` | User clicked close on a tab |
| `navigation:go` | `{ url: string }` | User entered a URL in the URL bar |
| `navigation:back` | — | Back button |
| `navigation:forward` | — | Forward button |
| `navigation:refresh` | — | Refresh button |
| `settings:open` | — | Open settings |
| `settings:save` | `{ apiKey?: string, model?: string }` | Save settings |

---

## Tab Management (`tabs.ts`)

- Each tab is a `WebContentsView` with its own CDP debugger session
- Tab state: `{ id, webContents, title, url, favicon }`
- Agent can operate across tabs via `new_tab`, `switch_tab`, `close_tab` tools
- When user manually switches tabs, the agent's next perceive cycle sees the new active tab
- Tab bar UI syncs with tab state via IPC

---

## Kill Switch Behavior

1. User clicks kill switch or presses `Cmd+Shift+K`
2. `running` flag set to `false` in agent loop
3. Current in-flight CDP command completes (no mid-action abort)
4. Agent loop stops — no further perceive/reason/act cycles
5. Chat sidebar shows "Agent paused" with a "Resume" button
6. Browser pane becomes fully interactive for manual use
7. User can resume — agent continues from current page state, not from where it stopped (re-perceives)
8. User can also type a new task to start fresh

---

## Settings

Minimal settings page (accessible from sidebar gear icon):

- **API Key** — Gemini API key input, stored via `safeStorage`
- **Model** — dropdown to select Gemini model variant
- **Sidebar width** — resizable by dragging the divider

No other settings for MVP.

---

## Tech Stack

| Concern | Choice |
|---------|--------|
| App framework | Electron (electron-forge) |
| UI framework | React |
| Styling | Tailwind CSS |
| Language | TypeScript |
| LLM SDK | `@google/generative-ai` |
| Packaging | electron-forge (macOS .dmg) |
| Key storage | Electron `safeStorage` |

---

## Project Structure

```
browser/
├── package.json
├── forge.config.ts
├── tsconfig.json
├── tailwind.config.ts
├── src/
│   ├── main/
│   │   ├── index.ts             # App entry, window creation
│   │   ├── cdp.ts               # CDP wrapper (webContents.debugger)
│   │   ├── perception.ts        # Element extraction via JS injection
│   │   ├── actions.ts           # Click, type, fill, scroll, navigate
│   │   ├── agent.ts             # Agent loop (perceive → reason → act)
│   │   ├── gemini.ts            # Gemini API client + tool definitions
│   │   ├── tabs.ts              # Tab lifecycle management
│   │   └── ipc.ts               # IPC handlers for renderer ↔ main
│   └── renderer/
│       ├── index.html
│       ├── App.tsx
│       ├── components/
│       │   ├── Sidebar.tsx      # Chat messages + kill switch + task input
│       │   ├── TabBar.tsx       # Tab strip
│       │   ├── UrlBar.tsx       # URL bar + nav buttons
│       │   └── StatusBar.tsx    # Tokens, cost, model
│       └── styles/
│           └── globals.css      # Tailwind imports
├── resources/
│   └── icon.png                 # App icon
└── README.md
```

---

## Out of Scope (MVP)

- Multi-provider support (OpenAI, Anthropic, Ollama)
- Browser extensions, bookmarks, history panel
- Profile management
- Screenshot-based vision fallback
- Auto-updater
- Windows/Linux packaging
- Task marketplace/sharing
- Memory/persistence across sessions
- Stealth/anti-detection (future: reference Browser Use MIT for ideas)
- CAPTCHA solving (future: reference Browser Use MIT for ideas)

---

## Future Phases

**Phase 2 — Multi-Provider:**
- Implement `LLMProvider` for OpenAI, Anthropic, Ollama, OpenRouter
- Provider selection in settings
- Model picker per provider

**Phase 3 — Stealth & CAPTCHA:**
- Reference Browser Use (MIT) for anti-detection techniques
- CAPTCHA solving integration
- Proxy support

**Phase 4 — Polish:**
- Element highlighting (show what the AI is looking at)
- Mouse cursor animation
- Screenshot fallback when agent is confused
- Cross-platform packaging (Windows, Linux)
- Auto-updater
