#!/usr/bin/env python3
"""Compile every comparison probe and runtime adapter; link and run nothing (#923).

    python3 tools/compile-check.py --base origin/rust-migration

Preflight runs this. It needs this worktree's configured `build-rust` for the
compiler and each adapter's compile command, and never configures one: without
it, it prints a notice and passes. It also passes, with a notice, when the
change since the base touches no shared header, generated ABI input, probe or
runtime adapter. Generated ABI headers come fresh from `tools/generate-abi.py`,
not from the build. Each comparison runs with PROBE_COMPILE_ONLY set, so
`probe` compiles for syntax only and runs nothing; a comparison that reads a
run's output stops there, which is reported and does not fail.
"""

import argparse
import filecmp
import os
import re
import shutil
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

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


def generated_abi_changed(fresh: Path) -> bool:
    """Whether the current Rust declarations generate other headers than the build's."""
    shutil.rmtree(fresh, ignore_errors=True)
    subprocess.run(
        [sys.executable, str(ROOT / "tools/generate-abi.py"), str(fresh)], check=True
    )
    built = BUILD / "generated-abi"
    if not built.is_dir():
        return True
    comparison = filecmp.dircmp(fresh, built)
    return bool(
        comparison.left_only
        or comparison.right_only
        or filecmp.cmpfiles(fresh, built, comparison.common_files, shallow=False)[1]
    )


def comparisons() -> list[Path]:
    """Comparison scripts that compile a fixture through probe."""
    return [
        path
        for path in sorted((ROOT / "tools").glob("*-comparison.py"))
        if re.search(r"\bprobe\.(compile|compare)\(", path.read_text())
    ]


def adapter(name: str) -> int:
    """Compile one runtime adapter's console source the way the runtime builds it."""
    from simulation import registry

    (module,) = [m for m in registry.adapter_modules() if m.__name__.endswith(name)]
    try:
        module.adapter(BUILD, OUT / "adapters" / name, migration.environment())
    except subprocess.CalledProcessError as error:
        print(f"compile-only: {error}", file=sys.stderr)
        return FAILED
    return 0


def jobs() -> list[tuple[str, list[str]]]:
    from simulation import registry

    python = sys.executable
    listed = [(path.stem, [python, str(path)]) for path in comparisons()]
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
    inputs = [path for path in changed(base) if INPUTS.match(path)]
    if not inputs and not generated_abi_changed(fresh):
        print(
            "skipped: the change touches no shared header, generated ABI, probe "
            "or runtime adapter"
        )
        return 0
    if not fresh.is_dir():
        generated_abi_changed(fresh)
    stale = str(BUILD / "generated-abi")
    env = migration.environment()
    env["CPATH"] = env.get("CPATH", "").replace(stale, str(fresh))
    env["PROBE_COMPILE_ONLY"] = str(fresh)
    shutil.rmtree(OUT / "logs", ignore_errors=True)
    (OUT / "logs").mkdir()
    started = time.monotonic()
    listed = jobs()
    workers = max(2, (os.cpu_count() or 2) - round(os.getloadavg()[0]))
    with ThreadPoolExecutor(workers) as pool:
        results = list(pool.map(lambda job: run(*job, env), listed))
    failed = [(name, log) for name, code, log in results if code == FAILED]
    stopped = [(name, log) for name, code, log in results if code not in (0, FAILED)]
    for name, log in stopped:
        print(f"stopped early, not a compile failure: {name}; see {log}")
    for name, log in failed:
        print(f"FAIL compile: {name}; see {log}")
    elapsed = time.monotonic() - started
    print(f"{len(listed) - len(failed)}/{len(listed)} compiled in {elapsed:.0f}s")
    return 1 if failed else 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="origin/rust-migration")
    parser.add_argument("--adapter", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.adapter:
        return adapter(args.adapter)
    return check(args.base)


if __name__ == "__main__":
    sys.exit(main())
