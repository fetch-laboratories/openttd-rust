# Towns and industries

<!-- Add a new entry at the end of this file. -->

## Built-in town-name generation

- Rust owns: 21 built-in generators, seed selection, English replacements, Finnish vowels, Czech grammatical matching and constants.
- C++ keeps: NewGRF routing, settings and save/load; town founding (#246) owns retries/uniqueness.
- Checks: harness samples seeds through read-only AI logs.
- Limits: The checks do not cover all 21 * 2^32 combinations, allocator failures or NewGRF town-name behavior.
- PRs: #98.

## Town growth and lifecycle state

- Rust owns: town tick traversal, road/build choices, terrain retries, house selection/placement/expansion, metadata, histories, ratings and building counts (#148); house tile loops and deletion belong to #256.
- C++ keeps: native field aliases.
- Checks: default `town-growth` simulation and native tests.
- Limits: Gaps remain in natural tunnel construction, custom NewGRF callbacks or complete historical saves. #148 is not a complete lifecycle port.
- PRs: #127, #216.

## Town founding, generation and names

- Rust owns: custom-name bytes, founding, placement/coastal/grid search, initial growth, generation counts/retries, layout, rename and name uniqueness.
- C++ keeps: map/pool shells, house specs, rendering/NewGRF resolvers, spatial/noise/news services, save decoding and presentation name cache.
- Checks: `python3 tools/migration.py simulate town-founding- --jobs 2`; module-owned adapters compare cancellation, full-width count clamps/RNG and otherwise unreachable name bytes.
- Limits: Gaps remain in real NewGRFs, historical saves, GUI threading or the final 10,000-attempt fallback (#156). House lifecycle is #256.
- PRs: #258.

## Town commands, authority actions and deletion (#247)

- Rust owns: encoded town text, deity/editor commands, eight authority actions, `TestTownOwnsBridge`, town destructor and cleanup.
- C++ keeps: pool/map identities, persistent-storage shells, costs, news/events and save decoding.
- Checks: `python3 tools/migration.py simulate town-commands town-commands-reload --jobs 2`.
- Limits: Gaps remain in station allocation and statue search, NewGRF callbacks, live-vehicle deletion, legacy text decoding, and threaded GUI (#156).
- PRs: #347.

## House lifecycle and tile simulation (#256)

- Rust owns: `TileLoop_Town`, construction/rebuild, cargo, lift animation, north-part normalization, clearing/terraform/placement, caches, yearly aging, NewGRF house loop, deletion, animation triggers and rerandomisation.
- C++ keeps: virtual facades and common Action2. House resolution (#409) shares the north-part helper; the tile loop uses `StationFinder` (#516).
- Checks: `python3 tools/migration.py simulate town-houses- --jobs 2`.
- Limits: Gaps remain in station removal on house clearance (#156).
- PRs: #269.

## House definition loading and publication (#477)

- Rust owns: property decoding, substitute allocation, multitile validation, publication, earliest-year repair and reset.
- C++ keeps: `HouseSpec` shells/defaults, GRF/sprite graphs, strings and badges.
- Limits: Gaps remain in historical HIDS, other climates, and unreached property branches (#156).
- PRs: #486.

## House and shared-town NewGRF resolution (#409)

- Rust owns: `HouseScope`/`HouseResolver` and `TownScope`/`TownResolver` query policy, scope selection, house classes and count reset.
- C++ keeps: virtual facades, Action2, specs/override managers and map/pool shells.
- Checks: default `town-houses-` scenarios with real groups reinstalled after reload.
- Limits: Gaps remain in external-GRF badge/content combinations, historical save adapters, and GUI-thread integration (#156).
- PRs: #416.

## Industry and tile definitions and shared mapping storage (#475)

- Rust owns: Action0 industry/tile properties and layouts, allocation, publication, reset/finalization and `OverrideManagerBase` mappings/overrides.
- C++ keeps: manager virtual facades, IIDS/TIDS save/error frames, specs/defaults, GRF files, Action2 graphs, strings and badges.
- Limits: Gaps remain in historical IIDS/TIDS and malformed saves, exhaustion, undefined-spec/property/layout branches, and arbitrary GRFs (#156).
- PRs: #479.

## Industry periodic production and builder ownership (#129)

- Rust owns: produced/accepted slots, histories, production fields, sound countdown, builder records, production/transport ticks, farm/lumber updates, NewGRF production, monthly statistics/closure, daily changes and builder targets/retries.
- C++ keeps: pool/map/spec storage, construction/landscape-clear transactions, presentation, bounded NewGRF resolution and load adapters; `CargoPayment` delivery is #117.
- Checks: default `industry-` scenarios and native ABI/lifetime tests.
- Limits: The checks do not exhaust legacy saves, NewGRF resolver programs or industry layouts.
- PRs: #144, #205; boundary #203.

## Industry construction and tile lifecycle (#150)

- Rust owns: industry metadata/text, type registries, builder scratch, destruction, generation, validation/funding/prospecting, initialization, construction completion, tile loops, animation, acceptance, clear/terraform and NewGRF triggers.
- C++ keeps: map/pool shells, rendering, specs, resolvers and persistent storage.
- Checks: `python3 tools/migration.py simulate industry-command-fund industry-command-prospect industry-command-clear industry-command-terraform --jobs 2`, default industry scenarios/comparisons and `industry-water-check-*` (#234).
- Limits: Gaps remain in legacy saves, resolver programs and industry layouts (#156).
- PRs: #218.

## Subsidy lifecycle, route search and awards (#316)

- Rust owns: subsidy scalars, route search, creation/deity checks, cache rebuild/deletion, monthly generation/expiry, delivery awards and company transfer/deletion.
- C++ keeps: pool identity/allocation, cargo specs, news formatting and event enqueueing.
- Checks: default `subsidies-control` and `subsidies-reload`; console adapter compiling unchanged reference monthly lambda.
- Limits: Gaps remain in external NewGRF programs, legacy conversion fixtures, and threaded GUI/presentation (#156).
- PRs: #346.

## NewGRF town names and Action 0F (#585)

- Rust owns: Action 0F parsing (`FeatureTownName`) in the INIT stage, per-GRF style and part lists, the generator-name list, recursive seeded generation, style/GRF lookups and cleanup. The loader calls the parser, cleanup and list rebuild directly. Both `TownNameParams` constructors are shared Rust logic; town founding reads its style setting directly and keeps the parameters locally. Name verification calls the Rust generator directly.
- C++ keeps: a facade. `TownNameParams`, the genworld drop-down and `GetGRFTownNameName` read the Rust lists; `GRFTownNameGenerate` passes its `StringBuilder` with an append callback that receives each selected part's bytes. `TranslateTTDPatchCodes`, `AddGRFString` and debug messages remain native, behind one loading service each; translated bytes are copied into the Rust part from a view backed by the native loader stack frame.
- Limits: Generation preserves the native `gen < styles.size()` assertion. Self-referencing definitions recurse without a guard, as in the original.
- PRs: pending.
