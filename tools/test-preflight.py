#!/usr/bin/env python3
"""Check preflight's reading of Quick and the pinned commit checker checkout."""

import contextlib
import importlib.util
import io
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from collections.abc import Generator
from contextlib import contextmanager, nullcontext
from functools import partial
from pathlib import Path
from subprocess import CompletedProcess
from types import ModuleType, SimpleNamespace
from typing import cast
from unittest.mock import Mock, patch

import git_hooks
import preflight
import preflight_stamp
import quick_workflow
import vcs
import yaml

REPO = Path(__file__).resolve().parents[1]


def workflow(*steps: quick_workflow.Step) -> quick_workflow.Workflow:
    return {"jobs": {"quick": {"steps": list(steps)}}}


def fixture_commit(root: Path, title: str) -> str:
    """Commit everything in a scratch repository, creating it if needed."""
    if not (root / ".git").exists():
        vcs.git("init", "-q", cwd=root)
    vcs.git("add", "-A", cwd=root)
    vcs.git(
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@example.invalid",
        "commit",
        "-qm",
        title,
        cwd=root,
    )
    return vcs.git("rev-parse", "HEAD", cwd=root).strip()


class WorkflowTests(unittest.TestCase):
    def test_every_quick_run_step_is_run_or_skipped(self) -> None:
        # Every job's steps, so preflight still runs each gate after the split (#1063).
        quick = yaml.safe_load(preflight.WORKFLOW.read_text())
        names = {
            step.get("name", "")
            for job in quick["jobs"].values()
            for step in job["steps"]
            if "run" in step
        }
        ran = [step.name for step in preflight.workflow_steps(quick, "base")]
        self.assertEqual(len(ran), len(set(ran)), "a step runs twice")
        self.assertEqual(set(ran) | set(preflight.SKIPPED), names, "unlisted step")
        self.assertFalse(set(ran) & set(preflight.SKIPPED))
        self.assertIn("Python complexity debt and lint policy", ran)

    def test_workflow_steps_read_every_job_in_file_order(self) -> None:
        """The Rust job's steps first, the reporting job's step skipped (#1063)."""
        quick: quick_workflow.Workflow = {
            "jobs": {
                "rust": {"steps": [{"name": "a", "run": "true"}]},
                "python": {"steps": [{"uses": "x"}, {"name": "b", "run": "true"}]},
                "quick": {
                    "needs": ["rust", "python"],
                    "steps": [
                        {
                            "name": "Require every Quick job to succeed",
                            "if": "needs.rust.result != 'success'",
                            "env": {"RESULTS": "${{ needs.rust.result }}"},
                            "run": "exit 1",
                        }
                    ],
                },
            }
        }
        ran = [step.name for step in preflight.workflow_steps(quick, "base")]
        self.assertEqual(ran, ["a", "b"])
        real = yaml.safe_load(preflight.WORKFLOW.read_text())
        ran = [step.name for step in preflight.workflow_steps(real, "base")]
        self.assertLess(
            ran.index("Rust formatting, check, warnings and tests"),
            ran.index("Python formatting, lint and maintenance regression checks"),
        )

    def test_a_pending_suppression_passes_quicks_lint_policy_step(self) -> None:
        """Preflight runs Quick's lint-policy line with no event set, so a
        suppression awaiting its acceptance passes and the tree can stamp; the
        merge group's event fails it (#1013)."""
        quick = yaml.safe_load(preflight.WORKFLOW.read_text())
        (step,) = [
            step
            for step in preflight.workflow_steps(quick, "base")
            if "tools/lint-policy.py" in step.command[-1]
        ]
        (line,) = [
            line for line in step.command[-1].splitlines() if "lint-policy.py" in line
        ]
        source = "pub struct Native;\n"
        allowed = (
            "#[allow(non_upper_case_globals)]\nimpl Native { pub const N: u8 = 1; }\n"
        )
        site = "rust/x.rs | impl Native | allow non_upper_case_globals"
        record = {
            "kind": "suppression",
            "tool": "rust",
            "site": site,
            "before": 0,
            "after": 1,
            "operation": "Native::N",
            "source_contract": "original spelling",
            "reason": "keep the original's name",
            "retiring_owner": "owner",
            "retirement_condition": "when the API goes",
        }
        modules = (
            "lint-policy",
            "cleanup_contract",
            "code_lines",
            "rust_syntax",
            "vcs",
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "tools").mkdir()
            for name in modules:
                shutil.copy(REPO / "tools" / f"{name}.py", root / "tools")
            (root / "rust").mkdir()
            (root / "rust/x.rs").write_text(source)
            commit = partial(fixture_commit, root)
            base = commit("Add: fixture")
            (root / "rust/x.rs").write_text(source + allowed)
            records = root / "migration/cleanup-records/t3-x.json"
            records.parent.mkdir(parents=True)
            records.write_text(json.dumps([record]))
            commit("Add: pending suppression")
            # No GitHub: an unreadable discussion leaves the acceptance unchecked.
            stubs = root / "stubs"
            stubs.mkdir()
            (stubs / "gh").write_text("#!/bin/sh\nexit 1\n")
            (stubs / "gh").chmod(0o755)
            env = preflight.environment() | step.env | {"BASE_REF": base}
            env.pop("GITHUB_EVENT_NAME", None)
            env.pop("GITHUB_EVENT_PATH", None)
            env["PATH"] = f"{stubs}{os.pathsep}{env['PATH']}"
            for event, code in ((None, 0), ("merge_group", 1)):
                if event:
                    env["GITHUB_EVENT_NAME"] = event
                done = subprocess.run(
                    [*preflight.SHELL, line],
                    cwd=root,
                    env=env,
                    capture_output=True,
                    text=True,
                )
                with self.subTest(event=event):
                    self.assertEqual(done.returncode, code, done.stdout + done.stderr)
                    shown = "pending review" if code == 0 else "lacks its reviewer"
                    self.assertIn(shown, done.stdout)

    def test_a_new_run_step_runs_without_a_list_entry(self) -> None:
        steps = preflight.workflow_steps(
            workflow({"uses": "actions/checkout@v6"}, {"name": "New", "run": "true"}),
            "base",
        )
        self.assertEqual([step.name for step in steps], ["New"])
        self.assertEqual(steps[0].command[-1], "true")

    def test_base_ref_bound_and_tokens_left_local(self) -> None:
        (step,) = preflight.workflow_steps(
            workflow(
                {
                    "name": "Gate",
                    "run": 'gate --base-ref "$BASE_REF"',
                    "env": {
                        "BASE_REF": "${{ inputs.base-ref || github.event.before }}",
                        "GH_TOKEN": "${{ github.token }}",
                        "GITHUB_TOKEN": "${{ github.token }}",
                        "PACKAGES": "valgrind",
                    },
                }
            ),
            "abc123",
        )
        self.assertEqual(step.env, {"BASE_REF": "abc123", "PACKAGES": "valgrind"})

    def test_unknown_expressions_and_conditions_stop_naming_the_step(self) -> None:
        for entry in (
            {"name": "Gate", "run": "echo ${{ github.sha }}"},
            {"name": "Gate", "run": "true", "env": {"X": "${{ github.sha }}"}},
            {"name": "Gate", "run": "true", "env": {"GH_TOKEN": "${{ secrets.X }}"}},
            {"name": "Gate", "run": "true", "env": {"BASE_REF": "${{ github.sha }}"}},
            {
                "name": "Gate",
                "run": "true",
                "env": {"BASE_REF": "${{ inputs.base-ref || secrets.X }}"},
            },
            {"name": "Gate", "run": "true", "env": {"BASE_REF": "${{ x }}${{ y }}"}},
            {"name": "Gate", "run": "true", "if": "github.event_name == 'push'"},
        ):
            with self.assertRaisesRegex(ValueError, "step 'Gate'"):
                preflight.workflow_steps(
                    workflow(cast(quick_workflow.Step, entry)), "base"
                )

    def test_step_selection(self) -> None:
        names = ["Rust checks", "Ruff ratchet", "Strings"]
        self.assertEqual(preflight.selected(names, None), names)
        self.assertEqual(
            preflight.selected(names, ["ruff", "STRINGS"]), ["Ruff ratchet", "Strings"]
        )
        with self.assertRaisesRegex(ValueError, "no step matches 'nothing'"):
            preflight.selected(names, ["nothing"])


STEPS = ["--step", "Short", "--step", "Fails", "--step", "Long"]


def fake_popen() -> tuple[type, list[list[str]]]:
    """A Popen stand-in whose step fails when its command ends in `false`,
    and the list of commands it was started with."""
    calls: list[list[str]] = []

    class Popen:
        def __init__(self, command: list[str], **_: object) -> None:
            calls.append(command)
            self.pid = -1  # signal_group never signals it
            self.returncode = 1 if command[-1] == "false" else 0

        def wait(self, _timeout: float | None = None) -> int:
            return self.returncode

    return Popen, calls


def steps_of(*pairs: tuple[str, str]) -> list[preflight.Step]:
    return [preflight.Step(name, [command]) for name, command in pairs]


