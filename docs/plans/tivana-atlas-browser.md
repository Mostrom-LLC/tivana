# Tivana Atlas — AI-Controlled Browser with BYOK

**Date:** 2026-03-28
**Author:** Jarvis (AI) + Kaise White
**Status:** Brainstorm / Draft

---

## Vision

An AI-controlled browser application — like ChatGPT Operator/Atlas — where users bring their own API keys (Gemini, Anthropic, OpenAI, or any OpenAI-compatible endpoint). The AI agent sees the page, reasons about what to do, and acts autonomously while the user watches and retains full control.

**What ChatGPT Operator has:**
- Embedded Chromium browser the AI controls
- Right sidebar showing agent reasoning/status
- "Take control" / "Stop" buttons for human override
- "Standing permissions" (user pre-authorizes actions)
- Agent uses the user's logged-in sessions
- Bottom bar showing active tasks and login status

**What we add:**
- **BYOK** — user plugs in their own Gemini, Anthropic, or OpenAI key
- **No vendor lock-in** — works with any LLM that speaks OpenAI-compatible API
- **Open source / self-hostable** — not a cloud service
- **Tivana perception protocol** — our secret weapon for structured page understanding
- **Task marketplace / sharing** — users share automation recipes

---

## Architecture Options

### Option A: Electron App (Recommended)

```
┌─────────────────────────────────────────────────────┐
│  Electron Shell (macOS / Windows / Linux)            │
│                                                      │
│  ┌──────────────────────┐  ┌──────────────────────┐ │
│  │   BrowserView /      │  │   Agent Sidebar      │ │
│  │   WebContentsView    │  │   (React)             │ │
│  │                      │  │                       │ │
│  │   Real Chromium      │  │   • Agent messages    │ │
│  │   with CDP access    │  │   • Reasoning trace   │ │
│  │   via debugger API   │  │   • Task status       │ │
│  │                      │  │   • Standing perms    │ │
│  │   User sees the      │  │   • Key config        │ │
│  │   page and watches   │  │   • Take Control btn  │ │
│  │   the AI work        │  │   • Stop btn          │ │
│  │                      │  │                       │ │
│  └──────────────────────┘  └──────────────────────┘ │
│                                                      │
│  ┌──────────────────────────────────────────────────┐│
│  │  Bottom Bar: Active tasks, login status, URL     ││
│  └──────────────────────────────────────────────────┘│
│                                                      │
│  ┌──────────────────────────────────────────────────┐│
│  │  Tivana Core (Rust, compiled into Electron)      ││
│  │  • CDP ↔ BrowserView bridge                      ││
│  │  • Perception engine (element extraction)         ││
│  │  • Action engine (click, type, scroll, fill)     ││
│  │  • Screenshot → LLM pipeline                     ││
│  │  • Stealth layer                                 ││
│  └──────────────────────────────────────────────────┘│
│                                                      │
│  ┌──────────────────────────────────────────────────┐│
│  │  Agent Loop (TypeScript)                         ││
│  │  • LLM provider abstraction (OpenAI / Anthropic  ││
│  │    / Gemini / Ollama / any OpenAI-compatible)    ││
│  │  • Task planner + executor                       ││
│  │  • Standing permissions system                   ││
│  │  • Memory (conversation + page history)          ││
│  │  • Tool definitions for the LLM                  ││
│  └──────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────┘
```

**Why Electron:**
- Real Chromium = real web compatibility, no Cloudflare issues
- `webContents.debugger` API = full CDP access, no extension needed
- Cross-platform (macOS, Windows, Linux)
- Can embed Tivana Rust core via native Node addon (napi-rs)
- Mature ecosystem, proven by VS Code, Slack, Discord
- User sees exactly what the AI sees — full transparency

**Why NOT Tauri:**
- WKWebView on macOS = Safari engine, not Chromium → site compatibility issues
- No CDP access on WKWebView
- Would need to bundle Chromium separately, defeating Tauri's size advantage

### Option B: Chrome Extension + Sidebar (Lighter weight)

