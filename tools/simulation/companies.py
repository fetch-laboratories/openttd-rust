"""Company lifecycle witnesses using real reference-built AI companies."""

import datetime
import hashlib
import lzma
import shutil
import struct
import subprocess
from pathlib import Path
from typing import TypedDict

from .core import (
    EMPTY_AI,
    ROOT,
    SNAPSHOT_TICKS,
    Reader,
    ai_config,
    patch,
    read_save,
    rows,
    run_game,
    set_ai_slots,
)

AI_FOLDER = "company-scenario-ai"


def command_gap():
    """Unchanged native financial commands for deity/network/Money boundaries."""
    import importlib.util

    import probe

    spec = importlib.util.spec_from_file_location(
        "migration", ROOT / "tools/migration.py"
    )
    migration = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(migration)
    out = ROOT / ".local/company-command-gap"
    out.mkdir(parents=True, exist_ok=True)
    sources = {
        name: subprocess.check_output(
            ["git", "show", migration.BASELINE["commit"] + ":src/" + name], cwd=ROOT
        ).decode()
        for name in ("misc_cmd.cpp", "company_cmd.cpp")
    }

    bodies = [
        probe.body(sources["company_cmd.cpp"], signature)
        for signature in (
            "Money Company::GetMaxLoan() const",
            "Money GetAvailableMoney(CompanyID company)",
            "Money GetAvailableMoneyForCommand()",
            "static void SubtractMoneyFromAnyCompany(",
        )
    ]
    bodies += [
        probe.body(sources["misc_cmd.cpp"], "CommandCost " + name + "(")
        for name in (
            "CmdIncreaseLoan",
            "CmdDecreaseLoan",
            "CmdSetCompanyMaxLoan",
            "CmdChangeBankBalance",
        )
    ]
    bodies.append(probe.body(sources["company_cmd.cpp"], "CommandCost CmdGiveMoney("))
    (out / "company-command-reference.inc").write_text("\n".join(bodies))
    subprocess.run(
        [
            "c++",
            "-std=c++20",
            "-O2",
            "-DPOINTER_IS_64BIT",
            "-I" + str(ROOT / "src"),
            "-I" + str(out),
            str(ROOT / "tools/simulation/company-command-reference.cpp"),
            str(migration.rust_archive(ROOT / "build-rust")),
            "-ldl",
            "-lpthread",
            "-lm",
            "-o",
            str(out / "probe"),
        ],
        cwd=ROOT,
        env=migration.environment(),
        check=True,
    )
    subprocess.run([str(out / "probe")], cwd=ROOT, check=True)


def scenarios(soak):
    return [
        {
            "name": f"companies-{operation}",
            "kind": "save",
            "companies": operation,
            "short_checkpoint": True,
            "save": str(ROOT / "migration/saves/padhattan-ridge-1996.sav"),
            "console": ["unpause"],
            "ticks": ticks,
        }
        for operation, ticks in (
            ("warning", 120),
            ("sale", 120),
            ("rank", 120),
            ("tie", 60),
            ("timeout", 60),
            ("limit", 60),
            ("acquire", 180),
            ("recover", 120),
            ("delete", 120),
            ("reuse", 120),
            ("finance", (24 if soak else 12) * SNAPSHOT_TICKS),
            ("reload", 120),
            ("competitors", 120),
            ("competitor-timer", 3 * SNAPSHOT_TICKS if soak else 120),
            ("competitor-reload", 120),
            ("identity", 120),
            ("identity-reload", 120),
        )
    ]


def uses_ai(scenario):
    return "companies" in scenario


def install(scenario, run_dir):
    if not uses_ai(scenario):
        return
    folder = run_dir / "ai/companies"
    shutil.copytree(scenario["scenario_ai"], folder)
    (folder / "parameters.nut").write_text(
        f"COMPANY_SETUP <- {str(scenario.get('company_setup', False)).lower()};\n"
        f'COMPANY_ACTION <- "{scenario["companies"]}";\n'
    )


