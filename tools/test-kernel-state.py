"""Default CI guard against giving migrated C++ globals thread-local scope."""

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def production_thread_locals(source: str) -> list[str]:
    """Discard cfg(test) items, then identify each thread_local static."""
    # Mask comments and string literals so their braces cannot affect item bounds.
    masked = re.sub(
        r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"',
        lambda match: " " * len(match[0]),
        source,
        flags=re.S,
    )
    while match := re.search(r"#\[cfg\(test\)\]", masked):
        begin = masked.find("{", match.end())
        semicolon = masked.find(";", match.end())
        if semicolon != -1 and (begin == -1 or semicolon < begin):
            end = semicolon + 1
            masked = (
                masked[: match.start()] + " " * (end - match.start()) + masked[end:]
            )
            continue
        if begin == -1:
            raise ValueError("cfg(test) item has no body or terminator")
        end, depth = begin + 1, 1
        while depth:
            depth += (masked[end] == "{") - (masked[end] == "}")
            end += 1
        masked = masked[: match.start()] + " " * (end - match.start()) + masked[end:]
    names = []
    for match in re.finditer(r"thread_local\s*!\s*([\{\(\[])", masked):
        opener = match[1]
        closer = {"{": "}", "(": ")", "[": "]"}[opener]
        end, depth = match.end(), 1
        while depth:
            depth += (masked[end] == opener) - (masked[end] == closer)
            end += 1
        names.extend(re.findall(r"\bstatic\s+(\w+)", masked[match.end() : end]))
    return names


class KernelStateTests(unittest.TestCase):
    def test_repository_globals_keep_process_scope(self) -> None:
        violations = []
        for path in sorted((ROOT / "rust/openttd-kernels/src").rglob("*.rs")):
            for name in production_thread_locals(path.read_text()):
                violations.append(f"{path.relative_to(ROOT)}: {name}")
        self.assertEqual(
            violations, [], "Production thread-local state: " + str(violations)
        )

    def test_test_items_and_comments_are_excluded(self) -> None:
        self.assertEqual(
            production_thread_locals(
                "// thread_local! { static COMMENT: X = X; }\n"
                "#[cfg(test)] mod tests { thread_local! { static TEST: X = X; } }\n"
                "#[cfg(test)] static TEST_SCALAR: u8 = 0;\n"
                "thread_local! { static GLOBAL: X = X; static SECOND: X = X; static DEPOT_SCRATCH: X = X; }"
                "thread_local!(static PAREN: X = X::new());"
                "thread_local![static BRACKET: [u8; 2] = [0; 2];];"
            ),
            ["GLOBAL", "SECOND", "DEPOT_SCRATCH", "PAREN", "BRACKET"],
        )


if __name__ == "__main__":
    unittest.main()
