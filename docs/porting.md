# Porting

For agents writing or reviewing port code, tests and Python tools.

## Terms

| Term | Meaning |
| --- | --- |
| Owner | Rust state that holds a component's data and runs its control flow. It may be one allocation per vehicle or a process-wide static. |
| Shell | A C++ object that holds a Rust owner and keeps its pool identity, fields not yet moved to Rust and save aliases. It is usually a pool item such as `Vehicle`, `Station` or `Town`. |
| Canonical state | The single authoritative copy of a datum, in Rust or C++. Canonical native state stays in C++, as with the map arrays. Read and write it in place without a mirror. |
| Facade | A C++ function or method that keeps the original signature and forwards to Rust. |
| Leaf | A typed `noexcept` C++ service that Rust calls for a shared operation or a field of a record still stored in C++. Shared operations include `Random`, sound and viewport. A leaf returns without reentering its calling owner. |
| Service table | A C++ struct of typed leaf function pointers, such as `OpenTTDTreeServices`. C++ uses designated initializers; Rust borrows the table for a call. Each field names one service, never an opcode. |
| Witness | A named branch counter in `witness.rs`. A scenario that sets `witnesses` requires each listed counter to be nonzero. |

## Ownership ports

- Prefer ports where Rust owns state and control flow. C++ keeps a thin facade and calls for shared operations (`Random`, allocation, progress, sound) and fields of records still stored in C++.
- Do not extract an algorithmic fragment while C++ keeps its state and loop unless the roadmap names it as a stepping stone.

## Storage ports

- The rewrite retires object classes and map accessors in headers (user, 2026-10-28). Once control flow is in Rust, move records to Rust: pool objects, then map arrays, in the roadmap's order.
- A pool's storage is moved when Rust owns its slot array, occupancy and allocation bookkeeping, allocation/free/iteration/reset policy and lifetime ordering, and no Rust code reaches a record field through a C++ service. Move every field Rust reads or writes, across one or more owners. A field stays in its C++ shell only while no Rust code touches it, and moves with the owner of its last C++ consumer; the shell and native registration facade retire with their callers. The map owner likewise takes array allocation/reset and the remaining map bodies. A shared-record checkpoint alone completes neither pool nor map ownership.
- Define each moved record once in Rust with a C-compatible layout. Generate its C++ declaration and check the layout at build. `tools/port-metrics.py` counts the declaration as compiled C++ glue. Both languages access fields in place.
- Unported C++ drawing, GUI, save/load, scripts and network use inline accessors over the shared layout, never a call per field. Only operations such as allocation, deletion, iteration with hooks and virtual behavior cross as calls. Delete replaced per-field services and reader carriers, together with their rows in the rebound-callee and shared-accessor baselines, and report the rows removed in the PR. Move a field and delete its Rust-called services in the same change. Do not route a service through the shared record (`Record()`) as an intermediate step.

## Fidelity

- Reproduce the original over its whole reachable input domain; add no guards, assertions, rejections or fatal paths absent from it. Where C++ wraps or truncates, use `wrapping_*` operations or `as` casts. Release overflow checks and `panic = "abort"` may only back up operations that cannot overflow in the original. This applies to new work; track known existing divergences as issues.
- Preserve state scope: a C++ global or static becomes a process-wide Rust static, never `thread_local!`. GUI input and single-player commands run on the main thread, the game loop on `ottd:game`, serialized by the game-state mutex; the single-threaded `-vnull` harness cannot reveal the difference.

## Boundary

- Call `Random`, pool operations, other shared services and field accessors for records still stored in C++ directly through `noexcept` C++ wrappers. Preserve the original's decisions and the order of its observable effects; its function boundaries are not part of that. An escaping exception terminates.
- Do not batch RNG or defer writes across callbacks. Keep source computation out of callback bodies.
- Allocation failure, debug or log I/O errors, and paths needing developer-only defines such as `RANDOM_DEBUG` are environmental failures, not simulation behavior.
- Use an action protocol to return control to C++ only where an exception can unwind through the call in ordinary play (script VMs, save/load errors). Name each such service. Reentry alone is not a reason: end every Rust borrow before a callback that can reenter Rust or mutate its state, then call directly.
- Use synchronous calls for entries. Do not use async, futures, tasks or mailboxes. Crossings allocate nothing per call. Where the original builds a local container, use a local `Vec`, not process-wide unsafe scratch.
- Give each service its own typed `noexcept` function, not an opcode switch or positional array. Return real types on both sides. Use designated initializers for tables.
- Access map and shared-layout record fields by field-sized raw load or store at the original observation point. For a record still stored in C++, resolve its owner once per entry. Use its shared typed field accessors.
- The following are design faults. Fix them before integration regardless of timing. Root grants no exceptions.
  - A reader or carrier written for one call site, including a service per `Debug` or `GrfMsg` message.
  - Allocation on a hot path or per call where the original has none.
  - Opcode dispatch or whole-record copies.
  - A Rust-to-C++-to-Rust bounce into code Rust owns.
