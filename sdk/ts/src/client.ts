/**
 * Tivana Client
 *
 * WebSocket client for connecting to the Tivana runtime.
 * Works with both Bun and Node.js.
 */

import type {
  ActionResult,
  ActionTarget,
  BatchAction,
  BatchResult,
  CaptchaInfo,
  CaptchaSolveResult,
  ClickOptions,
  ClickTarget,
  ClientOptions,
  Cookie,
  Element,
  FormField,
  FormFillResult,
  IncomingMessage,
  MutationCallback,
  MutationEvent,
  NetworkRequest,
  OutgoingMessage,
  PageState,
  ScreenshotOptions,
  ScreenshotResult,
  ScrollDirection,
  SessionCreateParams,
  SessionCreateResult,
  SessionInfo,
  SetCookieOptions,
  SmartFillProfile,
  SmartFillResult,
  TypeOptions,
  AccessibilitySnapshot,
  TextContent,
  PageMetadata,
  ElementInfo,
  TabInfo,
  ProxyConfig,
} from "./types";
import { PROTOCOL_VERSION } from "./types";

/** Default client options */
const DEFAULT_OPTIONS: Required<ClientOptions> = {
  url: "ws://localhost:9876",
  timeout: 30000,
  autoReconnect: false,
  reconnectDelay: 1000,
  maxReconnectDelay: 30000,
  onReconnect: () => {},
};

/** Pending request */
interface PendingRequest {
  resolve: (result: unknown) => void;
  reject: (error: Error) => void;
  timeout: ReturnType<typeof setTimeout>;
}

/**
 * Tivana Client
 *
 * Connects to the Tivana runtime and provides methods for browser perception and actions.
 */
/** Reconnect state */
type ReconnectState = "idle" | "reconnecting" | "connected";

/**
 * Tivana Client
 *
 * Connects to the Tivana runtime and provides methods for browser perception and actions.
 */
export class TivanaClient {
  private options: Required<ClientOptions>;
  private ws: WebSocket | null = null;
  private sessionId: string | null = null;
  private messageId = 0;
  private pending = new Map<string, PendingRequest>();
  private mutationCallbacks: MutationCallback[] = [];
  private connected = false;
  private nodeWs: typeof import("ws") | null = null;

  /** Reconnect state */
  private reconnectState: ReconnectState = "idle";
  private currentReconnectDelay = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private commandQueue: Array<{ method: string; params: unknown; resolve: (v: unknown) => void; reject: (e: Error) => void }> = [];

  /** Event listeners */
  private eventListeners: Map<string, Array<(...args: unknown[]) => void>> = new Map();

  constructor(options: ClientOptions = {}) {
    this.options = { ...DEFAULT_OPTIONS, ...options };
  }

  /** Register an event listener ('reconnecting' | 'reconnected' | 'disconnected') */
  on(event: string, listener: (...args: unknown[]) => void): () => void {
    const listeners = this.eventListeners.get(event) || [];
    listeners.push(listener);
    this.eventListeners.set(event, listeners);
    return () => {
      const arr = this.eventListeners.get(event) || [];
      const idx = arr.indexOf(listener);
      if (idx !== -1) arr.splice(idx, 1);
    };
  }

  private emit(event: string, ...args: unknown[]): void {
    const listeners = this.eventListeners.get(event) || [];
    for (const listener of listeners) {
      try { listener(...args); } catch { /* ignore */ }
    }
  }

  /**
   * Connect to the Tivana runtime
   */
  async connect(url?: string): Promise<void> {
    const wsUrl = url || this.options.url;

    // Use Bun WebSocket if available, otherwise fall back to Node ws
    // @ts-ignore - Bun is a global in Bun runtime
    if (typeof globalThis.Bun !== "undefined") {
      await this.connectBun(wsUrl);
    } else {
      await this.connectNode(wsUrl);
    }
  }

