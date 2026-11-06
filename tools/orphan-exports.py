#!/usr/bin/env python3
"""Reject Rust exports/declarations without production C++ users (#564)."""

import re
from pathlib import Path

import baseline_ratchet
from code_lines import RUST_EXPORT, without_comments

ROOT = Path(__file__).resolve().parents[1]
BASELINE = "tools/orphan-export-baseline.txt"
NAME = r"openttd_rust_\w+"
DECLARATION = re.compile(
    rf'(?m)^[ \t]*(?:extern\s+"C"\s+)?(?!return\b|else\b)'
    rf"[\w:<>]+[\w:<> \t*&]*[\s*&]({NAME})\s*\([^;{{}}]*\)\s*(?:noexcept\s*)?;"
)


def violations(files: dict[str, str]) -> set[str]:
    exports, declarations, boundary_declarations, references = (
        set(),
        set(),
        set(),
        set(),
    )
    for path, text in files.items():
        if path.startswith("rust/") and path.endswith(".rs"):
            exports.update(
                RUST_EXPORT.findall(without_comments(text, path, strings=True))
            )
        elif path.startswith("src/") and not path.startswith("src/tests/"):
            source = without_comments(text, path, strings=True)
            # extern "C" was masked with other strings; it isn't needed to find
            # names, but leaving its prefix would hide a one-line declaration.
            source = re.sub(r"\bextern\s+(?=\w)", "", source)
            found = set(DECLARATION.findall(source))
            declarations.update(found)
            if path.startswith("src/rust/"):
                boundary_declarations.update(found)
            references.update(re.findall(NAME, DECLARATION.sub("", source)))
    return {f"no declaration | {name}" for name in exports - declarations} | {
        f"no production user | {name}"
        for name in exports | boundary_declarations
        if name not in references
    }


def main() -> int:
    return baseline_ratchet.main(
        __doc__,
        ROOT,
        BASELINE,
        ("src", "rust"),
        {".cpp", ".h", ".hpp", ".inc", ".rs"},
        violations,
        new_label="new orphan",
        summary="Orphan exports: {count} baselined violations",
    )


if __name__ == "__main__":
    raise SystemExit(main())
