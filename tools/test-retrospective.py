#!/usr/bin/env python3
"""Exercise read-only evidence binding and overlapping milestone arithmetic."""

import json
import unittest
from dataclasses import replace
from typing import Any
from unittest import mock

import retrospective as facts


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


if __name__ == "__main__":
    unittest.main()
