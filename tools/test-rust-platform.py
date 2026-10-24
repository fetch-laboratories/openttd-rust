#!/usr/bin/env python3
"""Check platform freshness against effective configuration and archive changes."""

import hashlib
import json
import os
import runpy
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import migration

platform = runpy.run_path(str(Path(__file__).with_name("check-rust-platform.py")))
freshness = platform["freshness"]


class FreshnessTests(unittest.TestCase):
    def test_deployment_and_profile_invalidation(self):
        for system, minimum in (
            ("Darwin", "11.0"),
            ("Darwin", "12.0"),
            ("Windows", ""),
        ):
            for invalidates in (True, False):
                with self.subTest(
                    system=system, minimum=minimum, invalidates=invalidates
                ):
                    self.trial(system, minimum, invalidates)

    def trial(self, system, minimum, invalidates):
        with tempfile.TemporaryDirectory() as directory:
            build = Path(directory)
            archive = build / "archive"
            stamp = build / "rust-build-configuration.txt"
            successful = build / "rust-build-configuration.txt.successful"
            configuration = {"platform": system, "deployment_target": minimum}
            current = configuration.copy()
            original = json.dumps(configuration).encode()
            stamp.write_bytes(original)
            archive.touch()
            successful.write_text(hashlib.sha256(original).hexdigest())
            previous = original
            trials = []

            def run(argv, log, env=None):
                nonlocal previous
                if "-S" in argv:
                    if system == "Darwin":
                        target = next(
                            arg.split("=", 1)[1]
                            for arg in argv
                            if str(arg).startswith("-DCMAKE_OSX_DEPLOYMENT_TARGET=")
                        )
                        trials.append(target)
                        current["deployment_target"] = target
                        effective = json.dumps(current).encode()
                    else:
                        units = (env or os.environ).get(
                            "CARGO_PROFILE_RELEASE_CODEGEN_UNITS"
                        )
                        trials.append(units)
                        effective = (
                            original if units is None else original + units.encode()
                        )
                    if invalidates:
                        stamp.write_bytes(effective)
                elif stamp.read_bytes() != previous:
                    modified = archive.stat().st_mtime_ns + 1_000_000_000
                    os.utime(archive, ns=(modified, modified))
                    previous = stamp.read_bytes()
                    successful.write_text(hashlib.sha256(previous).hexdigest())
                return SimpleNamespace(returncode=0)

            with (
                patch.dict(os.environ, {}, clear=True),
                patch.dict(freshness.__globals__, run=run),
                patch.object(migration, "rust_archive", return_value=archive),
                patch.object(
                    migration, "rust_configuration", side_effect=lambda _: current
                ),
            ):
                if invalidates:
                    freshness(build, build, configuration)
                    changed = "11.0" if minimum == "12.0" else "12.0"
                    expected = (
                        [minimum, changed, minimum, minimum]
                        if system == "Darwin"
                        else [None, "2", None, None]
                    )
                    self.assertEqual(trials, expected)
                    self.assertEqual(stamp.read_bytes(), original)
                else:
                    with self.assertRaises(RuntimeError):
                        freshness(build, build, configuration)


if __name__ == "__main__":
    unittest.main()