  /**
   * Connect using Bun WebSocket (native)
   */
  private async connectBun(url: string): Promise<void> {
    return new Promise((resolve, reject) => {
      const ws = new WebSocket(url);

      ws.onopen = () => {
        this.ws = ws;
        this.connected = true;
        resolve();
      };

      ws.onerror = (event) => {
        reject(new Error(`WebSocket connection failed: ${event}`));
      };

      ws.onclose = () => {
        this.handleDisconnect();
      };

      ws.onmessage = (event) => {
        this.handleMessage(event.data as string);
      };
    });
  }

  /**
   * Connect using Node.js ws package
   */
  private async connectNode(url: string): Promise<void> {
    // Dynamic import for Node.js compatibility
    if (!this.nodeWs) {
      this.nodeWs = await import("ws");
    }

    const WebSocket = this.nodeWs.default || this.nodeWs;

    return new Promise((resolve, reject) => {
      const ws = new WebSocket(url);

      ws.on("open", () => {
        this.ws = ws as unknown as WebSocket;
        this.connected = true;
        resolve();
      });

      ws.on("error", (error: Error) => {
        reject(new Error(`WebSocket connection failed: ${error.message}`));
      });

      ws.on("close", () => {
        this.handleDisconnect();
      });

      ws.on("message", (data: Buffer) => {
        this.handleMessage(data.toString());
      });
    });
  }

  /**
   * Handle WebSocket disconnection
   */
  private handleDisconnect(): void {
    this.connected = false;
    this.ws = null;

    this.emit("disconnected");

    // Reject all pending requests
    for (const [id, request] of this.pending) {
      clearTimeout(request.timeout);
      request.reject(new Error("WebSocket disconnected"));
      this.pending.delete(id);
    }

    // Auto-reconnect with exponential backoff if enabled
    if (this.options.autoReconnect && this.reconnectState !== "reconnecting") {
      this.attemptReconnect();
    }
  }

  /**
   * Attempt to reconnect with exponential backoff
   */
  private attemptReconnect(): void {
    if (this.reconnectState === "reconnecting") return;
    this.reconnectState = "reconnecting";
    this.currentReconnectDelay = this.options.reconnectDelay;

    this.emit("reconnecting");
    this.scheduleReconnect();
  }

  private scheduleReconnect(): void {
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);

