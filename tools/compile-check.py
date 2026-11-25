#!/usr/bin/env python3
"""Compile every comparison probe, and build every runtime adapter; run nothing (#923).

    python3 tools/compile-check.py --base origin/rust-migration

Preflight runs this. It needs this worktree's configured `build-rust` for the
compiler and each adapter's compile command, and never configures one: without
it, it prints a notice and passes. It also passes, with a notice, when the
change since the base touches no shared header, generated ABI input, probe,
runtime adapter, game source or Rust source. Each comparison runs with
PROBE_COMPILE_ONLY set and generated ABI headers fresh from
`tools/generate-abi.py`, so `probe` compiles every variant for syntax only and
each comparison skips its runs (`probe.RUN`). Each runtime adapter is built as
CI's "Freeze runtime and scenario adapters" step builds it: compiled and linked
against both games' built objects, the candidate's `build-rust` and the
reference build, with no debug info, then deleted (#1072). Against the pinned
reference, whose objects never change, an adapter links only when a harness
source under tools/simulation/ changed, and is otherwise compiled for syntax.
When the fresh ABI headers differ from the build's, each candidate adapter
also compiles for syntax against the fresh ones. An adapter's missing input
fails the check; only a comparison's is a named skip. A console that only the candidate's
headers declare, or that calls a definition the change removed, fails here as
it would on CI. A link reads the build as it is, so the check names any changed
source newer than the candidate game: build again before trusting that result. Where `clang++` is installed, the build's compiles of
fork sources under tools/ and src/tests/ that read a touched file also run
through `clang++ -fsyntax-only -Wmissing-field-initializers` with the build's
other flags (#933); any Clang diagnostic there fails the check, as CI's Clang
annotation check would. Each child refuses to start any program but a compiler
or a read of the repository and build, and a refusal fails the check.
"""

import argparse
import fcntl
import filecmp
import json
import os
import re
import runpy
import shlex
import shutil
import subprocess
import sys
import time
from collections.abc import Generator
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager
from pathlib import Path

import probe

import migration

ROOT = migration.ROOT
BUILD = ROOT / "build-rust"
OUT = migration.LOCAL / "preflight/compile-check"
# Shared C++ headers, probe and adapter fixtures, and the drivers that build them.
INPUTS = re.compile(
    r"^(src/.*\.(h|hpp|inc)|tools/(probe|generate-abi)\.py|tools/[^/]*-comparison\.py"
    r"|tools/(migration|simulation)/.*\.(c|cpp|h|hpp|inc|py))$"
)
# Definitions an adapter links against: a removed one breaks the link with no
# header changed, as engines-console.cpp's own declaration did on #1056 (#1072).
DEFINITIONS = re.compile(r"^(src/(?!tests/).*\.(c|cpp)|rust/.*\.rs)$")
ROLES = ("candidate", "reference")
# The reference game is pinned, so its objects never change: only a changed
# harness source can newly break an adapter's link against it.
HARNESS = re.compile(r"^tools/simulation/")
COMPILE_ONLY = ", compile only"
FAILED = 3


def changed(base: str) -> list[str]:
    merge_base = migration.git("merge-base", base, "HEAD")
    # The working tree against the merge base: committed and uncommitted changes.
    return migration.git("diff", "--name-only", merge_base, "--").splitlines()


def generated_abi_changed(fresh: Path) -> list[str]:
    """Headers the current Rust declarations generate unlike the build's."""
    shutil.rmtree(fresh, ignore_errors=True)
    subprocess.run(
        [sys.executable, str(ROOT / "tools/generate-abi.py"), str(fresh)], check=True
    )
    built = BUILD / "generated-abi"
    if not built.is_dir():
        return ["(no generated-abi in build-rust)"]
    comparison = filecmp.dircmp(fresh, built)
    common = comparison.common_files
    differ = filecmp.cmpfiles(fresh, built, common, shallow=False)[1]
    return comparison.left_only + comparison.right_only + differ


