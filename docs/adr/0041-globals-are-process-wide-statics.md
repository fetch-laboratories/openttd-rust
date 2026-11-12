---
status: accepted
may change: agents
---
# A C++ global or static becomes a process-wide Rust static, not thread-local storage

Rust offers thread-local storage, where each thread sees its own copy of a variable, and it looks like the safe home for C++ global state. But when the game runs with a window, commands run on the main thread and the game loop on another, `ottd:game`, taking turns under a lock. One port (#216) kept town house counts in thread-local storage, so the two threads saw different counts, and a NewGRF house set would crash the game. The comparison harness runs the game on one thread without a window, so it cannot see this kind of fault.

A C++ global or static variable becomes a single process-wide Rust static, not thread-local storage ([fidelity](../porting.md#fidelity)).

An agent made this decision in the eighth steering review on 2026-10-12 (commit `b419a3c913`).

Reversing it would bring back a kind of crash the harness cannot detect.
