#!/usr/bin/env python3
"""Exercise request identity and the actual privileged full-result publisher."""

import argparse
import json
import re
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import ci

ROOT = Path(__file__).resolve().parents[1]
HEAD = "a" * 40
BASE = "b" * 40
MERGE = "d" * 40
TREE = "f" * 40
RUN = "https://github.com/example/fork/actions/runs/123"
PULL = {
    "state": "open",
    "head": {"sha": HEAD},
    "base": {"sha": BASE, "ref": "rust-migration"},
    "labels": [],
    "mergeable": True,
    "merge_commit_sha": MERGE,
}


def script_for(filename, job):
    text = (ROOT / ".github/workflows" / filename).read_text()
    section = text.split(f"  {job}:\n", 1)[1]
    section = re.split(r"\n  [a-z_]+:\n", section, maxsplit=1)[0]
    match = re.search(r"        script: \|\n((?:          .*\n|\n)+)", section)
    if match is None:
        raise AssertionError(f"No publisher script in {filename}:{job}")
    return "\n".join(line[10:] for line in match[1].splitlines())


def node(harness):
    return json.loads(subprocess.check_output(["node", "-e", harness], text=True))


def evaluate(expression, **variables):
    declarations = "\n".join(
        f"const {key} = {json.dumps(value)};" for key, value in variables.items()
    )
    return node(declarations + f"console.log(JSON.stringify({expression}));")


def good_needs():
    needs = {
        name: {"result": "success", "outputs": {}}
        for name in ("plan", "quick", "native", "platform")
    }
    needs["plan"]["outputs"] = {
        "head": HEAD,
        "base": BASE,
        "merge": MERGE,
        "pr": "170",
        "tree": TREE,
    }
    for name in ("native", "platform"):
        needs[name]["outputs"]["validated"] = "true"
    return needs


def execute_script(filename, job, pull, *, env=None, event=None, latest=None):
    script = script_for(filename, job)
    return node(f"""
const statuses = [], failures = [], outputs = {{}};
const context = {{repo: {{owner: 'example', repo: 'fork'}}, actor: 'root',
  serverUrl: 'https://github.com', runId: 123, payload: {{pull_request: {json.dumps(event)}}}}};
const core = {{setFailed: value => failures.push(value), setOutput: (k,v) => outputs[k] = v}};
const github = {{rest: {{pulls: {{get: async () => ({{data: {json.dumps(pull)}}})}}, repos: {{
  getCommit: async () => ({{data: {{commit: {{tree: {{sha: {json.dumps(TREE)}}}}}}}}}),
  getCollaboratorPermissionLevel: async () => ({{data: {{permission: 'write'}}}}),
  getCombinedStatusForRef: async () => ({{data: {{statuses: {json.dumps([latest] if latest else [])}}}}}),
  createCommitStatus: async value => statuses.push(value)
}}}}}};
Object.assign(process.env, {json.dumps(env or {})});
(async () => {{{script}}})().then(() => console.log(JSON.stringify({{statuses, failures, outputs}})))
  .catch(error => console.log(JSON.stringify({{error: error.message, statuses}})));
""")


def execute_publisher(needs, pull, superseded=False):
    latest = {
        "context": ci.CONTEXT,
        "target_url": "https://github.com/example/fork/actions/runs/"
        + ("999" if superseded else "123"),
    }
    return execute_script(
        "ci-request.yml",
        "publish",
        pull,
        env={"RESULTS": json.dumps(needs)},
        latest=latest,
    )


def execute_invalidator(event, pull, latest=None):
    return execute_script(
        "ci-invalidate.yml", "invalidate", pull, event=event, latest=latest
    )["statuses"]