# Comparisons that compile no fixture: they run built tools or Rust tests only.
NO_FIXTURE = {"generator-comparison"}
# The only programs a child may start: compilers and linkers, reads of the
# repository and build (git show, nm of built objects, ninja -t commands), and
# the binutils an adapter's interposed archive copy needs (ar, objcopy, readelf).
ALLOWED = re.compile(
    r"^((c|g|clang)\+\+|gcc|cc|clang|git|nm|ninja|ar|objcopy|readelf)(-[0-9.]+)?$"
)
RAN = 4
# A comparison stopped at a generated input no compile-only step writes (#933).
MISSING = 5


def comparisons() -> list[Path]:
    """Comparison scripts that compile a fixture through probe."""
    return [
        path
        for path in sorted((ROOT / "tools").glob("*-comparison.py"))
        if path.stem not in NO_FIXTURE
        and re.search(r"\bprobe\.(compile|compare)\(", path.read_text())
    ]


def guard() -> None:
    """Refuse to start any program a compile-only check must not run (#923)."""

    def audit(event: str, arguments: tuple[object, ...]) -> None:
        if event in ("os.system", "os.exec"):
            raise SystemExit(RAN)
        if event != "subprocess.Popen":
            return
        argv = arguments[1]
        argv = [argv] if isinstance(argv, (str, bytes, os.PathLike)) else argv
        argv = [str(part) for part in argv] if isinstance(argv, list | tuple) else []
        name = Path(argv[0]).name if argv else ""
        if not ALLOWED.match(name) or (name.startswith("ninja") and "-t" not in argv):
            print(f"compile-only: refused to run {argv[:1]}", file=sys.stderr)
            raise SystemExit(RAN)

    sys.addaudithook(audit)


def build_of(role: str) -> Path:
    return BUILD if role == "candidate" else migration.REFERENCE_BUILD


def build_adapter(adapter: str, role: str) -> None:
    """One adapter compiled and linked against one game, as CI freezes it (#1072)."""
    from simulation import registry

    names = registry.adapter_modules()
    (module,) = [m for m in names if m.__name__.rsplit(".", 1)[-1] == adapter]
    # A compile-only build of the same adapter can run beside its link.
    mode = "compile-only" if os.environ.get("PROBE_COMPILE_ONLY") else "linked"
    folder = OUT / "adapters" / role / mode / adapter
    shutil.rmtree(folder, ignore_errors=True)
    lock = (
        migration.candidate_lock(BUILD, shared=True)
        if role == "candidate"
        else migration.reference_lock(shared=True)
    )
    with lock:
        module.adapter(build_of(role), folder, migration.environment())
    # A linked adapter is a whole game, hundreds of megabytes: keep none.
    shutil.rmtree(folder, ignore_errors=True)


def child(comparison: str | None, adapter: str | None, role: str = "candidate") -> int:
    """One comparison's compiles, or one adapter's build, with every other program refused."""
    guard()
    try:
        if comparison:
            sys.argv = [comparison]
            runpy.run_path(comparison, run_name="__main__")
        else:
            build_adapter(str(adapter), role)
    except subprocess.CalledProcessError as error:
        print(f"compile-only: {error}", file=sys.stderr)
        return FAILED
    except FileNotFoundError as error:
        print(f"compile-only: stopped at {error.filename}", file=sys.stderr)
        # Only a comparison reads inputs a full run generates; an adapter's
        # inputs all come from a build, as on CI.
        return MISSING if comparison else FAILED
    return 0


def jobs(
    probes: bool,
    roles: tuple[str, ...],
    link_reference: bool = True,
    fresh_candidate: bool = False,
) -> list[tuple[str, list[str]]]:
    """Comparison compiles when ``probes``, then each adapter's build per role:
    the reference's compiled only, unless ``link_reference``, and the
    candidate's also compiled against the fresh ABI when ``fresh_candidate``."""
    from simulation import registry

    python = sys.executable
    listed = [
        (path.stem, [python, __file__, "--comparison", str(path)])
        for path in (comparisons() if probes else [])
    ]
    for module in registry.adapter_modules():
        name = module.__name__.rsplit(".", 1)[-1]
        for role in roles:
            command = [python, __file__, "--adapter", name, "--role", role]
            only = role == "reference" and not link_reference
            listed.append((f"adapter {name} {role}{COMPILE_ONLY * only}", command))
            if role == "candidate" and fresh_candidate:
                listed.append((f"adapter {name} candidate{COMPILE_ONLY}", command))
    return listed


