# Tivana Atlas Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an Electron-based AI browser with a Gemini-powered autonomous agent loop, chat sidebar, tab management, and kill switch.

**Architecture:** Three TypeScript layers in Electron's main process — CDP wrapper around `webContents.debugger`, perception+action engine ported from Tivana's Rust runtime JS scripts, and a Gemini function-calling agent loop. React renderer for the Atlas-style UI (browser left, chat right). IPC bridges main↔renderer.

**Tech Stack:** Electron (electron-forge), React, TypeScript, Tailwind CSS, `@google/generative-ai`

**Spec:** `docs/superpowers/specs/2026-03-28-tivana-atlas-design.md`

**Source files to port from:**
- Element extraction JS: `runtime/src/perceive.rs:816-1026`
- Page state JS: `runtime/src/perceive.rs:770-787`
- Action CDP patterns: `runtime/src/act.rs`
- SDK types reference: `sdk/ts/src/types.ts`

---

### Task 1: Scaffold Electron App

**Files:**
- Create: `browser/package.json`
- Create: `browser/forge.config.ts`
- Create: `browser/tsconfig.json`
- Create: `browser/src/main/index.ts`
- Create: `browser/src/renderer/index.html`
- Create: `browser/src/renderer/index.tsx`
- Create: `browser/src/renderer/App.tsx`

- [ ] **Step 1: Initialize Electron project with electron-forge**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana
mkdir browser && cd browser
npx create-electron-app@latest . --template=vite-typescript
```

If the template creates files in a different structure, adjust to match the spec's `src/main/` and `src/renderer/` layout.

- [ ] **Step 2: Install additional dependencies**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npm install react react-dom tailwindcss @tailwindcss/vite @google/generative-ai
npm install -D @types/react @types/react-dom
```

- [ ] **Step 3: Configure Tailwind CSS**

Create `browser/tailwind.config.ts`:
```typescript
import type { Config } from "tailwindcss";

export default {
  content: ["./src/renderer/**/*.{tsx,ts,html}"],
  theme: { extend: {} },
  plugins: [],
} satisfies Config;
```

Create `browser/src/renderer/styles/globals.css`:
```css
@import "tailwindcss";
```

- [ ] **Step 4: Set up minimal main process with a BrowserWindow**

Replace `browser/src/main/index.ts` with:
```typescript
import { app, BrowserWindow } from "electron";
import path from "node:path";

let mainWindow: BrowserWindow | null = null;

function createWindow(): void {
  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 800,
    minHeight: 600,
    titleBarStyle: "hiddenInset",
    webPreferences: {
      preload: path.join(__dirname, "../renderer/preload.js"),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  if (MAIN_WINDOW_VITE_DEV_SERVER_URL) {
    mainWindow.loadURL(MAIN_WINDOW_VITE_DEV_SERVER_URL);
  } else {
    mainWindow.loadFile(
      path.join(__dirname, `../renderer/${MAIN_WINDOW_VITE_NAME}/index.html`)
    );
  }
}

app.whenReady().then(createWindow);
app.on("window-all-closed", () => {
  if (process.platform !== "darwin") app.quit();
});
```

Note: The exact Vite dev server constant names depend on the forge template. Adjust `MAIN_WINDOW_VITE_DEV_SERVER_URL` and `MAIN_WINDOW_VITE_NAME` to match what forge generates. Also update `forge.config.ts` to include the preload entry point — add a `preload` entry in the renderer config pointing to `src/preload.ts`. The preload path in BrowserWindow webPreferences must match where forge outputs the compiled preload file.

- [ ] **Step 5: Set up minimal React renderer**

Replace `browser/src/renderer/App.tsx`:
```tsx
export default function App() {
  return (
    <div className="flex h-screen bg-gray-950 text-gray-100">
      <div className="flex-1 flex items-center justify-center text-gray-500">
        Browser pane (coming soon)
      </div>
      <div className="w-80 border-l border-gray-800 flex items-center justify-center text-gray-500">
        Sidebar (coming soon)
      </div>
    </div>
  );
}
```

Replace `browser/src/renderer/index.tsx`:
```tsx
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles/globals.css";

const root = createRoot(document.getElementById("root")!);
root.render(<App />);
```

- [ ] **Step 6: Verify the app launches**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npm start
```

Expected: Electron window opens with a dark background, "Browser pane" on the left, "Sidebar" on the right.

- [ ] **Step 7: Commit**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana
git add browser/
git commit -m "feat(atlas): scaffold Electron app with React + Tailwind"
```

---

### Task 2: Types

**Files:**
- Create: `browser/src/main/types.ts`
- Create: `browser/src/renderer/types.ts`

- [ ] **Step 1: Define shared types for main process**

Create `browser/src/main/types.ts`:
```typescript
// --- Page & Element types (ported from Tivana SDK) ---

export interface BoundingBox {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Element {
  id: string;
  role: string;
  name?: string;
  value?: string;
  bounds?: BoundingBox;
  visible: boolean;
  interactable: boolean;
  focused: boolean;
  enabled: boolean;
  checked?: boolean;
  selected?: boolean;
  expanded?: boolean;
  required?: boolean;
  description?: string;
}

export interface PageState {
  url: string;
  title: string;
  scrollX: number;
  scrollY: number;
  viewportWidth: number;
  viewportHeight: number;
  documentWidth: number;
  documentHeight: number;
  focusedElementId: string | null;
}

// --- Agent types ---

export interface AgentMessage {
  text: string;
  type: "thinking" | "action" | "result" | "error";
}

export interface AgentStatus {
  running: boolean;
  waiting: boolean;
}

export interface AgentUsage {
  inputTokens: number;
  outputTokens: number;
  cost: number;
}

// --- Tab types ---

export interface Tab {
  id: string;
  title: string;
  url: string;
  active: boolean;
}

// --- Tool types ---

export interface ToolCall {
  name: string;
  args: Record<string, unknown>;
}

export interface ToolResult {
  name: string;
  success: boolean;
  result?: string;
  error?: string;
}

// --- File library types ---

export interface StoredFile {
  id: string;
  name: string;
  kind: "resume" | "cover_letter" | "transcript" | "portfolio" | "other";
  mimeType: string;
  path: string;
  reusable: boolean;
  summary?: string;
  extractedText?: string;
  createdAt: string;
  lastUsedAt?: string;
}

// --- LLM types ---

export interface LLMResponse {
  text: string;
  toolCalls: ToolCall[];
  usage: { inputTokens: number; outputTokens: number };
}

export interface LLMProvider {
  chat(
    messages: ChatMessage[],
    tools: ToolDefinition[]
  ): Promise<LLMResponse>;
  name: string;
  model: string;
}

export interface ChatMessage {
  role: "user" | "model" | "tool";
  text?: string;
  toolCalls?: ToolCall[];
  toolResults?: ToolResult[];
}

export interface ToolDefinition {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
}
```

- [ ] **Step 2: Define renderer IPC types**

Create `browser/src/renderer/types.ts`:
```typescript
// Re-export types used by the renderer from the IPC contract

export interface AgentMessage {
  text: string;
  type: "thinking" | "action" | "result" | "error";
}

export interface AgentStatus {
  running: boolean;
  waiting: boolean;
}

export interface AgentUsage {
  inputTokens: number;
  outputTokens: number;
  cost: number;
}

export interface Tab {
  id: string;
  title: string;
  url: string;
  active: boolean;
}

export interface StoredFile {
  id: string;
  name: string;
  kind: "resume" | "cover_letter" | "transcript" | "portfolio" | "other";
  mimeType: string;
  reusable: boolean;
  summary?: string;
  createdAt: string;
  lastUsedAt?: string;
}
```

- [ ] **Step 3: Commit**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana
git add browser/src/main/types.ts browser/src/renderer/types.ts
git commit -m "feat(atlas): add shared type definitions"
```

---

### Task 3: CDP Wrapper

**Files:**
- Create: `browser/src/main/cdp.ts`
- Create: `browser/src/main/__tests__/cdp.test.ts`

- [ ] **Step 1: Write the CDP wrapper test**

Create `browser/src/main/__tests__/cdp.test.ts`:
```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import { CDPClient } from "../cdp";

function createMockWebContents() {
  const listeners = new Map<string, Function>();
  return {
    debugger: {
      attach: vi.fn(),
      detach: vi.fn(),
      sendCommand: vi.fn().mockResolvedValue({}),
      on: vi.fn((event: string, cb: Function) => listeners.set(event, cb)),
      isAttached: vi.fn().mockReturnValue(false),
    },
    on: vi.fn(),
    _listeners: listeners,
  } as any;
}

describe("CDPClient", () => {
  let wc: any;
  let cdp: CDPClient;

  beforeEach(() => {
    wc = createMockWebContents();
    cdp = new CDPClient();
  });

  it("attaches debugger on connect", async () => {
    await cdp.attach(wc);
    expect(wc.debugger.attach).toHaveBeenCalledWith("1.3");
  });

  it("sends CDP commands", async () => {
    await cdp.attach(wc);
    wc.debugger.sendCommand.mockResolvedValue({ result: { value: 42 } });
    const result = await cdp.send(wc, "Runtime.evaluate", {
      expression: "1+1",
    });
    expect(wc.debugger.sendCommand).toHaveBeenCalledWith(
      "Runtime.evaluate",
      { expression: "1+1" }
    );
    expect(result).toEqual({ result: { value: 42 } });
  });

  it("detaches debugger", async () => {
    await cdp.attach(wc);
    cdp.detach(wc);
    expect(wc.debugger.detach).toHaveBeenCalled();
  });

  it("tracks attached state", async () => {
    expect(cdp.isAttached(wc)).toBe(false);
    await cdp.attach(wc);
    expect(cdp.isAttached(wc)).toBe(true);
    cdp.detach(wc);
    expect(cdp.isAttached(wc)).toBe(false);
  });
});
```

- [ ] **Step 2: Install vitest and run the test to verify it fails**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npm install -D vitest
npx vitest run src/main/__tests__/cdp.test.ts
```

Expected: FAIL — `CDPClient` not found.

- [ ] **Step 3: Implement the CDP wrapper**

Create `browser/src/main/cdp.ts`:
```typescript
import type { WebContents } from "electron";

export class CDPClient {
  private attached = new Set<WebContents>();
  private eventHandlers = new Map<
    WebContents,
    Map<string, Set<(params: any) => void>>
  >();

  async attach(webContents: WebContents): Promise<void> {
    if (this.attached.has(webContents)) return;

    webContents.debugger.attach("1.3");
    this.attached.add(webContents);

    // Listen for CDP events
    const handlers = new Map<string, Set<(params: any) => void>>();
    this.eventHandlers.set(webContents, handlers);

    webContents.debugger.on("message", (_event, method, params) => {
      const subs = handlers.get(method);
      if (subs) {
        for (const handler of subs) handler(params);
      }
    });

    webContents.debugger.on("detach", (_event, reason) => {
      this.attached.delete(webContents);
      this.eventHandlers.delete(webContents);
      console.warn(`CDP detached: ${reason}`);
    });

    // Enable required CDP domains
    await this.send(webContents, "Page.enable", {});
    await this.send(webContents, "Runtime.enable", {});
    await this.send(webContents, "DOM.enable", {});
  }

  detach(webContents: WebContents): void {
    if (!this.attached.has(webContents)) return;
    try {
      webContents.debugger.detach();
    } catch {
      // Already detached
    }
    this.attached.delete(webContents);
    this.eventHandlers.delete(webContents);
  }

  async send(
    webContents: WebContents,
    method: string,
    params: Record<string, unknown> = {}
  ): Promise<any> {
    return webContents.debugger.sendCommand(method, params);
  }

  on(
    webContents: WebContents,
    method: string,
    handler: (params: any) => void
  ): void {
    const handlers = this.eventHandlers.get(webContents);
    if (!handlers) return;
    if (!handlers.has(method)) handlers.set(method, new Set());
    handlers.get(method)!.add(handler);
  }

  off(
    webContents: WebContents,
    method: string,
    handler: (params: any) => void
  ): void {
    const handlers = this.eventHandlers.get(webContents);
    if (!handlers) return;
    handlers.get(method)?.delete(handler);
  }

  isAttached(webContents: WebContents): boolean {
    return this.attached.has(webContents);
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npx vitest run src/main/__tests__/cdp.test.ts
```

Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana
git add browser/src/main/cdp.ts browser/src/main/__tests__/cdp.test.ts
git commit -m "feat(atlas): add CDP wrapper around webContents.debugger"
```

---

### Task 4: Perception Engine

**Files:**
- Create: `browser/src/main/perception.ts`
- Create: `browser/src/main/__tests__/perception.test.ts`

**Reference:** Port the element extraction JS from `runtime/src/perceive.rs:816-1026` and page state JS from `runtime/src/perceive.rs:770-787`. The scripts are JavaScript strings embedded in Rust — extract them verbatim and wrap in TypeScript functions.

- [ ] **Step 1: Write perception tests**

Create `browser/src/main/__tests__/perception.test.ts`:
```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  buildPageStateScript,
  buildElementsScript,
  parsePageState,
  parseElements,
} from "../perception";

