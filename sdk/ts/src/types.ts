/**
 * Tivana SDK Types
 *
 * Type definitions for the Tivana protocol - streaming browser perception for AI agents.
 */

//=============================================================================
// Protocol Types
//=============================================================================

/** Protocol version */
export const PROTOCOL_VERSION = "1.0";

/** Message type discriminator */
export type MessageType = "request" | "response" | "event" | "error";

/** Incoming message from runtime */
export interface IncomingMessage {
  id: string;
  type: MessageType;
  method?: string;
  sessionId?: string;
  result?: unknown;
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
}

/** Protocol error */
export interface ProtocolError {
  code: ErrorCode;
  message: string;
  data?: unknown;
}

/** Error codes */
export enum ErrorCode {
  // Protocol errors (1xxx)
  InvalidMessage = 1001,
  MissingField = 1002,
  InvalidField = 1003,
  UnknownMethod = 1004,

  // Session errors (2xxx)
  SessionNotFound = 2001,
  SessionClosed = 2002,
  SessionExists = 2003,
  InvalidSessionState = 2004,

  // Browser errors (3xxx)
  BrowserLaunchFailed = 3001,
  BrowserCrashed = 3002,
  BrowserDisconnected = 3003,
  NavigationFailed = 3004,

  // Action errors (4xxx)
  TargetNotFound = 4001,
  TargetAmbiguous = 4002,
  ActionFailed = 4003,
  ActionTimeout = 4004,

  // Perception errors (5xxx)
  PerceptionFailed = 5001,
  ElementNotAccessible = 5002,
  StyleComputationFailed = 5003,

  // Internal errors (9xxx)
  InternalError = 9001,
}

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

//=============================================================================
// Page State Types
//=============================================================================

/** Page state snapshot */
export interface PageState {
  /** Current URL */
  url: string;

  /** Page title */
  title: string;

  /** Currently focused element ID */
  focusedElement: string | null;

  /** Scroll position */
  scrollPosition: ScrollPosition;

  /** Viewport dimensions */
  viewport: Viewport;

  /** Capture timestamp (ms since epoch) */
  timestamp: number;
}

/** Scroll position */
export interface ScrollPosition {
  x: number;
  y: number;
}

/** Viewport dimensions */
export interface Viewport {
  width: number;
  height: number;
}

//=============================================================================
// Element Types
//=============================================================================

/** Element in the page tree */
export interface Element {
  /** Stable element ID (e.g., "e1", "e2") */
  id: string;

  // === Semantic ===
  /** Accessibility role */
  role: string;
  /** Accessible name/label */
  label: string;
  /** Current value (form elements) */
  value?: string;
  /** Visible text content */
  text?: string;

  // === State ===
  /** Element has focus */
  focused: boolean;
  /** Element is enabled */
  enabled: boolean;
  /** Element is visible in viewport */
  visible: boolean;
  /** Element can receive interactions */
  interactable: boolean;

  // === Geometry ===
  /** Bounding rectangle */
  bounds: Bounds;
  /** Padding */
  padding?: Spacing;
  /** Margin */
  margin?: Spacing;

  // === Typography ===
  /** Font properties */
  font?: FontStyle;
  /** Text alignment */
  textAlign?: string;

  // === Colors ===
  /** Background color */
  background?: string;

  // === Borders ===
  /** Border properties */
  border?: BorderStyle;

  // === Layout ===
  /** Display property */
  display?: string;
  /** Flex direction */
  flexDirection?: string;
  /** Justify content */
  justifyContent?: string;
  /** Align items */
  alignItems?: string;

  // === Visual State ===
  /** Opacity (0-1) */
  opacity?: number;
  /** Cursor style */
  cursor?: string;
  /** Overflow */
  overflow?: string;

  // === Accessibility ===
  /** Contrast ratio */
  contrastRatio?: number;
  /** Has visible focus indicator */
  focusVisible?: boolean;
  /** Tab index */
  tabIndex?: number;
  /** ARIA attributes */
  ariaAttributes?: Record<string, string>;
  /** Heading level (1-6) */
  headingLevel?: number;
  /** Alt text (images) */
  altText?: string;

  // === Hierarchy ===
  /** Child elements */
  children?: Element[];
}

/** Bounding rectangle */
export interface Bounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Spacing (padding/margin) */
export interface Spacing {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

/** Font style */
export interface FontStyle {
  family: string;
  size: string;
  weight: number;
  color: string;
  lineHeight?: string;
}

/** Border style */
export interface BorderStyle {
  width: string;
  style: string;
  color: string;
  radius?: string;
}

//=============================================================================
// Mutation Types
//=============================================================================

/** Mutation event */
export type Mutation =
  | { type: "added"; element: Element }
  | { type: "removed"; elementId: string }
  | { type: "changed"; elementId: string; changes: Record<string, unknown> }
  | {
      type: "focusChanged";
      previousElement: string | null;
      currentElement: string | null;
    }
  | { type: "navigation"; url: string };

/** Mutation event with timestamp */
export interface MutationEvent {
  mutations: Mutation[];
  timestamp: number;
}

//=============================================================================
// Action Types
//=============================================================================

/** Click target - element ID or selector */
export type ClickTarget = string | { role: string; label: string };

/** Mouse button */
export type MouseButton = "left" | "right" | "middle";

/** Click params */
export interface ClickParams {
  target: ClickTarget;
  button?: MouseButton;
  clickCount?: number;
}

/** Type params */
export interface TypeParams {
  text: string;
  target?: string;
}

/** Scroll behavior */
export type ScrollBehavior = "smooth" | "instant";

/** Scroll params */
export interface ScrollParams {
  target: string;
  behavior?: ScrollBehavior;
}

/** Action result */
export interface ActionResult {
  success: boolean;
  error?: string;
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

  /** Auto-reconnect on disconnect */
  autoReconnect?: boolean;

  /** Reconnect delay in ms (default: 1000) */
  reconnectDelay?: number;
}

/** Mutation callback */
export type MutationCallback = (event: MutationEvent) => void;
