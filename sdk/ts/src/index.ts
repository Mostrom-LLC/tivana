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

  // Session
  SessionStatus,
  SessionCreateParams,
  SessionCreateResult,

  // Page State
  PageState,
  ScrollPosition,
  Viewport,

  // Elements
  Element,
  Bounds,
  Spacing,
  FontStyle,
  BorderStyle,

  // Mutations
  Mutation,
  MutationEvent,
  MutationCallback,

  // Actions
  ClickTarget,
  MouseButton,
  ClickParams,
  TypeParams,
  ScrollBehavior,
  ScrollParams,
  ActionResult,

  // Options
  ClientOptions,
} from "./types";

export { ErrorCode, PROTOCOL_VERSION } from "./types";
