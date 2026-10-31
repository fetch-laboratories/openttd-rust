# NewGRF

<!-- Add a new entry at the end of this file. -->

## Vehicle NewGRF resolver runtime (#392)

- Rust owns: Vehicle resolver cache, scope/root selection, wagon overrides, real-group selection, vehicle/purchase variables, callback/property/probability entries, subtype/aircraft helpers, randomisation and trigger recursion.
- C++ keeps: Action2 evaluation, specifications, pools, saved vehicle random/waiting fields and rendering.
- Checks: Default fleet resolver/reload, rail and aircraft fixtures, native ABI and aircraft comparison probes.
- Limits: #156 (arbitrary external GRF programs, historical saves, GUI scheduling and remaining branch combinations); fixtures do not claim persistent synthetic sprite groups.
- PRs: #397, integrated in #415.

## NewGRF transactional storage (#393)

- Rust owns: Transactional storage modes, changed registry, temporary registers/generation keys, payloads/backups, industry/airport/town `StorePSA` and town GRFID selection/traversal.
- C++ keeps: Pool shells, IDs, feature associations, allocation/publication, town associations, save/load and error frames.
- Checks: Default `town-storage-reload` scenario and native storage tests.
- Limits: These remain #156 gaps: external NewGRF programs, complete pre-161 conversion saves and GUI thread scheduling.
- PRs: #400.

## Industry and industry-tile NewGRF resolvers (#398)

- Rust owns: Industry and industry-tile feature contexts, roots/town, variables, count/search, tile-relative helpers, probability/refusal and resolution policy.
- C++ keeps: Action2 frames, virtual facades, specs/map/pool identity and result copying.
- Checks: Default `industry-resolver` scenario and existing production/lifecycle/animation/acceptance and cargo probes.
- Limits: These remain #156 gaps: external and historical gaps.
- PRs: #407, integrated in #415.

## Rail-station and waypoint NewGRF resolution (#399)

- Rust owns: Rail-station/waypoint scope, root, cargo and town state, variables, group selection, platform/end/continuation, slope callbacks, randomisation and resolver caches.
- C++ keeps: Specs/associations, map/station shells, saved random/waiting bits, Action2 and optional TownScope frames.
- Checks: Default resolver/reload fixtures.
- Limits: These remain #156 gaps: arbitrary external-GRF, historical and GUI paths.
- PRs: #411.

## Common Action2 execution and invocation state (#417)

- Rust owns: Action2 group dispatch, deterministic adjustments, variables/procedures/ranges/errors/defaults, randomized selection, resolver reset/callback/rerandomisation state and shared global variables including Action7/9/D.
- C++ keeps: Parsed graphs/specs, pools, terminal payloads, profiler output and remaining feature scopes.
- Checks: Default `town-storage-reload` scenario, native layouts and table visitors.
- Limits: These remain #156 gaps: arbitrary external GRFs/loader conditionals, historical saves, remaining native feature combinations and GUI scheduling. Native signed dword MIN/-1 division overflow has a compiler/platform-dependent undefined result; the cross-game fixture excludes it and Rust uses explicit wrapping division.
- PRs: #425.

## Station and road-stop definitions and associations (#484)

- Rust owns: Station/road-stop Action0 properties, group publication, shared class reset/allocation/membership/lookup, rail association allocation/assignment/deallocation, trigger rebuilding and afterload rebinding.
- C++ keeps: GRF/spec allocations, payload/layout containers, station shells, saved slot records, sprite graphs, strings and badges.
- Limits: These remain #156 gaps: historical STNN/STNS/legacy save frames, real exhaustion, arbitrary GRFs and GUI scheduling.
- PRs: #488.

## Road-stop NewGRF runtime and associations (#410)

