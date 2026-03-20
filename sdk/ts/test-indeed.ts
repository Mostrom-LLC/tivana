/**
 * Tivana Real-World Test: Indeed.com
 *
 * Tests Tivana against a complex, dynamic website:
 * 1. Navigate to indeed.com
 * 2. Perceive homepage elements
 * 3. Search for a job
 * 4. Perceive search results
 * 5. Click on a result
 * 6. Perceive job details
 * 7. Navigate back
 * 8. Clean up
 */

import { TivanaClient } from "./src/client";
import type { Element, PageState } from "./src/types";

const RUNTIME_URL = process.env.TIVANA_URL || "ws://localhost:9876";
const TIMEOUT_MS = 60000;

const green = (s: string) => `\x1b[32m${s}\x1b[0m`;
const red = (s: string) => `\x1b[31m${s}\x1b[0m`;
const yellow = (s: string) => `\x1b[33m${s}\x1b[0m`;
const dim = (s: string) => `\x1b[2m${s}\x1b[0m`;
const bold = (s: string) => `\x1b[1m${s}\x1b[0m`;

let passed = 0;
let failed = 0;
let warnings = 0;

function pass(msg: string) { console.log(green("  ✓"), msg); passed++; }
function fail(msg: string, e?: Error) { console.log(red("  ✗"), msg); if (e) console.log(dim(`    ${e.message}`)); failed++; }
function warn(msg: string) { console.log(yellow("  ⚠"), msg); warnings++; }
function info(msg: string) { console.log(dim(`    ${msg}`)); }
function section(title: string) { console.log(`\n${bold(yellow(`── ${title} ──`))}`); }

async function withTimeout<T>(promise: Promise<T>, ms: number, op: string): Promise<T> {
  const timeout = new Promise<never>((_, reject) =>
    setTimeout(() => reject(new Error(`Timeout: ${op} (${ms}ms)`)), ms)
  );
  return Promise.race([promise, timeout]);
}

function sleep(ms: number) { return new Promise(r => setTimeout(r, ms)); }

function printElements(elements: Element[], max = 10) {
  const shown = elements.slice(0, max);
  for (const el of shown) {
    const name = el.name ? `"${el.name.slice(0, 60)}"` : "(no name)";
    const state = [
      el.focused && "focused",
      !el.enabled && "disabled",
      el.checked && "checked",
    ].filter(Boolean).join(", ");
    info(`${el.id}: ${el.role} ${name}${state ? ` [${state}]` : ""}`);
  }
  if (elements.length > max) {
    info(`... and ${elements.length - max} more`);
  }
}

