/**
 * Tivana Client
 *
 * WebSocket client for connecting to the Tivana runtime.
 * Works with both Bun and Node.js.
 */
import type { ActionResult, BatchAction, BatchResult, CaptchaInfo, CaptchaSolveResult, ClickOptions, ClickTarget, ClientOptions, Cookie, Element, FormField, FormFillResult, MutationCallback, NetworkRequest, PageEventCallback, PageEventType, PageState, ScreenshotOptions, ScreenshotResult, ScrollDirection, SessionCreateParams, SessionInfo, SetCookieOptions, SmartFillProfile, SmartFillResult, TypeOptions, AccessibilitySnapshot, TextContent, PageMetadata, ElementInfo, TabInfo, ProxyConfig } from "./types";
/**
 * Tivana Client
 *
 * Connects to the Tivana runtime and provides methods for browser perception and actions.
 */
export declare class TivanaClient {
    private options;
    private ws;
    private sessionId;
    private messageId;
    private pending;
    private mutationCallbacks;
    private pageEventCallbacks;
    private connected;
    private nodeWs;
    /** Reconnect state */
    private reconnectState;
    private currentReconnectDelay;
    private reconnectTimer;
    private commandQueue;
    /** Event listeners */
    private eventListeners;
    constructor(options?: ClientOptions);
    /** Register an event listener ('reconnecting' | 'reconnected' | 'disconnected') */
    on(event: string, listener: (...args: unknown[]) => void): () => void;
    private emit;
    /**
     * Connect to the Tivana runtime
     */
    connect(url?: string): Promise<void>;
    /**
     * Connect using Bun WebSocket (native)
     */
    private connectBun;
    /**
     * Connect using Node.js ws package
     */
    private connectNode;
    /**
     * Handle WebSocket disconnection
     */
    private handleDisconnect;
    /**
     * Attempt to reconnect with exponential backoff
     */
    private attemptReconnect;
    private scheduleReconnect;
    /**
     * Handle incoming message
     */
    private handleMessage;
    /**
     * Send a request and wait for response.
     * If reconnecting, queues the command for replay after reconnect.
     */
    private request;
    /**
     * Create a browser session
     */
    createSession(params?: SessionCreateParams): Promise<string>;
    /**
     * Close the current session
     */
    closeSession(): Promise<void>;
    /**
     * List all sessions
     */
    listSessions(): Promise<SessionInfo[]>;
    /**
     * Create a session from a Chrome extension-attached tab.
     * The extension must be connected to the runtime and have at least one tab attached.
     *
     * @param extensionSessionId - Optional specific extension session ID to use.
     *   If omitted, uses the first available extension tab.
     * @returns Extension tab info including the extensionSessionId to use for commands.
     */
    connectExtension(extensionSessionId?: string): Promise<{
        sessionId: string;
        extensionSessionId: string;
        tabId: number;
        targetId: string;
        url: string;
        title: string;
        connected: boolean;
    }>;
    /**
     * List all tabs currently attached via the Chrome extension.
     */
    extensionTabs(): Promise<{
        tabs: Array<{
            tabId: number;
            targetId: string;
            sessionId: string;
            url: string;
            title: string;
        }>;
        connected: boolean;
    }>;
    /**
     * List all open tabs in the browser
     */
    tabs(): Promise<TabInfo[]>;
    /**
     * Switch to a different tab by target ID
     */
    switchTab(targetId: string): Promise<{
        targetId: string;
        url: string;
        title: string;
    }>;
    /**
     * Open a new tab with optional URL
     */
    newTab(url?: string): Promise<{
        targetId: string;
        url: string;
        title: string;
    }>;
    /**
     * Close a tab by target ID
     */
    closeTab(targetId: string): Promise<{
        closed: boolean;
        targetId: string;
    }>;
    /**
     * Clean up orphaned about:blank tabs (closes all except the active tab)
     */
    cleanTabs(): Promise<{
        closed: number;
    }>;
    /**
     * Get current page state
     */
    pageState(): Promise<PageState>;
    /**
     * Get interactive elements on the page
     */
    elements(): Promise<Element[]>;
    /**
     * Get full accessibility tree snapshot
     */
    accessibilitySnapshot(): Promise<AccessibilitySnapshot>;
    /**
     * Get page text content
     */
    textContent(): Promise<TextContent>;
    /**
     * Get page metadata
     */
    metadata(): Promise<PageMetadata>;
    /**
     * Find elements matching a selector
     */
    findElements(selector: string): Promise<ElementInfo[]>;
    /**
     * Get all form fields on the page with full introspection data
     */
    formFields(): Promise<FormField[]>;
    /**
     * Evaluate a JavaScript expression on the page and return the result
     */
    evaluate<T = any>(expression: string, awaitPromise?: boolean): Promise<T>;
    /**
     * Evaluate a JavaScript expression on the page without returning a value
     */
    evaluateVoid(expression: string): Promise<void>;
    /**
     * Subscribe to mutation events
     */
    onMutation(callback: MutationCallback): () => void;
    /**
     * Subscribe to specific page event type or all events.
     * Returns an unsubscribe function.
     */
    onPageEvent(event: PageEventType | "*", callback: PageEventCallback): () => void;
    /**
     * Subscribe to all page events (alias for onPageEvent("*", callback))
     */
    onEvent(callback: PageEventCallback): () => void;
    /**
     * Start observation — tells runtime to begin streaming mutations and page events
     */
    startObservation(): Promise<void>;
    /**
     * Stop observation — tells runtime to stop streaming events
     */
    stopObservation(): Promise<void>;
    /**
     * Take a screenshot of the current page
     */
    screenshot(options?: ScreenshotOptions): Promise<ScreenshotResult>;
    /**
     * Enable network request capture (inject monitoring script)
     */
    enableNetworkCapture(): Promise<void>;
    /**
     * Get captured network requests
     *
     * @param urlPattern Optional URL substring to filter by
     */
    getNetworkRequests(urlPattern?: string): Promise<NetworkRequest[]>;
    /**
     * Clear all captured network requests
     */
    clearNetworkRequests(): Promise<void>;
    /**
     * Navigate to a URL
     */
    navigate(url: string): Promise<ActionResult>;
    /**
     * Click an element
     *
     * @param target Element ID (e.g., "e5"), selector string, or { role, label } object
     * @param options Click options
     */
    click(target: ClickTarget, options?: ClickOptions): Promise<ActionResult>;
    /**
     * Type text into the focused element or a target element
     *
     * @param text Text to type
     * @param target Optional element ID or selector to focus first
     * @param options Type options
     */
    type(text: string, target?: string, options?: TypeOptions): Promise<ActionResult>;
    /**
     * Press a key or key combination
     *
     * @param key Key to press (e.g., "Enter", "Tab", "a")
     * @param modifiers Modifier keys (e.g., ["Control", "Shift"])
     */
    press(key: string, modifiers?: string[]): Promise<ActionResult>;
    /**
     * Scroll the page or element into view
     *
     * @param target Element ID or selector to scroll into view
     * @param direction Scroll direction (if target not specified)
     * @param options Scroll options
     */
    scroll(target?: string, direction?: ScrollDirection, options?: {
        amount?: number;
        smooth?: boolean;
    }): Promise<ActionResult>;
    /**
     * Hover over an element
     *
     * @param target Element ID or selector
     */
    hover(target: string): Promise<ActionResult>;
    /**
     * Focus an element
     *
     * @param target Element ID or selector
     */
    focus(target: string): Promise<ActionResult>;
    /**
     * Select an option from a dropdown
     *
     * @param target Element ID or selector
     * @param value Value to select
     */
    select(target: string, value: string): Promise<ActionResult>;
    /**
     * Wait for a condition
     *
     * @param condition Wait condition
     * @param timeoutMs Timeout in milliseconds (default: 30000)
     */
    waitFor(condition: {
        type: "Element";
        selector: string;
    } | {
        type: "Visible";
        selector: string;
    } | {
        type: "Hidden";
        selector: string;
    } | {
        type: "Navigation";
    } | {
        type: "NetworkIdle";
        idleTimeMs?: number;
    } | {
        type: "Delay";
        durationMs: number;
    }, timeoutMs?: number): Promise<ActionResult>;
    /**
     * Wait for a CSS selector to match a visible element
     *
     * @param selector CSS selector to wait for
     * @param timeoutMs Timeout in milliseconds (default: 30000)
     */
    waitForSelector(selector: string, timeoutMs?: number): Promise<ActionResult>;
    /**
     * Wait for a page navigation (URL change)
     *
     * @param timeoutMs Timeout in milliseconds (default: 30000)
     */
    waitForNavigation(timeoutMs?: number): Promise<ActionResult>;
    /**
     * Wait for a JavaScript expression to return a truthy value
     *
     * @param expression JavaScript expression to evaluate
     * @param timeoutMs Timeout in milliseconds (default: 30000)
     */
    waitForFunction(expression: string, timeoutMs?: number): Promise<ActionResult>;
    /**
     * Detect CAPTCHA presence on the current page
     */
    detectCaptcha(): Promise<CaptchaInfo>;
    /**
     * Attempt to solve any detected CAPTCHA automatically
     */
    solveCaptcha(): Promise<CaptchaSolveResult>;
    /**
     * Execute a batch of actions in a single roundtrip
     *
     * @param actions Array of actions to execute sequentially
     * @param options Batch options
     */
    batch(actions: BatchAction[], options?: {
        stopOnError?: boolean;
    }): Promise<BatchResult>;
    /**
     * Fill a form in a single roundtrip
     *
     * @param fields Map of element IDs to values (string for text, boolean for checkboxes)
     * @param submit Optional element ID of submit button to click after filling
     */
    fillForm(fields: Record<string, string | boolean>, submit?: string): Promise<FormFillResult>;
    /**
     * Smart fill a form by matching field labels to a profile object
     *
     * @param profile Object with profile fields (firstName, email, etc.)
     * @param options Smart fill options
     */
    smartFill(profile: SmartFillProfile, options?: {
        skipRecaptcha?: boolean;
    }): Promise<SmartFillResult>;
    /**
     * Handle a JavaScript dialog (alert/confirm/prompt)
     *
     * @param action Whether to 'accept' or 'dismiss' the dialog
     * @param promptText Optional text to enter for prompt dialogs
     */
    handleDialog(action: "accept" | "dismiss", promptText?: string): Promise<ActionResult>;
    /**
     * Upload files to a file input element
     *
     * @param target Element ID or selector for the file input
     * @param filePaths Array of absolute file paths to upload
     */
    uploadFile(target: string, filePaths: string[]): Promise<ActionResult>;
    /**
     * Get all cookies for the current page
     */
    getCookies(): Promise<Cookie[]>;
    /**
     * Set a cookie
     *
     * @param name Cookie name
     * @param value Cookie value
     * @param options Optional cookie attributes
     */
    setCookie(name: string, value: string, options?: SetCookieOptions): Promise<void>;
    /**
     * Clear all browser cookies
     */
    clearCookies(): Promise<void>;
    /**
     * Get all localStorage entries
     */
    getLocalStorage(): Promise<Record<string, string>>;
    /**
     * Set a localStorage entry
     *
     * @param key Storage key
     * @param value Storage value
     */
    setLocalStorage(key: string, value: string): Promise<void>;
    /**
     * Get all sessionStorage entries
     */
    getSessionStorage(): Promise<Record<string, string>>;
    /**
     * Set a sessionStorage entry
     *
     * @param key Storage key
     * @param value Storage value
     */
    setSessionStorage(key: string, value: string): Promise<void>;
    /**
     * Clear both localStorage and sessionStorage
     */
    clearStorage(): Promise<void>;
    /**
     * Set proxy for the current session
     *
     * @param config Proxy configuration
     */
    setProxy(config: ProxyConfig): Promise<void>;
    /**
     * Set a proxy pool for rotation
     *
     * @param proxies Array of proxy configurations
     */
    setProxyPool(proxies: ProxyConfig[]): Promise<{
        poolSize: number;
        current: ProxyConfig | null;
    }>;
    /**
     * Rotate to the next proxy in the pool
     */
    rotateProxy(): Promise<ProxyConfig>;
    /**
     * Get the current proxy configuration
     */
    currentProxy(): Promise<ProxyConfig | null>;
    /**
     * Check if connected
     */
    isConnected(): boolean;
    /**
     * Get current session ID
     */
    getSessionId(): string | null;
    /**
     * Disconnect from runtime. Stops auto-reconnect if active.
     */
    disconnect(): void;
}
/**
 * Connect to Tivana runtime (convenience function)
 */
