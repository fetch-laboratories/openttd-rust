"""Pinned community scripts and a vanilla long-game save for #507."""

import argparse
import gzip
import hashlib
import json
import lzma
import re
import subprocess
import tempfile
from collections import Counter
from collections.abc import Mapping
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, NotRequired, TypedDict

from . import core

INPUTS = core.ROOT / "migration/community-ai"
FIXTURE = core.ROOT / "migration/saves/community-arctic-1970.sav"
YEAR = 366 * core.TICKS_PER_DAY
SCRIPTS = ("SimpleAI", "AdmiralAI", "trAIns")


@dataclass(frozen=True)
class Package:
    name: str
    version: str
    kind: str
    uid: str
    md5: str
    sha256: str
    license: str
    metadata: str
    publisher: str
    dependencies: list[str]

    @property
    def filename(self) -> str:
        return f"{self.uid}-{self.name}-{self.version}.tar.gz"

    @property
    def url(self) -> str:
        return f"https://bananas-cdn.openttd.org/{self.kind}/{self.uid}/{self.md5}/{self.filename}"


class Scenario(TypedDict):
    name: str
    kind: str
    community_ai: str
    ticks: int
    console: list[str]
    save: NotRequired[str]
    seed: NotRequired[int]
    map_log2: NotRequired[int]
    land_generator: NotRequired[int]
    settings: NotRequired[dict[str, dict[str, int | str]]]


class Run(TypedDict):
    dir: Path
    exit: int | str
    snapshots: list[Path]
    log: list[str]


@dataclass
class Activity:
    vehicles: dict[str, int] = field(default_factory=dict)
    delivered: dict[int, int] = field(default_factory=dict)
    income: dict[int, int] = field(default_factory=dict)
    loans: dict[int, list[int]] = field(default_factory=dict)
    bankruptcy_months: dict[int, int] = field(default_factory=dict)
    commands: dict[str, int] = field(default_factory=dict)
    moved: list[int] = field(default_factory=list)


def packages() -> list[Package]:
    return [
        Package(**row) for row in json.loads((INPUTS / "packages.json").read_text())
    ]


def fetch() -> dict[str, Path]:
    """Keep complete source archives; reject corrupted cache entries before use."""
    cache = core.ROOT / ".local/community-ai"
    cache.mkdir(parents=True, exist_ok=True)
    paths = {}
    with core.file_lock(cache / "download.lock", label="community AI inputs"):
        for package in packages():
            path = cache / package.filename
            if not path.exists():
                with tempfile.TemporaryDirectory(dir=cache) as temporary:
                    download = Path(temporary) / package.filename
                    subprocess.run(
                        [
                            "curl",
                            "--fail",
                            "--location",
                            "--silent",
                            "--show-error",
                            "--max-time",
                            "120",
                            package.url,
                            "--output",
                            str(download),
                        ],
                        check=True,
                    )
                    if (
                        hashlib.sha256(download.read_bytes()).hexdigest()
                        != package.sha256
                    ):
                        raise RuntimeError(
                            f"download hash mismatch: {package.filename}"
                        )
                    download.replace(path)
            if hashlib.sha256(path.read_bytes()).hexdigest() != package.sha256:
                raise RuntimeError(f"cached input hash mismatch: {package.filename}")
            paths[package.filename] = path
    return paths


def generated(climate: str, years: int) -> Scenario:
    return Scenario(
        name=f"community-ai-{climate}",
        kind="generate",
        community_ai=climate,
        seed=507,
        map_log2=8,
        land_generator=1,
        ticks=years * YEAR,
        console=[*(f"start_ai {name}" for name in SCRIPTS), "unpause"],
        settings={
            "game_creation": {"landscape": climate, "starting_year": 1950},
            "difficulty": {"terrain_type": 1, "quantity_sea_lakes": 0},
            "pf": {"forbid_90_deg": "false"},
            "construction": {"build_on_slopes": "true"},
        },
    )


