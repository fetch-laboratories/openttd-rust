# Vehicles

<!-- Add a new entry at the end of this file. -->

## Effect vehicles

- Rust owns: `effectvehicle.cpp`'s init and tick procedures and `_effect_procs` (`effect.rs`), `EffectVehicle::Tick` with the `delete v` of an expired effect, the three creators, which Rust callers call typed, and each shell's `animation_state` and `animation_substate`, reached through the vehicle record's `effect` link.
- C++ keeps: the shells, their pool allocation, save/load staging, drawing (`UpdateDeltaXY`, `GetTransparencyOption`), and the shared vehicle table's sprite writers, viewport and sound, which read or write members the shells store. `GetIndustryGfx` is the shared table's `industry_gfx` until its translation is ported (#565).
- Checks: `python3 tools/migration.py simulate effects`.
- Limits: Gaps remain in viewport pixels, audible output or full legacy fixtures.
- PRs: #105, #115, #300, #1044; service table #299.

## Disaster scheduling and vehicles

- Rust owns: every simulation body of `disaster_vehicle.cpp` (`disaster.rs`) under its original name, the constructor's body, `_disaster_images`, `_disastervehicle_tick_procs`, `_disasters` and `_disaster_delay`; the members `image_override`, `big_ufo_destroyer_target`, `flags` and `state`, reached through the vehicle record's `disaster` link; `GetDirectionTowards` (`vehicle.rs`); and the road owner's `disaster_vehicle`. `DisasterClearSquare` runs the Rust `CMD_LANDSCAPE_CLEAR` with the company backup, and `ReleaseDisastersTargetingIndustry` is called from the industry destructor's Rust body.
- C++ keeps: the `DisasterVehicle` shell with its save/load constructor, which the table's `allocate` calls, `ReleaseDisasterVehicle`'s `GetIfValid` lookup, `UpdateDeltaXY`, the daily `IntervalTimer` registration, `_disaster_delay`'s save address, and the pools, stations, industries, towns and company road counts the bodies reach through shared tables (#656, #634, #635, #1048).
- Checks: the `disasters-` scenarios, the ABI probe's disaster case and the Rust tests of `disaster.rs` and `vehicle.rs`.
- Limits: Gaps remain in legacy save fixtures, viewport pixels, sound or all NewGRF combinations.
- PRs: #118, #300, #520; file pass #1027 part 4.

## Road vehicle control and private state

- Rust owns: tick/day control, movement, blocking/overtaking, reversal, station/depot transitions, crash expiry, servicing, cache, seven scalars and path per shell; road YAPF (#124) writes that path. The seven scalars form the canonical generated shared prefix and both languages access them directly.
- C++ keeps: shells/pool identity, modern, split-vector and TTD/TTO save staging and counters, and the road command slot of the vehicle tick, which the session frame holds.
- Checks: default `roads-` simulation, original movement-table observations in Rust, and native shared-field layout/default/maximum-value checks.
- Limits: Gaps remain in legacy saves, custom NewGRFs, articulated/tram turns, sounds, and viewport pixels (#156).
- PRs: #128, #215, #319, #332; direct fields #744/#755.

## Ship controller and private state (#146)

- Rust owns: every simulation body of `ship_cmd.cpp`, all but the drawing and sound members (tick/movement, locks, acceleration, rotation/reversal, track/depot/destination policy, economy-day/service/cache, `CmdBuildShip`, nearest-depot region BFS, `GetVehicleTrackdir`, `IsShipDestinationTile`, `GetEffectiveWaterClass`), the ship owner (`state`, `rotation`, rotation coordinates and cached path) reached through the vehicle record's `ship` link, and the record's sprite-cache direction; ship YAPF (#119) owns search.
- C++ keeps: the `Ship` shell and its virtual members as one-call shims, the specialised `UpdateViewport` with the drawing members (#656), the depot pool walk (#638), sprites, sound, `UpdateDeltaXY` and modern/legacy save staging.
- Checks: `python3 tools/migration.py simulate water --jobs 2`; the ABI probe's ship case runs the real reverse and depot searches.
- Limits: Gaps remain in legacy saves, viewport pixels or arbitrary custom NewGRF combinations.
- PRs: #152; file pass #1059.

## Rail vehicles and controller state (#130)

- Rust owns: consist/cache/curve/speed policy, both tick passes, movement, reversal, crossing control, crash/deletion, servicing, day handling, reservation extension/rollback and temporary order lookahead; rail YAPF (#122) owns search.
- C++ keeps: train shells, pools, map and shared services.
- Checks: default rail simulation and native checks.
- Limits: Gaps remain in arbitrary NewGRFs, legacy saves, and unexercised controller branches (#156).
- PRs: #143, #195, #345.

## Aircraft controllers and airport movement

- Rust owns: both tick passes, movement/speed/flight, shadow/rotor, FTA traversal, terminal/helipad reservations, service/range, crash cleanup, airport lifecycle, private counters/cache and airport masks.
- C++ keeps: save aliases; the station and `AirportFTAClass` records, read through one leaf per field; vehicle/settings/RNG, viewport, NewGRF, sound, news, script-event and crash services. Rust owns `RotateAirportMovingData` (`airport.rs`) and sends the vehicle to a depot through the Rust command.
- Checks: default `aircraft` and `disasters` scenarios.
- Limits: Gaps remain in real NewGRF/refit/airport rotations, threaded GUI, legacy-save fixtures, and presentation (#156).
- PRs: #145, #238, #326.

## Vehicle scheduling and base maintenance (#213)

- Rust owns: calendar/economy-day scheduling, `CallVehicleTicks` traversal, survival, cargo aging, motion/sound, loading/fleet sequencing, renewal, service/depot resets, reliability, breakdowns, age and value.
- C++ keeps: `Vehicle` shells/pools, NewGRF callbacks, commands and presentation.
- Checks: default vehicle, fleet and aircraft simulations.
- Limits: Gaps remain in replacement and old-news branches, NewGRF and reentrant sound, custom/articulated combinations, and legacy saves (#156).
- PRs: #257.

## Fleet grouping and replacement lifecycle (#147)

- Rust owns: group metadata/names/children, statistics, engine counts, renewal rules/links, company renewal heads, pending tick-end map, group commands/traversal/livery, replacement/refit, build, rearrangement, rollback and execution.
- C++ keeps: pool identities, group numbers, GUI folding, save adapters, `Vehicle::group_id` and command costs.
- Checks: default `fleet-` simulation.
- Limits: Gaps remain in NewGRF refit/attach callbacks and articulated parts (#156).
- PRs: #197.

## Common vehicle transactions and nonrail construction (#314)

- Rust owns: `vehicle_cmd.cpp` build, sell, refit, clone, start/stop, mass start/stop, depot sell/replace, send-to-depot, naming and service interval commands; vehicle selection and depot/sort lists; road/aircraft construction, shadow/rotor and articulated assembly/check. Speculative refits restore both RNG streams; failed clones sell partial consists.
- C++ keeps: pools, shells, field stores, specs/NewGRF callbacks, unit allocation, command costs and UI; rail transactions belong to #315.
- Checks: default `fleet-` scenarios, including `transaction-build-road`, `transaction-build-aircraft` and `transaction-clone`.
- Limits: Gaps remain in articulated road vehicles (NewGRF only), wrong-depot road types, airport engine refusal, and clone build failure after front part (#156).
- PRs: #329.

## Rail construction and depot consist transactions (#315)

- Rust owns: `train_cmd.cpp` from `GetFreeWagonsInDepot` through `CmdSellRailWagon`: construction, joining/normalization, backup/restore, arrangement, dual-head/subtype normalization, validation, head promotion/demotion and rail sale.
- C++ keeps: shell allocation/field stores, unit-ID pools and presentation. A validation error leaves the chain split until the caller restores it.
- Checks: `python3 tools/migration.py simulate fleet-transaction-rail --jobs 2`.
- Limits: Gaps remain in NewGRF attach-callback results, GRF-version branches, and order-pool exhaustion (#156).
- PRs: #360.

## Engine lifecycle and availability (#220)

- Rust owns: engine lifecycle scalars/names, aging-stop state, startup/reliability, daily/monthly preview and availability, variant synchronization and control/name/visibility commands.
- C++ keeps: engine shells, pools, company masks, presentation and save/load errors.
- Checks: `python3 tools/migration.py simulate engines- --jobs 2`; identical adapter supplies variant and cutoff spec inputs to unchanged reference bodies.
- Limits: Gaps remain in external NewGRF programs/`retire_early`, ENGS and TTO/TTD fixtures, legacy preview conversion, and threaded GUI/presentation (#156).
- PRs: #248.

## Engine definition loading (#470)

- Rust owns: sorted engine identity mappings, NewGRF overrides, setup/allocation, temporary records/rail labels, refit masks, variant finalization and road/train binding; vehicle Action0 extends this owner (#491).
- C++ keeps: specs, shells, pools, GRF files, Action0 framing, serialization/errors, badge/livery stores and presentation.
- Limits: Gaps remain in historical mappings, pool exhaustion, malformed parser sequences, arbitrary GRFs, and GUI/network execution (#156).
- PRs: #474.

## Vehicle properties and deferred purchase order (#491)

- Rust owns: vehicle Action0 property loops, interpretation and base-price conversion; temporary engine records, cargo masks, rail labels and deferred purchase-order queue.
- C++ keeps: specs, shells/pool, file/cargo identity, sound/image validation, badge effects, checked reader, `EnginePreSort`, unstable sort, Action0/EIDS frames and Action3 consumers.
- Limits: Gaps remain in historical EIDS, pool exhaustion, arbitrary GRFs, malformed and property/climate combinations, graphics/audio, and GUI/network scheduling (#156).
- PRs: #495.

## Engine cargo and articulated query runtime (#439)

- Rust owns: cargo eligibility, capacity/refittability and articulated part, capacity, mask and live-chain queries.
- C++ keeps: specs, dynamic identity mapping, cargo definitions and shell allocation.
- Checks: default fleet capability/reload scenarios with real Action2 and build/refit/clone/replacement; same articulated chain inspected across save/load after transient graph reattachment.
- Limits: Gaps remain in dynamic mappings, v7 mirror output, zero/failing refit-capacity, part-only refittability, external GRFs, pool exhaustion, historical saves, and GUI scheduling (#156).
- PRs: #446.

## Orders lifecycle ownership

- Rust owns: order storage/links/backups, consist timetable/unbunching, traversal, edits, sharing, depot execution and backup lifecycle.
- C++ keeps: pool identity, vehicle shells, command execution, VM callbacks and save/load adapters.
- Checks: default orders simulation and existing tests.
- Limits: Gaps remain in NewGRF callback variants and legacy saves (#156).
- PRs: #176.

## Ship YAPF searches and canonical path cache

- Rust owns: region/track searches, search arenas, heap, estimates, corridor, attempts, lost paths, cache extraction and per-ship path; the ship, engine and settings reads, map reads, dock origins and water region calls.
- C++ keeps: ship/station pools and save staging; the controller calls the searches in Rust with the ship's own path.
- Checks: `python3 tools/migration.py simulate water --jobs 2`.
- Limits: Gaps remain in arbitrary maps, NewGRFs or complete legacy saves.
- PRs: #126, #308; scratch capacity #306; file pass #977.

## Rail YAPF search, caches and reservation

- Rust owns: four searches, node/segment arenas, heap, costs, lookahead, limits, reconstruction, six cache banks, rail-change invalidation, reservation traversal and signal rollback; controller reservation extension is #130.
- C++ keeps: world services and station triggers.
- Checks: default `padhattan` and `rail-reservation` simulations under both 90-degree policies; native unchanged-base/heap probes.
- Limits: The checks do not prove full rail-cost semantics for arbitrary maps, NewGRFs, legacy saves or every signal family.
- PRs: #133.

## Road YAPF search and path construction

- Rust owns: track/depot searches, exit keys, nodes/maps/heap, traversal/cost, heuristic, limits, fallback, reconstruction and station-area trimming into the road controller path.
- C++ keeps: map/pools/settings and station/stop observations.
- Checks: `python3 tools/migration.py simulate roads --jobs 2`; the original heap transcript in production Rust tests and retained native depot transcripts.
- Limits: Gaps remain in arbitrary maps, NewGRFs, articulated/tram, waypoints, stop/loop/segment branches, and legacy saves (#156).
- PRs: #135, #279, #289; scratch capacity #278; heap evidence #744/#755.

## Vehicle visual emission, cache and effect creation (#466)

- Rust owns: `Vehicle::UpdateVisualEffect`, `SpawnAdvancedVisualEffect`, `IsBridgeAboveVehicle` and `Vehicle::ShowVisualEffect` (`vehicle_effect.rs`, finished), each taking the vehicle and shared tables, with `Vehicle::IsPrimaryVehicle` over the record's `subtype`. The road, train and ship controllers call the bodies directly and pass their own `GetCurrentMaxSpeed`, which runs after the gate. The effect procedures and the three creation variants are `effect.rs`'s.
- C++ keeps: shell/pool allocation, `vcache` behind the record's `visual` link, parsed specs, `IsDepotTile` (a shared slot, since `IsHangarTile` reads the airport specification), the NewGRF error message, rendering, viewport and sound. `openttd_rust_visual_update` and `openttd_rust_visual_show` remain only for `tools/simulation/fleet-console.cpp` (#656).
- Checks: `effects-emission`, `effects-emission-reload` and `effects-emission-train` through the fleet harness; the ABI road probe (`road.inc`) for emission's call order inside a road tick; `vehicle_effect::tests` for the gate's single maximum-speed query and the power-change error; `vehicle_base::record_tests` for `IsPrimaryVehicle`.
- Limits: Gaps remain in external GRFs, pool exhaustion, historical saves, audible output, and GUI scheduling (#156).
- PRs: #468; file pass #1051 (#1027 part 2).

## Tile movement dispatch (#371)

- Rust owns: tile track status, admission, pixel height, foundation dispatch and tile policy, rail/road foundations, depot distances, portal frames and bridge entry preparation.
- C++ keeps: map arrays, pools, vehicle shells, storage/setters and native callers. Road controllers use the shared map descriptor for track-status tile reads; other movement entries and water-subtype reads use typed services.
- Checks: `python3 tools/migration.py rust-checks`, `build`, native ABI tests, rail/road/station scenarios and the `movement::evidence_tests` Rust tests. Frozen original foundation/extreme-coordinate observations run against production Rust; the scheduler and mutation/reset observations run in `tile_loop::comparison_tests`.
- Limits: Gaps remain in NewGRF foundations, train/ship bridges, road tunnels, partial-depot reversal timing, half-water, and extreme bridge-speed branches (#156).
- PRs: #374, #517, #523; movement evidence #748/#755.

## Vehicle station/depot visits and route-link prediction (#349)

- Rust owns: `BeginLoading`, `HandleLoading`, `LeaveStation`, `CancelReservation`, `ResetRefitCaps`, `CalcPercentVehicleFilled`, `VehicleEnterDepot`, `LinkRefresher` and both `IncreaseStats` overloads (`vehicle/visits.rs`, finished), with `RefreshFlags` in `linkgraph/refresh.rs`. Each body reads vehicle members at the original's points through the `VehicleRef` accessor named after each, orders in place through `OrderView`, and calls the shared window, news, script-event, cost-animation, assertion and command-cost services; `PlayLeaveStationSound` is one vehicle-table slot for every caller.
- C++ keeps: link-graph pools and allocation, station/vehicle shells, the shell members the visit table reads and writes (`last_loading_station`, `last_loading_tick`, `trip_occupancy`), `HideFillingPercent`, `GoodsEntry::GetOrCreateData`, NewGRF triggers and the `CommandCost` storage of the depot refit.
- Checks: `orders-visits` (padhattan-ridge-2000, cargodist) and station scenarios; `vehicle::visits::tests` for the refit stores, the leave-station occupancy, the depot halt and refit news and events, the road depot window order, the percent-filled assertion and the unreachable vehicle type.
- Limits: The checks do not cover link-graph pool exhaustion or a depot order that reaches the refit test without a refit (#156).
- PRs: #359; visits dispatch #555; file pass #987.

## Common ground-vehicle dynamics and canonical cache (#372)

- Rust owns: `GroundVehicle<T, Type>` (`ground_vehicle.rs`) instantiated for `Train` and `RoadVehicle` with their per-type acceleration methods: power and cargo cache rebuilds, acceleration, depot checks, crash slope flags, inclination/Z updates and speed accumulation. Each entry resolves its owners from the shared vehicle record.
- C++ keeps: shell/pool identity, rail-type and road-type info, cargo specs, slope-steepness settings, NewGRF properties, viewport and the base crash leaf.
- Checks: default road, rail and fleet scenarios; fleet console against unchanged reference bodies; `ground_vehicle::tests` for the native assertions and owner resolution.
- Limits: Gaps remain in external powered/articulated NewGRFs, maglev traffic, legacy saves, and threaded GUI (#156).
- PRs: #375; file pass #963.

## Vehicle tile occupancy and live spatial queries (#377)

- Rust owns: tile buckets, on-tile/near cursors, hash update/reset and ground/track/tunnel occupancy checks.
- C++ keeps: pools, IDs, viewport indexing and command costs. Lifetime rules: [world state](../design/world-state.md).
- Checks: default road/crash/flood/reload, train collision, road-stop and fleet scenarios; fleet console against unchanged public iterator bodies.
- Limits: Gaps remain in GUI and external articulated GRFs (#156).
- PRs: #384, integrated in #391.

## Original vehicle-list and fleet selection vocabulary (#734)

- `VehicleListType` retains all raw byte values, original list kinds and the end marker. Existing scalar adapters remain available. Private list generation and group insertion use the original domain names and explanatory comments.
- Both list-generation callers, unsupported-kind behavior, shared-order traversal, output clearing and callback order are preserved. Native identifier packing, vehicle/group ownership, storage and public scalar interfaces remain unchanged.
- Checks: original operations/order source review and combined exact-head Full. Archive equivalence is unclaimed; inherited fleet and mass-command coverage limits remain on #156.
- PRs: #760, integrated in #763. Remaining Orders/fleet vocabulary stays open.

## Original order location and distance vocabulary (#734)

- The existing location and order-distance helpers and their immediate callers use the original names, canonical IDs/coordinates and explanatory comments. Conditional traversal, recursive depth, signed casts, saved list-span handling, live lookups and aircraft-range caller order remain unchanged.
- Native order/vector storage, scalar interfaces and services stay with their existing owners. Checks are original operations/order review and combined exact-head Full; archive equivalence is unclaimed and inherited oversized-list, branch and historical-save limits remain on #156.
- PRs: #766, integrated in #769. Remaining Orders/fleet cleanup stays open.

## Original vehicle header vocabulary (#734)

- Original vehicle-state elements/sets, ground subtypes, NewGRF cache-valid bits, visual-effect models and custom-sprite values live in their header modules. Raw compatibility constants delegate to the canonical definitions; transparent carriers retain stored widths and unnamed bits.
- Local predicates, state updates, constructor sentinels and station-use helpers adopt those names without changing field layouts, scalar exports, service contracts, native packing or effect order. All generated ABI inputs remain unchanged.
- Checks: original operations/order review and exact-head Full. Archive equivalence is unclaimed; inherited ship-depot assertion and coverage limits remain on #156. Private-field/operator closure stays later.
- PRs: #771, integrated in #773. Other Vehicle workset files remain pending.

## Vehicle record and station-use file pass (#983)

- `vehicle_base.rs` and `vehicle_func.rs` are finished. The shared record's fields and `VehicleRef`'s readers and writers take the original members' names and types; `_raw` adapters serve unfinished files only. `VehicleRef::current_order` reads in place through `orders::OrderView`.
- `Vehicle::SetNext`'s self-link assertion and `GetNextArticulatedPart`'s precondition run in Rust through the native assertion policy; `GetNextVehicle`, `GetLastEnginePart` and `Move` follow their originals. C++ keeps the record allocation exports for the shell and the `SetNext` facade.
- Checks: `vehicle_base::record_tests`, `orders::order_view_tests`, source review and exact-head Full. Archive equivalence is unclaimed: signatures change.
- PRs: #996. Workset 07 continues in #984 to #987.

## Base vehicle maintenance file pass (#985)

- `vehicle/maintenance.rs` is finished. Its nine bodies read each record member in place, each member the shell still stores through its `VehicleRef` accessor, the breakdown, servicing and old-vehicle settings through the shared settings addresses, and the dates, game mode and clock from Rust, where the original reads them. The maintenance table holds its five tables, `Vehicle::MarkDirty` and the shell members' slots; no per-call composite record crosses.
- `EngineReplacementForCompany` and `EngineHasReplacementForCompany` are ported in `autoreplace_func.rs`; `GroupStatistics::VehicleReachedMinAge`, `Vehicle::HasDepotOrder`, `GetAvailableMoney`, the articulated refit and cargo queries and `Engine::GetCost` have Rust-internal forms the bodies call. `Vehicle::NeedsAutorenewing`'s owner assertion runs in Rust through the native assertion policy.
- C++ keeps the seven facades of `vehicle.cpp`'s maintenance functions, `PlayVehicleSound` and `SndPlayVehicleFx`, and the shell members until the vehicle storage port.
- Checks: `vehicle::maintenance::tests`, `autoreplace_func::tests`, the `windows-abi` maintenance, road, ship and aircraft probes, `vehicle_scheduler`'s native scalar test, the `vehicles-*` maintenance scenarios and exact-head Full. Archive equivalence is unclaimed: every body changes how it reaches its values.

## Vehicle scheduler file pass (#986)

- `vehicle/scheduler.rs` is finished. `RunVehicleCalendarDayProc`, `RunEconomyVehicleDayProc` and `CallVehicleTicks` read the vehicle pool through its descriptor and each member through its `VehicleRef` accessor, where the original reads it; the survival assertions and the three subtype casts run through the native assertion policy. Each day procedure reads the game mode and day fraction itself and dispatches `OnNewCalendarDay` and `OnNewEconomyDay` on the vehicle's type to the controllers' bodies, so the eight C++ overrides and the two base virtuals are gone. `CallVehicleTicks` dispatches `Tick` the same way to all six owners' bodies; the four controllers (train, road vehicle, ship and aircraft) each open their own `PerformanceAccumulator` where the original does, and effect and disaster ticks have none; `SchedulerServices` links the four controller tables and the disaster table.
- The vehicle's cargo list has one binding, `VehicleRef::cargo` over the resolver's slot. C++ keeps `PlayVehicleSound`, `MarkDirty` and the pool and shells.
- Checks: `vehicle::scheduler::tests` (post-Tick bodies over live records, the pool walk against `scheduler/pool-oracle.txt`, survival assertions, day procedures, the 32-day callback's results, the day dispatch and the `Tick` dispatch with its timing scopes), each controller's timing-scope test, the native session-tick case over a pooled vehicle, `RoadServiceBoundary`, source review and exact-head Full. Archive equivalence is unclaimed: every body changes how it reaches its values.
- PRs: #1045. `vehicle/occupancy.rs` follows in #1003.

## Vehicle tile hash file pass (#1003)

- `vehicle/occupancy.rs` is finished. The tile hash links `hash_tile_next`, `hash_tile_prev` and `hash_tile_current` are members of `VehicleRecord`, read and written in place; `UpdateVehicleTileHash`, the tile part of `ResetVehicleHash` and both iterators follow the originals member for member, and `Increment` keeps its assertion.
- `EnsureNoVehicleOnGround`, `TunnelBridgeIsFree` and `EnsureNoTrainOnTrackBits` are Rust functions that return the type of the vehicle in the way; `CommandServices::no_vehicle`, `no_train` and `wormhole_free` write it into a command's cost as `STR_ERROR_TRAIN_IN_THE_WAY` plus the type. Callers that test only success, such as the road tile loop and `DisasterClearSquare`, build no native cost.
- C++ keeps the iterator classes `VehiclesOnTile` and `VehiclesNearTileXY`, the functions `EnsureNoVehicleOnGround` and `TunnelBridgeIsFree` for the train GUI and the harness consoles (#736), and the viewport hash.
- Checks: `vehicle::occupancy::index_tests`, the train swap oracle, the aircraft, ship and disaster ABI fixtures, `vehicle_scheduler`'s copied-cursor test, the six `occupancy.*` branch witnesses and the fleet console's native probes, and exact-head Full. A native case observes the cost `error` callback receiving all 32 bits of a string; it retires with `CommandCostServices`.
- PRs: this pass.

## Original renewal and servicing vocabulary (#734)

- Renewal, servicing and automatic-servicing bodies use the original helper names, canonical vehicle states, subtype flags, engine/cargo identities and economy-month constant. Scalar maintenance observations and record layouts remain unchanged.
- Age and interval comparisons, live engine/company/refit reads, per-unit traversal, funding tests and depot-order short circuits preserve their existing operations and order. Native shells, service observations and presentation remain in C++.
- Limits: Existing cargo-predicate, historical-save, arbitrary NewGRF and scheduling qualifications remain on #156. Other maintenance families remain in the cleanup workset.
- PRs: #774, integrated in #777.

## Original vehicle breakdown vocabulary (#734)

- Breakdown eligibility, trigger and handling helpers use the original names, canonical vehicle/engine/climate states, sound values and smoke flag. Existing scalar entry points and the single helper bodies remain.
- Reliability updates, short-circuit guards, shared RNG draws, live vehicle reads, sound fallback and breakdown effects preserve their operations and order. Native maintenance observations, records and ownership are unchanged.
- Limits: Inherited observation differences, callback/lifetime, arbitrary NewGRF and scheduling qualifications remain on #156. Other maintenance families remain in the cleanup workset.
- PRs: #781, integrated in #785.

## Original Vehicle aging and value vocabulary (#734)

- The age, economy-age and value-decrease helpers use the original names, canonical dates, string identities, vehicle states and engine-subtype flags. Service observations, guards, anniversaries, warning/news order and depreciation operations remain unchanged.
- Other maintenance families, native records and scalar interfaces retain their existing owners. Source operations/order review and combined exact-head Full pass; archive equivalence is unclaimed. Inherited fidelity differences and coverage limits remain on #156.
- PRs: #788, integrated in #789. Other Vehicle vocabulary remains in the cleanup workset.

## Original Vehicle tile-hash and occupancy vocabulary (#734)

- The existing tile buckets, live cursors, ground/track/tunnel refusal checks and raw entry points use the original helper names, canonical tile/track/vehicle/string identities and explanatory comments. Hash arithmetic, bucket order, callback-sensitive link reads, stale-link handling and first-blocker observations are preserved.
- Cursor/link records, public scalar interfaces and native observations retain their shapes. Maintenance and scheduler bodies are unchanged. Source operations/order review and exact-head Full pass; archive equivalence is unclaimed. Inherited fidelity differences and coverage limits remain on #156.
- PR: #793, integration 124. Other Vehicle vocabulary remains in the cleanup workset.

## Original engine-replacement rule vocabulary (#734)

- Five existing renewal-rule helpers and their immediate transaction callers use original names, canonical engine/group identities and `GroupFlag`/`GroupFlags`. Raw masks, adapters and native engine-renew ownership are preserved.
- Ancestor-aware lookup/addition, exact removal, protection fallback, execution-only mutation, optional old-engine output, prepend order and next-before-delete traversal keep their existing operations and order. Unselected fleet and transaction bodies are unchanged.
- Source operations/order review, exact spelling acceptance and combined Full pass. Archive equivalence is unclaimed; inherited fidelity differences and coverage limits remain on #156. PR #796, integrated in #799; remaining Orders/fleet vocabulary stays open.

## Vehicle handles and the base vehicle table (#984)

- Rust owns: `GetNewVehiclePos` and `Vehicle::IsWaitingForUnbunching` (`vehicle.rs`), the tile and near-tile loops (`vehicle_func.rs`), `GetAdvanceDistance` and `GetAdvanceSpeed` (`vehicle_base.rs`), `GetVehicleProperty`, `GetEngineProperty` and `GetVehicleCallback` (`newgrf_vehicle.rs`), `PropertyID` (`newgrf_properties.rs`), and the typed train handle `train::Handle::from_vehicle`. Callers reach orders, unbunching, depot entry, the current order and the train owner directly; the tile index hashes with the shared map descriptor.
- C++ keeps: the base table's pool and shell operations, the fields the shell still stores, unported viewport, sound, window, NewGRF-error and path-result bodies, and links to other tables.
- Checks: the Rust tests of `vehicle.rs`, `orders.rs`, `road.rs`, `newgrf_vehicle.rs` and `occupancy.rs`, and the aircraft, road, ship and disaster ABI fixtures.
- PRs: #1031.