- Pass C++ class types such as `PoolID` and `StrongType` as their base types: MSVC returns class types through a hidden pointer (#355). Use the existing shared transparent ID newtypes and opaque pool-pointer marker types from their header modules.

## Callees and shared accessors

- Call the original helper, virtual method or named constant via its shared accessor or Rust port, not a hand-written equivalent. A raw store for a setter, a hand-written bound for `TileAddWrap`, or a literal for `INVALID_RAILTYPE` can diverge when the callee changes.
- Port each inline helper or constant from an original header once into the shared Rust module named after that header; every port imports it there. Examples: `slope_func.h`, `map_func.h`, `tunnelbridge.h`, command flags and price indices.
- Each shared accessor (vehicle field, map predicate, setting or constant) has one typed definition for every port; command ports share one set of command-check services. Quick CI's [shared-code checks](rust-migration.md#shared-code-checks-and-port-metrics) reject copies and rebound callees within their syntactic scope; review covers the rest.
- A PR adding a shared accessor deletes integrated copies; one removing a caller deletes its orphaned export, declaration or callback field. `tools/orphan-exports.py` counts no test or harness as a user of an `openttd_rust_*` export, and `tools/rust-only.py` rejects `WITH_RUST` in `src/`: move such a fixture onto the Rust entry and delete the body.

## Map access

- Simulation Rust reads and writes map arrays in place through the [shared map descriptor](design/world-state.md). Load it rather than call for it; `map_func::World` is its single type. Do not use a service per field or predicate.
- Prove the descriptor's interval once, not per entry: `Map::Allocate` in new games and loaders, and game-state mutex yields for generation progress and GRF scanning, run outside the game loop and command execution. Re-prove it if a change adds a reset or yield there. Raw field loads use no Rust references and end before a callback that can load, reset or yield.
- Shared header helpers preserve native assertions: on a failed precondition call the native assertion as the original accessor does, never the C++ accessor; each conversion deletes its replaced service. C++ keeps canonical map storage and accessors until the map storage port. Retire existing typed map services hottest first (#565), otherwise when a port touches them.

## Evidence

- Game-logic ports require the semantic simulation harness (`python3 tools/migration.py simulate`, #72) and existing tests. Integrate a new port only after harness scenarios exercise its component. Fidelity fixes to existing ports, such as #75, do not wait for that. Phases 3 to 6 reuse the harness where it observes output, such as saves, script commands and multiplayer sessions.
- New evidence lives in Rust tests or harness scenarios (user, 2026-11-06). C++ under `tools/` and `src/tests` is fork C++ that must retire; it shrinks or holds. The one exception is an observation of the C++ calling boundary itself that Rust tests and the harness cannot make: name it in the PR, with the boundary whose retirement removes it, for reviewer acceptance. Before removing an existing fixture, carry its observation into Rust tests or the harness with expectations taken from the original ([final exit](planning.md#goal-and-phases)), and register the replacement in `migration/evidence-replacements/` (`tools/evidence_replacements.py`). Quick's lost-cases step rejects a removed native CTest case without its `ctest:` or `source:` record; review judges that the named replacements carry every original observation.
- Extend harness scenarios instead of writing a comparison tool for each component. Put harness tooling for a port in its component's scenario module, built on the shared helpers: `tools/probe.py` for native gap probes, and `tools/simulation/core.py` and `runtime.py` for games, role builds, runtime copies and console capture. A port that touches a module with its own copy of one converts that copy (#673). Tooling is code to maintain: keep it proportionate to the C++ the port retires. Compare against unchanged reference bodies only for a concrete gap the harness cannot reach, named in the PR.
- Any check cited by a doc or PR, such as a branch witness or reference probe, runs in default CI and fails when missing; an opt-in profile or `__main__`-only probe is not evidence.
- A frozen oracle's capture recipe is provenance, not a CI check ([root, 2026-11-10](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6770051887)). A PR that changes an oracle, its recipe or the pin reruns `tools/oracle-capture.py` for the oracles it touches and reports the result, and its reviewer repeats the run.
- Third-party game content used only as test input, such as BaNaNaS AIs, NewGRF sets and their libraries, stays out of Git and out of every CI artifact (the repository is public). Pin each package by name, version and SHA-256 in a manifest, fetch it from the BaNaNaS CDN into ignored `.local/` storage, cache it in CI and fetch it in each job that runs games, as [community AI inputs](../migration/community-ai/README.md) do. The fork then redistributes nothing, so GPL corresponding-source work is not needed for it (user, 2026-11-02). Saves may reference such content by ID and checksum.
- Compare semantic state: screenshots alone and compressed-save byte equality cannot establish equivalent simulation. Screenshots and screen recordings attached to a PR can evidence graphical work (user, 2026-10-28). Retain small failures and explain discrepancies before accepting changes. Test success establishes covered behavior, not complete game equivalence.

## Code standards

- `python3 tools/preflight.py --base origin/rust-migration` runs the checks that enforce formatting and lint ([quick cleanup checks](rust-migration.md#quick-cleanup-checks)): C and C++ tab indentation in the commit checker, `cargo fmt` with the other Cargo checks, pinned Ruff and `ty` with annotation checks over `tools/`, and `tools/lint-policy.py` against new ignores and exclusions. Python tools use typed records (`dataclass`, `TypedDict`) for fixed-shape data, not bare dicts.
- Write Rust in the original's vocabulary (user, 2026-11-06). Each original enum, flag set, ID and named constant has one Rust definition in the module named after its original header: a `#[repr(transparent)]` wrapper over the original's underlying integer with associated constants and only the operators the original uses. Code reaches the raw integer at ABI, save and bitfield boundaries and wherever the original uses the value as an integer (table indexing, arithmetic, bit scans); a literal standing for a named value is still a finding. Use a wrapper, never a Rust `enum`, for a C++ enum: C++ enums can hold values outside their list, and a Rust enum holding one is undefined behaviour. Keep element and set types distinct (`DoCommandFlag` versus `DoCommandFlags`). Use the named constant wherever the original names the value.
- Carry the original body's explanatory comments with every ported or rewritten body, at the corresponding branch or operation: game rules, quirks, ordering, assumptions and exceptional cases, adapted to Rust names without losing meaning. Add comments for outside constraints such as MSVC ABI, random-draw order or measurements. Name the original callee or file; cite an issue only for an open gap. Comments describe the code as it is, without review defence, history or dates. State a safety or lifetime contract once per module or type; call sites note only differences.
- A ported command takes one argument struct whose fields follow the original command's parameters in name and order; execution flags and cost or scratch slots travel separately in its call context. Define FFI-crossing argument structs once in Rust with a C-compatible layout and generated declarations; the C++ facade builds one on the stack and passes `const Args*`. Helpers that share state take a context struct instead of a long parameter list.
- A body behind an FFI export lives in a Rust-internal function with the original's types and its argument or context struct. Rust callers call that function, never the export. Where giving the export an FFI argument struct would add generated declarations or C++ facade code that a change may not add, such as in the cleanup stage, the export stays a positional shim that builds the struct (steering 33; #827's waypoint commands).
- Keep unsafe and FFI boundaries narrow; document safety, ownership, lifetimes and panic behavior. Make overflow and integer conversion explicit and faithful. Avoid unrelated C++ warning cleanup and blanket warning changes.
- Group shared append lists by component. Reserve sections for concurrent ports. Sort entries by name: includes after stdafx and before safeguards, independent ABI probe calls, and shared table fields, mirrors, initializers and layouts (one field per line). Alphabetize witness blocks. Keep names and consecutive local indices together in `witness_blocks!`. Global indices and array lengths are derived.

## Tests

- A test fails when its named behavior breaks: run production code against an independent expectation (original, fixture or table). Do not match substrings of source, workflow or documentation text; parse structure when structure is the behavior, as with workflow permissions. Do not count incidental structure or check results at fixed historical commits. `cfg(test)` never changes a production layout or body.
- Judge redundancy by the behavior a test distinguishes, not how directly assertions follow implementation. Serialized-thread state tests must distinguish process-wide from thread-local storage. Prefer one table-driven test to copies.
