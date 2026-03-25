/**
 * 06 — Event Streaming
 *
 * Real-time page event monitoring using Tivana's observation API.
 * Start observation, subscribe to events, and watch the page change
 * as the user (or another agent) interacts with it.
 *
 * This demonstrates the observation lifecycle:
 *   startObservation() → onEvent() → stopObservation()
 *
 * Works best with the Chrome extension (extension-backed sessions)
 * so you can interact with the page while the script watches.
 *
 * Usage: bun run examples/06-event-streaming.ts [seconds]
 */

import { TivanaClient, type PageEvent } from "tivana";

const duration = parseInt(process.argv[2] || "30") * 1000;

async function main() {
  const client = new TivanaClient({ url: "ws://localhost:9876" });
  await client.connect();

  // Try extension session first, fall back to managed
  let sessionType = "managed";
  try {
    const ext = await client.request<any>("session.fromExtension", {});
    (client as any).sessionId = ext.sessionId;
    sessionType = "extension";
  } catch {
    await client.createSession();
    await client.navigate("https://news.ycombinator.com");
  }

  const page = await client.pageState();
  console.log(`\n👁️  Event Streaming (${sessionType} session)`);
  console.log(`📄 ${page.title}`);
  console.log(`🔗 ${page.url}`);
  console.log(`⏱️  Monitoring for ${duration / 1000} seconds...\n`);

  // Start observation
  await client.startObservation();

  // Event counters
  const counts: Record<string, number> = {};
  let totalMutations = 0;

  // Subscribe to all events
  client.onEvent((event: PageEvent) => {
    const now = new Date().toISOString().slice(11, 23);
    counts[event.type] = (counts[event.type] || 0) + 1;

    switch (event.type) {
      case "page.mutation": {
        const mutations = event.data as any[];
        totalMutations += mutations.length;
        const added = mutations.filter((m) => m.type === "Added").length;
        const removed = mutations.filter((m) => m.type === "Removed").length;
        const changed = mutations.filter((m) => m.type === "Changed" || m.type === "TextChanged").length;
        console.log(
          `  ${now} [mutation] ${mutations.length} changes (${added} added, ${removed} removed, ${changed} changed)`
        );
        // Show first enriched Added event
        const firstAdded = mutations.find((m: any) => m.type === "Added" && m.role);
        if (firstAdded) {
          console.log(`           ↳ +${firstAdded.elementId} [${firstAdded.role}] "${(firstAdded.name || "").slice(0, 40)}"`);
        }
        break;
      }
      case "page.loaded": {
        const d = event.data as any;
        console.log(`  ${now} [loaded] ${d.url}`);
        break;
      }
      case "page.navigated": {
        const d = event.data as any;
        console.log(`  ${now} [navigated] ${d.url}`);
        if (d.previousUrl) console.log(`           ↳ from: ${d.previousUrl}`);
        break;
      }
      case "page.focus": {
        const d = event.data as any;
        console.log(`  ${now} [focus] ${d.elementId || "none"} [${d.role || "?"}] "${(d.name || "").slice(0, 30)}"`);
        break;
      }
      case "page.scroll": {
        const d = event.data as any;
        console.log(`  ${now} [scroll] x=${d.scrollX} y=${d.scrollY}`);
        break;
      }
      case "page.resize": {
        const d = event.data as any;
        console.log(`  ${now} [resize] ${d.viewportWidth}×${d.viewportHeight}`);
        break;
      }
    }
  });

  if (sessionType === "extension") {
    console.log(`  Interact with the browser tab to see events...\n`);
  }

  // Wait for the duration
  await new Promise((r) => setTimeout(r, duration));

  // --- Summary ---
  console.log(`\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);
  console.log(`  Event Stream Summary`);
  console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);
  const totalEvents = Object.values(counts).reduce((a, b) => a + b, 0);
  console.log(`  Total event batches: ${totalEvents}`);
  console.log(`  Total DOM mutations: ${totalMutations}`);
  for (const [type, count] of Object.entries(counts).sort((a, b) => b[1] - a[1])) {
    console.log(`  ${type}: ${count}`);
  }
  console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n`);

  await client.stopObservation();

  if (sessionType === "managed") {
    await client.closeSession();
  }
  client.disconnect();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
