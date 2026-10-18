#!/usr/bin/env python3
"""Partition complete simulation evidence and export native build inputs for CI."""

import argparse
import collections
import json
import re
import shlex
import shutil
import statistics
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

from simulation import core, runtime

import migration

ROOT = Path(__file__).resolve().parents[1]
TIMINGS = ROOT / "migration/ci-scenario-seconds.json"


def fixture_family(scenario):
    # These preparation modules share mutable scratch fixtures within one run.
    # Keeping them together avoids rebuilding them separately on multiple runners.
    if "effects" in scenario:
        return "effects"
    if "disasters" in scenario:
        return "disasters"
    if "aircraft_control" in scenario:
        return "aircraft-control"
    return scenario["name"]


def partition(scenarios, seconds, count):
    if count < 1 or count > len(scenarios):
        raise ValueError("Shard count must be positive and no greater than inventory")
    default = statistics.median(seconds.values()) if seconds else 1
    families = collections.defaultdict(list)
    for scenario in scenarios:
        families[fixture_family(scenario)].append(scenario["name"])
    if count > len(families):
        raise ValueError("Shard count exceeds independent fixture families")
    weights = {
        name: seconds.get(name, default) for s in scenarios for name in [s["name"]]
    }
    ordered = sorted(
        families.values(), key=lambda names: (-sum(weights[n] for n in names), names)
    )
    shards = [{"id": index, "names": [], "seconds": 0} for index in range(count)]
    for names in ordered:
        shard = min(shards, key=lambda s: (s["seconds"], s["id"]))
        shard["names"] += names
        shard["seconds"] += sum(weights[n] for n in names)
    for shard in shards:
        shard["names"].sort()
        shard["seconds"] = round(shard["seconds"], 3)
    return shards


def plan(bundle, count):
    scenarios = core.scenario_list(False)
    names = [s["name"] for s in scenarios]
    if len(names) != len(set(names)):
        raise ValueError("Duplicate scenario names in inventory")
    shards = partition(scenarios, json.loads(TIMINGS.read_text())["seconds"], count)
    result = {
        "schema_version": 1,
        "candidate_commit": migration.git("rev-parse", "HEAD"),
        "inventory": sorted(names),
        "shards": shards,
    }
    (bundle / "shard-plan.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def validate_plan(value):
    names = sorted(s["name"] for s in core.scenario_list(False))
    shards = value["shards"]
    covered = [name for shard in shards for name in shard["names"]]
    if (
        value["schema_version"] != 1
        or value["inventory"] != names
        or sorted(covered) != names
        or not shards
        or any(not s["names"] for s in shards)
        or [s["id"] for s in shards] != list(range(len(shards)))
        or value["candidate_commit"] != migration.git("rev-parse", "HEAD")
    ):
        raise ValueError("Shard plan differs from complete inventory/candidate")


def run_shard(bundle, index, jobs, output):
    value = json.loads((bundle / "shard-plan.json").read_text())
    validate_plan(value)
    if not 0 <= index < len(value["shards"]):
        raise ValueError("Invalid shard index")
    old = set((ROOT / ".local/simulation").glob("*/report.json"))
    command = [
        sys.executable,
        str(ROOT / "tools/simulate.py"),
        "--frozen-runtimes",
        str(bundle),
        "--jobs",
        str(jobs),
        *value["shards"][index]["names"],
    ]
    completed = subprocess.run(command, cwd=ROOT)
    fresh = set((ROOT / ".local/simulation").glob("*/report.json")) - old
    if len(fresh) != 1:
        raise ValueError("Shard did not publish exactly one simulation report")
    report = json.loads(fresh.pop().read_text())
    report["shard"] = index
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2) + "\n")
    return completed.returncode


def aggregate(value, reports):
    validate_plan(value)
    expected = {s["id"]: s["names"] for s in value["shards"]}
    if len(reports) != len(expected) or {r.get("shard") for r in reports} != set(
        expected
    ):
        raise ValueError("Missing, duplicate or unexpected shard report")
    for report in reports:
        names = sorted(r["scenario"] for r in report["results"])
        if (
            names != expected[report["shard"]]
            or report.get("passed") is not True
            or any(r.get("passed") is not True for r in report["results"])
            or report.get("candidate_commit") != value["candidate_commit"]
            or report.get("mode") != "reference-vs-candidate"
            or report.get("benchmark_repetitions") != 0
        ):
            raise ValueError("Shard failed or differs from its exact inventory")
    return {
        "passed": True,
        "scenarios": len(value["inventory"]),
        "shards": len(reports),
    }


def unpack(archive_path):
    """Relocate only text build metadata; executable/archive bytes stay frozen."""
    with tarfile.open(archive_path) as archive:
        names = [member.name for member in archive.getmembers() if member.isfile()]
        archive.extractall(ROOT, filter="data")
    origin = json.loads((ROOT / ".local/native-export-root.json").read_text())
    paths = {origin["root"]: str(ROOT), origin["common_root"]: str(ROOT)}
    pattern = re.compile(
        "|".join(re.escape(p) for p in sorted(paths, key=len, reverse=True))
    )
    for name in names:
        if not name.endswith(("CMakeCache.txt", "compile_commands.json", ".ninja")):
            continue
        path = ROOT / name
        text = path.read_text()
        path.write_text(pattern.sub(lambda match: paths[match[0]], text))


