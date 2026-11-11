#!/usr/bin/env python3
"""Seed independent cleanup gate failures, including relocations and constant data."""

import contextlib
import importlib
import importlib.util
import io
import json
import subprocess
import tempfile
import unittest
from collections import Counter
from collections.abc import Mapping
from functools import partial
from pathlib import Path
from types import ModuleType
from unittest.mock import patch

import cleanup_contract
from baseline_ratchet import debt_errors
from cleanup_contract import associated_pull, reviewed
from rust_syntax import boundary_ids
from source_fixture import initialize
from vcs import git


def load(name: str) -> ModuleType:
    spec = importlib.util.spec_from_file_location(
        name, Path(__file__).with_name(name + ".py")
    )
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


LINT = load("lint-ratchet")
POLICY = load("lint-policy")
GROWTH = load("growth-ratchet")
PROOF = load("refactor-proof")


class DebtTests(unittest.TestCase):
    def test_only_growth_fails(self) -> None:
        baseline = {"one item/site": [101, 102]}
        self.assertFalse(debt_errors(baseline, baseline, baseline))
        for grown in (
            {"other site": [101]},
            {"one item/site": [101, 103]},
            {"one item/site": [101, 102, 102]},
        ):
            self.assertTrue(debt_errors(grown, baseline, baseline))
        # Improvements and retired rows pass whether or not they are recorded.
        for improved in ({}, {"one item/site": [101]}, {"one item/site": [100, 101]}):
            self.assertFalse(debt_errors(improved, baseline, baseline))
            self.assertFalse(debt_errors(improved, improved, baseline))
        # The recorded file may not grow past what base allows.
        self.assertTrue(debt_errors({"new": [1]}, {"new": [1]}, {}))
        self.assertTrue(
            debt_errors(
                {"one item/site": [100]},
                {"one item/site": [101]},
                {"one item/site": [100]},
            )
        )

    def test_intentional_debt_needs_exact_reviewed_operation(self) -> None:
        proposal = {
            "kind": "lint",
            "tool": "clippy",
            "site": "operation",
            "before": [],
            "after": [1],
            "operation": "native parameter list",
            "source_contract": "original helper takes eight arguments",
            "reason": "preserve behavior",
            "retiring_owner": "owner",
            "retirement_condition": "native boundary retires",
        }
        body = (
            "Agent: /review | Model: gpt-6.1-sol | Reasoning effort: high\n```cleanup-acceptance\n"
            + json.dumps({"head": "head", "accept": [proposal]})
            + "\n```"
        )
        with patch.object(
            LINT,
            "context",
            return_value=("/implementation", [proposal], [body], "head"),
        ):
            self.assertEqual(
                LINT.reviewed_debt("clippy", {"operation": [1]}, {}), {"operation": [1]}
            )
        with patch.object(
            LINT,
            "context",
            return_value=("/implementation", [proposal], [body], "stale"),
        ):
            self.assertEqual(LINT.reviewed_debt("clippy", {"operation": [1]}, {}), {})

    def test_rust_identity_survives_lines_and_qualifies_impl(self) -> None:
        text = "impl A { unsafe fn run() { use_it(); } }\nimpl B { fn run() {} }"
        key = LINT.rust_site("x.rs", text, text.index("fn run"), "missing_safety_doc")
        moved = "// extra line\n\n" + text.replace("use_it();", "use_it(); more();")
        key2 = LINT.rust_site(
            "x.rs", moved, moved.index("fn run"), "missing_safety_doc"
        )
        self.assertEqual(key, key2)
        a = LINT.rust_site("x.rs", text, text.index("fn run"), "too_many_arguments")
        b = LINT.rust_site("x.rs", text, text.rindex("fn run"), "too_many_arguments")
        self.assertNotEqual(a, b)

    def test_clippy_deduplicates_targets_but_preserves_two_sites(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = "rust/test.rs"
            (root / "rust").mkdir()
            text = (
                "fn run() { let é = 1; if é > 0 { use_it(); } if é > 0 { use_it(); } }"
            )
            (root / path).write_text(text)
            records = []
            for start in (text.index("{ use_it();"), text.rindex("{ use_it();")):
                span = {
                    "is_primary": True,
                    "file_name": path,
                    "byte_start": len(text[:start].encode()),
                    "byte_end": len(text[: start + 11].encode()),
                }
                records.append(
                    json.dumps(
                        {
                            "reason": "compiler-message",
                            "message": {
                                "code": {"code": "clippy::too_many_arguments"},
                                "message": "this function has too many arguments (8/7)",
                                "spans": [span],
                            },
                        }
                    )
                )
            rows = LINT.clippy_rows("\n".join(records * 2), root)
            self.assertEqual(list(rows.values()), [[8, 8]])
            self.assertTrue(debt_errors(rows, rows, LINT.clippy_rows(records[0], root)))

    def test_only_retained_clippy_families_are_collected_and_ratcheted(self) -> None:
        retained = {
            "too_many_arguments": 8,
            "missing_safety_doc": 1,
            "disallowed_macros": 1,
        }
        removed = (
            "too_many_lines",
            "cognitive_complexity",
            "excessive_nesting",
            "cast_possible_truncation",
            "cast_sign_loss",
            "cast_possible_wrap",
            "undocumented_unsafe_blocks",
            "multiple_unsafe_ops_per_block",
        )
        self.assertEqual(set(LINT.CLIPPY), set(retained))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "rust").mkdir()
            (root / "rust/test.rs").write_text("pub unsafe fn run() {}")
            diagnostic = {
                "reason": "compiler-message",
                "message": {
                    "code": {"code": ""},
                    "message": "",
                    "spans": [
                        {
                            "is_primary": True,
                            "file_name": "rust/test.rs",
                            "byte_start": 0,
                            "byte_end": 22,
                        }
                    ],
                },
            }
            for lint, magnitude in {**retained, **dict.fromkeys(removed, 1)}.items():
                with self.subTest(lint=lint):
                    diagnostic["message"]["code"]["code"] = "clippy::" + lint
                    diagnostic["message"]["message"] = f"measured ({magnitude}/0)"
                    rows = LINT.clippy_rows(json.dumps(diagnostic), root)
                    if lint in removed:
                        self.assertEqual(rows, {})
                        continue
                    self.assertEqual(list(rows.values()), [[magnitude]])
                    self.assertFalse(debt_errors(rows, rows, rows))
                    self.assertFalse(debt_errors({}, rows, rows))
                    key = next(iter(rows))
                    self.assertTrue(debt_errors({key: [magnitude + 1]}, rows, rows))

    def test_ruff_item_identity_and_growth(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            file = root / "tool.py"
            file.write_text("class C:\n    def work(self):\n        pass\n")
            diagnostic = {
                "filename": str(file),
                "location": {"row": 2},
                "message": "Too many branches (15 > 12)",
                "code": "PLR0912",
            }
            before = LINT.ruff_rows(json.dumps([diagnostic]), root)
            file.write_text("\n\n" + file.read_text())
            diagnostic["location"]["row"] = 4
            self.assertEqual(before, LINT.ruff_rows(json.dumps([diagnostic]), root))
            diagnostic["message"] = "Too many branches (16 > 12)"
            self.assertTrue(
                debt_errors(
                    LINT.ruff_rows(json.dumps([diagnostic]), root), before, before
                )
            )


class PolicyTests(unittest.TestCase):
    def test_original_name_suppression_requires_exact_item_review(self) -> None:
        import migration

        source = (
            "pub struct Native;\n#[allow(non_upper_case_globals)]\n"
            "impl Native { pub const OriginalName: u8 = 1; }\n"
        )
        site = next(iter(POLICY.rust_suppressions("rust/x.rs", source)))
        proposal = {
            "kind": "suppression",
            "tool": "rust",
            "site": site,
            "before": 0,
            "after": 1,
            "operation": "Native::OriginalName",
            "source_contract": "original header enumerator spelling",
            "reason": "Keep the original's public vocabulary",
            "retiring_owner": "header consumer",
            "retirement_condition": "When the corresponding API is removed",
        }
        head = "a" * 40

        def acceptance(
            agent: str = "/review",
            sha: str = head,
            payload: dict[str, str | int | float] | None = None,
        ) -> str:
            return (
                f"Agent: {agent} | Model: test | Reasoning effort: high\n"
                "```cleanup-acceptance\n"
                + json.dumps(
                    {"head": sha, "accept": [proposal if payload is None else payload]}
                )
                + "\n```"
            )

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            initialize(root, {"rust/x.rs": "pub struct Native;\n"})
            (root / "rust/x.rs").write_text(source)
            subprocess.run(
                [
                    "rustc",
                    "--crate-type",
                    "lib",
                    "-D",
                    "warnings",
                    "--out-dir",
                    str(root),
                    str(root / "rust/x.rs"),
                ],
                check=True,
                capture_output=True,
                env=migration.environment(),
            )
            for comments, proposals, expected in (
                ([], [proposal], 1),
                ([acceptance()], [proposal], 0),
                ([acceptance("/impl")], [proposal], 1),
                ([acceptance(sha="b" * 40)], [proposal], 1),
                (
                    [acceptance(payload=dict(proposal, after=2))],
                    [dict(proposal, after=2)],
                    1,
                ),
                (
                    [acceptance(payload=dict(proposal, source_contract=""))],
                    [dict(proposal, source_contract="")],
                    1,
                ),
                *(
                    (
                        [acceptance(payload=dict(proposal, **{key: value}))],
                        [dict(proposal, **{key: value})],
                        1,
                    )
                    for key, value in (
                        ("before", False),
                        ("before", 0.0),
                        ("after", True),
                        ("after", 1.0),
                    )
                ),
                *(
                    (
                        [acceptance(payload=dict(proposal, **{key: " "}))],
                        [dict(proposal, **{key: " "})],
                        1,
                    )
                    for key in (
                        "operation",
                        "source_contract",
                        "reason",
                        "retiring_owner",
                        "retirement_condition",
                    )
                ),
            ):
                with (
                    patch.object(POLICY, "ROOT", root),
                    patch.object(POLICY, "git", partial(git, cwd=root)),
                    patch.object(
                        POLICY,
                        "context",
                        return_value=("/impl", proposals, comments, head),
                    ),
                    patch("sys.argv", ["lint-policy.py", "--base-ref", "HEAD"]),
                ):
                    self.assertEqual(POLICY.main(), expected)
        for forbidden in (
            "rust/x.rs | module | allow non_upper_case_globals",
            "rust/x.rs | mod nested | allow non_upper_case_globals",
            "rust/x.rs | fn work | allow warnings",
            "rust/x.rs | fn work | allow clippy::all",
            "rust/x.rs | fn work | allow clippy::pedantic",
            "rust/x.rs | fn work | allow clippy :: all",
            "rust/x.rs | fn work | allow clippy::r#all",
            "rust/x.rs | fn work | allow unused",
            "rust/x.rs | fn work | allow rust_2024_compatibility",
            "tools/x.py | work | noqa:F401",
            *POLICY.rust_suppressions(
                "rust/x.rs",
                '#[allow(non_upper_case_globals)] #[doc="impl Native"] '
                "pub mod nested { pub const OriginalName: u8 = 1; }",
            ),
        ):
            exact = dict(proposal, site=forbidden)
            comment = (
                "Agent: /review | Model: test | Reasoning effort: high\n"
                "```cleanup-acceptance\n"
                + json.dumps({"head": head, "accept": [exact]})
                + "\n```"
            )
            self.assertFalse(
                POLICY.accepted_suppression(
                    forbidden, 0, 1, ("/impl", [exact], [comment], head)
                )
            )
        for attribute in ("#![allow(warnings)]", "# ! [allow(warnings)]"):
            self.assertEqual(
                list(POLICY.rust_suppressions("rust/x.rs", attribute + "\nfn x() {}")),
                ["rust/x.rs | module | allow warnings"],
            )

    def test_suppression_cannot_move_between_named_rust_targets(self) -> None:
        import migration

        source = (
            "pub struct A; pub struct B;\n"
            "impl A { #[allow(non_upper_case_globals)] pub const Original: u8 = 1; }\n"
            "impl B { #[allow(non_upper_case_globals)] pub const Original: u8 = 1; }\n"
            "pub mod a { #[allow(non_upper_case_globals)] pub const Original: u8 = 1; }\n"
            "pub mod b { #[allow(non_upper_case_globals)] pub const Original: u8 = 1; }\n"
            "#[allow(non_snake_case)] pub const fn First() {}\n"
            "#[allow(non_snake_case)] pub const fn Second() {}\n"
            '#[allow(non_upper_case_globals)] #[doc="impl Native"] '
            "pub mod nested { pub const OriginalName: u8 = 1; }\n"
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "fixture.rs"
            path.write_text(source)
            subprocess.run(
                [
                    "rustc",
                    "--crate-type",
                    "lib",
                    "-D",
                    "warnings",
                    "--out-dir",
                    directory,
                    str(path),
                ],
                check=True,
                capture_output=True,
                env=migration.environment(),
            )
        before = POLICY.rust_suppressions("rust/x.rs", source)
        for first, second in (
            ("impl A", "impl B"),
            ("mod a", "mod b"),
            ("fn First", "fn Second"),
        ):
            with self.subTest(first=first, second=second):
                # Compare the two complete target identities, not line offsets.
                sites = [site for site in before if first in site or second in site]
                self.assertEqual(len(sites), 2)
                self.assertTrue(all(before[site] == 1 for site in sites))
                self.assertNotEqual(sites[0], sites[1])

    def test_discovered_ruff_overrides_cannot_hide_ordinary_diagnostics(self) -> None:
        for path, table in (
            (".ruff.toml", "lint"),
            ("tools/ruff.toml", "lint"),
            ("tools/.ruff.toml", "lint"),
            ("tools/pyproject.toml", "tool.ruff.lint"),
        ):
            with self.subTest(path=path), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                initialize(
                    root,
                    {
                        "ruff.toml": '[lint]\nselect=["F"]\n',
                        "tools/x.py": "import unused\n",
                    },
                )
                command = ["uvx", "--from", "ruff==0.16.8", "ruff", "check", "tools/"]
                self.assertEqual(
                    subprocess.run(command, cwd=root, capture_output=True).returncode, 1
                )
                (root / path).write_text(f'[{table}]\nignore=["F401"]\n')
                subprocess.run(["git", "add", "."], cwd=root, check=True)
                self.assertEqual(
                    subprocess.run(command, cwd=root, capture_output=True).returncode, 0
                )

                with (
                    patch.object(POLICY, "ROOT", root),
                    patch.object(POLICY, "git", partial(git, cwd=root)),
                    patch("sys.argv", ["lint-policy.py", "--base-ref", "HEAD"]),
                ):
                    self.assertEqual(POLICY.main(), 1)

    def test_selection_exclusion_per_file_ignore_and_threshold(self) -> None:
        for before, after in (
            ({"lint": {"select": ["F", "B"]}}, {"lint": {"select": ["F"]}}),
            ({"lint": {}}, {"lint": {"per-file-ignores": {"*.py": ["F"]}}}),
            ({"exclude": []}, {"exclude": ["tools/"]}),
            ({"too-many-lines-threshold": 100}, {"too-many-lines-threshold": 101}),
            ({"unsafe_code": "deny"}, {"unsafe_code": "allow"}),
        ):
            self.assertTrue(POLICY.config_errors(before, after))
        self.assertFalse(
            POLICY.config_errors({"select": ["F"]}, {"select": ["F", "B"]})
        )

    def test_suppressions_ignore_strings_but_detect_blankets_and_noqa(self) -> None:
        before = {
            "x.rs": 'const X: &str = "#[allow(warnings)]"; // #[allow(all)]\nfn x() {}',
            "x.py": 'text = "# noqa"\n',
        }
        self.assertFalse(POLICY.suppressions(before))
        after = {
            "x.rs": "#![allow(warnings)]\n#[allow(clippy::all)]\nfn x() {}",
            "x.py": "import x # noqa: F401\n",
        }
        self.assertEqual(sum(POLICY.suppressions(after).values()), 3)


class GrowthTests(unittest.TestCase):
    def test_import_aliases_and_tuple_callback_fields_count_growth(self) -> None:
        before = boundary_ids({"rust/x.rs": "struct S { value: u8 }"})
        after = boundary_ids(
            {
                "rust/x.rs": 'mod a { pub type Callback = extern "C" fn(); } use a::Callback as Renamed; use Renamed as Twice; struct S { callback: Twice } #[repr(C)] struct Tuple(pub u8, pub Twice, pub Option<Renamed>); #[repr(C)] enum E { First(Twice), Second(Twice) }'
            }
        )
        self.assertEqual(
            after["callback_slots"],
            {
                "rust/x.rs | S.callback",
                "rust/x.rs | Tuple.1",
                "rust/x.rs | Tuple.2",
                "rust/x.rs | E::First.0",
                "rust/x.rs | E::Second.0",
            },
        )
        growth = GROWTH.increases(
            {"callback_slots": dict.fromkeys(before["callback_slots"], 1)},
            {"callback_slots": dict.fromkeys(after["callback_slots"], 1)},
        )
        self.assertEqual(growth[0]["new"], 5)

    def test_each_counter_is_independent(self) -> None:
        for counter in (
            "tools_cpp",
            "tests_cpp",
            "exports",
            "callback_slots",
            "generated_records",
            "outside_cpp",
        ):
            before = {counter: {"one": 1}, "other": {"gone": 100}}
            after = {counter: {"one": 2}, "other": {}}
            self.assertEqual(GROWTH.increases(before, after)[0]["counter"], counter)

    def test_ast_covers_export_name_unprefixed_alias_and_option(self) -> None:
        rows = boundary_ids(
            {
                "rust/x.rs": '#[unsafe(export_name="renamed")] pub extern "C" fn x() {}\n#[unsafe(no_mangle)] extern "C" fn plain() {}\ntype Callback = unsafe extern "C" fn(u8);\nstruct S { a: Callback, b: Option<Callback>, c: extern "Rust" fn() }\nfn arguments(callback: Callback) {}'
            }
        )
        self.assertEqual(rows["exports"], {"rust/x.rs | renamed", "rust/x.rs | plain"})
        self.assertEqual(rows["callback_slots"], {"rust/x.rs | S.a", "rust/x.rs | S.b"})
        with self.assertRaisesRegex(ValueError, "macro boundary"):
            boundary_ids(
                {
                    "rust/x.rs": 'macro_rules! export { () => { #[no_mangle] extern "C" fn x() {} } }'
                }
            )

    def test_compiler_pretty_syntax_keeps_boundary_declarations(self) -> None:
        text = '#[unsafe(no_mangle)] pub extern "C" fn entry() { if #[allow(non_exhaustive_omitted_patterns)] match a { true => false, _ => true } {} let _ = builtin # offset_of(Binding<A, B>, entry); } struct S { callback: extern "C" fn() }'
        rows = boundary_ids({"rust/expanded.rs": text}, expanded=True)
        self.assertEqual(rows["exports"], {"rust/expanded.rs | entry"})
        self.assertEqual(rows["callback_slots"], {"rust/expanded.rs | S.callback"})
        with self.assertRaisesRegex(ValueError, "parse error"):
            boundary_ids(
                {"rust/expanded.rs": "fn x() { builtin # unknown(); }"}, expanded=True
            )

    def test_real_metrics_distinguish_original_test_lines(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            initialize(root, {"src/tests/x.cpp": "int original = 1;\n"})
            original = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=root, text=True
            ).strip()
            (root / "src/tests/x.cpp").write_text("int original = 1;\nint fork = 2;\n")
            (root / "tools").mkdir()
            (root / "tools/x.cpp").write_text("int evidence = 3;\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(
                [
                    "git",
                    "-c",
                    "user.name=test",
                    "-c",
                    "user.email=t@invalid",
                    "commit",
                    "-qm",
                    "Add: fork",
                ],
                cwd=root,
                check=True,
            )
            with (
                patch.object(GROWTH.METRICS, "ROOT", root),
                patch.object(GROWTH.METRICS, "BASELINE", original),
            ):
                rows = GROWTH.METRICS.growth_inventory("HEAD")
                self.assertEqual(rows["tests_cpp"], {"src/tests/x.cpp": 1})
                self.assertEqual(rows["tools_cpp"], {"tools/x.cpp": 1})

    def test_commit_association_cannot_accept_another_pr_head(self) -> None:
        wrong = {"head": {"sha": "other"}, "state": "open"}
        merged = {"head": {"sha": "tested"}, "state": "closed", "merged_at": "date"}
        self.assertEqual(associated_pull([wrong, merged], "tested"), merged)
        self.assertIsNone(associated_pull([wrong], "tested"))

    def pr_context(
        self,
        pulls: Mapping[str, list[dict[str, object]]],
        unknown: frozenset[str] = frozenset(),
        event: Mapping[str, object] | None = None,
    ) -> str:
        """Run `context` on a two-parent HEAD with no network.

        GitHub's commit lookup fails for the `unknown` commits, as it does for an
        unpushed local merge. Without an `event` payload there is no event file.
        """
        shas = {"HEAD": "merge", "HEAD^1": "first", "HEAD^2": "second"}

        def fake_git(*args: str) -> str:
            if args[:2] == ("rev-list", "--parents"):
                return "merge first second\n"
            assert args[0] == "rev-parse", args
            return shas[args[1]] + "\n"

        def fake_gh(*args: str, capture_stderr: bool = False) -> str:
            route = args[1].removeprefix("repos/fork/")
            if route.startswith("commits/"):
                sha = route.split("/")[1]
                if sha in unknown:
                    raise subprocess.CalledProcessError(1, ["gh", *args])
                return json.dumps(pulls.get(sha, []))
            number = int(route.removeprefix("pulls/"))
            head = next(
                pull["head"]
                for found in pulls.values()
                for pull in found
                if pull["number"] == number
            )
            return json.dumps({"head": head, "body": f"PR {number}"})

        with (
            tempfile.TemporaryDirectory() as directory,
            patch.dict("os.environ", {"GITHUB_REPOSITORY": "fork"}) as environment,
            patch.object(cleanup_contract, "git", fake_git),
            patch.object(cleanup_contract, "gh", fake_gh),
            patch.object(cleanup_contract, "discussion", return_value=[]),
            patch.object(cleanup_contract, "records", side_effect=lambda body: [body]),
        ):
            environment.pop("GITHUB_EVENT_PATH", None)
            if event is not None:
                path = Path(directory) / "event.json"
                path.write_text(json.dumps(event))
                environment["GITHUB_EVENT_PATH"] = str(path)
            _, bodies, _, head = cleanup_contract.context()
        return f"{bodies[0] if bodies else None} at {head}"

    def test_local_merge_head_reads_its_own_pr(self) -> None:
        # A local `Update: Merge integration N` merge is the PR head; its second
        # parent is the base branch, which has no associated PR head.
        own: dict[str, object] = {
            "number": 7,
            "head": {"sha": "merge"},
            "state": "open",
        }
        self.assertEqual(self.pr_context({"merge": [own]}), "PR 7 at merge")

    def test_ci_merge_checkout_reads_second_parent_pr(self) -> None:
        # CI's synthetic merge commit has no PR; its second parent is the PR head.
        pr: dict[str, object] = {
            "number": 9,
            "head": {"sha": "second"},
            "state": "open",
        }
        self.assertEqual(self.pr_context({"second": [pr]}), "PR 9 at second")
        self.assertEqual(self.pr_context({}), "None at second")
        # A push of the merge to the base branch reads the PR it merged.
        merged: dict[str, object] = {
            "number": 4,
            "head": {"sha": "second"},
            "state": "closed",
            "merged_at": "date",
            "merge_commit_sha": "merge",
        }
        self.assertEqual(self.pr_context({"second": [merged]}), "PR 4 at second")

    def test_merge_group_reads_the_queued_pr(self) -> None:
        # Full in the merge queue checks out the group commit: its first parent is
        # the base or the group of the PR queued ahead, its second this PR's head.
        # GitHub associates no PR with an unmerged group commit.
        event = {"merge_group": {"head_sha": "merge", "base_ref": "refs/heads/x"}}
        queued: dict[str, object] = {
            "number": 12,
            "head": {"sha": "second"},
            "state": "open",
        }
        self.assertEqual(
            self.pr_context({"second": [queued]}, event=event), "PR 12 at second"
        )
        # A second queued entry: the PR ahead already merged as the first parent.
        ahead: dict[str, object] = {
            "number": 11,
            "head": {"sha": "ahead"},
            "state": "closed",
            "merged_at": "date",
            "merge_commit_sha": "first",
        }
        pulls = {"first": [ahead], "second": [queued]}
        self.assertEqual(self.pr_context(pulls, event=event), "PR 12 at second")
        # The PR ahead still queued is never read in place of this one.
        self.assertEqual(
            self.pr_context(
                {"first": [dict(ahead, state="open")], "second": [queued]}, event=event
            ),
            "PR 12 at second",
        )

    def test_unknown_head_lookup_is_no_pr(self) -> None:
        # GitHub errors for an unpushed commit; that is no PR, not an abort.
        self.assertEqual(
            self.pr_context({}, unknown=frozenset({"merge"})), "None at second"
        )

    def test_unpushed_local_merge_reads_first_parent_pr(self) -> None:
        # The first parent is the open PR's pushed head and the second is the base,
        # whose own merged PR (merged by another commit) must never match.
        own: dict[str, object] = {
            "number": 5,
            "head": {"sha": "first"},
            "state": "open",
        }
        base: dict[str, object] = {
            "number": 3,
            "head": {"sha": "second"},
            "state": "closed",
            "merged_at": "date",
            "merge_commit_sha": "base-merge",
        }
        self.assertEqual(
            self.pr_context(
                {"first": [own], "second": [base]}, unknown=frozenset({"merge"})
            ),
            "PR 5 at first",
        )
        self.assertEqual(
            self.pr_context({"second": [base]}, unknown=frozenset({"merge"})),
            "None at second",
        )

    def test_review_requires_head_payload_and_different_agent(self) -> None:
        delta = {"counter": "exports", "old": 1, "new": 2, "added": {"x": 1}}
        proposal = dict(
            delta,
            kind="growth",
            reason="source contract",
            existing_alternatives="none available",
            retiring_owner="owner",
            retirement_condition="native shell removed",
        )

        def report(agent: str, head: str) -> str:
            return (
                f"Agent: {agent} | Model: gpt-6.1-sol | Reasoning effort: high\n```cleanup-acceptance\n"
                + json.dumps({"head": head, "accept": [proposal]})
                + "\n```"
            )

        for comments in (
            [],
            [report("/implementation", "head")],
            [report("/review", "stale")],
            [report("/review", "a" * 40)],
        ):
            self.assertFalse(
                GROWTH.accepted(
                    delta, ("/implementation", [proposal], comments, "head")
                )
            )
        self.assertTrue(
            GROWTH.accepted(
                delta,
                ("/implementation", [proposal], [report("/review", "head")], "head"),
            )
        )
        changed = dict(proposal, reason="different operation")
        self.assertFalse(
            reviewed(changed, [report("/review", "head")], "/implementation", "head")
        )

    def test_growth_acceptance_binds_added_keys_not_counter_totals(self) -> None:
        reviewed_delta = {
            "counter": "tests_cpp",
            "old": 4786,
            "new": 4795,
            "added": {"src/tests/x.cpp": 9},
        }
        proposal = dict(
            reviewed_delta,
            kind="growth",
            reason="source contract",
            existing_alternatives="none available",
            retiring_owner="owner",
            retirement_condition="native shell removed",
        )
        totals_free = {k: v for k, v in proposal.items() if k not in ("old", "new")}

        def accepted(
            delta: Mapping[str, object], offered: Mapping[str, object]
        ) -> bool:
            return GROWTH.accepted(
                dict(delta),
                (
                    "/implementation",
                    [dict(offered)],
                    [acceptance_comment("head", offered)],
                    "head",
                ),
            )

        # Another PR's merge moved the counter's totals, not this PR's keys.
        moved = dict(reviewed_delta, old=4790, new=4799)
        self.assertTrue(accepted(moved, proposal))
        self.assertTrue(accepted(moved, totals_free))
        for changed in (
            dict(moved, added={"src/tests/x.cpp": 10}),
            dict(moved, added={"src/tests/x.cpp": 9, "src/tests/y.cpp": 1}),
            dict(moved, counter="tools_cpp"),
        ):
            self.assertFalse(accepted(changed, proposal))

    def test_growth_acceptance_holds_while_its_source_is_unchanged(self) -> None:
        earlier, missing = "a" * 40, "b" * 40
        records = {
            earlier: {"abi-record-x.h": "struct X { int a; };"},
            "HEAD": {"abi-record-x.h": "struct X { int a; };", "abi-record-y.h": ""},
        }
        files = {(earlier, "tools/x.cpp"): "int x;", ("HEAD", "tools/x.cpp"): "int x;"}

        def check(counter: str, key: str, edits: Mapping[str, str]) -> bool:
            delta = {"counter": counter, "old": 1, "new": 2, "added": {key: 1}}
            proposal = dict(
                delta,
                kind="growth",
                reason="source contract",
                existing_alternatives="none available",
                retiring_owner="owner",
                retirement_condition="native shell removed",
            )
            comment = (
                "Agent: /review | Model: gpt-6.1-sol | Reasoning effort: high\n"
                "```cleanup-acceptance\n"
                + json.dumps({"head": edits.get("at", earlier), "accept": [proposal]})
                + "\n```"
            )
            current = {rev: dict(rows) for rev, rows in records.items()}
            current["HEAD"].update({k: v for k, v in edits.items() if k != "at"})
            text = dict(files)
            text.update({("HEAD", k): v for k, v in edits.items() if k != "at"})
            with (
                patch.object(GROWTH, "known", lambda rev: rev != missing),
                patch.object(GROWTH.METRICS, "generated_records", current.__getitem__),
                patch.object(GROWTH.METRICS, "show", lambda r, p: text.get((r, p), "")),
            ):
                return GROWTH.accepted(
                    delta, ("/implementation", [proposal], [comment], "new-head")
                )

        self.assertTrue(check("generated_records", "abi-record-x.h", {}))
        self.assertFalse(
            check(
                "generated_records",
                "abi-record-x.h",
                {"abi-record-x.h": "struct X { int b; };"},
            )
        )
        self.assertFalse(check("generated_records", "abi-record-y.h", {}))
        self.assertFalse(check("generated_records", "abi-record-x.h", {"at": missing}))
        self.assertFalse(check("generated_records", "abi-record-x.h", {"at": "short"}))
        self.assertTrue(check("tools_cpp", "tools/x.cpp", {}))
        self.assertFalse(check("tools_cpp", "tools/x.cpp", {"tools/x.cpp": "int y;"}))
        self.assertFalse(check("exports", "tools/x.cpp", {}))


def acceptance_comment(head: str, proposal: Mapping[str, object]) -> str:
    return (
        "Agent: /review | Model: test | Reasoning effort: high\n"
        "```cleanup-acceptance\n"
        + json.dumps({"head": head, "accept": [proposal]})
        + "\n```"
    )


def commit(root: Path) -> str:
    git(
        "-c",
        "user.name=test",
        "-c",
        "user.email=test@example.invalid",
        "commit",
        "-qam",
        "Change: fixture",
        cwd=root,
    )
    return git("rev-parse", "HEAD", cwd=root).strip()


Item = tuple[str, ...]


def items(keys: Mapping[str, object]) -> set[Item]:
    return {tuple(key.split(" | ")[:2]) for key in keys}


class CarryTests(unittest.TestCase):
    OLD = "rust/a.rs | fn curve_limit | too_many_arguments | 4a33eacd5fa65f2b2e28"
    NEW = "rust/a.rs | fn get_curve_speed_limit | too_many_arguments | 4a33eacd5fa65f2b2e28"

    def ratchet(
        self,
        previous: dict[str, list[int]],
        head: dict[str, list[int]],
        *carries: dict,
        recorded: dict[str, list[int]] | None = None,
        at_base: set[Item] | None = None,
        at_head: set[Item] | None = None,
    ) -> tuple[list[str], str]:
        """Run the check; items default to those the debt rows name."""
        base_items = items(previous) if at_base is None else at_base
        head_items = items(head) if at_head is None else at_head

        def exists(kind: str, key: str, revision: str | None) -> bool:
            item = tuple(key.split(" | ")[:2])
            return item in (base_items if revision else head_items)

        output = io.StringIO()
        with (
            patch.object(
                LINT, "context", return_value=("/impl", list(carries), [], "h")
            ),
            patch.object(LINT, "item_exists", exists),
            contextlib.redirect_stdout(output),
        ):
            measured, stored = LINT.check(
                "clippy", head, head if recorded is None else recorded, previous, "b"
            )
        return measured + stored, output.getvalue()

    @staticmethod
    def carry(old: str, new: str, tool: str = "clippy") -> dict[str, str]:
        return {"kind": "carry", "tool": tool, "from": old, "to": new}

    def test_unambiguous_rename_and_move_need_no_carry(self) -> None:
        moved = self.OLD.replace("rust/a.rs", "rust/curve.rs")
        for new, values in ((self.NEW, [9]), (moved, [9]), (self.NEW, [8])):
            with self.subTest(new=new, values=values):
                errors, printed = self.ratchet({self.OLD: [9]}, {new: values})
                self.assertEqual(errors, [])
                self.assertIn(f"carried: {self.OLD} -> {new}", printed)

    def test_pairing_needs_a_real_rename(self) -> None:
        # Both functions exist at base and head; one improves and the other grows.
        both = items({self.OLD: 0, self.NEW: 0})
        errors, _ = self.ratchet(
            {self.OLD: [8]}, {self.NEW: [8]}, at_base=both, at_head=both
        )
        self.assertTrue(errors)
        # The new item existed at base: an existing function grew.
        errors, _ = self.ratchet({self.OLD: [8]}, {self.NEW: [8]}, at_base=both)
        self.assertTrue(errors)
        # The old item still exists at head: it improved, and something else grew.
        errors, _ = self.ratchet({self.OLD: [8]}, {self.NEW: [8]}, at_head=both)
        self.assertTrue(errors)
        # A different file and a different name never pair without a carry.
        far = self.NEW.replace("rust/a.rs", "rust/b.rs")
        errors, printed = self.ratchet({self.OLD: [8]}, {far: [8]})
        self.assertTrue(errors)
        errors, _ = self.ratchet({self.OLD: [8]}, {far: [8]}, self.carry(self.OLD, far))
        self.assertEqual(errors, [])

    def test_rename_is_recorded_once(self) -> None:
        # Recorded under its new key only: passes.
        self.assertEqual(self.ratchet({self.OLD: [9]}, {self.NEW: [9]})[0], [])
        # Old row left behind: the next change could restore the old item.
        for recorded in ({self.OLD: [9]}, {self.OLD: [9], self.NEW: [9]}):
            with self.subTest(recorded=recorded):
                errors, _ = self.ratchet(
                    {self.OLD: [9]}, {self.NEW: [9]}, recorded=recorded
                )
                self.assertIn(
                    f"rename not recorded: {self.OLD}; run with --write", errors
                )

    def test_stale_rows_allow_nothing_new(self) -> None:
        # The base file still lists an item that no longer exists at base.
        errors, _ = self.ratchet(
            {self.OLD: [9]}, {}, recorded={self.OLD: [9]}, at_base=set()
        )
        self.assertEqual(errors, [])
        errors, _ = self.ratchet({self.OLD: [9]}, {self.OLD: [9]}, at_base=set())
        self.assertTrue(errors)
        # An improvement left unrecorded keeps its row and still passes.
        errors, _ = self.ratchet(
            {self.OLD: [9]}, {self.OLD: [8]}, recorded={self.OLD: [9]}
        )
        self.assertEqual(errors, [])

    def test_ambiguous_renames_pair_by_item_then_file_or_need_a_carry(self) -> None:
        other_old = self.OLD.replace("curve_limit", "brake_limit")
        other_new = self.NEW.replace("get_curve_speed_limit", "get_brake_limit")
        moved = other_old.replace("rust/a.rs", "rust/b.rs")
        # A move keeps its item name, so it pairs even beside a rename.
        errors, _ = self.ratchet(
            {self.OLD: [9], other_old: [10]}, {self.NEW: [9], moved: [10]}
        )
        self.assertEqual(errors, [])
        # Two renames in one file are ambiguous and need declared carries.
        previous = {self.OLD: [9], other_old: [9]}
        head = {self.NEW: [9], other_new: [9]}
        errors, printed = self.ratchet(previous, head)
        self.assertTrue(errors)
        self.assertIn("carry candidates", printed)
        errors, _ = self.ratchet(
            previous,
            head,
            self.carry(self.OLD, self.NEW),
            self.carry(other_old, other_new),
        )
        self.assertEqual(errors, [])

    def test_growth_and_invalid_carries_fail(self) -> None:
        other = self.NEW.replace("too_many_arguments", "missing_safety_doc")
        for previous, head, carry in (
            ({self.OLD: [9]}, {self.NEW: [10]}, self.carry(self.OLD, self.NEW)),
            (
                {self.OLD: [9]},
                {self.OLD: [9], self.NEW: [9]},
                self.carry(self.OLD, self.NEW),
            ),
            (
                {self.OLD: [9], self.NEW: [8]},
                {self.NEW: [9]},
                self.carry(self.OLD, self.NEW),
            ),
            ({self.OLD: [1]}, {other: [1]}, self.carry(self.OLD, other)),
        ):
            with self.subTest(head=head, carry=carry):
                errors, printed = self.ratchet(previous, head, carry)
                self.assertTrue(errors)
                self.assertIn("invalid carry", printed)

    def test_carries_cannot_multiply_retired_debt(self) -> None:
        other = self.NEW.replace("get_curve_speed_limit", "another")
        for head, carries in (
            ({self.NEW: [9, 9]}, [self.carry(self.OLD, self.NEW)]),
            ({self.NEW: [9], other: [9]}, []),
            (
                {self.NEW: [9], other: [9]},
                [self.carry(self.OLD, self.NEW), self.carry(self.OLD, other)],
            ),
        ):
            self.assertTrue(self.ratchet({self.OLD: [9]}, head, *carries)[0])

    def test_merge_base_needs_common_history(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            initialize(root, {"a.txt": "a\n"})
            first = git("rev-parse", "HEAD", cwd=root).strip()
            git("checkout", "-q", "--orphan", "unrelated", cwd=root)
            (root / "a.txt").write_text("b\n")
            git("add", "-A", cwd=root)
            commit(root)
            git("checkout", "-q", "--detach", first, cwd=root)
            with patch.object(LINT, "git", partial(git, cwd=root)):
                self.assertEqual(LINT.merge_base(first), first)
                with self.assertRaises(SystemExit):
                    LINT.merge_base("unrelated")

    def test_write_records_a_rename_found_by_the_check(self) -> None:
        signature = "(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8, h: u8) {}\n"
        old = "rust/a.rs | fn curve_limit | too_many_arguments | " + LINT.ITEM
        new = old.replace("curve_limit", "get_curve_speed_limit")
        debt = {"schema": 1, "version": "1.99.0", "inventory_head": "x"}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            initialize(
                root,
                {
                    "rust/a.rs": "fn curve_limit" + signature,
                    "migration/clippy-debt.json": json.dumps(
                        {**debt, "sites": {old: [8]}}
                    ),
                },
            )
            base = git("rev-parse", "HEAD", cwd=root).strip()
            text = "fn get_curve_speed_limit" + signature
            (root / "rust/a.rs").write_text(text)
            commit(root)
            span = {
                "is_primary": True,
                "file_name": "rust/a.rs",
                "byte_start": 0,
                "byte_end": len(text.encode()),
            }
            replay = root / "clippy.json"
            replay.write_text(
                json.dumps(
                    {
                        "reason": "compiler-message",
                        "message": {
                            "code": {"code": "clippy::too_many_arguments"},
                            "message": "this function has too many arguments (8/7)",
                            "spans": [span],
                        },
                    }
                )
            )
            argv = ["lint-ratchet.py", "clippy", "--base-ref", base, "--input"]
            results = []
            for extra in ([], ["--write"], []):
                LINT.item_names.cache_clear()
                with (
                    patch.object(LINT, "ROOT", root),
                    patch.object(LINT, "git", partial(git, cwd=root)),
                    patch.object(LINT, "context", return_value=("", [], [], "h")),
                    patch("sys.argv", [*argv, str(replay), *extra]),
                    contextlib.redirect_stdout(io.StringIO()),
                ):
                    results.append(LINT.main())
            LINT.item_names.cache_clear()
            self.assertEqual(results, [1, 0, 0])
            recorded = json.loads((root / "migration/clippy-debt.json").read_text())
            self.assertEqual(recorded["sites"], {new: [8]})

    def test_attribute_and_signature_changes_invalidate_acceptance(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = "#[cfg_attr(test, allow(dead_code))]\npub fn work() {}\n"
            initialize(root, {"rust/x.rs": original})
            accepted = git("rev-parse", "HEAD", cwd=root).strip()
            site = "rust/x.rs | fn work | allow dead_code"
            row = "rust/x.rs | fn work | too_many_arguments | item"
            proposal = {"kind": "suppression"}
            comment = acceptance_comment(accepted, proposal)
            for changed in (
                original.replace(
                    "cfg_attr(test, allow(dead_code))", "allow(dead_code)"
                ),
                original.replace("pub fn", "pub unsafe fn"),
                original.replace("pub fn", "fn"),
                original.replace("pub fn", "/// changed contract\npub fn"),
            ):
                (root / "rust/x.rs").write_text(changed)
                for module, predicate in (
                    (POLICY, partial(POLICY.unchanged_scope, site)),
                    (LINT, partial(LINT.unchanged_item, "clippy", row)),
                ):
                    with (
                        patch.object(module, "ROOT", root),
                        patch.object(module, "git", partial(git, cwd=root)),
                    ):
                        self.assertFalse(
                            reviewed(
                                proposal, [comment], "/impl", "new-head", predicate
                            )
                        )

    def test_python_decorator_is_part_of_reviewed_item(self) -> None:
        before = "@original\ndef work():\n    return 1\n"
        after = before.replace("@original", "@changed")
        self.assertNotEqual(
            LINT.item_text("ruff", "tools/x.py", before, "work"),
            LINT.item_text("ruff", "tools/x.py", after, "work"),
        )

    def test_suppression_carry_and_fidelity_reasons(self) -> None:
        old = "rust/a.rs | impl Flag > const ReplaceProtection | allow non_upper_case_globals"
        new = old.replace("rust/a.rs", "rust/group.rs")
        before, after = Counter({old: 1}), Counter({new: 1})
        carry = self.carry(old, new, "rust")
        for carries, grown, expected in (
            ([carry], after, []),
            ([], after, [f"new suppression: {new} (1)"]),
            ([carry], Counter({new: 2}), [f"new suppression: {new} (2)"]),
            (
                [
                    self.carry(
                        old, "rust/a.rs | module | allow non_upper_case_globals", "rust"
                    )
                ],
                Counter({"rust/a.rs | module | allow non_upper_case_globals": 1}),
                [
                    "new suppression: rust/a.rs | module | allow non_upper_case_globals (1)"
                ],
            ),
        ):
            with (
                patch.object(
                    POLICY, "context", return_value=("/impl", carries, [], "h")
                ),
                contextlib.redirect_stdout(io.StringIO()),
            ):
                self.assertEqual(POLICY.suppression_errors(before, grown), expected)
        reason = 'reason = "original GroupFlag::ReplaceProtection enumerator"'
        source = (
            "pub struct GroupFlag;\n"
            f"#[allow(non_upper_case_globals, {reason})]\n"
            "impl GroupFlag { pub const ReplaceProtection: u8 = 1; }\n"
            f"#[expect(clippy::cast_possible_truncation, {reason})]\n"
            "pub fn length(x: u32) -> u8 { x as u8 }\n"
        )
        reasoned = POLICY.fidelity_sites({"rust/x.rs": source})
        self.assertEqual(
            reasoned,
            {
                "rust/x.rs | impl GroupFlag | allow non_upper_case_globals",
                "rust/x.rs | fn length | expect clippy::cast_possible_truncation",
            },
        )
        with patch.object(POLICY, "context", side_effect=AssertionError):
            self.assertEqual(
                POLICY.suppression_errors(
                    Counter(), POLICY.suppressions({"rust/x.rs": source}), reasoned
                ),
                [],
            )
        for rejected in (
            source.replace(f", {reason}", ""),
            source.split("#[expect", 1)[0].replace(
                "impl GroupFlag", "#[allow(non_upper_case_globals)]\nimpl GroupFlag"
            ),
            source.replace(reason, 'reason = " "'),
            f"#![allow(non_upper_case_globals, {reason})]\npub const Original: u8 = 1;\n",
            f"#[allow(non_upper_case_globals, {reason})]\n"
            "pub mod nested { pub const Original: u8 = 1; }\n",
            f"#[allow(nonstandard_style, {reason})]\nimpl A {{ const Original: u8 = 1; }}\n",
            f"#[allow(clippy::cast_lossless, {reason})]\nfn widen(x: u8) -> u32 {{ x as u32 }}\n",
        ):
            with self.subTest(rejected=rejected):
                self.assertEqual(POLICY.fidelity_sites({"rust/x.rs": rejected}), set())

    def test_accepted_contract_binds_to_unchanged_item_text(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            initialize(root, {"rust/x.rs": "pub struct Native;\npub fn other() {}\n"})
            base = git("rev-parse", "HEAD", cwd=root).strip()
            allowed = (
                "#[allow(non_upper_case_globals)]\n"
                "impl Native { pub const OriginalName: u8 = 1; }\n"
            )
            (root / "rust/x.rs").write_text(
                "pub struct Native;\npub fn other() {}\n" + allowed
            )
            reviewed_head = commit(root)
            site = "rust/x.rs | impl Native | allow non_upper_case_globals"
            proposal = {
                "kind": "suppression",
                "tool": "rust",
                "site": site,
                "before": 0,
                "after": 1,
                "operation": "Native::OriginalName",
                "source_contract": "original enumerator spelling",
                "reason": "Keep the original's public vocabulary",
                "retiring_owner": "header consumer",
                "retirement_condition": "When the corresponding API is removed",
            }
            comment = acceptance_comment(reviewed_head, proposal)
            (root / "rust/x.rs").write_text(
                "pub struct Native;\npub fn other() { let _ = 1; }\n" + allowed
            )
            unchanged = commit(root)
            (root / "rust/x.rs").write_text(
                "pub struct Native;\npub fn other() {}\n"
                + allowed.replace("= 1", "= 2")
            )
            changed = commit(root)
            for head, expected in ((unchanged, 0), (changed, 1)):
                git("checkout", "-q", head, cwd=root)
                with (
                    patch.object(POLICY, "ROOT", root),
                    patch.object(POLICY, "git", partial(git, cwd=root)),
                    patch.object(
                        POLICY,
                        "context",
                        return_value=("/impl", [proposal], [comment], head),
                    ),
                    patch("sys.argv", ["lint-policy.py", "--base-ref", base]),
                    contextlib.redirect_stdout(io.StringIO()),
                ):
                    self.assertEqual(POLICY.main(), expected)
            # The same binding holds for a reviewed lint row: `fn other` differs
            # from the reviewed head at `unchanged` and matches it at `changed`.
            row = "rust/x.rs | fn other | too_many_arguments | 4a33eacd5fa65f2b2e28"
            lint = dict(
                proposal, kind="lint", tool="clippy", site=row, before=[], after=[9]
            )
            comment = acceptance_comment(reviewed_head, lint)
            for head, expected in ((unchanged, {}), (changed, {row: [9]})):
                git("checkout", "-q", head, cwd=root)
                with (
                    patch.object(LINT, "ROOT", root),
                    patch.object(LINT, "git", partial(git, cwd=root)),
                    patch.object(
                        LINT, "context", return_value=("/impl", [lint], [comment], head)
                    ),
                    contextlib.redirect_stdout(io.StringIO()),
                ):
                    self.assertEqual(
                        LINT.reviewed_debt("clippy", {row: [9]}, {}), expected
                    )


class RetiredThresholdTests(unittest.TestCase):
    def test_only_the_three_retired_thresholds_may_be_removed(self) -> None:
        for key, before in (
            ("too-many-lines-threshold", 100),
            ("cognitive-complexity-threshold", 25),
            ("excessive-nesting-threshold", 5),
        ):
            self.assertTrue(POLICY.permitted_change(key, before, None))
            self.assertFalse(POLICY.permitted_change(key, before, before + 1))
        for key, before in (("too-many-arguments-threshold", 7), ("max-args", 5)):
            self.assertFalse(POLICY.permitted_change(key, before, None))

    def test_disallowed_macros_may_only_grow(self) -> None:
        key, macros = "disallowed-macros", ["std::assert", "std::panic"]
        self.assertTrue(POLICY.permitted_change(key, None, macros))
        self.assertTrue(POLICY.permitted_change(key, macros[:1], macros))
        self.assertFalse(POLICY.permitted_change(key, macros, macros[:1]))
        self.assertFalse(POLICY.permitted_change(key, macros, None))


class ArchiveTests(unittest.TestCase):
    def test_local_relocation_targets_remain_in_their_object(self) -> None:
        # Identical section/target names in different objects must not let a
        # changed function reuse another object's old read-only table.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            caller = root / "caller.c"
            caller.write_text(
                "int a(unsigned); int b(unsigned); int main(void) { return a(0) + 10*b(0); }"
            )
            archives = []
            results = []
            for variant, tables in (
                ("base", ("1,2,3,4", "5,6,7,8")),
                ("head", ("5,6,7,8", "1,2,3,4")),
            ):
                objects = []
                for name, table in zip(("a", "b"), tables, strict=True):
                    source = root / f"{variant}_{name}.c"
                    obj = source.with_suffix(".o")
                    source.write_text(
                        f"static const int values[4]={{ {table} }}; int {name}(unsigned x) {{ return values[x & 3]; }}"
                    )
                    subprocess.run(
                        [
                            "cc",
                            "-c",
                            "-O2",
                            "-ffunction-sections",
                            "-fdata-sections",
                            str(source),
                            "-o",
                            str(obj),
                        ],
                        check=True,
                    )
                    objects.append(str(obj))
                archive = root / f"{variant}.a"
                subprocess.run(["ar", "rcs", str(archive), *objects], check=True)
                executable = root / variant
                subprocess.run(
                    ["cc", str(caller), str(archive), "-o", str(executable)], check=True
                )
                results.append(
                    subprocess.run([str(executable)], check=False).returncode
                )
                archives.append(archive)
            self.assertEqual(results, [51, 15])
            self.assertFalse(PROOF.compare(archives[0], archives[1], {}))

    def test_code_relocation_data_and_audited_rename(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)

            def archive(
                name: str,
                function: str = "work",
                target: str = "target",
                constant: str = "hello",
                addition: int = 1,
            ) -> Path:
                source = root / (name + ".c")
                source.write_text(
                    f'extern int {target}(int); const char data[]="{constant}"; int {function}(int v) {{ return {target}(v) + {addition}; }}'
                )
                obj = root / (name + ".o")
                result = root / (name + ".a")
                subprocess.run(
                    [
                        "cc",
                        "-c",
                        "-O2",
                        "-fno-asynchronous-unwind-tables",
                        str(source),
                        "-o",
                        str(obj),
                    ],
                    check=True,
                )
                subprocess.run(["ar", "rcs", str(result), str(obj)], check=True)
                return result

            base = archive("base")
            self.assertTrue(PROOF.compare(base, archive("same"), {}))
            for changed in (
                archive("code", addition=2),
                archive("relocation", target="other"),
                archive("rodata", constant="world"),
            ):
                self.assertFalse(PROOF.compare(base, changed, {}))
            renamed = archive("renamed", function="renamed")
            self.assertFalse(PROOF.compare(base, renamed, {}))
            self.assertTrue(PROOF.compare(base, renamed, {"work": "renamed"}))
            with self.assertRaisesRegex(ValueError, "unused audited"):
                PROOF.compare(base, base, {"absent": "unused"})


class OrdinaryRustTests(unittest.TestCase):
    def test_debt_collection_repeats_the_gate_clippy_command(self) -> None:
        """Cargo replays the gate's run only for the same command, build
        directory and configuration; a second check of the crate costs a pass."""
        import migration

        gate = migration.clippy_command()
        for lint in LINT.CLIPPY:
            self.assertIn("clippy::" + lint, gate)
        self.assertEqual(gate[gate.index("--") + 1 :][:2], ["-D", "warnings"])
        calls = []

        def run(command: list[str], **options: object) -> object:
            calls.append((command, options))
            return subprocess.CompletedProcess(command, 0, "", "")

        with (
            patch.object(LINT.subprocess, "run", run),
            patch.object(
                LINT.subprocess, "check_output", lambda *a, **k: "rustc 1.99.0"
            ),
        ):
            LINT.collect("clippy", migration.ROOT)
        # The second replays only the non-test library unit (#891).
        ((command, options), (library, _)) = calls
        self.assertEqual(command, migration.clippy_command("--message-format=json"))
        self.assertEqual(
            library,
            migration.clippy_command("--message-format=json", targets="--lib"),
        )
        self.assertEqual(
            [part for part in command if part != "--message-format=json"], gate
        )
        env = options["env"]
        assert isinstance(env, dict)
        self.assertEqual(env["CARGO_TARGET_DIR"], str(migration.CARGO_TARGET_DIR))
        self.assertEqual(
            env["CLIPPY_CONF_DIR"], migration.environment()["CLIPPY_CONF_DIR"]
        )

    def test_a_failed_clippy_gate_shows_errors_without_debt_warnings(self) -> None:
        import migration

        log = (
            "    Checking x v0.1.0 (/x)\n"
            "warning: this function has too many arguments (8/7)\n  --> a.rs:1:1\n\n"
            "warning: `x` (lib) generated 1 warning\n"
            "error: equal expressions as operands to `==`\n  --> b.rs:2:2\n"
            "1 | true == true\n\n"
            "error: could not compile `x` (lib test) due to 1 previous error\n"
        )
        shown = migration.failure_output("rust-clippy", log)
        self.assertEqual(
            shown,
            "    Checking x v0.1.0 (/x)\n"
            "error: equal expressions as operands to `==`\n  --> b.rs:2:2\n"
            "1 | true == true\n\n"
            "error: could not compile `x` (lib test) due to 1 previous error\n",
        )
        self.assertEqual(migration.failure_output("rust-check", log), log)
        # Every header form starts a diagnostic: with a code, a lint path, or neither.
        for header in ("error[E0425]: ", "error: ", "error[clippy::eq_op]: "):
            coded = (
                "warning[W1]: debt\n  --> a.rs:1:1\n" + header + "cause\n  --> b.rs\n"
            )
            self.assertEqual(
                migration.failure_output("rust-clippy", coded),
                header + "cause\n  --> b.rs\n",
            )

    def test_four_rust_checks_fail_on_distinct_seeded_violations(self) -> None:
        import migration

        env = migration.environment()
        env["RUSTUP_TOOLCHAIN"] = "1.99.0"
        cases = [
            (["fmt", "--all", "--", "--check"], "pub fn x( )->i32{1}\n", "Diff in"),
            (
                ["check", "--workspace", "--all-targets", "--locked"],
                "pub fn x() { missing(); }\n",
                "cannot find function `missing`",
            ),
            (
                migration.clippy_command()[1:],
                "pub fn x() -> bool { true == true }\n",
                "equal expressions",
            ),
            (
                migration.clippy_command()[1:],
                '#[cfg(feature = "native-test-support")]\npub fn x() -> u8 { "x" }\n',
                "mismatched types",
            ),
            (
                ["test", "--workspace", "--locked"],
                '#[test]\nfn failure() { panic!("seeded"); }\n',
                "seeded",
            ),
        ]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "src").mkdir()
            (root / "Cargo.toml").write_text(
                '[package]\nname="openttd-kernels"\nversion="0.1.0"\nedition="2024"\n'
                "[features]\nnative-test-support = []\n"
            )
            (root / "src/lib.rs").write_text("")
            lock = subprocess.run(
                ["cargo", "generate-lockfile", "--offline"],
                cwd=root,
                env=env,
                capture_output=True,
            )
            self.assertEqual(lock.returncode, 0, lock.stderr.decode())
            for command, source, expected in cases:
                (root / "src/lib.rs").write_text(source)
                result = subprocess.run(
                    ["cargo", *command], cwd=root, env=env, capture_output=True
                )
                self.assertNotEqual(result.returncode, 0, command)
                self.assertIn(expected, (result.stdout + result.stderr).decode())
            (root / "migration/lints").mkdir(parents=True)
            (root / "migration/lints/clippy.toml").write_text(
                "too-many-arguments-threshold = 7\n"
            )
            (root / "src/lib.rs").write_text(
                "#![allow(clippy::all)]\n#[allow(clippy::too_many_arguments)]\npub fn many(a:i32,b:i32,c:i32,d:i32,e:i32,f:i32,g:i32,h:i32) { let _ = (a,b,c,d,e,f,g,h); }"
            )
            raw, _ = LINT.collect("clippy", root)
            self.assertIn("too_many_arguments", raw)
            # A Clippy error stops the collection and is shown as rust-checks
            # shows it, not as a count of debt.
            (root / "src/lib.rs").write_text("pub fn x() -> bool { true == true }\n")
            with self.assertRaisesRegex(ValueError, "equal expressions"):
                LINT.collect("clippy", root)
            # Cargo prints the library's warning summary directly above the test
            # target's error, with no blank line: the error must survive.
            # A lint error has no code; a compiler error has one (`error[E0425]`).
            for body, header, cause in (
                ("bool { true == true }", "error: ", "equal expressions"),
                ("i32 { missing }", "error[E0425]: ", "cannot find value"),
            ):
                (root / "src/lib.rs").write_text(
                    "pub unsafe fn undocumented() {}\n"
                    f"#[cfg(test)]\npub fn bad() -> {body}\n"
                )
                result = subprocess.run(
                    migration.clippy_command("-j1"),
                    cwd=root,
                    env=env,
                    capture_output=True,
                    text=True,
                )
                self.assertNotEqual(result.returncode, 0)
                raw = result.stdout + result.stderr
                # Cargo checks the library and its test target in either order,
                # so the error may sit directly below a warning summary, or no
                # warning may print at all. Whatever the order, no error line
                # may be lost; the synthetic case pins the adjacency itself.
                errors = [line for line in raw.splitlines() if line.startswith("error")]
                self.assertTrue(any(line.startswith(header + cause) for line in errors))
                self.assertTrue(any("could not compile" in line for line in errors))
                shown = migration.failure_output("rust-clippy", raw).splitlines()
                for line in errors:
                    self.assertIn(line, shown)
                self.assertFalse([line for line in shown if line.startswith("warning")])
            (root / "tools").mkdir()
            (root / "tools/x.py").write_text(
                "def many(x):  # noqa: C901\n"
                + "".join(f"    if x == {i}:\n        return {i}\n" for i in range(12))
            )
            raw, _ = LINT.collect("ruff", root)
            self.assertIn("C901", raw)


if __name__ == "__main__":
    unittest.main()
