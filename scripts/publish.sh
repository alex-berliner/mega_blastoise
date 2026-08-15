#!/usr/bin/env bash
# Build the web client and publish it to the gh-pages branch.
#
# GitHub Pages serves this repo from a branch rather than from Actions, so
# publishing is a local build plus a push. Switch back to the workflow with:
#   gh api -X PUT repos/<owner>/<repo>/pages -f build_type=workflow
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WEB="$ROOT/mega_blastoise_web"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "building wasm..."
(cd "$WEB" && wasm-pack build --target web --release >/dev/null)

echo "assembling site..."
SITE="$WORK/_site"
mkdir -p "$SITE/pkg"
cp "$WEB"/www/{index.html,style.css,app.js,ui_flow.html,device.html,device.css,device.js,devconsole.js} "$SITE"/
cp "$WEB"/pkg/*.js "$WEB"/pkg/*.wasm "$SITE/pkg/"
# Without this, Jekyll drops anything starting with an underscore.
touch "$SITE/.nojekyll"

echo "publishing..."
REMOTE="$(git -C "$ROOT" remote get-url origin)"
git clone -q --depth 1 --branch gh-pages "$REMOTE" "$WORK/ghp" 2>/dev/null \
  || { git clone -q --depth 1 "$REMOTE" "$WORK/ghp"; git -C "$WORK/ghp" checkout -q --orphan gh-pages; }
cd "$WORK/ghp"
find . -mindepth 1 -maxdepth 1 -not -name '.git' -exec rm -rf {} +
cp -r "$SITE/." .
git add -A
if git diff --cached --quiet; then
  echo "no changes to publish"
  exit 0
fi
git commit -q -m "Publish site from main@$(git -C "$ROOT" rev-parse --short HEAD)"
git push -q origin gh-pages
echo "pushed; requesting Pages build"
gh api -X POST "repos/$(gh repo view --json nameWithOwner --jq .nameWithOwner)/pages/builds" >/dev/null
echo "done: $(gh repo view --json homepageUrl --jq .homepageUrl 2>/dev/null || echo 'see repo Pages settings')"