Instead of a standalone app, enhance the existing Tivana extension with:
- A sidebar panel (Chrome Side Panel API) for agent reasoning
- Auto-attach to all tabs (no click needed)
- Native Messaging to Tivana runtime for reliability
- BYOK key storage in extension settings

**Pros:** Uses user's real Chrome, no separate app
**Cons:** MV3 service worker limitations, Chrome-only, extension review for debugger permission

### Option C: PWA with Native Bridge

A web app that connects to a local Tivana runtime via localhost WebSocket. The browser is the user's own Chrome. Less control but simplest to distribute.

**Pros:** No install, works in any browser
**Cons:** Limited CDP access, can't control the browser it's running in

---

## Recommended: Option A (Electron) — Detailed Plan

### Phase 1: Foundation (Week 1-2)
**Goal:** Basic Electron app that opens a browser and can be controlled by an LLM

1. **Electron scaffold**
   - electron-forge or electron-vite setup
   - Main process (Node.js) + Renderer process (React)
   - BrowserView for the controlled browser pane
   - React sidebar for agent UI

2. **Tivana Core integration**
   - Compile Tivana Rust core as napi-rs native addon
   - OR: ship Tivana binary and communicate via IPC
   - CDP bridge: `webContents.debugger.attach()` → Tivana perception engine
   - Element extraction, click, type, scroll, fill — all working via Electron CDP

3. **Basic agent loop**
   - Perceive page → send elements to LLM → parse action → execute
   - Single LLM provider (start with OpenAI)
   - Simple text-based sidebar showing agent thoughts

### Phase 2: Multi-Provider BYOK (Week 3)
**Goal:** Users can plug in any API key

4. **Provider abstraction layer**
   ```typescript
   interface LLMProvider {
     chat(messages: Message[], tools: Tool[]): AsyncGenerator<Delta>;
     supportsVision(): boolean;
     name: string;
   }
   
   class OpenAIProvider implements LLMProvider { ... }
   class AnthropicProvider implements LLMProvider { ... }
   class GeminiProvider implements LLMProvider { ... }
   class OllamaProvider implements LLMProvider { ... }  // Local models
   class OpenRouterProvider implements LLMProvider { ... }
   ```

5. **Settings UI**
   - API key input per provider (stored in OS keychain via keytar)
   - Model selection dropdown per provider
   - Test connection button
   - Usage/cost tracking display

6. **Tool definitions for the LLM**
   ```typescript
   const tools = [
     { name: "navigate", description: "Go to a URL", params: { url: "string" } },
     { name: "click", description: "Click element by ID", params: { id: "number" } },
     { name: "type", description: "Type text into element", params: { id: "number", text: "string" } },
     { name: "fill", description: "Set field value instantly", params: { id: "number", value: "string" } },
     { name: "scroll", description: "Scroll page", params: { direction: "up|down", amount: "number" } },
     { name: "select", description: "Select dropdown option", params: { id: "number", value: "string" } },
     { name: "screenshot", description: "Take screenshot for visual analysis" },
     { name: "wait", description: "Wait for page changes", params: { seconds: "number" } },
     { name: "done", description: "Task complete" },
     { name: "ask_user", description: "Ask user for input/clarification" },
   ];
   ```

### Phase 3: UX Polish (Week 4)
**Goal:** Match or exceed Operator UX

7. **Agent sidebar**
   - Streaming LLM responses (typing animation)
   - Collapsible reasoning sections
   - Element highlighting (show what the AI is looking at)
   - Action history with timestamps
   - Error display with retry options

8. **Control bar**
   - "Take Control" — pauses agent, gives keyboard/mouse back to user
   - "Stop" — kills current task
   - "Resume" — continues after user intervention
   - Active task indicator
   - Login status detection ("Logged in to LinkedIn, Indeed, ...")

9. **Standing permissions**
   - Pre-authorize actions: "You can apply to remote DevOps jobs $140K+"
   - Guardrails: "Never submit without showing me the final form"
   - "Always ask before uploading resume"
   - Stored per-task, user can revoke anytime

