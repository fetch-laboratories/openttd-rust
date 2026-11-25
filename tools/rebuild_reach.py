"""Count the C++ objects a change makes every local build compile again (#1070).

ccache and Ninja reuse an object only while every file it was compiled from is
unchanged, so a build that crosses a change compiles every object whose source,
header or generated ABI header that change touched. This module replays that
count from Git: it reads which files each object includes from a built
candidate tree (`ninja -t deps`), then for each commit compares those files
with the commit's first parent. Generated ABI headers are regenerated from each
commit's own Rust inputs with that commit's tools/generate-abi.py.

The include graph is the build tree's, applied to older commits, so a count is
exact for commits near the build and an estimate further back.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import statistics
import subprocess
import tarfile
import tempfile
from collections import Counter
from dataclasses import asdict, dataclass, field
from io import BytesIO
from pathlib import Path

ABI = "generated-abi/"
ABI_INPUTS = (
    "rust/openttd-kernels/src",
    "src/rust",
    "tools/migration/windows-abi",
    "tools/generate-abi.py",
)
DEFAULT_BASE = "origin/rust-migration"
DEPS_HEADER = re.compile(r"^(\S.*?): #deps \d+")
PULL = re.compile(r"Merge pull request #(\d+)")


@dataclass
class Commit:
    sha: str
    pull: int | None
    objects: int
    total: int
    headers: list[tuple[str, int]] = field(default_factory=list)
    abi_unknown: bool = False


def git(cwd: Path, *args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=cwd, check=True, capture_output=True, text=True
    ).stdout


def parse_deps(text: str, build: Path, source: Path) -> dict[str, set[str]]:
    """Each object's inputs as source-relative paths, or generated-abi/NAME."""
    objects: dict[str, set[str]] = {}
    inputs: set[str] | None = None
    build, source = build.resolve(), source.resolve()
    for line in text.splitlines():
        if header := DEPS_HEADER.match(line):
            name = header[1]
            inputs = objects.setdefault(name, set()) if name.endswith(".o") else None
        elif inputs is not None and line.strip():
            path = (build / line.strip()).resolve()
            if path.is_relative_to(build / ABI.rstrip("/")):
                inputs.add(ABI + path.name)
            elif path.is_relative_to(source) and not path.is_relative_to(build):
                inputs.add(path.relative_to(source).as_posix())
    return objects


def build_source(build: Path) -> Path:
    """The source tree a build was configured from; its includes name that tree."""
    cache = build / "CMakeCache.txt"
    if cache.exists():
        for line in cache.read_text().splitlines():
            if line.startswith("CMAKE_HOME_DIRECTORY:"):
                return Path(line.split("=", 1)[1])
    return build.parent


