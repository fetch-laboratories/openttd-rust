"""AddressSanitizer and UndefinedBehaviorSanitizer reports of a harness game (#505).

A sanitized game's reports end up in files beside its saves, so they never enter the
stderr log the two roles are compared on. A report not named in
`migration/sanitizer-suppressions.txt` fails the scenario, in either role: a
reference finding is original behavior and needs a suppression with its reason.
Only C and C++ are instrumented; the Rust archive is linked uninstrumented.
"""

import fnmatch
import re
from functools import cache
from pathlib import Path

MARKER = "OPENTTD_SANITIZE"
SUPPRESSIONS = (
    Path(__file__).resolve().parents[2] / "migration/sanitizer-suppressions.txt"
)
# Leaks at exit are original behavior; the harness launcher also denies the stop-the-world thread.
ASAN_OPTIONS = "detect_leaks=0"
# A path inside the game or fork source, whatever checkout it was built from.
SOURCE = re.compile(r"(?<![\w-])((?:src|rust)/[^\s:)]+)(?::(\d+))?")
UBSAN = re.compile(r"^(\S+?):(\d+):\d+: runtime error: ([^:,]+)", re.M)
ASAN = re.compile(r"ERROR: AddressSanitizer: (\S+)(.*?)(?:\n\n|\Z)", re.S)


def environment(run_dir: Path) -> dict[str, str]:
    """Options for one game: AddressSanitizer reports go to a file under the run."""
    directory = run_dir / "sanitizer"
    directory.mkdir()
    return {
        "ASAN_OPTIONS": f"{ASAN_OPTIONS}:log_path={directory}/asan",
        "UBSAN_OPTIONS": "print_stacktrace=1",
    }


def separate(run_dir: Path) -> None:
    """Move UBSan reports, which libubsan prints to stderr whatever `log_path` says,
    out of the log the two roles are compared on."""
    log = run_dir / "stderr.log"
    kept: list[bytes] = []
    moved: list[bytes] = []
    reporting = False
    for line in log.read_bytes().splitlines(keepends=True):
        reporting = b": runtime error: " in line or (
            reporting and (line.startswith(b"    #") or line == b"\n")
        )
        (moved if reporting else kept).append(line)
    if moved:
        log.write_bytes(b"".join(kept))
        (run_dir / "sanitizer/ubsan").write_bytes(b"".join(moved))


def location(path: str, line: str = "") -> str:
    found = SOURCE.search(path + (":" + line if line else ""))
    return found.group(0) if found else path


def signatures(text: str) -> list[str]:
    """One stable line per report: kind and first source location, without addresses."""
    found = [
        f"UndefinedBehaviorSanitizer: {kind} at {location(path, line)}"
        for path, line, kind in UBSAN.findall(text)
    ]
    for kind, trace in ASAN.findall(text):
        frames = re.findall(r"^\s*#\d+ .* in (.*)$", trace, re.M)
        first = next(
            (f for f in frames if SOURCE.search(f)), frames[0] if frames else ""
        )
        found.append(f"AddressSanitizer: {kind} at {location(first.split(' ')[-1])}")
    return sorted(set(found))


@cache
def suppressions() -> list[tuple[str, str]]:
    """(pattern, reason) per line `pattern # reason`; reasons name the original code or issue."""
    entries = []
    for line in SUPPRESSIONS.read_text().splitlines():
        if line.strip() and not line.startswith("#"):
            pattern, _, reason = line.partition(" # ")
            entries.append((pattern.strip(), reason.strip()))
    return entries


def check(run: dict, role: str, mode: str, result: dict) -> None:
    """Fail the scenario on each unsuppressed report; record suppressed ones."""
    for report in sorted((run["dir"] / "sanitizer").glob("*")):
        for signature in signatures(report.read_text(errors="replace")):
            reason = next(
                (r for p, r in suppressions() if fnmatch.fnmatchcase(signature, p)),
                None,
            )
            if reason is None:
                result["problems"].append(
                    f"{mode}: {role} sanitizer: {signature} ({report})"
                )
            else:
                result["notes"].append(
                    f"{mode}: {role} suppressed: {signature}: {reason}"
                )