def configure_competitors(path, record):
    """Declare fixed local scripts for vacant slots; preserve running AI data."""
    elements = dict(read_save(path)["AIPL"]["elements"])
    for index in (3, 4):
        if elements[index] != EMPTY_AI:
            raise RuntimeError("competitor input slot already has an AI configured")
        record["assignments"].append(
            ["AIPL", index, "configuration", "MigrationCompanies"]
        )
    config = ai_config(b"MigrationCompanies")
    prepared = set_ai_slots(path.read_bytes(), {3: config, 4: config})
    path.write_bytes(prepared)
    record["sha256"] = hashlib.sha256(prepared).hexdigest()


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if not uses_ai(scenario):
        return scenario
    folder = out / scenario["name"] / "input"
    folder.mkdir(parents=True, exist_ok=True)
    source = Path(scenario["save"])
    packed = source.read_bytes()
    raw = folder / "player.sav"
    raw.write_bytes(
        packed
        if packed[:4] == b"OTTN"
        else b"OTTN" + packed[4:8] + lzma.decompress(packed[8:])
    )
    jobs = rows(raw, "LGRJ")
    setup_input = folder / "setup-input.sav"
    setup_record = patch(
        raw,
        setup_input,
        {
            "LGRJ": {i: {"join_date": v["join_date"] + 365} for i, v in jobs.items()},
        },
    )
    setup = dict(
        scenario,
        save=str(setup_input),
        company_setup=True,
        ticks=19000,
        console=[
            "start_ai MigrationCompanies",
            "start_ai MigrationCompanies",
            "unpause",
        ],
    )
    built = run_game(
        setup,
        binaries["reference"],
        builds["reference"],
        folder / "setup",
        timeout,
        env,
        False,
    )
    assets = [
        line.split("COMPANY-ASSETS ", 1)[1]
        for line in built["log"]
        if "COMPANY-ASSETS " in line
    ]
    if built["exit"] or len(assets) != 1:
        raise RuntimeError("company preparation did not create actual AI assets")
    fixture = built["snapshots"][-1]
    companies = rows(fixture, "PLYR")
    if (
        set(companies) != {0, 1, 2}
        or "old_economy[1]/performance_history" not in companies[2]
    ):
        raise RuntimeError(
            "company preparation lacks three companies and quarterly histories"
        )
    operation = scenario["companies"]
    # Pinned-reference source CompanyCheckBankrupt uses money-loan < -maxloan;
    # months 4 warns, 7 opens sale and 10 deletes. Rank reads old_economy[1].
    finances = {
        1: {"money": -1_000_000, "current_loan": 300_000, "max_loan": 300_000},
        2: {"money": 2_000_000},
    }
    for i, score in ((0, 100), (2, 900)):
        finances.setdefault(i, {}).update(
            {f"old_economy[{q}]/performance_history": score for q in (0, 1)}
        )
    month = {"warning": 3, "sale": 6, "recover": 6, "delete": 9}.get(operation, 7)
    finances[1].update(
        months_of_bankruptcy=month,
        bankrupt_asked=2,
        bankrupt_value=10_000,
        bankrupt_timeout=0,
    )
    if operation in ("warning", "sale"):
        finances[1].update(bankrupt_asked=0, bankrupt_value=0)
    if operation == "recover":
        finances[1]["money"] = 500_000
    if operation in ("finance", "reuse", "reload") or operation.startswith(
        ("competitor", "identity")
    ):
        finances[1].update(money=500_000, months_of_bankruptcy=0, bankrupt_asked=0)
    if operation == "tie":
        finances[0]["old_economy[1]/performance_history"] = 900
    if operation == "timeout":
        finances[1].update(bankrupt_asked=6, bankrupt_timeout=15)
    date = rows(fixture, "DATE")[0]
    calendar = datetime.date.fromordinal(date["date"] - 365)
    day = (
        datetime.date(calendar.year, 12, 31)
        if operation == "finance"
        else datetime.date(calendar.year, 2, 28)
    )
    # Start immediately before the monthly boundary; same calendar/economy input
    # and RNG go to both roles. Pending joins stay beyond the short windows.
    changes = {
        "PLYR": finances,
        "DATE": {
            0: {
                "date": day.toordinal() + 365,
                "economy_date": day.toordinal() + 365,
                "date_fract": 73,
                "economy_date_fract": 73,
                "days_since_last_month": day.day - 1,
            }
        },
        "LGRJ": {
            i: {"join_date": v["join_date"] + 365}
            for i, v in rows(fixture, "LGRJ").items()
        },
    }
    if operation == "finance":
        changes["PLYR"][1].update(
            money_fraction=255,
            terraform_limit=0,
            clear_limit=0,
            tree_limit=0,
            block_preview=2,
        )
        changes["ECMY"] = {0: {"fluct": 1}}
        scenario = dict(
            scenario,
            console=[
                "setting economy.infrastructure_maintenance 1",
                "setting economy.inflation 1",
                "setting difficulty.economy 1",
                "unpause",
            ],
        )
    if operation == "limit":
        scenario = dict(scenario, console=["setting vehicle.max_roadveh 0", "unpause"])
    if operation == "reuse":
        scenario = dict(
            scenario,
            console=[
                "stop_ai 2",
                "start_ai MigrationCompanies",
                "reload_ai 2",
                "unpause",
            ],
        )
    if operation.startswith("competitor"):
        # There are already two actor AIs. Cap four requires two successive
        # OnTick Posts; interval one instead exercises the timer's single Post.
        timer = operation == "competitor-timer"
        # Keep the fixture's dates: startup needs no monthly boundary, and moving
        # time backwards makes existing link-graph edge dates invalid in a soak.
        changes["DATE"] = {
            0: {
                "competitors_interval": 1 if timer else 0,
                "competitors_interval_elapsed": 0,
                "competitors_interval_fired": not timer,
            }
        }
        scenario = dict(
            scenario,
            console=[
                "setting difficulty.max_no_competitors 4",
                f"setting difficulty.competitors_interval {int(timer)}",
                "unpause",
            ],
        )
    prepared = folder / "company-input.sav"
    record = patch(fixture, prepared, changes)
    if operation.startswith("competitor"):
        configure_competitors(prepared, record)
    record.update(
        committed_source=str(source),
        committed_sha256=hashlib.sha256(packed).hexdigest(),
        setup_input=setup_record,
        setup_console=setup["console"],
        assets=[int(x) for x in assets[0].split()],
        console=scenario["console"],
    )
    if operation in ("reload", "competitor-reload"):
        live = run_game(
            dict(scenario, save=str(prepared), ticks=120),
            binaries["reference"],
            builds["reference"],
            folder / "reload",
            timeout,
            env,
            False,
        )
        if live["exit"] or not live["snapshots"]:
            raise RuntimeError("company reference reload failed")
        if operation == "competitor-reload":
            check_competitors(scenario, live, prepared, live["snapshots"][-1])
        prepared = folder / "reload-input.sav"
        record["reload"] = patch(live["snapshots"][-1], prepared, {})
    if operation in ("delete", "acquire"):
        from .economy import active_monitors

        active_monitors(prepared, cargoes=(63,), amount=17)
    result["company_input"] = record
    if operation.startswith("identity"):
        from . import fleet
        from .console_adapter import prepared_console
        from .core import capture_pair, compare_saves

        executables = prepared_console(
            "fleet", fleet.adapter, binaries, builds, folder, env
        )
        scenario = dict(
            scenario,
            save=str(prepared),
            console=["start_ai MigrationCompanies"] * 12
            + ["fleet_scenario company-identity", "unpause"],
            executables=executables,
        )
        if operation == "identity-reload":
            runs = capture_pair(scenario, executables, builds, folder, timeout, env)
            saves = {}
            for role, run in runs.items():
                check(scenario, run, "capture", role, result)
                if run["exit"] or not run["snapshots"]:
                    raise RuntimeError("company identity capture failed")
                saves[role] = run["snapshots"][-1]
            differences = compare_saves(
                saves["reference"],
                saves["candidate"],
                scenario["name"],
                20,
                result["stats"],
            )
            traces = [
                [
                    line
                    for line in runs[role]["log"]
                    if line.startswith(("COMPANY-IDENTITY ", "COMPANY-CONTROL "))
                ]
                for role in ("reference", "candidate")
            ]
            if differences or traces[0] != traces[1]:
                result["differences"].extend(differences)
                raise RuntimeError("company identity capture differs")
            rng = [
                line.split()
                for line in traces[0]
                if line.startswith("COMPANY-IDENTITY rng ")
            ]
            if len(rng) != 1 or len(rng[0]) != 6:
                raise RuntimeError("company capture lacks an unambiguous RNG record")
            first, second = (int(value) for value in rng[0][-2:])
            record["interactive_reload"] = [first, second]
            scenario = dict(
                scenario,
                role_inputs={role: {"save": str(save)} for role, save in saves.items()},
                console=[f"fleet_scenario company-resume {first} {second}", "unpause"],
            )
        return scenario
    if operation == "acquire":
        from . import fleet
        from .console_adapter import prepared_console

        scenario = dict(
            scenario,
            executables=prepared_console(
                "fleet", fleet.adapter, binaries, builds, folder, env
            ),
            console=["fleet_scenario company-control-observe"] + scenario["console"],
        )
    return dict(scenario, save=str(prepared))


