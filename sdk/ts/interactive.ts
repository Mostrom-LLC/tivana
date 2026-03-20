/**
 * Tivana Interactive REPL
 * 
 * Connects to Tivana runtime, creates a session, and waits for commands via stdin.
 * Browser stays open until explicitly closed.
 */

import { TivanaClient } from "./src/client";

const RUNTIME_URL = process.env.TIVANA_URL || "ws://localhost:9876";
const client = new TivanaClient({ url: RUNTIME_URL, timeout: 60000 });

const green = (s: string) => `\x1b[32m${s}\x1b[0m`;
const yellow = (s: string) => `\x1b[33m${s}\x1b[0m`;
const red = (s: string) => `\x1b[31m${s}\x1b[0m`;
const dim = (s: string) => `\x1b[2m${s}\x1b[0m`;

async function run() {
  console.log(green("\n🌐 Tivana Interactive Session\n"));

  await client.connect();
  console.log(green("✓ Connected to runtime"));

  const sessionId = await client.createSession();
  console.log(green(`✓ Session: ${sessionId.slice(0, 8)}...`));
  console.log(dim("  Browser is open. Waiting for commands.\n"));

  console.log(yellow("Commands:"));
  console.log(dim("  navigate <url>     — Go to URL"));
  console.log(dim("  state              — Show page state"));
  console.log(dim("  elements           — List interactive elements"));
  console.log(dim("  click <id>         — Click element (e.g., click e5)"));
  console.log(dim("  type <id> <text>   — Type text into element"));
  console.log(dim("  press <key>        — Press key (e.g., Enter, Tab)"));
  console.log(dim("  scroll <dir> [amt] — Scroll (up/down/left/right)"));
  console.log(dim("  text               — Get page text content"));
  console.log(dim("  meta               — Get page metadata"));
  console.log(dim("  close              — Close session and exit\n"));

  const reader = require("readline").createInterface({
    input: process.stdin,
    output: process.stdout,
  });

  const prompt = () => reader.question(green("tivana> "), handleCommand);

  const handleCommand = async (line: string) => {
    const parts = line.trim().split(/\s+/);
    const cmd = parts[0]?.toLowerCase();

    if (!cmd) { prompt(); return; }

    try {
      switch (cmd) {
        case "navigate":
        case "goto":
        case "go": {
          const url = parts[1];
          if (!url) { console.log(red("Usage: navigate <url>")); break; }
          const result = await client.navigate(url);
          console.log(green(`✓ Navigated: ${result.success}`));
          const s = await client.pageState();
          console.log(dim(`  URL: ${s.url}`));
          console.log(dim(`  Title: ${s.title}`));
          break;
        }
        case "state":
        case "page": {
          const s = await client.pageState();
          console.log(`  URL:      ${s.url}`);
          console.log(`  Title:    ${s.title}`);
          console.log(`  Scroll:   ${s.scrollX}, ${s.scrollY}`);
          console.log(`  Viewport: ${s.viewportWidth}x${s.viewportHeight}`);
          console.log(`  Focused:  ${s.focusedElementId || "none"}`);
          break;
        }
        case "elements":
        case "els":
        case "el": {
          const els = await client.elements();
          console.log(green(`  ${els.length} interactive elements:`));
          for (const el of els) {
            const name = el.name ? `"${el.name.slice(0, 70)}"` : "(no name)";
            const extra = [
              el.focused && "focused",
              !el.enabled && "disabled",
              el.value && `value="${el.value.slice(0, 30)}"`,
            ].filter(Boolean).join(", ");
            console.log(dim(`  ${el.id}: ${el.role} ${name}${extra ? ` [${extra}]` : ""}`));
          }
          break;
        }
        case "click": {
          const target = parts[1];
          if (!target) { console.log(red("Usage: click <elementId>")); break; }
          const result = await client.click(target);
          console.log(green(`✓ Clicked ${target}: ${result.success}`));
          break;
        }
        case "type": {
          const target = parts[1];
          const text = parts.slice(2).join(" ");
          if (!target || !text) { console.log(red("Usage: type <elementId> <text>")); break; }
          const result = await client.type(text, target);
          console.log(green(`✓ Typed into ${target}: ${result.success}`));
          break;
        }
        case "press": {
          const key = parts[1];
          if (!key) { console.log(red("Usage: press <key>")); break; }
          const mods = parts.slice(2);
          const result = await client.press(key, mods.length > 0 ? mods : undefined);
          console.log(green(`✓ Pressed ${key}: ${result.success}`));
          break;
        }
        case "scroll": {
          const dir = (parts[1] || "down") as "up" | "down" | "left" | "right";
          const amt = parseInt(parts[2] || "300", 10);
          const result = await client.scroll(undefined, dir, { amount: amt });
          console.log(green(`✓ Scrolled ${dir} ${amt}px: ${result.success}`));
          break;
        }
        case "text": {
          const t = await client.textContent();
          console.log(`  Words: ${t.wordCount} | Chars: ${t.charCount}`);
          console.log(dim(`  ${t.text.slice(0, 500).replace(/\n/g, "\n  ")}`));
          break;
        }
        case "meta": {
          const m = await client.metadata();
          console.log(`  URL:   ${m.url}`);
          console.log(`  Title: ${m.title}`);
          if (m.description) console.log(`  Desc:  ${m.description.slice(0, 100)}`);
          if (m.language) console.log(`  Lang:  ${m.language}`);
          break;
        }
        case "close":
        case "quit":
        case "exit": {
          console.log(yellow("Closing session..."));
          await client.closeSession();
          client.disconnect();
          console.log(green("Done."));
          process.exit(0);
        }
        default:
          console.log(red(`Unknown command: ${cmd}`));
      }
    } catch (e) {
      console.log(red(`Error: ${(e as Error).message}`));
    }

    prompt();
  };

  prompt();
}

run().catch(e => { console.error("Fatal:", e); process.exit(1); });