describe("Perception scripts", () => {
  it("buildPageStateScript returns a self-executing JS function", () => {
    const script = buildPageStateScript();
    expect(script).toContain("window.scrollX");
    expect(script).toContain("window.innerWidth");
    expect(script).toMatch(/^\(\(\) => \{/);
  });

  it("buildElementsScript returns JS that queries interactive elements", () => {
    const script = buildElementsScript();
    expect(script).toContain("querySelectorAll");
    expect(script).toContain("__tivana_element_map");
    expect(script).toContain("getBoundingClientRect");
  });

  it("parsePageState extracts fields from CDP result", () => {
    const cdpResult = {
      result: {
        type: "object",
        value: {
          scroll_x: 0,
          scroll_y: 100,
          viewport_width: 1280,
          viewport_height: 720,
          document_width: 1280,
          document_height: 5000,
          focused_element_id: null,
        },
      },
    };
    const state = parsePageState(cdpResult, "https://example.com", "Example");
    expect(state.url).toBe("https://example.com");
    expect(state.title).toBe("Example");
    expect(state.scrollY).toBe(100);
    expect(state.viewportWidth).toBe(1280);
  });

  it("parseElements converts CDP result to Element[]", () => {
    const cdpResult = {
      result: {
        type: "object",
        value: [
          {
            id: "e1",
            role: "button",
            name: "Submit",
            value: null,
            bounds: { x: 10, y: 20, width: 100, height: 40 },
            visible: true,
            interactable: true,
            focused: false,
            enabled: true,
          },
        ],
      },
    };
    const elements = parseElements(cdpResult);
    expect(elements).toHaveLength(1);
    expect(elements[0].id).toBe("e1");
    expect(elements[0].role).toBe("button");
    expect(elements[0].name).toBe("Submit");
    expect(elements[0].interactable).toBe(true);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npx vitest run src/main/__tests__/perception.test.ts
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement perception engine**

Create `browser/src/main/perception.ts`. Port the JavaScript from `runtime/src/perceive.rs:770-787` (page state) and `runtime/src/perceive.rs:816-1026` (elements).

```typescript
import type { WebContents } from "electron";
import type { CDPClient } from "./cdp";
import type { Element, PageState } from "./types";

/**
 * Page state extraction script.
 * Ported from runtime/src/perceive.rs:770-787.
 */
export function buildPageStateScript(): string {
  return `(() => {
    const focused = document.activeElement;
    return {
      scroll_x: window.scrollX || window.pageXOffset || 0,
      scroll_y: window.scrollY || window.pageYOffset || 0,
      viewport_width: window.innerWidth || document.documentElement.clientWidth || 0,
      viewport_height: window.innerHeight || document.documentElement.clientHeight || 0,
      document_width: Math.max(document.body?.scrollWidth || 0, document.documentElement?.scrollWidth || 0),
      document_height: Math.max(document.body?.scrollHeight || 0, document.documentElement?.scrollHeight || 0),
      focused_element_id: focused && focused !== document.body ? (focused.getAttribute('data-tivana-id') || null) : null
    };
  })()`;
}

/**
 * Element extraction script.
 * Ported from runtime/src/perceive.rs:816-1026.
 *
 * Collects all interactive elements with stable IDs, roles, labels,
 * values, bounds, visibility, and interactability. Uses a WeakMap
 * for ID stability across calls within a session.
 */
export function buildElementsScript(): string {
  return `(() => {
    if (!window.__tivana_element_map) window.__tivana_element_map = new WeakMap();
    if (!window.__tivana_element_counter) window.__tivana_element_counter = 0;

    const map = window.__tivana_element_map;
    const selectors = [
      'a[href]', 'button', 'input', 'select', 'textarea',
      '[role="button"]', '[role="link"]', '[role="checkbox"]', '[role="radio"]',
      '[role="menuitem"]', '[role="tab"]', '[role="option"]', '[role="switch"]',
      '[role="slider"]', '[role="spinbutton"]', '[role="searchbox"]', '[role="textbox"]',
      '[role="combobox"]', '[tabindex]:not([tabindex="-1"])', '[contenteditable="true"]'
    ];

    const seen = new Set();
    const elements = [];

    function getId(el) {
      if (map.has(el)) return map.get(el);
      window.__tivana_element_counter++;
      const id = 'e' + window.__tivana_element_counter;
      map.set(el, id);
      el.setAttribute('data-tivana-id', id);
      return id;
    }

    function getRole(el) {
      const role = el.getAttribute('role');
      if (role) return role;
      const tag = el.tagName.toLowerCase();
      if (tag === 'input') return el.type || 'text';
      if (tag === 'a') return 'link';
      if (tag === 'select') return 'combobox';
      if (tag === 'textarea') return 'textbox';
      return tag;
    }

    function getName(el) {
      // 1. aria-label
      const ariaLabel = el.getAttribute('aria-label');
      if (ariaLabel) return ariaLabel.trim();

      // 2. aria-labelledby
      const labelledBy = el.getAttribute('aria-labelledby');
      if (labelledBy) {
        const parts = labelledBy.split(/\\s+/).map(id => {
          const ref = document.getElementById(id);
          return ref ? ref.textContent?.trim() : '';
        }).filter(Boolean);
        if (parts.length) return parts.join(' ');
      }

      // 3. Associated label via for attribute
      if (el.id) {
        const label = document.querySelector('label[for="' + CSS.escape(el.id) + '"]');
        if (label) return label.textContent?.trim() || '';
      }

      // 4. Parent label wrapper
      const parentLabel = el.closest('label');
      if (parentLabel) {
        const clone = parentLabel.cloneNode(true);
        clone.querySelectorAll('input, select, textarea').forEach(c => c.remove());
        const text = clone.textContent?.trim();
        if (text) return text;
      }

      // 5. Title, placeholder, text content, value fallbacks
      if (el.title) return el.title.trim();
      if (el.placeholder) return el.placeholder.trim();

      const text = el.textContent?.trim();
      if (text && text.length < 200) return text;

      if (el.value && typeof el.value === 'string') return el.value.trim();
      return '';
    }

    for (const selector of selectors) {
      for (const el of document.querySelectorAll(selector)) {
        if (seen.has(el)) continue;
        seen.add(el);

        const style = window.getComputedStyle(el);
        const rect = el.getBoundingClientRect();

        const visible = (
          style.display !== 'none' &&
          style.visibility !== 'hidden' &&
          parseFloat(style.opacity) > 0 &&
          (rect.width > 0 || rect.height > 0)
        );

        if (!visible) continue;

        let interactable = !el.disabled;
        if (interactable && rect.width > 0 && rect.height > 0) {
          const cx = rect.x + rect.width / 2;
          const cy = rect.y + rect.height / 2;
          const topEl = document.elementFromPoint(cx, cy);
          if (topEl && topEl !== el && !el.contains(topEl) && !topEl.contains(el)) {
            interactable = false;
          }
        }

        const id = getId(el);

        elements.push({
          id,
          role: getRole(el),
          name: getName(el) || undefined,
          value: (el.value !== undefined && el.value !== '') ? String(el.value) : undefined,
          bounds: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
          visible: true,
          interactable,
          focused: el === document.activeElement,
          enabled: !el.disabled,
          checked: el.checked !== undefined ? el.checked : undefined,
          selected: el.selected !== undefined ? el.selected : undefined,
          expanded: el.getAttribute('aria-expanded') !== null ? el.getAttribute('aria-expanded') === 'true' : undefined,
          required: el.required || false,
          description: el.getAttribute('aria-description') || undefined,
        });
      }
    }

    return elements;
  })()`;
}

/**
 * Parse CDP Runtime.evaluate result into PageState.
 */
export function parsePageState(
  cdpResult: any,
  url: string,
  title: string
): PageState {
  const v = cdpResult.result?.value ?? cdpResult;
  return {
    url,
    title,
    scrollX: v.scroll_x ?? 0,
    scrollY: v.scroll_y ?? 0,
    viewportWidth: v.viewport_width ?? 0,
    viewportHeight: v.viewport_height ?? 0,
    documentWidth: v.document_width ?? 0,
    documentHeight: v.document_height ?? 0,
    focusedElementId: v.focused_element_id ?? null,
  };
}

/**
 * Parse CDP Runtime.evaluate result into Element[].
 */
export function parseElements(cdpResult: any): Element[] {
  const arr = cdpResult.result?.value ?? cdpResult;
  if (!Array.isArray(arr)) return [];
  return arr.map((e: any) => ({
    id: e.id,
    role: e.role,
    name: e.name,
    value: e.value,
    bounds: e.bounds,
    visible: e.visible ?? true,
    interactable: e.interactable ?? false,
    focused: e.focused ?? false,
    enabled: e.enabled ?? true,
    checked: e.checked,
    selected: e.selected,
    expanded: e.expanded,
    required: e.required ?? false,
    description: e.description,
  }));
}

/**
 * High-level perceive function: runs both scripts via CDP and returns
 * parsed PageState + Element[].
 */
export async function perceive(
  cdp: CDPClient,
  webContents: WebContents
): Promise<{ pageState: PageState; elements: Element[] }> {
  const url = webContents.getURL();
  const title = webContents.getTitle();

  const [pageResult, elementsResult] = await Promise.all([
    cdp.send(webContents, "Runtime.evaluate", {
      expression: buildPageStateScript(),
      returnByValue: true,
    }),
    cdp.send(webContents, "Runtime.evaluate", {
      expression: buildElementsScript(),
      returnByValue: true,
    }),
  ]);

  return {
    pageState: parsePageState(pageResult, url, title),
    elements: parseElements(elementsResult),
  };
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npx vitest run src/main/__tests__/perception.test.ts
```

Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana
git add browser/src/main/perception.ts browser/src/main/__tests__/perception.test.ts
git commit -m "feat(atlas): add perception engine ported from Tivana runtime"
```

---

### Task 5: Action Engine

**Files:**
- Create: `browser/src/main/actions.ts`
- Create: `browser/src/main/__tests__/actions.test.ts`

**Reference:** Port CDP action patterns from `runtime/src/act.rs`. Key patterns: click uses `Input.dispatchMouseEvent`, type uses `Input.dispatchKeyEvent`, fill uses `Runtime.evaluate` with native setters, navigate uses `Page.navigate`, scroll uses `Runtime.evaluate`, select uses `Runtime.evaluate`, screenshot uses `Page.captureScreenshot`.

- [ ] **Step 1: Write action engine tests**

Create `browser/src/main/__tests__/actions.test.ts`:
```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import { Actions } from "../actions";

function createMockCDP() {
  return {
    send: vi.fn().mockResolvedValue({}),
    isAttached: vi.fn().mockReturnValue(true),
  } as any;
}

function createMockWebContents() {
  return {
    getURL: vi.fn().mockReturnValue("https://example.com"),
  } as any;
}

describe("Actions", () => {
  let cdp: any;
  let wc: any;
  let actions: Actions;

  beforeEach(() => {
    cdp = createMockCDP();
    wc = createMockWebContents();
    actions = new Actions(cdp);
  });

  it("navigate sends Page.navigate", async () => {
    await actions.navigate(wc, "https://example.com");
    expect(cdp.send).toHaveBeenCalledWith(wc, "Page.navigate", {
      url: "https://example.com",
    });
  });

  it("click resolves element and dispatches mouse events", async () => {
    // Mock element bounds lookup
    cdp.send.mockImplementation((_wc: any, method: string, params: any) => {
      if (method === "Runtime.evaluate") {
        return {
          result: {
            type: "object",
            value: { x: 100, y: 200, width: 50, height: 30 },
          },
        };
      }
      return {};
    });

    await actions.click(wc, "e1");

    const mouseEvents = cdp.send.mock.calls.filter(
      (c: any[]) => c[1] === "Input.dispatchMouseEvent"
    );
    // At minimum: mouseMoved, mousePressed, mouseReleased
    expect(mouseEvents.length).toBeGreaterThanOrEqual(3);
  });

  it("type dispatches key events for each character", async () => {
    await actions.type(wc, "e1", "hi");

    const keyEvents = cdp.send.mock.calls.filter(
      (c: any[]) => c[1] === "Input.dispatchKeyEvent"
    );
    // Each char: keyDown + char + keyUp = 3 events × 2 chars = 6 minimum
    // Plus initial click to focus
    expect(keyEvents.length).toBeGreaterThanOrEqual(4);
  });

  it("fill uses Runtime.evaluate with native setter", async () => {
    await actions.fill(wc, "e1", "test value");

    const evalCalls = cdp.send.mock.calls.filter(
      (c: any[]) =>
        c[1] === "Runtime.evaluate" &&
        typeof c[2]?.expression === "string" &&
        c[2].expression.includes("data-tivana-id")
    );
    expect(evalCalls.length).toBeGreaterThan(0);
  });

  it("screenshot returns base64 data", async () => {
    cdp.send.mockResolvedValue({ data: "iVBORw0KGgo=" });
    const result = await actions.screenshot(wc);
    expect(cdp.send).toHaveBeenCalledWith(
      wc,
      "Page.captureScreenshot",
      expect.objectContaining({ format: "png" })
    );
    expect(result).toBe("iVBORw0KGgo=");
  });

  it("scroll evaluates JavaScript scroll", async () => {
    await actions.scroll(wc, "down", 300);
    const evalCalls = cdp.send.mock.calls.filter(
      (c: any[]) =>
        c[1] === "Runtime.evaluate" &&
        c[2]?.expression?.includes("scrollBy")
    );
    expect(evalCalls.length).toBe(1);
  });

  it("select sets value via JS and dispatches change", async () => {
    await actions.select(wc, "e1", "option2");
    const evalCalls = cdp.send.mock.calls.filter(
      (c: any[]) =>
        c[1] === "Runtime.evaluate" &&
        c[2]?.expression?.includes("data-tivana-id")
    );
    expect(evalCalls.length).toBeGreaterThan(0);
  });

  it("uploadFile uses DOM.describeNode and DOM.setFileInputFiles", async () => {
    cdp.send.mockImplementation((_wc: any, method: string) => {
      if (method === "Runtime.evaluate") {
        return { result: { objectId: "obj-123" } };
      }
      if (method === "DOM.describeNode") {
        return { node: { backendNodeId: 456 } };
      }
      return {};
    });

    await actions.uploadFile(wc, "e1", "/path/to/resume.pdf");

    expect(cdp.send).toHaveBeenCalledWith(wc, "DOM.setFileInputFiles", {
      files: ["/path/to/resume.pdf"],
      backendNodeId: 456,
    });
  });

  it("hover dispatches mouseMoved event", async () => {
    cdp.send.mockImplementation((_wc: any, method: string) => {
      if (method === "Runtime.evaluate") {
        return {
          result: {
            type: "object",
            value: { x: 100, y: 200, width: 50, height: 30 },
          },
        };
      }
      return {};
    });

    await actions.hover(wc, "e1");

    const mouseEvents = cdp.send.mock.calls.filter(
      (c: any[]) =>
        c[1] === "Input.dispatchMouseEvent" &&
        c[2]?.type === "mouseMoved"
    );
    expect(mouseEvents.length).toBeGreaterThanOrEqual(1);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npx vitest run src/main/__tests__/actions.test.ts
```

Expected: FAIL — `Actions` not found.

- [ ] **Step 3: Implement the action engine**

Create `browser/src/main/actions.ts`:
```typescript
import type { WebContents } from "electron";
import type { CDPClient } from "./cdp";

/**
 * Action engine for browser manipulation via CDP.
 * Ported from runtime/src/act.rs.
 */
export class Actions {
  constructor(private cdp: CDPClient) {}

  /**
   * Navigate to a URL. Waits for page load.
   * Ref: act.rs:649-684
   */
  async navigate(wc: WebContents, url: string): Promise<void> {
    await this.cdp.send(wc, "Page.navigate", { url });
    // Wait for load — poll readyState
    await this.waitForLoad(wc, 30_000);
  }

  /**
   * Click an element by tivana ID.
   * Ref: act.rs:847-924, browser.rs:174-219
   */
  async click(wc: WebContents, elementId: string): Promise<void> {
    const bounds = await this.resolveElementBounds(wc, elementId);
    const cx = bounds.x + bounds.width / 2 + (Math.random() * 4 - 2);
    const cy = bounds.y + bounds.height / 2 + (Math.random() * 4 - 2);

    // Move mouse to element
    await this.cdp.send(wc, "Input.dispatchMouseEvent", {
      type: "mouseMoved",
      x: cx,
      y: cy,
    });

    // Click
    await this.cdp.send(wc, "Input.dispatchMouseEvent", {
      type: "mousePressed",
      x: cx,
      y: cy,
      button: "left",
      clickCount: 1,
    });

    await this.delay(50 + Math.random() * 100);

    await this.cdp.send(wc, "Input.dispatchMouseEvent", {
      type: "mouseReleased",
      x: cx,
      y: cy,
      button: "left",
      clickCount: 1,
    });

    await this.delay(100);
  }

  /**
   * Type text into an element character by character.
   * Ref: act.rs:937-1005, browser.rs:226-271
   */
  async type(
    wc: WebContents,
    elementId: string,
    text: string
  ): Promise<void> {
    // Focus the element first
    await this.focus(wc, elementId);

    for (const char of text) {
      await this.cdp.send(wc, "Input.dispatchKeyEvent", {
        type: "keyDown",
        text: char,
        key: char,
      });
      await this.cdp.send(wc, "Input.dispatchKeyEvent", {
        type: "keyUp",
        key: char,
      });
      // Human-like delay: ~80ms mean with variation
      await this.delay(40 + Math.random() * 80);
    }
  }

  /**
   * Set a field's value instantly via JS injection.
   * Ref: act.rs:1013-1095
   */
  async fill(
    wc: WebContents,
    elementId: string,
    value: string
  ): Promise<void> {
    const escaped = value.replace(/\\/g, "\\\\").replace(/'/g, "\\'");
    await this.cdp.send(wc, "Runtime.evaluate", {
      expression: `(() => {
        const el = document.querySelector('[data-tivana-id="${elementId}"]');
        if (!el) throw new Error('Element not found: ${elementId}');
        el.focus();
        const nativeSetter = Object.getOwnPropertyDescriptor(
          HTMLInputElement.prototype, 'value'
        )?.set || Object.getOwnPropertyDescriptor(
          HTMLTextAreaElement.prototype, 'value'
        )?.set;
        if (nativeSetter) {
          nativeSetter.call(el, '${escaped}');
        } else {
          el.value = '${escaped}';
        }
        el.dispatchEvent(new Event('input', { bubbles: true }));
        el.dispatchEvent(new Event('change', { bubbles: true }));
      })()`,
    });
  }

  /**
   * Scroll the page.
   * Ref: act.rs:1126-1188
   */
  async scroll(
    wc: WebContents,
    direction: "up" | "down",
    amount: number = 300
  ): Promise<void> {
    const y = direction === "down" ? amount : -amount;
    await this.cdp.send(wc, "Runtime.evaluate", {
      expression: `window.scrollBy({ top: ${y}, behavior: 'smooth' })`,
    });
    await this.delay(300);
  }

  /**
   * Select a dropdown option.
   * Ref: act.rs:1258-1325
   */
  async select(
    wc: WebContents,
    elementId: string,
    value: string
  ): Promise<void> {
    const escaped = value.replace(/\\/g, "\\\\").replace(/'/g, "\\'");
    await this.cdp.send(wc, "Runtime.evaluate", {
      expression: `(() => {
        const el = document.querySelector('[data-tivana-id="${elementId}"]');
        if (!el) throw new Error('Element not found: ${elementId}');
        const nativeSetter = Object.getOwnPropertyDescriptor(
          HTMLSelectElement.prototype, 'value'
        )?.set;
        if (nativeSetter) {
          nativeSetter.call(el, '${escaped}');
        } else {
          el.value = '${escaped}';
        }
        el.dispatchEvent(new Event('change', { bubbles: true }));
      })()`,
    });
  }

  /**
   * Take a screenshot.
   * Ref: perceive.rs:1313-1376
   */
  async screenshot(wc: WebContents): Promise<string> {
    const result = await this.cdp.send(wc, "Page.captureScreenshot", {
      format: "png",
    });
    return result.data;
  }

  /**
   * Hover over an element.
   * Ref: act.rs:1191-1223
   */
  async hover(wc: WebContents, elementId: string): Promise<void> {
    const bounds = await this.resolveElementBounds(wc, elementId);
    const cx = bounds.x + bounds.width / 2;
    const cy = bounds.y + bounds.height / 2;

    await this.cdp.send(wc, "Input.dispatchMouseEvent", {
      type: "mouseMoved",
      x: cx,
      y: cy,
    });
  }

  /**
   * Upload a file into a file input element.
   * Ref: act.rs:721-800
   * CDP pattern: Runtime.evaluate → DOM.describeNode → DOM.setFileInputFiles
   */
  async uploadFile(
    wc: WebContents,
    elementId: string,
    filePath: string
  ): Promise<void> {
    // Find the element and get its RemoteObject
    const evalResult = await this.cdp.send(wc, "Runtime.evaluate", {
      expression: `document.querySelector('[data-tivana-id="${elementId}"]')`,
    });

    const objectId = evalResult.result?.objectId;
    if (!objectId) {
      throw new Error(`File input element not found: ${elementId}`);
    }

    // Get backendNodeId via DOM.describeNode
    const describeResult = await this.cdp.send(wc, "DOM.describeNode", {
      objectId,
    });

    const backendNodeId = describeResult.node?.backendNodeId;
    if (!backendNodeId) {
      throw new Error(`Could not resolve backend node for: ${elementId}`);
    }

    // Set the file on the input
    await this.cdp.send(wc, "DOM.setFileInputFiles", {
      files: [filePath],
      backendNodeId,
    });
  }

  /**
   * Wait for a specified number of seconds.
   */
  async wait(seconds: number): Promise<void> {
    await this.delay(seconds * 1000);
  }

  // --- Private helpers ---

  private async focus(wc: WebContents, elementId: string): Promise<void> {
    await this.cdp.send(wc, "Runtime.evaluate", {
      expression: `document.querySelector('[data-tivana-id="${elementId}"]')?.focus()`,
    });
  }

  private async resolveElementBounds(
    wc: WebContents,
    elementId: string
  ): Promise<{ x: number; y: number; width: number; height: number }> {
    const result = await this.cdp.send(wc, "Runtime.evaluate", {
      expression: `(() => {
        const el = document.querySelector('[data-tivana-id="${elementId}"]');
        if (!el) return null;
        const r = el.getBoundingClientRect();
        return { x: r.x, y: r.y, width: r.width, height: r.height };
      })()`,
      returnByValue: true,
    });

    const bounds = result.result?.value;
    if (!bounds) {
      throw new Error(`Element not found: ${elementId}`);
    }
    return bounds;
  }

  private async waitForLoad(
    wc: WebContents,
    timeoutMs: number
  ): Promise<void> {
    const start = Date.now();
    while (Date.now() - start < timeoutMs) {
      const result = await this.cdp.send(wc, "Runtime.evaluate", {
        expression: "document.readyState",
        returnByValue: true,
      });
      const state = result.result?.value;
      if (state === "interactive" || state === "complete") return;
      await this.delay(100);
    }
  }

  private delay(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npx vitest run src/main/__tests__/actions.test.ts
```

Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana
git add browser/src/main/actions.ts browser/src/main/__tests__/actions.test.ts
git commit -m "feat(atlas): add action engine ported from Tivana runtime"
```

---

### Task 6: Tab Management

**Files:**
- Create: `browser/src/main/tabs.ts`
- Create: `browser/src/main/__tests__/tabs.test.ts`

- [ ] **Step 1: Write tab management tests**

Create `browser/src/main/__tests__/tabs.test.ts`:
```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock Electron modules before importing TabManager
vi.mock("electron", () => ({
  WebContentsView: vi.fn().mockImplementation(() => ({
    webContents: {
      on: vi.fn(),
      loadURL: vi.fn().mockResolvedValue(undefined),
      close: vi.fn(),
      getURL: vi.fn().mockReturnValue("about:blank"),
    },
    setBounds: vi.fn(),
    setVisible: vi.fn(),
  })),
  BrowserWindow: vi.fn(),
}));

import { TabManager } from "../tabs";

function createMockBrowserWindow() {
  return {
    contentView: {
      addChildView: vi.fn(),
      removeChildView: vi.fn(),
      children: [],
    },
    getBounds: vi.fn().mockReturnValue({ width: 1400, height: 900 }),
    on: vi.fn(),
  } as any;
}

function createMockCDP() {
  return {
    attach: vi.fn().mockResolvedValue(undefined),
    detach: vi.fn(),
  } as any;
}

describe("TabManager", () => {
  let win: any;
  let cdp: any;
  let tabs: TabManager;

  beforeEach(() => {
    win = createMockBrowserWindow();
    cdp = createMockCDP();
    tabs = new TabManager(win, cdp);
  });

  it("starts with no tabs", () => {
    expect(tabs.getAllTabs()).toHaveLength(0);
    expect(tabs.getActiveTab()).toBeNull();
    expect(tabs.getActiveWebContents()).toBeNull();
  });

  it("has expected methods", () => {
    expect(typeof tabs.newTab).toBe("function");
    expect(typeof tabs.switchTo).toBe("function");
    expect(typeof tabs.closeTab).toBe("function");
    expect(typeof tabs.getActiveTab).toBe("function");
    expect(typeof tabs.getActiveWebContents).toBe("function");
    expect(typeof tabs.getAllTabs).toBe("function");
    expect(typeof tabs.setOnChange).toBe("function");
    expect(typeof tabs.relayout).toBe("function");
  });

  it("creates a new tab and sets it active", async () => {
    const id = await tabs.newTab("https://example.com");
    expect(id).toBeDefined();
    expect(tabs.getAllTabs()).toHaveLength(1);
    expect(tabs.getAllTabs()[0].active).toBe(true);
    expect(cdp.attach).toHaveBeenCalled();
  });

  it("notifies onChange when tabs change", async () => {
    const onChange = vi.fn();
    tabs.setOnChange(onChange);
    await tabs.newTab("https://example.com");
    expect(onChange).toHaveBeenCalled();
  });
});
```

Note: Tab management depends heavily on Electron's `WebContentsView` which can't be unit tested without Electron. The implementation below is designed to be testable via integration tests when the app is running.

- [ ] **Step 2: Implement tab manager**

Create `browser/src/main/tabs.ts`:
```typescript
import { WebContentsView, type BrowserWindow } from "electron";
import type { CDPClient } from "./cdp";
import type { Tab } from "./types";

export class TabManager {
  private tabs: Map<string, { view: WebContentsView; tab: Tab }> = new Map();
  private activeTabId: string | null = null;
  private nextId = 1;
  private onChange: ((tabs: Tab[]) => void) | null = null;

  constructor(
    private window: BrowserWindow,
    private cdp: CDPClient,
    private sidebarWidth: number = 320
  ) {}

  setOnChange(cb: (tabs: Tab[]) => void): void {
    this.onChange = cb;
  }

  async newTab(url?: string): Promise<string> {
    const id = `tab-${this.nextId++}`;
    const view = new WebContentsView();

    // Set up the view
    this.layoutView(view);

    this.window.contentView.addChildView(view);

    const tab: Tab = {
      id,
      title: "New Tab",
      url: url || "about:blank",
      active: false,
    };

    this.tabs.set(id, { view, tab });

    // Attach CDP debugger
    await this.cdp.attach(view.webContents);

    // Track title and URL changes
    view.webContents.on("page-title-updated", (_e, title) => {
      tab.title = title;
      this.notifyChange();
    });

    view.webContents.on("did-navigate", (_e, url) => {
      tab.url = url;
      this.notifyChange();
    });

    view.webContents.on(
      "did-navigate-in-page",
      (_e, url) => {
        tab.url = url;
        this.notifyChange();
      }
    );

    // Navigate
    if (url) {
      await view.webContents.loadURL(url);
    }

    // Switch to this tab
    this.switchTo(id);

    return id;
  }

  switchTo(id: string): void {
    const entry = this.tabs.get(id);
    if (!entry) return;

    // Hide current tab
    if (this.activeTabId && this.activeTabId !== id) {
      const current = this.tabs.get(this.activeTabId);
      if (current) {
        current.tab.active = false;
        current.view.setVisible(false);
      }
    }

    // Show new tab
    entry.tab.active = true;
    entry.view.setVisible(true);
    this.activeTabId = id;
    this.layoutView(entry.view);
    this.notifyChange();
  }

  closeTab(id: string): void {
    const entry = this.tabs.get(id);
    if (!entry) return;

    this.cdp.detach(entry.view.webContents);
    this.window.contentView.removeChildView(entry.view);
    entry.view.webContents.close();
    this.tabs.delete(id);

    // If we closed the active tab, switch to another
    if (this.activeTabId === id) {
      this.activeTabId = null;
      const remaining = Array.from(this.tabs.keys());
      if (remaining.length > 0) {
        this.switchTo(remaining[remaining.length - 1]);
      }
    }

    this.notifyChange();
  }

  getActiveTab(): { view: WebContentsView; tab: Tab } | null {
    if (!this.activeTabId) return null;
    return this.tabs.get(this.activeTabId) ?? null;
  }

  getActiveWebContents(): Electron.WebContents | null {
    return this.getActiveTab()?.view.webContents ?? null;
  }

  getAllTabs(): Tab[] {
    return Array.from(this.tabs.values()).map((e) => e.tab);
  }

  getTabById(id: string): { view: WebContentsView; tab: Tab } | undefined {
    return this.tabs.get(id);
  }

  /**
   * Layout the view to fill the window minus sidebar and chrome.
   * Top 80px reserved for tab bar + URL bar. Right sidebarWidth for sidebar.
   */
  layoutView(view: WebContentsView): void {
    const bounds = this.window.getBounds();
    const topOffset = 80; // tab bar + URL bar height
    const bottomOffset = 24; // status bar height
    view.setBounds({
      x: 0,
      y: topOffset,
      width: bounds.width - this.sidebarWidth,
      height: bounds.height - topOffset - bottomOffset,
    });
  }

  relayout(): void {
    for (const entry of this.tabs.values()) {
      if (entry.tab.active) {
        this.layoutView(entry.view);
      }
    }
  }

  private notifyChange(): void {
    this.onChange?.(this.getAllTabs());
  }
}
```

- [ ] **Step 3: Run tests**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npx vitest run src/main/__tests__/tabs.test.ts
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana
git add browser/src/main/tabs.ts browser/src/main/__tests__/tabs.test.ts
git commit -m "feat(atlas): add tab manager with WebContentsView lifecycle"
```

---

### Task 7: Gemini Provider

**Files:**
- Create: `browser/src/main/gemini.ts`
- Create: `browser/src/main/__tests__/gemini.test.ts`

- [ ] **Step 1: Write Gemini provider tests**

Create `browser/src/main/__tests__/gemini.test.ts`:
```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  TOOL_DEFINITIONS,
  SYSTEM_PROMPT,
  buildGeminiTools,
  calculateCost,
} from "../gemini";

describe("Gemini provider", () => {
  it("defines all expected tools", () => {
    const toolNames = TOOL_DEFINITIONS.map((t) => t.name);
    expect(toolNames).toContain("navigate");
    expect(toolNames).toContain("click");
    expect(toolNames).toContain("type");
    expect(toolNames).toContain("fill");
    expect(toolNames).toContain("scroll");
    expect(toolNames).toContain("select");
    expect(toolNames).toContain("hover");
    expect(toolNames).toContain("screenshot");
    expect(toolNames).toContain("wait");
    expect(toolNames).toContain("attach_file");
    expect(toolNames).toContain("upload_file");
    expect(toolNames).toContain("new_tab");
    expect(toolNames).toContain("switch_tab");
    expect(toolNames).toContain("close_tab");
    expect(toolNames).toContain("done");
    expect(toolNames).toContain("ask_user");
  });

  it("system prompt instructs autonomous behavior", () => {
    expect(SYSTEM_PROMPT).toContain("autonomous");
    expect(SYSTEM_PROMPT).toContain("without asking for confirmation");
    expect(SYSTEM_PROMPT).not.toContain("are you sure");
  });

  it("buildGeminiTools converts to Gemini SDK format", () => {
    const tools = buildGeminiTools();
    expect(tools).toHaveLength(1); // One tool declaration with functionDeclarations
    expect(tools[0].functionDeclarations.length).toBe(
      TOOL_DEFINITIONS.length
    );
  });

  it("calculateCost computes USD from token counts", () => {
    // Gemini 2.5 Pro: $1.25/1M input, $10/1M output
    const cost = calculateCost(1_000_000, 1_000_000, "gemini-2.5-pro");
    expect(cost).toBeCloseTo(11.25, 1);
  });

  it("calculateCost returns 0 for unknown model", () => {
    const cost = calculateCost(1000, 1000, "unknown-model");
    expect(cost).toBe(0);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npx vitest run src/main/__tests__/gemini.test.ts
```

Expected: FAIL.

- [ ] **Step 3: Implement Gemini provider**

Create `browser/src/main/gemini.ts`:
```typescript
import { GoogleGenerativeAI, type GenerativeModel } from "@google/generative-ai";
import type {
  LLMProvider,
  LLMResponse,
  ChatMessage,
  ToolDefinition,
  ToolCall,
} from "./types";

export const SYSTEM_PROMPT = `You are an autonomous browser agent. You execute tasks without asking for confirmation. Use the provided tools to interact with the page. When you receive a task, do it. Do not ask "are you sure?" — the user already decided.

You can see the page as a list of interactive elements with IDs, roles, labels, and values. Use the element IDs to target actions. If an element isn't in the list, it may not be visible — try scrolling.

You may also receive a list of reusable local files the user has stored in Atlas. Use attach_file() when you need the contents of one for context, and use upload_file() when the page expects a file upload such as a resume.

When the task is complete, call done() with a summary of what you accomplished.
Only use ask_user() when you genuinely need information you cannot find on the page (e.g., a password, a preference not stated in the task).`;

export const TOOL_DEFINITIONS: ToolDefinition[] = [
  {
    name: "navigate",
    description: "Go to a URL",
    parameters: {
      type: "object",
      properties: { url: { type: "string", description: "The URL to navigate to" } },
      required: ["url"],
    },
  },
  {
    name: "click",
    description: "Click an element by its ID",
    parameters: {
      type: "object",
      properties: { id: { type: "string", description: "Element ID (e.g., e1)" } },
      required: ["id"],
    },
  },
  {
    name: "type",
    description: "Type text into an element",
    parameters: {
      type: "object",
      properties: {
        id: { type: "string", description: "Element ID" },
        text: { type: "string", description: "Text to type" },
      },
      required: ["id", "text"],
    },
  },
  {
    name: "fill",
    description: "Set a field value instantly without typing animation",
    parameters: {
      type: "object",
      properties: {
        id: { type: "string", description: "Element ID" },
        value: { type: "string", description: "Value to set" },
      },
      required: ["id", "value"],
    },
  },
  {
    name: "scroll",
    description: "Scroll the page",
    parameters: {
      type: "object",
      properties: {
        direction: { type: "string", enum: ["up", "down"] },
        amount: { type: "number", description: "Pixels to scroll (default 300)" },
      },
      required: ["direction"],
    },
  },
  {
    name: "select",
    description: "Select a dropdown option",
    parameters: {
      type: "object",
      properties: {
        id: { type: "string", description: "Element ID of the select" },
        value: { type: "string", description: "Option value to select" },
      },
      required: ["id", "value"],
    },
  },
  {
    name: "hover",
    description: "Hover over an element",
    parameters: {
      type: "object",
      properties: { id: { type: "string", description: "Element ID" } },
      required: ["id"],
    },
  },
  {
    name: "screenshot",
    description: "Capture a screenshot of the current page",
    parameters: { type: "object", properties: {} },
  },
  {
    name: "wait",
    description: "Wait for async page updates",
    parameters: {
      type: "object",
      properties: {
        seconds: { type: "number", description: "Seconds to wait" },
      },
      required: ["seconds"],
    },
  },
  {
    name: "attach_file",
    description:
      "Attach a stored local file to the current task context for reasoning (e.g., read resume contents)",
    parameters: {
      type: "object",
      properties: {
        fileId: { type: "string", description: "ID of the stored file" },
      },
      required: ["fileId"],
    },
  },
  {
    name: "upload_file",
    description:
      "Upload a stored local file into a file input element on the page",
    parameters: {
      type: "object",
      properties: {
        id: { type: "string", description: "Element ID of the file input" },
        fileId: { type: "string", description: "ID of the stored file" },
      },
      required: ["id", "fileId"],
    },
  },
  {
    name: "new_tab",
    description: "Open a new browser tab",
    parameters: {
      type: "object",
      properties: {
        url: { type: "string", description: "URL to open (optional)" },
      },
    },
  },
  {
    name: "switch_tab",
    description: "Switch to a tab by its index (0-based)",
    parameters: {
      type: "object",
      properties: {
        index: { type: "number", description: "Tab index" },
      },
      required: ["index"],
    },
  },
  {
    name: "close_tab",
    description: "Close a tab by its index (0-based)",
    parameters: {
      type: "object",
      properties: {
        index: { type: "number", description: "Tab index" },
      },
      required: ["index"],
    },
  },
  {
    name: "done",
    description: "Signal that the task is complete",
    parameters: {
      type: "object",
      properties: {
        summary: { type: "string", description: "Summary of what was accomplished" },
      },
      required: ["summary"],
    },
  },
  {
    name: "ask_user",
    description:
      "Ask the user for information you cannot find on the page (passwords, preferences, etc.)",
    parameters: {
      type: "object",
      properties: {
        question: { type: "string", description: "Question for the user" },
      },
      required: ["question"],
    },
  },
];

/**
 * Convert our tool definitions into the Gemini SDK format.
 */
export function buildGeminiTools(): any[] {
  return [
    {
      functionDeclarations: TOOL_DEFINITIONS.map((t) => ({
        name: t.name,
        description: t.description,
        parameters: t.parameters,
      })),
    },
  ];
}

/**
 * Pricing per million tokens by model.
 */
const PRICING: Record<string, { input: number; output: number }> = {
  "gemini-2.5-pro": { input: 1.25, output: 10.0 },
  "gemini-2.5-flash": { input: 0.15, output: 0.6 },
  "gemini-2.0-flash": { input: 0.1, output: 0.4 },
};

export function calculateCost(
  inputTokens: number,
  outputTokens: number,
  model: string
): number {
  // Normalize model name — strip date suffixes
  const base = Object.keys(PRICING).find((k) => model.startsWith(k));
  if (!base) return 0;
  const p = PRICING[base];
  return (inputTokens / 1_000_000) * p.input + (outputTokens / 1_000_000) * p.output;
}

/**
 * Gemini LLM provider implementation.
 */
export class GeminiProvider implements LLMProvider {
  name = "gemini";
  model: string;
  private genModel: GenerativeModel;

  constructor(apiKey: string, model: string = "gemini-2.5-pro") {
    this.model = model;
    const genAI = new GoogleGenerativeAI(apiKey);
    this.genModel = genAI.getGenerativeModel({
      model,
      systemInstruction: SYSTEM_PROMPT,
    });
  }

  async chat(
    messages: ChatMessage[],
    _tools: ToolDefinition[]
  ): Promise<LLMResponse> {
    const contents = messages.map((m) => {
      if (m.role === "tool") {
        return {
          role: "function" as const,
          parts: (m.toolResults ?? []).map((r) => ({
            functionResponse: {
              name: r.name,
              response: { result: r.success ? r.result : r.error },
            },
          })),
        };
      }

      if (m.toolCalls && m.toolCalls.length > 0) {
        return {
          role: "model" as const,
          parts: [
            ...(m.text ? [{ text: m.text }] : []),
            ...m.toolCalls.map((tc) => ({
              functionCall: { name: tc.name, args: tc.args },
            })),
          ],
        };
      }

      return {
        role: m.role === "user" ? ("user" as const) : ("model" as const),
        parts: [{ text: m.text ?? "" }],
      };
    });

    const result = await this.genModel.generateContent({
      contents,
      tools: buildGeminiTools(),
    });

    const response = result.response;
    const parts = response.candidates?.[0]?.content?.parts ?? [];

    const text = parts
      .filter((p: any) => p.text)
      .map((p: any) => p.text)
      .join("");

    const toolCalls: ToolCall[] = parts
      .filter((p: any) => p.functionCall)
      .map((p: any) => ({
        name: p.functionCall.name,
        args: p.functionCall.args ?? {},
      }));

    const usage = response.usageMetadata;

    return {
      text,
      toolCalls,
      usage: {
        inputTokens: usage?.promptTokenCount ?? 0,
        outputTokens: usage?.candidatesTokenCount ?? 0,
      },
    };
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npx vitest run src/main/__tests__/gemini.test.ts
```

Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana
git add browser/src/main/gemini.ts browser/src/main/__tests__/gemini.test.ts
git commit -m "feat(atlas): add Gemini provider with tool definitions and cost tracking"
```

---

### Task 8: Agent Loop

**Files:**
- Create: `browser/src/main/agent.ts`
- Create: `browser/src/main/__tests__/agent.test.ts`

- [ ] **Step 1: Write agent loop tests**

Create `browser/src/main/__tests__/agent.test.ts`:
```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import { AgentLoop } from "../agent";

function createMockProvider() {
  return {
    name: "mock",
    model: "mock-model",
    chat: vi.fn().mockResolvedValue({
      text: "I'll click the button.",
      toolCalls: [{ name: "click", args: { id: "e1" } }],
      usage: { inputTokens: 100, outputTokens: 50 },
    }),
  };
}

function createMockActions() {
  return {
    navigate: vi.fn(),
    click: vi.fn(),
    type: vi.fn(),
    fill: vi.fn(),
    scroll: vi.fn(),
    select: vi.fn(),
    hover: vi.fn(),
    screenshot: vi.fn().mockResolvedValue("base64data"),
    wait: vi.fn(),
  } as any;
}

function createMockPerceive() {
  return vi.fn().mockResolvedValue({
    pageState: {
      url: "https://example.com",
      title: "Example",
      scrollX: 0,
      scrollY: 0,
      viewportWidth: 1280,
      viewportHeight: 720,
      documentWidth: 1280,
      documentHeight: 2000,
      focusedElementId: null,
    },
    elements: [
      { id: "e1", role: "button", name: "Submit", visible: true, interactable: true },
    ],
  });
}

describe("AgentLoop", () => {
  let provider: any;
  let actions: any;
  let perceiveFn: any;
  let agent: AgentLoop;
  let messages: any[];

  beforeEach(() => {
    provider = createMockProvider();
    actions = createMockActions();
    perceiveFn = createMockPerceive();
    messages = [];
    agent = new AgentLoop({
      provider,
      actions,
      perceive: perceiveFn,
      onMessage: (msg) => messages.push(msg),
      onStatus: vi.fn(),
      onUsage: vi.fn(),
      getActiveWebContents: vi.fn().mockReturnValue({}),
      tabActions: {
        newTab: vi.fn().mockResolvedValue("tab-1"),
        switchTab: vi.fn(),
        closeTab: vi.fn(),
        getAllTabs: vi.fn().mockReturnValue([]),
      },
      fileActions: {
        getReusableFiles: vi.fn().mockReturnValue([]),
        getFile: vi.fn().mockReturnValue(undefined),
        markUsed: vi.fn(),
      },
    });
  });

  it("calls perceive then provider.chat then executes tool calls", async () => {
    // Make provider return done on second call
    provider.chat
      .mockResolvedValueOnce({
        text: "Clicking button",
        toolCalls: [{ name: "click", args: { id: "e1" } }],
        usage: { inputTokens: 100, outputTokens: 50 },
      })
      .mockResolvedValueOnce({
        text: "Done!",
        toolCalls: [{ name: "done", args: { summary: "Clicked button" } }],
        usage: { inputTokens: 100, outputTokens: 30 },
      });

    await agent.run("Click the submit button");

    expect(perceiveFn).toHaveBeenCalled();
    expect(provider.chat).toHaveBeenCalled();
    expect(actions.click).toHaveBeenCalledWith({}, "e1");
  });

  it("stops when kill is called", async () => {
    // Provider never returns done — agent should still stop
    provider.chat.mockResolvedValue({
      text: "Working...",
      toolCalls: [{ name: "click", args: { id: "e1" } }],
      usage: { inputTokens: 100, outputTokens: 50 },
    });

    // Kill after first iteration
    const runPromise = agent.run("Do something");
    // Give it a tick to start
    await new Promise((r) => setTimeout(r, 10));
    agent.kill();

    await runPromise;
    // Agent stopped — it ran at least once
    expect(provider.chat).toHaveBeenCalled();
  });

  it("handles ask_user by pausing and waiting for response", async () => {
    provider.chat
      .mockResolvedValueOnce({
        text: "What's your email?",
        toolCalls: [{ name: "ask_user", args: { question: "What is your email?" } }],
        usage: { inputTokens: 100, outputTokens: 40 },
      })
      .mockResolvedValueOnce({
        text: "Done",
        toolCalls: [{ name: "done", args: { summary: "Complete" } }],
        usage: { inputTokens: 100, outputTokens: 30 },
      });

    const runPromise = agent.run("Apply to job");
    // Wait for ask_user to trigger
    await new Promise((r) => setTimeout(r, 50));
    // Provide the answer
    agent.respondToAskUser("user@example.com");

    await runPromise;
    expect(provider.chat).toHaveBeenCalledTimes(2);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npx vitest run src/main/__tests__/agent.test.ts
```

Expected: FAIL.

- [ ] **Step 3: Implement the agent loop**

Create `browser/src/main/agent.ts`:
```typescript
import type { WebContents } from "electron";
import type {
  LLMProvider,
  ChatMessage,
  ToolCall,
  ToolResult,
  AgentMessage,
  AgentStatus,
  AgentUsage,
  Element,
  PageState,
  StoredFile,
} from "./types";
import type { Actions } from "./actions";
import { TOOL_DEFINITIONS, calculateCost } from "./gemini";

export interface AgentConfig {
  provider: LLMProvider;
  actions: Actions;
  perceive: (wc: WebContents) => Promise<{ pageState: PageState; elements: Element[] }>;
  onMessage: (msg: AgentMessage) => void;
  onStatus: (status: AgentStatus) => void;
  onUsage: (usage: AgentUsage) => void;
  getActiveWebContents: () => WebContents | null;
  tabActions: {
    newTab: (url?: string) => Promise<string>;
    switchTab: (index: number) => void;
    closeTab: (index: number) => void;
    getAllTabs: () => { id: string; title: string; url: string; active: boolean }[];
  };
  fileActions: {
    getReusableFiles: () => StoredFile[];
    getFile: (id: string) => StoredFile | undefined;
    markUsed: (id: string) => void;
  };
}

export class AgentLoop {
  private running = false;
  private waiting = false;
  private askUserResolve: ((answer: string) => void) | null = null;
  private history: ChatMessage[] = [];
  private totalUsage: AgentUsage = { inputTokens: 0, outputTokens: 0, cost: 0 };
  private consecutiveMalformed = 0;
  private consecutiveNetworkErrors = 0;
  private rateLimitBackoff = 1000; // starts at 1s, doubles each time, max 30s
  private maxMalformed = 3;
  private maxNetworkRetries = 3;

  constructor(private config: AgentConfig) {}

  async run(goal: string): Promise<void> {
    this.running = true;
    this.waiting = false;
    this.history = [];
    this.totalUsage = { inputTokens: 0, outputTokens: 0, cost: 0 };
    this.consecutiveMalformed = 0;
    this.consecutiveNetworkErrors = 0;
    this.rateLimitBackoff = 1000;

    this.config.onStatus({ running: true, waiting: false });
    this.config.onMessage({ text: `Task: ${goal}`, type: "thinking" });

    // Initial user message with goal
    this.history.push({ role: "user", text: goal });

    await this.mainLoop();

    this.config.onStatus({ running: false, waiting: false });
  }

  /**
   * Shared loop logic used by both run() and resume().
   * Handles auth errors, rate limits, network errors, and malformed calls separately.
   */
  private async mainLoop(): Promise<void> {
    while (this.running) {
      try {
        await this.step();
        // Successful step resets network error counter
        this.consecutiveNetworkErrors = 0;
        this.rateLimitBackoff = 1000;
      } catch (err: any) {
        const msg = err?.message ?? String(err);

        // Auth errors — stop immediately
        if (msg.includes("401") || msg.includes("403") || msg.includes("API key")) {
          this.config.onMessage({
            text: `API key error: ${msg}. Check your API key in Settings.`,
            type: "error",
          });
          this.stop();
          break;
        }

        // Rate limits — exponential backoff: 1s, 2s, 4s, ..., max 30s
        if (msg.includes("429")) {
          this.config.onMessage({
            text: `Rate limited. Retrying in ${this.rateLimitBackoff / 1000}s...`,
            type: "error",
          });
          await this.delay(this.rateLimitBackoff);
          this.rateLimitBackoff = Math.min(this.rateLimitBackoff * 2, 30_000);
          continue;
        }

        // Network errors — separate counter, retry up to 3 times with 2s delay
        if (msg.includes("ECONNREFUSED") || msg.includes("ETIMEDOUT") || msg.includes("fetch failed") || msg.includes("network")) {
          this.consecutiveNetworkErrors++;
          if (this.consecutiveNetworkErrors >= this.maxNetworkRetries) {
            this.config.onMessage({
              text: "Network error persists after 3 retries. Stopping.",
              type: "error",
            });
            this.stop();
            break;
          }
          this.config.onMessage({ text: `Network error. Retrying (${this.consecutiveNetworkErrors}/${this.maxNetworkRetries})...`, type: "error" });
          await this.delay(2000);
          continue;
        }

        // Other errors — stop
        this.config.onMessage({ text: `Error: ${msg}`, type: "error" });
        this.stop();
        break;
      }
    }
  }

  private async step(): Promise<void> {
    const wc = this.config.getActiveWebContents();
    if (!wc) {
      this.config.onMessage({ text: "No active tab", type: "error" });
      this.stop();
      return;
    }

    // 1. Perceive
    const { pageState, elements } = await this.config.perceive(wc);

    // Build context message with current page state
    const contextMsg = this.formatPageContext(pageState, elements);

    // Add page context as a user message
    this.history.push({ role: "user", text: contextMsg });

    // 2. Call LLM
    const response = await this.config.provider.chat(
      this.history,
      TOOL_DEFINITIONS
    );

    // Track usage
    this.totalUsage.inputTokens += response.usage.inputTokens;
    this.totalUsage.outputTokens += response.usage.outputTokens;
    this.totalUsage.cost = calculateCost(
      this.totalUsage.inputTokens,
      this.totalUsage.outputTokens,
      this.config.provider.model
    );
    this.config.onUsage({ ...this.totalUsage });

    // Show agent thinking text
    if (response.text) {
      this.config.onMessage({ text: response.text, type: "thinking" });
    }

    // Add assistant response to history
    this.history.push({
      role: "model",
      text: response.text,
      toolCalls: response.toolCalls,
    });

    // 3. Execute tool calls
    if (response.toolCalls.length === 0) {
      this.consecutiveMalformed++;
      if (this.consecutiveMalformed >= this.maxMalformed) {
        this.config.onMessage({
          text: "Agent produced no actions 3 times in a row. Stopping.",
          type: "error",
        });
        this.stop();
      }
      // Feed error back to model so it can self-correct
      this.history.push({
        role: "user",
        text: "You must use a tool to take action. Do not respond with text only.",
      });
      return;
    }

    this.consecutiveMalformed = 0; // Reset on successful tool calls
    const results: ToolResult[] = [];

    for (const toolCall of response.toolCalls) {
      if (!this.running) break;

      const result = await this.executeTool(toolCall, elements);
      results.push(result);

      if (toolCall.name === "done") {
        this.config.onMessage({
          text: `Complete: ${toolCall.args.summary}`,
          type: "result",
        });
        this.stop();
        return;
      }

      if (toolCall.name === "ask_user") {
        // Pause and wait for user input
        this.waiting = true;
        this.config.onStatus({ running: true, waiting: true });
        this.config.onMessage({
          text: String(toolCall.args.question),
          type: "action",
        });

        const answer = await new Promise<string>((resolve) => {
          this.askUserResolve = resolve;
        });
        this.askUserResolve = null;
        this.waiting = false;
        this.config.onStatus({ running: true, waiting: false });

        // Add user response
        results.push({
          name: "ask_user",
          success: true,
          result: answer,
        });
      }
    }

    // Add tool results to history
    this.history.push({ role: "tool", toolResults: results });

    // Check kill flag
    if (!this.running) return;
  }

  private async executeTool(
    toolCall: ToolCall,
    currentElements: Element[]
  ): Promise<ToolResult> {
    const { name, args } = toolCall;
    const wc = this.config.getActiveWebContents();

    this.config.onMessage({
      text: `${name}(${JSON.stringify(args)})`,
      type: "action",
    });

    try {
      // Validate element ID if applicable
      if (
        args.id &&
        ["click", "type", "fill", "select", "hover"].includes(name)
      ) {
        const found = currentElements.find((e) => e.id === args.id);
        if (!found) {
          return {
            name,
            success: false,
            error: `Element ${args.id} not found. Available IDs: ${currentElements
              .slice(0, 20)
              .map((e) => e.id)
              .join(", ")}`,
          };
        }
      }

      if (!wc && name !== "done" && name !== "ask_user") {
        return { name, success: false, error: "No active tab" };
      }

      switch (name) {
        case "navigate":
          await this.config.actions.navigate(wc!, String(args.url));
          return { name, success: true, result: `Navigated to ${args.url}` };

        case "click":
          await this.config.actions.click(wc!, String(args.id));
          return { name, success: true, result: `Clicked ${args.id}` };

        case "type":
          await this.config.actions.type(wc!, String(args.id), String(args.text));
          return { name, success: true, result: `Typed "${args.text}" into ${args.id}` };

        case "fill":
          await this.config.actions.fill(wc!, String(args.id), String(args.value));
          return { name, success: true, result: `Filled ${args.id} with "${args.value}"` };

        case "scroll":
          await this.config.actions.scroll(
            wc!,
            args.direction as "up" | "down",
            (args.amount as number) ?? 300
          );
          return { name, success: true, result: `Scrolled ${args.direction}` };

        case "select":
          await this.config.actions.select(wc!, String(args.id), String(args.value));
          return { name, success: true, result: `Selected "${args.value}" in ${args.id}` };

        case "hover":
          await this.config.actions.hover(wc!, String(args.id));
          return { name, success: true, result: `Hovered over ${args.id}` };

        case "screenshot":
          await this.config.actions.screenshot(wc!);
          return { name, success: true, result: "Screenshot captured" };

        case "wait":
          await this.config.actions.wait(args.seconds as number);
          return { name, success: true, result: `Waited ${args.seconds}s` };

        case "new_tab": {
          const tabId = await this.config.tabActions.newTab(args.url as string);
          return { name, success: true, result: `Opened tab ${tabId}` };
        }

        case "switch_tab":
          this.config.tabActions.switchTab(args.index as number);
          return { name, success: true, result: `Switched to tab ${args.index}` };

        case "close_tab":
          this.config.tabActions.closeTab(args.index as number);
          return { name, success: true, result: `Closed tab ${args.index}` };

        case "attach_file": {
          const file = this.config.fileActions.getFile(String(args.fileId));
          if (!file) {
            return { name, success: false, error: `File not found: ${args.fileId}` };
          }
          this.config.fileActions.markUsed(file.id);
          const content = file.extractedText || `[Binary file: ${file.name} (${file.mimeType})]`;
          return {
            name,
            success: true,
            result: `File "${file.name}" (${file.kind}):\n${content}`,
          };
        }

        case "upload_file": {
          const file = this.config.fileActions.getFile(String(args.fileId));
          if (!file) {
            return { name, success: false, error: `File not found: ${args.fileId}` };
          }
          await this.config.actions.uploadFile(wc!, String(args.id), file.path);
          this.config.fileActions.markUsed(file.id);
          return { name, success: true, result: `Uploaded "${file.name}" to ${args.id}` };
        }

        case "done":
          return { name, success: true, result: String(args.summary) };

        case "ask_user":
          return { name, success: true, result: "Waiting for user response" };

        default:
          return { name, success: false, error: `Unknown tool: ${name}` };
      }
    } catch (err: any) {
      return { name, success: false, error: err?.message ?? String(err) };
    }
  }

  private formatPageContext(pageState: PageState, elements: Element[]): string {
    const elementLines = elements
      .filter((e) => e.visible)
      .map((e) => {
        let line = `[${e.id}] ${e.role}`;
        if (e.name) line += ` "${e.name}"`;
        if (e.value) line += ` value="${e.value}"`;
        if (!e.interactable) line += " (not interactable)";
        if (e.focused) line += " (focused)";
        if (e.checked) line += " (checked)";
        if (e.required) line += " (required)";
        return line;
      })
      .join("\n");

    // Include tab list so agent knows about open tabs
    const tabs = this.config.tabActions.getAllTabs();
    const tabLines = tabs
      .map((t, i) => `  [${i}] ${t.url} ${t.active ? "(active)" : ""}`)
      .join("\n");

    // Include reusable files inventory
    const files = this.config.fileActions.getReusableFiles();
    const fileLines = files.length > 0
      ? files.map((f) => `  [${f.id}] ${f.kind}: "${f.name}" (${f.mimeType})${f.summary ? ` — ${f.summary}` : ""}`).join("\n")
      : "(no stored files)";

    return `Current page: ${pageState.url}
Title: ${pageState.title}
Viewport: ${pageState.viewportWidth}x${pageState.viewportHeight}
Scroll: (${pageState.scrollX}, ${pageState.scrollY}) of (${pageState.documentWidth}, ${pageState.documentHeight})

Open tabs:
${tabLines}

Stored files:
${fileLines}

Interactive elements:
${elementLines || "(no interactive elements found)"}`;
  }

  kill(): void {
    this.running = false;
    if (this.askUserResolve) {
      this.askUserResolve("");
      this.askUserResolve = null;
    }
  }

  async resume(goal?: string): Promise<void> {
    if (goal) {
      return this.run(goal);
    }
    // Resume from current state — re-run with existing history context
    this.running = true;
    this.consecutiveNetworkErrors = 0;
    this.rateLimitBackoff = 1000;
    this.config.onStatus({ running: true, waiting: false });
    await this.mainLoop();
    this.config.onStatus({ running: false, waiting: false });
  }

  respondToAskUser(answer: string): void {
    if (this.askUserResolve) {
      this.askUserResolve(answer);
    }
  }

  isRunning(): boolean {
    return this.running;
  }

  isWaiting(): boolean {
    return this.waiting;
  }

  private stop(): void {
    this.running = false;
  }

  private delay(ms: number): Promise<void> {
    return new Promise((r) => setTimeout(r, ms));
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npx vitest run src/main/__tests__/agent.test.ts
```

Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana
git add browser/src/main/agent.ts browser/src/main/__tests__/agent.test.ts
git commit -m "feat(atlas): add autonomous agent loop with kill switch"
```

---

### Task 9: IPC Handlers

**Files:**
- Create: `browser/src/main/ipc.ts`
- Create: `browser/src/preload.ts`

- [ ] **Step 1: Create preload script for context bridge**

Create `browser/src/preload.ts`:
```typescript
import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("atlas", {
  // Renderer → Main
  startTask: (goal: string) => ipcRenderer.send("task:start", { goal }),
  killAgent: () => ipcRenderer.send("task:kill"),
  resumeAgent: () => ipcRenderer.send("task:resume"),
  respondToAgent: (text: string) =>
    ipcRenderer.send("user:response", { text }),

  // Tab actions
  switchTab: (id: string) => ipcRenderer.send("tab:switch", { id }),
  newTab: () => ipcRenderer.send("tab:new"),
  closeTab: (id: string) => ipcRenderer.send("tab:close", { id }),

  // Navigation
  navigateTo: (url: string) => ipcRenderer.send("navigation:go", { url }),
  goBack: () => ipcRenderer.send("navigation:back"),
  goForward: () => ipcRenderer.send("navigation:forward"),
  refresh: () => ipcRenderer.send("navigation:refresh"),

  // Settings
  openSettings: () => ipcRenderer.send("settings:open"),
  saveSettings: (settings: { apiKey?: string; model?: string }) =>
    ipcRenderer.send("settings:save", settings),

  // File library
  addFiles: (paths: string[], reusable?: boolean, kind?: string) =>
    ipcRenderer.send("files:add", { paths, reusable, kind }),
  removeFile: (id: string) => ipcRenderer.send("files:remove", { id }),
  onFilesUpdate: (cb: (files: any[]) => void) => {
    const listener = (_event: any, files: any[]) => cb(files);
    ipcRenderer.on("files:update", listener);
    return () => ipcRenderer.removeListener("files:update", listener);
  },

  // Main → Renderer (listeners)
  onAgentMessage: (cb: (msg: any) => void) => {
    const listener = (_event: any, msg: any) => cb(msg);
    ipcRenderer.on("agent:message", listener);
    return () => ipcRenderer.removeListener("agent:message", listener);
  },
  onAgentStatus: (cb: (status: any) => void) => {
    const listener = (_event: any, status: any) => cb(status);
    ipcRenderer.on("agent:status", listener);
    return () => ipcRenderer.removeListener("agent:status", listener);
  },
  onAgentUsage: (cb: (usage: any) => void) => {
    const listener = (_event: any, usage: any) => cb(usage);
    ipcRenderer.on("agent:usage", listener);
    return () => ipcRenderer.removeListener("agent:usage", listener);
  },
  onTabsUpdate: (cb: (tabs: any[]) => void) => {
    const listener = (_event: any, tabs: any[]) => cb(tabs);
    ipcRenderer.on("tabs:update", listener);
    return () => ipcRenderer.removeListener("tabs:update", listener);
  },
  onNavigationUrl: (cb: (url: string) => void) => {
    const listener = (_event: any, data: { url: string }) => cb(data.url);
    ipcRenderer.on("navigation:url", listener);
    return () => ipcRenderer.removeListener("navigation:url", listener);
  },
});
```

- [ ] **Step 2: Create IPC handler setup for main process**

Create `browser/src/main/ipc.ts`:
```typescript
import { ipcMain, safeStorage, type BrowserWindow } from "electron";
import type { TabManager } from "./tabs";
import type { AgentLoop } from "./agent";
import type { AgentMessage, AgentStatus, AgentUsage } from "./types";

export interface IPCDeps {
  window: BrowserWindow;
  tabManager: TabManager;
  getAgent: () => AgentLoop | null;
  createAgent: (apiKey: string, model: string) => AgentLoop;
  getSettings: () => { apiKey: string | null; model: string };
  saveSettings: (settings: { apiKey?: string; model?: string }) => void;
}

/**
 * Wire up all IPC handlers between main and renderer.
 */
export function setupIPC(deps: IPCDeps): void {
  const { window: win, tabManager } = deps;

  // --- Renderer → Main ---

  ipcMain.on("task:start", async (_event, { goal }: { goal: string }) => {
    const settings = deps.getSettings();
    if (!settings.apiKey) {
      sendToRenderer(win, "agent:message", {
        text: "No API key configured. Open Settings to add your Gemini API key.",
        type: "error",
      } satisfies AgentMessage);
      return;
    }

    let agent = deps.getAgent();
    if (agent?.isRunning()) {
      agent.kill();
    }

    agent = deps.createAgent(settings.apiKey, settings.model);
    agent.run(goal);
  });

  ipcMain.on("task:kill", () => {
    deps.getAgent()?.kill();
  });

  ipcMain.on("task:resume", () => {
    deps.getAgent()?.resume();
  });

  ipcMain.on("user:response", (_event, { text }: { text: string }) => {
    deps.getAgent()?.respondToAskUser(text);
  });

  // Tab actions
  ipcMain.on("tab:switch", (_event, { id }: { id: string }) => {
    tabManager.switchTo(id);
    const tab = tabManager.getActiveTab();
    if (tab) {
      sendToRenderer(win, "navigation:url", { url: tab.tab.url });
    }
  });

  ipcMain.on("tab:new", async () => {
    await tabManager.newTab("about:blank");
  });

  ipcMain.on("tab:close", (_event, { id }: { id: string }) => {
    tabManager.closeTab(id);
  });

  // Navigation
  ipcMain.on("navigation:go", (_event, { url }: { url: string }) => {
    const wc = tabManager.getActiveWebContents();
    if (wc) {
      // Add protocol if missing
      const fullUrl = url.match(/^https?:\/\//) ? url : `https://${url}`;
      wc.loadURL(fullUrl);
    }
  });

  ipcMain.on("navigation:back", () => {
    tabManager.getActiveWebContents()?.goBack();
  });

  ipcMain.on("navigation:forward", () => {
    tabManager.getActiveWebContents()?.goForward();
  });

  ipcMain.on("navigation:refresh", () => {
    tabManager.getActiveWebContents()?.reload();
  });

  // Settings
  ipcMain.on("settings:open", () => {
    sendToRenderer(win, "settings:open", {});
  });

  ipcMain.on(
    "settings:save",
    (_event, settings: { apiKey?: string; model?: string }) => {
      deps.saveSettings(settings);
    }
  );

  // --- Tab change notifications ---
  tabManager.setOnChange((tabs) => {
    sendToRenderer(win, "tabs:update", tabs);
    const active = tabs.find((t) => t.active);
    if (active) {
      sendToRenderer(win, "navigation:url", { url: active.url });
    }
  });
}

/**
 * Helper: send message from main to renderer.
 */
export function sendToRenderer(
  win: BrowserWindow,
  channel: string,
  data: unknown
): void {
  if (!win.isDestroyed()) {
    win.webContents.send(channel, data);
  }
}
```

- [ ] **Step 3: Commit**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana
git add browser/src/main/ipc.ts browser/src/preload.ts
git commit -m "feat(atlas): add IPC bridge and preload script"
```

---

### Task 10: React UI — Sidebar, TabBar, UrlBar, StatusBar

**Files:**
- Create: `browser/src/renderer/components/Sidebar.tsx`
- Create: `browser/src/renderer/components/TabBar.tsx`
- Create: `browser/src/renderer/components/UrlBar.tsx`
- Create: `browser/src/renderer/components/StatusBar.tsx`
- Modify: `browser/src/renderer/App.tsx`
- Create: `browser/src/renderer/hooks/useAtlas.ts`

- [ ] **Step 1: Add TypeScript declaration for the preload API**

Create `browser/src/renderer/atlas.d.ts`:
```typescript
interface AtlasAPI {
  startTask: (goal: string) => void;
  killAgent: () => void;
  resumeAgent: () => void;
  respondToAgent: (text: string) => void;
  switchTab: (id: string) => void;
  newTab: () => void;
  closeTab: (id: string) => void;
  navigateTo: (url: string) => void;
  goBack: () => void;
  goForward: () => void;
  refresh: () => void;
  openSettings: () => void;
  saveSettings: (settings: { apiKey?: string; model?: string }) => void;
  onAgentMessage: (cb: (msg: import("./types").AgentMessage) => void) => () => void;
  onAgentStatus: (cb: (status: import("./types").AgentStatus) => void) => () => void;
  onAgentUsage: (cb: (usage: import("./types").AgentUsage) => void) => () => void;
  onTabsUpdate: (cb: (tabs: import("./types").Tab[]) => void) => () => void;
  onNavigationUrl: (cb: (url: string) => void) => () => void;
}

declare global {
  interface Window {
    atlas: AtlasAPI;
  }
}

export {};
```

- [ ] **Step 2: Create the useAtlas hook**

Create `browser/src/renderer/hooks/useAtlas.ts`:
```typescript
import { useState, useEffect, useCallback } from "react";
import type { AgentMessage, AgentStatus, AgentUsage, Tab } from "../types";

export function useAtlas() {
  const [messages, setMessages] = useState<AgentMessage[]>([]);
  const [status, setStatus] = useState<AgentStatus>({ running: false, waiting: false });
  const [usage, setUsage] = useState<AgentUsage>({ inputTokens: 0, outputTokens: 0, cost: 0 });
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [url, setUrl] = useState("");

  useEffect(() => {
    const unsubs = [
      window.atlas.onAgentMessage((msg) =>
        setMessages((prev) => [...prev, msg])
      ),
      window.atlas.onAgentStatus(setStatus),
      window.atlas.onAgentUsage(setUsage),
      window.atlas.onTabsUpdate(setTabs),
      window.atlas.onNavigationUrl(setUrl),
    ];
    return () => unsubs.forEach((u) => u());
  }, []);

  const startTask = useCallback((goal: string) => {
    setMessages([]);
    window.atlas.startTask(goal);
  }, []);

  return {
    messages,
    status,
    usage,
    tabs,
    url,
    startTask,
    kill: window.atlas.killAgent,
    resume: window.atlas.resumeAgent,
    respond: window.atlas.respondToAgent,
    switchTab: window.atlas.switchTab,
    newTab: window.atlas.newTab,
    closeTab: window.atlas.closeTab,
    navigateTo: window.atlas.navigateTo,
    goBack: window.atlas.goBack,
    goForward: window.atlas.goForward,
    refresh: window.atlas.refresh,
  };
}
```

- [ ] **Step 3: Create TabBar component**

Create `browser/src/renderer/components/TabBar.tsx`:
```tsx
import type { Tab } from "../types";

interface Props {
  tabs: Tab[];
  onSwitch: (id: string) => void;
  onNew: () => void;
  onClose: (id: string) => void;
}

export function TabBar({ tabs, onSwitch, onNew, onClose }: Props) {
  return (
    <div className="flex items-center h-10 bg-gray-900 border-b border-gray-800 px-2 gap-1 select-none">
      {tabs.map((tab) => (
        <div
          key={tab.id}
          onClick={() => onSwitch(tab.id)}
          className={`flex items-center gap-2 px-3 py-1.5 rounded-md text-sm cursor-pointer max-w-[200px] ${
            tab.active
              ? "bg-gray-800 text-gray-100"
              : "text-gray-400 hover:bg-gray-850 hover:text-gray-200"
          }`}
        >
          <span className="truncate">{tab.title || "New Tab"}</span>
          <button
            onClick={(e) => {
              e.stopPropagation();
              onClose(tab.id);
            }}
            className="text-gray-500 hover:text-gray-200 text-xs ml-1"
          >
            &times;
          </button>
        </div>
      ))}
      <button
        onClick={onNew}
        className="text-gray-500 hover:text-gray-200 px-2 py-1 text-lg"
      >
        +
      </button>
    </div>
  );
}
```

- [ ] **Step 4: Create UrlBar component**

Create `browser/src/renderer/components/UrlBar.tsx`:
```tsx
import { useState, useEffect, type KeyboardEvent } from "react";

interface Props {
  url: string;
  onNavigate: (url: string) => void;
  onBack: () => void;
  onForward: () => void;
  onRefresh: () => void;
}

export function UrlBar({ url, onNavigate, onBack, onForward, onRefresh }: Props) {
  const [input, setInput] = useState(url);

  useEffect(() => {
    setInput(url);
  }, [url]);

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      onNavigate(input);
    }
  };

  return (
    <div className="flex items-center h-10 bg-gray-900 border-b border-gray-800 px-2 gap-2">
      <button
        onClick={onBack}
        className="text-gray-400 hover:text-gray-200 px-1"
      >
        &larr;
      </button>
      <button
        onClick={onForward}
        className="text-gray-400 hover:text-gray-200 px-1"
      >
        &rarr;
      </button>
      <input
        type="text"
        value={input}
        onChange={(e) => setInput(e.target.value)}
        onKeyDown={handleKeyDown}
        className="flex-1 bg-gray-800 text-gray-200 rounded-md px-3 py-1.5 text-sm outline-none focus:ring-1 focus:ring-blue-500"
        placeholder="Enter URL..."
      />
      <button
        onClick={onRefresh}
        className="text-gray-400 hover:text-gray-200 px-1"
      >
        &#x21bb;
      </button>
    </div>
  );
}
```

- [ ] **Step 5: Create Sidebar component**

Create `browser/src/renderer/components/Sidebar.tsx`:
```tsx
import { useState, useRef, useEffect, type KeyboardEvent } from "react";
import type { AgentMessage, AgentStatus } from "../types";

interface Props {
  messages: AgentMessage[];
  status: AgentStatus;
  onStartTask: (goal: string) => void;
  onKill: () => void;
  onResume: () => void;
  onRespond: (text: string) => void;
}

const TYPE_COLORS: Record<AgentMessage["type"], string> = {
  thinking: "text-gray-400",
  action: "text-blue-400",
  result: "text-green-400",
  error: "text-red-400",
};

export function Sidebar({
  messages,
  status,
  onStartTask,
  onKill,
  onResume,
  onRespond,
}: Props) {
  const [input, setInput] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  const handleSubmit = () => {
    const text = input.trim();
    if (!text) return;
    setInput("");

    if (status.waiting) {
      onRespond(text);
    } else {
      onStartTask(text);
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  return (
    <div className="flex flex-col h-full w-80 border-l border-gray-800 bg-gray-950">
      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-3 space-y-2">
        {messages.map((msg, i) => (
          <div key={i} className={`text-sm ${TYPE_COLORS[msg.type]}`}>
            {msg.type === "action" && (
              <span className="text-gray-600 mr-1">&gt;</span>
            )}
            {msg.text}
          </div>
        ))}
        <div ref={messagesEndRef} />
      </div>

      {/* Kill / Resume */}
      <div className="px-3 pb-2">
        {status.running && !status.waiting && (
          <button
            onClick={onKill}
            className="w-full py-2 bg-red-600 hover:bg-red-700 text-white rounded-md text-sm font-medium"
          >
            Stop Agent
          </button>
        )}
        {!status.running && messages.length > 0 && (
          <button
            onClick={onResume}
            className="w-full py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-md text-sm font-medium"
          >
            Resume
          </button>
        )}
      </div>

      {/* Input */}
      <div className="p-3 border-t border-gray-800">
        <textarea
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={
            status.waiting
              ? "Agent is waiting for your answer..."
              : status.running
                ? "Agent is working..."
                : "Type a task..."
          }
          className="w-full bg-gray-800 text-gray-200 rounded-md px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-blue-500 resize-none"
          rows={2}
          disabled={status.running && !status.waiting}
        />
      </div>
    </div>
  );
}
```

- [ ] **Step 6: Create StatusBar component**

Create `browser/src/renderer/components/StatusBar.tsx`:
```tsx
import type { AgentUsage } from "../types";

interface Props {
  usage: AgentUsage;
  model: string;
  running: boolean;
  onOpenSettings: () => void;
}

export function StatusBar({ usage, model, running, onOpenSettings }: Props) {
  const formatCost = (cost: number) =>
    cost < 0.01 ? "<$0.01" : `$${cost.toFixed(2)}`;

  const formatTokens = (n: number) =>
    n >= 1000 ? `${(n / 1000).toFixed(1)}k` : String(n);

  return (
    <div className="flex items-center h-6 bg-gray-900 border-t border-gray-800 px-3 text-xs text-gray-500 gap-4 select-none">
      <span>
        Tokens: {formatTokens(usage.inputTokens + usage.outputTokens)}
      </span>
      <span>Cost: {formatCost(usage.cost)}</span>
      <span>{model}</span>
      <button
        onClick={onOpenSettings}
        className="hover:text-gray-200 ml-auto mr-2"
        title="Settings"
      >
        &#9881;
      </button>
      <span>
        {running ? (
          <span className="text-green-500">&#9679; Running</span>
        ) : (
          <span>&#9679; Idle</span>
        )}
      </span>
    </div>
  );
}
```

- [ ] **Step 7: Wire up App.tsx**

Replace `browser/src/renderer/App.tsx`:
```tsx
import { useState } from "react";
import { TabBar } from "./components/TabBar";
import { UrlBar } from "./components/UrlBar";
import { Sidebar } from "./components/Sidebar";
import { StatusBar } from "./components/StatusBar";
import { Settings } from "./components/Settings";
import { useAtlas } from "./hooks/useAtlas";

export default function App() {
  const atlas = useAtlas();
  const [showSettings, setShowSettings] = useState(false);

  return (
    <div className="flex flex-col h-screen bg-gray-950 text-gray-100">
      {/* Tab Bar */}
      <TabBar
        tabs={atlas.tabs}
        onSwitch={atlas.switchTab}
        onNew={atlas.newTab}
        onClose={atlas.closeTab}
      />

      {/* URL Bar */}
      <UrlBar
        url={atlas.url}
        onNavigate={atlas.navigateTo}
        onBack={atlas.goBack}
        onForward={atlas.goForward}
        onRefresh={atlas.refresh}
      />

      {/* Main content: browser pane + sidebar */}
      <div className="flex flex-1 overflow-hidden">
        {/* Browser pane is rendered by Electron's WebContentsView — this div is a spacer */}
        <div className="flex-1" />

        {/* Sidebar */}
        <Sidebar
          messages={atlas.messages}
          status={atlas.status}
          onStartTask={atlas.startTask}
          onKill={atlas.kill}
          onResume={atlas.resume}
          onRespond={atlas.respond}
        />
      </div>

      {/* Status Bar */}
      <StatusBar
        usage={atlas.usage}
        model="Gemini 2.5 Pro"
        running={atlas.status.running}
        onOpenSettings={() => setShowSettings(true)}
      />

      {/* Settings Modal */}
      {showSettings && (
        <Settings
          onSave={(s) => window.atlas.saveSettings(s)}
          onClose={() => setShowSettings(false)}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 8: Verify the app compiles and renders**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npm start
```

Expected: App launches with tab bar, URL bar, sidebar with input, and status bar. No functionality yet (browser pane is empty placeholder).

- [ ] **Step 9: Commit**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana
git add browser/src/renderer/
git commit -m "feat(atlas): add React UI — sidebar, tab bar, URL bar, status bar"
```

---

### Task 11: Wire Everything Together in Main Process

**Files:**
- Modify: `browser/src/main/index.ts`

This task integrates all layers: CDPClient, TabManager, Actions, Perception, AgentLoop, GeminiProvider, and IPC — into the Electron main process entry point.

- [ ] **Step 1: Update main process entry to wire all components**

Replace `browser/src/main/index.ts`:
```typescript
import { app, BrowserWindow, safeStorage } from "electron";
import path from "node:path";
import fs from "node:fs";
import { CDPClient } from "./cdp";
import { TabManager } from "./tabs";
import { Actions } from "./actions";
import { perceive } from "./perception";
import { AgentLoop } from "./agent";
import { GeminiProvider } from "./gemini";
import { setupIPC, sendToRenderer } from "./ipc";

let mainWindow: BrowserWindow | null = null;
let cdp: CDPClient;
let tabManager: TabManager;
let actions: Actions;
let agent: AgentLoop | null = null;

// Settings storage
const settingsPath = path.join(app.getPath("userData"), "settings.json");

interface Settings {
  encryptedApiKey: string | null;
  model: string;
}

function loadSettings(): Settings {
  try {
    const data = fs.readFileSync(settingsPath, "utf-8");
    return JSON.parse(data);
  } catch {
    return { encryptedApiKey: null, model: "gemini-2.5-pro" };
  }
}

function saveSettingsFile(settings: Settings): void {
  fs.writeFileSync(settingsPath, JSON.stringify(settings, null, 2));
}

function getDecryptedApiKey(): string | null {
  const settings = loadSettings();
  if (!settings.encryptedApiKey) return null;
  try {
    const buffer = Buffer.from(settings.encryptedApiKey, "base64");
    return safeStorage.decryptString(buffer);
  } catch {
    return null;
  }
}

function createWindow(): void {
  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 800,
    minHeight: 600,
    titleBarStyle: "hiddenInset",
    webPreferences: {
      preload: path.join(__dirname, "../preload.js"),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  // Initialize core layers
  cdp = new CDPClient();
  actions = new Actions(cdp);
  tabManager = new TabManager(mainWindow, cdp);

  // Wire IPC
  setupIPC({
    window: mainWindow,
    tabManager,
    getAgent: () => agent,
    createAgent: (apiKey: string, model: string) => {
      const provider = new GeminiProvider(apiKey, model);
      agent = new AgentLoop({
        provider,
        actions,
        perceive: (wc) => perceive(cdp, wc),
        onMessage: (msg) => sendToRenderer(mainWindow!, "agent:message", msg),
        onStatus: (status) =>
          sendToRenderer(mainWindow!, "agent:status", status),
        onUsage: (usage) => sendToRenderer(mainWindow!, "agent:usage", usage),
        getActiveWebContents: () => tabManager.getActiveWebContents(),
        tabActions: {
          newTab: (url) => tabManager.newTab(url),
          switchTab: (index) => {
            const tabs = tabManager.getAllTabs();
            if (tabs[index]) tabManager.switchTo(tabs[index].id);
          },
          closeTab: (index) => {
            const tabs = tabManager.getAllTabs();
            if (tabs[index]) tabManager.closeTab(tabs[index].id);
          },
          getAllTabs: () => tabManager.getAllTabs(),
        },
      });
      return agent;
    },
    getSettings: () => ({
      apiKey: getDecryptedApiKey(),
      model: loadSettings().model,
    }),
    saveSettings: (settings) => {
      const current = loadSettings();
      if (settings.apiKey !== undefined) {
        if (settings.apiKey && safeStorage.isEncryptionAvailable()) {
          current.encryptedApiKey = safeStorage
            .encryptString(settings.apiKey)
            .toString("base64");
        } else {
          current.encryptedApiKey = null;
        }
      }
      if (settings.model !== undefined) {
        current.model = settings.model;
      }
      saveSettingsFile(current);
    },
  });

  // Handle window resize — relayout tabs
  mainWindow.on("resize", () => {
    tabManager.relayout();
  });

  // Register keyboard shortcut for kill switch
  mainWindow.webContents.on("before-input-event", (_event, input) => {
    if (
      input.type === "keyDown" &&
      input.key === "K" &&
      input.shift &&
      (input.meta || input.control)
    ) {
      agent?.kill();
    }
  });

  // Load the renderer
  if (MAIN_WINDOW_VITE_DEV_SERVER_URL) {
    mainWindow.loadURL(MAIN_WINDOW_VITE_DEV_SERVER_URL);
  } else {
    mainWindow.loadFile(
      path.join(__dirname, `../renderer/${MAIN_WINDOW_VITE_NAME}/index.html`)
    );
  }

  // Open first tab
  tabManager.newTab("https://www.google.com");
}

app.whenReady().then(createWindow);

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") app.quit();
});

app.on("activate", () => {
  if (BrowserWindow.getAllWindows().length === 0) createWindow();
});
```

Note: `MAIN_WINDOW_VITE_DEV_SERVER_URL` and `MAIN_WINDOW_VITE_NAME` are forge-generated constants. Adjust to match your forge config. They may be accessed differently depending on the Vite plugin version — check the forge template's generated code.

- [ ] **Step 2: Verify the full app launches with a browser tab**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npm start
```

Expected: App launches, opens Google in the browser pane, sidebar visible with task input, tab bar shows "Google" tab.

- [ ] **Step 3: Test the agent loop end-to-end**

1. Enter your Gemini API key in settings (or temporarily hardcode for testing)
2. Type a simple task in the sidebar: "Navigate to github.com"
3. Agent should:
   - Perceive the current page
   - Call Gemini which returns a `navigate` tool call
   - Navigate to github.com
   - Perceive again
   - Call `done`

- [ ] **Step 4: Test the kill switch**

1. Start a longer task: "Search for Electron tutorials on Google"
2. Press `Cmd+Shift+K` while agent is running
3. Agent should stop, "Resume" button appears

- [ ] **Step 5: Commit**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana
git add browser/src/main/index.ts
git commit -m "feat(atlas): wire all layers in main process entry point"
```

---

### Task 12: Settings UI

**Files:**
- Create: `browser/src/renderer/components/Settings.tsx`
- Modify: `browser/src/renderer/App.tsx`

- [ ] **Step 1: Create Settings component**

Create `browser/src/renderer/components/Settings.tsx`:
```tsx
import { useState, type KeyboardEvent } from "react";

interface Props {
  onSave: (settings: { apiKey?: string; model?: string }) => void;
  onClose: () => void;
}

const MODELS = [
  { value: "gemini-2.5-pro", label: "Gemini 2.5 Pro" },
  { value: "gemini-2.5-flash", label: "Gemini 2.5 Flash" },
  { value: "gemini-2.0-flash", label: "Gemini 2.0 Flash" },
];

export function Settings({ onSave, onClose }: Props) {
  const [apiKey, setApiKey] = useState("");
  const [model, setModel] = useState("gemini-2.5-pro");

  const handleSave = () => {
    onSave({
      ...(apiKey ? { apiKey } : {}),
      model,
    });
    onClose();
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-gray-900 rounded-lg p-6 w-96 border border-gray-700">
        <h2 className="text-lg font-medium text-gray-100 mb-4">Settings</h2>

        <div className="space-y-4">
          <div>
            <label className="block text-sm text-gray-400 mb-1">
              Gemini API Key
            </label>
            <input
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="Enter API key..."
              className="w-full bg-gray-800 text-gray-200 rounded-md px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-blue-500"
            />
          </div>

          <div>
            <label className="block text-sm text-gray-400 mb-1">Model</label>
            <select
              value={model}
              onChange={(e) => setModel(e.target.value)}
              className="w-full bg-gray-800 text-gray-200 rounded-md px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-blue-500"
            >
              {MODELS.map((m) => (
                <option key={m.value} value={m.value}>
                  {m.label}
                </option>
              ))}
            </select>
          </div>
        </div>

        <div className="flex gap-2 mt-6">
          <button
            onClick={handleSave}
            className="flex-1 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-md text-sm font-medium"
          >
            Save
          </button>
          <button
            onClick={onClose}
            className="flex-1 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-md text-sm font-medium"
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Add settings toggle to App.tsx**

In `browser/src/renderer/App.tsx`, add settings state and render the Settings modal:

Add to imports:
```tsx
import { Settings } from "./components/Settings";
```

Add state in App component:
```tsx
const [showSettings, setShowSettings] = useState(false);
```

Add settings button to the sidebar area (before closing `</div>` of the flex container) and the modal:
```tsx
{showSettings && (
  <Settings
    onSave={(s) => window.atlas.saveSettings(s)}
    onClose={() => setShowSettings(false)}
  />
)}
```

Add a gear icon button to the status bar or sidebar header that calls `setShowSettings(true)`.

- [ ] **Step 3: Verify settings UI works**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npm start
```

Expected: Settings modal opens, API key can be entered, model selected, saved.

- [ ] **Step 4: Commit**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana
git add browser/src/renderer/
git commit -m "feat(atlas): add settings UI for API key and model selection"
```

---

### Task 13: File Library

**Files:**
- Create: `browser/src/main/files.ts`
- Create: `browser/src/main/__tests__/files.test.ts`
- Create: `browser/src/renderer/components/FileLibrary.tsx`
- Modify: `browser/src/main/ipc.ts`
- Modify: `browser/src/main/index.ts`
- Modify: `browser/src/renderer/App.tsx`
- Modify: `browser/src/renderer/hooks/useAtlas.ts`

**Reference:** Spec section "Local File Library (`files.ts`)"

- [ ] **Step 1: Write file library tests**

Create `browser/src/main/__tests__/files.test.ts`:
```typescript
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { FileLibrary } from "../files";
import fs from "node:fs";
import path from "node:path";
import os from "node:os";

describe("FileLibrary", () => {
  let lib: FileLibrary;
  let storageDir: string;

  beforeEach(() => {
    storageDir = path.join(os.tmpdir(), `atlas-files-test-${Date.now()}`);
    lib = new FileLibrary(storageDir);
  });

  afterEach(() => {
    fs.rmSync(storageDir, { recursive: true, force: true });
  });

  it("adds a file and returns its metadata", async () => {
    // Create a temp file to import
    const tmpFile = path.join(os.tmpdir(), "test-resume.txt");
    fs.writeFileSync(tmpFile, "John Doe - Software Engineer");

    const file = await lib.addFile(tmpFile, {
      reusable: true,
      kind: "resume",
    });

    expect(file.id).toBeDefined();
    expect(file.name).toBe("test-resume.txt");
    expect(file.kind).toBe("resume");
    expect(file.reusable).toBe(true);
    expect(file.mimeType).toBe("text/plain");
    expect(file.extractedText).toBe("John Doe - Software Engineer");

    fs.unlinkSync(tmpFile);
  });

  it("lists reusable files only", async () => {
    const tmp1 = path.join(os.tmpdir(), "reusable.txt");
    const tmp2 = path.join(os.tmpdir(), "onetime.txt");
    fs.writeFileSync(tmp1, "reusable");
    fs.writeFileSync(tmp2, "onetime");

    await lib.addFile(tmp1, { reusable: true, kind: "resume" });
    await lib.addFile(tmp2, { reusable: false, kind: "other" });

    const reusable = lib.getReusableFiles();
    expect(reusable).toHaveLength(1);
    expect(reusable[0].name).toBe("reusable.txt");

    fs.unlinkSync(tmp1);
    fs.unlinkSync(tmp2);
  });

  it("gets a file by ID", async () => {
    const tmp = path.join(os.tmpdir(), "findme.txt");
    fs.writeFileSync(tmp, "content");

    const added = await lib.addFile(tmp, { reusable: true, kind: "other" });
    const found = lib.getFile(added.id);
    expect(found?.id).toBe(added.id);

    fs.unlinkSync(tmp);
  });

  it("removes a file", async () => {
    const tmp = path.join(os.tmpdir(), "removeme.txt");
    fs.writeFileSync(tmp, "content");

    const added = await lib.addFile(tmp, { reusable: true, kind: "other" });
    lib.removeFile(added.id);

    expect(lib.getFile(added.id)).toBeUndefined();
    expect(lib.getAllFiles()).toHaveLength(0);

    fs.unlinkSync(tmp);
  });

  it("marks a file as used", async () => {
    const tmp = path.join(os.tmpdir(), "useme.txt");
    fs.writeFileSync(tmp, "content");

    const added = await lib.addFile(tmp, { reusable: true, kind: "other" });
    expect(added.lastUsedAt).toBeUndefined();

    lib.markUsed(added.id);
    const updated = lib.getFile(added.id);
    expect(updated?.lastUsedAt).toBeDefined();

    fs.unlinkSync(tmp);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npx vitest run src/main/__tests__/files.test.ts
```

Expected: FAIL — `FileLibrary` not found.

- [ ] **Step 3: Implement FileLibrary**

Create `browser/src/main/files.ts`:
```typescript
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import type { StoredFile } from "./types";

/**
 * Local file library for user-provided assets (resumes, cover letters, etc.).
 * Files are copied into Atlas-managed storage and metadata is persisted as JSON.
 */
export class FileLibrary {
  private files: Map<string, StoredFile> = new Map();
  private storageDir: string;
  private metadataPath: string;

  constructor(storageDir: string) {
    this.storageDir = storageDir;
    this.metadataPath = path.join(storageDir, "files.json");

    // Ensure storage directory exists
    fs.mkdirSync(storageDir, { recursive: true });

    // Load existing metadata
    this.loadMetadata();
  }

  /**
   * Import a file into the library. Copies the file to managed storage.
   */
  async addFile(
    sourcePath: string,
    options: {
      reusable?: boolean;
      kind?: StoredFile["kind"];
      summary?: string;
    } = {}
  ): Promise<StoredFile> {
    const id = crypto.randomUUID();
    const name = path.basename(sourcePath);
    const ext = path.extname(name).toLowerCase();
    const mimeType = this.guessMimeType(ext);

    // Copy file to managed storage
    const destPath = path.join(this.storageDir, `${id}${ext}`);
    fs.copyFileSync(sourcePath, destPath);

    // Extract text for text-based files
    let extractedText: string | undefined;
    if ([".txt", ".md", ".json", ".csv"].includes(ext)) {
      try {
        extractedText = fs.readFileSync(destPath, "utf-8");
      } catch {
        // Extraction failed — file still stored
      }
    }

    const file: StoredFile = {
      id,
      name,
      kind: options.kind ?? "other",
      mimeType,
      path: destPath,
      reusable: options.reusable ?? false,
      summary: options.summary,
      extractedText,
      createdAt: new Date().toISOString(),
    };

    this.files.set(id, file);
    this.saveMetadata();
    return file;
  }

  getFile(id: string): StoredFile | undefined {
    return this.files.get(id);
  }

  getAllFiles(): StoredFile[] {
    return Array.from(this.files.values());
  }

  getReusableFiles(): StoredFile[] {
    return this.getAllFiles().filter((f) => f.reusable);
  }

  removeFile(id: string): void {
    const file = this.files.get(id);
    if (file) {
      // Delete the stored copy
      try {
        fs.unlinkSync(file.path);
      } catch {
        // File may already be gone
      }
      this.files.delete(id);
      this.saveMetadata();
    }
  }

  markUsed(id: string): void {
    const file = this.files.get(id);
    if (file) {
      file.lastUsedAt = new Date().toISOString();
      this.saveMetadata();
    }
  }

  private guessMimeType(ext: string): string {
    const types: Record<string, string> = {
      ".pdf": "application/pdf",
      ".doc": "application/msword",
      ".docx": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
      ".txt": "text/plain",
      ".md": "text/markdown",
      ".json": "application/json",
      ".csv": "text/csv",
      ".png": "image/png",
      ".jpg": "image/jpeg",
      ".jpeg": "image/jpeg",
    };
    return types[ext] ?? "application/octet-stream";
  }

  private loadMetadata(): void {
    try {
      const data = fs.readFileSync(this.metadataPath, "utf-8");
      const files: StoredFile[] = JSON.parse(data);
      for (const f of files) {
        // Only load files that still exist on disk
        if (fs.existsSync(f.path)) {
          this.files.set(f.id, f);
        }
      }
    } catch {
      // No metadata yet — fresh library
    }
  }

  private saveMetadata(): void {
    const data = JSON.stringify(this.getAllFiles(), null, 2);
    fs.writeFileSync(this.metadataPath, data);
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npx vitest run src/main/__tests__/files.test.ts
```

Expected: All tests PASS.

- [ ] **Step 5: Add file IPC handlers to `ipc.ts`**

Add to the `setupIPC` function in `browser/src/main/ipc.ts`:
```typescript
// In the IPCDeps interface, add:
  fileLibrary: FileLibrary;

// In setupIPC, add these handlers:
  ipcMain.on("files:add", async (_event, { paths, reusable, kind }) => {
    for (const filePath of paths) {
      await deps.fileLibrary.addFile(filePath, { reusable: reusable ?? true, kind });
    }
    sendToRenderer(win, "files:update", deps.fileLibrary.getAllFiles());
  });

  ipcMain.on("files:remove", (_event, { id }) => {
    deps.fileLibrary.removeFile(id);
    sendToRenderer(win, "files:update", deps.fileLibrary.getAllFiles());
  });
```

- [ ] **Step 6: Wire FileLibrary in main process `index.ts`**

Add to `browser/src/main/index.ts`:
```typescript
import { FileLibrary } from "./files";

// After app.getPath("userData") is available:
const fileLibrary = new FileLibrary(
  path.join(app.getPath("userData"), "atlas-files")
);

// Pass to setupIPC:
  fileLibrary,

// Pass to createAgent's config:
  fileActions: {
    getReusableFiles: () => fileLibrary.getReusableFiles(),
    getFile: (id) => fileLibrary.getFile(id),
    markUsed: (id) => fileLibrary.markUsed(id),
  },
```

- [ ] **Step 7: Create FileLibrary UI component**

Create `browser/src/renderer/components/FileLibrary.tsx`:
```tsx
import type { StoredFile } from "../types";

interface Props {
  files: StoredFile[];
  onAddFiles: () => void;
  onRemove: (id: string) => void;
  onClose: () => void;
}

const KIND_LABELS: Record<StoredFile["kind"], string> = {
  resume: "Resume",
  cover_letter: "Cover Letter",
  transcript: "Transcript",
  portfolio: "Portfolio",
  other: "Other",
};

export function FileLibrary({ files, onAddFiles, onRemove, onClose }: Props) {
  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-gray-900 rounded-lg p-6 w-[480px] max-h-[600px] border border-gray-700 flex flex-col">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-medium text-gray-100">File Library</h2>
          <button
            onClick={onClose}
            className="text-gray-500 hover:text-gray-200"
          >
            &times;
          </button>
        </div>

        <p className="text-sm text-gray-400 mb-4">
          Files stored here are available to the agent for form filling and
          uploads. Reusable files persist across tasks.
        </p>

        <div className="flex-1 overflow-y-auto space-y-2 mb-4">
          {files.length === 0 ? (
            <p className="text-gray-500 text-sm text-center py-8">
              No files yet. Add a resume, cover letter, or other document.
            </p>
          ) : (
            files.map((file) => (
              <div
                key={file.id}
                className="flex items-center justify-between bg-gray-800 rounded-md px-3 py-2"
              >
                <div className="flex-1 min-w-0">
                  <div className="text-sm text-gray-200 truncate">
                    {file.name}
                  </div>
                  <div className="text-xs text-gray-500">
                    {KIND_LABELS[file.kind]} &middot; {file.mimeType}
                    {file.lastUsedAt &&
                      ` &middot; Last used ${new Date(file.lastUsedAt).toLocaleDateString()}`}
                  </div>
                </div>
                <button
                  onClick={() => onRemove(file.id)}
                  className="text-gray-500 hover:text-red-400 text-xs ml-2"
                >
                  Remove
                </button>
              </div>
            ))
          )}
        </div>

        <button
          onClick={onAddFiles}
          className="w-full py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-md text-sm font-medium"
        >
          Add Files...
        </button>
      </div>
    </div>
  );
}
```

- [ ] **Step 8: Add file library to useAtlas hook and App.tsx**

In `browser/src/renderer/hooks/useAtlas.ts`, add:
```typescript
const [files, setFiles] = useState<StoredFile[]>([]);

// In useEffect:
window.atlas.onFilesUpdate(setFiles),

// Return:
files,
addFiles: window.atlas.addFiles,
removeFile: window.atlas.removeFile,
```

In `browser/src/renderer/App.tsx`, add a file library button (e.g., in the status bar or sidebar) and the `FileLibrary` modal:
```tsx
const [showFiles, setShowFiles] = useState(false);

// Add file button to StatusBar or sidebar:
<button onClick={() => setShowFiles(true)}>Files</button>

// Render modal:
{showFiles && (
  <FileLibrary
    files={atlas.files}
    onAddFiles={() => {
      // Use Electron's dialog via IPC to pick files
      // For MVP: expose a pickFiles IPC that opens dialog.showOpenDialog
    }}
    onRemove={(id) => window.atlas.removeFile(id)}
    onClose={() => setShowFiles(false)}
  />
)}
```

Note: The "Add Files" button needs to trigger `dialog.showOpenDialog` in the main process. Add a `files:pick` IPC channel: renderer sends `files:pick`, main opens the native file picker dialog, imports selected files, and sends back `files:update`.

Add to `ipc.ts`:
```typescript
ipcMain.handle("files:pick", async () => {
  const { dialog } = require("electron");
  const result = await dialog.showOpenDialog(win, {
    properties: ["openFile", "multiSelections"],
    filters: [
      { name: "Documents", extensions: ["pdf", "doc", "docx", "txt", "md"] },
      { name: "All Files", extensions: ["*"] },
    ],
  });
  if (!result.canceled && result.filePaths.length > 0) {
    for (const filePath of result.filePaths) {
      await deps.fileLibrary.addFile(filePath, { reusable: true });
    }
    sendToRenderer(win, "files:update", deps.fileLibrary.getAllFiles());
  }
});
```

Add to preload:
```typescript
pickFiles: () => ipcRenderer.invoke("files:pick"),
```

- [ ] **Step 9: Verify file library works end-to-end**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npm start
```

1. Open File Library from the UI
2. Click "Add Files", select a `.txt` or `.pdf` file
3. File appears in the library list
4. Start a task that involves file upload (e.g., on a test form)
5. Agent should use `upload_file` to populate the file input

- [ ] **Step 10: Commit**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana
git add browser/src/main/files.ts browser/src/main/__tests__/files.test.ts browser/src/renderer/components/FileLibrary.tsx
git add browser/src/main/ipc.ts browser/src/main/index.ts browser/src/renderer/App.tsx browser/src/renderer/hooks/useAtlas.ts
git commit -m "feat(atlas): add local file library with attach_file and upload_file tools"
```

---

### Task 14: Final Integration Test

**Files:** None created — this is a manual verification task.

- [ ] **Step 1: Fresh start test**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/browser
npm start
```

1. App launches with Google loaded in browser pane
2. Enter Gemini API key in Settings
3. Type task: "Go to github.com and tell me what you see"
4. Watch agent navigate, perceive, and report back
5. Agent calls `done` — verify summary appears in sidebar

- [ ] **Step 2: Multi-tab test**

1. Click "+" to open new tab
2. Type task: "Open wikipedia.org in a new tab"
3. Verify agent uses `new_tab` tool

- [ ] **Step 3: Kill switch test**

1. Start task: "Search for 'electron tutorial' on Google and click the first result"
2. Press `Cmd+Shift+K` mid-task
3. Verify agent stops, "Resume" button appears
4. Click Resume — agent continues from current page state

- [ ] **Step 4: File library test**

1. Open File Library, add a `.txt` file
2. Verify file appears in the library
3. Start task: "Upload my resume to the file input on this page"
4. Verify agent uses `upload_file` tool
5. Remove the file from the library, verify it disappears

- [ ] **Step 5: Error handling test**

1. Set an invalid API key in Settings
2. Start a task
3. Verify error message appears in sidebar: "API key error..."

- [ ] **Step 6: Commit final state**

```bash
cd /Volumes/Samsung/repositories/mostrom/node-package-manager/tivana
git add -A browser/
git commit -m "feat(atlas): complete MVP — Electron AI browser with Gemini agent"
```
