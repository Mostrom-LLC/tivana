# Contributing to Tivana

Thank you for your interest in contributing to Tivana! This guide covers the process for contributing to the project.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/tivana.git
   cd tivana
   ```
3. **Create a branch** for your work:
   ```bash
   git checkout -b feat/my-feature
   ```

## Repository Structure

```
tivana/
├── runtime/          # Rust runtime (WebSocket server, CDP integration)
│   ├── src/          # Source code
│   └── tests/        # Integration tests
├── sdk/ts/           # TypeScript SDK (npm package)
│   ├── src/          # Source code
│   └── dist/         # Built output
├── extension/        # Chrome extension (optional browser transport)
├── examples/         # Working demos
├── docs/             # Architecture and protocol documentation
└── tasks/            # Planning documents
```

## Development Setup

### Runtime (Rust)

```bash
cd runtime
cargo build
cargo test
```

### SDK (TypeScript)

```bash
cd sdk/ts
bun install
bun run build
bun run typecheck
```

### Running Locally

```bash
# Terminal 1: Start runtime
cd runtime
cargo run -- --port 9876

# Terminal 2: Run an example
cd sdk/ts
bun run ../../examples/01-observe-and-explore.ts
```

## Commit Convention

We use conventional commits:

```
feat: add new perception capability
fix: resolve element visibility calculation
docs: update API reference
test: add browser integration tests
refactor: simplify extension routing
chore: update dependencies
```

Format: `type(scope): description`

- `feat` — new feature
- `fix` — bug fix
- `docs` — documentation only
- `test` — adding or updating tests
- `refactor` — code change that neither fixes a bug nor adds a feature
- `chore` — maintenance (deps, CI, tooling)

Optional scope examples: `runtime`, `sdk`, `extension`, `docs`, `examples`

## Pull Request Process

1. **Ensure tests pass** before submitting:
   ```bash
   cd runtime && cargo test
   cd sdk/ts && bun run typecheck
   ```

2. **Write clear PR descriptions** explaining what changed and why.

3. **Keep PRs focused** — one feature or fix per PR. Large PRs are harder to review.

4. **Update documentation** if your change affects the public API or behavior.

5. **Add examples** if your change introduces a new capability.

## What We're Looking For

### Good Contributions

- Bug fixes with test coverage
- New perception capabilities (element properties, page events)
- SDK ergonomics improvements
- Documentation improvements and typo fixes
- New examples demonstrating perception-first patterns
- Performance improvements with benchmarks

### Design Principles to Follow

- **Perception first** — Tivana provides semantic awareness, not scripted automation
- **No site-specific logic** — the runtime should work on any page
- **Agent decides** — Tivana provides eyes and hands, the agent provides the brain
- **Zero config by default** — things should work out of the box
- **Explicit over implicit** — observation lifecycle, connection state, errors

### What Doesn't Belong in the Runtime

- Hardcoded selectors or field matchers
- Site-specific automation scripts
- CAPTCHA solving as a first-class feature
- Stealth/anti-detection as the product story

## Reporting Issues

- **Bug reports** — include steps to reproduce, expected vs actual behavior, and runtime version
- **Feature requests** — describe the use case, not just the solution
- **Questions** — use GitHub Discussions if available, or open an issue tagged `question`

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you agree to uphold this code.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
