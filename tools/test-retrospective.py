#!/usr/bin/env python3
"""Exercise read-only evidence binding and overlapping milestone arithmetic."""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from argparse import Namespace
from dataclasses import replace
from pathlib import Path
from typing import Any
from unittest import mock

import base_update
import rebuild_reach
import retrospective as facts

SCRIPT = Path(__file__).resolve().with_name("retrospective.py")


class RetrospectiveTests(unittest.TestCase):
    def pull(self, number: int = 1) -> facts.PullFacts:
        return facts.PullFacts(
            number,
            f"https://example.invalid/pull/{number}",
            "open",
            "a" * 40,
            "b" * 40,
            "rust-migration",
            "b" * 40,
            None,
            "unknown",
            True,
            None,
            None,
            [],
            ["tools/shared.py"],
            [{"event": "created_at", "at": "2026-11-06T00:00:00Z", "cause": None}],
        )

    def test_current_head_carried_status_and_stale_review_are_separate(self) -> None:
        pull = {
            "number": 1,
            "html_url": "https://example.invalid/pr/1",
            "state": "open",
            "head": {"sha": "a" * 40},
            "base": {"sha": "b" * 40, "ref": "rust-migration"},
            "mergeable": None,
            "created_at": "2026-11-06T00:00:00Z",
        }
        tip = {"commit": {"sha": "e" * 40}}
        status = {
            "context": "Full validation",
            "state": "success",
            "description": "Carried from cccccccccc: non-Markdown tree unchanged",
            "target_url": "https://example.invalid/run/1",
        }
        arrays = {
            "issues/1/comments?per_page=100": [
                {
                    "body": "Reviewed commit: `ccccccc`\nAgent: /root/review",
                    "html_url": "https://example.invalid/report",
                    "updated_at": "2026-11-06T01:00:00Z",
                }
            ],
            "pulls/1/reviews?per_page=100": [],
            "issues/1/timeline?per_page=100": [
                {
                    "event": "reviewed",
                    "submitted_at": "2026-11-06T01:10:00Z",
                    "html_url": "https://example.invalid/review",
                }
            ],
            f"commits/{'a' * 40}/statuses?per_page=100": [status],
            "pulls/1/files?per_page=100": [{"filename": "tools/shared.py"}],
        }
        with (
            mock.patch.object(facts.ci, "api", side_effect=[pull, tip, pull, tip]),
            mock.patch.object(
                facts, "pages", side_effect=lambda repo, path: arrays[path]
            ),
            mock.patch.object(
                facts.ci, "passed_run", return_value=status["target_url"]
            ),
        ):
            result = facts.collect_pull("owner/repo", 1, latency=True)
        self.assertEqual(result.head, "a" * 40)
        self.assertEqual((result.base, result.base_tip), ("b" * 40, "e" * 40))
        self.assertIsNone(result.mergeable)
        self.assertTrue(result.stable_snapshot)
        self.assertEqual(
            next(m["at"] for m in result.milestones if m["event"] == "reviewed"),
            "2026-11-06T01:10:00Z",
        )
        self.assertEqual(result.passing_full_url, status["target_url"])
        assert result.full is not None
        self.assertEqual(result.full["description"], status["description"])
        self.assertFalse(result.reports[0].matches_head)
        self.assertEqual(result.reports[0].url, "https://example.invalid/report")
        other = replace(result, number=2, files=["tools/shared.py", "tools/other.py"])
        self.assertEqual(
            facts.overlaps([result, other]),
            [{"prs": [1, 2], "files": ["tools/shared.py"]}],
        )

        current = dict(pull)

        def changed_files(_repo: str, path: str) -> list[dict[str, Any]]:
            if path == "pulls/1/files?per_page=100":
                current["head"] = {"sha": "d" * 40}
            return arrays[path]

        moved = {"commit": {"sha": "f" * 40}}
        for responses, listing in (
            ([pull, tip, current, tip], changed_files),
            ([pull, tip, pull, moved], lambda _repo, path: arrays[path]),
        ):
            with (
                mock.patch.object(facts.ci, "api", side_effect=responses),
                mock.patch.object(facts, "pages", side_effect=listing),
                mock.patch.object(
                    facts.ci, "passed_run", return_value=status["target_url"]
                ),
            ):
                changed = facts.collect_pull("owner/repo", 1)
            self.assertFalse(changed.stable_snapshot)

    def test_overlap_is_elapsed_union_and_unknown_milestones_stay_absent(self) -> None:
        first = self.pull()
        first.milestones.append({"event": "merged_at", "at": "2026-11-06T02:00:00Z"})
        second = self.pull(2)
        second.milestones = [
            {"event": "created_at", "at": "2026-11-06T01:00:00Z"},
            {"event": "merged_at", "at": "2026-11-06T03:00:00Z"},
        ]
        open_pull = self.pull(3)
        runs = [
            {
                "display_title": "CI full PR 1 request-token",
                "html_url": "https://example.invalid/run/1",
                "created_at": "2026-11-06T01:00:00Z",
                "run_started_at": "2026-11-06T01:01:00Z",
                "updated_at": "2026-11-06T01:20:00Z",
                "conclusion": "success",
                "head_sha": "protected-branch-not-pr-head",
            }
        ]
        with mock.patch.object(
            facts.ci, "gh", return_value=json.dumps([{"workflow_runs": runs}])
        ):
            result = facts.latency_facts("owner/repo", [first, second, open_pull])
        self.assertEqual(
            result["completed_pr_intervals"],
            {
                "summed_per_pr_seconds": 14400,
                "elapsed_union_seconds": 10800,
            },
        )
        self.assertIsNone(result["prs"][2]["created_to_merge_seconds"])
        milestones = result["prs"][0]["milestones"]
        self.assertFalse(any(m["event"] == "review_report" for m in milestones))
        self.assertTrue(all(m.get("cause") is None for m in milestones))
        self.assertFalse(
            next(m for m in milestones if m["event"] == "full_updated_at")[
                "current_head_status_url_matches"
            ]
        )
        with self.assertRaises(ValueError):
            facts.elapsed([("2026-11-06T03:00:00Z", "2026-11-06T01:00:00Z")])

    def test_stocktake_invokes_one_metric_for_both_endpoints(self) -> None:
        first = {
            "metric_version": 2,
            "metric_implementation": "same-code-hash",
            "metric_scope": "same-source-inventory",
            "baseline": "original",
            "head": "first",
            "original_total": 10,
            "whole_fork_retired": 3,
            "compiled_cpp": 9,
            "fork_cpp": 12,
            "candidate_cpp_upper_bound": 11,
            "rust": 4,
            "tooling": 2,
        }
        second = dict(first, head="second", whole_fork_retired=4, compiled_cpp=8)
        with mock.patch.object(
            facts.subprocess,
            "check_output",
            side_effect=map(json.dumps, [first, second]),
        ) as metric:
            result = facts.stocktake("start", "end")
        commands = [call.args[0] for call in metric.call_args_list]
        self.assertEqual(commands[0][:-1], commands[1][:-1])
        self.assertEqual([command[-1] for command in commands], ["start", "end"])
        self.assertEqual(result["heads"], ["first", "second"])
        self.assertEqual(
            next(r for r in result["rows"] if r["metric"] == "compiled_cpp")["delta"],
            -1,
        )
        second["metric_version"] = 1
        with (
            mock.patch.object(
                facts.subprocess,
                "check_output",
                side_effect=map(json.dumps, [first, second]),
            ),
            self.assertRaises(ValueError),
        ):
            facts.stocktake("start", "end")


