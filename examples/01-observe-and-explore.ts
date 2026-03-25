/**
 * 01 — Observe and Explore
 *
 * Connect to Tivana, navigate to a page, and explore what's on it.
 * This is the most basic Tivana pattern: perceive the page, understand
 * its structure, and report what you see.
 *
 * No hardcoded selectors. No site-specific logic. Just awareness.
 *
 * Usage: bun run examples/01-observe-and-explore.ts [url]
 */

import { TivanaClient } from "tivana";

const url = process.argv[2] || "https://news.ycombinator.com";

async function main() {
  const client = new TivanaClient({ url: "ws://localhost:9876" });
  await client.connect();
  await client.createSession();

  console.log(`\n🔗 Navigating to ${url}\n`);
  await client.navigate(url);

  // --- Snapshot: full page state ---
  const page = await client.pageState();
  console.log(`📄 Page: ${page.title}`);
  console.log(`   URL: ${page.url}`);
  console.log(`   Viewport: ${page.viewportWidth}×${page.viewportHeight}`);
  console.log(`   Document: ${page.documentWidth}×${page.documentHeight}`);
  console.log(`   Scroll: (${page.scrollX}, ${page.scrollY})`);

  // --- Elements: semantic inventory ---
  const elements = await client.elements();
  const visible = elements.filter((e) => e.visible);
  const interactable = elements.filter((e) => (e as any).interactable);

  console.log(`\n🧩 Elements: ${elements.length} total`);
  console.log(`   Visible: ${visible.length}`);
  console.log(`   Interactable: ${interactable.length}`);

  // Role distribution
  const roles: Record<string, number> = {};
  for (const el of elements) {
    roles[el.role] = (roles[el.role] || 0) + 1;
  }
  console.log(`\n📊 Element roles:`);
  for (const [role, count] of Object.entries(roles).sort((a, b) => b[1] - a[1])) {
    console.log(`   ${role}: ${count}`);
  }

  // Named elements (elements with meaningful labels)
  const named = elements.filter((e) => e.name && e.name.length > 3);
  console.log(`\n🏷️  Named elements (${named.length}):`);
  for (const el of named.slice(0, 15)) {
    const flags = [
      el.visible ? "visible" : "hidden",
      (el as any).interactable ? "interactable" : "",
      el.focused ? "focused" : "",
      el.required ? "required" : "",
    ]
      .filter(Boolean)
      .join(", ");
    console.log(`   ${el.id} [${el.role}] "${el.name?.slice(0, 50)}" — ${flags}`);
  }

  // Links
  const links = elements.filter(
    (e) => e.role === "a" || e.role === "link"
  );
  console.log(`\n🔗 Links: ${links.length}`);
  for (const link of links.slice(0, 10)) {
    console.log(`   ${link.id} "${link.name?.slice(0, 60) || "(unnamed)"}"`);
  }

  // Forms
  const formElements = elements.filter((e) =>
    ["text", "email", "password", "search", "textarea", "select", "checkbox", "radio", "combobox"].includes(e.role)
  );
  if (formElements.length > 0) {
    console.log(`\n📝 Form elements: ${formElements.length}`);
    for (const fe of formElements) {
      console.log(
        `   ${fe.id} [${fe.role}] "${fe.name || "(no label)"}" value="${fe.value || ""}" ${fe.required ? "REQUIRED" : ""}`
      );
    }
  }

  // Buttons
  const buttons = elements.filter((e) => e.role === "button");
  if (buttons.length > 0) {
    console.log(`\n🔘 Buttons: ${buttons.length}`);
    for (const btn of buttons.slice(0, 10)) {
      console.log(`   ${btn.id} "${btn.name?.slice(0, 40) || "(unnamed)"}" enabled=${btn.enabled}`);
    }
  }

  // Accessibility snapshot
  const a11y = await client.accessibilitySnapshot();
  console.log(`\n♿ Accessibility snapshot:`);
  console.log(`   Interactive elements: ${a11y.interactiveElements.length}`);

  console.log(`\n✅ Exploration complete.\n`);

  await client.closeSession();
  client.disconnect();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
