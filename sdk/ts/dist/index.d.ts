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
export { TivanaClient, connect, getClient, observe, act } from "./client";
export type { MessageType, IncomingMessage, OutgoingMessage, ProtocolError, ErrorCodeType, SessionStatus, SessionCreateParams, SessionCreateResult, SessionInfo, PageState, BoundingBox, ElementStyles, Element, AccessibilitySnapshot, ElementInfo, TextContent, PageMetadata, MutationEvent, MutationCallback, PageEventType, PageLoadedEvent, PageNavigatedEvent, PageFocusEvent, PageScrollEvent, PageResizeEvent, PageEvent, PageEventCallback, ActionTarget, ClickTarget, ClickOptions, ClickParams, TypeOptions, TypeParams, ScrollDirection, ScrollOptions, ScrollParams, PressParams, SelectParams, WaitCondition, WaitParams, ActionResult, NavigationResult, FindElementsParams, BatchAction, BatchResult, FormFillResult, ClientOptions, } from "./types";
export { ErrorCode, PROTOCOL_VERSION } from "./types";
export type { TabInfo } from "./types";
//# sourceMappingURL=index.d.ts.map