10. **Visual feedback**
    - Highlight elements before clicking (red border flash)
    - Smooth scrolling to target elements
    - Mouse cursor animation showing where AI clicks
    - Screenshot overlay when AI is "looking" at the page

### Phase 4: Distribution (Week 5-6)
**Goal:** Users can download and run it

11. **Packaging**
    - electron-builder for macOS (.dmg), Windows (.exe), Linux (.AppImage)
    - Auto-updater via electron-updater
    - Code signing (Apple Developer ID, Windows Authenticode)
    - Notarization for macOS

12. **First-run experience**
    - Welcome screen with provider selection
    - API key entry
    - Quick tutorial: "Let's navigate to a website together"
    - Import bookmarks/passwords (optional)

---

## Technical Decisions

### Tivana Core: Native Addon vs Sidecar Binary

| Approach | Pros | Cons |
|----------|------|------|
| **napi-rs addon** | Single process, fast IPC, no port management | Complex build, tied to Electron's Node version |
| **Sidecar binary** | Existing Tivana binary works as-is, easier debugging | IPC overhead, port management, process lifecycle |

**Recommendation:** Start with sidecar binary (ship existing Tivana, communicate via IPC/WebSocket). Migrate to napi-rs addon later if performance matters.

### Actually, do we even need the Rust runtime?

With Electron's `webContents.debugger` API, we get full CDP access directly from Node.js. The perception engine (element extraction) and action engine (click, type) could be pure TypeScript running in the main process. The Rust runtime was needed for the standalone server model — but in Electron, Node.js IS the server.

**Recommendation:** Port Tivana's perception and action logic to TypeScript for the Electron app. Keep the Rust runtime as a standalone tool for non-Electron use cases (MCP server, CLI, SDK).

### LLM Integration: Function Calling vs ReAct

| Pattern | Approach | Best For |
|---------|----------|----------|
| **Function Calling** | LLM returns structured tool calls | OpenAI, Anthropic (native support) |
| **ReAct** | LLM outputs thoughts + actions in text | Gemini, open models |
| **Hybrid** | Use function calling when available, fall back to ReAct | Maximum compatibility |

**Recommendation:** Hybrid. Use native function/tool calling for OpenAI and Anthropic. Parse structured output for Gemini and Ollama.

### Screenshot vs Elements vs Both

| Mode | What LLM sees | Cost | Speed |
|------|---------------|------|-------|
| **Elements only** | Structured element list (IDs, roles, text) | Low (text tokens) | Fast |
| **Screenshot only** | Image of the page | High (vision tokens) | Slow |
| **Both** | Elements + screenshot | Highest | Slowest |
| **Elements + screenshot on demand** | Elements normally, screenshot when confused | Low normally | Fast normally |

**Recommendation:** Elements-first with screenshot fallback. Send structured elements to the LLM by default. If the LLM says "I'm confused" or "I can't find the element," take a screenshot and send it. This keeps costs low and speed high for most interactions.

---

## Competitive Landscape

| Product | BYOK | Open Source | Standalone App | Approach | Pricing |
|---------|------|-------------|----------------|----------|---------|
| **ChatGPT Operator** | ❌ OpenAI only | ❌ | ❌ (web only) | Consumer browser, human-in-loop | $20-200/mo |
| **Anthropic Computer Use** | ❌ Anthropic only | ❌ | ❌ | Screenshot-based, VM control | Usage-based |
| **Google Project Mariner** | ❌ Google only | ❌ | ❌ (Chrome ext) | Extension-based | Unknown |
| **Browser Use** | ✅ | ✅ Python | ❌ (library) | Playwright + LLM, stealth, CAPTCHA | Free (BYOK) |
| **Skyvern** | ❌ | Partial | ❌ (cloud/self-host) | Visual + DOM, form fill, MCP | Freemium |
| **AgentSmith** | ❌ | ❌ | ❌ (Chrome ext) | Extension, job-app focused | $?/mo |
| **Browserbase** | ❌ | ❌ | ❌ (cloud) | Headless browser infra | Usage-based |
| **Tivana Atlas** | ✅ Any provider | ✅ | ✅ Electron app | Perception protocol + BYOK | Free (BYOK) |

