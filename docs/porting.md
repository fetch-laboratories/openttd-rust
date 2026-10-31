# Porting

For agents writing or reviewing port code, tests and Python tools.

## Terms

| Term | Meaning |
| --- | --- |
| Owner | Rust state that holds a component's data and runs its control flow. It may be one allocation per vehicle or a process-wide static. |
| Shell | A C++ object that holds a Rust owner and keeps its pool identity, fields not yet moved to Rust and save aliases. It is usually a pool item such as `Vehicle`, `Station` or `Town`. |
| Canonical state | The single authoritative copy of a datum, in Rust or C++. Canonical native state stays in C++, as with the map arrays. Read and write it in place without a mirror. |
| Facade | A C++ function or method that keeps the original signature and forwards to Rust under `WITH_RUST`. |
| Leaf | A typed `noexcept` C++ service that Rust calls for a shared operation or a field of a record still stored in C++. Shared operations include `Random`, sound and viewport. A leaf returns without reentering its calling owner. |
| Service table | A C++ struct of typed leaf function pointers, such as `OpenTTDTreeServices`. C++ uses designated initializers; Rust borrows the table for a call. Each field names one service, never an opcode. |
| Witness | A named branch counter in `witness.rs`. A scenario that sets `witnesses` requires each listed counter to be nonzero. |

## Ownership ports

- Prefer ports where Rust owns state and control flow. C++ keeps a thin facade and calls for shared operations (`Random`, allocation, progress, sound) and fields of records still stored in C++.
- The Rust build does not compile the original body. Until #581, keep it in the `#else` of `#ifdef WITH_RUST` or under `#ifndef WITH_RUST`. The #581 sweep deletes these bodies. Later ports delete the bodies they replace. The pinned reference remains for review.
- Do not extract an algorithmic fragment while C++ keeps its state and loop unless the roadmap names it as a stepping stone.

## Storage ports

