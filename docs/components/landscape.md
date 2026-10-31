# Landscape

<!-- Add a new entry at the end of this file. -->

## TGP terrain generation

- Rust owns: Temporary height map, TerraGenesis stages, height estimation, noise, histogram, smoothing, regional curves and advance/apply loops.
- C++ keeps: Settings, RNG, map storage, progress and abort hooks.
- Checks: Default generation scenarios and native cancellation probe.
- Limits: Maximum-map performance and Windows/macOS runtime parity remain untested.
- PRs: #94; loops moved in #364.

## Landscape scheduler, ground and flooding

- Rust owns: `RunTileLoop` (LFSR walk and saved cursor), seven Rust tile owners; clear, rail, road and tunnel ground evolution; flooding, half-tile rail removal, dry-up, neighbour wakeups and flood-victim traversal.
- C++ keeps: Map arrays, allocation and writes; house, station and object tile handlers. Scheduler clear/water/tree reads use the shared map descriptor (#274); standalone water/tree, generation, planting, command and other owner reads use typed callbacks.
- Checks: `python3 tools/migration.py simulate landscape roads-flooding`.
- Limits: These remain #156 gaps: matrix, editor, NewGRF, threaded.
- PRs: #255; descriptor #351.

## Landscape clear, terraform and level (#348)

- Rust owns: `CmdLandscapeClear`, `CmdClearArea`, `terraform_cmd.cpp`, ordered height/dirty state, recursive propagation, validation, height commit, `CmdTerraformLand` and `CmdLevelLand`.
- C++ keeps: Map arrays and setters, unported tile procedures and presentation.
- Checks: Default `landscape` simulation.
- Limits: These remain #156 gaps: editor, NewGRF, threaded.
- PRs: #353.

## Landscape partial-pixel height

- Rust owns: `GetPartialPixelZ`, the landscape height kernel.
- C++ keeps: The public interface.
- Checks: `python3 tools/math-comparison.py`.
- Limits: Original signed-overflow inputs have no equivalence guarantee. The comparison excludes executed signed-overflow expressions and its bounded corpus does not cover every coordinate pair.
- PRs: #6 and #91.

## Spiral tile traversal

- Rust owns: Square/hole initialization, spiral position and movement, shell jumps, outside-map skipping, end detection and equality.
- C++ keeps: Typed iterator/sequence facade, `TileXY` dereference, wrappers, map storage, orthogonal and diagonal tile-area algorithms.
- Limits: Positive diameter/radius and increment-before-end are preconditions; the huge inherited perimeter after a wrapped extent is not traversed. Native generators do not use spiral traversal.
- PRs: #23.

## Tree generation, simulation and planting

- Rust owns: Tree generation, scattering, grove geometry, placement, growth, climate loops, tick seeding, persisted counter and planting commands.
- C++ keeps: Map/pools, rendering, editor forest-brush loop and save/load adapters; the brush calls Rust placement.
- Checks: `python3 tools/migration.py simulate trees`, Cargo tests and native ABI probe.
- Limits: Editor interaction, diagonal traversal, legacy saves, NewGRF ambient callbacks and abort rethrow are unreached.
- PRs: #102 and #208.

## Water-region cache and graph service

- Rust owns: Water-region cache, validity, tile labels, edge masks, patch counts, aqueduct flags, flood scratch, rebuild, invalidation and neighbour traversal.
- C++ keeps: Map storage, shared water track/follower queries, formatting and `water_regions.h` facade.
- Checks: `python3 tools/migration.py simulate water --jobs 2` and native ABI probes. See `tools/water-scenario-ai/README.md` for preparation, negative probes and optional crossing/timing measurements.
- Limits: Map replacement during a visitor violates original reference lifetimes. Arbitrary maps/NewGRFs and exhaustive region/path limits remain open.
- PRs: #114.

## Water construction, clearing and restoration (#294)

- Rust owns: Ship depot, lock and canal construction/removal, water clearing/restoration, river/desert traversal and owner/terraform/build-above policy.
- C++ keeps: Map primitives, depot identities, drawing, command results and area iterator.
- Checks: Default `water-commands` and water traffic/mutation/reload scenarios.
- Limits: These remain #156 gaps: pool exhaustion, construction under bridges, water objects, arbitrary NewGRF/legacy layouts, GUI.
- PRs: #317.

## Tunnel and bridge commands and ownership (#293)

- Rust owns: Bridge/tunnel construction, removal permission and clearing, ownership, terraform, build-above, reservations, infrastructure updates and hidden tunnel end tile.
- C++ keeps: Map/spec/pool storage, rendering, vehicle entry and command results.
- Checks: Default `roads-commands` scenario and reload.
- Limits: These remain #156 gaps: live train re-reservation, rectangular-map tunnel intersection choices, external NewGRFs, legacy layouts.
- PRs: #352.

## Object construction, generation and HQ lifecycle (#295)

- Rust owns: Object build/area commands, clearing, ownership, terraform/build-above, HQ lifecycle, colour updates, tile loops, generation, NewGRF variables, nearby/sister queries, animation triggers, type counts and cleared-area scratch; tile ownership uses the shared dispatcher (#333).
- C++ keeps: Object pool/specs/map/save/render records, NewGRF interpreter, badge, override and animation-frame services.
- Checks: Default `objects` scenarios and unchanged-body probes.
- Limits: These remain #156 gaps: external NewGRF/legacy, autoslope and generation branches.
- PRs: #313.

## Object invocation and eligibility (#430)

- Rust owns: Object/spec/tile/view context, root and parent selection, scope/feature/debug identity, callback entry and spec eligibility.
- C++ keeps: Object/spec vectors, overrides, map/save adapters, rendering and resolver/town facades.
- Checks: Default `objects-commands`, `objects-gaps` and `objects-reload` scenarios.
- Limits: #156 (external GRF loading and historical layouts); the fixture does not establish loader equivalence.
- PRs: #438, integrated in #444.

## Shared tile ownership dispatch (#331)

- Rust owns: The general `ChangeTileOwner` tile-kind dispatch. It calls the integrated rail, road, station, water and industry owners directly.
- C++ keeps: Clear, house, tree, void, tunnel/bridge and object handlers, which the Rust dispatch calls as typed leaves.
- Checks: Default `water-commands` and `ownership-dispatch` scenarios.
- PRs: #333 and #335.

## Landscape generation and rivers (#350)

- Rust owns: Landscape initialization and generation, terrain templates, coverage lines, desert/rainforest, springs, lakes/wetlands, river flow/widening and YAPF, heightmap greyscale conversion/loading, flat worlds, slope repair and ground watering.
- C++ keeps: Decoding, map storage/writes, sprite lookup and presentation; generation reads use typed map services.
- Checks: Default `generated.py`/`landscape.py` cases and native sea-order unit test.
- Limits: These remain #156 gaps: external heightmap formats, maximum dimensions, alternate graphics and late/GUI aborts.
- PRs: #364.
