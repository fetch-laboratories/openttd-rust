# Incremental world-state ownership

Design: [#90](https://github.com/fetch-laboratories/openttd-rust/issues/90). Implementation requires [roadmap](../roadmap.md) selection.

## Constraints in the current game

- `Map::Allocate` replaces both map arrays; mutable C++ tile references must move with their allocation. Tile indices are uint32 and type-dependent fields share bits.
- Pools select the first free slot, support indexed load allocation and ID reuse, optionally cache allocations and invoke destruction hooks. An ID is not a generation-checked lifetime guarantee.
- `StateGameLoop` orders timers, tiles, vehicles and landscape. `RunTileLoop` uses LFSR order and special handling for tile zero; sorting or parallelizing changes observations and shared random draws.
- Link graph jobs copy graph and settings; TGP's height map is temporary generator state. A general live-world snapshot would change observation semantics.

## Options and decision

Move component owners while retaining one shared-world store behind narrow operations until a selected storage port transfers it. A tick-by-tick mirror introduces two authorities and delayed mutations; transferring map and pools together requires changing references, polymorphic objects, save adapters and many callers.

Rust owns component-private state and control flow; C++ initially owns map arrays, live pools and shared services. Original bodies remain compiled only without `WITH_RUST`. #73 owns the temporary height map; #74 owns its computation snapshot, with scheduling, save/load and join in C++. Neither moves the global map or station pool. See [storage ports](../porting.md#storage-ports), [boundary](../porting.md#boundary) and [map access](../porting.md#map-access).

## Boundary safety and cost

- Resolve IDs under source lifetime rules: reacquiring an ID alone does not make reuse safe.
- A C++ object pointer may be an opaque context, never a Rust view of an STL or pool object.
- Document callback reentry and end affected world-storage access scopes before mutation, reallocation or destruction. See [boundary](../porting.md#boundary) for FFI and failure rules.
- The P2016 profile measured 349M glue calls, 79 per vehicle tick, accounting for 99% of the gap to the original (#275).

#213's post-tick contract exposes named native scalar addresses, not a Vehicle, container or bitset layout. `BaseBitSet::base_address` returns its actual `const Tstorage *`; the canonical typed vehicle service exposes front/current status addresses. Acquire after Tick survives, use at original branches under the game-state mutex, and discard before another Tick or fleet drain. Raw access ends before callbacks. Audited cargo/sound reentry cannot destroy shells, reset pools or yield the mutex. Native tests check types, address identity, source-order reads and permitted writes.

The call-scoped pool descriptor exposes the actual slot array, initialized `data.size()` and canonical `first_unused` address. After an allocating callback, read `first_unused` before a cached slot; refresh the descriptor if it exceeds captured size, since every reallocation raises it beyond the old size. Selected callbacks cannot reset the pool; descriptors end before load, reset or fleet drain. Native tests compare growth, deletion, holes and reuse with PoolIterator. Tick retains a typed native facade and controller timing. This contract authorizes no other raw field access. Acceptance uses measured crossings, bytes and CPU and wall-time gates.

#325's aircraft movement carrier exposes canonical x/y/z, speed, direction and subtype scalar addresses; standalone position needs subtype and z. Acquire after loading/order processing establishes a live shell, excluding the deleting crash path; discard before a callback that can destroy or replace it. Read afresh at source branches after position, rotor and viewport mutations. Writes and virtual operations retain canonical services. The shared vehicle layer defines each address once; native tests prove identity and fresh reads. This grants no shared Vehicle storage migration, chain view or general raw API.

#377 transfers the canonical tile bucket array and intrusive links. A stable native shell embeds a named FFI carrier exposed by a typed accessor. Bucket and link addresses remain stable until shell destruction or index reset. Under the mutex, an iterator retains its shell cursor and bounds and reads the live next link after callbacks. The current shell survives advancement; crash/flood callbacks may mutate links but cannot delete it. Reset ends cursors before load/pool reset and clears current-bucket pointers and buckets, retaining source stale links.

#275 resolves one shell for normal-update link carrier and tile; removal acquires links alone. Near filtering observes x/y and carrier together, reads next on rejection and reacquires links after visitors. Aircraft and road reuse call-local map dimensions because synchronous controller callbacks cannot reset/load the map or yield the mutex; nested entries get fresh metadata. Train, ship and other entries read dimensions per update. Road movement lazily acquires the canonical tile descriptor once per entry for track-status tile type and normal-road bits/status/road-type reads, reloading after mutations; partial fixture tables retain the typed backend. Pixel heights, aircraft, standalone movement, train, ship, YAPF and water-region entries remain callback-backed pending #565, hottest first.

## Map access decision (#108, revised 2026-10-28)

C++ retains canonical allocation, setters and field-wise save adapters until the map storage port. Rust accesses live fields through the [shared map view](../porting.md#map-access); C++ references may alias its pointers, so raw access does not permit concurrent conflicting access. Preserve observation and write order across callbacks.

`Map::Allocate` covers new games, modern and legacy loads and tests. Serialization reads live fields before asynchronous compression starts. Link-graph workers use private graph copies and map dimensions, not tile arrays; retain job join/reset ordering.

### Shared map view (#274, extended by #565)

The tile-type allocation/access descriptor has canonical base/extended pointers and count. Rust loads the C++ map statics under the game-state mutex, including `log_x`, size and native assertion policy; `map_func::World` is its sole type. Generated named ABI probes pin both layouts. See [map access](../porting.md#map-access) for field access, helper assertions and the game-loop interval proof. The view does not transfer storage.

In `tile_loop.rs`, handlers may mutate tiles and pools, run landscape, road and crash commands and queue script events (`InsertEvent`), but cannot reset, load, report progress or run a script VM. They do not change map dimensions, climate, tree placement, tick counter, editor mode, road-rebuild or ambient settings; `_misc_grf_features` changes only on reset and GRF `ActionD` loading. A batch may read these settings, including the ambient feature gate, once and retain descriptor metadata across dispatch. Ambient callbacks retain their RNG position; the tree loop reads fields after them at source observation points. Threaded and arbitrary NewGRF corpus limits remain open in #156.

#565's first maintenance batch extends the descriptor to standalone movement
track-status and pixel-Z calls (including outside-map terrain), and water-ground
restoration, flooding, tile-loop and neighbour entries, each through
`map_func::World`. Cached road-controller entries keep one `World` per entry. Pixel location, pixel slope,
maximum pixel height and normal-road status-tail callbacks are removed; the
header-named terrain helpers load canonical corner heights at each observation.
The native assertion service retains failed-precondition behavior for the
converted road and pixel helpers, without calling the native accessor. Other
helpers keep their existing fallbacks until their conversion; callbacks required
by other production consumers remain. Map allocation and native inline accessors remain
native. Train, ship, aircraft, YAPF and water-region leaves beyond these shared
movement calls remain for later #565 batches; #560 owns the complete follower.

## Transferring shared storage

Storage ports move every pool, then map arrays (user, 2026-10-28); vehicles are first (#563). Move each canonical owner, accessors, allocation/reset and destruction together. See [storage ports](../porting.md#storage-ports). A polymorphic C++ shell stays opaque to Rust and holds fields not yet migrated; each design places its shared record in or beside it. The map storage port moves the allocation without replacing C++ tile references. Pools retain ID, first-free, iteration and hook behavior; a compacting arena or generational IDs would change it. #566 measures drawing, which reads records every frame.

## Saves and evidence

Keep chunk IDs, versions, field widths and reference numbering. `src/saveload/map_sl.cpp` serializes fields rather than struct bytes; `saveload.cpp` maps references to index + 1, with zero for null and legacy load and pointer-fixup rules. Adapt the existing C++ adapters and load lifecycle when an owner moves, without changing the format. Rust-owned persistent state must be visible to saving before the mutating call returns.

See [port evidence](../porting.md#evidence). Keep desync snapshots and plain runs: rebuilding caches can conceal faults. #83's masked `round_trip_time` needs separate evidence if touched. Vehicle ports need applicable #86/#104 evidence and each controller needs branch witnesses; saves cannot expose every transient state.

## Cargo registry views (#467)

One process-wide Rust owner holds the cargo registry views; native spans alias its buffers until rebuild. Language-loader sorting may precede climate setup. Publish clears and populated views before native collation, and each town-list insertion before the special-cargo break. Rebuild/reset is serialized loading outside callbacks. Each cargo callback has its own Rust invocation and native virtual/profiler frame. This grants no additional map access.

## World-generation lifecycle (#546)

Generation state has one process-wide, static-lifetime Rust allocation. C++ aliases only the stable `_generating_world` bool subobject; both languages use one-byte, one-aligned bools and write only false or true. Startup precedes the game thread; subsequent accesses use the game-state mutex on both GUI threads. Native CompanyID backup and landscape/town/industry/object frames remain opaque native objects, with the backup's destructor/restoration contract. Native wrappers catch progress abortion, script and completion callbacks and save/load errors; resource frames outlive Rust entries. Rust chooses cleanup, storage rollback and company restoration before native mode switching can reenter generation. Map descriptors expire before progress updates; this grants no additional map access.

## Simulation-session lifecycle (#551)

Game mode, switch request and pause-bit subobjects share one stable Rust allocation. Native inline aliases begin enum/bitset lifetimes once and retain their addresses across loading and reset; the native exit flag retains atomic storage and synchronization. Native frames retain chrono/resource lifetimes and exceptions from scheduled console commands, scripts, save/load, generation and network operations that can execute scripts. Rust returns before native rethrow and CompanyID backup destruction. Storage mode changes occur at the original calls: failure before `PSM_LEAVE_GAMELOOP` leaves partial state; later script failures follow that transition. This grants no additional map-descriptor access.

## First vehicle storage port (#563)

Agent: /root/storage563 | Model: gpt-6-astra | Reasoning effort: high.

This is the selected phase 2 design; its first implementation issue is [#570](https://github.com/fetch-laboratories/openttd-rust/issues/570).

- Start implementation after #581 retires the build without Rust. Each moved record then has one definition and no C++ fallback copy.
- Use #564's metric for the next ranking round, not to rerank this selected design.
- Complete all three checkpoints: shared records and native adaptation, retirement of the data boundary, and direct Tick dispatch. A record-only scaffold is incomplete.

### Records and identities

- Keep the stable polymorphic C++ `Vehicle` shell and the native slot pool.
- Allocate a stable Rust `VehicleRecord` beside every shell, including effects and disasters.
- Define the record once with `repr(C)`, generate its C++ declaration and make it the target of typed Rust `VehicleRef`.
- Make `RustVehicle(native)` return the record and `VehicleOf(record)` read its opaque shell pointer. Neither crosses the boundary.
- Keep `Vehicle *` in native station lists, orders, GUI selections, pool slots and virtual operations. A record pointer is not a shell pointer.
- Audit reinterpret casts and legacy `void *` boundaries when changing the handle. Keep explicit shell conversion for legacy shell contexts.
- Keep native integer domains and initial values in the fields below. The listed types give storage widths; Rust uses existing transparent IDs and enums from their header modules.
- Generate C++ scalar base types and opaque pointers with typed inline conversions. Do not overlay `StrongType` or `EnumBitSet` on an integer.

| Canonical record | First fields |
| --- | --- |
| `VehicleRecord`, moved from `Vehicle` and `BaseVehicle` | `type`, `subtype`, `owner`, `direction`, `vehstatus`, `subspeed`, `acceleration`, `progress`, `tick_counter`, `running_ticks`, `day_counter`, `breakdown_ctr` as u8; `engine_type`, `cur_speed`, `cargo_cap`, `refit_cap`, `cargo_age_counter`, `last_station_visited` as u16; `tile`, `dest_tile`, `motion_counter` as u32; `x_pos`, `y_pos`, `z_pos` as i32. |
| `VehicleRecord`, moved from `VehicleCache` | `cached_max_speed` and `cached_cargo_age_period` as u16. `cached_vis_effect` stays in its existing Rust visual owner, accessed directly there. |
| `VehicleRecord`, moved from `Vehicle` | `next`, `previous`, `first` as nullable/stable record pointers. They replace the shell links, rather than cache them. `first` initially points to this record. |
| `TrainState`, existing Rust owner made shared | `flags`, `crash_anim_pos`, `wait_counter`, `cached_max_curve_speed` as u16; `compatible_railtypes`, `railtypes` as u64; `track`, `force_proceed`, `user_def_data` as u8; `cached_tilt` as bool; `cached_curve_speed_mod` as i16. Move `other_multiheaded_part` from the shell to a nullable record pointer here. |
| `GroundState` and `GroundVehicleCache`, existing Rust owners made generated | `flags` as u16; `cached_weight`, `cached_slope_resistance`, `cached_max_te`, `cached_power`, `cached_air_drag` as u32; `cached_axle_resistance`, `cached_max_track_speed`, `cached_total_length`, `first_engine`, `last_speed` as u16; `cached_veh_length` as u8. |

- Store the opaque shell pointer and stable typed links to the canonical controller, ground, order, consist, visual and occupancy owners in the record. These links identify owners; they do not copy fields or observations.
- Generate the controller link as a C union of typed pointers to the six Rust owners. Read only the arm selected by the constructor.
- Set a dispatch kind at construction for the base shell and six concrete subclasses. It identifies the former virtual Tick target, not mutable `Vehicle::type`.
- Keep the base Tick result and reread simulation `type` after a surviving Tick.
- Reuse `ConsistState` and `VehicleOrderState` for `current_order_time`, current order, order indices and shared-order links. Generate their flat C++ declarations from the existing Rust definitions; do not copy their fields into `VehicleRecord`.
- Leave `OrderList` and its containers for their own storage port.
- Reuse the existing ground and train owner allocations when exposing their layouts.
- Keep the native `TrainCache` sprite-group pointer, sprite sequences and bounds, viewport hash, cargo containers, Money/date fields and unselected vehicle fields in the shell.
- Create, initialize and destroy moved records in Rust. The shell holds the owning record pointer.
- Bind each existing type-specific owner in the derived constructor once available. Finish initialization before using the record.
- Allow slots to move, but keep live shells and records stable. Destruction invalidates both identities, even if a later shell reuses the VehicleID.
- Add no generation counter, reference count or new stale-ID rejection.

### C++ reads and writes

- Read and write the same record through inline `Record()` and typed getters/setters. Scalar arithmetic may use generated scalar members.
- Convert TileIndex, Owner, EngineID, StationID, Direction and flags in inline accessors, preserving source helpers and narrowing.
- Add no stored reference per field, shadow value or exported field getter. Diagnostic cache snapshots remain values; `cachecheck.cpp` compares and restores the same members as before.
- Adapt all native readers and writers, including infrequent ones. They remain consumers, not new owners of GUI, save/load, script or NewGRF logic.

| Consumers | Adaptation |
| --- | --- |
| `vehicle_base.h`, `vehicle_type.h`, `base_consist.h`, `ground_vehicle.hpp`, `train.h`, `roadveh.h`, `ship.h`, `aircraft.h`, `effectvehicle_base.h`, `disaster_vehicle.h` | Adapt fields, chain accessors, subtype/flag helpers, constructor defaults and owner lifetimes. `VehicleCache` keeps its native presentation facade and diagnostic copying. |
| `vehicle.cpp`, `vehicle_cmd.cpp`, the four controller `_cmd.cpp` files, `ground_vehicle.cpp`, `effectvehicle.cpp`, `disaster_vehicle.cpp`, `articulated_vehicles.cpp`, `autoreplace_cmd.cpp`, `order_cmd.cpp`, `group_cmd.cpp`, `economy.cpp`, `engine.cpp`, `company_cmd.cpp`, `roadstop.cpp`, `station_cmd.cpp`, `pbs.cpp`, rail/road/water/tunnel-bridge command and movement handlers | Access shared fields at original observations and writes; keep operations as calls. |
| Vehicle-consuming `src/rust/*services*`, `*adapter*`, `*_ffi.h`, and `src/pathfinder/**` | Convert handles explicitly and remove field callbacks. Keep canonical typed accessors for unmoved engine, cargo, road-type and station data. |
| `viewport.cpp`, `vehicle.cpp` drawing/hash code, `train_gui.cpp`, `roadveh_gui.cpp`, `aircraft_gui.cpp`, `vehicle_gui.cpp`, `depot_gui.cpp`, `build_vehicle_gui.cpp`, `smallmap_gui.cpp`, `news_gui.cpp`, `window.cpp`, `intro_gui.cpp`, vehicle lists and other `*_gui.cpp` users | Read records inline; preserve sprite, bounding-box, dirtying and picking operations. |
| `src/newgrf*.{cpp,h}`, `rust/vehicle_services_newgrf.h` | Keep native resolver contexts and shell identity; read record bytes for vehicle variables, sprite direction, livery, callbacks and consist queries. |
| `src/script/api/**`, vehicle/company/list/order queries and commands | Read fields inline; preserve validation, return widths and shell identity. Network replay and join saves use the same adapters, with no new network representation or packet field. |
| `saveload/vehicle_sl.cpp`, `saveload/oldloader_sl.cpp`, `saveload/afterload.cpp`, reference conversions in `saveload/saveload.cpp`, `cachecheck.cpp`, `src/tests/**`, `tools/simulation/*-console.cpp` | Adapt field addresses and fixups, including fixtures that assumed a Rust handle was a shell address. |

- Build platform-specific consumers; textual search cannot certify their types.
- Extend `layouts!` and `tools/generate-abi.py` so one typed field list generates the Rust struct, ordered field metadata and C++ members.
- Support only selected scalar/newtype, raw-pointer, record and tagged-owner-union forms. Generated headers are build outputs, not a second checked-in schema.
- Pin size, alignment, each field offset and width, including union arms, on every target. Use named ABI probes on GCC, Clang, MSVC and Emscripten; do not assume LP64 padding or class-return ABI.
- Pass IDs as scalar base widths and keep Rust IDs as transparent newtypes. Ordinary CI must fail if a cross-language declaration or probe is missing.

### Links, pool and lifetime

- Return `record_link->shell` inline to C++ from `Next`, `Previous` and `First`; return the record link to Rust.
- Port `Vehicle::SetNext` as one Rust operation. Keep its self-link assertion, old-suffix first-pointer rewrite, old previous clear, predecessor detach and new-suffix first-pointer rewrite in that order. Native calls use a thin typed facade; a raw `next` store is not a substitute.
- Port `Last`, `Move` and articulated/dualhead traversal helpers as named callees. Read next links afresh after callbacks; do not prefetch a chain into a vector.
- Keep `VehiclePool` as `Pool<Vehicle, VehicleID, 512>`: 32-bit IDs, `End` and maximum size 0xFF000, invalid 0xFFFFF, and ascending slot iteration.
- Allocate the lowest free bitmap bit at or after the native `first_free` word. Normal allocation advances `first_free`; indexed load allocation does not. Deletion does not lower the `first_unused` high-water mark.
- Keep the order of `CanAllocateItem` assertion credit, publication before construction, null holes and indexed-load range/duplicate errors. Do not add a dense arena or mirrored ID directory.
- Keep the live slot descriptor canonical. Slots contain native shells: resolve each shell once through the typed owner operation, or expose a generated slot view only when the slot owner moves. Do not reinterpret slots as record pointers.
- Reread `first_unused` after an allocating callback and refresh the descriptor before accessing a slot beyond its captured extent. The cursor cannot reset or yield; fleet drain, load and reset end cursors and record access scopes.
- Destroy in order: derived `PreDestructor` while its private owner is live, derived members, `Vehicle::~Vehicle`, base members, `Pool::FreeItem`.
- Keep the common record alive through the base destructor, including recursive next deletion, occupancy removal, viewport-hash removal and news cleanup. Do not access derived owner links after their original destruction point.
- During cleanup, set `cleaning` before ascending shell deletions. `PreDestructor` skips normal work, cargo receives `OnCleanPool`, free clears slots and counts, and bitmap clearing and `PostDestructor` are skipped.
- Outside cleanup, free lowers `first_free`, clears the bit, then calls `PostDestructor`. Keep the pool default `Tcache=false`.
- A later selected vehicle-pool port moves slots, bitmap and operations together. C++ retains a `PoolBase` registration facade, inline slot reads and hooks. This first port neither completes that work nor claims its savings.
- Coordinate the remaining base-lifecycle candidate with record release and `SetNext`; do not create another owner.

### Save/load

- Keep the `VEHS` chunk, field names, compatibility tables, version gates, array indices and reference numbering. Do not serialize whole records or bump the version.
- Use named `SlVehicleCommon` descriptors to address record fields without changing file names, such as naming `Record().x_pos` as `x_pos`.

| Fields | File representation |
| --- | --- |
| Tile, destination, x and y | U16 conversion before `SLV_6`, U32 after; x and y keep their signed in-memory bit interpretation. |
| Z | U8 to I32 before `SLV_164`, I32 after. |
| Direction, type/subtype, status, fractional speed, acceleration, progress, tick/day/running | Keep existing byte widths. |
| Engine, speed, cargo/refit capacity, cargo-age counter, station fields | Keep U16 and applicable `SLV_5`, `SLV_88`, `SLV_162`, `SLV_182` gates. |
| Motion | Keep U32 and `SLV_VEH_MOTION_COUNTER`. |
| Ground flags | Keep U16 and `SLV_139`. |
| Train flags | Keep U8 to U16 from `SLV_2` to `SLV_100`, then U16. |
| Crash/wait, force/track | Keep crash and wait U16, force and track U8; wait retains `SLV_136`. |

- Remove `TrainStateScope` and copied scalar staging. Named native descriptors address shared `TrainState` directly. Preserve partial writes on load errors, NULL cleanup and pointer-fixup order.
- Leave road/ship path staging and other unselected save adapters with their existing owners.
- Add internal `REF_VEHICLE_RECORD` and `REF_VEHICLE_RECORD_OLD` adapters for `next`. Use vehicle references' `SlSaveLoadRef` encoding and old 4.3/4.4 sentinel branch; resolve IDs through the native pool to records. These are not new serialized chunk types.
- Convert record references with `record->shell->index + 1`; modern null stays zero. End the loader's temporary integer-in-pointer phase before Rust follows links.
- Convert old-loader next assignments at their existing fixup points. Keep failures in C++, without a Rust frame spanning a throwing loader.
- Keep `previous`, `first`, `other_multiheaded_part`, caches, shell pointers and dispatch metadata NOSAVE; rebuild them at the original phase. Preserve sequential versus bracket multihead matching in `ConnectMultiheadedTrains`.

### Retiring the data boundary

- Remove every affected binding, native body, Rust table slot and ABI entry, not just the hot caller. Other ports adopt moved fields through the shared record in the same change.
- Delete 44 `TrainRead*` functions in `train_services.h` (219 body lines), 29 `OpenTTDTrain*Read` carriers in `train_ffi.h` (148 lines), 56 `OpenTTDTrainServices::read_*` slots and their 56 native bindings. These include `TrainReadNext`, `TrainReadMoveVehicle`, `TrainReadTickState`, `TrainReadLoco26`, `TrainReadStopLocation` and consist/collision/reverse readers. Use existing order/ground/visual owners directly, not replacement snapshots.
- Delete train write slots and bodies: `write_tile`, `write_dest`, `write_x`, `write_y`, `write_z`, `write_direction`, `write_speed`, `write_tick`, `write_running`, `write_day`, `write_order_time`, `write_progress`, `write_subspeed`, `write_gv_flags`, `write_acceleration`, `write_length`, `write_total_length`, `write_first_engine`, `write_max_speed`, `write_cargo_cap`, `write_refit_cap`, `write_cargo_age`, `write_last_station`, `write_status`.
- Keep `write_colourmap`, `write_crossing_bar` and `write_visit_type` for unmoved records only while those slots have production callers.
- Delete `TrainReservationRead{Free,Free1,New,Choose,Choose4,CheckNext}`, their carriers and slots, and reservation writes for moved destination, station and ground flags. Order-list queries keep their canonical owner; #560 and #565 own follower and map work.
- Delete the 21 `openttd_rust_train_state_get_*` and `set_*` exports/declarations and `TrainStateScope`. Make C++ Train getters inline; keep allocation and destruction as typed calls. Generate ground/consist/order records from current Rust layouts without another field inventory.
- Delete shared vehicle slots `write_counters`, `write_xy`, `write_day`, `write_direction`, `write_speed`, `last_station`, `write_last_station`, `write_dest`, `write_progress`, `write_max_speed`, `previous_tile`, `next`, `engine_id`, `write_tick`, `direction`, `status`, `tile`, `progress`, `dest_tile`, `subtype`, `x`, `y`, `z`, `tick`, `owner`, `speed`, `subspeed`, `running`, `order_time`, `vehicle_type`, `write_x`, `write_y`, `write_z`, `write_tile`, `write_status`, `write_subspeed`, `write_breakdown_counter`, `write_running`, `write_order_time`, `write_cargo_age`, `write_coordinates`, `coordinates`, `xy`, `speed_inputs`, `direction_progress`, `tick_status`, `altitude_inputs`, `position_observations`, `movement_observations`, `max_speed`, `occupancy_update`, `occupancy_near`.
- Delete matching `VehicleSvc*` and `RustVehicle*` field wrappers and unused signature adapters. Delete `OpenTTDVehicle{PreviousTile,Coordinates,XY,SpeedInputs,DirectionProgress,TickStatus,AltitudeInputs,PositionObservations,MovementObservations}` and `OpenTTDOccupancy{Update,Near}Observation` carriers.
- Keep `SetDestTile` as the type's setter, which can clear its path; keep viewport dirtying as an operation. Replace `GroundSvcUpdateViewportThenNext` with the viewport operation and live Rust next/progress reads; delete `OpenTTDGroundViewportNext`.
- Keep `CheckOrders` as an operation in `VehicleSvcCheckOrders` and read running afterward in Rust. Delete `OpenTTDVehicleDayCost`; retain the native profit field service and read running in place.
- Remove `OpenTTDRoad` observation wrappers/carriers `TickRead`, `MaxSpeedRead`, `SpeedRead`, `SpeedPart`, `Link`, `LinkTile`, `Step`, `CrashPart`, `Entered`, `Candidate`, `PartRead`, `PathRead`, `StopRead`, `OvertakeRead`, `ArrivalRead`, `SoundRead`, `ServiceRead`, `TurnRead`, `TrackdirRead`, and corresponding `RoadSvc*`/`Road*Read` definitions. Read common/ground/order fields directly; use canonical typed road-type, engine, station and settings accessors for unmoved inputs. Keep operation results such as `RoadTrackChoice` and `RoadDepotResult`.
- Remove moved fields and carriers from mixed aircraft/effect/disaster/ship observations. Keep operation results and canonical access to unmoved records.
- Retire `OpenTTDVehiclePostTick` (12 declaration lines), `VehicleSchedulerTick` (13 body lines), its callback/binding, `RustRoadTickRead` (11 body lines) and four compiled native Tick facades in the dispatch checkpoint.
- Treat the 64 declaration lines in named common observation carriers, PostTick and GroundViewportNext as source spans, not #564 code-line metrics or complete vehicle-header retirement. Report the actual whole-fork metric after consumers and generated declarations exist.

### Direct Tick dispatch and evidence

- Match constructor dispatch kind in the Rust scheduler and call train, road, ship, aircraft, effect and disaster Rust Tick owners directly. Base Tick returns true.
- On effect expiry, end the controller borrow and call the canonical native delete operation; remove the `RunEffect` trampoline.
- Delete native Tick bodies and declarations without production callers. Fixtures must not keep orphan production exports alive.
- Bind immutable controller service tables once at scheduler entry, preserving initialization dependencies.
- Keep timing for every road and ship Tick, train front engines only, and normal aircraft only after the early gate.
- Have a caller-owned native frame provide placement storage for `PerformanceAccumulator` and `OpenTTDRoadCommandSlot`. Typed begin/end operations construct and destroy them without calling Rust; Rust runs the controller between them.
- Create road scratch lazily, reset and destroy it each road Tick, and never share it across nested entries. Finish timing and scratch destruction before the survival assertion. Preserve presentation timing without an allocation or a timer wrapper that calls back into Rust.
- End field borrows before operations. After failure, access no dead record; after survival, reload type, first record and fields at source post-Tick branches.
- Treat P2016 credits as estimates: declarations/native adaptation 0; direct train reads about 52.8M Ir (0.391%); dispatch/post-Tick about 63.4M Ir (0.469%); disjoint total about 116.2M Ir (0.860%). The implementation issue records profile counts, replacement allowances and uncertainty.
- Do not add #275's wider opportunities again: 133M for dispatch and about 120M for train reads. Claim no viewport, #560 follower, #565 map, #555 repair or drawing savings. Allocation locality, register allocation and native drawing cost remain unmeasured.
- Extend vehicle/fleet/rail/road/ship/aircraft/effect/disaster scenarios and native fixtures for ordinary and indexed allocation, holes/growth/reuse during Tick, deletion of current and unvisited chain members, SetNext and multihead order, fresh fields after callbacks, writes from both languages, reload/cache rebuilding, timing gates and the survival assertion.
- Move synthetic native virtual-Tick tests to the Rust traversal kernel with test-supplied work. Test production dispatch with real shells; do not retain an unused vtable trampoline or change layouts under `cfg(test)`.
- Keep semantic decoders and both RNG streams unchanged. Use native ABI/lifetime checks for transient pointers and old-reference-width and invalid-reference fixtures for the save adapter.
- Consume #560's integrated follower and `GetPlatformLength` interfaces without claiming its credit. #562 owns station lifecycle and native loading-list removal; preserve stable shell pointers and PreDestructor reentry.
- Coordinate overlapping train/vehicle service files at final base update without an unmerged service dependency. Root sequences this implementation with the queued base vehicle lifecycle owner.
- Root must obtain #566's reference/candidate image and instruction comparison before calling this port drawing-validated or integrating it. Implementation may proceed while #566 is underway; `-vnull` play cannot substitute. #566 is selected but has no integrated workload/result at this design checkpoint; the merged-tip profile will measure savings.
