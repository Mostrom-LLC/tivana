# What It Is

A standardized way for AI agents (OpenClaw, Codex, Claude, OpenHands, etc.) to browse the web with human-like awareness — not scripted automation.

Unlike Playwright or Puppeteer which execute predefined scripts, Agent Browser Protocol provides continuous, semantic awareness of page state so agents can explore, notice anomalies, and make judgment calls.


---

## The Problem
Existing browser automation tools are built for testing, not agency.

- Playwright/Puppeteer: Execute scripts, check assertions, blind between steps
- Screenshots + Vision: Heavy, lossy, point-in-time, cannot reference elements
- CDP raw: Too low-level, requires browser internals knowledge
Humans catch bugs that tests miss because we see the whole page, notice things that "feel off," and have continuous awareness.


---

## How It Works
The protocol streams semantic page state to agents in real-time:

Agents send actions back by element reference (not coordinates):

- Click element by ID
- Type into focused element
- Scroll element into view
- Wait for condition

---

## Requirements
- Chromium-based browser (Chrome, Edge, Brave, Arc)
- Node.js 18+ (for TypeScript SDK)
- macOS, Linux, or Windows
- No browser extensions required

---

## Supported Agents
- OpenClaw — Planned
- Claude (computer use) — Planned
- Codex — Planned
- OpenHands — Planned
- Custom — Protocol is open

---

## Subpages
- Protocol Specification — Message formats, events, actions
- Element Model — How page elements are represented
- Action Primitives — Available agent actions
- Architecture — Runtime, CDP, data flow
- Integration Guide — How to connect an agent
- Developer Experience — 3-step setup, visual awareness
- Use Cases — Accessibility, visual regression, exploratory QA
- Success Criteria — DX, perception, action, performance targets
- Edge Cases — Dynamic content, auth, complex UIs
- Tech Stack — Rust runtime, TypeScript SDK
