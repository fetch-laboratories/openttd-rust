#!/usr/bin/env python3
"""Exercise request identity and the actual privileged full-result publisher."""

import argparse
import itertools
import json
import os
import re
import subprocess
import tempfile
import unittest
from collections.abc import Callable, Iterable, Mapping
from pathlib import Path
from typing import Any, cast
from unittest.mock import patch

import ci
import yaml

ROOT = Path(__file__).resolve().parents[1]
WORKFLOWS = ROOT / ".github/workflows"
HEAD = "a" * 40
BASE = "b" * 40
MERGE = "d" * 40
TREE = "f" * 40
PREVIOUS = "9" * 40
PARENT = "8" * 40  # The captured merge's first parent, deliberately not PULL's base.
RUN = "https://github.com/example/fork/actions/runs/123"
ACTIONS = {"login": "github-actions[bot]", "type": "Bot"}
PULL = {
    "state": "open",
    "head": {"sha": HEAD},
    "base": {"sha": BASE, "ref": "rust-migration"},
    "labels": [],
    "mergeable": True,
    "merge_commit_sha": MERGE,
}


def workflow(filename: str) -> dict[str, Any]:
    return yaml.safe_load((WORKFLOWS / filename).read_text())


def job(filename: str, name: str) -> dict[str, Any]:
    return workflow(filename)["jobs"][name]


def script_for(filename: str, name: str) -> str:
    (step,) = [s for s in job(filename, name)["steps"] if "script" in s.get("with", {})]
    return step["with"]["script"]


def node(harness: str) -> object:
    return json.loads(subprocess.check_output(["node", "-e", harness], text=True))


def javascript(expression: str | bool) -> str:
    """Translate one GitHub expression; hyphenated properties become subscripts."""
    if isinstance(expression, bool):
        return json.dumps(expression)
    expression = expression.strip().removeprefix("${{").removesuffix("}}")
    expression = expression.replace("fromJSON(", "JSON.parse(")
    parts = re.split(r"('[^']*')", expression)
    parts[::2] = [
        re.sub(r"\.([\w-]+)", lambda match: f"[{json.dumps(match[1])}]", part)
        for part in parts[::2]
    ]
    return "".join(parts)


def evaluate(
    expression: str | bool,
    *,
    cancelled: bool = False,
    interpolate: bool = False,
    **contexts: dict[str, Any],
) -> object:
    """Evaluate an `if:` expression, or interpolate `${{ }}` into a string."""
    if interpolate:
        parts = re.split(r"\$\{\{(.*?)\}\}", cast(str, expression))
        parts[::2] = map(json.dumps, parts[::2])
        parts[1::2] = [f"String({javascript(part)})" for part in parts[1::2]]
        expression = " + ".join(parts)
    else:
        expression = javascript(expression)
    declarations = "".join(
        f"const {key} = {json.dumps(value)};\n" for key, value in contexts.items()
    )
    results = [
        dependency.get("result", "success")
        for dependency in contexts.get("needs", {}).values()
    ]
    succeeded = not cancelled and all(result == "success" for result in results)
    failed = "failure" in results
    return node(
        f"const cancelled = () => {json.dumps(cancelled)}, always = () => true;\n"
        "const format = (text, ...args) => text.replace(/\\{(\\d+)\\}/g, (_, i) => String(args[i]));\n"
        f"const success = () => {json.dumps(succeeded)}, failure = () => {json.dumps(failed)};\n"
        + declarations
        + f"console.log(JSON.stringify({expression}));"
    )