class RunTests(unittest.TestCase):
    def run_steps(self, argv: list[str], timings: dict[str, float]) -> tuple:
        """main over three steps, the middle one failing, with these recorded
        timings: its code, commands, printed lines, saved timings and logs."""
        quick = workflow(
            {"name": "Short", "run": "true"},
            {"name": "Fails", "run": "false"},
            {"name": "Long", "run": "true"},
        )
        popen, calls = fake_popen()
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(preflight, "hooks_checkout", return_value=Path("/hooks")),
            patch.object(preflight, "git", return_value="sha"),
            patch.object(preflight.yaml, "safe_load", return_value=quick),
            patch.object(preflight.migration, "LOCAL", Path(directory)),
            patch.object(preflight.migration, "COMMON_LOCAL", Path(directory)),
            patch.object(preflight.subprocess, "Popen", popen),
            patch.object(sys, "argv", ["preflight.py", *STEPS, *argv]),
            patch("builtins.print") as printed,
        ):
            (Path(directory) / preflight.TIMINGS).parent.mkdir(parents=True)
            (Path(directory) / preflight.TIMINGS).write_text(json.dumps(timings))
            code = preflight.main()
            saved = json.loads((Path(directory) / preflight.TIMINGS).read_text())
            logs = sorted(p.name for p in Path(directory).glob("preflight/2*/*.log"))
        lines = "\n".join(str(call.args[0]) for call in printed.call_args_list)
        commands = [command[-1] for command in calls]
        return code, commands, lines, saved, logs

    def test_steps_run_longest_first_and_report_in_step_order(self) -> None:
        timings = {"Short": 1.0, "Fails": 5.0, "Long": 90.0, "Gone": 3.0}
        code, commands, lines, saved, logs = self.run_steps(["--jobs", "1"], timings)
        self.assertEqual(code, 1)
        self.assertEqual(commands, ["true", "false", "true"])  # workflow order
        code, commands, lines, saved, logs = self.run_steps(["--jobs", "2"], timings)
        self.assertEqual(code, 1)
        self.assertEqual(sorted(commands), ["false", "true", "true"])
        self.assertEqual(logs, ["01.log", "02.log", "03.log"])
        self.assertIn("failed: Fails;", lines)
        self.assertEqual(set(saved), {"Short", "Fails", "Long", "Gone"})
        self.assertLess(saved["Long"], 90)  # this run's time replaces the last
        # A step's result is printed whole, after its start line.
        self.assertRegex(lines, r"--> Fails\n(.*\n)*    exit 1; full output: .*02\.log")

    def test_steps_overlap_up_to_jobs(self) -> None:
        """Two steps each wait for the other: only a run of both at once ends."""
        for jobs, overlaps in ((2, True), (1, False)):
            meeting = threading.Barrier(2, timeout=1)

            def meet(*_: object, meeting: threading.Barrier = meeting) -> tuple:
                meeting.wait()
                return True, 0.0

            with (
                tempfile.TemporaryDirectory() as directory,
                patch.object(preflight.migration, "COMMON_LOCAL", Path(directory)),
                patch.object(preflight, "timed", side_effect=meet),
            ):
                steps = steps_of(("a", "true"), ("b", "true"))
                if overlaps:
                    self.assertEqual(
                        preflight.run_steps(steps, {}, Path(), jobs).failed, []
                    )
                else:
                    with self.assertRaises(threading.BrokenBarrierError):
                        preflight.run_steps(steps, {}, Path(), jobs)

    def test_a_verdict_belongs_to_its_step_not_its_name(self) -> None:
        """Steps that share a name, or start out of order, keep their own result."""
        popen, _ = fake_popen()
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(preflight.migration, "COMMON_LOCAL", Path(directory)),
            patch.object(preflight.subprocess, "Popen", popen),
            patch("builtins.print"),
        ):
            preflight.save_timings({"c": 9.0, "a": 1.0})
            for jobs in (1, 2):
                same = steps_of(("Same", "false"), ("Same", "true"))
                self.assertEqual(
                    preflight.run_steps(same, {}, Path(directory), jobs).failed,
                    ["Same"],
                )
                three = steps_of(("a", "false"), ("b", "true"), ("c", "false"))
                self.assertEqual(
                    preflight.run_steps(three, {}, Path(directory), jobs).failed,
                    ["a", "c"],
                )

    def test_an_interrupt_or_stop_signal_stops_every_running_step(self) -> None:
        """SIGINT or SIGTERM to a parallel run ends it promptly and leaves no
        step running, even steps that ignore SIGINT and have children."""
        for number in (signal.SIGINT, signal.SIGTERM):
            with tempfile.TemporaryDirectory() as directory:
                self.stop_run(Path(directory), number)

    def stop_run(self, root: Path, number: int) -> None:
        step = "trap '' INT; sleep 60 & echo $! > {}; touch {}; wait"
        script = (
            "import sys\nfrom pathlib import Path\n"
            f"sys.path.insert(0, {str(REPO / 'tools')!r})\n"
            "import preflight\n"
            f"root = Path({str(root)!r})\n"
            "preflight.migration.LOCAL = preflight.migration.COMMON_LOCAL = root\n"
            f"step = {step!r}\n"
            "steps = [preflight.Step(n, ['sh', '-c', step.format("
            "root / f'{n}.pid', root / f'{n}.started')]) for n in ('zzq1', 'zzq2')]\n"
            "preflight.workflow_steps = lambda *_: list(steps)\n"
            "sys.argv = ['preflight.py', '--step', 'zzq', '--jobs', '2']\n"
            "sys.exit(preflight.main())\n"
        )
        child = subprocess.Popen(
            [sys.executable, "-c", script],
            cwd=REPO,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        deadline = time.monotonic() + 30
        while not all((root / f"zzq{n}.started").exists() for n in "12"):
            self.assertLess(time.monotonic(), deadline, "steps did not start")
            self.assertIsNone(child.poll(), "preflight exited before its steps")
            time.sleep(0.05)
        started = time.monotonic()
        child.send_signal(number)
        self.assertEqual(child.wait(20), 130)
        self.assertLess(time.monotonic() - started, preflight.STOP_SECONDS + 3)
        for name in "12":
            pid = int((root / f"zzq{name}.pid").read_text())
            with self.assertRaises(ProcessLookupError):
                os.kill(pid, 0)

    def test_a_stop_during_a_launch_still_stops_that_step(self) -> None:
        """A SIGTERM that arrives after a step's process exists but before
        preflight has registered it, serial and parallel."""
        for jobs in ("1", "2"):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                script = (
                    "import os, signal, sys, time\nfrom pathlib import Path\n"
                    f"sys.path.insert(0, {str(REPO / 'tools')!r})\n"
                    "import preflight\n"
                    f"root = Path({directory!r})\n"
                    "preflight.migration.LOCAL = root\n"
                    "preflight.migration.COMMON_LOCAL = root\n"
                    "real = preflight.subprocess.Popen\n"
                    "def late(*args, **kwargs):\n"
                    "    process = real(*args, **kwargs)\n"
                    "    if args[0] != ['sleep', '60']:\n"
                    "        return process\n"
                    "    (root / 'pid').write_text(str(process.pid))\n"
                    "    os.kill(os.getpid(), signal.SIGTERM)\n"
                    "    time.sleep(0.5)\n"
                    "    return process\n"
                    "preflight.subprocess.Popen = late\n"
                    "steps = [preflight.Step('zzq', ['sleep', '60'])]\n"
                    "preflight.workflow_steps = lambda *_: list(steps)\n"
                    f"sys.argv = ['preflight.py', '--step', 'zzq', '--jobs', {jobs!r}]\n"
                    "sys.exit(preflight.main())\n"
                )
                child = subprocess.run(
                    [sys.executable, "-c", script],
                    cwd=REPO,
                    capture_output=True,
                    timeout=30,
                    check=False,
                )
                self.assertEqual(child.returncode, 130, child.stderr)
                self.assertIn(b"--> zzq", child.stdout)  # the stop came in the launch
                with self.assertRaises(ProcessLookupError):
                    os.kill(int((root / "pid").read_text()), 0)

    def test_concurrent_timing_saves_keep_every_step(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(preflight.migration, "COMMON_LOCAL", Path(directory)),
        ):

            def save(prefix: str) -> None:
                for number in range(40):
                    preflight.save_timings({f"{prefix}{number}": 1.0})

            writers = [threading.Thread(target=save, args=(p,)) for p in "xy"]
            for writer in writers:
                writer.start()
            for writer in writers:
                writer.join()
            self.assertEqual(len(preflight.last_timings()), 80)

    def test_full_runs_are_logged_and_subsets_are_not(self) -> None:
        quick = workflow({"name": "Only", "run": "true"})
        popen, _ = fake_popen()
        with (
            tempfile.TemporaryDirectory() as local,
            tempfile.TemporaryDirectory() as common,
            patch.object(preflight, "hooks_checkout", return_value=Path("/hooks")),
            patch.object(preflight, "git", return_value="sha"),
            patch.object(preflight, "uncommitted", return_value=""),
            patch.object(preflight, "stamp", return_value="stamp: test"),
            patch.object(preflight.yaml, "safe_load", return_value=quick),
            patch.object(preflight.migration, "LOCAL", Path(local)),
            patch.object(preflight.migration, "COMMON_LOCAL", Path(common)),
            patch.object(preflight.subprocess, "Popen", popen),
            patch("builtins.print"),
        ):
            # Serial first: it saves no timings, so only record_run makes the folder.
            for argv in (["--jobs", "1"], [], ["--step", "Only"]):
                with patch.object(sys, "argv", ["preflight.py", *argv]):
                    self.assertEqual(preflight.main(), 0)
            runs, unreadable = preflight.read_runs(Path(common) / preflight.RUNS)
        self.assertEqual([run["jobs"] for run in runs], [1, preflight.JOBS])
        self.assertEqual(unreadable, 0)

    def test_runs_are_counted_since_a_utc_time(self) -> None:
        rows = [
            ("2026-11-25T08:00:00Z", "a", 900, []),
            ("2026-11-25T11:00:00Z", "a", 200, []),
            ("2026-11-25T12:00:00Z", "a", 100, ["Gate"]),
            ("2026-11-25T13:00:00Z", "b", 480, []),
        ]
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(preflight.migration, "COMMON_LOCAL", Path(directory)),
            patch("builtins.print") as printed,
        ):
            log = Path(directory) / preflight.RUNS
            log.parent.mkdir()
            log.write_bytes(
                "".join(
                    json.dumps(
                        {"at": a, "head": h, "jobs": 4, "seconds": s, "failed": f}
                    )
                    + "\n"
                    for a, h, s, f in rows
                ).encode()
                + b'{"at": "2026-11-25T11:30:00Z", "seconds": 5}\n'
                + b"\xff\xfe not a run\n"
                + b'{"at": "2026-11-25T14:00'
            )
            # A run appended after a truncated line still counts.
            stamped = "2026-11-25T15:00:00Z"
            with patch.object(preflight.time, "strftime", return_value=stamped):
                preflight.record_run("c", 4, 300.0, [])
            self.assertEqual(preflight.report_runs("2026-11-25 10:00:00"), 0)
            self.assertEqual(preflight.report_runs("2999-01-01"), 0)
            self.assertEqual(preflight.report_runs("yesterday"), 2)
        lines = [str(call.args[0]) for call in printed.call_args_list]
        self.assertEqual(
            lines[:4],
            [
                "skipped 3 unreadable lines in preflight/runs.jsonl",
                "4 full runs since 2026-11-25 10:00:00 on 3 heads, 1 failed; "
                "median 250s, longest 480s, 18 minutes in all",
                "skipped 3 unreadable lines in preflight/runs.jsonl",
                "no full runs since 2999-01-01",
            ],
        )

    def test_the_longest_step_of_the_last_run_starts_first(self) -> None:
        started: list[str] = []
        steps = [preflight.Step(name, [name]) for name in ("a", "b", "c", "d")]

        def record(step: preflight.Step, *_: object) -> tuple[bool, float]:
            started.append(step.name)
            return True, 1.0

        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(preflight.migration, "COMMON_LOCAL", Path(directory)),
            patch.object(preflight, "timed", side_effect=record),
        ):
            preflight.save_timings({"a": 1.0, "b": 30.0, "c": 2.0})
            verdicts = preflight.run_steps(steps, {}, Path(directory), 1)
        # d has no time, so it may be the longest; then b, c and a.
        self.assertEqual(started, ["d", "b", "c", "a"])
        self.assertEqual(verdicts, ([], []))

    def test_the_rust_checks_start_first_and_their_failure_stops_the_rest(
        self,
    ) -> None:
        """A failure of the Rust checks cancels the queued steps and terminates
        the running ones, which keep their last times; --keep-going runs all.
        With two jobs the Rust checks fail only once the slow step runs."""
        rust = preflight.RUST_CHECKS
        for jobs, keep_going in ((1, False), (2, False), (2, True)):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                slow = "true" if keep_going else f"touch {root}/slow; sleep 60"
                wait = f"until [ -e {root}/slow ]; do sleep 0.05; done; "
                steps = [
                    preflight.Step("Slow", ["sh", "-c", slow]),
                    preflight.Step(rust, ["sh", "-c", (wait * (jobs == 2)) + "false"]),
                    preflight.Step("Quick", ["true"]),
                ]
                with (
                    patch.object(preflight.migration, "COMMON_LOCAL", root),
                    patch("builtins.print"),
                ):
                    preflight.save_timings({"Slow": 90.0, "Quick": 5.0})
                    if keep_going:
                        (root / "slow").touch()
                    began = time.monotonic()
                    verdicts = preflight.run_steps(steps, {}, root, jobs, keep_going)
                    took = time.monotonic() - began
                    saved = preflight.last_timings()
                self.assertEqual(verdicts.failed, [rust])
                if keep_going:
                    self.assertEqual(verdicts.stopped, [])
                    self.assertLess(saved["Slow"], 90.0)
                    continue
                self.assertEqual(verdicts.stopped, ["Slow", "Quick"])
                self.assertLess(took, preflight.STOP_SECONDS + 5)
                # The stopped steps keep their last full times.
                self.assertEqual((saved["Slow"], saved["Quick"]), (90.0, 5.0))
                self.assertIn(rust, saved)

    def test_a_stopped_run_lists_the_stopped_steps_and_records_no_stamp(
        self,
    ) -> None:
        rust = preflight.RUST_CHECKS
        quick = workflow(
            {"name": rust, "run": "false"}, {"name": "Second", "run": "true"}
        )
        # The commit checker and the Rust checks run; the other two only
        # with --keep-going.
        for extra, commands_run in (([], 2), (["--keep-going"], 4)):
            popen, commands = fake_popen()
            with (
                tempfile.TemporaryDirectory() as directory,
                patch.object(preflight, "hooks_checkout", return_value=Path("/hooks")),
                patch.object(preflight, "git", return_value="sha"),
                patch.object(preflight, "stamp") as stamp,
                patch.object(preflight.yaml, "safe_load", return_value=quick),
                patch.object(preflight.migration, "LOCAL", Path(directory)),
                patch.object(preflight.migration, "COMMON_LOCAL", Path(directory)),
                patch.object(preflight.subprocess, "Popen", popen),
                patch.object(sys, "argv", ["preflight.py", "--jobs", "1", *extra]),
                patch("builtins.print") as printed,
            ):
                self.assertEqual(preflight.main(), 1)
                runs, _ = preflight.read_runs(Path(directory) / preflight.RUNS)
            stamp.assert_not_called()
            self.assertEqual(len(commands), commands_run)
            self.assertEqual([run["failed"] for run in runs], [[rust]])
            lines = "\n".join(str(call.args[0]) for call in printed.call_args_list)
            if extra:
                self.assertNotIn("stopped", lines)
            else:
                self.assertIn("1/4 passed in", lines)
                self.assertIn("stopped: Second\n", lines)
                self.assertIn(f"stopped: {preflight.PROBES}", lines)

    def test_the_rust_checks_are_a_step_of_quick(self) -> None:
        quick = preflight.yaml.safe_load(preflight.WORKFLOW.read_text())
        names = [step.name for step in preflight.workflow_steps(quick, "base")]
        self.assertIn(preflight.RUST_CHECKS, names)

    def test_every_step_runs_and_any_failure_fails(self) -> None:
        quick = workflow(
            {"name": "First", "run": "false"}, {"name": "Second", "run": "true"}
        )
        popen, commands = fake_popen()
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(preflight, "hooks_checkout", return_value=Path("/hooks")),
            patch.object(preflight, "git", return_value="sha"),
            patch.object(preflight.yaml, "safe_load", return_value=quick),
            patch.object(preflight.migration, "LOCAL", Path(directory)),
            patch.object(preflight.migration, "COMMON_LOCAL", Path(directory)),
            patch.object(preflight.subprocess, "Popen", popen),
            patch.object(sys, "argv", ["preflight.py"]),
            patch("builtins.print"),
        ):
            self.assertEqual(preflight.main(), 1)
            self.assertEqual(len(commands), 4)
            self.assertIn(["sh", "/hooks/hooks/check-commits.sh", "sha..sha"], commands)
            self.assertEqual(
                sorted(command[-1] for command in commands),
                ["false", "sha", "sha..sha", "true"],
            )
            self.assertIn(
                ["python3", "tools/compile-check.py", "--base", "sha"], commands
            )

    def test_selected_steps_skip_the_commit_checker(self) -> None:
        quick = workflow({"name": "Only", "run": "true"})
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(preflight, "hooks_checkout") as hooks,
            patch.object(preflight, "git", return_value="sha"),
            patch.object(preflight.yaml, "safe_load", return_value=quick),
            patch.object(preflight.migration, "LOCAL", Path(directory)),
            patch.object(preflight.migration, "COMMON_LOCAL", Path(directory)),
            patch.object(sys, "argv", ["preflight.py", "--step", "only"]),
            patch("builtins.print"),
        ):
            self.assertEqual(preflight.main(), 0)
            hooks.assert_not_called()

    def test_changed_tracked_files_warn_at_start_and_in_summary(self) -> None:
        quick = workflow({"name": "Only", "run": "true"})
        changed = " M tools/preflight.py"
        for statuses, warnings in (
            (["", ""], 0),
            ([changed, changed], 2),
            (["", changed], 1),
        ):

            def git(
                _repo: Path, *arguments: str, statuses: list[str] = statuses
            ) -> str:
                return statuses.pop(0) if arguments[0] == "status" else "sha"

            with (
                tempfile.TemporaryDirectory() as directory,
                patch.object(preflight, "git", side_effect=git),
                patch.object(preflight.yaml, "safe_load", return_value=quick),
                patch.object(preflight.migration, "LOCAL", Path(directory)),
                patch.object(preflight.migration, "COMMON_LOCAL", Path(directory)),
                patch.object(preflight.migration, "COMMON_LOCAL", Path(directory)),
                patch.object(sys, "argv", ["preflight.py", "--step", "only"]),
                patch("builtins.print") as printed,
            ):
                self.assertEqual(preflight.main(), 0)
                lines = [str(call.args[0]) for call in printed.call_args_list]
                self.assertEqual(
                    sum("not the commit CI tests" in line for line in lines), warnings
                )
                if warnings:
                    self.assertIn("not the commit CI tests", lines[-1])

    def test_supplied_checker_must_match_pin_and_be_clean(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch.object(preflight.migration, "COMMON_LOCAL", root):
                with patch.object(preflight, "git", side_effect=["bad revision"]):
                    with self.assertRaisesRegex(ValueError, "pinned revision"):
                        preflight.hooks_checkout(root)
                with patch.object(
                    preflight,
                    "git",
                    side_effect=[preflight.HOOKS_REVISION, " M hooks/check-diff.py"],
                ):
                    with self.assertRaisesRegex(ValueError, "dirty"):
                        preflight.hooks_checkout(root)


def tool(name: str) -> ModuleType:
    """Import a hyphenated tool script."""
    spec = importlib.util.spec_from_file_location(
        name.replace("-", "_"), REPO / f"tools/{name}.py"
    )
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def commit(repo: Path, title: str, files: dict[str, str]) -> str:
    for name, text in files.items():
        (repo / name).parent.mkdir(parents=True, exist_ok=True)
        (repo / name).write_text(text)
    subprocess.run(["git", "add", "-A"], cwd=repo, check=True)
    subprocess.run(["git", "commit", "-qm", title], cwd=repo, check=True)
    return preflight_stamp.git("rev-parse", "HEAD")


@contextmanager
def scratch_repo() -> Generator[Path]:
    """A repository whose base commit origin/rust-migration names; ROOT points at it."""
    with tempfile.TemporaryDirectory() as directory:
        repo = Path(directory)
        identity = {
            "GIT_AUTHOR_NAME": "Seed",
            "GIT_AUTHOR_EMAIL": "seed@example.invalid",
            "GIT_COMMITTER_NAME": "Seed",
            "GIT_COMMITTER_EMAIL": "seed@example.invalid",
        }
        subprocess.run(["git", "init", "-q", "-b", "work", str(repo)], check=True)
        (repo / ".git/info/exclude").write_text(".local/\n")
        with (
            patch.dict(os.environ, identity),
            patch.object(preflight_stamp, "ROOT", repo),
            patch.object(preflight.migration, "ROOT", repo),
        ):
            base = commit(repo, "Add: Base", {"src/a.cpp": "int a;\n"})
            preflight_stamp.git(
                "update-ref", "refs/remotes/origin/rust-migration", base
            )
            yield repo


def seeded_push(repo: Path) -> str:
    """One commit carrying each fast-tier failure #920 names."""
    toolchain = (REPO / "rust-toolchain.toml").read_text()
    return commit(
        repo,
        "Merge: Seeded failures",
        {
            "rust-toolchain.toml": toolchain,
            "Cargo.toml": '[package]\nname = "seed"\nversion = "0.1.0"\n',
            "src/lib.rs": "pub fn f( ) {}\n",
            "tools/seed.py": "import os\nx=1\n",
            "rust/oracle.txt": "row \n",
        },
    )


class StampTests(unittest.TestCase):
    def test_built_docs_match_cmake_and_the_workflows(self) -> None:
        root = REPO
        sources = [root / "CMakeLists.txt", *sorted((root / "cmake").glob("*.cmake"))]
        built = {
            match
            for path in sources
            for match in re.findall(
                r"\$\{CMAKE_SOURCE_DIR\}/docs/([^\s\"@)]+)", path.read_text()
            )
        }
        # Doxygen's docs target takes each `@image` a source names from IMAGE_PATH.
        self.assertIn(
            "IMAGE_PATH             = ./docs/\n", (root / "Doxyfile.in").read_text()
        )
        images = subprocess.run(
            [
                "git",
                "grep",
                "-h",
                "-o",
                "-I",
                "-E",
                r"[@\\]image[[:space:]]+.*",
                "--",
                "src",
            ],
            cwd=root,
            text=True,
            capture_output=True,
            check=True,
        ).stdout
        named = {line.split()[2].strip('"') for line in images.splitlines()}
        self.assertTrue(named)
        for name in named:
            self.assertTrue((root / "docs" / name).is_file(), name)
        self.assertEqual(set(preflight_stamp.BUILT_DOCS), built | named)
        plan = (root / ".github/workflows/ci-request.yml").read_text()
        listed = re.search(r"const builtDocs = \[(.*?)\];", plan, re.S)
        assert listed
        self.assertEqual(
            re.findall(r"'([^']+)'", listed[1]), list(preflight_stamp.BUILT_DOCS)
        )

    def test_a_stamp_covers_its_tree_and_docs_only_differences(self) -> None:
        with scratch_repo() as repo:
            code = commit(repo, "Add: Code", {"tools/x.py": "x = 1\n"})
            preflight_stamp.record("base", code)
            docs = commit(
                repo,
                "Doc: Note",
                {
                    "docs/note.md": "note\n",
                    "AGENTS.md": "a\n",
                    "docs/skills/x/agents/openai.yaml": "interface: {}\n",
                    ".github/PULL_REQUEST_TEMPLATE.md": "## Checks\n",
                    ".github/ISSUE_TEMPLATE/bug.yaml": "name: Bug\n",
                    ".codex/config.toml": 'model = "m"\n',
                    # Git quotes this name in line output; the stamp reads it raw.
                    "docs/skills/caf\u00e9/agents/openai.yaml": "interface: {}\n",
                    # A name that is not UTF-8, as Git and this filesystem allow.
                    os.fsdecode(b"docs/caf\xe9.yaml"): "interface: {}\n",
                },
            )
            self.assertEqual((preflight_stamp.stamp_for(docs) or {}).get("head"), code)
            for name, text in (
                ("docs/fonts.md", "built\n"),
                ("docs/obg_format.txt", "built\n"),
                ("docs/openttd.6", "built\n"),
                ("docs/elrail_tile.png", "built\n"),
                ("docs/skills/x/issues.py", "x = 1\n"),
                ("docs/skills/x/run.sh", "true\n"),
                (".github/workflows/x.yml", "on: push\n"),
                (".github/dependabot.yml", "version: 2\n"),
            ):
                with self.subTest(name):
                    read = commit(
                        repo, "Change: Read by a build or check", {name: text}
                    )
                    self.assertIsNone(preflight_stamp.stamp_for(read))
                    subprocess.run(
                        ["git", "reset", "-q", "--hard", docs], cwd=repo, check=True
                    )
            tooling = commit(repo, "Change: Tool", {"tools/x.py": "x = 2\n"})
            self.assertIsNone(preflight_stamp.stamp_for(tooling))

    def test_only_a_complete_clean_run_on_a_protected_base_stamps(self) -> None:
        with scratch_repo() as repo:
            base = preflight_stamp.git("rev-parse", "HEAD")
            head = commit(repo, "Add: Work", {"tools/y.py": "y = 1\n"})
            options = preflight.argparse.Namespace(step=None)
            for step, warning, checked, against, expected in (
                (["Rust"], "", head, base, "--step ran a subset"),
                (None, "changed", head, base, "tracked files differ"),
                (None, "", base, base, "not the checked-out commit"),
                (None, "", head, head, "is not on origin/rust-migration"),
                (None, "", head, base, "stamp: recorded"),
            ):
                with self.subTest(expected=expected):
                    options.step = step
                    message = preflight.stamp(options, against, checked, warning)
                    self.assertIn(expected, message)
            self.assertEqual((preflight_stamp.stamp_for(head) or {}).get("base"), base)

    def test_a_tree_whose_only_open_item_is_a_pending_suppression_stamps(
        self,
    ) -> None:
        """preflight.main runs Quick's real lint-policy line, with the commit
        checker and compile probes stubbed: a suppression awaiting its
        acceptance passes and the tree stamps; the merge group's event fails it
        and records no stamp (#1013)."""
        for event, code in (("", 0), ("merge_group", 1)):
            with self.subTest(event=event), scratch_repo() as repo:
                head = pending_suppression(repo)
                self.assertEqual(self.preflight(repo, event), code)
                stamped = preflight_stamp.stamp_for(head)
                self.assertEqual(stamped is not None, code == 0)

    def preflight(self, repo: Path, event: str) -> int:
        quick = yaml.safe_load(preflight.WORKFLOW.read_text())
        (entry,) = [
            quick_workflow.Step(**step)
            for step in quick_workflow.steps(quick)
            if "tools/lint-policy.py" in step.get("run", "")
        ]
        entry["run"] = next(
            line for line in entry["run"].splitlines() if "lint-policy.py" in line
        )
        stubs = repo / ".local/stubs"
        stubs.mkdir(parents=True)
        (stubs / "gh").write_text("#!/bin/sh\nexit 1\n")
        (stubs / "gh").chmod(0o755)
        env = {
            key: value
            for key, value in preflight.environment().items()
            if key not in ("GITHUB_EVENT_NAME", "GITHUB_EVENT_PATH")
        }
        env["PATH"] = f"{stubs}{os.pathsep}{env['PATH']}"
        if event:
            env["GITHUB_EVENT_NAME"] = event
        passing = preflight.Step(preflight.COMMITS, ["true"])
        with (
            patch.object(preflight.yaml, "safe_load", return_value=workflow(entry)),
            patch.object(preflight, "hooks_checkout", return_value=Path("/hooks")),
            patch.object(preflight, "commit_step", return_value=passing),
            patch.object(preflight, "environment", return_value=env),
            patch.object(preflight.migration, "LOCAL", repo / ".local"),
            patch.object(preflight.migration, "COMMON_LOCAL", repo / ".local"),
            patch.object(sys, "argv", ["preflight.py"]),
            patch("builtins.print"),
        ):
            return preflight.main()


def pending_suppression(repo: Path) -> str:
    """Commit the gate's modules as the protected base, then a Rust item
    suppression and its unaccepted record as the head."""
    modules = ("lint-policy", "cleanup_contract", "code_lines", "rust_syntax", "vcs")
    files = {
        f"tools/{name}.py": (REPO / "tools" / f"{name}.py").read_text()
        for name in modules
    }
    files["tools/compile-check.py"] = "pass\n"
    files["rust/x.rs"] = "pub struct Native;\n"
    base = commit(repo, "Add: Gate", files)
    preflight_stamp.git("update-ref", "refs/remotes/origin/rust-migration", base)
    site = "rust/x.rs | impl Native | allow non_upper_case_globals"
    record = {
        "kind": "suppression",
        "tool": "rust",
        "site": site,
        "before": 0,
        "after": 1,
        "operation": "Native::N",
        "source_contract": "original spelling",
        "reason": "keep the original's name",
        "retiring_owner": "owner",
        "retirement_condition": "when the API goes",
    }
    return commit(
        repo,
        "Add: Pending suppression",
        {
            "rust/x.rs": "pub struct Native;\n#[allow(non_upper_case_globals)]\n"
            "impl Native { pub const N: u8 = 1; }\n",
            "migration/cleanup-records/work.json": json.dumps([record]),
        },
    )


class BaseTests(unittest.TestCase):
    def test_a_moved_base_is_read_at_the_merge_base(self) -> None:
        with scratch_repo() as repo:
            fork = preflight_stamp.git("rev-parse", "HEAD")
            head = commit(repo, "Add: Work", {"tools/y.py": "y = 1\n"})
            preflight_stamp.git("checkout", "-q", "-b", "base", fork)
            tip = commit(repo, "Add: Later base", {"tools/z.py": "z = 1\n"})
            preflight_stamp.git("checkout", "-q", "work")
            preflight_stamp.git("update-ref", "refs/remotes/origin/rust-migration", tip)
            for given in ("origin/rust-migration", fork):
                with self.subTest(given=given), patch("builtins.print"):
                    options = preflight.argparse.Namespace(base=given, head="HEAD")
                    self.assertEqual(preflight.bases(options), (fork, head))
            run = partial(vcs.git, cwd=repo)
            self.assertEqual(vcs.merge_base(tip, run=run), fork)
            self.assertEqual(vcs.merge_base(fork, run=run), fork)
            preflight_stamp.git("checkout", "-q", "--orphan", "unrelated")
            commit(repo, "Add: Unrelated", {"a.txt": "a\n"})
            with self.assertRaises(SystemExit), patch("sys.stderr"):
                vcs.merge_base(tip, run=run)


class PushTests(unittest.TestCase):
    def push(self, repo: Path, sha: str, **environ: str) -> str:
        base = preflight.revision(preflight.PROTECTED)
        logs = repo / ".local/push"
        logs.mkdir(parents=True, exist_ok=True)
        with patch.dict(os.environ, environ), patch("builtins.print"):
            env = preflight.environment()
            return git_hooks.check_push("refs/heads/seed", sha, base, env, logs)

    def test_each_seeded_failure_is_refused_and_named(self) -> None:
        try:
            preflight.hooks_checkout()
        except (OSError, ValueError, subprocess.CalledProcessError) as error:
            self.skipTest(f"pinned commit checker unavailable: {error}")
        with scratch_repo() as repo:
            refusal = self.push(repo, seeded_push(repo), PREFLIGHT_CHECKPOINT="seed")
            # The control fixes each seed, so the tools ran and judged the content.
            clean = {"src/lib.rs": "pub fn f() {}\n", "tools/seed.py": "x = 1\n"}
            subprocess.run(["git", "reset", "-q", "HEAD~"], cwd=repo, check=True)
            (repo / "rust/oracle.txt").write_text("row\n")
            control = self.push(repo, commit(repo, "Add: Control", clean))
        self.assertIn("no preflight stamp", control)
        self.assertIn("fast tier failed", refusal)
        for name in (
            preflight.COMMITS,
            "cargo fmt --check",
            "ruff check",
            "ruff format",
        ):
            self.assertIn(name, refusal)

    def test_stamp_override_and_ready_pr_decide_a_passing_fast_tier(self) -> None:
        passing = [preflight.Step("Fast", ["true"])]
        with (
            scratch_repo() as repo,
            patch.object(git_hooks, "fast_tier", return_value=passing),
            patch.object(preflight_stamp, "common_local", return_value=repo / ".local"),
        ):
            docs = commit(repo, "Doc: Only", {"docs/x.md": "x\n"})
            self.assertEqual(self.push(repo, docs), "")
            code = commit(repo, "Add: Code", {"tools/z.py": "z = 1\n"})
            self.assertIn("no preflight stamp", self.push(repo, code))
            with patch.object(git_hooks, "ready_pr", return_value=920):
                refusal = self.push(repo, code, PREFLIGHT_CHECKPOINT="wip")
                self.assertIn("PR #920 is ready", refusal)
            with patch.object(git_hooks, "ready_pr", return_value=None):
                self.assertEqual(self.push(repo, code, PREFLIGHT_CHECKPOINT="wip"), "")
            (entry,) = (repo / ".local/overrides.log").read_text().splitlines()
            self.assertEqual(json.loads(entry)["reason"], "wip")
            preflight_stamp.record("base", code)
            self.assertEqual(self.push(repo, code), "")

    def hook_push(
        self, repo: Path, sha: str, ref: str, url: str, queue: object
    ) -> tuple[int, str, Mock, Mock]:
        """Run the production pre-push entry with the queue reader patched; queue
        is the BranchQueue it returns or the exception it raises."""
        reader = Mock(side_effect=queue) if isinstance(queue, Exception) else Mock()
        reader.return_value = queue
        fast = Mock(return_value=[preflight.Step("Fast", ["true"])])
        stdin = io.StringIO(f"refs/heads/work {sha} {ref} {git_hooks.ZERO}\n")
        output, errors = io.StringIO(), io.StringIO()
        with (
            patch.object(git_hooks.ci, "branch_queue", reader),
            patch.object(git_hooks, "fast_tier", fast),
            patch.object(preflight.migration, "LOCAL", repo / ".local"),
            patch.object(preflight_stamp, "common_local", return_value=repo / ".local"),
            patch.object(sys, "stdin", stdin),
            contextlib.redirect_stdout(output),
            contextlib.redirect_stderr(errors),
        ):
            code = git_hooks.main(["git_hooks.py", "pre-push", "origin", url])
        return code, output.getvalue() + errors.getvalue(), reader, fast

    def test_a_push_to_the_base_waits_for_an_empty_merge_queue(self) -> None:
        base = "refs/heads/rust-migration"
        url = "https://github.com/owner/name.git"
        queued = git_hooks.ci.BranchQueue(
            (git_hooks.ci.QueueMember(state="QUEUED", position=1, number=1066),), 1
        )
        empty = git_hooks.ci.BranchQueue((), 0)
        with scratch_repo() as repo:
            docs = commit(repo, "Doc: Only", {"docs/x.md": "x\n"})
            code, output, reader, fast = self.hook_push(repo, docs, base, url, queued)
            self.assertEqual(code, 1)
            self.assertIn("#1066 queued at position 1", output)
            self.assertIn("push when the queue is empty", output)
            reader.assert_called_once_with("owner/name", "rust-migration")
            fast.assert_not_called()  # Refused before any fast-tier work.
            code, output, _, fast = self.hook_push(repo, docs, base, url, empty)
            self.assertEqual(code, 0)  # On to the docs-only decision.
            self.assertIn("docs-only change", output)
            fast.assert_called_once()
            work = commit(repo, "Add: Code", {"tools/z.py": "z = 1\n"})
            code, output, _, _ = self.hook_push(repo, work, base, url, empty)
            self.assertEqual(code, 1)  # And to the stamp decision.
            self.assertIn("no preflight stamp", output)

    def test_a_push_of_another_ref_reads_no_queue(self) -> None:
        queued = git_hooks.ci.BranchQueue(
            (git_hooks.ci.QueueMember(state="QUEUED", position=1, number=1066),), 1
        )
        with scratch_repo() as repo:
            docs = commit(repo, "Doc: Only", {"docs/x.md": "x\n"})
            code, _, reader, _ = self.hook_push(
                repo, docs, "refs/heads/t3/work", "https://github.com/o/n", queued
            )
        self.assertEqual(code, 0)
        reader.assert_not_called()

    def test_a_queue_larger_than_its_page_is_counted(self) -> None:
        page = tuple(
            git_hooks.ci.QueueMember(state="AWAITING_CHECKS", position=n, number=n)
            for n in range(1, 11)
        )
        with scratch_repo() as repo:
            docs = commit(repo, "Doc: Only", {"docs/x.md": "x\n"})
            code, output, _, _ = self.hook_push(
                repo,
                docs,
                "refs/heads/rust-migration",
                "https://github.com/o/n",
                git_hooks.ci.BranchQueue(page, 12),
            )
        self.assertEqual(code, 1)
        self.assertIn("holds 12 PRs", output)
        self.assertIn("#10 queued awaiting_checks at position 10 and 2 more", output)

    def test_a_failed_queue_read_refuses_the_push(self) -> None:
        with scratch_repo() as repo:
            docs = commit(repo, "Doc: Only", {"docs/x.md": "x\n"})
            for error in (
                git_hooks.ci.QueueUnread("no merge queue reads for o/n"),
                subprocess.CalledProcessError(1, ["gh"], stderr="HTTP 500"),
            ):
                with self.subTest(error=type(error).__name__):
                    code, output, _, fast = self.hook_push(
                        repo,
                        docs,
                        "refs/heads/rust-migration",
                        "git@github.com:o/n.git",
                        error,
                    )
                    self.assertEqual(code, 1)
                    self.assertIn("pre-push check could not run", output)
                    fast.assert_not_called()
                    self.assertIn(str(error), output)

    def test_the_queue_read_is_for_the_push_destination(self) -> None:
        """A clone may fetch from one repository and push to another."""
        empty = git_hooks.ci.BranchQueue((), 0)
        with scratch_repo() as repo:
            subprocess.run(
                ["git", "remote", "add", "origin", "https://github.com/fetch/other"],
                cwd=repo,
                check=True,
            )
            docs = commit(repo, "Doc: Only", {"docs/x.md": "x\n"})
            for url in (
                "https://github.com/push/dest.git",
                "git@github.com:push/dest.git",
                "ssh://git@github.com/push/dest",
                "ssh://git@ssh.github.com:443/push/dest.git",
                "https://GitHub.com/push/dest.git",
                "https://github.com:443/push/dest.git",
                "https://github.com:0443/push/dest",
            ):
                with self.subTest(url=url):
                    _, _, reader, _ = self.hook_push(
                        repo, docs, "refs/heads/rust-migration", url, empty
                    )
                    reader.assert_called_once_with("push/dest", "rust-migration")
            code, _, reader, _ = self.hook_push(
                repo, docs, "refs/heads/rust-migration", str(repo), empty
            )
        self.assertEqual(code, 0)
        reader.assert_not_called()  # A path destination has no merge queue.

    def test_pre_commit_runs_the_pinned_diff_rules_on_staged_files(self) -> None:
        try:
            preflight.hooks_checkout()
        except (OSError, ValueError, subprocess.CalledProcessError) as error:
            self.skipTest(f"pinned commit checker unavailable: {error}")
        with scratch_repo() as repo:
            for text, code in (("clean\n", 0), ("trailing \n", 1)):
                (repo / "rust/oracle.txt").parent.mkdir(exist_ok=True)
                (repo / "rust/oracle.txt").write_text(text)
                subprocess.run(["git", "add", "-A"], cwd=repo, check=True)
                with self.subTest(text=text), patch("sys.stderr"):
                    self.assertEqual(bool(git_hooks.pre_commit()), bool(code))

    def test_pre_commit_hook_under_gits_variables_in_main_and_linked(self) -> None:
        # #933: git exports GIT_DIR (linked worktree) or an absolute
        # GIT_INDEX_FILE (`commit -a`) to the hook; neither may reach the checker.
        try:
            preflight.hooks_checkout()
        except (OSError, ValueError, subprocess.CalledProcessError) as error:
            self.skipTest(f"pinned commit checker unavailable: {error}")
        with tempfile.TemporaryDirectory() as directory:
            main, linked, runner = (Path(directory) / name for name in ("m", "l", "r"))
            runner.write_text(
                "import subprocess, sys\nfrom pathlib import Path\n"
                f"sys.path.insert(0, {str(REPO / 'tools')!r})\n"
                "import git_hooks, migration\n"
                "top = subprocess.check_output(['git', 'rev-parse', "
                "'--show-toplevel'], text=True)\n"
                "migration.ROOT = Path(top.strip())\n"
                f"migration.COMMON_LOCAL = Path({str(preflight.migration.COMMON_LOCAL)!r})\n"
                "sys.exit(git_hooks.pre_commit())\n"
            )
            env = {
                key: value
                for key, value in os.environ.items()
                if key not in preflight.REPOSITORY_VARIABLES
            } | {"GIT_AUTHOR_NAME": "Seed", "GIT_AUTHOR_EMAIL": "s@example.invalid"}
            env |= {"GIT_COMMITTER_NAME": "Seed", "GIT_COMMITTER_EMAIL": "s@x.invalid"}

            def git(cwd: Path, *arguments: str) -> CompletedProcess[str]:
                return subprocess.run(
                    ["git", *arguments],
                    cwd=cwd,
                    env=env,
                    capture_output=True,
                    text=True,
                    check=False,
                )

            git(Path(directory), "init", "-q", "-b", "work", str(main))
            (main / "a.cpp").write_text("int a;\n")
            git(main, "add", "a.cpp")
            git(main, "commit", "-qm", "Add: Base")
            git(main, "worktree", "add", "-q", "-b", "side", str(linked))
            hook = main / ".git/hooks/pre-commit"
            hook.write_text(f"#!/bin/sh\nexec {sys.executable} {runner}\n")
            hook.chmod(0o755)
            for checkout in (main, linked):
                for text, refused in (("int a; \n", True), ("int b;\n", False)):
                    (checkout / "a.cpp").write_text(text)
                    result = git(checkout, "commit", "-qam", "Change: A")
                    with self.subTest(checkout=checkout.name, text=text):
                        self.assertNotIn("unavailable", result.stdout + result.stderr)
                        self.assertEqual(result.returncode != 0, refused, result)

    def test_installer_writes_shims_and_keeps_foreign_hooks(self) -> None:
        with tempfile.TemporaryDirectory() as directory, patch("builtins.print"):
            hooks = Path(directory)
            (hooks / "pre-push").write_text("#!/bin/sh\nexit 0\n")
            self.assertEqual(git_hooks.install(hooks, force=False), 1)
            self.assertEqual(git_hooks.install(hooks, force=True), 0)
            self.assertEqual(git_hooks.install(hooks, force=False), 0)
            for hook in git_hooks.HOOKS:
                shim = (hooks / hook).read_text()
                self.assertIn(f'tools/git_hooks.py" {hook}', shim)
                self.assertTrue(os.access(hooks / hook, os.X_OK))


class OracleCaptureTests(unittest.TestCase):
    def test_capture_programs_are_tab_indented_without_trailing_whitespace(
        self,
    ) -> None:
        capture = tool("oracle-capture")
        text = "int f()\n{\n    if (x) {  \n\t    g(); \n  }\n}\n"
        self.assertEqual(
            capture.tabbed(text), "int f()\n{\n\tif (x) {\n\t\tg();\n\t}\n}\n"
        )


class CompileCheckTests(unittest.TestCase):
    def test_inputs_select_headers_abi_probes_and_adapters_only(self) -> None:
        check = tool("compile-check")
        for path, selected in (
            ("src/station_base.h", True),
            ("src/rust/abi_ffi.h", True),
            ("tools/generate-abi.py", True),
            ("tools/admin-conversion-comparison.py", True),
            ("tools/migration/admin-conversion-comparison.cpp", True),
            ("tools/simulation/station-resolver-console.hpp", True),
            ("tools/simulation/stations.py", True),
            ("src/station_cmd.cpp", False),
            ("rust/openttd-kernels/src/station.rs", False),
            ("tools/preflight.py", False),
        ):
            with self.subTest(path=path):
                self.assertEqual(bool(check.INPUTS.match(path)), selected)

    def test_definitions_select_game_and_rust_sources_but_not_fork_tests(self) -> None:
        # #1072: #1056 removed a definition that engines-console.cpp declared itself.
        check = tool("compile-check")
        for path, selected in (
            ("src/engine.cpp", True),
            ("src/saveload/company_sl.cpp", True),
            ("rust/openttd-kernels/src/company.rs", True),
            ("src/tests/station.cpp", False),
            ("src/engine_func.h", False),
            ("tools/preflight.py", False),
        ):
            with self.subTest(path=path):
                self.assertEqual(bool(check.DEFINITIONS.match(path)), selected)

    def test_each_adapter_builds_against_both_games_without_compile_only(self) -> None:
        # #1072: #1059's orders console compiled for the candidate only, and
        # failed on CI against the reference's headers.
        check = tool("compile-check")
        modules = [SimpleNamespace(__name__=f"simulation.{n}") for n in ("a", "b")]
        with patch("simulation.registry.adapter_modules", return_value=modules):
            listed = check.jobs(False, check.ROLES)
        self.assertEqual(
            [name for name, _ in listed],
            ["adapter a candidate", "adapter a reference"]
            + ["adapter b candidate", "adapter b reference"],
        )
        self.assertEqual(listed[1][1][-2:], ["--role", "reference"])
        with patch("simulation.registry.adapter_modules", return_value=modules[:1]):
            compiled = check.jobs(False, check.ROLES, link_reference=False)
        names = [name for name, _ in compiled]
        self.assertEqual(
            names, ["adapter a candidate", "adapter a reference, compile only"]
        )
        self.assertEqual([check.adapter_job(name) for name in names], [True, False])
        seen: dict[str, dict[str, str]] = {}
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(check, "BUILD", Path(directory)),
            patch.object(check, "OUT", Path(directory) / "out"),
            patch.object(check.migration, "REFERENCE_BUILD", Path(directory)),
            patch.object(check, "changed", return_value=["src/engine.cpp"]),
            patch.object(check, "generated_abi_changed", return_value=[]),
            patch.object(check.shutil, "which", return_value=None),
            patch.object(check, "jobs", return_value=listed),
            patch.object(
                check,
                "run",
                side_effect=lambda name, _c, env: (
                    seen.setdefault(name, env) and (name, 0, Path(directory))
                ),
            ),
            patch("builtins.print"),
        ):
            for name in ("CMakeCache.txt", "compile_commands.json"):
                (Path(directory) / name).write_text("")
            self.assertEqual(check.check("base"), 0)
        self.assertEqual(sorted(seen), sorted(name for name, _ in listed))
        for env in seen.values():
            self.assertNotIn("PROBE_COMPILE_ONLY", env)

    def fixture_build(self, root: Path, defined: bool) -> Path:
        """A one-object game whose ninja link line the adapter reads, and a
        console that calls a definition the game may have removed."""
        build = root / "build-rust"
        (build / "src").mkdir(parents=True)
        body = "void Removed() {}\n" if defined else ""
        (build / "src/game.cpp").write_text(body + "int main() { return 0; }\n")
        (build / "src/console.cpp").write_text("")
        (build / "build.ninja").write_text(
            "rule cxx\n  command = c++ -c $in -o $out\n"
            "rule link\n  command = c++ $in -o $out\n"
            "build game.o: cxx src/game.cpp\n"
            "build openttd-rust: link game.o\n"
        )
        subprocess.run(["ninja", "-C", str(build)], check=True, capture_output=True)
        entry = {
            "file": str(build / "src/console.cpp"),
            "directory": str(build),
            "command": f"c++ -std=c++20 -I{root} -o console.o -c {build / 'src/console.cpp'}",
        }
        (build / "compile_commands.json").write_text(json.dumps([entry]))
        (root / "abi.h").write_text("void Removed();\n")
        source = root / "fixture-console.cpp"
        source.write_text(
            '#include "abi.h"\n'
            'extern "C" void __wrap__Z22IConsoleStdLibRegisterv() { Removed(); }\n'
        )
        return source

    def adapter_check(self, root: Path, source: Path, compile_only: str = "") -> int:
        """The check's child for one adapter whose build is the fixture."""
        check = tool("compile-check")
        import simulation.console_adapter as consoles

        def adapter(build: Path, folder: Path, env: dict[str, str]) -> Path:
            return consoles.console_adapter(build, folder, env, source, "fixture")

        module = SimpleNamespace(__name__="simulation.fixture", adapter=adapter)
        environ = {k: v for k, v in os.environ.items() if k != "PROBE_COMPILE_ONLY"}
        if compile_only:
            environ["PROBE_COMPILE_ONLY"] = compile_only
        with (
            patch("simulation.registry.adapter_modules", return_value=[module]),
            patch.object(check, "BUILD", root / "build-rust"),
            patch.object(check, "OUT", root / "out"),
            patch.object(check.migration, "candidate_lock", return_value=nullcontext()),
            patch.object(check.migration, "environment", return_value=environ),
            patch.dict(os.environ, environ, clear=True),
            patch.object(sys, "addaudithook"),
            patch("sys.stderr"),
        ):
            return check.child(None, "fixture", "candidate")

    def test_an_adapter_calling_a_removed_definition_fails_at_its_link(self) -> None:
        # #1072: engines-console.cpp declared ClearEnginesHiddenFlagOfCompany
        # itself, so only CI's link of the adapter failed on #1056.
        if not shutil.which("ninja"):
            self.skipTest("no ninja on this host")
        check = tool("compile-check")
        for defined, code in ((True, 0), (False, check.FAILED)):
            with self.subTest(defined=defined), tempfile.TemporaryDirectory() as d:
                root = Path(d).resolve()
                source = self.fixture_build(root, defined)
                self.assertEqual(self.adapter_check(root, source), code)
                # A linked adapter is deleted, pass or fail.
                self.assertFalse(
                    (
                        root / "out/adapters/candidate/linked/fixture/openttd-fixture"
                    ).exists()
                )

    def test_a_fresh_abi_header_fails_beside_a_link_against_the_built_one(self) -> None:
        # Re-check of #1079: one console and one include, `abi.h`, whose
        # build-rust/generated-abi copy declares Removed and whose fresh copy,
        # from current Rust declarations, does not. Each job runs as its own
        # process with the environment check() chose for it.
        if not shutil.which("ninja"):
            self.skipTest("no ninja on this host")
        check = tool("compile-check")
        child = (
            "import importlib.util, sys\n"
            "from contextlib import nullcontext\n"
            "from pathlib import Path\n"
            "from types import SimpleNamespace\n"
            "import probe, simulation.registry as registry\n"
            "import simulation.console_adapter as consoles\n"
            "root, role, script, source = sys.argv[1:5]\n"
            "build = Path(root) / 'build-rust'\n"
            "spec = importlib.util.spec_from_file_location('check', script)\n"
            "g = importlib.util.module_from_spec(spec)\n"
            "spec.loader.exec_module(g)\n"
            "g.BUILD, g.OUT, probe.BUILD = build, Path(root) / 'out', build\n"
            "g.migration.candidate_lock = lambda *a, **k: nullcontext()\n"
            "def adapter(b, folder, env):\n"
            "    return consoles.console_adapter(b, folder, env, Path(source), 'fixture')\n"
            "module = SimpleNamespace(__name__='simulation.fixture', adapter=adapter)\n"
            "registry.adapter_modules = lambda: [module]\n"
            "sys.exit(g.child(None, 'fixture', role))\n"
        )
        with tempfile.TemporaryDirectory() as d:
            root = Path(d).resolve()
            source = self.fixture_build(root, defined=True)
            build = root / "build-rust"
            (root / "abi.h").unlink()
            (build / "generated-abi").mkdir()
            (build / "generated-abi/abi.h").write_text("void Removed();\n")
            (root / "out/generated-abi").mkdir(parents=True)
            (root / "out/generated-abi/abi.h").write_text("void Renamed();\n")
            commands = json.loads((build / "compile_commands.json").read_text())
            commands[0]["command"] += f" -I{build / 'generated-abi'}"
            (build / "compile_commands.json").write_text(json.dumps(commands))
            script = str(REPO / "tools/compile-check.py")

            def run(
                name: str, command: list[str], env: dict[str, str]
            ) -> tuple[str, int, Path]:
                role = command[command.index("--role") + 1]
                log = root / "out/logs" / f"{name.replace(' ', '-')}.log"
                argv = [
                    sys.executable,
                    "-c",
                    child,
                    str(root),
                    role,
                    script,
                    str(source),
                ]
                with log.open("w") as handle:
                    code = subprocess.run(
                        argv,
                        cwd=REPO / "tools",
                        env=env,
                        stdout=handle,
                        stderr=subprocess.STDOUT,
                    ).returncode
                return name, code, log

            module = SimpleNamespace(__name__="simulation.fixture")
            environ = {k: v for k, v in os.environ.items() if k != "PROBE_COMPILE_ONLY"}
            with (
                patch("simulation.registry.adapter_modules", return_value=[module]),
                patch.object(check, "BUILD", build),
                patch.object(check, "OUT", root / "out"),
                patch.object(check.migration, "REFERENCE_BUILD", root / "none"),
                patch.object(check.migration, "environment", return_value=environ),
                patch.object(check, "changed", return_value=["src/engine.cpp"]),
                patch.object(check, "generated_abi_changed", return_value=["abi.h"]),
                patch.object(check.shutil, "which", return_value=None),
                patch.object(check, "run", side_effect=run),
                patch.object(check, "comparisons", return_value=[]),
                patch("builtins.print") as printed,
            ):
                (build / "CMakeCache.txt").write_text("")
                self.assertEqual(check.check("base"), 1)
            lines = [call.args[0] for call in printed.call_args_list]
            failed = [line for line in lines if line.startswith("FAIL")]
            self.assertEqual(len(failed), 1, lines)
            self.assertIn("adapter fixture candidate, compile only", failed[0])
            log = root / "out/logs/adapter-fixture-candidate,-compile-only.log"
            self.assertIn("Removed", log.read_text())

    def test_each_child_holds_the_scratch_lock_until_it_exits(self) -> None:
        # Re-check of #1079: a check stopped alone left children writing.
        check = tool("compile-check")
        with (
            tempfile.TemporaryDirectory() as d,
            patch.object(check, "OUT", Path(d) / "out"),
        ):
            with check.exclusive():
                (Path(d) / "out/logs").mkdir()
                (held,) = check.HELD
                probe_fd = "import os, sys; os.fstat(int(sys.argv[1]))"
                command = [sys.executable, "-c", probe_fd, str(held)]
                self.assertEqual(check.run("lock", command, dict(os.environ))[1], 0)
            self.assertEqual(check.HELD, [])
            # With no lock held, a child gets no descriptor to keep.
            (Path(d) / "out/logs").mkdir(exist_ok=True)
            self.assertNotEqual(check.run("lock", command, dict(os.environ))[1], 0)

    def test_a_link_names_changed_sources_newer_than_the_candidate_game(self) -> None:
        check = tool("compile-check")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "build-rust").mkdir()
            (root / "src").mkdir()
            game = root / "build-rust/openttd-rust"
            game.write_text("")
            os.utime(game, (100, 100))
            (root / "src/engine.cpp").write_text("")
            (root / "src/old.cpp").write_text("")
            os.utime(root / "src/old.cpp", (50, 50))
            with (
                patch.object(check, "ROOT", root),
                patch.object(check, "BUILD", root / "build-rust"),
            ):
                paths = ["src/engine.cpp", "src/old.cpp", "docs/x.md"]
                self.assertEqual(check.newer_than_build(paths), ["src/engine.cpp"])

    def test_skips_with_a_notice_without_a_configured_build(self) -> None:
        check = tool("compile-check")
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(check, "BUILD", Path(directory)),
            patch.object(check, "changed") as changed,
            patch("builtins.print") as printed,
        ):
            self.assertEqual(check.check("base"), 0)
            changed.assert_not_called()
        self.assertIn("no configured build-rust", printed.call_args.args[0])

    def test_seeded_removed_member_fails_the_probe_by_name(self) -> None:
        probe = tool("probe")
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            (out / "shared.h").write_text("struct Station { int town; };\n")
            fixture = out / "probe.cpp"
            fixture.write_text(
                '#include "shared.h"\nint main() { return Station{}.town; }\n'
            )
            command = [
                "c++",
                "-std=c++20",
                "-I",
                out,
                fixture,
                "-o",
                out / "probe",
                "lib.a",
            ]
            with patch.object(probe, "GENERATED", str(out / "abi")):
                self.assertNotIn("lib.a", probe.compile_only(command) or [])
                # A generator run and a link alone are not compiles; neither runs.
                self.assertIsNone(probe.compile_only([out / "strgen", "-L", out]))
                link = probe.compile_only(["c++", out / "a.o", "-o", out / "a"])
                self.assertIsNone(link)
                probe.compile(command, out / "pass.log")
                self.assertFalse((out / "probe").exists())
                (out / "shared.h").write_text("struct Station { };\n")
                with patch("sys.stderr"), self.assertRaises(SystemExit) as stopped:
                    probe.compile(command, out / "fail.log")
                self.assertEqual(stopped.exception.code, probe.COMPILE_FAILED)
                self.assertIn("town", (out / "fail.log").read_text())
                with patch.object(probe, "RUN", False):
                    with self.assertRaisesRegex(RuntimeError, "runs nothing"):
                        probe.execute([out / "probe"], out / "run")
        check = tool("compile-check")
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(check, "OUT", Path(directory)),
        ):
            (Path(directory) / "logs").mkdir()
            failing = [sys.executable, "-c", f"raise SystemExit({check.FAILED})"]
            name, code, log = check.run(
                "admin-conversion-comparison", failing, dict(os.environ)
            )
            self.assertEqual(
                (name, code), ("admin-conversion-comparison", check.FAILED)
            )

    def test_a_child_refuses_every_program_but_compilers_and_reads(self) -> None:
        # Review of #927: a comparison must not start a built binary (#923).
        script = REPO / "tools/compile-check.py"
        for program, code in (("git --version", 0), ("true", 4), ("./reference", 4)):
            with self.subTest(program=program):
                body = (
                    "import runpy, subprocess, sys; sys.argv = ['x']; "
                    f"check = runpy.run_path({str(script)!r}); check['guard'](); "
                    f"subprocess.run({program.split()!r}, capture_output=True)"
                )
                result = subprocess.run(
                    [sys.executable, "-c", body],
                    cwd=REPO / "tools",
                    capture_output=True,
                    check=False,
                )
                self.assertEqual(result.returncode, code, result.stderr)

    def test_a_missing_generated_input_is_a_named_skip_not_a_failure(self) -> None:
        # #933: script-list-comparison read bindings its skipped cmake -P writes.
        check = tool("compile-check")
        with tempfile.TemporaryDirectory() as directory:
            seeded = Path(directory) / "seeded-comparison.py"
            seeded.write_text(f"open({str(Path(directory) / 'gen.hpp')!r})\n")
            with patch("sys.stderr"), patch.object(sys, "addaudithook"):
                self.assertEqual(check.child(str(seeded), None), check.MISSING)
            done = [("seeded", check.MISSING, Path(directory) / "log")]
            with (
                patch.object(check, "BUILD", Path(directory)),
                patch.object(check, "OUT", Path(directory) / "out"),
                patch.object(check, "generated_abi_changed", return_value=["seeded.h"]),
                patch.object(check, "changed", return_value=[]),
                patch.object(check.shutil, "which", return_value=None),
                patch.object(check, "jobs", return_value=[("seeded", [])]),
                patch.object(check, "run", return_value=done[0]),
                patch("builtins.print") as printed,
            ):
                for name in ("CMakeCache.txt", "compile_commands.json"):
                    (Path(directory) / name).write_text("")
                self.assertEqual(check.check("base"), 0)
            lines = [call.args[0] for call in printed.call_args_list]
            self.assertTrue(any(line.startswith("SKIP seeded") for line in lines))

    def test_clang_fails_a_missing_initializer_only_behind_a_touched_file(self) -> None:
        # #933: GCC does not report -Wmissing-field-initializers here (#885, #929).
        if not shutil.which("clang++"):
            self.skipTest("no clang++ on this host")
        check = tool("compile-check")
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory).resolve()
            (out / "tools").mkdir()
            (out / "services.h").write_text("struct S { int a; int b; };\n")
            (out / "tools/probe.inc").write_text("S s{ .a = 1 };\n")
            source = out / "tools/probe.cpp"
            source.write_text('#include "../services.h"\n#include "probe.inc"\n')
            command = f"/usr/bin/c++ -std=c++20 -O2 -flifetime-dse=1 -o x.o -c {source}"
            entry = {"file": str(source), "directory": str(out), "command": command}
            (out / "compile_commands.json").write_text(json.dumps([entry]))
            (out / "clang").mkdir()
            with (
                patch.object(check, "BUILD", out),
                patch.object(check, "OUT", out),
                patch.object(check, "ROOT", out),
            ):
                (warning,), checked, listed = check.clang_pass(
                    {out / "services.h"}, out
                )
                self.assertEqual((checked, listed), (1, 1))
                self.assertIn("probe.inc:1", warning)
                self.assertIn("-Wmissing-field-initializers", warning)
                untouched = check.clang_pass({out / "other.h"}, out)
                self.assertEqual(untouched, ([], 0, 1))

    def test_comparisons_guard_their_runs_and_abi_is_always_regenerated(self) -> None:
        check = tool("compile-check")
        for path in check.comparisons():
            text = path.read_text()
            runs = re.search(
                r"probe\.execute\(|subprocess\.(run|Popen)\(|scenario\(", text
            )
            with self.subTest(comparison=path.stem):
                self.assertTrue(not runs or "probe.RUN" in text)
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(check, "BUILD", Path(directory)),
            patch.object(check, "OUT", Path(directory) / "out"),
            patch.object(check, "changed", return_value=["src/shared.h"]),
            patch.object(check, "generated_abi_changed", return_value=[]) as abi,
            patch.object(check.shutil, "which", return_value=None),
            patch.object(check, "jobs", return_value=[]),
            patch("builtins.print"),
        ):
            for name in ("CMakeCache.txt", "compile_commands.json"):
                (Path(directory) / name).write_text("")
            (Path(directory) / "out/generated-abi").mkdir(parents=True)
            self.assertEqual(check.check("base"), 0)
            abi.assert_called_once()

    def test_a_fork_test_source_alone_runs_only_the_clang_pass(self) -> None:
        # #933: c11ad644db's warning was in src/tests/station_service.cpp alone.
        check = tool("compile-check")
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(check, "BUILD", Path(directory)),
            patch.object(check, "OUT", Path(directory) / "out"),
            patch.object(check, "changed", return_value=["src/tests/station.cpp"]),
            patch.object(check, "generated_abi_changed", return_value=[]),
            patch.object(check.shutil, "which", return_value="clang++"),
            patch.object(check, "jobs") as jobs,
            patch.object(
                check, "clang_pass", return_value=(["x:1:1: warning: w"], 1, 1)
            ),
            patch("builtins.print"),
        ):
            for name in ("CMakeCache.txt", "compile_commands.json"):
                (Path(directory) / name).write_text("")
            self.assertEqual(check.check("base"), 1)
            jobs.assert_not_called()