- Rust owns: Road-stop/road-waypoint resolver context, root/cargo selection, parent and variables, callback/randomisation control, spec-slot selection, assignment/removal and trigger-cache reconstruction.
- C++ keeps: Station spec/tile vectors, saved bits and serialization, specs, Action2 and rendering.
- Checks: Explicit `CBID_RANDOM_TRIGGER` fixtures.
- Limits: #156 (historical, external-GRF and unreachable reseed/consumption branches); transient spec/group reattachment after load does not establish arbitrary external-GRF loading.
- PRs: #422, integrated in #427.

## Airport and airport-tile NewGRF runtime (#418)

- Rust owns: Airport and airport-tile resolver contexts, roots/scopes/town, variables, random packing, tile callbacks and airport selection/availability/bounds.
- C++ keeps: Parsing, spec/layout/override storage, rendering, preview drawing, text, hangar and rotation algorithms; `GetAirportCallback` has no Rust export.
- Checks: Default airport/reload fixtures.
- Limits: These remain #156 gaps: external loading, historical saves, GUI scheduling and native removal setup. Temporary admission builds carry no storage-writing group because native removal leaves a deleted PSA pointer; retained airport PSA is saved and reloaded.
- PRs: #428, integrated in #444.

## Airport and airport-tile definitions (#492)

- Rust owns: Airport/tile Action0 interpretation, allocation, reset, publication, override consumption, temporary file-spec cleanup and class binding.
- C++ keeps: Spec payloads, layouts, defaults, strings, badges, FSMs, graphics, stations and APID/ATID save/error frames.
- Limits: These remain #156 gaps: arbitrary external grammar, historical malformed save mappings, remaining layout rotations and offset combinations, missing-GRF recovery and GUI/network scheduling. The airport-removal PSA gap is outside this definition owner.
- PRs: #499.

## Generic NewGRF callbacks (#429)

- Rust owns: Generic feature registry, candidate traversal, invocation/root/scope/variables, AI selection and ambient enable/chance/parameter/playback policy.
- C++ keeps: Files, parsed graphs, specs, map, loading callers, playback and evaluation/profiler frame.
- Checks: Default `landscape-generic-callbacks` scenario.
- Limits: These remain #156 gaps: external-GRF loading, script station-build integration, other feature registrations, arbitrary programs, historical saves and GUI scheduling.
- PRs: #436, integrated in #444.

## Staged NewGRF loading interpreter (#471)

- Rust owns: Staged/file traversal, pseudo-sprite decoding, actions 6/7/9/8/D/E/10, patch flags/variables, GRM, labels, Action6 buffers, reset/status, finalization, `_cur_gps`, spriteset maps and spritegroup references.
- C++ keeps: Config/GRF/spec objects, parameters, sprite/file I/O, feature parsers and engine-definition initialization/finalization.
- Limits: These remain #156 gaps: arbitrary feature grammars/data sections, historical saves, network clock and static influence, remaining GRM/variable/target combinations and GUI scheduling.
- PRs: #473.

## Action2 graph construction and publication (#485)

- Rust owns: Action2 decoder, callback-result interner, range normalization, real-group simplification and tile-layout interpretation.
- C++ keeps: Polymorphic sprite groups, final payload vectors, constructors/destructors, graphics resources and bridge-recolouring facade.
- Limits: These remain #156 gaps: arbitrary grammars, remaining malformed and advanced-layout combinations, historical loading, rendering and network scheduling.
- PRs: #493.

## Cargo registry, translation and callback lifecycle (#467)

- Rust owns: Cargo label/default/climate tables, masks, sorted/reverse order, town-production lists, translation, finalization, callback invocation, identity, sprites and register output.
- C++ keeps: CargoSpec/default data, per-file GRF vectors, strings, class/name sort, Action0 reader/error frames and resolver/profiler frames.
- Limits: These remain #156 gaps: arbitrary external GRF loading, historical saves and GUI scheduling. Runtime fixture graphs reconstructed after reload do not establish file-loaded persistence.
- PRs: #472.
