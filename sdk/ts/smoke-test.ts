/**
 * Tivana SDK Smoke Test
 *
 * This script tests the full Tivana flow:
 * 1. Connect to runtime
 * 2. Create a session (launches browser)
 * 3. Navigate to a page
 * 4. Get page state
 * 5. Get elements
 * 6. Interact with elements (click, type)
 * 7. Close session
 * 8. Disconnect
 *
 * Prerequisites:
 * - Tivana runtime running at ws://localhost:9876
 * - Start with: ./target/release/tivana start
 *
 * Run with: bun run smoke-test.ts
 *       or: npx tsx smoke-test.ts
 */

import { TivanaClient } from "./src/client";
import type { Element, PageState } from "./src/types";

const RUNTIME_URL = process.env.TIVANA_URL || "ws://localhost:9876";
const TIMEOUT_MS = 60000; // 60 seconds for browser operations

// Colors for output
const green = (s: string) => `\x1b[32m${s}\x1b[0m`;
const red = (s: string) => `\x1b[31m${s}\x1b[0m`;
const yellow = (s: string) => `\x1b[33m${s}\x1b[0m`;
const dim = (s: string) => `\x1b[2m${s}\x1b[0m`;

function pass(msg: string) {
  console.log(green("✓"), msg);
}

function fail(msg: string, error?: Error) {
  console.log(red("✗"), msg);
  if (error) {
    console.log(dim(`  ${error.message}`));
  }
}

function info(msg: string) {
  console.log(yellow("→"), msg);
}

function section(title: string) {
  console.log("\n" + yellow(`=== ${title} ===`));
}

async function withTimeout<T>(
  promise: Promise<T>,
  ms: number,
  operation: string
): Promise<T> {
  const timeout = new Promise<never>((_, reject) =>
    setTimeout(
      () => reject(new Error(`Timeout: ${operation} took longer than ${ms}ms`)),
      ms
    )
  );
  return Promise.race([promise, timeout]);
}

