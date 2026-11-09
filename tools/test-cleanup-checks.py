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
from cleanup_contract import associated_pull, reviewed, source_reviews
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
    def test_new_grown_stale_and_baseline_growth(self) -> None:
        baseline = {"one item/site": [101, 102]}
        self.assertFalse(debt_errors(baseline, baseline, baseline))
        for changed in (
            {"other site": [101]},
            {"one item/site": [101, 103]},
            {"one item/site": [101, 102, 102]},
            {},
        ):
            self.assertTrue(debt_errors(changed, baseline, baseline))
        self.assertTrue(debt_errors({"new": [1]}, {"new": [1]}, {}))
        self.assertTrue(
            debt_errors(
                {"one item/site": [101]},
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
        start = text.index("fn run")
        key, _ = LINT.rust_site("x.rs", text, start, start + 8, "missing_safety_doc")
        moved = "// extra line\n\n" + text
        key2, _ = LINT.rust_site(
            "x.rs",
            moved,
            moved.index("fn run"),
            moved.index("fn run") + 8,
            "missing_safety_doc",
        )
        self.assertEqual(key, key2)
        a = LINT.rust_site(
            "x.rs",
            text,
            text.index("fn run"),
            text.index("fn run") + 6,
            "too_many_lines",
        )[0]
        b = LINT.rust_site(
            "x.rs",
            text,
            text.rindex("fn run"),
            text.rindex("fn run") + 6,
            "too_many_lines",
        )[0]
        self.assertNotEqual(a, b)

    def test_nesting_magnitude_ignores_earlier_deeper_blocks(self) -> None:
        old = "fn work() { if a { if b { if c { deeper(); } } } if d { work(); } }"
        start = old.index("{ work();")
        key, magnitude = LINT.rust_site(
            "x.rs", old, start, start + 11, "excessive_nesting"
        )
        new = old.replace("if d { work(); }", "if d { if e { work(); } }")
        other = new.index("{ work();")
        key2, grown = LINT.rust_site(
            "x.rs", new, other, other + 11, "excessive_nesting"
        )
        self.assertEqual(key, key2)
        self.assertEqual(grown, magnitude + 1)

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
                                "code": {"code": "clippy::excessive_nesting"},
                                "message": "this block is too deeply nested",
                                "spans": [span],
                            },
                        }
                    )
                )
            rows = LINT.clippy_rows("\n".join(records * 2), root)
            self.assertEqual(list(rows.values()), [[2, 2]])
            self.assertTrue(debt_errors(LINT.clippy_rows(records[0], root), rows, rows))

    def test_only_retained_clippy_families_are_collected_and_ratcheted(self) -> None:
        retained = {
            "too_many_lines": 101,
            "too_many_arguments": 8,
            "cognitive_complexity": 26,
            "excessive_nesting": 6,
            "missing_safety_doc": 1,
        }
        removed = (
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
                    self.assertTrue(debt_errors({}, rows, rows))
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

    def pr_context(self, pulls: Mapping[str, list[dict[str, object]]]) -> str:
        """Run `context` on a two-parent HEAD with no event payload."""
        shas = {"HEAD": "merge", "HEAD^2": "second"}

        def fake_git(*args: str) -> str:
            if args[:2] == ("rev-list", "--parents"):
                return "merge first second\n"
            assert args[0] == "rev-parse", args
            return shas[args[1]] + "\n"

        def fake_gh(*args: str) -> str:
            route = args[1].removeprefix("repos/fork/")
            if route.startswith("commits/"):
                return json.dumps(pulls.get(route.split("/")[1], []))
            number = int(route.removeprefix("pulls/"))
            head = next(
                pull["head"]
                for found in pulls.values()
                for pull in found
                if pull["number"] == number
            )
            return json.dumps({"head": head, "body": f"PR {number}"})

        with (
            patch.dict("os.environ", {"GITHUB_REPOSITORY": "fork"}) as environment,
            patch.object(cleanup_contract, "git", fake_git),
            patch.object(cleanup_contract, "gh", fake_gh),
            patch.object(cleanup_contract, "discussion", return_value=[]),
            patch.object(cleanup_contract, "source_reviews", return_value=[]),
            patch.object(cleanup_contract, "records", side_effect=lambda body: [body]),
        ):
            environment.pop("GITHUB_EVENT_PATH", None)
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
                GROWTH.accepted(delta, [proposal], comments, "/implementation", "head")
            )
        self.assertTrue(
            GROWTH.accepted(
                delta,
                [proposal],
                [report("/review", "head")],
                "/implementation",
                "head",
            )
        )
        changed = dict(proposal, reason="different operation")
        self.assertFalse(
            reviewed(changed, [report("/review", "head")], "/implementation", "head")
        )


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