    this.reconnectTimer = setTimeout(async () => {
      try {
        await this.connect();

        // Re-attach session if we had one
        if (this.sessionId) {
          await this.request("session.get", { sessionId: this.sessionId });
        }

        // Reconnected successfully
        this.reconnectState = "connected";
        this.currentReconnectDelay = this.options.reconnectDelay;

        this.emit("reconnected");
        this.options.onReconnect();

        // Replay queued commands
        const queue = [...this.commandQueue];
        this.commandQueue = [];
        for (const cmd of queue) {
          try {
            const result = await this.request(cmd.method, cmd.params);
            cmd.resolve(result);
          } catch (e) {
            cmd.reject(e instanceof Error ? e : new Error(String(e)));
          }
        }
      } catch {
        // Exponential backoff: double the delay up to max
        this.currentReconnectDelay = Math.min(
          this.currentReconnectDelay * 2,
          this.options.maxReconnectDelay,
        );
        this.scheduleReconnect();
      }
    }, this.currentReconnectDelay);
  }

  /**
   * Handle incoming message
   */
  private handleMessage(data: string): void {
    let message: IncomingMessage;
    try {
      message = JSON.parse(data);
    } catch {
      console.error("Failed to parse message:", data);
      return;
    }

    // Handle events (mutations, page events, etc.)
    if (message.type === "event") {
      // Handle mutation events
      if (message.event === "page.mutation" || message.event === "mutations") {
        const events = (message.data as MutationEvent[]) || [];
        for (const callback of this.mutationCallbacks) {
          try {
            callback(events);
          } catch (e) {
            console.error("Mutation callback error:", e);
          }
        }
      }
      return;
    }

    // Handle response
    const pending = this.pending.get(message.id);
    if (!pending) {
      console.warn("Received response for unknown request:", message.id);
      return;
    }

    clearTimeout(pending.timeout);
    this.pending.delete(message.id);

    if (message.error) {
      pending.reject(
        new Error(`[${message.error.code}] ${message.error.message}`)
      );
    } else {
      pending.resolve(message.result);
    }
  }

  /**
   * Send a request and wait for response.
   * If reconnecting, queues the command for replay after reconnect.
   */
  private async request<T>(method: string, params?: unknown): Promise<T> {
    if (!this.connected || !this.ws) {
      // If we're reconnecting, queue the command
      if (this.reconnectState === "reconnecting") {
        return new Promise<T>((resolve, reject) => {
          this.commandQueue.push({
            method,
            params,
            resolve: resolve as (v: unknown) => void,
            reject,
          });
        });
      }
      throw new Error("Not connected to runtime");
    }

    const id = `req-${++this.messageId}`;
    const message: OutgoingMessage = {
      id,
      type: "request",
      method,
      params: params ?? {},
      version: PROTOCOL_VERSION,
    };

    if (this.sessionId && method !== "session.create") {
      message.sessionId = this.sessionId;
    }

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`Request timeout: ${method}`));
      }, this.options.timeout);

      this.pending.set(id, {
        resolve: resolve as (result: unknown) => void,
        reject,
        timeout,
      });

      const data = JSON.stringify(message);
      if (typeof this.ws?.send === "function") {
        this.ws.send(data);
      }
    });
  }

  //===========================================================================
  // Session API
  //===========================================================================

  /**
   * Create a browser session
   */
  async createSession(params?: SessionCreateParams): Promise<string> {
    const result = await this.request<SessionCreateResult>(
      "session.create",
      params || {}
    );
    this.sessionId = result.sessionId;
    return this.sessionId;
  }

  /**
   * Close the current session
   */
  async closeSession(): Promise<void> {
    if (!this.sessionId) {
      throw new Error("No active session");
    }
    await this.request("session.close");
    this.sessionId = null;
  }

  /**
   * List all sessions
   */
  async listSessions(): Promise<SessionInfo[]> {
    const result = await this.request<{ sessions: SessionInfo[] }>(
      "session.list"
    );
    return result.sessions;
  }

  //===========================================================================
  // Extension API
  //===========================================================================

  /**
   * Create a session from a Chrome extension-attached tab.
   * The extension must be connected to the runtime and have at least one tab attached.
   *
   * @param extensionSessionId - Optional specific extension session ID to use.
   *   If omitted, uses the first available extension tab.
   * @returns Extension tab info including the extensionSessionId to use for commands.
   */
  async connectExtension(extensionSessionId?: string): Promise<{
    sessionId: string;
    extensionSessionId: string;
    tabId: number;
    targetId: string;
    url: string;
    title: string;
    connected: boolean;
  }> {
    const result = await this.request("session.fromExtension", {
      ...(extensionSessionId ? { extensionSessionId } : {}),
    });
    if (result.sessionId) {
      this.sessionId = result.sessionId;
    }
    return result;
  }

  /**
   * List all tabs currently attached via the Chrome extension.
   */
  async extensionTabs(): Promise<{
    tabs: Array<{
      tabId: number;
      targetId: string;
      sessionId: string;
      url: string;
      title: string;
    }>;
    connected: boolean;
  }> {
    return this.request("extension.tabs");
  }

  //===========================================================================
  // Tab Management API
  //===========================================================================

  /**
   * List all open tabs in the browser
   */
  async tabs(): Promise<TabInfo[]> {
    const result = await this.request<{ tabs: TabInfo[]; count: number }>(
      "session.tabs"
    );
    return result.tabs;
  }

  /**
   * Switch to a different tab by target ID
   */
  async switchTab(targetId: string): Promise<{ targetId: string; url: string; title: string }> {
    return this.request("session.switchTab", { targetId });
  }

  /**
   * Open a new tab with optional URL
   */
  async newTab(url?: string): Promise<{ targetId: string; url: string; title: string }> {
    return this.request("session.newTab", url ? { url } : {});
  }

  /**
   * Close a tab by target ID
   */
  async closeTab(targetId: string): Promise<{ closed: boolean; targetId: string }> {
    return this.request("session.closeTab", { targetId });
  }

  /**
   * Clean up orphaned about:blank tabs (closes all except the active tab)
   */
  async cleanTabs(): Promise<{ closed: number }> {
    return this.request("session.cleanTabs");
  }

  //===========================================================================
  // Perception API
  //===========================================================================

  /**
   * Get current page state
   */
  async pageState(): Promise<PageState> {
    return this.request<PageState>("perceive.pageState");
  }

  /**
   * Get interactive elements on the page
   */
  async elements(): Promise<Element[]> {
    return this.request<Element[]>("perceive.elements");
  }

  /**
   * Get full accessibility tree snapshot
   */
  async accessibilitySnapshot(): Promise<AccessibilitySnapshot> {
    return this.request<AccessibilitySnapshot>("perceive.accessibilitySnapshot");
  }

  /**
   * Get page text content
   */
  async textContent(): Promise<TextContent> {
    return this.request<TextContent>("perceive.textContent");
  }

  /**
   * Get page metadata
   */
  async metadata(): Promise<PageMetadata> {
    return this.request<PageMetadata>("perceive.metadata");
  }

  /**
   * Find elements matching a selector
   */
  async findElements(selector: string): Promise<ElementInfo[]> {
    return this.request<ElementInfo[]>("perceive.findElements", { selector });
  }

  /**
   * Get all form fields on the page with full introspection data
   */
  async formFields(): Promise<FormField[]> {
    const result = await this.request<{ fields: FormField[] }>(
      "perceive.formFields"
    );
    return result.fields;
  }

  /**
   * Evaluate a JavaScript expression on the page and return the result
   */
  async evaluate<T = any>(expression: string, awaitPromise?: boolean): Promise<T> {
    const result = await this.request<{ result: T }>("perceive.evaluate", {
      expression,
      awaitPromise,
    });
    return result.result;
  }

  /**
   * Evaluate a JavaScript expression on the page without returning a value
   */
  async evaluateVoid(expression: string): Promise<void> {
    await this.request("perceive.evaluateVoid", { expression });
  }

  /**
   * Subscribe to mutation events
   */
  onMutation(callback: MutationCallback): () => void {
    this.mutationCallbacks.push(callback);
    return () => {
      const index = this.mutationCallbacks.indexOf(callback);
      if (index !== -1) {
        this.mutationCallbacks.splice(index, 1);
      }
    };
  }

  /**
   * Take a screenshot of the current page
   */
  async screenshot(options?: ScreenshotOptions): Promise<ScreenshotResult> {
    return this.request<ScreenshotResult>("perceive.screenshot", options || {});
  }

  //===========================================================================
  // Network Monitoring API
  //===========================================================================

  /**
   * Enable network request capture (inject monitoring script)
   */
  async enableNetworkCapture(): Promise<void> {
    await this.request("network.enable");
  }

  /**
   * Get captured network requests
   *
   * @param urlPattern Optional URL substring to filter by
   */
  async getNetworkRequests(urlPattern?: string): Promise<NetworkRequest[]> {
    const result = await this.request<{ requests: NetworkRequest[] }>(
      "network.requests",
      urlPattern ? { urlPattern } : {}
    );
    return result.requests;
  }

  /**
   * Clear all captured network requests
   */
  async clearNetworkRequests(): Promise<void> {
    await this.request("network.clear");
  }

  //===========================================================================
  // Action API
  //===========================================================================

  /**
   * Navigate to a URL
   */
  async navigate(url: string): Promise<ActionResult> {
    return this.request<ActionResult>("act.navigate", { url });
  }

  /**
   * Click an element
   *
   * @param target Element ID (e.g., "e5"), selector string, or { role, label } object
   * @param options Click options
   */
  async click(
    target: ClickTarget,
    options?: ClickOptions
  ): Promise<ActionResult> {
    // Convert ClickTarget to ActionTarget
    const actionTarget: ActionTarget =
      typeof target === "string"
        ? target.startsWith("e") && /^e\d+$/.test(target)
          ? { elementId: target }
          : { selector: target }
        : { role: target.role, label: target.label };

    return this.request<ActionResult>("act.click", {
      target: actionTarget,
      ...options,
    });
  }

  /**
   * Type text into the focused element or a target element
   *
   * @param text Text to type
   * @param target Optional element ID or selector to focus first
   * @param options Type options
   */
  async type(
    text: string,
    target?: string,
    options?: TypeOptions
  ): Promise<ActionResult> {
    const actionTarget: ActionTarget | undefined = target
      ? target.startsWith("e") && /^e\d+$/.test(target)
        ? { elementId: target }
        : { selector: target }
      : undefined;

    return this.request<ActionResult>("act.type", {
      text,
      target: actionTarget,
      ...options,
    });
  }

  /**
   * Press a key or key combination
   *
   * @param key Key to press (e.g., "Enter", "Tab", "a")
   * @param modifiers Modifier keys (e.g., ["Control", "Shift"])
   */
  async press(key: string, modifiers?: string[]): Promise<ActionResult> {
    return this.request<ActionResult>("act.press", {
      key,
      modifiers: modifiers || [],
    });
  }

  /**
   * Scroll the page or element into view
   *
   * @param target Element ID or selector to scroll into view
   * @param direction Scroll direction (if target not specified)
   * @param options Scroll options
   */
  async scroll(
    target?: string,
    direction?: ScrollDirection,
    options?: { amount?: number; smooth?: boolean }
  ): Promise<ActionResult> {
    const actionTarget: ActionTarget | undefined = target
      ? target.startsWith("e") && /^e\d+$/.test(target)
        ? { elementId: target }
        : { selector: target }
      : undefined;

    return this.request<ActionResult>("act.scroll", {
      target: actionTarget,
      direction,
      ...options,
    });
  }

  /**
   * Hover over an element
   *
   * @param target Element ID or selector
   */
  async hover(target: string): Promise<ActionResult> {
    const actionTarget: ActionTarget = target.startsWith("e") &&
      /^e\d+$/.test(target)
      ? { elementId: target }
      : { selector: target };

    return this.request<ActionResult>("act.hover", { target: actionTarget });
  }

  /**
   * Focus an element
   *
   * @param target Element ID or selector
   */
  async focus(target: string): Promise<ActionResult> {
    const actionTarget: ActionTarget = target.startsWith("e") &&
      /^e\d+$/.test(target)
      ? { elementId: target }
      : { selector: target };

    return this.request<ActionResult>("act.focus", { target: actionTarget });
  }

  /**
   * Select an option from a dropdown
   *
   * @param target Element ID or selector
   * @param value Value to select
   */
  async select(target: string, value: string): Promise<ActionResult> {
    const actionTarget: ActionTarget = target.startsWith("e") &&
      /^e\d+$/.test(target)
      ? { elementId: target }
      : { selector: target };

    return this.request<ActionResult>("act.select", {
      target: actionTarget,
      value,
    });
  }

  /**
   * Wait for a condition
   *
   * @param condition Wait condition
   * @param timeoutMs Timeout in milliseconds (default: 30000)
   */
  async waitFor(
    condition:
      | { type: "Element"; selector: string }
      | { type: "Visible"; selector: string }
      | { type: "Hidden"; selector: string }
      | { type: "Navigation" }
      | { type: "NetworkIdle"; idleTimeMs?: number }
      | { type: "Delay"; durationMs: number },
    timeoutMs?: number
  ): Promise<ActionResult> {
    return this.request<ActionResult>("act.waitFor", {
      condition,
      timeoutMs: timeoutMs || this.options.timeout,
    });
  }

  /**
   * Wait for a CSS selector to match a visible element
   *
   * @param selector CSS selector to wait for
   * @param timeoutMs Timeout in milliseconds (default: 30000)
   */
  async waitForSelector(
    selector: string,
    timeoutMs?: number
  ): Promise<ActionResult> {
    return this.request<ActionResult>("act.waitForSelector", {
      selector,
      timeoutMs: timeoutMs || this.options.timeout,
    });
  }

  /**
   * Wait for a page navigation (URL change)
   *
   * @param timeoutMs Timeout in milliseconds (default: 30000)
   */
  async waitForNavigation(timeoutMs?: number): Promise<ActionResult> {
    return this.request<ActionResult>("act.waitForNavigation", {
      timeoutMs: timeoutMs || this.options.timeout,
    });
  }

  /**
   * Wait for a JavaScript expression to return a truthy value
   *
   * @param expression JavaScript expression to evaluate
   * @param timeoutMs Timeout in milliseconds (default: 30000)
   */
  async waitForFunction(
    expression: string,
    timeoutMs?: number
  ): Promise<ActionResult> {
    return this.request<ActionResult>("act.waitForFunction", {
      expression,
      timeoutMs: timeoutMs || this.options.timeout,
    });
  }

  //===========================================================================
  // CAPTCHA API
  //===========================================================================

  /**
   * Detect CAPTCHA presence on the current page
   */
  async detectCaptcha(): Promise<CaptchaInfo> {
    return this.request<CaptchaInfo>("captcha.detect");
  }

  /**
   * Attempt to solve any detected CAPTCHA automatically
   */
  async solveCaptcha(): Promise<CaptchaSolveResult> {
    return this.request<CaptchaSolveResult>("captcha.solve");
  }

  //===========================================================================
  // Batch & Form Fill API
  //===========================================================================

  /**
   * Execute a batch of actions in a single roundtrip
   *
   * @param actions Array of actions to execute sequentially
   * @param options Batch options
   */
  async batch(
    actions: BatchAction[],
    options?: { stopOnError?: boolean }
  ): Promise<BatchResult> {
    // Convert string targets to ActionTarget objects
    const wireActions = actions.map((a) => {
      const wire: Record<string, unknown> = { type: a.type };
      if (a.target) {
        const t = a.target;
        wire.target =
          t.startsWith("e") && /^e\d+$/.test(t)
            ? { elementId: t }
            : { selector: t };
      }
      if (a.text !== undefined) wire.text = a.text;
      if (a.key !== undefined) wire.key = a.key;
      if (a.modifiers !== undefined) wire.modifiers = a.modifiers;
      if (a.direction !== undefined) wire.direction = a.direction;
      if (a.amount !== undefined) wire.amount = a.amount;
      if (a.url !== undefined) wire.url = a.url;
      if (a.value !== undefined) wire.value = a.value;
      if (a.delayMs !== undefined) wire.delayMs = a.delayMs;
      return wire;
    });

    return this.request<BatchResult>("act.batch", {
      actions: wireActions,
      stopOnError: options?.stopOnError ?? false,
    });
  }

  /**
   * Fill a form in a single roundtrip
   *
   * @param fields Map of element IDs to values (string for text, boolean for checkboxes)
   * @param submit Optional element ID of submit button to click after filling
   */
  async fillForm(
    fields: Record<string, string | boolean>,
    submit?: string
  ): Promise<FormFillResult> {
    return this.request<FormFillResult>("act.fillForm", {
      fields,
      submit,
    });
  }

  /**
   * Smart fill a form by matching field labels to a profile object
   *
   * @param profile Object with profile fields (firstName, email, etc.)
   * @param options Smart fill options
   */
  async smartFill(
    profile: SmartFillProfile,
    options?: { skipRecaptcha?: boolean }
  ): Promise<SmartFillResult> {
    return this.request<SmartFillResult>("act.smartFill", {
      profile,
      skipRecaptcha: options?.skipRecaptcha ?? false,
    });
  }

  //===========================================================================
  // Dialog Handling API (MOS-122)
  //===========================================================================

  /**
   * Handle a JavaScript dialog (alert/confirm/prompt)
   *
   * @param action Whether to 'accept' or 'dismiss' the dialog
   * @param promptText Optional text to enter for prompt dialogs
   */
  async handleDialog(
    action: "accept" | "dismiss",
    promptText?: string
  ): Promise<ActionResult> {
    return this.request<ActionResult>("act.handleDialog", {
      action,
      promptText,
    });
  }

  //===========================================================================
  // File Upload API (MOS-128)
  //===========================================================================

  /**
   * Upload files to a file input element
   *
   * @param target Element ID or selector for the file input
   * @param filePaths Array of absolute file paths to upload
   */
  async uploadFile(
    target: string,
    filePaths: string[]
  ): Promise<ActionResult> {
    const actionTarget: ActionTarget =
      target.startsWith("e") && /^e\d+$/.test(target)
        ? { elementId: target }
        : { selector: target };

    return this.request<ActionResult>("act.uploadFile", {
      target: actionTarget,
      filePaths,
    });
  }

  //===========================================================================
  // Cookie & Storage API (MOS-127)
  //===========================================================================

  /**
   * Get all cookies for the current page
   */
  async getCookies(): Promise<Cookie[]> {
    const result = await this.request<{ cookies: Cookie[] }>(
      "storage.getCookies"
    );
    return result.cookies;
  }

  /**
   * Set a cookie
   *
   * @param name Cookie name
   * @param value Cookie value
   * @param options Optional cookie attributes
   */
  async setCookie(
    name: string,
    value: string,
    options?: SetCookieOptions
  ): Promise<void> {
    await this.request("storage.setCookie", {
      name,
      value,
      ...options,
    });
  }

  /**
   * Clear all browser cookies
   */
  async clearCookies(): Promise<void> {
    await this.request("storage.clearCookies");
  }

  /**
   * Get all localStorage entries
   */
  async getLocalStorage(): Promise<Record<string, string>> {
    const result = await this.request<{ entries: Record<string, string> }>(
      "storage.getLocalStorage"
    );
    return result.entries;
  }

  /**
   * Set a localStorage entry
   *
   * @param key Storage key
   * @param value Storage value
   */
  async setLocalStorage(key: string, value: string): Promise<void> {
    await this.request("storage.setLocalStorage", { key, value });
  }

  /**
   * Get all sessionStorage entries
   */
  async getSessionStorage(): Promise<Record<string, string>> {
    const result = await this.request<{ entries: Record<string, string> }>(
      "storage.getSessionStorage"
    );
    return result.entries;
  }

  /**
   * Set a sessionStorage entry
   *
   * @param key Storage key
   * @param value Storage value
   */
  async setSessionStorage(key: string, value: string): Promise<void> {
    await this.request("storage.setSessionStorage", { key, value });
  }

  /**
   * Clear both localStorage and sessionStorage
   */
  async clearStorage(): Promise<void> {
    await this.request("storage.clear");
  }

  //===========================================================================
  // Proxy API (MOS-120)
  //===========================================================================

  /**
   * Set proxy for the current session
   *
   * @param config Proxy configuration
   */
  async setProxy(config: ProxyConfig): Promise<void> {
    await this.request("proxy.set", config);
  }

  /**
   * Set a proxy pool for rotation
   *
   * @param proxies Array of proxy configurations
   */
  async setProxyPool(proxies: ProxyConfig[]): Promise<{ poolSize: number; current: ProxyConfig | null }> {
    return this.request("proxy.pool", { proxies });
  }

  /**
   * Rotate to the next proxy in the pool
   */
  async rotateProxy(): Promise<ProxyConfig> {
    const result = await this.request<{ proxy: ProxyConfig }>("proxy.rotate");
    return result.proxy;
  }

  /**
   * Get the current proxy configuration
   */
  async currentProxy(): Promise<ProxyConfig | null> {
    const result = await this.request<{ proxy: ProxyConfig | null }>("proxy.current");
    return result.proxy;
  }

  //===========================================================================
  // Connection API
  //===========================================================================

  /**
   * Check if connected
   */
  isConnected(): boolean {
    return this.connected;
  }

  /**
   * Get current session ID
   */
  getSessionId(): string | null {
    return this.sessionId;
  }

  /**
   * Disconnect from runtime. Stops auto-reconnect if active.
   */
  disconnect(): void {
    // Stop any reconnect attempts
    this.reconnectState = "idle";
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    // Reject queued commands
    for (const cmd of this.commandQueue) {
      cmd.reject(new Error("Client disconnected"));
    }
    this.commandQueue = [];

    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.connected = false;
    this.sessionId = null;
  }
}

