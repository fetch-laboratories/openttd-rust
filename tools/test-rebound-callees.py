#!/usr/bin/env python3
"""Exercise the rebound service-table callee scan and its shrinking baseline."""

import contextlib
import importlib.util
import io
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "rebound", Path(__file__).with_name("rebound-callees.py")
)
assert SPEC and SPEC.loader
REBOUND = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(REBOUND)

SERVICES = (
    "inline uint32_t RustTileOwner(uint32_t tile) noexcept { return tile; }\n"
    "template <class T> uint16_t RustSpecIndex(const T *spec) noexcept;\n"
    "namespace ns { bool RustValid(int id) noexcept; }\n"
    "const OpenTTDSharedServices &GetRustSharedServices() noexcept;\n"
    "size_t capacity() const noexcept;\n"
)


class ReboundTests(unittest.TestCase):
    def test_comments_and_directives_before_a_single_field(self) -> None:
        files = {"src/services.h": SERVICES}
        for name, initializer in {
            "a": "{/* note */ .owner = RustTileOwner}",
            "b": "{\n#ifdef WITH_RUST\n.owner = RustTileOwner\n#endif\n}",
            "c": "{. /* note */ owner /* note */ = RustTileOwner}",
            "d": "{// note\n.owner = RustTileOwner}",
        }.items():
            files[f"src/{name}.cpp"] = f"const S {name}{initializer};\n"
        self.assertEqual(
            REBOUND.violations(files),
            {f"rebound | RustTileOwner | src/{name}.cpp | owner" for name in "abcd"},
        )

    def test_rebound_and_forwarded_fields(self) -> None:
        files = {
            "src/rust/services.h": SERVICES,
            "src/town.cpp": "static const OpenTTDTownServices services{\n"
            "\t.owner = RustTileOwner,\n"
            "\t.index = RustSpecIndex<HouseSpec>,\n"
            "\t.valid = &ns::RustValid,\n"
            "\t.lambda = [](uint32_t t) noexcept { return RustTileOwner(t); },\n"
            "\t.empty = STR_EMPTY,\n"
            "\t.shared = &GetRustSharedServices(),\n"
            "\t.snow = GetRustSharedServices().snow_line,\n"
            "};\n"
            "void Run(size_t capacity) { Use({.capacity = capacity}); }\n",
            "src/industry.cpp": "const OpenTTDIndustryServices table = {\n"
            "#ifdef WITH_RUST\n"
            "\t.tile_owner = RustTileOwner,\n"
            "#endif\n"
            "\t.index = RustSpecIndex<IndustrySpec>, .once = RustOnce,\n"
            "\t// .valid = ns::RustValid,\n"
            '\t.name = "{.valid = ns::RustValid}",\n'
            "\t.nested = {.inner = RustTileOwner},\n"
            "};\n",
            "src/tests/stub.cpp": "const X stub{.valid = ns::RustValid};\n",
            "rust/lib.rs": "const X: T = T { valid: RustValid };\n",
        }
        self.assertEqual(
            REBOUND.violations(files),
            {
                "rebound | RustTileOwner | src/town.cpp | owner",
                "rebound | RustTileOwner | src/industry.cpp | tile_owner",
                "rebound | RustTileOwner | src/industry.cpp | inner",
                "rebound | RustSpecIndex | src/town.cpp | index",
                "rebound | RustSpecIndex | src/industry.cpp | index",
                "forwarded | src/town.cpp | snow = GetRustSharedServices().snow_line",
            },
        )
        files["src/station.cpp"] = "const S a{.valid = ns::RustValid};\n"
        files["src/water.cpp"] = "const S a{.owner = RustTileOwner};\n"
        hits = REBOUND.violations(files)
        self.assertIn("rebound | ns::RustValid | src/station.cpp | valid", hits)
        self.assertIn("rebound | ns::RustValid | src/town.cpp | valid", hits)
        self.assertIn("rebound | RustTileOwner | src/water.cpp | owner", hits)
        files["src/water.cpp"] += "const S b{.owner = RustTileOwner};\n"
        self.assertIn(
            "rebound | RustTileOwner | src/water.cpp | owner | 2",
            REBOUND.violations(files),
        )

    def test_cli_ratchet_and_stale_entries(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(REBOUND, "ROOT", Path(directory)),
        ):
            root = Path(directory)
            baseline = "rebound | RustTileOwner | src/a.cpp | owner\n"
            baseline += "rebound | RustTileOwner | src/b.cpp | owner\n"
            for path, text in {
                "src/services.h": SERVICES,
                "src/a.cpp": "const A a{.owner = RustTileOwner};\n",
                "src/b.cpp": "const B b{.owner = RustTileOwner};\n",
                REBOUND.BASELINE: baseline,
            }.items():
                target = root / path
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text(text)
            REBOUND.git("init", "-q")
            REBOUND.git("add", ".")
            REBOUND.git(
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-qm",
                "Add: fixture",
            )

            def check() -> int:
                with (
                    patch.object(
                        sys, "argv", ["rebound-callees.py", "--base-ref", "HEAD"]
                    ),
                    contextlib.redirect_stdout(io.StringIO()),
                ):
                    return REBOUND.main()

            self.assertEqual(check(), 0)
            (root / "src/c.cpp").write_text("const C c{.owner = RustTileOwner};\n")
            self.assertEqual(check(), 1)
            with (root / REBOUND.BASELINE).open("a") as output:
                output.write("rebound | RustTileOwner | src/c.cpp | owner\n")
            self.assertEqual(check(), 1)  # Adding exemptions never fixes it.
            (root / "src/c.cpp").unlink()
            (root / "src/b.cpp").write_text("const B b{.owner = RustOther};\n")
            (root / REBOUND.BASELINE).write_text(baseline)
            self.assertEqual(check(), 1)  # Retired exemptions must disappear.
            (root / REBOUND.BASELINE).write_text("")
            self.assertEqual(check(), 0)


if __name__ == "__main__":
    unittest.main()
