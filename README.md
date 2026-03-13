# Tivana

**Streaming browser perception protocol for AI agents.**

Tivana gives AI agents human-like awareness of web pages — not scripted automation, but continuous, semantic perception of page state so agents can explore, notice anomalies, and make judgment calls.

## Why Tivana?

Existing browser automation tools are built for testing, not agency:

- **Playwright/Puppeteer** — Execute predefined scripts, blind between steps
- **Screenshots + Vision** — Heavy, lossy, point-in-time, can't reference elements
- **Raw CDP** — Too low-level, requires browser automation expertise

Humans catch bugs that tests miss because we see the whole page, notice things that "feel off," and have continuous awareness. Tivana gives agents the same capability.

## 3-Step Setup

```bash
npm install tivana
npx tivana
```

```typescript
import { observe, act } from "tivana";

// Receive streaming page state
observe((page) => {
  console.log(`Now at: ${page.url}`);
  console.log(`Elements: ${page.elements.length}`);
});

// Take actions by element reference
await act.click("e3");
await act.type("hello world");
```

## What the Agent Sees

```typescript
{
  url: "https://github.com/login",
  title: "Sign in to GitHub",
  
  elements: [
    {
      id: "e1",
      role: "textbox",
      label: "Username",
      focused: true,
      bounds: { x: 200, y: 150, width: 280, height: 40 },
      font: { family: "Inter", size: "16px", weight: 400, color: "#24292f" },
      background: "#ffffff",
      border: { width: "1px", color: "#d0d7de", radius: "6px" }
    },
    // ... full visual + semantic data for every element
  ]
}
```

## Full Visual Awareness

Unlike accessibility-tree-only approaches, Tivana includes computed styles:

- **Typography** — font family, size, weight, color, line-height
- **Colors** — background, foreground, border colors
- **Geometry** — bounds, padding, margin
- **Borders** — width, style, color, radius
- **Layout** — display, flex properties, alignment
- **Accessibility** — contrast ratio, focus visibility, ARIA attributes

This enables use cases like visual regression testing, accessibility auditing, and design system validation.

## Requirements

- Chromium-based browser (Chrome, Edge, Brave, Arc)
- Node.js 18+
- macOS, Linux, or Windows
- No browser extensions required

## Architecture

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Browser    │────►│   Runtime    │────►│    Agent     │
│  (Chromium)  │◄────│  (tivana)    │◄────│   (LLM/AI)   │
└──────────────┘     └──────────────┘     └──────────────┘
```

- **Runtime**: Rust + Raw CDP (chromiumoxide, tokio)
- **Agent SDK**: TypeScript (thin WebSocket client)
- **Protocol**: JSON over WebSocket

## Documentation

See the [docs](./docs) folder for detailed documentation:

- [What It Is](./docs/what-it-is.md) — Overview and supported agents
- [Protocol Specification](./docs/protocol-specification.md) — Message formats
- [Element Model](./docs/element-model.md) — Full visual + semantic schema
- [Action Primitives](./docs/action-primitives.md) — Available agent actions
- [Architecture](./docs/architecture.md) — Runtime, CDP, data flow
- [Integration Guide](./docs/integration-guide.md) — How to connect an agent
- [Developer Experience](./docs/developer-experience.md) — 3-step setup, visual awareness
- [Use Cases](./docs/use-cases.md) — Accessibility, visual regression, exploratory QA
- [Success Criteria](./docs/success-criteria.md) — DX, perception, action targets
- [Edge Cases](./docs/edge-cases.md) — Dynamic content, auth, complex UIs
- [Tech Stack](./docs/tech-stack.md) — Rust runtime, TypeScript SDK

## License

MIT