def check_competitors(scenario, run, source, final):
    """Require runtime creation/events, fixed configurations and saved timer state."""
    operation = scenario["companies"]
    before, after = rows(source, "PLYR"), rows(final, "PLYR")
    created = (
        {3} if operation == "competitor-timer" and scenario["ticks"] == 120 else {3, 4}
    )
    reload = operation == "competitor-reload" and set(before) == {0, 1, 2, 3, 4}
    if set(before) != ({0, 1, 2, 3, 4} if reload else {0, 1, 2}):
        raise RuntimeError("competitor input has unexpected company IDs")
    if set(after) != {0, 1, 2} | created:
        raise RuntimeError("competitor Posts did not create the expected companies")
    log = "\n".join(run["log"])
    for index in created:
        if f"COMPANY-START {index}" not in log:
            raise RuntimeError(f"competitor {index} did not start the local AI")
        # Loading a running AI does not broadcast CompanyNew. An existing actor
        # receiving this event proves creation occurred after input loading.
        if not reload and f"COMPANY-NEW 2 {index}" not in log:
            raise RuntimeError(f"competitor {index} lacks its runtime creation event")
        if not after[index]["is_ai"]:
            raise RuntimeError(f"competitor {index} is not an AI company")
    chunks = read_save(final)
    for index, body in chunks["AIPL"]["elements"]:
        if index not in created:
            continue
        # Running AIPL rows append VM data beyond the table header. Read both
        # declared configuration prefixes without interpreting opaque VM state.
        reader = Reader(body)
        for _ in range(2):
            name = reader.take(reader.gamma()).decode()
            settings = reader.take(reader.gamma())
            version = struct.unpack(">I", reader.take(4))[0]
            if name != "MigrationCompanies" or settings or version != 1:
                raise RuntimeError(
                    f"competitor {index} did not retain its fixed configuration"
                )
    date = rows(final, "DATE")[0]
    if date["competitors_interval_fired"] or not date["competitors_interval"]:
        raise RuntimeError("competitor timer did not reset after startup")
    settings = rows(final, "PATS")[0]
    if settings["difficulty.max_no_competitors"] != 4 or settings[
        "difficulty.competitors_interval"
    ] != (operation == "competitor-timer"):
        raise RuntimeError("competitor settings did not take effect")
    return {
        "created": [] if reload else sorted(created),
        "reloaded": sorted(created) if reload else [],
        "timer": {
            key: value for key, value in date.items() if key.startswith("competitors_")
        },
    }


