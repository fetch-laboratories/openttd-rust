"""Measure the default corpus on the pinned original; this is guidance, not evidence."""

import argparse
import concurrent.futures
import importlib.util
import json
import re
import shutil
import sys
from collections.abc import Iterable
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, NotRequired, TypedDict

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from validation_execution import run_logged  # noqa: E402

import migration  # noqa: E402
from simulation import core  # noqa: E402

GCOVR = [
    "uvx",
    "--from",
    "gcovr==8.4",
    "gcovr",
    "--no-markers",
    "--no-exclude-noncode-lines",
    "--include-internal-functions",
]


class Branch(TypedDict):
    count: int


class Line(TypedDict):
    line_number: int
    count: int
    branches: list[Branch]
    function_name: NotRequired[str]


class Function(TypedDict):
    name: str
    demangled_name: str
    lineno: int
    execution_count: int


class Source(TypedDict):
    file: str
    lines: list[Line]
    functions: list[Function]


class Trace(TypedDict):
    files: list[Source]


class Counts(TypedDict):
    lines: int
    lines_covered: int
    branches: int
    branches_covered: int


class FunctionSummary(Function, Counts):
    pass


class FileSummary(Counts):
    file: str
    functions: list[FunctionSummary]


class Report(TypedDict):
    totals: Counts
    files: list[FileSummary]
    unique_lines: dict[str, list[tuple[str, int]]]
    baseline: NotRequired[dict[str, Any]]
    scenarios: NotRequired[list[str]]
    uninstrumented_sources: NotRequired[list[str]]


def counts(lines: list[Line]) -> Counts:
    return Counts(
        lines=len({line["line_number"] for line in lines}),
        lines_covered=len({line["line_number"] for line in lines if line["count"]}),
        branches=sum(len(line["branches"]) for line in lines),
        branches_covered=sum(
            bool(b["count"]) for line in lines for b in line["branches"]
        ),
    )


def reached(trace: Trace) -> set[tuple[str, int]]:
    return {
        (source["file"], line["line_number"])
        for source in trace["files"]
        for line in source["lines"]
        if line["count"]
    }


def summarize(merged: Trace, scenarios: Iterable[tuple[str, Trace]]) -> Report:
    """Keep physical lines unique across template instances; retain compiler branches."""
    files: list[FileSummary] = []
    for source in merged["files"]:
        functions = [
            FunctionSummary(
                **function,
                **counts(
                    [
                        line
                        for line in source["lines"]
                        if line.get("function_name") == function["name"]
                    ]
                ),
            )
            for function in source["functions"]
        ]
        files.append(
            FileSummary(
                file=source["file"], **counts(source["lines"]), functions=functions
            )
        )
    owners: dict[tuple[str, int], str | None] = {}
    unique: dict[str, list[tuple[str, int]]] = {}
    for name, trace in scenarios:
        unique[name] = []
        for point in reached(trace):
            owners[point] = None if point in owners else name
    for point, owner in sorted(owners.items()):
        if owner is not None:
            unique[owner].append(point)
    return {
        "totals": Counts(
            lines=sum(file["lines"] for file in files),
            lines_covered=sum(file["lines_covered"] for file in files),
            branches=sum(file["branches"] for file in files),
            branches_covered=sum(file["branches_covered"] for file in files),
        ),
        "files": files,
        "unique_lines": unique,
    }


def fraction(covered: int, total: int) -> str:
    return (
        f"{covered} of {total} ({100 * covered / total:.2f}%)"
        if total
        else "0 of 0 (no emitted code)"
    )


def markdown(report: Report) -> str:
    totals = report["totals"]
    text = [
        "# Original simulation coverage",
        "",
        "Guidance only. Linux GCC, -O0, all compiler branches (including exception edges).",
        f"Lines: {fraction(totals['lines_covered'], totals['lines'])}. Branches: {fraction(totals['branches_covered'], totals['branches'])}.",
        "",
        "## Twenty least-covered files",
        "",
        "| Original file | Lines | Branches |",
        "| --- | --- | --- |",
    ]
    files = sorted(
        (file for file in report["files"] if file["lines"]),
        key=lambda f: (f["lines_covered"] / f["lines"], f["file"]),
    )
    text += [
        f"| `{f['file']}` | {fraction(f['lines_covered'], f['lines'])} | {fraction(f['branches_covered'], f['branches'])} |"
        for f in files[:20]
    ]
    zero = [name for name, lines in report["unique_lines"].items() if not lines]
    text += [
        "",
        f"## Scenarios with no unique original line ({len(zero)} of {len(report['unique_lines'])})",
        "",
        *[f"- `{name}`" for name in zero],
        "",
    ]
    return "\n".join(text)


def checked(command: list[str], log: Path, env: dict[str, str]) -> None:
    if run_logged(command, cwd=core.ROOT, env=env, log=log, phase=log.stem):
        raise RuntimeError(f"Coverage command failed; see {log}")


