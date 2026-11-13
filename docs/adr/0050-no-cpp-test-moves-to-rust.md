---
status: accepted
may change: user
---
# No C++ test is moved to Rust; each retires with the code it reaches

This replaces [ADR 0023](0023-cpp-tests-stay-until-code-is-ported.md), "C++ tests of C++ code stay in C++ until that code is ported". It also ends the fourth part of [ADR 0004](0004-cleanup-before-porting.md), "Porting stays paused until the cleanup is finished", which moved tests out of C++ where that removed more C++ than it added (#736).

ADR 0023 kept only C++ tests of C++ code in C++. Agents read that as leaving C++ tests of Rust code free to move, and after it was recorded they moved two more probes (#936 and #955): about 4,800 changed lines to delete about 320 counted lines of C++ probes, less the recipe C++ they added. Over the cleanup stage, 21 pull requests removed about 4,400 raw lines of C++ under `tools/` and `src/tests` and added about 25,000 lines of Rust tests and recorded expectations. The late transfers cost 7.1 to 11.0 changed lines for each net C++ line, one (#931) added C++, and the last (#950) projected 10 to 13.

No C++ test is moved to Rust or the harness. A C++ test stays in C++ and is deleted inside the port that removes the code, binding, pool or boundary it reaches; that port carries its observations. New tests are still written in Rust ([ADR 0001](0001-full-rust-rewrite.md)). The tests already moved stay in Rust.

The decision is the user's. On 2026-11-12, agreeing to ADR 0023's draft, the user wrote: "the C++ tests of C++ code should not be converted to Rust that would be fucking stupid." On 2026-11-13, on learning what #736 had been: "i told the agents that transferring tests from C++ to Rust is fucking stupid! they're going to be retired anyway!"

The cost is that the remaining C++ tests are compiled and patched until their owners retire them, and run on Linux only. Each has a named owner in #736's classification. Reversing this spends agent-hours moving tests that a later port deletes.

## History

ADR 0023 stood from 2026-11-12 to 2026-11-13.
