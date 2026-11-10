#!/usr/bin/env python3
"""Check preflight's reading of Quick and the pinned commit checker checkout."""

import sys
import tempfile
import unittest
from pathlib import Path
from subprocess import CompletedProcess
from typing import Any
from unittest.mock import patch

import preflight
import yaml


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
            ]
            self.assertEqual(preflight.main(), 1)
            commands = [call.args[0] for call in run.call_args_list]
            self.assertEqual(
                commands[0], ["sh", "/hooks/hooks/check-commits.sh", "sha..sha"]
            )
            self.assertEqual(
                [command[-1] for command in commands[1:]], ["false", "true"]
            )

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


if __name__ == "__main__":
    unittest.main()
