---
status: accepted
may change: agents
---
# Rust calls C++ directly and waits for the answer

Rust code often needs a service the C++ side still provides, such as the shared random-number generator. The first ports handed each such call back to C++ through an asynchronous protocol, one that sets the work up to finish later. Every new port copied it, and each call allocated memory. On 2026-10-07 an agent's profile traced most of the Rust game's 2.54-fold slowdown on the main test save to this template and to whole-record reads (commit `215fc55567`).

Rust calls each shared service through its own typed C++ function that cannot throw an exception, and waits for the answer. Calls allocate nothing. An exception that escapes such a function ends the game. Failures outside normal play, such as running out of memory or a debug log failing to write, are not behaviour the port must match. Control returns to C++ through a protocol only where an exception can pass through the call in ordinary play: script engines and save or load errors ([boundary](../porting.md#boundary)).

An agent made this decision in a steering session the user started on 2026-10-04 (commit `1a883a37f1`, #107), and the 2026-10-07 audit tightened it (#168). #107 says the user ruled on the out-of-scope failures, but that session's log no longer exists and no words of the user were found. The rejected alternative was reproducing exception behaviour exactly in those failure paths.

Reversing it would undo the conversions built on it; #168 is still open for the rest.
