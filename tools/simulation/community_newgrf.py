"""A larger arctic save played by the community AIs on the pinned NewGRF sets (#507).

The unchanged original plays fifty years on a 256x256 arctic map with the four
#545 community AIs and the three #670 OpenGFX+ sets loaded from their complete,
uncommitted packages. Reload scenarios then compare the matured save, so its
saved GRF identities, community-set vehicles and industries and the AIs' saved
state are loaded and played in both builds."""

import argparse
import hashlib
import json
import lzma
import tempfile
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import cast

from . import community_ai, core, newgrf_sets

FIXTURE = core.ROOT / "migration/saves/community-newgrf-arctic-2000.sav"
YEARS = 50
COMMAND = "python3 -m tools.simulation.community_newgrf generate"


class Scenario(community_ai.Scenario):
    community_newgrf: bool


Run = core.Run


class Result(core.Result, total=False):
    community_newgrf_input: dict[str, object]
    community_newgrf_witnesses: dict[str, dict[str, dict[str, int]]]


@dataclass(frozen=True)
class NewGRF:
    """One saved NGRF identity: what the save references instead of the set."""

    filename: str
    grfid: str
    md5sum: str


@dataclass(frozen=True)
class Provenance:
    reference_commit: str
    license: str
    scenario: Scenario
    sha256: str
    packages_sha256: str
    command: str
    end_moment: tuple[int, int, int]
    wall_seconds: int
    newgrfs: list[NewGRF]


def packages_sha256() -> str:
    """The complete manifest, AI and NewGRF rows alike, in canonical form."""
    rows = json.loads((community_ai.INPUTS / "packages.json").read_text())
    return hashlib.sha256((json.dumps(rows, indent=2) + "\n").encode()).hexdigest()


def generated() -> Scenario:
    """#545's community game with the #670 sets in the new-game configuration."""
    scenario = Scenario(
        **community_ai.generated("arctic", YEARS), community_newgrf=True
    )
    scenario["name"] = "community-newgrf-arctic"
    scenario["settings"] = {**scenario["settings"], "newgrf": newgrf_sets.GRFS}
    return scenario


def reload(name: str, ticks: int) -> Scenario:
    return Scenario(
        name=name,
        kind="save",
        community_ai="reload",
        community_newgrf=True,
        save=str(FIXTURE),
        console=["unpause"],
        ticks=ticks,
        witnesses=True,
    )


def scenarios(soak: bool) -> list[Scenario]:
    cases = [reload("community-newgrf-large-reload", 2 * core.SNAPSHOT_TICKS)]
    if soak:
        cases.append(
            reload("community-newgrf-large-reload-long", 2 * community_ai.YEAR)
        )
    return cases


def install(scenario: Scenario, run_dir: Path) -> None:
    if not scenario.get("community_newgrf"):
        return
    community_ai.install(scenario, run_dir)
    newgrf_sets.install_sets(run_dir)


def game_args(scenario: Scenario) -> list[str]:
    if not scenario.get("community_newgrf"):
        return []
    return [*community_ai.game_args(scenario), "-d", "grf=1"]


def game_environment(scenario: Scenario) -> dict[str, str]:
    return (
        core.WITNESS_ENVIRONMENT
        if scenario.get("community_newgrf") and scenario.get("witnesses")
        else {}
    )


def saved_newgrfs(path: Path) -> list[NewGRF]:
    return [
        NewGRF(
            filename=str(row["filename"]),
            grfid=f"{cast(int, row['ident.grfid']):08x}",
            md5sum=bytes(cast(list[int], row["ident.md5sum"])).hex(),
        )
        for _, row in sorted(core.rows(path, "NGRF").items())
    ]


def prepare(
    scenario: Scenario,
    binaries: dict[str, Path],
    builds: dict[str, Path],
    out: Path,
    timeout: int,
    env: dict[str, str],
    result: Result,
) -> Scenario:
    if scenario.get("community_ai") != "reload":
        return scenario
    path, record = core.normalize(
        Path(scenario["save"]), out / scenario["name"] / "input"
    )
    provenance = json.loads(FIXTURE.with_suffix(".json").read_text())
    if record["source_sha256"] != provenance["sha256"]:
        raise RuntimeError("community NewGRF save differs from its provenance")
    if packages_sha256() != provenance["packages_sha256"]:
        raise RuntimeError("community packages differ from the save's provenance")
    result["community_newgrf_input"] = record
    prepared = Scenario(**scenario)
    prepared["save"] = str(path)
    return prepared


def check(
    scenario: Scenario,
    run: Run,
    mode: core.Mode,
    role: core.Role,
    result: Result,
) -> None:
    if not scenario.get("community_newgrf"):
        return
    community_ai.check(scenario, run, mode, role, cast(dict[str, object], result))
    provenance = json.loads(FIXTURE.with_suffix(".json").read_text())
    expected = [NewGRF(**row) for row in provenance["newgrfs"]]
    for snapshot in run["snapshots"]:
        if saved_newgrfs(snapshot) != expected:
            raise RuntimeError(f"{snapshot.name} lost the save's NewGRF identities")
    for prefix, required in newgrf_sets.REQUIRED.items():
        if counts := core.branch_witnesses(scenario, run, role, prefix, required):
            result.setdefault("community_newgrf_witnesses", {}).setdefault(mode, {})[
                prefix
            ] = counts


def generate_fixture(destination: Path, timeout: int) -> None:
    """Play the game in the unchanged original only and save it compressed."""
    import migration

    migration.ensure_reference()
    scenario = generated()
    output = core.ROOT / ".local/community-newgrf-generation"
    output.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="run-", dir=output))
    with migration.reference_lock(shared=True):
        binary = core.copy_runtime(
            migration.REFERENCE_BUILD,
            migration.REFERENCE_BUILD / "openttd",
            output / "runtime",
        )
    # The generation case is not a registered scenario, so this module's hooks
    # are applied here rather than through core.hook_modules.
    plan = core.prepare_game(
        scenario,
        binary,
        binary.parent,
        output / "game",
        timeout,
        migration.environment(),
        False,
    )
    install(scenario, plan["dir"])
    plan["command"] += game_args(scenario)
    started = time.monotonic()
    run = core.read_game(core.execute_game(plan))
    wall_seconds = round(time.monotonic() - started)
    community_ai.check(scenario, run, "plain", "reference", {})
    final = run["snapshots"][-1]
    newgrfs = saved_newgrfs(final)
    if len(newgrfs) != len(newgrf_sets.GRFS):
        raise RuntimeError(f"expected {len(newgrf_sets.GRFS)} NewGRFs: {newgrfs}")
    raw = final.read_bytes()
    packed = b"OTTX" + raw[4:8] + lzma.compress(raw[8:])
    destination.write_bytes(packed)
    provenance = Provenance(
        reference_commit=migration.BASELINE["commit"],
        license="GPL-2.0-only",
        scenario=scenario,
        sha256=hashlib.sha256(packed).hexdigest(),
        packages_sha256=packages_sha256(),
        command=COMMAND,
        end_moment=core.save_moment(final),
        wall_seconds=wall_seconds,
        newgrfs=newgrfs,
    )
    destination.with_suffix(".json").write_text(
        json.dumps(asdict(provenance), indent=2) + "\n"
    )
    print(
        f"Generated {destination.name} in {wall_seconds} s: {provenance.sha256}",
        flush=True,
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("generate",))
    parser.add_argument("--timeout", type=int, default=14400)
    args = parser.parse_args()
    generate_fixture(FIXTURE, args.timeout)


if __name__ == "__main__":
    main()