class ControlInput(TypedDict):
    console: list[str]


class ControlObservation(TypedDict):
    log: list[str]


def check_control(
    scenario: ControlInput, run: ControlObservation, final: Path
) -> list[str]:
    """Require native notifications and exact allow-list bytes before and after loading."""
    trace = [line for line in run["log"] if line.startswith("COMPANY-CONTROL ")]
    keys = [
        value
        for field, value in rows(final, "PLYR")[0].items()
        if field.startswith("allow_list[")
    ]
    expected = ["A" * 64, "z" * 64]
    if keys != expected:
        raise RuntimeError(f"company allow-list contents differ: {keys!r}")
    if [
        line.split()[-1] for line in trace if line.startswith("COMPANY-CONTROL key 0 ")
    ] != expected:
        raise RuntimeError("company allow-list native reload observation is absent")
    if "fleet_scenario company-identity" in scenario["console"]:
        if "COMPANY-CONTROL complete rng-unchanged true" not in trace:
            raise RuntimeError("company control did not preserve both RNG streams")
        if sum(line.startswith("COMPANY-CONTROL local ") for line in trace) != 6:
            raise RuntimeError("company control local context transitions are absent")
        if sum(line.startswith("COMPANY-CONTROL ownership ") for line in trace) != 8:
            raise RuntimeError(
                "company control encoded ownership observations are absent"
            )
        if sum(line.startswith("COMPANY-CONTROL allow ") for line in trace) != 17:
            raise RuntimeError("company control allow-list cases are absent")
    return trace


