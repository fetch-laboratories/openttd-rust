#!/usr/bin/env python3
"""Exercise request identity and the actual privileged full-result publisher."""

import argparse
import io
import itertools
import json
import os
import re
import shutil
import subprocess
import tempfile
import threading
import unittest
from collections.abc import Callable, Iterable, Mapping
from dataclasses import dataclass, field, replace
from datetime import datetime, timezone
from functools import partial
from pathlib import Path
from typing import Any, TypedDict, cast
from unittest.mock import patch
from urllib.parse import unquote

import ci
import ci_log
import preflight_stamp
import quick_workflow
import review_report
import review_start
import source_fixture
import status_comment
import vcs
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
    "body": "Agent: /implementation | Model: m | Reasoning effort: high",
    "created_at": "2026-11-11T00:00:00Z",
}


# Paths the docs-only rule must judge so, beside every tracked path near them.
DOCS_ONLY_CASES = {
    "AGENTS.md": True,
    "docs/roadmap.md": True,
    "docs/skills/x/SKILL.md": True,
    "docs/skills/x/agents/openai.yaml": True,
    "docs/elrail.svg": True,
    "docs/landscape.html": True,
    "docs/x/data.json": True,
    "docs/fonts.md.txt": True,
    ".github/PULL_REQUEST_TEMPLATE.md": True,
    ".github/ISSUE_TEMPLATE/bug.yaml": True,
    ".codex/config.toml": True,
    "docs/fonts.md": False,
    "docs/obg_format.txt": False,
    "docs/openttd.6": False,
    "docs/skills/steer/issues.py": False,
    "docs/skills/x/run.sh": False,
    "docs.md": False,
    "docsx/a.md": False,
    "README.md": False,
    "tools/ci.py": False,
    "migration/baseline.json": False,
    ".github/workflows/ci-quick.yml": False,
    ".github/dependabot.yml": False,
    ".github/PULL_REQUEST_TEMPLATE.md.py": False,
    ".codexx/config.toml": False,
    "docs/elrail_tile.png": False,
    "docs/x/caf\u00e9.yaml": True,
}


def near_docs_only_paths() -> set[str]:
    """Tracked root-level files and every tracked file under the docs-only trees."""
    tracked = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT, text=True)
    return {
        path
        for path in tracked.split("\0")
        if path
        and ("/" not in path or path.startswith(("docs/", ".github/", ".codex/")))
    }


def starts_workflow(patterns: list[str], path: str) -> bool:
    """GitHub's `paths` filter for one path: the last pattern that matches decides."""
    started = False
    for pattern in patterns:
        glob = "".join(
            ".*" if part == "**" else "[^/]*" if part == "*" else re.escape(part)
            for part in re.split(r"(\*\*|\*)", pattern.removeprefix("!"))
        )
        if re.fullmatch(glob, path):
            started = not pattern.startswith("!")
    return started


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


WORLD = {
    "event_name",
    "latest",
    "commits",
    "runs",
    "progress",
    "statuses",
    "files",
    "tick",
}


def split_world(options: Mapping[str, Any]) -> dict[str, Any]:
    """Pass `options` to execute_script, moving its WORLD keys into `world`."""
    rest = {key: value for key, value in options.items() if key not in WORLD}
    world = {key: value for key, value in options.items() if key in WORLD}
    return {**rest, "world": {**rest.get("world", {}), **world}}


def execute_script(
    filename: str,
    name: str,
    pull: object,
    *,
    env: Mapping[str, str] | None = None,
    event: object = None,
    payload: Mapping[str, object] | None = None,
    previous: Iterable[object] = (),
    trees: object = None,
    compare: str | None = None,
    world: Mapping[str, Any] | None = None,
) -> dict[str, Any]:
    """Run a status writer; `previous` holds the statuses of the PREVIOUS head.

    `world` holds the rest of GitHub's answers, each optional (see WORLD):
    `event_name`; `latest`, the combined Full validation status of other refs;
    `commits`, getCommit by ref; `runs`, the workflow run listing; `progress`,
    the successive getWorkflowRun answers by run ID; `statuses`, the listed
    statuses of other refs; `files`, the PR file list (None fails the lookup);
    and `tick`, the milliseconds each Date.now() call advances the clock.
    """
    world = dict(world or {})
    if unknown := set(world) - WORLD:
        raise TypeError(f"Unknown world keys: {sorted(unknown)}")
    event_name = world.get("event_name", "workflow_dispatch")
    latest, commits = world.get("latest"), world.get("commits")
    runs = world.get("runs", "async () => ({data: {workflow_runs: []}})")
    progress, statuses = world.get("progress"), world.get("statuses")
    files, tick = world.get("files"), world.get("tick", 0)
    script = script_for(filename, name)
    payload = {"pull_request": event, **(payload or {})}
    compare = compare or "async () => ({data: {behind_by: 0}})"
    listed = {PREVIOUS: list(previous)}
    listed |= {ref: list(rows) for ref, rows in (statuses or {}).items()}
    tested = {"parents": [{"sha": PARENT}], "commit": {"tree": {"sha": TREE}}}
    return cast(
        dict[str, Any],
        node(f"""
const statuses = [], failures = [], outputs = {{}}, delays = [], cancels = [], trees = {json.dumps(trees or {})};
const commits = {json.dumps(commits or {})}, progress = {json.dumps(progress or {})};
let now = 0;
Date.now = () => (now += {tick});
const setTimeout = (resolve, delay) => {{ delays.push(delay); resolve(); }};
const context = {{repo: {{owner: 'example', repo: 'fork'}}, actor: 'root', eventName: {json.dumps(event_name)},
  serverUrl: 'https://github.com', runId: 123, payload: {json.dumps(payload)}}};
const core = {{setFailed: value => failures.push(value), setOutput: (k,v) => outputs[k] = v,
  info: () => {{}}, notice: () => {{}}}};
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
  actions: {{listWorkflowRuns: {runs}, cancelWorkflowRun: async args => cancels.push(args.run_id),
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
(async () => {{{script}}})().then(() => console.log(JSON.stringify({{statuses, failures, outputs, delays, cancels}})))
  .catch(error => console.log(JSON.stringify({{error: error.message, statuses, delays}})));
"""),
    )


def execute_publisher(
    needs: object,
    pull: object,
    superseded: bool = False,
    queue: bool = False,
    latest: object = True,
) -> dict[str, Any]:
    """Publish with `latest` the newest Full validation status on the target:
    by default the plan's pending status, or a newer request's if superseded."""
    if latest is True:
        latest = pending(RUN.replace("123", "999") if superseded else RUN)
    target = MERGE if queue else HEAD
    return execute_script(
        "ci-request.yml",
        "publish",
        pull,
        env={"RESULTS": json.dumps(needs)},
        **split_world(
            {
                "statuses": {target: [latest] if latest else []},
                **(QUEUE if queue else {}),
            }
        ),
    )


def pending(url: str = RUN, creator: object = ACTIONS) -> dict[str, object]:
    """A Full validation status a request's plan wrote."""
    return {"context": ci.CONTEXT, "target_url": url, "creator": creator}


