#!/usr/bin/env python3
"""Publish every past assembly view listed in site/views/views.json.

For each entry, its commit's hardware/ref_design/viewer/site goes to
<out>/views/<slug>/. The Pages workflow runs this from a shallow checkout, so a
commit that isn't here is fetched (depth 1) by its SHA, then by its tag
(viewer/<slug>, for a commit on no pushed branch). A
missing commit only warns: the deploy carries on without that view.

    python3 scripts/site_views.py _site
"""
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile

VIEWER = "hardware/ref_design/viewer/site"


def git(*args, **kw):
    return subprocess.run(["git", *args], **kw)


def have(sha):
    return git("cat-file", "-e", sha + "^{commit}", capture_output=True).returncode == 0


def fetch(entry):
    if have(entry["sha"]):
        return True
    for ref in (entry["sha"], entry.get("tag")):
        if ref and git("fetch", "--quiet", "--depth=1", "origin", ref).returncode == 0 and have(entry["sha"]):
            return True
    return False


def warn(msg):
    print(f"::warning::{msg}")


def main():
    out = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "_site") / "views"
    entries = json.loads(pathlib.Path("site/views/views.json").read_text())
    # One fetch for everything missing; if any SHA is unreachable it fails as a
    # whole, and fetch() tries each entry on its own.
    missing = [e["sha"] for e in entries if not have(e["sha"])]
    if missing:
        git("fetch", "--quiet", "--depth=1", "origin", *missing, capture_output=True)
    published = 0
    for e in entries:
        slug = e["slug"]
        if not fetch(e):
            warn(f"views: {slug}: commit {e['sha'][:12]} (tag {e.get('tag', 'none')}) not on origin; not published")
            continue
        with tempfile.TemporaryDirectory() as tmp:
            archive = git("archive", e["sha"], VIEWER, capture_output=True)
            if archive.returncode != 0:
                warn(f"views: {slug}: no {VIEWER} in {e['sha'][:12]}; not published")
                continue
            subprocess.run(["tar", "-x", "-C", tmp], input=archive.stdout, check=True)
            dest = out / slug
            shutil.rmtree(dest, ignore_errors=True)
            shutil.copytree(pathlib.Path(tmp) / VIEWER, dest)
        # A snapshot's own links point at its siblings (../assembly/ and the like),
        # which sit two levels up from /views/<slug>/.
        page = dest / "index.html"
        if page.exists():
            page.write_text(page.read_text().replace('href="../assembly', 'href="../../assembly'))
        published += 1
        print(f"views: {slug} <- {e['sha'][:12]}")
    print(f"views: {published} of {len(entries)} published")


if __name__ == "__main__":
    main()
