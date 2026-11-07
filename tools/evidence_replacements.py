"""Executable replacement registration for retiring native evidence (#736).

Each migration/evidence-replacements/*.json is a list of per-file records:
{"retires": ["source:src/tests/example.cpp", "ctest:original test name"],
 "oracle": "Pinned original inputs, trace provenance and retained observations",
 "observations": {"named behavior": ["rust:lib:module::tests::case",
                                       "scenario:default-scenario-name"]}}

Use rust:<lib or auto-discovered integration target>:<exact test name>.
Every observation needs a destination. Review establishes that these names carry
all original observations; registration establishes that they actually execute.
No manifest is needed until a component transfers evidence.
"""

import argparse
import json
import os
import re
import subprocess
import sys
from collections.abc import Sequence
from pathlib import Path
from typing import TypedDict

ROOT = Path(__file__).resolve().parents[1]
MANIFESTS = "migration/evidence-replacements/*.json"
NATIVE_SUFFIXES = {".cpp", ".cc", ".cxx", ".h", ".hpp", ".inc"}


class Replacement(TypedDict):
    retires: list[str]
    oracle: str
    observations: dict[str, list[str]]


def load(root: Path = ROOT) -> list[Replacement]:
    records: list[Replacement] = []
    for path in sorted(root.glob(MANIFESTS)):
        value = json.loads(path.read_text())
        if not isinstance(value, list) or not value:
            raise ValueError(f"{path.name}: expected nonempty replacement records")
        for record in value:
            if (
                not isinstance(record, dict)
                or set(record) != {"retires", "oracle", "observations"}
                or not isinstance(record["retires"], list)
                or not record["retires"]
                or not all(isinstance(item, str) for item in record["retires"])
                or not isinstance(record["oracle"], str)
                or not record["oracle"].strip()
                or not isinstance(record["observations"], dict)
                or not record["observations"]
            ):
                raise ValueError(f"{path.name}: incomplete replacement contract")
            for name, checks in record["observations"].items():
                if (
                    not name.strip()
                    or not isinstance(checks, list)
                    or not checks
                    or not all(isinstance(check, str) for check in checks)
                ):
                    raise ValueError(f"{path.name}: empty observation {name!r}")
            records.append(record)
    return records


def destinations(records: Sequence[Replacement]) -> set[str]:
    return {
        check
        for record in records
        for checks in record["observations"].values()
        for check in checks
    }


def rust_selector(check: str, root: Path = ROOT) -> list[str]:
    parts = check.split(":", 2)
    if (
        len(parts) != 3
        or parts[0] != "rust"
        or not re.fullmatch(r"[A-Za-z_]\w*(?:::[A-Za-z_]\w*)*", parts[2])
    ):
        raise ValueError(f"invalid Rust replacement: {check}")
    target, name = parts[1:]
    if target == "lib":
        return ["--lib", name]
    if not re.fullmatch(r"[\w-]+", target) or not any(
        path.is_file()
        for path in (
            root / f"rust/openttd-kernels/tests/{target}.rs",
            root / f"rust/openttd-kernels/tests/{target}/main.rs",
        )
    ):
        raise ValueError(f"missing auto-discovered Rust test target: {target}")
    return ["--test", target, name]


def errors(
    records: Sequence[Replacement], default: Sequence[str] | None, root: Path = ROOT
) -> list[str]:
    found = []
    owners: set[str] = set()
    for record in records:
        for retired in record["retires"]:
            if not re.fullmatch(
                r"(?:source|target|ctest|comparison|scenario):.+", retired
            ):
                found.append(f"invalid retired evidence identity: {retired}")
            if retired in owners:
                found.append(f"duplicate replacement owner: {retired}")
            owners.add(retired)
    for check in sorted(destinations(records)):
        if check.startswith("scenario:"):
            scenario = check.removeprefix("scenario:")
            if not scenario.strip() or (
                default is not None and scenario not in default
            ):
                found.append(f"replacement is not a default harness scenario: {check}")
        else:
            try:
                rust_selector(check, root)
            except ValueError as error:
                found.append(str(error))
    # Reference CTest verification runs before simulation: its replacement must
    # be a registered Rust CTest, not a promise to run a scenario later.
    for record in records:
        if any(item.startswith("ctest:") for item in record["retires"]) and any(
            not any(check.startswith("rust:") for check in checks)
            for checks in record["observations"].values()
        ):
            found.append("reference CTest observations require Rust CTest replacements")
    return found


