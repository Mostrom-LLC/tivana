# Tivana Examples

Working examples that demonstrate Tivana's perception-first approach to agent browsing.

Each example shows a different use case where an agent perceives the page, reasons about what it sees, and acts based on judgment — not hardcoded scripts.

## Examples

| Example | Description | Key concept |
|---------|-------------|-------------|
| [01-observe-and-explore](01-observe-and-explore.ts) | Connect, observe, and explore a page | Perception basics, element discovery |
| [02-agent-loop](02-agent-loop.ts) | Perceive → reason → act loop | Agent decision-making pattern |
| [03-accessibility-review](03-accessibility-review.ts) | Review a page for accessibility issues | Judgment-based perception |
| [04-anomaly-detection](04-anomaly-detection.ts) | Detect visual and structural anomalies | Exploratory QA |
| [05-form-awareness](05-form-awareness.ts) | Perceive and understand form structure | Semantic form understanding |
| [06-event-streaming](06-event-streaming.ts) | Real-time page event monitoring | Observation lifecycle |
| [07-design-token-extraction](07-design-token-extraction.ts) | Extract design tokens from any site | W3C DTCG output |

## Running

### Prerequisites

1. Build and start the Tivana runtime:

```bash
cd runtime
cargo build --release
./target/release/tivana --port 9876
```

2. Install SDK dependencies:

```bash
cd sdk/ts
bun install
```

### Run an example

After installing the SDK (`npm install tivana` or linking locally):

```bash
# With bun (from repo root)
bun run examples/01-observe-and-explore.ts

# With tsx
npx tsx examples/01-observe-and-explore.ts
```

For local development (without npm install):

```bash
cd sdk/ts && bun install && cd ../..
bun run examples/01-observe-and-explore.ts
```

With the Chrome extension (for real browser sessions):

```bash
# Start runtime, connect extension, then:
bun run examples/06-event-streaming.ts
```

## Design Philosophy

These examples intentionally avoid:
- Hardcoded selectors or field matchers
- Site-specific automation logic
- Label heuristics or radio button handlers

Instead, they show agents that:
- Perceive the full page state
- Reason about what they see
- Make judgment calls based on semantic understanding
- Adapt to whatever page is in front of them
