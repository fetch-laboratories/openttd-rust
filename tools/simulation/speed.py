"""Quiet speed pass: frozen runtimes, interleaved wall pairs and new anchors (AGENTS.md).

`prepare` freezes both runtimes and proves their instruction identities against
an optional `simulate --instructions` report; it runs no games. `collect` holds
other builds, links, profiles and games stopped only while its 144 timed games
run, then reports the median of four interleaved three-pair medians per speed
scenario as `SPEED_PASS` rows for instructions.py.
"""

import argparse
import json
import os
import re
import shutil
import signal
import statistics
import subprocess
import sys
import threading
import time
from pathlib import Path

from . import core, instructions

GROUPS, PAIRS = 4, 3
ROLES = ("reference", "candidate")
NAMES = {"reference": "openttd", "candidate": "openttd-rust"}
# Process names (/proc comm, 15 characters) of builds, links and profiles.
HEAVY = {
    "as",
    "cargo",
    "cc1",
    "cc1plus",
    "ccache",
    "clang",
    "clang++",
    "cmake",
    "collect2",
    "g++",
    "gcc",
    "gmake",
    "ld",
    "ld.bfd",
    "ld.gold",
    "ld.lld",
    "lld",
    "lto1",
    "make",
    "mold",
    "ninja",
    "rustc",
    "rustfmt",
    "valgrind",
    "valgrind.bin",
}


def load_migration():
    import importlib.util

    spec = importlib.util.spec_from_file_location(
        "migration", core.ROOT / "tools/migration.py"
    )
    migration = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(migration)
    return migration


def libraries(binary, env):
    """The libraries the loader resolves, which instructions.identity also hashes."""
    loaded = subprocess.check_output(["ldd", str(binary)], env=env, text=True)
    return {
        path: instructions.digest(Path(path).read_bytes())
        for path in re.findall(r"(/\S+) \(", loaded)
    }


def speed_scenarios():
    return [
        s for s in core.scenario_list(False) if s["name"] in instructions.SPEED_BUDGETS
    ]


def prepare(out, profile, migration):
    """Freeze runtimes and record identities; preparation hooks may run no game."""
    head, status = (
        migration.git("rev-parse", "HEAD"),
        migration.git("status", "--short"),
    )
    if status:
        sys.exit("speed prepare requires a clean checkout: anchors name its head")
    out.mkdir(parents=True)
    bundle = out / "frozen"
    bundle.mkdir()
    env = migration.environment()
    with migration.reference_lock(shared=True):
        reference = migration.REFERENCE_BUILD.resolve()
        core.copy_runtime(
            reference, reference / NAMES["reference"], bundle / "reference-runtime"
        )
    candidate = core.ROOT / "build-rust"
    core.copy_runtime(
        candidate,
        candidate / NAMES["candidate"],
        bundle / "candidate-runtime",
        candidate=True,
    )
    builds = {role: bundle / f"{role}-runtime" for role in ROLES}
    binaries = {role: builds[role] / NAMES[role] for role in ROLES}
    if not all((builds[role] / "instruction-build.json").is_file() for role in ROLES):
        sys.exit(
            "build first with: python3 tools/migration.py simulate --instructions --list"
        )
    for role in ROLES:
        # --frozen-runtimes copies every adapter folder; speed scenarios use none.
        for module in core.adapter_modules():
            (builds[role] / module.__name__.rsplit(".", 1)[1]).mkdir()
    manifest = {
        "head": head,
        "ld_library_path": env.get("LD_LIBRARY_PATH", ""),
        "profile": str(profile) if profile else None,
        "executables": {
            role: instructions.digest(binaries[role].read_bytes()) for role in ROLES
        },
        "libraries": {role: libraries(binaries[role], env) for role in ROLES},
        "identities": {},
    }
    records = {}
    if profile:
        report = json.loads(profile.read_text())
        if not report["passed"] or report["candidate_commit"] != head:
            sys.exit(f"{profile} is not a passing profile of {head}")
        records = {r["scenario"]: r["instruction_speed"] for r in report["results"]}

    def no_game(*args, **kwargs):
        raise RuntimeError("speed preparation runs no game")

    games = [m for m in (core, *core.scenario_modules()) if hasattr(m, "run_game")]
    saved = [m.run_game for m in games]
    for module in games:
        module.run_game = no_game
    try:
        for scenario in speed_scenarios():
            name, result = scenario["name"], {}
            for module in sorted(
                core.scenario_modules(), key=lambda m: getattr(m, "PREPARE_ORDER", 4)
            ):
                if hook := getattr(module, "prepare", None):
                    scenario = hook(scenario, binaries, builds, out, 0, env, result)
            identity = {}
            for role in ROLES:
                # The same per-role scenario run_scenario passes to run_game.
                setup = dict(
                    scenario,
                    **scenario.get("role_inputs", {}).get(role, {}),
                    witnesses=False,
                )
                folder = out / name / "plain" / role
                core.prepare_run_dir(setup, builds[role], folder)
                _, info = instructions.prepare(
                    setup, [], binaries[role], builds[role], folder, dict(env), False
                )
                identity[role] = info["identity"]
            compatible = instructions.digest(
                [identity["candidate"]["compatible"], identity["reference"]]
            )
            manifest["identities"][name] = compatible
            if profile:
                record = records[name]
                if (
                    record["candidate"]["identity"]["compatible"] != compatible
                    or record["candidate"]["identity"]["executable"]
                    != manifest["executables"]["candidate"]
                    or record["reference"]["identity"]["executable"]
                    != manifest["executables"]["reference"]
                ):
                    sys.exit(f"{name}: frozen runtimes differ from {profile}")
    finally:
        for module, run_game in zip(games, saved, strict=True):
            module.run_game = run_game
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    proved = "matching the profile" if profile else "without a profile"
    print(
        f"Prepared {len(manifest['identities'])} speed scenarios {proved}; no games ran: {out}"
    )
    return 0