export declare function connect(url?: string): Promise<TivanaClient>;
/**
 * Get global client instance
 */
export declare function getClient(): TivanaClient;
/**
 * Observe page events (convenience function).
 *
 * Starts observation, subscribes to events, and fires an initial page.loaded snapshot.
 * Returns a cleanup function that unsubscribes and stops observation.
 *
 * @param callback Called for each page event
 * @param options Optional filter for specific event types
 */
export declare function observe(callback: PageEventCallback, options?: {
    events?: PageEventType[];
}): Promise<() => void>;
/**
 * Action object for convenience
 */
export declare const act: {
    click(target: ClickTarget): Promise<ActionResult>;
    type(text: string, target?: string): Promise<ActionResult>;
    navigate(url: string): Promise<ActionResult>;
    scroll(target?: string, direction?: ScrollDirection): Promise<ActionResult>;
    press(key: string, modifiers?: string[]): Promise<ActionResult>;
    hover(target: string): Promise<ActionResult>;
    focus(target: string): Promise<ActionResult>;
    select(target: string, value: string): Promise<ActionResult>;
    batch(actions: BatchAction[], options?: {
        stopOnError?: boolean;
    }): Promise<BatchResult>;
    fillForm(fields: Record<string, string | boolean>, submit?: string): Promise<FormFillResult>;
    smartFill(profile: SmartFillProfile, options?: {
        skipRecaptcha?: boolean;
    }): Promise<SmartFillResult>;
    waitForSelector(selector: string, timeoutMs?: number): Promise<ActionResult>;
    waitForNavigation(timeoutMs?: number): Promise<ActionResult>;
    waitForFunction(expression: string, timeoutMs?: number): Promise<ActionResult>;
};
//# sourceMappingURL=client.d.ts.map