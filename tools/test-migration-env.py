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


if __name__ == "__main__":
    unittest.main()
