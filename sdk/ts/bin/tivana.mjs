#!/usr/bin/env node

/**
 * Tivana CLI — starts the Tivana runtime.
 *
 * On first run, downloads the prebuilt binary for your platform from
 * GitHub releases. Cached at ~/.tivana/bin/tivana.
 *
 * Usage:
 *   npx tivana                    # Start runtime (default port 9876)
 *   npx tivana --headless         # Headless mode
 *   npx tivana --port 3000        # Custom port
 *   npx tivana --connect 9222     # Attach to existing Chrome
 *   npx tivana --help             # Show all options
 */

import { existsSync, mkdirSync, chmodSync, createWriteStream, unlinkSync, renameSync } from "node:fs";
import { join } from "node:path";
import { homedir, platform, arch } from "node:os";
import { spawn } from "node:child_process";
import { get as httpsGet } from "node:https";
import { pipeline } from "node:stream/promises";
import { createReadStream } from "node:fs";
import { createGunzip } from "node:zlib";

const REPO = "Mostrom-LLC/tivana";
const VERSION = "0.1.0";
const BINARY_NAME = "tivana";
const CACHE_DIR = join(homedir(), ".tivana", "bin");

/**
 * Resolve platform + arch to the GitHub release asset name.
 */
function getAssetName() {
  const p = platform();
  const a = arch();

  const targets = {
    "darwin-arm64": "tivana-aarch64-apple-darwin",
    "darwin-x64": "tivana-x86_64-apple-darwin",
    "linux-x64": "tivana-x86_64-unknown-linux-gnu",
    "linux-arm64": "tivana-aarch64-unknown-linux-gnu",
  };

  const key = `${p}-${a}`;
  const target = targets[key];

  if (!target) {
    console.error(`❌ Unsupported platform: ${p}-${a}`);
    console.error(`   Supported: ${Object.keys(targets).join(", ")}`);
    console.error(`\n   You can build from source instead:`);
    console.error(`   cd runtime && cargo build --release`);
    process.exit(1);
  }

  return `${target}.tar.gz`;
}

/**
 * Follow redirects and download a file.
 */
function download(url, dest, maxRedirects = 5) {
  return new Promise((resolve, reject) => {
    if (maxRedirects <= 0) return reject(new Error("Too many redirects"));

    httpsGet(url, { headers: { "User-Agent": "tivana-cli" } }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        return download(res.headers.location, dest, maxRedirects - 1).then(resolve, reject);
      }
      if (res.statusCode !== 200) {
        return reject(new Error(`Download failed: HTTP ${res.statusCode}`));
      }

      const file = createWriteStream(dest);
      res.pipe(file);
      file.on("finish", () => file.close(resolve));
      file.on("error", reject);
    }).on("error", reject);
  });
}

/**
 * Extract a .tar.gz to a directory.
 */
async function extractTarGz(tarGzPath, destDir) {
  // Use system tar (available on all supported platforms)
  return new Promise((resolve, reject) => {
    const tar = spawn("tar", ["xzf", tarGzPath, "-C", destDir], { stdio: "inherit" });
    tar.on("close", (code) => (code === 0 ? resolve() : reject(new Error(`tar exited with ${code}`))));
    tar.on("error", reject);
  });
}

/**
 * Download and cache the Tivana binary.
 */
async function ensureBinary() {
  const binaryPath = join(CACHE_DIR, BINARY_NAME);

  if (existsSync(binaryPath)) {
    return binaryPath;
  }

  const asset = getAssetName();
  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${asset}`;

  console.log(`⬇️  Downloading Tivana v${VERSION} for ${platform()}-${arch()}...`);
  console.log(`   ${url}`);

  mkdirSync(CACHE_DIR, { recursive: true });

  const tarPath = join(CACHE_DIR, asset);

  try {
    await download(url, tarPath);
    console.log(`📦 Extracting...`);
    await extractTarGz(tarPath, CACHE_DIR);

    // Clean up tar
    try { unlinkSync(tarPath); } catch {}

    if (!existsSync(binaryPath)) {
      throw new Error(`Binary not found after extraction. Expected: ${binaryPath}`);
    }

    chmodSync(binaryPath, 0o755);
    console.log(`✅ Installed to ${binaryPath}\n`);
    return binaryPath;
  } catch (err) {
    // Clean up on failure
    try { unlinkSync(tarPath); } catch {}
    try { unlinkSync(binaryPath); } catch {}

    console.error(`\n❌ Failed to download prebuilt binary.`);
    console.error(`   ${err.message}`);
    console.error(`\n   This might mean:`);
    console.error(`   • No release exists for v${VERSION} yet`);
    console.error(`   • Your platform (${platform()}-${arch()}) isn't supported`);
    console.error(`\n   Build from source instead:`);
    console.error(`   cd runtime && cargo build --release`);
    console.error(`   ./target/release/tivana`);
    process.exit(1);
  }
}

