---
status: accepted
may change: agents
---
# Typed C++ IDs cross the calling layer as their plain integer types

The original wraps many integers in small C++ classes, such as company and vehicle IDs. When a function returns such a class, Microsoft's Windows compiler hands it back through a hidden pointer, while Rust expects it in a register. A service that returned a company ID this way gave wrong results on Windows only ([#355](https://github.com/fetch-laboratories/openttd-rust/issues/355)).

Typed C++ IDs, such as `PoolID` and `StrongType` values, are passed and returned between C++ and Rust as their plain integer types. Inside each language they keep their typed wrappers ([boundary](../porting.md#boundary)).

An agent made this decision in the 17th steering review on 2026-10-22 (commit `db8bb1aa39`).

Reversing it would be quick to type, but the fault it brings back shows only on Windows, so Linux and macOS testing would not catch it.
