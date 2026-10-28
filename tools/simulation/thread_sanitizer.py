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
LOCK = re.compile(r"^  Mutex (M\d+) acquired here while holding mutex (M\d+) .+:$")
CYCLE = re.compile(
    r"^  Cycle in lock order graph: ((?:M\d+ \(0x[0-9a-f]+\) => ){2,}M\d+)$"
)
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
        parts = text.split(DELIMITER)
        footer = re.fullmatch(
            r"ThreadSanitizer: reported (\d+) warnings", parts[-1].strip()
        )
        if (
            len(parts) < 3
            or len(parts) % 2 != 1
            or parts[0].strip()
            or any(part.strip() for part in parts[2:-1:2])
            or footer is None
            or int(footer[1]) != len(parts) // 2
        ):
            raise RuntimeError(f"incomplete/unclassified TSan output in {path}")
        for block in parts[1::2]:
            warning = WARNING.search(block)
            if warning is None or warning[1] not in (
                "data race",
                "lock-order-inversion (potential deadlock)",
            ):
                raise RuntimeError(f"unclassified TSan output in {path}: {block[:200]}")
            if (
                not block.strip()
                .splitlines()[-1]
                .startswith(f"SUMMARY: ThreadSanitizer: {warning[1]}")
            ):
                raise RuntimeError(f"TSan report lacks its final summary: {path}")
            stacks: list[list[str]] = []
            edges: dict[tuple[str, str], list[str]] = {}
            cycle: list[str] = []
            allocation: list[str] = []
            identity = "lock order" if warning[1].startswith("lock-order") else ""
            active: list[str] | None = None
            for line in block.splitlines():
                if access := ACCESS.match(line):
                    stacks.append([f"{access[1].lower()} of size {access[2]}"])
                    active = stacks[-1]
                elif lock := LOCK.match(line):
                    stacks.append(["mutex acquisition"])
                    active = stacks[-1]
                    edge = (lock[2], lock[1])
                    if edge in edges:
                        raise RuntimeError(f"duplicate TSan lock edge: {path}")
                    edges[edge] = active
                elif graph := CYCLE.match(line):
                    cycle = re.findall(r"M\d+", graph[1])
                elif line.startswith("  Location is "):
                    identity = re.sub(r" at 0x[0-9a-f]+.*$", "", line.strip())
                    identity = re.sub(r"\bthread T\d+\.$", "worker thread.", identity)
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
            if warning[1].startswith("lock-order"):
                expected = list(zip(cycle, cycle[1:], strict=False))
                if (
                    len(cycle) < 3
                    or cycle[0] != cycle[-1]
                    or len(set(cycle[:-1])) != len(cycle) - 1
                    or set(expected) != edges.keys()
                ):
                    raise RuntimeError(
                        f"TSan report lacks a complete lock cycle: {path}"
                    )
                ordered = [" > ".join(edges[edge]) for edge in expected]
                # Rotate the directed cycle; report-local mutex IDs have no cross-run identity.
                identity = "lock cycle"
                accesses = [
                    min(
                        " => ".join(ordered[i:] + ordered[:i])
                        for i in range(len(ordered))
                    )
                ]
            if allocation:
                identity += " allocated at " + " > ".join(allocation)
            found.add(
                f"TSan {warning[1]}: {identity}: {' <> '.join(accesses)}", block.strip()
            )
    return found