/**
 * Check if running from the repo with a local build available.
 */
function findLocalBinary() {
  // Check common locations relative to the package
  const candidates = [
    join(process.cwd(), "runtime", "target", "release", BINARY_NAME),
    join(process.cwd(), "..", "runtime", "target", "release", BINARY_NAME),
    join(process.cwd(), "..", "..", "runtime", "target", "release", BINARY_NAME),
  ];

  for (const p of candidates) {
    if (existsSync(p)) return p;
  }

  return null;
}

/**
 * Handle `npx tivana extension` — copy extension to ~/.tivana/extension/
 * and print install instructions.
 */
async function handleExtensionAsync(subArgs) {
  const { fileURLToPath } = await import("node:url");
  const { cpSync } = await import("node:fs");
  const extDest = join(homedir(), ".tivana", "extension");

  if (subArgs.includes("--path")) {
    console.log(extDest);
    process.exit(0);
  }

  if (subArgs.includes("--help")) {
    console.log(`tivana extension — Install the Tivana Chrome extension\n`);
    console.log(`Usage:`);
    console.log(`  npx tivana extension           Copy extension to ~/.tivana/extension/ and print instructions`);
    console.log(`  npx tivana extension --open     Copy and open the folder (macOS/Linux)`);
    console.log(`  npx tivana extension --path     Print the extension directory path`);
    console.log(`  npx tivana extension --help     Show this help`);
    process.exit(0);
  }

  // Find the extension source — could be in the npm package or repo
  const thisFile = fileURLToPath(import.meta.url);
  const candidates = [
    join(thisFile, "..", "..", "extension"),       // npm package: sdk/ts/bin/../extension/
    join(thisFile, "..", "..", "..", "extension"),  // repo: sdk/ts/bin/../../../extension/
    join(process.cwd(), "extension"),              // cwd
  ];

  let extSrc = null;
  for (const c of candidates) {
    if (existsSync(join(c, "manifest.json"))) {
      extSrc = c;
      break;
    }
  }

  if (!extSrc) {
    console.error("❌ Extension files not found in this package.");
    console.error("   Clone the repo to get them: https://github.com/Mostrom-LLC/tivana");
    process.exit(1);
  }

  // Copy to ~/.tivana/extension/
  mkdirSync(extDest, { recursive: true });
  cpSync(extSrc, extDest, { recursive: true, force: true });

  console.log(`✅ Extension installed to: ${extDest}\n`);
  console.log(`To load in Chrome:`);
  console.log(`  1. Open chrome://extensions`);
  console.log(`  2. Enable "Developer mode" (top-right toggle)`);
  console.log(`  3. Click "Load unpacked"`);
  console.log(`  4. Select: ${extDest}`);
  console.log(`  5. Click the Tivana Bridge icon on any tab to attach\n`);

  if (subArgs.includes("--open")) {
    if (platform() === "darwin") {
      spawn("open", [extDest], { stdio: "inherit" });
    } else if (platform() === "linux") {
      spawn("xdg-open", [extDest], { stdio: "inherit" });
    }
  }

  process.exit(0);
}

async function main() {
  const args = process.argv.slice(2);

  // --version flag (only if no other args)
  if (args.length === 1 && (args[0] === "--version" || args[0] === "-v")) {
    console.log(`tivana v${VERSION}`);
    process.exit(0);
  }

  // Subcommand: extension
  if (args[0] === "extension") {
    await handleExtensionAsync(args.slice(1));
    return;
  }

  // --cli-help (help about the CLI wrapper itself, not the runtime)
  if (args.includes("--cli-help")) {
    console.log(`Tivana CLI v${VERSION}`);
    console.log(`\nWraps the Tivana runtime binary. Downloads it on first use.\n`);
    console.log(`Commands:`);
    console.log(`  npx tivana                     Start the runtime`);
    console.log(`  npx tivana extension            Install Chrome extension`);
    console.log(`\nBinary cache: ${CACHE_DIR}`);
    console.log(`\nTo clear the cache: rm -rf ~/.tivana/bin`);
    console.log(`To build from source: cd runtime && cargo build --release`);
    process.exit(0);
  }

  // Find binary: local build first, then cached download
  let binaryPath = findLocalBinary();

  if (binaryPath) {
    // Using local build — no download needed
  } else {
    binaryPath = await ensureBinary();
  }

  // Spawn the runtime with all passed args
  const child = spawn(binaryPath, args, {
    stdio: "inherit",
    env: { ...process.env },
  });

  child.on("error", (err) => {
    console.error(`Failed to start Tivana: ${err.message}`);
    process.exit(1);
  });

  child.on("close", (code) => {
    process.exit(code ?? 0);
  });

  // Forward signals
  for (const sig of ["SIGINT", "SIGTERM", "SIGHUP"]) {
    process.on(sig, () => child.kill(sig));
  }
}

main();