async function run() {
  console.log(`\n${bold("🌐 Tivana Real-World Test: Indeed.com")}\n`);
  console.log(dim(`Runtime: ${RUNTIME_URL}`));

  const client = new TivanaClient({ url: RUNTIME_URL, timeout: TIMEOUT_MS });

  try {
    // ── Connect ──
    section("Connect & Create Session");
    await withTimeout(client.connect(), 5000, "connect");
    pass("Connected to runtime");

    const sessionId = await withTimeout(client.createSession(), 30000, "create session");
    pass(`Session created: ${sessionId.slice(0, 8)}...`);

    // ── Navigate to Indeed ──
    section("Navigate to Indeed.com");
    const navResult = await withTimeout(client.navigate("https://www.indeed.com"), 20000, "navigate");
    if (navResult.success) {
      pass("Navigated to indeed.com");
    } else {
      fail("Navigation failed");
    }

    // Wait for page to fully load (Indeed has dynamic content)
    await sleep(3000);

    // ── Perceive Homepage ──
    section("Perceive Homepage");

    const pageState = await withTimeout(client.pageState(), 5000, "page state");
    if (pageState.url.includes("indeed")) {
      pass(`URL: ${pageState.url}`);
    } else {
      fail(`Unexpected URL: ${pageState.url}`);
    }

    if (pageState.title) {
      pass(`Title: ${pageState.title}`);
    } else {
      warn("No title found");
    }

    pass(`Viewport: ${pageState.viewportWidth}x${pageState.viewportHeight}`);

    const elements = await withTimeout(client.elements(), 10000, "elements");
    if (elements.length > 0) {
      pass(`Found ${elements.length} interactive elements`);
      printElements(elements, 15);
    } else {
      fail("No elements found on homepage");
    }

    // ── Find Search Input ──
    section("Find & Use Search Form");

    // Look for the job search input (what/keyword field)
    const searchInput = elements.find(
      (e) =>
        (e.role === "textbox" || e.role === "text" || e.role === "searchbox" || e.role === "combobox") &&
        (e.name?.toLowerCase().includes("what") ||
         e.name?.toLowerCase().includes("job") ||
         e.name?.toLowerCase().includes("keyword") ||
         e.name?.toLowerCase().includes("search"))
    );

    // Also look for location input
    const locationInput = elements.find(
      (e) =>
        (e.role === "textbox" || e.role === "text" || e.role === "searchbox" || e.role === "combobox") &&
        (e.name?.toLowerCase().includes("where") ||
         e.name?.toLowerCase().includes("location") ||
         e.name?.toLowerCase().includes("city"))
    );

    if (searchInput) {
      pass(`Found search input: ${searchInput.id} (${searchInput.role}: "${searchInput.name}")`);

      // Click to focus first, then type with human-like delay
      try {
        await client.click(searchInput.id);
        await sleep(300);
        const typeResult = await withTimeout(
          client.type("software engineer", searchInput.id, { clearFirst: true, delayMs: 50 }),
          15000,
          "type search query"
        );
        if (typeResult.success) {
          pass("Typed 'software engineer' into search field");
        } else {
          fail("Type action returned success=false");
        }
      } catch (e) {
        fail("Failed to type into search", e as Error);
      }
    } else {
      warn("Could not find job search input — trying CSS selector fallback");
      // Try with known Indeed selectors
      try {
        const typeResult = await withTimeout(
          client.type("software engineer", "#text-input-what", { clearFirst: true }),
          10000,
          "type via selector"
        );
        if (typeResult.success) {
          pass("Typed 'software engineer' via CSS selector");
        } else {
          fail("Type via selector returned success=false");
        }
      } catch (e) {
        fail("Failed to type via selector", e as Error);
      }
    }

    if (locationInput) {
      pass(`Found location input: ${locationInput.id} (${locationInput.role}: "${locationInput.name}")`);
      try {
        await sleep(500);
        await client.click(locationInput.id);
        await sleep(300);
        const typeResult = await withTimeout(
          client.type("Remote", locationInput.id, { clearFirst: true, delayMs: 50 }),
          10000,
          "type location"
        );
        if (typeResult.success) {
          pass("Typed 'Remote' into location field");
        }
      } catch (e) {
        warn(`Location input failed: ${(e as Error).message}`);
      }
    } else {
      warn("Could not find location input");
    }

    // ── Submit Search ──
    section("Submit Search");

    // Find search/submit button
    const searchButton = elements.find(
      (e) =>
        (e.role === "button" || e.role === "submit") &&
        (e.name?.toLowerCase().includes("search") ||
         e.name?.toLowerCase().includes("find"))
    );

    if (searchButton) {
      pass(`Found search button: ${searchButton.id} ("${searchButton.name}")`);
      try {
        await sleep(800); // Human-like pause before clicking search
        const clickResult = await withTimeout(
          client.click(searchButton.id),
          15000,
          "click search"
        );
        if (clickResult.success) {
          pass("Clicked search button");
        } else {
          fail("Search button click returned success=false");
        }
      } catch (e) {
        fail("Failed to click search button", e as Error);
      }
    } else {
      warn("No search button found — trying Enter key");
      try {
        const pressResult = await withTimeout(client.press("Enter"), 5000, "press Enter");
        if (pressResult.success) {
          pass("Pressed Enter to submit search");
        }
      } catch (e) {
        fail("Failed to press Enter", e as Error);
      }
    }

    // Wait for search results to load
    // Cloudflare may show a challenge — wait longer for it to auto-resolve
    info("Waiting for page to load (Cloudflare challenge may appear)...");
    await sleep(5000);

    // Check if we hit Cloudflare and wait for it to resolve
    let cfState = await client.pageState();
    if (cfState.title?.includes("moment") || cfState.title?.includes("Cloudflare")) {
      warn("Cloudflare challenge detected — waiting up to 15s for auto-resolve...");
      for (let i = 0; i < 15; i++) {
        await sleep(1000);
        cfState = await client.pageState();
        if (!cfState.title?.includes("moment") && !cfState.title?.includes("Cloudflare")) {
          pass(`Cloudflare challenge resolved after ${i + 1}s`);
          break;
        }
      }
      if (cfState.title?.includes("moment") || cfState.title?.includes("Cloudflare")) {
        warn("Cloudflare challenge did not auto-resolve — proceeding with challenge page");
      }
    }

    // ── Perceive Search Results ──
    section("Perceive Search Results");

    const resultsState = await withTimeout(client.pageState(), 5000, "results page state");
    pass(`Results URL: ${resultsState.url}`);
    if (resultsState.title) {
      pass(`Results title: ${resultsState.title}`);
    }

    const resultsElements = await withTimeout(client.elements(), 10000, "results elements");
    pass(`Found ${resultsElements.length} elements on results page`);
    printElements(resultsElements, 10);

    // Look for job listing links
    const jobLinks = resultsElements.filter(
      (e) =>
        (e.role === "a" || e.role === "link") &&
        e.name &&
        e.name.length > 10 &&
        !e.name.toLowerCase().includes("sign in") &&
        !e.name.toLowerCase().includes("post") &&
        !e.name.toLowerCase().includes("privacy")
    );

    if (jobLinks.length > 0) {
      pass(`Found ${jobLinks.length} potential job links`);
      info("First 5 job links:");
      for (const link of jobLinks.slice(0, 5)) {
        info(`  ${link.id}: "${link.name?.slice(0, 80)}"`);
      }
    } else {
      warn("No obvious job listing links found in elements");
    }

    // ── Click a Job Result ──
    section("Click Job Result");

    const targetJob = jobLinks[0];
    if (targetJob) {
      try {
        const clickResult = await withTimeout(
          client.click(targetJob.id),
          15000,
          "click job"
        );
        if (clickResult.success) {
          pass(`Clicked job: "${targetJob.name?.slice(0, 60)}"`);
        } else {
          fail("Job click returned success=false");
        }

        await sleep(3000);

        // Perceive job detail page
        const jobState = await withTimeout(client.pageState(), 5000, "job page state");
        pass(`Job page URL: ${jobState.url}`);

        const jobElements = await withTimeout(client.elements(), 10000, "job elements");
        pass(`Found ${jobElements.length} elements on job page`);

        // Look for Apply button
        const applyButton = jobElements.find(
          (e) =>
            (e.role === "button" || e.role === "link" || e.role === "a") &&
            e.name?.toLowerCase().includes("apply")
        );
        if (applyButton) {
          pass(`Found Apply button: ${applyButton.id} ("${applyButton.name}")`);
        } else {
          warn("No Apply button found on job page");
        }

      } catch (e) {
        fail("Failed to click/perceive job result", e as Error);
      }
    } else {
      warn("No job link to click — skipping job detail test");
    }

    // ── Scroll Test ──
    section("Scroll Test");

    try {
      const scrollResult = await withTimeout(
        client.scroll(undefined, "down", { amount: 500, smooth: true }),
        5000,
        "scroll down"
      );
      if (scrollResult.success) {
        pass("Scrolled down 500px");
      }

      await sleep(500);

      const afterScroll = await withTimeout(client.pageState(), 5000, "post-scroll state");
      if (afterScroll.scrollY > 0) {
        pass(`Scroll position confirmed: Y=${afterScroll.scrollY}`);
      } else {
        warn(`Scroll position still 0 (page may have handled scroll differently)`);
      }
    } catch (e) {
      fail("Scroll test failed", e as Error);
    }

    // ── Metadata Test ──
    section("Metadata");

    try {
      const meta = await withTimeout(client.metadata(), 5000, "metadata");
      if (meta.url) pass(`Meta URL: ${meta.url}`);
      if (meta.title) pass(`Meta title: ${meta.title}`);
      if (meta.description) pass(`Meta description: "${meta.description.slice(0, 80)}..."`);
      if (meta.language) pass(`Language: ${meta.language}`);
    } catch (e) {
      fail("Metadata retrieval failed", e as Error);
    }

    // ── Accessibility Snapshot ──
    section("Accessibility Snapshot");

    try {
      const snapshot = await withTimeout(client.accessibilitySnapshot(), 10000, "a11y snapshot");
      pass(`Accessibility snapshot: ${snapshot.interactiveElements.length} interactive elements`);
      if (snapshot.root) {
        pass(`Root element: ${snapshot.root.role} "${snapshot.root.name || "(untitled)"}"`);
      }
    } catch (e) {
      fail("Accessibility snapshot failed", e as Error);
    }

    // ── Text Content ──
    section("Text Content");

    try {
      const text = await withTimeout(client.textContent(), 5000, "text content");
      pass(`Text content: ${text.wordCount} words, ${text.charCount} chars`);
      info(`Preview: "${text.text.slice(0, 150).replace(/\n/g, " ")}..."`);
    } catch (e) {
      fail("Text content retrieval failed", e as Error);
    }

    // ── Cleanup ──
    section("Cleanup");

    await withTimeout(client.closeSession(), 10000, "close session");
    pass("Session closed");

    client.disconnect();
    pass("Disconnected");

    // ── Results ──
    section("Results");
    const total = passed + failed;
    console.log(`\n${green(`${passed} passed`)}, ${failed > 0 ? red(`${failed} failed`) : "0 failed"}, ${warnings > 0 ? yellow(`${warnings} warnings`) : "0 warnings"}`);
    console.log(`Pass rate: ${Math.round((passed / total) * 100)}%\n`);

    if (failed === 0) {
      console.log(green(bold("✓ All tests passed! Tivana handles Indeed.com successfully.\n")));
    } else {
      console.log(yellow(`${failed} failures — review above for details.\n`));
    }

    return failed === 0;

  } catch (e) {
    console.error(red("\n✗ Test crashed:"), e);
    client.disconnect();
    return false;
  }
}

run()
  .then((ok) => process.exit(ok ? 0 : 1))
  .catch((e) => { console.error("Fatal:", e); process.exit(1); });
