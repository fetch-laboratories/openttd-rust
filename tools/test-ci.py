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
from pathlib import Path
from unittest.mock import patch

import ci
import yaml

ROOT = Path(__file__).resolve().parents[1]
WORKFLOWS = ROOT / ".github/workflows"
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


def workflow(filename):
    return yaml.safe_load((WORKFLOWS / filename).read_text())


def job(filename, name):
    return workflow(filename)["jobs"][name]


def script_for(filename, name):
    (step,) = [s for s in job(filename, name)["steps"] if "script" in s.get("with", {})]
    return step["with"]["script"]


def node(harness):
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


def evaluate(expression, *, cancelled=False, interpolate=False, **contexts):
    """Evaluate an `if:` expression, or interpolate `${{ }}` into a string."""
    if interpolate:
        parts = re.split(r"\$\{\{(.*?)\}\}", expression)
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
        f"const success = () => {json.dumps(succeeded)}, failure = () => {json.dumps(failed)};\n"
        + declarations
        + f"console.log(JSON.stringify({expression}));"
    )


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


def execute_script(filename, name, pull, *, env=None, event=None, latest=None):
    script = script_for(filename, name)
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
        "target_url": RUN.replace("123", "999") if superseded else RUN,
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


def run_policy(
    *,
    event="push",
    on_demand="true",
    files=(),
    trees=None,
    pulls=(),
    statuses=(),
    compare=None,
):
    """Run the actual policy script; `trees` maps the base and pushed tree SHAs."""
    script = script_for("ci-policy.yml", "policy")
    compare = compare or f"async () => ({{data: {{files: {json.dumps(files)}}}}})"
    return node(f"""
process.env.CI_ON_DEMAND = {json.dumps(on_demand)};
const outputs = {{}}, delays = [], trees = {json.dumps(trees or {})};
let calls = 0;
const setTimeout = (resolve, delay) => {{ delays.push(delay); resolve(); }};
const context = {{eventName: {json.dumps(event)}, repo: {{owner: 'example', repo: 'fork'}},
  serverUrl: 'https://github.com', sha: 'head',
  payload: {{before: 'base', pull_request: {{base: {{ref: 'rust-migration'}}}}}}}};
const core = {{setOutput: (k, v) => outputs[k] = v, info: () => {{}}, warning: () => {{}}}};
const github = {{paginate: async () => {json.dumps(list(pulls))}, rest: {{
  git: {{getTree: async args => ({{data: {{sha: args.tree_sha, ...trees[args.tree_sha]}}}})}},
  repos: {{
    getCommit: async args => ({{data: {{commit: {{tree: {{sha: args.ref === 'base' ? '{BASE}' : '{TREE}'}}}}}}}}),
    getCombinedStatusForRef: async () => ({{data: {{statuses: {json.dumps(list(statuses))}}}}}),
    compareCommitsWithBasehead: {compare}
  }}
}}}};
(async () => {{{script}}})().then(() => console.log(JSON.stringify({{outputs, calls, delays}})))
  .catch(error => console.log(JSON.stringify({{outputs, calls, delays, error: error.message}})));
""")


def entry(path, sha=HEAD, mode="100644", kind="blob"):
    return {"path": path, "type": kind, "mode": mode, "sha": sha}


def tree(*entries, **fields):
    return {"truncated": False, "tree": list(entries), **fields}


def validation(full="success", marker="success", run=RUN, marker_run=RUN, tree=TREE):
    """Full validation status and its Validated tree marker on a merged head."""
    return [
        {"context": "Full validation", "state": full, "target_url": run},
        {
            "context": "Validated tree",
            "state": marker,
            "target_url": marker_run,
            "description": tree,
        },
    ]


def runs(step, cache_only, steps):
    """Evaluate a ci-linux or ci-windows step condition on a protected push."""
    return evaluate(
        step.get("if", True),
        inputs={"cache-only": cache_only, "rust": True, "compiler": "gcc"},
        github={"event_name": "push", "ref": "refs/heads/rust-migration"},
        steps=steps,
    )


