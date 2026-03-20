/**
 * Tivana SDK Types
 *
 * Type definitions for the Tivana protocol - streaming browser perception for AI agents.
 * These types match the Rust runtime implementation.
 */

//=============================================================================
// Protocol Types
//=============================================================================

/** Protocol version */
export const PROTOCOL_VERSION = "1.0";

/** Message type discriminator */
export type MessageType = "request" | "response" | "event";

/** Incoming message from runtime */
export interface IncomingMessage {
  id: string;
  type: MessageType;
  method?: string;
  event?: string;
  sessionId?: string;
  result?: unknown;
  data?: unknown;
  error?: ProtocolError;
  version?: string;
}

/** Outgoing message to runtime */
export interface OutgoingMessage {
  id: string;
  type: MessageType;
  method: string;
  sessionId?: string;
  params?: unknown;
  version?: string;
}

/** Protocol error */
export interface ProtocolError {
  code: string;
  message: string;
  data?: unknown;
}

/** Error codes (string-based to match Rust serde serialization) */
export const ErrorCode = {
  // Protocol errors
  InvalidMessage: "invalid_message",
  MissingField: "missing_field",
  InvalidField: "invalid_field",
  UnknownMethod: "unknown_method",

  // Session errors
  SessionNotFound: "session_not_found",
  SessionClosed: "session_closed",
  SessionExists: "session_exists",
  InvalidSessionState: "invalid_session_state",

  // Browser errors
  BrowserLaunchFailed: "browser_launch_failed",
  BrowserCrashed: "browser_crashed",
  BrowserDisconnected: "browser_disconnected",
  NavigationFailed: "navigation_failed",

  // Action errors
  TargetNotFound: "target_not_found",
  TargetAmbiguous: "target_ambiguous",
  ActionFailed: "action_failed",
  ActionTimeout: "action_timeout",

  // Perception errors
  PerceptionFailed: "perception_failed",
  ElementNotAccessible: "element_not_accessible",
  StyleComputationFailed: "style_computation_failed",

  // Internal errors
  InternalError: "internal_error",
} as const;

export type ErrorCodeType = (typeof ErrorCode)[keyof typeof ErrorCode];

//=============================================================================
// Session Types
//=============================================================================

/** Session status */
export type SessionStatus = "created" | "launching" | "active" | "closed";

/** Session create params */
export interface SessionCreateParams {
  /** Override headless mode */
  headless?: boolean;
}

/** Session create result */
export interface SessionCreateResult {
  sessionId: string;
  status: SessionStatus;
}

/** Session info */
export interface SessionInfo {
  sessionId: string;
  status: SessionStatus;
  url?: string;
}

//=============================================================================
// Tab Types
//=============================================================================

/** Information about a browser tab */
export interface TabInfo {
  /** CDP target ID */
  targetId: string;
  /** Current URL */
  url: string;
  /** Page title */
  title: string;
  /** Whether this is the currently active tab */
  active: boolean;
}

//=============================================================================
// Page State Types
//=============================================================================

/** Page state snapshot - matches Rust PageState struct */
export interface PageState {
  /** Current URL */
  url: string;

  /** Page title (may be null) */
  title: string | null;

  /** Currently focused element ID */
  focusedElementId: string | null;

  /** Scroll X position */
  scrollX: number;

  /** Scroll Y position */
  scrollY: number;

  /** Viewport width */
  viewportWidth: number;

  /** Viewport height */
  viewportHeight: number;

  /** Document width */
  documentWidth: number;

  /** Document height */
  documentHeight: number;

  /** Capture timestamp (ms since epoch) */
  timestampMs: number;
}

//=============================================================================
// Element Types
//=============================================================================

/** Bounding box coordinates - matches Rust BoundingBox struct */
export interface BoundingBox {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Element styles - matches Rust ElementStyles struct */
export interface ElementStyles {
  /** Font family */
  fontFamily?: string;
  /** Font size */
  fontSize?: string;
  /** Font weight */
  fontWeight?: string;
  /** Text color */
  color?: string;
  /** Background color */
  backgroundColor?: string;
  /** Border style */
  border?: string;
  /** Display type */
  display?: string;
  /** Visibility */
  visibility?: string;
}

/** Element in the page tree - matches Rust Element struct */
export interface Element {
  /** Stable element ID (e.g., "e1", "e2") */
  id: string;

  /** Accessibility role */
  role: string;

  /** Accessible name/label */
  name?: string;

  /** Current value (for inputs) */
  value?: string;

  /** Accessible description */
  description?: string;

  /** Bounding box */
  bounds?: BoundingBox;

  /** Computed styles (subset) */
  styles?: ElementStyles;

  /** Element has focus */
  focused: boolean;

  /** Element is enabled */
  enabled: boolean;

  /** Checked state (checkboxes, radios) */
  checked?: boolean;

  /** Selected state */
  selected?: boolean;

  /** Expanded state (accordions, dropdowns) */
  expanded?: boolean;

  /** Required state (form elements) */
  required?: boolean;

  /** Child elements */
  children?: Element[];
}

/** Accessibility snapshot - matches Rust AccessibilitySnapshot struct */
export interface AccessibilitySnapshot {
  /** Root element */
  root?: Element;

  /** Flat list of all interactive elements */
  interactiveElements: Element[];

