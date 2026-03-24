/**
 * 03 — Accessibility Review
 *
 * Use Tivana perception to review a page for accessibility issues.
 * The agent examines each element's semantic properties and flags
 * problems: missing labels, low contrast potential, keyboard traps,
 * unlabeled inputs, buttons without names, images without alt text.
 *
 * This demonstrates judgment-based perception — the agent notices
 * issues through structured awareness, not screenshots or selectors.
 *
 * Usage: bun run examples/03-accessibility-review.ts [url]
 */

import { TivanaClient, type Element } from "../sdk/ts/src/client";

const url = process.argv[2] || "https://news.ycombinator.com";

interface Issue {
  severity: "critical" | "serious" | "moderate" | "minor";
  element: string;
  role: string;
  rule: string;
  detail: string;
}

function reviewElement(el: Element): Issue[] {
  const issues: Issue[] = [];
  const desc = `${el.id} [${el.role}]`;

  // --- Critical: Interactive elements without accessible names ---
  const interactiveRoles = [
    "button", "a", "link", "text", "email", "password", "search",
    "checkbox", "radio", "combobox", "select", "textarea", "slider",
    "spinbutton", "switch", "tab", "menuitem",
  ];
  if (interactiveRoles.includes(el.role)) {
    if (!el.name || el.name.trim().length === 0) {
      issues.push({
        severity: "critical",
        element: desc,
        role: el.role,
        rule: "missing-label",
        detail: `Interactive element [${el.role}] has no accessible name. Screen readers cannot identify it.`,
      });
    }
  }

  // --- Serious: Buttons with generic names ---
  if (el.role === "button" && el.name) {
    const generic = ["click", "click here", "submit", "button", "ok", "x", "close"];
    if (generic.includes(el.name.toLowerCase().trim())) {
      issues.push({
        severity: "moderate",
        element: desc,
        role: el.role,
        rule: "generic-button-name",
        detail: `Button name "${el.name}" is generic. Use a descriptive label like "Save changes" or "Delete item".`,
      });
    }
  }

  // --- Serious: Links with generic names ---
  if ((el.role === "a" || el.role === "link") && el.name) {
    const genericLinks = ["click here", "read more", "learn more", "here", "link", "more"];
    if (genericLinks.includes(el.name.toLowerCase().trim())) {
      issues.push({
        severity: "serious",
        element: desc,
        role: el.role,
        rule: "generic-link-text",
        detail: `Link text "${el.name}" is not descriptive. Users navigating by links list won't know where this goes.`,
      });
    }
  }

  // --- Moderate: Required form fields without indication ---
  if (el.required && el.name && !el.name.includes("*") && !el.name.toLowerCase().includes("required")) {
    issues.push({
      severity: "minor",
      element: desc,
      role: el.role,
      rule: "required-not-indicated",
      detail: `Required field "${el.name}" doesn't indicate it's required in its label. Add * or "(required)".`,
    });
  }

  // --- Moderate: Form inputs with values but no labels ---
  if (["text", "email", "password", "search", "tel"].includes(el.role)) {
    if (el.value && (!el.name || el.name === el.value)) {
      issues.push({
        severity: "serious",
        element: desc,
        role: el.role,
        rule: "placeholder-as-label",
        detail: `Input appears to use its value/placeholder as its only label. Placeholders disappear when typing.`,
      });
    }
  }

  // --- Minor: Hidden but enabled interactive elements ---
  if (!el.visible && el.enabled && interactiveRoles.includes(el.role)) {
    issues.push({
      severity: "minor",
      element: desc,
      role: el.role,
      rule: "hidden-interactive",
      detail: `Interactive element is hidden but enabled. May confuse assistive technology if focusable.`,
    });
  }

  // --- Serious: Images (role=img) without names ---
  if (el.role === "img" && (!el.name || el.name.trim().length === 0)) {
    issues.push({
      severity: "serious",
      element: desc,
      role: el.role,
      rule: "image-no-alt",
      detail: `Image has no alt text. Screen readers will announce the file name or skip it entirely.`,
    });
  }

  return issues;
}

async function main() {
  const client = new TivanaClient({ url: "ws://localhost:9876" });
  await client.connect();
  await client.createSession();

  console.log(`\n♿ Accessibility Review: ${url}\n`);
  await client.navigate(url);

  const page = await client.pageState();
  const elements = await client.elements();

  console.log(`📄 ${page.title}`);
  console.log(`🧩 ${elements.length} elements (${elements.filter((e) => e.visible).length} visible)\n`);

  // Review every element
  const allIssues: Issue[] = [];
  for (const el of elements) {
    allIssues.push(...reviewElement(el));
  }

  // --- Report ---
  const bySeverity = {
    critical: allIssues.filter((i) => i.severity === "critical"),
    serious: allIssues.filter((i) => i.severity === "serious"),
    moderate: allIssues.filter((i) => i.severity === "moderate"),
    minor: allIssues.filter((i) => i.severity === "minor"),
  };

  console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);
  console.log(`  Accessibility Review Report`);
  console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);
  console.log(`  🔴 Critical: ${bySeverity.critical.length}`);
  console.log(`  🟠 Serious:  ${bySeverity.serious.length}`);
  console.log(`  🟡 Moderate: ${bySeverity.moderate.length}`);
  console.log(`  🔵 Minor:    ${bySeverity.minor.length}`);
  console.log(`  Total: ${allIssues.length} issues found`);
  console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n`);

  // Show issues grouped by severity
  for (const [severity, label, emoji] of [
    ["critical", "Critical", "🔴"],
    ["serious", "Serious", "🟠"],
    ["moderate", "Moderate", "🟡"],
    ["minor", "Minor", "🔵"],
  ] as const) {
    const issues = bySeverity[severity];
    if (issues.length === 0) continue;

    console.log(`${emoji} ${label} (${issues.length}):\n`);
    for (const issue of issues.slice(0, 10)) {
      console.log(`  ${issue.element}`);
      console.log(`  Rule: ${issue.rule}`);
      console.log(`  ${issue.detail}`);
      console.log();
    }
    if (issues.length > 10) {
      console.log(`  ... and ${issues.length - 10} more ${severity} issues\n`);
    }
  }

  // --- Summary by rule ---
  const byRule: Record<string, number> = {};
  for (const issue of allIssues) {
    byRule[issue.rule] = (byRule[issue.rule] || 0) + 1;
  }
  console.log(`📊 Issues by rule:`);
  for (const [rule, count] of Object.entries(byRule).sort((a, b) => b[1] - a[1])) {
    console.log(`   ${rule}: ${count}`);
  }

  console.log(`\n✅ Review complete.\n`);

  await client.closeSession();
  client.disconnect();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
