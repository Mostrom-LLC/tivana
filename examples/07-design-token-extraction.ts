/**
 * 07 — Design Token Extraction
 *
 * Extract design tokens from any website using Tivana perception.
 * Scans all elements' computed styles to discover colors, typography,
 * spacing, shadows, and border radii — then outputs W3C DTCG-format
 * design tokens.
 *
 * This is the Tivana equivalent of tools like Dembrandt, but built
 * on perception rather than scraping. Works on any page, including
 * pages behind auth (via extension mode).
 *
 * Usage: bun run examples/07-design-token-extraction.ts [url]
 */

import { TivanaClient } from "tivana";

const url = process.argv[2] || "https://stripe.com";

// --- Extraction scripts (run in page context via evaluate) ---

const EXTRACT_COLORS_SCRIPT = `(() => {
  const colors = new Map();
  const els = document.querySelectorAll('*');
  for (const el of els) {
    const s = getComputedStyle(el);
    for (const prop of ['color', 'backgroundColor', 'borderColor', 'outlineColor']) {
      const v = s[prop];
      if (v && v !== 'rgba(0, 0, 0, 0)' && v !== 'transparent') {
        colors.set(v, (colors.get(v) || 0) + 1);
      }
    }
  }
  return JSON.stringify([...colors.entries()].sort((a, b) => b[1] - a[1]).slice(0, 50));
})()`;

const EXTRACT_TYPOGRAPHY_SCRIPT = `(() => {
  const fonts = new Map();
  const els = document.querySelectorAll('*');
  for (const el of els) {
    if (!el.textContent?.trim()) continue;
    const s = getComputedStyle(el);
    const key = JSON.stringify({
      fontFamily: s.fontFamily.split(',')[0].trim().replace(/['"]/g, ''),
      fontSize: s.fontSize,
      fontWeight: s.fontWeight,
      lineHeight: s.lineHeight,
      letterSpacing: s.letterSpacing,
    });
    fonts.set(key, (fonts.get(key) || 0) + 1);
  }
  return JSON.stringify([...fonts.entries()]
    .map(([k, c]) => ({ ...JSON.parse(k), count: c }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 30));
})()`;

const EXTRACT_SPACING_SCRIPT = `(() => {
  const spacings = new Map();
  const els = document.querySelectorAll('*');
  for (const el of els) {
    const s = getComputedStyle(el);
    for (const prop of ['marginTop', 'marginRight', 'marginBottom', 'marginLeft',
                         'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft',
                         'gap', 'rowGap', 'columnGap']) {
      const v = s[prop];
      if (v && v !== '0px' && v !== 'normal' && v !== 'auto') {
        spacings.set(v, (spacings.get(v) || 0) + 1);
      }
    }
  }
  return JSON.stringify([...spacings.entries()].sort((a, b) => b[1] - a[1]).slice(0, 30));
})()`;

const EXTRACT_SHADOWS_SCRIPT = `(() => {
  const shadows = new Map();
  const els = document.querySelectorAll('*');
  for (const el of els) {
    const s = getComputedStyle(el);
    for (const prop of ['boxShadow', 'textShadow']) {
      const v = s[prop];
      if (v && v !== 'none') {
        shadows.set(v, (shadows.get(v) || 0) + 1);
      }
    }
  }
  return JSON.stringify([...shadows.entries()].sort((a, b) => b[1] - a[1]).slice(0, 20));
})()`;

const EXTRACT_RADII_SCRIPT = `(() => {
  const radii = new Map();
  const els = document.querySelectorAll('*');
  for (const el of els) {
    const v = getComputedStyle(el).borderRadius;
    if (v && v !== '0px') {
      radii.set(v, (radii.get(v) || 0) + 1);
    }
  }
  return JSON.stringify([...radii.entries()].sort((a, b) => b[1] - a[1]).slice(0, 20));
})()`;

// --- Color conversion helpers ---

function parseRgb(str: string): { r: number; g: number; b: number; a: number } | null {
  const m = str.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\)/);
  if (!m) return null;
  return { r: +m[1], g: +m[2], b: +m[3], a: m[4] !== undefined ? +m[4] : 1 };
}

function rgbToHex(r: number, g: number, b: number): string {
  return `#${[r, g, b].map((c) => c.toString(16).padStart(2, "0")).join("")}`;
}

