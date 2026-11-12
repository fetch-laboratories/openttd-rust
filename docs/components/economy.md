# Economy

<!-- Add a new entry at the end of this file. -->

## Station cargo-list queries

- Rust owns: selector/filter rules, pending run keys, totals, flush/merge policy, cumulative-share decoding and query-plan selection; it reuses the ScriptList owner.
- C++ keeps: station/cargo validation, pool and `GoodsEntry` access, packet/flow iterators and scalar extraction.
- Checks: unchanged stationlist script, both scripted suites and bounded cargo mode of `python3 tools/script-list-comparison.py`.
- PRs: #53.

## Station cargo service

- Rust owns: loading queue, station service counters, acceptance mask and cargo rating/pickup/speed/age/waiting/fraction metadata.
- C++ keeps: packet and link-graph pool shells, map and geometry storage.
- Checks: `python3 tools/migration.py simulate stations economy`.
- Limits: Gaps remain in legacy cases, NewGRFs and refits (#156).
- PRs: #142, #217; boundary #214.

## Station construction, joining and removal (#267)

- Rust owns: rail platform/waypoint, road stop, airport, dock and oil-rig commands; sign/name/owner, terrain, clearance, joining, deletion/reuse and docking-neighbour decisions, including `water_cmd.cpp` docking eligibility/registration.
- C++ keeps: shell/spec/road-stop allocation, NewGRF resolvers, viewport sign indices and rendering/description.
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
- C++ keeps: packet pool identity/allocation/hooks, world access, thread join and save adapters.
- Checks: default cargo-storage, economy, station, Padhattan and aircraft scenarios; `python3 tools/cargo-storage-comparison.py` for unreachable pool-limit Split failure.
- Limits: Gaps remain in legacy conversions and complete refit/autoreplace transactions (#156).
- PRs: #151, #268.

## Cargo payment and delivery

- Rust owns: `CargoPayment` state/lifetime, final/transfer payment, settlement, industry acceptance, statistics/payment ordering, destination deduplication and production flush.
- C++ keeps: pool shells, world storage and UI. Production resolution keeps one native resolver in caller-owned stack storage across the Rust repeat loop.
- Checks: default `economy` simulation; production cargo callback fixtures with #467 and narrow arithmetic fixture.
- Limits: The checks do not exhaust NewGRF resolver execution, legacy CAPY files or industry layouts. Boundary conversions remain in #168, #199 and #500.
- PRs: #123.

## Company identity, faces and liveries

- Rust owns: company colour/name/president generation, manager-face validation/bit helpers, livery propagation, identity commands, engine livery-scheme selection and company-colour cache.
- C++ keeps: company/vehicle shells and other fields, face tables and string rendering; console face parsing and network key containers remain native.
- PRs: #579.

## Company finances and economy lifecycle

- Rust owns: cash, loans, histories, rating/value, construction budgets, maintenance/interest/year rotation, bankruptcy/recovery, ranked offers, company creation/deletion/acquisition and ownership transfer; economy scheduling, prices, inflation/recession, scores and tick cursor.
- C++ keeps: identity, settings, networking, UI, AI and world services; `StartupCompanies`/`InitializeCompanies` timer/reset control.
- Checks: `python3 tools/migration.py simulate companies economy`; native CTest.
- Limits: Gaps remain in subsidies, exclusive rights, goals, story pages, hostile purchase, multiplayer runtime or NewGRFs.
- PRs: #149, #209.

## Link graph job computation

- Rust owns: demands, both MCF passes, cycle elimination, paths, flow mapping, job graph/settings and abort flags.
- C++ keeps: worker launch/join, LGRP/LGRJ save/load and station flow join.
- Checks: unchanged regressions/comparisons and `python3 tools/migration.py simulate` with manual/cargodist road saves and reload from a reference snapshot with outstanding LGRJ records; prepared road inputs reset GLOG history.
- Limits: The checks do not exhaust graphs, allocator failures or abort timing. Resource exhaustion timing differs with the allocation strategy.
- PRs: #95.

## Link graph scheduler ownership (#373)

- Rust owns: queued/running jobs, rotation, spawn/join/requeue, pool-wide date shifts, pause policy and abort/completion publication.
- C++ keeps: pools, job shells, worker launch/join and LGRS save/load.
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
- Checks: `python3 tools/migration.py simulate economy-manual economy-cargodist economy-stockpile companies-delete companies-acquire engines-monitors`, the Rust `cargo_payment::income_tests` and `cargo_payment::settlement_tests` and native company/ABI tests.
- Limits: Gaps remain in historical monitor-key conversion and external GameScript callers (#156).
- PRs: #388, integrated in #391.

## Station naming, tile loop and lifecycle (#562)

- Rust owns: `station_lifecycle.rs` default naming and suffix/industry-name exclusion, station tile loops (called directly by the tile scheduler with its map descriptor), destructor ordering, facility addition and airport maintenance; it shares the station service owner and calls the Rust queue, aircraft, order, link-graph, flow, cargo and catchment owners directly. Station map fields use the shared map descriptor; industry-map lookups preserve the original helper assertions and pool operations.
- C++ keeps: pool shells, unmoved record fields, the road-stop tile-data vector with `SetRoadStopTileData` and `RemoveRoadStopTileData`, native member destruction, viewport sign indices, news and windows. The native destructor is one Rust call followed by ordinary member destruction; each destructor owner access ends before a callback.
- PRs: #568.

## Station spatial queries and road-stop selection (#602)

- Rust owns: the process-wide station simulation index, rebuild/mutation/radius traversal, station sign index orchestration, nearby rating filters, accessible road-stop head/next selection and rail trigger-area decisions. Town, command, vehicle, crash, YAPF and NewGRF consumers call these owners directly.
- C++ keeps: native pools and records, canonical whole station/waypoint tile areas, qualified base sign movement and waypoint presentation, viewport indices, script/news/sound operations, and one stateless STL median-selection/strict-partition primitive. Each subtree with at least two IDs calls the primitive once; tree state and traversal remain Rust-owned.
- Limits: External NewGRF programs, historical station saves and GUI/game-thread presentation are outside the bounded default fixtures (#156). Native STL permutation and consumer observations are fixture-only support; configured-platform results remain validation work until their checks run.
- PRs: #612 (draft).

## Company control, ownership checks and defaults (#610)

- Rust owns: local-company transitions, ownership assertions/errors, allow-list validation and notification eligibility, first-playable selection and service-interval dispatch. Company, session, generation, command and vehicle consumers enter the existing owners directly.
- C++ keeps: company shells and pool operations, current/local context and settings storage, key Add/Remove containers, encoded owner formatting, presentation and nested service commands.
- Limits: Actual network packets, client/admin delivery, GUI rendering and terminating assertion paths remain outside the bounded native/reload evidence (#156). Interactive RNG is carried explicitly across reload because the save omits it.
- PRs: #611.

## Original station, road-stop and monitor vocabulary (#734)

- Station and road-stop bodies use the original status, goods-state, rectangle, naming, graphics and specification elements and sets. `cargopacket::MoveToAction` supplies the original movement actions; `CargoPacket::MAX_COUNT` belongs to the existing packet record, with scalar adapters retained.
- Cargo monitor keys and private accounting/encode/decode helpers use `CargoMonitorID`. Unsigned key ordering, overflow-safe amount updates, save iteration and callback observations remain unchanged; exported monitor keys and services stay scalar.
- Native shells, pools, specs, attachments, vector owners and save facades remain with their existing owners. No C++ ownership, ABI or argument-struct change is included.
- Checks: original operations/order source reviews and combined exact-head Full. Existing monitor, station and arbitrary-NewGRF coverage limits remain on #156. C++ and boundary counts hold.
- PRs: #756 and #757, integrated in #758. The other five Cargo cleanup files remain pending.

## Original company face vocabulary (#734)

- Face helpers use the original names and explanations. `company_type::CompanyManagerFace` is the opaque native marker; the existing `Face` name remains an alias.
- Raw face bits, live style/variable tables, style/label writes, validation and world/interactive RNG selection retain their operations and order. Native face storage, strings, service tables and scalar interfaces remain unchanged.
- Checks: original operations/order source review and combined exact-head Full. Archive equivalence is unclaimed; inherited face/input, historical-save and GUI limits remain on #156.
- PRs: #762, integrated in #763. Remaining Company/economy vocabulary stays open.

## Original CargoPacket movement vocabulary (#734)

- Packet movement, staging and reservation comparisons use the canonical `MoveToAction` and existing `CargoPacket::MAX_COUNT`. Raw scalar adapters, combined station/action keys, list lifetime and cache update order remain unchanged; no second packet record or service is introduced.
- Checks: original operations/order source review and combined exact-head Full. Archive equivalence is unclaimed. Existing packet-precondition, historical-save and native lifetime limits remain on #156.
- PRs: #761, integrated in #765. Flow, payment and registry vocabulary remain in the cleanup workset.

## Original Livery scheme vocabulary (#734)

- `LiveryScheme` preserves the original byte values and unknown raw values; existing scalar adapters delegate to the canonical scheme definitions. Company livery reset/update and colour-command bodies use the original names, comments and state flags.
- Separate live colour reads, inheritance, command rejection, cache/admin/GUI/vehicle updates and effect order remain unchanged. Native company/vehicle storage, livery record layout and scalar ABI are retained.
- Checks: original operations/order review and combined exact-head Full. Archive equivalence is unclaimed; inherited colour, mod, historical-save and GUI-schedule limits remain on #156.
- PRs: #767, integrated in #769. Other Company/economy cleanup stays open.

## Original Cargo flow statistics vocabulary (#734)

- Existing flow-statistic helpers use the original names and explanations with canonical StationID and CargoType values. Cumulative keys, unrestricted bounds, sentinel handling, flow invalidation/restriction/release and post-join reconciliation keep their operations and order.
- Flow/FlowMap identities, scalar adapters, allocation/drop staging, callbacks and native facade contracts remain unchanged. Checks are original operations/order review and exact-head Full; archive equivalence is unclaimed and inherited Cargo qualifications remain on #156.
- PRs: #770, integrated in #773. Payment, registry and cargotype vocabulary remain in the cleanup workset.

## Original cargo payment vocabulary (#734)

- Income and delivery bodies use the original helper, transit-decay and callback names, with canonical cargo and owner identities. Fixed-point constants retain the original widths and arithmetic; native record views and scalar adapters remain unchanged.
- Delivery scans, destination insertion, refusal and acceptance observations, statistics, monitors, subsidy calculation, settlement and production flush keep their existing order. Native resolvers, world/pool storage and presentation remain in C++.
- Limits: Existing callback-domain, overflow, historical-save, NewGRF and live presentation qualifications remain on #156. No additional ownership or coverage closure is claimed.
- PRs: #775, integrated in #777. Other Cargo vocabulary remains in the cleanup workset.

## Original Company generation vocabulary (#734)

- Company colour, company name and president-name generation use the original helper names, typed local string/colour identities and explanatory comments. `gfx_type::Colours` supplies the complete byte-backed enumeration; existing scalar constants and helper reexports retain their callers.
- The 100 colour draws, weighting/elimination, live company/name reads, retry sequence, native name frames and news/admin effects keep their existing order. Native company storage, raw records and scalar interfaces remain unchanged.
- Limits: Raw invalid-index domains, arbitrary names/mods, historical saves, presentation and scheduling remain on #156. Other Company/economy bodies remain in the cleanup workset.
- PRs: #778, integrated in #780.

## Original Cargo registry setup and label vocabulary (#734)

- Climate setup, label rebuild/lookup and default-cargo classification use the original names and canonical cargo labels/types. The existing vector and ordered map retain their sole storage, unsigned comparison, raw domains and first-slot priority.
- Native copy/publication, validity and label reads, mask/array writes and callback order remain unchanged. Raw pointer views, registry services and scalar interfaces retain their layouts and owners.
- Limits: Inherited eager label snapshots, duplicate-slot index reads and container-allocation differences remain separate from coverage gaps on #156. Callback/sprite and other registry vocabulary remain in the cleanup workset.
- PRs: #782, integrated in #785.

## Original engine livery vocabulary (#734)

- The existing engine-livery helper uses the original name and canonical cargo, vehicle, engine-class, misc-flag and livery vocabulary. A compatibility reexport preserves its existing engine-loading caller and one helper body.
- Parent/articulated overrides, repeated live reads, default-cargo fallbacks, class/MU/tram/passenger decisions and raw return/assertion domains keep their operations and order. Company generation and all native records/services remain unchanged.
- Limits: Inherited original differences, raw-domain, native callback/lifetime, mod/save and presentation qualifications remain on #156. Other Company/economy families remain in the cleanup workset.
- PRs: #784, integrated in #786.

## Original Cargo sorting vocabulary (#734)

- The existing sorted-cargo initialisation body uses canonical CargoType and TownProductionEffect names and original comments. Native comparison, publication, reverse-index writes, per-town appends and the special-cargo break retain their existing order.
- Raw services, registry storage and other helpers remain unchanged. Source operations/order review and combined exact-head Full pass; archive equivalence is unclaimed. Inherited observation/container differences and coverage limits remain on #156.
- PRs: #787, integrated in #789. Other Cargo registry families remain in the cleanup workset.

## Original Company rename vocabulary (#734)

- Company and president rename adapters retain one shared helper while adopting original command identities, canonical string identities and explanatory comments. Reset, length/uniqueness checks, execution guards, native name frames, automatic company naming and notifications preserve their captured operations and order.
- Native company/name storage, raw service/record contracts and other Company families remain unchanged. Source review and combined exact-head Full pass; archive equivalence is unclaimed. Inherited read-staging differences and coverage limits remain on #156.
- PRs: #791, integrated in #794. Other Company/economy vocabulary remains in the cleanup workset.

## Original Cargo loading policy vocabulary (#734)

- Existing finalisation, translation-table selection, cargo translation, inverse mapping and fallback bodies use canonical cargo identities and original comments. Distinct pre-v7/usebit rules, eager reads, clear/traverse order, first matching label, narrowing, file reread and resize/write order remain unchanged.
- Raw label/file/spec records, services and exports retain their shapes and owners. Source review and combined exact-head Full pass; archive equivalence is unclaimed. Inherited read-staging/container differences and coverage limits remain on #156.
- PRs: #792, integrated in #794. Other registry callback/sprite vocabulary remains in the cleanup workset.