async function runSmokeTest(): Promise<boolean> {
  console.log("\n🔥 Tivana SDK Smoke Test\n");
  info(`Runtime URL: ${RUNTIME_URL}`);

  const client = new TivanaClient({
    url: RUNTIME_URL,
    timeout: TIMEOUT_MS,
  });

  let passed = 0;
  let failed = 0;

  try {
    // =========================================================================
    // Step 1: Connect to runtime
    // =========================================================================
    section("Connection");

    try {
      await withTimeout(client.connect(), 5000, "connect");
      pass("Connected to runtime");
      passed++;
    } catch (error) {
      fail("Failed to connect to runtime", error as Error);
      failed++;
      console.log(
        dim("\nMake sure the Tivana runtime is running:")
      );
      console.log(dim("  ./target/release/tivana start"));
      return false;
    }

    if (!client.isConnected()) {
      fail("Client reports not connected");
      failed++;
      return false;
    }

    // =========================================================================
    // Step 2: Create session (launches browser)
    // =========================================================================
    section("Session");

    let sessionId: string;
    try {
      sessionId = await withTimeout(
        client.createSession({ headless: true }),
        30000,
        "create session"
      );
      pass(`Created session: ${sessionId}`);
      passed++;
    } catch (error) {
      fail("Failed to create session", error as Error);
      failed++;
      client.disconnect();
      return false;
    }

    // Verify session ID
    if (client.getSessionId() === sessionId) {
      pass("Session ID stored correctly");
      passed++;
    } else {
      fail("Session ID mismatch");
      failed++;
    }

    // =========================================================================
    // Step 3: Navigate to page
    // =========================================================================
    section("Navigation");

    try {
      const navResult = await withTimeout(
        client.navigate("https://example.com"),
        15000,
        "navigate"
      );
      if (navResult.success) {
        pass("Navigated to https://example.com");
        passed++;
      } else {
        fail("Navigation returned success=false");
        failed++;
      }
    } catch (error) {
      fail("Failed to navigate", error as Error);
      failed++;
    }

    // =========================================================================
    // Step 4: Get page state
    // =========================================================================
    section("Page State");

    let pageState: PageState | null = null;
    try {
      pageState = await withTimeout(
        client.pageState(),
        5000,
        "get page state"
      );

      // Verify URL
      if (pageState.url.includes("example.com")) {
        pass(`URL: ${pageState.url}`);
        passed++;
      } else {
        fail(`Unexpected URL: ${pageState.url}`);
        failed++;
      }

      // Verify title
      if (pageState.title && pageState.title.includes("Example")) {
        pass(`Title: ${pageState.title}`);
        passed++;
      } else {
        fail(`Unexpected title: ${pageState.title}`);
        failed++;
      }

      // Verify viewport
      if (pageState.viewportWidth > 0 && pageState.viewportHeight > 0) {
        pass(`Viewport: ${pageState.viewportWidth}x${pageState.viewportHeight}`);
        passed++;
      } else {
        fail("Invalid viewport dimensions");
        failed++;
      }

      // Verify timestamp
      if (pageState.timestampMs > 0) {
        pass(`Timestamp: ${new Date(pageState.timestampMs).toISOString()}`);
        passed++;
      } else {
        fail("Missing timestamp");
        failed++;
      }
    } catch (error) {
      fail("Failed to get page state", error as Error);
      failed++;
    }

    // =========================================================================
    // Step 5: Get elements
    // =========================================================================
    section("Elements");

    let elements: Element[] = [];
    try {
      elements = await withTimeout(client.elements(), 5000, "get elements");

      if (elements.length > 0) {
        pass(`Found ${elements.length} interactive elements`);
        passed++;

        // Log first few elements
        const sample = elements.slice(0, 5);
        for (const el of sample) {
          console.log(
            dim(`  ${el.id}: ${el.role} "${el.name || "(no name)"}"`)
          );
        }

        // Verify element structure
        const firstEl = elements[0];
        if (firstEl.id && firstEl.role !== undefined) {
          pass("Elements have required fields (id, role)");
          passed++;
        } else {
          fail("Elements missing required fields");
          failed++;
        }

        // Check for bounds
        const elWithBounds = elements.find((e) => e.bounds);
        if (elWithBounds) {
          pass(
            `Elements have bounds: ${JSON.stringify(elWithBounds.bounds)}`
          );
          passed++;
        } else {
          info("No elements with bounds found (may be expected)");
        }
      } else {
        fail("No elements found");
        failed++;
      }
    } catch (error) {
      fail("Failed to get elements", error as Error);
      failed++;
    }

    // =========================================================================
    // Step 6: Click a link
    // =========================================================================
    section("Click Action");

    // Look for the "More information..." link on example.com
    const link = elements.find(
      (e) =>
        e.role === "a" ||
        e.role === "link" ||
        (e.name && e.name.toLowerCase().includes("more information"))
    );

    if (link) {
      try {
        const clickResult = await withTimeout(
          client.click(link.id),
          10000,
          "click"
        );

        if (clickResult.success) {
          pass(`Clicked element: ${link.id} (${link.role}: "${link.name}")`);
          passed++;

          // Wait a moment for navigation
          await new Promise((resolve) => setTimeout(resolve, 2000));

          // Check if URL changed
          const newState = await client.pageState();
          if (newState.url !== pageState?.url) {
            pass(`Navigation occurred: ${newState.url}`);
            passed++;
          } else {
            info("URL did not change (link may be same-page or blocked)");
          }
        } else {
          fail("Click returned success=false");
          failed++;
        }
      } catch (error) {
        fail("Failed to click", error as Error);
        failed++;
      }
    } else {
      info("No clickable link found, skipping click test");
    }

    // Navigate back to example.com for typing test
    try {
      await client.navigate("https://example.com");
    } catch {
      // Ignore navigation errors
    }

    // =========================================================================
    // Step 7: Type action (test typing even without input)
    // =========================================================================
    section("Type Action");

    // Look for an input element
    const input = elements.find(
      (e) =>
        e.role === "textbox" ||
        e.role === "input" ||
        e.role === "text" ||
        e.role === "searchbox"
    );

    if (input) {
      try {
        const typeResult = await withTimeout(
          client.type("Hello Tivana!", input.id),
          5000,
          "type"
        );

        if (typeResult.success) {
          pass(`Typed into element: ${input.id}`);
          passed++;
        } else {
          fail("Type returned success=false");
          failed++;
        }
      } catch (error) {
        fail("Failed to type", error as Error);
        failed++;
      }
    } else {
      info("No input element found on example.com, testing type without target");

      // Test typing without a target (should type into focused element or fail gracefully)
      try {
        const typeResult = await withTimeout(
          client.type("Test"),
          5000,
          "type without target"
        );
        info(`Type without target: success=${typeResult.success}`);
      } catch (error) {
        info(`Type without target failed (expected): ${(error as Error).message}`);
      }
    }

    // =========================================================================
    // Step 8: Test press action
    // =========================================================================
    section("Press Action");

    try {
      const pressResult = await withTimeout(
        client.press("Tab"),
        5000,
        "press Tab"
      );

      if (pressResult.success) {
        pass("Pressed Tab key");
        passed++;
      } else {
        fail("Press returned success=false");
        failed++;
      }
    } catch (error) {
      fail("Failed to press key", error as Error);
      failed++;
    }

    // =========================================================================
    // Step 9: Test scroll action
    // =========================================================================
    section("Scroll Action");

    try {
      const scrollResult = await withTimeout(
        client.scroll(undefined, "down", { amount: 100, smooth: true }),
        5000,
        "scroll"
      );

      if (scrollResult.success) {
        pass("Scrolled down 100px");
        passed++;
      } else {
        fail("Scroll returned success=false");
        failed++;
      }
    } catch (error) {
      fail("Failed to scroll", error as Error);
      failed++;
    }

    // =========================================================================
    // Step 10: Test metadata
    // =========================================================================
    section("Metadata");

    try {
      const metadata = await withTimeout(
        client.metadata(),
        5000,
        "get metadata"
      );

      if (metadata.url) {
        pass(`Metadata URL: ${metadata.url}`);
        passed++;
      }
      if (metadata.title) {
        pass(`Metadata title: ${metadata.title}`);
        passed++;
      }
    } catch (error) {
      fail("Failed to get metadata", error as Error);
      failed++;
    }

    // =========================================================================
    // Step 11: Close session
    // =========================================================================
    section("Cleanup");

    try {
      await withTimeout(client.closeSession(), 10000, "close session");
      pass("Closed session");
      passed++;
    } catch (error) {
      fail("Failed to close session", error as Error);
      failed++;
    }

    // Verify session cleared
    if (client.getSessionId() === null) {
      pass("Session ID cleared");
      passed++;
    } else {
      fail("Session ID not cleared");
      failed++;
    }

    // =========================================================================
    // Step 12: Disconnect
    // =========================================================================
    client.disconnect();

    if (!client.isConnected()) {
      pass("Disconnected from runtime");
      passed++;
    } else {
      fail("Still connected after disconnect");
      failed++;
    }

    // =========================================================================
    // Results
    // =========================================================================
    section("Results");

    const total = passed + failed;
    const passRate = Math.round((passed / total) * 100);

    console.log(`\n${green(`${passed} passed`)}, ${failed > 0 ? red(`${failed} failed`) : "0 failed"}`);
    console.log(`Pass rate: ${passRate}%`);

    if (failed === 0) {
      console.log(green("\n✓ All tests passed!\n"));
      return true;
    } else {
      console.log(red(`\n✗ ${failed} tests failed\n`));
      return false;
    }
  } catch (error) {
    console.error(red("\n✗ Smoke test crashed:"), error);
    client.disconnect();
    return false;
  }
}

// Run the smoke test
runSmokeTest()
  .then((success) => {
    process.exit(success ? 0 : 1);
  })
  .catch((error) => {
    console.error("Fatal error:", error);
    process.exit(1);
  });
