#!/usr/bin/env python3
"""Exercise bounded validation, lock coordination, and runtime capture."""

import contextlib
import inspect
import io
import json
import os
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from collections.abc import Callable
from dataclasses import replace
from pathlib import Path
from types import SimpleNamespace
from typing import IO, cast
from unittest.mock import patch

import validation_execution
from simulation import configuration, core, game
from validation_execution import file_lock, run_logged

import migration

TOOLS = Path(__file__).resolve().parent


def running(pid: int) -> bool:
    """Return whether pid exists and is not a zombie; a reaped pid has exited."""
    try:
        state = Path(f"/proc/{pid}/stat").read_text()
    except (FileNotFoundError, ProcessLookupError):
        return False
    return state.split(") ", 1)[1][0] != "Z"


def ready_clock(log: Path, lines: int, limit: float = 30.0) -> SimpleNamespace:
    """Stand in for `run_logged`'s `time`, with a clock that starts at child readiness.

    The clock reads zero until the log holds `lines` lines, so a timeout or
    heartbeat window counts from the child's readiness, not from interpreter
    startup under host load. After `limit` real seconds without readiness it
    reports the real elapsed time, so a child that never becomes ready still
    times out and fails the test's log assertion.
    """
    real = time.monotonic
    origin = real()
    ready: list[float] = []

    def monotonic() -> float:
        now = real()
        if not ready:
            try:
                text = log.read_text()
            except FileNotFoundError:
                text = ""
            if len(text.splitlines()) < lines:
                return 0.0 if now - origin < limit else now - origin
            ready.append(now)
        return now - ready[0]

    return SimpleNamespace(monotonic=monotonic)


class HeartbeatSignal(io.StringIO):
    """Captured driver output that marks a file once a heartbeat line is written."""

    def __init__(self, marker: Path) -> None:
        super().__init__()
        self.marker = marker

    def write(self, text: str) -> int:
        written = super().write(text)
        if ": running " in text:
            self.marker.touch()
        return written


