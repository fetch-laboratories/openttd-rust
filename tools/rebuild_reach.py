"""Count the C++ objects whose inputs a change touches (#1070).

Ninja reruns an object's compile command when any file it was compiled from
changes, and ccache serves that command only when its preprocessed text was
compiled before. So a build that crosses a change compiles again at most every
object whose source, header or generated ABI header the change touched; an
object whose only change is a comment or other preprocessor-invisible text still
hits the cache. This module replays that count from Git: it reads which files each object includes from a built
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
PULL = re.compile(r"^Merge pull request #(\d+)")
DAY = re.compile(r"^\d{4}-\d{2}-\d{2}$")


@dataclass
class Commit:
    sha: str
    pull: int | None
    objects: int
    total: int
    headers: list[tuple[str, int]] = field(default_factory=list)
    abi_unknown: bool = False


@dataclass
class Summary:
    commits: int
    commits_that_recompile: int
    median: float
    total: int


@dataclass
class Report:
    commits: list[Commit]
    summary: Summary


@dataclass
class Reach:
    objects: int
    reach: dict[str, int]


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
    """A saved graph names paths relative to the build that wrote it, so
    --deps needs that build's --build (its CMakeCache.txt gives the source)."""
    if args.deps and not args.build:
        raise ValueError("--deps FILE needs --build DIR, the build that wrote it")
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


def since(value: str) -> str:
    """A bare date means that day's start in UTC, not the current time of day."""
    return f"{value} 00:00:00 +0000" if DAY.match(value) else value


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
        "--merges",
        "--reverse",
        "--format=%H %s",
        f"--since={since(args.since)}",
        args.base,
    )
    return [
        sha for sha, _, subject in (line.partition(" ") for line in log.splitlines())
        if PULL.match(subject)
    ]  # fmt: skip


def measure(cwd: Path, objects: dict[str, set[str]], sha: str) -> Commit:
    changed, abi_unknown = changed_inputs(cwd, sha)
    count, headers = invalidated(objects, changed, abi_unknown)
    pull = PULL.match(git(cwd, "log", "-1", "--format=%s", sha))
    return Commit(
        sha[:10],
        int(pull[1]) if pull else None,
        count,
        len(objects),
        headers.most_common(3),
        abi_unknown,
    )


def summary(measured: list[Commit]) -> Summary:
    """Median and total over the commits that touch any object's inputs."""
    counts = [c.objects for c in measured if c.objects]
    return Summary(
        len(measured),
        len(counts),
        statistics.median(counts) if counts else 0,
        sum(counts),
    )


def add_parser(commands: argparse._SubParsersAction[argparse.ArgumentParser]) -> None:
    command = commands.add_parser(
        "rebuilds",
        help="C++ objects whose inputs each merge changes",
        description=(
            "For each commit, count the candidate's C++ objects whose source, header"
            " or generated ABI header differs from its first parent: a local build"
            " that crosses the commit reruns those compiles, and ccache serves only"
            " the ones whose preprocessed text it has seen, such as comment-only"
            " edits (#1070). The count is an estimate from one build's include graph."
            " With no commits, take the pull-request merges on the base's"
            " first-parent line since --since (a bare date is that day from 00:00"
            " UTC). --reach HEADER counts the objects that include a header."
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
        "--deps",
        metavar="FILE",
        help="saved `ninja -t deps` output of --build, which it then requires",
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
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
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
            print(json.dumps(asdict(Reach(len(objects), reach)), indent=2))
            return 0
        for header, count in reach.items():
            print(f"{header}: {count} of {len(objects)} objects")
        return 0
    measured = [measure(cwd, objects, sha) for sha in commits(cwd, args)]
    totals = summary(measured)
    if args.json:
        print(json.dumps(asdict(Report(measured, totals)), indent=2))
        return 0
    for c in measured:
        pull = f"#{c.pull}" if c.pull else "-"
        top = ", ".join(f"{h} {n}" for h, n in c.headers)
        flag = " (ABI headers not generated; all counted)" if c.abi_unknown else ""
        print(f"{c.sha} {pull}: {c.objects} of {c.total} objects{flag}; {top}")
    print(
        f"Median {totals.median} objects over the {totals.commits_that_recompile}"
        f" of {totals.commits} commits that change any; total {totals.total}."
    )
    return 0
