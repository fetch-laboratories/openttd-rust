#!/usr/bin/env python3
"""Check preflight's reading of Quick and the pinned commit checker checkout."""

import importlib.util
import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from collections.abc import Generator
from contextlib import contextmanager
from pathlib import Path
from subprocess import CompletedProcess
from types import ModuleType
from typing import Any
from unittest.mock import patch

import git_hooks
import preflight
import preflight_stamp
import yaml

REPO = Path(__file__).resolve().parents[1]


def workflow(*steps: dict[str, Any]) -> dict[str, Any]:
    return {"jobs": {preflight.JOB: {"steps": list(steps)}}}


class WorkflowTests(unittest.TestCase):
    def test_every_quick_run_step_is_run_or_skipped(self) -> None:
        quick = yaml.safe_load(preflight.WORKFLOW.read_text())
        names = [
            step.get("name", "")
            for step in quick["jobs"][preflight.JOB]["steps"]
            if "run" in step
        ]
        ran = [step.name for step in preflight.workflow_steps(quick, "base")]
        self.assertEqual(
            sorted(ran + list(preflight.SKIPPED)), sorted(names), "unlisted step"
        )
        self.assertIn("Python complexity debt and lint policy", ran)

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
                preflight.workflow_steps(workflow(entry), "base")

    def test_step_selection(self) -> None:
        names = ["Rust checks", "Ruff ratchet", "Strings"]
        self.assertEqual(preflight.selected(names, None), names)
        self.assertEqual(
            preflight.selected(names, ["ruff", "STRINGS"]), ["Ruff ratchet", "Strings"]
        )
        with self.assertRaisesRegex(ValueError, "no step matches 'nothing'"):
            preflight.selected(names, ["nothing"])


class RunTests(unittest.TestCase):
    def test_every_step_runs_and_any_failure_fails(self) -> None:
        quick = workflow(
            {"name": "First", "run": "false"}, {"name": "Second", "run": "true"}
        )
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(preflight, "hooks_checkout", return_value=Path("/hooks")),
            patch.object(preflight, "git", return_value="sha"),
            patch.object(preflight.yaml, "safe_load", return_value=quick),
            patch.object(preflight.migration, "LOCAL", Path(directory)),
            patch.object(preflight.subprocess, "run") as run,
            patch.object(sys, "argv", ["preflight.py"]),
            patch("builtins.print"),
        ):
            run.side_effect = [
                CompletedProcess([], 0),
                CompletedProcess([], 1),
                CompletedProcess([], 0),
                CompletedProcess([], 0),
            ]
            self.assertEqual(preflight.main(), 1)
            commands = [call.args[0] for call in run.call_args_list]
            self.assertEqual(
                commands[0], ["sh", "/hooks/hooks/check-commits.sh", "sha..sha"]
            )
            self.assertEqual(
                [command[-1] for command in commands[1:]], ["false", "true", "sha"]
            )
            self.assertEqual(commands[-1][:2], ["python3", "tools/compile-check.py"])

    def test_selected_steps_skip_the_commit_checker(self) -> None:
        quick = workflow({"name": "Only", "run": "true"})
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(preflight, "hooks_checkout") as hooks,
            patch.object(preflight, "git", return_value="sha"),
            patch.object(preflight.yaml, "safe_load", return_value=quick),
            patch.object(preflight.migration, "LOCAL", Path(directory)),
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
            match.removeprefix("docs/").removesuffix(".md")
            for path in sources
            for match in re.findall(
                r"\$\{CMAKE_SOURCE_DIR\}/(docs/[^\s\"@/]+\.md)", path.read_text()
            )
        }
        self.assertEqual(set(preflight_stamp.BUILT_DOCS), built)
        plan = (root / ".github/workflows/ci-request.yml").read_text()
        listed = re.search(r"const builtDocs = \[(.*?)\];", plan, re.S)
        assert listed
        self.assertEqual(
            re.findall(r"'([^']+)'", listed[1]), list(preflight_stamp.BUILT_DOCS)
        )

    def test_a_stamp_covers_its_tree_and_markdown_only_differences(self) -> None:
        with scratch_repo() as repo:
            code = commit(repo, "Add: Code", {"tools/x.py": "x = 1\n"})
            preflight_stamp.record("base", code)
            docs = commit(
                repo, "Doc: Note", {"docs/note.md": "note\n", "AGENTS.md": "a\n"}
            )
            self.assertEqual((preflight_stamp.stamp_for(docs) or {}).get("head"), code)
            built = commit(repo, "Doc: Fonts", {"docs/fonts.md": "built\n"})
            self.assertIsNone(preflight_stamp.stamp_for(built))
            subprocess.run(["git", "reset", "-q", "--hard", docs], cwd=repo, check=True)
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
            patch.object(check, "generated_abi_changed", return_value=False) as abi,
            patch.object(check, "jobs", return_value=[]),
            patch("builtins.print"),
        ):
            for name in ("CMakeCache.txt", "compile_commands.json"):
                (Path(directory) / name).write_text("")
            (Path(directory) / "out/generated-abi").mkdir(parents=True)
            self.assertEqual(check.check("base"), 0)
            abi.assert_called_once()


if __name__ == "__main__":
    unittest.main()
