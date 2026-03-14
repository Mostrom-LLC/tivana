/**
 * Tivana Client
 *
 * WebSocket client for connecting to the Tivana runtime.
 * Works with both Bun and Node.js.
 */

import type {
  ActionResult,
  ClickParams,
  ClickTarget,
  ClientOptions,
  Element,
  IncomingMessage,
  MutationCallback,
  MutationEvent,
  OutgoingMessage,
  PageState,
  ScrollBehavior,
  ScrollParams,
  SessionCreateParams,
  SessionCreateResult,
  TypeParams,
} from "./types";
import { ErrorCode, PROTOCOL_VERSION } from "./types";

/** Default client options */
const DEFAULT_OPTIONS: Required<ClientOptions> = {
  url: "ws://localhost:9876",
  timeout: 30000,
  autoReconnect: false,
  reconnectDelay: 1000,
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
export class TivanaClient {
  private options: Required<ClientOptions>;
  private ws: WebSocket | null = null;
  private sessionId: string | null = null;
  private messageId = 0;
  private pending = new Map<string, PendingRequest>();
  private mutationCallbacks: MutationCallback[] = [];
  private connected = false;
  private nodeWs: typeof import("ws") | null = null;

  constructor(options: ClientOptions = {}) {
    this.options = { ...DEFAULT_OPTIONS, ...options };
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

    // Reject all pending requests
    for (const [id, request] of this.pending) {
      clearTimeout(request.timeout);
      request.reject(new Error("WebSocket disconnected"));
      this.pending.delete(id);
    }

    // Auto-reconnect if enabled
    if (this.options.autoReconnect) {
      setTimeout(() => {
        this.connect().catch(console.error);
      }, this.options.reconnectDelay);
    }
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

    // Handle events (mutations)
    if (message.type === "event" && message.method === "page.mutation") {
      const event = message.result as MutationEvent;
      for (const callback of this.mutationCallbacks) {
        try {
          callback(event);
        } catch (e) {
          console.error("Mutation callback error:", e);
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

    if (message.type === "error" || message.error) {
      const error = message.error || {
        code: ErrorCode.InternalError,
        message: "Unknown error",
      };
      pending.reject(
        new Error(`[${error.code}] ${error.message}`)
      );
    } else {
      pending.resolve(message.result);
    }
  }

  /**
   * Send a request and wait for response
   */
  private async request<T>(method: string, params?: unknown): Promise<T> {
    if (!this.connected || !this.ws) {
      throw new Error("Not connected to runtime");
    }

    const id = `req-${++this.messageId}`;
    const message: OutgoingMessage = {
      id,
      type: "request",
      method,
      params,
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
  async listSessions(): Promise<string[]> {
    const result = await this.request<{ sessions: string[] }>("session.list");
    return result.sessions;
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
   * Get element tree with full visual and semantic data
   */
  async elements(): Promise<Element[]> {
    const result = await this.request<{ elements: Element[] }>(
      "perceive.elements"
    );
    return result.elements;
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
   * @param target Element ID (e.g., "e5") or selector ({ role: "button", label: "Submit" })
   * @param options Click options
   */
  async click(
    target: ClickTarget,
    options?: Omit<ClickParams, "target">
  ): Promise<ActionResult> {
    const params: ClickParams = { target, ...options };
    return this.request<ActionResult>("act.click", params);
  }

  /**
   * Type text
   *
   * @param text Text to type
   * @param target Optional element ID to focus first
   */
  async type(text: string, target?: string): Promise<ActionResult> {
    const params: TypeParams = { text, target };
    return this.request<ActionResult>("act.type", params);
  }

  /**
   * Scroll element into view
   */
  async scroll(
    target: string,
    behavior?: ScrollBehavior
  ): Promise<ActionResult> {
    const params: ScrollParams = { target, behavior };
    return this.request<ActionResult>("act.scroll", params);
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
   * Disconnect from runtime
   */
  disconnect(): void {
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
  return client.onMutation(async (event) => {
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

  async scroll(target: string, behavior?: ScrollBehavior): Promise<ActionResult> {
    return getClient().scroll(target, behavior);
  },
};
