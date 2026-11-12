# Reader and pool placement

Readers: planners and briefers of phase-3 reader owners and phase-2 pool owners, including the [`brief-storage-owner`](../skills/brief-storage-owner/SKILL.md) skill. It holds until phase 2 ends.

These are the accepted ownership boundaries between the reader phase (phase 3), the storage phase (phase 2) and later phases. The roadmap's [Phases](../roadmap.md#phases) holds the current reader envelope and its [after-cleanup order](../roadmap.md#after-cleanup) the queued owners. The [world-state design](world-state.md) holds the storage layout each pool owner builds on.

## Readers

- #669 holds the remaining reader owners and their selection events (the recorded conditions that bring each up for selection). It is not an additive retirement promise.
- #653's industry definitions are integrated. Specification files classified under simulation storage or logic add work outside the NewGRF bucket.
- Retain the script VM, registration glue, stack conversions and exception and suspension shims until their phase-6 owner.
- Executable API header methods belong to their API owners. Whole headers and new forwarding methods are not savings.
- Reader evidence compares API return values, errors, command results and enumeration, and protocol packets, ordering, and timeout, error and connection decisions: world-state equality alone misses these observations.
- Reader owners use integrated interfaces only. Root supplies a shared service once under [integration](../root-workflow.md#integration); a service need does not hold the whole owner behind an unrelated draft. Real state and lifetime dependencies keep their selection events.

## Every pool brief

- Pool briefs include allocation bookkeeping (`data`, `used_bitmap`, `first_free`, `first_unused`, `items`, the assertion credit `checked`, `cleaning` and any applicable allocation-cache state), allocation and indexed allocation, reuse, iteration, reset and cleanup, and the construction publication and destruction hooks.
- The generic native pool template earns no repeated deletion credit while any C++ pool instantiates it.
- Every Rust-used name, container or cache field moves in phase 2. Only fields used by original C++ alone may await a named later owner.
- Record lookup, coupled save descriptors and generated declarations follow integrated #570.

## Shell dispositions

These exact remaining dispositions constrain briefs. A shell is the C++ object that holds a Rust owner and keeps its pool identity ([porting terms](../porting.md#terms)).

| Boundary | Phase-2 ownership and exact later-owned remainder |
| --- | --- |
| Vehicle/type shells | Combined #656/#598/#599 takes maintenance, cargo/identity/loading, resolver fields, names and Rust-used sprite/cache fields; existing controller/cargo/order/visual/occupancy owners stay canonical. `coord`, `bounds`, viewport hash links and `fill_percent_te_id` belong to phase-5 drawing/viewport; `old_orders` belongs to #669 legacy conversion. Identity/vtable/owner-pointer scaffolding retires with final callers. |
| Stations/road stops/cargo | #634 takes names and Rust-used caches, spec-reference vectors, custom road-stop data, catchment/loading/nearby-list facades, GoodsEntry cargo/flow attachments and CargoPacket/CargoPayment pool completion. Link-graph owner takes flow/completed-job publication. `sign` belongs to phase 5; spec definitions belong to #669. |
| Town/industry/subsidy | #635 takes town `psa_list`, Rust-used `cached_name`/caches, industry `psa`, town/station links and subsidy flags; existing Rust name/cargo/count/nearby owners stay canonical. Town `cache.sign`/`show_zone` belong to phase 5. Industry `cached_name` may await phase-4 formatting only without a Rust field route; presentation facade storage retires with phase-4 string callers. Persistent-storage payload/pool is #638; industry definitions #653 and HouseSpec #669 after #628. |
| Engine | #637/#497 takes info, vehicle-specific info, original image/type/list position, GRF/override/badge payloads, lifecycle and name access after adopting #655. Only `display_flags`/`display_last_variant` await phase-5 build-list GUI. |
| Orders/lists/backups | All current fields and pool state move in phase 2; `old_order_index` and `OldOrderSaveLoadItem::{index,next,order}` belong to #669 legacy conversion. |
| Company/group | Phase 2 takes settings, finance/statistics aliases, identity, names/livery and free-unit/free-group state used by Rust. AI instance/info/config payload belongs to #669 script orchestration; authorization-list payload to #669 network remainder. Rust-used links move with the record. Only `Group::folded` awaits phase-5 GUI. |
| LinkGraph/LinkGraphJob | Graph/settings snapshots, join/abort state, annotations, flows and publication move in phase 2, including #168/#199/#500 completed-job publication. A native thread handle may remain behind a named join/start operation until platform threading moves; no field-callback exception follows. |
| Small pools | #638 includes objects, depots, signs, goals, story pages/elements, league tables/elements and persistent storage, plus #498 payloads and Rust-used strings. Only `Sign::sign` awaits phase-5 drawing/viewport. |

## Estimates

- Every issue estimate is net compiled C++ removed, including retained and generated declarations, facades and fixtures. Rewritten consumers earn no reduction credit ([cost decisions](../planning.md#cost-decisions)).
- #658's +445 code-line `.inc` re-baseline is reported separately from its small cleanup savings and from #657's deletion savings.
