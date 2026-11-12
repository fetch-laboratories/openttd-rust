# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place ([records](workflow.md#records)); `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands

The figures in this section measure merge 32 (`985abab91f`), the last stocktake. The base is now merge 44 (`93eedd81b2`); the merge-44 stocktake and profile are due ([resume checkpoint](#resume-checkpoint)).

- At merge 44, `migration/finished-files.txt` lists 116 finished files, against 99 at merge 32. #737 and #738 are closed at their end states. No rollback has happened since merge 32, and no confirmed base regression is open.
- #735 holds at 121 functions over seven parameters: 70 are FFI exports, which keep positional lists while their C++ callers remain. Its movable inventory is the 51 helpers and commands, and all but two commands sit in unfinished files, so the count falls as the remaining worksets pass.
- Residue in finished files rose with the files finished since merge 20 (missing comment lines 135 to 161). Quick's residue gate stops further growth; [closure](#cleanup-work-order) owns the rest.
- #736's classification left about 1,430 movable lines of 20,053; about 600 remain under the movable rule, and the rest retires with pool and reader owners ([work order](#cleanup-work-order) item 4).
- Full phase-3 planning is not triggered, and near-full reproduction is not met.

Reproduce the census with `python3 tools/port-metrics.py --whole-fork` (metric v2).

| Measured quantity | Merge 32 |
| --- | ---: |
| Original C++ remaining | 221,957 |
| Fork source glue, including generated records | 32,947 |
| Compiled source and generated records C++ | 254,904 |
| Tracked tools C++ evidence | 15,016 |
| ABI-generated non-record C++ evidence | 5,358 |
| Measured fork C++ series | 275,278 |
| Original C++ retired | 62,398 of 284,355 (21.94%) |
| Simulation C++ retired | 41,231 of 61,148 (67.43%) |
| Historical regex exports / callback slots | 1,508 / 3,210 |
| Shared-accessor / rebound-callee / orphan-export baseline rows | 47 / 244 / 27 |
| Exports held only by uncalled members (baseline only shrinks) | 23 |
| Rust / tooling code lines | 182,354 / 66,055 |

| Source area / phase relationship | Original | Retired | Remaining | Fork glue |
| --- | ---: | ---: | ---: | ---: |
| Simulation logic (phase 1) | 47,424 | 37,992 | 9,432 | 5,909 |
| Simulation storage (phase 2) | 13,724 | 3,239 | 10,485 | 1,385 |
| NewGRF, save/load, script API and network (phase 3) | 51,794 | 13,074 | 38,720 | 3,354 |
| Support, graphics and viewport (phase 4) | 32,602 | 980 | 31,622 | 548 |
| GUI and platform (phase 5) | 64,120 | 415 | 63,705 | 465 |
| Vendored (phase 6) | 55,148 | 2,317 | 52,831 | 0 |
| Data tables | 17,295 | 4,367 | 12,928 | 14 |
| Upstream tests and fork test glue | 2,248 | 14 | 2,234 | 4,787 |
| Boundary layer | 0 | 0 | 0 | 16,485 |

Coverage remains bounded by the existing saves, NewGRFs, scenarios and retained native fixtures; entry witnesses are not branch coverage. New gaps are on #156. #503 deep checks remain user-requested.

### Cleanup stream inventories

The comment, integer, wrapper, constant and literal rows are analyzer estimates from `tools/code-standards-inventory.py`, not automatic source findings. Root adds the next column at its stocktake ([selection](planning.md#selection)).

| Stream | Inventory | Merge 32 |
| --- | --- | ---: |
| #734 | Workset files with a finished pass | 99 of 319 |
| #734 | Workset lines in finished files | 66,913 of 190,751 (35.1%) |
| #734 | Explanatory comment lines missing inside matched ported bodies | 2,250; 161 in finished files |
| #734 | Bare-integer parameters and fields whose concept has a named type | 1,663; 171 in finished files |
| #734 | Transparent wrapper types / named constants | 309 / 3,662; 58 / 716 in finished files |
| #734 | Literal sites standing for a named value | 12; 0 in finished files |
| #735 | Functions over seven parameters | 121 (70 FFI, 25 helpers, 26 commands) |
| #735 | `too_many_arguments` ratchet rows / `allow` sites | 48 / 10 |
| #736 | Fork C++ evidence under `tools/` / `src/tests`, code lines | 15,016 / 4,787 |
| #738 | Rule corpus lines | 3,266 |
| None | Ruff rows | 286 |

Counting commands:

- **#734:** reconcile the issue workset manifest with merged complete-file pass PRs, then count current files with `wc -l`; a deleted finished entry remains in the denominator, and mechanical caller edits do not finish a file. Run `python3 tools/code-standards-inventory.py [--head REV]` for the tree and with the finished paths for that subset.
- **#735:** use the tracked long-signature inventory counter, `migration/clippy-debt.json` (`too_many_arguments` keys) and `git grep 'allow(clippy::too_many_arguments' -- rust/`.
- **#736, the census baseline rows and the Clippy and Ruff rows:** from `python3 tools/port-metrics.py --whole-fork`, `python3 tools/orphan-exports.py --report-uncalled`, `wc -l` of the three baseline files, and the debt files (keys are `sites` entries, diagnostics the sum of their list lengths). Expanded exports: `growth_inventory` in `tools/port-metrics.py`.
- **#738:** `cat AGENTS.md $(git ls-files 'docs/*.md' 'docs/**/*.md' | grep -v -E '^docs/(roadmap.md|components/)') | wc -l`.

### Speed

The [merge-32 profile](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6810087211) on clean `985abab91f` used `simulate --instructions --jobs 2`. All seven canonical exit saves and full logs match. The driver exits 1 on its anchor identity guard for every scenario, because `rust/openttd-kernels/Cargo.toml`, a file the identity hashes, gained a `native-debug` feature; nothing else in the identity differs from merge 20. The merge-32 projections below are therefore computed from the report's instruction counts and the retained anchors by the [formula](speed.md#instruction-projections), not printed by the driver; the next quiet pass re-anchors ([#275](https://github.com/fetch-laboratories/openttd-rust/issues/275)). The profile's output is retained in locked `t3-656b5a85-5eedda484b0f05a1`, the previous tip for attribution at the next profile.

| Scenario | Original Ir | Candidate Ir (merge 32) | Projection (merge 32) |
| --- | ---: | ---: | ---: |
| TGP | 2,265,528,237 | 2,789,433,667 | 1.173339x |
| Opus | 21,773,343,583 | 27,485,703,563 | 1.345115x |
| Grok | 20,969,727,163 | 26,243,562,014 | 1.346652x |
| P1996 | 3,600,502,896 | 4,514,871,657 | 1.352193x |
| P2000 | 6,049,922,238 | 7,434,777,978 | 1.444531x |
| P2016 | 11,343,424,651 | 13,729,352,305 | 1.382593x |
| Drawing (diagnostic) | 2,149,015,961 | 2,154,418,251 | 1.004635x |

No policy projection rose 3% or entered its band: generation rose 1.04% and stays below its 1.235x band and 1.3x ceiling, and the play records stay on their anchor floor, the highest at 1.444531x against the 1.52x band and 1.6x ceiling. No wall pass is due, and these instruction projections are not new wall-time measurements.

## Course

- Execution order: the cleanup stage, then phase 3 readers, then phase 2 storage with each pool's save/load, then phases 4, 5 and 6 ([goal and phases](planning.md#goal-and-phases), [ADR 0003](adr/0003-work-order-cleanup-readers-storage.md)).
- Yardstick: net fork C++ removed, counting C++ under `tools/` and `src/tests` ([cost decisions](planning.md#cost-decisions), [ADR 0022](adr/0022-progress-is-net-cpp-removed.md)).
- Limits: [implementer slots](team.md#slots) and the [stocktake cadence](root-workflow.md#cadences).

## Phases

Phase numbers name content; the execution order sets the sequence. Phase 1 (simulation logic) exited at integration 63. The last navigation's position table, by source area and the phase that removes it, is in this file at `78553df0a1`.

| Order | Stage | Size now | Exit (end state) | Status |
| ---: | --- | --- | --- | --- |
| 1 | Cleanup | #734: 116 files finished at merge 44 (99 of 319 workset files and 66,913 of 190,751 lines at merge 32). #735: 121 long functions, 48 debt keys and 10 allows (merge 32). #736: 15,016 counted tools C++ lines and 4,787 fork test lines (merge 32), plus separately inventoried embedded C++. #737 and #738 are closed at their end states | #734: all 319 files finished and closure (work order item 6) done; #735 closes with it. #736: no file the [classification](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6804817013) calls movable remains; each remaining C++ file under `tools/` and `src/tests`, and any embedded C++, either checks the calling boundary itself or reaches a live native pool, reader or production caller, and names the owner or boundary whose retirement removes it ([evidence](porting.md#evidence)). The 65 pool and reader fixtures stay native until their owners retire them. Lint ratchets shrink per PR but set no exit, and the structural Clippy rows are outside it. Read each end state from the code and inventories, not from issue state | Current |
| 2 | Phase 3: readers (script API, NewGRF, network, content, admin, save/load outside pool chunks) | About 37,800 compiled lines: script API 13,252, network 10,628, NewGRF 5,184, and save/load 13,023 less the pool and map chunks. The [queued owners](#after-cleanup) estimate 4,600 to 7,000 net C++ removed | Readers run in Rust with evidence for their observable outputs. Every remaining body in these buckets has a named later owner: VM and binding shims phase 6, script and network GUI phase 5, pool chunks phase 2 | Next. Plan it in full before cleanup's last items are dispatched ([goal and phases](planning.md#goal-and-phases)) |
| 3 | Phase 2: storage, each pool with its save/load chunks, then the map | 11,899 compiled lines in storage, about 4,300 save/load chunk lines, the logic residue assigned to pool owners, and record field routes in the 16,759-line boundary layer | Rust owns every pool's records, slots, allocation, iteration, lifetime and save/load, then map allocation and remaining map bodies; no Rust-used field behind a native service | Queued; placement in the [reader and pool placement](design/reader-pool-placement.md) design |
| 4 | Support | About 32,200: support 21,191 and graphics and viewport 10,982, plus their data tables and startup | As in [planning](planning.md#goal-and-phases) | #3; planned when it becomes the next phase ([ADR 0005](adr/0005-plan-only-the-next-phase.md)) |
| 5 | GUI, drawing and platform | About 64,200: GUI 52,227 and platform 11,943, plus simulation and data-table drawing | As in [planning](planning.md#goal-and-phases) | As phase 4 |
| 6 | Vendored and final retirement | Vendored 52,831, plus whatever boundary, test and evidence C++ remains | Every original and fork-written C++ line retired and candidate native targets removed ([goal and phases](planning.md#goal-and-phases)) | As phase 4 |

Root's first actions on resume:
1. Follow the [resume checkpoint](#resume-checkpoint).
2. Drive each cleanup stream to its end state above. Check that #736's body lists its remaining batches; record delivered batches there.
3. Before the cleanup stage's last items are dispatched, spawn the phase-3 planner ([start-development](skills/start-development/SKILL.md) step 3).

## Cleanup work order

Each stream issue holds its batches, rules, proof and inventory. [Selection](planning.md#selection) ranks cleanup work by stream inventory. File ownership and file-set checks follow [team](team.md#ready-queue-and-pipelining); what a cleanup PR needs to merge is in [integration](root-workflow.md#integration).

Slots: while ready worksets remain, one [implementer slot](team.md#slots) runs #734/#735 file passes and the other runs items 2 to 4 in order. #738 runs as one docs agent outside the implementer count.

1. **#734/#735 file passes.** One PR per workset in the #734 manifest finishes every file it touches, applying the [code standards](porting.md#code-standards) to every line. Sizing, splits and single-file passes follow [sizing](planning.md#sizing); proof follows [cleanup proof](porting.md#cleanup-proof). Each pass reports its files' residue from `tools/code-standards-inventory.py`.

   Order, by fan-in (measured at `5af17025a3`). Delivered: the 03 remainder and its residue pass, and 23's remainder (#884).
   - (b) 13 Map/terrain (fan-in 90) with 23's type-bearing files. `map_func.rs`, `tile_map.rs`, the 13 leaves (#878) and 23's type-bearing files are finished; `tile_loop.rs`, `tilearea.rs`, `tree_cmd.rs` and `trees.rs` of workset 13, and 23's `native_test_support.rs` and `native_test_support/movement.rs`, are not yet in `migration/finished-files.txt`.
   - (c) 09 (59), 08 (58), 07 (52), 06 (45), 04 (38), 05 (27). The calls between 04 and 06 run both ways (06 to 04 in 2 files, 04 to 06 in 1), so strict fan-in governs.
   - (d) The rest by fan-in: 16 (67), 21 (62), 15 (42), 18 (37), 10 (33), 11 (28), 14 (27), 17 (26), 12 (25). The leaf worksets 22 (17), 20 (6) and 19 (1) go last. F's closure stays last.

   Twenty-four workset files exceed the single-file pass size; the largest are `station_cmd.rs` (4,868), `orders.rs` (4,841), `town.rs` (3,373), `train.rs` (3,121), `industry.rs` (2,874) and `company.rs` (2,698). #734 reports progress in lines finished as well as files.
2. **Tools maintenance.** #938 (three `tools/ci.py` defects in `enqueue` and `queue-wait`) is the next tools PR; fold later small tools items into it ([batching](root-workflow.md#integration)). The remaining #736 end-state classification still names each retained evidence boundary; new inventory visibility is not C++ deletion.
3. **#857**, delivered in #871.
4. **#736 test evidence**, one PR per batch where the batch fits the [sizing](planning.md#sizing) bound. Each item follows the [movable rule](porting.md#tests-in-rust-and-c) and the oracle rule on #736, and its records claim only what its tests observe; T follows W and F only where a named remaining consumer requires it. Remaining, in the [classification](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6804817013)'s order:
   - the authentication probe (#936), the train edge and aircraft range probes, and the ScriptList owner and cargo sections, each oracle with an `oracle-capture.py` recipe ([ready queue](#ready-queue));
   - **#736's end-state inventory,** the stream's last PR, confirming the classification.

   The Catch2 files that delete whole, `src/tests/train.cpp` and the ScriptList persistence section fail that rule and stay native: the Catch2 files retire with the C++ they exercise, `train.cpp`'s PBS and reservation cases with #725 and its speed case with #656. The console fixtures and Catch2 files that reach live pools (65 files, 14,273 counted lines) retire inside the ports that retire their pools or readers. The 33 files and 3,909 lines of retained boundary evidence leave with their boundaries.
5. **#738**, delivered; the stream is closed at its end state.
6. **#734 closure**, after the last workset pass: private wrapper fields, generator support, and removal of the raw adapters and aliases. Seventeen Rust files (2,944 lines; `test_tables.rs` is 1,735 of them) are in no workset. Each joins one before closure, the type files with F ([#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6767381933)). The end state and later stocktakes and steering measure with `tools/code-standards-inventory.py`. The closure residue sweep takes [the recorded residue list](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6789587702) unless a later pass touches the file; its macro sites are #892's. #892 rides remaining file passes, then that sweep and closure; no unrecorded production assertion macro site remains at closure, measured as #892 specifies.

   #734 also owns the `unwrap`/`expect` and release-overflow fidelity gaps recorded in #156: remaining file passes handle their selected files, and closure handles recorded residue in already-finished files. Compare each with the original's preconditions and observable failure behaviour; an occurrence alone is not a defect. This ownership does not expand #892's macro scope or add a census, lint rule or gate.
7. **#893 cleanup-exit evidence**, after closure and before the first phase-3 reader dispatch: run the coverage measurement, seeded-fault pilot and all-member deep check. Pause implementers for the two host measurements. Read the report and handle findings under [deep checks](deep-checks.md#policy) before dispatch. Phase-3 planning follows its normal trigger and does not wait for these results.

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

Then storage, each pool with its save/load chunks, redone from current base with each closed draft's branch as reference (the branches are kept on origin, and each PR records its head and state): #656/#598/#599 vehicle (#714), #634 station/road-stop/cargo (#700), #638/#498 small pools (#723), #635 town/industry/subsidy (#704), #637/#497 remaining engine pools, #717 attachment, #725 YAPF, #906 orders, #907 companies and groups, then the map (#565's remainder). #716 tracks the all-pools closure. Each pool brief follows the [reader and pool placement](design/reader-pool-placement.md) design.

## Trackers and exclusions

- #156 (coverage gaps), #275 (speed), #724 and #710 (host) stay open trackers.
- #500 is delivered by #734; #168 and #199 attach to #737 and the owners that touch those records. #737 has closed and #500 is still open, so these three await root's placement ruling.
- #693 keeps the cancelled reconnect failure and two completed original/original runs that did not reproduce it; the cause is unresolved and its two-run authorization is exhausted. Diagnosis returns with #730's network owner after cleanup, or sooner if hangs exceed one rerun in ten Full requests at a stocktake (provisional).
- #503's coverage continuation runs last.
- #939 lists 35 rules that a script, hook or lint could hold instead of a doc, each with the doc lines it retires. Root takes them as tools PRs, best first.
- #860 lists the development-process changes the user approved: the merge queue ([ADR 0019](adr/0019-merge-queue.md)), the per-merge builds ([ADR 0018](adr/0018-builds-before-every-merge.md)), CI scenario comparison ([ADR 0020](adr/0020-scenario-comparisons-on-ci.md)), direct reviewer and implementer messages, and the wind-down-only checkpoint ([ADR 0026](adr/0026-roadmap-commit-timing.md)).
- #896 (paid CI minutes) stays with its author for the one-day weighted-minutes and network-failure follow-up. The simulation shards are back on Blacksmith; which jobs run on paid runners is the user's decision ([ADR 0009](adr/0009-paid-runner-assignment.md)).

Excluded, with issue placement and return conditions last rechecked at merge 20; near-full reproduction and fork release-source publication triggers are not met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Latest steering actions

Steering 34 is the latest; its findings, standing corrections and actions are in commits `3a2d52b870` and `5600800c76`. Its actions are now rules in the [PR lifecycle](workflow.md#pr-lifecycle), [team](team.md), the [implementer brief](implementer-brief.md#finishing-a-file-pass) and [porting](porting.md#tests-in-rust-and-c). Steering 35 judges the provisional items: the planner refresh trigger ([ready queue](team.md#ready-queue-and-pipelining)), #901's false-failure threshold, and whether defects escaped once the acceptance review stage was dropped.

## Ready queue

Undispatched work orders ([team](team.md#ready-queue-and-pipelining)). The open component PRs are #936 and #937, within the [branch limit](root-workflow.md#integration). Entry 2 is the file-pass slot's, and entries 3 and 4 are the other slot's ([work order](#cleanup-work-order)); a free slot takes its own top ready entry, and the other slot's when it has none. File sets were checked at merge 40 (`24c0fc242b`) with `python3 tools/retrospective.py readiness`; since then #926, #934, #932 and #935 have merged, so a planner re-checks them at merge 44 before dispatch ([resume checkpoint](#resume-checkpoint)). Sizing, fan-in and held callers are on [#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6812366384), with the file table in the [merge-35 analysis](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6811109267); the #736 estimates are on [#736](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6812356028). Every entry keeps its carries and proposals in its [cleanup records](rust-migration.md#quick-cleanup-checks) file, not in the PR body.

1. **8a** (vehicle definition file passes) is dispatched as #937. Before review, run the fixtures that call these files' exports, which keep their positional lists: `src/tests/engine.cpp` (road type availability), and `src/tests/train.cpp`, `src/tests/follower_fixture.hpp` and `tools/migration/train-edge-comparison.cpp` (`openttd_rust_train_state_new` and `_destroy`). No fixture calls the two airport property exports directly; `src/rust/airport_definitions.cpp` reaches them when a NewGRF loads, so run a NewGRF loading comparison that defines an airport.
2. **9a, #734/#735 pathfinding file passes: `pathfinder_func.rs`, `follow_track.rs`, `node_hash.rs`, `yapf_node.rs` and `water_regions.rs`.** The callees of `pbs.rs` and the three YAPF files, so they pass first in what is left of workset 09. Include #892's assertion class and #734's recorded fatal-path fidelity gaps in the every-line pass: one macro site in `node_hash.rs`, and three `unwrap`/`expect` sites each in `follow_track.rs` and `water_regions.rs`.
   - Size: 1,259 raw lines, 11 `extern "C" fn` items and 78 primitive-integer declarations. About 1,825 changed lines at 9m's realized 1.45 changed lines per raw line, the ratio of a shared callee with outside callers; about 1,180 at 20c's 0.94. With `pbs.rs` (771 raw lines) the pass would be about 2,945 at 1.45, so `pbs.rs` leaves at the file boundary. Refine the size at the first checkpoint. Above about 1,000 changed lines it takes one reviewer per function group.
   - Its wait on the 9m pass (#926) has resolved: #926 merged as merge 41. These files call `movement.rs` in 12 places and adopt its typed forms. At its head `20357960f4`, #926 edited none of the five files, but eight files that call them (`station_cmd.rs`, `town.rs`, `train.rs`, `train_reservation.rs`, `ship_control.rs`, `rail_cmd.rs`, `road_yapf.rs` and `test_tables.rs`). Entry 1 (#937) touches none of the five; both keep a raw adapter in `train.rs` and `train_reservation.rs`, on different lines.
   - The pass converts the callers in finished files itself, as mechanical edits listed in the PR body: `station_service.rs` (one `follow_track::platform_length_full` call) and the test module of `town.rs` (`water_regions::Context` and `Regions`, and two export calls that move to the typed functions). Unfinished callers keep a raw adapter only where a signature changes: `pbs.rs`, `rail_yapf.rs`, `road_yapf.rs`, `ship_yapf.rs`, `train.rs`, `train_reservation.rs`, `ship_control.rs`, `water.rs`, `landscape_generation.rs` and `native_test_support.rs`. Leave `test_tables.rs` alone beyond a changed signature.
   - `follow_track.rs` drops its ten raw adapter calls (`reverse_diag_dir_raw` nine times and `station_rail_raw` once) for the typed forms the finished `direction_func.rs` and `station_map.rs` define. `water_regions.rs` has seven bare `tile: u32` sites, and the bodies behind its ten exports move to typed Rust-internal functions.
   - `src/tests/water_regions.cpp` and `water_regions_protocol.hpp` stay native ([ruling](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6812253795)) and call this pass's exports, which keep their positional lists. Run them, the follower and YAPF Catch2 cases and `tools/simulation/pbs-console.cpp` before review.
3. **#736 ScriptList owner and cargo sections** (work order item 4, the [classification](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6804817013)'s item 6 less the persistence section): move the `OwnerGaps` and `CargoGaps` sections of `tools/migration/script-list-comparison.cpp` into Rust oracle tests. The file and its driver stay for the valuation, persistence, filter, load, metamethod and nested sections, which open a VM.
   - It passes the movable rule on current counts. The probe is unchanged since the [estimate](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6812356028) (last edit `7843e5b780`): `OwnerGaps` is 59 non-blank, non-comment lines, and the cargo block between the prelude and the shared record helpers is 145, of which `ListToken` (8) stays for the VM sections; the estimate counted 119 of it as the section. So 178 to about 200 counted C++ lines deleted against an estimated 5 to 25 counted recipe lines: both sections drive the public `ScriptList` API or the extracted collector on the pinned original as they stand, and the comparison is `probe.compare` in Python. These are line counts, not `port-metrics.py`'s; report its two counts at the first checkpoint. If under 150 counted lines move, stop and tell root.
   - Size: about 1,100 to 1,500 changed lines outside oracle text, from the delivered probes' 6.3 to 8.5 changed lines per counted line (#929 realized 8.5 at acceptance: 1,420 for 168); the classification estimated 1,560. One new recipe, `script_list_owner`, which also captures the cargo records.
   - Files: `tools/migration/script-list-comparison.cpp` (the two sections, `CargoProbeList`, `GapCargoCollector`, `CargoToken`, the private-counter access and the owner-only helpers `keys`, `Fill` and `Drain` leave; `Hex`, `Content`, `Record`, `Require`, `TestList` and `ListToken` stay) and `main`; `tools/script-list-comparison.py`, whose `cargo-reducer.hpp` extraction (about 70 lines) leaves with the cargo section; `tools/oracle-capture.py` and one patch under `tools/oracle-recipes/`; separate Rust test modules with their oracle text, with a one-line hook in the finished `station_cargo.rs`, so that the residue gate stays clean, and in `script_list.rs` (workset 19, unfinished); a record under `migration/evidence-replacements/`. The probe's row in `migration/harness-inventory.json` stays.
   - Held callers: none leaves. The cargo collector's six exports (`openttd_rust_cargo_init`, `_finish`, `_packet`, `_origin`, `_share` and `_plan`) are called by production `src/script/api/script_stationlist.cpp`, and `openttd_rust_list_token` by the VM sections that stay, so no export becomes Rust-internal and `tools/uncalled-export-baseline.txt` has no row for this item.
   - Both sections observe the candidate through the fork's C++ `ScriptList` and `ScriptStationList_Cargo` adapter classes; a Rust test calls the owner beneath them. The VM sections that stay still construct those classes. If the first checkpoint finds an adapter observation that no retained section carries, that case stays native and names its boundary (the phase-3 script API reader); record it, and claim only what the test observes.
   - Follow the oracle rule on #736 and the delivered probes' pattern (#915, #929): freeze the original's observations before deleting the producers. `docs/components/economy.md` and `network-script.md` name this comparison's cargo mode as a check; update those lines to the Rust tests.
   - Ready. At merge 40 no open PR edited the probe, `script_list.rs` or `station_cargo.rs`. #935 has merged, so preflight's compile-only mode skips the generated bindings, compiles what is left of the fixture and compiles it with Clang too: the shrunken file must be clean under `-Wmissing-field-initializers`. The authentication probe (#936) and entry 4 add recipes to `tools/oracle-capture.py`; those additions merge by base update.
4. **#736 train edge and aircraft range probes** (work order item 4, the [classification](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6804817013)'s item 4 less `src/tests/train.cpp`): retire `tools/migration/train-edge-comparison.cpp` with `tools/train-edge-comparison.py`, and `tools/migration/aircraft-range.cpp` with `finite_range_probe` in `tools/simulation/aircraft.py`, into Rust oracle tests.
   - Both pass the movable rule ([estimates](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6812356028)): 296 counted C++ lines deleted (154 and 142) against an estimated 25 to 45 counted recipe lines, because each probe's reference shell and case loop run on the pinned original as they stand and only the printing is new. `src/tests/train.cpp` fails the rule and stays native: do not move it.
   - Size: 352 raw C++ lines and about 100 lines of Python deleted; two new recipes, `train_edge` (four extracted bodies, 20,000 curve and 20,000 reversal trials) and `aircraft_range` (the event bodies `aircraft.py` extracts, 400 cases). About 1,900 to 2,500 changed lines outside oracle text, from the two delivered probes' 6.3 to 8.5 changed lines per counted line. Report counted C++ deleted, counted recipe C++ added and changed lines at the first checkpoint. If it projects past about 2,000, split at the file boundary, the train edge probe first.
   - Files: the two probes and their drivers; `tools/oracle-capture.py` and two patches under `tools/oracle-recipes/`; separate Rust test modules with their oracle text, each with a one-line hook in `train.rs` and `aircraft.rs`; rows in `test_tables.rs`; a record under `migration/evidence-replacements/`; `tools/uncalled-export-baseline.txt`. The probes' `native_sources` rows in `migration/harness-inventory.json` stay, as the two delivered probes kept theirs. `aircraft-boundary.hpp`, `controller-orders.hpp` and `fixture_map.hpp` stay: the ABI probe includes them.
   - Follow the oracle rule on #736 and the delivered probes' pattern (#915): freeze the original's observations before deleting the producers, and claim in the record only what the tests observe. The train edge probe draws 16 random vehicles for each trial, so print only what the swap changed and the event order, not every vehicle's state, and report the oracle's bytes at the first checkpoint.
   - With the probe gone, `Train::GetCurveSpeedLimit` has no caller. Delete the member, its declaration and its baseline row, and make `openttd_rust_train_curve_limit` Rust-internal if nothing else calls it, as the link graph probe did for `LinkGraphSchedule::Dequeue`.
   - Its wait on the 9m pass (#926) has resolved: #926 merged as merge 41, after editing `train-edge-comparison.cpp`, `train.rs`, `aircraft.rs` and `test_tables.rs` at its head `20357960f4`. Capture from the probe as #926 left it. The authentication probe (#936) adds a recipe to `tools/oracle-capture.py` and narrows rows of `tools/uncalled-export-baseline.txt`, and entry 3 adds a recipe; those additions merge by base update. Entries 1 and 2 each keep a raw adapter in `train.rs`, and entry 1 (#937) leaves `openttd_rust_train_state_new` and `_destroy`, which the probe calls, with their positional lists; the hook line merges beside them.

**Later.** In the file-pass slot, 9s (`signal.rs`, 1,271 raw lines) waited only on #926, which has merged. 9p (`pbs.rs`, 771 raw lines, about 1,120 changed lines at 1.45), the pass for `ship_yapf.rs` and `road_yapf.rs` (1,947 raw lines, split or not by 9a's realized ratio) and the single-file pass for `rail_yapf.rs` (1,903 raw lines) wait on 9a, and the last on 9p too ([order](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6811109267)). In workset 08 only entry 1's eight files are free of 09: `aircraft.rs`, `disaster.rs`, `effect.rs`, `ground.rs`, `ground_vehicle.rs`, `road.rs`, `ship_control.rs`, `train.rs`, `train_reservation.rs` and `vehicle_effect.rs` (16,020 raw lines) each call `movement.rs`, and `train.rs`, `train_reservation.rs` and `ship_control.rs` also call 9a's, 9p's and 9s's files. They follow workset 09, the four over 1,500 lines (`train.rs`, `road.rs`, `aircraft.rs` and `train_reservation.rs`) as single-file passes. In the other slot, after entries 3 and 4, #736 has only its end-state inventory left, and #938 is the only undispatched tools item.

Full phase-3 planning remains untriggered: at merge 40, 193 of #734's 319 files, in 19 worksets, were in no entry, and #736's end-state inventory has none either.

## Resume checkpoint

**Paused at the user's request (lenient wind-down, 2026-11-12).** Base code is `93eedd81b2` (#935, merge 44); this docs commit is the pause checkpoint. The merge queue drained and is empty: #926, #934, #932 and #935 merged as merges 41 to 44. #932 and #935 were each removed once and requeued once, unchanged, as the wind-down allowed: #932 for a `network-protocol` hang in run 43147704245 ([#693](https://github.com/fetch-laboratories/openttd-rust/issues/693#issuecomment-6813852347)), #935 for a reference-side `content-protocol` miss in run 43150720812. No rollback was performed and no confirmed base regression is identified. The push runs on `93eedd81b2` (Quick 43153736985, baseline validation 43153737112, `CI - Build` 43153737108) completed with `success` after the pause. No new implementation, repair, review, build, simulation, CI request or retry is authorized until resume. An independently confirmed base regression still follows the automatic rollback rule.

Agent: /root | Model: claude-fable-5-2 | Reasoning effort: high

| Remaining PR | Pushed head | Checks, jobs and state at pause | Next action |
| --- | --- | --- | --- |
| #936, #736 authentication probe (draft) | `86a683d198` | Returned to draft. `/root/review-936-all` raised two findings (adapter observations; the uncalled `GetAuthenticationMethod` chain); their fixes are `041f3a1280`, `b9d1369f49` and `86a683d198`, pushed as a logged checkpoint with no preflight stamp. Preflight passed 15 of 16 at `b9d1369f49`; the failing growth step passed alone once the proposal was added. `native` 43149400253 passed, on the old head `5363b6f9c3` only. No job runs. Checkout `t3-bc6721f0`, clean and pushed. | Merge base (merges 41 to 44), full preflight, `python3 tools/ci.py ready 936`, `native` at the head, then one re-check of findings 1 and 2 naming the three commits. That report must accept the 22 lines of test growth proposed in `migration/cleanup-records/cleanup-736-auth-probe.json`. |
| #937, 8a vehicle definition file passes (draft) | `bd44b53777` | Code for all eight files, the override-manager boundary fix and the caller conversions are pushed: 1,386 changed lines, one lint carry in its record file, growth holding. Cargo checks, 184 workspace tests and the Clippy, lint-policy, growth and residue gates passed locally. Nothing else ran. No job runs. Checkout `t3-c294893f`, clean and pushed, with `openttd_test` and `openttd_rust_abi` built at the head. | Merge base and convert `movement.rs`'s calls into these files, now that #926 has merged; run the entry's fixtures; full preflight; `native`; review in the two groups its PR body lists; queue. |

All implementers, planners and routine agents of this session are stopped, and root holds no PR watch. Implementation resumes on `claude-opus-5-5`, medium, and planning on `claude-fable-5-2`, medium; at most two implementers work at once and six component branches stay unmerged.

Verified CI state at the pause: `CI_ON_DEMAND`, `CI_MERGE_QUEUE` and `MIGRATION_CCACHE` are `true`; `CI_LINUX_SIMULATION` is `blacksmith-4vcpu-ubuntu-2404`; `CI_LINUX_COMPARISONS`, `CI_LINUX_PLATFORM` and `CI_MACOS_HEAVY` stay unset. The hooks of #927 are installed in the shared clone (`python3 tools/git_hooks.py install`).

| Other work | State and resume boundary |
| --- | --- |
| Ready queue | The section above was written at merge 40 and was not refreshed. Its entry 1 (8a) is dispatched as #937. Entry 2 (9a pathfinding passes) and entry 4 (train edge and aircraft range probes) waited only on #926, which has merged, so they are ready once a planner re-checks their file sets at merge 44. Entry 3 (the #736 ScriptList owner and cargo sections) was ready and is undispatched. None was started. Refresh the section before dispatching. |
| Next tools PR | #938: three small `tools/ci.py` defects in `enqueue` and `queue-wait`. Undispatched. |
| Platform cadence | Passed after merges 33 and 38. Due now, after merge 43; not started. |
| Stocktake and instruction profile | Last at merge 32. Due now, after merge 44; not started. The profile driver exits 1 on the anchor identity guard until a quiet pass re-anchors (#275), so the merge-32 medians were computed by hand. Notes kept for the stocktake: #926 took five reviewer sessions after its source review and five `native` failures; fixture churn was first found by CI; reviewers worked in an implementer's shared checkout; `t3_background_wait` returned at once (#710); Windows x64 took 17 minutes in Full run 43120192348; the #923 compile step took 163 to 289 seconds on a loaded host and needs an idle figure. |
| Phase-3 planning | Not triggered. |

Flakes and limits carried forward:

- #693: this session saw three network-timing failures with no code cause, each cleared by one unchanged retry: a `network-protocol-loopback` hang on #916, the `network-protocol` hang that removed #932 (run 43147704245) and the reference-side `content-protocol` miss that removed #935 (run 43150720812). Earlier windows recorded 2 such failures in 42 simulation-tier runs, then none among 63 native and queue requests at the merge-20 stocktake; the windows are not combined into a rate. No source or oracle change and no new local network diagnosis is authorized by this checkpoint. Keep `.local/retained-network-693` and the linked artifacts; no candidate or base regression is inferred from a timeout.
- Metric scope completeness stays with #736's end-state inventory. No later phase is selected. #898 is a partial evidence transfer; native ownership, loader and error observations, pool and ABI identity and retained fixture consumers remain as its accepted report states.

Retained state for housekeeping:

- `python3 tools/housekeeping.py --apply` ran at this wind-down and removed 21.8 GB: 23 checkouts merged into `origin/rust-migration`, with nothing dirty or unpushed among them. The SSD has 53 GB free.
- Keep #936's checkout `t3-bc6721f0` and #937's `t3-c294893f`: each is its open PR's one checkout.
- Keep the locked merge-32 profile checkout `t3-656b5a85` until the merge-44 profile is recorded on #275. Preserve all current speed, deep and evidence anchors; the speed anchors live in the main checkout's `.local/speed-pass`.
- The profile checkouts of merges 8 and 20 and of integrations 96, 98, 100 and 143 (`t3-ee66f61c`, `t3-9bfeabfd`, `t3-8ed2642d`, `t3-62945724`, `t3-a6181dc7`, `t3-d3ff7564`) were released on 2026-11-12 (#924) and survive only by the tool's 12-hour activity rule; a later run may remove them.
- The list below is carried forward unchanged as preservation guidance from earlier checkpoints; it does not assert that every old checkout still exists. `/steer` may reconcile stale references, preserving uncommitted or unpushed work and evidence.

- Keep the planning checkout `t3-20fec9ce` with its private plan, and origin `preserve/housekeeping-foundation-6ab9c20e5d`. Preserve every checkout with uncommitted or unpushed work.
- Keep `.local/worktrees/root-profile-125`, `.local/worktrees/root-profile-119` and `.local/worktrees/root-profile-113` with accepted reports, raw Callgrind and frozen runtimes. Keep `t3-a490295e` (profile 131, clean `305ac20c28`) with its HDD `.local` outputs. Profile 119 measures clean `4d3a1d1d41`, not integration 120.
- Keep main `.local/integration-records/`, `.local/speed-pass/` anchors, frozen runtimes and the retained simulation evidence.
- Keep `t3-f4cf7a0b` (one unique unpushed commit, `125ea2440a`). Leave the staged 11-08 trees `t3-03c1e2da`, `t3-1ddaadfd`, `t3-5575ebe4`, `t3-83082b90`, `t3-c68d914a` and `t3-e0361d8c` untouched.
- Leave dirty or staged trees untouched: `t3-3b491ddb` (five files), locked #565 WIP `t3-431c84b4`, locked native evidence `t3-d78d5f8b`, `t3-32cf4edc` (`tools/simulation/roads.py` staged) and `t3-e130d5a3` (`migration/saves/README.md` staged).
- Keep protected `.local/worktrees/root-docs-72` and locked `.local/worktrees/profile-after*`. Do not push stale root-docs guidance over current main. Keep the pinned original unchanged.
