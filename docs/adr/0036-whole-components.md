---
status: accepted
may change: agents
---
# Rust takes over whole components, state and control flow together

A port can move one algorithm into Rust while C++ keeps the data and the loop that calls it, or move a whole component, data and control flow together. By 2026-10-03 the first ports had added about 5,800 lines of Rust, 5,600 lines of comparison tooling and 3,300 lines of new C++ glue, almost all of it for utilities and bundled libraries. Each extracted fragment added a new crossing between the languages, layout checks and a comparison tool of its own, but retired little C++ (commit `923f697159`).

A port moves a component's data and its control loop into Rust. C++ keeps only a thin front end and calls for shared services. Moving an algorithm fragment while C++ keeps the data and loop is allowed only where the roadmap names it as a stepping stone ([ownership ports](../porting.md#ownership-ports)).

An agent made this decision in a steering session on 2026-10-03. That session's log no longer exists; its commit is the only record.

Reversing it would undo the work order since 2026-10-03 and the owner and shell vocabulary the rule docs use.

## History

On 2026-10-02 the user told the founding agent to "prioritize areas where you do not need to build additional comparisons. modules that are already heavily tested". The founding rules followed that. The 2026-10-03 session reversed it, because the tested modules were utilities and bundled cryptography, not the game. No message from the user approving the reversal was found.
