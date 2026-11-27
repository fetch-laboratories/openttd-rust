# NewGRF

<!-- Add a new entry at the end of this file. -->

## Vehicle NewGRF resolver runtime (#392)

- Rust owns: Vehicle resolver cache, scope/root selection, wagon overrides, real-group selection, vehicle/purchase variables, callback/property/probability entries, subtype/aircraft helpers, randomisation and trigger recursion.
- C++ keeps: Specifications, pools, saved vehicle random/waiting fields and rendering.
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
- C++ keeps: Parsing, spec/layout/override storage, rendering, preview drawing, text, hangar and rotation algorithms. `GetAirportCallback`, which the original does not call, has no C++ or Rust body.
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
- C++ keeps: Config/GRF/spec objects, parameters, sprite/file I/O and feature parsers.
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

## GRF text and Actions 3, 4 and 0x13 (#652, #497)

- Rust owns: GRF text storage, language selection, TTDPatch byte translation and choice lists, string insertion/lookup/cleanup, Action 3 dispatch and binding order with process-wide engine history, Action 4 naming and Action 0x13 translation of another GRF's strings.
- C++ keeps: Formatting-time text-stack/control-code processing (phase 4), GRFConfig text lists and native string tails, legacy TTD string mapping, language-map payloads, engine binding operations and storage (#637), ObjectSpec payloads (#638), and other spec payloads accessed through the shared spec table and named binding operations.
- Limits: #156 tracks remaining malformed-choice/control combinations, table and engine-pool exhaustion, historical saves, rendering, external GRFs and GUI/network scheduling. Text views retain the original invalidation rules.
- PRs: #655.

## NewGRF vocabulary, global variables, generic callbacks and town names (#1062 part 1)

- Rust owns: `newgrf.h`, `newgrf_callbacks.h`, `newgrf_commons.h`, `newgrf_config.h`, `newgrf_engine.h` and `newgrf_sound.h` vocabulary as wrappers; `GetGlobalVariable`, `HasGrfMiscBit` and `GetActiveCargoLabel`; the generic callback registry `_gcl` and every `newgrf_generic.cpp` body; the town-name lists `_grf_townnames` and `_grf_townname_names`, every `newgrf_townname.cpp` body and Action 0F. `newgrf::GRFFileRef` is the handle through which Rust reads a C++-stored `GRFFile`'s members, over the shared table's member readers.
- C++ keeps: `GRFFile` and `GRFConfig` storage and their member readers (#669), `PlayTileSound` (#669), the generic resolver's native frame (#1064), `CargoSpec::bitnum` (#1057) and the facades `AddGenericCallback`, `GetAiPurchaseCallbackResult`, `GRFTownNameGenerate` and `GetGRFTownNameList` for C++ callers.
- Limits: #156 tracks Action D's target `0x9E` and `GetActiveCargoLabel`'s `NOT_REACHED` arm, which no scenario reaches. The other files' `GRFFile` member slots move to the handle in this workset's later parts and those files' passes (#734).
- PRs: #1080.

## NewGRF classes, override managers, properties and parser results (#1064 part 1)

- Rust owns: `NewGRFClass` for stations, road stops, airports and objects (`classes`, each class's specs and UI count, and every `newgrf_class_func.h` body), with each instantiation's `InsertDefaults` and `IsUIAvailable` in its feature module; `OverrideManagerBase`'s storage and every body of `newgrf_commons.cpp`'s base class with `IndustryOverrideManager::GetID`, and the registered managers `_house_mngr`, `_industry_mngr`, `_industile_mngr`, `_airport_mngr` and `_airporttile_mngr`; `newgrf_properties.h`'s `PropertyID` and `newgrf_internal.h`'s `ChangeInfoResult` and `MAX_SPRITEGROUP`.
- C++ keeps: the `NewGRFClass` facade, whose inline `Get` reads the shared class record in place, and the manager objects, their virtuals, copy and lifetime, the spans the facade and `SlObject` read and write, and `ObjectOverrideManager::SetEntitySpec`.
- Limits: #156 tracks `Allocate`'s full-registry branch, which no scenario reaches. The raw `PROP_*` and `CIR_*` adapters serve unfinished files until their passes (#734).
- PRs: #1096.

## Rail and road type registry, definitions and availability (#1064 part 2)

- Rust owns: the globals and bodies over `_railtypes` and `_roadtypes` (`ResetRailTypes`, `InitRailTypes`, `AllocateRailType`, `GetRailTypeByLabel`, the translations, `ConvertRailTypes` and the label lists, with their road counterparts), the four Action 0 handlers of `newgrf_act0_railtypes.cpp` and `newgrf_act0_roadtypes.cpp`, and the engine availability test of `GetCompanyRailTypes` and `GetCompanyRoadTypes`.
- C++ keeps: the type records and the stores Rust calls, `GRFFile` storage behind `GRFFileRef`'s six type readers, `AddStringForMapping` behind one shared binding, the RAIL and ROTT chunk handlers, and the availability table's engine iteration and company stores.
- Limits: the lookup by label resolves each element through a slot call, which no retained profile observes, since none loads a NewGRF. `engine_loading` keeps two raw adapters (`railtype_list_raw`, `rail_label_raw`) until its pass (#734).
- PRs: #1098.

## Byte reader, storage arrays, spec leaves and translation tables (#1064 part 3)

- Rust owns: `ByteReader` over the pseudo-sprite buffer of the actions Rust interprets, with `ReadExtendedByte` and `ReadVarSize` written once for it and for the view of a C++ reader; `PersistentStorageArray`'s storage, backup and bodies, the three mode flags, `_changed_storage_arrays` and `SwitchMode` with its discard message; `TemporaryStorageArray`, whose one instance, `ResolverObject::temp_store`, is in `newgrf_spritegroup.rs` behind `get_register` and `set_register`; the three `StorePSA` bodies of industries, airports and towns in their features' files; `LoadTranslationTable`.
- C++ keeps: the `ByteReader` the unported handlers build, which Rust reads through `ByteReaderRef` until #669 ports those handlers; the `PersistentStorage` pool and the shell that holds `grfid`, `feature`, `tile` and the saver's `storage` alias; the six spec records behind the spec table; the translation vectors of `GRFFile` (#669).
- Limits: `store_value`, `get_register` and the `StorePSA` bodies run only for a loaded NewGRF, which no retained profile loads.
- PRs: (this pass).