//=============================================================================
// Convenience Exports
//=============================================================================

/** Global client instance for simple usage */
let globalClient: TivanaClient | null = null;

/**
 * Connect to Tivana runtime (convenience function)
 */
export async function connect(url?: string): Promise<TivanaClient> {
  const client = new TivanaClient({ url });
  await client.connect();
  globalClient = client;
  return client;
}

/**
 * Get global client instance
 */
export function getClient(): TivanaClient {
  if (!globalClient) {
    throw new Error("Not connected. Call connect() first.");
  }
  return globalClient;
}

/**
 * Observe page state (convenience function)
 */
export async function observe(
  callback: (page: PageState, elements: Element[]) => void | Promise<void>
): Promise<() => void> {
  const client = getClient();

  // Initial state
  const [page, elements] = await Promise.all([
    client.pageState(),
    client.elements(),
  ]);
  await callback(page, elements);

  // Subscribe to mutations
  return client.onMutation(async () => {
    const [page, elements] = await Promise.all([
      client.pageState(),
      client.elements(),
    ]);
    await callback(page, elements);
  });
}

/**
 * Action object for convenience
 */
export const act = {
  async click(target: ClickTarget): Promise<ActionResult> {
    return getClient().click(target);
  },

  async type(text: string, target?: string): Promise<ActionResult> {
    return getClient().type(text, target);
  },

  async navigate(url: string): Promise<ActionResult> {
    return getClient().navigate(url);
  },

  async scroll(
    target?: string,
    direction?: ScrollDirection
  ): Promise<ActionResult> {
    return getClient().scroll(target, direction);
  },

  async press(key: string, modifiers?: string[]): Promise<ActionResult> {
    return getClient().press(key, modifiers);
  },

  async hover(target: string): Promise<ActionResult> {
    return getClient().hover(target);
  },

  async focus(target: string): Promise<ActionResult> {
    return getClient().focus(target);
  },

  async select(target: string, value: string): Promise<ActionResult> {
    return getClient().select(target, value);
  },

  async batch(
    actions: BatchAction[],
    options?: { stopOnError?: boolean }
  ): Promise<BatchResult> {
    return getClient().batch(actions, options);
  },

  async fillForm(
    fields: Record<string, string | boolean>,
    submit?: string
  ): Promise<FormFillResult> {
    return getClient().fillForm(fields, submit);
  },

  async smartFill(
    profile: SmartFillProfile,
    options?: { skipRecaptcha?: boolean }
  ): Promise<SmartFillResult> {
    return getClient().smartFill(profile, options);
  },

  async waitForSelector(
    selector: string,
    timeoutMs?: number
  ): Promise<ActionResult> {
    return getClient().waitForSelector(selector, timeoutMs);
  },

  async waitForNavigation(timeoutMs?: number): Promise<ActionResult> {
    return getClient().waitForNavigation(timeoutMs);
  },

  async waitForFunction(
    expression: string,
    timeoutMs?: number
  ): Promise<ActionResult> {
    return getClient().waitForFunction(expression, timeoutMs);
  },
};