class GateTests(unittest.TestCase):
    def test_plan_checks_mergeability_and_checks_out_merge_while_status_targets_head(
        self,
    ):
        for mergeable in (True, False, None):
            pull = dict(PULL, mergeable=mergeable)
            result = execute_script(
                "ci-request.yml",
                "plan",
                pull,
                env={"PR": "170", "HEAD": HEAD, "PROFILE": "full"},
            )
            if mergeable is True:
                self.assertEqual(result["outputs"]["merge"], MERGE)
                self.assertEqual(result["statuses"][0]["sha"], HEAD)
            else:
                self.assertIn("unmergeable", result["error"])
                self.assertEqual(result["statuses"], [])
        workflow = (ROOT / ".github/workflows/ci-request.yml").read_text()
        self.assertEqual(workflow.count("ref: ${{ needs.plan.outputs.merge }}"), 4)
        self.assertNotIn("ref: ${{ needs.plan.outputs.head }}", workflow)
        quick = (ROOT / ".github/workflows/ci-quick.yml").read_text()
        self.assertIn("HEAD: ${{ inputs.checkout-ref }}^2", quick)

    def test_full_success(self):
        result = execute_publisher(good_needs(), PULL)
        self.assertEqual(result["statuses"][0]["state"], "success")
        self.assertEqual(result["statuses"][0]["sha"], HEAD)
        self.assertEqual(result["statuses"][0]["context"], ci.CONTEXT)
        self.assertFalse(result["failures"])

    def test_superseded_request_cannot_overwrite_newer_status(self):
        result = execute_publisher(good_needs(), PULL, superseded=True)
        self.assertEqual(result["statuses"], [])
        self.assertTrue(result["failures"])

    def test_status_writers_share_a_non_cancelling_per_pr_concurrency_group(self):
        groups = []
        for filename, job in (
            ("ci-request.yml", "plan"),
            ("ci-request.yml", "publish"),
            ("ci-invalidate.yml", "invalidate"),
        ):
            workflow = (ROOT / ".github/workflows" / filename).read_text()
            section = workflow.split(f"  {job}:\n", 1)[1]
            section = re.split(r"\n  [a-z_]+:\n", section, maxsplit=1)[0]
            concurrency = re.search(
                r"    concurrency:\n      group: (.+)\n      cancel-in-progress: (.+)",
                section,
            )
            self.assertIsNotNone(concurrency)
            self.assertEqual(concurrency[2], "false")
            expression = re.fullmatch(r"validation-status-\${{ (.+) }}", concurrency[1])
            self.assertIsNotNone(expression)
            # Dispatch inputs are strings; pull_request_target provides a number.
            for inputs in ({"pr": "170"}, {}):
                groups.append(
                    evaluate(
                        "'validation-status-' + (" + expression[1] + ")",
                        inputs=inputs,
                        github={"event": {"pull_request": {"number": 170}}},
                    )
                )
        self.assertEqual(groups, ["validation-status-170"] * 6)

    def test_delayed_invalidation_preserves_current_request(self):
        event = dict(PULL, number=170, updated_at="2026-10-07T12:00:00Z")
        for timestamp in (event["updated_at"], "2026-10-07T12:00:01Z"):
            with self.subTest(timestamp=timestamp):
                latest = {
                    "context": ci.CONTEXT,
                    "updated_at": timestamp,
                    "target_url": "https://github.com/example/fork/actions/runs/123",
                }
                self.assertEqual(execute_invalidator(event, PULL, latest), [])

    def test_invalidation_only_marks_current_unvalidated_head_pending(self):
        event = dict(PULL, number=170, updated_at="2026-10-07T12:00:00Z")
        older = {"context": ci.CONTEXT, "updated_at": "2026-10-07T11:59:59Z"}
        for latest in (None, older):
            with self.subTest(latest=latest):
                statuses = execute_invalidator(event, PULL, latest)
                self.assertEqual(len(statuses), 1)
                self.assertEqual(statuses[0]["state"], "pending")
                self.assertEqual(statuses[0]["sha"], HEAD)
                self.assertEqual(statuses[0]["context"], ci.CONTEXT)
        pull = dict(PULL, head={"sha": "c" * 40})
        self.assertEqual(execute_invalidator(event, pull), [])

    def test_each_non_success_result_is_rejected(self):
        for job in good_needs():
            for outcome in ("failure", "skipped", "cancelled", "timed_out", ""):
                with self.subTest(job=job, outcome=outcome):
                    needs = good_needs()
                    needs[job]["result"] = outcome
                    self.assertEqual(
                        execute_publisher(needs, PULL)["statuses"][0]["state"],
                        "failure",
                    )

    def test_missing_job_or_validation_proof_is_rejected(self):
        for job in ("quick", "native", "platform"):
            needs = good_needs()
            del needs[job]
            self.assertEqual(
                execute_publisher(needs, PULL)["statuses"][0]["state"], "failure"
            )
            if job in ("native", "platform"):
                needs = good_needs()
                needs[job]["outputs"] = {}
                self.assertEqual(
                    execute_publisher(needs, PULL)["statuses"][0]["state"], "failure"
                )

    def test_changed_pull_cannot_publish_success(self):
        for change in (
            {"head": {"sha": "c" * 40}},
            {"state": "closed"},
            {"base": {"ref": "master", "sha": BASE}},
        ):
            self.assertEqual(
                execute_publisher(good_needs(), dict(PULL, **change))["statuses"][0][
                    "state"
                ],
                "failure",
            )

    def test_base_advances_and_mergeability_pending_preserve_exact_head_validation(
        self,
    ):
        for change in (
            {"base": {"ref": "rust-migration", "sha": "e" * 40}},
            {"merge_commit_sha": "e" * 40},
            {"mergeable": None},
            {"mergeable": False},
        ):
            result = execute_publisher(good_needs(), dict(PULL, **change))
            self.assertEqual(result["statuses"][0]["state"], "success")
            marker = result["statuses"][1]
            self.assertEqual(marker["context"], "Validated tree")
            self.assertEqual(marker["description"], TREE)
            self.assertEqual(marker["target_url"], result["statuses"][0]["target_url"])

    def test_no_candidate_execution_in_privileged_jobs(self):
        workflow = (ROOT / ".github/workflows/ci-request.yml").read_text()
        for job in ("plan", "publish"):
            section = workflow.split(f"  {job}:\n", 1)[1]
            section = re.split(r"\n  [a-z_]+:\n", section, maxsplit=1)[0]
            self.assertIn("statuses: write", section)
            self.assertNotIn("actions/checkout", section)
            self.assertNotIn("run:", section)
        self.assertIn("github.ref == 'refs/heads/rust-migration'", workflow)
        self.assertIn("vars.CI_ON_DEMAND != 'true'", workflow)
        self.assertIn(
            "needs.plan.outputs.profile == 'full'", workflow.split("  publish:\n", 1)[1]
        )
        self.assertIn("if (profile === 'full')", script_for("ci-request.yml", "plan"))

    def test_push_invalidation_has_no_candidate_checkout(self):
        workflow = (ROOT / ".github/workflows/ci-invalidate.yml").read_text()
        self.assertIn("pull_request_target:", workflow)
        self.assertNotIn("actions/checkout", workflow)
        self.assertIn("state: 'pending'", workflow)
        self.assertNotIn("state: 'success'", workflow)


