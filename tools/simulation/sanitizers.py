"""ASan and UBSan reports of sanitized games, split into original and candidate-only.

`migration.py build|simulate --sanitize` exports `OPENTTD_SANITIZE=1`, which points
`migration.REFERENCE_BUILD` and `migration.CANDIDATE_BUILD` at the sanitized trees.
ASan writes each report to the run's `sanitizer/` folder. GCC's UBSan runtime
writes its reports to stderr whatever `log_path` says, so they are split out of
the compared debug log here. A report key keeps the file, function and message kind but
not line numbers or values, which differ between the reference and candidate trees.
"""

import re
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path

SUPPRESSIONS = Path(__file__).with_name("ubsan-suppressions.txt")
REPORTS = "sanitizer"
# The game launcher's RLIMIT_NPROC=0 leaves LeakSanitizer no thread to stop the world.
COMMON_OPTIONS = "detect_leaks=0"

UBSAN_REPORT = re.compile(r"^(?P<path>.+?):\d+:\d+: runtime error: (?P<message>.*)$")
UBSAN_SUMMARY = "SUMMARY: UndefinedBehaviorSanitizer:"
# Stack frames, `note:` lines and the shadow-byte or caret lines of vptr reports.
CONTINUATION = re.compile(r"^(\s+#\d+ |.*: note: |\s+[0-9a-f]{2}( |$)|\s+[\^~ ]+$)")
FRAME = re.compile(r"^\s+#\d+ 0x[0-9a-f]+ in (?P<function>.+?)(?: (?P<location>\S+))?$")
ASAN_REPORT = re.compile(r"ERROR: AddressSanitizer: (?P<kind>\S+)")
NUMBER = re.compile(r"\b(0x[0-9a-f]+|-?\d+)\b")
RUST_HASH = re.compile(r"::h[0-9a-f]{16}\b")


@dataclass
class Findings:
    """One role's report keys in one scenario, each with its first report text."""

    reports: dict[str, str] = field(default_factory=dict)

    def add(self, key: str, text: str) -> None:
        self.reports.setdefault(key, text)


def environment(run_dir: Path) -> dict[str, str]:
    folder = run_dir / REPORTS
    folder.mkdir(exist_ok=True)
    # GCC's UBSan replaces the common flags with UBSAN_OPTIONS at its first report.
    common = f"{COMMON_OPTIONS}:log_path={folder / 'report'}"
    return {
        "ASAN_OPTIONS": common,
        "UBSAN_OPTIONS": f"{common}:print_stacktrace=1:suppressions={SUPPRESSIONS}",
    }


def relative(path: str, roots: Sequence[Path], builds: Sequence[Path] = ()) -> str:
    """Match paired source/build trees, with nested source roots listed first."""
    # ccache rewrites source paths relative to the original compilation directory.
    locations = (
        [(root, Path(path)) for root in roots]
        if Path(path).is_absolute()
        else [(root, build / path) for root, build in zip(roots, builds, strict=False)]
    )
    for root, location in locations:
        resolved = location.resolve()
        if resolved.is_relative_to(root):
            return str(resolved.relative_to(root))
    return path


def frame_of(lines: Iterable[str]) -> tuple[str, str]:
    """The first frame outside the sanitizer runtime, without its Rust hash."""
    for line in lines:
        if (found := FRAME.match(line)) and "libsanitizer" not in (
            found["location"] or ""
        ):
            return RUST_HASH.sub("", found["function"]), found["location"] or "?"
    return "?", "?"


def split_log(
    lines: Sequence[str], roots: Sequence[Path], builds: Sequence[Path] = ()
) -> tuple[list[str], Findings]:
    """Remove UBSan report blocks from a debug log and return them as findings."""
    kept: list[str] = []
    findings = Findings()
    index = 0
    while index < len(lines):
        line = lines[index]
        report = UBSAN_REPORT.match(line)
        if report is None:
            if not line.startswith(UBSAN_SUMMARY):
                kept.append(line)
            index += 1
            continue
        end = index + 1
        while end < len(lines) and CONTINUATION.match(lines[end]):
            end += 1
        block = lines[index:end]
        if end < len(lines) and not lines[end]:
            end += 1  # The blank line that closes a report.
        message = NUMBER.sub("N", report["message"])
        path = relative(report["path"], roots, builds)
        findings.add(
            f"UBSan {path} in {frame_of(block)[0]}: {message}", "\n".join(block)
        )
        index = end
    return kept, findings


def file_findings(
    run_dir: Path,
    roots: Sequence[Path],
    findings: Findings,
    builds: Sequence[Path] = (),
) -> None:
    """Reports in the run's log_path files: ASan's, and UBSan's should one land there."""
    for path in sorted((run_dir / REPORTS).glob("report.*")):
        text = path.read_text(errors="replace")
        _, reports = split_log(text.splitlines(), roots, builds)
        for key, block in reports.reports.items():
            findings.add(key, block)
        if found := ASAN_REPORT.search(text):
            tail = text[found.end() :].splitlines()
            function, location = frame_of(tail)
            source = relative(re.sub(r":\d+(?::\d+)?$", "", location), roots, builds)
            findings.add(
                f"ASan {found['kind']} at {source} in {function}", text[found.start() :]
            )


@dataclass(frozen=True)
class Comparison:
    original: list[str]
    candidate_only: list[str]
    reference_only: list[str]


def compare(roles: Mapping[str, Findings]) -> Comparison:
    """A report in both builds is original behaviour; one only in the candidate is a fault."""
    reference = roles["reference"].reports
    candidate = roles["candidate"].reports
    return Comparison(
        sorted(reference.keys() & candidate.keys()),
        sorted(candidate.keys() - reference.keys()),
        sorted(reference.keys() - candidate.keys()),
    )
