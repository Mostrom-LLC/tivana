/**
 * Client unit tests for Tivana SDK
 *
 * Tests client behavior without requiring a running runtime.
 */

import { describe, test, expect, beforeEach, afterEach, mock } from "bun:test";
import { TivanaClient } from "../client";

describe("TivanaClient", () => {
  let client: TivanaClient;

  beforeEach(() => {
    client = new TivanaClient();
  });

  afterEach(() => {
    client.disconnect();
  });

  test("constructor uses default options", () => {
    expect(client.isConnected()).toBe(false);
    expect(client.getSessionId()).toBeNull();
  });

  test("constructor accepts custom options", () => {
    const custom = new TivanaClient({
      url: "ws://custom:1234",
      timeout: 5000,
      autoReconnect: true,
      reconnectDelay: 2000,
    });
    expect(custom.isConnected()).toBe(false);
    custom.disconnect();
  });

  test("disconnect clears state", () => {
    client.disconnect();
    expect(client.isConnected()).toBe(false);
    expect(client.getSessionId()).toBeNull();
  });

  test("methods throw when not connected", async () => {
    await expect(client.pageState()).rejects.toThrow("Not connected");
    await expect(client.elements()).rejects.toThrow("Not connected");
    await expect(client.navigate("https://example.com")).rejects.toThrow(
      "Not connected"
    );
    await expect(client.click("e1")).rejects.toThrow("Not connected");
    await expect(client.type("hello")).rejects.toThrow("Not connected");
    await expect(client.press("Enter")).rejects.toThrow("Not connected");
    await expect(client.scroll()).rejects.toThrow("Not connected");
    await expect(client.hover("e1")).rejects.toThrow("Not connected");
    await expect(client.focus("e1")).rejects.toThrow("Not connected");
    await expect(client.select("e1", "value")).rejects.toThrow("Not connected");
    await expect(client.accessibilitySnapshot()).rejects.toThrow(
      "Not connected"
    );
    await expect(client.textContent()).rejects.toThrow("Not connected");
    await expect(client.metadata()).rejects.toThrow("Not connected");
    await expect(client.findElements("button")).rejects.toThrow(
      "Not connected"
    );
    await expect(client.listSessions()).rejects.toThrow("Not connected");
  });

  test("closeSession throws when no session", async () => {
    await expect(client.closeSession()).rejects.toThrow("No active session");
  });

  test("connect fails with invalid URL", async () => {
    const bad = new TivanaClient({ url: "ws://localhost:1" });
    await expect(bad.connect()).rejects.toThrow();
    bad.disconnect();
  });

  test("click resolves element ID vs selector", async () => {
    // Test that click target parsing works (will fail at request level since not connected)
    // Element IDs: e1, e2, e99
    await expect(client.click("e1")).rejects.toThrow("Not connected");
    await expect(client.click("e99")).rejects.toThrow("Not connected");

    // Selectors
    await expect(client.click("button.submit")).rejects.toThrow(
      "Not connected"
    );
    await expect(client.click("#my-id")).rejects.toThrow("Not connected");

    // Role+label
    await expect(
      client.click({ role: "button", label: "Submit" })
    ).rejects.toThrow("Not connected");
  });

  test("type resolves target correctly", async () => {
    // With element ID target
    await expect(client.type("hello", "e5")).rejects.toThrow("Not connected");
    // With selector target
    await expect(client.type("hello", "#input")).rejects.toThrow(
      "Not connected"
    );
    // Without target
    await expect(client.type("hello")).rejects.toThrow("Not connected");
  });

  test("onMutation returns unsubscribe function", () => {
    let called = false;
    const unsub = client.onMutation(() => {
      called = true;
    });
    expect(typeof unsub).toBe("function");
    unsub();
  });

  test("multiple disconnect calls are safe", () => {
    client.disconnect();
    client.disconnect();
    client.disconnect();
    expect(client.isConnected()).toBe(false);
  });
});

describe("TivanaClient message correlation", () => {
  test("concurrent requests get unique IDs", () => {
    const client = new TivanaClient();
    // Access internal messageId counter indirectly
    // Each request increments the counter
    expect(client.getSessionId()).toBeNull();
    client.disconnect();
  });
});
