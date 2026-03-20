/**
 * Tivana SDK
 *
 * Streaming browser perception protocol for AI agents.
 *
 * @example
 * ```typescript
 * import { connect, observe, act } from 'tivana';
 *
 * // Connect to runtime
 * await connect();
 *
 * // Create session (launches browser)
 * const client = getClient();
 * await client.createSession();
 *
 * // Navigate
 * await act.navigate('https://example.com');
 *
 * // Observe page state
 * observe((page, elements) => {
 *   console.log(`URL: ${page.url}`);
 *   console.log(`Elements: ${elements.length}`);
 * });
 *
 * // Take actions
 * await act.click('e5');
 * await act.type('hello world');
 * ```
 */

// Client
export { TivanaClient, connect, getClient, observe, act } from "./client";

// Types
export type {
  // Protocol
  MessageType,
  IncomingMessage,
  OutgoingMessage,
  ProtocolError,
  ErrorCodeType,

  // Session
  SessionStatus,
  SessionCreateParams,
  SessionCreateResult,
  SessionInfo,

  // Page State
  PageState,
  BoundingBox,
  ElementStyles,

  // Elements
  Element,
  AccessibilitySnapshot,
  ElementInfo,
  TextContent,
  PageMetadata,

  // Mutations
  MutationEvent,
  MutationCallback,

  // Actions
  ActionTarget,
  ClickTarget,
  ClickOptions,
  ClickParams,
  TypeOptions,
  TypeParams,
  ScrollDirection,
  ScrollOptions,
  ScrollParams,
  PressParams,
  SelectParams,
  WaitCondition,
  WaitParams,
  ActionResult,
  NavigationResult,
  FindElementsParams,

  // Batch & Form Fill
  BatchAction,
  BatchResult,
  FormFillResult,

  // Options
  ClientOptions,
} from "./types";

export { ErrorCode, PROTOCOL_VERSION } from "./types";
export type { TabInfo } from "./types";
