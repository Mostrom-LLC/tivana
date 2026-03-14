# Tivana TypeScript SDK

TypeScript client for the Tivana streaming browser perception protocol.

## Installation

```bash
npm install tivana
# or
bun add tivana
```

## Quick Start

```typescript
import { TivanaClient } from 'tivana';

const client = new TivanaClient();

// Connect to the runtime
await client.connect('ws://localhost:9222');

// Create a browser session
const sessionId = await client.createSession();

// Navigate and perceive
const state = await client.navigate('https://example.com');
console.log(`Page: ${state.title}`);
console.log(`Elements: ${state.elements.length}`);

// Take actions
await client.click('e3');
await client.type('hello world');

// Clean up
await client.closeSession();
client.disconnect();
```

## API

### TivanaClient

#### Constructor Options

```typescript
const client = new TivanaClient({
  timeout: 30000,        // Request timeout (ms)
  autoReconnect: false,  // Auto-reconnect on disconnect
  reconnectDelay: 1000,  // Reconnect delay (ms)
});
```

#### Methods

| Method | Description |
|--------|-------------|
| `connect(url)` | Connect to the Tivana runtime |
| `createSession()` | Create a new browser session |
| `navigate(url)` | Navigate to a URL |
| `pageState()` | Get current page state |
| `elements()` | Get all page elements |
| `click(target)` | Click an element |
| `type(text, target?)` | Type text |
| `scroll(target)` | Scroll element into view |
| `onMutation(callback)` | Register mutation listener |
| `closeSession()` | Close the session |
| `disconnect()` | Disconnect from runtime |

## Element Model

Each element includes:

- **Identity**: `id`
- **Semantic**: `role`, `label`, `value`, `text`
- **State**: `focused`, `enabled`, `visible`, `interactable`
- **Geometry**: `bounds`, `padding`, `margin`
- **Typography**: `font`, `textAlign`
- **Colors**: `background`
- **Borders**: `border`
- **Accessibility**: `contrastRatio`, `ariaAttributes`, etc.

## Running the Example

```bash
cd sdk/ts
npm install
bun run example.ts
```

## License

MIT
