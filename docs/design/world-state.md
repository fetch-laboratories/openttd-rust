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
indices and typed pool IDs, with copied scalar records where useful; it retains
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
Shared services (`Random`, map and pool accessors) are `noexcept` C++ wrappers
that Rust calls directly, so ports keep the original control flow. Environmental
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
entries still read dimensions per update. No map allocation descriptor is added.

Per-field callbacks inside tile or vehicle loops may dominate runtime. Prefer
copied records for fields observed together and batches only where the original
has no intervening observable operation. Do not batch RNG or defer writes across
callbacks. A later bounded raw map view needs a proved exclusive access interval
and explicit layout/lifetime rules; it is not the default interface. Measure
crossings per update, copied bytes and elapsed time on the same harness scenarios
before enlarging the boundary. Keep source computation out of callback bodies.

## Map access decision (#108, accepted 2026-10-04)

Keep canonical map arrays in C++ and use direct `noexcept` operations for the
next selected tile-heavy owners. Prefer existing semantic operations and copied
records over one callback per bitfield; trees already copy ten words per read,
and water-region traversal calls track/follower services rather than individual
map fields. C++ storage ownership remains selected. New owners use integrated
typed services; no transfer is selected.

A future raw view must retain one canonical allocation, pin both C++ and Rust
size/alignment/offsets, use field-sized raw `ptr::read`/`write` with no Rust
references/slices over shared arrays, and refresh pointers at each component
entry. End its access scope before callbacks that may reenter, replace the map,
or run arbitrary code. Prove a serial access interval excluding concurrent map reset or mutation.
C++ field references may alias those pointers; absence of Rust references does
not permit concurrent conflicting access. Preserve original
observation/write order rather than copying a whole tile back after a callback.

Moving allocation to Rust adds reset, destruction, allocator and C++ object-
lifetime obligations without itself removing calls or aliases. Do not couple
that move to the next simulation port. `Map::Allocate` covers new-game, modern
and legacy load, and tests. Existing field-wise save adapters remain unchanged;
serialization reads live fields before the asynchronous compression thread starts.
Link-graph workers use private graph copies and map dimensions, not tile arrays;
retain the existing job join/reset ordering. Portable builds keep the C++ owner.

### Selected shared map view (#274, 2026-10-21)

The selected maintenance PR extends the existing tile-type allocation/access
descriptor with canonical base/extended pointers and count. Its first consumers
are scheduler-dispatched clear, water and tree tile loops, including shore-water
reentry. Other entries retain typed native services until separately audited.
Storage, allocation, setters, save adapters and worker ordering stay in C++.

Acquire once at an audited entry under the game-state mutex. Use field-sized raw
loads, no Rust references/slices, and reload after mutations at the original
observation points. Descriptor metadata may survive only callbacks proved unable
to reset/replace the map or yield; end its lifetime before any unproved callback.
Same-thread reentry needs this proof even while the mutex remains held.

Define each source helper once in its header-named Rust module. Generated named
ABI probes pin both layouts; preserve native helper assertions under the native
assertion policy captured once per entry. Assertion-disabled builds gain no new
panics or rejected inputs. Delete displaced reads and orphan bindings; remaining
unaudited callers share the canonical native fallback.

Extending the view requires default mutation/reallocation checks and semantic
comparisons. This selects a bounded candidate, not a storage transfer or a
general raw API.

## Transferring shared storage later
Root selects a storage port when measured coupling warrants it. Move its canonical
owner, accessors, allocation/reset and destruction together;
retain C++ facades for remaining callers. Map fields keep their encodings. Pools
keep IDs, first-free selection, iteration and hook order; changing to a compacting
arena or generational IDs is not a compatible shortcut. Any retained C++ object
shell owns only the fields not yet migrated, never a synchronized second copy.

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