def good_needs() -> dict[str, dict[str, Any]]:
    needs: dict[str, dict[str, Any]] = {
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


def execute_script(
    filename: str,
    name: str,
    pull: object,
    *,
    env: Mapping[str, str] | None = None,
    event: object = None,
    latest: object = None,
    payload: Mapping[str, object] | None = None,
    previous: Iterable[object] = (),
    trees: object = None,
    compare: str | None = None,
    event_name: str = "workflow_dispatch",
    commits: Mapping[str, object] | None = None,
    runs: str = "async () => ({data: {workflow_runs: []}})",
    progress: Mapping[int, list[object]] | None = None,
    statuses: Mapping[str, Iterable[object]] | None = None,
    files: object = None,
    tick: int = 0,
) -> dict[str, Any]:
    """Run a status writer; `previous` holds the statuses of the PREVIOUS head.

    `commits` answers getCommit by ref, `runs` is the workflow run listing,
    `progress` the successive getWorkflowRun answers by run ID, `statuses` the
    listed statuses of other refs and `files` the PR file list (None fails the
    lookup). Each Date.now() call advances the clock by `tick` milliseconds.
    """
    script = script_for(filename, name)
    payload = {"pull_request": event, **(payload or {})}
    compare = compare or "async () => ({data: {behind_by: 0}})"
    listed = {PREVIOUS: list(previous)}
    listed |= {ref: list(rows) for ref, rows in (statuses or {}).items()}
    tested = {"parents": [{"sha": PARENT}], "commit": {"tree": {"sha": TREE}}}
    return cast(
        dict[str, Any],
        node(f"""
const statuses = [], failures = [], outputs = {{}}, delays = [], trees = {json.dumps(trees or {})};
const commits = {json.dumps(commits or {})}, progress = {json.dumps(progress or {})};
let now = 0;
Date.now = () => (now += {tick});
const setTimeout = (resolve, delay) => {{ delays.push(delay); resolve(); }};
const context = {{repo: {{owner: 'example', repo: 'fork'}}, actor: 'root', eventName: {json.dumps(event_name)},
  serverUrl: 'https://github.com', runId: 123, payload: {json.dumps(payload)}}};
const core = {{setFailed: value => failures.push(value), setOutput: (k,v) => outputs[k] = v,
  info: () => {{}}}};
const github = {{paginate: async (method, args) => {{
  if (method === github.rest.pulls.listFiles) {{
    const files = {json.dumps(files)};
    if (files === null) throw new Error('Not Found');
    return files;
  }}
  const listed = {json.dumps(listed)};
  if (!(args.ref in listed)) throw new Error('Unexpected status ref');
  return listed[args.ref];
}}, rest: {{pulls: {{get: async () => ({{data: {json.dumps(pull)}}}), listFiles: () => {{}}}},
  git: {{getTree: async args => ({{data: {{sha: args.tree_sha, ...trees[args.tree_sha]}}}})}},
  actions: {{listWorkflowRuns: {runs},
    getWorkflowRun: async args => ({{data: progress[args.run_id].shift()}})}},
  repos: {{
  getCommit: async args => ({{data: commits[args.ref] ?? {json.dumps(tested)}}}),
  getCollaboratorPermissionLevel: async () => ({{data: {{permission: 'write'}}}}),
  getCombinedStatusForRef: async args => ({{data: {{statuses: args.ref === {json.dumps(PREVIOUS)} ?
    {json.dumps(list(previous))}.map(({{creator, ...status}}) => status) : {json.dumps([latest] if latest else [])}}}}}),
  compareCommitsWithBasehead: {compare},
  listCommitStatusesForRef: () => {{}},
  createCommitStatus: async value => statuses.push(value)
}}}}}};
Object.assign(process.env, {json.dumps(env or {})});
(async () => {{{script}}})().then(() => console.log(JSON.stringify({{statuses, failures, outputs, delays}})))
  .catch(error => console.log(JSON.stringify({{error: error.message, statuses, delays}})));
"""),
    )


def execute_publisher(
    needs: object,
    pull: object,
    superseded: bool = False,
    queue: bool = False,
    latest: object = True,
    creator: object = ACTIONS,
) -> dict[str, Any]:
    """Publish with `latest` the newest Full validation status on the target,
    which the plan marked pending unless it is superseded or None."""
    if latest is True:
        latest = {
            "context": ci.CONTEXT,
            "target_url": RUN.replace("123", "999") if superseded else RUN,
            "creator": creator,
        }
    target = MERGE if queue else HEAD
    return execute_script(
        "ci-request.yml",
        "publish",
        pull,
        env={"RESULTS": json.dumps(needs)},
        statuses={target: [latest] if latest else []},
        **cast(dict[str, Any], QUEUE if queue else {}),
    )


def execute_invalidator(
    event: object, pull: object, latest: object = None, **carry: object
) -> list[dict[str, Any]]:
    return execute_script(
        "ci-invalidate.yml",
        "invalidate",
        pull,
        event=event,
        latest=latest,
        **cast(dict[str, Any], carry),
    )["statuses"]


SYNCHRONIZE = {"action": "synchronize", "before": PREVIOUS}
QUEUE_REF = f"refs/heads/gh-readonly-queue/rust-migration/pr-170-{BASE}"
# A merge group for PR 170: its commit MERGE merges HEAD into PARENT, the base
# with any PRs queued ahead.
QUEUE: dict[str, Any] = {
    "event_name": "merge_group",
    "payload": {
        "merge_group": {
            "head_sha": MERGE,
            "head_ref": QUEUE_REF,
            "base_sha": BASE,
            "base_ref": "refs/heads/rust-migration",
        }
    },
}
GROUP = {"parents": [{"sha": PARENT}, {"sha": HEAD}], "commit": {"tree": {"sha": TREE}}}


def run_policy(
    *,
    event: str = "push",
    on_demand: str = "true",
    files: object = (),
    trees: object = None,
    pulls: Iterable[object] = (),
    statuses: Iterable[object] = (),
    pushed: Iterable[object] = (),
    compare: str | None = None,
) -> dict[str, Any]:
    """Run the actual policy script; `trees` maps the base and pushed tree SHAs.

    `statuses` are the merged PR head's and `pushed` the pushed commit's own.
    """
    script = script_for("ci-policy.yml", "policy")
    compare = compare or f"async () => ({{data: {{files: {json.dumps(files)}}}}})"
    return cast(
        dict[str, Any],
        node(f"""
process.env.CI_ON_DEMAND = {json.dumps(on_demand)};
const outputs = {{}}, delays = [], trees = {json.dumps(trees or {})};
let calls = 0;
const setTimeout = (resolve, delay) => {{ delays.push(delay); resolve(); }};
const context = {{eventName: {json.dumps(event)}, repo: {{owner: 'example', repo: 'fork'}},
  serverUrl: 'https://github.com', sha: 'head',
  payload: {{before: 'base', pull_request: {{base: {{ref: 'rust-migration'}}}}}}}};
const core = {{setOutput: (k, v) => outputs[k] = v, info: () => {{}}, warning: () => {{}}}};
const listed = {{head: {json.dumps(list(pushed))}, {json.dumps(HEAD)}: {json.dumps(list(statuses))}}};
const github = {{paginate: async (method, args) =>
  method === github.rest.repos.listPullRequestsAssociatedWithCommit ? {json.dumps(list(pulls))} : listed[args.ref],
  rest: {{
  git: {{getTree: async args => ({{data: {{sha: args.tree_sha, ...trees[args.tree_sha]}}}})}},
  repos: {{
    getCommit: async args => ({{data: {{commit: {{tree: {{sha: args.ref === 'base' ? '{BASE}' : '{TREE}'}}}}}}}}),
    listPullRequestsAssociatedWithCommit: () => {{}}, listCommitStatusesForRef: () => {{}},
    compareCommitsWithBasehead: {compare}
  }}
}}}};
(async () => {{{script}}})().then(() => console.log(JSON.stringify({{outputs, calls, delays}})))
  .catch(error => console.log(JSON.stringify({{outputs, calls, delays, error: error.message}})));
"""),
    )


def entry(
    path: str, sha: str = HEAD, mode: str = "100644", kind: str = "blob"
) -> dict[str, str]:
    return {"path": path, "type": kind, "mode": mode, "sha": sha}


def tree(*entries: dict[str, str], **fields: object) -> dict[str, object]:
    return {"truncated": False, "tree": list(entries), **fields}


def validation(
    full: str = "success",
    marker: str = "success",
    run: str | None = RUN,
    marker_run: str | None = RUN,
    tree: str = TREE,
) -> list[dict[str, object]]:
    """Full validation status and its Validated tree marker on a merged head."""
    return [
        {
            "context": "Full validation",
            "state": full,
            "target_url": run,
            "creator": ACTIONS,
        },
        {
            "context": "Validated tree",
            "state": marker,
            "target_url": marker_run,
            "description": tree,
            "creator": ACTIONS,
        },
    ]


CODE, DOC = entry("src/game.cpp"), entry("docs/a.md", BASE)
INVENTORY = tree(CODE, DOC)


def inventory_cases() -> list[tuple[dict[str, object], str]]:
    """Trees compared with INVENTORY; "false" means the same non-Markdown inventory."""
    moved_doc = entry("docs/new.md", MERGE)
    return [
        (tree(moved_doc, CODE), "false"),
        (tree(CODE), "false"),
        (tree(dict(CODE, sha=MERGE)), "true"),
        (tree(dict(CODE, mode="100755")), "true"),
        (tree(dict(CODE, path="src/other.cpp")), "true"),
        (tree(dict(CODE, type="commit", mode="160000")), "true"),
        (tree(moved_doc), "true"),
        (tree(CODE, truncated=True), "true"),
        ({"tree": [CODE]}, "true"),
        ({"truncated": False}, "true"),
        (tree(), "true"),
        (tree({}), "true"),
        (tree(CODE, dict(DOC, sha="bad")), "true"),
        (tree(CODE, dict(DOC, mode="bad")), "true"),
        (tree(CODE, DOC, DOC), "true"),
        (tree(CODE, dict(DOC, type="commit", mode="160000")), "true"),
        (tree(CODE, dict(DOC, path="../a.md")), "true"),
        # Packaged or root-level Markdown is a build input.
        (tree(CODE, DOC, entry("README.md", MERGE)), "true"),
        (tree(CODE, DOC, entry("docs/fonts.md", MERGE)), "true"),
        (tree(CODE, dict(DOC, path="docs/fonts.md")), "true"),
        (tree(CODE, sha=None), "true"),
    ]


def runs(step: dict[str, Any], cache_only: bool, steps: dict[str, Any]) -> object:
    """Evaluate a ci-linux or ci-windows step condition on a protected push."""
    return evaluate(
        step.get("if", True),
        inputs={
            "cache-only": cache_only,
            "rust": True,
            "compiler": "gcc",
            "semantic-subset": False,
        },
        github={"event_name": "push", "ref": "refs/heads/rust-migration"},
        steps=steps,
    )


class GateTests(unittest.TestCase):
    def test_plan_checks_mergeability_and_marks_only_full_requests_pending(
        self,
    ) -> None:
        for mergeable, profile in itertools.product(
            (True, False, None), ("rust", "full")
        ):
            with self.subTest(mergeable=mergeable, profile=profile):
                env = {"PR": "170", "HEAD": HEAD, "PROFILE": profile}
                pull = dict(PULL, mergeable=mergeable)
                result = execute_script("ci-request.yml", "plan", pull, env=env)
                pending = [(HEAD, "pending")] if profile == "full" else []
                if mergeable is not True:
                    self.assertIn("unmergeable", result["error"])
                    pending = []
                else:
                    self.assertEqual(result["outputs"]["merge"], MERGE)
                states = [(s["sha"], s["state"]) for s in result["statuses"]]
                self.assertEqual(states, pending)

    def test_checker_only_full_requests_skip_builds(self) -> None:
        checker = [
            {"filename": "tools/lint-ratchet.py"},
            {"filename": "tools/test-cleanup-checks.py"},
            {"filename": "migration/clippy-debt.json"},
            {"filename": "migration/lints/clippy.toml"},
            {"filename": "docs/roadmap.md"},
            {"filename": "AGENTS.md"},
        ]
        renamed = {
            "filename": "tools/cleanup_contract.py",
            "previous_filename": "tools/ci.py",
        }
        for label, profile, files, expected in (
            ("checker only", "full", checker, "true"),
            (
                "checker rename",
                "full",
                [dict(renamed, previous_filename="tools/lint-policy.py")],
                "true",
            ),
            ("rename from a built file", "full", [renamed], "false"),
            (
                "rename into packaged docs",
                "full",
                [{"filename": "docs/fonts.md", "previous_filename": "docs/x.md"}],
                "false",
            ),
            ("root Markdown", "full", [{"filename": "README.md"}], "false"),
            ("packaged docs", "full", [{"filename": "docs/desync.md"}], "false"),
            (
                "other Markdown",
                "full",
                [{"filename": "media/baseset/OpenTTD-font.md"}],
                "false",
            ),
            (
                "workflow",
                "full",
                [*checker, {"filename": ".github/workflows/ci-request.yml"}],
                "false",
            ),
            ("other tool", "full", [{"filename": "tools/simulate.py"}], "false"),
            ("near miss", "full", [{"filename": "tools/lint-ratchet.pyc"}], "false"),
            ("nested", "full", [{"filename": "src/tools/lint-ratchet.py"}], "false"),
            ("source", "full", [{"filename": "src/rust/x.h"}], "false"),
            ("no filename", "full", [{}], "false"),
            ("no files", "full", [], "false"),
            ("missing list", "full", None, "false"),
            ("comparison cap", "full", [{"filename": "docs/a.md"}] * 300, "false"),
            ("rust profile", "rust", checker, "false"),
            ("merge base is not the first parent", "full", checker, "false"),
        ):
            with self.subTest(label):
                env = {"PR": "170", "HEAD": HEAD, "PROFILE": profile}
                data: dict[str, object] = {} if files is None else {"files": files}
                # An overlapping base advance can give a stale test merge another
                # best common ancestor; only the first parent's own diff counts.
                merge_base = HEAD if label.startswith("merge base") else PARENT
                data["merge_base_commit"] = {"sha": merge_base}
                compare = f"""async args => {{
                  if (args.basehead !== '{PARENT}...{MERGE}') throw new Error('basehead');
                  return {{data: {json.dumps(data)}}};
                }}"""
                result = execute_script(
                    "ci-request.yml", "plan", PULL, env=env, compare=compare
                )
                self.assertEqual(result["outputs"]["checker"], expected)
        for name in ("native", "platform"):
            condition = job("ci-request.yml", name)["if"]
            for value, expected in (("true", False), ("false", True)):
                outputs = {"profile": "full", "checker": value, "carried": ""}
                needs = {"plan": {"result": "success", "outputs": outputs}}
                self.assertEqual(evaluate(condition, needs=needs), expected)

    def test_inert_markdown_excludes_every_markdown_build_input(self) -> None:
        sources = [ROOT / "CMakeLists.txt", *sorted((ROOT / "cmake").glob("*.cmake"))]
        built = {
            match
            for path in sources
            for match in re.findall(
                r"\$\{CMAKE_SOURCE_DIR\}/([^\s\"@]+\.md)", path.read_text()
            )
        }
        self.assertIn("docs/fonts.md", built)
        tracked = subprocess.check_output(
            ["git", "ls-files", "*.md"], cwd=ROOT, text=True
        ).split()
        blocks = []
        for filename, name in (
            ("ci-request.yml", "plan"),
            ("ci-invalidate.yml", "invalidate"),
            ("ci-policy.yml", "policy"),
        ):
            script = script_for(filename, name)
            start = script.index("// Markdown that no build reads")
            blocks.append(
                script[start : script.index(");", script.index("const inert")) + 2]
            )
        # Indentation follows each script's nesting; the code itself must match.
        self.assertEqual(len({" ".join(block.split()) for block in blocks}), 1)
        paths = sorted(
            built | set(tracked) | {"docs/skills/x/SKILL.md", "docs/fonts.md.txt"}
        )
        verdicts = cast(
            dict[str, bool],
            node(
                blocks[0]
                + f"\nconsole.log(JSON.stringify(Object.fromEntries({json.dumps(paths)}.map(p => [p, inert(p)]))));"
            ),
        )
        for path in built:
            self.assertFalse(verdicts[path], path)
        for path, verdict in verdicts.items():
            expected = (
                path == "AGENTS.md"
                or path.startswith("docs/")
                and path.endswith(".md")
                and path not in built
            )
            self.assertEqual(verdict, expected, path)

    def test_checker_only_publisher_needs_quick_and_marks_no_tree(self) -> None:
        def needs(quick: str) -> dict[str, dict[str, Any]]:
            result = good_needs()
            result["plan"]["outputs"]["checker"] = "true"
            result["quick"]["result"] = quick
            for name in ("native", "platform"):
                result[name] = {"result": "skipped", "outputs": {}}
            return result

        passed = execute_publisher(needs("success"), PULL)
        self.assertEqual([s["context"] for s in passed["statuses"]], [ci.CONTEXT])
        self.assertEqual(passed["statuses"][0]["state"], "success")
        self.assertFalse(passed["failures"])
        for outcome in ("failure", "skipped", "cancelled"):
            with self.subTest(outcome):
                failed = execute_publisher(needs(outcome), PULL)
                self.assertEqual(failed["statuses"][0]["state"], "failure")
        moved = dict(PULL, head={"sha": "c" * 40})
        self.assertEqual(
            execute_publisher(needs("success"), moved)["statuses"][0]["state"],
            "failure",
        )
        # Without the plan's checker output, skipped builds still fail.
        unmarked = needs("success")
        del unmarked["plan"]["outputs"]["checker"]
        self.assertEqual(
            execute_publisher(unmarked, PULL)["statuses"][0]["state"], "failure"
        )

    def test_privileged_jobs_run_only_for_protected_full_requests(self) -> None:
        plan = job("ci-request.yml", "plan")["if"]
        repo = {"repo": {"full_name": "fork"}}
        same = {"label": {"name": "ci:full"}, "pull_request": {"head": repo}}
        fork = dict(same, pull_request={"head": {"repo": {"full_name": "other"}}})
        dispatch, pull_request = "workflow_dispatch", "pull_request"
        queue = {"merge_group": {"base_ref": "refs/heads/rust-migration"}}
        other = {"merge_group": {"base_ref": "refs/heads/master"}}
        for event, ref, on_demand, payload, expected in (
            (dispatch, "refs/heads/rust-migration", "true", {}, True),
            (dispatch, "refs/heads/feature", "true", {}, False),
            ("merge_group", QUEUE_REF, "true", queue, True),
            ("merge_group", QUEUE_REF, "", queue, True),
            ("merge_group", QUEUE_REF, "true", other, False),
            (pull_request, "refs/pull/1/merge", "false", same, True),
            (pull_request, "refs/pull/1/merge", "true", same, False),
            (pull_request, "refs/pull/1/merge", "false", fork, False),
            (pull_request, "refs/pull/1/merge", "false", {**same, "label": {}}, False),
        ):
            github = {"event_name": event, "ref": ref, "event": payload}
            github["repository"] = "fork"
            variables = {"CI_ON_DEMAND": on_demand}
            with self.subTest(event=event, ref=ref, on_demand=on_demand):
                result = evaluate(plan, github=github, vars=variables)
                self.assertEqual(result, expected)
        publish = job("ci-request.yml", "publish")["if"]
        for event, result, profile, expected in (
            (dispatch, "success", "full", True),
            (dispatch, "success", "rust", False),
            (dispatch, "failure", "full", False),
            # A merge group publishes a failed plan, so the queue drops the PR at once.
            ("merge_group", "success", "full", True),
            ("merge_group", "failure", "", True),
            ("merge_group", "cancelled", "", True),
            ("merge_group", "skipped", "", False),
        ):
            needs = {"plan": {"result": result, "outputs": {"profile": profile}}}
            github = {"event_name": event}
            self.assertEqual(evaluate(publish, needs=needs, github=github), expected)

    def test_publisher_requires_every_job_and_the_unchanged_head(self) -> None:
        cases = [("success", good_needs(), PULL, False, "success")]
        cases.append(("superseded", good_needs(), PULL, True, None))
        for creator in (None, {}, {"login": "writer", "type": "User"}):
            # A writer's status naming this run does not make the request this run's.
            with self.subTest(creator=creator):
                result = execute_publisher(good_needs(), PULL, creator=creator)
                self.assertEqual(result["statuses"], [])
                self.assertTrue(result["failures"])
        for name in good_needs():
            for outcome in ("failure", "skipped", "cancelled", "timed_out", ""):
                needs = good_needs()
                needs[name]["result"] = outcome
                cases.append((f"{name} {outcome}", needs, PULL, False, "failure"))
        for name in ("quick", "native", "platform"):
            needs = good_needs()
            del needs[name]
            cases.append((f"missing {name}", needs, PULL, False, "failure"))
            if name != "quick":
                needs = good_needs()
                needs[name]["outputs"] = {}
                cases.append((f"unproved {name}", needs, PULL, False, "failure"))
        for change, expected in (
            ({"head": {"sha": "c" * 40}}, "failure"),
            ({"state": "closed"}, "failure"),
            ({"base": {"ref": "master", "sha": BASE}}, "failure"),
            # Base advances and pending mergeability keep exact-head validation.
            ({"base": {"ref": "rust-migration", "sha": "e" * 40}}, "success"),
            ({"merge_commit_sha": "e" * 40}, "success"),
            ({"mergeable": None}, "success"),
            ({"mergeable": False}, "success"),
        ):
            pull = dict(PULL, **change)
            cases.append((str(change), good_needs(), pull, False, expected))
        for label, needs, pull, superseded, expected in cases:
            with self.subTest(label):
                result = execute_publisher(needs, pull, superseded)
                if expected is None:
                    self.assertEqual(result["statuses"], [])
                    self.assertTrue(result["failures"])
                    continue
                full, *marker = result["statuses"]
                observed = (full["state"], full["sha"], full["context"])
                self.assertEqual(observed, (expected, HEAD, ci.CONTEXT))
                if expected == "success":
                    self.assertFalse(result["failures"])
                    tree = ("Validated tree", TREE, full["target_url"])
                    observed = [
                        (m["context"], m["description"], m["target_url"])
                        for m in marker
                    ]
                    self.assertEqual(observed, [tree])

    def test_status_writers_share_a_non_cancelling_per_pr_concurrency_group(
        self,
    ) -> None:
        groups = []
        for filename, name in (
            ("ci-request.yml", "plan"),
            ("ci-request.yml", "publish"),
            ("ci-invalidate.yml", "invalidate"),
        ):
            concurrency = job(filename, name)["concurrency"]
            self.assertIs(concurrency["cancel-in-progress"], False)
            # Dispatch inputs are strings; pull_request_target provides a number.
            github = {"event": {"pull_request": {"number": 170}}}
            for inputs in ({"pr": "170"}, {}):
                group = concurrency["group"]
                groups.append(
                    evaluate(group, interpolate=True, inputs=inputs, github=github)
                )
        self.assertEqual(groups, ["validation-status-170"] * 6)

    def test_invalidation_only_marks_current_unvalidated_head_pending(self) -> None:
        event = dict(PULL, number=170, updated_at="2026-10-07T12:00:00Z")
        moved = dict(PULL, head={"sha": "c" * 40})
        # A delayed invalidation preserves a request started at or after the push.
        for updated, pull, expected in (
            (None, PULL, [HEAD]),
            ("2026-10-07T11:59:59Z", PULL, [HEAD]),
            (event["updated_at"], PULL, []),
            ("2026-10-07T12:00:01Z", PULL, []),
            (None, moved, []),
        ):
            latest = updated and {"context": ci.CONTEXT, "updated_at": updated}
            with self.subTest(updated=updated, pull=pull["head"]):
                statuses = execute_invalidator(event, pull, latest)
                observed = [(s["sha"], s["state"], s["context"]) for s in statuses]
                self.assertEqual(
                    observed, [(sha, "pending", ci.CONTEXT) for sha in expected]
                )

    def test_unchanged_non_markdown_tree_carries_the_previous_head_result(self) -> None:
        event = dict(PULL, number=170, updated_at="2026-10-07T12:00:00Z")
        carried = [
            (HEAD, "success", ci.CONTEXT, RUN, None),
            (HEAD, "success", "Validated tree", RUN, BASE),
        ]
        pending = [(HEAD, "pending", ci.CONTEXT, None, None)]
        good = {"payload": SYNCHRONIZE, "previous": validation(tree=BASE)}
        behind = "async () => ({data: {behind_by: 1}})"
        missing = "async () => { throw new Error('Not Found'); }"
        bad_runs = (None, "", "run", RUN + "/extra", RUN.replace("fork", "other"))
        cases = [("matching tree", {}, carried)]
        for index in (0, 1):
            for creator in (None, {}, {"login": "writer", "type": "User"}):
                forged = validation(tree=BASE)
                forged[index] = dict(forged[index], creator=creator)
                cases.append(
                    (
                        f"untrusted status {index}: {creator}",
                        {"previous": forged},
                        pending,
                    )
                )
                # A newer untrusted status must not expose an older trusted result.
                cases.append(
                    (
                        f"untrusted over trusted {index}: {creator}",
                        {"previous": [forged[index], *validation(tree=BASE)]},
                        pending,
                    )
                )
        for label, change in (
            ("opened", {"payload": {"action": "opened", "before": PREVIOUS}}),
            ("reopened", {"payload": {"action": "reopened"}}),
            ("no previous head", {"payload": {"action": "synchronize"}}),
            ("bad previous head", {"payload": dict(SYNCHRONIZE, before="9")}),
            ("head behind base", {"compare": behind}),
            ("lookup failure", {"compare": missing}),
            ("no previous result", {"previous": []}),
            ("pending result", {"previous": validation("pending", tree=BASE)}),
            ("failed result", {"previous": validation("failure", tree=BASE)}),
            ("failed marker", {"previous": validation(marker="failure", tree=BASE)}),
            ("other marker run", {"previous": validation(marker_run="old", tree=BASE)}),
            ("marker not a tree", {"previous": validation(tree="tree")}),
            ("marker missing", {"previous": validation(tree=BASE)[:1]}),
            *(
                (
                    f"run {run}",
                    {"previous": validation(run=run, marker_run=run, tree=BASE)},
                )
                for run in bad_runs
            ),
        ):
            cases.append((label, change, pending))
        for label, change, expected in cases:
            with self.subTest(label):
                carry = {"trees": {BASE: INVENTORY, TREE: tree(CODE)}, **good, **change}
                statuses = execute_invalidator(event, PULL, **carry)
                observed = [
                    (
                        s["sha"],
                        s["state"],
                        s["context"],
                        s.get("target_url"),
                        s["description"] if s["context"] == "Validated tree" else None,
                    )
                    for s in statuses
                ]
                self.assertEqual(observed, expected)
        # A request started after the push keeps its own status.
        latest = {"context": ci.CONTEXT, "updated_at": "2026-10-07T12:00:01Z"}
        carry = {"trees": {BASE: INVENTORY, TREE: tree(CODE)}, **good}
        self.assertEqual(execute_invalidator(event, PULL, latest, **carry), [])

    def test_carry_and_post_merge_refresh_share_the_inventory_rule(self) -> None:
        event = dict(PULL, number=170, updated_at="2026-10-07T12:00:00Z")
        for after, expected in inventory_cases():
            for incomplete_side in (False, True):
                left, right = (
                    (after, INVENTORY) if incomplete_side else (INVENTORY, after)
                )
                with self.subTest(after=after, incomplete_side=incomplete_side):
                    statuses = execute_invalidator(
                        event,
                        PULL,
                        payload=SYNCHRONIZE,
                        previous=validation(tree=BASE),
                        trees={BASE: left, TREE: right},
                    )
                    state = "success" if expected == "false" else "pending"
                    self.assertEqual(statuses[0]["state"], state)

    def test_write_jobs_never_check_out_or_run_candidate_code(self) -> None:
        # Structure is the behavior here: pull_request_target and workflow_run jobs,
        # and every job granted a write permission, execute only the github-script
        # body of the protected definition.
        privileged = []
        for path in sorted(WORKFLOWS.glob("*.yml")):
            definition = yaml.safe_load(path.read_text())
            triggers = definition[True]
            untrusted = isinstance(triggers, dict) and bool(
                {"pull_request_target", "workflow_run"} & set(triggers)
            )
            for name, body in definition["jobs"].items():
                permissions = body.get("permissions", definition.get("permissions", {}))
                writes = permissions == "write-all" or (
                    isinstance(permissions, dict) and "write" in permissions.values()
                )
                if not (writes or untrusted):
                    continue
                privileged.append((path.name, name))
                with self.subTest(workflow=path.name, job=name):
                    self.assertNotIn("uses", body)
                    for step in body["steps"]:
                        self.assertNotIn("run", step)
                        self.assertRegex(step["uses"], r"^actions/github-script@")
        self.assertTrue(privileged)


EARLIER = "c" * 40  # An earlier group commit of the same PR head.
EARLIER_RUN = RUN.replace("123", "100")


def earlier_run(**fields: object) -> dict[str, object]:
    """The workflow run that tested EARLIER, before the queue rebuilt the group."""
    branch = f"gh-readonly-queue/rust-migration/pr-170-{PREVIOUS}"
    return {
        "id": 100,
        "event": "merge_group",
        "head_branch": branch,
        "head_sha": EARLIER,
        "status": "completed",
        "conclusion": "success",
        **fields,
    }


def plan_queue(
    *,
    listed: Iterable[object] | None = None,
    commits: Mapping[str, object] | None = None,
    statuses: Mapping[str, Iterable[object]] | None = None,
    trees: object = None,
    **options: object,
) -> dict[str, Any]:
    """Plan a merge group for PR 170 whose earlier group EARLIER passed in run 100.

    EARLIER's tree BASE differs from the group's tree TREE only in Markdown.
    """
    runs = json.dumps(list(listed) if listed is not None else [earlier_run()])
    options.setdefault(
        "runs",
        f"""async args => {{
      if (args.workflow_id !== 'ci-request.yml' || args.event !== 'merge_group') throw new Error('runs');
      return {{data: {{workflow_runs: {runs}}}}};
    }}""",
    )
    earlier = {
        "parents": [{"sha": BASE}, {"sha": HEAD}],
        "commit": {"tree": {"sha": BASE}},
    }
    return execute_script(
        "ci-request.yml",
        "plan",
        options.pop("pull", PULL),
        env={"PR": "", "HEAD": "", "PROFILE": str(options.pop("profile", "full"))},
        commits={MERGE: GROUP, EARLIER: earlier, **(commits or {})},
        statuses=statuses
        if statuses is not None
        else {EARLIER: validation(run=EARLIER_RUN, marker_run=EARLIER_RUN, tree=BASE)},
        trees=trees if trees is not None else {TREE: tree(CODE), BASE: INVENTORY},
        **cast(dict[str, Any], {**QUEUE, **options}),
    )


class MergeQueueTests(unittest.TestCase):
    def test_merge_group_plan_tests_the_group_commit_against_its_first_parent(
        self,
    ) -> None:
        # Mergeability is the PR's own merge, which the queue does not test.
        result = plan_queue(listed=[], pull=dict(PULL, mergeable=None))
        self.assertEqual(
            result["outputs"],
            {
                "head": HEAD,
                # The first parent is the base plus any PRs queued ahead, so a
                # second queued PR's gates count only its own changes.
                "base": PARENT,
                "merge": MERGE,
                "tree": TREE,
                "profile": "full",
                "pr": "170",
                "checker": "false",
                "carried": "",
                "carried-tree": "",
            },
        )
        observed = [(s["sha"], s["state"], s["target_url"]) for s in result["statuses"]]
        self.assertEqual(observed, [(MERGE, "pending", RUN)])
        group = QUEUE["payload"]["merge_group"]
        other_head = dict(GROUP, parents=[{"sha": PARENT}, {"sha": "e" * 40}])
        for label, change in (
            ("other base", {"head_ref": QUEUE_REF.replace("rust-migration", "master")}),
            ("no number", {"head_ref": QUEUE_REF.replace("pr-170", "pr-")}),
            ("zero", {"head_ref": QUEUE_REF.replace("pr-170", "pr-0")}),
            ("suffix", {"head_ref": QUEUE_REF + "x"}),
            ("missing ref", {"head_ref": None}),
            ("bad group commit", {"head_sha": "commit"}),
        ):
            with self.subTest(label):
                payload = {"merge_group": dict(group, **change)}
                failed = plan_queue(payload=payload)
                self.assertIn("Invalid validation request", failed["error"])
                self.assertEqual(failed["statuses"], [])
        for label, options in (
            ("other head", {"commits": {MERGE: other_head}}),
            (
                "one parent",
                {"commits": {MERGE: dict(GROUP, parents=[{"sha": PARENT}])}},
            ),
            ("closed", {"pull": dict(PULL, state="closed")}),
            ("other base", {"pull": dict(PULL, base={"ref": "master", "sha": BASE})}),
            ("native profile", {"profile": "native"}),
        ):
            with self.subTest(label):
                failed = plan_queue(**cast(dict[str, Any], options))
                self.assertTrue(failed["error"])
                self.assertEqual(failed["statuses"], [])

    def test_rebuilt_group_carries_an_earlier_run_of_the_same_head_and_tree(
        self,
    ) -> None:
        running = earlier_run(status="in_progress", conclusion=None)
        trusted = validation(run=EARLIER_RUN, marker_run=EARLIER_RUN, tree=BASE)
        bad_runs = (None, "", "run", RUN + "/extra", RUN.replace("fork", "other"))
        checker = (
            """async () => ({data: {merge_base_commit: {sha: '%s'},
          files: [{filename: 'tools/lint-ratchet.py'}]}})"""
            % PARENT
        )
        chained = RUN.replace("123", "50")
        carried = (EARLIER_RUN, BASE)
        cases: list[tuple[str, dict[str, Any], tuple[str, str], list[int]]] = [
            ("passed", {}, carried, []),
            (
                "still testing",
                {"listed": [running], "progress": {100: [running, earlier_run()]}},
                carried,
                [30000, 30000],
            ),
            # An earlier group that itself carried a result passes it on.
            (
                "carried earlier",
                {
                    "statuses": {
                        EARLIER: validation(
                            run=chained, marker_run=chained, tree=PREVIOUS
                        )
                    },
                    "trees": {TREE: tree(CODE), BASE: INVENTORY, PREVIOUS: INVENTORY},
                },
                (chained, PREVIOUS),
                [],
            ),
        ]
        normal: list[tuple[str, dict[str, Any], list[int]]] = [
            ("no earlier run", {"listed": []}, []),
            ("later run", {"listed": [earlier_run(id=200)]}, []),
            ("this run", {"listed": [earlier_run(id=123)]}, []),
            ("other event", {"listed": [earlier_run(event="pull_request")]}, []),
            ("failed", {"listed": [earlier_run(conclusion="failure")]}, []),
            ("cancelled", {"listed": [earlier_run(conclusion="cancelled")]}, []),
            ("bad commit", {"listed": [earlier_run(head_sha="commit")]}, []),
            (
                "still testing at the deadline",
                {
                    "listed": [running],
                    "progress": {100: [running] * 5},
                    "tick": 20 * 60000,
                },
                [30000, 30000],
            ),
            (
                "failed after the wait",
                {
                    "listed": [running],
                    "progress": {100: [earlier_run(conclusion="failure")]},
                },
                [30000],
            ),
            (
                "other head",
                {
                    "commits": {
                        EARLIER: dict(GROUP, parents=[{"sha": BASE}, {"sha": "e" * 40}])
                    }
                },
                [],
            ),
            (
                "other tree",
                {"trees": {TREE: tree(CODE), BASE: tree(dict(CODE, sha=MERGE))}},
                [],
            ),
            (
                "incomplete tree",
                {"trees": {TREE: tree(CODE), BASE: tree(CODE, truncated=True)}},
                [],
            ),
            (
                "marker tree differs",
                {
                    "statuses": {
                        EARLIER: validation(
                            run=EARLIER_RUN, marker_run=EARLIER_RUN, tree=PREVIOUS
                        )
                    },
                    "trees": {
                        TREE: tree(CODE),
                        BASE: INVENTORY,
                        PREVIOUS: tree(dict(CODE, sha=MERGE)),
                    },
                },
                [],
            ),
            ("no statuses", {"statuses": {EARLIER: []}}, []),
            (
                "listing fails",
                {"runs": "async () => { throw new Error('Not Found'); }"},
                [],
            ),
            ("status lookup fails", {"statuses": {}}, []),
            # A checker-only group runs Quick alone, which costs less than a wait.
            ("checker-only group", {"compare": checker}, []),
        ]
        for branch in ("pr-17", "pr-1700", "pr-170x", "master/pr-170"):
            name = f"gh-readonly-queue/rust-migration/{branch}-{PREVIOUS}"
            if branch.startswith("master"):
                name = f"gh-readonly-queue/{branch}-{PREVIOUS}"
            normal.append(
                (f"branch {branch}", {"listed": [earlier_run(head_branch=name)]}, [])
            )
        for state, marker in (
            ("pending", "success"),
            ("failure", "success"),
            ("success", "failure"),
        ):
            forged = validation(state, marker, EARLIER_RUN, EARLIER_RUN, BASE)
            normal.append(
                (f"result {state} {marker}", {"statuses": {EARLIER: forged}}, [])
            )
        for index in (0, 1):
            for creator in (None, {}, {"login": "writer", "type": "User"}):
                forged = [dict(status) for status in trusted]
                forged[index]["creator"] = creator
                normal.append(
                    (
                        f"untrusted {index} {creator}",
                        {"statuses": {EARLIER: forged}},
                        [],
                    )
                )
        for label, statuses in (
            (
                "marker of another run",
                validation(run=EARLIER_RUN, marker_run=RUN, tree=BASE),
            ),
            (
                "marker not a tree",
                validation(run=EARLIER_RUN, marker_run=EARLIER_RUN, tree="tree"),
            ),
            ("marker missing", trusted[:1]),
            *(
                (f"run {run}", validation(run=run, marker_run=run, tree=BASE))
                for run in bad_runs
            ),
        ):
            normal.append((label, {"statuses": {EARLIER: statuses}}, []))
        cases += [
            (label, options, ("", ""), delays) for label, options, delays in normal
        ]
        for label, options, (url, validated), delays in cases:
            with self.subTest(label):
                result = plan_queue(**options)
                outputs = result["outputs"]
                self.assertEqual(
                    (outputs["carried"], outputs["carried-tree"]), (url, validated)
                )
                self.assertEqual(result["delays"], delays)
                # The group is marked pending either way; only the publisher decides.
                self.assertEqual([s["state"] for s in result["statuses"]], ["pending"])
        # A requested Full of a PR head never carries a merge group's result.
        requested = execute_script(
            "ci-request.yml",
            "plan",
            PULL,
            env={"PR": "170", "HEAD": HEAD, "PROFILE": "full"},
            commits={MERGE: GROUP},
            runs=f"async () => ({{data: {{workflow_runs: {json.dumps([earlier_run()])}}}}})",
        )
        self.assertEqual(requested["outputs"]["carried"], "")

    def test_group_carry_uses_the_shared_inventory_rule(self) -> None:
        for after, expected in inventory_cases():
            for incomplete_side in (False, True):
                left, right = (
                    (after, INVENTORY) if incomplete_side else (INVENTORY, after)
                )
                with self.subTest(after=after, incomplete_side=incomplete_side):
                    result = plan_queue(trees={BASE: left, TREE: right})
                    carried = EARLIER_RUN if expected == "false" else ""
                    self.assertEqual(result["outputs"]["carried"], carried)

    def test_carried_group_skips_every_validation_job(self) -> None:
        for carried in ("", EARLIER_RUN):
            outputs = {"profile": "full", "checker": "false", "carried": carried}
            needs = {"plan": {"result": "success", "outputs": outputs}}
            for name in ("quick", "native", "platform"):
                with self.subTest(job=name, carried=carried):
                    condition = job("ci-request.yml", name)["if"]
                    self.assertEqual(evaluate(condition, needs=needs), not carried)

    def test_group_publisher_writes_the_group_commit(self) -> None:
        def observe(result: dict[str, Any]) -> list[tuple[object, ...]]:
            return [
                (s["sha"], s["state"], s["context"], s["target_url"], s["description"])
                for s in result["statuses"]
            ]

        passed = execute_publisher(
            dict(good_needs(), plan=dict(good_needs()["plan"])), PULL, queue=True
        )
        description = "All required validation passed for this PR head"
        self.assertEqual(
            observe(passed),
            [
                (MERGE, "success", ci.CONTEXT, RUN, description),
                (MERGE, "success", "Validated tree", RUN, TREE),
            ],
        )
        failed = good_needs()
        failed["native"]["result"] = "failure"
        self.assertEqual(
            observe(execute_publisher(failed, PULL, queue=True))[0][:4],
            (MERGE, "failure", ci.CONTEXT, RUN),
        )
        carried = good_needs()
        carried["plan"]["outputs"].update(
            {"carried": EARLIER_RUN, "carried-tree": BASE}
        )
        for name in ("quick", "native", "platform"):
            carried[name] = {"result": "skipped", "outputs": {}}
        result = execute_publisher(carried, PULL, queue=True)
        self.assertFalse(result["failures"])
        description = "Carried from run 100: non-Markdown tree unchanged"
        self.assertEqual(
            observe(result),
            [
                (MERGE, "success", ci.CONTEXT, EARLIER_RUN, description),
                (MERGE, "success", "Validated tree", EARLIER_RUN, BASE),
            ],
        )
        moved = execute_publisher(
            carried, dict(PULL, head={"sha": "e" * 40}), queue=True
        )
        self.assertEqual(observe(moved)[0][:4], (MERGE, "failure", ci.CONTEXT, RUN))
        # A failed plan marked nothing pending; the group still fails at once.
        unplanned = {"plan": {"result": "failure", "outputs": {}}}
        for name in ("quick", "native", "platform"):
            unplanned[name] = {"result": "skipped", "outputs": {}}
        result = execute_publisher(unplanned, PULL, queue=True, latest=None)
        self.assertEqual(
            observe(result),
            [(MERGE, "failure", ci.CONTEXT, RUN, "Merge group plan failed")],
        )
        self.assertTrue(result["failures"])
        superseded = execute_publisher(good_needs(), PULL, superseded=True, queue=True)
        self.assertEqual(superseded["statuses"], [])
        for creator in (None, {}, {"login": "writer", "type": "User"}):
            with self.subTest(creator=creator):
                forged = execute_publisher(
                    good_needs(), PULL, queue=True, creator=creator
                )
                self.assertEqual(forged["statuses"], [])
                self.assertTrue(forged["failures"])
        # Outside the queue an unmarked head is still a superseded request.
        self.assertEqual(
            execute_publisher(good_needs(), PULL, latest=None)["statuses"], []
        )

    def test_group_runs_and_status_locks_are_per_run(self) -> None:
        definition = workflow("ci-request.yml")
        github = {"event_name": "merge_group", "run_id": 42, "event": QUEUE["payload"]}
        expressions = [
            definition["concurrency"]["group"],
            job("ci-request.yml", "plan")["concurrency"]["group"],
            job("ci-request.yml", "publish")["concurrency"]["group"],
        ]
        groups = [
            evaluate(group, interpolate=True, inputs={}, github=github)
            for group in expressions
        ]
        self.assertEqual(
            groups,
            [
                "requested-ci-queue-42-full",
                "validation-status-queue-42",
                "validation-status-queue-42",
            ],
        )

    def test_run_names_keep_request_titles_and_name_the_queue(self) -> None:
        name = workflow("ci-request.yml")["run-name"]
        for inputs, github, expected in (
            (
                {"profile": "rust", "pr": "170", "request": "unique"},
                {"event_name": "workflow_dispatch", "run_id": 7, "event": {}},
                "CI rust PR 170 unique",
            ),
            (
                {},
                {
                    "event_name": "pull_request",
                    "run_id": 7,
                    "event": {"pull_request": {"number": 170}},
                },
                "CI full PR 170 bootstrap-7",
            ),
            (
                {},
                {"event_name": "merge_group", "run_id": 7, "event": QUEUE["payload"]},
                f"CI full queue {QUEUE_REF}",
            ),
        ):
            with self.subTest(github["event_name"]):
                self.assertEqual(
                    evaluate(name, interpolate=True, inputs=inputs, github=github),
                    expected,
                )

    def test_required_pull_request_checks_report_skipped_in_the_queue(self) -> None:
        # A required check must report on the group commit, and skipped counts as
        # passed; Full's own Quick runs these checks on the group.
        for filename, name in (
            ("commit-checker.yml", "commit-checker"),
            ("unused-strings.yml", "unused-strings"),
            ("script-missing-mode-enforcement.yml", "script-missing-mode-enforcement"),
            ("ci-quick.yml", "quick"),
        ):
            triggers = cast(dict[Any, Any], workflow(filename))[True]
            self.assertIn("merge_group", triggers, filename)
            condition = job(filename, name)["if"]
            for event, checkout, expected in (
                ("pull_request", "", True),
                ("push", "", True),
                ("merge_group", "", False),
                # Full calls Quick inside the merge group.
                ("merge_group", MERGE, filename == "ci-quick.yml"),
            ):
                with self.subTest(filename=filename, event=event, checkout=checkout):
                    observed = evaluate(
                        condition,
                        github={"event_name": event},
                        inputs={"checkout-ref": checkout},
                    )
                    self.assertEqual(observed, expected)

    def test_queue_mode_satisfies_a_head_without_workflow_changes(self) -> None:
        event = dict(PULL, number=170, updated_at="2026-10-07T12:00:00Z")
        code = [{"filename": "src/game.cpp"}]
        queued = (HEAD, "success", ci.CONTEXT, RUN, ci.QUEUED)
        pending = (
            HEAD,
            "pending",
            ci.CONTEXT,
            None,
            "Request full CI for this PR head before merge",
        )
        for label, mode, files, expected in (
            ("code", "true", code, queued),
            (
                "local action",
                "true",
                [{"filename": ".github/actions/apt-install/action.yml"}],
                queued,
            ),
            ("mode unset", "", code, pending),
            ("mode off", "false", code, pending),
            (
                "workflow",
                "true",
                [*code, {"filename": ".github/workflows/ci-request.yml"}],
                pending,
            ),
            (
                "workflow renamed away",
                "true",
                [
                    {
                        "filename": "tools/x.yml",
                        "previous_filename": ".github/workflows/x.yml",
                    }
                ],
                pending,
            ),
            ("no filename", "true", [{}], pending),
            (
                "bad rename",
                "true",
                [{"filename": "a", "previous_filename": 1}],
                pending,
            ),
            ("no files", "true", [], pending),
            ("file cap", "true", code * 3000, pending),
            ("lookup failure", "true", None, pending),
        ):
            with self.subTest(label):
                statuses = execute_invalidator(
                    event, PULL, env={"CI_MERGE_QUEUE": mode}, files=files
                )
                observed = [
                    (
                        s["sha"],
                        s["state"],
                        s["context"],
                        s.get("target_url"),
                        s["description"],
                    )
                    for s in statuses
                ]
                self.assertEqual(observed, [expected])
        # A carried Full result still wins, so the head keeps a tested tree marker.
        statuses = execute_invalidator(
            event,
            PULL,
            env={"CI_MERGE_QUEUE": "true"},
            files=code,
            payload=SYNCHRONIZE,
            previous=validation(tree=BASE),
            trees={BASE: INVENTORY, TREE: tree(CODE)},
        )
        self.assertEqual(
            [s["context"] for s in statuses], [ci.CONTEXT, "Validated tree"]
        )
        # A request started after the push keeps its own status.
        latest = {"context": ci.CONTEXT, "updated_at": "2026-10-07T12:00:01Z"}
        self.assertEqual(
            execute_invalidator(
                event, PULL, latest, env={"CI_MERGE_QUEUE": "true"}, files=code
            ),
            [],
        )


class CacheTests(unittest.TestCase):
    def prune(self, caches: object) -> dict[str, Any]:
        script = script_for("ci-cache.yml", "prune")
        return cast(
            dict[str, Any],
            node(f"""
const deleted = [], failures = [], context = {{repo: {{}}}};
const core = {{info: () => {{}}, setFailed: value => failures.push(value)}};
const github = {{paginate: async () => {json.dumps(caches)}, rest: {{actions: {{
  getActionsCacheList: () => {{}}, deleteActionsCacheById: async value => deleted.push(value.cache_id)
}}}}}};
(async () => {{{script}}})().then(() => console.log(JSON.stringify({{deleted, failures}})));
"""),
        )

    def test_actual_cache_controller_keeps_one_entry_per_prefix_and_eight_gib(
        self,
    ) -> None:
        gib = 1024**3
        prefixes = "compiler-v1-linux compiler-v1-linux ccache-linux-v2-gcc"
        prefixes += " ccache-macos-v2-arm64 ccache-msvc-v1-x64"
        prefixes += " ccache-emscripten-v1-4.0.23 ccache-emscripten-v1-4.0.23"
        caches = [
            {"id": i, "key": f"{prefix}-{str(i) * 40}", "created_at": f"2026-10-{i:02}"}
            | {"size_in_bytes": gib}
            for i, prefix in enumerate(prefixes.split(), start=1)
        ]
        self.assertEqual(self.prune(caches), {"deleted": [1, 6], "failures": []})
        caches[-1]["size_in_bytes"] = 5 * gib
        self.assertEqual(self.prune(caches), {"deleted": [1, 2, 3, 6], "failures": []})
        # Unmanaged caches are never removed.
        unmanaged = {"id": 99, "key": "unmanaged", "created_at": "date"}
        result = self.prune([unmanaged | {"size_in_bytes": 9 * gib}])
        self.assertEqual(result["deleted"], [])
        self.assertTrue(result["failures"])

    def test_pruning_follows_only_protected_push_runs(self) -> None:
        predicate = job("ci-cache.yml", "prune")["if"]
        for event, branch, expected in (
            ("push", "rust-migration", True),
            ("pull_request", "rust-migration", False),
            ("push", "feature", False),
        ):
            github = {
                "event": {"workflow_run": {"event": event, "head_branch": branch}}
            }
            self.assertEqual(evaluate(predicate, github=github), expected)


class PolicyTests(unittest.TestCase):
    def test_platform_matrix_matches_profile_and_cadence(self) -> None:
        jobs = workflow("ci-build.yml")["jobs"]
        # A direct dispatch is the cadence; controller dispatches carry a checkout.
        dispatch = "workflow_dispatch"
        all_linux, both_windows = ["clang", "sdl2", "dedicated"], ["x86", "x64"]
        for profile, checkout, event, heavy, covered, linux, windows, macos in (
            ("full", "merge", dispatch, "", "", all_linux, both_windows, False),
            ("native", "merge", dispatch, "", "", ["sdl2"], [], False),
            ("platform", "merge", dispatch, "", "", all_linux, both_windows, True),
            ("", "", dispatch, "", "", ["clang", "dedicated"], both_windows, True),
            # A covered push refreshes every cached flavour Full restores.
            (
                "",
                "",
                "push",
                "false",
                "true",
                ["sdl2", "dedicated"],
                both_windows,
                False,
            ),
            ("", "", "push", "true", "", all_linux, both_windows, True),
            ("", "", "pull_request", "true", "", ["sdl2"], ["x64"], True),
        ):
            contexts: dict[str, Any] = {
                "inputs": {"checkout-ref": checkout, "profile": profile},
                "github": {"event_name": event},
                "needs": {"policy": {"outputs": {"heavy": heavy, "covered": covered}}},
            }
            with self.subTest(profile=profile, event=event, covered=covered):
                for name, axis, expected in (
                    ("linux", "flavour", linux),
                    ("windows", "arch", windows),
                ):
                    body = jobs[name]
                    selected = (
                        evaluate(body["strategy"]["matrix"][axis], **contexts)
                        if evaluate(body["if"], **contexts)
                        else []
                    )
                    self.assertEqual(selected, expected)
                self.assertEqual(evaluate(jobs["macos"]["if"], **contexts), macos)
                self.assertEqual(
                    evaluate(jobs["emscripten"]["if"], **contexts),
                    # A covered push refreshes the Emscripten compiler cache.
                    bool(
                        checkout
                        and profile != "native"
                        or heavy == "true"
                        or covered == "true"
                    ),
                )

    def test_quick_uses_no_local_action(self) -> None:
        """On a pull request Quick checks out the PR head, and a branch older
        than a local action does not contain it (#862 broke Quick that way)."""
        steps = workflow("ci-quick.yml")["jobs"]["quick"]["steps"]
        self.assertTrue(steps)
        for step in steps:
            self.assertFalse(str(step.get("uses", "")).startswith("./"), step)
        # Its package install is the shared action's script, copied, so the two
        # cannot drift: the copy once lost the action's index and dpkg handling.
        (shared,) = yaml.safe_load(
            (ROOT / ".github/actions/apt-install/action.yml").read_text()
        )["runs"]["steps"]
        (inline,) = (step for step in steps if "PACKAGES" in step.get("env", {}))
        self.assertEqual(inline["run"], shared["run"])
        self.assertEqual(set(inline["env"]), set(shared["env"]))

    def test_semantic_subset_requires_the_protected_cadence(self) -> None:
        jobs = workflow("ci-build.yml")["jobs"]
        for event, checkout, branch, expected in (
            ("workflow_dispatch", "", "rust-migration", True),
            ("workflow_dispatch", "merge", "rust-migration", False),
            ("workflow_dispatch", "", "feature", False),
            ("pull_request", "", "feature", False),
            ("push", "", "rust-migration", False),
        ):
            self.assertEqual(
                evaluate(
                    jobs["platform-reference"]["if"],
                    inputs={"checkout-ref": checkout},
                    github={"event_name": event, "ref": "refs/heads/" + branch},
                ),
                expected,
            )
        for name, matrix, selected in (
            ("windows", {"arch": "x64"}, True),
            ("windows", {"arch": "x86"}, False),
            ("macos", {"name": "arm64 - Release"}, True),
            ("macos", {"name": "arm64 - Debug"}, False),
        ):
            for result in ("success", "failure", "skipped"):
                self.assertEqual(
                    evaluate(
                        jobs[name]["with"]["semantic-subset"],
                        matrix=matrix,
                        needs={"platform-reference": {"result": result}},
                    ),
                    selected and result == "success",
                )

    def test_platform_cache_refresh_is_only_a_covered_protected_push(self) -> None:
        build = workflow("ci-build.yml")["jobs"]
        protected = "refs/heads/rust-migration"
        for name in ("emscripten", "linux", "windows"):
            # A covered push runs the cached builds only to refresh their caches.
            outputs = {"heavy": "false", "covered": "true"}
            runs_covered = evaluate(
                build[name]["if"],
                inputs={"checkout-ref": "", "profile": ""},
                needs={"policy": {"result": "success", "outputs": outputs}},
                github={"event_name": "push"},
            )
            self.assertTrue(runs_covered, name)
            for checkout, event, ref, covered, expected in (
                ("", "push", protected, "true", True),
                ("merge", "push", protected, "true", False),
                ("", "pull_request", protected, "true", False),
                ("", "push", "refs/heads/master", "true", False),
                ("", "push", protected, "false", False),
            ):
                result = evaluate(
                    build[name]["with"]["cache-only"],
                    inputs={"checkout-ref": checkout},
                    github={"event_name": event, "ref": ref},
                    needs={"policy": {"outputs": {"covered": covered}}},
                )
                self.assertEqual(result, expected)

    def test_cache_refresh_skips_exactly_the_test_and_evidence_steps(self) -> None:
        windows = {
            "Rust checks for actual selected target",
            "Test",
            "Fresh Rust native generators",
            "Validate refusals and actual archive freshness",
            "Retain native Windows Rust and test evidence",
        }
        for filename, name, skipped in (
            ("ci-emscripten.yml", "emscripten", {"Test Rust ABI"}),
            ("ci-linux.yml", "linux", {"Test"}),
            ("ci-windows.yml", "windows", windows),
        ):
            steps = job(filename, name)["steps"]
            outputs = {step["id"]: {"outputs": {}} for step in steps if "id" in step}
            observed = {
                step.get("name")
                for step in steps
                if runs(step, False, outputs) and not runs(step, True, outputs)
            }
            self.assertEqual(observed, skipped, filename)
        linux = job("ci-linux.yml", "linux")["if"]
        for cache_only, compiler, expected in (
            (True, "clang", False),
            (True, "gcc", True),
            (False, "clang", True),
        ):
            inputs = {"cache-only": cache_only, "compiler": compiler}
            self.assertEqual(evaluate(linux, inputs=inputs), expected)

    def test_heavy_predicates_bypass_skipped_policy_but_respect_cancellation(
        self,
    ) -> None:
        dispatch = "workflow_dispatch"
        for filename, names in (
            ("ci-build.yml", ("emscripten", "linux", "macos", "windows")),
            ("rust-migration.yml", ("build", "comparisons", "simulation")),
        ):
            jobs = workflow(filename)["jobs"]
            for name in names:
                cases = [
                    (False, "head", "", "full", "push", True),
                    (False, "", "true", "full", "push", True),
                    (False, "", "false", "full", "push", False),
                    (True, "head", "true", "full", "push", False),
                    (True, "", "true", "full", "push", False),
                ]
                if filename == "ci-build.yml" and name in ("linux", "windows"):
                    cases += [
                        (False, "", "", "", dispatch, True),
                        (True, "", "", "", dispatch, False),
                    ]
                if name == "macos":
                    # Full skips macOS; platform, heavy pushes and dispatch run it.
                    cases = [
                        (False, "head", "", "full", dispatch, False),
                        (False, "head", "", "platform", dispatch, True),
                        (False, "", "true", "", "push", True),
                        (False, "", "false", "", "push", False),
                        (False, "", "", "", dispatch, True),
                        (True, "", "", "", dispatch, False),
                    ]
                for cancelled, checkout, heavy, profile, event, expected in cases:
                    needs = {
                        "policy": {
                            "result": "skipped" if checkout else "success",
                            "outputs": {"heavy": heavy},
                        }
                    }
                    needs["build"] = {"result": "success"}
                    with self.subTest(filename=filename, job=name, cancelled=cancelled):
                        result = evaluate(
                            jobs[name]["if"],
                            cancelled=cancelled,
                            inputs={"checkout-ref": checkout, "profile": profile},
                            needs=needs,
                            github={"event_name": event},
                        )
                        self.assertEqual(result, expected)

    def test_protected_push_groups(self) -> None:
        def groups(filename: str, event: str = "push") -> list[object]:
            github = {"workflow": filename, "event_name": event, "run_id": 1}
            github["ref"] = "refs/heads/rust-migration"
            group = workflow(filename)["concurrency"]["group"]
            return [
                evaluate(
                    group, interpolate=True, inputs={}, github=github | {"sha": sha}
                )
                for sha in ("code-merge", "later-merge")
            ]

        # Heavy post-merge validation coalesces queued pushes to the newest tip.
        for filename in ("ci-build.yml", "rust-migration.yml"):
            first, second = groups(filename)
            self.assertEqual(first, second)
        first, second = groups("ci-quick.yml")
        self.assertNotEqual(first, second)
        # A cadence dispatch never displaces a queued push validation.
        dispatched = groups("ci-build.yml", "workflow_dispatch")
        self.assertNotEqual(groups("ci-build.yml"), dispatched)

    def test_platform_annotations_run_after_each_platform_and_gate_completion(
        self,
    ) -> None:
        jobs = workflow("ci-build.yml")["jobs"]
        for platform in ("emscripten", "linux", "macos", "windows"):
            annotations = jobs[f"annotations_{platform}"]
            self.assertEqual(annotations["needs"], platform)
            for result, event in itertools.product(
                ("success", "failure", "skipped"),
                ("pull_request", "workflow_dispatch", "push"),
            ):
                needs = {platform: {"result": result}}
                ran = evaluate(
                    annotations["if"],
                    needs=needs,
                    inputs={"checkout-ref": ""},
                    github={"event_name": event},
                )
                self.assertEqual(ran, result != "skipped" and event != "push")
        self.assertTrue(
            evaluate(
                jobs["check_annotations"]["if"],
                inputs={"checkout-ref": ""},
                github={"event_name": "workflow_dispatch"},
                needs={"linux": {"result": "success"}},
            )
        )
        run = jobs["complete"]["steps"][0]["run"]
        script = run.split("<<'PY'\n", 1)[1].rsplit("PY\n", 1)[0]
        for profile, platforms in (
            ("full", ["linux", "emscripten", "windows"]),
            ("platform", ["linux", "emscripten", "windows", "macos"]),
            ("native", ["linux"]),
        ):
            required = [
                "check_annotations",
                *platforms,
                *[f"annotations_{name}" for name in platforms],
            ]
            cases = [
                (None, "success"),
                *itertools.product(
                    required, ("failure", "skipped", "cancelled", "missing")
                ),
            ]
            for failed, result in cases:
                needs = {name: {"result": "success"} for name in required}
                if failed:
                    if result == "missing":
                        del needs[failed]
                    else:
                        needs[failed]["result"] = result
                with (
                    self.subTest(profile=profile, failed=failed, result=result),
                    tempfile.TemporaryDirectory() as directory,
                ):
                    output = Path(directory) / "output"
                    env = {
                        **os.environ,
                        "PROFILE": profile,
                        "RESULTS": json.dumps(needs),
                    }
                    env["GITHUB_OUTPUT"] = str(output)
                    completed = subprocess.run(
                        ["python3", "-c", script], capture_output=True, env=env
                    )
                    self.assertEqual(completed.returncode == 0, failed is None)
                    self.assertEqual(output.exists(), failed is None)

    def test_coverage_lookup_retries_transients_and_falls_back_only_after_exhaustion(
        self,
    ) -> None:
        source = tree(entry("src/main.cpp"))
        for failure, recover, expected_calls, expected in (
            ({"status": 500}, True, 2, {"heavy": "false"}),
            ({"status": 503}, False, 3, {"heavy": "true"}),
            ({"status": 429}, False, 3, {"heavy": "true"}),
            ({"code": "ECONNRESET"}, False, 3, {"heavy": "true"}),
            *[({"status": code}, False, 1, {}) for code in (400, 401, 403, 404, 422)],
            ({}, False, 1, {}),
        ):
            error = f"Object.assign(new Error('API failed'), {json.dumps(failure)})"
            compare = f"""async () => {{
              if (++calls === 1 || !{json.dumps(recover)}) throw {error};
              return {{data: {{files: [{{filename: 'docs/readme.md'}}]}}}};
            }}"""
            with self.subTest(failure=failure, recover=recover):
                result = run_policy(trees={BASE: source, TREE: source}, compare=compare)
                self.assertEqual(result["calls"], expected_calls)
                self.assertEqual(result["outputs"], expected)
                self.assertEqual(result["delays"], [1000, 2000][: expected_calls - 1])
                self.assertEqual("error" in result, not expected)

    def test_bootstrap_on_demand_and_docs_classification(self) -> None:
        readme = tree(entry("README.md"))
        renamed = {"filename": "README.md", "previous_filename": "tools/tool.py"}
        for on_demand, event, files, expected in (
            ("", "pull_request", [], "true"),
            ("false", "pull_request", [], "true"),
            ("true", "pull_request", [], "false"),
            ("true", "push", [{"filename": "docs/roadmap.md"}], "false"),
            ("true", "push", [{"filename": "tools/ci.py"}], "true"),
            ("true", "push", [renamed], "true"),
            ("true", "push", [{"filename": "docs/new.md"}] * 300, "true"),
            ("true", "push", [{"filename": "README.md"}], "true"),
            ("true", "push", [{"filename": "docs/multiplayer.md"}], "true"),
            ("true", "push", [{"filename": "AGENTS.md"}], "false"),
            ("true", "push", [], "true"),
            ("true", "push", None, "true"),
        ):
            count = None if files is None else len(files)
            with self.subTest(on_demand=on_demand, event=event, files=count):
                trees = {BASE: readme, TREE: readme}
                result = run_policy(
                    event=event, on_demand=on_demand, files=files, trees=trees
                )
                self.assertEqual(result["outputs"]["heavy"], expected)

    def test_docs_fast_path_checks_tree_types_and_complete_inventories(self) -> None:
        code = entry("src/game.cpp")
        link = entry("docs/module.md", mode="160000", kind="commit")
        doc = entry("docs/module.md")
        files = [{"filename": "docs/module.md"}]
        renamed = [{"filename": "docs/new.md", "previous_filename": "docs/module.md"}]
        docs_before, docs_after = tree(code, doc), tree(code, dict(doc, sha=MERGE))
        cases = [
            (tree(code, link), tree(code, dict(link, sha=MERGE)), files, "true"),
            (tree(code), tree(code, dict(link, sha=MERGE)), files, "true"),
            (tree(code, link), tree(code), files, "true"),
            (
                tree(code, link),
                tree(code, dict(link, path="docs/new.md")),
                renamed,
                "true",
            ),
            (docs_before, docs_after, files, "false"),
        ]
        for bad in (dict(docs_before, truncated=True), dict(docs_before, tree=[{}])):
            cases += [
                (bad, docs_after, files, "true"),
                (docs_before, bad, files, "true"),
            ]
        for left, right, changed, expected in cases:
            with self.subTest(left=left, right=right, files=changed):
                result = run_policy(files=changed, trees={BASE: left, TREE: right})
                self.assertEqual(result["outputs"]["heavy"], expected)

    def test_only_matching_successful_merged_tree_skips_validation(self) -> None:
        bad_runs = (None, "", "run", RUN + "/extra", RUN.replace("fork", "other"))
        for full, marker, same_run, tree_sha, merged, run, expected in (
            ("success", "success", True, TREE, True, RUN, "false"),
            ("success", "success", True, "other", True, RUN, "true"),
            ("pending", "success", True, TREE, True, RUN, "true"),
            ("failure", "success", True, TREE, True, RUN, "true"),
            ("success", "failure", True, TREE, True, RUN, "true"),
            ("success", "success", False, TREE, True, RUN, "true"),
            ("success", "success", True, TREE, False, RUN, "true"),
            *(("success", "success", True, TREE, True, r, "true") for r in bad_runs),
        ):
            statuses = validation(
                full, marker, run, run if same_run else "old", tree_sha
            )
            pull = dict(PULL, merged_at="date" if merged else None)
            files = [{"filename": "src/code.cpp"}]
            with self.subTest(full=full, marker=marker, run=run, merged=merged):
                result = run_policy(files=files, pulls=[pull], statuses=statuses)
                self.assertEqual(result["outputs"]["heavy"], expected)
                # A merge-queue merge pushes the tested group commit itself, which
                # needs no merged PR.
                if merged:
                    result = run_policy(files=files, pushed=statuses)
                    self.assertEqual(result["outputs"]["heavy"], expected)

    def test_coverage_trusts_only_the_publishers_statuses(self) -> None:
        files = [{"filename": "src/code.cpp"}]
        merged = [dict(PULL, merged_at="date")]
        trusted = validation()
        for index in (0, 1):
            for creator in (None, {}, {"login": "writer", "type": "User"}):
                forged = [dict(status) for status in trusted]
                forged[index]["creator"] = creator
                with self.subTest(index=index, creator=creator):
                    for result in (
                        run_policy(files=files, pushed=forged),
                        run_policy(files=files, pulls=merged, statuses=forged),
                    ):
                        self.assertEqual(result["outputs"]["heavy"], "true")
                    # A newer untrusted status hides an older trusted one.
                    result = run_policy(files=files, pushed=[forged[index], *trusted])
                    self.assertEqual(result["outputs"]["heavy"], "true")
        # An untested pushed commit still finds the merged PR head's result.
        result = run_policy(
            files=files, pushed=validation("failure"), pulls=merged, statuses=trusted
        )
        self.assertEqual(result["outputs"], {"heavy": "false", "covered": "true"})

    def test_markdown_tree_differences_require_complete_inventories(self) -> None:
        for after, expected in inventory_cases():
            for incomplete_side in (False, True):
                left, right = (
                    (after, INVENTORY) if incomplete_side else (INVENTORY, after)
                )
                with self.subTest(after=after, incomplete_side=incomplete_side):
                    outputs = run_policy(
                        files=[{"filename": "src/game.cpp"}],
                        trees={BASE: left, TREE: right},
                        pulls=[dict(PULL, merged_at="date")],
                        statuses=validation(tree=BASE),
                    )["outputs"]
                    covered = "true" if expected == "false" else None
                    self.assertEqual(outputs["heavy"], expected)
                    self.assertEqual(outputs.get("covered"), covered)


def pull_and_statuses(
    statuses: object,
) -> Callable[[str, str], object]:
    """Answer `ci.api` with the PR for pull paths and `statuses` for commit paths."""
    return lambda _repository, path: (
        PULL if path.startswith("pulls/") else {"statuses": statuses}
    )


class RequestTests(unittest.TestCase):
    def test_each_profile_dispatches_captured_head_on_protected_ref(self) -> None:
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
                    patch.object(ci, "api", side_effect=pull_and_statuses([])),
                    patch.object(ci, "gh", return_value=json.dumps([[]])) as gh,
                ):
                    request_path = ci.request(args)
                invocation = gh.call_args.args
                self.assertIn(f"head={HEAD}", invocation)
                self.assertIn(f"profile={profile}", invocation)
                self.assertEqual(
                    invocation[invocation.index("--ref") + 1], "rust-migration"
                )
                record = json.loads(cast(Path, request_path).read_text())
                self.assertEqual(record["head"], HEAD)
                self.assertEqual(record["base"], BASE)
                self.assertEqual(len(record["request"]), 32)

    def test_full_request_stops_on_a_head_that_already_passed(self) -> None:
        passed = {
            "context": ci.CONTEXT,
            "state": "success",
            "target_url": RUN,
            "creator": ACTIONS,
        }
        for profile, statuses, bootstrap, dispatched in (
            ("full", [passed], False, False),
            ("full", [dict(passed, state="pending")], False, True),
            ("full", [dict(passed, state="failure")], False, True),
            ("full", [dict(passed, context="Validated tree")], False, True),
            ("full", [dict(passed, creator=None)], False, True),
            (
                "full",
                [dict(passed, creator={"login": "writer", "type": "User"})],
                False,
                True,
            ),
            ("full", [dict(passed, creator=None), passed], False, True),
            # The merge-queue status on a PR head is no Full of that head.
            ("full", [dict(passed, description=ci.QUEUED)], False, True),
            ("full", [passed], True, True),
            ("native", [passed], False, True),
        ):
            with (
                self.subTest(profile=profile, statuses=statuses, bootstrap=bootstrap),
                tempfile.TemporaryDirectory() as directory,
            ):
                args = argparse.Namespace(
                    repo="example/fork",
                    pr=170,
                    profile=profile,
                    bootstrap=bootstrap,
                    request_file=Path(directory) / "request.json",
                )
                with (
                    patch.object(ci, "api", side_effect=pull_and_statuses(statuses)),
                    patch.object(
                        ci,
                        "gh",
                        side_effect=lambda *args, statuses=statuses: json.dumps(
                            [] if args[0] == "variable" else [statuses]
                        ),
                    ) as gh,
                    patch("builtins.print"),
                ):
                    path = ci.request(args)
                writes = [call for call in gh.call_args_list if call.args[0] != "api"]
                self.assertEqual(bool(writes), dispatched)
                self.assertEqual(path is not None, dispatched)

    def test_run_discovery_uses_unique_request_identity(self) -> None:
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

    def test_push_invalidates_record_head(self) -> None:
        with self.assertRaisesRegex(ValueError, "head changed"):
            ci.validate_pull(PULL, "c" * 40)

    def test_base_advance_during_run_keeps_waiting_on_unchanged_head(self) -> None:
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

    def test_status_and_wait_report_failed_child_while_other_jobs_continue(
        self,
    ) -> None:
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
            run = {
                "id": 123,
                "status": "in_progress",
                "conclusion": None,
                "html_url": RUN,
            }
            failed = {
                "name": "Linux / CI",
                "conclusion": "failure",
                "html_url": RUN + "/job/1",
            }
            pages = [
                {"jobs": [{"conclusion": "success"}, {"conclusion": None}]},
                {"jobs": [failed]},
            ]
            with (
                patch.object(ci, "api", side_effect=[PULL, run]),
                patch.object(ci, "gh", return_value=json.dumps(pages)) as gh,
            ):
                _, observed = cast(tuple[dict, dict[str, Any]], ci.status(path))
                self.assertIn("--paginate", gh.call_args.args)
                self.assertIn("filter=latest", gh.call_args.args[-1])
            self.assertEqual(observed["failed_jobs"], [failed])
            self.assertEqual(observed["status"], "in_progress")
            with (
                patch.object(ci, "status", return_value=(record, observed)),
                patch.object(ci, "verify_completion") as complete,
                patch.object(ci.time, "sleep") as sleep,
            ):
                with self.assertRaisesRegex(
                    ValueError, "Failed child jobs: Linux / CI"
                ):
                    ci.wait(path, 60, 15)
                complete.assert_not_called()
                sleep.assert_not_called()
            with (
                patch.object(ci, "status", return_value=(record, observed)),
                patch("sys.argv", ["ci.py", "status", str(path)]),
                patch("builtins.print"),
            ):
                self.assertEqual(ci.main(), 1)
            ci.verify_children({"failed_jobs": []})

    def test_wait_returns_within_shell_cap_and_names_resume_command(self) -> None:
        running = {"status": "in_progress", "html_url": RUN, "failed_jobs": []}
        record = {"profile": "full", "head": HEAD}
        clock = itertools.count(0, 100)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "request.json"
            with (
                patch.object(ci, "status", return_value=(record, running)),
                patch.object(ci.time, "monotonic", side_effect=lambda: next(clock)),
                patch.object(ci.time, "sleep") as sleep,
                patch("builtins.print"),
            ):
                with self.assertRaisesRegex(
                    ci.WaitTimeout, f"resume with: python3 tools/ci.py wait {path}$"
                ):
                    ci.wait(path, 540, 15)
            self.assertTrue(sleep.called)
            for argv in (
                ["ci.py", "wait", str(path)],
                ["ci.py", "request", "170", "--wait"],
            ):
                with (
                    patch("sys.argv", argv),
                    patch.object(ci, "request", return_value=path),
                    patch.object(
                        ci, "wait", side_effect=ci.WaitTimeout("resume")
                    ) as waited,
                    patch("builtins.print") as printed,
                ):
                    self.assertEqual(ci.main(), 2)
                self.assertLess(waited.call_args.args[1], 600)
                printed.assert_called_once()

    def test_successful_run_without_full_status_is_rejected(self) -> None:
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
