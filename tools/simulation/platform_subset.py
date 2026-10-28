"""Cadence-only native platform runs against a same-run Linux reference bundle."""

import argparse
import contextlib
import hashlib
import json
import os
import shutil
import sys
from pathlib import Path
from typing import Any, TypedDict, cast

import migration

from . import core

NAMES = (
    "play-opus-55-167-002-manual",
    "play-grok-159-001-manual",
    "play-padhattan-ridge-2000-manual",
    "generate-tgp-128-1",
    "aircraft-route-manual",
)
MODES = (("snapshots", True), ("plain", False))
Scenario = dict[str, Any]


class Capture(TypedDict):
    exit: int | str
    seconds: float
    command: list[str]
    snapshots: list[str]


class Bundle(TypedDict):
    candidate_commit: str
    baseline: str
    run_id: str
    inventory: list[str]
    captures: dict[str, Capture]
    input_hashes: dict[str, str]


def identity() -> Bundle:
    return {
        "candidate_commit": migration.git("rev-parse", "HEAD"),
        "baseline": migration.BASELINE["commit"],
        "run_id": os.environ.get("GITHUB_RUN_ID", "local"),
        "inventory": list(NAMES),
        "captures": {},
        "input_hashes": {},
    }


def stored_run(bundle: Path, key: str, record: Capture) -> dict[str, Any]:
    run = core.read_game(
        {"dir": bundle / "captures" / key, **record, "instructions": None}
    )
    if record["exit"] != 0 or [p.name for p in run["snapshots"]] != record["snapshots"]:
        raise ValueError(f"Missing or failed Linux reference capture: {key}")
    return run


def load_bundle(folder: Path) -> Bundle:
    value = cast(Bundle, json.loads((folder / "bundle.json").read_text()))
    expected = identity()
    if {k: v for k, v in value.items() if k not in ("captures", "input_hashes")} != {
        k: v for k, v in expected.items() if k not in ("captures", "input_hashes")
    }:
        raise ValueError(
            "Linux reference differs from workflow run, commit or inventory"
        )
    keys = {f"{name}/{mode}" for name in NAMES for mode, _ in MODES}
    if set(value["captures"]) != keys:
        raise ValueError("Linux reference omitted or duplicated a case/mode")
    for key, record in value["captures"].items():
        stored_run(folder, key, record)
    return value


def scenarios(output: Path) -> list[Scenario]:
    available = {s["name"]: s for s in core.scenario_list(False)}
    cases = [dict(available[name]) for name in NAMES]
    for case in cases:
        case["settings"] = {
            "gui": {"threaded_saves": "false"},
            "linkgraph": {"recalc_time": 9000},
        }
        if case["kind"] != "save":
            continue
        original = Path(case["save"])
        folder = output / "input" / case["name"]
        source, _ = core.normalize(original, folder)
        date = core.rows(source, "DATE")[0]["economy_date"]
        case["platform_end_date"] = date + (case["ticks"] + 73) // 74
        prepared = folder / "platform-no-join.sav"
        core.patch(
            source,
            prepared,
            {
                "PATS": {0: {"linkgraph.recalc_time": 9000}},
                "LGRJ": {
                    index: {"join_date": date + 4500}
                    for index in core.rows(source, "LGRJ")
                },
            },
        )
        case["save"] = str(prepared)
        if case.get("aircraft_fixture"):
            record = json.loads(original.with_suffix(".json").read_text())
            record["normalized_sha256"] = hashlib.sha256(
                prepared.read_bytes()
            ).hexdigest()
            prepared.with_suffix(".json").write_text(json.dumps(record))
    return cases


