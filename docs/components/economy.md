# Economy

<!-- Add a new entry at the end of this file. -->

## Station cargo-list queries

- Rust owns: selector/filter rules, pending run keys, totals, flush/merge policy, cumulative-share decoding and query-plan selection; it reuses the ScriptList owner.
- C++ keeps: station/cargo validation, pool and `GoodsEntry` access, packet/flow iterators and scalar extraction.
- Checks: unchanged stationlist script, both scripted suites and bounded cargo mode of `python3 tools/script-list-comparison.py`.
- PRs: #53.

## Station cargo service

- Rust owns: loading queue, station service counters, acceptance mask and cargo rating/pickup/speed/age/waiting/fraction metadata.
- C++ keeps: packet, flow, link-graph, map and geometry storage.
- Checks: `python3 tools/migration.py simulate stations economy`.
- Limits: Gaps remain in legacy cases, NewGRFs and refits (#156).
- PRs: #142, #217; boundary #214.

## Station construction, joining and removal (#267)

- Rust owns: rail platform/waypoint, road stop, airport, dock and oil-rig commands; sign/name/owner, terrain, clearance, joining, deletion/reuse and docking-neighbour decisions, including `water_cmd.cpp` docking eligibility/registration.
- C++ keeps: shell/spec/road-stop allocation, NewGRF resolvers, spatial indices, rendering/description and vehicle-entry/track/slope services.
- Checks: `python3 tools/migration.py simulate stations-commands aircraft-controller-removal water-structures-mutate --jobs 2`; default #320 cases, builtin tram, airport noise and reload cases are required.
- Limits: Gaps remain in external NewGRF programs, legacy cases and unusual occupied or reservation branches (#156).
- PRs: #303, #323.

## Station catchment, nearby sets and rectangles (#516)

- Rust owns: station rectangles, catchment bitmaps and delivery lists; town/industry nearby-station sets; rectangle/catchment/delivery/nearby maintenance, `CatchmentCoversTown`, `RecomputeCatchment(ForAll)`, `ForAllStationsAroundTiles`, `StationFinder` and production/acceptance scans.
- C++ keeps: pool shells, map, specs, presentation and native station/rectangle facades.
- Checks: default `stations-commands-catchment`, `stations-commands-catchment-reload` and affected station, subsidy, cargo-monitor, house, industry and object scenarios.
- Limits: Gaps remain in waypoint/buoy nearby-search candidates and `BeforeAddTile` try-mode assertion (#156).
- PRs: #522; acceptance #555.

## Cargo storage and movement (#139)

- Rust owns: packet fields, station/vehicle lists, count/age/feeder/action/reservation caches, flow shares/origin maps, packet operations, movement/staging/rerouting, flow reconciliation, capacity redistribution and autoreplace transfer.
- C++ keeps: packet pool identity/allocation/hooks, payment/delivery (#117), loading (#125), link-graph computation (#74), world access, thread join and save adapters.
- Checks: default cargo-storage, economy, station, Padhattan and aircraft scenarios; `python3 tools/cargo-storage-comparison.py` for unreachable pool-limit Split failure.
- Limits: Gaps remain in legacy conversions and complete refit/autoreplace transactions (#156).
- PRs: #151, #268.

## Cargo payment and delivery

- Rust owns: `CargoPayment` state/lifetime, final/transfer payment, settlement, industry acceptance, statistics/payment ordering, destination deduplication and production flush.
- C++ keeps: pool shells, world/packet storage, loading/reservation, subsidy checks and UI. Production resolution keeps one native resolver in caller-owned stack storage across the Rust repeat loop.
- Checks: default `economy` simulation; production cargo callback fixtures with #467 and narrow arithmetic fixture.
- Limits: The checks do not exhaust NewGRF resolver execution, legacy CAPY files or industry layouts. Boundary conversions remain in #168, #199 and #500.
- PRs: #123.

## Company identity, faces and liveries

- Rust owns: company colour/name/president generation, manager-face validation/bit helpers, livery propagation, identity commands, engine livery-scheme selection and company-colour cache.
- C++ keeps: company/vehicle shells and other fields, face tables and string rendering; console face parsing, local company selection and network allow lists remain with existing owners.
- PRs: #579.

## Company finances and economy lifecycle

- Rust owns: cash, loans, histories, rating/value, construction budgets, maintenance/interest/year rotation, bankruptcy/recovery, ranked offers, company creation/deletion/acquisition and ownership transfer; economy scheduling, prices, inflation/recession, scores and tick cursor.
- C++ keeps: identity, settings, networking, UI, AI and world services; `StartupCompanies`/`InitializeCompanies` timer/reset control.
- Checks: `python3 tools/migration.py simulate companies economy`; native CTest.
- Limits: Gaps remain in subsidies, exclusive rights, goals, story pages, hostile purchase, multiplayer runtime or NewGRFs.
- PRs: #149, #209.

## Link graph job computation

- Rust owns: demands, both MCF passes, cycle elimination, paths, flow mapping and job graph/settings.
- C++ keeps: worker launch/join, abort flags, LGRP/LGRJ save/load and station flow join.
- Checks: unchanged regressions/comparisons and `python3 tools/migration.py simulate` with manual/cargodist road saves and reload from a reference snapshot with outstanding LGRJ records; prepared road inputs reset GLOG history.
- Limits: The checks do not exhaust graphs, allocator failures or abort timing. Resource exhaustion timing differs with the allocation strategy.
- PRs: #95.

## Link graph scheduler ownership (#373)

- Rust owns: queued/running jobs, rotation, spawn/join/requeue, pool-wide date shifts and pause policy.
- C++ keeps: pools, job snapshots, worker launch/join, abort/completion publication and LGRS save/load.
- Checks: default symmetric/asymmetric cargodist reloads and module-compiled unchanged pinned scheduler bodies.
- Limits: Gaps remain in actual concurrent workers, multiplayer and GUI timing, and historical LGRS versions (#156).
- PRs: #380, integrated in #391.

## Live link graph contents and mutation (#378)

- Rust owns: graph cargo/date/nodes/edges, lifecycle/copy/init, mutation, supply/demand/location, restriction/release, merge/scaling, compression, dates and queries.
- C++ keeps: pool IDs/reuse, station membership, LGRP/LGRJ save/load and worker annotations.
- Checks: station fixture against unchanged reference bodies; station stale/removal/reload and symmetric/asymmetric LGRJ reload.
- Limits: Gaps remain in legacy/corrupt-load matrices and real-thread timing (#156).
- PRs: #383.

## Cargo-monitor accounting and ordered maps (#386)

- Rust owns: pickup/delivery maps, bit helpers, activation/query/reset/stop, cleanup and delivery accounting.
- C++ keeps: script validation, stations/pools, assertions and CMDL/CMPU save/load.
- Checks: `python3 tools/migration.py simulate economy-manual economy-cargodist economy-stockpile companies-delete companies-acquire engines-monitors`, `python3 tools/cargo-income-comparison.py` and native company/payment/ABI tests.
- Limits: Gaps remain in historical monitor-key conversion and external GameScript callers (#156).
- PRs: #388, integrated in #391.

## Station naming, tile loop and lifecycle (#562)

- Rust owns: `station_lifecycle.rs` default naming and suffix/industry-name exclusion, station tile loops (called directly by the tile scheduler with its map descriptor), destructor ordering, facility addition and airport maintenance; it shares the station service owner and calls the Rust queue, aircraft, order, link-graph, flow, cargo and catchment owners directly. Station map fields use the shared map descriptor; industry-map lookups preserve the original helper assertions and pool operations.
- C++ keeps: pool shells, unmoved record fields, the road-stop tile-data vector with `SetRoadStopTileData` and `RemoveRoadStopTileData`, native member destruction, spatial indices, news and windows. The native destructor is one Rust call followed by ordinary member destruction; each destructor owner access ends before a callback.
- Limits: `GetPlatformLength` remains with #560.
- PRs: #568.

## Station spatial queries and road-stop selection (#602)

- Rust owns: the process-wide station simulation index, rebuild/mutation/radius traversal, station sign index orchestration, nearby rating filters, accessible road-stop head/next selection and rail trigger-area decisions. Town, command, vehicle, crash, YAPF and NewGRF consumers call these owners directly.
- C++ keeps: native pools and records, canonical whole station/waypoint tile areas, qualified base sign movement and waypoint presentation, viewport indices, script/news/sound operations, and one stateless STL median-selection/strict-partition primitive. Each subtree with at least two IDs calls the primitive once; tree state and traversal remain Rust-owned.
- Limits: External NewGRF programs, historical station saves and GUI/game-thread presentation are outside the bounded default fixtures (#156). Native STL permutation and consumer observations are fixture-only support; configured-platform results remain validation work until their checks run.
- PRs: #612 (draft).
