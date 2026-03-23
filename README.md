# Tivana

Perception-first browser protocol for AI agents.

Tivana gives agents continuous, semantic awareness of web pages so they can perceive what is on screen, reason about it, and take action. The agent makes the decisions. Tivana provides the eyes and hands.

Status: pre-release.

## What Tivana Is

- A protocol and runtime for semantic browser perception.
- A thin action layer for clicking, typing, scrolling, and navigation by semantic target.
- A foundation for exploratory QA, accessibility review, visual reasoning, and human-observable agent browsing.

## What Tivana Is Not

- A hardcoded site automation framework.
- A collection of brittle field matchers and workflow scripts.
- A stealth, CAPTCHA, or proxy product.
- A replacement for agent reasoning.

## Core Model

The intended loop is simple:

1. Perceive the page.
2. Reason about what the page means.
3. Act on the page.
4. Observe the result and adapt.

```text
Browser -> Tivana runtime -> Agent
   ^            |             |
   |            v             |
   +-------- semantic action <-+
```

Tivana should not contain site-specific business logic like "if the label contains sponsorship, answer No." That belongs in the agent loop, not in the runtime.

## Current Capabilities

### Perceive

- `perceive.pageState`
- `perceive.elements`
- `perceive.accessibilitySnapshot`
- `perceive.textContent`
- `perceive.metadata`
- `perceive.mutations` at the protocol/runtime layer

### Act

- `act.navigate`
- `act.click`
- `act.type`
- `act.press`
- `act.scroll`
- `act.hover`
- `act.focus`
- `act.select`

### Secondary Utilities

These exist, but they are not the center of the product story:

- JavaScript evaluation
- screenshots
- network capture
- tabs
- cookies and storage
- extension-backed session transport

## Quick Start

### 1. Build the runtime

```bash
git clone https://github.com/Mostrom-LLC/tivana.git
cd tivana/runtime
cargo build --release
```

### 2. Start Tivana

```bash
./target/release/tivana
```

Common options:

```bash
./target/release/tivana --headless
./target/release/tivana --port 9876
./target/release/tivana --connect 9222
```

### 3. Use the SDK

```bash
cd ../sdk/ts
bun install
```

## Minimal Perceive -> Reason -> Act Example

```typescript
import { TivanaClient } from "tivana";

const client = new TivanaClient();
await client.connect("ws://localhost:9876");
await client.createSession();

await client.navigate("https://example.com");

const page = await client.pageState();
const elements = await client.elements();

console.log(page.url);
console.log(elements.map((el) => `${el.id} ${el.role} ${el.name ?? ""}`));

// Your agent decides what to do from the current page state.
const target = elements.find((el) => el.role === "link" && el.name?.includes("More information"));

if (target) {
  await client.click(target.id);
}

await client.closeSession();
client.disconnect();
```

## Example Agent Loop

Illustrative only. The important point is where the reasoning lives.

```typescript
import { TivanaClient } from "tivana";

type AgentDecision =
  | { type: "click"; target: string }
  | { type: "type"; target: string; text: string }
  | { type: "navigate"; url: string }
  | { type: "done"; summary: string };

async function decideWithModel(input: {
  goal: string;
  page: unknown;
  elements: unknown;
}): Promise<AgentDecision> {
  // Send Tivana perception to your model here.
  throw new Error("Implement model call");
}

const client = new TivanaClient();
await client.connect("ws://localhost:9876");
await client.createSession();

const goal = "Sign in if a sign-in form is present.";

for (let step = 0; step < 20; step++) {
  const [page, elements] = await Promise.all([
    client.pageState(),
    client.elements(),
  ]);

  const decision = await decideWithModel({ goal, page, elements });

  if (decision.type === "done") {
    console.log(decision.summary);
    break;
  }

  if (decision.type === "navigate") {
    await client.navigate(decision.url);
    continue;
  }

  if (decision.type === "click") {
    await client.click(decision.target);
    continue;
  }

  if (decision.type === "type") {
    await client.type(decision.text, decision.target);
  }
}
```

## Architecture

```text
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Browser    │<--->│   Runtime    │<--->│    Agent     │
│  (Chromium)  │     │   (Tivana)   │     │  (LLM/Code)  │
└──────────────┘     └──────────────┘     └──────────────┘
```

- Runtime: Rust, WebSocket, protocol routing, browser integration.
- Browser transport: managed Chromium and extension-backed sessions currently exist.
- Agent surface: TypeScript SDK today, protocol is transportable to other agent clients.
- Design intent: perception first, actions second.

## Use Cases

- Exploratory QA
- accessibility review
- semantic browsing by coding agents
- visual anomaly detection
- flow validation where the agent must adapt instead of following a rigid script

## Current Project Direction

The runtime and SDK already have solid Perceive + Act primitives. The next focus is making the agent loop first-class:

- observation should be the default integration path
- mutation/event streaming should be explicit and ergonomic in the SDK
- examples and demos should show agent judgment, not hardcoded site scripts

See [tasks/refocus-plan.md](/Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/tasks/refocus-plan.md) for the reset plan.

## Repository Map

- [docs/what-it-is.md](/Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/docs/what-it-is.md)
- [docs/protocol-specification.md](/Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/docs/protocol-specification.md)
- [docs/architecture.md](/Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/docs/architecture.md)
- [docs/use-cases.md](/Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/docs/use-cases.md)
- [sdk/ts/README.md](/Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/sdk/ts/README.md)

## Running Tests

```bash
cd runtime
cargo test

# Browser-dependent tests
cargo test --test browser_test -- --ignored --nocapture
cargo test --test realistic_browser_test -- --ignored --nocapture --test-threads=1
```

## License

MIT © Mostrom LLC 2025