def pack(bundle, count):
    bundle = bundle.resolve()
    runtime.export(bundle, migration)
    origin = ROOT / ".local/native-export-root.json"
    origin.write_text(
        json.dumps({"root": str(ROOT), "common_root": str(migration.COMMON_ROOT)})
        + "\n"
    )
    # Consumers compile only narrow probes. They need archive/headers/configuration,
    # never game objects, Cargo incremental state or already-run test results.
    support = [
        origin,
        ROOT / "build-rust/CMakeCache.txt",
        ROOT / "build-rust/compile_commands.json",
        ROOT / "build-rust/generated",
        migration.rust_archive(ROOT / "build-rust"),
    ]
    with tarfile.open(
        bundle.parent / "native-support.tar", "w", dereference=True
    ) as archive:
        for path in support:
            archive.add(path, arcname=str(path.relative_to(ROOT)))
    # The integer comparison executes the already-built native generators. The
    # widget comparison links real game objects; keep this larger payload out of
    # simulation shards rather than rebuilding or substituting those inputs.
    comparison = [
        (origin, str(origin.relative_to(ROOT))),
        (ROOT / "build-rust/build.ninja", "build-rust/build.ninja"),
        (
            ROOT / "build-rust/CMakeFiles/rules.ninja",
            "build-rust/CMakeFiles/rules.ninja",
        ),
    ]
    tools = ROOT / ".local/build-tools-rust"
    comparison += [
        (tools / "CMakeCache.txt", ".local/build-tools-rust/CMakeCache.txt"),
        (
            migration.rust_archive(tools),
            str(migration.rust_archive(tools).relative_to(ROOT)),
        ),
    ]
    for kind in ("strgen", "settingsgen"):
        relative = f"src/{kind}/{kind}"
        comparison += [
            (tools / relative, f".local/build-tools-rust/{relative}"),
            (
                migration.REFERENCE_BUILD / relative,
                f".local/build-reference/{relative}",
            ),
        ]
    command = subprocess.check_output(
        ["ninja", "-t", "commands", "openttd_test"], cwd=ROOT / "build-rust", text=True
    ).splitlines()[-1]
    for token in shlex.split(command):
        if not token.endswith((".o", ".a")):
            continue
        path = Path(token)
        path = path if path.is_absolute() else ROOT / "build-rust" / path
        if path.is_relative_to(ROOT):
            comparison.append((path, str(path.relative_to(ROOT))))
    with tarfile.open(
        bundle.parent / "native-comparison-support.tar", "w", dereference=True
    ) as archive:
        for name, path in dict((name, path) for path, name in comparison).items():
            if path.suffix == ".o" or path.name in ("strgen", "settingsgen"):
                with tempfile.TemporaryDirectory(dir=ROOT / ".local") as temporary:
                    exported = Path(temporary) / path.name
                    runtime.strip_debug(path, exported)
                    archive.add(exported, arcname=name)
            else:
                archive.add(path, arcname=name)
    value = plan(bundle, count)
    with tarfile.open(
        bundle.parent / "native-runtimes.tar", "w", dereference=True
    ) as archive:
        archive.add(bundle, arcname=bundle.name)
    print(json.dumps({"shards": [s["id"] for s in value["shards"]]}))


def collect_evidence(destination):
    """Retain reports, concise logs and failing saves, excluding rebuildable output."""
    roots = [
        ROOT / ".local/simulation",
        ROOT / ".local/verification",
        ROOT / ".local/comparison-logs",
        *ROOT.glob(".local/*-comparison"),
        *ROOT.glob(".local/native-*.json"),
    ]
    for base in roots:
        for path in [base] if base.is_file() else base.rglob("*"):
            if not path.is_file() or any(
                part in ("cargo", "CMakeFiles", "untracked") for part in path.parts
            ):
                continue
            if path.suffix not in (
                ".json",
                ".log",
                ".txt",
                ".patch",
                ".sav",
                ".inc",
                ".hpp",
                ".xml",
            ):
                continue
            target = destination / path.relative_to(ROOT / ".local")
            target.parent.mkdir(parents=True, exist_ok=True)
            if path.stat().st_size > 2 * 1024 * 1024 and path.suffix in (
                ".log",
                ".txt",
            ):
                # Massive math transcripts are redundant with the pass/fail/count report.
                with path.open("rb") as source:
                    source.seek(-1024 * 1024, 2)
                    target.write_bytes(
                        b"[Last 1 MiB; full transcript omitted]\n" + source.read()
                    )
            else:
                shutil.copy2(path, target)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    restore = commands.add_parser("unpack")
    restore.add_argument("archive", type=Path)
    export = commands.add_parser("pack")
    export.add_argument("--bundle", type=Path, default=ROOT / ".local/native-runtimes")
    export.add_argument("--shards", type=int, default=6)
    run = commands.add_parser("shard")
    run.add_argument("index", type=int)
    run.add_argument("--bundle", type=Path, default=ROOT / ".local/native-runtimes")
    run.add_argument("--jobs", type=int, default=2)
    run.add_argument("--output", type=Path, required=True)
    report = commands.add_parser("report")
    report.add_argument("directory", type=Path)
    report.add_argument("--bundle", type=Path, default=ROOT / ".local/native-runtimes")
    evidence = commands.add_parser("evidence")
    evidence.add_argument("destination", type=Path)
    args = parser.parse_args()
    if args.command == "unpack":
        unpack(args.archive)
    elif args.command == "pack":
        pack(args.bundle, args.shards)
    elif args.command == "shard":
        if args.jobs < 1:
            parser.error("--jobs must be positive")
        return run_shard(args.bundle, args.index, args.jobs, args.output)
    elif args.command == "report":
        value = json.loads((args.bundle / "shard-plan.json").read_text())
        reports = [
            json.loads(p.read_text())
            for p in sorted(args.directory.glob("shard-*.json"))
        ]
        print(json.dumps(aggregate(value, reports)))
    else:
        collect_evidence(args.destination)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