class CarryTests(unittest.TestCase):
    OLD = "rust/a.rs | fn curve_limit | too_many_lines | 4a33eacd5fa65f2b2e28"
    NEW = "rust/a.rs | fn get_curve_speed_limit | too_many_lines | 4a33eacd5fa65f2b2e28"

    def ratchet(
        self, previous: dict[str, list[int]], head: dict[str, list[int]], *carries: dict
    ) -> tuple[list[str], str]:
        output = io.StringIO()
        with (
            patch.object(
                LINT, "context", return_value=("/impl", list(carries), [], "h")
            ),
            contextlib.redirect_stdout(output),
        ):
            allowed = LINT.reviewed_debt("clippy", head, previous, head)
        return debt_errors(head, head, allowed), output.getvalue()

    @staticmethod
    def carry(old: str, new: str, tool: str = "clippy") -> dict[str, str]:
        return {"kind": "carry", "tool": tool, "from": old, "to": new}

    def test_rename_move_and_changed_nesting_fingerprint_carry(self) -> None:
        moved = self.NEW.replace("rust/a.rs", "rust/curve.rs")
        nest = "rust/a.rs | fn curve_limit | excessive_nesting | 2839a77535b6b381d6fa"
        renamed = "rust/a.rs | fn model | excessive_nesting | 0000000000aaaaaaaaaa"
        for old, new, values in (
            (self.OLD, self.NEW, [281]),
            (self.OLD, moved, [281]),
            (nest, renamed, [6]),
        ):
            with self.subTest(new=new):
                errors, _ = self.ratchet(
                    {old: values}, {new: values}, self.carry(old, new)
                )
                self.assertEqual(errors, [])
        # The check prints a ready record for a one-to-one same-lint pair.
        errors, printed = self.ratchet({self.OLD: [281]}, {self.NEW: [281]})
        self.assertTrue(errors)
        self.assertIn(
            "ready carry: " + json.dumps(self.carry(self.OLD, self.NEW)), printed
        )

    def test_invalid_ratchet_carries_fail(self) -> None:
        other = self.NEW.replace("too_many_lines", "too_many_arguments")
        for previous, head, carry in (
            ({self.OLD: [281]}, {self.NEW: [292]}, self.carry(self.OLD, self.NEW)),
            (
                {self.OLD: [281]},
                {self.OLD: [281], self.NEW: [281]},
                self.carry(self.OLD, self.NEW),
            ),
            (
                {self.OLD: [281], self.NEW: [120]},
                {self.NEW: [281]},
                self.carry(self.OLD, self.NEW),
            ),
            ({self.OLD: [8]}, {other: [8]}, self.carry(self.OLD, other)),
            (
                {self.OLD: [281]},
                {self.NEW: [281]},
                self.carry(self.OLD, self.NEW, "ruff"),
            ),
        ):
            with self.subTest(head=head, carry=carry):
                errors, printed = self.ratchet(previous, head, carry)
                self.assertTrue(errors)
                self.assertEqual(carry["tool"] == "clippy", "invalid carry" in printed)

    def test_carries_cannot_multiply_retired_debt(self) -> None:
        other = self.NEW.replace("get_curve_speed_limit", "another")
        for head, carries in (
            ({self.NEW: [281, 281]}, [self.carry(self.OLD, self.NEW)]),
            (
                {self.NEW: [281], other: [281]},
                [self.carry(self.OLD, self.NEW), self.carry(self.OLD, other)],
            ),
        ):
            self.assertTrue(self.ratchet({self.OLD: [281]}, head, *carries)[0])

    def test_component_reviews_use_agent_paths_under_shared_login(self) -> None:
        proposal = {"kind": "lint"}
        owner = acceptance_comment("h", proposal).replace("/review", "/impl")
        independent = acceptance_comment("h", proposal)
        body = '```cleanup-acceptance\n{"kind":"reviews","pulls":[12]}\n```'
        with (
            patch("cleanup_contract.gh", return_value=json.dumps({"body": owner})),
            patch("cleanup_contract.discussion", return_value=[owner, independent]),
        ):
            comments = source_reviews("repos/shared/account", body, 13)
        self.assertEqual(comments, [independent])
        self.assertTrue(reviewed(proposal, comments, "/integration", "h"))
        self.assertFalse(reviewed(proposal, [owner], "/impl", "h"))

    def test_attribute_and_signature_changes_invalidate_acceptance(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = "#[cfg_attr(test, allow(dead_code))]\npub fn work() {}\n"
            initialize(root, {"rust/x.rs": original})
            accepted = git("rev-parse", "HEAD", cwd=root).strip()
            site = "rust/x.rs | fn work | allow dead_code"
            row = "rust/x.rs | fn work | too_many_lines | item"
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
            row = "rust/x.rs | fn other | too_many_lines | 4a33eacd5fa65f2b2e28"
            lint = dict(
                proposal, kind="lint", tool="clippy", site=row, before=[], after=[120]
            )
            comment = acceptance_comment(reviewed_head, lint)
            for head, expected in ((unchanged, {}), (changed, {row: [120]})):
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
                        LINT.reviewed_debt("clippy", {row: [120]}, {}), expected
                    )


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
                [
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--locked",
                    "--",
                    "-D",
                    "warnings",
                ],
                "pub fn x() -> bool { true == true }\n",
                "equal expressions",
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
                '[package]\nname="seeded-checks"\nversion="0.1.0"\nedition="2024"\n'
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
            (root / "tools").mkdir()
            (root / "tools/x.py").write_text(
                "def many(x):  # noqa: C901\n"
                + "".join(f"    if x == {i}:\n        return {i}\n" for i in range(12))
            )
            raw, _ = LINT.collect("ruff", root)
            self.assertIn("C901", raw)


if __name__ == "__main__":
    unittest.main()
