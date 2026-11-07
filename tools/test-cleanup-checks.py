#!/usr/bin/env python3
"""Seed independent cleanup gate failures, including relocations and constant data."""

import importlib
import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from types import ModuleType
from unittest.mock import patch

from baseline_ratchet import debt_errors
from cleanup_contract import associated_pull, reviewed
from rust_syntax import boundary_ids
from source_fixture import initialize


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
            "operation": "native cast",
            "source_contract": "original narrows to u8",
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
        text = "impl A { fn run() { unsafe { use_it(); } } }\nimpl B { fn run() {} }"
        start = text.index("unsafe")
        key, _ = LINT.rust_site(
            "x.rs", text, start, start + 8, "undocumented_unsafe_blocks"
        )
        moved = "// extra line\n\n" + text
        key2, _ = LINT.rust_site(
            "x.rs",
            moved,
            moved.index("unsafe"),
            moved.index("unsafe") + 8,
            "undocumented_unsafe_blocks",
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
            text = "fn run() { let é = 1; let _ = é as u8; let _ = é as u8; }"
            (root / path).write_text(text)
            records = []
            for start in (text.index("é as"), text.rindex("é as")):
                span = {
                    "is_primary": True,
                    "file_name": path,
                    "byte_start": len(text[:start].encode()),
                    "byte_end": len(text[: start + 7].encode()),
                }
                records.append(
                    json.dumps(
                        {
                            "reason": "compiler-message",
                            "message": {
                                "code": {"code": "clippy::cast_sign_loss"},
                                "message": "casting may lose the sign",
                                "spans": [span],
                            },
                        }
                    )
                )
            rows = LINT.clippy_rows("\n".join(records * 2), root)
            self.assertEqual(list(rows.values()), [[1, 1]])

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