function colorName(hex: string): string {
  // Simple heuristic names for common ranges
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2 / 255;

  if (l < 0.08) return "black";
  if (l > 0.95) return "white";
  if (max - min < 20) return l > 0.5 ? "gray-light" : "gray-dark";
  if (r > g && r > b) return r > 200 && g < 100 ? "red" : "orange";
  if (g > r && g > b) return "green";
  if (b > r && b > g) return b > 200 && r > 100 ? "purple" : "blue";
  if (r > 200 && g > 200) return "yellow";
  return "accent";
}

// --- DTCG token generation ---

interface DesignTokens {
  color: Record<string, { $value: string; $type: "color"; uses: number }>;
  typography: Record<string, { $value: Record<string, string>; $type: "typography"; uses: number }>;
  spacing: Record<string, { $value: string; $type: "dimension"; uses: number }>;
  shadow: Record<string, { $value: string; $type: "shadow"; uses: number }>;
  borderRadius: Record<string, { $value: string; $type: "dimension"; uses: number }>;
}

async function main() {
  const client = new TivanaClient({ url: "ws://localhost:9876" });
  await client.connect();

  // Try extension first, fall back to managed
  let sessionType = "managed";
  try {
    const ext = await client.request<any>("session.fromExtension", {});
    (client as any).sessionId = ext.sessionId;
    sessionType = "extension";
  } catch {
    await client.createSession();
  }

  console.log(`\n🎨 Design Token Extraction (${sessionType} session)`);
  console.log(`🔗 ${url}\n`);

  if (sessionType === "managed") {
    await client.navigate(url);
    await new Promise((r) => setTimeout(r, 3000)); // Let page render fully
  }

  const page = await client.pageState();
  console.log(`📄 ${page.title}`);
  console.log(`   Viewport: ${page.viewportWidth}×${page.viewportHeight}`);
  console.log(`   Document: ${page.documentWidth}×${page.documentHeight}\n`);

  // --- Extract all tokens in parallel ---
  console.log(`⏳ Extracting design tokens...\n`);

  const [colorsRaw, typographyRaw, spacingRaw, shadowsRaw, radiiRaw] = await Promise.all([
    client.evaluate(EXTRACT_COLORS_SCRIPT),
    client.evaluate(EXTRACT_TYPOGRAPHY_SCRIPT),
    client.evaluate(EXTRACT_SPACING_SCRIPT),
    client.evaluate(EXTRACT_SHADOWS_SCRIPT),
    client.evaluate(EXTRACT_RADII_SCRIPT),
  ]);

  const colors: [string, number][] = JSON.parse(colorsRaw as string);
  const typography: any[] = JSON.parse(typographyRaw as string);
  const spacing: [string, number][] = JSON.parse(spacingRaw as string);
  const shadows: [string, number][] = JSON.parse(shadowsRaw as string);
  const radii: [string, number][] = JSON.parse(radiiRaw as string);

  // --- Build DTCG tokens ---
  const tokens: DesignTokens = {
    color: {},
    typography: {},
    spacing: {},
    shadow: {},
    borderRadius: {},
  };

  // Colors
  const seenHex = new Set<string>();
  let colorIdx = 0;
  for (const [rgb, count] of colors) {
    const parsed = parseRgb(rgb);
    if (!parsed) continue;
    const hex = rgbToHex(parsed.r, parsed.g, parsed.b);
    if (seenHex.has(hex)) continue;
    seenHex.add(hex);
    const name = `${colorName(hex)}-${++colorIdx}`;
    tokens.color[name] = {
      $value: parsed.a < 1 ? `${hex}${Math.round(parsed.a * 255).toString(16).padStart(2, "0")}` : hex,
      $type: "color",
      uses: count,
    };
  }

  // Typography
  for (let i = 0; i < typography.length; i++) {
    const t = typography[i];
    const name = `style-${i + 1}`;
    tokens.typography[name] = {
      $value: {
        fontFamily: t.fontFamily,
        fontSize: t.fontSize,
        fontWeight: t.fontWeight,
        lineHeight: t.lineHeight,
        letterSpacing: t.letterSpacing,
      },
      $type: "typography",
      uses: t.count,
    };
  }

  // Spacing
  for (const [value, count] of spacing) {
    const key = `space-${value.replace(/[^0-9.]/g, "")}`;
    if (!tokens.spacing[key]) {
      tokens.spacing[key] = { $value: value, $type: "dimension", uses: count };
    }
  }

  // Shadows
  for (let i = 0; i < shadows.length; i++) {
    tokens.shadow[`shadow-${i + 1}`] = {
      $value: shadows[i][0],
      $type: "shadow",
      uses: shadows[i][1],
    };
  }

  // Border radii
  for (const [value, count] of radii) {
    const key = `radius-${value.replace(/[^0-9.]/g, "") || "mixed"}`;
    if (!tokens.borderRadius[key]) {
      tokens.borderRadius[key] = { $value: value, $type: "dimension", uses: count };
    }
  }

  // --- Display results ---
  console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);
  console.log(`  Design Token Report: ${new URL(page.url).hostname}`);
  console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);

  // Colors
  console.log(`\n🎨 Colors (${Object.keys(tokens.color).length} unique):\n`);
  for (const [name, token] of Object.entries(tokens.color).slice(0, 15)) {
    const bar = "█".repeat(Math.min(Math.ceil(token.uses / 10), 20));
    console.log(`  ${token.$value}  ${name.padEnd(18)} ${bar} (${token.uses})`);
  }
  if (Object.keys(tokens.color).length > 15) {
    console.log(`  ... +${Object.keys(tokens.color).length - 15} more`);
  }

  // Typography
  console.log(`\n📝 Typography (${Object.keys(tokens.typography).length} styles):\n`);
  for (const [name, token] of Object.entries(tokens.typography).slice(0, 10)) {
    const v = token.$value;
    console.log(
      `  ${name.padEnd(12)} ${v.fontFamily.padEnd(20)} ${v.fontSize.padEnd(6)} w${v.fontWeight} (${token.uses} uses)`
    );
  }

  // Spacing
  console.log(`\n📐 Spacing (${Object.keys(tokens.spacing).length} values):\n`);
  for (const [name, token] of Object.entries(tokens.spacing).slice(0, 12)) {
    const bar = "▪".repeat(Math.min(Math.ceil(token.uses / 20), 20));
    console.log(`  ${token.$value.padEnd(10)} ${name.padEnd(14)} ${bar} (${token.uses})`);
  }

  // Shadows
  if (Object.keys(tokens.shadow).length > 0) {
    console.log(`\n🌑 Shadows (${Object.keys(tokens.shadow).length}):\n`);
    for (const [name, token] of Object.entries(tokens.shadow).slice(0, 5)) {
      console.log(`  ${name}: ${token.$value.slice(0, 70)}${token.$value.length > 70 ? "..." : ""} (${token.uses})`);
    }
  }

  // Border Radii
  if (Object.keys(tokens.borderRadius).length > 0) {
    console.log(`\n⭕ Border Radii (${Object.keys(tokens.borderRadius).length}):\n`);
    for (const [name, token] of Object.entries(tokens.borderRadius).slice(0, 8)) {
      console.log(`  ${token.$value.padEnd(14)} ${name} (${token.uses} uses)`);
    }
  }

  // --- W3C DTCG output ---
  const dtcg = {
    $schema: "https://design-tokens.github.io/community-group/format/",
    $description: `Design tokens extracted from ${page.url} by Tivana`,
    color: Object.fromEntries(
      Object.entries(tokens.color).map(([k, v]) => [k, { $value: v.$value, $type: v.$type }])
    ),
    typography: Object.fromEntries(
      Object.entries(tokens.typography).map(([k, v]) => [k, { $value: v.$value, $type: v.$type }])
    ),
    spacing: Object.fromEntries(
      Object.entries(tokens.spacing).map(([k, v]) => [k, { $value: v.$value, $type: v.$type }])
    ),
    shadow: Object.fromEntries(
      Object.entries(tokens.shadow).map(([k, v]) => [k, { $value: v.$value, $type: v.$type }])
    ),
    borderRadius: Object.fromEntries(
      Object.entries(tokens.borderRadius).map(([k, v]) => [k, { $value: v.$value, $type: v.$type }])
    ),
  };

  console.log(`\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);
  console.log(`  W3C DTCG Output`);
  console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n`);
  console.log(JSON.stringify(dtcg, null, 2).slice(0, 3000));
  if (JSON.stringify(dtcg).length > 3000) {
    console.log(`\n  ... (${JSON.stringify(dtcg).length} bytes total — save with --output flag)`);
  }

  console.log(`\n✅ Extraction complete.`);
  console.log(`   ${Object.keys(tokens.color).length} colors, ${Object.keys(tokens.typography).length} typography styles, ${Object.keys(tokens.spacing).length} spacing values, ${Object.keys(tokens.shadow).length} shadows, ${Object.keys(tokens.borderRadius).length} border radii\n`);

  if (sessionType === "managed") {
    await client.closeSession();
  }
  client.disconnect();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