class CacheTests(unittest.TestCase):
    def prune(self, caches):
        script = script_for("ci-cache.yml", "prune")
        return node(f"""
const deleted = [], failures = [], context = {{repo: {{}}}};
const core = {{info: () => {{}}, setFailed: value => failures.push(value)}};
const github = {{paginate: async () => {json.dumps(caches)}, rest: {{actions: {{
  getActionsCacheList: () => {{}}, deleteActionsCacheById: async value => deleted.push(value.cache_id)
}}}}}};
(async () => {{{script}}})().then(() => console.log(JSON.stringify({{deleted, failures}})));
""")

    def test_actual_cache_controller_keeps_one_entry_per_prefix_and_eight_gib(self):
        gib = 1024**3
        caches = [
            {
                "id": i,
                "key": prefix + "-" + str(i) * 40,
                "created_at": f"2026-10-{i:02}",
                "size_in_bytes": gib,
            }
            for i, prefix in enumerate(
                (
                    "compiler-v1-linux",
                    "compiler-v1-linux",
                    "ccache-linux-v2-gcc",
                    "ccache-macos-v2-arm64",
                    "ccache-msvc-v1-x64",
                ),
                start=1,
            )
        ]
        result = self.prune(caches)
        self.assertEqual(result["deleted"], [1])
        self.assertEqual(result["failures"], [])
        caches[-1]["size_in_bytes"] = 7 * gib
        result = self.prune(caches)
        self.assertEqual(result["deleted"], [1, 2, 3])
        self.assertEqual(result["failures"], [])
        result = self.prune(
            [
                {
                    "id": 99,
                    "key": "unmanaged",
                    "created_at": "date",
                    "size_in_bytes": 9 * gib,
                }
            ]
        )
        self.assertEqual(result["deleted"], [])  # Unmanaged caches are never removed.
        self.assertTrue(result["failures"])
        workflow = (ROOT / ".github/workflows/ci-cache.yml").read_text()
        self.assertIn("actions: write", workflow)
        self.assertIn("github.event.workflow_run.event == 'push'", workflow)
        self.assertIn("head_branch == 'rust-migration'", workflow)
        self.assertNotIn("actions/checkout", workflow)
        self.assertNotIn("download-artifact", workflow)
        self.assertNotRegex(workflow, r"(?m)^\s+(?:- )?run:")