def adapter_job(name: str) -> bool:
    """An adapter build that links, so runs with the build's own environment."""
    return name.startswith("adapter ") and not name.endswith(COMPILE_ONLY)


def newer_than_build(paths: list[str]) -> list[str]:
    """Changed sources edited after the candidate game was last linked."""
    game = BUILD / "openttd-rust"
    if not game.is_file():
        return []
    built = game.stat().st_mtime
    sources = [
        ROOT / path for path in paths if DEFINITIONS.match(path) or INPUTS.match(path)
    ]
    return [
        str(path.relative_to(ROOT))
        for path in sources
        if path.is_file() and path.stat().st_mtime > built
    ]


def run(name: str, command: list[str], env: dict[str, str]) -> tuple[str, int, Path]:
    log = OUT / "logs" / f"{name.replace(' ', '-')}.log"
    with log.open("w") as handle:
        code = subprocess.run(
            command, cwd=ROOT, env=env, stdout=handle, stderr=subprocess.STDOUT
        ).returncode
    return name, code, log


DIAGNOSTIC = re.compile(r"^\S+:\d+:\d+: (warning|error): ")
# CompileFlags.cmake gives these to GCC only; clang++ rejects or inverts them.
GCC_ONLY = {"-flifetime-dse=1", "-Wno-redundant-move"}


def clang(entry: dict[str, str], fresh: Path, touched: set[Path]) -> list[str] | None:
    """Clang's diagnostics for one build compile, or None when it reads no touched file."""
    argv = probe.compile_only(shlex.split(entry["command"]), str(fresh)) or []
    source = Path(entry["file"]).resolve().relative_to(ROOT)
    stem = OUT / "clang" / str(source).replace("/", "-")
    command = ["clang++", *(a for a in argv[1:] if a not in GCC_ONLY)]
    directory = Path(entry["directory"])
    # Preprocessing alone lists what the compile reads, at a fraction of its cost.
    depends = [*command, "-M", "-MF", f"{stem}.d"]
    if subprocess.run(depends, cwd=directory, capture_output=True).returncode:
        return [f"{entry['file']}: clang++ could not preprocess it"]
    read = Path(f"{stem}.d").read_text().replace("\\\n", " ").split()[1:]
    if not touched & {(directory / path).resolve() for path in read}:
        return None
    extra = ["-Wmissing-field-initializers", "-Wno-unknown-warning-option"]
    result = subprocess.run(
        [*command, *extra], cwd=directory, capture_output=True, text=True
    )
    return [line for line in result.stderr.splitlines() if DIAGNOSTIC.match(line)]


def clang_pass(touched: set[Path], fresh: Path) -> tuple[list[str], int, int]:
    """Diagnostics, compiles checked and fork compiles listed (#933)."""
    fork = [
        entry
        for entry in json.loads((BUILD / "compile_commands.json").read_text())
        if Path(entry["file"]).resolve().is_relative_to(ROOT / "tools")
        or Path(entry["file"]).resolve().is_relative_to(ROOT / "src/tests")
    ]
    with ThreadPoolExecutor(workers()) as pool:
        results = list(pool.map(lambda entry: clang(entry, fresh, touched), fork))
    checked = [lines for lines in results if lines is not None]
    return (
        sorted({line for lines in checked for line in lines}),
        len(checked),
        len(fork),
    )


def workers() -> int:
    return max(2, (os.cpu_count() or 2) - round(os.getloadavg()[0]))


@contextmanager
def exclusive() -> Generator[None]:
    """One check at a time per worktree: each clears and fills the same tree."""
    OUT.mkdir(parents=True, exist_ok=True)
    with (OUT.parent / "compile-check.lock").open("w") as handle:
        try:
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            print("waiting for another compile check in this worktree")
            fcntl.flock(handle, fcntl.LOCK_EX)
        yield


def check(base: str) -> int:
    with exclusive():
        return checked(base)


