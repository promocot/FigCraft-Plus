#!/usr/bin/env python3
"""Connect the custom features to a FigCraft source tree.

1. Copy everything under overlay/ into it (our own files, same paths as in FigCraft).
2. Apply hooks/*.json in name order: small edits that wire our files in. Each hook is one of
     {"file", "after": regex, "insert": text}    insert lines after the first line matching
     {"file", "find": text, "replace": text}     replace exact text (it must occur exactly once)
   A hook whose result is already there is skipped, so running twice is harmless.

Usage: python3 scripts/apply.py <path-to-figcraft-checkout>
Exits non-zero, naming the hook, when an anchor is no longer found in the new FigCraft version.
"""

import json
import re
import shutil
import sys
from pathlib import Path

here = Path(__file__).resolve().parent.parent


def fail(msg: str) -> None:
    print(f"error: {msg}", file=sys.stderr)
    sys.exit(1)


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: apply.py <figcraft-dir>")
    target = Path(sys.argv[1]).resolve()
    if not (target / "Cargo.toml").is_file() or not (target / "crates").is_dir():
        fail(f"{target} is not a FigCraft checkout")

    print("==> Copying overlay files")
    for src in sorted((here / "overlay").rglob("*")):
        if src.is_file():
            rel = src.relative_to(here / "overlay")
            dst = target / rel
            if dst.exists() and dst.read_bytes() != src.read_bytes():
                print(f"   ! {rel} exists in FigCraft: replaced by ours (check it is still needed)")
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(src, dst)
            print(f"   + {rel}")

    print("==> Applying hooks")
    for hook_file in sorted((here / "hooks").glob("*.json")):
        hooks = json.loads(hook_file.read_text(encoding="utf-8"))
        for n, h in enumerate(hooks, 1):
            where = f"{hook_file.name} #{n} ({h.get('note', h['file'])})"
            path = target / h["file"]
            if not path.is_file():
                fail(f"{where}: {h['file']} no longer exists in FigCraft")
            text = path.read_text(encoding="utf-8")
            if "after" in h:
                insert = h["insert"].rstrip("\n") + "\n"
                if insert in text:
                    print(f"   = {where} (already there)")
                    continue
                m = re.search(h["after"], text, re.MULTILINE)
                if not m:
                    fail(f"{where}: no line matches {h['after']!r}; FigCraft changed here, update the hook")
                end = text.find("\n", m.end())
                end = len(text) if end < 0 else end + 1
                text = text[:end] + insert + text[end:]
            else:
                find, replace = h["find"], h["replace"]
                if replace in text:
                    print(f"   = {where} (already there)")
                    continue
                count = text.count(find)
                if count != 1:
                    fail(f"{where}: expected the anchor text once, found it {count} times; FigCraft changed here, update the hook")
                text = text.replace(find, replace)
            path.write_text(text, encoding="utf-8", newline="\n")
            print(f"   ok {where}")
    print("==> Done: custom features connected")


if __name__ == "__main__":
    main()
