---
status: accepted
may change: user
---
# The whole game is rewritten in Rust, and the Rust build is the only build

The fork began by porting pieces of OpenTTD to Rust while keeping each original C++ body behind a compile switch, so the game could still be built without Rust. As long as any build used that C++ fallback, no original line could be deleted. In October 2026, `src/` held 2,369 of these switches (`WITH_RUST` conditionals), and CI (the automated build and test runs on GitHub) spent two builds on code the Rust game never ran ([#581](https://github.com/fetch-laboratories/openttd-rust/issues/581)).

Every line of C++ in the repo is eventually deleted: the original game code, the glue between C++ and Rust, the tools and the tests. There is no C++-only build of the game any more, so the Rust build is the only build. Because all the C++ goes, new tests are written in Rust or as scenarios for the comparison harness (the tool that runs the original game and the Rust game side by side), never in C++.

On 2026-10-28 the user wrote: "of course the code will eventually be retired. we are REWRITING." Agents then planned and carried out the removal of the fallback build in #581, closed on 2026-10-31; the record holds no message from the user about that step. Writing new tests in Rust was part of the cleanup plan the user approved on 2026-11-06. A new C++ test is code that must be deleted again later. ADR [0023](0023-cpp-tests-stay-until-code-is-ported.md) covers C++ tests that already exist.

Reversing this would be very costly. No figure is recorded, but every port since #581 assumes there is no fallback build.
