# CLAUDE.md — Tivana

Tivana is a browser perception protocol for AI agents.

## Skills

The `skills/tivana/` directory contains a SKILL.md that teaches agents how to use Tivana's SDK.

```
skills/tivana/SKILL.md — Full API reference, agent loop patterns, examples
```

## Project Structure

- `runtime/` — Rust runtime (Cargo, WebSocket server, CDP bridge)
- `sdk/ts/` — TypeScript SDK (`tivana` npm package)
- `extension/` — Chrome extension for real browser tab sessions
- `examples/` — 7 working demos
- `docs/` — Protocol specification, architecture, observation guide
- `docs-site/` — Fumadocs documentation site

## Development

```bash
# Build runtime
cd runtime && cargo build --release

# Run tests
cargo test

# Start runtime
./target/release/tivana

# SDK
cd sdk/ts && bun install && bun run build
```

## Key Files

- `docs/protocol-specification.md` — Full protocol spec
- `docs/architecture.md` — System architecture
- `docs/observation-guide.md` — Snapshot vs event model
- `sdk/ts/README.md` — SDK API reference
- `skills/tivana/SKILL.md` — Agent skill definition
