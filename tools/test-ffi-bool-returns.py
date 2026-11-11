#!/usr/bin/env python3
"""Distinguish unsafe C bool returns from byte returns and inbound declarations."""

import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "ffi_bool_returns", Path(__file__).with_name("ffi-bool-returns.py")
)
assert spec and spec.loader
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class BoolReturnTests(unittest.TestCase):
    def check(self, source: str) -> list[str]:
        return checker.violations({"entry.rs": source})

    def test_bool_and_byte_definitions_are_distinguished(self) -> None:
        for result, failures in (("bool", 1), ("u8", 0)):
            with self.subTest(result=result):
                source = f'pub unsafe extern "C" fn entry(x: bool) -> {result} {{ x.into() }}'
                self.assertEqual(len(self.check(source)), failures)

    def test_callbacks_alias_chains_and_nested_parameters(self) -> None:
        source = """type Truth = bool;
type Result = Truth;
unsafe extern "C" fn callback<T>(x: extern "C" fn(bool) -> bool) -> Result { x(true) }
"""
        self.assertEqual(len(self.check(source)), 1)
        self.assertEqual(
            len(
                checker.violations(
                    {
                        "types.rs": "type Truth = bool;",
                        "entry.rs": 'extern "C" fn entry() -> types::Truth { true }',
                    }
                )
            ),
            1,
        )

    def test_equivalent_c_abi_and_return_type_spellings(self) -> None:
        for abi in ('extern "C"', "extern"):
            for result, failures in (("bool", 1), ("u8", 0)):
                for spelling in (
                    result,
                    f"({result})",
                    f"((::core::primitive::{result}))",
                ):
                    with self.subTest(abi=abi, spelling=spelling):
                        source = f"{abi} fn entry() -> {spelling} {{ false.into() }}"
                        self.assertEqual(len(self.check(source)), failures)

    def test_parenthesized_aliases_and_renamed_imports(self) -> None:
        for result, failures in (("bool", 1), ("u8", 0)):
            for binding in (
                f"type Truth = ((::core::primitive::{result}));",
                f"use core::primitive::{result} as Truth;",
            ):
                with self.subTest(binding=binding):
                    source = (
                        binding + '\nextern "C" fn entry() -> Truth { false.into() }'
                    )
                    self.assertEqual(len(self.check(source)), failures)

    def test_file_test_modules_are_mocks(self) -> None:
        mock = 'extern "C" fn belongs() -> bool { true }'
        for parent, child, declaration, failures in (
            (
                "a/industry.rs",
                "a/industry/evidence.rs",
                "#[cfg(test)]\nmod evidence;",
                0,
            ),
            ("a/lib.rs", "a/evidence.rs", "#[cfg(test)] mod evidence;", 0),
            ("a/industry.rs", "a/industry/evidence.rs", "mod evidence;", 1),
            ("a/industry.rs", "a/evidence.rs", "#[cfg(test)]\nmod evidence;", 1),
        ):
            with self.subTest(parent=parent, child=child, declaration=declaration):
                found = checker.violations({parent: declaration, child: mock})
                self.assertEqual(len(found), failures)

    def test_declarations_parameters_out_pointers_and_comments_are_allowed(
        self,
    ) -> None:
        self.assertEqual(
            self.check("""unsafe extern "C" { fn imported() -> bool; }
extern "C" fn store(input: bool, output: *mut bool) {}
fn rust_bool() -> bool { true }
// extern "C" fn comment() -> bool { false }
const TEXT: &str = "extern \\"C\\" fn string() -> bool { false }";
"""),
            [],
        )

    def test_transparent_wrappers_follow_the_field_return_abi(self) -> None:
        for scalar, failures in (("bool", 1), ("u8", 0)):
            for field in ("(pub Scalar);", "{ pub value: Scalar, }"):
                with self.subTest(scalar=scalar, field=field):
                    files = {
                        "types.rs": f"""type Scalar = ({scalar});
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Flag {field}
#[repr(transparent)]
pub(crate) struct Outer(pub(crate) Flag);
type Result = Outer;
""",
                        "entry.rs": 'extern "C" fn entry() -> types::Result { todo!() }',
                    }
                    self.assertEqual(len(checker.violations(files)), failures)
        self.assertEqual(
            self.check('struct Flag(bool); extern "C" fn entry() -> Flag { todo!() }'),
            [],
        )

    def test_macro_definitions_and_return_type_arguments(self) -> None:
        self.assertEqual(
            len(
                self.check("""macro_rules! entry {
    ($name:ident) => { extern "C" fn $name() -> bool { false } };
}""")
            ),
            1,
        )
        self.assertEqual(
            len(
                self.check("""macro_rules! entry {
    ($name:ident, $ret:ty) => { extern "C" fn $name() -> $ret { 0 } };
}
entry!(test => bool);
""")
            ),
            1,
        )

    def test_rust_only_service_mocks_are_excluded(self) -> None:
        self.assertEqual(
            self.check("""#[cfg(test)]
mod tests {
    extern "C" fn mock() -> bool { true }
}
#[cfg(test)]
extern "C" fn fixture() -> bool { true }
"""),
            [],
        )


if __name__ == "__main__":
    unittest.main()
