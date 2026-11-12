---
status: accepted
may change: user
---
# The port replaces the original game's parts in place, one at a time, inside the working game

A port can be written from scratch as a new game beside the original, or built by swapping the original's parts for Rust one at a time inside the working game. A from-scratch Rust version of OpenTTD, `openttdrs`, already existed.

The fork starts from the original OpenTTD repository. Its C++ parts are replaced with Rust one at a time, or in groups that must move together, and the game stays complete and playable throughout. The Rust code is compiled into a library that is linked into the game's program.

The approach is the user's. On 2026-10-02 they asked: "in theory, could we do the opposite? replace modules in openttd with rust in-place one at a time? (where possible, probably some will need changed in parallel)". The agent recommended it: the working game is always there to compare against, and each comparison covers only the part being replaced. It rejected `openttdrs` as a base because its systems behave differently from the original, its pathfinder's route costs among them, so matching the original would mean replacing them anyway. [ADR 0001](0001-full-rust-rewrite.md) is where this method ends: when the last part is replaced, the Rust build is the only build.

Reversing it would discard everything: the build, the calling layer between C++ and Rust, the comparison harness and every port assume it.