def check_no_join(case: Scenario, run: dict[str, Any]) -> None:
    for path in run["snapshots"]:
        state = core.read_save(path)
        date = core.rows(state, "DATE")[0]["economy_date"]
        end = case.get("platform_end_date", date + (case["ticks"] + 73) // 74)
        if date > end or core.rows(state, "PATS")[0]["linkgraph.recalc_time"] != 9000:
            raise ValueError("Platform fixture exceeded its fixed no-join interval")
        jobs = core.rows(state, "LGRJ")
        if any(job["join_date"] <= end for job in jobs.values()):
            raise ValueError(
                "Platform fixture has a link-graph join due during its run"
            )
        if case["kind"] == "generate" and jobs:
            raise ValueError("Unserved generated map unexpectedly has link-graph jobs")


def main(arguments: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--reference-binary", type=Path)
    source.add_argument("--stored-reference", type=Path)
    parser.add_argument("--candidate", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout", type=int, default=1200)
    args = parser.parse_args(arguments)
    if bool(args.candidate) != bool(args.stored_reference):
        parser.error("--stored-reference requires --candidate and vice versa")
    output = args.output.resolve()
    output.mkdir(parents=True)
    bundle = args.stored_reference.resolve() if args.stored_reference else output
    metadata = load_bundle(bundle) if args.stored_reference else identity()
    original = args.reference_binary is not None
    binary = (args.reference_binary if original else args.candidate).resolve()
    runtime = output / "runtime"
    with (
        migration.reference_lock(shared=True) if original else contextlib.nullcontext()
    ):
        if original:
            migration.ensure_reference()
        frozen = core.copy_runtime(
            binary.parent, binary, runtime, candidate=not original
        )
    if original:
        shutil.copytree(runtime / "baseset/opengfx", bundle / "opengfx")
    else:
        shutil.copytree(
            bundle / "opengfx", runtime / "baseset/opengfx", dirs_exist_ok=True
        )
    results = []
    cases = scenarios(output)
    hashes = {
        case["name"]: hashlib.sha256(Path(case["save"]).read_bytes()).hexdigest()
        for case in cases
        if case["kind"] == "save"
    }
    if not original and metadata["input_hashes"] != hashes:
        raise ValueError("Prepared platform inputs differ from the Linux reference")
    metadata["input_hashes"] = hashes
    for case in cases:
        name = case["name"]
        if original:
            for mode, desync in MODES:
                run = core.run_game(
                    case,
                    frozen,
                    runtime,
                    bundle / "captures" / name / mode,
                    args.timeout,
                    migration.environment(),
                    desync,
                )
                check_no_join(case, run)
                metadata["captures"][f"{name}/{mode}"] = {
                    "exit": run["exit"],
                    "seconds": run["seconds"],
                    "command": run["command"],
                    "snapshots": [p.name for p in run["snapshots"]],
                }

        def runner(
            scenario: Scenario,
            executable: Path,
            build: Path,
            run_dir: Path,
            timeout: int,
            env: dict[str, str],
            desync: bool,
            *,
            isolate: bool,
        ) -> dict[str, Any]:
            key = f"{scenario['name']}/{'snapshots' if desync else 'plain'}"
            if run_dir.name == "reference":
                return stored_run(bundle, key, metadata["captures"][key])
            run = core.run_game(
                scenario,
                executable,
                build,
                run_dir,
                timeout,
                env,
                desync,
                native_threads=True,
            )
            check_no_join(scenario, run)
            return run

        result = core.run_scenario(
            case,
            {
                "reference": frozen if original else bundle / "oracle",
                "candidate": frozen,
            },
            {"reference": runtime, "candidate": runtime},
            output / "validation",
            20,
            args.timeout,
            migration.environment(),
            game_runner=runner,
            prepared=True,
        )
        result["scenario"] = "platform-no-join-" + name
        result.pop("plain_speed")
        results.append(result)
        print(f"{name}: {'passed' if result['passed'] else 'FAILED'}", flush=True)
    passed = all(result["passed"] for result in results)
    (output / "report.json").write_text(
        json.dumps(
            {
                "execution": "prepared no-join cases; native graph threads, synchronous saves; Linux reference uses RLIMIT_NPROC fallback",
                "results": results,
                "passed": passed,
            },
            indent=2,
        )
        + "\n"
    )
    if original and passed:
        (bundle / "bundle.json").write_text(json.dumps(metadata, indent=2) + "\n")
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
