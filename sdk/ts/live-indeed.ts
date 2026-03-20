/**
 * Tivana Live Test: Indeed.com
 * 
 * Interactive test — browser stays open until manually stopped.
 * Press Ctrl+C to close.
 */

import { TivanaClient } from "./src/client";

const RUNTIME_URL = process.env.TIVANA_URL || "ws://localhost:9876";

const green = (s: string) => `\x1b[32m${s}\x1b[0m`;
const yellow = (s: string) => `\x1b[33m${s}\x1b[0m`;
const dim = (s: string) => `\x1b[2m${s}\x1b[0m`;
const bold = (s: string) => `\x1b[1m${s}\x1b[0m`;

function sleep(ms: number) { return new Promise(r => setTimeout(r, ms)); }

async function run() {
  console.log(`\n${bold("🌐 Tivana Live Test: Indeed.com")}\n`);

  const client = new TivanaClient({ url: RUNTIME_URL, timeout: 60000 });

  // Cleanup on exit
  process.on("SIGINT", () => {
    console.log(yellow("\n\nCaught Ctrl+C — closing session..."));
    client.closeSession().then(() => {
      client.disconnect();
      console.log(green("Done. Browser closed."));
      process.exit(0);
    }).catch(() => {
      client.disconnect();
      process.exit(0);
    });
  });

  // Connect
  await client.connect();
  console.log(green("✓ Connected to runtime"));

  const sessionId = await client.createSession();
  console.log(green(`✓ Session: ${sessionId.slice(0, 8)}...`));
  console.log(dim("  (Browser window should be open now)\n"));

  // Step 1: Navigate
  console.log(yellow("── Navigate to Indeed.com ──"));
  await client.navigate("https://www.indeed.com");
  await sleep(3000);

  const state = await client.pageState();
  console.log(green(`✓ URL: ${state.url}`));
  console.log(green(`✓ Title: ${state.title}`));
  console.log(green(`✓ Viewport: ${state.viewportWidth}x${state.viewportHeight}`));

  // Step 2: Perceive elements
  console.log(yellow("\n── Homepage Elements ──"));
  const elements = await client.elements();
  console.log(green(`✓ Found ${elements.length} interactive elements`));
  for (const el of elements.slice(0, 20)) {
    const name = el.name ? `"${el.name.slice(0, 60)}"` : "(no name)";
    console.log(dim(`  ${el.id}: ${el.role} ${name}`));
  }
  if (elements.length > 20) console.log(dim(`  ... +${elements.length - 20} more`));

  // Step 3: Search
  console.log(yellow("\n── Search for 'software engineer' ──"));
  const searchInput = elements.find(e =>
    (e.role === "combobox" || e.role === "textbox" || e.role === "searchbox") &&
    (e.name?.toLowerCase().includes("job") || e.name?.toLowerCase().includes("what") || e.name?.toLowerCase().includes("search"))
  );

  if (searchInput) {
    console.log(green(`✓ Found: ${searchInput.id} — ${searchInput.role}: "${searchInput.name}"`));
    
    await client.click(searchInput.id);
    await sleep(500);
    await client.type("software engineer", searchInput.id, { clearFirst: true });
    await sleep(300);
    console.log(green("✓ Typed 'software engineer'"));
  } else {
    console.log(yellow("⚠ No search input found"));
  }

  // Location
  const locInput = elements.find(e =>
    (e.role === "combobox" || e.role === "textbox") &&
    (e.name?.toLowerCase().includes("where") || e.name?.toLowerCase().includes("location") || e.name?.toLowerCase().includes("edit location"))
  );

  if (locInput) {
    await sleep(500);
    await client.click(locInput.id);
    await sleep(300);
    await client.type("Remote", locInput.id, { clearFirst: true });
    console.log(green("✓ Typed 'Remote' in location"));
  }

  // Submit
  const searchBtn = elements.find(e =>
    (e.role === "button") && e.name?.toLowerCase().includes("search")
  );

  if (searchBtn) {
    await sleep(1000);
    await client.click(searchBtn.id);
    console.log(green("✓ Clicked Search"));
  } else {
    await client.press("Enter");
    console.log(green("✓ Pressed Enter"));
  }

  // Wait and check results
  console.log(yellow("\n── Waiting for results ──"));
  await sleep(5000);

  let resultsState = await client.pageState();
  console.log(dim(`  Title: ${resultsState.title}`));
  console.log(dim(`  URL: ${resultsState.url}`));

  // If Cloudflare, wait longer
  if (resultsState.title?.includes("moment") || resultsState.title?.includes("Cloudflare")) {
    console.log(yellow("⚠ Cloudflare challenge detected — waiting up to 20s..."));
    for (let i = 0; i < 20; i++) {
      await sleep(1000);
      resultsState = await client.pageState();
      if (!resultsState.title?.includes("moment") && !resultsState.title?.includes("Cloudflare")) {
        console.log(green(`✓ Challenge resolved after ${i + 1}s!`));
        break;
      }
      process.stdout.write(".");
    }
    console.log();
  }

  // Show results page elements
  console.log(yellow("\n── Results Page Elements ──"));
  const resultEls = await client.elements();
  console.log(green(`✓ Found ${resultEls.length} elements`));
  for (const el of resultEls.slice(0, 20)) {
    const name = el.name ? `"${el.name.slice(0, 80)}"` : "(no name)";
    console.log(dim(`  ${el.id}: ${el.role} ${name}`));
  }
  if (resultEls.length > 20) console.log(dim(`  ... +${resultEls.length - 20} more`));

  // Keep alive
  console.log(bold(green("\n\n✓ Live test complete. Browser stays open.")));
  console.log(yellow("Press Ctrl+C to close the browser and exit.\n"));

  // Keep process alive
  await new Promise(() => {});
}

run().catch(e => {
  console.error("Fatal:", e);
  process.exit(1);
});