### Key competitor takeaways:
- **Browser Use** (https://docs.browser-use.com/) — Python automation platform. Persistent sessions, CAPTCHA solving, proxies, multi-step workflows. Positioned as "state-of-the-art AI browser automation." Closer to unattended automation than a consumer browser. Good stealth docs.
- **Skyvern** (https://www.skyvern.com/) — Agent-first browser automation. AI fills forms, extracts data, multi-step workflows. Their examples explicitly include automating job applications. Has MCP integration.
- **AgentSmith** (https://agentsmith.so/) — Chrome extension directly targeting job applications. "Fill out these job applications with my resume info" across LinkedIn, Indeed, Greenhouse. Most directly competing with our immediate use case.

### Our differentiators vs ALL of them:
1. **BYOK** — Browser Use is BYOK too, but the rest aren't. We're the only Electron app with BYOK.
2. **Open source Electron app** — not a cloud service, not a Chrome extension, not a Python library
3. **Perception protocol** — structured element extraction > raw screenshots (cheaper + faster than Computer Use)
4. **Cost transparency** — see exactly how many tokens each action costs
5. **Privacy** — everything runs locally, no data sent to our servers
6. **Cross-LLM** — works with OpenAI, Anthropic, Gemini, Ollama, OpenRouter, any OpenAI-compatible endpoint
7. **Desktop-native** — real app with system tray, notifications, auto-updater

---

## Revenue Model Options

Since the browser is free (BYOK), revenue comes from:

1. **Pro features** — advanced task scheduling, parallel browsers, team sharing
2. **Task marketplace** — sell/buy automation recipes (10% cut)
3. **Managed hosting** — run Tivana Atlas in the cloud (Browserbase competitor)
4. **Enterprise** — SSO, audit logs, compliance, centralized key management
5. **Sponsored models** — LLM providers pay for default placement in provider list

---

## Naming

- **Tivana Atlas** — keeps the Tivana brand, "Atlas" nods to the inspiration
- **Tivana Browser** — straightforward
- **Tivana Agent** — emphasizes the AI aspect
- **Superbrowser** — catchy, different
- **Pilot** — "your AI co-pilot for the web"
- **Navigator** — clean, descriptive

---

## Minimum Viable Product (2-week sprint)

If we want something working in 2 weeks:

1. Electron app with BrowserView + React sidebar
2. Single LLM provider (OpenAI function calling)
3. Tivana perception engine (port element extraction to TS)
4. Basic tools: navigate, click, type, fill, scroll, screenshot
5. Agent loop: perceive → think → act → report
6. Settings page for API key
7. "Take control" / "Stop" buttons
8. macOS .dmg package

**NOT in MVP:** Multi-provider, standing permissions, task marketplace, auto-updater, Windows/Linux builds

---

## Open Questions

1. Should we fork Electron or use vanilla? (Cypress forks Electron for deeper control)
2. Profile management — one profile or multiple? (personal, work, etc.)
3. Should the browser support extensions? (password managers, ad blockers)
4. How do we handle 2FA / OTP — ask user to type it, or integrate with authenticator apps?
5. Multi-tab support — one agent per tab, or one agent controlling multiple tabs?
6. Should we ship Tivana CLI and Tivana Atlas as separate products or one package?
7. What's the name? (Need to decide before creating repo/branding)

---

## Next Steps

1. **Kaise decides:** Go or no-go on Electron approach
2. **Name the product**
3. **Create new repo** (or monorepo alongside Tivana)
4. **Scaffold Electron app** with BrowserView + React sidebar
5. **Port Tivana perception to TypeScript** (or use Rust sidecar)
6. **Implement agent loop** with OpenAI function calling
7. **Build settings UI** for API key entry
8. **Test with job applications** — dogfood immediately
