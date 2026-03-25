/**
 * 04 — Anomaly Detection
 *
 * Use Tivana perception to detect visual and structural anomalies
 * on a page. The agent examines the element tree and flags things
 * that "feel off" — overlapping elements, elements outside viewport,
 * disabled buttons that should be enabled, empty containers, etc.
 *
 * This demonstrates exploratory QA — the agent notices issues through
 * perception, not by checking against a hardcoded list of expected states.
 *
 * Usage: bun run examples/04-anomaly-detection.ts [url]
 */

import { TivanaClient, type Element, type PageState } from "tivana";

const url = process.argv[2] || "https://news.ycombinator.com";

interface Anomaly {
  type: string;
  severity: "high" | "medium" | "low";
  element?: string;
  detail: string;
}

function detectAnomalies(page: PageState, elements: Element[]): Anomaly[] {
  const anomalies: Anomaly[] = [];

  // --- Page-level checks ---

  // No title
  if (!page.title || page.title.trim().length === 0) {
    anomalies.push({
      type: "missing-title",
      severity: "high",
      detail: "Page has no title. This hurts SEO, bookmarks, and screen reader navigation.",
    });
  }

  // Very few interactive elements on a complex page
  if (elements.length < 3 && page.documentHeight > 1000) {
    anomalies.push({
      type: "sparse-interaction",
      severity: "medium",
      detail: `Only ${elements.length} interactive elements on a ${page.documentHeight}px tall page. May indicate broken rendering or missing content.`,
    });
  }

  // Page much wider than viewport (horizontal overflow)
  if (page.documentWidth > page.viewportWidth + 50) {
    anomalies.push({
      type: "horizontal-overflow",
      severity: "medium",
      detail: `Document width (${page.documentWidth}px) exceeds viewport (${page.viewportWidth}px) by ${page.documentWidth - page.viewportWidth}px. Likely causes horizontal scrollbar.`,
    });
  }

  // --- Element-level checks ---

  for (const el of elements) {
    const desc = `${el.id} [${el.role}] "${(el.name || "").slice(0, 30)}"`;

    // Elements with zero dimensions but visible
    if (el.bounds && el.bounds.width === 0 && el.bounds.height === 0 && el.visible) {
      anomalies.push({
        type: "zero-size-visible",
        severity: "medium",
        element: desc,
        detail: `Element reports as visible but has 0×0 dimensions. May be invisible to users but present in DOM.`,
      });
    }

    // Elements far outside viewport
    if (el.bounds && el.visible) {
      const offRight = el.bounds.x - page.viewportWidth;
      const offBottom = el.bounds.y - (page.scrollY + page.viewportHeight);
      const offLeft = -(el.bounds.x + el.bounds.width);
      const offTop = -(el.bounds.y + el.bounds.height);

      if (offRight > 500 || offLeft > 500) {
        anomalies.push({
          type: "far-offscreen-horizontal",
          severity: "low",
          element: desc,
          detail: `Element is ${Math.max(offRight, offLeft).toFixed(0)}px off-screen horizontally. May be a layout bug or CSS issue.`,
        });
      }
    }

    // Overlapping elements (simple check: same bounds)
    for (const other of elements) {
      if (other.id === el.id) continue;
      if (el.bounds && other.bounds &&
        Math.abs(el.bounds.x - other.bounds.x) < 2 &&
        Math.abs(el.bounds.y - other.bounds.y) < 2 &&
        Math.abs(el.bounds.width - other.bounds.width) < 2 &&
        Math.abs(el.bounds.height - other.bounds.height) < 2 &&
        el.bounds.width > 0) {
        anomalies.push({
          type: "overlapping-elements",
          severity: "medium",
          element: desc,
          detail: `Overlaps with ${other.id} [${other.role}] at same position (${el.bounds.x.toFixed(0)}, ${el.bounds.y.toFixed(0)}). May be a z-index issue or duplicate rendering.`,
        });
        break; // Only report first overlap per element
      }
    }

    // Disabled buttons in prominent positions
    if (el.role === "button" && !el.enabled && el.visible) {
      anomalies.push({
        type: "disabled-button",
        severity: "low",
        element: desc,
        detail: `Visible but disabled button. Consider showing why it's disabled or hiding it.`,
      });
    }

    // Very long element names (potential text overflow)
    if (el.name && el.name.length > 80 && el.bounds && el.bounds.width < 200) {
      anomalies.push({
        type: "potential-text-overflow",
        severity: "low",
        element: desc,
        detail: `Name is ${el.name.length} chars but element is only ${el.bounds.width.toFixed(0)}px wide. Text may overflow or be truncated.`,
      });
    }

    // Focused but not visible
    if (el.focused && !el.visible) {
      anomalies.push({
        type: "focused-hidden",
        severity: "high",
        element: desc,
        detail: `Element has focus but is not visible. Users cannot see where they are. Keyboard trap risk.`,
      });
    }

    // Interactable but tiny (hard to click)
    if ((el as any).interactable && el.bounds &&
      el.bounds.width > 0 && el.bounds.height > 0 &&
      (el.bounds.width < 20 || el.bounds.height < 20)) {
      anomalies.push({
        type: "tiny-target",
        severity: "medium",
        element: desc,
        detail: `Interactive element is only ${el.bounds.width.toFixed(0)}×${el.bounds.height.toFixed(0)}px. WCAG recommends minimum 44×44px touch targets.`,
      });
    }
  }

  // Deduplicate
  const seen = new Set<string>();
  return anomalies.filter((a) => {
    const key = `${a.type}:${a.element || "page"}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

async function main() {
  const client = new TivanaClient({ url: "ws://localhost:9876" });
  await client.connect();
  await client.createSession();

  console.log(`\n🔍 Anomaly Detection: ${url}\n`);
  await client.navigate(url);

  const page = await client.pageState();
  const elements = await client.elements();

  console.log(`📄 ${page.title}`);
  console.log(`🧩 ${elements.length} elements\n`);

  const anomalies = detectAnomalies(page, elements);

  // --- Report ---
  const bySeverity = {
    high: anomalies.filter((a) => a.severity === "high"),
    medium: anomalies.filter((a) => a.severity === "medium"),
    low: anomalies.filter((a) => a.severity === "low"),
  };

  console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);
  console.log(`  Anomaly Detection Report`);
  console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);
  console.log(`  🔴 High:   ${bySeverity.high.length}`);
  console.log(`  🟡 Medium: ${bySeverity.medium.length}`);
  console.log(`  🔵 Low:    ${bySeverity.low.length}`);
  console.log(`  Total: ${anomalies.length} anomalies found`);
  console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n`);

  for (const [severity, emoji] of [
    ["high", "🔴"],
    ["medium", "🟡"],
    ["low", "🔵"],
  ] as const) {
    const items = bySeverity[severity];
    if (items.length === 0) continue;

    console.log(`${emoji} ${severity.toUpperCase()} (${items.length}):\n`);
    for (const a of items.slice(0, 15)) {
      if (a.element) console.log(`  ${a.element}`);
      console.log(`  [${a.type}] ${a.detail}`);
      console.log();
    }
    if (items.length > 15) {
      console.log(`  ... and ${items.length - 15} more\n`);
    }
  }

  // Summary by type
  const byType: Record<string, number> = {};
  for (const a of anomalies) {
    byType[a.type] = (byType[a.type] || 0) + 1;
  }
  console.log(`📊 Anomalies by type:`);
  for (const [type, count] of Object.entries(byType).sort((a, b) => b[1] - a[1])) {
    console.log(`   ${type}: ${count}`);
  }

  console.log(`\n✅ Detection complete.\n`);

  await client.closeSession();
  client.disconnect();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
