# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-08, stocktake 131)

Agent: /root | Model: claude-opus-5-5 | Reasoning effort: xhigh

Development resumed on 2026-11-08 in the cleanup stage; the session ran two implementers until the user restored the four-implementer cap the same evening (user, 2026-11-08). Code main was integration 132, `5661d54e0a` (#807), when this section was written; the [resume checkpoint](#resume-checkpoint) gives the current tip. The table measures stocktake 131 (`305ac20c28`, #808); 130 and 131 change tooling only, and 132 is the first #734 file pass (Rust only, fork C++ and boundary unchanged). Integrations 104 to 132 each passed review and exact-head Full with matching tested and merged trees; none was rolled back. [Profile 131](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6726331131) is accepted on clean `305ac20c28`, with profiles 113/119/125 retained. Platform cadence [130 passed](https://github.com/fetch-laboratories/openttd-rust/actions/runs/42511544906). Stocktakes and profiles now run every twelfth integration (user, 2026-11-08), so 137 is skipped and the next is 143. Platform cadence 135 failed and was fixed in 137; the next follows 140.

Fork C++ change since stocktake 125 (2026-11-07): −467 counted lines, all on 11-07 (127 and 129); 0 on 11-08. Tooling since 129: +33 (130) and +563 (131) code lines.

| Measured quantity | Integration 131 |
| --- | ---: |
| Original C++ remaining | 221,645 |
| Fork source glue, including generated records | 34,923 |
| Compiled source and generated records C++ | 256,568 |
| Tracked tools C++ evidence | 16,869 |
| ABI-generated non-record C++ evidence | 5,522 |
| Measured fork C++ series (scope below) | 278,959 |
| Original C++ retired | 62,710 of 284,355 (22.05%) |
| Simulation C++ retired | 41,550 of 61,148 (67.95%) |
| Expanded Rust exports / callback fields | 1,709 / 3,418 |
| Historical regex exports / callback slots | 1,553 / 3,376 |
| Shared-accessor / rebound-callee / orphan-export baseline rows | 117 / 314 / 40 |
| Retained Clippy keys / diagnostics | 664 / 757 |
| Ruff keys / diagnostics | 296 / 296 |
| Raw GLOBAL Rust / tooling inventories (scope below) | 175,659 / 63,820 |

Fork-added C++ under `src/tests` (5,684 lines) is within source glue. The 6,137 ABI-generated lines are 615 record declarations within source glue plus the 5,522 evidence lines above. The series does not yet inventory extracted-original comparison fragments or C++ embedded in Python strings; that scope follow-up belongs to #736 and earns no deletion credit. Raw GLOBAL Rust includes oracle text and native support. Clippy retains five approved families.

| Source area / phase relationship | Original | Retired | Remaining | Fork glue |
| --- | ---: | ---: | ---: | ---: |
| Simulation logic (phase 1) | 47,424 | 38,338 | 9,086 | 6,556 |
| Simulation storage (phase 2) | 13,724 | 3,212 | 10,512 | 1,388 |
| NewGRF, save/load, script API and network (phase 3) | 51,794 | 13,067 | 38,727 | 3,360 |
| Support, graphics and viewport (phase 4) | 32,602 | 980 | 31,622 | 551 |
| GUI and platform (phase 5) | 64,120 | 415 | 63,705 | 465 |
| Vendored (phase 6) | 55,148 | 2,317 | 52,831 | 0 |
| Data tables | 17,295 | 4,367 | 12,928 | 14 |
| Upstream tests and fork test glue | 2,248 | 14 | 2,234 | 5,684 |
| Boundary layer | 0 | 0 | 0 | 16,905 |

These are source-area subtotals, not ownership or phase-exit claims. Null-driver games, bounded NewGRF/historical-save corpora and retained native fixtures leave the recorded limits on #156; entry witnesses are not branch coverage. The inherited missing CargoPacket precondition assertions remain a fidelity difference (#726). #503 deep checks remain user-requested.

### Cleanup stream inventories

Measured by the overhaul evaluation's analyzers on the overhaul snapshot `6fd364ffdb` and on integration 129. Root's stocktake updates the rows marked † ([selection](planning.md#selection)), here at integration 132; steering re-measures the others.

| Stream | Inventory | Overhaul | Integration 129 | Integration 132 † |
| --- | --- | ---: | ---: | ---: |
| #734 | Workset files with a finished pass † | 0 of 319 | 0 of 319; 132 touched | 12 of 319 (#807) |
| #734 | Explanatory comment lines missing inside matched ported bodies | 3,139 | 2,827 | – |
| #734 | Bare-integer parameters and fields whose concept has a named type | 1,957 | 1,947 | – |
| #734 | Transparent wrapper types / named constants | 69 / 2,181 | 266 / 3,411 | – |
| #735 | Functions over seven parameters | 150 | 141 | – |
| #735 | `too_many_arguments` ratchet rows / `allow` sites † | 71 / – | 63 / 19 | 62 / 19 |
| #736 | Fork C++ evidence under `tools/` / `src/tests`, code lines † | – | 16,869 / 5,684 | 16,869 / 5,684 |
| #737 | Shared-accessor / rebound-callee / orphan-export rows † | 117 / 314 / 42 | 117 / 314 / 40 | 117 / 314 / 40 |
| #738 | Rule corpus lines † | 3,312 | 3,362 | 3,120 |
| None | Clippy nesting / length / complexity rows; Ruff rows | 440 / 106 / 49; 296 | unchanged | 440 / 106 / 49; 296 |

Since integration 103, raw fork C++ fell by 923 lines: `tools/` −649, `src/tests` −192, other `src/` −82. Original C++ retired held at 22.05%. #734 added about 4,900 Rust lines in 33 PRs. Rust comments rose by 3,524 lines, but only 438 of those sit inside bodies. Counting commands: #734 from its issue's per-workset counts; #735 from `migration/clippy-debt.json` and `grep -rn 'allow(clippy::too_many_arguments' rust/`; #736 and #737 from `python3 tools/port-metrics.py --whole-fork` and the baselines; #738 from `cat AGENTS.md $(git ls-files 'docs/*.md' 'docs/**/*.md' | grep -v -E '^docs/(roadmap.md|components/)') | wc -l`. No stream owns the structural rows: refactoring waits for the user. Between 129 and 132, #734 finished 12 files, #735 retired one parameter-count row (`DoCreateTown`), #738 cut 243 corpus lines, and #736/#737 held while their first items (#736 U, #809) are in progress.

### Speed

The instruction counts below measure profile 131 on clean `305ac20c28` (integration 131), with Callgrind/LibVEX 3.26.0 (dispatcher 3.22.0), GCC 13.3.0 and Rust 1.99.0.

| Scenario | Original Ir | Candidate Ir (131) | Projection (131) | Change since 125 |
| --- | ---: | ---: | ---: | ---: |
| TGP | 2,265,528,237 | 2,783,264,555 | 1.168178x | +0.0007% |
| Opus | 21,773,343,583 | 27,864,805,576 | 1.345115x | +0.0002% |
| Grok | 20,969,727,163 | 26,586,023,846 | 1.346652x | −0.0004% |
| P1996 | 3,600,502,896 | 4,589,688,730 | 1.352193x | −0.0260% |
| P2000 | 6,049,922,238 | 7,576,521,293 | 1.444531x | +0.2550% |
| P2016 | 11,343,424,651 | 13,924,870,889 | 1.382593x | −0.0102% |
| Drawing | 2,149,015,961 | 2,154,257,138 | 1.004635x | −0.0045% |

All seven nonempty canonical exit-save comparisons pass, and all six policy records are below band and ceiling. No loop Ir rose by 3% since 125, so there is no attribution trigger. Five play counts remain below anchor Ir, so their projections stay at the anchor floors. No anchor or history was reset. The five play scenarios now report nonempty stderr, the original's own script debug lines (a saved AI is unavailable, so the dummy AI loads), identical between reference and candidate apart from timestamps.

## Course (user, 2026-11-06)

- **Execution order:** the cleanup stage, then phase 3 readers, then phase 2 storage with each pool's save/load, then phases 4, 5 and 6 ([planning](planning.md#goal-and-phases)).
- **Yardstick:** net fork C++ removed, counting C++ under `tools/` and `src/tests`. Rust exports plus callback slots fall or hold; growth needs a reason the reviewer accepts ([cost decisions](planning.md#cost-decisions)).
- **Limits:** at most four implementation agents ([team](team.md#roles-and-models)); stocktake and profile every twelfth integration (user, 2026-11-08; every sixth from 2026-11-06, and stocktake 137 is skipped).
- **Enforcement:** checks enforce mechanical rules on every push (#733); reviewers work from the [review checklist](review-checklist.md); implementers read the [implementer brief](implementer-brief.md) and their work order.
- **Sizes, never durations:** plans, issues and briefs size work in counts (files, lines, call sites), never in time (user, 2026-11-06).

## Phases

Phase numbers name content; the execution order above sets the sequence. Phase 1 (simulation logic) exited at integration 63.

| Order | Stage | Exit outcome | Status |
| ---: | --- | --- | --- |
| 1 | Cleanup | Streams #733 to #737 done, #736 except boundary checks tied to a later phase, #738 applied; lint and growth baselines shrinking | Current |
| 2 | Phase 3: readers (script API, NewGRF, network, content, admin, save/load outside pool chunks) | Readers run in Rust with evidence for their observable outputs; VM and binding shims are named for phase 6 | Next; planned in the [work order](#after-cleanup) |
| 3 | Phase 2: storage, each pool with its save/load chunks, then the map | Rust owns every pool's records, slots, allocation, iteration, lifetime and save/load, then map allocation and remaining map bodies; no Rust-used field behind a native service | Queued; placement in [accepted placement](#accepted-reader-and-pool-placement-2026-11-03-after-integration-67) |
| 4 to 6 | Support; GUI, drawing and platform; vendored and final retirement | As in [planning](planning.md#goal-and-phases) | #3; plans deferred until adjacent |

## Cleanup work order

Each stream issue holds its batches, rules, proof and inventory. [Selection](planning.md#selection) ranks cleanup work by stream inventory. A file belongs to at most one open PR across all streams: check open PRs' file sets before starting. Files whose edits merge mechanically are exempt, including `lib.rs` module lists, `migration/*-debt.json`, `migration/harness-inventory.json` and generated registries; resolve those edits when updating from base. Cleanup PRs integrate on their stream's proof, review and exact-head Full, growing neither fork C++ nor the boundary ([integration](root-workflow.md#integration)).

Slots: while ready worksets remain, #734/#735 file passes hold two of the four implementer slots. The other two run items 2 to 4 in order; when #806 integrates, its slot passes to item 4. #738 runs as one docs agent outside the implementer count.

1. **#734/#735 file passes.** One PR per workset in the #734 manifest finishes every file it touches, in one pass:
   - the original's wrapper types in internal signatures, return types and accessors. FFI exports, service tables and generated declarations keep base integers (#355);
   - named constants wherever a literal stands for a named value;
   - the original's explanatory comments at the matching branches;
   - #735's argument and context structs for the functions defined in those files, including FFI exports with their C++ facades.

   A typed accessor replaces its raw form. Callers outside the pass use a temporary raw adapter that delegates to the typed accessor, and each adopts the typed form in its own file's pass. The adapter retires with its last caller, or at #734 closure. Split a workset only at a file boundary, and only when its diff would exceed about 2,000 changed lines. That size is provisional: the 33 vocabulary PRs averaged about 150 lines. Above about 1,000 changed lines, assign one reviewer per function group ([review](workflow.md#review)). Later PRs touch a finished file only for a mechanical shared-file edit or for a caller adopting a changed signature.

   Order (adopted 2026-11-08 from the overhaul thread's proposal, made at the user's request; it replaces closing passes for 01 to 09 and then the largest integer-site count first): pass a workset before its callers. A file passed after its callees adopts their typed forms in its one pass; one passed before them keeps raw calls and is touched again. Fan-in is the number of files outside the workset that reference it through `crate::` (measured at `5af17025a3`).
   - (a) Finish the 01 to 03 remainders: `town.rs` (#819), `industry.rs`, and the 03 station remainder.
   - (b) 13 Map/terrain (fan-in 90; `map_func` alone 85), with 23's type-bearing files: `bitmath_func` (39), `random_func` (21), `pool`, `math`, `kdtree`, `binaryheap`, `integer` and `container_func`. 23's fan-in of 137 is mostly `game_state`, `debug` and `witness` (45 to 46 each), which gain nothing from types; they and `raw_field` stay in 23's remainder.
   - (c) 09 (59), 08 (58), 07 (52), 06 (45), 04 (38), 05 (27). The proposal put 04 before 06, but the calls between those two worksets run both ways (06 to 04 in 2 files, 04 to 06 in 1), so strict fan-in governs.
   - (d) The rest by fan-in: 16 (67), 21 (62), 15 (42), 18 (37), 10 (33), 11 (28), 14 (27), 17 (26), 12 (25), and 23's remainder. The leaf worksets 22 (17), 20 (6) and 19 (1) go last. F's closure stays last.

   Large files are not left for later. A file over about 1,500 lines gets its own single-file pass, scheduled immediately after the workset pass that skipped it. Twenty-four workset files exceed that size; the largest are `station_cmd.rs` (4,868), `orders.rs` (4,841), `town.rs` (3,373), `train.rs` (3,121), `industry.rs` (2,874) and `company.rs` (2,698). A single file cannot split at a file boundary, so a single-file pass above 2,000 changed lines stays one PR with one reviewer per function group, as #819 did. The 03 remainder therefore becomes three passes: `station_cmd.rs`, `station_service.rs` (2,285), and `station.rs`, `station_map.rs`, `station_cmd/waypoint.rs` and `station_lifecycle.rs` together. #734 reports progress in lines finished as well as files: 18,297 of 173,403 lines (10.6%) and 33 of 319 files at integration 138.

   Worksets 02 and 03 are checked off, but their files keep raw internal signatures and literals; for example `station_cmd.rs:815` still has `0xffff` for `INVALID_STRING_ID` and `:4135` still has `9` for `AT_OILRIG`. A pass's reviewer applies the checklist's code standards to every line left in its files, not only to changed lines. A remaining literal for a named value, a raw internal signature or a hand-written copy of a shared helper is a finding. Proof follows the [cleanup checklist](review-checklist.md#cleanup-prs). Skip the archive-equivalence attempt for a pass that changes signatures or adds argument structs: it cannot pass, and #801's attempt differed in 15 of 370 object groups.
2. **#806**: carries for renamed lint keys and fidelity suppressions, as one tools PR. File passes in flight adopt it when they update from base.
3. **#726**: the steering-31 maintenance batch, including the inherited CargoPacket assertion fix. Its #741 dependency integrated in 104.
4. **#736 test evidence and #737 boundary debt**, ranked by C++ removed per PR, one PR per batch where the batch fits the review bound above. Fold any item under about 150 counted C++ lines into another item of its batch; the after-128 queue's items were 40 to 90 lines each. Order:
   - #736 U: the seven comparison probes, about 1,430 raw lines. Each already prints a deterministic stream, so freezing the original's stream retires it.
   - #737 V: about 400 glue lines, including the eight Orders field callbacks.
   - #736 P `packet-comparison.cpp`.
   - #737 W.
   - #736's remaining W/F families folded by file, including cargo income and Engine runtime.

   Every #736 item follows the oracle rule on #736. Root re-checks source boundaries and live reservations before commissioning each item.
5. **#738 rules**: one docs agent works #738's items; root reviews and commits each docs-only change between integrations.
6. **#734 closure**, after the last workset pass: private wrapper fields, generator support, and removal of the raw adapters and aliases.

Corrections adopted on 2026-11-07 under [#746](https://github.com/fetch-laboratories/openttd-rust/issues/746), still in force:

- #734's proof uses the [cleanup checklist](review-checklist.md#cleanup-prs): unchanged operations and order, plus exact-head Full when compiled code differs. Per-PR Callgrind profiles and disassembly accounting are not required.
- #747 is integrated through #751/#752: the Clippy ratchet retains complexity, parameter count, nesting and safety documentation. Remove unnecessary existing cast suppressions on later base updates.
- #734 permits the original's integer uses through wrapper accessors and measures wrapper adoption rather than Rust enum counts. The implementer brief places helpers by their original file, including `.cpp`-only helpers (#738).

## After cleanup

Readers first, in this order, re-estimated by fork C++ when selected:

| Rank | Owner | Issue estimate | Notes |
| ---: | --- | ---: | --- |
| 1 | #669 afterload/legacy | 1,800 to 2,600 | Conditional rank: re-check all-pool writes against boundary falls or holds before selection; pool records remain C++-owned until phase 2, so this owner may need to follow storage |
| 2 | #730 network core/discovery/resource | 700 to 1,100 | Resume from closed draft #731's branch |
| 3 | #669 GRF configuration/common publication/actions | 500 to 800 | Remaining SpriteGroupPool lifetime |
| 4 | #573 continuation | 350 to 550 | Codec, corrupt input, LoadCheck and wrappers; pool chunks move with their pool |
| 5 | #668 coordinator/STUN/TURN | 400 to 600 | |
| 6 | #669 admin, then script discovery/configuration | 350 to 550; 500 to 800 | Admin includes its socket pool policy |

Then storage, each pool with its save/load chunks, redone from current base with each closed draft's branch as reference: #656/#598/#599 vehicle (#714), #634 station/road-stop/cargo (#700), #638/#498 small pools (#723), #635 town/industry/subsidy (#704), #637/#497 remaining engine pools, #717 attachment, #725 YAPF, orders, companies, then the map (#565's remainder). #716 tracks the all-pools closure.

## Closed drafts

Closed 2026-11-06 with branches kept; heads and states are also on each PR and issue. Redo each from the cleaned base in the [after-cleanup order](#after-cleanup), using its branch as reference.

| PR | Issue | Head | State at close |
| --- | --- | --- | --- |
| #700 station | #634 | `0256b0bbdb` | Cargo, builds and 12 focused cases passed; selected-native result unverified; foreign Industry/Town/PSA/virtual-dispatch seams open |
| #704 town/industry | #635 | `60757916b4` | Unfinished 15-file foreign-consumer edit; Clippy failures; `OpenTTDIndustryTown` still has native news callers |
| #714 vehicle | #656 | `20c5f1bb99` | Cargo passed; native/runtime pending; engine field route open; conflicts with 102 |
| #723 small pools/ObjectSpec | #638 | `2ec78fb676` | Four checkpoints pushed at close, unvalidated; next was adopting `Shared.grf_price_shift` |
| #731 network | #730 | `655d8f8990` | Saved unfinished lifetime work, unvalidated; prior `595232f50c` passed Cargo and native |

## Accepted reader and pool placement (2026-11-03, after integration 67)

Agent: /root/place-after67-reader-plan | Model: gpt-6.1-sol | Reasoning effort: high

Root accepted these ownership boundaries. The current reader envelope is in
[Phases](#phases); #669 holds the remaining owners and their selection events,
not an additive retirement promise. #653's industry definitions are integrated;
specification files classified under simulation storage/logic add work outside
the NewGRF bucket. Retain the VM, registration glue, stack conversions and
exception/suspension shims until their phase-6 owner. Executable API header
methods belong to their API owners; whole headers and new forwarding methods
are not savings.

Pool briefs include allocation bookkeeping (`data`, `used_bitmap`, `first_free`,
`first_unused`, `items`, assertion credit `checked`, `cleaning` and applicable
allocation-cache state), allocation/indexed allocation, reuse, iteration,
reset/cleanup and construction publication/destruction hooks. The generic
native pool template earns no repeated deletion credit while any C++ pool
instantiates it. Every Rust-used name/container/cache field moves in phase 2;
only exact original-C++-only fields may await a named later owner. Record lookup,
coupled save descriptors and generated declarations follow integrated #570.

Exact remaining shell dispositions constrain briefs:

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

All issue estimates are net compiled C++ removed, including retained/generated
declarations, facades and fixtures. Rewritten consumers receive no reduction
credit. #658's **+445 code-line .inc re-baseline** is reported separately from
its small cleanup savings and #657's deletion savings. Reader evidence compares
API return values/errors/command results/enumeration and protocol packets,
ordering, timeout/error/connection decisions; world-state equality alone misses
these observations. Reader owners use integrated interfaces only. Root supplies
a shared service once under [integration](workflow.md#integration); a service
need does not park the whole owner behind an unrelated draft. Real state and
lifetime dependencies retain their selection events.

## Thirty-second steering review (2026-11-07)

Agent: /root (Claude Code steering review) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host

Root was paused for this review. Its focus was the overhaul thread's evaluation of cleanup integrations 104 to 129. This review verified that evaluation against the code and ran three independent Opus audits: Station contexts #801 with Train/Cargo vocabulary #800, the largest vocabulary batch #758, and the evidence transfers #802/#803.

Findings:

- **The process is sound, but delivery is thin.** Every integration had review and exact-head Full, with no rollback, and the audits found no reachable divergence. Yet the [stream inventories](#cleanup-stream-inventories) moved 0 to 10% in 26 integrations.
- **Ranking by fork C++ starved #734/#735 (running dry).** Under [selection](planning.md#selection), #734 and #735 always ranked last, because they remove no C++ by design. After 125, root left vocabulary for evidence families. Its after-128 queue held three items of 40 to 90 lines.
- **#734 was cut into helper-sized slices (template drift).**
  - It ran as 33 PRs of about 150 lines each. `town/founding.rs` and `company_identity.rs` each took 5 PRs, and `town_type.rs` and `cargo_registry.rs` 4 each.
  - Root's briefs set a "bounded family" per PR, kept raw signatures and accessors, and left argument structs to "#735 later". The work order then sent #735 back over the same files.
  - Steering 31's rule that converging owners integrate checkpoints, written for storage owners, carried into the cleanup.
- **The "finished" worksets are not finished.** Worksets 02 and 03, checked off in 110, keep raw internal signatures and literals for named values (`subsidy.rs:475`, `station_cmd.rs:2415`). #801 rewrote `station_cmd.rs:815` and kept `0xffff` for `INVALID_STRING_ID`. #797 left a hand-written `DirDifference` (`train.rs:688`). The source reviews checked operations and order, but missed these code-standard findings.
- **Contracts are a hurdle.**
  - 143 contract records appear in 29 PR bodies, and 67 of them repeat at integration heads. #800, #801, #797 and #798 carried about 128,000 characters of reports and contracts for about 170 net lines.
  - The overhaul's proposed "same token hash" carry would be too weak: every function-level Clippy row shares the constant fingerprint of `item`. #806 replaces it with declared carries, reasoned fidelity suppressions and content-bound acceptance.
- **Oracles carry redundant text.**
  - The 127–129 oracles hold 2,071,485 raw bytes (158,323 gzip) for 467 lines. Git stores them compressed, so the cost is the checkout in each worktree and the test binaries.
  - Storing each distinct output once and dropping echoed inputs would cut them to about 243 KB with no new dependency. The Town authority matrix has 5 distinct outputs across 14,976 rows.
  - The tests call production exports or their inner helpers, and recomputing 436 rows from the original source found no mismatch.
  - Two native checks lapsed: the Industry production-callback export forwarder and the Company ABI field offsets (#156).
- **Two items slipped.** #726, selected by steering 31, never started, although its #741 dependency integrated in 104. #738 never ran, and the corpus grew from 3,312 to 3,362 lines.
- **Accepted:** the overhaul's decisions 1, 3 and 4, plus decision 2 with a stronger carry rule. Nesting, length and complexity rows did not move; refactoring waits for the user.

Actions:

1. [Selection](planning.md#selection) ranks and reports cleanup work by stream inventory. Stocktakes update the [inventory table](#cleanup-stream-inventories) (`/start-development` step 4).
2. #734/#735 run as one pass per workset that finishes every file it touches, with two implementer slots ([work order](#cleanup-work-order), [integration](root-workflow.md#integration)). Root rewrites #734's body as a per-workset count of finished files and moves checkpoint history to the PRs. #735's batches fold into the passes.
3. #806 removes rename, fidelity and renewal contracts. Real growth keeps the reviewed contract.
4. #736 items store each distinct output once and report oracle bytes, under a provisional budget on #736. Batch U leads #736's ranking.
5. #726 runs after #806. #738 gets one docs agent.
6. Review reports and integration notes follow [review](workflow.md#review): findings, fixing commits and disposition. A base update records only its applicable condition, and no report re-certifies base adoption or metric provenance.
7. No user direction is reversed. The 11-06 yardstick still governs owners and forbids fork C++ or boundary growth in cleanup PRs. It never ranked work inside the cleanup stage the user set.

## Earlier steering, still in force

Ownership, original behavior, process scope, named callees, typed shared services
and exact random-draw order come first. Preserve default semantic evidence,
record gaps on #156 and keep retirement distinct from glue. No hot allocation,
opcode dispatch, whole-record copies or owner bounces; no C++ frame is kept only
for attribution. Rust reaches the map by a load through `map_func::World`, with
native assertions and no C++ fallback, and descriptor reads stay within
`docs/design/world-state.md`. The rebound-callee, shared-accessor and
orphan-export baselines only shrink. An owner does not port support code whose
data stays in C++ behind one service per field. Ready owners proceed beside
speed work; only timed collection holds builds and games. Ceiling and band
decisions use the three-pass record; the projection factor and speed thresholds
remain provisional, and 0.1% to 0.3% speed candidates get no attribution
research. Testing is maintenance: scenarios arrive with the ports that need
them. Six branches, batched maintenance, fixing reviews, strict exact-combination
Full, bounded rollback and action-based wind-down remain. A historical mention
is not placement; CI-found behavior fixes get a scoped review; deep checks run
only on the user's request; active speed dependencies are preserved. Third-party
test inputs stay out of Git and CI artifacts ([evidence](porting.md#evidence)). Every agent runs
its own long jobs and ends a turn only on a named job ID; root checks any child
that ends a turn without one, and any child idle on a job that has exited. A
base update that meets a review condition needs only CI. Each new scenario,
fixture or checker names the behavior existing checks cannot distinguish; this
rule is provisional, on a basis of 1.5 tooling lines per retired line. Roadmap
commits happen only at stocktakes, course changes and wind-down. #627's
per-container typed leaves for STL containers are a phase-3 necessity; they do
not justify per-field leaves in phase-2 records. Every owner's cost is its
fork-C++ change, decided before review. Rust-defined `extern "C"` functions
return 0-or-1 `u8`, not `bool` (#644). Read a failed Full to its last job before
recording its cause. The decided beside placement and its revisit conditions live in the world-state design. Per-message `Debug` and `GrfMsg` services are design faults
after #657 integrated. A change to Cargo, the toolchain or cmake runs the
platform profile once before Full. AGENTS.md and its routing table carry the
rules; issue and PR bodies carry current actions and evidence. Native fatal paths keep the native error
box and crash reason (#709). Native diagnostic subsets address a named failure
or uncovered fixture; exact-head Full is the final simulation tier. A completion
claim names only the delivered scope (#636). Release a quiet-host hold when the
timed games end; a rejected peripheral host action does not end the assignment.

Steering 31 (2026-11-06), still in force: an ownership port's checkpoint integrates once it meets the [integration](root-workflow.md#integration) rule and the owner continues in a fresh PR, while cleanup passes finish their files instead; a field moves in the same change that deletes its Rust-called services, before pool adoption, and no service routes through `Record()`; the whole-fork metric runs twice per PR; status comments change at milestones; long jobs start as `exec python3 tools/signal-witness.py -- <command>`, and a host-killed job (exit 143, no logged failure) reruns once incrementally (#724).

## Recent completed work

Evidence and review reports stay in the linked PRs. Metrics are actual reductions where stated, not sums of rewritten lines. Per-integration rows for 69 to 129 are in the roadmap at `a9ca01d657`.

| Integrations | Work | Result |
| --- | --- | --- |
| 69–102 | Porting through steering 31: settings, corpus, typing, shared records, GRF text, logging, vehicle records #636, script APIs, industry specs, game protocol, fatal reporting, accounting tools, map leaves, shared pool core/facade, content state #706, graph/job pool #719, engine records #722 | Compiled C++ −4,289 through 88; #722 −852 at 101. Original retirement reached 22.07%. |
| 103, 107 | #733 checks #742; lint narrowing #747/#751 | Quick CI runs Cargo checks and the lint, policy, growth and proof gates. Tooling +1,956 code lines. |
| 105, 109–126 | #734 foundation F #739 and 33 vocabulary PRs (#753–#798) | Rust +4,900 added lines; C++ and boundary hold. Worksets 02 and 03 checked off; 132 of 319 workset files touched. |
| 128 | #735 P1 Station helper contexts #801 (`5d43172624`) | Eight parameter-count rows retire; Rust +310/−200. |
| 104, 107, 108, 127, 129 | #736 R0 #740, cargo/engine #749/#750, MovementGap #748, Company finance #802 (`0c8a0f8a05`), Town/Industry policy #803 (`7b6887f43d`) | 127 and 129 retire 467 counted C++ lines for 1,273 net Rust test lines and 2,071,485 raw oracle bytes. All 40,320 finance and 21,128 policy cases preserved. |
| 104, 106, 108 | #737 C #741, H #745, R #744 | Compiled C++ −151, exports −10. |
| 130 | Housekeeping retention of unpublished detached commits #804/#805 (`b7b686b51a`) | Tooling +33 code lines; C++, Rust and boundary unchanged. |
| 131 | #806 declared lint carries, fidelity suppressions and content-bound acceptance #808 (`305ac20c28`) | Tooling +563 code lines; C++, Rust and boundary unchanged. Module-scoped rows still bind acceptance to the exact head (#806). |
| 132 | #734/#735 workset 01 Town/house pass, 12 of 14 files #807 (`5661d54e0a`) | Two function-group reviews and exact-head Full. Five wrappers, about 45 constant/helper sites and 110 original comment lines restored; one parameter-count row retires. Fork C++ and boundary unchanged. `town.rs` and `town/house.rs` remain. |

## Recurring

- After every twelfth integration (user, 2026-11-08), take stock under `/start-development` step 4 and profile the merged tip in the background ([speed](speed.md#instruction-projections)). This supersedes the skill's "every sixth integration" (user, 2026-11-06) until `/steer` or `/navigate` updates it. A running profile counts as one implementer. Stocktake/profile 131 are accepted; profiles 113/119/125 remain retained for comparison. Stocktake/profile 137 are skipped (user, 2026-11-08); the next regular stocktake/profile follows integration 143.
- Run the platform cadence after every fifth integration; 120, 125 and [130](https://github.com/fetch-laboratories/openttd-rust/actions/runs/42511544906) passed. [135](https://github.com/fetch-laboratories/openttd-rust/actions/runs/42526994364) failed on Windows x86: #810's 64-bit oracle tests ran on a 32-bit build. #818 fixed this as integration 137, after which integrations resumed. The next cadence follows 140. A new failure stops integrations.
- Deep checks run only on the user's request (user, 2026-10-31).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions.
- #156 (coverage gaps), #275 (speed), #724 and #710 (host) stay open trackers. #500 is delivered by #734; #168 and #199 attach to #737 and the owners that touch those records. #693 retains the cancelled reconnect failure and two completed original/original runs that did not reproduce it; the cause remains unresolved and that two-run authorization is exhausted. Further work stays with #736's batch P, without a selected repair or extra diagnostic run. #503's coverage continuation runs last.

Excluded, with issue placement and return conditions rechecked on 2026-11-07; near-full reproduction and fork release-source publication triggers are not met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Ready queue

Work orders ready to dispatch, which root selects when a slot frees ([delegation](team.md#ready-queue-and-pipelining)). The planner refreshes this section when fewer than two entries are ready. File sets are checked against open PRs and reservations.

| # | Work order | Files | Waits on |
| ---: | --- | --- | --- |
| 1 | #824: housekeeping reclaims `build-rust` from finished kept worktrees | `tools/housekeeping.py` and its test | Ready |
| 2 | #734 02 `industry.rs` single-file pass (2,874 lines) | `industry.rs`, plus callers adopting typed forms | #823 (`industry/resolver.rs`) |
| 3 | #734 03 `station_cmd.rs` single-file pass (4,868 lines) | `station_cmd.rs`, plus callers | #823 (`station_cmd.rs` caller edits); #737 W waits behind it |
| 4 | #734 03 station group: `station.rs`, `station_map.rs`, `station_cmd/waypoint.rs`, `station_lifecycle.rs`, with #815's inherited items | those files | #823 (`station_lifecycle.rs`) |
| 5 | #734 03 `station_service.rs` single-file pass (2,285 lines) | `station_service.rs` | Check against the #737 V remainder reservation |
| 6 | #734 13 Map/terrain core with 23's type-bearing files | `map_func.rs`, `tile_map.rs`, `slope_func.rs`, `direction_func.rs`, `tilearea.rs` and the rest of 13's non-tree files; `bitmath_func.rs`, `random_func.rs`, `pool.rs`, `math.rs`, `kdtree.rs` | #823 (`map_func.rs`, `slope_func.rs`, `direction_func.rs`, `math.rs`) |

## Resume checkpoint

Agent: /root | Model: claude-opus-5-5 | Reasoning effort: xhigh

Active with the four-implementer cap restored (user, 2026-11-08, after an earlier two-implementer limit the same day). Two slots run #734/#735 file passes, and two run work order items 2 to 4 in order, as the work order sets. A running profile takes a slot. [Delegation](team.md#delegation-user-2026-11-08) applies (user, 2026-11-08):
- an Opus xhigh integrator owns integration;
- each finding's reviewer verifies its fixes;
- reviewers start from the [review brief](review-brief.md);
- a planner keeps the [ready queue](#ready-queue) filled;
- implementers pipeline onto the next ready entry while their PR waits.

Code main is integration 138, `8e967f83c5`. Two branches are unintegrated: #823 and the batch V remainder.

| Work | Owner (T3 child) | Next action |
| --- | --- | --- |
| #823, integrating #819 (`town.rs` pass), #821 (industry resolver pass) and #820 (#736 P packet probe) | Integrator | Exact-head Full is requested at `d6cf15acd7`. Merge as integration 139, post the note, close the components and clean up. All reviews carried; fixes spot-checked or verified. |
| #737 batch V remainder | `/root/cleanup_737_v_vehicle_boundary` | Family 5 is committed (`4b449ceb27`). Next it pushes a draft PR, then the station visit-type and position families. |
| Ready queue | Implementers of #819, #820, #821 (pipelining) | Entry 1 (#824) can start now; entries 2 to 6 wait for #823. |
| D1 from #819 | #736 W | Retire the test-only `openttd_rust_town_expand` after moving its fixture observations to Rust tests. |
| #737 W | Unassigned | Waits behind the `station_cmd.rs` pass and the industry passes, whose files it overlaps. |
| Next stocktake/profile and cadence | Root; integrator for cadence | Stocktake/profile after 143; platform cadence after 140. |

Pass briefs add `cargo check -p openttd-kernels --features native-test-support --all-targets --locked`: `rust-checks` does not compile that feature, and #807's first Full failed on such a caller. A root acceptance comment uses the plain `Agent: /root | Model: ... | Reasoning effort: ...` form, which the lint gates parse. #710 records six missed Codex reviewer resumes on 11-08. Base updates are merge commits titled `Update: Merge integration N into ...`: `gh pr update-branch` titles them `Merge branch ...`, which the commit checker rejects. That is why #822 used #816's earlier reviewed head. Root clears finished reviewers' `build-rust` at integrations, since those builds filled the SSD on 11-08 (#710).

The [cleanup work order](#cleanup-work-order) replaces the private after-128 static queue; its three items lead item 4.

Remaining placement and limits:

- Metric scope completeness stays with #736: account for unlisted extracted/embedded C++ and separate executable source, oracle text and native support. Preserve raw reports/provenance and distinguish coverage corrections from deletions.
- Keep #693 / #736 P's failed and passing network traces and bounded original/original nonreproduction. The initial #800 content failure differs from prior reconnect/multiplayer stalls. No stacks/connector state proved a cause; the two-run authorization remains exhausted. No repair, retry or new diagnostic is selected.
- No later phase is selected.

Retained state for housekeeping:

- Keep the planning checkout `t3-20fec9ce` with its private plan, and origin `preserve/housekeeping-foundation-6ab9c20e5d`. Preserve every checkout with uncommitted or unpushed work. #805 integrated in 130, so `tools/housekeeping.py` now keeps detached unpublished commits; the 11-08 apply removed 40 eligible entries (1.7 GB).
- Keep `t3-8ed2642d` (profile 96), `t3-62945724` (98) and `t3-a6181dc7` (100) with their comparisons and frozen runtimes. Keep `.local/worktrees/root-profile-125`, `.local/worktrees/root-profile-119` and `.local/worktrees/root-profile-113` with accepted reports, raw Callgrind and frozen runtimes. Profile 119 measures clean `4d3a1d1d41`, not integration 120.
- Keep `.local/retained-network-693` with the failed Full artifact, inconclusive diagnostics, passing traces and bounded original/original evidence. Keep main `.local/integration-records/`, `.local/speed-pass/` anchors, frozen runtimes and the retained simulation evidence.
- Leave dirty/staged trees untouched: `t3-3b491ddb` (five files), locked #565 WIP `t3-431c84b4`, locked native evidence `t3-d78d5f8b`, `t3-32cf4edc` (`tools/simulation/roads.py` staged), and `t3-e130d5a3` (`migration/saves/README.md` staged).
- Keep protected `.local/worktrees/root-docs-72` and locked `.local/worktrees/profile-after*`. Never push stale root-docs guidance over current main. Keep the pinned original and main's uncommitted user-owned `.codex/config.toml` unchanged.