def read_deps(args: argparse.Namespace, checkout: Path) -> dict[str, set[str]]:
    build = Path(args.build or checkout / "build-rust")
    if args.deps:
        text = Path(args.deps).read_text()
    else:
        text = subprocess.run(
            ["ninja", "-C", str(build), "-t", "deps"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    return parse_deps(text, build, build_source(build))


def blobs(cwd: Path, sha: str) -> dict[str, str]:
    listing = git(cwd, "ls-tree", "-r", "-z", sha)
    result = {}
    for entry in filter(None, listing.split("\0")):
        meta, path = entry.split("\t", 1)
        result[path] = meta.split()[2]
    return result


def abi_headers(cwd: Path, sha: str) -> dict[str, str] | None:
    """Digest of each generated ABI header at a commit; None if it cannot be made."""
    present = [p for p in ABI_INPUTS if git(cwd, "ls-tree", sha, "--", p).strip()]
    if "tools/generate-abi.py" not in present:
        return {}
    archive = subprocess.run(
        ["git", "archive", sha, *present], cwd=cwd, check=True, capture_output=True
    ).stdout
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        with tarfile.open(fileobj=BytesIO(archive)) as tar:
            tar.extractall(root, filter="data")
        output = root / "out"
        generated = subprocess.run(
            ["python3", "-I", str(root / "tools/generate-abi.py"), str(output)],
            capture_output=True,
        )
        if generated.returncode:
            return None
        return {
            ABI + path.name: hashlib.sha256(path.read_bytes()).hexdigest()
            for path in output.iterdir()
        }


def changed_inputs(cwd: Path, sha: str) -> tuple[set[str], bool]:
    """Files whose content differs from the first parent, and whether the ABI
    headers could not be generated at one end (then all of them count)."""
    parent = git(cwd, "rev-parse", f"{sha}^1").strip()
    old, new = blobs(cwd, parent), blobs(cwd, sha)
    changed = {p for p in old.keys() | new.keys() if old.get(p) != new.get(p)}
    old_abi, new_abi = abi_headers(cwd, parent), abi_headers(cwd, sha)
    if old_abi is None or new_abi is None:
        return changed, True
    return changed | {
        p for p in old_abi.keys() | new_abi.keys() if old_abi.get(p) != new_abi.get(p)
    }, False


def invalidated(
    objects: dict[str, set[str]], changed: set[str], abi_unknown: bool = False
) -> tuple[int, Counter[str]]:
    """Objects with a changed input, and how many of them include each changed file."""
    count, headers = 0, Counter[str]()
    for inputs in objects.values():
        hit = {p for p in inputs if p in changed or (abi_unknown and p.startswith(ABI))}
        if hit:
            count += 1
            headers.update(p for p in hit if not p.endswith((".cpp", ".cc")))
    return count, headers


def commits(cwd: Path, args: argparse.Namespace) -> list[str]:
    if args.revs:
        return [
            git(cwd, "rev-parse", "--verify", f"{r}^{{commit}}").strip()
            for r in args.revs
        ]
    log = git(
        cwd,
        "log",
        "--first-parent",
        "--reverse",
        "--format=%H %s",
        f"--since={args.since}",
        args.base,
    )
    return [line.split()[0] for line in log.splitlines() if PULL.search(line)]


def measure(cwd: Path, objects: dict[str, set[str]], sha: str) -> Commit:
    changed, abi_unknown = changed_inputs(cwd, sha)
    count, headers = invalidated(objects, changed, abi_unknown)
    pull = PULL.search(git(cwd, "log", "-1", "--format=%s", sha))
    return Commit(
        sha[:10],
        int(pull[1]) if pull else None,
        count,
        len(objects),
        headers.most_common(3),
        abi_unknown,
    )


def summary(measured: list[Commit]) -> dict[str, float | int]:
    """Median and total over the commits that recompile anything."""
    counts = [c.objects for c in measured if c.objects]
    return {
        "commits": len(measured),
        "commits_that_recompile": len(counts),
        "median": statistics.median(counts) if counts else 0,
        "total": sum(counts),
    }


def add_parser(commands: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    command = commands.add_parser(
        "rebuilds",
        help="C++ objects each merge makes local builds compile again",
        description=(
            "For each commit, count the candidate's C++ objects whose source, header"
            " or generated ABI header differs from its first parent: every local"
            " build that crosses the commit compiles those again (#1070). With no"
            " commits, take the pull-request merges on the base's first-parent line"
            " since --since. --reach HEADER counts the objects that include a header."
        ),
    )
    command.add_argument("revs", nargs="*", metavar="REV", help="commits to measure")
    command.add_argument(
        "--since", default="7 days ago", help="date for the merges (default 7 days ago)"
    )
    command.add_argument(
        "--base", default=DEFAULT_BASE, metavar="REF", help=f"default {DEFAULT_BASE}"
    )
    command.add_argument(
        "--build", metavar="DIR", help="built candidate tree (default build-rust)"
    )
    command.add_argument(
        "--deps", metavar="FILE", help="saved `ninja -t deps` output of --build"
    )
    command.add_argument(
        "--reach",
        nargs="+",
        metavar="HEADER",
        help="count objects including each header",
    )
    command.add_argument("--json", action="store_true")


def run(args: argparse.Namespace, cwd: Path) -> int:
    source = Path(git(cwd, "rev-parse", "--show-toplevel").strip())
    try:
        objects = read_deps(args, source)
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"rebuilds: cannot read the include graph: {error}")
        print("Build the candidate once (python3 tools/migration.py build), or pass")
        print("--build DIR for another worktree's build-rust or --deps FILE.")
        return 2
    if not objects:
        print("rebuilds: the build tree lists no compiled objects; build it first.")
        return 2
    if args.reach:
        reach = {h: sum(h in inputs for inputs in objects.values()) for h in args.reach}
        if args.json:
            print(json.dumps({"objects": len(objects), "reach": reach}, indent=2))
        for header, count in reach.items():
            print(f"{header}: {count} of {len(objects)} objects")
        return 0
    measured = [measure(cwd, objects, sha) for sha in commits(cwd, args)]
    totals = summary(measured)
    if args.json:
        print(
            json.dumps(
                {"commits": [asdict(c) for c in measured], "summary": totals}, indent=2
            )
        )
        return 0
    for c in measured:
        pull = f"#{c.pull}" if c.pull else "-"
        top = ", ".join(f"{h} {n}" for h, n in c.headers)
        flag = " (ABI headers not generated; all counted)" if c.abi_unknown else ""
        print(f"{c.sha} {pull}: {c.objects} of {c.total} objects{flag}; {top}")
    print(
        f"Median {totals['median']} objects over the {totals['commits_that_recompile']}"
        f" of {totals['commits']} commits that recompile any; total {totals['total']}."
    )
    return 0
