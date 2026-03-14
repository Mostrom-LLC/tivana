# Building Tivana

## Requirements

### Rust Runtime

- **Rust**: 1.75+ (for async traits)
- **Chromium**: Chrome, Edge, Brave, or Arc installed
- **Platform**: macOS, Linux, or Windows

### TypeScript SDK

- **Bun**: 1.0+ (recommended) or Node.js 18+
- **npm/bun**: For package management

## Building the Runtime

```bash
cd runtime

# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=tivana=debug cargo run -- --port 9876 --headed
```

The binary will be at `target/release/tivana` (or `target/debug/tivana`).

## Building the SDK

```bash
cd sdk/ts

# Install dependencies
bun install

# Build
bun run build

# Type check
bun run typecheck

# Run tests
bun test

# Run example
bun run example
```

## Running Tivana

### Start the Runtime

```bash
# Default (headed mode, port 9876)
./tivana

# Headless mode
./tivana --headless

# Custom port
./tivana --port 8080

# With custom Chrome path
./tivana --chrome-path /path/to/chrome
```

### Use the SDK

```typescript
import { TivanaClient } from "tivana";

const client = new TivanaClient();
await client.connect("ws://localhost:9876");
await client.createSession();

// Navigate
await client.navigate("https://example.com");

// Get page state
const page = await client.pageState();
console.log(page.url, page.title);

// Get elements
const elements = await client.elements();
for (const el of elements) {
  console.log(`${el.id}: ${el.role} "${el.label}"`);
}

// Click
await client.click("e5");

// Type
await client.type("hello world", "e3");

// Close
await client.closeSession();
client.disconnect();
```

## Development Environment

### CI Requirements

- GitHub Actions runners need Chromium installed
- Use `ubuntu-latest` with Chrome pre-installed
- Or add `google-chrome-stable` via apt

### Docker

If running in Docker, ensure:
- No sandbox mode (`--no-sandbox` flag is included by default)
- Shared memory is adequate (`--shm-size=2g`)

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY runtime/ .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y chromium && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/tivana /usr/local/bin/
EXPOSE 9876
CMD ["tivana", "--headless", "--port", "9876"]
```

## Troubleshooting

### Browser fails to launch

- Ensure Chrome/Chromium is installed and accessible
- Try specifying `--chrome-path` explicitly
- In containers, ensure `--no-sandbox` is applied (default)

### WebSocket connection refused

- Check the runtime is running: `netstat -tlnp | grep 9876`
- Verify firewall rules allow the port
- Try `--host 0.0.0.0` for external access

### Elements not found

- Elements are indexed from 1: `e1`, `e2`, etc.
- IDs reset after navigation
- Call `elements()` to get fresh IDs after page changes
