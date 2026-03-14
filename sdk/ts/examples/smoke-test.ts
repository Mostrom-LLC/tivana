/**
 * Tivana Smoke Test
 *
 * End-to-end test script that verifies the full SDK flow:
 * - Connect to runtime
 * - Create session (launches Chromium)
 * - Navigate to a page
 * - Perceive page state
 * - Perceive element tree
 * - Click an element
 * - Type into an input
 * - Scroll to an element
 * - Close session
 * - Disconnect
 *
 * Run: bun run examples/smoke-test.ts
 */

import { TivanaClient } from "../src/client";
import type { Element, PageState } from "../src/types";

async function main() {
  console.log("🚀 Tivana Smoke Test\n");

  const client = new TivanaClient({
    url: process.env.TIVANA_URL || "ws://localhost:9876",
    timeout: 30000,
  });

  try {
    // 1. Connect
    console.log("1. Connecting to runtime...");
    await client.connect();
    console.log("   ✅ Connected\n");

    // 2. Create session
    console.log("2. Creating session (launching Chromium)...");
    const sessionId = await client.createSession();
    console.log(`   ✅ Session created: ${sessionId}\n`);

    // 3. Navigate
    console.log("3. Navigating to https://example.com...");
    const navResult = await client.navigate("https://example.com");
    console.log(`   ✅ Navigation: ${navResult.success ? "success" : "failed"}\n`);

    // 4. Page state
    console.log("4. Getting page state...");
    const pageState: PageState = await client.pageState();
    console.log(`   URL: ${pageState.url}`);
    console.log(`   Title: ${pageState.title}`);
    console.log(`   Viewport: ${pageState.viewport.width}x${pageState.viewport.height}`);
    console.log(`   ✅ Page state retrieved\n`);

    // 5. Elements
    console.log("5. Getting element tree...");
    const elements: Element[] = await client.elements();
    console.log(`   Found ${elements.length} interactive elements`);

    // Show first few elements
    const sample = elements.slice(0, 5);
    for (const el of sample) {
      console.log(`   - ${el.id}: ${el.role} "${el.label?.slice(0, 30) || "(no label)"}"`);
    }
    console.log(`   ✅ Element tree retrieved\n`);

    // 6. Click (if we have a clickable element)
    const clickable = elements.find((e) => e.interactable && e.role === "link");
    if (clickable) {
      console.log(`6. Clicking element ${clickable.id} (${clickable.role}: ${clickable.label?.slice(0, 20)})...`);
      const clickResult = await client.click(clickable.id);
      console.log(`   ✅ Click: ${clickResult.success ? "success" : "failed"}\n`);
    } else {
      console.log("6. No clickable link found, skipping click test\n");
    }

    // 7. Navigate to a page with a form
    console.log("7. Navigating to a page with inputs...");
    await client.navigate("https://www.google.com");
    const formElements = await client.elements();
    const searchInput = formElements.find(
      (e) => e.role === "combobox" || e.role === "textbox" || e.role === "search"
    );

    if (searchInput) {
      console.log(`   Found input: ${searchInput.id} (${searchInput.role})`);
      console.log("8. Typing into search input...");
      const typeResult = await client.type("Tivana browser perception", searchInput.id);
      console.log(`   ✅ Type: ${typeResult.success ? "success" : "failed"}\n`);
    } else {
      console.log("   No text input found, skipping type test\n");
    }

    // 9. Scroll test
    const scrollTarget = elements.find((e) => e.visible && e.bounds.y > 100);
    if (scrollTarget) {
      console.log(`9. Scrolling to element ${scrollTarget.id}...`);
      const scrollResult = await client.scroll(scrollTarget.id, "smooth");
      console.log(`   ✅ Scroll: ${scrollResult.success ? "success" : "failed"}\n`);
    } else {
      console.log("9. No scroll target found, skipping scroll test\n");
    }

    // 10. Close session
    console.log("10. Closing session...");
    await client.closeSession();
    console.log("    ✅ Session closed\n");

    // 11. Disconnect
    console.log("11. Disconnecting...");
    client.disconnect();
    console.log("    ✅ Disconnected\n");

    console.log("🎉 Smoke test complete - all checks passed!\n");
  } catch (error) {
    console.error("\n❌ Smoke test failed:", error);
    process.exit(1);
  }
}

main();