def collect(
    prefix: Path, output: Path, build: Path, filters: list[str], env: dict[str, str]
) -> None:
    """Merge process trees before gcovr, retaining the absolute object-path layout."""
    processes = sorted(prefix.iterdir())
    if not processes or any(not list(process.rglob("*.gcda")) for process in processes):
        raise RuntimeError(f"Missing process coverage in {prefix}")
    merged = prefix / "merged"
    shutil.move(processes[0], merged)
    for process in processes[1:]:
        temporary = prefix / "merge-next"
        checked(
            ["gcov-tool", "merge", str(merged), str(process), "-o", str(temporary)],
            prefix / "merge.log",
            env,
        )
        shutil.rmtree(merged)
        shutil.rmtree(process)
        temporary.rename(merged)
    for data in merged.rglob("*.gcda"):
        original = Path("/") / data.relative_to(merged)
        notes = original.with_suffix(".gcno")
        if not notes.is_file():
            raise RuntimeError(f"Missing instrumentation notes: {notes}")
        data.with_suffix(".gcno").symlink_to(notes)
    checked(
        [
            *GCOVR,
            "--root",
            str(migration.REFERENCE),
            "--object-directory",
            str(build),
            *filters,
            "--json",
            str(output),
            str(merged),
        ],
        output.with_suffix(".log"),
        env,
    )
    shutil.rmtree(prefix)


def measure(
    scenario: dict[str, Any],
    binary: Path,
    runtime: Path,
    out: Path,
    build: Path,
    filters: list[str],
    timeout: int,
) -> str:
    name = scenario["name"]
    folder = out / "runs" / name
    folder.mkdir(parents=True)
    prefix = out / "profiles" / name
    prefix.mkdir(parents=True)
    env = dict(migration.environment(), OPENTTD_COVERAGE_PREFIX=str(prefix))
    binaries = dict(reference=binary, candidate=binary)
    builds = dict(reference=runtime, candidate=runtime)
    last: dict[str, Any] = {}

    def reference_game(
        s: dict[str, Any],
        executable: Path,
        data: Path,
        run_dir: Path,
        seconds: int,
        base_env: dict[str, str],
        desync: bool,
        *,
        isolate: bool = False,
    ) -> dict[str, Any]:
        # Self-mode's candidate checks inspect the same original run. Preparation
        # hooks still run their original reference-only captures in this workspace.
        nonlocal last
        if run_dir.name == "reference":
            last = core.run_game(
                s, executable, data, run_dir, seconds, base_env, desync, isolate=isolate
            )
        return last

    result = core.run_scenario(
        scenario, binaries, builds, folder, 20, timeout, env, game_runner=reference_game
    )
    (folder / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(f"coverage {name}: {'ok' if result['passed'] else 'FAIL'}", flush=True)
    if not result["passed"]:
        raise RuntimeError(
            f"Reference scenario failed: {name}; see {folder / 'result.json'}"
        )
    output = out / "traces" / f"{name}.json"
    collect(prefix, output, build, filters, env)
    shutil.rmtree(folder)
    return name


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build", type=Path, required=True)
    parser.add_argument("--jobs", type=int, default=1)
    parser.add_argument("--timeout", type=int, default=600)
    args = parser.parse_args()
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S.%fZ")
    out = migration.LOCAL / "coverage" / stamp
    out.mkdir(parents=True)
    (out / "traces").mkdir()
    spec = importlib.util.spec_from_file_location(
        "port_metrics", core.ROOT / "tools/port-metrics.py"
    )
    assert spec is not None and spec.loader is not None
    metrics = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(metrics)
    inventory = [
        path
        for path in migration.git(
            "ls-files", "src", cwd=migration.REFERENCE
        ).splitlines()
        if metrics.simulation_source(path)
    ]
    filters = [
        "--filter",
        "(?:"
        + "|".join(re.escape(str(migration.REFERENCE / path)) for path in inventory)
        + ")$",
    ]
    env = migration.environment()
    runtime = out / "reference-runtime"
    with migration.candidate_lock(args.build, shared=True):
        migration.ensure_reference()
        binary = core.copy_runtime(args.build, args.build / "openttd", runtime)
        for module in core.adapter_modules():
            kind = module.__name__.rsplit(".", 1)[1]
            module.adapter(args.build, runtime / kind, env)
        for data in args.build.rglob("*.gcda"):
            data.unlink()
        checked(
            [
                *GCOVR,
                "--root",
                str(migration.REFERENCE),
                "--object-directory",
                str(args.build),
                *filters,
                "--json",
                str(out / "traces/zero.json"),
                str(args.build),
                str(runtime),
            ],
            out / "zero.log",
            env,
        )
        scenarios = core.scenario_list(False)
        for module in core.scenario_modules():
            if hasattr(module, "AI_FOLDER"):
                frozen = out / "scenario-ai" / module.AI_FOLDER
                shutil.copytree(core.ROOT / "tools" / module.AI_FOLDER, frozen)
                for scenario in scenarios:
                    if module.uses_ai(scenario):
                        scenario["scenario_ai"] = str(frozen)
        with concurrent.futures.ThreadPoolExecutor(args.jobs) as pool:
            futures = [
                pool.submit(
                    measure, s, binary, runtime, out, args.build, filters, args.timeout
                )
                for s in scenarios
            ]
            names = sorted(
                future.result() for future in concurrent.futures.as_completed(futures)
            )
        checked(
            [
                *GCOVR,
                "--root",
                str(migration.REFERENCE),
                "--json-add-tracefile",
                str(out / "traces/*.json"),
                "--json",
                str(out / "merged.json"),
            ],
            out / "merge.log",
            env,
        )
        report = summarize(
            json.loads((out / "merged.json").read_text()),
            (
                (name, json.loads((out / "traces" / f"{name}.json").read_text()))
                for name in names
            ),
        )
        report.update(
            baseline=migration.BASELINE,
            scenarios=names,
            uninstrumented_sources=sorted(
                set(inventory) - {f["file"] for f in report["files"]}
            ),
        )
        (out / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        (out / "summary.md").write_text(markdown(report))
        migration.ensure_reference()
    print(f"Coverage report: {out / 'summary.md'}", flush=True)


if __name__ == "__main__":
    main()
