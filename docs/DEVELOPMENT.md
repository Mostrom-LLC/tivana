# Tivana Development Guide

This guide covers building, running, and testing Tivana locally for contributors and developers.

## Prerequisites

### Required

| Tool | Version | Purpose |
|------|---------|---------|
| **Rust** | 1.75+ | Runtime compilation |
| **Chromium** | Any recent | Browser automation (Chrome, Edge, Brave, Arc) |
| **Bun** | 1.0+ | TypeScript SDK (recommended) |

### Optional

| Tool | Version | Purpose |
|------|---------|---------|
| **Node.js** | 18+ | Alternative to Bun |
| **Docker** | 20+ | Containerized builds |

### Installing Prerequisites

**Rust:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update
```

**Bun:**
```bash
curl -fsSL https://bun.sh/install | bash
```

**Chromium (if not installed):**
```bash
# macOS
brew install --cask chromium

# Ubuntu/Debian
sudo apt-get install chromium-browser

# Or use Chrome, Edge, Brave, or Arc - Tivana auto-detects
```

## Building the Runtime

```bash
cd runtime

# Development build (faster compile, slower runtime)
cargo build

# Release build (optimized for production)
cargo build --release

# Binary location
# Debug: target/debug/tivana
# Release: target/release/tivana
```

### Build with Logging

```bash
RUST_LOG=tivana=debug cargo build
```

### Cross-Compilation

```bash
# For Linux (from macOS)
rustup target add x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-gnu
```

## Running the Runtime

### Basic Usage

```bash
# Headed mode (default) - browser visible
./target/release/tivana

# Headless mode - no browser window
./target/release/tivana --headless

# Custom port (default: 9876)
./target/release/tivana --port 8080

# Custom Chrome path
./target/release/tivana --chrome-path /path/to/chrome
```

### With Debug Logging

```bash
RUST_LOG=tivana=debug ./target/release/tivana --headed
```

### Verifying Runtime

```bash
# Check if running
curl -s http://localhost:9876/health || echo "Not running"

# Or check port
lsof -i :9876
```

## Building the TypeScript SDK

```bash
cd sdk/ts

# Install dependencies
bun install

# Build TypeScript
bun run build

# Type check (no emit)
bun run typecheck
```

### SDK Output

- `dist/index.js` - Compiled JavaScript
- `dist/index.d.ts` - TypeScript declarations

## Running Tests

### Rust Unit Tests

```bash
cd runtime

# All unit tests
cargo test

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture
```

### Browser Integration Tests

These tests require Chromium installed and accessible.

```bash
cd runtime

# Basic browser tests
cargo test --test browser_test -- --ignored --nocapture

# Realistic tests (uses https://the-internet.herokuapp.com)
cargo test --test realistic_browser_test -- --ignored --nocapture --test-threads=1
```

### SDK Smoke Test

The SDK doesn't have unit tests — it requires the runtime to be running. Use the smoke test for validation:

```bash
# Terminal 1: Start runtime
./target/release/tivana

# Terminal 2: Run smoke test
cd sdk/ts
bun run smoke-test.ts
```

The smoke test connects to the runtime, navigates to a page, and verifies perception/action methods.

### SDK Type Checking

```bash
cd sdk/ts