  /** Snapshot timestamp */
  timestampMs: number;
}

/** Element info from find_elements - matches Rust ElementInfo struct */
export interface ElementInfo {
  /** CSS selector path */
  selector: string;

  /** Element tag name */
  tagName: string;

  /** Element text content */
  text?: string;

  /** Element attributes */
  attributes: Record<string, string>;

  /** Bounding box */
  bounds?: BoundingBox;
}

/** Text content - matches Rust TextContent struct */
export interface TextContent {
  /** Raw text content */
  text: string;

  /** Word count */
  wordCount: number;

  /** Character count */
  charCount: number;
}

/** Page metadata - matches Rust PageMetadata struct */
export interface PageMetadata {
  /** Page URL */
  url: string;

  /** Page title */
  title?: string;

  /** Meta description */
  description?: string;

  /** Favicon URL */
  favicon?: string;

  /** Open Graph image */
  ogImage?: string;

  /** Language */
  language?: string;
}

//=============================================================================
// Mutation Types
//=============================================================================

/** Mutation event - matches Rust MutationEvent enum */
export type MutationEvent =
  | { type: "Added"; elementId: string; parentId?: string }
  | { type: "Removed"; elementId: string }
  | {
      type: "Changed";
      elementId: string;
      attribute: string;
      oldValue?: string;
      newValue?: string;
    }
  | { type: "TextChanged"; elementId: string; text: string };

/** Mutation callback */
export type MutationCallback = (events: MutationEvent[]) => void;

//=============================================================================
// Action Types
//=============================================================================

/** Action target - matches Rust ActionTarget struct */
export interface ActionTarget {
  /** Element ID (e.g., "e1", "e2" from perceive.elements) */
  elementId?: string;

  /** CSS selector */
  selector?: string;

  /** Text content to match */
  text?: string;

  /** Aria role */
  role?: string;

  /** Aria label */
  label?: string;

  /** Coordinates (x, y) */
  coordinates?: [number, number];
}

/** Click target - element ID, selector, or role+label object */
export type ClickTarget = string | { role: string; label: string };

/** Click options - matches Rust ClickOptions struct */
export interface ClickOptions {
  /** Button: "left", "right", "middle" */
  button?: "left" | "right" | "middle";

  /** Number of clicks (1 for single, 2 for double) */
  clickCount?: number;

  /** Delay between mousedown and mouseup in ms */
  delayMs?: number;

  /** Modifier keys to hold */
  modifiers?: string[];
}

/** Type options - matches Rust TypeOptions struct */
export interface TypeOptions {
  /** Delay between keystrokes in ms */
  delayMs?: number;

  /** Clear existing content first */
  clearFirst?: boolean;
}

/** Scroll direction */
export type ScrollDirection = "up" | "down" | "left" | "right";

/** Scroll options - matches Rust ScrollOptions struct */
export interface ScrollOptions {
  /** Scroll direction */
  direction: ScrollDirection;

  /** Amount to scroll in pixels */
  amount?: number;

  /** Smooth scrolling */
  smooth?: boolean;
}

/** Wait condition - matches Rust WaitCondition enum */
export type WaitCondition =
  | { type: "Element"; selector: string }
  | { type: "Visible"; selector: string }
  | { type: "Hidden"; selector: string }
  | { type: "Navigation" }
  | { type: "NetworkIdle"; idleTimeMs: number }
  | { type: "Delay"; durationMs: number };

/** Action result - matches Rust ActionResult struct */
export interface ActionResult {
  /** Whether the action succeeded */
  success: boolean;

  /** Updated page state after action */
  pageState?: PageState;

  /** Action-specific result data */
  data?: unknown;

  /** Duration of the action in milliseconds */
  durationMs: number;
}

/** Navigation result */
export interface NavigationResult {
  /** Final URL after navigation */
  url: string;

  /** HTTP status code */
  status?: number;
}

//=============================================================================
// Request/Response Parameter Types
//=============================================================================

/** Click params */
export interface ClickParams {
  target: ActionTarget;
  button?: string;
  clickCount?: number;
  delayMs?: number;
  modifiers?: string[];
}

/** Type params */
export interface TypeParams {
  text: string;
  target?: ActionTarget;
  delayMs?: number;
  clearFirst?: boolean;
}

/** Scroll params */
export interface ScrollParams {
  target?: ActionTarget;
  direction?: ScrollDirection;
  amount?: number;
  smooth?: boolean;
}

/** Press params */
export interface PressParams {
  key: string;
  modifiers?: string[];
}

/** Select params */
export interface SelectParams {
  target: ActionTarget;
  value: string;
}

/** Wait params */
export interface WaitParams {
  condition: WaitCondition;
  timeoutMs?: number;
}

/** Find elements params */
export interface FindElementsParams {
  selector: string;
}

//=============================================================================
// Client Options
//=============================================================================

/** Client connection options */
export interface ClientOptions {
  /** WebSocket URL (default: ws://localhost:9876) */
  url?: string;

  /** Request timeout in ms (default: 30000) */
  timeout?: number;

  /** Auto-reconnect on disconnect with exponential backoff */
  autoReconnect?: boolean;

  /** Initial reconnect delay in ms (default: 1000, doubles up to maxReconnectDelay) */
  reconnectDelay?: number;

  /** Maximum reconnect delay in ms (default: 30000) */
  maxReconnectDelay?: number;

  /** Callback invoked when reconnection succeeds */
  onReconnect?: () => void;
}