def scenarios(soak: bool) -> list[Scenario]:
    cases = [
        generated(climate, 10 if soak else 2)
        for climate in (("temperate", "arctic") if soak else ("temperate",))
    ]
    cases.append(
        Scenario(
            name="community-ai-large-reload",
            kind="save",
            community_ai="reload",
            save=str(FIXTURE),
            console=["unpause"],
            ticks=2 * YEAR if soak else 2 * core.SNAPSHOT_TICKS,
        )
    )
    return cases


def install(scenario: Mapping[str, Any], run_dir: Path) -> None:
    if "community_ai" not in scenario:
        return
    paths = fetch()
    for package in packages():
        destination = (
            run_dir / "ai" / ("library" if package.kind == "ai-library" else "")
        )
        destination.mkdir(parents=True, exist_ok=True)
        (destination / package.filename.removesuffix(".gz")).write_bytes(
            gzip.decompress(paths[package.filename].read_bytes())
        )


def game_args(scenario: Mapping[str, Any]) -> list[str]:
    return ["-d", "script=4,desync=1"] if "community_ai" in scenario else []


def prepare(
    scenario: Mapping[str, Any],
    binaries: dict[str, Path],
    builds: dict[str, Path],
    out: Path,
    timeout: int,
    env: dict[str, str],
    result: dict[str, Any],
) -> Mapping[str, Any]:
    if scenario.get("community_ai") != "reload":
        return scenario
    path, record = core.normalize(
        Path(scenario["save"]), out / scenario["name"] / "input"
    )
    provenance = json.loads(FIXTURE.with_suffix(".json").read_text())
    if record["source_sha256"] != provenance["sha256"]:
        raise RuntimeError("community save differs from its provenance")
    result["community_input"] = record
    return dict(scenario, save=str(path))


def activity(run: Run, source: Path | None = None) -> Activity:
    observed = Activity()
    poses: dict[int, set[tuple[int, int]]] = {}
    snapshots = ([source] if source else []) + run["snapshots"]
    for path in snapshots:
        companies = core.rows(path, "PLYR")
        for cid in range(1, 4):
            company = companies.get(cid)
            if company is None or not company["is_ai"]:
                raise RuntimeError(f"community AI {cid} missing from {path.name}")
            reader = core.Reader(dict(core.read_save(path)["AIPL"]["elements"])[cid])
            if reader.take(reader.gamma()).decode() != SCRIPTS[cid - 1]:
                raise RuntimeError(f"community AI {cid} was replaced")
            observed.loans.setdefault(cid, []).append(company["current_loan"])
            observed.bankruptcy_months[cid] = max(
                observed.bankruptcy_months.get(cid, 0), company["months_of_bankruptcy"]
            )
            observed.income[cid] = max(
                observed.income.get(cid, 0),
                *(v for k, v in company.items() if k.endswith("/income")),
            )
            observed.delivered[cid] = max(
                observed.delivered.get(cid, 0),
                *(sum(v) for k, v in company.items() if k.endswith("/delivered_cargo")),
            )
        counts: Counter[str] = Counter()
        for vid, vehicle in core.rows(path, "VEHS").items():
            if vehicle["type"] not in range(4):
                continue
            kind = ("train", "roadveh", "ship", "aircraft")[vehicle["type"]]
            prefix = f"{kind}[0]/common[0]/"
            owner = vehicle[prefix + "owner"]
            if owner in range(1, 4):
                counts[f"{owner}:{kind}"] += 1
                poses.setdefault(vid, set()).add(
                    (vehicle[prefix + "x_pos"], vehicle[prefix + "y_pos"])
                )
        observed.vehicles = dict(counts)
    observed.moved = sorted(
        vid for vid, positions in poses.items() if len(positions) > 1
    )
    observed.loans = {
        cid: sorted(set(values)) for cid, values in observed.loans.items()
    }
    return observed