class ExecutionTests(unittest.TestCase):
    def test_package_module_import_needs_no_pythonpath(self) -> None:
        subprocess.run(
            [
                sys.executable,
                "-c",
                "from tools.simulation import core; import migration; assert core.file_lock and migration.COMMON_LOCAL",
            ],
            cwd=TOOLS.parent,
            env=dict(os.environ, PYTHONPATH=""),
            check=True,
        )

    def capture(
        self, script: str, directory: str, phase: str, **options: bool
    ) -> tuple[int, Path, str]:
        log, output = Path(directory) / f"{phase}.log", io.StringIO()
        with contextlib.redirect_stdout(output):
            code = run_logged(
                [sys.executable, "-c", script],
                cwd=directory,
                env=None,
                log=log,
                phase=phase,
                heartbeat=0.02,
                **options,
            )
        return code, log, output.getvalue()

    def test_quiet_child_reports_heartbeat_and_failure(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            # The quiet child exits only after the driver has reported a heartbeat
            # (bounded at 30 s), so the claim holds however the host schedules it.
            log, marker = Path(directory) / "quiet.log", Path(directory) / "heartbeat"
            output = HeartbeatSignal(marker)
            script = (
                "import os,time; deadline=time.monotonic()+30\n"
                + f"while not os.path.exists({str(marker)!r}) and time.monotonic()<deadline: time.sleep(.01)\n"
                + "print('retained'); raise SystemExit(7)"
            )
            with contextlib.redirect_stdout(output):
                code = run_logged(
                    [sys.executable, "-c", script],
                    cwd=directory,
                    env=None,
                    log=log,
                    phase="quiet",
                    heartbeat=0.02,
                )
            self.assertEqual(code, 7)
            self.assertEqual(log.read_text(), "retained\n")
            for expected in ("quiet: running", str(log), "exit 7"):
                self.assertIn(expected, output.getvalue())

    def test_streaming_preserves_split_utf8_and_complete_log(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            code, log, output = self.capture(
                "import os,time; os.write(1,b'\\xe2'); time.sleep(.08); os.write(1,b'\\x82\\xac\\n')",
                directory,
                "stream",
                stream=True,
            )
            self.assertEqual(code, 0)
            self.assertEqual(log.read_text(), "€\n")
            self.assertIn("€\n", output)
            self.assertNotIn("�", output)

    def test_timeout_reaps_child_and_retains_output(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "timeout.log"
            supervisors: list[subprocess.Popen[bytes]] = []
            popen = cast("Callable[..., subprocess.Popen[bytes]]", subprocess.Popen)

            def recorded(*args: object, **kwargs: object) -> subprocess.Popen[bytes]:
                supervisors.append(popen(*args, **kwargs))
                return supervisors[-1]

            # The 0.09 s timeout counts from the child's `started` line.
            with (
                contextlib.redirect_stdout(io.StringIO()),
                patch.object(validation_execution, "time", ready_clock(log, 1)),
                patch.object(validation_execution.subprocess, "Popen", recorded),
            ):
                with self.assertRaises(subprocess.TimeoutExpired):
                    run_logged(
                        [
                            sys.executable,
                            "-c",
                            "import time; print('started', flush=True); time.sleep(20)",
                        ],
                        cwd=directory,
                        env=None,
                        log=log,
                        phase="timeout",
                        heartbeat=0.03,
                        timeout=0.09,
                    )
            self.assertEqual(log.read_text(), "started\n")
            # run_logged waited on its supervisor before raising: it is reaped (#847).
            self.assertEqual(len(supervisors), 1)
            self.assertIsNotNone(supervisors[0].returncode)

    @unittest.skipUnless(sys.platform == "linux", "Linux descendant process state")
    def test_killed_driver_group_or_timeout_leaves_no_running_descendants(self) -> None:
        signals = (signal.SIGTERM, signal.SIGHUP, signal.SIGKILL, None)
        for index, killed in enumerate(signals * 2):
            with (
                self.subTest(signal=killed),
                tempfile.TemporaryDirectory() as directory,
            ):
                log = Path(directory) / "descendant.log"
                child = "import os,signal,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); print(os.getpid(),flush=True); time.sleep(60)"
                command = (
                    "import os,subprocess,sys,time; print(os.getpid(),flush=True); "
                    + f"subprocess.Popen([sys.executable,'-c',{child!r}]); time.sleep(60)"
                )
                # In the timeout case the driver's 0.5 s timeout counts from both
                # descendants having logged their pids.
                driver = (
                    "import time\nfrom pathlib import Path\nfrom types import SimpleNamespace\n"
                    + inspect.getsource(ready_clock)
                    + "import validation_execution\n"
                    + f"validation_execution.time = ready_clock(Path({str(log)!r}), 2)\n"
                    + f"validation_execution.run_logged({[sys.executable, '-c', command]!r},cwd={directory!r},env=None,log={str(log)!r},phase='kill',timeout={None if killed else 0.5!r})"
                )
                process = subprocess.Popen(
                    [sys.executable, "-c", driver],
                    cwd=TOOLS,
                    start_new_session=True,
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                )
                descendants: list[int] = []
                try:
                    deadline = time.monotonic() + 30
                    while not log.exists() or len(log.read_text().splitlines()) < 2:
                        if time.monotonic() >= deadline:
                            self.fail("descendants did not start")
                        time.sleep(0.01)
                    descendants = list(map(int, log.read_text().splitlines()))
                    if killed:
                        (os.killpg if index < len(signals) else os.kill)(
                            process.pid, killed
                        )
                    process.wait(timeout=30)
                    for descendant in descendants:
                        while running(descendant):
                            if time.monotonic() >= deadline:
                                self.fail(
                                    f"descendant {descendant} survived driver death"
                                )
                            time.sleep(0.01)
                finally:
                    process.kill() if process.poll() is None else None
                    process.wait()
                    for descendant in descendants:
                        try:
                            os.kill(descendant, signal.SIGKILL)
                        except ProcessLookupError:
                            pass

    @unittest.skipUnless(os.name == "posix", "POSIX cross-process locks")
    def test_wait_reports_holder_and_releases(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            lock = Path(directory) / "held.lock"
            script = (
                "from validation_execution import file_lock; import sys; "
                "\nwith file_lock(sys.argv[1], heartbeat=.03, label='child'): print('entered', flush=True)"
            )
            with file_lock(lock, label="parent"):
                child = subprocess.Popen(
                    [sys.executable, "-c", script, str(lock)],
                    cwd=TOOLS,
                    stdout=subprocess.PIPE,
                    text=True,
                )
                line = cast(IO[str], child.stdout).readline()
                self.assertIn("waiting", line)
                self.assertIn(f'"pid": {os.getpid()}', line)
                self.assertIn('"label": "parent"', line)
                self.assertIsNone(child.poll())
            with child:
                remaining = cast(IO[str], child.stdout).read()
                self.assertEqual(child.wait(timeout=5), 0)
            self.assertIn("acquired", remaining)
            self.assertIn("entered", remaining)
            self.assertEqual(
                list(lock.with_name(lock.name + ".holders").glob("*.json")), []
            )


class RuntimeTests(unittest.TestCase):
    def initialize_repo(self, root: Path) -> None:
        for arguments in (
            ("init", "-q"),
            ("config", "user.email", "test@example.invalid"),
            ("config", "user.name", "Fixture"),
        ):
            subprocess.run(["git", "-C", str(root), *arguments], check=True)
        (root / ".gitignore").write_text(".local/\nbuild-rust/\nreference-build/\n")
        (root / "source").write_text("initial")
        subprocess.run(
            ["git", "-C", str(root), "add", "source", ".gitignore"], check=True
        )
        subprocess.run(
            ["git", "-C", str(root), "commit", "-qm", "Add: Fixture"], check=True
        )

    @unittest.skipUnless(os.name == "posix", "POSIX cross-process locks")
    def test_capture_waits_for_build_and_copies_completed_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            build = root / "build"
            build.mkdir()
            binary = build / "openttd-rust"
            binary.write_text("old")
            (build / "CMakeCache.txt").write_text("configuration")
            script = (
                "from simulation.core import copy_runtime; from pathlib import Path; import sys; "
                "copy_runtime(Path(sys.argv[1]),Path(sys.argv[2]),Path(sys.argv[3]),candidate=True)"
            )
            with migration.candidate_lock(build):
                child = subprocess.Popen(
                    [
                        sys.executable,
                        "-c",
                        script,
                        str(build),
                        str(binary),
                        str(root / "frozen"),
                    ],
                    cwd=TOOLS,
                    stdout=subprocess.PIPE,
                    text=True,
                )
                self.assertIn(
                    "candidate capture: waiting", cast(IO[str], child.stdout).readline()
                )
                self.assertFalse((root / "frozen").exists())
                binary.write_text("new")
            with child:
                cast(IO[str], child.stdout).read()
                self.assertEqual(child.wait(timeout=5), 0)
            self.assertEqual((root / "frozen/openttd-rust").read_text(), "new")

    def test_direct_game_call_initializes_clone_lock(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            with (
                patch.object(
                    configuration,
                    "_current",
                    replace(
                        configuration.configuration(), common_local=Path(directory)
                    ),
                ),
                patch.object(game, "GAME_LOCK", None),
            ):
                with core.game_slot():
                    self.assertTrue((Path(directory) / "simulation-game.lock").exists())

    @unittest.skipUnless(os.name == "posix", "POSIX cross-process locks")
    def test_benchmark_enters_while_another_shared_game_is_running(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            lock = Path(directory) / "simulation-game.lock"
            script = (
                "from simulation import core, game, configuration; from pathlib import Path; import sys\n"
                "game.GAME_LOCK = Path(sys.argv[1])\n"
                "with core.game_slot(): print('entered', flush=True)\n"
            )
            with file_lock(lock, shared=True, label="ordinary game"):
                with subprocess.Popen(
                    [sys.executable, "-c", script, str(lock)],
                    cwd=TOOLS,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                ) as child:
                    try:
                        output, errors = child.communicate(timeout=5)
                    finally:
                        if child.poll() is None:
                            child.kill()
                            child.communicate(timeout=5)
                    self.assertEqual(child.returncode, 0, errors)
                    self.assertIn("entered", output)
            self.assertEqual(
                list(lock.with_name(lock.name + ".holders").glob("*.json")), []
            )


class NarrowChecksTests(unittest.TestCase):
    initialize_repo = RuntimeTests.initialize_repo

    def exercise_checks(self, *, failing: bool = False) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.initialize_repo(root)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            cargo = bin_dir / "cargo"
            cargo.write_text(
                f"#!{sys.executable}\nimport os,sys,json\n"
                "print(json.dumps({'args':sys.argv[1:],'jobs':os.environ.get('CARGO_BUILD_JOBS')}))\n"
                + ("sys.exit(3 if sys.argv[1]=='clippy' else 0)\n" if failing else "")
            )
            cargo.chmod(0o755)
            git = migration.git
            env = dict(os.environ, PATH=str(bin_dir) + os.pathsep + os.environ["PATH"])
            with (
                patch.object(migration, "ROOT", root),
                patch.object(migration, "LOCAL", root / ".local"),
                patch.object(
                    migration, "git", side_effect=lambda *args: git(*args, cwd=root)
                ),
                patch.object(migration, "environment", return_value=env),
                patch.object(
                    migration,
                    "ensure_reference",
                    side_effect=AssertionError("reference preparation"),
                ),
                patch.object(
                    migration,
                    "reference_lock",
                    side_effect=AssertionError("reference lock"),
                ),
                patch.object(
                    migration.shutil,
                    "which",
                    side_effect=AssertionError("cache preparation"),
                ),
                # Without --jobs the driver takes the free cores, 8 - round(2.4).
                patch.object(migration.os, "cpu_count", return_value=8),
                patch.object(migration.os, "getloadavg", return_value=(2.4, 9, 9)),
                patch.object(
                    sys,
                    "argv",
                    ["migration.py", "rust-checks"]
                    + ([] if failing else ["--jobs", "2"]),
                ),
                contextlib.redirect_stdout(io.StringIO()),
                contextlib.redirect_stderr(io.StringIO()),
            ):
                if failing:
                    with self.assertRaises(RuntimeError):
                        migration.main()
                else:
                    migration.main()
            report = json.loads(
                next((root / ".local/verification").glob("*/report.json")).read_text()
            )
            self.assertEqual(report["schema_version"], 1)
            self.assertEqual(report["action"], "rust-checks")
            self.assertFalse(report["candidate_rust_enabled"])
            self.assertFalse(report["compiler_cache"]["enabled"])
            self.assertEqual(report["passed"], not failing)
            names = [c["name"] for c in report["commands"]]
            self.assertEqual(
                names,
                ["rust-fmt", "rust-check", "rust-clippy"]
                + ([] if failing else ["rust-tests"]),
            )
            for command in report["commands"]:
                log = json.loads((root / command["log"]).read_text())
                self.assertEqual(log["jobs"], "6" if failing else "2")

    def test_free_cores_keep_two_jobs_on_a_loaded_host(self) -> None:
        with (
            patch.object(migration.os, "cpu_count", return_value=8),
            patch.object(migration.os, "getloadavg", return_value=(7.6, 0, 0)),
        ):
            self.assertEqual(migration.free_cores(), 2)

    def test_rust_checks_never_prepares_native_or_reference_build(self) -> None:
        self.exercise_checks()

    def test_rust_checks_retains_failure_log(self) -> None:
        self.exercise_checks(failing=True)


class NativeJobLimitTests(unittest.TestCase):
    initialize_repo = RuntimeTests.initialize_repo

    def test_cmake_child_cargo_inherits_driver_job_budget(self) -> None:
        shared_caches = {
            migration.COMMON_LOCAL / f"build-reference{suffix}" / "CMakeCache.txt": None
            for suffix in ("", "-sanitized")
        }
        for cache in shared_caches:
            shared_caches[cache] = cache.read_bytes() if cache.exists() else None
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.initialize_repo(root)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            cargo = bin_dir / "cargo"
            cargo.write_text(
                f"#!{sys.executable}\nimport os\nprint('nested cargo jobs='+os.environ['CARGO_BUILD_JOBS'])\n"
            )
            cargo.chmod(0o755)
            cmake = bin_dir / "cmake"
            cmake.write_text(
                f"#!{sys.executable}\nimport sys,subprocess\nfrom pathlib import Path\n"
                "args=sys.argv[1:]\n"
                "root=Path(__file__).resolve().parents[1]\n"
                "for flag in ('-S', '-B', '--build'):\n"
                "    if flag in args: Path(args[args.index(flag)+1]).resolve().relative_to(root)\n"
                "if '--build' in args:\n"
                "    build=Path(args[args.index('--build')+1])\n"
                "    subprocess.run(['cargo'],check=True)\n"
                "    (build/('openttd-rust' if build.name=='build-rust' else 'openttd')).write_text('built')\n"
                "else:\n"
                "    build=Path(args[args.index('-B')+1]); build.mkdir(parents=True,exist_ok=True)\n"
                "    (build/'CMakeCache.txt').write_text('fixture configuration')\n"
            )
            cmake.chmod(0o755)
            git = migration.git
            env = dict(os.environ, PATH=str(bin_dir) + os.pathsep + os.environ["PATH"])
            with (
                patch.object(migration, "ROOT", root),
                patch.object(migration, "LOCAL", root / ".local"),
                patch.object(migration, "COMMON_LOCAL", root / ".local"),
                patch.object(migration, "REFERENCE", root / "reference"),
                patch.object(migration, "REFERENCE_BUILD", root / "reference-build"),
                patch.object(migration, "CANDIDATE_BUILD", root / "build-rust"),
                patch.object(migration, "SANITIZE", False),
                patch.object(
                    migration, "git", side_effect=lambda *args: git(*args, cwd=root)
                ),
                patch.object(migration, "environment", return_value=env),
                patch.object(migration, "ensure_reference"),
                patch.object(
                    migration, "reference_lock", return_value=contextlib.nullcontext()
                ),
                patch.object(
                    migration, "verify_compiler_cache_options", return_value={}
                ),
                patch.object(migration, "rust_configuration", return_value={}),
                patch.object(migration, "cmake_cache", return_value={}),
                patch.object(migration, "supply_graphics"),
                patch.object(
                    sys, "argv", ["migration.py", "build", "--jobs", "2", "--no-ccache"]
                ),
                contextlib.redirect_stdout(io.StringIO()),
            ):
                migration.main()
            report = json.loads(
                next((root / ".local/verification").glob("*/report.json")).read_text()
            )
            self.assertTrue(report["passed"])
            for cache, before in shared_caches.items():
                self.assertEqual(cache.read_bytes() if cache.exists() else None, before)
            for role in ("reference", "candidate"):
                build = root / (
                    "reference-build" if role == "reference" else "build-rust"
                )
                self.assertEqual(
                    (build / "CMakeCache.txt").read_text(), "fixture configuration"
                )
                command = next(
                    c for c in report["commands"] if c["name"] == f"{role}-build"
                )
                self.assertEqual(command["argv"][-2:], ["--parallel", "2"])
                self.assertIn(
                    "nested cargo jobs=2", (root / command["log"]).read_text()
                )


if __name__ == "__main__":
    unittest.main()
