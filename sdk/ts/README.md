# Tivana TypeScript SDK

TypeScript client for Tivana's browser perception protocol.

Use this SDK when you want an agent runner to:

1. inspect the current page semantically
2. decide what to do next
3. act through Tivana
4. repeat

The SDK is not meant to be a place for site-specific automation logic. Tivana provides perception and action primitives; your agent provides the reasoning.

## Installation

```bash
npm install tivana
```

For local development:

```bash
cd sdk/ts
bun install
```

## Recommended Usage

Use `TivanaClient` directly for explicit agent loops.

```typescript
import { TivanaClient } from "tivana";

const client = new TivanaClient({ url: "ws://localhost:9876" });
await client.connect();
await client.createSession();

await client.navigate("https://example.com");

const page = await client.pageState();
const elements = await client.elements();

console.log(page.title);
console.log(elements.length);
```

## Example Agent Loop

```typescript
import { TivanaClient } from "tivana";

type AgentDecision =
  | { type: "click"; target: string }
  | { type: "type"; target: string; text: string }
  | { type: "press"; key: string; modifiers?: string[] }
  | { type: "navigate"; url: string }
  | { type: "done"; summary: string };

async function decideWithModel(input: {
  goal: string;
  profile: Record<string, unknown>;
  page: unknown;
  elements: unknown;
}): Promise<AgentDecision> {
  // Call your model here with Tivana perception.
  throw new Error("Implement model call");
}

const client = new TivanaClient();
await client.connect("ws://localhost:9876");
await client.createSession();

const goal = "Complete the current flow using the provided profile.";
const profile = {
  sponsorshipRequired: false,
  authorizedToWork: true,
};

for (let step = 0; step < 30; step++) {
  const [page, elements] = await Promise.all([
    client.pageState(),
    client.elements(),
  ]);

  const decision = await decideWithModel({ goal, profile, page, elements });

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
    continue;
  }

  if (decision.type === "press") {
    await client.press(decision.key, decision.modifiers);
  }
}
```

## Core API

### Session

- `connect(url?)`
- `createSession(params?)`
- `closeSession()`
- `listSessions()`
- `disconnect()`

### Perception

- `pageState()`
- `elements()`
- `accessibilitySnapshot()`
- `textContent()`
- `metadata()`
- `findElements(selector)`
- `formFields()`

These methods are the heart of Tivana. Most agent integrations should start here.

### Actions

- `navigate(url)`
- `click(target, options?)`
- `type(text, target?, options?)`
- `press(key, modifiers?)`
- `scroll(target?, direction?, options?)`
- `hover(target)`
- `focus(target)`
- `select(target, value)`
- `waitFor(condition, timeoutMs?)`

These actions should be driven by current perception, not by brittle hardcoded assumptions.

## Secondary API

Available, but not the primary product story:

- `evaluate(expression, awaitPromise?)`
- `evaluateVoid(expression)`
- `screenshot(options?)`
- `enableNetworkCapture()`
- `getNetworkRequests(urlPattern?)`
- tab management helpers
- cookies and storage helpers
- extension-backed session helpers

## Observation & Page Events

The SDK provides first-class observation of page events via `observe()`:

```typescript
import { TivanaClient, observe, act } from "tivana";

const client = new TivanaClient();
await client.connect("ws://localhost:9876");
await client.createSession();
await client.navigate("https://example.com");

// Observe page events
const stop = await observe(async (event) => {
  console.log(`[${event.type}]`, event.data);

  if (event.type === "page.loaded") {
    const elements = await client.elements();
    // Agent reasons about elements and decides...
  }
});

// Later: stop()
```

Supported event types: `page.mutation`, `page.loaded`, `page.navigated`, `page.focus`, `page.scroll`, `page.resize`.

You can filter to specific events:

```typescript
const stop = await observe(callback, {
  events: ["page.loaded", "page.navigated"],
});
```

For lower-level control, use the `TivanaClient` event API directly:

```typescript
await client.startObservation();
client.onPageEvent("page.navigated", (event) => { ... });
client.onEvent((event) => { ... }); // all events
await client.stopObservation();
```

The legacy `onMutation(callback)` API continues to work for DOM mutation events only.

## Targets

The SDK supports semantic targets for actions:

```typescript
await client.click("e5");
await client.click({ role: "button", label: "Continue" });
await client.type("hello@example.com", "e3");
```

Prefer element IDs from `elements()` as the primary target model.

## Types

### PageState

```typescript
interface PageState {
  url: string;
  title: string | null;
  focusedElementId: string | null;
  scrollX: number;
  scrollY: number;
  viewportWidth: number;
  viewportHeight: number;
  documentWidth: number;
  documentHeight: number;
  timestampMs: number;
}
```

### Element

```typescript
interface Element {
  id: string;
  role: string;
  name?: string;
  value?: string;
  description?: string;
  bounds?: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
  styles?: {
    fontFamily?: string;
    fontSize?: string;
    fontWeight?: string;
    color?: string;
    backgroundColor?: string;
    border?: string;
    display?: string;
    visibility?: string;
  };
  focused: boolean;
  enabled: boolean;
  checked?: boolean;
  selected?: boolean;
  expanded?: boolean;
  required?: boolean;
  children?: Element[];
}
```

### ActionResult

```typescript
interface ActionResult {
  success: boolean;
  pageState?: PageState;
  data?: unknown;
  durationMs: number;
}
```

## Error Handling

Errors are surfaced with structured codes:

```typescript
try {
  await client.click("e999");
} catch (error) {
  console.error(String(error));
}
```

Common categories:

- protocol errors
- session errors
- browser errors
- action errors
- internal errors

## Runtime

The SDK requires the Tivana runtime to be running.

```bash
cd tivana/runtime
cargo build --release
./target/release/tivana --port 9876
```

See the root [README.md](/Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/README.md) for the broader project direction.

## License

MIT