def processes():
    """Live processes: comm, state, parent, owner and CPU ticks."""
    found = {}
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
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


def ours(pid, table):
    """This collector and its descendants: the timed games and their launchers."""
    while pid > 1:
        if pid == os.getpid():
            return True
        pid = table.get(pid, {}).get("parent", 0)
    return False


class Hold:
    """Stop other builds, links, profiles and games; resume them on release."""

    def __init__(self, log):
        self.log, self.stopped, self.done = log, set(), threading.Event()
        self.lock, self.thread = threading.Lock(), None
        self.hz = os.sysconf("SC_CLK_TCK")

    def heavy(self, row):
        return row["comm"] in HEAVY or row["comm"].startswith(("openttd", "callgrind-"))

    def scan(self, before, elapsed):
        table = processes()
        with self.lock:
            for pid, row in table.items():
                if row["state"] in "ZXTt" or not self.heavy(row) or ours(pid, table):
                    continue
                if row["uid"] != os.getuid():
                    continue  # Recorded below; another user's process cannot be held.
                try:
                    os.kill(pid, signal.SIGSTOP)
                    self.stopped.add(pid)
                except ProcessLookupError:
                    pass
        busy = [
            dict(
                pid=pid,
                comm=row["comm"],
                cores=round(
                    (row["ticks"] - before[pid]["ticks"]) / self.hz / elapsed, 3
                ),
                ours=ours(pid, table),
                state=row["state"],
            )
            for pid, row in table.items()
            if pid in before
            and row["ticks"] - before[pid]["ticks"] > 0.02 * self.hz * elapsed
        ]
        unheld = [
            dict(pid=pid, comm=row["comm"], uid=row["uid"])
            for pid, row in table.items()
            if row["state"] not in "ZXTt" and self.heavy(row) and not ours(pid, table)
        ]
        entry = dict(
            time=time.time(),
            load=os.getloadavg(),
            held=len(self.stopped),
            busy=busy,
            unheld=unheld,
        )
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
        with self.lock:
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
    env = migration.environment()
    if env.get("LD_LIBRARY_PATH", "") != manifest["ld_library_path"]:
        sys.exit("LD_LIBRARY_PATH differs from preparation")
    bundle = out / "frozen"
    for role in ROLES:
        binary = bundle / f"{role}-runtime" / NAMES[role]
        if instructions.digest(binary.read_bytes()) != manifest["executables"][role]:
            sys.exit(f"frozen {role} executable changed")
        if libraries(binary, env) != manifest["libraries"][role]:
            sys.exit(f"{role} loader libraries differ from preparation")
    names = [s["name"] for s in speed_scenarios()]
    expected = GROUPS * len(names) * PAIRS * len(ROLES)
    hold = Hold(out / "observations.jsonl")
    played, group = [], 0
    run_game = core.run_game

    def timed(*args, **kwargs):
        result = run_game(*args, **kwargs)
        played.append([group, args[0]["name"], Path(args[3]).name, result["seconds"]])
        if len(played) == expected:
            hold.release()  # The last timed game has exited.
        return result

    reports = []
    simulation = migration.LOCAL / "simulation"
    core.run_game = timed
    # Alternate launch order: each scenario gets six pairs of each order.
    core.pair_order = lambda repetition: ROLES[:: 1 - 2 * ((group + repetition) % 2)]
    core.prune_simulation_runs = lambda root: None
    hold.start()
    try:
        for group in range(GROUPS):
            before = set(simulation.glob("*/report.json"))
            sys.argv = [
                "simulate.py",
                *names,
                "--benchmark",
                str(PAIRS),
                "--jobs",
                "1",
                "--frozen-runtimes",
                str(bundle),
            ]
            code = core.main()
            (report,) = set(simulation.glob("*/report.json")) - before
            shutil.copy2(report, out / f"group-{group}.json")
            reports.append(str(report))
            if code:
                break
    finally:
        hold.release()
        core.run_game = run_game
    (out / "games.json").write_text(json.dumps(played, indent=2) + "\n")
    return summarize(out, manifest, names, len(reports))


def summarize(out, manifest, names, groups):
    """Parse after release; refuse anchors from incomplete or failing groups."""
    profile = json.loads(Path(manifest["profile"]).read_text())
    counts = {r["scenario"]: r["instruction_speed"] for r in profile["results"]}
    medians = {name: [] for name in names}
    for group in range(groups):
        report = json.loads((out / f"group-{group}.json").read_text())
        for result in report["results"]:
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
            name,
            statistics.median(medians[name]),
            counts[name]["candidate"]["Ir"],
            manifest["identities"][name],
        )
        for name in names
    ]
    (out / "anchors.json").write_text(
        json.dumps(dict(head=manifest["head"], medians=medians, rows=rows), indent=2)
        + "\n"
    )
    loads = [
        json.loads(line)
        for line in (out / "observations.jsonl").read_text().splitlines()
    ]
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
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("action", choices=("prepare", "collect"))
    parser.add_argument("out", type=Path, help="pass directory (prepare creates it)")
    parser.add_argument(
        "--profile",
        type=Path,
        help="simulate --instructions report.json of the same head (prepare)",
    )
    args = parser.parse_args(argv)
    migration = load_migration()
    if args.action == "prepare":
        return prepare(args.out.resolve(), args.profile, migration)
    if args.profile:
        parser.error("--profile is recorded by prepare")
    return collect(args.out.resolve(), migration)
