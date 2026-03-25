/**
 * 02 — Agent Loop
 *
 * The canonical Tivana pattern: perceive → reason → act → repeat.
 *
 * This example navigates to a page and runs a goal-driven loop.
 * The "reasoning" here is simple rule-based logic, but in a real
 * agent it would be an LLM call with the perception as context.
 *
 * The key insight: the agent never needs to know the site structure
 * in advance. It discovers what's on the page and decides what to do.
 *
 * Usage: bun run examples/02-agent-loop.ts
 */

import { TivanaClient, type Element, type PageState } from "@mostrom/tivana";

// --- Agent decision types ---
type Decision =
  | { action: "click"; elementId: string; reason: string }
  | { action: "type"; elementId: string; text: string; reason: string }
  | { action: "scroll"; direction: "down" | "up"; reason: string }
  | { action: "done"; summary: string };

/**
 * Simple rule-based "reasoning" — replace this with an LLM call.
 *
 * In a real agent, you'd send the page state and elements to a model:
 *   const decision = await llm.decide({ goal, page, elements });
 *
 * The model would return one of the Decision types above.
 */
function reason(
  goal: string,
  page: PageState,
  elements: Element[],
  step: number
): Decision {
  // Goal: find and click the first interesting link
  if (step === 0) {
    // First step: look for a search box
    const searchInput = elements.find(
      (e) =>
        (e.role === "search" || e.role === "searchbox" || e.role === "text" || e.role === "combobox") &&
        e.name?.toLowerCase().includes("search")
    );
    if (searchInput) {
      return {
        action: "type",
        elementId: searchInput.id,
        text: "Tivana browser protocol",
        reason: `Found search input: ${searchInput.id} [${searchInput.role}] "${searchInput.name}"`,
      };
    }
  }

  // Look for a submit/search button after typing
  if (step === 1) {
    const submitBtn = elements.find(
      (e) =>
        e.role === "button" &&
        (e.name?.toLowerCase().includes("search") ||
          e.name?.toLowerCase().includes("submit") ||
          e.name?.toLowerCase().includes("go"))
    );
    if (submitBtn) {
      return {
        action: "click",
        elementId: submitBtn.id,
        reason: `Found submit button: ${submitBtn.id} "${submitBtn.name}"`,
      };
    }
  }

  // Look for content links
  const contentLinks = elements.filter(
    (e) =>
      (e.role === "a" || e.role === "link") &&
      e.visible &&
      e.name &&
      e.name.length > 10 &&
      !e.name.toLowerCase().includes("sign") &&
      !e.name.toLowerCase().includes("login")
  );

  if (contentLinks.length > 0 && step >= 2) {
    const target = contentLinks[Math.min(step - 2, contentLinks.length - 1)];
    return {
      action: "click",
      elementId: target.id,
      reason: `Exploring link: ${target.id} "${target.name?.slice(0, 50)}"`,
    };
  }

  // Scroll if we haven't found enough
  if (elements.filter((e) => e.visible).length < 5) {
    return {
      action: "scroll",
      direction: "down",
      reason: "Few visible elements, scrolling to reveal more",
    };
  }

  return {
    action: "done",
    summary: `Explored ${page.url} — found ${elements.length} elements, ${contentLinks.length} content links`,
  };
}

async function main() {
  const client = new TivanaClient({ url: "ws://localhost:9876" });
  await client.connect();
  await client.createSession();

  const goal = "Explore the page and find interesting content.";
  const startUrl = "https://news.ycombinator.com";
  const maxSteps = 5;

  console.log(`\n🎯 Goal: ${goal}`);
  console.log(`🔗 Starting at: ${startUrl}\n`);

  await client.navigate(startUrl);

  for (let step = 0; step < maxSteps; step++) {
    // --- Perceive ---
    const [page, elements] = await Promise.all([
      client.pageState(),
      client.elements(),
    ]);
    console.log(`\n━━━ Step ${step + 1}/${maxSteps} ━━━`);
    console.log(`📄 ${page.title?.slice(0, 60)} (${elements.length} elements)`);

    // --- Reason ---
    const decision = reason(goal, page, elements, step);
    console.log(`🧠 Decision: ${decision.action}`);

    // --- Act ---
    if (decision.action === "done") {
      console.log(`✅ ${decision.summary}`);
      break;
    }

    if (decision.action === "click") {
      console.log(`   ${decision.reason}`);
      await client.click(decision.elementId);
      await new Promise((r) => setTimeout(r, 2000));
    }

    if (decision.action === "type") {
      console.log(`   ${decision.reason}`);
      await client.type(decision.text, decision.elementId);
      await new Promise((r) => setTimeout(r, 1000));
    }

    if (decision.action === "scroll") {
      console.log(`   ${decision.reason}`);
      await client.scroll(undefined, decision.direction);
      await new Promise((r) => setTimeout(r, 1000));
    }
  }

  console.log(`\n🏁 Agent loop complete.\n`);

  await client.closeSession();
  client.disconnect();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