class GateTests(unittest.TestCase):
    def test_plan_checks_mergeability_and_marks_only_full_requests_pending(self):
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

    def test_privileged_jobs_run_only_for_protected_full_requests(self):
        plan = job("ci-request.yml", "plan")["if"]
        repo = {"repo": {"full_name": "fork"}}
        same = {"label": {"name": "ci:full"}, "pull_request": {"head": repo}}
        fork = dict(same, pull_request={"head": {"repo": {"full_name": "other"}}})
        dispatch, pull_request = "workflow_dispatch", "pull_request"
        for event, ref, on_demand, payload, expected in (
            (dispatch, "refs/heads/rust-migration", "true", {}, True),
            (dispatch, "refs/heads/feature", "true", {}, False),
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
        for result, profile, expected in (
            ("success", "full", True),
            ("success", "rust", False),
            ("failure", "full", False),
        ):
            needs = {"plan": {"result": result, "outputs": {"profile": profile}}}
            self.assertEqual(evaluate(publish, needs=needs), expected)

    def test_publisher_requires_every_job_and_the_unchanged_head(self):
        cases = [("success", good_needs(), PULL, False, "success")]
        cases.append(("superseded", good_needs(), PULL, True, None))
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

    def test_status_writers_share_a_non_cancelling_per_pr_concurrency_group(self):
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

    def test_invalidation_only_marks_current_unvalidated_head_pending(self):
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

    def test_write_jobs_never_check_out_or_run_candidate_code(self):
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
        prefixes = "compiler-v1-linux compiler-v1-linux ccache-linux-v2-gcc"
        prefixes += " ccache-macos-v2-arm64 ccache-msvc-v1-x64"
        caches = [
            {"id": i, "key": f"{prefix}-{str(i) * 40}", "created_at": f"2026-10-{i:02}"}
            | {"size_in_bytes": gib}
            for i, prefix in enumerate(prefixes.split(), start=1)
        ]
        self.assertEqual(self.prune(caches), {"deleted": [1], "failures": []})
        caches[-1]["size_in_bytes"] = 7 * gib
        self.assertEqual(self.prune(caches), {"deleted": [1, 2, 3], "failures": []})
        # Unmanaged caches are never removed.
        unmanaged = {"id": 99, "key": "unmanaged", "created_at": "date"}
        result = self.prune([unmanaged | {"size_in_bytes": 9 * gib}])
        self.assertEqual(result["deleted"], [])
        self.assertTrue(result["failures"])

    def test_pruning_follows_only_protected_push_runs(self):
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
            ("full", "merge", dispatch, "", "", ["sdl2"], ["x64"], False),
            ("native", "merge", dispatch, "", "", ["sdl2"], [], False),
            ("platform", "merge", dispatch, "", "", all_linux, both_windows, True),
            ("", "", dispatch, "", "", ["clang", "dedicated"], ["x86"], True),
            ("", "", "push", "false", "true", ["sdl2"], ["x64"], False),
            ("", "", "push", "true", "", all_linux, both_windows, True),
            ("", "", "pull_request", "true", "", ["sdl2"], ["x64"], True),
        ):
            contexts = {
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
                    bool(checkout and profile != "native" or heavy == "true"),
                )

    def test_platform_cache_refresh_is_only_a_covered_protected_push(self):
        build = workflow("ci-build.yml")["jobs"]
        protected = "refs/heads/rust-migration"
        for name in ("linux", "windows"):
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

    def test_cache_refresh_skips_exactly_the_test_and_evidence_steps(self):
        windows = {
            "Rust checks for actual selected target",
            "Test",
            "Fresh Rust native generators",
            "Validate refusals and actual archive freshness",
            "Retain native Windows Rust and test evidence",
        }
        for filename, name, skipped in (
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

    def test_protected_push_groups(self):
        def groups(filename, event="push"):
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
    ):
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
              return {{data: {{files: [{{filename: 'README.md'}}]}}}};
            }}"""
            with self.subTest(failure=failure, recover=recover):
                result = run_policy(trees={BASE: source, TREE: source}, compare=compare)
                self.assertEqual(result["calls"], expected_calls)
                self.assertEqual(result["outputs"], expected)
                self.assertEqual(result["delays"], [1000, 2000][: expected_calls - 1])
                self.assertEqual("error" in result, not expected)

    def test_bootstrap_on_demand_and_docs_classification(self):
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

    def test_docs_fast_path_checks_tree_types_and_complete_inventories(self):
        code = entry("src/game.cpp")
        link = entry("module.md", mode="160000", kind="commit")
        doc = entry("module.md")
        files = [{"filename": "module.md"}]
        renamed = [{"filename": "new.md", "previous_filename": "module.md"}]
        docs_before, docs_after = tree(code, doc), tree(code, dict(doc, sha=MERGE))
        cases = [
            (tree(code, link), tree(code, dict(link, sha=MERGE)), files, "true"),
            (tree(code), tree(code, dict(link, sha=MERGE)), files, "true"),
            (tree(code, link), tree(code), files, "true"),
            (tree(code, link), tree(code, dict(link, path="new.md")), renamed, "true"),
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

    def test_only_matching_successful_merged_tree_skips_validation(self):
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

    def test_markdown_tree_differences_require_complete_inventories(self):
        code, doc = entry("src/game.cpp"), entry("README.md", BASE)
        moved_doc = entry("docs/new.md", MERGE)
        before = tree(code, doc)
        cases = [
            (tree(moved_doc, code), "false"),
            (tree(code), "false"),
            (tree(dict(code, sha=MERGE)), "true"),
            (tree(dict(code, mode="100755")), "true"),
            (tree(dict(code, path="src/other.cpp")), "true"),
            (tree(dict(code, type="commit", mode="160000")), "true"),
            (tree(moved_doc), "true"),
            (tree(code, truncated=True), "true"),
            ({"tree": [code]}, "true"),
            ({"truncated": False}, "true"),
            (tree(), "true"),
            (tree({}), "true"),
            (tree(code, dict(doc, sha="bad")), "true"),
            (tree(code, dict(doc, mode="bad")), "true"),
            (tree(code, doc, doc), "true"),
            (tree(code, dict(doc, type="commit", mode="160000")), "true"),
            (tree(code, dict(doc, path="../a.md")), "true"),
            (tree(code, sha=None), "true"),
        ]
        for after, expected in cases:
            for incomplete_side in (False, True):
                left, right = (after, before) if incomplete_side else (before, after)
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

    def test_status_and_wait_report_failed_child_while_other_jobs_continue(self):
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
                _, observed = ci.status(path)
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
