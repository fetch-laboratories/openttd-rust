"""Compare the semantic simulation state of the reference and candidate games."""

import argparse
import concurrent.futures
import json
import os
import shutil
import time
from collections.abc import Mapping, Sequence
from datetime import datetime, timezone
from pathlib import Path
from types import ModuleType
from typing import Any

from . import configuration
from .game import copy_runtime
from .models import ROOT, Scenario
from .registry import adapter_modules, scenario_list, scenario_modules
from .savegame import KNOWN_FAILURES, MASKS
from .scenario_runner import run_scenario


def prune_simulation_runs(root: Path) -> None:
    """Keep the newest ten completed runs; unpublished reports are still active."""
    completed = sorted(
        (run for run in root.iterdir() if (run / "report.json").is_file()),
        key=lambda run: run.name,
        reverse=True,
    )
    for run in completed[10:]:
        shutil.rmtree(run, ignore_errors=True)


def main(driver: ModuleType | None = None) -> int:
    if driver is None:
        import migration as driver

    migration = driver
    configuration.configure(
        configuration.Configuration(
            common_local=migration.COMMON_LOCAL,
            reference=migration.REFERENCE,
            reference_build=migration.REFERENCE_BUILD,
            candidate_build=migration.CANDIDATE_BUILD,
            sanitize=migration.SANITIZE,
        )
    )
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "names", nargs="*", help="run only scenarios whose name contains one of these"
    )
    parser.add_argument(
        "--soak", action="store_true", help="larger set: more seeds, sizes and years"
    )
    parser.add_argument(
        "--self", action="store_true", help="compare the reference with itself"
    )
    parser.add_argument(
        "--candidate",
        type=Path,
        help="candidate binary (default: openttd-rust in migration.CANDIDATE_BUILD)",
    )
    parser.add_argument(
        "--jobs", type=int, default=max(1, min(4, (os.cpu_count() or 2) // 2))
    )
    parser.add_argument(
        "--limit", type=int, default=20, help="differences reported per snapshot"
    )
    parser.add_argument(
        "--timeout", type=int, default=1200, help="seconds per game run"
    )
    parser.add_argument("--list", action="store_true")
    parser.add_argument("--frozen-runtimes", type=Path, help="CI runtime bundle")
    parser.add_argument(
        "--instructions",
        action="store_true",
        help="profile the speed scenarios; no wall-speed samples",
    )
    parser.add_argument(
        "--benchmark",
        type=int,
        metavar="REPETITIONS",
        help="repeat only plain pairs in isolation; retain final scenario outputs",
    )
    parser.add_argument(
        "--prepare-water-save",
        choices=("ferry", "structures"),
        help="build a committed ship fixture using only the pinned reference",
    )
    parser.add_argument(
        "--prepare-aircraft-save",
        action="store_true",
        help="build the supplemental aircraft fixture with the pinned reference",
    )
    args = parser.parse_args()

    if args.jobs < 1:
        parser.error("--jobs must be positive")
    if args.benchmark is not None and args.benchmark < 1:
        parser.error("--benchmark repetitions must be positive")
    if args.benchmark and args.jobs != 1:
        parser.error("--benchmark requires --jobs 1")

    if args.instructions and (args.benchmark or args.soak):
        parser.error("--instructions conflicts with --benchmark/--soak")
    if migration.SANITIZE and (
        args.instructions or args.benchmark or args.frozen_runtimes
    ):
        parser.error(
            "sanitized games take no --instructions, --benchmark or --frozen-runtimes"
        )
    every = scenario_list(args.soak)
    if args.instructions:
        from .instructions import PROFILED

        every = [s for s in every if s["name"] in PROFILED]
    # An exact scenario name selects only that scenario; anything else is a substring filter.
    unmatched = [
        name for name in args.names if not any(name in s["name"] for s in every)
    ]
    if unmatched:
        parser.error(f"no scenario matches selectors {unmatched}; see --list")
    scenarios = [
        s
        for s in every
        if not args.names
        or any(
            n == s["name"] if any(n == t["name"] for t in every) else n in s["name"]
            for n in args.names
        )
    ]
    if not scenarios:
        parser.error(f"no scenario matches {args.names}; see --list")
    if args.list:
        print("\n".join(s["name"] for s in scenarios))
        return 0
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    simulation = migration.LOCAL / "simulation"
    simulation.mkdir(parents=True, exist_ok=True)
    prune_simulation_runs(simulation)
    out = simulation / f"{stamp}-{os.getpid()}"  # concurrent runs never share
    out.mkdir()
    migration.COMMON_LOCAL.mkdir(parents=True, exist_ok=True)
    try:
        if args.prepare_water_save:
            from .ships import prepare_water_save

            prepare_water_save(args.prepare_water_save, migration, out, args.timeout)
            return 0
        if args.prepare_aircraft_save:
            from .aircraft import prepare_aircraft_save

            prepare_aircraft_save(migration, out, args.timeout)
            return 0
        report = run_simulation(args, scenarios, migration, out, parser, stamp)
    finally:
        for role in ("reference", "candidate"):
            shutil.rmtree(out / f"{role}-runtime", ignore_errors=True)
    # Publish completion only after workers and runtime cleanup finish. Atomic
    # replacement keeps concurrent startup pruning away from active runs.
    temporary = out / "report.json.tmp"
    temporary.write_text(json.dumps(report, indent=2) + "\n")
    temporary.replace(out / "report.json")
    results = report["results"]
    passed = sum(r["passed"] for r in results)
    print(
        f"Simulation: {passed}/{len(results)} scenarios equal, "
        f"{sum(r['snapshots'] for r in results)} snapshots in {report['seconds']} s. Report: {out / 'report.json'}"
    )
    return 0 if report["passed"] else 1


def sanitizer_summary(
    results: Sequence[Mapping[str, Any]],
) -> dict[str, dict[str, list[str]]]:
    """Each sanitizer report key with the scenarios that raised it, by category."""
    summary: dict[str, dict[str, list[str]]] = {
        "original": {},
        "candidate_only": {},
        "reference_only": {},
    }
    for result in results:
        for category, keys in summary.items():
            for key in result.get("sanitizer", {}).get(category, []):
                keys.setdefault(key, []).append(result["scenario"])
    return summary


def run_simulation(
    args: argparse.Namespace,
    scenarios: list[Scenario],
    migration: ModuleType,
    out: Path,
    parser: argparse.ArgumentParser,
    stamp: str,
) -> dict[str, Any]:
    candidate_commit = migration.git("rev-parse", "HEAD")
    candidate_status = migration.git("status", "--short")
    builds = {role: out / f"{role}-runtime" for role in ("reference", "candidate")}
    if args.frozen_runtimes:
        if args.self or args.candidate:
            parser.error("--frozen-runtimes conflicts with --self/--candidate")
        bundle = args.frozen_runtimes.resolve()
        binaries: dict[str, Path] = {}
        for role in ("reference", "candidate"):
            source = (
                bundle
                / f"{role}-runtime"
                / ("openttd" if role == "reference" else "openttd-rust")
            )
            binaries[role] = copy_runtime(source.parent, source, builds[role])
            for module in adapter_modules():
                name = module.__name__.rsplit(".", 1)[1]
                shutil.copytree(source.parent / name, builds[role] / name)
    else:
        # The game finds its data next to the executable, so copy the shared
        # reference's runtime while no other worktree can rebuild it,
        # then run without holding the lock.
        with migration.reference_lock(shared=True):
            shared = migration.REFERENCE_BUILD.resolve()
            reference = copy_runtime(shared, shared / "openttd", builds["reference"])
        binaries = {"reference": reference}
        if args.self:
            builds["candidate"], binaries["candidate"] = builds["reference"], reference
        else:
            candidate = (
                args.candidate or migration.CANDIDATE_BUILD / "openttd-rust"
            ).resolve()
            if not candidate.is_file():
                parser.error(
                    f"candidate binary {candidate} is missing; run python3 tools/migration.py build"
                )
            binaries["candidate"] = copy_runtime(
                candidate.parent, candidate, builds["candidate"], candidate=True
            )
    # Freeze the two scenario AIs before workers launch; both roles install the
    # same immutable script bytes while command parameters remain per-run.
    for module in scenario_modules():
        if not hasattr(module, "AI_FOLDER"):
            continue
        folder = module.AI_FOLDER
        selected = [s for s in scenarios if module.uses_ai(s)]
        if not selected:
            continue
        frozen = out / "scenario-ai" / folder
        shutil.copytree(ROOT / "tools" / folder, frozen)
        for scenario in selected:
            scenario["scenario_ai"] = str(frozen)

    started = time.monotonic()
    results = []
    with concurrent.futures.ThreadPoolExecutor(args.jobs) as pool:
        # The driver's environment supplies bootstrapped runtime libraries locally.
        env = migration.environment()
        futures = {
            pool.submit(
                run_scenario,
                s,
                binaries,
                builds,
                out,
                args.limit,
                args.timeout,
                env,
                args.benchmark or 0,
                args.instructions,
            ): s
            for s in scenarios
        }
        for future in concurrent.futures.as_completed(futures):
            result = future.result()
            results.append(result)
            status = "ok  " if result["passed"] else "FAIL"
            known = (
                f", {len(result['known_failures'])} known"
                if result["known_failures"]
                else ""
            )
            ratio = result["plain_speed"]["median_candidate_reference_ratio"]
            speed = f", plain {ratio:.3f}x" if ratio is not None else ""
            budget = result["plain_speed"].get("budget")
            if budget is not None:
                speed += (
                    f", record {budget['record']:.3f}x "
                    f"({budget['change_percent']:+.1f}%)"
                )
            print(
                f"{status} {result['scenario']}: {result['snapshots']} snapshots, "
                f"{result['stats']['chunks']} chunks, {result['stats']['elements']} elements{known}{speed}",
                flush=True,
            )
            for problem in result["problems"]:
                print(f"     {problem}", flush=True)
            for diff in result["differences"][: args.limit]:
                print(
                    f"     {diff['snapshot']} {diff['chunk']}/{diff['element']}/{diff['field']}: "
                    f"{diff['reference']} -> {diff['candidate']}",
                    flush=True,
                )
    results.sort(key=lambda r: r["scenario"])
    report = {
        "schema_version": 1,
        "mode": "reference-vs-reference" if args.self else "reference-vs-candidate",
        "execution": "Linux child RLIMIT_NPROC=0; original synchronous thread-failure fallback",
        "started_at": stamp,
        "benchmark_repetitions": args.benchmark or 0,
        "jobs": args.jobs,
        "candidate_commit": candidate_commit,
        "candidate_status": candidate_status,
        "masks": {":".join(key): reason for key, reason in MASKS.items()},
        "known_failures": {
            ":".join(key): issue for key, issue in KNOWN_FAILURES.items()
        },
        "sanitizers": sanitizer_summary(results) if migration.SANITIZE else None,
        "masked_differences": {
            key: sum(r["stats"]["masked"].get(key, 0) for r in results)
            for key in sorted({k for r in results for k in r["stats"]["masked"]})
        },
        "seconds": round(time.monotonic() - started, 1),
        "results": results,
        "passed": all(r["passed"] for r in results),
    }
    return report
