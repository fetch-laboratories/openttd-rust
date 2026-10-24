"""Quiet speed pass: frozen runtimes, interleaved wall pairs and new anchors (AGENTS.md).

`prepare` freezes both runtimes and proves their instruction identities against
an optional `simulate --instructions` report; it runs no games. `collect` holds
other builds, links, profiles and games stopped only while its 144 timed games
run, then prints the median of four interleaved three-pair medians per speed
scenario as `SPEED_PASS` rows for instructions.py.
"""

import argparse
import json
import os
import shutil
import signal
import statistics
import sys
import threading
import time
from pathlib import Path

from . import core, instructions

GROUPS, PAIRS = 4, 3
ROLES = ("reference", "candidate")
NAMES = {"reference": "openttd", "candidate": "openttd-rust"}
# Builds, links and profiles by /proc comm (at most 15 characters).
HEAVY = set(
    "as cargo cc1 cc1plus ccache clang clang++ cmake collect2 g++ gcc gmake ld "
    "ld.bfd ld.gold ld.lld lld lto1 make mold ninja rustc rustfmt valgrind".split()
)


def speed_scenarios():
    return [
        s for s in core.scenario_list(False) if s["name"] in instructions.SPEED_BUDGETS
    ]


def prepare(out, profile, migration):
    """Freeze runtimes and record identities; preparation hooks may run no game."""
    head = migration.git("rev-parse", "HEAD")
    if migration.git("status", "--short"):
        sys.exit("speed prepare requires a clean checkout: anchors name its head")
    bundle, env = out / "frozen", migration.environment()
    bundle.mkdir(parents=True)
    with migration.reference_lock(shared=True):
        reference = migration.REFERENCE_BUILD.resolve()
        core.copy_runtime(
            reference, reference / "openttd", bundle / "reference-runtime"
        )
    candidate = core.ROOT / "build-rust"
    core.copy_runtime(
        candidate,
        candidate / "openttd-rust",
        bundle / "candidate-runtime",
        candidate=True,
    )
    builds = {role: bundle / f"{role}-runtime" for role in ROLES}
    binaries = {role: builds[role] / NAMES[role] for role in ROLES}
    if not all((builds[role] / "instruction-build.json").is_file() for role in ROLES):
        sys.exit("first run: python3 tools/migration.py simulate --instructions --list")
    for role in ROLES:
        # --frozen-runtimes copies every adapter folder; speed scenarios use none.
        for module in core.adapter_modules():
            (builds[role] / module.__name__.rsplit(".", 1)[1]).mkdir()
    records = {}
    if profile:
        report = json.loads(profile.read_text())
        if not report["passed"] or report["candidate_commit"] != head:
            sys.exit(f"{profile} is not a passing profile of {head}")
        records = {r["scenario"]: r["instruction_speed"] for r in report["results"]}
    manifest = dict(
        head=head,
        ld_library_path=env.get("LD_LIBRARY_PATH", ""),
        profile=str(profile) if profile else None,
        executables={r: instructions.digest(binaries[r].read_bytes()) for r in ROLES},
        identities={},
    )

    def no_game(*args, **kwargs):
        raise RuntimeError("speed preparation runs no game")

    games = [m for m in (core, *core.scenario_modules()) if hasattr(m, "run_game")]
    saved = [m.run_game for m in games]
    for module in games:
        module.run_game = no_game
    try:
        for scenario in speed_scenarios():
            name, identity = scenario["name"], {}
            scenario = core.prepare_scenario(
                scenario, binaries, builds, out, 0, env, {}
            )
            for role in ROLES:
                # The per-role scenario run_scenario passes to run_game.
                setup = dict(
                    scenario,
                    **scenario.get("role_inputs", {}).get(role, {}),
                    witnesses=False,
                )
                folder = out / name / "plain" / role
                core.prepare_run_dir(setup, builds[role], folder)
                identity[role] = instructions.prepare(
                    setup, [], binaries[role], builds[role], folder, dict(env), False
                )[1]["identity"]
            # instructions.finish combines the candidate identity the same way.
            identity["candidate"]["compatible"] = instructions.digest(
                [identity["candidate"]["compatible"], identity["reference"]]
            )
            manifest["identities"][name] = identity["candidate"]["compatible"]
            if profile and any(
                records[name][role]["identity"] != identity[role] for role in ROLES
            ):
                sys.exit(f"{name}: frozen runtimes differ from {profile}")
    finally:
        for module, run_game in zip(games, saved, strict=True):
            module.run_game = run_game
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    proved = "matching the profile" if profile else "without a profile"
    print(f"Prepared {len(manifest['identities'])} scenarios {proved}, no games: {out}")
    return 0


def processes():
    """Live processes by pid: comm, state, parent, owner and CPU ticks."""
    found = {}
    for entry in Path("/proc").iterdir():
        try:
            raw = (entry / "stat").read_text()
            fields = raw[raw.rindex(")") + 2 :].split()
            found[int(entry.name)] = dict(
                comm=raw[raw.index("(") + 1 : raw.rindex(")")],
                state=fields[0],
                parent=int(fields[1]),
                ticks=int(fields[11]) + int(fields[12]),
                uid=entry.stat().st_uid,
            )
        except (OSError, ValueError, IndexError):
            continue
    return found


