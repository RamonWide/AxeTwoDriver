"""Writes the licence notices of every crate compiled into the Windows programs.

usage: python scripts/third-party.py <output file>

Reads `cargo metadata` for the Windows target and collects the licence files
each dependency ships. Identical texts are printed once, with the crates they
apply to.
"""
import hashlib
import json
import os
import subprocess
import sys

TARGET = "x86_64-pc-windows-msvc"
NOTICE_PREFIXES = ("license", "licence", "copying", "notice", "unlicense", "copyright")


def main() -> None:
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    meta = json.loads(
        subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--locked", "--filter-platform", TARGET],
            cwd=root,
            capture_output=True,
            text=True,
            check=True,
        ).stdout
    )
    used = {node["id"] for node in meta["resolve"]["nodes"]}
    own = set(meta["workspace_members"])
    crates = sorted(
        (p for p in meta["packages"] if p["id"] in used and p["id"] not in own),
        key=lambda p: (p["name"], p["version"]),
    )

    texts: dict[str, tuple[str, list[str]]] = {}
    without_file = []
    lines = [
        "Third-party software in AxeTwoDriver",
        "====================================",
        "",
        "The programs in this download are built from the following open-source",
        "Rust crates. Each is listed with the licence its authors chose; the",
        "licence texts follow the list.",
        "",
    ]
    for p in crates:
        label = f"{p['name']} {p['version']}"
        lines.append(f"  {label:<40} {p.get('license') or 'see its licence file'}")
        folder = os.path.dirname(p["manifest_path"])
        found = False
        for name in sorted(os.listdir(folder)):
            path = os.path.join(folder, name)
            if not name.lower().startswith(NOTICE_PREFIXES) or not os.path.isfile(path):
                continue
            with open(path, encoding="utf-8", errors="replace") as handle:
                text = handle.read().replace("\r\n", "\n").strip()
            if not text:
                continue
            found = True
            key = hashlib.sha256(text.encode()).hexdigest()
            texts.setdefault(key, (text, []))[1].append(label)
        if not found:
            without_file.append(f"{label} ({p.get('license')}, {p.get('repository')})")

    if without_file:
        lines += ["", "Crates that ship no licence file; see their repositories:", ""]
        lines += [f"  {entry}" for entry in without_file]

    for text, labels in sorted(texts.values(), key=lambda item: -len(item[1])):
        lines += ["", "", "-" * 78, "Applies to: " + ", ".join(sorted(set(labels))), "-" * 78, "", text]

    with open(sys.argv[1], "w", encoding="utf-8", newline="\n") as handle:
        handle.write("\n".join(lines) + "\n")
    print(f"{len(crates)} crates, {len(texts)} distinct licence texts, {len(without_file)} without a file")


main()
