#!/usr/bin/env bash
# Connects the custom features to a FigCraft source tree:
#   1. copies everything under overlay/ into it (our own files, same paths as in FigCraft);
#   2. applies every patches/*.patch in order (the few lines that hook our files in).
#
# Usage: scripts/apply.sh <path-to-figcraft-checkout>
# Exits non-zero, naming the file, when a patch no longer fits the new FigCraft version.
set -euo pipefail

here="$(cd "$(dirname "$0")/.." && pwd)"
target="${1:?usage: apply.sh <figcraft-dir>}"
target="$(cd "$target" && pwd)"

if [ ! -f "$target/Cargo.toml" ] || [ ! -d "$target/crates" ]; then
  echo "error: $target is not a FigCraft checkout" >&2
  exit 2
fi

echo "==> Copying overlay files"
(cd "$here/overlay" && find . -type f) | while read -r f; do
  f="${f#./}"
  if [ -e "$target/$f" ]; then
    echo "   ! $f already exists in FigCraft: it is replaced by ours (check it is still needed)"
  fi
  mkdir -p "$target/$(dirname "$f")"
  cp "$here/overlay/$f" "$target/$f"
  echo "   + $f"
done

echo "==> Applying patches"
shopt -s nullglob
for p in "$here"/patches/*.patch; do
  name="$(basename "$p")"
  if git -C "$target" apply --check "$p" 2>/dev/null; then
    git -C "$target" apply "$p"
    echo "   ok $name"
  elif git -C "$target" apply --3way "$p"; then
    echo "   ok $name (merged around upstream changes)"
  else
    echo "error: $name no longer applies to this FigCraft version." >&2
    echo "       The hook lines it changes were edited upstream; update the patch." >&2
    exit 1
  fi
done

echo "==> Done: custom features connected"
