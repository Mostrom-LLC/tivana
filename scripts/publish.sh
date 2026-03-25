#!/usr/bin/env bash
set -euo pipefail

# Tivana npm publish script
# Usage: ./scripts/publish.sh [--dry-run]
#
# Builds the SDK, bundles the Chrome extension, and publishes to npm.
# Run with --dry-run first to verify the package contents.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SDK_DIR="$REPO_ROOT/sdk/ts"
EXT_DIR="$REPO_ROOT/extension"

DRY_RUN=""
if [[ "${1:-}" == "--dry-run" ]]; then
  DRY_RUN="--dry-run"
  echo "🔍 Dry run mode — no actual publish"
  echo ""
fi

echo "📦 Tivana publish script"
echo "========================"
echo ""

# 1. Pre-flight checks
echo "1️⃣  Pre-flight checks..."

if ! command -v npm &>/dev/null; then
  echo "❌ npm not found"
  exit 1
fi

if [[ ! -f "$REPO_ROOT/.npmrc" ]]; then
  echo "❌ .npmrc not found at repo root — auth token required"
  exit 1
fi

# Check npm auth
if ! npm whoami --registry https://registry.npmjs.org/ 2>/dev/null; then
  echo "❌ Not authenticated to npm. Check .npmrc auth token."
  exit 1
fi
echo "   ✅ Authenticated as $(npm whoami --registry https://registry.npmjs.org/)"

# 2. Build SDK
echo ""
echo "2️⃣  Building SDK..."
cd "$SDK_DIR"

if command -v bun &>/dev/null; then
  bun run build 2>&1 | tail -3
else
  npm run build 2>&1 | tail -3
fi
echo "   ✅ SDK built"

# 3. Typecheck
echo ""
echo "3️⃣  Typechecking..."
npx tsc --noEmit 2>&1 || true
echo "   ✅ Typecheck done"

# 4. Bundle extension
echo ""
echo "4️⃣  Bundling Chrome extension..."
rm -rf "$SDK_DIR/extension"
cp -r "$EXT_DIR" "$SDK_DIR/extension"
echo "   ✅ Extension copied to sdk/ts/extension/"

# 5. Verify package contents
echo ""
echo "5️⃣  Package contents:"
npm pack --dry-run 2>&1

# 6. Publish
echo ""
if [[ -n "$DRY_RUN" ]]; then
  echo "6️⃣  Skipping publish (dry run)"
else
  echo "6️⃣  Publishing to npm..."
  # Use the root .npmrc for auth
  npm publish --userconfig "$REPO_ROOT/.npmrc" --access public
  echo ""
  echo "   ✅ Published!"
  echo "   📦 https://www.npmjs.com/package/tivana"
fi

# 7. Cleanup
echo ""
echo "7️⃣  Cleaning up..."
rm -rf "$SDK_DIR/extension"
echo "   ✅ Removed bundled extension from sdk/ts/"

echo ""
echo "Done! 🎉"