class FastTests(unittest.TestCase):
    """--fast: the pre-push fast tier and Quick's ty line, with no stamp."""

    @contextmanager
    def fakes(self, failing: str = "") -> Generator[Path]:
        """Fake cargo, uvx and commit checker that log each call and fail when
        their arguments contain `failing`."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            calls = root / "calls"
            script = (
                f'#!/bin/sh\necho "$(basename "$0") $*" >> {calls}\n'
                f'case "$*" in *"{failing or "-never-"}"*) exit 1;; esac\n'
            )
            for name in ("bin/cargo", "bin/uvx", "hooks/hooks/check-commits.sh"):
                (root / name).parent.mkdir(parents=True, exist_ok=True)
                (root / name).write_text(script)
                (root / name).chmod(0o755)
            path = f"{root / 'bin'}{os.pathsep}{os.environ['PATH']}"
            with (
                patch.object(preflight, "revision", return_value="sha"),
                patch.object(preflight, "hooks_checkout", return_value=root / "hooks"),
                patch.object(preflight, "environment", return_value={"PATH": path}),
                patch.object(preflight.migration, "LOCAL", root / "local"),
                patch.object(preflight_stamp, "record") as record,
            ):
                yield calls
            record.assert_not_called()

    def fast(self, *argv: str) -> tuple[int, list[str]]:
        with (
            patch.object(sys, "argv", ["preflight.py", "--fast", *argv]),
            patch("builtins.print") as printed,
        ):
            code = preflight.main()
        return code, [" ".join(map(str, call.args)) for call in printed.call_args_list]

    def test_runs_the_fast_tier_and_ty_once_each_without_a_stamp(self) -> None:
        quick = [
            *preflight.quick_workflow.ruff_lines(),
            preflight.quick_workflow.ty_line(),
        ]
        with self.fakes() as calls:
            code, lines = self.fast()
            self.assertEqual(code, 0)
            self.assertEqual(
                calls.read_text().splitlines(),
                [
                    "check-commits.sh sha..sha",
                    "cargo fmt --all -- --check",
                    *quick,
                ],
            )
        self.assertIn("5/5 passed", lines[-1])
        self.assertIn("no stamp", lines[-1])

    def test_a_failed_step_prints_the_command_that_fixes_it(self) -> None:
        with self.fakes(failing="format") as calls:
            code, lines = self.fast()
            self.assertEqual(len(calls.read_text().splitlines()), 5)
        self.assertEqual(code, 1)
        failed = [line for line in lines if line.startswith("failed: ")]
        self.assertEqual(len(failed), 1)
        self.assertIn("ruff format --check", failed[0])
        self.assertTrue(failed[0].endswith("; fix: python3 tools/migration.py fmt"))
        with self.fakes(failing="ty@") as calls:
            code, lines = self.fast()
        self.assertEqual(code, 1)
        (failed_ty,) = [line for line in lines if line.startswith("failed: ")]
        self.assertNotIn("fix:", failed_ty)

    def test_list_names_the_steps_and_runs_nothing(self) -> None:
        with (
            self.fakes() as calls,
            patch.object(preflight, "hooks_checkout") as hooks,
        ):
            code, lines = self.fast("--list")
            self.assertFalse(calls.exists())
            hooks.assert_not_called()
        self.assertEqual(code, 0)
        self.assertEqual(lines[0].splitlines()[0], preflight.COMMITS)
        self.assertEqual(len(lines[0].splitlines()), 5)

    def test_fast_cannot_be_combined_with_step(self) -> None:
        with (
            patch.object(sys, "argv", ["preflight.py", "--fast", "--step", "ruff"]),
            patch("sys.stderr", new_callable=io.StringIO) as stderr,
            self.assertRaises(SystemExit) as stopped,
        ):
            preflight.main()
        self.assertEqual(stopped.exception.code, 2)
        self.assertIn("not allowed with argument --fast", stderr.getvalue())


class CommitMsgTests(unittest.TestCase):
    """The commit-msg hook: the pinned message check, then this fork's remedy."""

    @contextmanager
    def repo(self, missing_checker: bool = False) -> Generator[Path]:
        """A repository on branch work whose commit-msg hook runs this git_hooks,
        with origin/rust-migration one commit ahead of the shared base."""
        if not missing_checker:
            try:
                preflight.hooks_checkout()
            except (OSError, ValueError, subprocess.CalledProcessError) as error:
                self.skipTest(f"pinned commit checker unavailable: {error}")
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory) / "repo"
            runner = Path(directory) / "runner.py"
            gone = (
                "def gone() -> None:\n    raise ValueError('no checkout')\n"
                "preflight.hooks_checkout = gone\n"
            )
            runner.write_text(
                f"import sys\nsys.path.insert(0, {str(REPO / 'tools')!r})\n"
                "import git_hooks, preflight\n"
                + (gone if missing_checker else "")
                + "sys.exit(git_hooks.main(['git_hooks.py', 'commit-msg', *sys.argv[1:]]))\n"
            )
            self.git(Path(directory), "init", "-q", "-b", "work", str(repo))
            self.write(repo, "a.txt", "a\n")
            self.git(repo, "commit", "-qm", "Add: Base", "--no-verify")
            self.git(repo, "switch", "-qc", "base")
            self.write(repo, "b.txt", "b\n")
            self.git(repo, "commit", "-qm", "Add: Base change", "--no-verify")
            self.git(repo, "update-ref", "refs/remotes/origin/rust-migration", "HEAD")
            self.git(repo, "switch", "-q", "work")
            hook = repo / ".git/hooks/commit-msg"
            hook.write_text(f'#!/bin/sh\nexec {sys.executable} {runner} "$@"\n')
            hook.chmod(0o755)
            yield repo

    def git(self, cwd: Path, *arguments: str) -> CompletedProcess[str]:
        env = {
            key: value
            for key, value in os.environ.items()
            if key not in preflight.REPOSITORY_VARIABLES
        }
        env |= {"GIT_AUTHOR_NAME": "Seed", "GIT_AUTHOR_EMAIL": "s@example.invalid"}
        env |= {"GIT_COMMITTER_NAME": "Seed", "GIT_COMMITTER_EMAIL": "s@x.invalid"}
        return subprocess.run(
            ["git", *arguments],
            cwd=cwd,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )

    def write(self, repo: Path, name: str, text: str) -> None:
        (repo / name).parent.mkdir(parents=True, exist_ok=True)
        (repo / name).write_text(text)
        self.git(repo, "add", "-A")

    def commit(self, repo: Path, message: str) -> CompletedProcess[str]:
        self.write(repo, "c.txt", message)
        return self.git(repo, "commit", "-qm", message)

    def test_each_refusal_names_its_fix(self) -> None:
        with self.repo() as repo:
            for message, expected in (
                ("Tools: Add a thing", "Accepted prefixes: Add, Feature, Change"),
                ("Fix: A dash \u2014 here", "line 1 column 13: U+2014 EM DASH"),
                ("Fix: Body\n\nhas\ta tab", "line 3 column 4: a tab"),
                ("fixup! Fix: Body", "A `fixup!` title is refused here"),
                ("squash! Fix: Body", "Give the commit a normal title"),
                ("amend! Fix: Body", "the commit checker refuses it at push"),
            ):
                with self.subTest(message=message):
                    result = self.commit(repo, message)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("***", result.stderr)
                    self.assertIn(expected, result.stderr)
            accepted = self.commit(repo, "Fix #12: A plain title")
            self.assertEqual(accepted.returncode, 0, accepted.stderr)
            self.assertEqual(accepted.stderr, "")
            refused = self.commit(repo, "Fix: Body\n\nhas\ta tab")
            self.assertNotIn("Accepted prefixes", refused.stderr)

    def test_a_base_update_prints_its_parents_and_the_exact_command(self) -> None:
        with self.repo() as repo:
            self.write(repo, "w.txt", "w\n")
            self.git(repo, "commit", "-qm", "Add: Work", "--no-verify")
            merge = self.git(repo, "merge", "--no-edit", "origin/rust-migration")
            self.assertNotEqual(merge.returncode, 0)
            self.assertIn("This is a base update", merge.stderr)
            self.assertIn("parent 1 (HEAD):       ", merge.stderr)
            self.assertIn("Add: Work\n", merge.stderr)
            self.assertIn("parent 2 (MERGE_HEAD): ", merge.stderr)
            self.assertIn("Add: Base change", merge.stderr)
            command = 'git commit -m "Update: Merge rust-migration into work"'
            self.assertIn(f"commit-msg:   {command}\n", merge.stderr)
            done = self.git(
                repo, "commit", "-m", "Update: Merge rust-migration into work"
            )
            self.assertEqual(done.returncode, 0, done.stderr)
            parents = self.git(repo, "rev-list", "--parents", "-n1", "HEAD").stdout
            self.assertEqual(len(parents.split()), 3)

    def test_a_merge_of_another_branch_gets_no_command(self) -> None:
        with self.repo() as repo:
            self.git(repo, "switch", "-qc", "other", "work")
            self.write(repo, "o.txt", "o\n")
            self.git(repo, "commit", "-qm", "Add: Other", "--no-verify")
            self.git(repo, "switch", "-q", "work")
            self.write(repo, "w.txt", "w\n")
            self.git(repo, "commit", "-qm", "Add: Work", "--no-verify")
            merge = self.git(repo, "merge", "--no-edit", "other")
            self.assertNotEqual(merge.returncode, 0)
            self.assertIn("This merge is not a base update", merge.stderr)
            self.assertIn("Accepted prefixes", merge.stderr)
            self.assertNotIn("git commit -m", merge.stderr)

    def test_a_missing_checker_lets_the_commit_through_with_a_notice(self) -> None:
        with self.repo(missing_checker=True) as repo:
            result = self.commit(repo, "Tools: Add a thing")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("pinned checker unavailable, not checked", result.stderr)

    def test_install_names_a_foreign_commit_msg_hook_and_keeps_it(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            hooks = Path(directory)
            foreign = '#!/usr/bin/env python3\n"""Catch message failures."""\n'
            (hooks / "commit-msg").write_text(foreign)
            with patch("builtins.print") as printed:
                self.assertEqual(git_hooks.install(hooks, force=False), 1)
            message = printed.call_args.args[0]
            self.assertIn("Catch message failures", message)
            self.assertIn("install --force", message)
            self.assertEqual((hooks / "commit-msg").read_text(), foreign)
            with patch("builtins.print"):
                self.assertEqual(git_hooks.install(hooks, force=True), 0)
            self.assertIn(git_hooks.MARKER, (hooks / "commit-msg").read_text())

    def run_shim(self, repo: Path, hook: str, python: Path) -> CompletedProcess[str]:
        """Run an installed shim in repo with a recording python3 first on PATH."""
        env = os.environ | {"PATH": f"{python.parent}{os.pathsep}{os.environ['PATH']}"}
        shim = repo / ".git/hooks" / hook
        return subprocess.run(
            [str(shim), "argument"], cwd=repo, env=env, text=True, capture_output=True
        )

    @contextmanager
    def shimmed(self) -> Generator[tuple[Path, Path, Path]]:
        """A repository with every shim installed, a python3 that records its
        arguments instead of running anything, and the record it writes."""
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory) / "repo"
            self.git(Path(directory), "init", "-q", "-b", "work", str(repo))
            with patch("builtins.print"):
                self.assertEqual(git_hooks.install(repo / ".git/hooks", force=False), 0)
            record = Path(directory) / "record"
            python = Path(directory) / "bin/python3"
            python.parent.mkdir()
            python.write_text(f'#!/bin/sh\necho "$@" >> {record}\n')
            python.chmod(0o755)
            yield repo, python, record

    def test_every_installed_shim_runs_this_file_for_its_hook(self) -> None:
        # A shim that stopped recognising this file would do nothing, silently.
        with self.shimmed() as (repo, python, record):
            production = (REPO / "tools/git_hooks.py").read_text()
            self.write(repo, "tools/git_hooks.py", production)
            for hook in git_hooks.HOOKS:
                with self.subTest(hook=hook):
                    record.unlink(missing_ok=True)
                    result = self.run_shim(repo, hook, python)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    ran = record.read_text().split()
                    self.assertEqual(ran[1:], [hook, "argument"])
                    # Git names the repository by its physical path.
                    self.assertTrue(Path(ran[0]).samefile(repo / "tools/git_hooks.py"))

    def test_every_hook_name_reaches_its_handler(self) -> None:
        handlers = {
            "pre-push": "pre_push",
            "pre-commit": "pre_commit",
            "commit-msg": "commit_msg",
        }
        self.assertEqual(set(handlers), set(git_hooks.HOOKS))
        for hook, handler in handlers.items():
            with (
                self.subTest(hook=hook),
                patch.object(git_hooks, handler, return_value=7) as called,
                patch("sys.stdin", io.StringIO("")),
            ):
                self.assertEqual(git_hooks.main(["git_hooks.py", hook, "argument"]), 7)
                called.assert_called_once()

    def test_a_shim_does_nothing_where_the_file_does_not_list_its_hook(self) -> None:
        with self.shimmed() as (repo, python, record):
            older = 'HOOKS = ("pre-push", "pre-commit")\n'
            self.write(repo, "tools/git_hooks.py", older)
            result = self.run_shim(repo, "commit-msg", python)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(record.exists())
            for hook in ("pre-push", "pre-commit"):
                self.run_shim(repo, hook, python)
            self.assertEqual(len(record.read_text().splitlines()), 2)
            (repo / "tools/git_hooks.py").unlink()
            record.unlink()
            self.assertEqual(self.run_shim(repo, "pre-push", python).returncode, 0)
            self.assertFalse(record.exists())


if __name__ == "__main__":
    unittest.main()