def check(scenario, run, mode, role, result):
    if not uses_ai(scenario):
        return
    if not run["snapshots"]:
        raise RuntimeError("company witness has no output save")
    operation = scenario["companies"]
    source, final = Path(scenario["save"]), run["snapshots"][-1]
    before, after = rows(source, "PLYR"), rows(final, "PLYR")
    if operation.startswith("identity"):
        control_trace = check_control(scenario, run, final)
        log = run["log"]
        if len(after) != 15 or len({c["colour"] for c in after.values()}) != 15:
            raise RuntimeError(
                "company identity did not exhaust the reachable colour pool"
            )
        if sum(c["is_ai"] for c in after.values()) != 14:
            raise RuntimeError("company identity did not create fourteen AI companies")
        if "fleet_scenario company-identity" in scenario["console"]:
            if "COMPANY-IDENTITY complete true" not in log:
                raise RuntimeError("company identity commands did not complete")
        if not any(line.startswith("COMPANY-IDENTITY rng ") for line in log):
            raise RuntimeError("company identity lacks both RNG streams")
        if scenario["console"][0].startswith("fleet_scenario company-resume "):
            if not any(
                line.startswith("COMPANY-IDENTITY command face-after-reload true ")
                for line in log
            ):
                raise RuntimeError(
                    "company identity did not consume the reloaded RNG input"
                )
        if (
            after[1]["name_1"] == after[2]["name_1"]
            and after[1]["name_2"] == after[2]["name_2"]
        ):
            raise RuntimeError("company generated-name collision was not resolved")
        result[f"{mode}_{role}_identity"] = {
            "companies": len(after),
            "ai_companies": sum(c["is_ai"] for c in after.values()),
            "control_trace": control_trace,
            "trace": [
                line
                for line in log
                if line.startswith(("COMPANY-IDENTITY ", "COMPANY-CONTROL "))
            ],
        }
        return
    if operation.startswith("competitor"):
        result[f"{mode}_{role}_competitors"] = check_competitors(
            scenario, run, source, final
        )
    log = "\n".join(run["log"])

    def require(condition, message):
        if not condition:
            raise RuntimeError(f"company {operation}: {message}")

    if operation == "warning":
        require(
            after[1]["months_of_bankruptcy"] == 4 and "COMPANY-TROUBLE 2 1" in log,
            "bankruptcy warning/event absent",
        )
        require("COMPANY-OFFER" not in log, "warning incorrectly opened takeover")
    if operation in ("sale", "rank"):
        require(
            after[1]["bankrupt_asked"] == 6
            and after[1]["bankrupt_timeout"] > 0
            and "COMPANY-OFFER 2 1" in log,
            "higher-performance offer absent",
        )
        if operation == "sale":
            require(
                after[1]["months_of_bankruptcy"] == 7
                and after[1]["bankrupt_value"] > 0,
                "sale transition absent",
            )
    if operation == "tie":
        require(
            after[1]["bankrupt_asked"] == 3 and after[1]["bankrupt_timeout"] > 0,
            "equal-performance first-pool-ID tie absent",
        )
    if operation == "timeout":
        require(
            after[1]["bankrupt_asked"] == 7 and after[1]["bankrupt_timeout"] > 15,
            "expired offer did not move to next candidate",
        )
    if operation == "limit":
        require(
            after[1]["bankrupt_asked"] == 65535
            and after[1]["bankrupt_timeout"] == 0
            and "COMPANY-OFFER" not in log,
            "vehicle-limit exclusion absent",
        )
    if operation == "recover":
        require(
            after[1]["months_of_bankruptcy"] == 0 and after[1]["bankrupt_asked"] == 0,
            "positive balance did not clear bankruptcy",
        )
    depot, station, vehicle, group = result["company_input"]["assets"]
    if operation in ("acquire", "delete", "reuse"):
        if operation != "reuse":
            for cid in ("CMDL", "CMPU"):
                old = {v["number"]: v["amount"] for v in rows(source, cid).values()}
                new = {v["number"]: v["amount"] for v in rows(final, cid).values()}
                require(
                    old
                    and new == {k: v for k, v in old.items() if (k >> 25) & 15 != 1},
                    "selective monitor cleanup " + cid,
                )
        vehicles, groups, stations = (
            rows(final, "VEHS"),
            rows(final, "GRPS"),
            rows(final, "STNN"),
        )
        if operation == "acquire":
            require(
                1 not in after
                and "COMPANY-MERGER-END" in log
                and "COMPANY-CHECK merger-test-unchanged true" in log,
                "native test/execute merger absent",
            )
            require(
                vehicles[vehicle]["roadveh[0]/common[0]/owner"] == 2
                and groups[group]["owner"] == 2,
                "vehicle/group ownership not transferred",
            )
            require(
                stations[station]["normal[0]/base[0]/owner"] == 2
                and read_save(final)["MAPO"]["raw"][depot] == 2,
                "station/map ownership not transferred",
            )
            old_towns, new_towns = rows(source, "CITY"), rows(final, "CITY")
            rated = [i for i, town in old_towns.items() if town["have_ratings"] & 2]
            require(
                rated
                and all(
                    new_towns[i]["have_ratings"] & 4
                    and not new_towns[i]["have_ratings"] & 2
                    for i in rated
                ),
                "native station construction town rating not transferred",
            )
        else:
            require(
                vehicle not in vehicles and group not in groups,
                "deleted company assets survived",
            )
            if operation == "delete":
                require(1 not in after, "bankrupt company survived")
            else:
                require(
                    1 in after
                    and after[1]["months_of_bankruptcy"] == 0
                    and after[1]["money_fraction"] == 0,
                    "reused company ID retained finances",
                )
                require(
                    "COMPANY-START 1" in log,
                    "AI VM startup/stop/restart absent",
                )
    if operation == "finance":
        require("COMPANY-FINANCE-END" in log, "loan test/execute/repayment absent")
        require(
            after[1]["yearly_expenses"] != before[1]["yearly_expenses"]
            and after[1]["cur_economy[0]/expenses"] < 0
            and sum(after[1]["yearly_expenses"][6::13]) > 0
            and sum(after[1]["yearly_expenses"][11::13]) > 0
            and any(after[1]["yearly_expenses"][13:]),
            "maintenance/interest/year rotation absent",
        )
        require(
            after[1]["old_economy[0]/performance_history"] > 0
            and after[1]["block_preview"] < 2,
            "quarter scoring/preview rotation absent",
        )
        require(
            all(
                after[1][key] > 0
                for key in ("terraform_limit", "clear_limit", "tree_limit")
            ),
            "construction limits not refilled",
        )
        economy = rows(final, "ECMY")[0]
        require(
            economy["inflation_prices"] > 65536
            and economy["inflation_payment"] > 65536
            and economy["fluct"] != 1,
            "monthly inflation/recession absent",
        )
        require(
            after[0]["money_fraction"] != before[0]["money_fraction"],
            "fractional running costs absent",
        )
    if operation == "acquire":
        nested = [
            line for line in run["log"] if line.startswith("COMPANY-CONTROL nested ")
        ]
        require(
            bool(nested), "company acquisition nested service-interval arguments absent"
        )
        result[f"{mode}_{role}_service_commands"] = nested
    if operation == "reload":
        require(
            set(after) == {0, 1, 2}
            and after[1]["money_fraction"] == before[1]["money_fraction"],
            "live company reload absent",
        )
    result[f"{mode}_{role}_companies"] = {
        i: {
            key: row[key]
            for key in (
                "money",
                "money_fraction",
                "current_loan",
                "months_of_bankruptcy",
                "bankrupt_asked",
                "bankrupt_timeout",
            )
        }
        for i, row in after.items()
    }


if __name__ == "__main__":
    command_gap()
