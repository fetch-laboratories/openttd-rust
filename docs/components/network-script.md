# Network and script

<!-- Add a new entry at the end of this file. -->

## Authentication and maintained cryptographic constructions

- Rust owns: Single and combined authentication handlers, method selection and fallback, X25519 sessions, key derivation, packet encryption streams, UID/file hashing and plugin signature verification. Pinned maintained crates supply X25519, BLAKE2b, ChaCha20, Poly1305 and SHA-512; Rust composes Monocypher 4 incremental rekeying and cofactored Ed25519 verification.
- C++ keeps: Native virtual facades, password/authorized-key providers and UI callbacks, file reads, UID clock/formatting and secret-key string validation. Packet operations use the integrated Rust packet owner directly.
- Checks: Rust known-answer tests from the pinned original, `python3 tools/auth-comparison.py`, `python3 tools/packet-comparison.py` and mixed multiplayer modes.
- Limits: Original packet short-source preconditions remain in #41. Cryptographic security and complete secret erasure are not established by these checks.
- PRs: #43, #52, #57, #61, #642.

## Paired Script Admin conversion

- Rust owns: Both Script Admin VM/JSON conversion directions and their control flow.
- C++ keeps: `ScriptAdminMakeJSON` and `ScriptEventAdminPort::GetObject` interfaces, VM, JSON objects, network framing and exception cleanup.
- Checks: `python3 tools/admin-conversion-comparison.py`.
- Limits: Game-log storage, arbitrary depths, network simulation and complete script equivalence are outside the comparison.
- PRs: #45.

## ScriptList storage and iteration

- Rust owns: ScriptList indexes, sorting, iteration, mutation accounting, filters and list algebra. Rank arithmetic and modification tokens wrap (#260).
- C++ keeps: Script identity/bindings, pool enumeration, VM/error/operation charging and save/load adapters.
- Checks: `python3 tools/script-list-comparison.py` and the token-wrap unit test.
- Limits: Callbacks that invalidate iterators while evading the modification check remain outside the checks.
- PRs: #46.

## ScriptList VM control

- Rust owns: `Valuate`, VM-filtered `FillList`, `SaveObject`, `LoadObject`, `_get`, `_set` and `_nexti` control.
- C++ keeps: VM actions, live pool reads, diagnostics and command/operation-limiter scopes.
- Checks: `python3 tools/script-list-comparison.py` and generated AI/GameScript/template bindings.
- Limits: Original failed-getter undefined values (#67), unrepresentable conversions, signed overflow and invalidated iterators remain outside defined behavior. The comparison fixture uses four typed items; only the game regressions cover production pools and the CPU limit.
- PRs: #70.

## Packet storage, framing and serialization

- Rust owns: Packet byte allocations, growth, copy and destruction, limit and cursor, typed binary and string send/receive, validation, length prefixes, framing and transfer planning/commit. The shared record is generated from Rust; native code reads its fields in place. Rust network callers use the typed packet API directly.
- C++ keeps: The Packet facade, socket and queue shells, result string/vector allocations and callable transfer frames. Native virtual encryption implementations retain their exception frames; Rust streams are resolved through a native metadata accessor and called directly.
- Checks: `python3 tools/auth-comparison.py`, scripted regressions and `python3 tools/packet-comparison.py`.
- Limits: The original nonempty-short-source `Recv_bytes` precondition remains deferred in #41. Allocation failure is environmental.
- PRs: #60, #645.
