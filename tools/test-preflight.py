#!/usr/bin/env python3
"""Check preflight's reading of Quick and the pinned commit checker checkout."""

import importlib.util
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from collections.abc import Generator
from contextlib import contextmanager
from functools import partial
from pathlib import Path
from subprocess import CompletedProcess
from types import ModuleType
from typing import Any
from unittest.mock import patch

import git_hooks
import preflight
import preflight_stamp
import vcs
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
