# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-09, stocktake 143)

Agent: /root/stocktake_143 | Model: claude-opus-5-5 | Reasoning effort: medium

Integrations 132 to 143 merged between 19:23 UTC on 11-08 and 02:31 UTC on 11-09 with the four-implementer cap (user, 2026-11-08), a delegated integrator and a planner-filled ready queue. The table measures stocktake 143 (`da020b52c7`, #827) against stocktake 131 (`305ac20c28`). Integrations 132 to 143 each passed review and exact-head Full; none was rolled back. Integration 141's Full needed one authorized rerun of a simulation shard that hung on `network-protocol-loopback` (#693). Platform cadence 135 failed on Windows x86 oracle tests and 137 (#818) fixed it; cadence 140 failed on a `-Wmissing-field-initializers` warning from #814 and 141 (#831) fixed it, with the rerun green. The next cadence follows 145, and the next stocktake and profile follow 155. Profile 143 ([#275](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6731513225)) has all seven scenarios equal and all six policy records below band and ceiling. TGP's projection rose 5.32%, which triggers attribution (see [Speed](#speed)).

Fork C++ change since stocktake 131: −1,393 counted lines. On 11-08 (132 to 138) −1,067: −759 by 137, almost all #810's #736 U vector transfer (135, tools C++ −746), and −308 at 138 (#814 #737 V, compiled −263). On 11-09 (139 to 143) −326: −77 at 139 (#820 #736 P), −246 at 142 (#829 #736 W) and −3 at 143 (#827). Original C++ retired fell by 9 lines in simulation logic at 138 (#814's glue edits in `src/economy.cpp`, `src/order_cmd.cpp`, `src/train_cmd.cpp` and `src/vehicle.cpp`; 137 still measures 62,710). Root checked every line #814 added to those four files against the pinned original, and none is original code. The −9 is a classification shift in the metric, not restored original C++ (root, 2026-11-09). Tooling since 131: −1,692 code lines, mostly #810's Python fixture retirement at 135; 140 (#826) added 287.

| Measured quantity | Integration 131 | Integration 143 |
| --- | ---: | ---: |
| Original C++ remaining | 221,645 | 221,654 |
| Fork source glue, including generated records | 34,923 | 34,616 |
| Compiled source and generated records C++ | 256,568 | 256,270 |
| Tracked tools C++ evidence | 16,869 | 15,821 |
| ABI-generated non-record C++ evidence | 5,522 | 5,475 |
| Measured fork C++ series (scope below) | 278,959 | 277,566 |
| Original C++ retired | 62,710 of 284,355 (22.05%) | 62,701 of 284,355 (22.05%) |
| Simulation C++ retired | 41,550 of 61,148 (67.95%) | 41,541 of 61,148 (67.94%) |
| Expanded Rust exports / callback fields | 1,709 / 3,418 | 1,708 / 3,374 |
| Historical regex exports / callback slots | 1,553 / 3,376 | 1,552 / 3,332 |
| Shared-accessor / rebound-callee / orphan-export baseline rows | 117 / 314 / 40 | 104 / 304 / 39 |
| Retained Clippy keys / diagnostics | 664 / 757 | 649 / 739 |
| Ruff keys / diagnostics | 296 / 296 | 287 / 287 |
| Raw GLOBAL Rust / tooling inventories (scope below) | 175,659 / 63,820 | 190,613 / 62,128 |
| Play-save speed ratio | profile 131 (below) | No wall pass. Projections are 1.345x to 1.445x (P2000 highest), unchanged at the anchor floors |

Fork-added C++ under `src/tests` (5,615 lines) is within source glue. The 6,090 ABI-generated lines are 615 record declarations within source glue plus the 5,475 evidence lines above. The series does not yet inventory extracted-original comparison fragments or C++ embedded in Python strings; that scope follow-up belongs to #736 and earns no deletion credit. Raw GLOBAL Rust includes oracle text and native support; most of its +14,954 is #734 file passes and #736 oracle tests and data. Clippy retains five approved families.

| Source area / phase relationship | Original | Retired | Remaining | Fork glue |
| --- | ---: | ---: | ---: | ---: |
| Simulation logic (phase 1) | 47,424 | 38,329 | 9,095 | 6,456 |
| Simulation storage (phase 2) | 13,724 | 3,212 | 10,512 | 1,387 |
| NewGRF, save/load, script API and network (phase 3) | 51,794 | 13,067 | 38,727 | 3,360 |
| Support, graphics and viewport (phase 4) | 32,602 | 980 | 31,622 | 551 |
| GUI and platform (phase 5) | 64,120 | 415 | 63,705 | 465 |
| Vendored (phase 6) | 55,148 | 2,317 | 52,831 | 0 |
| Data tables | 17,295 | 4,367 | 12,928 | 14 |
| Upstream tests and fork test glue | 2,248 | 14 | 2,234 | 5,615 |
| Boundary layer | 0 | 0 | 0 | 16,768 |

These are source-area subtotals, not ownership or phase-exit claims. Since 131, simulation logic glue fell by 100, storage glue by 1, test glue by 69 and boundary glue by 137; phases 3 to 6 are unchanged. Null-driver games, bounded NewGRF/historical-save corpora and retained native fixtures leave the recorded limits on #156; entry witnesses are not branch coverage. Since 131, #156 records two further limits, neither a demonstrated defect: #810's encoded corpus, history and fatal-check reach, and the industry property branches that #821's cited scenarios do not supply. The inherited missing CargoPacket precondition assertions remain a fidelity difference (#726). #503 deep checks remain user-requested.

Unintegrated branches at 143: five component PRs, #825 (#737 V remainder part 1), #832 (#734 13/23 part a), #834 (#815 station items), #837 (#737 V remainder part 2) and #833 (#736 W landscape loop, accepted and queued). Draft #838 (#830) opened after 143.

### Cleanup stream inventories

Measured by the overhaul evaluation's analyzers on the overhaul snapshot `6fd364ffdb` and on integration 129. Root's stocktake updates the rows marked † ([selection](planning.md#selection)), here at integration 143; steering re-measures the others.

| Stream | Inventory | Overhaul | Integration 129 | Integration 132 | Integration 143 † |
| --- | --- | ---: | ---: | ---: | ---: |
| #734 | Workset files with a finished pass † | 0 of 319 | 0 of 319; 132 touched | 12 of 319 (#807) | 41 of 319 |
| #734 | Workset lines in finished files † | – | – | – | 32,080 of 176,930 (18.1%) |
| #734 | Explanatory comment lines missing inside matched ported bodies | 3,139 | 2,827 | – | – |
| #734 | Bare-integer parameters and fields whose concept has a named type | 1,957 | 1,947 | – | – |
| #734 | Transparent wrapper types / named constants | 69 / 2,181 | 266 / 3,411 | – | – |
| #735 | Functions over seven parameters | 150 | 141 | – | – |
| #735 | `too_many_arguments` ratchet rows / `allow` sites † | 71 / – | 63 / 19 | 62 / 19 | 53 / 11 |
| #736 | Fork C++ evidence under `tools/` / `src/tests`, code lines † | – | 16,869 / 5,684 | 16,869 / 5,684 | 15,821 / 5,615 |
| #737 | Shared-accessor / rebound-callee / orphan-export rows † | 117 / 314 / 42 | 117 / 314 / 40 | 117 / 314 / 40 | 104 / 304 / 39 |
| #738 | Rule corpus lines † | 3,312 | 3,362 | 3,120 | 3,229 |
| None | Clippy nesting / length / complexity rows; Ruff rows | 440 / 106 / 49; 296 | unchanged | 440 / 106 / 49; 296 | 438 / 105 / 47; 287 |

Counting commands: #734 from its issue's Worksets table and file lists, with lines as `wc -l` of each workset file under `rust/openttd-kernels/src` at the measured head (the issue's 18,297 of 173,403 lines at integration 138 reproduces this way); #735 from `migration/clippy-debt.json` and `git grep 'allow(clippy::too_many_arguments' -- rust/`; #736 and #737 from `python3 tools/port-metrics.py --whole-fork` and the baselines; #738 from `cat AGENTS.md $(git ls-files 'docs/*.md' 'docs/**/*.md' | grep -v -E '^docs/(roadmap.md|components/)') | wc -l`. No stream owns the structural rows: refactoring waits for the user.

Between 132 and 143:
- **#734** finished 29 more files: workset 01 completes with #812 (`town/house.rs`, 136) and #819 (`town.rs`, 139); workset 02 completes with #811 (9 files, 134), #816 (3, 138), #821 (2, 139) and #828 (`industry.rs`, 142); workset 03 reaches 12 of 14 with #813 (8, 136) and #827 (the station group, 143). `station_cmd.rs` and `station_service.rs` remain; #832 (13/23 part a) is open. Finished lines rose from 18,297 at 138 to 32,080; the workset total grew from 173,403 to 176,930 lines as passes restored comments.
- **#735** retired nine `too_many_arguments` rows and eight `allow` sites inside #734 passes: one at 136, five rows and eight sites at 138, one at 139 and two at 143 (#827's waypoint builders).
- **#736** retired 1,048 tools C++ lines and 69 `src/tests` lines: U at 135 (#810, −746), P at 139 (#820, −77) and the W town observations at 142 (#829, −223). #833 (W landscape loop) is queued.
- **#737** V (#814, 138) cut 13 shared-accessor, 8 rebound-callee and 1 orphan-export rows; #827 removed two more rebound-callee rows at 143. The V remainder is open as #825 and #837; W waits behind the `station_cmd.rs` and industry passes.
- **#738** grew by 109 lines (3,120 to 3,229) with no cut: the new `docs/review-brief.md` (+70) and Claude Code delegation in `docs/team.md` (+36).
- **Lint baselines** fell: Clippy keys 664 to 649, Ruff 296 to 287 (#826 split `worktree_refusal` and `remove`).

### Speed

Profile 143 on clean `da020b52c7` (integration 143, [#275](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6731513225)) used the same tools as profile 131: Callgrind/LibVEX 3.26.0 (dispatcher 3.22.0), GCC 13.3.0 and Rust 1.99.0.

| Scenario | Original Ir | Candidate Ir (143) | Projection (143) | Ir change since 131 |
| --- | ---: | ---: | ---: | ---: |
| TGP | 2,265,528,237 | 2,857,546,676 | 1.230318x | +2.6689% |
| Opus | 21,773,343,583 | 27,569,546,889 | 1.345115x | −1.0596% |
| Grok | 20,969,727,163 | 26,309,996,893 | 1.346652x | −1.0382% |
| P1996 | 3,600,502,896 | 4,564,266,895 | 1.352193x | −0.5539% |
| P2000 | 6,049,922,238 | 7,497,361,602 | 1.444531x | −1.0448% |
| P2016 | 11,343,424,651 | 13,744,477,974 | 1.382593x | −1.2955% |
| Drawing | 2,149,015,961 | 2,154,326,198 | 1.004635x | +0.0032% |

All seven nonempty canonical exit-save comparisons pass, and all six policy records are below band and ceiling. The five play loops fell, consistent with #737 batch V moving vehicle reads and station loading into Rust; their projections stay at the anchor floors.

TGP's projection rose 5.32% (1.168178x to 1.230318x), over the 3% projection-rise row, so root ruled that the row applies. TGP is 0.0047x under its 1.235x band start. [Attribution](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6731594943) traces it to two causes:
- +57.0M in `tile_loop::run`: #816 passes `&s.industry`, which spills the services copy (integration 138).
- +27.4M in `town::get_town_radius_group`: #819's `HZ_ZONE_ALL` loop runs through a 16-step filtered `HouseZones::iter()` (integration 139).

#814 offsets 10.3M. The fix-forward PR (`/root/cleanup_734_pass_02c_industry_resolver`) recovers an estimated 85M, back to the anchor floor. Ownership PRs proceed meanwhile. If TGP reaches its band first, a speed pass follows. No anchor or history was reset.

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

   Large files are not left for later. A file over about 1,500 lines gets its own single-file pass, placed by fan-in like everything else: before its workset's other files when they call it, and otherwise immediately after the workset pass that skipped it (root, 2026-11-09, after the planner showed that `station_cmd.rs` calls the station group and that nearly every 09 file calls `movement.rs`). Twenty-four workset files exceed that size; the largest are `station_cmd.rs` (4,868), `orders.rs` (4,841), `town.rs` (3,373), `train.rs` (3,121), `industry.rs` (2,874) and `company.rs` (2,698). A single file cannot split at a file boundary, so a single-file pass above 2,000 changed lines stays one PR with one reviewer per function group, as #819 did. The 03 remainder therefore becomes three passes: `station_cmd.rs`, `station_service.rs` (2,285), and `station.rs`, `station_map.rs`, `station_cmd/waypoint.rs` and `station_lifecycle.rs` together. #734 reports progress in lines finished as well as files: 18,297 of 173,403 lines (10.6%) and 33 of 319 files at integration 138.

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

Agent: /root/planner_ready_queue_146 | Model: claude-opus-5-5 | Reasoning effort: high

Checked on 2026-11-09 against:
- open PRs #825, #837, #838, #839, #840, #841 (#832 with #834) and #842;
- the dispatched #736 F cargo work, which has no PR or reservation yet and is taken to hold `cargo_payment.rs` and `cargo_storage.rs`;
- entry 5's hold on `station_service.rs`;
- the reservations on #734, #735, #736 and #737.

Counts are at `1b8b85c524` (integration 144; 145 is CI-only). Integration numbers follow the checkpoint's order: 146 is #841, 147 is #838 and 148 is #825. If the integrator combines #838 and #825, read 148 as the integration that carries #825.

**Root, 11-09:** integration 146 combines #832, #834, #838, #825 and #840 (`root/integrate-146`). So read "146 to 148" and "after 148" below as "after 146". #840's files are free from 146, and 9m, 15 and 6b-1 are ready at 146. 6b-1 goes first, so 9m waits until 6b-1 integrates.

How entries are measured:
- **Scope.** A file pass covers every line of its files with all four parts of work order item 1: wrapper types in internal signatures, return types and accessors; named constants; the original's comments; and #735's argument and context structs.
- **Integer sites** are named primitive integer declarations (`name: u16` and the like, outside comments and strings), counted as the overhaul inventory and #734's Worksets table count them.
- **Changed lines** are 8.6 to 15.5 per integer site, as #807 and #819 measured. A pass whose diff exceeds about 2,000 changed lines splits at a file boundary.
- **Callers** are Rust files that name the module (`crate::` paths, grouped `use` and `super::`). Under the [#734 naming decision](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6729665785), every caller of a typed helper changes.
- **Held callers.** "Held after 148" lists callers that #837, #839, #840, the F cargo work or entry 5 still hold once 146 to 148 land. Under root's [narrow allowance](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6730007460), a pass may rename a call to `<name>_raw` in a held file when the edit is one token per site, a trial merge with the holder is clean and the PR body lists the sites. Any other edit to a held file waits for its holder.
- **Overlaps.** Callers also count against other queue entries: one file belongs to one open PR.

Dispatched:

| Work order | Owner | State |
| --- | --- | --- |
| #734 13/23 entry 6a (#832) with #815's `road_stop.rs` and `station_association.rs` items (#834), combined as #841 | `/root/cleanup_734_pass_02c_industry_resolver`; `/root/cleanup_734_pass_01c_town_rs` | Integration 146 |
| Entry 10: #830 industry allows with #815's `CreateNewIndustry` item (#838) | `/root/cleanup_734_pass_02c_industry_resolver` | Accepted at `00519a0107`; in integration 146 |
| #737 V remainder part 1 (#825), outside the queue | `/root/cleanup_737_v_vehicle_boundary` | Accepted at `cb2b8b77b4`; in integration 146 |
| #737 V remainder part 2 (#837): engine, fleet and vehicle duplicate bindings | `/root/cleanup_737_v_vehicle_boundary` | Draft. Its [reservation](https://github.com/fetch-laboratories/openttd-rust/issues/737#issuecomment-6730521670) also holds `company_identity.rs`, `fleet.rs` and `fleet/pending.rs` |
| #734 03 entry 4a: `station_cmd.rs` single-file pass (#839; 4,866 lines, 286 integer sites) | `/root/cleanup_734_pass_01c_town_rs` | Draft, groups 1 and 2 pushed |
| #734 03 entry 5: `station_service.rs` single-file pass (2,286 lines, 63 sites, 542 to 976 changed lines, 18 caller files) | `/root/cleanup_734_pass_01c_town_rs` | Follows 4a. #825 released the file on #737 |
| Entry 12: #736 W house protocol (#840) | `/root/cleanup_736_p_packet` | Accepted at `b3fd687af2`; in integration 146 |
| Entry 14's cargo half: #736 F `cargo_payment.rs` and `cargo_storage.rs` test modules | `/root/cleanup_736_p_packet` | In progress: income oracle captured from the original only |
| TGP speed fix-forward (#843): `tile_loop.rs` industry arm, `HouseZones::iter()` | `/root/cleanup_734_pass_02c_industry_resolver` | In review; TGP −80.6M Ir |
| Entry 18: #835 timing races | `/root/cleanup_734_pass_02c_industry_resolver` | Dispatched as its own small tools PR while #843 is in review |
| #734 13 entry 6b-1 | `/root/cleanup_734_pass_02c_industry_resolver` | Starts after 146 and #843, with the four workset 13 leaves folded in |

Entry 6b-1:
- **File set.** The checkpoint does not record 6b-1's file set. Entry 6b's files are assumed: `direction_func.rs` (123 lines, 16 sites, 25 callers), `slope_func.rs` (201, 21, 28) and `spiral.rs` (157, 22, 10). Together: 481 lines, 59 sites, 507 to 914 changed lines.
- **Held after 148.** `town.rs`, `town/house.rs` and `town/resolver.rs` (#840); `ground_vehicle.rs`, `newgrf_vehicle.rs` and `vehicle_effect.rs` (#837, 9 sites); `station_cmd.rs` and `station_cmd/waypoint.rs` (#839, 10 sites). #837's and #839's sites fit the allowance.
- **`change_diag_dir_typed`.** 6b-1 also removes this leftover in `town.rs` (root, #734).
- **Fold.** Workset 13's leaves `clear_map.rs` (120, 7), `tree_map.rs` (58, 5), `void_map.rs` (25, 1) and `tree_cmd.rs` (4, 0) total 207 lines, 13 sites and 112 to 202 changed lines, too small for their own PR. Their callers are free after 148, and seven of them are also 6b callers. Fold them into 6b-1 if its owner has not posted a file set; otherwise they stay with entry 7.

Ranked ready entries: 9m (integration 148), 15 (integration 148) and 16 (now) for #734/#735 slots; 17 (integration 146) for an item 4 slot; 18 folds into the next maintenance PR. The rest wait as stated.

Undispatched, work order item 1:

9m. **#734 09 `movement.rs` single-file pass, with `signal_func.rs`.**
   - **Files.** `movement.rs` (1,554 lines, 76 sites), its test module `movement/tests.rs` (238, 9) and `signal_func.rs` (33, 2). Together: 1,825 lines, 87 sites, 748 to 1,348 changed lines.
   - **Why first in 09.** Nearly every 09 file calls `movement.rs`. Among 09 files, `movement.rs` calls only `signal_func.rs`, which is too small for its own PR.
   - **Fan-in.** 65 caller files name `movement`, and 4 name `signal_func`. These functions would change signature: `track_status`, `pixel_z`, `foundation_slope`, `enter`, `water_tracks`, `depot_ticks`, `should_stop`, the rail and road foundation helpers and `signal_along_trackdir`. Their names occur about 70 times in about 29 files. Most callers use only `native_assert`, `CachedTable` or the service table, which keep their form.
   - **Held now.** #825 holds 9 callers, including `train.rs`, `road.rs` and `ship_control.rs`. #841 holds `company.rs`, `newgrf_station.rs`, `road_stop.rs` and `station.rs`, and #838 holds `industry/policy.rs`.
   - **Held after 148.**
     - #837: `ground_vehicle.rs` (5 sites) and `vehicle_effect.rs` (1).
     - #839: `station_cmd.rs` (1, `water_tracks`).
     - #840: `town.rs` (1, `foundation_slope`).
     - Those eight sites fit the allowance. The other held callers (`animated_tile.rs`, `engine_loading.rs`, `newgrf_vehicle.rs` and `station_cmd/waypoint.rs`) use only unchanged items.
   - **Ready at integration 148.**
   - **Overlap with 6b-1.** 6b-1's callers include `movement.rs` and `movement/tests.rs` themselves and 18 of this entry's caller files, among them `train.rs`, `road.rs`, `ground.rs`, `water.rs`, `rail_cmd.rs`, `road_cmd.rs`, `signal.rs` and `disaster.rs`. The two cannot be open together. 6b-1 ranks first: (b) comes before (c). If 148 lands while 6b-1 still waits on #840, root may run this entry first, and 6b-1 then waits for it.
9a. **#734 09 part a.**
   - **Files.** `node_hash.rs` (75, 5), `yapf_node.rs` (30, 2), `pathfinder_func.rs` (61, 3), `follow_track.rs` (563, 34), `pbs.rs` (750, 48) and `water_regions.rs` (503, 33). Together: 1,982 lines, 125 sites, 1,075 to 1,938 changed lines.
   - **Fan-in.** 19 caller files.
   - **Held after 148.**
     - #839: `station_cmd.rs`, six `Pbs` method calls (`train_for`, `platform` and `reserved`).
     - Entry 5: `station_service.rs`, one `platform_length_full` call.
     - #840: `town.rs` and `test_tables.rs`, which name only `water_regions` and `signal` types and exports.
   - **Waits on 9m.** `follow_track.rs`, `pbs.rs`, `water_regions.rs` and `pathfinder_func.rs` call `movement.rs`, and `pbs.rs` and `water_regions.rs` are 9m callers. Ready when 9m integrates, with #839's six sites under the allowance.
9b. **#734 09 part b.**
   - **Files.** `signal.rs` (1,171, 56), `road_yapf.rs` (907, 86) and `ship_yapf.rs` (1,032, 78). Together: 3,110 lines, 220 sites, 1,892 to 3,410 changed lines, so it splits at a file boundary, for example `signal.rs` with `ship_yapf.rs`, then `road_yapf.rs`.
   - **Fan-in.** `signal.rs` has 9 caller files, held after 148 only by #839 (`station_cmd.rs`, two `add_track` calls) and #840 (`test_tables.rs`, types only). The two yapf files have no Rust callers. #825 released `ship_yapf.rs`.
   - **Waits on 9a.** Both yapf files call `follow_track.rs` and `node_hash.rs`.
9c. **#734 09 `rail_yapf.rs` single-file pass.**
   - **Size.** 1,901 lines and 184 sites, 1,582 to 2,852 changed lines. Above 2,000 it stays one PR with one reviewer per function group.
   - **Fan-in.** None in Rust.
   - **Waits on 9a.** It calls `follow_track.rs`, `node_hash.rs` and `pbs.rs`. It shares no file with 9b, so the two can run together.
15. **#734 07 vehicle transactions.**
    - **Files,** all under `vehicle_transactions/` except the first: `vehicle_transactions.rs` (707, 56), `refit.rs` (655, 39), `mass.rs` (306, 27), `clone_name.rs` (92, 0), `rail.rs` (993, 22), `articulated.rs` (270, 0), `build.rs` (312, 13), `clone.rs` (415, 9) and `construct.rs` (321, 5). Together: 4,071 lines, 171 sites, 1,471 to 2,650 changed lines.
    - **Split.** If the diff passes 2,000 lines, it splits at a file boundary. The first part is `vehicle_transactions.rs`, `refit.rs`, `mass.rs` and `clone_name.rs` (122 sites, 1,049 to 1,891), which the other five call. The second is the other five (49 sites, 421 to 760).
    - **Fan-in outside the group:** 4 files.
      - `fleet/transactions.rs` (#837) has five calls: `build`, `refit`, `rail::nested_move`, `sell` and `startstop`.
      - `station_service.rs` (entry 5) has one `refit::refit` call.
      - `settings_table.rs` and `vehicle/visits.rs` name only module paths and types.
    - **Held now.** #825 holds `articulated.rs`, `build.rs`, `clone.rs` and `construct.rs`.
    - **Ready at integration 148.** #837's five sites fit the allowance. If entry 5 has opened, its one site does too; otherwise this pass edits it, and entry 5 adopts the typed form.
    - **Order.** 07's core is these files' callee and would go first: `vehicle.rs` (429, 12; 48 callers), `vehicle_base.rs` (849, 53; 25) and `vehicle_func.rs` (102, 2; 7). After 148 the core still has 14 held callers, in #837 and its reservation, #839, #840's `test_tables.rs`, the F cargo files and entry 5. This group therefore keeps raw calls into the core, and the core's later pass edits them.
    - **Rank.** 08 ranks above 07, but its files are 9m callers or #837's (`ground_vehicle.rs`, `vehicle_effect.rs`), so 08 follows 9m. 15 shares no file with 9m, 6b-1 or 16.
16. **#734 21 `saveload.rs` single-file pass.**
    - **Size.** 1,909 lines, 98 sites, 843 to 1,519 changed lines.
    - **Fan-in.** One caller, `session.rs`, which is free. The file is over 1,500 lines and `session.rs` (also in 21) calls it, so it gets its own pass before 21's other files.
    - **Ready now.** No file is in an open PR, a reservation or another queue entry.
    - **Rank.** In (d), workset 21 (fan-in 62) comes second, after 16 (67). Workset 16 is not ready:
      - its two largest files are held: `newgrf_vehicle.rs` (1,689 lines) by #837 and `newgrf_station.rs` (1,156) by #841 until 146;
      - its other files have callers in #837 or #840, or in 9m or 6b-1.
7. **#734 13 part b: map core.** Not ready.
   - **First pass.** `map_func.rs` (650, 103; 87 callers) and `tile_map.rs` (593, 64; 34) call each other, so they pass together: 1,243 lines, 167 sites and 1,436 to 2,588 changed lines in the two files, plus caller edits in up to 87 files. It stays one PR with one reviewer per function group.
   - **Second pass.** `tilearea.rs` (648, 49; 20) and `tile_loop.rs` (593 lines with #833's tests, 18 sites; #840 holds its test module), with the four leaves unless 6b-1 takes them: 1,448 lines, 80 sites, 688 to 1,240 changed lines.
   - **Held after 148.** `map_func.rs` still has ten held callers, beyond the allowance:
     - #840: `animated_tile.rs`, `tile_loop.rs`, `town.rs` and `town/house.rs`;
     - #837: `ground_vehicle.rs` and `vehicle_effect.rs`;
     - #839: `station_cmd.rs` and `station_cmd/waypoint.rs`;
     - F cargo: `cargo_storage.rs`;
     - entry 5: `station_service.rs`.
   - **Waits on** #840, #837, the F cargo PR, #839 and entry 5, and on 6b-1, because `tile_map.rs` calls `slope_func`.
8. **#734 13 `trees.rs` single-file pass.**
   - **Size.** 2,062 lines and 137 sites, 1,178 to 2,124 changed lines.
   - **Callers.** Six files: `genworld.rs`, `landscape_cmd.rs`, `landscape_generation.rs`, `native_test_support.rs`, `session.rs` and `tile_loop.rs`. Of those, only `tile_loop.rs` (#840) is held after 148.
   - **Waits on entry 7** by placement: it calls the map core. It is also a 6b-1 caller, through `slope_func`, and is in #737 W's manifest.

Undispatched, work order item 4:

17. **#736 F vehicle scheduler and company economy.**
    - **Retire.** The behavioural cases of `src/tests/vehicle_scheduler.cpp` (479 raw lines, 10 cases) and `src/tests/company_economy.cpp` (539, 11), with their registration entries.
    - **Rust tests.** Test modules in `vehicle/scheduler.rs`, `vehicle/occupancy.rs`, `vehicle/maintenance.rs`, `company.rs` and `command.rs`, with expectations from the unchanged original under #736's oracle rule.
    - **Keep.** Layout, ABI, alias and native-lifetime cases stay as calling-boundary checks: "consumed boundary layouts", "consumed native ABI", "distinct native shells", "canonical owner layouts and property copy lifetime". The counted C++ removed is therefore below 1,018 lines, and the first checkpoint measures it.
    - **Colour alias case.** It calls `company_identity.rs`, which #837 reserves, so it waits on #837 or stays.
    - **Ready at integration 146,** when #841 releases `company.rs`. Entry 9m names `command.rs` but changes none of its calls: the one `movement` call there, `pixel_z`, takes pixel coordinates.
    - **Other F families.** These wait on 9m's files or #837:
      - `roadvehicle.cpp` (277), `effectvehicle.cpp` (228) with `effect_protocol.hpp` (170), `orders.cpp` (176), `train.cpp` (106) and `aircraft-range.cpp` (171) target `road.rs`, `effect.rs`, `orders.rs`, `train*.rs` and `aircraft.rs`, which are 9m callers or need 9m's typed calls.
      - `fleet.cpp` (98) targets `fleet.rs` (#837).
11. **#737 W World and stations.** Not ready.
    - **Size.** The overhaul manifest has 180 gross C++ lines in 83 files. Root re-checks source boundaries before commissioning.
    - **Held after 148.**
      - #839: `station_cmd.rs`, `station_command_ffi.h` and `station_command_services.hpp`.
      - #840: `town.rs`, `town/house.rs`, `town/resolver.rs`, `town_ffi.h` and `house_ffi.h`.
    - **No split.** The station and town files carry 68 of the 180 lines, and the rest carry 112. Both parts are under the 150-line fold bound, so W stays one PR.
    - **Overlaps.** Its Rust files also include `pbs.rs` and `signal.rs` (9a, 9b), `trees.rs` (entry 8) and `landscape.rs` and `landscape_generation.rs` (6b-1 callers).
    - **Ready** when #839 and #840 integrate, after 148, at a point where those entries are not open.
14. **#736 F Engine runtime, entry 14's second half.** The Engine runtime cases of `src/tests/engine.cpp` (247) move to `engine.rs` test modules. Waits on #837, which holds `engine.rs`.

Undispatched, maintenance:

18. **#835: timing races in `tools/test-validation-execution.py`.**
    - **Scope.** Tooling only; the file is in no open PR.
    - **Placement.** Its issue places it in the next maintenance batch, too small for its own PR. Root dispatched it as its own small tools PR rather than folding it into #843, which was already in review.

## Resume checkpoint

Agent: /root | Model: claude-opus-5-5 | Reasoning effort: xhigh

Active with the four-implementer cap restored (user, 2026-11-08, after an earlier two-implementer limit the same day). Two slots run #734/#735 file passes, and two run work order items 2 to 4 in order, as the work order sets. A running profile takes a slot. [Delegation](team.md#delegation-user-2026-11-08) applies (user, 2026-11-08):
- an Opus integrator at medium effort owns integration;
- each finding's reviewer verifies its fixes;
- reviewers start from the [review brief](review-brief.md);
- a planner keeps the [ready queue](#ready-queue) filled;
- implementers pipeline onto the next ready entry while their PR waits.

Code main is integration 144, `1b8b85c524` (#833). Stocktake 143 and profile 143 are in "Where the fork stands". Profile 143's attribution is done, and its slot is released. Unintegrated branches: #825, #832, #834, #837, #838, #839, #840, #841 and #842.

| Work | Owner (T3 child) | Next action |
| --- | --- | --- |
| Integration queue | Root for 145; `/root/integrator` from 146 | 145 is #842, root's Emscripten apt-mirror fix (Canonical's archive timed out three times on 11-09), on bootstrap Full with `CI_ON_DEMAND` unset; root restores the variable after the merge. 146 is #841 (#832 and #834), after a base update and a new Full; the platform cadence runs alongside it. Then #838, and #825 once its re-check passes. |
| #825, #737 V remainder part 1 | `/root/cleanup_737_v_vehicle_boundary`; re-check by `/root/review_825_recheck` | The stub-table fix is pushed at `cb2b8b77b4`. The re-check verifies the merge and the stubs, then re-posts both acceptances at that head. |
| #837, #737 V remainder part 2 | same | Draft, two milestones pushed. Resuming with a merge of 144. |
| #838, #734 entry 10 (#830 allows) | `/root/cleanup_734_pass_02c_industry_resolver`; reviewed by `/root/review_838_industry_allows` | Accepted at `00519a0107`: eight naming findings fixed by renames, nine suppressions and two lint-growth rows accepted, lint-policy at 0 errors. Queued after 146. |
| #734 entry 6b-1 | same | Starts after the speed fix, once 146 and #840 merge. |
| #734 4a, `station_cmd.rs` pass (#839) | `/root/cleanup_734_pass_01c_town_rs` | Draft, groups 1 and 2 pushed, group 3 (road stops) in progress. It includes a `Fix:` commit for the `TILE_HEIGHT_STEP` error parameter. Five reviewers by group when ready. Entry 5 (`station_service.rs`) follows. |
| #840, #736 W house-protocol follow-up (entry 12) | `/root/cleanup_736_p_packet` | Draft; local gates pass, the native verify is running. An evidence reviewer follows when it is ready. |
| TGP speed fix-forward | `/root/cleanup_734_pass_02c_industry_resolver` | One maintenance PR: pass a local `IndustryServices` copy in `tile_loop.rs`, and make `HouseZones::iter()` walk only the set bits. Measured on TGP and P1996 Ir. |
| Next stocktake/profile and cadence | Root; integrator for cadence | Stocktake/profile after 155; platform cadence alongside 146 (due after 145). |

Pass briefs add `cargo check -p openttd-kernels --features native-test-support --all-targets --locked`: `rust-checks` does not compile that feature, and #807's first Full failed on such a caller. A root acceptance comment uses the plain `Agent: /root | Model: ... | Reasoning effort: ...` form, which the lint gates parse. #710 records six missed Codex reviewer resumes on 11-08. Base updates are merge commits titled `Update: Merge integration N into ...`: `gh pr update-branch` titles them `Merge branch ...`, which the commit checker rejects. That is why #822 used #816's earlier reviewed head. Root clears finished reviewers' `build-rust` at integrations, since those builds filled the SSD on 11-08 (#710).

The [cleanup work order](#cleanup-work-order) replaces the private after-128 static queue; its three items lead item 4.

Remaining placement and limits:

- Metric scope completeness stays with #736: account for unlisted extracted/embedded C++ and separate executable source, oracle text and native support. Preserve raw reports/provenance and distinguish coverage corrections from deletions.
- Keep #693 / #736 P's failed and passing network traces and bounded original/original nonreproduction. The initial #800 content failure differs from prior reconnect/multiplayer stalls. No stacks/connector state proved a cause; the two-run authorization remains exhausted. No repair, retry or new diagnostic is selected.
- No later phase is selected.

Retained state for housekeeping:

- Keep the planning checkout `t3-20fec9ce` with its private plan, and origin `preserve/housekeeping-foundation-6ab9c20e5d`. Preserve every checkout with uncommitted or unpushed work. #805 integrated in 130, so `tools/housekeeping.py` now keeps detached unpublished commits; the 11-08 apply removed 40 eligible entries (1.7 GB).
- Keep `t3-8ed2642d` (profile 96), `t3-62945724` (98) and `t3-a6181dc7` (100) with their comparisons and frozen runtimes. Keep `.local/worktrees/root-profile-125`, `.local/worktrees/root-profile-119` and `.local/worktrees/root-profile-113` with accepted reports, raw Callgrind and frozen runtimes. Keep `t3-a490295e` (profile 131, clean `305ac20c28`) with its HDD `.local` (`profile-131-prepare`, `simulation`, `verification`). Keep `t3-d3ff7564` (profile 143, clean `da020b52c7`) with its HDD `.local` (`profile-143-prepare`, `simulation`, `verification`, `profile-143.log`). Its `build-rust` was reclaimed on 11-09; the outputs are intact. Profile 119 measures clean `4d3a1d1d41`, not integration 120.
- Keep `.local/retained-network-693` with the failed Full artifact, inconclusive diagnostics, passing traces and bounded original/original evidence. Keep main `.local/integration-records/`, `.local/speed-pass/` anchors, frozen runtimes and the retained simulation evidence.
- Leave dirty/staged trees untouched: `t3-3b491ddb` (five files), locked #565 WIP `t3-431c84b4`, locked native evidence `t3-d78d5f8b`, `t3-32cf4edc` (`tools/simulation/roads.py` staged), and `t3-e130d5a3` (`migration/saves/README.md` staged).
- Keep protected `.local/worktrees/root-docs-72` and locked `.local/worktrees/profile-after*`. Never push stale root-docs guidance over current main. Keep the pinned original and main's uncommitted user-owned `.codex/config.toml` unchanged.
