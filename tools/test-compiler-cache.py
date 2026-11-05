#!/usr/bin/env python3
"""Check shared cache provenance and explicit return to ordinary CMake mode."""

import shutil
import tempfile
import unittest
from pathlib import Path

import migration


class CompilerCacheTests(unittest.TestCase):
    def test_provenance_and_ambient_policy_overrides(self) -> None:
        original = {
            "PATH": "/bin",
            "CCACHE_SLOPPINESS": "time_macros",
            "CCACHE_BASEDIR": "/tmp",
        }
        reference = migration.compiler_cache_environment(original, "reference")
        candidate = migration.compiler_cache_environment(original, "candidate")
        self.assertNotEqual(reference["CCACHE_DIR"], candidate["CCACHE_DIR"])
        self.assertEqual(reference["PATH"], "/bin")
        self.assertNotIn("CCACHE_SLOPPINESS", reference)
        # Ambient base_dir is replaced by the role root that holds source and build.
        self.assertEqual(
            reference["CCACHE_BASEDIR"], str(migration.COMMON_ROOT.resolve())
        )
        self.assertEqual(candidate["CCACHE_BASEDIR"], str(migration.ROOT.resolve()))
        # Every worktree of a clone shares one cache per role.
        self.assertEqual(
            Path(candidate["CCACHE_DIR"]).parent,
            migration.COMMON_LOCAL / "compiler-cache",
        )
        self.assertEqual(original["CCACHE_SLOPPINESS"], "time_macros")
        self.assertEqual(
            migration.compiler_cache_environment(original, "candidate", bypass=True)[
                "CCACHE_DISABLE"
            ],
            "1",
        )
        with self.assertRaises(ValueError):
            migration.compiler_cache_environment(original, "../reference")

    def test_disabled_mode_clears_launchers_and_pch_override(self) -> None:
        options = migration.compiler_cache_options(None)
        self.assertIn("-DCMAKE_C_COMPILER_LAUNCHER=", options)
        self.assertIn("-DCMAKE_CXX_COMPILER_LAUNCHER=", options)
        self.assertIn("-DCMAKE_DISABLE_PRECOMPILE_HEADERS=OFF", options)
        with tempfile.TemporaryDirectory() as directory:
            build = Path(directory)
            (build / "CMakeCache.txt").write_text(
                "CMAKE_C_COMPILER_LAUNCHER:STRING=/usr/bin/ccache\nCMAKE_CXX_COMPILER_LAUNCHER:STRING=/usr/bin/ccache\nCMAKE_DISABLE_PRECOMPILE_HEADERS:BOOL=ON\n"
            )
            with self.assertRaises(RuntimeError):
                migration.verify_compiler_cache_options(build, None)
            migration.verify_compiler_cache_options(build, "/usr/bin/ccache")

    @unittest.skipUnless(
        shutil.which("ccache", path=migration.environment()["PATH"]),
        "ccache not installed",
    )
    def test_cache_key_ignores_workflow_and_driver_edits(self) -> None:
        identity = migration.compiler_cache_compatibility()
        self.assertEqual(set(identity["policy"]), {"migration/ccache.conf"})
        self.assertTrue(identity["prefix"].startswith("compiler-v1-"))


if __name__ == "__main__":
    unittest.main()
