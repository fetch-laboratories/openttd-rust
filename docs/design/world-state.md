# Incremental world-state ownership

Design: [#90](https://github.com/fetch-laboratories/openttd-rust/issues/90). Implementation
tasks require [roadmap](../roadmap.md) selection; this note moves no code.

## Constraints in the current game
- `src/map_func.h` exposes mutable tile-field references into C++-owned base and
  extended arrays; `Map::Allocate` replaces both (`src/map.cpp`). Tile indices are
  uint32, and type-dependent fields share bits. Storage cannot move independently
  of these accessors and their outstanding references.
- `src/core/pool_type.hpp` stores pointers indexed by typed IDs. Its companion
  `pool_func.hpp` chooses the first free slot, supports indexed load allocation,
  reuses IDs, optionally caches allocations, and invokes destruction hooks.
  Lifetimes, iteration and allocation order are behavior; an ID is not a
  generation-checked lifetime guarantee.
- `StateGameLoop` orders timers, tiles, vehicles and landscape; `RunTileLoop`
  uses an LFSR order plus special handling for tile zero. Parallelizing or sorting
  those updates would change immediate observations and shared random draws.
- Link graph jobs already copy their graph and settings (`linkgraphjob.h`). TGP's
  height map is temporary generator state. These are useful ownership boundaries;
  a general live-world snapshot would introduce different observation semantics.

## Options and decision
1. Mirror and reconcile each tick: two authorities, copying and delayed mutations
   make fidelity fragile.
2. Transfer map/pools together: requires changing tile references, polymorphic
   objects, save adapters and many callers.
3. Move component owners now; keep one shared-world store behind narrow operations
   until a selected port can transfer it. **Choose this.**

Rust owns component-private state and complete control flow. C++ initially owns
map arrays, live pools and shared services. Its facade supplies operations on tile
indices and typed pool IDs; map fields are read in place through the shared map
view, and pool records move to shared layouts through storage ports. It retains
neither the migrated algorithm nor a second authoritative copy.
Original bodies remain compiled only without `WITH_RUST`. #73 owns the temporary
height map; #74 owns its computation snapshot, with scheduling/save/load/join in
C++. Neither requires moving the global map or station pool.

For a live component, invoke shared operations at the original observation and
mutation points. Preserve partial effects, traversal order, ID widths/sentinels,
integer narrowing and random draws. Do not add lifetime checks that reject source
behavior; design short access scopes that uphold the original preconditions.

## Boundary safety and cost
Use explicit scalar FFI and opaque Rust owners with one destruction path. A C++
object pointer may be an opaque context, never a Rust view of an STL/pool object.
No Rust reference into world storage survives an operation that can mutate,
reallocate or destroy it. Resolve IDs when needed under the source's lifetime
rules; reacquiring an ID alone does not make reuse safe. Document callback reentry
for each component and release affected owner borrows before it can reenter.
Shared services (`Random`, pool operations, and the accessors of records not yet
moved) are `noexcept` C++ wrappers that Rust calls directly, so ports keep the
original control flow. Environmental
failures (allocation, debug/log I/O, developer-only defines such as `RANDOM_DEBUG`)
terminate inside the wrapper; they are not simulation behavior. Script VMs and
save/load errors use a return-to-C++ action protocol where an ordinary-play C++
exception can unwind. Reentry or mutation alone does not require that protocol:
end every affected Rust borrow before calling a typed service directly.
Neither C++ exceptions nor Rust panics unwind
across FFI; for ordinary-play failures preserve source failure order.

#213's selected post-tick experiment returns one bundle of named native scalar
addresses. C++ keeps vehicle shells and allocation; Rust forms no Vehicle,
container or bitset layout view or reference. A read-only `BaseBitSet::base_address`
returns the actual `const Tstorage *` member; one canonical typed vehicle service
exposes front/current status addresses. No bitset layout cast is permitted. Native
tests check actual types, address identity, source-order reads and permitted writes.
Addresses are acquired after Tick survives, used only at original branches under
the game-state mutex and discarded before another Tick or fleet drain. All raw
accesses end before callbacks. Audited cargo/sound paths may reenter but cannot
destroy these shells, reset pools or yield the mutex.

A call-scoped pool descriptor exposes the actual slot array, its initialized
`data.size()` and the address of canonical `first_unused`, not a vector-object
layout or shadow state. Rust reads that scalar after potentially allocating
callbacks before accessing any cached slot. A value beyond captured size requires
a fresh descriptor: every reallocation raises `first_unused` beyond the old size.
Native tests compare live growth, deletion, holes and ID reuse to PoolIterator.
No selected callback can reset the pool; descriptors end before load/reset or fleet
drain. Tick is a typed native facade retaining controller timing; Rust owns the
surrounding scheduler and post-tick policy. This contract authorizes no other raw
field access. Measured total crossings, bytes and CPU/wall gates decide acceptance.

#325 selects aircraft movement observations as a separate narrow contract.
One call-scoped typed carrier holds addresses of canonical native x/y/z, speed,
direction and subtype scalars used by the controller; standalone position needs
only subtype and z. This exposes no Vehicle layout, offsets or persistent mirror.
Rust uses short raw scalar loads at the original branches, including after native
position, rotor and viewport mutations; no Rust reference or cached scalar crosses
a callback. Writes and virtual operations retain their canonical services.
Acquire access after loading/order processing establishes a live shell, excluding
the deleting crash path. Discard it before any callback that may destroy or replace
that shell; ID reuse is not a lifetime repair. The shared vehicle layer defines
each typed address once. Default native tests prove address identity and fresh
reads after mutation, while existing reentry witnesses and semantic checks remain.
This selects no shared Vehicle storage migration, chain view or general raw API.

#377 transfers only the canonical tile bucket array and intrusive tile links.
The stable native shell embeds one named FFI link carrier; its typed accessor
exposes that carrier, never a Vehicle layout or copied vehicle record. Rust uses
raw field-sized reads/writes under the game-state mutex. Bucket/link addresses
remain stable until shell destruction or index reset. An iterator retains only
its shell cursor and local bounds, and reads the live next link after callbacks.
The current shell survives advancement as required by the original iterators;
crash/flood callbacks may mutate links but do not delete that current shell.
No link borrow spans reentry. Reset ends all cursors before load/pool reset and
clears only current-bucket pointers and buckets, retaining source stale links.

#275's normal-update observation resolves one shell for its link carrier and
tile; removal still acquires links alone. Near filtering observes x/y and the
carrier together, reads next only on rejection, and reacquires live links after
visitors. No field address beyond #377's links is exposed. Aircraft and road
reuse existing call-local map dimensions: their synchronous controller callbacks
(orders, GRF/sound, movement, crash, viewport) cannot reset/load the map or yield
the game-state mutex. Nested entries own fresh metadata; train/ship and other
entries still read dimensions per update. Road movement observations also
acquire the canonical base/extended tile descriptor lazily once per entry. Only
the track-status tile type and normal-road bits/status/road-type reads use it;
fields are reloaded after native mutations. Partial fixture tables retain the
typed backend. Pixel heights, aircraft, standalone movement, train, ship, YAPF and
water-region entries are still callback-backed; #565 moves them to the shared map
view, hottest first. Descriptor metadata shares the controller's
no-reset/no-load/no-yield interval above.

Data crosses the boundary in place; operations cross as calls. A call per field,
or a reader written for one call site, makes boundary cost grow with the work
done and the interface grow with call sites: the 2026-10-28 P2016 profile spent
349M glue calls, 79 per vehicle tick, and they were 99% of the gap to the
original (#275). Map fields are read through the shared map view below; pool
records move to shared layouts through storage ports ("Transferring shared
storage"). Until a record's storage port, entries use that record's shared typed
accessors. Do not batch RNG or defer writes across callbacks, and keep source
computation out of callback bodies.

## Map access decision (#108, revised 2026-10-28)

C++ keeps the canonical map allocation, its setters and save adapters until the
map storage port. Rust simulation code reads and writes map fields in place
through the shared map view, at every entry, instead of typed per-field or
per-predicate services. The 2026-10-04 decision kept typed services; the profile
above showed their cost, and the view's interval holds for the whole game loop.

The view retains one canonical allocation, pins both C++ and Rust
size/alignment/offsets, uses field-sized raw `ptr::read`/`write` with no Rust
references/slices over shared arrays, and refreshes pointers at each component
entry. End its access scope before callbacks that may load, reset or replace the
map, or yield the game-state mutex. C++ field references may alias those
pointers; absence of Rust references does not permit concurrent conflicting
access. Preserve original observation/write order rather than copying a whole
tile back after a callback.

`Map::Allocate` covers new-game, modern and legacy load, and tests. Existing
field-wise save adapters remain unchanged; serialization reads live fields before
the asynchronous compression thread starts. Link-graph workers use private graph
copies and map dimensions, not tile arrays; retain the existing job join/reset
ordering. Portable builds keep the C++ owner.

### Shared map view (#274, extended by #565)

The view is the tile-type allocation/access descriptor with canonical
base/extended pointers and count. Its first consumers were the scheduler-dispatched
clear, water and tree tile loops, including shore-water reentry, and road
track-status reads (#523); #565 extends it to every Rust simulation entry.
Storage, allocation, setters, save adapters and worker ordering stay in C++.

Acquire at an entry under the game-state mutex. Use field-sized raw loads, no
Rust references/slices, and reload after mutations at the original observation
points. The interval is proved once for the game loop and command execution
(below), not per entry; descriptor metadata ends before any callback that can
load, reset or replace the map or yield, such as generation progress. Same-thread
reentry keeps the descriptor only within that interval.

Define each source helper once in its header-named Rust module. Generated named
ABI probes pin both layouts; preserve native helper assertions under the native
assertion policy captured once per entry. Assertion-disabled builds gain no new
panics or rejected inputs. Delete displaced reads and orphan bindings; remaining
unaudited callers share the canonical native fallback.

Every entry relies on this interval. `Map::Allocate` runs only from game reset
(`InitializeGame`), the MAPS and legacy loaders and tests, and the mutex yield
(`GameLoopPause`) only in generation progress and GRF scanning, all outside the
game loop and command execution. A change that adds a reset or yield site inside
them re-proves the interval. In the tile scheduler (`tile_loop.rs`), tile handlers may mutate tiles and pools,
run landscape, road and crash commands or queue script events (`InsertEvent`),
but never reset, load, report progress or run a script VM, and none changes map
dimensions, climate, tree placement, the tick counter, editor mode or the
road-rebuild and ambient settings; `_misc_grf_features` changes only on reset and
GRF `ActionD` loading. So a batch reads those settings once and keeps only
descriptor metadata across dispatch, ending each field-sized read first. The
ambient feature gate is one of those batch reads; ambient callbacks keep their
original RNG position, and the tree loop reads each field after them, where the
source reads it.
Threaded and arbitrary NewGRF corpus limits are open in #156.

Each converted entry keeps its semantic scenarios; default native checks cover
mutation and reallocation. The view is map access only, not a storage transfer.

## Transferring shared storage
Storage ports are selected work (user direction, 2026-10-28): the rewrite moves
every pool and finally the map arrays into Rust, so their object classes and
accessors retire like any other original code. Order them by the code they
retire, the crossings they remove and the glue they delete: per-tick vehicle
records first (#563), then the other pools, then the map's definitions and
allocation. Move a port's canonical owner, accessors, allocation/reset and
destruction together.

A moved record is defined once, in Rust, with a C-compatible layout. Its C++
declaration is generated from that definition (extending `layouts!` and
`tools/generate-abi.py`) and its layout is checked on every CI compiler, so
remaining C++ callers (GUI, drawing, save adapters, scripts, NewGRF callbacks)
keep inline accessors that read and write the same memory. No exported call per
field, and no synchronized second copy. Operations with behavior (allocation,
deletion, iteration with hooks, virtual dispatch) cross as calls. Each port
deletes the per-field services and reader carriers its record replaces. A
polymorphic C++ shell (vtable, STL members) stays opaque to Rust and owns only
the fields not yet migrated; the shared record sits in it or beside it, as each
design states. C++ tile references point into the one map allocation, so the
map's storage port moves that allocation without replacing them.

Map fields keep their encodings. Pools keep IDs, first-free selection, iteration
and hook order; changing to a compacting arena or generational IDs is not a
compatible shortcut. Drawing reads these records every frame and no current
check times it; #566 measures it.

## Saves and evidence
Keep chunk IDs, versions, field widths and reference numbering unchanged.
`src/saveload/map_sl.cpp` serializes tile fields rather than struct bytes;
`saveload.cpp` maps current object references to index + 1 (zero is null), with
legacy load rules and pointer fixups. Keep those C++ adapters initially; when an
owner moves, adapt its reads/writes and load lifecycle without changing the format.
Rust-owned persistent state must be visible to saving before the call returns.

Extend the component module under `tools/simulation/` for save/load and
mutation/reuse paths. Run `python3 tools/migration.py simulate` with default scenarios.
Keep desync snapshots and plain runs: cache rebuilding can conceal
faults. The existing semantic decoder remains unchanged. Port divergences belong in `KNOWN_FAILURES` with an issue, never new
`MASKS`; #83's masked `round_trip_time` needs separate evidence if touched. Saves
do not expose every transient state: add a narrow direct check only for a named
unreachable gap. Vehicle ports need applicable #86/#104 evidence; the aircraft fixture is now
integrated in #132. Each controller still needs its own branch witnesses. Retain
existing tests.

## Cargo registry views (#467)

One process-wide Rust owner holds cargo label/default/climate tables, masks, sorted/reverse
order and town-production lists. CargoSpec and per-file GRF storage stay native. Native spans
alias Rust buffers until the next rebuild; the language-loader sort may precede climate setup.
Clears and populated views are published before native collation, and each town-list insertion
is visible before the special-cargo break. Rebuild/reset runs under serialized loading, outside
callbacks. Runtime lookups retain no container borrow across native services; each cargo
callback has its own Rust invocation and native virtual/profiler frame, and common Action2 and
storage are direct Rust callees. This adds no map access permission to #274.

## World-generation lifecycle (#546)

Generation state has one process-wide Rust allocation with static lifetime. C++ receives
only the address of its bool subobject for `_generating_world`: both languages use the
one-byte, one-aligned bool representation, and all writes store false or true. The address
never changes or crosses reset allocation; Rust accesses fields through raw field-sized
operations, never a reference that could exclude native aliases. Startup precedes the game
thread, and subsequent accesses use the game-state mutex on both GUI threads.

Native CompanyID backup and landscape/town/industry/object frames remain native objects;
Rust receives opaque typed pointers and consumed scalar metadata, without a layout view of
StrongType, STL or pool objects. The backup retains its native destructor/restoration
contract. Native wrappers catch progress abortion, script and completion callbacks and
save/load errors before returning across FFI. Resource frames outlive those Rust entries,
and Rust chooses source cleanup, storage rollback and company restoration before native
mode switching can reenter generation. No owner borrow spans those calls. Scheduler map
descriptors live only inside each existing audited loop interval and expire before the
next progress update. This adds no map-view permission.
