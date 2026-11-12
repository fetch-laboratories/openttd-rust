---
status: accepted
may change: agents
---
# Release builds keep Rust's integer overflow checks on and abort on panic

When a whole number grows past what its type can hold, C++ wraps it around or cuts it short. Rust's normal release build also wraps, silently. With overflow checks on, Rust stops the program instead, so a place where the port forgot to wrap the way the original does shows up as a crash, not as a quiet difference from the original.

Rust's build settings keep overflow checks on in release builds, and every build stops at once on a panic (Rust's crash) instead of unwinding. Where the original wraps or truncates, the Rust code does so explicitly; the checks only back up operations that cannot overflow in the original, and the port adds no checks the original lacks ([fidelity](../porting.md#fidelity)). The Rust code is always built with release settings, so Rust's debug assertions do not follow a C++ debug build.

Agents made this decision. The settings arrived with the first port on 2026-10-02 (commit `c2ecc4d57e`), and a steering session on 2026-10-03 added the rule of explicit wrapping (commit `cde1eea0ee`). The commits are the only record; no words of the user on it were found. The rejected alternatives were Rust's default release behaviour, faster but silent, and defensive checks the original lacks.

Turning the checks off is one line, but every port's review relies on them as a safety net.
