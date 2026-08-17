#!/usr/bin/env bash
# Publish the web client.
#
# This used to build the site locally and push it to a gh-pages branch. It
# must not do that any more, and the reason is worth writing down, because the
# failure it caused was expensive to read.
#
# The site is deployed by .github/workflows/pages.yml, which runs on every
# push to main and uploads a Pages artifact. That requires the repository's
# Pages source to be `build_type: workflow`. Pushing a gh-pages branch and
# then asking for a build the old way — `POST /pages/builds` — flips the
# repository BACK to `build_type: legacy` with its source set to that branch.
# The two paths cannot both be live: whichever ran last owns the site, and the
# legacy builder then sat wedged in `building` while the workflow, which was
# still succeeding on every push, deployed nothing anyone could see.
#
# So publishing is now just "make sure main is pushed", plus an explicit
# re-run for when you want a deploy without a commit.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

REPO="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"

BUILD_TYPE="$(gh api "repos/$REPO/pages" --jq .build_type 2>/dev/null || echo unknown)"
if [ "$BUILD_TYPE" != "workflow" ]; then
  echo "Pages is on '$BUILD_TYPE'; the workflow needs 'workflow'. Fixing."
  gh api -X PUT "repos/$REPO/pages" -f build_type=workflow >/dev/null
fi

if [ -n "$(git status --porcelain -- mega_blastoise_web .github/workflows/pages.yml)" ]; then
  echo "warning: uncommitted web changes; the deploy builds from origin/main, not your tree" >&2
fi

if [ -n "$(git log --oneline @{u}..HEAD 2>/dev/null)" ]; then
  echo "pushing main..."
  git push origin main
else
  echo "main already pushed; dispatching a rebuild"
  gh workflow run pages.yml --ref main
fi

echo "watching the deploy..."
sleep 5
gh run watch "$(gh run list --workflow=pages.yml --limit 1 --json databaseId --jq '.[0].databaseId')" --exit-status \
  && echo "done: $(gh api "repos/$REPO/pages" --jq .html_url)"