def execute_invalidator(
    event: object, pull: object, latest: object = None, **carry: object
) -> list[dict[str, Any]]:
    return execute_script(
        "ci-invalidate.yml",
        "invalidate",
        pull,
        event=event,
        **split_world({"latest": latest, **carry}),
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


PUSHED = "head"  # The commit a protected push moved the base to.
ON_DEMAND = {"CI_ON_DEMAND": "true"}
QUEUE_MODE = {**ON_DEMAND, "CI_MERGE_QUEUE": "true"}


def run_policy(
    *,
    event: str = "push",
    variables: Mapping[str, str] = ON_DEMAND,
    files: object = (),
    trees: object = None,
    pulls: Iterable[object] = (),
    statuses: Mapping[str, Iterable[object]] | None = None,
    compare: str | None = None,
) -> dict[str, Any]:
    """Run the actual policy script; `trees` maps the base and pushed tree SHAs.

    `variables` are the repository variables; `statuses` maps HEAD, the merged
    PR's head, and PUSHED, the pushed commit, to their statuses.
    """
    listed = {HEAD: [], PUSHED: []}
    listed |= {ref: list(rows) for ref, rows in (statuses or {}).items()}
    script = script_for("ci-policy.yml", "policy")
    compare = compare or f"async () => ({{data: {{files: {json.dumps(files)}}}}})"
    return cast(
        dict[str, Any],
        node(f"""
Object.assign(process.env, {{CI_ON_DEMAND: '', CI_MERGE_QUEUE: ''}}, {json.dumps(dict(variables))});
const outputs = {{}}, delays = [], trees = {json.dumps(trees or {})};
let calls = 0;
const setTimeout = (resolve, delay) => {{ delays.push(delay); resolve(); }};
const context = {{eventName: {json.dumps(event)}, repo: {{owner: 'example', repo: 'fork'}},
  serverUrl: 'https://github.com', sha: {json.dumps(PUSHED)},
  payload: {{before: 'base', pull_request: {{base: {{ref: 'rust-migration'}}}}}}}};
const core = {{setOutput: (k, v) => outputs[k] = v, info: () => {{}}, warning: () => {{}}}};
const listed = {json.dumps(listed)};
const github = {{paginate: async (method, args) =>
  method === github.rest.repos.listPullRequestsAssociatedWithCommit ? {json.dumps(list(pulls))} : listed[args.ref],
  rest: {{
  git: {{getTree: async args => ({{data: {{sha: args.tree_sha, ...trees[args.tree_sha]}}}})}},
  repos: {{
    getCommit: async args => ({{data: {{commit: {{tree: {{sha: args.ref === 'base' ? '{BASE}' : '{TREE}'}}}}}}}}),
    listPullRequestsAssociatedWithCommit: () => {{}}, listCommitStatusesForRef: () => {{}},
    getCombinedStatusForRef: async args => ({{data: {{statuses: listed[args.ref].map(({{creator, ...status}}) => status)}}}}),
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


def runs(
    step: dict[str, Any],
    cache_only: bool,
    steps: dict[str, Any],
) -> object:
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

    def test_superseded_request_ends_cancelled_not_failed(self) -> None:
        superseded = job("ci-request.yml", "superseded")
        self.assertIn("github.event_name != 'merge_group'", superseded["if"])
        self.assertEqual(superseded["permissions"]["actions"], "write")
        env = {"PR": "170", "HEAD": HEAD}
        for label, pull, cancelled in (
            ("current head", PULL, []),
            ("moved head", dict(PULL, head={"sha": PREVIOUS}), [123]),
            ("closed", dict(PULL, state="closed"), [123]),
        ):
            with self.subTest(label):
                result = execute_script("ci-request.yml", "superseded", pull, env=env)
                self.assertEqual(result["cancels"], cancelled)

    def test_requested_plan_bases_gates_on_the_merges_first_parent(self) -> None:
        # PULL's base SHA predates what merged into base since its last push; the
        # gates must count only the PR's own changes in the tested merge (#919).
        for profile in ("native", "full"):
            with self.subTest(profile=profile):
                env = {"PR": "170", "HEAD": HEAD, "PROFILE": profile}
                result = execute_script("ci-request.yml", "plan", PULL, env=env)
                self.assertEqual(result["outputs"]["base"], PARENT)
                self.assertNotEqual(BASE, PARENT)
        orphan = {MERGE: {"parents": [], "commit": {"tree": {"sha": TREE}}}}
        env = {"PR": "170", "HEAD": HEAD, "PROFILE": "native"}
        world = {"commits": orphan}
        result = execute_script("ci-request.yml", "plan", PULL, env=env, world=world)
        self.assertIn("no first parent", result["error"])

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

    def test_inert_files_exclude_every_build_input(self) -> None:
        sources = [ROOT / "CMakeLists.txt", *sorted((ROOT / "cmake").glob("*.cmake"))]
        tracked = set(
            subprocess.check_output(
                ["git", "ls-files", "-z"], cwd=ROOT, text=True
            ).split("\0")
        ) - {""}
        built = {
            match
            for path in sources
            for match in re.findall(
                r"\$\{CMAKE_SOURCE_DIR\}/([^\s\"@)]+)", path.read_text()
            )
        } & tracked
        for name in ("README.md", "docs/fonts.md", "docs/obg_format.txt"):
            self.assertIn(name, built)
        self.assertIn("docs/openttd.6", built)
        blocks = []
        for filename, name in (
            ("ci-request.yml", "plan"),
            ("ci-invalidate.yml", "invalidate"),
            ("ci-policy.yml", "policy"),
        ):
            script = script_for(filename, name)
            start = script.index("// Docs-only files, which no build")
            blocks.append(
                script[start : script.index(");", script.index("const inert")) + 2]
            )
        # Indentation follows each script's nesting; the code itself must match.
        self.assertEqual(len({" ".join(block.split()) for block in blocks}), 1)
        expected = DOCS_ONLY_CASES
        paths = sorted(built | near_docs_only_paths() | set(expected))
        verdicts = cast(
            dict[str, bool],
            node(
                blocks[0]
                + f"\nconsole.log(JSON.stringify(Object.fromEntries({json.dumps(paths)}.map(p => [p, inert(p)]))));"
            ),
        )
        for path in built:
            self.assertFalse(verdicts[path], path)
        for path, verdict in expected.items():
            self.assertEqual(verdicts[path], verdict, path)
        # The hook's rule and the workflows' rule are one rule.
        for path, verdict in verdicts.items():
            self.assertEqual(verdict, preflight_stamp.inert(path), path)

    def test_a_docs_only_push_starts_no_heavy_workflow(self) -> None:
        # A push that starts these would take the pending slot of their
        # branch-wide concurrency group from a code push, then skip its builds.
        filters = []
        for filename in ("ci-build.yml", "rust-migration.yml"):
            # YAML reads the key `on` as the boolean true.
            push = cast(dict[Any, Any], workflow(filename))[True]["push"]
            self.assertNotIn("paths-ignore", push)
            filters.append(push["paths"])
        self.assertEqual(filters[0], filters[1])
        # The filter is the rule: a path starts them unless it is docs-only.
        for path in sorted(near_docs_only_paths() | set(DOCS_ONLY_CASES)):
            self.assertEqual(
                starts_workflow(filters[0], path), not preflight_stamp.inert(path), path
            )
        # A push starts them when any one of its files does: Markdown that a
        # build packages beside a docs-only file still validates.
        mixed = ("docs/fonts.md", "docs/skills/x/agents/openai.yaml")
        self.assertEqual(
            [starts_workflow(filters[0], path) for path in mixed], [True, False]
        )
        built = [
            pattern.removeprefix("docs/")
            for pattern in filters[0]
            if pattern.startswith("docs/") and "*" not in pattern
        ]
        self.assertEqual(built, list(preflight_stamp.BUILT_DOCS))

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
                result = execute_publisher(
                    good_needs(), PULL, latest=pending(creator=creator)
                )
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
        "path": ".github/workflows/ci-request.yml",
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
        trees=trees if trees is not None else {TREE: tree(CODE), BASE: INVENTORY},
        **split_world(
            {
                "commits": {MERGE: GROUP, EARLIER: earlier, **(commits or {})},
                "statuses": statuses
                if statuses is not None
                else {
                    EARLIER: validation(
                        run=EARLIER_RUN, marker_run=EARLIER_RUN, tree=BASE
                    )
                },
                **QUEUE,
                **options,
            }
        ),
    )


ORIGIN = "e" * 40  # The group commit an origin run built, before EARLIER.


def chain(
    *, second: str = HEAD, built: str = PREVIOUS, **fields: object
) -> dict[str, Any]:
    """plan_queue options where EARLIER carried a result built by run 50 on ORIGIN.

    `fields` change run 50, `second` ORIGIN's PR head and `built` its tree; the
    carried marker names tree PREVIOUS.
    """
    origin = RUN.replace("123", str(fields.get("id", 50)))
    branch = f"gh-readonly-queue/rust-migration/pr-170-{BASE}"
    run = earlier_run(id=50, head_sha=ORIGIN, head_branch=branch) | fields
    return {
        "statuses": {EARLIER: validation(run=origin, marker_run=origin, tree=PREVIOUS)},
        "commits": {
            ORIGIN: {
                "parents": [{"sha": BASE}, {"sha": second}],
                "commit": {"tree": {"sha": built}},
            }
        },
        "progress": {run["id"]: [run]},
        "trees": {TREE: tree(CODE), BASE: INVENTORY, PREVIOUS: INVENTORY},
    }


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
                chain(),
                (chained, PREVIOUS),
                [],
            ),
        ]
        normal: list[tuple[str, dict[str, Any], list[int]]] = [
            # The result must name the run that built it, never this or a later run.
            *(
                (f"result of run {origin}", chain(id=origin), [])
                for origin in (100 + 1, 123, 200)
            ),
            ("origin unknown", dict(chain(), progress={}), []),
            ("origin failed", chain(conclusion="failure"), []),
            ("origin running", chain(status="in_progress", conclusion=None), []),
            ("origin of another event", chain(event="workflow_dispatch"), []),
            (
                "origin of another workflow",
                chain(path=".github/workflows/other.yml"),
                [],
            ),
            (
                "origin of another PR",
                chain(head_branch=f"gh-readonly-queue/rust-migration/pr-17-{BASE}"),
                [],
            ),
            ("origin of another head", chain(second="e" * 40), []),
            ("origin built another tree", chain(built=BASE), []),
            # A direct result's marker must name the candidate group's own tree.
            (
                "marker names another tree",
                {
                    "statuses": {
                        EARLIER: validation(
                            run=EARLIER_RUN, marker_run=EARLIER_RUN, tree=PREVIOUS
                        )
                    },
                    "trees": {TREE: tree(CODE), BASE: INVENTORY, PREVIOUS: INVENTORY},
                },
                [],
            ),
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
            world={
                "commits": {MERGE: GROUP},
                "runs": f"async () => ({{data: {{workflow_runs: {json.dumps([earlier_run()])}}}}})",
            },
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
                    good_needs(), PULL, queue=True, latest=pending(creator=creator)
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
                # The head ends the request identifier, not the name (#922).
                {"profile": "rust", "pr": "170", "request": "unique", "head": HEAD},
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
            if quick := filename == "ci-quick.yml":
                self.assertIn("ready_for_review", triggers["pull_request"]["types"])
            # Quick's gate jobs share the reporting job's condition (#1063).
            names = [name, *(("rust", "python", "cleanup") if quick else ())]
            conditions = [job(filename, each)["if"] for each in names]
            for (condition, each), (
                event,
                checkout,
                draft,
                expected,
            ) in itertools.product(
                zip(conditions, names, strict=True),
                (
                    ("pull_request", "", False, True),
                    # A draft's checkpoint runs no Quick; ready_for_review runs it (#920).
                    ("pull_request", "", True, not quick),
                    ("push", "", False, True),
                    ("merge_group", "", False, False),
                    # Full calls Quick inside the merge group.
                    ("merge_group", MERGE, False, quick),
                    # A requested profile or bootstrap calls Quick, draft or not.
                    ("pull_request", MERGE, True, True),
                ),
            ):
                with self.subTest(
                    filename=filename, job=each, event=event, draft=draft
                ):
                    observed = evaluate(
                        condition,
                        github={
                            "event_name": event,
                            "event": {"pull_request": {"draft": draft}},
                        },
                        inputs={"checkout-ref": checkout},
                    )
                    self.assertEqual(observed, expected)

    def test_queue_mode_refuses_an_ordinary_full_for_workflow_changes(self) -> None:
        # Both status writers decide "changes no workflow definition" identically.
        blocks = []
        for filename, name in (
            ("ci-invalidate.yml", "invalidate"),
            ("ci-request.yml", "plan"),
        ):
            script = script_for(filename, name)
            start = script.index("// The queue runs the group commit's own workflow")
            end = script.index("};", script.index("const definitionsUnchanged")) + 2
            blocks.append(" ".join(script[start:end].split()))
        self.assertEqual(len(set(blocks)), 1)
        code = [{"filename": "src/game.cpp"}]
        changed = [*code, {"filename": ".github/workflows/ci-request.yml"}]
        bootstrap = {"event_name": "pull_request"}
        for label, mode, profile, options, files, refused in (
            ("code", "true", "full", {}, code, False),
            ("workflow", "true", "full", {}, changed, True),
            ("unreadable list", "true", "full", {}, None, True),
            ("file cap", "true", "full", {}, code * 3000, True),
            # Today's mode, diagnosis profiles and root's bootstrap are unaffected.
            ("mode unset", "", "full", {}, changed, False),
            ("native", "true", "native", {}, changed, False),
            ("bootstrap", "true", "full", bootstrap, changed, False),
        ):
            with self.subTest(label):
                env = {"PR": "170", "HEAD": HEAD, "PROFILE": profile}
                result = execute_script(
                    "ci-request.yml",
                    "plan",
                    dict(PULL, number=170),
                    env={**env, "CI_MERGE_QUEUE": mode},
                    world={"files": files, **options},
                )
                if refused:
                    self.assertTrue(result.get("error"))
                    self.assertEqual(result["statuses"], [])
                else:
                    self.assertNotIn("error", result)
                    pending = [HEAD] if profile == "full" else []
                    self.assertEqual([s["sha"] for s in result["statuses"]], pending)

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
            (
                "workflow deleted",
                "true",
                [{"filename": ".github/workflows/x.yml", "status": "removed"}],
                pending,
            ),
            (
                "renamed into workflows",
                "true",
                [{"filename": ".github/workflows/x.yml", "previous_filename": "x.yml"}],
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


APT_STUB = r"""#!/bin/bash
kind=install
for arg; do
  case $arg in
    update) kind=update ;;
    --download-only) kind=download ;;
  esac
done
# The host apt uses: the lowest priority of a list, else the first.
host=$(awk 'match($0, /:\/\/[^\/]*\/ubuntu/) {
  host = substr($0, RSTART + 3, RLENGTH - 10)
  rank = 1e9
  if (match($0, /priority:[0-9]+/)) rank = substr($0, RSTART + 9) + 0
  if (best == "" || rank < low) { best = host; low = rank }
} END { print best }' "$APT_IN_USE")
echo "$kind $host" >> "$APT_LOG"
result=$(head -n 1 "$APT_RESULTS")
sed -i 1d "$APT_RESULTS"
# As apt does, an update fails on a mirror list that an active source uses and
# that holds no active line; a stanza with `Enabled: no` is not read.
if [ "$kind" = update ]; then
  for file in "$APT_ETC"/sources.list "$APT_ETC"/sources.list.d/*.sources \
      "$APT_ETC"/sources.list.d/*.list; do
    [ -f "$file" ] || continue
    for list in $(awk 'BEGIN { RS = "" }
        tolower($0) !~ /enabled:[[:space:]]*(no|false|off|without|disable)/' "$file" |
        grep -v '^[[:space:]]*#' | grep -o 'mirror+file:[^[:space:]]*' |
        sed 's/^mirror+file://'); do
      grep -q '^[[:space:]]*[^#[:space:]]' "$list" 2>/dev/null || exit 100
    done
  done
fi
exit "${result:-0}"
"""

SECURITY = "http://security.ubuntu.com/ubuntu/\tpriority:3\n"

# An unfinished pull_request run of ci-quick.yml at the head: the Quick
# validation check reports the other jobs and appears only when they end (#1063).
QUICK_RUNNING: ci_log.Run = {
    "id": 77,
    "status": "in_progress",
    "conclusion": None,
    "run_started_at": "2026-11-12T01:00:00Z",
    "html_url": "https://github.com/example/fork/actions/runs/77",
}


class AptInstallTests(unittest.TestCase):
    """The apt-install action's script against stub apt-get, sudo and timeout
    commands and temporary sources (#1026)."""

    def install(
        self, files: Mapping[str, str], in_use: str, results: Iterable[int]
    ) -> tuple[int, list[str], dict[str, str], str]:
        """Run the script with `files` under a temporary /etc/apt, apt-get
        answering `results` in turn and logging the first host of `in_use`:
        the exit code, apt-get's calls, the files afterwards and the warnings."""
        (step,) = yaml.safe_load(
            (ROOT / ".github/actions/apt-install/action.yml").read_text()
        )["runs"]["steps"]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            etc = root / "etc/apt"
            (etc / "sources.list.d").mkdir(parents=True)
            for name, text in files.items():
                (etc / name).write_text(text.replace("ETC", str(etc)))
            bin_dir = root / "bin"
            bin_dir.mkdir()
            for name, text in (
                ("apt-get", APT_STUB),
                ("sudo", '#!/bin/bash\nexec "$@"\n'),
                ("timeout", '#!/bin/bash\nshift\nexec "$@"\n'),
            ):
                (bin_dir / name).write_text(text)
                (bin_dir / name).chmod(0o755)
            (root / "results").write_text("".join(f"{r}\n" for r in results))
            env = dict(
                os.environ,
                PATH=f"{bin_dir}:{os.environ['PATH']}",
                PACKAGES="valgrind",
                RECOMMENDS="false",
                APT_IN_USE=str(etc / in_use),
                APT_ETC=str(etc),
                APT_LOG=str(root / "log"),
                APT_RESULTS=str(root / "results"),
            )
            script = step["run"].replace("/etc/apt", str(etc))
            done = subprocess.run(
                ["bash", "--noprofile", "--norc", "-eo", "pipefail", "-c", script],
                env=env,
                stdin=subprocess.DEVNULL,
                capture_output=True,
                text=True,
                check=False,
                timeout=60,
            )
            log = (root / "log").read_text().splitlines()
            # Every file, so a test can read a source the script wrote.
            after = {
                str(path.relative_to(etc)): path.read_text().replace(str(etc), "ETC")
                for path in sorted(etc.rglob("*"))
                if path.is_file()
            }
        self.stdout = done.stdout
        warnings = "\n".join(
            line for line in done.stdout.splitlines() if line.startswith("::warning")
        )
        return done.returncode, log, after, warnings

    HOSTED = {
        "apt-mirrors.txt": "http://us-east-2.ec2.archive.ubuntu.com/ubuntu/\t"
        "priority:1\nhttp://archive.ubuntu.com/ubuntu/\tpriority:2\n" + SECURITY,
        "sources.list.d/ubuntu.sources": "Types: deb\n"
        "URIs: mirror+file:ETC/apt-mirrors.txt\nSuites: noble noble-updates\n",
    }

    def test_each_failed_attempt_moves_to_a_host_not_yet_tried(self) -> None:
        # GitHub's hosted image: a mirror list, its EC2 host first.
        # The index updates, the download stalls, then Azure's update stalls.
        code, log, after, warnings = self.install(
            self.HOSTED, "apt-mirrors.txt", [0, 1, 1, 0, 0, 0]
        )
        self.assertEqual(code, 0)
        self.assertEqual(
            log,
            [
                "update us-east-2.ec2.archive.ubuntu.com",
                "download us-east-2.ec2.archive.ubuntu.com",
                # The changed mirror list invalidates the index: update again.
                "update azure.archive.ubuntu.com",
                "update archive.ubuntu.com",
                "download archive.ubuntu.com",
                "install archive.ubuntu.com",
            ],
        )
        self.assertIn(
            "attempt 1 failed or stalled on us-east-2.ec2.archive.ubuntu.com; "
            "retrying on azure.archive.ubuntu.com",
            warnings,
        )
        self.assertIn(
            "attempt 2 failed or stalled on azure.archive.ubuntu.com; "
            "retrying on archive.ubuntu.com",
            warnings,
        )
        # The security source and the rest of the list keep their hosts.
        self.assertTrue(after["apt-mirrors.txt"].endswith(SECURITY))
        self.assertEqual(
            after["sources.list.d/ubuntu.sources"],
            self.HOSTED["sources.list.d/ubuntu.sources"],
        )

    def test_a_retry_moves_every_entry_off_the_stalled_host(self) -> None:
        # apt takes a list's lowest priority first, whatever its line; a retry
        # that left the stalled host in front would try it again.
        files = dict(
            self.HOSTED,
            **{
                "apt-mirrors.txt": "http://azure.archive.ubuntu.com/ubuntu/\t"
                "priority:2\nhttp://us-east-2.ec2.archive.ubuntu.com/ubuntu/\t"
                "priority:1\n" + SECURITY
            },
        )
        code, log, after, warnings = self.install(
            files, "apt-mirrors.txt", [0, 1, 0, 0, 0]
        )
        self.assertEqual(code, 0)
        self.assertEqual(
            log,
            [
                "update us-east-2.ec2.archive.ubuntu.com",
                "download us-east-2.ec2.archive.ubuntu.com",
                "update azure.archive.ubuntu.com",
                "download azure.archive.ubuntu.com",
                "install azure.archive.ubuntu.com",
            ],
        )
        self.assertIn("stalled on us-east-2.ec2.archive.ubuntu.com", warnings)
        self.assertEqual(
            after["apt-mirrors.txt"],
            "http://azure.archive.ubuntu.com/ubuntu/\tpriority:2\n"
            "http://azure.archive.ubuntu.com/ubuntu/\tpriority:1\n" + SECURITY,
        )

    def test_a_retry_moves_every_active_list_off_its_host(self) -> None:
        # Two mirror lists for different suites, each on its own host: the one
        # a retry did not pick must not keep its stalled host.
        files = {
            "main-mirrors.txt": "http://us-east-2.ec2.archive.ubuntu.com/ubuntu/\n",
            "updates-mirrors.txt": "http://mirror.example.net/ubuntu/\n",
            "sources.list.d/ubuntu.sources": "Types: deb\n"
            "URIs: mirror+file:ETC/main-mirrors.txt\nSuites: noble\n\n"
            "Types: deb\nURIs: mirror+file:ETC/updates-mirrors.txt\n"
            "Suites: noble-updates\n",
        }
        code, log, after, _ = self.install(
            files, "updates-mirrors.txt", [0, 1, 0, 0, 0]
        )
        self.assertEqual(code, 0)
        self.assertEqual(
            log,
            [
                "update mirror.example.net",
                "download mirror.example.net",
                "update azure.archive.ubuntu.com",
                "download azure.archive.ubuntu.com",
                "install azure.archive.ubuntu.com",
            ],
        )
        for name in ("main-mirrors.txt", "updates-mirrors.txt"):
            self.assertEqual(after[name], "http://azure.archive.ubuntu.com/ubuntu/\n")

    def test_a_first_attempt_that_succeeds_rewrites_nothing(self) -> None:
        code, log, after, warnings = self.install(
            self.HOSTED, "apt-mirrors.txt", [0, 0, 0]
        )
        self.assertEqual(code, 0)
        host = "us-east-2.ec2.archive.ubuntu.com"
        self.assertEqual(log, [f"update {host}", f"download {host}", f"install {host}"])
        self.assertEqual(after, self.HOSTED)
        self.assertEqual(warnings, "")

    def test_a_configured_azure_mirror_is_not_tried_twice(self) -> None:
        # Emscripten's container: plain deb lines, pointed at Azure first.
        files = {
            "sources.list": "deb http://azure.archive.ubuntu.com/ubuntu noble main\n"
            "deb http://security.ubuntu.com/ubuntu noble-security main\n"
        }
        azure, archive = "azure.archive.ubuntu.com", "archive.ubuntu.com"
        for label, results, code, tail in (
            ("to success", [0, 1, 0, 1, 0, 0], 0, [f"install {archive}"]),
            ("to failure", [0, 1, 0, 1, 1], 1, []),
        ):
            with self.subTest(label):
                status, log, after, warnings = self.install(
                    files, "sources.list", results
                )
                self.assertEqual(status, code)
                self.assertEqual(
                    log,
                    [
                        f"update {azure}",
                        f"download {azure}",
                        f"update {archive}",
                        f"download {archive}",
                        # Sources unchanged since its update: the index is reused.
                        f"download {archive}",
                        *tail,
                    ],
                )
                self.assertIn(
                    f"attempt 2 failed or stalled on {archive}; retrying on {archive}",
                    warnings,
                )
                self.assertEqual(
                    after["sources.list"],
                    files["sources.list"].replace(f"//{azure}/", f"//{archive}/"),
                )

    FALLBACK = "sources.list.d/apt-install-fallback.sources"
    # Sources that name no Ubuntu archive in an active line (run 45526257014).
    UNNAMED = {
        "empty sources.list.d": {},
        "an unused mirror list": {
            "apt-mirrors.txt": HOSTED["apt-mirrors.txt"],
            "sources.list.d/ubuntu.sources": "# Types: deb\n"
            "# URIs: mirror+file:ETC/apt-mirrors.txt\n# Suites: noble\n",
        },
        "a disabled stanza": {
            "sources.list.d/ubuntu.sources": "Types: deb\n"
            "URIs: http://us-east-2.ec2.archive.ubuntu.com/ubuntu/\n"
            "Suites: noble\nComponents: main\nEnabled: no\n",
        },
        # apt reads `false` as `no` too (review of #1065).
        "a stanza disabled as false": {
            "sources.list.d/ubuntu.sources": "Types: deb\n"
            "URIs: http://us-east-2.ec2.archive.ubuntu.com/ubuntu/\n"
            "Suites: noble\nComponents: main\nEnabled: false\n",
        },
    }

    def test_sources_naming_no_archive_get_azure_then_canonicals_archive(
        self,
    ) -> None:
        codename = subprocess.run(
            ["bash", "-c", ". /etc/os-release && echo $VERSION_CODENAME"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip()
        azure, archive = "azure.archive.ubuntu.com", "archive.ubuntu.com"
        for (label, files), (results, hosts) in itertools.product(
            self.UNNAMED.items(),
            (
                # The first download fails: Azure on the second attempt.
                ([0, 1, 0, 0, 0], [azure, azure, azure]),
                # Azure's update fails too: Canonical's archive on the third.
                ([0, 1, 1, 0, 0, 0], [azure, archive, archive, archive]),
            ),
        ):
            with self.subTest(label, attempts=len(hosts)):
                code, log, after, warnings = self.install(files, self.FALLBACK, results)
                self.assertEqual(code, 0)
                # The first attempt runs on the sources as found.
                self.assertEqual(
                    log,
                    [
                        "update ",
                        "download ",
                        *(f"update {host}" for host in hosts[:-2]),
                        f"download {hosts[-1]}",
                        f"install {hosts[-1]}",
                    ],
                )
                self.assertIn(
                    "attempt 1 failed or stalled on the sources as found; "
                    f"retrying on {azure}",
                    warnings,
                )
                self.assertEqual(
                    after.pop(self.FALLBACK),
                    "Types: deb\n"
                    f"URIs: http://{hosts[-1]}/ubuntu\n"
                    f"Suites: {codename} {codename}-updates {codename}-backports "
                    f"{codename}-security\n"
                    "Components: main restricted universe multiverse\n"
                    "Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg\n",
                )
                if len(hosts) == 4:
                    self.assertIn(
                        f"attempt 2 failed or stalled on {azure}; retrying on {archive}",
                        warnings,
                    )

    EMPTY = "# http://us-east-2.ec2.archive.ubuntu.com/ubuntu/\n"
    # A deb822 file: a stanza on a mirror list with no active line, then a
    # working repository that must stay (review of #1065).
    BROKEN = (
        "Types: deb\nURIs: mirror+file:ETC/apt-mirrors.txt\nSuites: noble\n"
        "Components: main\nEnabled: yes\n\n"
        "Types: deb\nURIs: file:REPO\nSuites: ./\nTrusted: yes\n"
    )

    def test_an_entry_using_a_mirror_list_with_no_active_line_is_set_aside(
        self,
    ) -> None:
        """apt fails its whole update on such a list, so the fallback source
        alone would not save the job (review of #1065)."""
        disabled = (
            "Types: deb\nURIs: mirror+file:ETC/apt-mirrors.txt\nSuites: noble\n"
            "Components: main\nEnabled: no\n\n"
            "Types: deb\nURIs: file:REPO\nSuites: ./\nTrusted: yes\n\n"
        )
        for label, name, before, after_text in (
            ("deb822", "sources.list.d/ubuntu.sources", self.BROKEN, disabled),
            (
                "one-line",
                "sources.list",
                "deb mirror+file:ETC/apt-mirrors.txt noble main\n",
                "# deb mirror+file:ETC/apt-mirrors.txt noble main\n",
            ),
        ):
            with self.subTest(label):
                files = {"apt-mirrors.txt": self.EMPTY, name: before}
                code, log, after, warnings = self.install(
                    files, self.FALLBACK, [0, 0, 0, 0]
                )
                self.assertEqual(code, 0)
                azure = "azure.archive.ubuntu.com"
                # The first update fails on the empty list; Azure's succeeds.
                self.assertEqual(
                    log,
                    [
                        "update ",
                        f"update {azure}",
                        f"download {azure}",
                        f"install {azure}",
                    ],
                )
                self.assertIn(f"retrying on {azure}", warnings)
                self.assertIn(self.FALLBACK, after)
                self.assertEqual(after[name], after_text)

    @unittest.skipUnless(shutil.which("apt-get"), "needs apt-get")
    def test_real_apt_updates_only_once_the_entry_is_set_aside(self) -> None:
        """An isolated apt update, as the stub models it: it fails on the
        sources as found and succeeds on them as the script leaves them, with
        the working stanza still read."""
        _, _, after, _ = self.install(
            {
                "apt-mirrors.txt": self.EMPTY,
                "sources.list.d/ubuntu.sources": self.BROKEN,
            },
            self.FALLBACK,
            [0, 0, 0, 0],
        )
        # The fallback names the internet archive; this update stays offline.
        del after[self.FALLBACK]
        found = {
            "apt-mirrors.txt": self.EMPTY,
            "sources.list.d/ubuntu.sources": self.BROKEN,
        }
        self.assertEqual(self.real_update(found)[0], 100)
        self.assertEqual(self.real_update(after), (0, True))

    def real_update(self, files: Mapping[str, str]) -> tuple[int, bool]:
        """apt-get update over `files` under a temporary Dir, with an empty
        local repository at REPO: its exit code and whether it read REPO."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            etc, repo = root / "etc", root / "repo"
            for path in (
                etc / "sources.list.d",
                root / "state/lists/partial",
                root / "cache/archives/partial",
                repo,
            ):
                path.mkdir(parents=True)
            (repo / "Packages").write_text("")
            for name, text in files.items():
                (etc / name).write_text(
                    text.replace("ETC", str(etc)).replace("REPO", str(repo))
                )
            done = subprocess.run(
                [
                    "apt-get",
                    *("-o", f"Dir::Etc={etc}", "-o", f"Dir::State={root / 'state'}"),
                    *("-o", f"Dir::Cache={root / 'cache'}", "-o", "Debug::NoLocking=1"),
                    *("-o", "Dir::State::status=/dev/null", "update"),
                ],
                capture_output=True,
                text=True,
                check=False,
                timeout=60,
            )
            read = any(
                "_repo_" in path.name for path in (root / "state/lists").iterdir()
            )
        return done.returncode, read

    def test_a_failed_attempt_prints_the_sources_active_lines(self) -> None:
        files = self.UNNAMED["an unused mirror list"]
        _, _, _, _ = self.install(files, self.FALLBACK, [0, 1, 0, 0, 0])
        group = self.stdout.split("::group::apt sources after attempt 1\n")[1]
        group = group.split("::endgroup::")[0]
        self.assertIn("/sources.list.d/ubuntu.sources:\n", group)
        self.assertIn(
            "/apt-mirrors.txt:\nhttp://us-east-2.ec2.archive.ubuntu.com/ubuntu/", group
        )
        # Commented lines are left out: they are what apt does not read.
        self.assertNotIn("# URIs", group)
        self.assertNotIn("attempt 2", self.stdout)
        # The last attempt's failure prints them too.
        self.install(files, self.FALLBACK, [0, 1, 1, 1, 1])
        self.assertIn("::group::apt sources after attempt 3\n", self.stdout)

    def test_another_mirror_list_a_source_names_is_rewritten(self) -> None:
        # Blacksmith's runners name their own list, not apt-mirrors.txt.
        files = {
            "blacksmith-ubuntu-mirrors.txt": "http://mirror.example.net/ubuntu/\n",
            "sources.list.d/ubuntu.sources": "Types: deb\n"
            "URIs: mirror+file:ETC/blacksmith-ubuntu-mirrors.txt\n",
        }
        code, log, after, _ = self.install(
            files, "blacksmith-ubuntu-mirrors.txt", [1, 0, 0, 0]
        )
        self.assertEqual(code, 0)
        self.assertEqual(
            log,
            [
                "update mirror.example.net",
                "update azure.archive.ubuntu.com",
                "download azure.archive.ubuntu.com",
                "install azure.archive.ubuntu.com",
            ],
        )
        self.assertEqual(
            after["blacksmith-ubuntu-mirrors.txt"],
            "http://azure.archive.ubuntu.com/ubuntu/\n",
        )


class PolicyTests(unittest.TestCase):
    def test_platform_matrix_matches_profile_and_cadence(self) -> None:
        jobs = workflow("ci-build.yml")["jobs"]
        # A direct dispatch is the cadence; controller dispatches carry a checkout.
        dispatch = "workflow_dispatch"
        all_linux, x64 = ["clang", "sdl2", "dedicated"], ["x64"]
        for profile, checkout, event, heavy, covered, linux, windows, macos, rust32 in (
            ("full", "merge", dispatch, "", "", all_linux, x64, False, True),
            # Native adds Clang and the 32-bit Rust job before the queue (#904).
            ("native", "merge", dispatch, "", "", ["clang", "sdl2"], [], False, True),
            ("platform", "merge", dispatch, "", "", all_linux, x64, True, True),
            ("", "", dispatch, "", "", ["clang", "dedicated"], x64, True, True),
            # A covered push refreshes every cached flavour Full restores and tests nothing.
            ("", "", "push", "false", "true", ["sdl2", "dedicated"], x64, False, False),
            ("", "", "push", "true", "", all_linux, x64, True, True),
            ("", "", "pull_request", "true", "", ["sdl2"], x64, True, False),
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
                    matrix = body["strategy"]["matrix"][axis]
                    selected = (
                        (
                            matrix
                            if isinstance(matrix, list)
                            else evaluate(matrix, **contexts)
                        )
                        if evaluate(body["if"], **contexts)
                        else []
                    )
                    self.assertEqual(selected, expected)
                self.assertEqual(evaluate(jobs["rust-32bit"]["if"], **contexts), rust32)
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
        steps = quick_workflow.steps(
            cast(quick_workflow.Workflow, workflow("ci-quick.yml"))
        )
        for step in steps:
            self.assertFalse(str(step.get("uses", "")).startswith("./"), step)
        # Its package install is the shared action's script, copied, so the two
        # cannot drift: the copy once lost the action's index and dpkg handling.
        # Exactly one step across the jobs carries it.
        (shared,) = yaml.safe_load(
            (ROOT / ".github/actions/apt-install/action.yml").read_text()
        )["runs"]["steps"]
        (inline,) = (step for step in steps if "PACKAGES" in step.get("env", {}))
        self.assertEqual(inline["run"], shared["run"])
        self.assertEqual(set(inline["env"]), set(shared["env"]))

    def test_quick_validation_needs_every_other_quick_job(self) -> None:
        """The required check reports the split jobs, so a job added later
        cannot be left out of it (#1063)."""
        jobs = workflow("ci-quick.yml")["jobs"]
        self.assertEqual(jobs["quick"]["name"], "Quick validation")
        self.assertEqual(set(jobs["quick"]["needs"]), set(jobs) - {"quick"})
        for name, body in jobs.items():
            if name != "quick":
                self.assertNotIn("needs", body, name)
                self.assertEqual(body["runs-on"], "ubuntu-24.04", name)
                # Each job keys its own uv cache on Quick's own pins (#940).
                (uv,) = (s for s in body["steps"] if "setup-uv" in s.get("uses", ""))
                self.assertEqual(
                    uv["with"]["cache-dependency-glob"],
                    ".github/workflows/ci-quick.yml",
                )
                self.assertEqual(uv["with"]["cache-suffix"], name)

    def test_quick_validation_fails_unless_every_job_succeeded(self) -> None:
        """A skipped required check passes, so the reporting job always runs
        and its one step fails on any other result (#1063)."""
        body = job("ci-quick.yml", "quick")
        (step,) = body["steps"]
        needed = body["needs"]
        contexts: dict[str, Any] = {"inputs": {"checkout-ref": MERGE}, "github": {}}
        for failed, result in itertools.product(
            needed, ("failure", "cancelled", "skipped", "success")
        ):
            needs = {
                name: {"result": result if name == failed else "success"}
                for name in needed
            }
            with self.subTest(job=failed, result=result):
                # always(): the job runs whatever the others did.
                self.assertTrue(evaluate(body["if"], needs=needs, **contexts))
                runs = evaluate(step["if"], needs=needs, **contexts)
                self.assertEqual(runs, result != "success")
                if not runs:
                    continue
                # The step that runs fails the job, as its own shell exits.
                results = evaluate(
                    step["env"]["RESULTS"], interpolate=True, needs=needs, **contexts
                )
                done = subprocess.run(
                    [
                        "bash",
                        "--noprofile",
                        "--norc",
                        "-eo",
                        "pipefail",
                        "-c",
                        step["run"],
                    ],
                    env=dict(os.environ, RESULTS=str(results)),
                    capture_output=True,
                    text=True,
                    check=False,
                )
                self.assertNotEqual(done.returncode, 0)
                self.assertIn(f"{result}", done.stdout)

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

    def test_windows_targets_x64_alone(self) -> None:
        """Windows x86 is retired (#899), with no Rust-only path left."""
        windows = workflow("ci-build.yml")["jobs"]["windows"]
        self.assertEqual(windows["strategy"]["matrix"]["arch"], ["x64"])
        self.assertNotIn("rust-only", windows["with"])
        # PyYAML reads the `on:` key as True.
        triggers = cast(dict[Any, Any], workflow("ci-windows.yml"))[True]
        inputs = triggers["workflow_call"]["inputs"]
        self.assertNotIn("rust-only", inputs)

    def test_rust_32bit_job_targets_i686_on_a_free_runner(self) -> None:
        """Its execution is shown by CI and root's width-dependent negative control (#899)."""
        body = workflow("ci-build.yml")["jobs"]["rust-32bit"]
        self.assertEqual(body["runs-on"], "ubuntu-24.04")
        self.assertEqual(body["env"]["RUST_TARGET"], "i686-unknown-linux-gnu")

    def test_heavy_predicates_bypass_skipped_policy_but_respect_cancellation(
        self,
    ) -> None:
        dispatch = "workflow_dispatch"
        for filename, names in (
            ("ci-build.yml", ("emscripten", "linux", "macos", "windows", "rust-32bit")),
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
                if name == "rust-32bit":
                    cases = [
                        (False, "head", "", "full", dispatch, True),
                        (False, "head", "", "platform", dispatch, True),
                        (False, "head", "", "native", dispatch, True),
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
                "rust-32bit",
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
                    event=event,
                    variables={"CI_ON_DEMAND": on_demand},
                    files=files,
                    trees=trees,
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
                for variables in (ON_DEMAND, QUEUE_MODE):
                    result = run_policy(
                        variables=variables,
                        files=files,
                        pulls=[pull],
                        statuses={HEAD: statuses},
                    )
                    self.assertEqual(result["outputs"]["heavy"], expected)
                # A merge-queue merge pushes the tested group commit itself, which
                # needs no merged PR.
                if merged:
                    result = run_policy(
                        variables=QUEUE_MODE, files=files, statuses={PUSHED: statuses}
                    )
                    self.assertEqual(result["outputs"]["heavy"], expected)

    def test_queue_mode_coverage_trusts_only_the_publishers_statuses(self) -> None:
        files = [{"filename": "src/code.cpp"}]
        merged = [dict(PULL, merged_at="date")]
        trusted = validation()
        for index in (0, 1):
            for creator in (None, {}, {"login": "writer", "type": "User"}):
                forged = [dict(status) for status in trusted]
                forged[index]["creator"] = creator
                with self.subTest(index=index, creator=creator):
                    for result in (
                        run_policy(
                            variables=QUEUE_MODE,
                            files=files,
                            statuses={PUSHED: forged},
                        ),
                        run_policy(
                            variables=QUEUE_MODE,
                            files=files,
                            pulls=merged,
                            statuses={HEAD: forged},
                        ),
                    ):
                        self.assertEqual(result["outputs"]["heavy"], "true")
                    # A newer untrusted status hides an older trusted one.
                    result = run_policy(
                        variables=QUEUE_MODE,
                        files=files,
                        statuses={PUSHED: [forged[index], *trusted]},
                    )
                    self.assertEqual(result["outputs"]["heavy"], "true")
        # An untested pushed commit still finds the merged PR head's result.
        result = run_policy(
            variables=QUEUE_MODE,
            files=files,
            pulls=merged,
            statuses={PUSHED: validation("failure"), HEAD: trusted},
        )
        self.assertEqual(result["outputs"], {"heavy": "false", "covered": "true"})

    def test_coverage_is_unchanged_without_queue_mode(self) -> None:
        files = [{"filename": "src/code.cpp"}]
        merged = [dict(PULL, merged_at="date")]
        # The pushed commit's own statuses count only in merge-queue mode.
        result = run_policy(files=files, statuses={PUSHED: validation()})
        self.assertEqual(result["outputs"], {"heavy": "true"})
        # Merged PR heads keep the combined-status rule, which sees no creator.
        forged = [dict(status, creator=None) for status in validation()]
        result = run_policy(files=files, pulls=merged, statuses={HEAD: forged})
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
                        statuses={HEAD: validation(tree=BASE)},
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


REQUIRE_STAMP = ci.require_stamp


class QuickCheck(TypedDict):
    """The fields of a Quick check run that `latest_quick` and `ready` read."""

    conclusion: str
    started_at: str


class RequestTests(unittest.TestCase):
    def setUp(self) -> None:
        # Stamps are tools/test-preflight.py's; these tests read only the request.
        stamp = patch.object(ci, "require_stamp")
        self.stamp = stamp.start()
        self.addCleanup(stamp.stop)
        # The one-native-run rule has its own tests, on enqueue's recorded answers.
        first = patch.object(ci, "require_first_native")
        first.start()
        self.addCleanup(first.stop)

    def test_request_and_ready_need_a_stamp_before_any_change(self) -> None:
        self.stamp.side_effect = ValueError("no preflight stamp")
        args = argparse.Namespace(
            repo="example/fork",
            pr=170,
            profile="native",
            bootstrap=False,
            unstamped=None,
            diagnose=None,
            head=None,
            wait=False,
        )
        with (
            patch.object(ci, "api", side_effect=pull_and_statuses([])),
            patch.object(ci, "gh") as gh,
        ):
            for command in (ci.request, ci.ready):
                with self.assertRaisesRegex(ValueError, "no preflight stamp"):
                    command(args)
            gh.assert_not_called()
        self.assertEqual(self.stamp.call_args.args, (170, HEAD, None))

    def ready(
        self,
        quick: list[QuickCheck],
        runs: list[ci_log.Run] | None = None,
        **pull: object,
    ) -> list[tuple]:
        """Run ready on PULL with pull's changes, `quick` as the head's checks and
        `runs` as its pull_request runs of ci-quick.yml."""

        def answers(_repository: str, path: str) -> object:
            if path.startswith("pulls/"):
                return dict(PULL, **pull)
            if "actions/workflows/ci-quick.yml/runs" in path:
                return {"workflow_runs": runs or []}
            return {"check_runs": quick}

        args = argparse.Namespace(repo="example/fork", pr=170)
        with (
            patch.object(ci, "api", side_effect=answers),
            patch.object(ci, "gh", return_value="") as gh,
            patch("builtins.print"),
        ):
            ci.ready(args)
        return [call.args for call in gh.call_args_list]

    def test_ready_restarts_a_quick_that_only_skipped(self) -> None:
        mark = ("pr", "ready", "170", "--repo", "example/fork")
        skipped = QuickCheck(conclusion="skipped", started_at="2026-11-12T01:00:00Z")
        passed = QuickCheck(conclusion="success", started_at="2026-11-12T00:59:00Z")
        self.assertEqual(self.ready([skipped], draft=False), [(*mark, "--undo"), mark])
        self.assertEqual(self.ready([skipped, passed], draft=False), [mark])
        self.assertEqual(self.ready([skipped], draft=True), [mark])
        self.assertEqual(self.ready([], draft=False), [mark])

    def test_ready_marks_nothing_again_while_quick_runs(self) -> None:
        """A draft's skip with Quick's run still going is a run, not a skip to
        restart (#1063); with no such run it is restarted as before."""
        mark = ("pr", "ready", "170", "--repo", "example/fork")
        skipped = QuickCheck(conclusion="skipped", started_at="2026-11-12T01:00:00Z")
        self.assertEqual(self.ready([skipped], [QUICK_RUNNING], draft=False), [mark])
        self.assertEqual(
            self.ready(
                [skipped], [{**QUICK_RUNNING, "status": "completed"}], draft=False
            ),
            [(*mark, "--undo"), mark],
        )

    def test_ready_refuses_a_body_without_its_attribution_line(self) -> None:
        with self.assertRaisesRegex(ValueError, "attribution line"):
            self.ready([], draft=True, body="Agent: /x")

    def test_unstamped_request_is_logged_not_checked(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(
                ci.preflight_stamp, "common_local", return_value=Path(directory)
            ),
            patch.object(ci.preflight_stamp, "stamp_for") as stamp_for,
            patch("builtins.print"),
        ):
            REQUIRE_STAMP(917, HEAD, "seeded debug-assert control")
            stamp_for.assert_not_called()
            (entry,) = (Path(directory) / "overrides.log").read_text().splitlines()
        self.assertEqual(
            {k: v for k, v in json.loads(entry).items() if k in ("kind", "reason")},
            {"kind": "unstamped", "reason": "seeded debug-assert control"},
        )

    def test_each_profile_dispatches_captured_head_on_protected_ref(self) -> None:
        for profile in ci.PROFILES:
            with tempfile.TemporaryDirectory() as directory:
                args = argparse.Namespace(
                    repo="example/fork",
                    pr=170,
                    profile=profile,
                    bootstrap=False,
                    unstamped=None,
                    diagnose=None,
                    head=None,
                    wait=False,
                    request_file=Path(directory) / "request.json",
                )
                with (
                    patch.object(ci, "api", side_effect=pull_and_statuses([])),
                    patch.object(
                        ci,
                        "gh",
                        side_effect=lambda *args: json.dumps(
                            [] if args[0] == "variable" else [[]]
                        ),
                    ) as gh,
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
                request_id, head = record["request"].split("-")
                self.assertEqual((len(request_id), head), (32, HEAD))

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
                    unstamped=None,
                    diagnose=None,
                    head=None,
                    wait=False,
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

    def test_queue_mode_full_request_refuses_workflow_changes(self) -> None:
        code = [{"filename": "src/game.cpp"}]
        workflow = [*code, {"filename": ".github/workflows/ci-request.yml"}]
        renamed = [
            {"filename": "a.yml", "previous_filename": ".github/workflows/a.yml"}
        ]
        for mode, profile, bootstrap, files, refused in (
            ("true", "full", False, code, False),
            ("true", "full", False, workflow, True),
            ("true", "full", False, renamed, True),
            ("true", "full", False, [], True),
            ("true", "full", False, code * 3000, True),
            ("true", "full", False, [{}], True),
            # Today's mode, diagnosis profiles and root's bootstrap are unaffected.
            ("", "full", False, workflow, False),
            ("true", "native", False, workflow, False),
            ("true", "full", True, workflow, False),
        ):
            # Root unsets every gating variable for a bootstrap request.
            variables = [] if bootstrap else [{"name": "CI_MERGE_QUEUE", "value": mode}]

            def answer(
                *args: str, files: object = files, variables: object = variables
            ) -> str:
                if args[0] == "variable":
                    return json.dumps(variables)
                if args[-1].startswith("repos/example/fork/pulls/170/files"):
                    return json.dumps([files])
                return json.dumps([[]])

            with (
                self.subTest(mode=mode, profile=profile, files=len(files)),
                tempfile.TemporaryDirectory() as directory,
            ):
                args = argparse.Namespace(
                    repo="example/fork",
                    pr=170,
                    profile=profile,
                    bootstrap=bootstrap,
                    unstamped=None,
                    diagnose=None,
                    head=None,
                    wait=False,
                    request_file=Path(directory) / "request.json",
                )
                with (
                    patch.object(ci, "api", side_effect=pull_and_statuses([])),
                    patch.object(ci, "gh", side_effect=answer) as gh,
                    patch("builtins.print"),
                ):
                    if refused:
                        with self.assertRaisesRegex(ValueError, "bootstrap Full"):
                            ci.request(args)
                    else:
                        ci.request(args)
                writes = [
                    c for c in gh.call_args_list if c.args[0] in ("workflow", "pr")
                ]
                self.assertEqual(not writes, refused)

    def test_full_request_dispatches_when_variables_are_unreadable(self) -> None:
        def answer(*args: str) -> str:
            if args[0] == "variable":
                raise subprocess.CalledProcessError(1, ["gh", *args], stderr="HTTP 403")
            return json.dumps([[]])

        with tempfile.TemporaryDirectory() as directory:
            args = argparse.Namespace(
                repo="example/fork",
                pr=170,
                profile="full",
                bootstrap=False,
                unstamped=None,
                diagnose=None,
                head=None,
                wait=False,
                request_file=Path(directory) / "request.json",
            )
            with (
                patch.object(ci, "api", side_effect=pull_and_statuses([])),
                patch.object(ci, "gh", side_effect=answer) as gh,
                patch("builtins.print"),
            ):
                self.assertIsNotNone(ci.request(args))
            self.assertEqual(gh.call_args.args[:2], ("workflow", "run"))

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
            with self.assertRaisesRegex(ValueError, "conflicts|mergeability"):
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
            ci.verify_children({"repository": "example/fork"}, {"failed_jobs": []})

    def test_wait_times_out_with_exit_2_and_names_resume_command(self) -> None:
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
            # Waits run as background jobs, so both default to queue-wait's limit.
            for argv, timeout in (
                (["ci.py", "wait", str(path)], 5400),
                (["ci.py", "request", "170", "--wait"], 5400),
                (["ci.py", "request", "170", "--wait", "--timeout", "60"], 60),
                (["ci.py", "wait", str(path), "--timeout", "120"], 120),
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
                self.assertEqual(waited.call_args.args[1:], (timeout, 15, False))
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


def requested(
    profile: str, request: str, created: str, conclusion: str | None = None
) -> ci_log.Run:
    """A dispatched ci-request.yml run as GitHub lists it, still queued without a
    conclusion; its ID is the hour it was created."""
    return {
        "id": int(created[11:13]),
        "run_attempt": 1,
        "status": "queued" if conclusion is None else "completed",
        "conclusion": conclusion,
        "display_title": f"CI {profile} PR 170 {request}-{HEAD}",
        "event": "workflow_dispatch",
        "head_branch": "rust-migration",
        "created_at": created,
        "html_url": f"https://github.com/example/fork/actions/runs/{created[11:13]}",
    }


class RequestPathTests(unittest.TestCase):
    """request waits out pending mergeability; wait takes a PR number (P3)."""

    PENDING = dict(PULL, mergeable=None, merge_commit_sha=None)

    def setUp(self) -> None:
        stamp = patch.object(ci, "require_stamp")
        self.stamp = stamp.start()
        self.addCleanup(stamp.stop)
        # The one-native-run rule has its own tests, on enqueue's recorded answers.
        first = patch.object(ci, "require_first_native")
        first.start()
        self.addCleanup(first.stop)
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.directory = Path(directory.name)
        requests = patch.object(ci, "REQUESTS", self.directory / "requests")
        requests.start()
        self.addCleanup(requests.stop)

    def request(
        self, *pulls: dict, **options: object
    ) -> tuple[Path | None, list[tuple[str, ...]], str, int]:
        """Run request against successive reads of the PR, on a fake clock."""
        reads = iter(pulls)
        calls: list[tuple[str, ...]] = []
        args = argparse.Namespace(
            **{
                "repo": "example/fork",
                "pr": 170,
                "profile": "native",
                "bootstrap": False,
                "unstamped": None,
                "diagnose": None,
                "head": None,
                "wait": False,
                "request_file": None,
                **options,
            }
        )
        clock = itertools.count(0, 3)
        with (
            patch.object(ci, "api", side_effect=lambda _r, _p: next(reads)),
            patch.object(ci, "gh", side_effect=lambda *a: calls.append(a) or ""),
            patch.object(ci.time, "monotonic", side_effect=lambda: next(clock)),
            patch.object(ci.time, "sleep") as sleep,
            patch("builtins.print") as printed,
        ):
            try:
                path = ci.request(args)
            finally:
                self.output = "\n".join(str(c.args[0]) for c in printed.call_args_list)
                self.calls, self.sleeps = calls, sleep.call_count
        return path, calls, self.output, sleep.call_count

    def test_pending_then_mergeable_dispatches_the_pinned_head(self) -> None:
        path, calls, output, sleeps = self.request(self.PENDING, self.PENDING, PULL)
        self.assertEqual(sleeps, 2)
        self.assertEqual(output.count(f"Waiting up to {ci.MERGEABLE_WAIT} seconds"), 1)
        ((dispatch),) = calls
        self.assertEqual(dispatch[:2], ("workflow", "run"))
        self.assertIn(f"head={HEAD}", dispatch)
        self.assertTrue(
            output.endswith(
                f"Wait for it as a background job: python3 tools/ci.py wait {path}"
            )
        )

    def test_pending_then_conflict_or_closed_stops_at_once(self) -> None:
        for label, after, message in (
            ("conflict", dict(PULL, mergeable=False), "conflicts with rust-migration"),
            ("closed", dict(self.PENDING, state="closed"), "must be open"),
        ):
            with self.subTest(label), self.assertRaisesRegex(ValueError, message):
                self.request(self.PENDING, after)
            self.assertNotIn("Requested", self.output)

    def test_a_head_that_moves_while_pending_is_followed_once(self) -> None:
        # Just after a push GitHub still answers with the previous head.
        stale = dict(self.PENDING, head={"sha": PREVIOUS})
        path, calls, output, _ = self.request(stale, PULL, PULL)
        self.assertIsNotNone(path)
        self.assertIn("requesting for the new head", output)
        ((dispatch),) = calls
        self.assertIn(f"head={HEAD}", dispatch)
        self.assertEqual(
            [c.args[1] for c in self.stamp.call_args_list], [PREVIOUS, HEAD]
        )
        # A head that moves again is refused, and so is any move from a named head.
        moved = dict(PULL, head={"sha": PARENT})
        with self.assertRaisesRegex(ci.HeadMoved, "head changed"):
            self.request(stale, PULL, self.PENDING, moved)
        self.assertNotIn("Requested", self.output)
        named = dict(self.PENDING, head={"sha": PREVIOUS})
        with self.assertRaisesRegex(ci.HeadMoved, "head changed"):
            self.request(named, PULL, head=PREVIOUS)
        self.assertNotIn("requesting for the new head", self.output)

    def test_conflict_is_worded_apart_from_pending(self) -> None:
        with self.assertRaisesRegex(ValueError, "merge origin/rust-migration"):
            self.request(dict(PULL, mergeable=False))
        self.assertNotIn("Waiting", self.output)
        # Pending past the bound stops without dispatching, and says so.
        with self.assertRaisesRegex(ci.MergeabilityPending, "still computing"):
            self.request(*[self.PENDING] * (ci.MERGEABLE_WAIT // 3 + 2))
        self.assertNotIn("Requested", self.output)

    def test_head_option_refuses_another_remote_head(self) -> None:
        # GitHub keeps answering with another head: refused at the bound.
        reads = [PULL] * (ci.MERGEABLE_WAIT // 3 + 2)
        with self.assertRaisesRegex(ValueError, f"head on GitHub is {HEAD}, not"):
            self.request(*reads, head=PREVIOUS)
        self.assertEqual(self.output.count("waiting up to"), 1)
        self.stamp.assert_not_called()
        # Just after a push the first read shows the old head, then the pushed one.
        stale = dict(PULL, head={"sha": PREVIOUS})
        path, calls, output, sleeps = self.request(stale, PULL, PULL, head=HEAD)
        self.assertIsNotNone(path)
        self.assertEqual(sleeps, 1)
        self.assertIn(f"head={HEAD}", calls[0])
        path, calls, _, _ = self.request(PULL, head=HEAD[:12])
        self.assertIsNotNone(path)
        self.assertEqual(calls[0][:2], ("workflow", "run"))
        with patch("sys.argv", ["ci.py", "request", "170", "--head", "main"]):
            with patch("sys.stderr"), self.assertRaises(SystemExit):
                ci.main()

    def checkout(self, branch: str = "feature", url: str = "") -> tuple[str, str]:
        """A clone on BRANCH whose origin is URL, with origin/BRANCH at its HEAD,
        as ci.py's own checkout; returns the HEAD and an earlier commit."""
        root = self.directory / "checkout"
        git = partial(subprocess.run, cwd=root, check=True, capture_output=True)
        root.mkdir()
        git(["git", "init", "--quiet", "-b", branch])
        url = url or "https://github.com/example/fork.git"
        git(["git", "remote", "add", "origin", url])
        for message in ("Add: one", "Add: two"):
            git(
                ["git", "-c", "user.name=t", "-c", "user.email=t@t", "commit"]
                + ["--quiet", "--allow-empty", "-m", message]
            )
        rev = partial(subprocess.check_output, cwd=root, text=True)
        head = rev(["git", "rev-parse", "HEAD"]).strip()
        git(["git", "update-ref", f"refs/remotes/origin/{branch}", head])
        root_patch = patch.object(ci.vcs, "ROOT", root)
        root_patch.start()
        self.addCleanup(root_patch.stop)
        self.git = git
        return head, rev(["git", "rev-parse", "HEAD~1"]).strip()

    def pull(
        self, sha: str, repository: str = "example/fork", **fields: object
    ) -> dict:
        head = ci.PullHead(sha=sha, ref="feature", repo={"full_name": repository})
        return dict(PULL, head=head, **fields)

    def test_the_prs_checkout_waits_for_its_pushed_head(self) -> None:
        # The two cases of 2026-11-24: the first read still shows the old head.
        local, previous = self.checkout()
        stale, pushed = self.pull(previous), self.pull(local)
        path, calls, output, sleeps = self.request(stale, pushed, pushed)
        self.assertIsNotNone(path)
        self.assertEqual(sleeps, 1)
        ((dispatch),) = calls
        self.assertIn(f"head={local}", dispatch)
        self.assertIn(f"Requesting for this checkout's HEAD {local}", output)

    def test_another_branch_or_a_detached_head_reads_github(self) -> None:
        local, previous = self.checkout(branch="other")
        path, calls, output, sleeps = self.request(self.pull(previous))
        self.assertEqual((sleeps, len(calls)), (0, 1))
        self.assertIn(f"head={previous}", calls[0])
        self.git(["git", "switch", "--quiet", "--detach", "HEAD"])
        path, calls, output, sleeps = self.request(self.pull(previous))
        self.assertIsNotNone(path)
        self.assertEqual(sleeps, 0)
        self.assertIn(f"head={previous}", calls[0])
        self.assertNotIn("this checkout", output)

    def test_an_unpushed_or_stale_checkout_refuses_at_once(self) -> None:
        local, previous = self.checkout()
        commit = ["git", "-c", "user.name=t", "-c", "user.email=t@t", "commit"]
        # Ahead: a commit made after the last push.
        self.git([*commit, "--quiet", "--allow-empty", "-m", "Add: unpushed"])
        ahead = ci.checkout_git("rev-parse", "HEAD")
        # Behind: origin has a commit this checkout lacks.
        cases = {"ahead": (ahead, local), "behind": (previous, local)}
        for name, (head, tracking) in cases.items():
            with self.subTest(name=name):
                self.git(["git", "reset", "--quiet", "--hard", head])
                self.git(["git", "update-ref", "refs/remotes/origin/feature", tracking])
                with self.assertRaisesRegex(
                    ValueError, "push, update the checkout"
                ) as caught:
                    self.request(self.pull(local))
                self.assertIn(head, str(caught.exception))
                self.assertIn(tracking, str(caught.exception))
                self.assertEqual((self.calls, self.sleeps), ([], 0))

    def test_a_same_named_branch_of_another_repository_reads_github(self) -> None:
        for url in (
            "git@github.com:example/other.git",
            "https://mirror-github.com/example/fork.git",
        ):
            with self.subTest(url=url):
                shutil.rmtree(self.directory / "checkout", ignore_errors=True)
                local, previous = self.checkout(url=url)
                path, calls, _, sleeps = self.request(self.pull(previous))
                self.assertEqual(sleeps, 0)
                self.assertIn(f"head={previous}", calls[0])

    def test_remote_urls_name_a_github_repository_only_on_github(self) -> None:
        cases = {
            "https://github.com/example/fork.git": "example/fork",
            "https://token@github.com/example/fork": "example/fork",
            "ssh://git@github.com:22/example/fork.git": "example/fork",
            "ssh://git@github.com/example/fork.git": "example/fork",
            "git@github.com:example/fork.git": "example/fork",
            "github.com:example/fork": "example/fork",
            "https://mirror-github.com/example/fork.git": None,
            "https://mirror.example?x@github.com/example/fork.git": None,
            "https://github.com.example/example/fork.git": None,
            "git@github.com:/example/fork.git": None,
            "https://github.com/example/fork/extra": None,
            "/srv/git/fork.git": None,
        }
        for url, expected in cases.items():
            with self.subTest(url=url):
                self.assertEqual(ci.github_repository(url), expected)

    def test_an_upstream_on_another_remote_does_not_name_the_branch(self) -> None:
        local, previous = self.checkout(branch="work")
        self.git(
            ["git", "remote", "add", "fork", "https://github.com/example/fork.git"]
        )
        self.git(["git", "update-ref", "refs/remotes/fork/feature", local])
        self.git(["git", "config", "branch.work.remote", "fork"])
        self.git(["git", "config", "branch.work.merge", "refs/heads/feature"])
        path, calls, output, sleeps = self.request(self.pull(previous))
        self.assertEqual(sleeps, 0)
        self.assertIn(f"head={previous}", calls[0])
        # On origin the same upstream names the PR's branch.
        self.git(["git", "config", "branch.work.remote", "origin"])
        self.git(["git", "update-ref", "refs/remotes/origin/feature", local])
        path, calls, output, sleeps = self.request(self.pull(local))
        self.assertIn(f"Requesting for this checkout's HEAD {local}", output)

    def test_the_checkout_pin_is_not_retried_on_another_head(self) -> None:
        local, previous = self.checkout()
        pending = self.pull(local, mergeable=None, merge_commit_sha=None)
        with self.assertRaisesRegex(
            ValueError, f"checkout's HEAD {local} was the head"
        ):
            self.request(pending, self.pull(previous))
        self.assertEqual(self.calls, [])

    def test_an_explicit_head_wins_over_the_checkout(self) -> None:
        local, previous = self.checkout()
        path, calls, output, sleeps = self.request(self.pull(previous), head=previous)
        self.assertEqual(sleeps, 0)
        self.assertIn(f"head={previous}", calls[0])
        self.assertNotIn("this checkout", output)

    def test_request_with_wait_prints_no_separate_wait_command(self) -> None:
        _, _, output, _ = self.request(PULL, wait=True)
        self.assertNotIn("Wait for it", output)

    def record(self, name: str, **fields: object) -> Path:
        record = {
            "schema_version": 1,
            "repository": "example/fork",
            "pr": 170,
            "head": HEAD,
            "profile": "native",
            "request": f"{name}-{HEAD}",
            "bootstrap": False,
            "created_at": "2026-11-12T01:00:00Z",
            **fields,
        }
        path = ci.REQUESTS / f"{name}.json"
        ci.write_request(path, record)
        return path

    def wait_pr(
        self, *extra: str, runs: list[list[ci_log.Run]] | None = None
    ) -> tuple[int, Path | None, str, list[tuple[str, ...]]]:
        """Run `ci.py wait 170 ...`, recording the request file wait was given."""
        calls: list[tuple[str, ...]] = []

        def gh(*args: str) -> str:
            calls.append(args)
            return json.dumps([{"workflow_runs": page} for page in runs or []])

        pull = dict(PULL, number=170, created_at="2026-11-11T00:00:00Z")
        argv = ["ci.py", "wait", "170", "--repo", "example/fork", *extra]
        with (
            patch.object(ci, "api", return_value=pull),
            patch.object(ci, "gh", side_effect=gh),
            patch.object(ci, "wait") as waited,
            patch("sys.argv", argv),
            patch("builtins.print") as printed,
            patch("sys.stderr") as stderr,
        ):
            code = ci.main()
        output = "\n".join(str(c.args[0]) for c in printed.call_args_list)
        output += "".join(str(c.args[0]) for c in stderr.write.call_args_list)
        path = waited.call_args.args[0] if waited.called else None
        return code, path, output, calls

    def test_wait_on_a_pr_takes_the_newest_local_record_at_head(self) -> None:
        self.record("old", created_at="2026-11-12T00:00:00Z")
        newest = self.record("new", run_url=RUN)
        self.record("moved", head=PREVIOUS, created_at="2026-11-12T09:00:00Z")
        self.record("label", bootstrap=True, created_at="2026-11-12T09:00:00Z")
        code, path, output, calls = self.wait_pr()
        self.assertEqual((code, path), (0, newest))
        self.assertIn(f"newest native request for PR #170 at {HEAD}", output)
        self.assertIn(f"(from local record): {RUN}", output)
        self.assertEqual(len(calls), 1)

    def test_wait_on_a_pr_finds_a_remote_request_through_every_page(self) -> None:
        older = requested("native", "c" * 32, "2026-11-12T01:00:00Z")
        newer = requested("native", "e" * 32, "2026-11-12T03:00:00Z")
        foreign = [newer.copy() for _ in range(3)]
        foreign[0]["display_title"] = f"CI native PR 171 x-{HEAD}"
        foreign[1]["display_title"] = f"CI native PR 170 x-{PREVIOUS}"
        foreign[2]["head_branch"] = "untrusted"
        code, path, output, calls = self.wait_pr(runs=[[older, *foreign], [newer]])
        self.assertEqual(code, 0)
        self.assertEqual(path, ci.REQUESTS / f"{'e' * 32}-{HEAD}.json")
        record = json.loads(cast(Path, path).read_text())
        self.assertEqual((record["run_id"], record["profile"]), (3, "native"))
        self.assertIn("(from GitHub)", output)
        ((query),) = calls
        self.assertIn("--paginate", query)
        self.assertIn("created=2026-11-11T00%3A00%3A00Z..", query[-1])

    def test_wait_on_a_pr_chooses_among_local_and_remote_requests(self) -> None:
        local = self.record("c" * 32, created_at="2026-11-12T01:00:00Z")
        remote = requested("native", "e" * 32, "2026-11-12T02:00:00Z")
        # The local request is on GitHub too: one request, not two.
        listed = requested("native", "c" * 32, "2026-11-12T01:00:00Z")
        code, path, output, calls = self.wait_pr(runs=[[listed, remote]])
        self.assertEqual(code, 0)
        self.assertEqual(path, ci.REQUESTS / f"{'e' * 32}-{HEAD}.json")
        self.assertIn("(from GitHub)", output)
        self.assertEqual(len(calls), 1)
        # A newer local request GitHub does not list yet is still the newest.
        newest = self.record("f" * 32, created_at="2026-11-12T03:00:00Z")
        code, path, output, _ = self.wait_pr(runs=[[listed, remote]])
        self.assertEqual((code, path), (0, newest))
        self.assertIn("(from local record)", output)
        # With a newer remote request of another profile, that one is followed
        # and the other is named with the command that waits on it.
        rust = requested("rust", "a" * 32, "2026-11-12T04:00:00Z")
        code, path, output, _ = self.wait_pr(runs=[[listed, rust]])
        self.assertEqual((code, path), (0, ci.REQUESTS / f"{'a' * 32}-{HEAD}.json"))
        self.assertIn("also has a native request", output)
        self.assertIn("wait 170 --profile native", output)
        self.assertTrue(local.exists())

    def test_wait_on_a_pr_with_several_profiles_follows_the_newest(self) -> None:
        native = self.record("native")
        rust = self.record("rust", profile="rust", created_at="2026-11-12T05:00:00Z")
        code, path, output, _ = self.wait_pr()
        self.assertEqual((code, path), (0, rust))
        self.assertIn("also has a native request", output)
        self.assertIn("wait 170 --profile native", output)
        code, path, _, _ = self.wait_pr("--profile", "native")
        self.assertEqual((code, path), (0, native))

    def test_wait_on_a_pr_without_a_request_names_the_request_command(self) -> None:
        self.record("native")
        code, _, output, calls = self.wait_pr("--profile", "full")
        self.assertEqual(code, 1)
        self.assertIn("request 170 --profile full --wait", output)
        self.assertEqual(len(calls), 1)

    def test_wait_on_a_file_rejects_pr_options(self) -> None:
        path = self.record("native")
        with (
            patch("sys.argv", ["ci.py", "wait", str(path), "--profile", "rust"]),
            patch.object(ci, "wait") as waited,
            patch("sys.stderr"),
        ):
            self.assertEqual(ci.main(), 1)
        waited.assert_not_called()


def gateway_error(*args: str) -> subprocess.CalledProcessError:
    return subprocess.CalledProcessError(
        1, ["gh", *args], stderr="gh: HTTP 502: Bad Gateway (https://api.github.com/)"
    )


class RetryTests(unittest.TestCase):
    """Reads survive a gateway error; nothing that writes is repeated (#1013)."""

    def run_gh(
        self, failures: int, *args: str
    ) -> tuple[str | None, list[tuple[str, ...]], Exception | None]:
        calls: list[tuple[str, ...]] = []

        def flaky(*arguments: str, **_: object) -> str:
            calls.append(arguments)
            if len(calls) <= failures:
                raise gateway_error(*arguments)
            return "answer\n"

        with (
            patch.object(ci.vcs, "gh", side_effect=flaky),
            patch.object(ci.time, "sleep"),
            patch("sys.stderr"),
        ):
            try:
                return ci.gh(*args), calls, None
            except subprocess.CalledProcessError as error:
                return None, calls, error

    def test_a_read_that_fails_once_with_a_502_succeeds(self) -> None:
        queue = ("api", "graphql", "-f", f"query={ci.QUEUE_STATE}", "-F", "n=1")
        for read in (
            ("api", "repos/x/y/pulls/1"),
            ("api", "--paginate", "--slurp", "repos/x/y/actions/runs"),
            ("variable", "list", "--repo", "x/y"),
            ("repo", "view", "--json", "nameWithOwner"),
            queue,
            ("api", "graphql", "-f", f"query={ci.BASE_QUEUE}", "-F", "branch=b"),
        ):
            with self.subTest(read=read[:2]):
                answer, calls, _ = self.run_gh(1, *read)
                self.assertEqual((answer, len(calls)), ("answer", 2))
                _, calls, error = self.run_gh(5, *read)
                self.assertEqual(len(calls), ci.READ_ATTEMPTS)
                self.assertIsNotNone(error)

    def branch_queue(self, queue: object) -> tuple[ci.BranchQueue, tuple[str, ...]]:
        answer = json.dumps({"data": {"repository": {"mergeQueue": queue}}})
        with patch.object(ci, "gh", return_value=answer) as gh:
            read = ci.branch_queue("owner/name", "rust-migration")
        (arguments,) = (call.args for call in gh.call_args_list)
        return read, arguments

    def test_an_empty_base_queue_reads_as_empty(self) -> None:
        read, arguments = self.branch_queue({"entries": {"totalCount": 0, "nodes": []}})
        self.assertEqual(read, ci.BranchQueue((), 0))
        self.assertTrue(ci.read_only(arguments))
        self.assertEqual(arguments[2:4], ("--hostname", "github.com"))
        for value in ("owner=owner", "name=name", "branch=rust-migration"):
            # -f: a name such as 2048 must stay a String, not become an Int.
            self.assertEqual(arguments[arguments.index(value) - 1], "-f")

    def test_a_queued_base_names_each_pr_and_counts_past_the_page(self) -> None:
        nodes = [
            {"state": "QUEUED", "position": 1, "pullRequest": {"number": 1066}},
            {"state": "AWAITING_CHECKS", "position": 2, "pullRequest": {"number": 7}},
        ]
        read, _ = self.branch_queue({"entries": {"totalCount": 14, "nodes": nodes}})
        self.assertEqual(read.total, 14)
        self.assertEqual(
            read.entries,
            (
                {"state": "QUEUED", "position": 1, "number": 1066},
                {"state": "AWAITING_CHECKS", "position": 2, "number": 7},
            ),
        )
        self.assertEqual(
            ci.entry_text(read.entries[1]), "queued awaiting_checks at position 2"
        )

    def test_a_missing_base_queue_is_a_failed_read(self) -> None:
        node = {"state": "QUEUED", "position": 1, "pullRequest": None}
        for queue in (
            None,
            {"entries": None},
            {"entries": {"totalCount": 1, "nodes": None}},
            {"entries": {"totalCount": 1, "nodes": [None]}},
            {"entries": {"totalCount": 1, "nodes": [node]}},
            {"entries": {"totalCount": None, "nodes": []}},
        ):
            with (
                self.subTest(queue=queue),
                self.assertRaisesRegex(ci.QueueUnread, "owner/name rust-migration"),
            ):
                self.branch_queue(queue)

    def test_an_ordinary_number_is_no_gateway_error(self) -> None:
        for text, expected in (
            ("gh: HTTP 502: Bad Gateway (https://api.github.com/graphql)", True),
            ("HTTP 504: Gateway Timeout", True),
            ("error: 503 Service Unavailable", True),
            ("gh: Pull request #502 is already merged", False),
            ("run 503 not found (HTTP 404)", False),
            ("gh: Bad credentials (HTTP 401)", True),
            ("gh: Requires authentication (HTTP 401)", False),
            ("gh: Not Found (HTTP 404)", False),
        ):
            error = subprocess.CalledProcessError(1, "gh", stderr=text)
            self.assertEqual(ci.transient(error), expected, text)

    def test_a_read_answered_bad_credentials_twice_returns_the_third_answer(
        self,
    ) -> None:
        calls: list[tuple[str, ...]] = []

        def unauthorized(*arguments: str, **_: object) -> str:
            calls.append(arguments)
            if len(calls) <= 2 or arguments[1] == "-X":
                raise subprocess.CalledProcessError(
                    1, ["gh", *arguments], stderr="gh: Bad credentials (HTTP 401)"
                )
            return "answer\n"

        with (
            patch.object(ci.vcs, "gh", side_effect=unauthorized),
            patch.object(ci.time, "sleep"),
            patch("sys.stderr"),
        ):
            self.assertEqual(ci.gh("api", "repos/x/y/pulls/1"), "answer")
            self.assertEqual(len(calls), 3)
            calls.clear()
            with self.assertRaises(subprocess.CalledProcessError):
                ci.gh("api", "-X", "PATCH", "repos/x/y/issues/comments/1")
            self.assertEqual(len(calls), 1)

    def test_writes_and_other_errors_are_not_repeated(self) -> None:
        for write in (
            ("workflow", "run", "ci-request.yml", "-f", "pr=1"),
            ("pr", "merge", "1", "--merge"),
            ("run", "rerun", "1", "--failed"),
            ("variable", "set", "CI_MACOS_HEAVY", "--body", "x"),
            ("api", "-X", "PATCH", "repos/x/y/issues/comments/1"),
            ("api", "repos/x/y/issues/1/comments", "-f", "body=x"),
            ("api", "graphql", "-f", "query=mutation { x }"),
        ):
            with self.subTest(write=write[:2]):
                _, calls, error = self.run_gh(1, *write)
                self.assertEqual(len(calls), 1)
                self.assertIsNotNone(error)
        with (
            patch.object(
                ci.vcs,
                "gh",
                side_effect=subprocess.CalledProcessError(1, "gh", stderr="HTTP 404"),
            ) as called,
            self.assertRaises(subprocess.CalledProcessError),
        ):
            ci.gh("api", "repos/x/y/pulls/1")
        self.assertEqual(called.call_count, 1)

    def enqueue_after(
        self, merge_failures: int, queued_after_failure: bool, state_failures: int
    ) -> tuple[str, int]:
        """enqueue with gh's merge and queue-state calls failing as given."""
        merges = 0
        states = 0
        entry = {"state": "QUEUED", "position": 1}

        def fake(*args: str, **_: object) -> str:
            nonlocal merges, states
            if args[:2] == ("pr", "merge"):
                merges += 1
                if merges <= merge_failures:
                    raise gateway_error(*args)
                return ""
            states += 1
            if states <= state_failures:
                raise gateway_error(*args)
            queued = merges > merge_failures or queued_after_failure
            pull = {
                "mergeQueueEntry": entry if queued else None,
                "autoMergeRequest": None,
            }
            return json.dumps({"data": {"repository": {"pullRequest": pull}}})

        args = argparse.Namespace(repo="example/fork", pr=170, retry=None)
        with (
            patch.object(ci.vcs, "gh", side_effect=fake),
            patch.object(ci, "api", return_value=PULL),
            patch.object(ci, "enqueue_checks", return_value=HEAD),
            patch.object(ci, "requeue_gate"),
            patch.object(ci.time, "sleep"),
            patch("sys.stderr"),
            patch("builtins.print"),
        ):
            return ci.enqueue(args), merges

    def test_enqueue_survives_a_502_on_the_merge_or_the_queue_read(self) -> None:
        # The merge took effect though its answer was lost: no second merge.
        self.assertEqual(self.enqueue_after(1, True, 0), ("queued at position 1", 1))
        # It did not: one more merge.
        self.assertEqual(self.enqueue_after(1, False, 0), ("queued at position 1", 2))
        # The merge answered; the queue-state read after it is retried.
        self.assertEqual(self.enqueue_after(0, False, 1), ("queued at position 1", 1))


def job_row(job_id: int, name: str, conclusion: str | None) -> ci_log.Job:
    return {
        "id": job_id,
        "run_id": 123,
        "name": name,
        "status": "completed" if conclusion else "in_progress",
        "conclusion": conclusion,
        "html_url": f"https://github.com/example/fork/actions/runs/123/job/{job_id}",
    }


class RunWaitTests(unittest.TestCase):
    """wait --run, --complete and the Quick waits, on a fake clock (#1013)."""

    def wait_run(
        self,
        argv: list[str],
        reads: list[tuple[str, str | None, list[ci_log.Job]]],
        latest: int | None = 1,
    ) -> tuple[int, str, list[str]]:
        """`reads` are successive (status, conclusion, jobs) of the attempt."""
        steps = iter(reads)
        paths: list[str] = []
        current: dict[str, Any] = {}

        def api(_repository: str, path: str) -> dict[str, Any]:
            paths.append(path)
            if path == "actions/runs/123":
                if latest is None:  # The latest-attempt lookup fails.
                    raise subprocess.CalledProcessError(1, "gh", stderr="HTTP 502")
                return {"run_attempt": latest}
            status, conclusion, jobs = next(steps, reads[-1])
            current["jobs"] = jobs
            return {"status": status, "conclusion": conclusion}

        def jobs(_repository: str, _run: int, attempt: int | None) -> list[object]:
            paths.append(f"jobs of attempt {attempt}")
            return current["jobs"]

        clock = itertools.count(0, 10)
        with (
            patch.object(ci, "api", side_effect=api),
            patch.object(ci, "run_jobs", side_effect=jobs),
            patch.object(ci, "failure_logs") as self.logs,
            patch.object(ci.time, "monotonic", side_effect=lambda: next(clock)),
            patch.object(ci.time, "sleep"),
            patch("sys.argv", ["ci.py", "wait", *argv, "--repo", "example/fork"]),
            patch("builtins.print") as printed,
            patch("sys.stderr") as stderr,
        ):
            code = ci.main()
        output = "\n".join(str(c.args[0]) for c in printed.call_args_list)
        output += "".join(str(c.args[0]) for c in stderr.write.call_args_list)
        return code, output, paths

    FAILED = job_row(1, "native / build", "failure")
    RUNNING = job_row(2, "platform / Windows", None)
    PASSED = job_row(2, "platform / Windows", "success")

    def test_complete_outlives_a_failed_job_and_exits_1_at_completion(self) -> None:
        reads: list[tuple[str, str | None, list[ci_log.Job]]] = [
            ("in_progress", None, [self.FAILED, self.RUNNING]),
            ("in_progress", None, [self.FAILED, self.RUNNING]),
            ("completed", "failure", [self.FAILED, self.PASSED]),
        ]
        code, output, paths = self.wait_run(["--run", "123", "--complete"], reads)
        self.assertEqual(code, 1)
        self.assertEqual(output.count("failed job: native / build"), 1)
        self.assertIn("gh run rerun 123 --failed", output)
        self.assertEqual(paths.count("actions/runs/123/attempts/1"), 3)
        # Without --complete the first failed job ends the wait.
        code, output, paths = self.wait_run(["--run", "123"], reads)
        self.assertEqual(code, 1)
        self.assertEqual(paths.count("actions/runs/123/attempts/1"), 1)
        self.assertIn("wait with --complete", output)
        # A passing run exits 0.
        code, _, _ = self.wait_run(["--run", RUN], [("completed", "success", [])])
        self.assertEqual(code, 0)

    def test_an_attempt_url_pins_that_attempt_and_resumes_on_it(self) -> None:
        url = RUN + "/attempts/1"
        code, output, paths = self.wait_run(
            ["--run", url, "--timeout", "30"],
            [("in_progress", None, [self.RUNNING])],
            latest=2,
        )
        self.assertEqual(code, 2)
        self.assertNotIn("actions/runs/123", paths)
        self.assertTrue(all("attempts/2" not in p for p in paths))
        self.assertIn(f"wait --run {url}", output)
        # Without one, the latest attempt read at the start is pinned.
        code, output, paths = self.wait_run(
            ["--run", "123", "--complete", "--timeout", "30"],
            [("in_progress", None, [self.RUNNING])],
            latest=2,
        )
        self.assertEqual(code, 2)
        self.assertIn("jobs of attempt 2", paths)
        self.assertIn(f"wait --run {RUN}/attempts/2 --complete", output)

    def test_a_pinned_attempts_failure_summary_reads_that_attempt(self) -> None:
        failed: list[tuple[str, str | None, list[ci_log.Job]]] = [
            ("completed", "failure", [self.FAILED])
        ]
        code, output, _ = self.wait_run(["--run", RUN + "/attempts/1"], failed, 2)
        self.assertEqual(code, 1)
        self.assertEqual(self.logs.call_args.args, ("example/fork", 123, 1))
        code, output, _ = self.wait_run(
            ["--run", RUN + "/attempts/1", "--complete"], failed, 2
        )
        self.assertEqual(self.logs.call_args.args, ("example/fork", 123, 1))
        self.assertIn("reruns attempt 2's failed jobs, not attempt 1's", output)

    def test_a_failed_attempt_lookup_keeps_the_pinned_summary(self) -> None:
        failed: list[tuple[str, str | None, list[ci_log.Job]]] = [
            ("completed", "failure", [self.FAILED])
        ]
        code, output, _ = self.wait_run(
            ["--run", RUN + "/attempts/1", "--complete"], failed, None
        )
        self.assertEqual(code, 1)
        self.assertIn("gh run rerun 123 --failed", output)
        self.assertEqual(self.logs.call_args.args, ("example/fork", 123, 1))

    def test_a_bare_number_is_still_a_pr(self) -> None:
        with (
            patch.object(ci, "resolve_request", return_value=Path("r.json")) as found,
            patch.object(ci, "wait") as waited,
            patch("sys.argv", ["ci.py", "wait", "123", "--repo", "example/fork"]),
        ):
            self.assertEqual(ci.main(), 0)
        self.assertEqual(found.call_args.args[1:], (123, None))
        self.assertEqual(waited.call_args.args[0], Path("r.json"))

    def quick(
        self,
        argv: list[str],
        checks: list[dict[str, object] | None],
        draft: bool = True,
        heads: list[str] | None = None,
        runs: list[ci_log.Run] | None = None,
    ) -> tuple[int, str, list[tuple[str, ...]]]:
        """Run argv against successive Quick check runs of the PR's head; the PR
        is a draft until marked ready, its head follows `heads`, and `runs` are
        its pull_request runs of ci-quick.yml."""
        reads = iter(checks)
        moves = iter(heads or [])
        calls: list[tuple[str, ...]] = []

        def api(_repository: str, path: str) -> dict[str, Any]:
            if path == "pulls/170":
                marked = any(call[:2] == ("pr", "ready") for call in calls)
                head = next(moves, (heads or [HEAD])[-1])
                return dict(PULL, draft=draft and not marked, head={"sha": head})
            if "actions/workflows/ci-quick.yml/runs" in path:
                return {"workflow_runs": runs or []}
            check = next(reads, checks[-1])
            return {"check_runs": [check] if check else []}

        clock = itertools.count(0, 10)
        with (
            patch.object(ci, "api", side_effect=api),
            patch.object(ci, "gh", side_effect=lambda *a: calls.append(a) or ""),
            patch.object(ci, "require_stamp"),
            patch.object(ci, "failure_logs"),
            patch.object(ci.time, "monotonic", side_effect=lambda: next(clock)),
            patch.object(ci.time, "sleep"),
            patch("sys.argv", ["ci.py", *argv, "--repo", "example/fork"]),
            patch("builtins.print") as printed,
            patch("sys.stderr") as stderr,
        ):
            code = ci.main()
        output = "\n".join(str(c.args[0]) for c in printed.call_args_list)
        output += "".join(str(c.args[0]) for c in stderr.write.call_args_list)
        return code, output, calls

    def check(self, status: str, conclusion: str | None) -> dict[str, object]:
        return {
            "status": status,
            "conclusion": conclusion,
            "started_at": "2026-11-12T01:00:00Z",
            "html_url": "https://github.com/example/fork/runs/7",
            "details_url": RUN + "/job/7",
        }

    def test_ready_wait_and_wait_quick_return_0_1_and_2(self) -> None:
        skipped = dict(self.check("completed", "skipped"), started_at="")
        running = self.check("in_progress", None)
        for outcome, code in (("success", 0), ("failure", 1)):
            done = self.check("completed", outcome)
            with self.subTest(outcome=outcome):
                # Just after the ready mark only the draft's skip exists.
                status, _, calls = self.quick(
                    ["ready", "170", "--wait"], [skipped, None, running, done]
                )
                self.assertEqual(status, code)
                self.assertEqual(
                    calls, [("pr", "ready", "170", "--repo", "example/fork")]
                )
                status, _, _ = self.quick(["wait", "170", "--quick"], [running, done])
                self.assertEqual(status, code)
        status, output, _ = self.quick(
            ["wait", "170", "--quick", "--timeout", "30"], [running]
        )
        self.assertEqual(status, 2)
        self.assertIn("resume with: python3 tools/ci.py wait 170 --quick", output)
        status, output, _ = self.quick(
            ["ready", "170", "--wait", "--timeout", "30"], [skipped]
        )
        self.assertEqual(status, 2)
        # A skip alone is no verdict: on a draft wait --quick says to mark the
        # PR ready at once; on a ready PR it waits a while for the run first.
        status, output, _ = self.quick(["wait", "170", "--quick"], [skipped])
        self.assertEqual(status, 1)
        self.assertIn("ready 170 --wait", output)
        done = self.check("completed", "success")
        status, _, _ = self.quick(
            ["wait", "170", "--quick"], [skipped, skipped, done], draft=False
        )
        self.assertEqual(status, 0)
        status, output, _ = self.quick(
            ["wait", "170", "--quick"], [skipped], draft=False
        )
        self.assertEqual(status, 1)
        self.assertIn("ready 170 --wait", output)
        # A caller's timeout shorter than the grace is a timeout, exit 2.
        status, output, _ = self.quick(
            ["wait", "170", "--quick", "--timeout", "5"], [skipped], draft=False
        )
        self.assertEqual(status, 2)
        self.assertIn("wait 170 --quick", output)

    def test_wait_quick_waits_out_a_run_whose_check_has_not_appeared(self) -> None:
        """On a ready PR with only the draft's skip, an unfinished Quick run is
        waited for past MERGEABLE_WAIT (#1063)."""
        skipped = dict(self.check("completed", "skipped"), started_at="")
        done = self.check("completed", "success")
        # The clock moves 10 seconds a read: 40 skipped reads pass the grace.
        reads = 2 + ci.MERGEABLE_WAIT // 10
        status, output, _ = self.quick(
            ["wait", "170", "--quick"],
            [*[skipped] * reads, done],
            draft=False,
            runs=[QUICK_RUNNING],
        )
        self.assertEqual(status, 0)
        self.assertIn(f"in_progress: {QUICK_RUNNING['html_url']}", output)
        self.assertNotIn("only skipped", output)
        # With no unfinished run the skip is a draft's, refused as before.
        status, output, _ = self.quick(
            ["wait", "170", "--quick"],
            [*[skipped] * reads, done],
            draft=False,
            runs=[{**QUICK_RUNNING, "status": "completed"}],
        )
        self.assertEqual(status, 1)
        self.assertIn("ready 170 --wait", output)

    def test_ready_wait_follows_the_head_it_marked(self) -> None:
        # A push lands between the ready mark and the wait: the wait stops.
        done = self.check("completed", "success")
        status, output, _ = self.quick(
            ["ready", "170", "--wait"], [done], heads=[HEAD, PREVIOUS]
        )
        self.assertEqual(status, 1)
        self.assertIn("PR head changed", output)


class QueueWaitTests(unittest.TestCase):
    """Recorded gh responses for each way a PR leaves, or stays in, the queue."""

    QUEUED = {
        "state": "OPEN",
        "mergeCommit": None,
        "mergeQueueEntry": {"state": "AWAITING_CHECKS", "position": 1},
        "autoMergeRequest": None,
    }
    MERGED = {
        "state": "MERGED",
        "mergeCommit": {"oid": MERGE},
        "mergeQueueEntry": None,
        "autoMergeRequest": None,
    }
    REMOVED = {
        "state": "OPEN",
        "mergeCommit": None,
        "mergeQueueEntry": None,
        "autoMergeRequest": None,
    }
    RUNS = {
        "workflow_runs": [
            {
                "id": 7,
                "head_branch": f"gh-readonly-queue/rust-migration/pr-170-{BASE}",
                "created_at": "2026-11-11T01:00:00Z",
                "status": "completed",
                "conclusion": "failure",
                "html_url": "https://github.com/example/fork/actions/runs/7",
            },
            {
                "id": 6,
                "head_branch": f"gh-readonly-queue/rust-migration/pr-170-{PREVIOUS}",
                "created_at": "2026-11-11T00:00:00Z",
                "status": "completed",
                "conclusion": "success",
                "html_url": "https://github.com/example/fork/actions/runs/6",
            },
            {
                "id": 5,
                "head_branch": f"gh-readonly-queue/rust-migration/pr-1700-{BASE}",
                "created_at": "2026-11-11T02:00:00Z",
                "status": "completed",
                "conclusion": "failure",
                "html_url": "https://github.com/example/fork/actions/runs/5",
            },
        ]
    }
    JOBS = [
        {
            "jobs": [
                {"name": "quick / Quick validation", "conclusion": "success"},
                {
                    "name": "native / simulation (1)",
                    "conclusion": "failure",
                    "html_url": "https://github.com/example/fork/actions/runs/7/job/9",
                },
            ]
        }
    ]

    def run_main(
        self, states: list[dict[str, Any]], *extra: str, argv: list[str] | None = None
    ) -> tuple[int, str, list[tuple[str, ...]]]:
        answers = iter(states)
        calls: list[tuple[str, ...]] = []

        def answer(*args: str) -> str:
            calls.append(args)
            if args[:2] == ("api", "graphql"):
                return json.dumps(
                    {"data": {"repository": {"pullRequest": next(answers)}}}
                )
            if "runs?event=merge_group" in args[1]:
                return json.dumps(self.RUNS)
            if "/runs/7/jobs" in args[-1]:
                return json.dumps(self.JOBS)
            raise AssertionError(args)

        clock = itertools.count(0, 15)
        argv = argv or ["ci.py", "queue-wait", "170", "--repo", "example/fork", *extra]
        with (
            patch.object(ci, "gh", side_effect=answer),
            patch.object(ci.time, "monotonic", side_effect=lambda: next(clock)),
            patch.object(ci.time, "sleep") as sleep,
            patch("sys.argv", argv),
            patch("builtins.print") as printed,
        ):
            code = ci.main()
        output = "\n".join(str(call.args[0]) for call in printed.call_args_list)
        self.assertTrue(all(call.args[0] <= 15 for call in sleep.call_args_list))
        return code, output, calls

    def test_merged_prints_merge_commit(self) -> None:
        code, output, calls = self.run_main([self.QUEUED, self.QUEUED, self.MERGED])
        self.assertEqual(code, 0)
        self.assertEqual(
            output,
            f"queued awaiting_checks at position 1\nPR #170 merged: {MERGE}",
        )
        query = calls[0]
        self.assertIn("number=170", query)
        self.assertIn("owner=example", query)
        self.assertIn("name=fork", query)

    def test_removed_prints_failed_run_and_jobs(self) -> None:
        code, output, _ = self.run_main([self.QUEUED, self.REMOVED, self.REMOVED])
        self.assertEqual(code, 3)
        # The recorded answers hold no run or log: the summary says so in one
        # line and the removal's exit status stands (P2).
        self.assertIn("Could not read the failed jobs' logs", output.splitlines()[-1])
        self.assertIn("python3 tools/ci.py log 7 --repo example/fork", output)
        self.assertEqual(
            output.splitlines()[1:-1],
            [
                "leaving the merge queue",
                "PR #170 left the merge queue (open); merge-group run "
                "completed/failure: https://github.com/example/fork/actions/runs/7",
                "failed job: native / simulation (1) "
                "(https://github.com/example/fork/actions/runs/7/job/9)",
            ],
        )
        closed = dict(self.REMOVED, state="CLOSED")
        code, output, _ = self.run_main([closed, closed])
        self.assertEqual(code, 3)
        self.assertIn("left the merge queue (closed)", output)

    def test_open_read_between_leaving_the_queue_and_merged_keeps_waiting(
        self,
    ) -> None:
        # #938: GitHub reads a merging PR as open with no entry for a moment.
        code, output, _ = self.run_main([self.QUEUED, self.REMOVED, self.MERGED])
        self.assertEqual(code, 0)
        self.assertEqual(
            output.splitlines(),
            [
                "queued awaiting_checks at position 1",
                "leaving the merge queue",
                f"PR #170 merged: {MERGE}",
            ],
        )

    def test_queued_entry_names_its_state_once(self) -> None:
        # #938: an entry in state QUEUED printed "queued queued".
        entry = {"state": "QUEUED", "position": 2}
        queued = dict(self.QUEUED, mergeQueueEntry=entry)
        _, output, _ = self.run_main([queued, self.MERGED])
        self.assertEqual(output.splitlines()[0], "queued at position 2")

    def test_pending_auto_merge_keeps_waiting(self) -> None:
        pending = dict(self.REMOVED, autoMergeRequest={"enabledAt": "date"})
        code, output, _ = self.run_main([pending, self.MERGED])
        self.assertEqual(code, 0)
        self.assertTrue(output.startswith("awaiting entry to the merge queue\n"))

    def test_still_queued_times_out_with_resume_command(self) -> None:
        code, output, _ = self.run_main([self.QUEUED] * 5, "--timeout", "30")
        self.assertEqual(code, 2)
        prefix = "resume with: python3 tools/ci.py "
        last = output.splitlines()[-1]
        self.assertEqual(
            last,
            "PR #170 still queued after 30 seconds; "
            + prefix
            + "queue-wait 170 --repo example/fork",
        )
        # The printed command resumes on the same repository.
        resume = ["ci.py", *last.split(prefix, 1)[1].split()]
        code, _, calls = self.run_main([self.MERGED], argv=resume)
        self.assertEqual(code, 0)
        self.assertIn("owner=example", calls[0])
        self.assertIn("name=fork", calls[0])


@dataclass
class EnqueueWorld:
    """Recorded answers for enqueue on PR 170: quick (its Quick check run),
    requested (its one dispatched run) or dispatched (several) or pages (pages of
    them), labeled (label runs), queue (merge-group runs), ejected (the head they
    tested), behind and base_files (the base's lead), commits and total (the
    comparison), files (the PR's), recorded and recorded_jobs (the run a request
    record names), retry, entry and armed (the queue's answer), records and
    comments (cleanup records and the discussion)."""

    quick: ci.CheckRun | None = field(default_factory=lambda: EnqueueTests.QUICK)
    # The head's pull_request runs of ci-quick.yml.
    quick_runs: list[ci_log.Run] = field(default_factory=list)
    requested: ci_log.Run | None = field(default_factory=lambda: EnqueueTests.REQUESTED)
    dispatched: list[ci_log.Run] | None = None
    pages: list[list[ci_log.Run]] | None = None
    labeled: list[ci_log.Run] = field(default_factory=list)
    queue: list[ci.QueueRun] = field(default_factory=list)
    ejected: str = HEAD
    behind: int = 0
    base_files: list[dict[str, str]] = field(default_factory=list)
    commits: list[str] = field(default_factory=list)
    total: int | None = None
    files: list[dict[str, str]] = field(
        default_factory=lambda: [{"filename": "tools/ci.py"}]
    )
    recorded: ci_log.Run | None = None
    recorded_jobs: list[dict[str, str]] = field(default_factory=list)
    # The run `request --diagnose 3` names.
    named: ci_log.Run | None = None
    retry: str | None = None
    entry: dict[str, object] | None = None
    armed: dict[str, object] | None = None
    records: list[dict[str, Any]] = field(default_factory=list)
    comments: list[dict[str, str]] = field(default_factory=list)


class EnqueueTests(unittest.TestCase):
    """Recorded gh answers for ci.py enqueue's gates (#922)."""

    QUICK: ci.CheckRun = {
        "status": "completed",
        "conclusion": "success",
        "started_at": "2026-11-12T01:00:00Z",
        "html_url": "https://github.com/example/fork/runs/1",
    }
    REQUESTED: ci_log.Run = {
        "id": 2,
        "display_title": f"CI native PR 170 {'c' * 32}-{HEAD}",
        "head_branch": "rust-migration",
        "status": "completed",
        "conclusion": "success",
        "created_at": "2026-11-12T02:00:00Z",
        "html_url": "https://github.com/example/fork/actions/runs/2",
    }
    EJECTED = {
        "head_branch": f"gh-readonly-queue/rust-migration/pr-170-{BASE}",
        "head_sha": MERGE,
        "created_at": "2026-11-12T03:00:00Z",
        "status": "completed",
        "conclusion": "failure",
        "html_url": "https://github.com/example/fork/actions/runs/3",
    }

    def enqueue(self, **options: object) -> tuple[str, list[tuple[str, ...]]]:
        """Run enqueue against an EnqueueWorld of `options`."""
        world = EnqueueWorld(**cast(dict[str, Any], options))
        args = argparse.Namespace(repo="example/fork", pr=170, retry=world.retry)
        return self.within(world, lambda: ci.enqueue(args)), self.calls

    def within(self, world: EnqueueWorld, command: Callable[[], str]) -> str:
        """Run `command` against `world`'s recorded answers."""
        requested = world.requested
        dispatched = world.dispatched
        if dispatched is None:
            dispatched = [requested] if requested else []
        compare = {
            "behind_by": world.behind,
            "commits": [{"sha": sha} for sha in world.commits],
            "total_commits": len(world.commits) if world.total is None else world.total,
        }
        answers: dict[str, object] = {
            "pulls/170": dict(PULL, created_at="2026-11-11T00:00:00Z"),
            "check-runs?check_name=Quick%20validation": {
                "check_runs": [world.quick] if world.quick else []
            },
            "actions/workflows/ci-quick.yml/runs": {"workflow_runs": world.quick_runs},
            "actions/runs/4/jobs": {
                "jobs": [{"name": "plan", "conclusion": "skipped"}]
            },
            "actions/runs/5/jobs": {
                "jobs": [{"name": "plan", "conclusion": "success"}]
            },
            "actions/runs/6/jobs": {"jobs": []},
            "actions/runs/9/jobs": {"jobs": world.recorded_jobs},
            "event=merge_group": {"workflow_runs": world.queue},
            f"commits/{MERGE}": {"parents": [{"sha": PARENT}, {"sha": world.ejected}]},
            f"compare/rust-migration...{HEAD}": compare,
            f"compare/{HEAD}...rust-migration": {"files": world.base_files},
            "actions/runs/9": world.recorded,
            "actions/runs/3": world.named,
        }
        listings: dict[str, object] = {
            "event=workflow_dispatch": [
                {"workflow_runs": page}
                for page in (world.pages if world.pages is not None else [dispatched])
            ],
            "event=pull_request": [{"workflow_runs": world.labeled}],
            "pulls/170/files": [world.files],
        }
        calls: list[tuple[str, ...]] = []
        self.calls = calls

        def api(repository: str, path: str) -> dict:
            # The longest key a path contains answers it: runs/9/jobs, not runs/9.
            key = max((key for key in answers if key in path), key=len)
            return cast(dict, answers[key])

        def gh(*args: str) -> str:
            if args[:2] == ("api", "--paginate"):
                (pages,) = [v for key, v in listings.items() if key in args[-1]]
                return json.dumps(pages)
            calls.append(args)
            state = {
                "mergeQueueEntry": world.entry,
                "autoMergeRequest": world.armed,
            }
            return json.dumps({"data": {"repository": {"pullRequest": state}}})

        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(ci, "api", side_effect=api),
            patch.object(ci, "gh", side_effect=gh),
            patch.object(ci, "added_records", return_value=world.records),
            patch.object(
                ci.cleanup_contract,
                "discussion",
                return_value=[row["body"] for row in world.comments],
            ),
            patch.object(ci, "require_stamp") as stamp,
            patch.object(
                ci.preflight_stamp, "common_local", return_value=Path(directory)
            ),
            patch("builtins.print"),
            patch.object(ci, "REQUESTS", Path(directory)),
        ):
            if world.recorded:
                record = {"pr": 170, "head": HEAD, "run_id": 9}
                (Path(directory) / "old.json").write_text(json.dumps(record))
            self.stamp = stamp
            self.overrides = Path(directory) / "overrides.log"
            return command()

    def test_red_quick_or_requested_run_refuses_before_queueing(self) -> None:
        red = dict(self.QUICK, conclusion="failure")
        for label, options, message in (
            ("red Quick", {"quick": red}, "runs/1"),
            (
                "no Quick",
                {"quick": None},
                # The refusal says what to do in each case (#940).
                r"no run\. .*ci\.py wait 170 --quick\. .*push a commit without \[skip ci\]",
            ),
            (
                "pending Quick",
                {"quick": dict(red, status="in_progress", conclusion=None)},
                "in_progress",
            ),
            # Quick's jobs still run, so its check has not appeared (#1063).
            (
                "running Quick, no check",
                {"quick": None, "quick_runs": [QUICK_RUNNING]},
                f"in_progress/None: {QUICK_RUNNING['html_url']}",
            ),
            (
                "running Quick, a draft's skip",
                {
                    "quick": dict(red, conclusion="skipped"),
                    "quick_runs": [QUICK_RUNNING],
                },
                f"in_progress/None: {QUICK_RUNNING['html_url']}",
            ),
            (
                "red request",
                {"requested": dict(self.REQUESTED, conclusion="failure")},
                "runs/2",
            ),
            (
                "other head",
                {
                    "requested": dict(
                        self.REQUESTED, display_title=f"CI native PR 170 x-{BASE}"
                    )
                },
                "--profile native",
            ),
        ):
            with self.subTest(label):
                with self.assertRaisesRegex(ValueError, message):
                    self.enqueue(**options)
                self.assertEqual(self.calls, [])

    # A merge-group run as GitHub returns it by ID: the failed Full a request names.
    FAILED_FULL = dict(
        EJECTED,
        id=3,
        event="merge_group",
        path=".github/workflows/ci-request.yml",
        display_title="CI full queue refs/heads/gh-readonly-queue/x",
    )

    def first_native(
        self, options: Mapping[str, object], diagnose: str | None = None
    ) -> str:
        """Run request's one-native-run check on an EnqueueWorld of `options`;
        return what it logged."""
        world = EnqueueWorld(**cast(dict[str, Any], dict(options)))

        def check() -> str:
            ci.require_first_native(
                "example/fork", 170, "2026-11-11T00:00:00Z", HEAD, diagnose
            )
            return self.overrides.read_text() if self.overrides.exists() else ""

        return self.within(world, check)

    def test_a_second_native_run_needs_the_failed_full_it_diagnoses(self) -> None:
        """The rule of workflow.md's checks, which only the doc held (#896)."""
        green = self.REQUESTED
        earlier = dict(green, id=1, display_title=f"CI native PR 170 x-{BASE}")
        later = dict(green, id=7, created_at="2026-11-12T05:00:00Z")
        rust = dict(later, display_title=f"CI rust PR 170 r-{HEAD}")
        platform = dict(later, display_title=f"CI platform PR 170 p-{HEAD}")
        workflow = [{"filename": ".github/workflows/ci-quick.yml"}]
        for label, options in (
            ("green at the head", {}),
            (
                "green on an earlier commit",
                {"requested": earlier, "commits": [BASE, HEAD]},
            ),
            ("a later green rust run", {"dispatched": [green, rust]}),
            # A later run of any profile, red or still going, reopens nothing.
            (
                "a red native since",
                {"dispatched": [green, dict(later, conclusion="failure")]},
            ),
            (
                "a red platform run since",
                {"dispatched": [green, dict(platform, conclusion="failure")]},
            ),
            (
                "a platform run still going",
                {
                    "dispatched": [
                        green,
                        dict(platform, status="in_progress", conclusion=None),
                    ]
                },
            ),
            # A removal the request does not name waives nothing.
            ("removed by the queue since", {"queue": [self.EJECTED]}),
            # A workflow change's passing run is already at its head.
            ("workflow files, green at the head", {"files": workflow}),
        ):
            with self.subTest(label):
                with self.assertRaisesRegex(
                    ValueError,
                    r"one `native` run.*runs/2.*enqueue 170.*--diagnose RUN",
                ):
                    self.first_native(options)

    def test_native_runs_again_only_where_the_rule_allows(self) -> None:
        green = self.REQUESTED
        red = dict(green, conclusion="failure")
        rust = dict(green, display_title=f"CI rust PR 170 r-{HEAD}")
        earlier = dict(green, display_title=f"CI native PR 170 x-{BASE}")
        # A checker-only Full runs Quick alone, so no green Full is a native run.
        full = dict(earlier, display_title=f"CI full PR 170 f-{BASE}")
        workflow = [{"filename": ".github/workflows/ci-quick.yml"}]
        for label, options in (
            ("no run yet", {"requested": None}),
            ("only a red native", {"requested": red}),
            ("only a green rust run", {"requested": rust}),
            (
                "a green Full on an earlier commit",
                {"requested": full, "commits": [BASE, HEAD]},
            ),
            # enqueue needs a workflow change's passing run at its head.
            (
                "workflow files, green on an earlier commit",
                {"requested": earlier, "commits": [BASE, HEAD], "files": workflow},
            ),
        ):
            with self.subTest(label):
                self.assertEqual(self.first_native(options), "")
        (entry,) = self.first_native(
            {"named": self.FAILED_FULL}, diagnose="3"
        ).splitlines()
        self.assertEqual(
            {k: v for k, v in json.loads(entry).items() if k in ("kind", "reason")},
            {"kind": "diagnose", "reason": self.EJECTED["html_url"]},
        )
        url = "https://github.com/example/fork/actions/runs/3"
        self.assertIn(url, self.first_native({"named": self.FAILED_FULL}, url))

    def test_diagnose_names_a_failed_full_of_this_pr(self) -> None:
        failed = self.FAILED_FULL
        requested = dict(
            failed,
            event="workflow_dispatch",
            head_branch="rust-migration",
            display_title=f"CI full PR 170 f-{HEAD}",
        )
        for label, named in (
            ("a requested Full", requested),
            ("a bootstrap Full", dict(requested, event="pull_request")),
            ("a cancelled group", dict(failed, conclusion="cancelled")),
        ):
            with self.subTest(label):
                self.assertTrue(self.first_native({"named": named}, "3"))
        other = "gh-readonly-queue/rust-migration/pr-171-" + BASE
        for label, named, message in (
            ("a green Full", dict(failed, conclusion="success"), "did not fail"),
            (
                "a Full still going",
                dict(failed, status="in_progress", conclusion=None),
                "did not fail",
            ),
            ("another PR's group", dict(failed, head_branch=other), "not a Full of"),
            (
                "another PR's request",
                dict(requested, display_title=f"CI full PR 171 f-{HEAD}"),
                "not a Full of",
            ),
            (
                "a failed native",
                dict(requested, display_title=f"CI native PR 170 n-{HEAD}"),
                "not a Full of",
            ),
            (
                "another workflow",
                dict(failed, path=".github/workflows/ci-build.yml"),
                "not a Full of",
            ),
        ):
            with self.subTest(label):
                with self.assertRaisesRegex(ValueError, message):
                    self.first_native({"named": named}, "3")
        with self.assertRaisesRegex(ValueError, "run ID or URL"):
            self.first_native({}, "the queue's run")

    def request_native(
        self, diagnose: str | None, **options: object
    ) -> tuple[list[str], list[tuple[str, ...]]]:
        """Run `request --profile native` itself on an EnqueueWorld of `options`;
        return the request records it wrote and its gh writes."""
        world = EnqueueWorld(**cast(dict[str, Any], options))
        args = argparse.Namespace(
            repo="example/fork",
            pr=170,
            profile="native",
            bootstrap=False,
            unstamped=None,
            diagnose=diagnose,
            head=HEAD,
            wait=False,
            request_file=None,
        )
        records: list[str] = []

        def request() -> str:
            try:
                ci.request(args)
            finally:
                records.extend(path.name for path in ci.REQUESTS.glob("*.json"))
            return ""

        self.within(world, request)
        return records, self.calls

    def test_request_refuses_before_any_record_or_dispatch(self) -> None:
        """The check is the request command's own, not only a function beside it."""
        with self.assertRaisesRegex(ValueError, "one `native` run"):
            self.request_native(None)
        self.assertEqual(self.calls, [])
        # The refusal wrote no request record either.
        records, calls = self.request_native(None, requested=None)
        self.assertEqual(len(records), 1)
        self.assertEqual([call[:2] for call in calls], [("workflow", "run")])
        self.assertIn("profile=native", calls[0])
        records, calls = self.request_native("3", named=self.FAILED_FULL)
        self.assertEqual(len(records), 1)
        self.assertEqual([call[:2] for call in calls], [("workflow", "run")])

    def test_green_head_queues_and_reports_entry_or_armed_auto_merge(self) -> None:
        entry = {"state": "AWAITING_CHECKS", "position": 2}
        reported, calls = self.enqueue(entry=entry)
        self.assertEqual(reported, "queued awaiting_checks at position 2")
        merge = calls[0]
        self.assertEqual(merge[:3], ("pr", "merge", "170"))
        self.assertEqual(merge[merge.index("--match-head-commit") + 1], HEAD)
        reported, _ = self.enqueue(entry={"state": "QUEUED", "position": 1})
        self.assertEqual(reported, "queued at position 1")
        reported, _ = self.enqueue(armed={"enabledAt": "date"})
        self.assertTrue(reported.startswith("auto-merge armed"))
        with self.assertRaisesRegex(ValueError, "neither queued"):
            self.enqueue()

    def test_request_before_922_is_found_through_its_record(self) -> None:
        # Its run name has no head; the newest of the record's run and the named
        # run decides.
        old = dict(self.REQUESTED, id=9, display_title="CI native PR 170 r")
        red = dict(old, conclusion="failure", created_at="2026-11-12T04:00:00Z")
        with self.assertRaisesRegex(ValueError, "runs/2"):
            self.enqueue(recorded=red)
        entry = {"state": "QUEUED", "position": 1}
        reported, _ = self.enqueue(requested=old, recorded=self.REQUESTED, entry=entry)
        self.assertTrue(reported.startswith("queued"))

    def test_only_a_bootstrap_label_run_counts_as_a_request(self) -> None:
        # Every label starts a ci-request.yml run; one whose plan skipped tested
        # nothing and must not displace the passing request.
        later = {
            "status": "completed",
            "conclusion": "success",
            "created_at": "2026-11-12T05:00:00Z",
            "display_title": "CI full PR 170 bootstrap-1",
            "head_sha": HEAD,
        }
        unrelated = dict(later, id=4, conclusion="skipped", html_url="unrelated label")
        entry = {"state": "QUEUED", "position": 1}
        reported, _ = self.enqueue(labeled=[unrelated], entry=entry)
        self.assertTrue(reported.startswith("queued"))
        bootstrap = dict(later, id=5, conclusion="failure", html_url="bootstrap")
        with self.assertRaisesRegex(ValueError, "completed/failure: bootstrap"):
            self.enqueue(labeled=[unrelated, bootstrap])
        # Another PR's bootstrap, or one at a commit outside the PR, is not its.
        for other in (
            dict(bootstrap, display_title="CI full PR 171 bootstrap-1"),
            dict(bootstrap, head_sha=PREVIOUS),
        ):
            reported, _ = self.enqueue(labeled=[other], entry=entry)
            self.assertTrue(reported.startswith("queued"))

    def at(
        self, commit: str, run_id: int, created: str, **fields: object
    ) -> ci_log.Run:
        """A dispatched native run of PR 170 testing `commit`."""
        run = cast(ci_log.Run, dict(self.REQUESTED, **fields))
        run["id"] = run_id
        run["display_title"] = f"CI native PR 170 {'c' * 32}-{commit}"
        run["created_at"] = created
        run["html_url"] = f"https://github.com/example/fork/actions/runs/{run_id}"
        return run

    def test_a_green_run_on_an_earlier_pr_commit_counts(self) -> None:
        # #1026: a fix pushed after the passing run queues on Quick alone.
        entry = {"state": "QUEUED", "position": 1}
        earlier = self.at(PREVIOUS, 2, "2026-11-12T02:00:00Z")
        reported, _ = self.enqueue(
            requested=earlier, commits=[PREVIOUS, HEAD], entry=entry
        )
        self.assertTrue(reported.startswith("queued"))
        # A green run at the head passes as before.
        reported, _ = self.enqueue(commits=[PREVIOUS, HEAD], entry=entry)
        self.assertTrue(reported.startswith("queued"))

    def test_the_newest_run_on_the_pr_commits_decides(self) -> None:
        green = self.at(PARENT, 2, "2026-11-12T02:00:00Z")
        commits = [PARENT, PREVIOUS, HEAD]
        for label, newer, message in (
            (
                "failed on a later commit",
                self.at(PREVIOUS, 7, "2026-11-12T03:00:00Z", conclusion="failure"),
                f"native at {PREVIOUS}, is not green: completed/failure: .*runs/7",
            ),
            (
                "running at the head",
                self.at(
                    HEAD,
                    8,
                    "2026-11-12T03:00:00Z",
                    status="in_progress",
                    conclusion=None,
                ),
                f"native at {HEAD}, is not green: in_progress/None: .*runs/8",
            ),
        ):
            with self.subTest(label):
                with self.assertRaisesRegex(ValueError, message):
                    self.enqueue(dispatched=[newer, green], commits=commits)
                self.assertEqual(self.calls, [])

    def test_a_run_outside_the_pr_commits_is_no_run(self) -> None:
        # A rebase or force-push drops the commits earlier runs tested.
        outside = self.at(PREVIOUS, 2, "2026-11-12T02:00:00Z")
        with self.assertRaisesRegex(ValueError, "no requested run.*--profile native"):
            self.enqueue(requested=outside, commits=[PARENT, HEAD])

    def test_a_bootstrap_not_yet_planned_is_pending(self) -> None:
        # Its jobs list is empty until the plan job starts: it may be a
        # bootstrap, so it holds the PR out like a running run.
        planning = {
            "id": 6,
            "status": "queued",
            "conclusion": None,
            "created_at": "2026-11-12T05:00:00Z",
            "display_title": "CI full PR 170 bootstrap-6",
            "head_sha": HEAD,
            "html_url": "planning",
        }
        with self.assertRaisesRegex(ValueError, "full bootstrap at .*queued/None"):
            self.enqueue(labeled=[planning])
        # Once finished without a plan job, it tested nothing.
        done = dict(planning, status="completed", conclusion="success")
        reported, _ = self.enqueue(
            labeled=[dict(done, conclusion="startup_failure")],
            entry={"state": "QUEUED", "position": 1},
        )
        self.assertTrue(reported.startswith("queued"))

    def test_a_passing_run_past_the_first_hundred_dispatches_is_found(self) -> None:
        others = [
            dict(
                self.REQUESTED,
                id=100 + n,
                display_title=f"CI native PR 171 {'c' * 32}-{HEAD}",
                created_at="2026-11-13T00:00:00Z",
            )
            for n in range(100)
        ]
        entry = {"state": "QUEUED", "position": 1}
        reported, _ = self.enqueue(
            pages=[others, [self.REQUESTED]], requested=None, entry=entry
        )
        self.assertTrue(reported.startswith("queued"))

    def test_equal_creation_times_resolve_by_id(self) -> None:
        created = "2026-11-12T02:00:00Z"
        red = self.at(HEAD, 7, created, conclusion="failure")
        green = self.at(PREVIOUS, 8, created)
        commits = [PREVIOUS, HEAD]
        entry = {"state": "QUEUED", "position": 1}
        for order in ([red, green], [green, red]):
            reported, _ = self.enqueue(dispatched=order, commits=commits, entry=entry)
            self.assertTrue(reported.startswith("queued"))
        for order in ([red, dict(green, id=6)], [dict(green, id=6), red]):
            with self.assertRaisesRegex(ValueError, "runs/7"):
                self.enqueue(dispatched=order, commits=commits)

    def test_a_truncated_comparison_counts_the_head_only(self) -> None:
        earlier = self.at(PREVIOUS, 2, "2026-11-12T02:00:00Z")
        commits = [PREVIOUS] + [f"{n:040x}" for n in range(248)] + [HEAD]
        self.assertEqual(len(commits), 250)
        reported, _ = self.enqueue(
            requested=earlier,
            commits=commits,
            total=250,
            entry={"state": "QUEUED", "position": 1},
        )
        self.assertTrue(reported.startswith("queued"))
        with self.assertRaisesRegex(ValueError, "no requested run"):
            self.enqueue(requested=earlier, commits=commits, total=251)
        reported, _ = self.enqueue(
            commits=commits, total=251, entry={"state": "QUEUED", "position": 1}
        )
        self.assertTrue(reported.startswith("queued"))

    def test_a_workflow_change_needs_its_passing_run_at_the_head(self) -> None:
        # ci-invalidate.yml can carry a bootstrap's Full status across an inert
        # docs push; the bootstrap itself must be at the head (CI administration).
        bootstrap = {
            "id": 5,
            "status": "completed",
            "conclusion": "success",
            "created_at": "2026-11-12T05:00:00Z",
            "display_title": "CI full PR 170 bootstrap-5",
            "head_sha": PREVIOUS,
            "html_url": "bootstrap",
        }
        workflow = [{"filename": ".github/workflows/ci-quick.yml"}]
        with self.assertRaisesRegex(
            ValueError,
            f"changes .github/workflows/.*full bootstrap bootstrap.*{PREVIOUS}",
        ):
            self.enqueue(labeled=[bootstrap], commits=[PREVIOUS, HEAD], files=workflow)
        self.assertEqual(self.calls, [])
        entry = {"state": "QUEUED", "position": 1}
        reported, _ = self.enqueue(
            labeled=[dict(bootstrap, head_sha=HEAD)],
            commits=[PREVIOUS, HEAD],
            files=workflow,
            entry=entry,
        )
        self.assertTrue(reported.startswith("queued"))
        # Without a workflow change the same ancestor bootstrap counts.
        reported, _ = self.enqueue(
            labeled=[bootstrap], commits=[PREVIOUS, HEAD], entry=entry
        )
        self.assertTrue(reported.startswith("queued"))

    def test_a_workflow_change_needs_a_green_bootstrap_at_its_head(self) -> None:
        # A green native run at the head is not the bootstrap Full CI
        # administration requires; one on an earlier commit can carry its Full
        # status across an inert push.
        bootstrap = {
            "id": 5,
            "status": "completed",
            "conclusion": "success",
            "created_at": "2026-11-12T01:00:00Z",
            "display_title": "CI full PR 170 bootstrap-5",
            "head_sha": PREVIOUS,
            "html_url": "bootstrap",
        }
        workflow = [{"filename": ".github/workflows/ci-quick.yml"}]
        commits = [PREVIOUS, HEAD]
        for label, labeled, state in (
            ("ancestor bootstrap", [bootstrap], "none"),
            ("no bootstrap", [], "none"),
            (
                "red bootstrap at head",
                [dict(bootstrap, head_sha=HEAD, conclusion="failure")],
                "bootstrap",
            ),
        ):
            with self.subTest(label):
                with self.assertRaisesRegex(
                    ValueError, f"green bootstrap Full at its head {HEAD}: {state}"
                ):
                    self.enqueue(labeled=labeled, commits=commits, files=workflow)
        reported, _ = self.enqueue(
            labeled=[dict(bootstrap, head_sha=HEAD)],
            commits=commits,
            files=workflow,
            entry={"state": "QUEUED", "position": 1},
        )
        self.assertTrue(reported.startswith("queued"))

    def test_a_recorded_label_run_is_judged_as_a_listed_one(self) -> None:
        # A request record can name a label run the listing excluded; it counts
        # only if it is a bootstrap.
        label = dict(
            self.REQUESTED,
            id=9,
            event="pull_request",
            display_title="CI full PR 170 bootstrap-9",
            head_sha=HEAD,
            conclusion="failure",
            created_at="2026-11-12T05:00:00Z",
            html_url="label",
        )
        reported, _ = self.enqueue(
            recorded=label,
            recorded_jobs=[{"name": "plan", "conclusion": "skipped"}],
            entry={"state": "QUEUED", "position": 1},
        )
        self.assertTrue(reported.startswith("queued"))
        with self.assertRaisesRegex(ValueError, "full bootstrap at .*: label"):
            self.enqueue(
                recorded=label,
                recorded_jobs=[{"name": "plan", "conclusion": "success"}],
            )
        # GitHub's word on its PR and tested commit decides, not the record's.
        for other in (
            dict(label, display_title="CI full PR 171 bootstrap-9"),
            dict(label, head_sha=PARENT),
        ):
            reported, _ = self.enqueue(
                recorded=other,
                recorded_jobs=[{"name": "plan", "conclusion": "success"}],
                entry={"state": "QUEUED", "position": 1},
            )
            self.assertTrue(reported.startswith("queued"))

    def test_ejected_head_queues_again_once_only_with_retry(self) -> None:
        entry = {"state": "QUEUED", "position": 1}
        with self.assertRaisesRegex(ValueError, "runs/3.*--retry REASON"):
            self.enqueue(queue=[self.EJECTED])
        reported, _ = self.enqueue(
            queue=[self.EJECTED], retry="lost runner", entry=entry
        )
        self.assertTrue(reported.startswith("queued"))
        twice = [self.EJECTED, dict(self.EJECTED, created_at="2026-11-12T00:00:00Z")]
        with self.assertRaisesRegex(ValueError, "2 times.*root"):
            self.enqueue(queue=twice, retry="lost runner", entry=entry)
        passed = dict(self.EJECTED, conclusion="success")
        reported, _ = self.enqueue(queue=[passed], entry=entry)
        self.assertTrue(reported.startswith("queued"))

    def test_run_cancelled_by_a_job_timeout_counts_as_a_removal(self) -> None:
        # #938: a job timeout ends the merge-group run cancelled, not failed.
        entry = {"state": "QUEUED", "position": 1}
        cancelled = dict(self.EJECTED, conclusion="cancelled")
        with self.assertRaisesRegex(ValueError, "runs/3.*--retry REASON"):
            self.enqueue(queue=[cancelled])
        reported, _ = self.enqueue(queue=[cancelled], retry="job timeout", entry=entry)
        self.assertTrue(reported.startswith("queued"))
        twice = [cancelled, dict(self.EJECTED, created_at="2026-11-12T00:00:00Z")]
        with self.assertRaisesRegex(ValueError, "2 times.*root"):
            self.enqueue(queue=twice, retry="job timeout", entry=entry)

    def test_new_head_after_ejection_needs_base_and_a_stamp(self) -> None:
        entry = {"state": "QUEUED", "position": 1}
        code = [{"filename": "tools/ci.py"}]
        with self.assertRaisesRegex(
            ValueError, "3 commits behind, changing tools/ci.py"
        ):
            self.enqueue(
                queue=[self.EJECTED], ejected=PREVIOUS, behind=3, base_files=code
            )
        self.enqueue(queue=[self.EJECTED], ejected=PREVIOUS, entry=entry)
        self.assertEqual(self.stamp.call_args.args, (170, HEAD, None))

    def test_a_head_behind_only_by_inert_markdown_queues_again(self) -> None:
        entry = {"state": "QUEUED", "position": 1}
        roadmap = [{"filename": "docs/roadmap.md"}, {"filename": "AGENTS.md"}]
        self.enqueue(
            queue=[self.EJECTED],
            ejected=PREVIOUS,
            behind=2,
            base_files=roadmap,
            entry=entry,
        )
        self.assertEqual(self.stamp.call_args.args, (170, HEAD, None))
        for label, files in (
            ("code", [{"filename": "tools/ci.py"}]),
            ("built doc", [{"filename": "docs/multiplayer.md"}]),
            (
                "rename from code",
                [{"filename": "docs/x.md", "previous_filename": "tools/x.py"}],
            ),
            ("listing at the limit", [{"filename": "docs/roadmap.md"}] * 300),
        ):
            with self.subTest(label), self.assertRaisesRegex(ValueError, "behind"):
                self.enqueue(
                    queue=[self.EJECTED], ejected=PREVIOUS, behind=1, base_files=files
                )

    def test_enqueue_refuses_a_body_without_its_attribution_line(self) -> None:
        for body in (
            "",
            "Agent: /x | Model: m",
            "Agent /x | Model: m | Reasoning effort: high",
        ):
            with (
                self.subTest(body=body),
                patch.dict(PULL, body=body),
                self.assertRaisesRegex(ValueError, "attribution line"),
            ):
                self.enqueue()
        self.assertEqual(self.calls, [])

    def test_enqueue_names_every_missing_acceptance(self) -> None:
        growth = {"kind": "growth", "counter": "tools_cpp", "added": {"t.cpp": 3}}
        lint = {
            "kind": "lint",
            "tool": "clippy",
            "site": "rust/x.rs | fn f | too_many_lines | item",
            "before": [],
            "after": [120],
        }
        carry = {"kind": "carry", "tool": "clippy", "from": "a", "to": "b"}
        records = [growth, lint, carry]

        def accepting(*accepted: Mapping[str, object]) -> dict[str, str]:
            return {
                "body": "Agent: /review | Model: m | Reasoning effort: high\n"
                "```cleanup-acceptance\n"
                + json.dumps({"head": PREVIOUS, "accept": list(accepted)})
                + "\n```"
            }

        entry = {"state": "QUEUED", "position": 1}
        for comments, missing in (
            ([], 2),
            ([accepting(growth)], 1),
            # The implementer's own block accepts nothing.
            (
                [
                    dict(
                        accepting(growth, lint),
                        body=accepting(growth, lint)["body"].replace(
                            "/review", "/implementation"
                        ),
                    )
                ],
                2,
            ),
        ):
            with (
                self.subTest(comments=len(comments), missing=missing),
                self.assertRaisesRegex(
                    ValueError, f"lacks {missing} cleanup"
                ) as raised,
            ):
                self.enqueue(records=records, comments=comments, entry=entry)
            self.assertEqual(self.calls, [])
            text = str(raised.exception)
            self.assertIn("too_many_lines", text)
            self.assertNotIn("carry", text)
        # An acceptance read at an earlier head holds at this one.
        reported, calls = self.enqueue(
            records=records, comments=[accepting(growth, lint)], entry=entry
        )
        self.assertTrue(reported.startswith("queued"))
        self.assertEqual(calls[0][:2], ("pr", "merge"))

    def test_records_and_an_unsafe_item_are_read_from_commits(self) -> None:
        site = "rust/x.rs | impl Native | allow unsafe_code"
        record = {
            "kind": "suppression",
            "tool": "rust",
            "site": site,
            "before": 0,
            "after": 1,
            "operation": "o",
            "source_contract": "s",
            "reason": "r",
            "retiring_owner": "w",
            "retirement_condition": "c",
        }
        item = "#[allow(unsafe_code)]\nimpl Native { pub const N: u8 = 1; }\n"
        records = "migration/cleanup-records/t3-x.json"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source_fixture.initialize(root, {"rust/x.rs": "pub struct Native;\n"})
            base = vcs.git("rev-parse", "HEAD", cwd=root).strip()

            def commit(files: Mapping[str, str]) -> str:
                for path, text in files.items():
                    (root / path).parent.mkdir(parents=True, exist_ok=True)
                    (root / path).write_text(text)
                vcs.git("add", "-A", cwd=root)
                vcs.git(
                    "-c",
                    "user.name=t",
                    "-c",
                    "user.email=t@example.invalid",
                    "commit",
                    "-qm",
                    "Change: x",
                    cwd=root,
                )
                return vcs.git("rev-parse", "HEAD", cwd=root).strip()

            reviewed = commit(
                {
                    "rust/x.rs": "pub struct Native;\n" + item,
                    records: json.dumps([record]),
                }
            )
            other = commit({"rust/x.rs": "pub struct Native;\nfn g() {}\n" + item})
            changed = commit(
                {"rust/x.rs": "pub struct Native;\n" + item.replace("1", "2")}
            )
            # The checkout stays at the base: records come from the commits.
            vcs.git("checkout", "-q", base, cwd=root)
            body = (
                "Agent: /review | Model: m | Reasoning effort: high\n"
                "```cleanup-acceptance\n"
                + json.dumps({"head": reviewed, "accept": [record]})
                + "\n```"
            )
            rooted = partial(vcs.git, cwd=root)
            for head, refused in (
                (reviewed, ""),
                (other, ""),
                (changed, "item changed"),
            ):
                with (
                    self.subTest(head=head),
                    patch.object(ci.cleanup_contract, "git", rooted),
                    patch.object(ci.cleanup_contract, "ROOT", root),
                    # lint-policy.py runs as a process in this repository.
                    patch.dict(os.environ, {"GIT_DIR": str(root / ".git")}),
                    patch.object(
                        ci,
                        "api",
                        return_value={"merge_base_commit": {"sha": base}},
                    ),
                    patch.object(
                        ci.cleanup_contract, "discussion", return_value=[body]
                    ),
                ):
                    self.assertEqual(ci.added_records("example/fork", head), [record])
                    check = partial(
                        ci.missing_acceptances, "example/fork", 170, PULL, head
                    )
                    if refused:
                        with self.assertRaisesRegex(ValueError, refused):
                            check()
                    else:
                        check()
            with (
                patch.object(ci.cleanup_contract, "git", rooted),
                patch.object(
                    ci, "api", return_value={"merge_base_commit": {"sha": base}}
                ),
                self.assertRaisesRegex(ValueError, f"{PREVIOUS} is not in this clone"),
            ):
                ci.added_records("example/fork", PREVIOUS)

    def test_a_comparator_that_fails_is_an_error_not_a_changed_item(self) -> None:
        site = "rust/x.rs | impl Native | allow unsafe_code"
        unsafe = {"kind": "suppression", "site": site, "before": 0, "after": 1}
        comment = (
            "Agent: /review | Model: m | Reasoning effort: high\n"
            "```cleanup-acceptance\n"
            + json.dumps({"head": PREVIOUS, "accept": [unsafe]})
            + "\n```"
        )
        crashed = subprocess.CompletedProcess([], 2, "", "ModuleNotFoundError: x")
        with (
            patch.object(ci, "added_records", return_value=[unsafe]),
            patch.object(ci.cleanup_contract, "discussion", return_value=[comment]),
            patch.object(ci.cleanup_contract, "has_commit", return_value=True),
            patch.object(ci.subprocess, "run", return_value=crashed),
            self.assertRaisesRegex(ValueError, "failed \\(exit 2\\): ModuleNotFound"),
        ):
            ci.missing_acceptances("example/fork", 170, PULL, HEAD)

    def test_an_unreadable_accepted_commit_does_not_hide_other_records(
        self,
    ) -> None:
        site = "rust/x.rs | impl Native | allow unsafe_code"
        unsafe = {"kind": "suppression", "site": site, "before": 0, "after": 1}
        growth = {"kind": "growth", "counter": "tools_cpp", "added": {"t.cpp": 1}}

        def block(at: str) -> str:
            return (
                "Agent: /review | Model: m | Reasoning effort: high\n"
                "```cleanup-acceptance\n"
                + json.dumps({"head": at, "accept": [unsafe]})
                + "\n```"
            )

        for comments, expected in (
            # An older acceptance at a commit this clone lacks, then one at head.
            ([block(PREVIOUS), block(HEAD)], ["growth tools_cpp"]),
            ([block(PREVIOUS)], ["growth tools_cpp", f"fetch the accepted {PREVIOUS}"]),
        ):
            with (
                self.subTest(comments=len(comments)),
                patch.object(ci, "added_records", return_value=[unsafe, growth]),
                patch.object(ci.cleanup_contract, "discussion", return_value=comments),
                self.assertRaises(ValueError) as raised,
            ):
                ci.missing_acceptances("example/fork", 170, PULL, HEAD)
            text = str(raised.exception)
            for part in expected:
                self.assertIn(part, text)
            self.assertIn(f"lacks {len(expected)} cleanup", text)

    def test_a_skipped_quick_counts_only_when_alone(self) -> None:
        skipped = dict(
            self.QUICK, conclusion="skipped", started_at="2026-11-12T01:00:05Z"
        )
        for runs in ([self.QUICK, skipped], [skipped, self.QUICK]):
            with patch.object(
                ci, "api", return_value={"check_runs": [dict(run) for run in runs]}
            ):
                self.assertEqual(ci.latest_quick("example/fork", HEAD), self.QUICK)
        with self.assertRaisesRegex(ValueError, "ci.py ready 170"):
            self.enqueue(quick=skipped)


def comment(number: int, body: str, at: str) -> review_report.Comment:
    return {
        "id": number,
        "html_url": f"https://github.com/example/fork/pull/170#issuecomment-{number}",
        "updated_at": at,
        "body": body,
    }


def review(sha: str, agent: str, closing: str) -> str:
    attribution = f"Agent: {agent} | Model: m-1 | Reasoning effort: high"
    return f"Reviewed commit: {sha}\n{attribution}\n{closing}"


class ReviewReportTests(unittest.TestCase):
    def test_agent_and_disposition_come_from_the_report_or_are_unknown(self) -> None:
        attribution = "Agent: /root/r | Model: m-1 | Reasoning effort: high"
        fence = '```cleanup-acceptance\n{"accept": ["x"]}\n```'
        for body, agent, disposition, scope in (
            (
                f"x\n{attribution}\nDisposition: accepted, whole PR",
                "/root/r",
                "accepted",
                "whole PR",
            ),
            (
                f"{attribution}\n**Disposition:** findings to remediate, group 2\n",
                "/root/r",
                "findings to remediate",
                "group 2",
            ),
            (
                f"Disposition: accepted\n{fence}\n{attribution}",
                "/root/r",
                "accepted",
                None,
            ),
            (f"{attribution}\nAccepted.", "/root/r", "accepted", None),
            (f"{attribution}\nLooks fine to me", "/root/r", "unknown", None),
            (
                "no attribution\nDisposition: accepted",
                "unknown",
                "accepted",
                None,
            ),
            ("", "unknown", "unknown", None),
        ):
            with self.subTest(body=body):
                (report,) = ci.review_report.review_reports(
                    [comment(1, f"Reviewed commit: {HEAD}\n" + body, "t")], [], HEAD
                )
                self.assertEqual(
                    (report.agent, report.disposition, report.scope),
                    (agent, disposition, scope),
                )

    def test_quoted_earlier_report_does_not_take_the_disposition(self) -> None:
        earlier = review(PREVIOUS, "/root/old", "Disposition: findings to remediate")
        attribution = "Agent: /root/new | Model: m-1 | Reasoning effort: high"
        quoted = "\n".join("> " + line for line in earlier.splitlines())
        for body in (
            f"Reviewed commit: {HEAD}\n{attribution}\n```\n{earlier}\n```\n"
            "Disposition: accepted, group 1",
            f"Reviewed commit: {HEAD}\n{attribution}\n{quoted}\n"
            "Disposition: accepted, group 1",
            f"**Reviewed commit:** `{HEAD}`\n{attribution}\n~~~\n{earlier}\n~~~\n"
            "Disposition: accepted, group 1",
        ):
            with self.subTest(body=body):
                reports = ci.review_report.review_reports(
                    [comment(1, body, "t")], [], HEAD
                )
                self.assertEqual(
                    [(r.sha, r.agent, r.disposition) for r in reports],
                    [(HEAD, "/root/new", "accepted")],
                )

    def test_only_a_comment_that_opens_with_the_header_is_a_report(self) -> None:
        earlier = review(PREVIOUS, "/root/old", "Disposition: findings to remediate")
        attribution = "Agent: /root/new | Model: m-1 | Reasoning effort: high"
        for body in (
            # Prose that quotes a header in inline code, then a disposition.
            f"{attribution}\nThe earlier report said `Reviewed commit: {PREVIOUS}`.\n"
            "Disposition: accepted, group 1",
            # An earlier report in a tilde fence ahead of the real header.
            f"~~~\n{earlier}\n~~~\nReviewed commit: {HEAD}\n{attribution}\n"
            "Disposition: accepted, group 1",
            f"{attribution}\nReviewed commit: {HEAD}\nDisposition: accepted",
        ):
            with self.subTest(body=body):
                reports = ci.review_report.review_reports(
                    [comment(1, body, "t")], [], HEAD
                )
                self.assertEqual(reports, [])

    def test_github_reviews_and_older_heads_are_reported_as_such(self) -> None:
        reviews: list[review_report.Review] = [
            {
                "commit_id": PREVIOUS,
                "html_url": "r",
                "submitted_at": "t",
                "state": "COMMENTED",
                "body": review(PREVIOUS, "/root/g", "Disposition: accepted"),
            }
        ]
        (report,) = ci.review_report.review_reports([], reviews, HEAD)
        self.assertEqual(
            (report.source, report.matches_head, report.agent, report.disposition),
            ("GitHub review: COMMENTED", False, "/root/g", "accepted"),
        )


class CheckRun(TypedDict):
    """A Quick validation check run as the check-runs API lists it."""

    status: str
    conclusion: str | None
    started_at: str | None
    html_url: str


def quick_run(conclusion: str = "success") -> CheckRun:
    return {
        "status": "completed",
        "conclusion": conclusion,
        "started_at": "2026-11-12T01:00:00Z",
        "html_url": "https://github.com/example/fork/runs/1",
    }


def queue_state(
    state: str = "OPEN", entry: ci.QueueEntry | None = None
) -> ci.QueueState:
    """The GraphQL pull request: merged at MERGE, or open with an optional entry."""
    return {
        "state": state,
        "mergeCommit": {"oid": MERGE} if state == "MERGED" else None,
        "mergeQueueEntry": entry,
        "autoMergeRequest": None,
    }


@dataclass
class World:
    """Recorded answers for one PR: quick (its Quick check run), runs (pages of
    dispatched runs), queue (the GraphQL pull request), heads (successive reads
    of the PR's head), comments (pages of comments), group (merge-group runs),
    commits (the PR's own, oldest first), labeled (its bootstrap runs),
    later_runs (the dispatched runs any later listing sees) and requests (this
    checkout's request records, whose runs GitHub no longer has)."""

    quick: CheckRun | None = field(default_factory=quick_run)
    runs: list[list[ci_log.Run]] = field(default_factory=list)
    queue: ci.QueueState = field(default_factory=queue_state)
    heads: list[str] = field(default_factory=lambda: [HEAD, HEAD])
    comments: list[list[review_report.Comment]] = field(default_factory=lambda: [[]])
    group: list[ci.QueueRun] = field(default_factory=list)
    records: list[dict[str, Any]] = field(default_factory=list)
    commits: list[str] = field(default_factory=lambda: [HEAD])
    labeled: list[ci_log.Run] = field(default_factory=list)
    later_runs: list[list[ci_log.Run]] | None = None
    requests: list[dict[str, object]] = field(default_factory=list)
    quick_runs: list[ci_log.Run] = field(default_factory=list)


class FakeGitHub:
    """A World as the API and gh answer for it, recording gh's calls."""

    def __init__(self, world: World) -> None:
        self.world = world
        self.heads = iter(world.heads)
        self.calls: list[tuple[str, ...]] = []
        self.listed = 0

    def api(self, _repository: str, path: str) -> object:
        answers = {
            "check-runs": {
                "check_runs": [self.world.quick] if self.world.quick else []
            },
            "actions/workflows/ci-quick.yml/runs": {
                "workflow_runs": self.world.quick_runs
            },
            "event=merge_group": {"workflow_runs": self.world.group},
            "compare/rust-migration...": {
                "commits": [{"sha": sha} for sha in self.world.commits],
                "total_commits": len(self.world.commits),
            },
            "/jobs": {"jobs": [{"name": "plan", "conclusion": "success"}]},
            f"commits/{MERGE}": {"parents": [{"sha": PARENT}, {"sha": HEAD}]},
        }
        if path == "pulls/170":
            return dict(
                PULL,
                title="Port the thing",
                created_at="2026-11-11T00:00:00Z",
                mergeable_state="clean",
                head={"sha": next(self.heads)},
            )
        if path.startswith("actions/runs/") and not path.endswith("/jobs"):
            raise subprocess.CalledProcessError(1, ["gh", "api", path], "HTTP 404")
        (answer,) = [value for key, value in answers.items() if key in path]
        return answer

    def gh(self, *args: str) -> str:
        self.calls.append(args)
        if args[:2] == ("api", "graphql"):
            pull = self.world.queue
            return json.dumps({"data": {"repository": {"pullRequest": pull}}})
        runs = self.world.runs
        if "event=workflow_dispatch" in args[-1]:
            self.listed += 1
            if self.listed > 1 and self.world.later_runs is not None:
                runs = self.world.later_runs
        answers = {
            "/comments": self.world.comments,
            "/reviews": [[]],
            "event=workflow_dispatch": [{"workflow_runs": p} for p in runs],
            "event=pull_request": [{"workflow_runs": self.world.labeled}],
            "pulls/170/files": [[{"filename": "tools/ci.py"}]],
        }
        (answer,) = [value for key, value in answers.items() if key in args[-1]]
        return json.dumps(answer)


class RunSearchTests(unittest.TestCase):
    """workflow_runs reads a PR's runs back to its opening past GitHub's
    1,000-run search limit."""

    def test_a_saturated_search_is_split_until_each_part_is_whole(self) -> None:
        opened = datetime(2026, 11, 1, tzinfo=timezone.utc)
        until = datetime(2026, 11, 3, tzinfo=timezone.utc)
        middle = datetime(2026, 11, 2, tzinfo=timezone.utc)
        spans: list[tuple[datetime, datetime]] = []

        def gh(*args: str) -> str:
            created = unquote(args[-1].split("created=")[1].split("&")[0])
            start, end = (
                datetime.fromisoformat(part.replace("Z", "+00:00"))
                for part in created.split("..")
            )
            spans.append((start, end))
            if (start, end) == (opened, until):
                # Cut at the limit: only the newest runs come back.
                runs, total = [{"id": 3}], 1001
            elif start >= middle:
                runs, total = [{"id": 3}, {"id": 2}], 2
            else:
                # The run at the middle is listed by both halves.
                runs, total = [{"id": 2}, {"id": 1}], 2
            return json.dumps([{"total_count": total, "workflow_runs": runs}])

        with patch.object(ci, "gh", side_effect=gh):
            runs = ci.workflow_runs(
                "example/fork", "workflow_dispatch", "2026-11-01T00:00:00Z", until
            )
        self.assertEqual(sorted(run["id"] for run in runs), [1, 2, 3])
        self.assertEqual(spans[0], (opened, until))
        self.assertEqual(min(start for start, _ in spans), opened)
        self.assertEqual(len(spans), 3)

    def test_a_whole_search_is_read_once(self) -> None:
        with patch.object(
            ci,
            "gh",
            return_value=json.dumps([{"total_count": 1, "workflow_runs": [{"id": 1}]}]),
        ) as gh:
            runs = ci.workflow_runs(
                "example/fork", "pull_request", "2026-11-01T00:00:00Z"
            )
        self.assertEqual([run["id"] for run in runs], [1])
        self.assertEqual(gh.call_count, 1)
        self.assertIn(
            "event=pull_request&created=2026-11-01T00%3A00%3A00Z..",
            gh.call_args.args[-1],
        )


class StateTests(unittest.TestCase):
    """ci.py state PR against recorded, paginated gh answers (P1)."""

    RUST_RED = requested(
        "rust",
        "c" * 32,
        "2026-11-12T02:00:00Z",
        conclusion="failure",
    )
    NATIVE_GREEN = requested(
        "native",
        "e" * 32,
        "2026-11-12T04:00:00Z",
        conclusion="success",
    )
    NATIVE_OLD = requested(
        "native",
        "f" * 32,
        "2026-11-12T01:00:00Z",
        conclusion="failure",
    )

    def world(self) -> World:
        """Rust red, then on a second page native green over an older red run."""
        return World(runs=[[self.RUST_RED], [self.NATIVE_GREEN, self.NATIVE_OLD]])

    def state(
        self, *argv: str, world: World | None = None
    ) -> tuple[int, str, list[tuple[str, ...]]]:
        """Run `ci.py state 170` (or `argv`) against a FakeGitHub of `world`."""
        github = FakeGitHub(world or self.world())
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(ci, "REQUESTS", Path(directory)),
            patch.object(ci, "api", side_effect=github.api),
            patch.object(ci, "gh", side_effect=github.gh),
            patch.object(ci, "added_records", return_value=github.world.records),
            patch(
                "sys.argv",
                ["ci.py", *(argv or ("state", "170")), "--repo", "example/fork"],
            ),
            patch("builtins.print") as printed,
        ):
            for number, record in enumerate(github.world.requests):
                (Path(directory) / f"{number}.json").write_text(json.dumps(record))
            code = ci.main()
        output = "\n".join(str(c.args[0]) for c in printed.call_args_list)
        self.listed = github.listed
        return code, output, github.calls

    def test_one_screen_with_mixed_profiles_and_bounded_reports(self) -> None:
        reports = [
            comment(
                1,
                review(PREVIOUS, "/root/old", "Disposition: accepted, whole PR"),
                "2026-11-12T01:00:00Z",
            ),
            comment(
                2,
                "Status: working\nAgent: /root/impl | Model: m | Reasoning effort: high",
                "2026-11-12T01:30:00Z",
            ),
            comment(
                3,
                review(
                    HEAD, "/root/new", "Disposition: findings to remediate, group 1"
                ),
                "2026-11-12T05:00:00Z",
            ),
            comment(
                4,
                f"Reviewed commit: {HEAD}\nno attribution, no disposition",
                "2026-11-12T06:00:00Z",
            ),
        ] + [
            comment(
                10 + n,
                review(HEAD, f"/root/r{n}", "Disposition: accepted"),
                f"2026-11-12T0{n}:10:00Z",
            )
            for n in range(4)
        ]
        code, output, calls = self.state(
            world=replace(self.world(), comments=[reports[:3], reports[3:]])
        )
        self.assertEqual(code, 0)
        lines = output.splitlines()
        url = "https://github.com/example/fork/pull/170#issuecomment-"
        self.assertEqual(
            lines[:6],
            [
                "PR #170 open, ready: Port the thing",
                f"head {HEAD}, mergeable: yes (clean)",
                "Quick at head: success https://github.com/example/fork/runs/1",
                "native at head: success https://github.com/example/fork/actions/runs/04",
                "rust at head: failure https://github.com/example/fork/actions/runs/02",
                "Queue: not queued",
            ],
        )
        self.assertEqual(lines[6], "Review reports: 7, newest first")
        self.assertEqual(lines[7], f"  unknown: unknown at head {url}4")
        self.assertEqual(
            lines[8], f"  /root/new: findings to remediate (group 1) at head {url}3"
        )
        self.assertEqual(lines[12], f"  2 older: {url}1, {url}10")
        self.assertIn(f"/root/r3: accepted at head {url}13", output)
        self.assertNotIn("/root/impl", output)
        self.assertEqual(len(lines), 14)
        # enqueue reads every page, as state does: native's green run on the
        # second page is the newest.
        self.assertEqual(lines[13], "enqueue checks: pass")
        # Every listing is paginated.
        self.assertTrue(
            all("--paginate" in call for call in calls if call[1] != "graphql")
        )

    def test_old_acceptance_is_at_its_sha_not_invalid(self) -> None:
        old = comment(
            1,
            review(PREVIOUS[:12], "/root/old", "Disposition: accepted, whole PR"),
            "t",
        )
        _, output, _ = self.state(world=replace(self.world(), comments=[[old]]))
        self.assertIn(
            f"  /root/old: accepted (whole PR) at {PREVIOUS[:10]}, not head", output
        )

    def test_queue_lines(self) -> None:
        failed: ci.QueueRun = {
            "id": 7,
            "head_branch": f"gh-readonly-queue/rust-migration/pr-170-{BASE}",
            "created_at": "2026-11-11T01:00:00Z",
            "status": "completed",
            "conclusion": "cancelled",
            "html_url": "https://github.com/example/fork/actions/runs/7",
            "head_sha": MERGE,
        }
        passed = failed.copy()
        passed["conclusion"] = "success"
        queued = queue_state(entry={"state": "QUEUED", "position": 2})
        green = World(runs=[[self.NATIVE_GREEN]])
        for world, line in (
            (replace(green, queue=queued), "Queue: queued at position 2"),
            (green, "Queue: not queued"),
            (replace(green, group=[passed]), "Queue: not queued"),
            (
                replace(green, group=[failed]),
                "Queue: not queued; last merge-group run cancelled: "
                + failed["html_url"],
            ),
            (replace(green, queue=queue_state("MERGED")), "Queue: merged"),
        ):
            with self.subTest(line):
                _, output, _ = self.state(world=world)
                self.assertIn(line + "\n", output)

    def test_a_running_quick_without_its_check_shows_as_running(self) -> None:
        """No Quick validation check yet, Quick's jobs still running (#1063)."""
        running = World(
            runs=[[self.NATIVE_GREEN]], quick=None, quick_runs=[QUICK_RUNNING]
        )
        _, output, _ = self.state(world=running)
        self.assertIn(f"Quick at head: in_progress {QUICK_RUNNING['html_url']}", output)
        self.assertIn("enqueue checks: blocked: Quick validation at", output)
        _, output, _ = self.state(world=replace(running, quick_runs=[]))
        self.assertNotIn(QUICK_RUNNING["html_url"], output)

    def test_green_head_passes_enqueue_checks_only_as_checks(self) -> None:
        green = World(runs=[[self.NATIVE_GREEN]])
        code, output, calls = self.state(world=green)
        self.assertEqual(code, 0)
        self.assertEqual(output.splitlines()[-1], "enqueue checks: pass")
        self.assertFalse([call for call in calls if call[0] != "api"])
        red = replace(green, quick=quick_run("failure"))
        _, output, _ = self.state(world=red)
        self.assertIn("enqueue checks: blocked: Quick validation at", output)
        growth = {"kind": "growth", "counter": "tools_cpp", "added": {"t.cpp": 1}}
        _, output, _ = self.state(world=replace(green, records=[growth]))
        self.assertIn("enqueue checks: blocked: PR #170 at", output)
        self.assertIn("growth tools_cpp", output)

    def test_newest_run_below_head_is_shown_as_enqueue_reads_it(self) -> None:
        # Equal creation times resolve by id, in the gate and here alike.
        created = "2026-11-12T03:00:00Z"
        red = requested("rust", "c" * 32, created, "failure")
        red["id"] = 7
        green = requested("native", "e" * 32, created, "success")
        green["id"] = 8
        green["display_title"] = f"CI native PR 170 {'e' * 32}-{PREVIOUS}"
        world = World(runs=[[red, green]], commits=[PREVIOUS, HEAD])
        _, output, _ = self.state(world=world)
        lines = output.splitlines()
        self.assertIn(f"rust at head: failure {red['html_url']}", lines)
        self.assertIn(
            f"Newest requested run, native at {PREVIOUS[:10]}, 1 commit(s) below "
            f"head: success {green['html_url']}",
            lines,
        )
        self.assertEqual(lines[-1], "enqueue checks: pass")
        _, raw, _ = self.state("state", "170", "--json", world=world)
        newest = json.loads(raw)["newest_request"]
        self.assertEqual(
            (newest["profile"], newest["commit"], newest["behind"]),
            ("native", PREVIOUS, 1),
        )
        # With the ids swapped the red run at the head is the newest.
        swapped = replace(world, runs=[[{**red, "id": 9}, green]])
        _, output, _ = self.state(world=swapped)
        self.assertNotIn("Newest requested run", output)
        self.assertIn("enqueue checks: blocked: The newest requested run", output)
        _, raw, _ = self.state("state", "170", "--json", world=swapped)
        self.assertEqual(json.loads(raw)["newest_request"]["behind"], 0)

    def test_one_read_of_the_runs_serves_the_view_and_the_checks(self) -> None:
        # A run requested while state reads must not split its view from its
        # verdict.
        green = requested("native", "e" * 32, "2026-11-12T02:00:00Z", "success")
        green["display_title"] = f"CI native PR 170 {'e' * 32}-{PREVIOUS}"
        red = requested("native", "f" * 32, "2026-11-12T08:00:00Z", "failure")
        world = World(
            runs=[[green]], later_runs=[[red, green]], commits=[PREVIOUS, HEAD]
        )
        _, output, _ = self.state(world=world)
        self.assertEqual(self.listed, 1)
        self.assertIn(
            f"Newest requested run, native at {PREVIOUS[:10]}, 1 commit(s) below "
            f"head: success {green['html_url']}",
            output,
        )
        self.assertEqual(output.splitlines()[-1], "enqueue checks: pass")

    def test_a_vanished_recorded_run_blocks_only_the_checks(self) -> None:
        record = {"pr": 170, "head": HEAD, "run_id": 77, "profile": "native"}
        world = replace(self.world(), requests=[record])
        code, output, _ = self.state(world=world)
        self.assertEqual(code, 0)
        lines = output.splitlines()
        self.assertEqual(lines[0], "PR #170 open, ready: Port the thing")
        # The runs GitHub listed still show; only the checks name the lost run.
        self.assertIn(
            "native at head: success https://github.com/example/fork/actions/runs/04",
            lines,
        )
        self.assertTrue(lines[-1].startswith("enqueue checks: blocked: "), lines[-1])
        self.assertIn("run 77 of 0.json", lines[-1])

    def test_moved_head_prints_no_mixed_view(self) -> None:
        code, output, _ = self.state(
            world=replace(self.world(), heads=[HEAD, PREVIOUS])
        )
        self.assertEqual(code, 1)
        (line,) = output.splitlines()
        self.assertIn(f"head moved from {HEAD} to {PREVIOUS}", line)
        self.assertTrue(line.endswith("run python3 tools/ci.py state 170 again"))

    def test_json_and_status_alias_show_the_same_facts(self) -> None:
        _, text, _ = self.state()
        _, alias, _ = self.state("status", "170")
        self.assertEqual(alias, text)
        _, raw, _ = self.state("state", "170", "--json")
        facts = json.loads(raw)
        self.assertEqual(facts["head"], HEAD)
        self.assertEqual(sorted(facts["requests"]), ["native", "rust"])
        self.assertEqual(facts["queue"], {"queued": False, "text": "not queued"})
        self.assertEqual(
            text.splitlines()[-1], "enqueue checks: " + facts["enqueue_checks"]
        )


JOB_URL = "https://github.com/example/fork/actions/runs/7/job/9"
RAW_LOG = (
    "﻿2026-11-12T01:00:00.1000000Z ##[group]Run actions/checkout\n"
    "2026-11-12T01:00:01.0000000Z checked out\n"
    "2026-11-12T01:00:05.0000000Z ##[group]Run cargo test\n"
    "2026-11-12T01:00:06.0000000Z \x1b[31merror[E0308]\x1b[0m: mismatched types\n"
    "  continuation of the error\n"
    "2026-11-12 not GitHub's envelope\n"
    "2026-11-12T01:00:06.5000000Z " + "x" * 400 + "\n"
    "2026-11-12T01:00:07.0000000Z ##[error]Process completed with exit code 101.\n"
    "2026-11-12T01:00:07.5000000Z Post job cleanup, in the same second\n"
    "2026-11-12T01:00:09.0000000Z Post job cleanup\n"
)


def log_step(
    number: int, name: str, conclusion: str, start: int, end: int
) -> ci_log.Step:
    """A step of a job of run 7, from second `start` past 01:00 to `end`."""
    return {
        "number": number,
        "name": name,
        "conclusion": conclusion,
        "started_at": f"2026-11-12T01:00:{start:02}Z",
        "completed_at": f"2026-11-12T01:00:{end:02}Z",
    }


def log_job(
    job_id: int = 9,
    attempt: int = 1,
    conclusion: str | None = "failure",
    steps: list[ci_log.Step] | None = None,
    name: str = "native / test",
) -> ci_log.Job:
    """A job of run 7 as the jobs API lists it, by default failing in step 2;
    without a conclusion it is still running."""
    if steps is None:
        steps = [
            log_step(1, "Checkout", "success", 0, 2),
            log_step(2, "Test", "failure", 5, 7),
            log_step(3, "Post", "success", 9, 10),
        ]
    url = JOB_URL if job_id == 9 else JOB_URL.replace("/9", f"/{job_id}")
    return {
        "id": job_id,
        "run_id": 7,
        "run_attempt": attempt,
        "name": name,
        "status": "in_progress" if conclusion is None else "completed",
        "conclusion": conclusion,
        "html_url": url,
        "steps": steps,
    }


def passed(job: ci_log.Job) -> list[ci_log.Step]:
    """The job's steps, each concluding success."""
    steps = [step.copy() for step in job.get("steps") or []]
    for step in steps:
        step["conclusion"] = "success"
    return steps


def log_run(
    attempt: int = 1, status: str = "completed", conclusion: str | None = "failure"
) -> ci_log.Run:
    """Run 7 as the runs API gives it."""
    return {
        "id": 7,
        "run_attempt": attempt,
        "status": status,
        "conclusion": conclusion,
        "html_url": "https://github.com/example/fork/actions/runs/7",
    }


class LogTests(unittest.TestCase):
    """ci.py log against recorded jobs and logs, with the cache in a temporary
    directory (P2)."""

    RUN = log_run()

    def setUp(self) -> None:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.cache = Path(directory.name)
        patcher = patch.object(ci_log, "LOGS", self.cache)
        patcher.start()
        self.addCleanup(patcher.stop)

    def log(
        self,
        *argv: str,
        jobs: list[ci_log.Job],
        logs: Iterable[str | None] = (RAW_LOG,),
        run: ci_log.Run | None = None,
    ) -> tuple[int, str, list[int]]:
        """Run `ci.py log ARGV`; `logs` are the successive downloads."""
        answers = iter(logs)
        downloads: list[int] = []

        def download(_repository: str, job_id: int) -> str | None:
            downloads.append(job_id)
            return next(answers)

        def api(_repository: str, path: str) -> object:
            if path == "actions/runs/7":
                return run or self.RUN
            (job,) = [j for j in jobs if path == f"actions/jobs/{j['id']}"]
            return job

        pages = json.dumps([{"jobs": jobs[:1]}, {"jobs": jobs[1:]}])
        with (
            patch.object(ci, "api", side_effect=api),
            patch.object(ci, "gh", return_value=pages),
            patch.object(ci_log, "download", side_effect=download),
            patch("sys.argv", ["ci.py", *argv, "--repo", "example/fork"]),
            patch("builtins.print") as printed,
        ):
            code = ci.main()
        output = "\n".join(str(c.args[0]) for c in printed.call_args_list)
        return code, output, downloads

    def test_failing_step_without_colour_or_stamps_and_one_download(self) -> None:
        jobs = [log_job(8, conclusion="success", steps=[], name="quick")]
        code, output, downloads = self.log("log", "7", jobs=[*jobs, log_job()])
        self.assertEqual(code, 0)
        self.assertNotIn("\x1b", output)
        self.assertNotIn("01:00:06", output)
        self.assertIn(f"== native / test (failure): {JOB_URL}", output)
        self.assertIn("Failing step: 2 Test (failure)", output)
        self.assertIn("  4: error[E0308]: mismatched types", output)
        self.assertIn("  8: ##[error]Process completed with exit code 101.", output)
        # Only stamps in GitHub's envelope are removed; unstamped lines stay in
        # their step; long lines are cut; other steps' lines are left out.
        self.assertIn("  6: 2026-11-12 not GitHub's envelope", output)
        self.assertIn("  5:   continuation of the error", output)
        self.assertIn("x" * ci_log.WIDTH + " [cut]", output)
        self.assertNotIn("checked out", output)
        self.assertNotIn("Post job cleanup", output)
        self.assertNotIn("still running", output)
        cached = self.cache / "example/fork/7/attempt-1/9.log"
        self.assertEqual(cached.read_text(), RAW_LOG)
        self.assertEqual(downloads, [9])
        # Later reads, filtered or raw, come from the cache.
        _, grepped, again = self.log(
            "logs", "7", "--grep", "mismatch", jobs=[log_job()]
        )
        self.assertEqual(
            grepped.splitlines()[-1], "  4: error[E0308]: mismatched types"
        )
        _, full, _ = self.log("log", JOB_URL, "--full", jobs=[log_job()])
        self.assertIn("\x1b[31merror[E0308]", full)
        self.assertEqual(again, [])

    def test_unavailable_or_empty_log_is_not_cached(self) -> None:
        cached = self.cache / "example/fork/7/attempt-1/9.log"
        _, output, _ = self.log("log", "7", jobs=[log_job()], logs=[None])
        self.assertIn("No log: GitHub has no log for it yet (404)", output)
        self.assertIn(f"it was not cached. Job: {JOB_URL}", output)
        self.assertFalse(cached.exists())
        _, output, _ = self.log("log", "7", jobs=[log_job()], logs=["\n"])
        self.assertIn("GitHub served an empty log", output)
        self.assertFalse(cached.exists())
        _, output, downloads = self.log("log", "7", jobs=[log_job()])
        self.assertIn("Failing step: 2 Test", output)
        self.assertEqual(downloads, [9])
        self.assertTrue(cached.exists())

    def test_download_tells_a_missing_log_from_an_api_error(self) -> None:
        def result(code: int, stdout: bytes, stderr: bytes) -> object:
            return subprocess.CompletedProcess(["gh"], code, stdout, stderr)

        with patch.object(ci_log.subprocess, "run") as run:
            run.return_value = result(1, b"", b"gh: Not Found (HTTP 404)")
            self.assertIsNone(ci_log.download("example/fork", 9))
            run.return_value = result(1, b"partial", b"gh: Server Error (HTTP 502)")
            with self.assertRaises(subprocess.CalledProcessError):
                ci_log.download("example/fork", 9)
            run.return_value = result(0, b"ok \xff", b"")
            self.assertEqual(ci_log.download("example/fork", 9), "ok �")
        failed = subprocess.CalledProcessError(1, ["gh"], "", "HTTP 502")
        with patch.object(ci_log, "download", side_effect=failed):
            with self.assertRaises(subprocess.CalledProcessError):
                ci_log.job_log("example/fork", log_job())
        self.assertEqual(list(self.cache.rglob("*")), [])

    def test_second_attempt_reads_its_own_log(self) -> None:
        first = self.cache / "example/fork/7/attempt-1/9.log"
        first.parent.mkdir(parents=True)
        first.write_text("2026-11-12T01:00:06.0000000Z error: first attempt\n")
        retried = log_job(19, attempt=2)
        run = log_run(attempt=2)
        _, output, downloads = self.log("log", "7", jobs=[retried], run=run)
        self.assertIn("Run 7 attempt 2 (failure)", output)
        self.assertIn("mismatched types", output)
        self.assertNotIn("first attempt", output)
        self.assertEqual(downloads, [19])
        self.assertTrue((self.cache / "example/fork/7/attempt-2/19.log").exists())

    def test_two_writers_publish_the_same_log(self) -> None:
        path = self.cache / "example/fork/7/attempt-1/9.log"
        both = threading.Barrier(2, timeout=5)
        replace = os.replace
        errors: list[BaseException] = []

        def meet_then_replace(source: str, target: Path) -> None:
            both.wait()
            replace(source, target)

        def writer() -> None:
            try:
                ci_log.store(path, RAW_LOG)
            except BaseException as error:
                errors.append(error)

        with patch.object(ci_log.os, "replace", side_effect=meet_then_replace):
            threads = [threading.Thread(target=writer) for _ in range(2)]
            for thread in threads:
                thread.start()
            for thread in threads:
                thread.join()
        self.assertEqual(errors, [])
        self.assertEqual(path.read_text(), RAW_LOG)
        self.assertEqual(list(path.parent.iterdir()), [path])

    def test_failed_write_leaves_no_temporary_file(self) -> None:
        path = self.cache / "example/fork/7/attempt-1/9.log"
        with (
            patch.object(ci_log.os, "replace", side_effect=OSError("disk full")),
            self.assertRaises(OSError),
        ):
            ci_log.store(path, RAW_LOG)
        self.assertEqual(list(path.parent.iterdir()), [])

    def test_attempt_url_reads_that_attempt_not_the_latest(self) -> None:
        first = log_job()
        rerun = log_job(19, attempt=2, conclusion="success", steps=passed(first))
        runs = {
            "actions/runs/7": log_run(attempt=2, conclusion="success"),
            "actions/runs/7/attempts/1": log_run(),
        }

        def gh(*args: str) -> str:
            jobs = [first] if "/attempts/1/jobs" in args[-1] else [rerun]
            return json.dumps([{"jobs": jobs}])

        url = "https://github.com/example/fork/actions/runs/7/attempts/1"
        with (
            patch.object(ci, "api", side_effect=lambda _, path: runs[path]),
            patch.object(ci, "gh", side_effect=gh),
            patch.object(ci_log, "download", return_value=RAW_LOG),
            patch("sys.argv", ["ci.py", "log", url]),
            patch("builtins.print") as printed,
        ):
            self.assertEqual(ci.main(), 0)
        output = "\n".join(str(c.args[0]) for c in printed.call_args_list)
        self.assertIn("Run 7 attempt 1 (failure)", output)
        self.assertIn("Failing step: 2 Test (failure)", output)
        self.assertTrue((self.cache / "example/fork/7/attempt-1/9.log").exists())

    def test_cancelled_job_without_a_failed_step_shows_its_tail(self) -> None:
        lost = log_job(conclusion="cancelled", steps=passed(log_job()))
        _, output, _ = self.log("log", "7", jobs=[lost])
        self.assertIn("== native / test (cancelled)", output)
        self.assertIn("No failed step: the job ended cancelled", output)
        self.assertIn("Post job cleanup", output)

    def test_running_jobs_are_named_and_have_no_log_yet(self) -> None:
        running = log_job(
            10,
            conclusion=None,
            steps=[],
            name="native / simulation (1)",
        )
        run = log_run(status="in_progress", conclusion=None)
        _, output, _ = self.log("log", "7", jobs=[log_job(), running], run=run)
        self.assertIn("Run 7 attempt 1 (in_progress)", output)
        self.assertIn(
            "1 jobs still running: the failures shown may not be the last", output
        )
        self.assertIn("docs/workflow.md (Removal)", output)
        self.assertIn("Failing step: 2 Test", output)
        _, output, downloads = self.log(
            "log", "7", "--job", "simulation", jobs=[log_job(), running], run=run
        )
        self.assertIn(
            "No log: the job is still running; GitHub serves no log yet", output
        )
        self.assertIn(running["html_url"], output)
        self.assertEqual(downloads, [])
        _, output, _ = self.log("log", "7", jobs=[running], run=run)
        self.assertIn("No failed job yet", output)

    def test_targets_name_a_run_a_job_or_a_request(self) -> None:
        url = "https://github.com/example/fork/actions/runs/7/attempts/2"
        args = argparse.Namespace(pr=None, target=url, repo=None, profile=None)
        self.assertEqual(ci.log_target(args), ci.LogTarget("example/fork", 7, 2))
        args.target = JOB_URL
        self.assertEqual(ci.log_target(args), ci.LogTarget("example/fork", 7, None, 9))
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "request.json"
            ci.write_request(
                path, {"schema_version": 1, "repository": "example/fork", "run_id": 7}
            )
            args.target = str(path)
            with patch.object(ci, "api", return_value=self.RUN):
                self.assertEqual(ci.log_target(args), ci.LogTarget("example/fork", 7))
        args.target = None
        with self.assertRaisesRegex(ValueError, "python3 tools/ci.py log --pr N"):
            ci.log_target(args)

    def test_pr_names_the_run_it_selected(self) -> None:
        rust = requested("rust", "c" * 32, "2026-11-12T02:00:00Z")
        group = {
            "id": 3,
            "head_branch": f"gh-readonly-queue/rust-migration/pr-170-{BASE}",
            "head_sha": MERGE,
            "created_at": "2026-11-12T03:00:00Z",
            "run_attempt": 2,
            "html_url": "https://github.com/example/fork/actions/runs/3",
        }
        answers = {
            "pulls/170": dict(PULL, created_at="2026-11-11T00:00:00Z"),
            f"commits/{MERGE}": {"parents": [{"sha": PARENT}, {"sha": PREVIOUS}]},
        }
        with (
            patch.object(ci, "api", side_effect=lambda _, path: answers[path]),
            patch.object(
                ci,
                "dispatched_runs",
                return_value=[ci.DispatchedRun(rust, "rust", f"{'c' * 32}-{HEAD}")],
            ),
            patch.object(ci, "queue_runs", return_value=[group]),
            patch("builtins.print") as printed,
        ):
            self.assertEqual(ci.pr_run("example/fork", 170, None), 3)
            self.assertEqual(ci.pr_run("example/fork", 170, "rust"), 2)
        lines = [str(c.args[0]) for c in printed.call_args_list]
        self.assertEqual(
            lines[0],
            "Selected the newest merge-group run for PR #170: run 3 attempt 2, "
            f"tested head {PREVIOUS} (PR head {HEAD}): "
            "https://github.com/example/fork/actions/runs/3",
        )
        self.assertIn("Also found: requested rust run 2", lines[1])
        self.assertIn(
            f"requested rust run for PR #170: run 2 attempt 1, tested head {HEAD}",
            lines[2],
        )

    def test_failed_wait_prints_the_summary_and_keeps_its_status(self) -> None:
        record = {"repository": "example/fork", "profile": "native", "head": HEAD}
        failing = dict(self.RUN, status="in_progress", failed_jobs=[log_job()])
        argv = ["ci.py", "wait", "request.json"]
        for summary, expected in (
            (["Run 7 attempt 1 (failure)", "Failing step: 2 Test"], "Failing step"),
            (
                subprocess.CalledProcessError(1, ["gh"], "", "HTTP 502"),
                "Could not read",
            ),
        ):
            with (
                patch.object(ci, "status", return_value=(record, failing)),
                patch.object(ci, "log_lines", side_effect=[summary]),
                patch("sys.argv", argv),
                patch("builtins.print") as printed,
            ):
                self.assertEqual(ci.main(), 1)
            output = "\n".join(str(c.args[0]) for c in printed.call_args_list)
            self.assertIn("Failed child jobs: native / test", output)
            self.assertIn(expected, output.splitlines()[-1])
        # A failure that is not the run's (a moved head) prints no summary.
        with (
            patch.object(ci, "status", side_effect=ValueError("PR head changed")),
            patch.object(ci, "log_lines") as lines,
            patch("sys.argv", argv),
            patch("builtins.print"),
        ):
            self.assertEqual(ci.main(), 1)
        lines.assert_not_called()


ACCOUNT = "fork-bot"
ISSUES = "https://api.github.com/repos/example/fork/issues/"


def attribution(agent: str) -> str:
    return f"Agent: {agent} | Model: m-1 | Reasoning effort: high"


class FakeComments:
    """Comments on example/fork #170 (a PR) as gh answers for them: listed over
    two pages, read by ID, patched and posted. `post` may land and then fail."""

    def __init__(self, bodies: list[tuple[str, str]], post: str = "ok") -> None:
        self.store = [
            self.comment(1000 + i, body, author)
            for i, (body, author) in enumerate(bodies)
        ]
        self.post = post
        self.writes: list[tuple[str, str, dict[str, str]]] = []
        self.reads_by_id: dict[int, status_comment.Comment] = {}

    @staticmethod
    def comment(
        number: int,
        body: str,
        author: str = ACCOUNT,
        issue: int = 170,
        updated: str = "2026-11-12T01:00:00Z",
    ) -> status_comment.Comment:
        return {
            "id": number,
            "body": body,
            "user": {"login": author},
            "issue_url": f"{ISSUES}{issue}",
            "html_url": f"https://github.com/example/fork/pull/{issue}#issuecomment-{number}",
            "updated_at": updated,
        }

    def by_id(self, number: int) -> status_comment.Comment:
        if number in self.reads_by_id:
            return self.reads_by_id[number]
        (found,) = [c for c in self.store if c["id"] == number]
        return found

    def gh(
        self, *args: str, payload: str | None = None, timeout: float | None = None
    ) -> str:
        path = next(a for a in args if a.startswith("repos/") or a == "user")
        if path == "user":
            return ACCOUNT + "\n"
        if "--method" in args:
            return self.write(
                args[args.index("--method") + 1], path, json.loads(payload or "{}")
            )
        if path.endswith("/comments?per_page=100"):
            return json.dumps([self.store[:1], self.store[1:]])
        if match := re.search(r"/issues/comments/(\d+)$", path):
            return json.dumps(self.by_id(int(match[1])))
        if path.endswith("/issues/170"):
            return json.dumps({"number": 170, "pull_request": {}})
        if path.endswith("/issues/171"):
            return json.dumps({"number": 171})
        raise AssertionError(args)

    def write(self, method: str, path: str, data: dict[str, str]) -> str:
        self.writes.append((method, path, data))
        if method == "PATCH":
            comment = self.by_id(int(path.rsplit("/", 1)[1]))
            comment["body"] = data["body"]
            comment["updated_at"] = "2026-11-12T02:00:00Z"
            return json.dumps(comment)
        if self.post == "fails":
            raise subprocess.CalledProcessError(1, ["gh"], "", "HTTP 502")
        comment = self.comment(2000 + len(self.store), data["body"])
        self.store.append(comment)
        if self.post == "times out":
            raise subprocess.TimeoutExpired(["gh"], 60)
        return json.dumps(comment)


class CommentTests(unittest.TestCase):
    """ci.py comment against recorded comments (P4)."""

    def run_comment(
        self, github: FakeComments, *argv: str, body: str | None = None
    ) -> tuple[int, str, str]:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "status.md"
            if body is not None:
                path.write_text(body)
                argv = (*argv, "--body-file", str(path))
            with (
                patch.object(status_comment, "gh", side_effect=github.gh),
                patch(
                    "sys.argv", ["ci.py", "comment", *argv, "--repo", "example/fork"]
                ),
                patch("sys.stdout", new_callable=io.StringIO) as out,
                patch("sys.stderr", new_callable=io.StringIO) as err,
            ):
                code = ci.main()
        return code, out.getvalue(), err.getvalue()

    def test_lone_review_report_is_neither_chosen_nor_overwritten(self) -> None:
        report = review(HEAD, "/root/rev", "Disposition: accepted, whole PR")
        github = FakeComments([(report, ACCOUNT)])
        code, out, _ = self.run_comment(github, "170", "--agent", "/root/rev", "--show")
        self.assertEqual(code, 0)
        self.assertIn("No status comment for /root/rev on PR #170.", out)
        self.assertIn("1000 (review report, not adoptable)", out)
        self.assertNotIn("--comment-id", out)
        code, _, err = self.run_comment(
            github, "170", "--agent", "/root/rev", "--comment-id", "1000"
        )
        self.assertEqual(code, 1)
        self.assertIn("is a review report or cleanup acceptance", err)
        code, out, _ = self.run_comment(
            github,
            "170",
            "--agent",
            "/root/rev",
            "--model",
            "m-1",
            "--effort",
            "high",
            body="Status: re-check started",
        )
        self.assertEqual(code, 0)
        self.assertEqual([w[0] for w in github.writes], ["POST"])
        self.assertEqual(github.store[0]["body"], report)
        self.assertEqual(
            github.store[1]["body"],
            "<!-- status comment: /root/rev -->\nStatus: re-check started\n\n"
            + attribution("/root/rev"),
        )
        self.assertIn("Show it again: python3 tools/ci.py comment 170 --repo", out)

    def test_marked_comment_is_replaced_and_found_again(self) -> None:
        marked = "<!-- status comment: /root/a -->\nStatus: old\n\n" + attribution(
            "/root/a"
        )
        github = FakeComments([("unrelated", "someone"), (marked, ACCOUNT)])
        new = "Status: new\n\n" + attribution("/root/a")
        code, out, _ = self.run_comment(github, "170", "--agent", "/root/a", body=new)
        self.assertEqual(code, 0)
        self.assertEqual(
            github.writes[0][:2], ("PATCH", "repos/example/fork/issues/comments/1001")
        )
        self.assertEqual(
            github.store[1]["body"], "<!-- status comment: /root/a -->\n" + new
        )
        self.assertIn("#issuecomment-1001", out)
        code, out, err = self.run_comment(github, "170", "--agent", "/root/a", "--show")
        self.assertEqual(out.strip(), new)
        self.assertIn("Status comment: https://github.com/example/fork/pull/170", err)

    def test_change_since_the_first_read_refuses_the_patch(self) -> None:
        marked = "<!-- status comment: /root/a -->\nStatus\n\n" + attribution("/root/a")
        github = FakeComments([(marked, ACCOUNT)])
        github.reads_by_id[1000] = FakeComments.comment(
            1000, marked, updated="2026-11-12T03:00:00Z"
        )
        code, _, err = self.run_comment(
            github,
            "170",
            "--agent",
            "/root/a",
            body="Status: mine\n" + attribution("/root/a"),
        )
        self.assertEqual(code, 1)
        self.assertIn("changed since it was read", err)
        self.assertEqual(github.writes, [])

    def test_several_marked_comments_are_listed_not_chosen(self) -> None:
        marked = "<!-- status comment: /root/a -->\nStatus\n\n" + attribution("/root/a")
        github = FakeComments([(marked, ACCOUNT), (marked, ACCOUNT)])
        for extra in (("--show",), ("--model", "m-1", "--effort", "high")):
            code, _, err = self.run_comment(
                github,
                "170",
                "--agent",
                "/root/a",
                *extra,
                body=None if extra[0] == "--show" else "x",
            )
            self.assertEqual(code, 1)
            self.assertIn("2 status comments for /root/a on PR #170: 1000 ", err)
            self.assertIn("#issuecomment-1001", err)
        self.assertEqual(github.writes, [])

    def test_agent_paths_match_exactly(self) -> None:
        marked = "<!-- status comment: /root/ab -->\nStatus\n\n" + attribution(
            "/root/ab"
        )
        github = FakeComments(
            [(marked, ACCOUNT), ("Old status\n" + attribution("/root/ab"), ACCOUNT)]
        )
        code, out, _ = self.run_comment(github, "170", "--agent", "/root/a", "--show")
        self.assertIn("No status comment for /root/a", out)
        self.assertNotIn("1001", out)
        code, _, err = self.run_comment(
            github, "170", "--agent", "/root/a", "--comment-id", "1001"
        )
        self.assertIn("has attribution for ['/root/ab'], not /root/a", err)
        code, _, err = self.run_comment(
            github, "170", "--agent", "/root/a", body="x\n" + attribution("/root/ab")
        )
        self.assertIn("attribution names /root/ab, not /root/a", err)
        code, _, _ = self.run_comment(
            github,
            "170",
            "--agent",
            "/root/a",
            "--model",
            "m-1",
            "--effort",
            "high",
            body="x",
        )
        self.assertEqual(code, 0)
        self.assertEqual([w[0] for w in github.writes], ["POST"])
        self.assertTrue(
            github.store[0]["body"].startswith("<!-- status comment: /root/ab -->")
        )

    def test_forged_or_conflicting_attribution_is_refused(self) -> None:
        forged = "<!-- status comment: /root/a -->\nForged\n\n" + attribution("/root/a")
        github = FakeComments([(forged, "someone-else")])
        refusals = {
            "x\n"
            + attribution("/root/a")
            + "\n"
            + attribution("/root/a"): "exactly one",
            "x\nAgent: /root/a | Model: m-1": "exactly one",
            "<!-- status comment: /root/b -->\nx\n" + attribution("/root/a"): "marker",
            "x": "pass --model and --effort",
        }
        for body, reason in refusals.items():
            code, _, err = self.run_comment(
                github, "170", "--agent", "/root/a", body=body
            )
            self.assertEqual(code, 1, body)
            self.assertIn(reason, err)
        code, _, err = self.run_comment(
            github,
            "170",
            "--agent",
            "/root/a",
            "--model",
            "m-2",
            body="x\n" + attribution("/root/a"),
        )
        self.assertIn("conflicts with the body's line", err)
        self.assertEqual(github.writes, [])
        # Another account's marker is not this agent's status comment.
        code, _, _ = self.run_comment(
            github, "170", "--agent", "/root/a", body="Mine\n" + attribution("/root/a")
        )
        self.assertEqual([w[0] for w in github.writes], ["POST"])
        self.assertEqual(github.store[0]["body"], forged)

    def test_adoption_checks_place_author_and_content(self) -> None:
        legacy = "Status: started\n\n" + attribution("/root/a")
        github = FakeComments([(legacy, ACCOUNT), (legacy, "someone-else")])
        github.reads_by_id[1002] = FakeComments.comment(1002, legacy, issue=171)
        code, _, err = self.run_comment(
            github, "170", "--agent", "/root/a", "--comment-id", "1002"
        )
        self.assertIn(f"is on {ISSUES}171, not PR #170", err)
        code, _, err = self.run_comment(
            github, "170", "--agent", "/root/a", "--comment-id", "1001"
        )
        self.assertIn(f"was written by someone-else, not {ACCOUNT}", err)
        self.assertEqual(github.writes, [])
        code, out, _ = self.run_comment(github, "170", "--agent", "/root/a", "--show")
        self.assertIn("1000 (comment)", out)
        self.assertNotIn("1001", out)
        self.assertIn("--comment-id 1000", out)
        code, _, _ = self.run_comment(
            github, "170", "--agent", "/root/a", "--comment-id", "1000"
        )
        self.assertEqual(code, 0)
        self.assertEqual(
            github.store[0]["body"], "<!-- status comment: /root/a -->\n" + legacy
        )
        code, out, _ = self.run_comment(github, "170", "--agent", "/root/a", "--show")
        self.assertEqual(out.strip(), legacy)

    def test_create_that_times_out_after_landing_is_found_not_repeated(self) -> None:
        github = FakeComments([], post="times out")
        body = "Status\n" + attribution("/root/a")
        code, out, _ = self.run_comment(github, "170", "--agent", "/root/a", body=body)
        self.assertEqual(code, 0)
        self.assertIn("did not answer", out)
        self.assertIn("#issuecomment-2000", out)
        self.assertEqual([w[0] for w in github.writes], ["POST"])
        failing = FakeComments([], post="fails")
        code, _, err = self.run_comment(failing, "170", "--agent", "/root/a", body=body)
        self.assertEqual(code, 1)
        self.assertIn("none is on PR #170; run the same command again", err)
        self.assertEqual([w[0] for w in failing.writes], ["POST"])

    def test_show_with_nothing_to_show_creates_nothing(self) -> None:
        github = FakeComments([])
        code, out, _ = self.run_comment(github, "170", "--agent", "/root/a", "--show")
        self.assertEqual(code, 0)
        self.assertIn("No status comment for /root/a on PR #170.", out)
        self.assertIn("--body-file FILE --model MODEL --effort EFFORT", out)
        self.assertEqual(github.writes, [])
        code, _, err = self.run_comment(github, "171", "--agent", "/root/a", "--show")
        self.assertIn("#171 is an issue; name it with --issue", err)
        code, out, _ = self.run_comment(
            github, "--issue", "171", "--agent", "/root/a", "--show"
        )
        self.assertIn("on issue #171", out)

    def test_a_plan_review_is_not_adopted_as_a_status_comment(self) -> None:
        """`Reviewed plan:` is a report header too (#1063), in every form the
        report writer accepts."""
        plan = (
            "Reviewed plan: #171 at 1007aa96f8\n\n"
            + attribution("/root/rev")
            + "\n\nplan accepted"
        )
        bold = plan.replace("Reviewed plan:", "**Reviewed plan:**")
        self.assertEqual(review_report.reviewed_plan(bold), (171, "1007aa96f8"))
        self.assertTrue(status_comment.is_report(bold))
        github = FakeComments([])
        github.store.append(FakeComments.comment(1000, plan, issue=171))
        code, _, err = self.run_comment(
            github, "--issue", "171", "--agent", "/root/rev", "--comment-id", "1000"
        )
        self.assertEqual(code, 1)
        self.assertIn("is a review report or cleanup acceptance", err)
        self.assertEqual(github.writes, [])


class ReportTests(unittest.TestCase):
    """ci.py report: post or replace a review report from a file (#1063)."""

    REPORT = review(HEAD, "/root/rev", "Disposition: accepted, whole PR behaviour")

    def run_report(
        self, github: FakeComments, *argv: str, body: str | None = None
    ) -> tuple[int, str, str]:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.md"
            if body is not None:
                path.write_text(body)
                argv = (*argv, "--body-file", str(path))
            with (
                patch.object(status_comment, "gh", side_effect=github.gh),
                # These reports sign m-1; ReviewerTests holds the model check.
                patch.object(review_start, "review_models", return_value={"m-1"}),
                patch.object(
                    review_start, "review_pairs", return_value={("m-1", "high")}
                ),
                patch.object(review_start, "running_model", return_value=None),
                patch("sys.argv", ["ci.py", "report", *argv, "--repo", "example/fork"]),
                patch("sys.stdout", new_callable=io.StringIO) as out,
                patch("sys.stderr", new_callable=io.StringIO) as err,
            ):
                code = ci.main()
        return code, out.getvalue(), err.getvalue()

    def test_no_report_posts_the_body_unchanged(self) -> None:
        github = FakeComments([])
        code, out, _ = self.run_report(
            github, "170", "--agent", "/root/rev", body=self.REPORT + "\n"
        )
        self.assertEqual(code, 0)
        self.assertEqual(
            github.writes,
            [
                (
                    "POST",
                    "repos/example/fork/issues/170/comments",
                    {"body": self.REPORT + "\n"},
                )
            ],
        )
        self.assertIn("#issuecomment-2000", out)
        code, out, _ = self.run_report(github, "170", "--agent", "/root/rev", "--show")
        self.assertEqual(out, self.REPORT + "\n\n")

    def test_one_report_is_patched_unless_it_changed_since_read(self) -> None:
        old = review(HEAD, "/root/rev", "Disposition: findings to remediate, whole PR")
        status = "<!-- status comment: /root/rev -->\nStatus\n" + attribution(
            "/root/rev"
        )
        other = review(HEAD, "/root/other", "Disposition: accepted, whole PR porting")
        github = FakeComments([(other, ACCOUNT), (status, ACCOUNT), (old, ACCOUNT)])
        code, _, _ = self.run_report(
            github, "170", "--agent", "/root/rev", body=self.REPORT
        )
        self.assertEqual(code, 0)
        self.assertEqual(
            github.writes,
            [
                (
                    "PATCH",
                    "repos/example/fork/issues/comments/1002",
                    {"body": self.REPORT},
                )
            ],
        )
        # Another agent's report and the agent's own status comment are left alone.
        self.assertEqual([c["body"] for c in github.store[:2]], [other, status])
        changed = FakeComments([(old, ACCOUNT)])
        changed.reads_by_id[1000] = FakeComments.comment(
            1000, old, updated="2026-11-12T03:00:00Z"
        )
        code, _, err = self.run_report(
            changed, "170", "--agent", "/root/rev", body=self.REPORT
        )
        self.assertEqual(code, 1)
        self.assertIn("changed since it was read", err)
        self.assertEqual(changed.writes, [])

    def test_two_reports_by_the_agent_are_refused_and_listed(self) -> None:
        github = FakeComments([(self.REPORT, ACCOUNT), (self.REPORT, ACCOUNT)])
        code, _, err = self.run_report(
            github, "170", "--agent", "/root/rev", body=self.REPORT
        )
        self.assertEqual(code, 1)
        self.assertIn("2 reports by /root/rev on PR #170: 1000", err)
        self.assertIn("#issuecomment-1001", err)
        self.assertEqual(github.writes, [])

    def test_a_body_the_readers_cannot_read_is_refused_before_any_write(self) -> None:
        fenced = (
            f"Reviewed commit: {HEAD}\n```\n{attribution('/root/rev')}\n```\n"
            "Disposition: accepted, whole PR"
        )
        for body, refusal in (
            ("Findings: none\n" + self.REPORT, "opens with `Reviewed commit: <sha>`"),
            (
                review(HEAD, "/root/other", "Disposition: accepted"),
                "has ['/root/other']",
            ),
            (fenced, "has none"),
            # An attribution inside a sentence or a four-backtick fence is none
            # (review of #1065).
            (
                f"Reviewed commit: {HEAD}\nAs {attribution('/root/rev')} said.\n"
                "Disposition: accepted, whole PR",
                "has none",
            ),
            (fenced.replace("```", "````"), "has none"),
            # A mixed closer does not close a fence, and an unclosed fence runs
            # to the end (re-check of #1065).
            (
                f"Reviewed commit: {HEAD}\n````\n````~\n{attribution('/root/rev')}\n"
                "````\nDisposition: accepted, whole PR",
                "has none",
            ),
            (
                f"Reviewed commit: {HEAD}\n```\n{attribution('/root/rev')}\n"
                "Disposition: accepted, whole PR",
                "has none",
            ),
            (review(HEAD, "/root/rev", "Looks fine."), "gives no disposition"),
            (
                f"Reviewed plan: #170 at {HEAD}\n{attribution('/root/rev')}\nplan accepted",
                "opens with `Reviewed commit: <sha>`",
            ),
        ):
            github = FakeComments([])
            with self.subTest(refusal):
                code, _, err = self.run_report(
                    github, "170", "--agent", "/root/rev", body=body
                )
                self.assertEqual(code, 1)
                self.assertIn(refusal, err)
                self.assertEqual(github.writes, [])
        # A fenced attribution alone cannot make a report of unknown authorship.
        self.assertEqual(review_report.agent(fenced), review_report.UNKNOWN)

    def test_a_plan_review_goes_on_its_issue(self) -> None:
        plan = f"Reviewed plan: #171 at {HEAD}\n\n{attribution('/root/rev')}\n\nplan accepted"
        github = FakeComments([])
        code, _, err = self.run_report(
            github, "--issue", "171", "--agent", "/root/rev", body=plan
        )
        self.assertEqual((code, err), (0, ""))
        self.assertEqual(
            github.writes,
            [("POST", "repos/example/fork/issues/171/comments", {"body": plan})],
        )
        for body in (
            plan.replace("#171", "#170"),
            plan.replace("plan accepted", "accepted"),
            plan.replace("plan accepted", "plan acceptedly"),
        ):
            code, _, _ = self.run_report(
                FakeComments([]), "--issue", "171", "--agent", "/root/rev", body=body
            )
            self.assertEqual(code, 1)

    def test_a_create_that_does_not_answer_finds_the_landed_report(self) -> None:
        github = FakeComments([], post="times out")
        code, out, _ = self.run_report(
            github, "170", "--agent", "/root/rev", body=self.REPORT
        )
        self.assertEqual(code, 0)
        self.assertIn("did not answer", out)
        self.assertIn("#issuecomment-2000", out)
        self.assertEqual([w[0] for w in github.writes], ["POST"])


@dataclass
class ReviewerFixture:
    """What GitHub holds for example/fork #170 beyond its reports."""

    models: dict[str, str] = field(default_factory=dict)
    extra: list[status_comment.Comment] = field(default_factory=list)
    submitted: list[review_report.Review] = field(default_factory=list)
    files: tuple[str, ...] = ("rust/openttd-kernels/src/x.rs",)


class ReviewerTests(unittest.TestCase):
    """ci.py reviewer: the review row's call within the review limits, and the
    report check that refuses a report off the review row (#1091)."""

    URL = "https://github.com/example/fork/pull/170"

    def report(
        self, number: int, agent: str, model: str = "gpt-6.2-sol"
    ) -> status_comment.Comment:
        body = (
            f"Reviewed commit: {HEAD}\n"
            f"Agent: /root/{agent} | Model: {model} | Reasoning effort: high\n"
            "Disposition: findings to remediate, whole PR"
        )
        return FakeComments.comment(
            number, body, updated=f"2026-11-25T2{number % 10}:00:00Z"
        )

    def run_reviewer(
        self,
        agents: list[str],
        *argv: str,
        ledger: Path,
        pr: ReviewerFixture | None = None,
    ) -> tuple[int, str, str]:
        pr = pr or ReviewerFixture()
        models, submitted, files = pr.models, pr.submitted, pr.files
        comments = [
            self.report(1000 + i, a, models.get(a, "gpt-6.2-sol"))
            for i, a in enumerate(agents)
        ] + pr.extra

        def gh(
            *args: str, payload: str | None = None, timeout: float | None = None
        ) -> str:
            path = next(a for a in args if a.startswith("repos/"))
            if path.endswith("/pulls/170"):
                return json.dumps({"head": {"sha": HEAD}, "html_url": self.URL})
            if path.endswith("/issues/170/comments?per_page=100"):
                return json.dumps([comments])
            if path.endswith("/pulls/170/reviews?per_page=100"):
                return json.dumps([submitted])
            if path.endswith("/pulls/170/files?per_page=100"):
                return json.dumps([[{"filename": name} for name in files]])
            raise AssertionError(args)

        with (
            patch.object(status_comment, "gh", side_effect=gh),
            patch.object(review_start, "LEDGER", ledger),
            patch.dict(os.environ, {"CLAUDECODE": "1"}),
            patch(
                "sys.argv",
                ["ci.py", "reviewer", "170", *argv, "--repo", "example/fork"],
            ),
            patch("sys.stdout", new_callable=io.StringIO) as out,
            patch("sys.stderr", new_callable=io.StringIO) as err,
        ):
            try:
                code = ci.main()
            except SystemExit as stop:
                code = int(stop.code or 0)
        return code, out.getvalue(), err.getvalue()

    def test_the_review_row_comes_from_the_role_table(self) -> None:
        row = review_start.review_row()
        self.assertEqual(set(row.values()), {("gpt-6.2-sol", "high")})
        self.assertEqual(
            review_start.target("gpt-6.2-sol", "high"),
            {
                "providerInstanceId": "codex",
                "model": "gpt-6.2-sol",
                "options": {"reasoningEffort": "high"},
            },
        )

    def test_stages_are_read_from_names_old_and_new(self) -> None:
        for name, stage in (
            ("review-170-behaviour", "behaviour"),
            ("review-170-porting-2", "porting"),
            ("review-170-pr-behaviour", "behaviour"),
            ("review-170-recheck", "recheck"),
            ("review-170-recheck-2", "recheck"),
            ("review-170-records-recheck", "recheck"),
            ("review-170-base-behaviour", "base"),
            ("review-170-base85-behaviour", "base"),
            ("review-170-records-porting-3", "scoped"),
            ("review-170-porting-control", "scoped"),
        ):
            self.assertEqual(review_start.stage(170, "/root/" + name), stage, name)

    def test_the_first_call_is_on_the_review_row_with_the_fixed_message(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            code, out, err = self.run_reviewer([], "behaviour", ledger=Path(directory))
            self.assertEqual(code, 0, err)
            call = json.loads(out)
            self.assertEqual(call["target"]["model"], "gpt-6.2-sol")
            self.assertEqual(call["target"]["providerInstanceId"], "codex")
            self.assertEqual(call["isolation"], "worktree")
            self.assertEqual(
                call["task"],
                f"Review PR {self.URL} under `docs/review-brief.md` for behaviour. You are "
                "/root/review-170-behaviour, on gpt-6.2-sol at high effort. Review the whole PR.",
            )
            # The start is counted before its report lands: a second is refused.
            code, _, err = self.run_reviewer([], "behaviour", ledger=Path(directory))
            self.assertEqual(code, 1)
            self.assertIn("a second whole-PR behaviour review", err)

    def test_a_re_check_names_every_first_stage_report(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            agents = ["review-170-behaviour", "review-170-porting"]
            code, out, err = self.run_reviewer(
                agents, "recheck", "--commits", "aaa", "bbb", ledger=Path(directory)
            )
            self.assertEqual(code, 0, err)
            task = json.loads(out)["task"]
            self.assertIn(
                "You are /root/review-170-recheck, on gpt-6.2-sol at high effort.", task
            )
            self.assertIn("commits aaa bbb against the findings in", task)
            self.assertIn("#issuecomment-1000 ", task)
            self.assertIn("#issuecomment-1001.", task)
            code, _, err = self.run_reviewer(
                agents + ["review-170-recheck"],
                "recheck",
                "--commits",
                "c",
                ledger=Path(directory),
            )
            self.assertEqual(code, 1)
            self.assertIn("a second re-check", err)

    def test_a_third_scoped_review_goes_to_root_first(self) -> None:
        agents = [
            "review-170-behaviour",
            "review-170-porting",
            "review-170-recheck",
            "review-170-records-porting",
            "review-170-base-behaviour",
        ]
        with tempfile.TemporaryDirectory() as directory:
            ledger = Path(directory)
            code, out, err = self.run_reviewer(
                agents, "records", "--commits", "c1", ledger=ledger
            )
            self.assertEqual(code, 0, err)
            self.assertIn(
                "/root/review-170-records-porting-2,", json.loads(out)["task"]
            )
            # A base update's review does not count; the start above does.
            code, out, err = self.run_reviewer(
                agents, "records", "--commits", "c2", ledger=ledger
            )
            self.assertEqual(code, 1)
            self.assertEqual(out, "")
            self.assertIn("Send root this line", err)
            self.assertIn(
                f"PR #170, head {HEAD[:10]}: a records review of c2 would be a "
                "scoped review after 2 others",
                err,
            )
            self.assertIn(
                "(behaviour, porting, recheck, records-porting, base-behaviour, records-porting-2)",
                err,
            )
            self.assertEqual(len(json.loads((ledger / "170.json").read_text())), 1)
            self.assertEqual(
                self.run_reviewer(agents, "base", "--commits", "c2", ledger=ledger)[0],
                0,
            )
            code, out, _ = self.run_reviewer(
                agents,
                "records",
                "--commits",
                "c2",
                "--root",
                "approve",
                "--root-answer",
                "one more, then residue",
                ledger=ledger,
            )
            self.assertEqual(code, 0)
            self.assertIn(
                "/root/review-170-records-porting-3,", json.loads(out)["task"]
            )
            entries = json.loads((ledger / "170.json").read_text())
            self.assertEqual(entries[-1]["root"], "one more, then residue")

    def test_an_adr_limit_is_not_lifted_by_root(self) -> None:
        agents = ["review-170-behaviour", "review-170-recheck"]
        with tempfile.TemporaryDirectory() as directory:
            code, out, err = self.run_reviewer(
                agents,
                "recheck",
                "--commits",
                "c",
                "--root",
                "approve",
                "--root-answer",
                "residue",
                ledger=Path(directory),
            )
        self.assertEqual((code, out), (1, ""))
        self.assertIn("a second re-check", err)
        self.assertNotIn("Send root", err)

    def test_a_re_check_waits_for_every_first_stage_report(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            ledger = Path(directory)
            self.assertEqual(self.run_reviewer([], "porting", ledger=ledger)[0], 0)
            code, _, err = self.run_reviewer(
                ["review-170-behaviour"], "recheck", "--commits", "c", ledger=ledger
            )
            self.assertEqual(code, 1)
            self.assertIn("it needs a porting report", err)
            agents = ["review-170-behaviour", "review-170-porting"]
            code, _, err = self.run_reviewer(
                agents, "recheck", "--commits", "c", ledger=ledger
            )
            self.assertEqual(code, 0, err)

    def test_a_start_can_be_printed_again_or_released(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            ledger = Path(directory)
            _, first, _ = self.run_reviewer([], "behaviour", ledger=ledger)
            code, again, _ = self.run_reviewer(
                [], "behaviour", "--again", ledger=ledger
            )
            self.assertEqual((code, again), (0, first))
            self.assertEqual(len(json.loads((ledger / "170.json").read_text())), 1)
            code, out, _ = self.run_reviewer(
                [], "--release", "review-170-behaviour", ledger=ledger
            )
            self.assertEqual(code, 0)
            self.assertEqual(json.loads((ledger / "170.json").read_text()), [])
            self.assertEqual(self.run_reviewer([], "behaviour", ledger=ledger)[0], 0)
            # A start whose reviewer reported is a review, not a start to free.
            code, _, err = self.run_reviewer(
                ["review-170-behaviour"],
                "--release",
                "review-170-behaviour",
                ledger=ledger,
            )
            self.assertEqual(code, 1)
            self.assertIn("has reported", err)

    def test_a_ledger_it_cannot_read_is_named_not_taken_as_empty(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / "170.json").write_text("{not json")
            code, _, err = self.run_reviewer([], "behaviour", ledger=Path(directory))
        self.assertEqual(code, 1)
        self.assertIn("is not a ledger of starts", err)

    def test_reports_with_no_disposition_or_off_the_row_spend_nothing(self) -> None:
        open_ended = FakeComments.comment(
            1100,
            f"Reviewed commit: {HEAD}\n"
            "Agent: /root/review-170-behaviour | Model: gpt-6.2-sol | Reasoning effort: high\n"
            "Still reading.",
        )
        submitted: list[review_report.Review] = [
            {
                "html_url": "https://github.com/example/fork/pull/170#pullrequestreview-9",
                "state": "COMMENTED",
                "commit_id": HEAD,
                "submitted_at": "2026-11-25T20:00:00Z",
                "body": "Agent: /root/review-170-porting | Model: claude-opus-5-5 | "
                "Reasoning effort: medium\nDisposition: accepted, whole PR",
            }
        ]
        with tempfile.TemporaryDirectory() as directory:
            ledger = Path(directory)
            for kind in ("behaviour", "porting"):
                code, _, err = self.run_reviewer(
                    [],
                    kind,
                    ledger=ledger,
                    pr=ReviewerFixture(extra=[open_ended], submitted=submitted),
                )
                self.assertEqual(code, 0, err)

    def test_a_re_check_needs_the_first_stage_the_pr_shape_needs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            ledger = Path(directory)
            only = ["review-170-behaviour"]
            code, _, err = self.run_reviewer(
                only, "recheck", "--commits", "c", ledger=ledger
            )
            self.assertEqual(code, 1)
            self.assertIn("it needs a porting report", err)
            # A porting report off the review row is no porting report.
            both = ["review-170-behaviour", "review-170-porting"]
            off = {"review-170-porting": "claude-opus-5-5"}
            code, _, _ = self.run_reviewer(
                both,
                "recheck",
                "--commits",
                "c",
                ledger=ledger,
                pr=ReviewerFixture(models=off),
            )
            self.assertEqual(code, 1)
            # A PR with no game code has the behaviour review alone.
            code, _, err = self.run_reviewer(
                only,
                "recheck",
                "--commits",
                "c",
                ledger=ledger,
                pr=ReviewerFixture(files=("tools/x.py",)),
            )
            self.assertEqual(code, 0, err)

    def test_root_must_approve_in_so_many_words(self) -> None:
        agents = ["review-170-records-porting", "review-170-records-porting-2"]
        with tempfile.TemporaryDirectory() as directory:
            code, _, _ = self.run_reviewer(
                agents,
                "records",
                "--commits",
                "c",
                "--root",
                "record it as residue and queue",
                ledger=Path(directory),
            )
            self.assertEqual(code, 2)
            code, _, err = self.run_reviewer(
                agents,
                "records",
                "--commits",
                "c",
                "--root",
                "approve",
                ledger=Path(directory),
            )
            self.assertEqual(code, 1)
            self.assertIn("--root-answer", err)

    def test_an_ambiguous_retry_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            ledger = Path(directory)
            for commit in ("a", "b"):
                self.run_reviewer([], "records", "--commits", commit, ledger=ledger)
            code, _, err = self.run_reviewer([], "records", "--again", ledger=ledger)
            self.assertEqual(code, 1)
            self.assertIn("name the one whose dispatch failed", err)
            code, out, _ = self.run_reviewer(
                [], "records", "--again", "review-170-records-porting", ledger=ledger
            )
            self.assertEqual(code, 0)
            self.assertIn("commits a.", json.loads(out)["task"])

    def test_a_ledger_entry_without_a_usable_call_is_refused(self) -> None:
        entry = {
            "name": "review-170-behaviour",
            "kind": "behaviour",
            "at": "x",
            "call": None,
        }
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / "170.json").write_text(json.dumps([entry]))
            code, _, err = self.run_reviewer(
                [], "behaviour", "--again", ledger=Path(directory)
            )
        self.assertEqual(code, 1)
        self.assertIn("is not a ledger of starts", err)

    def test_a_comment_with_two_attributions_is_no_review(self) -> None:
        two = FakeComments.comment(
            1200,
            f"Reviewed commit: {HEAD}\n"
            "Agent: /root/review-170-behaviour | Model: gpt-6.2-sol | Reasoning effort: high\n"
            "Agent: /root/review-170-porting | Model: gpt-6.2-sol | Reasoning effort: high\n"
            "Disposition: accepted, whole PR",
        )
        with tempfile.TemporaryDirectory() as directory:
            code, _, err = self.run_reviewer(
                [], "behaviour", ledger=Path(directory), pr=ReviewerFixture(extra=[two])
            )
        self.assertEqual(code, 0, err)

    def test_a_void_review_counts_for_nothing(self) -> None:
        agents = ["review-170-behaviour", "review-170-porting"]
        models = {a: "claude-opus-5-5" for a in agents}
        with tempfile.TemporaryDirectory() as directory:
            code, out, err = self.run_reviewer(
                agents,
                "behaviour",
                ledger=Path(directory),
                pr=ReviewerFixture(models=models),
            )
            self.assertEqual(code, 0, err)
            self.assertIn("/root/review-170-behaviour-2,", json.loads(out)["task"])
            code, out, err = self.run_reviewer(
                agents,
                "--check",
                ledger=Path(directory),
                pr=ReviewerFixture(models=models),
            )
            self.assertIn("VOID, off the review row", out)
            self.assertIn("Past the limits: 0\n", out)
        chain = ["review-170-behaviour", "review-170-porting", "review-170-recheck"] + [
            f"review-170-records-porting-{n}" for n in (1, 2, 3, 4)
        ]
        with tempfile.TemporaryDirectory() as directory:
            _, out, _ = self.run_reviewer(chain, "--check", ledger=Path(directory))
            self.assertIn(
                "Past the limits: 2 (review-170-records-porting-3, "
                "review-170-records-porting-4)",
                out,
            )
            self.assertFalse((Path(directory) / "171.json").exists())

    def test_a_report_off_the_review_row_is_refused(self) -> None:
        signed = (
            f"Reviewed commit: {HEAD}\n"
            "Agent: /root/rev | Model: claude-opus-5-5 | Reasoning effort: medium\n"
            "Disposition: accepted, whole PR"
        )
        right = signed.replace("claude-opus-5-5", "gpt-6.2-sol").replace(
            "medium", "high"
        )
        low = right.replace("effort: high", "effort: low")
        self.assertIn(
            "gpt-6.2-sol at Reasoning effort: low",
            review_start.report_refusal(low, environ={}) or "",
        )
        self.assertIn(
            "signed Model: claude-opus-5-5",
            review_start.report_refusal(signed, environ={}) or "",
        )
        self.assertIsNone(review_start.report_refusal(right, environ={}))
        with tempfile.TemporaryDirectory() as directory:
            proc = Path(directory)
            (proc / "42").mkdir()
            (proc / "42" / "cmdline").write_bytes(
                b"claude\0--effort=medium\0--model=claude-opus-5-5\0"
            )
            claude = {"CLAUDECODE": "1", "CLAUDE_PID": "42"}
            self.assertEqual(
                review_start.running_model(claude, proc), "claude-opus-5-5"
            )
            # Signing the review row's model does not hide the process's own.
            why = review_start.report_refusal(right, environ=claude, proc=proc)
            self.assertIn("you run on claude-opus-5-5", why or "")
            self.assertEqual(
                review_start.running_model(
                    {"CLAUDECODE": "1", "CLAUDE_PID": "7"}, proc
                ),
                "a Claude Code model",
            )
        github = FakeComments([])
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.md"
            path.write_text(signed)
            with (
                patch.object(status_comment, "gh", side_effect=github.gh),
                patch.object(review_start, "running_model", return_value=None),
                patch(
                    "sys.argv",
                    [
                        "ci.py",
                        "report",
                        "170",
                        "--agent",
                        "/root/rev",
                        "--body-file",
                        str(path),
                        "--repo",
                        "example/fork",
                    ],
                ),
                patch("sys.stdout", new_callable=io.StringIO),
                patch("sys.stderr", new_callable=io.StringIO) as err,
            ):
                self.assertEqual(ci.main(), 1)
        self.assertIn(
            "not posted: the report is signed Model: claude-opus-5-5", err.getvalue()
        )
        self.assertEqual(github.writes, [])


class FailureCountTests(unittest.TestCase):
    def test_count_leaves_out_control_branches_unless_asked(self) -> None:
        runs = [
            {
                "html_url": "u1",
                "name": "Quick validation",
                "event": "pull_request",
                "head_branch": "control/900-x",
            },
            {
                "html_url": "u2",
                "name": "Requested validation",
                "event": "workflow_dispatch",
                "head_branch": "rust-migration",
                "display_title": "CI native PR 901 r " + HEAD,
            },
            {
                "html_url": "u3",
                "name": "Requested validation",
                "event": "merge_group",
                "head_branch": f"gh-readonly-queue/rust-migration/pr-902-{BASE}",
            },
            {
                "html_url": "u4",
                "name": "Quick validation",
                "event": "pull_request",
                "head_branch": "tools/1-x",
            },
        ]
        refs = {"pulls/901": "control/901-y", "pulls/902": "tools/902-z"}

        def gh(*args: str) -> str:
            return json.dumps(
                [{"workflow_runs": runs if "status=failure" in args[-1] else []}]
            )

        for controls, expected in (
            (False, "2 failed runs since 2026-11-10, leaving out 2 on control/"),
            (True, "4 failed runs since 2026-11-10"),
        ):
            args = argparse.Namespace(
                repo="example/fork", since="2026-11-10", controls=controls
            )
            with (
                patch.object(ci, "gh", side_effect=gh),
                patch.object(
                    ci, "api", side_effect=lambda _, path: {"head": {"ref": refs[path]}}
                ),
                patch("builtins.print") as printed,
            ):
                ci.failures(args)
            self.assertEqual(printed.call_args.args[0], expected)


if __name__ == "__main__":
    unittest.main()
