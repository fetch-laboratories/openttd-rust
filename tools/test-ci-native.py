#!/usr/bin/env python3
"""Exercise complete shard coverage and native packaging inputs."""

import copy
import json
import tarfile
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import ci_native
from simulation import core, industries, runtime


class ShardTests(unittest.TestCase):
    def fixture(self, folder):
        value = {
            "schema_version": 1,
            "candidate_commit": "head",
            "inventory": ["a", "b"],
            "shards": [{"id": 0, "names": ["a"]}, {"id": 1, "names": ["b"]}],
        }
        reports = [
            {
                "shard": index,
                "passed": True,
                "results": [{"scenario": name, "passed": True}],
                "candidate_commit": "head",
                "mode": "reference-vs-candidate",
                "benchmark_repetitions": 0,
            }
            for index, name in enumerate(("a", "b"))
        ]
        return value, reports

    def check(self, mutate=None, mutate_plan=None):
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(ci_native.migration, "git", return_value="head"),
            patch.object(
                ci_native.core,
                "scenario_list",
                return_value=[{"name": "a"}, {"name": "b"}],
            ),
        ):
            value, reports = self.fixture(Path(directory))
            if mutate:
                mutate(reports)
            if mutate_plan:
                mutate_plan(value)
            return ci_native.aggregate(value, reports)

    def test_complete_inventory_passes(self):
        self.assertEqual(self.check(), {"passed": True, "scenarios": 2, "shards": 2})

    def test_missing_duplicate_empty_failed_and_wrong_build_shards_fail(self):
        changes = [
            lambda r: r.pop(),
            lambda r: r.append(copy.deepcopy(r[0])),
            lambda r: r[1].update(shard=0),
            lambda r: r[1].update(results=[]),
            lambda r: r[1].update(results=[{"scenario": "a", "passed": True}]),
            lambda r: r[1].update(passed=False),
            lambda r: r[1]["results"][0].update(passed=False),
            lambda r: r[1].update(candidate_commit="other"),
            lambda r: r[1].update(mode="reference-vs-reference"),
            lambda r: r[1].update(benchmark_repetitions=3),
        ]
        for index, change in enumerate(changes):
            with self.subTest(change=index), self.assertRaises(ValueError):
                self.check(mutate=change)

    def test_plan_cannot_omit_scenarios_or_shards(self):
        for change in (
            lambda v: v.update(inventory=["a"]),
            lambda v: v["shards"].pop(),
            lambda v: v["shards"][1].update(names=[]),
            lambda v: v["shards"][1].update(id=7),
            lambda v: v.update(candidate_commit="other"),
        ):
            with self.assertRaises(ValueError):
                self.check(mutate_plan=change)

    def test_real_partition_is_complete_and_keeps_fixture_families_whole(self):
        scenarios = ci_native.core.scenario_list(False)
        weights = json.loads(ci_native.TIMINGS.read_text())["seconds"]
        shards = ci_native.partition(scenarios, weights, 3)
        self.assertEqual(
            sorted(n for s in shards for n in s["names"]),
            sorted(s["name"] for s in scenarios),
        )
        for family in ("effects", "disasters", "aircraft-control"):
            names = {
                s["name"] for s in scenarios if ci_native.fixture_family(s) == family
            }
            self.assertEqual(
                sum(bool(names.intersection(s["names"])) for s in shards), 1
            )
        self.assertTrue(all(s["names"] for s in shards))
        self.assertLess(
            max(s["seconds"] for s in shards) / min(s["seconds"] for s in shards), 1.02
        )
        for count in (0, -1, len(scenarios) + 1):
            with self.assertRaises(ValueError):
                ci_native.partition(scenarios, weights, count)


class FrozenRuntimeTests(unittest.TestCase):
    def test_frozen_industry_console_needs_no_native_build_tree(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bundle, output = root / "bundle", root / "output"
            output.mkdir()
            builds = {}
            for role in ("reference", "candidate"):
                build = bundle / f"{role}-runtime"
                build.mkdir(parents=True)
                (build / ("openttd" if role == "reference" else "openttd-rust")).touch()
                for module in core.scenario_modules():
                    if hasattr(module, "adapter"):
                        (build / module.__name__.rsplit(".", 1)[1]).mkdir()
                (build / "industries/openttd-industries").write_bytes(role.encode())
                builds[role] = output / build.name
            args = SimpleNamespace(
                frozen_runtimes=bundle,
                self=False,
                candidate=None,
                jobs=1,
                limit=20,
                timeout=10,
                benchmark=None,
                instructions=False,
            )
            scenario = next(
                s for s in industries.scenarios(False) if s["industry"] == "resolver"
            )
            with (
                patch.object(ci_native.migration, "git", return_value="head"),
                patch.object(ci_native.migration, "environment", return_value={}),
                patch.object(ci_native.migration, "REFERENCE_BUILD", root / "absent"),
            ):
                core.run_simulation(args, [], ci_native.migration, output, None, "test")
                binaries = {
                    role: build / ("openttd" if role == "reference" else "openttd-rust")
                    for role, build in builds.items()
                }
                prepared = industries.prepare(
                    scenario, binaries, builds, output, 10, {}, {}
                )
            for role, executable in prepared["executables"].items():
                self.assertEqual(executable.read_bytes(), role.encode())
            with self.assertRaisesRegex(RuntimeError, "no branch-witnesses.json"):
                industries.check(
                    dict(prepared, witnesses=True),
                    {"dir": output},
                    "snapshots",
                    "candidate",
                    {},
                )

    def test_native_metadata_relocates_without_touching_binary_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            staging, worker = root / "source", root / "worker"
            (staging / ".local").mkdir(parents=True)
            (staging / "build-rust").mkdir()
            (staging / ".local/native-export-root.json").write_text(
                json.dumps({"root": str(staging), "common_root": str(staging.parent)})
            )
            (staging / "build-rust/CMakeCache.txt").write_text(
                f"RUST_TARGET_DIR={staging}/build-rust/cargo\n"
            )
            (staging / "build-rust/input.o").write_bytes(b"binary bytes stay exact")
            archive = root / "support.tar"
            with tarfile.open(archive, "w") as stream:
                stream.add(staging / ".local", arcname=".local")
                stream.add(staging / "build-rust", arcname="build-rust")
            worker.mkdir()
            with patch.object(ci_native, "ROOT", worker):
                ci_native.unpack(archive)
            self.assertEqual(
                (worker / "build-rust/CMakeCache.txt").read_text(),
                f"RUST_TARGET_DIR={worker}/build-rust/cargo\n",
            )
            self.assertEqual(
                (worker / "build-rust/input.o").read_bytes(), b"binary bytes stay exact"
            )

    def test_packaged_adapter_is_copied_without_relinking(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "runtime/fleet/openttd-fleet"
            source.parent.mkdir(parents=True)
            source.write_bytes(b"adapter executable")
            target = runtime.prepared_adapter(
                "fleet",
                "reference",
                {"reference": source.parent.parent},
                root / "output",
            )
            self.assertEqual(target.read_bytes(), source.read_bytes())
            self.assertEqual(list(target.parent.iterdir()), [target])
            self.assertIsNone(
                runtime.prepared_adapter(
                    "orders",
                    "reference",
                    {"reference": source.parent.parent},
                    root / "missing",
                )
            )


if __name__ == "__main__":
    unittest.main()
