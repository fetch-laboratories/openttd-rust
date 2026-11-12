---
status: accepted
may change: agents
---
# Progress is measured by net lines of C++ removed

The fork used to measure progress by lines of the original game ported to Rust. On 2026-11-06 an agent found that the measure missed what was happening. It left out 16.6 thousand lines of C++ test programs under `tools/`. The six open storage drafts removed 3,237 lines of game C++ but added 1,037 lines of that test C++, giving back about a third. And the calling layer between C++ and Rust (Rust functions exported to C++, and callback slots) kept growing.

Progress is measured by net lines of C++ removed from the repo, counting tools and tests, not by lines of the original game ported. The calling layer between C++ and Rust must not grow without a reason a reviewer accepts.

The measure was in the plan the user approved on 2026-11-06 ("Yardstick: net fork C++ removed, counting `tools/` and tests"). Only code lines count: blank and comment lines were dropped at the user's request on 2026-10-28 ("that seems really dumb"). The measure guides the work but does not set each phase's goal; on 2026-10-31 the user wrote "removing glue code is a factor not the whole point." Since every line of C++ must go ([ADR 0001](0001-full-rust-rewrite.md)), C++ added anywhere is work added. [docs/planning.md](../planning.md) holds how owners are ranked and costed by it.

Changing it would change how every piece of work is ranked and gated, and break the comparison with every earlier stocktake.
