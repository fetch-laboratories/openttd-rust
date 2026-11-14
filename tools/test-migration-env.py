#!/usr/bin/env python3
"""Check `migration.py env` and the host build RPATH against a fixture `.local`."""

import contextlib
import io
import json
import os
import shlex
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import migration

LIBRARIES = "deps/usr/lib/x86_64-linux-gnu"
PATH_LISTS = {
    "PATH",
    "LD_LIBRARY_PATH",
    "CMAKE_PREFIX_PATH",
    "CMAKE_INCLUDE_PATH",
    "CPATH",
    "PKG_CONFIG_PATH",
}
DIRECTORIES = {
    "CARGO_HOME",
    "RUSTUP_HOME",
    "PKG_CONFIG_SYSROOT_DIR",
    "CARGO_TARGET_DIR",
}
# The checkout's own Clippy configuration, which hand-run Clippy shares with
# the gate and the debt ratchet.
CHECKOUT = {"CLIPPY_CONF_DIR": migration.ROOT / migration.CLIPPY_CONFIGURATION}


class MigrationEnvTests(unittest.TestCase):
    def setUp(self) -> None:
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        # A space and a quote make unquoted or badly quoted exports fail to eval.
        self.local = Path(scratch.name).resolve() / "fixture .local's"
        self.local.mkdir()
        for name in ("LOCAL", "COMMON_LOCAL"):
            patch = mock.patch.object(migration, name, self.local)
            patch.start()
            self.addCleanup(patch.stop)
        self.build = self.local / "build-rust"
        patch = mock.patch.object(migration, "CANDIDATE_BUILD", self.build)
        patch.start()
        self.addCleanup(patch.stop)
        patch = mock.patch.object(migration, "CARGO_TARGET_DIR", self.build / "cargo")
        patch.start()
        self.addCleanup(patch.stop)
        # A `migration.py env` shell already holds the checkout's own values,
        # which `env` then leaves out; start each test without them.
        patch = mock.patch.dict(os.environ)
        patch.start()
        self.addCleanup(patch.stop)
        for name in CHECKOUT:
            os.environ.pop(name, None)

    def bootstrap(self) -> None:
        for directory in ("cargo/bin", "rustup", "deps/usr/bin", LIBRARIES):
            (self.local / directory).mkdir(parents=True)

    def env_output(self) -> str:
        output = io.StringIO()
        with (
            mock.patch.object(sys, "argv", ["migration.py", "env"]),
            contextlib.redirect_stdout(output),
        ):
            migration.main()
        return output.getvalue()

    def exports(self) -> dict[str, str]:
        exports = {}
        for line in self.env_output().splitlines():
            keyword, assignment = shlex.split(line)
            self.assertEqual(keyword, "export")
            name, value = assignment.split("=", 1)
            exports[name] = value
        return exports

    def inside(self, value: str) -> bool:
        return Path(value).resolve().is_relative_to(self.local)

    def test_exports_point_inside_bootstrapped_local(self) -> None:
        self.bootstrap()
        exports = self.exports()
        self.assertEqual(set(exports), PATH_LISTS | DIRECTORIES | set(CHECKOUT))
        for name, path in CHECKOUT.items():
            self.assertEqual(exports[name], str(path))
        for name in DIRECTORIES:
            self.assertTrue(self.inside(exports[name]), name)
        for name in PATH_LISTS:
            inherited = os.environ.get(name)
            added = exports[name]
            if inherited:
                self.assertTrue(added.endswith(os.pathsep + inherited), name)
                added = added[: -len(os.pathsep + inherited)]
            for entry in added.split(os.pathsep):
                self.assertTrue(self.inside(entry), f"{name}: {entry}")
        self.assertIn(str(self.local / LIBRARIES), exports["LD_LIBRARY_PATH"])

    def test_checkout_values_are_exported_unless_already_correct(self) -> None:
        for name, path in CHECKOUT.items():
            os.environ[name] = "/elsewhere"
            self.assertEqual(self.exports()[name], str(path))
            os.environ[name] = str(path)
            self.assertNotIn(name, self.exports())
            self.assertEqual(migration.environment()[name], str(path))

    def test_shell_eval_reproduces_exports(self) -> None:
        self.bootstrap()
        text = self.env_output()
        shell = subprocess.run(
            [
                "bash",
                "-c",
                'eval "$1" && exec "$2" -c "import json, os; print(json.dumps(dict(os.environ)))"',
                "bash",
                text,
                sys.executable,
            ],
            check=True,
            capture_output=True,
            text=True,
        )
        evaluated = json.loads(shell.stdout)
        exports = self.exports()
        self.assertTrue(exports)
        for name, value in exports.items():
            self.assertEqual(evaluated.get(name), value, name)

    def test_generated_headers_exported_without_bootstrap(self) -> None:
        with mock.patch.dict(os.environ, {}, clear=True):
            self.assertEqual(
                self.exports(),
                {
                    "CARGO_TARGET_DIR": str(self.build / "cargo"),
                    "CPATH": str(self.build / "generated-abi"),
                    **{name: str(path) for name, path in CHECKOUT.items()},
                },
            )

    @unittest.skipUnless(shutil.which("c++"), "needs a C++ compiler")
    def test_fork_header_finds_generated_record_with_reference_flags(self) -> None:
        generated = self.build / "generated-abi"
        generated.mkdir(parents=True)
        (generated / "abi-record-settings.h").write_text("struct Settings {};\n")
        fork = self.local / "fork"
        fork.mkdir()
        (fork / "services_ffi.h").write_text('#include "abi-record-settings.h"\n')
        source = self.local / "probe.cpp"
        source.write_text('#include "services_ffi.h"\nSettings settings;\n')
        subprocess.run(
            ["c++", "-fsyntax-only", "-I", str(fork), str(source)],
            env=migration.environment(),
            check=True,
            capture_output=True,
        )

    def test_build_rpath_follows_bootstrapped_libraries(self) -> None:
        self.assertEqual(migration.deps_build_options(), [])
        self.bootstrap()
        options = dict(
            option[2:].split("=", 1) for option in migration.deps_build_options()
        )
        self.assertEqual(options["CMAKE_BUILD_RPATH"], str(self.local / LIBRARIES))
        # DT_RUNPATH would not serve libicuuc.so's own load of libicudata.so.
        self.assertIn("--disable-new-dtags", options["CMAKE_EXE_LINKER_FLAGS"])


