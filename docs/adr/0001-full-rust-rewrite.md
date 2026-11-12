---
status: accepted
may change: user
---
# The whole game is rewritten in Rust, and the Rust build is the only build

Every port used to keep the original C++ body behind a compile switch (`WITH_RUST`), so the game could still be built without Rust. As long as any build used that C++ fallback, no original line could be deleted. In October 2026, `src/` held 2,369 of these switches, and CI (the automated build and test runs on GitHub) spent two builds on code the Rust game did not run ([#581](https://github.com/fetch-laboratories/openttd-rust/issues/581)).

Every line of C++ in the repo is eventually deleted: the original game code, the glue between C++ and Rust, the tools and the tests. There is no C++-only build, so the Rust build is the only build. New tests are written in Rust or as scenarios for the comparison harness ([ADR 0033](0033-correctness-is-compared-game-state.md)), not in C++. A quick CI check rejects any new `WITH_RUST` switch in `src/`.

The goal is the user's from the first day. On 2026-10-02 they told the founding agent: "your goal is to continue until all modules are replaced with rust." On 2026-10-28 they repeated it: "of course the code will eventually be retired. we are REWRITING." Agents then planned and carried out the removal of the fallback in #581, closed on 2026-10-31; the record holds no message from the user about that step. [ADR 0023](0023-cpp-tests-stay-until-code-is-ported.md) covers C++ tests that already exist.

Reversing this would be very costly. No figure is recorded, but every port since #581 assumes there is no fallback build.

## History

From the first port on 2026-10-02 until #581, each port kept its C++ original under `#ifndef WITH_RUST` for a "portable build" without Rust. This ADR replaced that design, so the fallback is not a safety net to restore.
