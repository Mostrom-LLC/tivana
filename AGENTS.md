# AGENTS.md — Tivana

Tivana is a browser perception protocol for AI agents.

## Skills

Read `skills/tivana/SKILL.md` for the full Tivana API, agent loop patterns, and examples.

## Project Structure

- `runtime/` — Rust runtime (WebSocket server, CDP bridge)
- `sdk/ts/` — TypeScript SDK (`tivana` npm package)
- `extension/` — Chrome extension for real browser tab sessions
- `examples/` — 7 working demos
- `docs/` — Protocol and architecture documentation

## Development

```bash
cd runtime && cargo build --release && cargo test
cd sdk/ts && bun install && bun run build
```