class LinkLocalTests(unittest.TestCase):
    """A worktree without its `.local` link gets one from the machine's hook."""

    def setUp(self) -> None:
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.main = Path(scratch.name).resolve() / "main"
        self.calls = Path(scratch.name).resolve() / "calls"
        env = {"GIT_CONFIG_GLOBAL": os.devnull, "GIT_CONFIG_NOSYSTEM": "1"}

        def git(*args: str, cwd: Path = self.main) -> None:
            subprocess.run(
                [
                    "git",
                    "-c",
                    "user.name=t",
                    "-c",
                    "user.email=t@example.invalid",
                    *args,
                ],
                cwd=cwd,
                env=os.environ | env,
                check=True,
                capture_output=True,
            )

        self.main.mkdir()
        git("init", "-q")
        git("commit", "-q", "--allow-empty", "-m", "Add: Seed")
        self.worktree = self.main.parent / "worktree"
        git("worktree", "add", "-q", "--detach", str(self.worktree))
        self.script = self.main / ".git/hooks/offload-local"
        self.script.parent.mkdir(exist_ok=True)
        self.stand_in("os.symlink(os.environ['TARGET'], '.local')\n")
        self.target = self.main.parent / "hdd"
        self.target.mkdir()

    def stand_in(self, body: str) -> None:
        """An offload-local that records each call, its cwd and argv, then runs body."""
        self.script.write_text(
            "import os, sys\n"
            f"open({str(self.calls)!r}, 'a').write(os.getcwd() + ' ' + ' '.join(sys.argv[1:]) + '\\n')\n"
            + body
        )

    def link(self, root: Path) -> str:
        stderr = io.StringIO()
        with (
            mock.patch.dict(os.environ, {"TARGET": str(self.target)}),
            contextlib.redirect_stderr(stderr),
        ):
            migration.link_local(root)
        return stderr.getvalue()

    def called(self) -> list[str]:
        return self.calls.read_text().splitlines() if self.calls.exists() else []

    def test_a_missing_link_is_made_once(self) -> None:
        output = self.link(self.worktree)
        self.assertEqual(self.called(), [f"{self.worktree} hook"])
        self.assertEqual(Path(os.readlink(self.worktree / ".local")), self.target)
        self.assertIn("linked", output)
        self.link(self.worktree)
        self.assertEqual(len(self.called()), 1)

    def test_an_existing_local_and_the_main_checkout_are_left_alone(self) -> None:
        (self.worktree / ".local").mkdir()
        self.link(self.worktree)
        self.link(self.main)
        (self.worktree / ".local").rmdir()
        (self.worktree / ".local").symlink_to(self.main.parent / "unmounted")
        self.link(self.worktree)
        self.assertEqual(self.called(), [])
        self.assertFalse((self.main / ".local").exists())

    def test_a_failing_hook_prints_one_line_and_continues(self) -> None:
        self.stand_in("sys.exit(3)\n")
        output = self.link(self.worktree)
        self.assertEqual(len(output.splitlines()), 1)
        self.assertIn("could not link", output)
        self.assertFalse((self.worktree / ".local").is_symlink())

    def test_no_hook_script_does_nothing(self) -> None:
        self.script.unlink()
        self.assertEqual(self.link(self.worktree), "")
        self.assertFalse(os.path.lexists(self.worktree / ".local"))


