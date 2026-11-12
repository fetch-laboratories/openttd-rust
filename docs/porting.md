# Porting

## Terms

| Term | Meaning |
| --- | --- |
| Owner | Rust state that holds a component's data and runs its control flow. It may be one allocation per vehicle or a process-wide static. |
| Shell | A C++ object that holds a Rust owner and keeps its pool identity, fields not yet moved to Rust and save aliases. It is usually a pool item such as `Vehicle`, `Station` or `Town`. |
| Leaf | A typed `noexcept` C++ service that Rust calls for a shared operation (`Random`, sound, viewport) or a field of a record still stored in C++. A leaf returns without reentering its calling owner. |

## Ownership ports

- Prefer ports where Rust owns state and control flow, and C++ keeps a thin facade.
- Do not extract an algorithmic fragment while C++ keeps its state and loop unless the roadmap names it as a stepping stone.

## Storage ports

- A pool's storage is moved when Rust owns its slot array, occupancy and allocation bookkeeping, allocation/free/iteration/reset policy and lifetime ordering, and no Rust code reaches a record field through a C++ service.
- Move every field Rust reads or writes, across one or more owners. A field stays in its C++ shell only while no Rust code touches it, and moves with the owner of its last C++ consumer. The map owner likewise takes array allocation/reset and the remaining map bodies. A shared-record checkpoint alone completes neither pool nor map ownership.
- Define each moved record once in Rust with a C-compatible layout; generate its C++ declaration and check the layout at build. Both languages access fields in place.
- Unported C++ uses inline accessors over the shared layout, not a call per field. Only operations such as allocation, deletion, iteration with hooks and virtual behavior cross as calls.
- Move a field and delete its Rust-called services in the same change, before pool adoption.

## Fidelity

- The pinned original is the specification, quirks included ([ADR 0002](adr/0002-original-game-is-the-specification.md)). Reproduce it over its whole reachable input domain, beyond the paths scenarios exercise. Track known existing divergences as issues.
- Add no guards, assertions, rejections or fatal paths absent from the original, and drop none it has: `tile + offset` is not `TileAdd`.
- Native fatal paths keep the native error box and crash reason, which a Rust `panic!` or `unreachable!` loses. A `switch` with `NOT_REACHED` stays a match whose default arm takes the native unreachable path.
- Operators and control flow follow the original's. Where C++ wraps or truncates, use `wrapping_*` operations or `as` casts. Release overflow checks and `panic = "abort"` may only back up operations that cannot overflow in the original.
- Keep the exact sequence of random draws, with their width, and every state read and write at its original observation point.
- A C++ global or static becomes a process-wide Rust static, not `thread_local!`: GUI input runs on the main thread and the game loop on `ottd:game`, and the single-threaded `-vnull` harness cannot show the difference.
- Preserve lifetimes: allocation and ID reuse, publication and destruction order. End each borrow before a callback, mutation, container growth, throw or deletion that can invalidate it.

## Boundary

- Call shared services and field accessors for records still stored in C++ directly through `noexcept` C++ wrappers. Preserve the original's decisions and the order of its observable effects, including partial effects on early return; its function boundaries are not part of that. An escaping exception terminates.
- Do not batch RNG or defer writes across callbacks. Keep source computation out of callback bodies.
- Allocation failure, debug or log I/O errors, and paths needing developer-only defines such as `RANDOM_DEBUG` are environmental failures, not simulation behavior.
- Use an action protocol to return control to C++ only where an exception can unwind through the call in ordinary play (script VMs, save/load errors). Reentry alone is not a reason: end every Rust borrow before the callback, then call directly.
- Entries are synchronous calls, without async, futures, tasks or mailboxes, and allocate nothing per call.
- Give each service its own typed `noexcept` function with real types on both sides. Use designated initializers for service tables.
- Access map and shared-layout record fields by field-sized raw load or store at the original observation point. For a record still stored in C++, resolve its owner once per entry.
- Internal signatures use the original's wrapper types. FFI exports, `repr(C)` records, service tables and generated declarations pass C++ class types such as `PoolID` and `StrongType` as their base integers, because MSVC returns class types through a hidden pointer.
- A Rust-defined `extern "C"` function or callback returns a 0-or-1 `u8`, not `bool`, and C++ compares it `!= 0`.

