---
status: accepted
may change: agents
---
# C++ tests of C++ code stay in C++ until that code is ported

The cleanup moves tests off C++ where that removes more C++ than it adds ([ADR 0004](0004-cleanup-before-porting.md)). Moving a test that checks code still written in C++ does not: the test needs C++ glue to reach that code, and it is lost or weakened along the way. By 2026-11-11 such moves had delivered far less than planned. Two of them removed 27 of 951 and 154 of 836 expected lines, and six observations were lost or weakened. On 2026-11-12, #931 measured a move of four whole test files at 29 more lines of C++, not fewer.

A C++ test that checks code still written in C++ stays in C++ until that code is ported. It is not converted to Rust early.

Agents made this decision: the 34th steering review on 2026-11-11, and root after #931's measurement on 2026-11-12. New tests are still written in Rust ([ADR 0001](0001-full-rust-rewrite.md)). The C++ test is deleted when the code it checks moves to Rust.

Reversing it would cost agent-hours on moves that add C++ and lose observations.
