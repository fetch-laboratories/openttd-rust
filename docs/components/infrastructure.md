# Infrastructure

<!-- Add a new entry at the end of this file. -->

## Rail and road introduction and construction availability (#440)

- Rust owns: Rail/road type availability and validation, engine aggregation, date-introduction fixed points and rail introduction reducer.
- C++ keeps: Type and engine specs/defaults, company masks and presentation.
- Limits: These remain #156 gaps: live GameScript construction, editor/generation commands, legacy saves and GUI scheduling.
- PRs: #447.

## Rail/road definitions, translation and saved identity (#476)

- Rust owns: Rail/road/tram reservation, property interpretation, allocation/reset/publication, lookup, translation, saved-label conversion, sorted lists and masks; cargo/rail/road/tram translation-table loading.
- C++ keeps: TypeInfo/default tables, strings, badges, sorting, GUI sprite resolution, per-file GRF labels/maps and RAIL/ROTT serialization; conversion reads typed map accessors.
- C++ keeps: TypeInfo/default tables, strings, badges, sorting, GUI sprite resolution, per-file labels and maps, and save/load frames.
- Limits: Rendering/resolver ownership, historical save variants and GUI scheduling remain outside this component.
- PRs: #481.

## Rail signal blocks and fixed work sets (#219)

- Rust owns: Signal-block exploration, immediate update/propagation, first-segment classification and 64/256/128 work sets.
- C++ keeps: Public signal constants, map/pools/spatial queries, commands and rendering.
- Checks: Default `signals` simulation.
- Limits: These remain #156 gaps: real tbd/global propagation overflow, threaded play and arbitrary mod layouts.
- PRs: #245.

## Rail track, depot and signal commands (#265)

- Rust owns: Rail track build/remove/drag, depot construction/removal, signal commands and autofill, rail conversion and tile clear, owner-change, autoslope/terraform and build-above policy.
- C++ keeps: Map reads, crossing/depot writes, nested commands, PBS, conversion iterator, affected-train list, tile loops, track status and drawing.
- Checks: `python3 tools/migration.py simulate signals-rail-commands --jobs 2`.
- Limits: These remain #156 gaps: coast/half-tile water and PBS path-end removal.
- PRs: #270.

## Road track, depot and conversion commands (#266)

- Rust owns: Road removal permissions and slopes, single/long build/removal, depots, clear/owner/nearest-town updates, terraform, conversion, build-above, prices and tile clearing.
- C++ keeps: Road-type allocation, drawing, track-status/vehicle-enter, tile loops and map services.
- C++ keeps: Road-type allocation, drawing, track status, vehicle entry and tile loops.
- Checks: Default `roads-commands` and `roads-commands-reload` scenarios.
- Limits: The fixture declares builtin-like road-type inputs; external NewGRF programs and legacy save variants remain #156 gaps.
- PRs: #290.

## Road stop bays and drive-through entries (#354)

- Rust owns: RoadStop status and drive-through entries, bay admission/freeing, entrance status, `GetByTile`, continuation, `Enter`/`Leave`, joining/splitting, `Rebuild`, `CheckIntegrity`, destruction and `AfterLoadRoadStops`.
- C++ keeps: Shell, pool, links, station stop lists, vehicle reads, tile hash and `GetNextRoadStop`.
- Checks: `stations-commands-road-stops` and `stations-commands-road-stops-reload`.
- Limits: These remain #156 gaps: articulated bay refusal and unreachable `include` duplicate.
- PRs: #362.

## Waypoint construction and command policy (#366)

- Rust owns: Deleted-waypoint reuse, rail/road axis selection, tile validation, rail/road/buoy construction, buoy removal, naming, rename and sign-move policy.
- C++ keeps: Waypoint/BaseStation shells, spec allocation, map setters, presentation, layout/iterator and command costs.
- Checks: `python3 tools/migration.py rust-checks` and `python3 tools/migration.py simulate stations-commands-waypoints stations-commands-waypoints-reload stations-commands-commands stations-commands-reload water-commands`.
- Limits: These remain #156 gaps: external station/road-stop GRFs, exhausted pools/spec lists and legacy saves.
- PRs: #369.

## Animated-tile scheduling and shared animation control (#385)

- Rust owns: Animated-tile queue, scheduling/control, trigger traversal and dispatch.
- C++ keeps: Map frames/bits, road records, pools/specs/PSA, resolvers and ANIT serialization.
- Checks: `python3 tools/migration.py simulate objects-gaps objects-reload effects-controllers-1`.
- Limits: Area/spec/member/mask reads and initial RNG before area are source-compared, not independently traced (#156). Historical/pre6/TTO saves, external GRFs, GUI threads and raw>=64 outside pinned Linux compilation remain #156 gaps.
- PRs: #389.

## Complete PBS reservation service (#367)

- Rust owns: PBS reservation reads/writes, platform scans, traversal, loop termination, train selection and waiting policy.
- C++ keeps: Map bits and spatial enumeration.
- Checks: Default rail-reservation, reload, controller and signal scenarios.
- Limits: These remain #156 gaps: external NewGRF trigger reentry, custom railtype turn flags, legacy conversion and threaded GUI coverage.
- PRs: #370.

## Track follower and platform length (#560)

- Rust owns: Rail, road and water track followers, admission, reversal, tunnel/bridge/platform skipping, reserved-track masking, errors, speed limits and station platform-length scans.
- C++ keeps: Map storage, vehicle shells and unmoved record fields; follower reads use the shared map descriptor.
- Limits: These remain #156 gaps: arbitrary NewGRF combinations, legacy saves and exhaustive map layouts.
- PRs: #569.