class BaseUpdateTests(unittest.TestCase):
    """Fixture histories: `main` is the PR branch, `origin/rust-migration` the base."""

    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.repo = Path(self.temp.name)
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "user.name", "Fixture")
        self.write(
            {
                "tools/x.py": "a\nb\nc\n",
                "docs/x.md": "a\nb\nc\n",
                "tools/old.py": "".join(f"line {n}\n" for n in range(9)),
            }
        )
        self.commit("init")
        self.git("branch", "base")

    def tearDown(self) -> None:
        self.temp.cleanup()

    def git(self, *arguments: str) -> str:
        return subprocess.run(
            ["git", *arguments],
            cwd=self.repo,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()

    def write(self, files: dict[str, str]) -> None:
        for name, text in files.items():
            (self.repo / name).parent.mkdir(parents=True, exist_ok=True)
            (self.repo / name).write_text(text)

    def commit(self, title: str) -> str:
        self.git("add", "-A")
        self.git("commit", "-q", "-m", title)
        return self.git("rev-parse", "HEAD")

    def on(self, branch: str, files: dict[str, str], title: str) -> None:
        self.git("checkout", "-q", branch)
        self.write(files)
        self.commit(title)
        self.git("checkout", "-q", "main")

    def merge(
        self, branch: str = "base", resolved: dict[str, str] | None = None
    ) -> str:
        """Merge `branch` into main, resolving conflicts with `resolved`."""
        subprocess.run(
            ["git", "merge", "-q", "--no-ff", "--no-commit", branch],
            cwd=self.repo,
            capture_output=True,
        )
        self.write(resolved or {})
        sha = self.commit("Update: merge")
        self.git("update-ref", "refs/remotes/origin/rust-migration", "base")
        return sha

    def conflict(self, path: str) -> None:
        self.on("base", {path: "a\nBASE\nc\n"}, "base side")
        self.on("main", {path: "a\nMINE\nc\n"}, "branch side")

    def cli(self, *arguments: str, where: str = "") -> tuple[int, str]:
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "base-update", *arguments],
            cwd=self.repo / where,
            capture_output=True,
            text=True,
            env={**os.environ, "GIT_CONFIG_GLOBAL": os.devnull},
        )
        return result.returncode, result.stdout + result.stderr

    def classify(self, sha: str = "HEAD") -> base_update.Merge:
        return base_update.classify(self.repo, sha, "origin/rust-migration")

    def test_clean_merge_needs_only_ci(self) -> None:
        self.on("base", {"tools/x.py": "A\nb\nc\n"}, "base side")
        self.on("main", {"tools/x.py": "a\nb\nC\n"}, "branch side")
        sha = self.merge()
        code, out = self.cli()
        self.assertEqual(code, 0, out)
        self.assertIn(f"Base update {sha[:10]}: clean; needs only CI", out)
        self.assertIn("docs/workflow.md#integration", out)

    def test_conflict_kept_on_both_sides_needs_only_ci(self) -> None:
        self.conflict("tools/x.py")
        self.merge(resolved={"tools/x.py": "a\nMINE\nBASE\nc\n"})
        code, out = self.cli("--json")
        self.assertEqual(code, 0, out)
        result = json.loads(out)
        self.assertEqual(result["merges"][0]["classification"], "both-sides")
        self.assertEqual(result["merges"][0]["label"], "both-sides")
        self.assertEqual(result["schema_version"], 1)
        self.assertIn("both-sides; needs only CI", result["status_line"])

    def test_conflict_resolved_with_an_edit_needs_review(self) -> None:
        self.conflict("tools/x.py")
        sha = self.merge(resolved={"tools/x.py": "a\nMERGED\nc\n"})
        code, out = self.cli(sha)
        self.assertEqual(code, base_update.EXIT_NEEDS_REVIEW, out)
        self.assertIn("needs review", out)
        self.assertIn("tools/x.py: conflict resolved with an edit", out)
        self.assertIn(
            "a fresh reviewer checks the conflict resolution in tools/x.py", out
        )
        self.assertIn(f"git show --remerge-diff {sha[:10]}", out)

    def test_relative_diff_config_and_a_subdirectory_keep_root_paths(self) -> None:
        self.conflict("tools/x.py")
        self.merge(resolved={"tools/x.py": "a\nMERGED\nc\n"})
        self.git("config", "diff.relative", "true")
        code, out = self.cli(where="tools")
        self.assertEqual(code, base_update.EXIT_NEEDS_REVIEW, out)
        self.assertIn("in tools/x.py", out)

    def test_reordered_sides_and_merge_time_edits_need_review(self) -> None:
        self.conflict("tools/x.py")
        self.merge(resolved={"tools/x.py": "a\nBASE\nMINE\nc\n"})
        self.assertEqual(self.classify().classification, base_update.NEEDS_REVIEW)
        self.on("base", {"docs/x.md": "a\nb\nc\nd\n"}, "base docs")
        self.merge(resolved={"tools/old.py": "edited during the merge\n"})
        merge = self.classify()
        self.assertEqual(merge.classification, base_update.NEEDS_REVIEW)
        self.assertEqual(
            [(f.path, f.reason) for f in merge.findings],
            [("tools/old.py", "merge-time edit outside any conflict")],
        )

    def test_conflict_outside_guarded_paths_is_clean(self) -> None:
        self.conflict("docs/x.md")
        self.merge(resolved={"docs/x.md": "a\nREWRITTEN\nc\n"})
        merge = self.classify()
        self.assertEqual(merge.classification, base_update.CLEAN)
        self.assertEqual(
            [(f.path, f.guarded) for f in merge.findings], [("docs/x.md", False)]
        )

    def test_rename_follows_the_guarded_path(self) -> None:
        lines = [f"line {n}\n" for n in range(9)]
        self.on(
            "base", {"tools/old.py": "".join(lines[:4] + ["BASE\n"] + lines[5:])}, "b"
        )
        self.git("mv", "tools/old.py", "tools/new name.py")
        self.write({"tools/new name.py": "".join(lines[:4] + ["MINE\n"] + lines[5:])})
        self.commit("rename on the branch")
        kept = lines[:4] + ["MINE\n", "BASE\n"] + lines[5:]
        self.merge(resolved={"tools/new name.py": "".join(kept)})
        merge = self.classify()
        self.assertEqual(merge.classification, base_update.BOTH_SIDES)
        self.assertEqual([f.path for f in merge.findings], ["tools/new name.py"])
        self.git("reset", "-q", "--hard", "HEAD^")
        edited = lines[:4] + ["EDITED\n"] + lines[5:]
        self.merge(resolved={"tools/new name.py": "".join(edited)})
        self.assertEqual(self.classify().classification, base_update.NEEDS_REVIEW)

    def test_merge_of_another_branch_is_not_a_base_update(self) -> None:
        self.git("branch", "feature")
        self.on("feature", {"docs/x.md": "a\nfeature\nc\n"}, "feature")
        sha = self.merge("feature")
        self.git("update-ref", "refs/remotes/origin/rust-migration", "base")
        code, out = self.cli(sha)
        self.assertEqual(code, base_update.EXIT_NOT_BASE, out)
        self.assertIn("not a base update", out)
        self.assertIn("is not on origin/rust-migration", out)
        code, out = self.cli("--base", "feature", sha)
        self.assertEqual(code, 0, out)

    def test_one_parent_commit_is_not_a_base_update(self) -> None:
        self.on("main", {"tools/x.py": "one parent\n"}, "ordinary commit")
        self.git("update-ref", "refs/remotes/origin/rust-migration", "base")
        code, out = self.cli()
        self.assertEqual(code, base_update.EXIT_NOT_BASE, out)
        self.assertIn("it has 1 parent; a base update has two", out)
        self.assertIn("--since REVIEWED_SHA", out)

    def test_since_classifies_every_merge_after_the_reviewed_head(self) -> None:
        reviewed = self.git("rev-parse", "HEAD")
        self.on("base", {"docs/x.md": "a\nb\nc\nbase\n"}, "base docs")
        first = self.merge()
        self.conflict("tools/x.py")
        second = self.merge(resolved={"tools/x.py": "a\nMERGED\nc\n"})
        self.write({"tools/x.py": "a\nMERGED\nc\nlater\n"})
        self.commit("a later commit")
        code, out = self.cli("--since", reviewed)
        self.assertEqual(code, base_update.EXIT_NEEDS_REVIEW, out)
        self.assertIn(f"{first[:10]} parents", out)
        self.assertIn(
            f"Base updates since {reviewed[:10]}: {first[:10]} clean,"
            f" {second[:10]} needs review",
            out,
        )
        code, out = self.cli("--since", second)
        self.assertEqual(code, 0, out)
        self.assertIn("No merges since", out)
        code, out = self.cli("--since", second, "--json")
        self.assertEqual(code, 0, out)
        result = json.loads(out)
        self.assertEqual(
            (result["merges"], result["review_files"], result["exit_code"]),
            ([], [], 0),
        )
        self.assertIn("No merges since", result["status_line"])
        code, out = self.cli("--since", reviewed, second)
        self.assertEqual(code, 2, out)
        self.assertIn("not both", out)
        self.git("branch", "elsewhere", reviewed)
        self.on("elsewhere", {"docs/x.md": "elsewhere\n"}, "not on this branch")
        code, out = self.cli("--since", "elsewhere")
        self.assertEqual(code, 2, out)
        self.assertIn("pass the reviewed head of this branch", out)

    def test_unparseable_or_unsupported_remerge_output_needs_review(self) -> None:
        self.conflict("tools/x.py")
        self.merge(resolved={"tools/x.py": "a\nMINE\nBASE\nc\n"})
        broken = "diff --git a/tools/x.py b/tools/x.py\n@@ -1,3 +1,2 @@\n a\n"
        for output in (broken, "warning: something new\n"):
            with mock.patch.object(base_update, "remerge", return_value=output):
                merge = self.classify()
            self.assertEqual(merge.label, "needs review: could not classify")
        failed = subprocess.CompletedProcess([], 128, b"", b"unknown option")
        real = base_update.git_bytes

        def old_git(cwd: Path, *arguments: str) -> subprocess.CompletedProcess[bytes]:
            return failed if "--remerge-diff" in arguments else real(cwd, *arguments)

        with mock.patch.object(base_update, "git_bytes", side_effect=old_git):
            merge = self.classify()
        self.assertTrue(merge.unclassified)
        self.assertIn("unknown option", merge.detail)
        args = Namespace(
            merge="HEAD", since=None, base="origin/rust-migration", json=False
        )
        with (
            mock.patch.object(base_update, "git_bytes", side_effect=old_git),
            mock.patch("builtins.print") as printed,
        ):
            code = base_update.run(args, self.repo)
        self.assertEqual(code, base_update.EXIT_NEEDS_REVIEW)
        self.assertIn(
            "needs review: could not classify", printed.call_args_list[-1].args[0]
        )

    def test_parser_reads_quoted_paths_and_marker_triples(self) -> None:
        text = (
            'diff --git "a/tools/\\303\\244.py" "b/tools/\\303\\244.py"\n'
            "remerge CONFLICT (modify/delete): gone\n"
        )
        [diff] = base_update.parse(text)
        self.assertEqual(diff.paths, ["tools/\u00e4.py"])
        self.assertIsNotNone(base_update.reason(diff))
        lone = base_update.FileDiff(
            ["x"], ["CONFLICT (content): x"], ["======="], [], 1
        )
        self.assertFalse(base_update.markers_only(lone))


