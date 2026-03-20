/**
 * Type and constant tests for Tivana SDK
 */

import { describe, test, expect } from "bun:test";
import { PROTOCOL_VERSION, ErrorCode } from "../types";

describe("Protocol constants", () => {
  test("PROTOCOL_VERSION is defined", () => {
    expect(PROTOCOL_VERSION).toBe("1.0");
  });

  test("ErrorCode has all expected codes", () => {
    // Protocol errors
    expect(ErrorCode.InvalidMessage).toBe("invalid_message");
    expect(ErrorCode.MissingField).toBe("missing_field");
    expect(ErrorCode.InvalidField).toBe("invalid_field");
    expect(ErrorCode.UnknownMethod).toBe("unknown_method");

    // Session errors
    expect(ErrorCode.SessionNotFound).toBe("session_not_found");
    expect(ErrorCode.SessionClosed).toBe("session_closed");
    expect(ErrorCode.SessionExists).toBe("session_exists");

    // Browser errors
    expect(ErrorCode.BrowserLaunchFailed).toBe("browser_launch_failed");
    expect(ErrorCode.BrowserCrashed).toBe("browser_crashed");
    expect(ErrorCode.BrowserDisconnected).toBe("browser_disconnected");
    expect(ErrorCode.NavigationFailed).toBe("navigation_failed");

    // Action errors
    expect(ErrorCode.TargetNotFound).toBe("target_not_found");
    expect(ErrorCode.TargetAmbiguous).toBe("target_ambiguous");
    expect(ErrorCode.ActionFailed).toBe("action_failed");
    expect(ErrorCode.ActionTimeout).toBe("action_timeout");

    // Perception errors
    expect(ErrorCode.PerceptionFailed).toBe("perception_failed");
    expect(ErrorCode.ElementNotAccessible).toBe("element_not_accessible");

    // Internal errors
    expect(ErrorCode.InternalError).toBe("internal_error");
  });
});
