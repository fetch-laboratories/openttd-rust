---
status: accepted
may change: user
---
# The game is ported to Rust, not Go

The project began on 2026-10-02 with the user asking an agent to "evaluate the viability of rewriting openttd in either rust or go. choose the better option." Both were credible. Go is memory-safe, its garbage collector (automatic memory cleanup) would suit the game, and it has a game engine suited to automated testing.

The game is ported to Rust.

The agent recommended Rust for three reasons. Its compiler checks who owns and changes each piece of memory, which catches an autonomous agent's mistakes before the game runs. Rust code can sit beside C++ in one program, which made the in-place method of [ADR 0029](0029-replace-in-place.md) possible. And it makes a long-running simulation's state explicit. The user did not state the choice in words but acted on it, asking the same evening for the fork to be called "just OpenTTD-Rust or something like that".

Reversing it would throw away every port: about 200,000 lines of Rust in 355 files under `rust/` by 2026-11-12.