class Replaced(Exception):
    """Raised by a stubbed os.execvpe in place of replacing the test process."""


class MigrationExecTests(unittest.TestCase):
    """`exec` runs one command under the variables `env` exports."""

    def setUp(self) -> None:
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.scratch = Path(scratch.name).resolve()
        self.record = self.scratch / "record.json"
        bin_directory = self.scratch / "bin"
        bin_directory.mkdir()
        # A fake command that records what it was given and exits with $STATUS.
        recorder = bin_directory / "fake-recorder"
        recorder.write_text(
            f"#!{sys.executable}\n"
            "import json, os, sys\n"
            "names = ('CARGO_TARGET_DIR', 'CLIPPY_CONF_DIR', 'CPATH', 'PATH')\n"
            "record = {'argv': sys.argv[1:], 'cwd': os.getcwd(), 'pid': os.getpid(),\n"
            "          'env': {name: os.environ.get(name) for name in names}}\n"
            "open(os.environ['RECORD'], 'w').write(json.dumps(record))\n"
            "sys.exit(int(os.environ.get('STATUS', '0')))\n"
        )
        recorder.chmod(0o755)
        self.env = os.environ | {
            "PATH": f"{bin_directory}{os.pathsep}{os.environ.get('PATH', '')}",
            "RECORD": str(self.record),
        }

    def run_exec(
        self, *argv: str, cwd: Path, status: int = 0
    ) -> tuple[subprocess.Popen[str], str]:
        process = subprocess.Popen(
            [sys.executable, str(migration.ROOT / "tools/migration.py"), *argv],
            cwd=cwd,
            env=self.env | {"STATUS": str(status)},
            stderr=subprocess.PIPE,
            text=True,
        )
        _, stderr = process.communicate(timeout=60)
        return process, stderr

    def test_runs_argv_unchanged_in_cwd_with_driver_environment(self) -> None:
        argv = ["a b", "$HOME", "*", "--flag", ""]
        cwd = migration.ROOT / "tools"
        process, stderr = self.run_exec(
            "exec", "--", "fake-recorder", *argv, cwd=cwd, status=7
        )
        self.assertEqual(process.returncode, 7, stderr)
        record = json.loads(self.record.read_text())
        self.assertEqual(record["argv"], argv)
        self.assertEqual(Path(record["cwd"]).resolve(), cwd)
        # The command replaced migration.py rather than running as its child.
        self.assertEqual(record["pid"], process.pid)
        self.assertEqual(
            record["env"]["CARGO_TARGET_DIR"], str(migration.ROOT / "build-rust/cargo")
        )
        self.assertEqual(
            record["env"]["CLIPPY_CONF_DIR"],
            str(migration.ROOT / migration.CLIPPY_CONFIGURATION),
        )
        self.assertIn(
            str(migration.CANDIDATE_BUILD / "generated-abi"), record["env"]["CPATH"]
        )

    def test_runs_outside_any_checkout(self) -> None:
        process, stderr = self.run_exec("exec", "fake-recorder", cwd=self.scratch)
        self.assertEqual(process.returncode, 0, stderr)
        self.assertEqual(Path(json.loads(self.record.read_text())["cwd"]), self.scratch)

    def test_refuses_another_checkout(self) -> None:
        other = self.scratch / "other"
        subprocess.run(["git", "init", "-q", str(other)], check=True)
        process, stderr = self.run_exec("exec", "--", "fake-recorder", cwd=other)
        self.assertEqual(process.returncode, 1)
        self.assertIn(f"{other} is in the checkout", stderr)
        self.assertIn(f"cd into {migration.ROOT}", stderr)
        self.assertFalse(self.record.exists())

    def test_missing_command_names_the_driver_path(self) -> None:
        process, stderr = self.run_exec("exec", "--", "no-such-tool", cwd=self.scratch)
        self.assertEqual(process.returncode, 127)
        self.assertIn("cannot run no-such-tool", stderr)

    def test_usage_errors_run_nothing(self) -> None:
        for argv in (["exec"], ["exec", "--"], ["exec", "-x", "fake-recorder"]):
            process, stderr = self.run_exec(*argv, cwd=self.scratch)
            self.assertEqual(process.returncode, 2, argv)
            self.assertIn("exec -- CMD", stderr)
        process, stderr = self.run_exec("--jobs", "2", "exec", "true", cwd=self.scratch)
        self.assertEqual(process.returncode, 2)
        self.assertIn("put exec first", stderr)
        self.assertFalse(self.record.exists())

    def test_help_runs_nothing(self) -> None:
        output = io.StringIO()
        with (
            mock.patch.object(migration.os, "execvpe") as execvpe,
            contextlib.redirect_stdout(output),
        ):
            self.assertEqual(migration.run_agent_command(["exec", "--help"]), 0)
        execvpe.assert_not_called()
        self.assertIn("exec -- CMD", output.getvalue())

    def test_exec_and_env_share_one_environment(self) -> None:
        with (
            mock.patch.object(migration.os, "execvpe", side_effect=Replaced) as execvpe,
            mock.patch.object(migration, "other_checkout", return_value=None),
            self.assertRaises(Replaced),
        ):
            migration.run_agent_command(["exec", "--", "cargo", "fmt"])
        name, argv, env = execvpe.call_args.args
        self.assertEqual((name, argv), ("cargo", ["cargo", "fmt"]))
        self.assertEqual(env, migration.driver_environment())
        for line in migration.shell_exports().splitlines():
            variable, value = shlex.split(line)[1].split("=", 1)
            self.assertEqual(env[variable], value, variable)

    def test_driver_modes_are_left_to_main(self) -> None:
        self.assertIsNone(migration.run_agent_command([]))
        self.assertIsNone(migration.run_agent_command(["env"]))
        self.assertIsNone(migration.run_agent_command(["--jobs", "2", "exec"]))


if __name__ == "__main__":
    unittest.main()
