#!/usr/bin/env bash
# Build the full kleis.io site and deploy to Cloudflare Pages.
#
# Usage:
#   ./scripts/deploy-site.sh           # build + deploy
#   ./scripts/deploy-site.sh --dry-run # build only, skip deploy
#
# Prerequisites:
#   - mdbook installed          (cargo install mdbook)
#   - wrangler installed + auth (npm i -g wrangler && wrangler login)
#   - nvm with Node.js ≥ 20     (nvm install 20)

set -euo pipefail

# Wrangler requires Node.js ≥ 20. Switch via nvm if available.
export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
if [ -s "$NVM_DIR/nvm.sh" ]; then
    . "$NVM_DIR/nvm.sh"
    nvm use 20 --silent 2>/dev/null || nvm use node --silent 2>/dev/null || true
fi

PROJECT_NAME="kleis"
SITE_DIR="_site"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DRY_RUN=false

for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN=true ;;
        *) echo "Unknown flag: $arg"; exit 1 ;;
    esac
done

cd "$REPO_ROOT"

# ── 1. Build mdBook ─────────────────────────────────────────────────
echo "==> Building mdBook…"
(cd docs/manual && mdbook build)

# ── 2. SEO post-processing ──────────────────────────────────────────
if [ -x scripts/postbuild-manual.sh ]; then
    echo "==> Running SEO post-build…"
    ./scripts/postbuild-manual.sh docs/manual/book
fi

# ── 3. Assemble _site ───────────────────────────────────────────────
echo "==> Assembling ${SITE_DIR}/…"
rm -rf "$SITE_DIR"
mkdir -p "$SITE_DIR"

cp index.html "$SITE_DIR/"
cp favicon.svg "$SITE_DIR/"
cp favicon.ico "$SITE_DIR/" 2>/dev/null || true
cp favicon-48.png "$SITE_DIR/" 2>/dev/null || true
cp favicon-96.png "$SITE_DIR/" 2>/dev/null || true
cp apple-touch-icon.png "$SITE_DIR/" 2>/dev/null || true
cp sitemap.xml "$SITE_DIR/"
cp robots.txt "$SITE_DIR/" 2>/dev/null || true
cp -r static "$SITE_DIR/static"

mkdir -p "$SITE_DIR/docs/manual"
cp -r docs/manual/book "$SITE_DIR/docs/manual/book"

mkdir -p "$SITE_DIR/docs/papers"
cp -r docs/papers/* "$SITE_DIR/docs/papers/"

FILE_COUNT=$(find "$SITE_DIR" -type f | wc -l | tr -d ' ')
echo "    ${FILE_COUNT} files assembled"

# ── 4. Deploy ────────────────────────────────────────────────────────
if [ "$DRY_RUN" = true ]; then
    echo "==> Dry run — skipping deploy. Site ready in ${SITE_DIR}/"
    exit 0
fi

if ! command -v wrangler &>/dev/null; then
    echo "Error: wrangler not found. Install with: npm i -g wrangler" >&2
    exit 1
fi

echo "==> Deploying to Cloudflare Pages (project: ${PROJECT_NAME})…"
wrangler pages deploy "$SITE_DIR" --project-name="$PROJECT_NAME"

echo "==> Done! Site deployed to Cloudflare Pages."
echo "    Add kleis.io as a custom domain in the Cloudflare dashboard if not already configured."