- The rewrite retires object classes and map accessors in headers (user, 2026-10-28). Once control flow is in Rust, move records to Rust: pool objects (vehicles first, #563; then stations, orders, towns, industries, companies and the rest), then map arrays.
- Define each moved record once in Rust with a C-compatible layout. Generate its C++ declaration and check the layout at build. `tools/port-metrics.py` counts the declaration as compiled C++ glue. Both languages access fields in place.
- Unported C++ drawing, GUI, save/load, scripts and network use inline accessors over the shared layout, never a call per field. Only operations such as allocation, deletion, iteration with hooks and virtual behavior cross as calls. Delete replaced per-field services and reader carriers.

## Fidelity

- Reproduce the original over its whole reachable input domain; add no guards, assertions, rejections or fatal paths absent from it. Where C++ wraps or truncates, use `wrapping_*` operations or `as` casts. Release overflow checks and `panic = "abort"` may only back up operations that cannot overflow in the original. This applies to new work; track known existing divergences as issues.
- Preserve state scope: a C++ global or static becomes a process-wide Rust static, never `thread_local!`. GUI input and single-player commands run on the main thread, the game loop on `ottd:game`, serialized by the game-state mutex; the single-threaded `-vnull` harness cannot reveal the difference.

- Preserve the exact sequence of random draws; see [every agent](../AGENTS.md#every-agent).

## Boundary

- Call `Random`, pool operations, other shared services and field accessors for records still stored in C++ directly through `noexcept` C++ wrappers. Preserve the original control flow. An escaping exception terminates.
- Do not batch RNG or defer writes across callbacks. Keep source computation out of callback bodies.
- Allocation failure, debug or log I/O errors, and paths needing developer-only defines such as `RANDOM_DEBUG` are environmental failures, not simulation behavior.
- Use an action protocol to return control to C++ only where an exception can unwind through the call in ordinary play (script VMs, save/load errors). Name each such service. Reentry alone is not a reason: end every Rust borrow before a callback that can reenter Rust or mutate its state, then call directly.
- Use synchronous calls for entries. Do not use async, futures, tasks or mailboxes. Crossings allocate nothing per call. Where the original builds a local container, use a local `Vec`, not process-wide unsafe scratch.
- Give each service its own typed `noexcept` function, not an opcode switch or positional array. Return real types on both sides. Use designated initializers for tables.
- Access map and shared-layout record fields by field-sized raw load or store at the original observation point. For a record still stored in C++, resolve its owner once per entry. Use its shared typed field accessors.
- The following are design faults. Fix them before integration regardless of timing. Root grants no exceptions.
  - A reader or carrier written for one call site.
  - Allocation on a hot path or per call where the original has none.
  - Opcode dispatch or whole-record copies.
  - A Rust-to-C++-to-Rust bounce into code Rust owns.
- Pass C++ class types such as `PoolID` and `StrongType` as their base types: MSVC returns class types through a hidden pointer (#355). Existing ports convert under #168 and #199. Once #500's first PR adds shared `repr(transparent)` ID newtypes and opaque pool-pointer marker types in header modules, new ports adopt them from the start; convert existing modules when a maintenance batch touches them, never in a whole-fork sweep while drafts are open.

## Callees and shared accessors

- Call the original helper, virtual method or named constant via its shared accessor or Rust port, not a hand-written equivalent. A raw store for a setter, a hand-written bound for `TileAddWrap`, or a literal for `INVALID_RAILTYPE` can diverge when the callee changes.
- Port each inline helper or constant from an original header once into the shared Rust module named after that header; every port imports it there. Examples: `slope_func.h`, `map_func.h`, `tunnelbridge.h`, command flags and price indices.
- Each shared accessor (vehicle field, map predicate, setting or constant) has one typed definition for every port; command ports share one set of command-check services. `tools/shared-accessors.py` rejects new copies in quick CI (#249). `tools/rebound-callees.py` rejects a callee directly bound into a second service table or a field forwarded from another table within its documented syntactic scope.
- Neither checker sees macros or compound callbacks; reviewers grep those callees by hand ([review](workflow.md#review)).
- A PR adding a shared accessor deletes integrated copies; one removing a caller deletes its orphaned export, declaration or callback field. Tests and harnesses do not keep a `WITH_RUST` body or `openttd_rust_*` export alive: move the fixture onto the Rust entry and delete the body.

## Map access

- Simulation Rust reads and writes map arrays in place through the [shared map descriptor](design/world-state.md). Load it rather than call for it; `map_func::World` is its single type. Do not use a service per field or predicate.
- Prove the descriptor's interval once, not per entry: `Map::Allocate` in new games and loaders, and game-state mutex yields for generation progress and GRF scanning, run outside the game loop and command execution. Re-prove it if a change adds a reset or yield there. Raw field loads use no Rust references and end before a callback that can load, reset or yield.
- Shared header helpers preserve native assertions: on a failed precondition call the native assertion as the original accessor does, never the C++ accessor; each conversion deletes its replaced service. C++ keeps canonical map storage and accessors until the map storage port. Retire existing typed map services hottest first (#565), otherwise when a port touches them.

## Evidence

- Game-logic ports require the semantic simulation harness (`python3 tools/migration.py simulate`, #72) and existing tests. Integrate a new port only after harness scenarios exercise its component. Fidelity fixes to existing ports, such as #75, do not wait for that. Phases 3 to 6 reuse the harness where it observes output, such as saves, script commands and multiplayer sessions.
- Extend harness scenarios instead of writing a comparison tool for each component. Put harness tooling for a port in its component's scenario module. Tooling is code to maintain: keep it proportionate to the C++ the port retires. Compare against unchanged reference bodies only for a concrete gap the harness cannot reach, named in the PR.
- Any check cited by a doc or PR, such as a branch witness or reference probe, runs in default CI and fails when missing; an opt-in profile or `__main__`-only probe is not evidence.
- Compare semantic state: screenshots alone and compressed-save byte equality cannot establish equivalent simulation. Screenshots and screen recordings attached to a PR can evidence graphical work (user, 2026-10-28). Retain small failures and explain discrepancies before accepting changes. Test success establishes covered behavior, not complete game equivalence.

- Keep the reference worktree unchanged. Never change expected results together with candidate behavior; see [every agent](../AGENTS.md#every-agent).

## Code standards

- Use tabs in C and C++; use `cargo fmt` in Rust. Run the [Cargo checks](rust-migration.md#build-and-verification) for Rust changes. The migration driver and CI enforce them.
- Python tools pass `uvx --from ruff==0.16.8 ruff check tools/`, `uvx --from ruff==0.16.8 ruff format --check tools/` and `uvx ty@0.0.92 check`. Ruff ANN checks new and converted files; explicit per-file ignores only shrink. Add every new or converted module to `ty.toml`'s include list. Annotate every parameter and return of new or rewritten Python functions; use typed records (`dataclass`, `TypedDict`) for fixed-shape data, not bare dicts. Add hints to existing code incrementally (#501).
- Comments state what code cannot: original quirks, outside constraints such as MSVC ABI or random-draw order, or measurements, in one line where possible. Name the original callee or file; cite an issue only for an open gap. Exclude review defense, history and process words (such as "not a second X", "no longer", "reviewed" and dates). State a safety or lifetime contract once per module or type; call sites note only differences.
- Keep unsafe and FFI boundaries narrow; document safety, ownership, lifetimes and panic behavior. Make overflow and integer conversion explicit and faithful. Avoid unrelated C++ warning cleanup and blanket warning changes.
- Group shared append lists by component. Reserve sections for concurrent ports. Sort entries by name: includes after stdafx and before safeguards, independent ABI probe calls, and shared table fields, mirrors, initializers and layouts (one field per line). Alphabetize witness blocks. Keep names and consecutive local indices together in `witness_blocks!`. Global indices and array lengths are derived.

## Tests

- A test fails when its named behavior breaks: run production code against an independent expectation (original, fixture or table). Do not match substrings of source, workflow or documentation text; parse structure when structure is the behavior, as with workflow permissions. Do not count incidental structure or check results at fixed historical commits. `cfg(test)` never changes a production layout or body.
- Judge redundancy by the behavior a test distinguishes, not how directly assertions follow implementation. Serialized-thread state tests must distinguish process-wide from thread-local storage. Prefer one table-driven test to copies.
