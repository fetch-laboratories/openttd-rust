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
import signal
import statistics
import sys
import threading
import time
from pathlib import Path

from deep import LOCK_NAME
from validation_execution import file_lock

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
        profile=str(profile.resolve()) if profile else None,
        profile_sha256=instructions.digest(profile.read_bytes()) if profile else None,
    )
    _, identities = prepare_inputs(out, binaries, builds, env)
    if profile and any(
        records[name][role]["identity"] != identity[role]
        for name, identity in identities.items()
        for role in ROLES
    ):
        sys.exit(f"frozen runtimes differ from {profile}")
    manifest["runtime_identities"] = identities
    manifest["identities"] = {
        name: identity["candidate"]["compatible"]
        for name, identity in identities.items()
    }
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    proved = "matching the profile" if profile else "without a profile"
    print(f"Prepared {len(identities)} scenarios {proved}, no games: {out}")
    return 0


def prepare_inputs(out, binaries, builds, env):
    """Resolve the exact profiled inputs without running any preparation game."""
    scenarios, identities = [], {}

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
            identities[name] = identity
            scenarios.append(scenario)
    finally:
        for module, run_game in zip(games, saved, strict=True):
            module.run_game = run_game
    return scenarios, identities


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
        self.error = None

    def scan(self, before, elapsed, quiescing=False):
        table = processes()

        def ours(pid):  # The collector and its timed games.
            while pid > 1 and pid != os.getpid():
                pid = table.get(pid, {}).get("parent", 0)
            return pid == os.getpid()

        observed = {
            pid: row
            for pid, row in table.items()
            if (
                row["comm"] in HEAVY or row["comm"].startswith(("openttd", "callgrind"))
            )
            and not ours(pid)
        }
        other = {p: r for p, r in observed.items() if r["state"] not in "ZXTt"}
        # Stopping a process cannot undo its overlap with the timed games.
        overlap = {
            p: r["comm"]
            for p, r in observed.items()
            if not quiescing
            and (p in other or r["ticks"] > before.get(p, {"ticks": 0})["ticks"])
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
        entry = dict(
            time=time.time(),
            load=os.getloadavg(),
            busy=busy,
            unheld=unheld,
            overlap=overlap,
        )
        with self.log.open("a") as log:
            log.write(json.dumps(entry) + "\n")
        return table

    def start(self):
        before, started = self.scan({}, 1, True), time.monotonic()

        def watch():
            nonlocal before, started
            try:
                while not self.done.wait(2):
                    now = time.monotonic()
                    before, started = self.scan(before, now - started), now
                self.scan(before, max(time.monotonic() - started, 0.001))
            except Exception as error:
                self.error = error

        self.thread = threading.Thread(target=watch, daemon=True)
        self.thread.start()

    def release(self):
        if self.done.is_set():
            return
        self.done.set()
        if self.thread is not None:
            self.thread.join()
        for pid in self.stopped:
            try:
                os.kill(pid, signal.SIGCONT)
            except ProcessLookupError:
                pass
        print(f"Released {len(self.stopped)} held processes", flush=True)
        if self.error is not None:
            raise RuntimeError("speed-pass process observation failed") from self.error


def collection_lock(migration):
    """Wait for any deep run (tools/deep.py) and keep it out until released."""
    return file_lock(migration.COMMON_LOCAL / LOCK_NAME, label="deep check")


def collect(out, migration):
    manifest = json.loads((out / "manifest.json").read_text())
    if not manifest["profile"]:
        sys.exit("collect needs a bundle prepared with --profile for anchor Ir")
    if migration.git("rev-parse", "HEAD") != manifest["head"] or migration.git(
        "status", "--short"
    ):
        sys.exit("collect requires the clean checkout used by prepare")
    if (
        instructions.digest(Path(manifest["profile"]).read_bytes())
        != manifest["profile_sha256"]
    ):
        sys.exit("instruction profile changed after preparation")
    env = migration.environment()
    if env.get("LD_LIBRARY_PATH", "") != manifest["ld_library_path"]:
        sys.exit("LD_LIBRARY_PATH differs from preparation")
    bundle = out / "frozen"
    builds = {role: bundle / f"{role}-runtime" for role in ROLES}
    binaries = {role: builds[role] / NAMES[role] for role in ROLES}
    with collection_lock(migration):
        scenarios, identities = prepare_inputs(out, binaries, builds, env)
        if identities != manifest["runtime_identities"]:
            sys.exit("runtime, libraries or inputs changed after preparation")
        collect_games(out, scenarios, binaries, builds, env)
    return summarize(out, manifest, [s["name"] for s in scenarios], GROUPS)


def collect_games(out, scenarios, binaries, builds, env):
    """Prepare every launch before the hold; compare every pair after release."""
    plans = {}
    for group in range(GROUPS):
        for scenario in scenarios:
            for pair in range(PAIRS):
                for role in ROLES:
                    setup = dict(
                        scenario,
                        **scenario.get("role_inputs", {}).get(role, {}),
                        witnesses=False,
                    )
                    folder = (
                        out / f"group-{group}" / scenario["name"] / str(pair) / role
                    )
                    plans[group, scenario["name"], pair, role] = core.prepare_game(
                        setup,
                        binaries[role],
                        builds[role],
                        folder,
                        1200,
                        env,
                        False,
                        isolate=True,
                    )
    hold, runs = Hold(out / "observations.jsonl"), {}
    try:
        hold.start()
        for group in range(GROUPS):
            for scenario in scenarios:
                for pair in range(PAIRS):
                    for role in ROLES[:: 1 - 2 * ((group + pair) % 2)]:
                        key = group, scenario["name"], pair, role
                        runs[key] = core.execute_game(plans[key])
    finally:
        hold.release()  # The final timed game exits before any output is decoded.
    for group in range(GROUPS):
        results = []
        for scenario in scenarios:
            recorded = iter(
                core.read_game(runs[group, scenario["name"], pair, role])
                for pair in range(PAIRS)
                for role in ROLES
            )
            results.append(
                core.run_scenario(
                    scenario,
                    binaries,
                    builds,
                    out,
                    20,
                    1200,
                    env,
                    PAIRS,
                    game_runner=lambda *args, recorded=recorded, **kwargs: next(
                        recorded
                    ),
                    prepared=True,
                )
            )
        (out / f"group-{group}.json").write_text(
            json.dumps({"results": results}, indent=2) + "\n"
        )


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
    loads = [
        json.loads(line)
        for line in (out / "observations.jsonl").read_text().splitlines()
    ]
    if any(entry["unheld"] or entry["overlap"] for entry in loads):
        sys.exit("other builds, profiles or games ran during collection; no anchors")
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
    print(
        f"{len(loads)} observations: maximum 1-minute load "
        f"{max(entry['load'][0] for entry in loads):.2f}, "
        f"{sum(bool(entry['unheld']) for entry in loads)} with unheld processes"
    )
    print(f'SPEED_HEAD = "{manifest["head"][:10]}"\nSPEED_PASS = (')
    history = {name: ratios for name, ratios, _, _ in instructions.SPEED_PASS}
    for name, ratio, ir, identity in rows:
        ratios = (*history.get(name, ())[-2:], ratio)
        row = (name, ratios, ir, identity)
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