class PolicyTests(unittest.TestCase):
    def test_platform_cache_refresh_is_only_a_covered_protected_push(self):
        workflow = (ROOT / ".github/workflows/ci-build.yml").read_text()
        for job in ("linux", "windows"):
            section = workflow.split(f"  {job}:\n", 1)[1]
            section = re.split(r"\n  [a-z_]+:\n", section, maxsplit=1)[0]
            expression = re.search(r"      cache-only: \${{ (.+) }}", section)[1]
            expression = expression.replace(
                "inputs.checkout-ref", 'inputs["checkout-ref"]'
            )
            for checkout, event, ref, covered, expected in (
                ("", "push", "refs/heads/rust-migration", "true", True),
                ("merge", "push", "refs/heads/rust-migration", "true", False),
                ("", "pull_request", "refs/heads/rust-migration", "true", False),
                ("", "push", "refs/heads/master", "true", False),
                ("", "push", "refs/heads/rust-migration", "false", False),
            ):
                self.assertEqual(
                    evaluate(
                        expression,
                        inputs={"checkout-ref": checkout},
                        github={"event_name": event, "ref": ref},
                        needs={"policy": {"outputs": {"covered": covered}}},
                    ),
                    expected,
                )
        excluded = {
            "ci-linux.yml": ["Test"],
            "ci-windows.yml": [
                "Rust checks for actual selected target",
                "Test",
                "Fresh Rust native generators",
                "Validate refusals and actual archive freshness",
                "Record native CRT ABI linkage and fresh outputs",
                "Retain native Windows Rust and test evidence",
            ],
        }
        for filename, labels in excluded.items():
            child = (ROOT / ".github/workflows" / filename).read_text()
            for label in labels:
                step = child.split(f"    - name: {label}\n", 1)[1].split(
                    "    - name:", 1
                )[0]
                self.assertRegex(step, r"      if: .* !inputs.cache-only")
        self.assertIn(
            "!inputs.cache-only || inputs.compiler != 'clang'",
            (ROOT / ".github/workflows/ci-linux.yml").read_text(),
        )

    def test_heavy_predicates_bypass_skipped_policy_but_respect_cancellation(self):
        for filename, jobs in (
            ("ci-build.yml", ("emscripten", "linux", "macos", "windows")),
            ("rust-migration.yml", ("build", "comparisons", "simulation")),
        ):
            workflow = (ROOT / ".github/workflows" / filename).read_text()
            for job in jobs:
                section = workflow.split(f"  {job}:\n", 1)[1]
                predicate = re.search(r"^    if: (.+)$", section, re.MULTILINE)[1]
                self.assertNotIn("always()", predicate)
                self.assertIn("!cancelled()", predicate)
                expression = predicate.removeprefix("${{ ").removesuffix(" }}")
                cases = [
                    (False, "head", "", "full", "push", True),
                    (False, "", "true", "full", "push", True),
                    (False, "", "false", "full", "push", False),
                    (True, "head", "true", "full", "push", False),
                    (True, "", "true", "full", "push", False),
                ]
                if job == "macos":
                    # Full skips macOS; platform, heavy pushes and dispatch run it (#357).
                    dispatch = "workflow_dispatch"
                    cases = [
                        (False, "head", "", "full", dispatch, False),
                        (False, "head", "", "platform", dispatch, True),
                        (False, "", "true", "", "push", True),
                        (False, "", "false", "", "push", False),
                        (False, "", "", "", dispatch, True),
                        (True, "", "", "", dispatch, False),
                    ]
                for cancelled, checkout, heavy, profile, event, expected in cases:
                    inputs = {"checkout-ref": checkout, "profile": profile}
                    needs = {
                        "policy": {"outputs": {"heavy": heavy}},
                        "build": {"result": "success"},
                    }
                    with self.subTest(filename=filename, job=job, cancelled=cancelled):
                        result = evaluate(
                            expression.replace(
                                "inputs.checkout-ref", 'inputs["checkout-ref"]'
                            ).replace("cancelled()", str(cancelled).lower()),
                            inputs=inputs,
                            needs=needs,
                            github={"event_name": event},
                        )
                        self.assertEqual(result, expected)

    def test_protected_push_groups(self):
        def groups(filename):
            workflow = (ROOT / ".github/workflows" / filename).read_text()
            expression = re.search(r"^  group: (.+)$", workflow, re.MULTILINE)[1]
            expression = (
                expression.rsplit("${{ ", 1)[1]
                .removesuffix(" }}")
                .replace("inputs.checkout-ref", 'inputs["checkout-ref"]')
            )
            self.assertIn("github.ref != 'refs/heads/rust-migration'", workflow)
            return workflow, [
                evaluate(
                    expression,
                    inputs={},
                    github={
                        "ref": "refs/heads/rust-migration",
                        "sha": sha,
                        "run_id": 1,
                    },
                )
                for sha in ("code-merge", "later-merge")
            ]

        # Heavy post-merge validation coalesces queued pushes to the newest tip.
        # Markdown-only pushes never start, so they cannot displace a code push.
        for filename in ("ci-build.yml", "rust-migration.yml"):
            workflow, (first, second) = groups(filename)
            self.assertEqual(first, second)
            push = re.search(r"^  push:\n((?:    .*\n)+)", workflow, re.MULTILINE)[1]
            self.assertIn("paths-ignore: ['**.md']", push)
        _, (first, second) = groups("ci-quick.yml")
        self.assertNotEqual(first, second)
        # A dispatched macOS run never displaces a queued push validation (#357).
        workflow = (ROOT / ".github/workflows/ci-build.yml").read_text()
        self.assertIn("-${{ github.event_name }}-build-", workflow)

    def test_bootstrap_on_demand_and_docs_classification(self):
        script = script_for("ci-policy.yml", "policy")
        cases = [
            ("", "pull_request", [], "true"),
            ("false", "pull_request", [], "true"),
            ("true", "pull_request", [], "false"),
            ("true", "push", [{"filename": "docs/roadmap.md"}], "false"),
            ("true", "push", [{"filename": "tools/ci.py"}], "true"),
            (
                "true",
                "push",
                [{"filename": "README.md", "previous_filename": "tools/tool.py"}],
                "true",
            ),
            ("true", "push", [{"filename": "docs/new.md"}] * 300, "true"),
            ("true", "push", [], "true"),
            ("true", "push", None, "true"),
        ]
        for variable, event, files, expected in cases:
            with self.subTest(
                variable=variable,
                event=event,
                files=len(files) if files is not None else None,
            ):
                harness = f"""
                process.env.CI_ON_DEMAND = {json.dumps(variable)};
                const outputs = {{}};
                const context = {{eventName: {json.dumps(event)}, repo: {{}}, sha: 'head',
                  payload: {{before: 'base', pull_request: {{base: {{ref: 'rust-migration'}}}}}}}};
                const core = {{setOutput: (key, value) => outputs[key] = value, info: () => {{}}}};
                const github = {{paginate: async () => [], rest: {{
                  git: {{getTree: async args => ({{data: {{sha: args.tree_sha, truncated: false,
                    tree: [{{path: 'README.md', type: 'blob', mode: '100644', sha: '{HEAD}'}}]}}}})}},
                  repos: {{
                  getCommit: async () => ({{data: {{commit: {{tree: {{sha: "tree"}}}}}}}}),
                  compareCommitsWithBasehead: async () => ({{data: {{files: {json.dumps(files)}}}}})
                }}}}}};
                (async () => {{{script}}})().then(() => console.log(JSON.stringify(outputs)));
                """
                result = node(harness)
                self.assertEqual(result["heavy"], expected)

    def test_docs_fast_path_checks_tree_types_and_complete_inventories(self):
        script = script_for("ci-policy.yml", "policy")
        code = {"path": "src/game.cpp", "type": "blob", "mode": "100644", "sha": HEAD}
        link = {"path": "module.md", "type": "commit", "mode": "160000", "sha": HEAD}
        doc = dict(link, type="blob", mode="100644")
        before = {"truncated": False, "tree": [code, link]}
        after = {"truncated": False, "tree": [code, dict(link, sha=MERGE)]}
        files = [{"filename": "module.md"}]
        cases = [
            (before, after, files, "true"),
            ({"truncated": False, "tree": [code]}, after, files, "true"),
            (before, {"truncated": False, "tree": [code]}, files, "true"),
            (
                before,
                {"truncated": False, "tree": [code, dict(link, path="new.md")]},
                [{"filename": "new.md", "previous_filename": "module.md"}],
                "true",
            ),
        ]
        docs_before = {"truncated": False, "tree": [code, doc]}
        docs_after = {"truncated": False, "tree": [code, dict(doc, sha=MERGE)]}
        cases.append((docs_before, docs_after, files, "false"))
        for bad in (dict(docs_before, truncated=True), dict(docs_before, tree=[{}])):
            cases.extend(
                [(bad, docs_after, files, "true"), (docs_before, bad, files, "true")]
            )
        for left, right, changed, expected in cases:
            harness = f"""
            process.env.CI_ON_DEMAND = 'true';
            const outputs = {{}};
            const context = {{eventName: 'push', repo: {{owner: 'example', repo: 'fork'}},
              sha: 'head', payload: {{before: 'base'}}}};
            const core = {{setOutput: (k,v) => outputs[k] = v, info: () => {{}}}};
            const github = {{paginate: async () => [], rest: {{
              git: {{getTree: async args => ({{data: {{sha: args.tree_sha,
                ...(args.tree_sha === '{BASE}' ? {json.dumps(left)} : {json.dumps(right)})}}}})}},
              repos: {{
                getCommit: async args => ({{data: {{commit: {{tree: {{sha: args.ref === 'base' ? '{BASE}' : '{TREE}'}}}}}}}}),
                compareCommitsWithBasehead: async () => ({{data: {{files: {json.dumps(changed)}}}}})
              }}
            }}}};
            (async () => {{{script}}})().then(() => console.log(JSON.stringify(outputs)));
            """
            with self.subTest(left=left, right=right, files=changed):
                self.assertEqual(node(harness)["heavy"], expected)

    def test_only_matching_successful_merged_tree_skips_validation(self):
        script = script_for("ci-policy.yml", "policy")
        bad_runs = (None, "", "run", RUN + "/extra", RUN.replace("fork", "other"))
        for full, marker, same_run, tree, merged, run, expected in (
            ("success", "success", True, TREE, True, RUN, "false"),
            ("success", "success", True, "other", True, RUN, "true"),
            ("pending", "success", True, TREE, True, RUN, "true"),
            ("failure", "success", True, TREE, True, RUN, "true"),
            ("success", "failure", True, TREE, True, RUN, "true"),
            ("success", "success", False, TREE, True, RUN, "true"),
            ("success", "success", True, TREE, False, RUN, "true"),
            *(
                ("success", "success", True, TREE, True, run, "true")
                for run in bad_runs
            ),
        ):
            statuses = [
                {"context": "Full validation", "state": full, "target_url": run},
                {
                    "context": "Validated tree",
                    "state": marker,
                    "target_url": run if same_run else "old",
                    "description": tree,
                },
            ]
            pull = dict(PULL, merged_at="date" if merged else None)
            harness = f"""
            process.env.CI_ON_DEMAND = 'true';
            const outputs = {{}};
            const context = {{eventName: 'push', repo: {{owner: 'example', repo: 'fork'}}, serverUrl: 'https://github.com', sha: 'head', payload: {{before: 'base'}}}};
            const core = {{setOutput: (k, v) => outputs[k] = v, info: () => {{}}}};
            const github = {{paginate: async () => [{json.dumps(pull)}], rest: {{repos: {{
              getCommit: async () => ({{data: {{commit: {{tree: {{sha: {json.dumps(TREE)}}}}}}}}}),
              getCombinedStatusForRef: async () => ({{data: {{statuses: {json.dumps(statuses)}}}}}),
              compareCommitsWithBasehead: async () => ({{data: {{files: [{{filename: 'src/code.cpp'}}]}}}})
            }}}}}};
            (async () => {{{script}}})().then(() => console.log(JSON.stringify(outputs)));
            """
            with self.subTest(
                full=full, marker=marker, same_run=same_run, tree=tree, merged=merged
            ):
                self.assertEqual(node(harness)["heavy"], expected)

    def test_markdown_tree_differences_require_complete_inventories(self):
        script = script_for("ci-policy.yml", "policy")
        code = {"path": "src/game.cpp", "type": "blob", "mode": "100644", "sha": HEAD}
        doc = {"path": "README.md", "type": "blob", "mode": "100644", "sha": BASE}
        before = {"truncated": False, "tree": [code, doc]}
        changed_doc = dict(doc, sha=MERGE, path="docs/new.md")
        gitlink = dict(doc, type="commit", mode="160000")
        cases = [
            ({"truncated": False, "tree": [changed_doc, code]}, "false"),
            ({"truncated": False, "tree": [code]}, "false"),
            ({"truncated": False, "tree": [dict(code, sha=MERGE)]}, "true"),
            ({"truncated": False, "tree": [dict(code, mode="100755")]}, "true"),
            ({"truncated": False, "tree": [dict(code, path="src/other.cpp")]}, "true"),
            (
                {
                    "truncated": False,
                    "tree": [dict(code, type="commit", mode="160000")],
                },
                "true",
            ),
            ({"truncated": False, "tree": [changed_doc]}, "true"),
            ({"truncated": True, "tree": [code]}, "true"),
            ({"tree": [code]}, "true"),
            ({"truncated": False}, "true"),
            ({"truncated": False, "tree": []}, "true"),
            ({"truncated": False, "tree": [{}]}, "true"),
            ({"truncated": False, "tree": [code, dict(doc, sha="bad")]}, "true"),
            ({"truncated": False, "tree": [code, dict(doc, mode="bad")]}, "true"),
            ({"truncated": False, "tree": [code, doc, doc]}, "true"),
            ({"truncated": False, "tree": [code, gitlink]}, "true"),
            ({"truncated": False, "tree": [code, dict(doc, path="../a.md")]}, "true"),
            ({"truncated": False, "tree": [code], "sha": None}, "true"),
        ]
        for after, expected in cases:
            for incomplete_side in (False, True):
                left, right = (after, before) if incomplete_side else (before, after)
                harness = f"""
                process.env.CI_ON_DEMAND = 'true';
                const outputs = {{}};
                const context = {{eventName: 'push', repo: {{owner: 'example', repo: 'fork'}}, serverUrl: 'https://github.com', sha: 'head', payload: {{before: 'base'}}}};
                const core = {{setOutput: (k,v) => outputs[k] = v, info: () => {{}}}};
                const statuses = [
                  {{context: 'Full validation', state: 'success', target_url: '{RUN}'}},
                  {{context: 'Validated tree', state: 'success', target_url: '{RUN}', description: '{BASE}'}}
                ];
                const github = {{paginate: async () => [{{...{json.dumps(PULL)}, merged_at: 'date'}}], rest: {{
                  git: {{getTree: async args => ({{data: {{sha: args.tree_sha, ...(args.tree_sha === '{BASE}' ? {json.dumps(left)} : {json.dumps(right)})}}}})}},
                  repos: {{
                    getCommit: async () => ({{data: {{commit: {{tree: {{sha: '{TREE}'}}}}}}}}),
                    getCombinedStatusForRef: async () => ({{data: {{statuses}}}}),
                    compareCommitsWithBasehead: async () => ({{data: {{files: [{{filename: 'src/game.cpp'}}]}}}})
                  }}
                }}}};
                (async () => {{{script}}})().then(() => console.log(JSON.stringify(outputs)));
                """
                with self.subTest(after=after, incomplete_side=incomplete_side):
                    outputs = node(harness)
                    self.assertEqual(outputs["heavy"], expected)
                    self.assertEqual(
                        outputs.get("covered"), "true" if expected == "false" else None
                    )