### Design faults

Fix these before integration; root grants no exceptions. A file pass fixes every one in the files it finishes and cannot leave one as residue.

- A reader or carrier written for one call site, including a service per `Debug` or `GrfMsg` message.
- A Rust-to-C++-to-Rust bounce into code Rust owns.
- A moved field behind a service, or a Rust-called service routed through `Record()`.
- Allocation on a hot path or per call where the original has none, including a process-wide scratch pool (`GameState<Vec<...>>`) where the original builds a local container.
- Opcode dispatch, such as a `mode` parameter choosing among operations, or whole-record copies.
- An owner resolved once per field instead of once per entry.

## Callees and shared accessors

- Call the original helper, virtual method or named constant via its shared accessor or Rust port, not a hand-written equivalent: a raw store for a setter, a hand-written bound for `TileAddWrap` or a literal for `INVALID_RAILTYPE` diverges when the callee changes.
- Port each helper or constant once into the shared Rust module named after its original file, header or `.cpp`, and import it from there. A pass adds a missing port there; that does not make the pass own that file.
- Before adding a binding, search `src/` for its callee, including macros and compound callbacks such as `CheckOwnership` and `GetRoadOwner`.
- Each shared accessor (vehicle field, map predicate, setting or constant) has one typed definition; command ports share one set of command-check services. A PR adding one deletes the integrated copies.
- Delete the C++ a change replaces in the same change, with its exports, declarations and service fields.
- A typed accessor replaces its raw form. A raw `_raw` adapter delegates to the typed definition and serves only callers in files no pass has finished, until its last caller goes or #734 closes. A pass converts callers in already-finished files itself. Rust code calls the typed form, not a raw adapter or an `openttd_rust_*` export.

## Map access

