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
ordering.

### Shared map view (#274, extended by #565)

The view is the tile-type allocation/access descriptor with canonical
base/extended pointers and count. Its first consumers were the scheduler-dispatched
clear, water and tree tile loops, including shore-water reentry, and road
track-status reads (#523); #565 extends it to every Rust simulation entry.
Storage, allocation, setters, save adapters and worker ordering stay in C++.

Rust reaches the view under the game-state mutex by a load from the canonical
C++ map statics (base and extended pointers, count, `log_x`, size and the
native assertion policy), never by a C++ call per entry, and `map_func::World`
is its one type; an entry needs no fetch slot of its own. Use field-sized raw
loads, no Rust references/slices, and reload after mutations at the original observation
points. The interval is proved once for the game loop and command execution
(below), not per entry; descriptor metadata ends before any callback that can
load, reset or replace the map or yield, such as generation progress. Same-thread
reentry keeps the descriptor only within that interval.

Define each source helper once in its header-named Rust module. Generated named
ABI probes pin both layouts; preserve native helper assertions under the native
assertion policy. Assertion-disabled builds gain no new panics or rejected
inputs. A helper whose precondition fails calls the native assertion, as the
original accessor does (`map_value` in movement.rs), never the C++ accessor, so
a conversion deletes the service it replaces with its orphan bindings. Callers
not yet converted keep their typed service until a port touches them.

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

## Simulation-session lifecycle (#551)

The canonical game mode, switch request and pause-bit subobjects have one stable
Rust allocation. Native inline aliases begin their enum/bitset lifetimes once and
retain those addresses across loading and repeated reset. Rust uses field-sized
raw accesses, with no owner reference spanning native alias writes, callback
reentry, progress yields, networking or load/reset. The native exit flag retains
its atomic storage and synchronization contract.

Session entries compose existing Rust owners directly. Native stack frames retain
chrono/resource lifetimes and exceptions from scheduled console commands, scripts,
save/load, generation and network operations that can execute scripts. Rust returns
before native rethrow and CompanyID backup destruction. Storage mode changes occur
only where the original calls them: failure before PSM_LEAVE_GAMELOOP leaves its
partial state, while later script failures occur after that transition. This owner
adds no map-descriptor permission.

## First vehicle storage port (#563)

Agent: /root/storage563 | Model: gpt-6-astra | Reasoning effort: high.

This is the selected phase 2 design. Its first implementation issue is
[#570](https://github.com/fetch-laboratories/openttd-rust/issues/570). Root queues that issue in the roadmap; implementation starts when its branch
and agent slots are available. #564's metric precedes the next ranking round, not this
already-selected design. Its implementation starts after #581 retires the build
without Rust, so each moved record has one definition and no C++-owned copy for a
fallback build. The first port is one ownership change with three
implementation checkpoints: shared records and native adaptation; retirement of
the displaced data boundary; direct Tick dispatch. A record-only scaffold does
not complete it.

### Records and identities

Keep the stable polymorphic C++ `Vehicle` shell and the current native slot pool
for this first port. Allocate a stable Rust `VehicleRecord` beside each shell,
including effects and disasters. Define it once with `repr(C)`, generate its C++
declaration, and make it the canonical target of the typed Rust `VehicleRef`.
`RustVehicle(native)` returns the record; `VehicleOf(record)` reads its opaque
native-shell pointer. Neither helper calls across the boundary. Native station
lists, orders, GUI selections, pool slots and virtual operations continue to hold
`Vehicle *`. A record pointer is not a native shell pointer. Audit every existing
reinterpret cast and every legacy `void *` boundary when changing this handle;
legacy shell contexts retain their explicit native-shell conversion.

The first fields are the following, retaining their native integer domains and
initial values. Here `u8`, `u16`, `u32` and `i32` describe storage widths; Rust
uses the existing transparent IDs/enums from the corresponding header modules.
The generated C++ storage uses scalar base types and opaque pointers, with typed
inline native conversions, rather than overlaying a `StrongType` or `EnumBitSet`
object onto an integer.

| Canonical record | First fields |
| --- | --- |
| `VehicleRecord`, moved from `Vehicle` and `BaseVehicle` | `type`, `subtype`, `owner`, `direction`, `vehstatus`, `subspeed`, `acceleration`, `progress`, `tick_counter`, `running_ticks`, `day_counter`, `breakdown_ctr` as u8; `engine_type`, `cur_speed`, `cargo_cap`, `refit_cap`, `cargo_age_counter`, `last_station_visited` as u16; `tile`, `dest_tile`, `motion_counter` as u32; `x_pos`, `y_pos`, `z_pos` as i32. |
| `VehicleRecord`, moved from `VehicleCache` | `cached_max_speed` and `cached_cargo_age_period` as u16. `cached_vis_effect` stays in its existing Rust visual owner, accessed directly there. |
| `VehicleRecord`, moved from `Vehicle` | `next`, `previous`, `first` as nullable/stable record pointers. They replace the shell links, rather than cache them. `first` initially points to this record. |
| `TrainState`, existing Rust owner made shared | `flags`, `crash_anim_pos`, `wait_counter`, `cached_max_curve_speed` as u16; `compatible_railtypes`, `railtypes` as u64; `track`, `force_proceed`, `user_def_data` as u8; `cached_tilt` as bool; `cached_curve_speed_mod` as i16. Move `other_multiheaded_part` from the shell to a nullable record pointer here. |
| `GroundState` and `GroundVehicleCache`, existing Rust owners made generated | `flags` as u16; `cached_weight`, `cached_slope_resistance`, `cached_max_te`, `cached_power`, `cached_air_drag` as u32; `cached_axle_resistance`, `cached_max_track_speed`, `cached_total_length`, `first_engine`, `last_speed` as u16; `cached_veh_length` as u8. |

The record also holds its opaque native shell pointer and stable typed links to
already-canonical controller, ground, order, consist, visual and occupancy
owners. These are owner identities, not copies of their fields or call-site
observation bundles. The controller link is a generated C union of individually
typed pointers to the six existing Rust owners. Only its constructor-selected
arm is read. A construction-time dispatch kind distinguishes the base shell and
the six concrete subclasses; it represents the former virtual Tick target, not
a second copy of mutable `Vehicle::type`. A plain base shell retains the original
base Tick result. Re-read the simulation `type` after a surviving Tick.

`current_order_time`, current order, order indices and shared-order links already
have Rust owners (`ConsistState` and `VehicleOrderState`). Reuse them in place;
do not put copies in `VehicleRecord`. Generate their existing flat declarations
from the Rust definitions as part of using them through this record. Leave
`OrderList` storage and its containers to their own storage port. Similarly,
reuse the existing ground/train owner allocations; exposing their layouts does
not require replacing or reallocating their contents. The native `TrainCache`
sprite-group pointer, sprite sequences/bounds, viewport hash, cargo containers,
Money/date fields and unselected vehicle fields stay in the shell.

The ownership path creates, initializes and destroys each moved record in Rust.
The C++ shell stores the owning record pointer. Derived constructors bind the
already-existing type-specific owner once it exists; constructor initialization
must finish before a record can be used. No owner allocation occurs on a Tick,
field access, chain traversal or boundary crossing. Slots may move; neither a
live shell nor its records may move. Destruction invalidates both identities,
including if a later shell reuses the same VehicleID. No generation counter,
reference count or new stale-ID rejection is introduced.

### C++ reads and writes

A C++ inline `Record()` access and typed inline getters/setters read or write the
same record memory. Scalar arithmetic can use the generated scalar member;
TileIndex, Owner, EngineID, StationID, Direction and flag uses convert at the
inline accessor, preserving the source helper and narrowing rules. There are no
stored reference members for every moved field, shadow values or exported
per-field getters. Existing diagnostic cache snapshots remain explicit values,
not authorities. In particular, `cachecheck.cpp` must still compare the same
cache members and restore what it previously restored.

All native reader families adapt in this port, including the less frequent
writers. The conversion scope is:

- `vehicle_base.h`, `vehicle_type.h`, `base_consist.h`, `ground_vehicle.hpp`,
  `train.h`, `roadveh.h`, `ship.h`, `aircraft.h`, `effectvehicle_base.h` and
  `disaster_vehicle.h`: field declarations, chain accessors, subtype/flag helpers,
  constructor defaults and owner lifetimes. `VehicleCache` retains only the
  still-native presentation facade and diagnostic copying it needs.
- `vehicle.cpp`, `vehicle_cmd.cpp`, the four controller `_cmd.cpp` files,
  `ground_vehicle.cpp`, `effectvehicle.cpp`, `disaster_vehicle.cpp`,
  `articulated_vehicles.cpp`, `autoreplace_cmd.cpp`, `order_cmd.cpp`, `group_cmd.cpp`,
  `economy.cpp`, `engine.cpp`, `company_cmd.cpp`, `roadstop.cpp`, `station_cmd.cpp`,
  `pbs.cpp`, rail/road/water/tunnel-bridge command and movement handlers: use the
  shared record at each original observation/write and keep operations as calls.
- All vehicle-consuming `src/rust/*services*`, `*adapter*` and `*_ffi.h` bindings,
  and `src/pathfinder/**`: convert handles explicitly; remove field callbacks,
  rather than redirecting them into new Rust field exports. Unmoved engine,
  cargo, road-type and station data retain their canonical typed accessors.
- `viewport.cpp`, `vehicle.cpp` drawing/hash code, `train_gui.cpp`,
  `roadveh_gui.cpp`, `aircraft_gui.cpp`, `vehicle_gui.cpp`, `depot_gui.cpp`,
  `build_vehicle_gui.cpp`, `smallmap_gui.cpp`, `news_gui.cpp`, `window.cpp`,
  `intro_gui.cpp`, vehicle lists and other `*_gui.cpp` users: inline record reads,
  with unchanged sprite, bounding-box, dirtying and picking operations.
- `src/newgrf*.{cpp,h}` and `rust/vehicle_services_newgrf.h`: retain native
  resolver contexts/shell identity, but read the same record bytes for vehicle
  variables, sprite direction, livery, callbacks and consist queries.
- `src/script/api/**`, vehicle/company/list/order queries and commands:
  preserve validation, return widths and shell identity; field reads are inline.
  Network command replay and join saves use these same command/save adapters;
  no new network representation or packet field is introduced.
- `saveload/vehicle_sl.cpp`, `saveload/oldloader_sl.cpp`,
  `saveload/afterload.cpp`, the reference conversions in `saveload/saveload.cpp`,
  `cachecheck.cpp`, `src/tests/**` and `tools/simulation/*-console.cpp`: adapt
  direct field addresses and fixups, including test fixtures that formerly
  assumed a Rust handle was a shell address.

This covers users outside the simulation inventory as consumers, not new owners
of their GUI, save/load, script or NewGRF logic. Build coverage is needed for
platform-specific consumers; textual search alone cannot certify their types.

Generate the declarations by extending the existing `layouts!` vocabulary and
`tools/generate-abi.py`: a typed record declaration emits the Rust struct,
ordered field metadata and generated C++ member declarations from the same field
list. The generator supports only the selected scalar/newtype, raw-pointer,
record and tagged-owner-union forms, not arbitrary C++/Rust syntax. Generated
headers are build outputs, not a second checked-in schema. Pin size, alignment,
every field offset and field width, including union arms, on each target. Use
existing named ABI probes on GCC, Clang, MSVC and Emscripten; do not assume LP64
padding or class-return ABI. Calls carrying IDs continue to use scalar base
widths; Rust-side IDs stay transparent newtypes. Cross-language checks must fail
in ordinary CI when a declaration or probe is absent.

### Links, pool and lifetime

`Next`, `Previous` and `First` return `record_link->shell` to C++ inline and the
record link itself to Rust. Port `Vehicle::SetNext` as one Rust operation, with
its original self-link assertion, old-suffix first-pointer rewrite, old previous
clear, predecessor detach, and new-suffix first-pointer rewrite in that order.
Port `Last`, `Move` and the articulated/dualhead traversal helpers as their named
callees. A raw `next` store is not a substitute for `SetNext`. Native calls to
that operation use a thin typed facade. Reads of a next link after a callback
remain fresh; do not prefetch a chain into a vector.

`VehiclePool` remains `Pool<Vehicle, VehicleID, 512>` in the first port:
32-bit IDs; `End`/maximum size 0xFF000; invalid 0xFFFFF; ascending slot iteration;
lowest available bitmap bit at/after the native `first_free` word; normal
allocation advances `first_free`, indexed load allocation does not; `first_unused`
is a high-water mark that deletion does not lower. The `CanAllocateItem` assertion
credit, allocation-before-constructor publication, null holes and indexed-load
range/duplicate errors retain their existing order. Do not substitute a dense
arena or add a mirrored ID directory to find records.

The existing live slot descriptor remains canonical. Its slot values are native
shells; convert each encountered shell to its record once through the typed
owner-resolution operation, or expose a generated slot view only when the slot
owner itself moves. Do not reinterpret the native slot array as record pointers.
After an allocating callback, re-read `first_unused`; refresh the descriptor
before reading a slot beyond its captured extent. The cursor does not reset or
yield. Fleet drain/load/reset ends all cursors and record access scopes.

Normal destruction retains derived `PreDestructor` while the derived private
owner is live, then its member destruction, then `Vehicle::~Vehicle`, then base
member destruction, then `Pool::FreeItem`. The common record must outlive the
entire base destructor, including recursive next deletion, occupancy removal,
viewport-hash removal and news cleanup. Derived owner links are not accessed after
their original destruction point. Cleanup sets `cleaning` before ascending shell
deletions: `PreDestructor` skips normal work, cargo gets `OnCleanPool`, free
clears slots and counts, and bitmap clearing/PostDestructor are skipped exactly
as now. Free outside cleanup lowers `first_free`, clears the bit, then invokes
the existing PostDestructor hook. Vehicle uses the pool's default `Tcache=false`;
no allocation-cache policy is invented.

A later selected vehicle-pool port moves the slot array, used bitmap and these
operations together; C++ keeps a `PoolBase` registration facade and the same
inline slot reads and hooks. It is not part of this first issue, receives no
estimated saving here, and must not be described as completed by the hot-record
port. The remaining base-lifecycle candidate must coordinate with this record's
release and `SetNext`, rather than create another owner.

### Save/load

Keep the `VEHS` chunk, table field names, compatibility tables, version gates,
array indices and reference numbering. `SlVehicleCommon` addresses moved scalar
fields directly through the record, using the existing named descriptor form so
`Record().x_pos` does not silently rename the file field to `Record().x_pos`.
No whole-record serialization and no version bump. In particular:

- Tile/destination and x/y retain their pre-SLV_6 U16 conversion and later U32
  file widths; x/y retain their existing signed in-memory bit interpretation.
  Z retains pre-SLV_164 U8-to-I32 and later I32. Direction, type/subtype, status,
  fractional speed, acceleration, progress and tick/day/running bytes do not widen.
- Engine, speed, cargo/refit capacity, cargo-age counter and station fields retain
  their existing U16 widths and gates (`SLV_5`, `SLV_88`, `SLV_162`, `SLV_182`
  as applicable). Motion keeps `SLV_VEH_MOTION_COUNTER` and U32. Ground flags
  keep SLV_139/U16. Train flags retain SLV_2-to-SLV_100 U8-to-U16 and later U16;
  crash/wait remain U16, force/track U8, and wait retains SLV_136.
- `TrainStateScope` and its copied scalar save staging disappear: native named
  descriptors address the generated shared TrainState directly. Preserve partial
  load writes on error, NULL cleanup and pointer-fixup order. Road/ship path
  staging and other unselected save adapters stay with their existing owners.

For `next`, the canonical link is now a record pointer. Add internal
`REF_VEHICLE_RECORD`/`REF_VEHICLE_RECORD_OLD` adapters to the existing reference
conversion, not a file format. They use the same `SlSaveLoadRef` encoding and
old 4.3/4.4 sentinel branch as vehicle references, and resolve IDs through the
unchanged native pool before returning its record. Reference-to-integer follows
`record->shell->index + 1`; null remains zero in modern saves. These internal
reference kinds are not serialized as a new chunk type. The loader's temporary
integer-in-pointer phase ends before Rust can follow links. Old-loader direct
next assignments convert to records at their existing fixup points; failures
stay in C++, with no Rust frame spanning a throwing loader. `previous`, `first`,
`other_multiheaded_part`, caches, native-shell pointers and dispatch metadata stay
NOSAVE and rebuild at the original phase. Preserve the sequential versus bracket
multihead matching in `ConnectMultiheadedTrains`.

### Retiring the data boundary

The first issue removes the complete affected bindings, native bodies, Rust table
slots and ABI entries, not only the hot production caller. All other ports that
use a moved field adopt the shared record during this same change.

- Delete all 44 `TrainRead*` functions in `train_services.h` (219 physical body
  lines), all 29 `OpenTTDTrain*Read` carrier declarations in `train_ffi.h` (148
  lines), all 56 `OpenTTDTrainServices::read_*` slots and their 56 native bindings.
  The list includes `TrainReadNext`, `TrainReadMoveVehicle`, `TrainReadTickState`,
  `TrainReadLoco26`, `TrainReadStopLocation` and the consist/collision/reverse
  readers. Existing order/ground/visual owners supply their fields directly;
  no carrier is replaced with another snapshot.
- Delete train write slots/bodies `write_tile`, `write_dest`, `write_x`, `write_y`,
  `write_z`, `write_direction`, `write_speed`, `write_tick`, `write_running`,
  `write_day`, `write_order_time`, `write_progress`, `write_subspeed`,
  `write_gv_flags`, `write_acceleration`, `write_length`, `write_total_length`,
  `write_first_engine`, `write_max_speed`, `write_cargo_cap`, `write_refit_cap`,
  `write_cargo_age`, `write_last_station`, `write_status`. `write_colourmap`,
  `write_crossing_bar` and `write_visit_type` concern unmoved records and remain
  only while they have production callers.
- Delete the six `TrainReservationRead{Free,Free1,New,Choose,Choose4,CheckNext}`
  functions and matching carriers/slots, and the reservation writes for moved
  destination/station/ground flags. Order-list queries still call their existing
  canonical owner. Leave follower/map work to #560/#565.
- Delete the 21 `openttd_rust_train_state_get_*` and `set_*` exports/declarations and
  `TrainStateScope`; C++ Train getters become inline record access. The existing
  allocation/destruction operations remain typed calls. Ground/consist/order
  records are generated from their current Rust layouts, with no parallel field
  inventory.
- Delete shared vehicle slots `write_counters`, `write_xy`, `write_day`,
  `write_direction`, `write_speed`, `last_station`, `write_last_station`,
  `write_dest`, `write_progress`, `write_max_speed`, `previous_tile`, `next`,
  `engine_id`, `write_tick`, `direction`, `status`, `tile`, `progress`,
  `dest_tile`, `subtype`, `x`, `y`, `z`, `tick`, `owner`, `speed`, `subspeed`,
  `running`, `order_time`, `vehicle_type`, `write_x`, `write_y`, `write_z`,
  `write_tile`, `write_status`, `write_subspeed`, `write_breakdown_counter`,
  `write_running`, `write_order_time`, `write_cargo_age`, `write_coordinates`,
  `coordinates`, `xy`, `speed_inputs`, `direction_progress`, `tick_status`,
  `altitude_inputs`, `position_observations`, `movement_observations`, `max_speed`,
  `occupancy_update` and `occupancy_near`, plus their matching `VehicleSvc*` and
  `RustVehicle*` field wrappers and signature adapters once unused. Delete the
  corresponding `OpenTTDVehicle{PreviousTile,Coordinates,XY,SpeedInputs,
  DirectionProgress,TickStatus,AltitudeInputs,PositionObservations,
  MovementObservations}` and `OpenTTDOccupancy{Update,Near}Observation` carriers.
- Split behavioral operations from observations: `SetDestTile` still calls the
  type's setter, which can clear its path; viewport dirtying still performs the
  original operation. `GroundSvcUpdateViewportThenNext` becomes the viewport
  operation followed by live Rust next/progress reads; delete
  `OpenTTDGroundViewportNext`. `VehicleSvcCheckOrders` keeps CheckOrders as an
  operation and Rust reads running afterwards. `OpenTTDVehicleDayCost` disappears;
  profit stays a shared native field service and running is read in place.
- Remove road observation wrappers/carriers `TickRead`, `MaxSpeedRead`,
  `SpeedRead`, `SpeedPart`, `Link`, `LinkTile`, `Step`, `CrashPart`, `Entered`,
  `Candidate`, `PartRead`, `PathRead`, `StopRead`, `OvertakeRead`, `ArrivalRead`,
  `SoundRead`, `ServiceRead`, `TurnRead`, `TrackdirRead` (prefix `OpenTTDRoad`,
  with corresponding `RoadSvc*`/`Road*Read` definitions). Read common/ground/order
  fields directly; use the canonical typed road-type, engine, station and
  settings accessors for the still-unmoved inputs. Keep operation result types
  such as `RoadTrackChoice`/`RoadDepotResult`. Apply the same rule to mixed
  aircraft/effect/disaster/ship observations: remove the moved fields/carrier,
  retain only actual operation results or canonical access to unmoved records.

`OpenTTDVehiclePostTick` (12 declaration lines), `VehicleSchedulerTick` (13 body
lines), its callback/binding, `RustRoadTickRead` (11 body lines) and the four
compiled native Tick facades also retire in the dispatch checkpoint below.
The named common observation carriers plus PostTick and GroundViewportNext total
64 physical declaration lines. These are source spans, not #564 code-line metrics
or a claim that complete vehicle headers retire. The implementation reports the
actual whole-fork metric after all consumers and generated declarations exist.

### Direct Tick dispatch and evidence

The Rust scheduler matches the constructor's dispatch kind and directly calls
the train, road, ship and aircraft Rust Tick implementations. Effect and disaster
Tick already return to Rust through native facades; call their owners directly
as well. On effect expiry, end the controller borrow and invoke the canonical
native delete operation; do not retain `RunEffect` merely for this trampoline.
Base Tick still returns true. Native `Tick` bodies/declarations with no remaining
production caller are deleted. Fixtures do not keep
orphan production exports alive.

Bind immutable controller service tables once at scheduler entry, preserving
existing initialization dependencies. Keep the original timing scope: road and
ship always; train front engines only; aircraft normal aircraft only, after its
early gate. A caller-owned native frame supplies placement storage for
`PerformanceAccumulator` and `OpenTTDRoadCommandSlot`. Typed begin/end operations
construct/destruct those native objects without calling Rust. Rust invokes its
controller between them. Road scratch is lazy, reset/destructed each road Tick,
and never shared across nested entries. Timing and scratch destruction finish
before the scheduler's original survival assertion. This replaces the virtual
bounce without an allocation, a callback-into-Rust timer wrapper or loss of
presentation timing. End all field borrows before operations; after failure,
access no dead record. After survival, reload type, first record and fields at
the source post-Tick branches instead of filling an address bundle.

The three checkpoints have conservative P2016 planning credits, not measured
speed results: declarations/native adaptation receives zero; direct train record
reads approximately 52.8M Ir (0.391%); dispatch/post-Tick approximately 63.4M
(0.469%). Their disjoint sum is about 116.2M (0.860%). The implementation issue
records the retained-profile counts, replacement allowances and uncertainty.
The wider #275 opportunities of 133M for dispatch and about 120M for train reads
are not added again. No viewport, #560 follower, #565 map, #555 repair or drawing
saving is claimed. Allocation locality, register allocation and native drawing
cost remain unmeasured until implementation evidence exists.

Extend the existing vehicle/fleet/rail/road/ship/aircraft/effect/disaster scenarios
and native fixtures. Required distinctions are ordinary and indexed allocation,
holes/growth/reuse during Tick, deletion of current and unvisited chain members,
SetNext and multihead ordering, fresh fields after callbacks, shared-state writes
from both languages, reload and cache rebuilding, each mode's timing gates, and
the original survival assertion. Move the synthetic native virtual-Tick tests
onto the Rust traversal kernel with test-supplied work, and test concrete
production dispatch with real shells; do not keep an unused vtable trampoline
for fixtures or change production layouts under `cfg(test)`. Keep existing
semantic decoders and both RNG streams unchanged. Native ABI/lifetime checks
cover transient pointer behavior that saves cannot observe; old-reference-width
and invalid-reference fixtures cover the changed save address adapter.

#560 owns the follower and GetPlatformLength; this port consumes its integrated
interfaces and claims none of its credit. #562 owns station lifecycle and native
loading-list removal; stable shell pointers and PreDestructor reentry remain its
contract. Coordinate overlapping train/vehicle service files at final base
update, rather than making an unmerged service dependency. Root must sequence
the implementation with the queued base vehicle lifecycle owner.

#566 is the drawing-evidence dependency. It is selected but has no integrated
workload/result at this design checkpoint. The implementation may be prepared
while #566 proceeds, but root must obtain its reference/candidate image and
instruction comparison before calling the first storage port drawing-validated
or integrating it under the roadmap's requirement. A `-vnull` play result cannot
substitute for it. The expected instruction savings are planning estimates; the merged-tip profile
will measure the result.
