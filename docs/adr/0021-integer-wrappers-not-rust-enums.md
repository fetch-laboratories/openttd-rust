---
status: accepted
may change: agents
---
# Rust wraps the original's integer types instead of using Rust enums

The original game names its values with C++ enums, flag sets and ID types. The Rust port had dropped most of those names for bare numbers ([ADR 0004](0004-cleanup-before-porting.md)). The obvious fix, a Rust `enum` for each C++ enum, is unsafe here. A C++ enum variable can hold a number that is not on its list, and the original relies on that in places. A Rust enum holding such a number is undefined behaviour: the program may do anything.

Rust represents each of the original's enums, flag sets and IDs as a thin wrapper around the same integer, not as a Rust enum.

The overhaul agent made this rule on 2026-11-06, in the cleanup plan the user approved. It is recorded so that a later agent does not reopen the obvious alternative. [docs/porting.md](../porting.md) holds the detailed rules.

Changing it would mean rewriting every wrapper: 309 wrapper types and 3,662 named constants existed by merge 32.