- Simulation Rust reads and writes map arrays in place through the [shared map descriptor](design/world-state.md); `map_func::World` is its single type.
- Prove the descriptor's interval once, not per entry: `Map::Allocate` in new games and loaders, and game-state mutex yields for generation progress and GRF scanning, run outside the game loop and command execution. Re-prove it if a change adds a reset or yield there. Raw field loads use no Rust references and end before a callback that can load, reset or yield.
- Shared header helpers preserve native assertions: on a failed precondition call the native assertion as the original accessor does. Retire existing typed map services hottest first (#565), otherwise when a port touches them.

## Evidence

- Game-logic ports require the semantic simulation harness (`python3 tools/migration.py simulate`) and existing tests. Integrate a new port only after harness scenarios exercise its component; scenarios arrive with the port that needs them. Fidelity fixes to existing ports do not wait for that.
- Repair a fixture's input, not its expected result, and explain every changed fixture or expectation in the PR.
- Each new scenario, fixture, test or checker names the behavior existing checks cannot distinguish; one that names none, or compares an operation with itself, is deleted (provisional). Do not trace pure helpers solely to reproduce the source call sequence.
- Any check cited by a doc or PR, such as a branch witness or reference probe, runs in default CI and fails when missing; an opt-in profile or `__main__`-only probe is not evidence.
- Compare semantic state: screenshots and compressed-save byte equality cannot establish equivalent simulation, though screenshots and recordings can evidence graphical work.
- When a PR introduces or uncovers a branch no scenario reaches, edit #156's current list.
- Extend harness scenarios instead of writing a comparison tool per component, in the component's scenario module, using `tools/probe.py` for native gap probes and `tools/simulation/core.py` and `runtime.py` for games and builds. Keep tooling proportionate to the C++ the port retires.
- Third-party test content such as BaNaNaS AIs and NewGRF sets stays out of Git and every CI artifact ([ADR 0015](adr/0015-third-party-test-content.md)): pin each package by name, version and SHA-256 in a manifest and fetch it from the BaNaNaS CDN into ignored `.local/` storage, as [community AI inputs](../migration/community-ai/README.md) do.

### Tests in Rust and C++

- New tests are Rust tests or harness scenarios, not C++ ([ADR 0001](adr/0001-full-rust-rewrite.md)). The one exception: an observation of the C++ calling boundary itself that Rust tests and the harness cannot make may extend a kept native test, named in the PR with the boundary whose retirement removes it.
- A C++ test of code still written in C++, including a live native binding, pool or production caller, stays in C++ until that code is ported ([ADR 0023](adr/0023-cpp-tests-stay-until-code-is-ported.md)), and retires inside that port.
- Move a C++ test to Rust or the harness only where they reach what it observed, and only where the counted C++ it deletes clearly exceeds the counted C++ its oracle recipe adds ([ADR 0004](adr/0004-cleanup-before-porting.md)); its first checkpoint reports both. Carry each observation with expectations taken from the original.
- A PR that changes an oracle, its capture recipe or the pin reruns `tools/oracle-capture.py` for the oracles it touches and reports the result; its reviewer repeats the run.

### Cleanup proof

- A type, name or comment change proves it changes no behavior: identical compiled Rust code, or a source review showing each changed function performs the same operations in the same order, plus the merge queue's Full. Per-PR Callgrind profiles and disassembly accounting are not required.
- Skip the archive-equivalence attempt for a pass that changes signatures or adds argument structs: it cannot pass.

## Code standards

- A file pass applies these standards to every line left in its files; every other PR to every line it changes or adds. Later PRs touch a finished file only for a mechanical shared-file edit or for a caller adopting a changed signature.
- Python tools use typed records (`dataclass`, `TypedDict`) for fixed-shape data, not bare dicts.
- Group shared append lists by component and sort entries by name: includes after stdafx and before safeguards, ABI probe calls, and shared table fields, mirrors, initializers and layouts (one field per line). Alphabetize witness blocks.

### Original vocabulary

- Each original enum, flag set, ID and named constant has one Rust definition in the module named after its original header (`track_type.rs` for `track_type.h`): a `#[repr(transparent)]` wrapper over the original's underlying integer with associated constants and only the operators the original uses, not a Rust `enum` ([ADR 0021](adr/0021-integer-wrappers-not-rust-enums.md)).
- Use the named constant wherever the original names the value. Literals the original writes as literals stay.
- Reach the raw integer through the wrapper's accessors, only at ABI, save and bitfield boundaries and where the original uses the value as an integer.
- Keep element and set types distinct (`DoCommandFlag` versus `DoCommandFlags`). An internal signature takes `DoCommandFlags` where the original does, not a projected flag such as `execute: bool`.

### Comments

- Carry the original body's explanatory comments with every ported or rewritten body, at the corresponding branch or operation, adapted to Rust names.
- Comment outside constraints such as MSVC ABI or random-draw order. Cite an issue only for an open gap.
- Comments describe the code as it is, without review defence, history or dates. State a safety or lifetime contract once per module or type.

### Argument structs

- A ported command takes one argument struct whose fields follow the original command's parameters in name and order; execution flags and cost or scratch slots travel separately in its call context. Define FFI-crossing argument structs once in Rust with generated declarations; the C++ facade passes `const Args*`.
- Helpers that share state take a context struct. A non-command helper over seven parameters is a finding.
- A body behind an FFI export lives in a Rust-internal function with the original's types and its argument or context struct, which Rust callers call. Where an FFI argument struct would add C++ a change may not add, the export keeps its positional base-integer list as a shim that builds the struct.

## Tests

- A test fails when its named behavior breaks: run production code against an independent expectation (original, fixture or table). Do not match substrings of source, workflow or documentation text, count incidental structure or check results at fixed historical commits.
- `cfg(test)` does not change a production layout or body. A test observes the production path through a recording mock of its service, not a trap compiled into the body.
- A test of serialized-thread state distinguishes process-wide from thread-local storage.