def missing(retired: Sequence[str], records: Sequence[Replacement]) -> list[str]:
    registered = {item for record in records for item in record["retires"]}
    return [
        f"missing replacement contract: {item}"
        for item in retired
        if item not in registered
    ]


def ctest_name(check: str) -> str:
    return f"evidence-{check}"


def ctest_errors(
    reference: set[str], candidate: set[str], records: Sequence[Replacement]
) -> list[str]:
    """Accept a missing reference name only when all its Rust checks ran in CTest."""
    found = []
    for name in sorted(reference - candidate):
        matches = [record for record in records if f"ctest:{name}" in record["retires"]]
        if (
            not matches
            or any(
                not any(check.startswith("rust:") for check in checks)
                for record in matches
                for checks in record["observations"].values()
            )
            or any(
                ctest_name(check) not in candidate
                for record in matches
                for check in destinations([record])
                if check.startswith("rust:")
            )
        ):
            found.append(
                f"Candidate removed reference test without executable replacement: {name}"
            )
    return found


def run_rust(
    check: str, root: Path = ROOT, cargo: str = "cargo", target_dir: Path | None = None
) -> None:
    """Cargo succeeds with zero matches; require an exact, nonempty test run."""
    env = os.environ.copy()
    env.setdefault("CARGO_TARGET_DIR", str(target_dir or root / "build-rust/cargo"))
    result = subprocess.run(
        [
            cargo,
            "test",
            "--manifest-path",
            str(root / "Cargo.toml"),
            "--locked",
            "-p",
            "openttd-kernels",
            *rust_selector(check, root),
            "--",
            "--exact",
        ],
        cwd=root,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    print(result.stdout, end="", flush=True)
    if result.returncode or not re.search(
        r"test result: ok\. [1-9]\d* passed; 0 failed; 0 ignored;", result.stdout
    ):
        raise RuntimeError(f"Rust replacement failed or ran no tests: {check}")


def cmake(records: Sequence[Replacement]) -> str:
    lines = ["# Generated by tools/evidence_replacements.py; do not edit."]

    # Bracket quoting preserves CTest names without interpreting CMake syntax.
    def quote(value: str) -> str:
        if "]=]" in value:
            raise ValueError(f"unsupported CMake delimiter in {value!r}")
        return f"[=[{value}]=]"

    sources = sorted(
        {
            item.removeprefix("source:")
            for record in records
            for item in record["retires"]
            if item.startswith("source:")
        }
    )
    lines.append("set(EVIDENCE_RETIRED_SOURCES " + " ".join(map(quote, sources)) + ")")
    for check in sorted(destinations(records)):
        if check.startswith("rust:"):
            lines.append(
                f"add_test(NAME {quote(ctest_name(check))} COMMAND "
                '"${Python3_EXECUTABLE}" "${CMAKE_SOURCE_DIR}/tools/evidence_replacements.py" '
                '--cargo "${CARGO_EXECUTABLE}" --target-dir "${CMAKE_BINARY_DIR}/cargo" '
                f"--run-rust {quote(check)})"
            )
            lines.append(
                f"set_tests_properties({quote(ctest_name(check))} PROPERTIES RUN_SERIAL TRUE)"
            )
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    choice = parser.add_mutually_exclusive_group(required=True)
    choice.add_argument("--cmake", type=Path)
    choice.add_argument("--run-rust")
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--target-dir", type=Path)
    args = parser.parse_args()
    if args.run_rust:
        run_rust(args.run_rust, cargo=args.cargo, target_dir=args.target_dir)
    else:
        records = load()
        # Quick CI validates default scenario membership against the live harness.
        # CMake's Emscripten host uses Python 3.10, so configuration must not import
        # the Python 3.11 simulation runtime.
        found = errors(records, None)
        if found:
            sys.exit("\n".join(found))
        args.cmake.write_text(cmake(records))


if __name__ == "__main__":
    main()