GENERATOR = """import pathlib, sys
root = pathlib.Path(__file__).resolve().parents[1]
text = (root / "rust/openttd-kernels/src/record.rs").read_text()
if "broken" in text:
    sys.exit(1)
out = pathlib.Path(sys.argv[1])
out.mkdir(parents=True, exist_ok=True)
(out / "abi-record-x.h").write_text(text)
"""


class RebuildReachTests(unittest.TestCase):
    """A fixture build: a.o includes wide.h and the generated record; b.o
    includes wide.h; c.o includes neither."""

    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.repo = Path(self.temp.name) / "repo"
        self.build = self.repo / "build-rust"
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "user.name", "Fixture")
        self.write(
            {
                "src/a.cpp": "a\n",
                "src/b.cpp": "b\n",
                "src/c.cpp": "c\n",
                "src/wide.h": "1\n",
                "docs/x.md": "1\n",
                "rust/openttd-kernels/src/record.rs": "1\n",
                "tools/generate-abi.py": GENERATOR,
                ".gitignore": "build-rust/\n",
            }
        )
        self.commit("init")
        self.build.mkdir()
        (self.build / "CMakeCache.txt").write_text(
            f"CMAKE_HOME_DIRECTORY:INTERNAL={self.repo}\n"
        )
        self.deps = Path(self.temp.name) / "deps.txt"
        self.deps.write_text(
            "CMakeFiles/lib.dir/src/a.cpp.o: #deps 4, deps mtime 1 (VALID)\n"
            "    ../src/a.cpp\n    /usr/include/stdio.h\n    ../src/wide.h\n"
            f"    {self.build}/generated-abi/abi-record-x.h\n\n"
            "CMakeFiles/lib.dir/src/b.cpp.o: #deps 2, deps mtime 1 (VALID)\n"
            f"    {self.repo}/src/b.cpp\n    ../src/wide.h\n\n"
            "CMakeFiles/lib.dir/src/c.cpp.o: #deps 1, deps mtime 1 (VALID)\n"
            "    ../src/c.cpp\n\n"
            "generated/strings.h: #deps 1, deps mtime 1 (VALID)\n    ../src/wide.h\n"
        )

    def tearDown(self) -> None:
        self.temp.cleanup()

    def git(self, *arguments: str) -> str:
        if not self.repo.exists():
            self.repo.mkdir(parents=True)
        return subprocess.run(
            ["git", *arguments],
            cwd=self.repo,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()

    def write(self, files: dict[str, str]) -> None:
        for name, text in files.items():
            (self.repo / name).parent.mkdir(parents=True, exist_ok=True)
            (self.repo / name).write_text(text)

    def commit(self, title: str) -> str:
        self.git("add", "-A")
        self.git("commit", "-q", "-m", title)
        return self.git("rev-parse", "HEAD")

    def cli(self, *arguments: str) -> tuple[int, str]:
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "rebuilds", "--build", str(self.build),
             "--deps", str(self.deps), *arguments],
            cwd=self.repo, capture_output=True, text=True,
            env={**os.environ, "GIT_CONFIG_GLOBAL": os.devnull},
        )  # fmt: skip
        return result.returncode, result.stdout + result.stderr

    def counts(self, *shas: str) -> list[int]:
        code, out = self.cli("--json", *shas)
        self.assertEqual(code, 0, out)
        return [c["objects"] for c in json.loads(out)["commits"]]

    def test_deps_keep_objects_source_files_and_generated_headers(self) -> None:
        objects = rebuild_reach.parse_deps(self.deps.read_text(), self.build, self.repo)
        self.assertEqual(
            objects["CMakeFiles/lib.dir/src/a.cpp.o"],
            {"src/a.cpp", "src/wide.h", "generated-abi/abi-record-x.h"},
        )
        self.assertEqual(
            objects["CMakeFiles/lib.dir/src/b.cpp.o"], {"src/b.cpp", "src/wide.h"}
        )
        self.assertEqual(len(objects), 3)

    def test_a_change_recompiles_the_objects_that_include_it(self) -> None:
        self.write({"src/wide.h": "2\n"})
        wide = self.commit("wide")
        self.write({"docs/x.md": "2\n"})
        docs = self.commit("docs")
        self.write({"src/c.cpp": "c2\n"})
        source = self.commit("source")
        self.assertEqual(self.counts(wide, docs, source), [2, 0, 1])
        code, out = self.cli(wide)
        self.assertIn("2 of 3 objects; src/wide.h 2", out)
        self.assertIn("Median 2 objects over the 1 of 1 commits that change any", out)

    def test_generated_headers_follow_their_rust_inputs(self) -> None:
        self.write({"rust/openttd-kernels/src/record.rs": "2\n"})
        record = self.commit("record")
        self.write({"rust/openttd-kernels/src/record.rs": "broken\n"})
        broken = self.commit("broken")
        self.assertEqual(self.counts(record, broken), [1, 1])
        code, out = self.cli(broken)
        self.assertIn("ABI headers not generated; all counted", out)

    def test_merges_since_come_from_the_base_first_parent_line(self) -> None:
        self.write({"src/c.cpp": "c3\n"})
        self.commit("Mention Merge pull request #99 in a regular commit")
        self.git("checkout", "-q", "-b", "topic")
        self.write({"src/wide.h": "3\n"})
        self.commit("topic")
        self.git("checkout", "-q", "main")
        self.git(
            "merge",
            "-q",
            "--no-ff",
            "-m",
            "Merge pull request #7 from x/topic",
            "topic",
        )
        self.git("update-ref", "refs/remotes/origin/rust-migration", "main")
        code, out = self.cli("--since", "1 day ago")
        self.assertEqual(code, 0, out)
        self.assertIn("#7: 2 of 3 objects", out)
        self.assertNotIn("#99", out)
        self.assertIn("over the 1 of 1 commits", out)

    def test_a_bare_date_starts_at_midnight_utc(self) -> None:
        self.assertEqual(rebuild_reach.since("2026-11-23"), "2026-11-23 00:00:00 +0000")
        self.assertEqual(rebuild_reach.since("7 days ago"), "7 days ago")

    def test_each_end_uses_its_own_generator(self) -> None:
        generator = GENERATOR.replace("write_text(text)", "write_text(text + 'v2')")
        self.write({"tools/generate-abi.py": generator})
        self.assertEqual(self.counts(self.commit("generator")), [1])

    def test_a_saved_graph_needs_the_build_that_wrote_it(self) -> None:
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "rebuilds", "--deps", str(self.deps), "--reach", "src/wide.h"],
            cwd=self.repo, capture_output=True, text=True,
        )  # fmt: skip
        self.assertEqual(result.returncode, 2)
        self.assertIn("--deps FILE needs --build DIR", result.stdout)

    def test_a_build_without_its_cmake_cache_is_refused(self) -> None:
        (self.build / "CMakeCache.txt").unlink()
        code, out = self.cli("--reach", "src/wide.h")
        self.assertEqual(code, 2, out)
        self.assertIn("names no CMAKE_HOME_DIRECTORY", out)

    def test_reach_json_is_the_whole_output(self) -> None:
        code, out = self.cli("--reach", "src/wide.h", "--json")
        self.assertEqual(code, 0, out)
        self.assertEqual(json.loads(out), {"objects": 3, "reach": {"src/wide.h": 2}})

    def test_reach_counts_the_objects_including_a_header(self) -> None:
        code, out = self.cli("--reach", "src/wide.h", "generated-abi/abi-record-x.h")
        self.assertEqual(code, 0, out)
        self.assertIn("src/wide.h: 2 of 3 objects", out)
        self.assertIn("generated-abi/abi-record-x.h: 1 of 3 objects", out)

    def test_a_missing_build_says_how_to_make_one(self) -> None:
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "rebuilds", "--deps", str(self.repo / "none")],
            cwd=self.repo, capture_output=True, text=True,
        )  # fmt: skip
        self.assertEqual(result.returncode, 2)
        self.assertIn("python3 tools/migration.py build", result.stdout)


if __name__ == "__main__":
    unittest.main()
