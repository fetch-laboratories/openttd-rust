# Network and script

<!-- Add a new entry at the end of this file. -->

## Authentication and streaming owners

- Rust owns: X25519 sessions, encryption contexts, and stable key/session and vendor-context storage.
- C++ keeps: Bundled Monocypher algorithms; C++ supplies each vendor context's size/alignment and starts its trivial lifetime.
- Checks: `python3 tools/auth-comparison.py`.
- PRs: #43.

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

## ChaCha20, Poly1305 and AEAD primitives

- Rust owns: Monocypher 4.0.2 ChaCha20, Poly1305 and AEAD primitives.
- C++ keeps: C interfaces, context types and lifetime, wipe and verify leaves; other Monocypher algorithms.
- Checks: `python3 tools/auth-comparison.py` and generated layout probes.
- PRs: #52.

## BLAKE2b family

- Rust owns: BLAKE2b compression, initialization, buffering, digest serialization and context wiping.
- C++ keeps: Public C functions, context lifetime and wipe leaf.
- Checks: `python3 tools/auth-comparison.py`.
- Limits: Keys of 65-128 bytes fit the original padded block and keep source behavior (the documented interface limit is 64); keys over 128 overrun original storage and have no reproduction guarantee.
- PRs: #57.

## Packet framing, binary serialization and transfer state

- Rust owns: Packet limit and cursor, binary encoding/decoding, length prefixes, framing and transfer planning/commit.
- C++ keeps: Vector/string storage, socket policy, encryption/transfer callbacks and string/byte collection and sanitation.
- Checks: `python3 tools/auth-comparison.py`, scripted regressions and `python3 tools/packet-comparison.py`.
- PRs: #60.

## Normal X25519 and Montgomery ladder

- Rust owns: `crypto_x25519`, `crypto_x25519_public_key`, `crypto_eddsa_trim_scalar`, the Montgomery ladder and private field operations.
- C++ keeps: Public interfaces, shared-field helpers for untouched Edwards, conversions, Elligator and other families, wipe and verify leaves.
- Checks: `python3 tools/auth-comparison.py`.
- Limits: Shared-field helper duplication is unfinished migration; compiler spills/copies are not erased.
- PRs: #61.
