"""C++ TSan conflicts, including library stacks and global/allocation identity."""

import re
from collections.abc import Sequence
from pathlib import Path

from . import sanitizers

DELIMITER = "=================="
WARNING = re.compile(r"WARNING: ThreadSanitizer: (.+?) \(pid=\d+\)")
ACCESS = re.compile(
    r"^  (?:Previous )?((?:[Aa]tomic )?(?:[Rr]ead|[Ww]rite)) of size (\d+) .*:$"
)
LOCK = re.compile(r"^  Mutex M\d+ acquired here while holding mutex M\d+ .+:$")
FRAME = re.compile(r"^\s+#\d+ (?:0x[0-9a-f]+ (?:in )?)?(.+?) (\S+) \(([^)]+)\)")


def environment(run: Path) -> dict[str, str]:
    folder = run / sanitizers.REPORTS
    folder.mkdir()
    return {"TSAN_OPTIONS": f"halt_on_error=0:exitcode=0:log_path={folder / 'report'}"}


def frame_key(line: str, roots: Sequence[Path], builds: Sequence[Path]) -> str:
    frame = FRAME.match(line)
    if frame is None:
        raise RuntimeError(f"unclassified TSan frame: {line}")
    function, location, module = frame.groups()
    if location == "<null>":
        # Module offsets identify unsymbolized library code, independently of ASLR.
        return f"{function} in {Path(module).name}"
    location = re.sub(r":\d+(?::\d+)?$", "", location)
    source = sanitizers.relative(location, roots, builds)
    if "/libsanitizer/" in source:
        source = "libsanitizer/" + source.split("/libsanitizer/", 1)[1]
    return f"{sanitizers.RUST_HASH.sub('', function)} at {source}"


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
            if warning is None or warning[1] not in (
                "data race",
                "lock-order-inversion (potential deadlock)",
            ):
                raise RuntimeError(f"unclassified TSan output in {path}: {block[:200]}")
            stacks: list[list[str]] = []
            allocation: list[str] = []
            identity = "lock order" if warning[1].startswith("lock-order") else ""
            active: list[str] | None = None
            for line in block.splitlines():
                if access := ACCESS.match(line):
                    stacks.append([f"{access[1].lower()} of size {access[2]}"])
                    active = stacks[-1]
                elif LOCK.match(line):
                    stacks.append(["mutex acquisition"])
                    active = stacks[-1]
                elif line.startswith("  Location is "):
                    identity = re.sub(r" at 0x[0-9a-f]+.*$", "", line.strip())
                    active = allocation
                elif not line.strip():
                    active = None
                elif active is not None and re.match(r"\s+#\d+ ", line):
                    active.append(frame_key(line, roots, builds))
            if (
                len(stacks) < 2
                or any(len(stack) < 2 for stack in stacks)
                or not identity
            ):
                raise RuntimeError(
                    f"TSan report lacks conflicting stacks or location: {path}"
                )
            accesses = sorted(" > ".join(stack) for stack in stacks)
            if allocation:
                identity += " allocated at " + " > ".join(allocation)
            found.add(
                f"TSan {warning[1]}: {identity}: {' <> '.join(accesses)}", block.strip()
            )
    return found
