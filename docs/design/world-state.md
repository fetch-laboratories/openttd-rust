# Incremental world-state ownership

Readers: storage owners, their briefers and reviewers. Design: [#90](https://github.com/fetch-laboratories/openttd-rust/issues/90). Implementation requires [roadmap](../roadmap.md) selection; the phase boundaries of each pool brief are in the [reader and pool placement](reader-pool-placement.md) design. Ownership transfers follow [storage ports](../porting.md#storage-ports), [boundary](../porting.md#boundary) and [map access](../porting.md#map-access); this file holds the decisions and contracts specific to world state.

## Constraints in the current game

- `Map::Allocate` replaces both map arrays; mutable C++ tile references must move with their allocation. Tile indices are uint32 and type-dependent fields share bits.
- Pools select the first free slot, support indexed load allocation and ID reuse, optionally cache allocations and invoke destruction hooks. An ID is not a generation-checked lifetime guarantee: resolve IDs under source lifetime rules, since reacquiring an ID alone does not make reuse safe. Compacting pools, generation counters, reference counts or new stale-ID rejection would change original behavior.
- `StateGameLoop` orders timers, tiles, vehicles and landscape. `RunTileLoop` uses LFSR order and special handling for tile zero; sorting or parallelizing changes observations and shared random draws.
- Link graph jobs copy graph and settings; TGP's height map is temporary generator state. A general live-world snapshot would change observation semantics. Rust owns the temporary height map and the link graph job's computation snapshot, with job scheduling, save/load and join in C++; neither moves the global map or station pool.
- C++ owns map arrays and live pools until their storage owner moves them. A C++ object pointer may be an opaque context, never a Rust view of an STL or pool object. Polymorphic shells remain opaque.

## Raw access contracts

Rust reads native storage in place only under these contracts and the [map access decision](#map-access-decision-108). Each authorizes no other raw field access.

- **Post-tick scalars (#213).** Named native scalar addresses, not a Vehicle, container or bitset layout. `BaseBitSet::base_address` returns its actual `const Tstorage *`; the canonical typed vehicle service exposes front/current status addresses. Acquire after Tick survives, use at original branches under the game-state mutex, and discard before another Tick or fleet drain. Raw access ends before callbacks. Audited cargo/sound reentry cannot destroy shells, reset pools or yield the mutex. Native tests check types, address identity, source-order reads and permitted writes.
- **Pool descriptor.** The call-scoped descriptor exposes the actual slot array, initialized `data.size()` and canonical `first_unused` address. After an allocating callback, read `first_unused` before a cached slot; refresh the descriptor if it exceeds captured size, since every reallocation raises it beyond the old size. Selected callbacks cannot reset the pool; cursors cannot reset or yield, and descriptors end before load, reset or fleet drain. Native tests compare growth, deletion, holes and reuse with PoolIterator. Tick retains a typed native facade and controller timing.
- **Tile buckets (#377).** Rust holds the canonical tile bucket array and intrusive links. A stable native shell embeds a named FFI carrier exposed by a typed accessor. Bucket and link addresses remain stable until shell destruction or index reset. Under the mutex, an iterator retains its shell cursor and bounds and reads the live next link after callbacks. The current shell survives advancement; crash/flood callbacks may mutate links but cannot delete it. Reset ends cursors before load/pool reset and clears current-bucket pointers and buckets, retaining source stale links.
- **Movement links (#275).** Normal update resolves one shell for link carrier and tile; removal acquires links alone. Near filtering observes x/y and carrier together, reads next on rejection and reacquires links after visitors. Aircraft and road reuse call-local map dimensions because synchronous controller callbacks cannot reset/load the map or yield the mutex; nested entries get fresh metadata. Train, ship and other entries read dimensions per update. Road movement lazily acquires the canonical tile descriptor once per entry for track-status tile type and normal-road bits/status/road-type reads, reloading after mutations; partial fixture tables retain the typed backend.

## Map access decision (#108)

C++ retains canonical allocation, setters and field-wise save adapters until the map storage port. Rust accesses live fields through the [shared map view](../porting.md#map-access); C++ references may alias its pointers, so raw access does not permit concurrent conflicting access. Preserve observation and write order across callbacks.

`Map::Allocate` covers new games, modern and legacy loads and tests. Serialization reads live fields before asynchronous compression starts. Link-graph workers use private graph copies and map dimensions, not tile arrays; retain job join/reset ordering.

### Shared map view (#274, extended by #565)

The descriptor has canonical base/extended pointers and count. Its shared table holds addresses of the native map statics; entries load their current pointers, `log_x` and size. Generated named ABI probes pin both layouts. [Map access](../porting.md#map-access) defines `map_func::World`, field access, native assertions and the game-loop interval proof; the view does not transfer storage.

In `tile_loop.rs`, handlers may mutate tiles and pools, run landscape, road and crash commands and queue script events (`InsertEvent`), but cannot reset, load, report progress or run a script VM. They do not change map dimensions, climate, tree placement, tick counter, editor mode, road-rebuild or ambient settings; `_misc_grf_features` changes only on reset and GRF `ActionD` loading. A batch may read these settings, including the ambient feature gate, once and retain descriptor metadata across dispatch. Ambient callbacks retain their RNG position; the tree loop reads fields after them at source observation points. Threaded and arbitrary NewGRF corpus limits remain open in #156.

#565 converts Rust entries to `map_func::World` in batches; its body holds the converted entries. Cached road-controller entries keep one `World` per entry; header-named terrain helpers load canonical corner heights at each observation. Unconverted helpers keep their fallbacks, and callbacks required by other production consumers remain. Road YAPF searches resolve road/tram kind, owner, compatible road types, display speed and order speed once per search: their synchronous query callbacks cannot mutate these vehicle facts, while tile fields and tile speed limits are observed at each source point. Ship YAPF and water-region entries do not use `map_func::World` yet; train, aircraft and rail and road YAPF are partly converted, and their remaining leaves wait for later #565 batches.

## Saves

- Keep chunk IDs, versions, field widths and reference numbering. `src/saveload/map_sl.cpp` serializes fields rather than struct bytes; `saveload.cpp` maps references to index + 1, with zero for null and legacy load and pointer-fixup rules. Adapt the existing C++ adapters and load lifecycle when an owner moves, without changing the format, serializing whole records or bumping the version. Rust-owned persistent state must be visible to saving before the mutating call returns.
- Named native descriptors address shared record fields directly without renaming file fields (`Record().x_pos` saves as `x_pos`), and without copied scalar staging. Preserve partial writes on load errors, NULL cleanup and pointer-fixup order.
- Record references use internal adapters with the existing reference encoding and old sentinel branches, resolving IDs through the native pool; they are not new chunk types. End the loader's temporary integer-in-pointer phase before Rust follows links. Keep loader failures in C++, without a Rust frame spanning a throwing loader.
- Back links, caches, shell pointers and dispatch metadata stay NOSAVE and rebuild at the original phase.

## Records and evidence

- Build platform-specific consumers; textual search cannot certify their types.
- Pin size, alignment and each field offset and width, including union arms, with named ABI probes on GCC, Clang, MSVC and Emscripten; do not assume LP64 padding or class-return ABI. Ordinary CI fails if a cross-language declaration or probe is missing. Generate scalar base types with typed inline conversions; do not overlay `StrongType` or `EnumBitSet` on an integer. Generated headers are build outputs, not a second checked-in schema.
- Keep desync snapshots and plain runs ([evidence](../porting.md#evidence)): rebuilding caches can conceal faults. Vehicle ports need applicable #86/#104 evidence and branch witnesses per controller; saves cannot expose every transient state.

## Process-wide owners

These owners are delivered; their contracts bind later changes to them.

- **Cargo registry views (#467).** One process-wide Rust owner holds the cargo registry views; native spans alias its buffers until rebuild. Language-loader sorting may precede climate setup. Publish clears and populated views before native collation, and each town-list insertion before the special-cargo break. Rebuild/reset is serialized loading outside callbacks. Each cargo callback has its own Rust invocation and native virtual/profiler frame. This grants no additional map access.
- **World-generation lifecycle (#546).** Generation state has one process-wide, static-lifetime Rust allocation. C++ aliases only the stable `_generating_world` bool subobject; both languages use one-byte, one-aligned bools and write only false or true. Startup precedes the game thread; subsequent accesses use the game-state mutex on both GUI threads. Native CompanyID backup and landscape/town/industry/object frames remain opaque native objects, with the backup's destructor/restoration contract. Native wrappers catch progress abortion, script and completion callbacks and save/load errors; resource frames outlive Rust entries. Rust chooses cleanup, storage rollback and company restoration before native mode switching can reenter generation. Map descriptors expire before progress updates; this grants no additional map access.
- **Simulation-session lifecycle (#551).** Game mode, switch request and pause-bit subobjects share one stable Rust allocation. Native inline aliases begin enum/bitset lifetimes once and retain their addresses across loading and reset; the native exit flag retains atomic storage and synchronization. Native frames retain chrono/resource lifetimes and exceptions from scheduled console commands, scripts, save/load, generation and network operations that can execute scripts. Rust returns before native rethrow and CompanyID backup destruction. Storage mode changes occur at the original calls: failure before `PSM_LEAVE_GAMELOOP` leaves partial state; later script failures follow that transition. This grants no additional map-descriptor access.

## Vehicle records (#570) and direct Tick dispatch (#656)

#563's design delivered shared records and the data-boundary retirement through #570 and #636; its field tables and deletion inventories stay on [#563](https://github.com/fetch-laboratories/openttd-rust/issues/563) and [#570](https://github.com/fetch-laboratories/openttd-rust/issues/570). #656 owns the residual fields, pool completion and direct Tick dispatch.

### Records, links and lifetime

- A stable Rust `VehicleRecord` sits beside every polymorphic C++ `Vehicle` shell, including effects and disasters; the native slot pool stays. `RustVehicle(native)` returns the record and `VehicleOf(record)` reads its opaque shell pointer; neither crosses the boundary. Typed Rust `VehicleRef` targets the record.
- Native station lists, orders, GUI selections, pool slots and virtual operations keep `Vehicle *`; a record pointer is not a shell pointer. Audit reinterpret casts and legacy `void *` boundaries when changing a handle.
- The record stores the opaque shell pointer and stable typed links to the canonical controller, ground, order, consist, visual and occupancy owners. Links identify owners; they do not copy fields or observations. The controller link is a C union of typed pointers to the six Rust owners: read only the arm selected by the constructor. `ConsistState` and `VehicleOrderState` remain their own records.
- Rust creates, initializes and destroys records; the shell holds the owning record pointer. Bind each type-specific owner in the derived constructor and finish initialization before using the record. Slots may move, but live shells and records stay stable. Destruction invalidates both identities, even if a later shell reuses the VehicleID.
- `SetDestTile` stays the type's setter, which can clear its path. `CheckOrders` and viewport dirtying stay operations. Diagnostic cache snapshots remain values; `cachecheck.cpp` compares and restores the same members as the original.
- `Next`, `Previous` and `First` return `record_link->shell` inline to C++ and the record link to Rust. `SetNext` is one Rust operation: self-link assertion, old-suffix first-pointer rewrite, old previous clear, predecessor detach and new-suffix first-pointer rewrite, in that order; a raw `next` store is not a substitute. Read next links afresh after callbacks; do not prefetch a chain into a vector.
- `VehiclePool` is `RustPool<Vehicle, VehicleID, 512>` (`src/rust/pool.hpp`: native item and lifetime operations over the canonical Rust pool policy): 32-bit IDs, `End` and maximum size 0xFF000, invalid 0xFFFFF, ascending slot iteration and default `Tcache=false`.
- Allocate the lowest free bitmap bit at or after the native `first_free` word. Normal allocation advances `first_free`; indexed load allocation does not. Deletion does not lower the `first_unused` high-water mark. Keep the order of `CanAllocateItem` assertion credit, publication before construction, null holes and indexed-load range/duplicate errors.
- Slots contain native shells: resolve each shell once through the typed owner operation, or expose a generated slot view only when the slot owner moves. Do not reinterpret slots as record pointers.
- Destroy in order: derived `PreDestructor` while its private owner is live, derived members, `Vehicle::~Vehicle`, base members, `Pool::FreeItem`. Keep the common record alive through the base destructor, including recursive next deletion, occupancy removal, viewport-hash removal and news cleanup. Access no derived owner link after its original destruction point.
- During cleanup, set `cleaning` before ascending shell deletions: `PreDestructor` skips normal work, cargo receives `OnCleanPool`, free clears slots and counts, and bitmap clearing and `PostDestructor` are skipped. Outside cleanup, free lowers `first_free`, clears the bit, then calls `PostDestructor`.
- `VEHS` keeps its field names, compatibility tables, version gates, array indices and reference numbering. Rebuild `previous`, `first` and `other_multiheaded_part` at the original phase, preserving sequential versus bracket multihead matching in `ConnectMultiheadedTrains`.

### Direct Tick dispatch

- Set a dispatch kind at construction for the base shell and six concrete subclasses; it identifies the former virtual Tick target, not mutable `Vehicle::type`. The Rust scheduler matches it and calls the train, road, ship, aircraft, effect and disaster Rust Tick owners directly; base Tick returns true. Keep the base Tick result and reread simulation `type` after a surviving Tick.
- On effect expiry, end the controller borrow and call the canonical native delete operation; remove the `RunEffect` trampoline. The dispatch also retires `OpenTTDVehiclePostTick`, `VehicleSchedulerTick` with its callback and binding, and the native controller Tick facades.
- Bind immutable controller service tables once at scheduler entry, preserving initialization dependencies.
- Keep timing for every road and ship Tick, train front engines only, and normal aircraft only after the early gate. A caller-owned native frame provides placement storage for `PerformanceAccumulator` and `OpenTTDRoadCommandSlot`; typed begin/end operations construct and destroy them without calling Rust, and Rust runs the controller between them.
- Create road scratch lazily, reset and destroy it each road Tick, and never share it across nested entries. Finish timing and scratch destruction before the survival assertion, without an allocation or a timer wrapper that calls back into Rust.
- End field borrows before operations. After failure, access no dead record; after survival, reload type, first record and fields at source post-Tick branches. Preserve stable shell pointers and PreDestructor reentry from station loading lists.
- Extend vehicle, fleet, rail, road, ship, aircraft, effect and disaster scenarios and native fixtures for ordinary and indexed allocation, holes/growth/reuse during Tick, deletion of current and unvisited chain members, SetNext and multihead order, fresh fields after callbacks, writes from both languages, reload/cache rebuilding, timing gates and the survival assertion. Move synthetic native virtual-Tick tests to the Rust traversal kernel with test-supplied work, and test production dispatch with real shells; keep no unused vtable trampoline. Keep semantic decoders and both RNG streams unchanged.

## Later pool record placement (#634, #635; decided 2026-11-05 after integration 82)

Later pool records sit beside their shells, as #570's do. The decision and its measurements are on [#634](https://github.com/fetch-laboratories/openttd-rust/issues/634#issuecomment-6671702315).

Basis: beside placement costs one extra instruction per C++ read, at most 0.033% of play-save loop Ir, and inside placement would save at most 0.024% for any family.

**Conditions, which every brief states:**
1. **No out-of-line record↔shell call.**
   - Rust's typed ID resolution returns the record.
   - Slot iteration reaches the record in one load.
   - C++ reads through inline `Record()` accessors (`this->record->field`). A Rust-called service routed through `Record()` is still a [design fault](../porting.md#design-faults).
2. **No stored references.** Every reference member is replaced, including the 448 `GoodsEntry` references per station and the town and industry blocks.

**Per family:**
- Station, waypoint, `GoodsEntry`, airport, road stop, town, industry and subsidy: beside.
- Cargo packet and payment: beside now. Co-allocating the record in the same pool slot is decided at #634's pool-completion checkpoint. Their second allocation is the one measurable cost: 0.052% on P2016 and 0.011% on Opus.
- Vehicles stay beside, with no conversion. Two out-of-line hops cost 0.44% to 0.51% gross and go to #656 or a #275 repair batch:
  - `VehicleSvcTileLinks`: move `tile_links` into the record.
  - `VehicleRecordFromShell`: give the scheduler and the station loading queue record pointers.

**If inside placement is ever chosen**, use allocation in the pool slot just before the shell, not a C++ member or base class. A base class can reuse tail padding on Itanium and not on MSVC, and the vehicle record has 4 bytes of it.

**Revisit a family's placement** at its pool-allocation step if either of these happens:
- its record↔shell hops reach 0.5% of loop Ir (#275's threshold);
- cache profiling shows misses on record lines.

## Shared pool policy (#716)

- Pool owners use the integrated shared Rust allocation/bookkeeping primitive from #716. Its generated canonical slot/descriptor storage carries shell and record identities, occupancy, cursors, counts, native assertion credit, cleaning and cache state. Typed IDs load records from slots directly; native consumers read the shared layout inline. Reuse the existing shared-record generator, NativeVec, PoolCore, NativeOps, PoolConfig, TypedPool and RustPool definitions, without a second metadata authority or stored field references.
- Native allocation/deallocation, pool-index access and actual shell destruction remain typed operations while C++ shells exist. Preserve publication before construction, the live shell slot throughout destruction, record binding and retirement at their existing owner-member points, and free-before-post-hook ordering. No borrow spans hooks or storage invalidation. Indexed load errors return to C++ before `SlErrorCorruptFmt` can throw. Native registration and cleanup-mask order remain unchanged; borrowed resolver shells stay nonpooled.
- Reuse one native compatibility facade. The native generic template remains until its final instantiation retires; credit that deletion once. Records, containers and control-flow completion remain separate obligations of each owner.
- All-pools-before-map includes the non-simulation Pool registrations: NetworkClientInfoPool and NetworkClientSocketPool transfer with #669 network core/discovery/resource ownership, NetworkAdminSocketPool with its admin owner, and SpriteGroupPool with GRF publication/lifetime. These pools must supply actual allocation/bookkeeping/lifetime closure before map allocation. Native read-only iterator wrappers may remain with their named reader/support/GUI consumers; they are not a second mutable pool authority and retire with the final candidate-C++ consumers.
- Use #719's shared shell-identity BindRecordIfOwned extension where required. Do not infer membership from an uninitialized or copied index, or retire a caller-owned record when destroying a borrowed shell. Preserve the normal vehicle indexed path and original owner/drop timing. Pooled production shells bind and retire by index. The O(extent) shell-identity scan serves only nonpooled shells: it makes LGRP load and CleanPool quadratic for pooled shells, and CargoPacket churn would make it a hot path. Delete fixture-only stack or copy shells rather than keeping the scan alive for them.

## Link graph and job storage (#717)

`LinkGraphPool` and `LinkGraphJobPool` are already `RustPool`s on the shared pool core (#719). The selected owner moves graph/job records, private graph/settings snapshots, annotations, flows, join/abort state and completed-job publication into their canonical Rust owners. It uses #716 for shared pool policy and preserves allocation IDs, source worker startup/failure fallback, cancellation/join/reset order and save references. A native thread handle may remain behind named start/join operations until platform threading moves; Rust-used state has no field-callback exception.

Station #634 owns its record/container adaptation; #717 owns flow contents and completion publication at the original observation points. Both reuse one shared definition. Worker-private snapshots cannot borrow live-world storage; publication, destruction and reset end affected borrows before reentry. Native GUI, save, script and network consumers retain only their named later-owned shell operations and inline shared-field reads.

## Engine and renewal storage (#637 with #497)

The selected owner follows the [shared pool policy](#shared-pool-policy-716). Define Engine and EngineRenew records once through the existing shared-record generator, beside the shell under the [placement decision](#later-pool-record-placement-634-635-decided-2026-11-05-after-integration-82). Adopt #655's integrated loading behavior and the single native pool facade and operation schema reserved to #714; do not create a competing definition or make independent record work depend on another draft.

| Allocation | Canonical owner and consumers | Lifetime and remaining native owner |
| --- | --- | --- |
| Engine record | Consolidate existing Rust lifecycle/name storage with info, vehicle-specific info, original image/type/list position, GRF cargo bindings, ordered wagon overrides and badges. Rust loading, lifecycle, controllers and resolvers use the record; native save, script, strings and drawing callers read its shared layout. | Preserve default/monostate and typed constructor publication order, including original table copies, wagon life and aircraft cargo defaults. Temporary nonpooled prototypes and indexed pooled construction are distinct: never inspect an uninitialized prototype pool index to decide ownership. Borrowed GRF/SpriteGroup pointers expire with reload/reset. |
| EngineRenew record and company head | Replace the existing fleet-owner aliases and native-node links with canonical record links. Fleet replacement/group operations and ERNW/company save adapters retain list order and reference identity. | Initialize a new node before publishing the head; single removal updates predecessor/head before deletion. Remove-all saves next, deletes each node, then clears the head. The head remains company-owned; each node's record retires at its actual owning-member destruction. |
| Pool slots | EngineID and EngineRenewID retain u16 coordinates, maximum 64000 and invalid FFFF. Both pools are Normal and uncached; growth is 64 and 16 respectively. Reuse the shared Rust allocation, bitmap, cursor/count/credit, cleanup and live-iteration policy. | Retain native registration order/masks, publication before construction, live shell slots throughout destruction and free-before-post-hook order. Growth invalidates descriptor buffers, not separately allocated shells/records. ID reuse starts a new lifetime. |
| Native shell | Keep identity/index and inline record access for remaining native callers. Only display_flags and display_last_variant remain field owners for phase 5's build-list GUI. | Name each remaining native consumer, including build/autoreplace/engine GUIs, vehicle drawing, script_engine and save/afterload/old-loader adapters. Networking retains command payloads and synchronized execution. Shell compatibility retires with its final callers. |

Make SetCustomEngineSprites, SetWagonOverrideSprites and SetEngineGRF Rust-internal and remove the typed engine-string leaf and displaced property/resolver services. Preserve sorted cargo bindings, ordered override matching, incremental Action 3/4 publication and the integrated last_engines reset behavior. End record/vector/name views before mutation, notification, reentry, destruction and GRF reset; the current reset order rebuilds engines before freeing files.

ENGN retains lifecycle/name staging before definition setup and copies only saved fields afterwards, including gates 31/84/104/179/193 and legacy preview conversion. ENGS's 256 legacy names, EIDS ordering and ERNW's widths, next fixups, group gates 60/71 and when-old gate 175 remain unchanged. The company renewal head keeps REF_ENGINE_RENEWS from version 19.

The estimate and checkpoint cost belong to [#637](https://github.com/fetch-laboratories/openttd-rust/issues/637). Reuse engine/fleet ABI and lifetime fixtures, engine/renewal/resolver/reload scenarios, newgrf-loading-bindings and properties observations, and existing drawing checks; extend only a concrete uncovered allocation or lifetime observation.

## Small simulation pools and ObjectSpec (#638 with #498)

The phase-2 owner contains nine pools: Object, Depot, Sign, Goal, StoryPage, StoryPageElement, LeagueTable, LeagueTableElement and PersistentStorage. ObjectSpec global/per-file allocations and ordered publication are coupled #498, not another phase-3 owner. The estimate belongs to [#638](https://github.com/fetch-laboratories/openttd-rust/issues/638).

| Allocation | Canonical Rust state and consumers | Lifetime and retained native owner |
| --- | --- | --- |
| Object | Type, town link, area, build date, colour and view; commands, generation, resolver and animation owners read records directly. | Preserve map publication/default order and stale-object deletion during old-save fixup. Drawing, scripts and OBJS read the shared record through inline access. |
| Depot | Town number, tile/link, owned name bytes and build date; rail/road/water, orders, fleet and town consumers use the record. | Preserve cleaning and missing/mismatched-tile returns. Otherwise reset order backups, remove depot orders, close depot window, then depot-list window. |
| Sign | Owned name, coordinates and owner. | Allocate, initialize, position, then notify lists; removal dirties/removes viewport entry before deletion and notification. Only Sign::sign remains with phase-5 viewport/drawing. |
| Goal and story records | Company/type/destination/progress/completion and encoded bytes; page date/sort/title; element page/type/reference/text. | Preserve validation/test-execute ordering and page-element cascade iteration. Reconstruct next-sort with source wrapping; IDs do not extend referenced lifetimes. |
| League records | Table title/header/footer and element table/rating/company/text/score/link. | Preserve signed ratings, tie order, link validation and reset invalidation. GUI/script/save readers use canonical fields. |
| PersistentStorage | Existing Rust values, backup and transaction ownership consolidate with native grfid/feature/tile metadata and actual pool state. | Preserve native BasePersistentStorageArray pointer identity/order in the transaction registry. ClearChanges restores values without unregistering; actual destruction unregisters before payload release. Legacy arrays remain separate nonpooled owners. |
| ObjectSpec | Canonical global specs, stable per-file definitions, defaults, empty fallback, containers and borrowed GRF/SpriteGroup links. | Deferred string writes read the definition's current class. Finish resize/move before class binding; views expire on growth/reset/reload. Drawing/script/native resolvers use the shared payload until their consumer phases migrate. |

All nine pools are Normal and uncached. Object uses u32 IDs, maximum 0xFF0000, invalid FFFFFFFF and growth 64; PersistentStorage uses u32, maximum 0xFF000, invalid FFFFF and growth 1. LeagueTable uses u8, maximum 255, invalid FF and growth 4. Other IDs are u16, maximum 64000 and invalid FFFF; growth is 64 except Sign's 16. Preserve registration/cleanup masks, publication before construction, assertion credit, occupancy/cursors/counts, indexed reuse and free-before-post-hook order. Descriptor buffers expire on growth; separately allocated live shells and records remain stable. Bind at the actual owning-member point and retire before record destruction while the native shell slot is still live.

PersistentStorage needs a concrete nonpooled distinction: OldPersistentStorage has static legacy instances and tests use stack 16/256-value arrays. Its storage base initializes Rust payload before the PoolItem base; never read an uninitialized index or bind every base array. Preserve the actual native base-subobject address used by the registry. Reuse the one shell-identity binding extension from #719. Town/industry attachment vectors belong to #635; #638 owns the PSA payload and pool. Shared consumers use one create/destroy/reference table.

ObjectSpec includes IgnoreObjectProperty, ObjectChangeInfo, ResetObjects, BindToClasses, SetEntitySpec and FinaliseObjectsArray. Reuse override_manager, newgrf_class::object, strings and badge owners. Preserve limit-check-before-resize, resize-before-interpretation otherwise, ignored widths/greatest result, property08 allocation before class dword, current-class deferred names, size/view repair, build-to-clear cost copy, dword dates, sticky 2CC, separate animation writes and atomic badge destination publication with partial seen state. Finalize in source file/spec order, consult saved mapping before transmitter allocation, preserve exhaustion logging/return, clear/shrink per-file payloads, then bind classes. ResetObjects does not reset mappings; persistent-NewGRF reset omits object manager; OBID resets separately. Register only the real global manager.

Preserve OBID/OBJS, DEPT, SIGN, GOAL, STPA/STPE, LEAT/LEAE and PSAC identity, widths, gates and index-plus-one references. This includes OBJS 148/155/186, DEPT 6/141/142, SIGN 5/6/83/84/103/164/171, goal 182, story 185, signed league rating at SLV_LINKGRAPH_EDGES, and PSAC 16-to-256 extension/grfid gate6/pre161 industry-airport copies and REF_STORAGE fixups. Reuse existing object, sign, objective, depot, storage and reload observations. `tools/simulation/objects.py` has a reload mode; the planned objects-definitions scenario is not yet present. Add production parsing/publication observations where saved states erase definitions, class order or partial parser behavior. Keep arbitrary-GRF, malformed-save, GUI/thread and uncovered exhaustion limits on #156.

The first coherent checkpoint completes Depot/Sign records, owned names, native inline consumers, direct Rust routes and actual pool adoption while preserving cleanup and viewport notifications. Seven other pools and ObjectSpec remain explicit until completed.