def check(
    scenario: Mapping[str, Any],
    run: Run,
    mode: str,
    role: str,
    result: dict[str, Any],
) -> None:
    if "community_ai" not in scenario:
        return
    if run["exit"] or not run["snapshots"]:
        raise RuntimeError("community game did not finish")
    if any("died unexpectedly" in line or "[S]" in line for line in run["log"]):
        raise RuntimeError("community script failed")
    commands = (run["dir"] / "save/autosave/commands-out.log").read_text()
    commands = re.sub(r"(?m)^\[[^\]]*\] ", "", commands).replace(
        str(run["dir"]), "<run>"
    )
    key = f"{mode}_community_commands"
    digest = hashlib.sha256(commands.encode()).hexdigest()
    if role == "reference":
        result[key] = digest
    elif result[key] != digest:
        raise RuntimeError("complete community command/debug logs differ")
    observed = activity(
        run, Path(scenario["save"]) if scenario["kind"] == "save" else None
    )
    executed = re.findall(r"^cmd: .*?; ([0-9a-f]{2}); .*\((Cmd\w+)\)$", commands, re.M)
    observed.commands = dict(Counter(f"{cid}:{name}" for cid, name in executed))
    for cid in range(1, 4):
        if observed.income[cid] <= 0 or observed.delivered[cid] <= 0:
            raise RuntimeError(
                f"community AI {cid} has no transport revenue/deliveries"
            )
        if scenario["kind"] == "generate" and not any(
            owner == f"{cid:02x}" and name.startswith("CmdBuild")
            for owner, name in executed
        ):
            raise RuntimeError(f"community AI {cid} did not construct anything")
    if (mode == "snapshots" or scenario["kind"] == "save") and not observed.moved:
        raise RuntimeError("community vehicles did not move")
    result[f"{mode}_{role}_community"] = asdict(observed)


@dataclass(frozen=True)
class Provenance:
    reference_commit: str
    license: str
    scenario: Scenario
    sha256: str
    packages_sha256: str
    command: str
    end_moment: tuple[int, int, int]


def generate_fixture(destination: Path, timeout: int) -> None:
    import migration

    migration.ensure_reference()
    scenario = generated("arctic", 20)
    output = core.ROOT / ".local/community-ai-generation"
    output.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="run-", dir=output))
    with migration.reference_lock(shared=True):
        binary = core.copy_runtime(
            migration.REFERENCE_BUILD,
            migration.REFERENCE_BUILD / "openttd",
            output / "runtime",
        )
    run = core.run_game(
        scenario,
        binary,
        binary.parent,
        output / "game",
        timeout,
        migration.environment(),
        False,
    )
    check(scenario, run, "plain", "reference", {})
    raw = run["snapshots"][-1].read_bytes()
    packed = b"OTTX" + raw[4:8] + lzma.compress(raw[8:])
    destination.write_bytes(packed)
    provenance = Provenance(
        reference_commit=migration.BASELINE["commit"],
        license="GPL-2.0-only",
        scenario=scenario,
        sha256=hashlib.sha256(packed).hexdigest(),
        packages_sha256=hashlib.sha256(
            (INPUTS / "packages.json").read_bytes()
        ).hexdigest(),
        command="python3 -m tools.simulation.community_ai generate",
        end_moment=core.save_moment(run["snapshots"][-1]),
    )
    destination.with_suffix(".json").write_text(
        json.dumps(asdict(provenance), indent=2) + "\n"
    )
    print(f"Generated {destination.name}: {provenance.sha256}", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("fetch", "generate"))
    parser.add_argument("--timeout", type=int, default=7200)
    args = parser.parse_args()
    if args.action == "fetch":
        print(f"Validated {len(fetch())} community source packages")
    else:
        generate_fixture(FIXTURE, args.timeout)


if __name__ == "__main__":
    main()