class Hold:
    """Stop other builds, links, profiles and games; resume them on release."""

    def __init__(self, log):
        self.log, self.stopped, self.done = log, set(), threading.Event()
        self.hz, self.thread = os.sysconf("SC_CLK_TCK"), None

    def scan(self, before, elapsed):
        table = processes()

        def ours(pid):  # The collector and its timed games.
            while pid > 1 and pid != os.getpid():
                pid = table.get(pid, {}).get("parent", 0)
            return pid == os.getpid()

        other = {
            pid: row
            for pid, row in table.items()
            if row["state"] not in "ZXTt"
            and (
                row["comm"] in HEAVY or row["comm"].startswith(("openttd", "callgrind"))
            )
            and not ours(pid)
        }
        for pid, row in other.items():
            if row["uid"] == os.getuid():  # Other users' processes stay recorded.
                try:
                    os.kill(pid, signal.SIGSTOP)
                    self.stopped.add(pid)
                except ProcessLookupError:
                    pass
        busy = {
            pid: [row["comm"], round(cpu, 3), ours(pid)]
            for pid, row in table.items()
            if (
                cpu := (row["ticks"] - before.get(pid, row)["ticks"])
                / self.hz
                / elapsed
            )
            > 0.02
        }
        unheld = {p: r["comm"] for p, r in other.items() if p not in self.stopped}
        entry = dict(time=time.time(), load=os.getloadavg(), busy=busy, unheld=unheld)
        with self.log.open("a") as log:
            log.write(json.dumps(entry) + "\n")
        return table

    def start(self):
        def watch():
            before, started = self.scan({}, 1), time.monotonic()
            while not self.done.wait(2):
                now = time.monotonic()
                before, started = self.scan(before, now - started), now

        self.thread = threading.Thread(target=watch, daemon=True)
        self.thread.start()

    def release(self):
        if self.done.is_set():
            return
        self.done.set()
        self.thread.join()
        for pid in self.stopped:
            try:
                os.kill(pid, signal.SIGCONT)
            except ProcessLookupError:
                pass
        print(f"Released {len(self.stopped)} held processes", flush=True)


def collect(out, migration):
    manifest = json.loads((out / "manifest.json").read_text())
    if not manifest["profile"]:
        sys.exit("collect needs a bundle prepared with --profile for anchor Ir")
    if (
        migration.environment().get("LD_LIBRARY_PATH", "")
        != manifest["ld_library_path"]
    ):
        sys.exit("LD_LIBRARY_PATH differs from preparation")
    bundle = out / "frozen"
    for role in ROLES:
        binary = bundle / f"{role}-runtime" / NAMES[role]
        if instructions.digest(binary.read_bytes()) != manifest["executables"][role]:
            sys.exit(f"frozen {role} executable changed")
    names = [s["name"] for s in speed_scenarios()]
    remaining = [GROUPS * len(names) * PAIRS * len(ROLES)]
    hold, group, run_game = Hold(out / "observations.jsonl"), 0, core.run_game

    def timed(*args, **kwargs):
        result = run_game(*args, **kwargs)
        remaining[0] -= 1
        if not remaining[0]:
            hold.release()  # The last timed game has exited.
        return result

    simulation = migration.LOCAL / "simulation"
    core.run_game, core.prune_simulation_runs = timed, lambda root: None
    # Each scenario gets six pairs launched in each order.
    core.pair_order = lambda repetition: ROLES[:: 1 - 2 * ((group + repetition) % 2)]
    hold.start()
    try:
        for group in range(GROUPS):
            before = set(simulation.glob("*/report.json"))
            sys.argv = ["simulate.py", *names, "--benchmark", str(PAIRS), "--jobs", "1"]
            sys.argv += ["--frozen-runtimes", str(bundle)]
            code = core.main()
            (report,) = set(simulation.glob("*/report.json")) - before
            shutil.copy2(report, out / f"group-{group}.json")
            if code:
                break
    finally:
        hold.release()
        core.run_game = run_game
    return summarize(out, manifest, names, group + 1)


def summarize(out, manifest, names, groups):
    """Parse after release; refuse anchors from incomplete or failing groups."""
    profile = json.loads(Path(manifest["profile"]).read_text())
    counts = {r["scenario"]: r["instruction_speed"] for r in profile["results"]}
    medians = {name: [] for name in names}
    for group in range(groups):
        for result in json.loads((out / f"group-{group}.json").read_text())["results"]:
            speed = result["plain_speed"]
            if not result["passed"] or len(speed["samples"]) != PAIRS:
                sys.exit(f"group {group} {result['scenario']} failed; no anchors")
            medians[result["scenario"]].append(
                speed["median_candidate_reference_ratio"]
            )
    if any(len(values) != GROUPS for values in medians.values()):
        sys.exit(f"only {groups} of {GROUPS} groups completed; no anchors")
    rows = [
        (
            n,
            statistics.median(medians[n]),
            counts[n]["candidate"]["Ir"],
            manifest["identities"][n],
        )
        for n in names
    ]
    (out / "anchors.json").write_text(
        json.dumps(dict(head=manifest["head"], medians=medians, rows=rows), indent=2)
        + "\n"
    )
    loads = [json.loads(line) for line in (out / "observations.jsonl").open()]
    print(
        f"{len(loads)} observations: maximum 1-minute load "
        f"{max(entry['load'][0] for entry in loads):.2f}, "
        f"{sum(bool(entry['unheld']) for entry in loads)} with unheld processes"
    )
    print(f'SPEED_HEAD = "{manifest["head"][:10]}"\nSPEED_PASS = (')
    for row in rows:
        print(f"    {row!r},".replace("'", '"'))
    print(")")
    return 0


def main(argv):
    import migration

    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("action", choices=("prepare", "collect"))
    parser.add_argument("out", type=Path, help="pass directory (prepare creates it)")
    parser.add_argument(
        "--profile", type=Path, help="prepare: simulate --instructions report.json"
    )
    args = parser.parse_args(argv)
    if args.action == "prepare":
        return prepare(args.out.resolve(), args.profile, migration)
    if args.profile:
        parser.error("--profile is recorded by prepare")
    return collect(args.out.resolve(), migration)