# Type check only (no runtime required)
bun run typecheck
```

### Running All Tests (CI-style)

```bash
# From repo root
cd runtime && cargo test
cd runtime && cargo test --test browser_test -- --ignored
# SDK validation requires runtime - run smoke test manually
```

## Project Structure

```
tivana/
├── runtime/                 # Rust runtime
│   ├── src/
│   │   ├── main.rs         # Entry point
│   │   ├── cli.rs          # CLI argument parsing
│   │   ├── server.rs       # WebSocket server
│   │   ├── session.rs      # Browser session management
│   │   ├── browser.rs      # CDP browser control
│   │   ├── perceive.rs     # Element perception
│   │   ├── act.rs          # Action execution
│   │   ├── protocol.rs     # Protocol types
│   │   └── error.rs        # Error types
│   ├── tests/
│   │   ├── browser_test.rs
│   │   └── realistic_browser_test.rs
│   └── Cargo.toml
├── sdk/
│   └── ts/                  # TypeScript SDK
│       ├── src/
│       │   ├── index.ts    # Main exports
│       │   ├── client.ts   # TivanaClient
│       │   └── types.ts    # Type definitions
│       ├── tests/
│       └── package.json
├── docs/                    # Documentation
├── skills/                  # OpenClaw skills
└── README.md
```

## Development Workflow

### Making Changes to the Runtime

1. Edit Rust source in `runtime/src/`
2. Run `cargo check` for fast feedback
3. Run `cargo test` for unit tests
4. Run `cargo build` to compile
5. Test manually with the SDK smoke test

### Making Changes to the SDK

1. Edit TypeScript in `sdk/ts/src/`
2. Run `bun run typecheck` for type errors
3. Test manually with `bun run smoke-test.ts` (requires runtime)

### Adding a New Action

1. Add protocol message to `runtime/src/protocol.rs`
2. Implement handler in `runtime/src/act.rs`
3. Add method to `sdk/ts/src/client.ts`
4. Add types to `sdk/ts/src/types.ts`
5. Add tests to both runtime and SDK

## Troubleshooting

### Browser Fails to Launch

**Symptom:** `Browser launch failed` error

**Solutions:**
1. Verify Chrome/Chromium is installed:
   ```bash
   which chromium || which google-chrome || which chrome
   ```
2. Specify path explicitly:
   ```bash
   ./tivana --chrome-path /Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome
   ```
3. In containers, ensure sandbox is disabled (automatic in Tivana)
4. Check shared memory in Docker: `docker run --shm-size=2g`

### WebSocket Connection Refused

**Symptom:** `ECONNREFUSED` when connecting from SDK

**Solutions:**
1. Verify runtime is running:
   ```bash
   ps aux | grep tivana
   lsof -i :9876
   ```
2. Check firewall rules
3. Use `--host 0.0.0.0` for external access:
   ```bash
   ./tivana --host 0.0.0.0 --port 9876
   ```

### Elements Not Found After Navigation

**Symptom:** `click("e5")` fails with "element not found"

**Explanation:** Element IDs reset after each navigation. Always call `elements()` again after `navigate()`.

**Correct Pattern:**
```typescript
await client.navigate("https://example.com");
const elements = await client.elements();  // Get fresh IDs
await client.click(elements[0].id);
```

### Tests Hang or Timeout

**Symptom:** Integration tests hang indefinitely

**Solutions:**
1. Use `--test-threads=1` for browser tests:
   ```bash
   cargo test -- --test-threads=1
   ```
2. Increase timeout in test code
3. Check for browser zombie processes:
   ```bash
   pkill -f chromium
   ```

### SDK Type Errors After Runtime Changes

**Symptom:** TypeScript errors about mismatched types

**Solution:** Regenerate types from protocol:
1. Update `sdk/ts/src/types.ts` to match `runtime/src/protocol.rs`
2. Run `bun run typecheck`

### Memory Issues in Docker

**Symptom:** Browser crashes with OOM

**Solutions:**
1. Increase shared memory:
   ```bash
   docker run --shm-size=2g
   ```
2. Use headless mode:
   ```bash
   ./tivana --headless
   ```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `warn` | Log level (`error`, `warn`, `info`, `debug`, `trace`) |
| `CHROME_PATH` | auto-detect | Path to Chrome/Chromium executable |
| `TIVANA_PORT` | `9876` | WebSocket server port |
| `TIVANA_HOST` | `127.0.0.1` | WebSocket server host |

## CI/CD Notes

### GitHub Actions

```yaml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: oven-sh/setup-bun@v1
      
      # Chrome is pre-installed on ubuntu-latest
      - run: cd runtime && cargo test
      - run: cd runtime && cargo test --test browser_test -- --ignored
      - run: cd sdk/ts && bun install && bun run typecheck
```

### Docker Build

```dockerfile
FROM rust:1.75-bookworm as builder
WORKDIR /app
COPY runtime/ ./runtime/
RUN cd runtime && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y chromium && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/runtime/target/release/tivana /usr/local/bin/
EXPOSE 9876
CMD ["tivana", "--headless", "--host", "0.0.0.0"]
```

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make changes with tests
4. Run full test suite
5. Submit PR with description

See [CONTRIBUTING.md](../CONTRIBUTING.md) for detailed guidelines.
