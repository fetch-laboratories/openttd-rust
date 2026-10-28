"""C++ TSan reports: both conflicting access stacks and the global/allocation identity."""

import re
from collections.abc import Sequence
from pathlib import Path

from . import sanitizers

DELIMITER = "=================="
WARNING = re.compile(r"WARNING: ThreadSanitizer: (.+?) \(pid=\d+\)")
ACCESS = re.compile(
    r"^  (?:Previous )?(?:Atomic )?(?:Read|Write|read|write) of size \d+ .*:$"
)
FRAME = re.compile(r"^\s+#\d+ (.+?) (\S+:\d+(?::\d+)?) \(")


def environment(run: Path) -> dict[str, str]:
    folder = run / sanitizers.REPORTS
    folder.mkdir()
    return {"TSAN_OPTIONS": f"halt_on_error=0:exitcode=0:log_path={folder / 'report'}"}


def findings(
    run: Path, roots: Sequence[Path], builds: Sequence[Path]
) -> sanitizers.Findings:
    found = sanitizers.Findings()
    for path in sorted((run / sanitizers.REPORTS).glob("report.*")):
        text = path.read_text(errors="replace")
        for block in text.split(DELIMITER):
            if not block.strip() or re.fullmatch(
                r"ThreadSanitizer: reported \d+ warnings", block.strip()
            ):
                continue
            warning = WARNING.search(block)
            if warning is None or warning[1] != "data race":
                raise RuntimeError(f"unclassified TSan output in {path}: {block[:200]}")
            stacks: list[list[str]] = []
            identity = ""
            active = False
            for line in block.splitlines():
                if ACCESS.match(line):
                    stacks.append([])
                    active = True
                elif not line.strip():
                    active = False
                elif active and (frame := FRAME.match(line)):
                    location = re.sub(r":\d+(?::\d+)?$", "", frame[2])
                    source = sanitizers.relative(location, roots, builds)
                    # Library/runtime frames vary with compiler and are retained in the full report.
                    if source.startswith(("src/", "rust/")):
                        stacks[-1].append(f"{frame[1]} at {source}")
                elif line.startswith("  Location is "):
                    identity = re.sub(r" at 0x[0-9a-f]+.*$", "", line.strip())
            if len(stacks) != 2 or not all(stacks) or not identity:
                raise RuntimeError(
                    f"TSan report lacks two game stacks or location: {path}"
                )
            accesses = sorted(" > ".join(stack) for stack in stacks)
            found.add(
                f"TSan {warning[1]}: {identity}: {' <> '.join(accesses)}", block.strip()
            )
    return found
