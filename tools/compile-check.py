#!/usr/bin/env python3
"""Compile every comparison probe and runtime adapter; link and run nothing (#923).

    python3 tools/compile-check.py --base origin/rust-migration

Preflight runs this. It needs this worktree's configured `build-rust` for the
compiler and each adapter's compile command, and never configures one: without
it, it prints a notice and passes. It also passes, with a notice, when the
change since the base touches no shared header, generated ABI input, probe or
runtime adapter. Generated ABI headers come fresh from `tools/generate-abi.py`,
not from the build. Each comparison runs with PROBE_COMPILE_ONLY set, so
`probe` compiles every variant for syntax only and each comparison skips its
runs (`probe.RUN`). Where `clang++` is installed, the build's compiles of
fork sources under tools/ and src/tests/ that read a touched file also run
through `clang++ -fsyntax-only -Wmissing-field-initializers` with the build's
other flags (#933); any Clang diagnostic there fails the check, as CI's Clang
annotation check would. Each child refuses to start any program but a compiler
or a read of the repository and build, and a refusal fails the check.
"""

import argparse
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
from concurrent.futures import ThreadPoolExecutor
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
# The only programs a compile-only child may start: compilers, and reads of the
# repository and build (git show, nm of built objects, ninja -t commands).
ALLOWED = re.compile(r"^((c|g|clang)\+\+|gcc|cc|clang|git|nm|ninja)(-[0-9.]+)?$")
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


def child(comparison: str | None, adapter: str | None) -> int:
    """One comparison's compiles, or one adapter's, with every other program refused."""
    guard()
    try:
        if comparison:
            sys.argv = [comparison]
            runpy.run_path(comparison, run_name="__main__")
        else:
            from simulation import registry

            names = registry.adapter_modules()
            (module,) = [m for m in names if m.__name__.rsplit(".", 1)[-1] == adapter]
            folder = OUT / "adapters" / str(adapter)
            module.adapter(BUILD, folder, migration.environment())
    except subprocess.CalledProcessError as error:
        print(f"compile-only: {error}", file=sys.stderr)
        return FAILED
    except FileNotFoundError as error:
        print(f"compile-only: stopped at {error.filename}", file=sys.stderr)
        return MISSING
    return 0


def jobs() -> list[tuple[str, list[str]]]:
    from simulation import registry

    python = sys.executable
    listed = [
        (path.stem, [python, __file__, "--comparison", str(path)])
        for path in comparisons()
    ]
    for module in registry.adapter_modules():
        name = module.__name__.rsplit(".", 1)[-1]
        listed.append((f"adapter {name}", [python, __file__, "--adapter", name]))
    return listed


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


def check(base: str) -> int:
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
    if not abi and not any(INPUTS.match(path) for path in paths):
        print(
            "skipped: the change touches no shared header, generated ABI, probe "
            "or runtime adapter"
        )
        return 0
    stale = str(BUILD / "generated-abi")
    env = migration.environment()
    env["CPATH"] = env.get("CPATH", "").replace(stale, str(fresh))
    env["PROBE_COMPILE_ONLY"] = str(fresh)
    for folder in ("logs", "clang"):
        shutil.rmtree(OUT / folder, ignore_errors=True)
        (OUT / folder).mkdir()
    started = time.monotonic()
    listed = jobs()
    with ThreadPoolExecutor(workers()) as pool:
        results = list(pool.map(lambda job: run(*job, env), listed))
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
    print(f"{len(listed) - len(failed)}/{len(listed)} compiled in {elapsed:.0f}s")
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
    args = parser.parse_args()
    if args.adapter or args.comparison:
        return child(args.comparison, args.adapter)
    return check(args.base)


if __name__ == "__main__":
    sys.exit(main())