class RequestTests(unittest.TestCase):
    def test_each_profile_dispatches_captured_head_on_protected_ref(self):
        for profile in ci.PROFILES:
            with tempfile.TemporaryDirectory() as directory:
                args = argparse.Namespace(
                    repo="example/fork",
                    pr=170,
                    profile=profile,
                    bootstrap=False,
                    request_file=Path(directory) / "request.json",
                )
                with (
                    patch.object(ci, "api", return_value=PULL),
                    patch.object(ci, "gh") as gh,
                ):
                    request_path = ci.request(args)
                invocation = gh.call_args.args
                self.assertIn(f"head={HEAD}", invocation)
                self.assertIn(f"profile={profile}", invocation)
                self.assertEqual(
                    invocation[invocation.index("--ref") + 1], "rust-migration"
                )
                record = json.loads(request_path.read_text())
                self.assertEqual(record["head"], HEAD)
                self.assertEqual(record["base"], BASE)
                self.assertEqual(len(record["request"]), 32)

    def test_run_discovery_uses_unique_request_identity(self):
        record = {
            "profile": "rust",
            "pr": 170,
            "request": "unique",
            "bootstrap": False,
        }
        run = {
            "id": 1,
            "display_title": "CI rust PR 170 unique",
            "event": "workflow_dispatch",
            "head_branch": "rust-migration",
        }
        other = dict(run, id=2, display_title="CI rust PR 170 other")
        self.assertEqual(ci.find_run(record, [other, run]), run)
        self.assertIsNone(ci.find_run(record, [dict(run, head_branch="untrusted")]))
        with self.assertRaises(ValueError):
            ci.find_run(record, [run, dict(run, id=3)])

    def test_push_invalidates_record_head(self):
        with self.assertRaisesRegex(ValueError, "head changed"):
            ci.validate_pull(PULL, "c" * 40)

    def test_base_advance_during_run_keeps_waiting_on_unchanged_head(self):
        run = {
            "id": 123,
            "status": "completed",
            "conclusion": "success",
            "html_url": "https://github.com/example/fork/actions/runs/123",
        }
        statuses = [
            {"context": ci.CONTEXT, "target_url": run["html_url"], "state": "success"}
        ]
        for mergeable in (None, False):
            pull = dict(
                PULL,
                mergeable=mergeable,
                base={"sha": "e" * 40, "ref": "rust-migration"},
            )
            with self.assertRaisesRegex(ValueError, "mergeability"):
                ci.validate_pull(pull)
            with tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / "request.json"
                record = {
                    "schema_version": 1,
                    "repository": "example/fork",
                    "pr": 170,
                    "head": HEAD,
                    "profile": "full",
                    "run_id": 123,
                }
                ci.write_request(path, record)
                with patch.object(ci, "api", side_effect=[pull, run]):
                    self.assertEqual(ci.status(path), (record, run))
                with patch.object(
                    ci, "api", side_effect=[pull, {"statuses": statuses}]
                ):
                    ci.verify_completion(record, run)
                moved = dict(pull, head={"sha": "c" * 40})
                with patch.object(ci, "api", side_effect=[moved, run]):
                    with self.assertRaisesRegex(ValueError, "head changed"):
                        ci.status(path)

    def test_successful_run_without_full_status_is_rejected(self):
        record = {
            "repository": "example/fork",
            "pr": 170,
            "head": HEAD,
            "profile": "full",
        }
        run = {
            "conclusion": "success",
            "html_url": "https://github.com/example/fork/actions/runs/123",
        }
        with patch.object(ci, "api", side_effect=[PULL, {"statuses": []}]):
            with self.assertRaisesRegex(ValueError, "did not publish"):
                ci.verify_completion(record, run)
        for state in ("pending", "failure"):
            statuses = [
                {"context": ci.CONTEXT, "target_url": run["html_url"], "state": state}
            ]
            with patch.object(ci, "api", side_effect=[PULL, {"statuses": statuses}]):
                with self.assertRaises(ValueError):
                    ci.verify_completion(record, run)


if __name__ == "__main__":
    unittest.main()