def checked(base: str) -> int:
    if (
        not (BUILD / "CMakeCache.txt").is_file()
        or not (BUILD / "compile_commands.json").is_file()
    ):
        print(
            "skipped: this worktree has no configured build-rust, and this check "
            "never configures one; CI compiles the probes and adapters"
        )
        return 0
    OUT.mkdir(parents=True, exist_ok=True)
    fresh = OUT / "generated-abi"
    # Always regenerate: a selected run must compile against this tree's ABI.
    abi = generated_abi_changed(fresh)
    paths = changed(base)
    probes = bool(abi) or any(INPUTS.match(path) for path in paths)
    links = probes or any(DEFINITIONS.match(path) for path in paths)
    # A fork test source alone selects only the Clang pass (#933).
    if not links and not any(path.startswith("src/tests/") for path in paths):
        print(
            "skipped: the change touches no shared header, generated ABI, probe, "
            "runtime adapter, game or Rust source, or fork test source"
        )
        return 0
    roles = ROLES
    if not (migration.REFERENCE_BUILD / "compile_commands.json").is_file():
        print(
            f"SKIP reference adapters: no built reference at {migration.REFERENCE_BUILD}; "
            "candidate adapters that read it, such as network_protocol, fail. Run "
            "python3 tools/migration.py build to check them as CI does"
        )
        roles = ("candidate",)
    stale = str(BUILD / "generated-abi")
    # An adapter compiles and links against its build's own ABI, as CI's does.
    linked = migration.environment()
    env = dict(linked)
    linked["ADAPTER_BUILD_CHECK"] = "1"
    env["CPATH"] = env.get("CPATH", "").replace(stale, str(fresh))
    env["PROBE_COMPILE_ONLY"] = str(fresh)
    for folder in ("logs", "clang", "adapters"):
        shutil.rmtree(OUT / folder, ignore_errors=True)
        (OUT / folder).mkdir()
    started = time.monotonic()
    harness = any(HARNESS.match(path) for path in paths)
    # A build older than the Rust declarations links against its old headers;
    # the fresh ones must still compile, as preflight checked before #1072.
    listed = jobs(probes, roles, harness, bool(abi)) if links else []
    with ThreadPoolExecutor(workers()) as pool:
        results = list(
            pool.map(
                lambda job: run(*job, linked if adapter_job(job[0]) else env), listed
            )
        )
    labels = {FAILED: "compile failed", RAN: "tried to run a program"}
    for name, _code, log in (result for result in results if result[1] == MISSING):
        print(
            f"SKIP {name}: needs an input only a full run generates, so its later "
            f"compiles were not checked; see {log}"
        )
    failed = [
        (name, code, log) for name, code, log in results if code not in (0, MISSING)
    ]
    for name, code, log in failed:
        print(f"FAIL {labels.get(code, 'error')}: {name}; see {log}")
    elapsed = time.monotonic() - started
    print(f"{len(listed) - len(failed)}/{len(listed)} built in {elapsed:.0f}s")
    if any(adapter_job(name) for name, _command in listed) and (
        newer := newer_than_build(paths)
    ):
        print(
            f"NOTE build-rust predates {len(newer)} changed source(s), such as "
            f"{newer[0]}: the adapters linked against older objects, so a result "
            "here can be wrong; run python3 tools/migration.py build, then this again"
        )
    if not shutil.which("clang++", path=env.get("PATH")):
        print("Clang pass skipped: no clang++ on this host")
        return 1 if failed else 0
    started = time.monotonic()
    named = [ROOT / path for path in paths] + [fresh / name for name in abi]
    touched = {path.resolve() for path in named}
    diagnostics, checked, listed_fork = clang_pass(touched, fresh)
    for line in diagnostics:
        print(f"FAIL clang: {line}")
    elapsed = time.monotonic() - started
    print(
        f"Clang pass: {checked} of {listed_fork} fork compiles read a touched file; {elapsed:.0f}s"
    )
    return 1 if failed or diagnostics else 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="origin/rust-migration")
    parser.add_argument("--adapter", help=argparse.SUPPRESS)
    parser.add_argument("--comparison", help=argparse.SUPPRESS)
    parser.add_argument(
        "--role", choices=ROLES, default="candidate", help=argparse.SUPPRESS
    )
    args = parser.parse_args()
    if args.adapter or args.comparison:
        return child(args.comparison, args.adapter, args.role)
    return check(args.base)


if __name__ == "__main__":
    sys.exit(main())
