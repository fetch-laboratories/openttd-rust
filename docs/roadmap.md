# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place ([records](workflow.md#records)); `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands

The stocktake at merge 47 (`6d8a24a401`) is complete: steering 35 measured the census and inventories, and root added the platform cadence, the instruction profile and #892's counts on resuming. The other column is merge 32 (`985abab91f`), the stocktake before it, 15 merges earlier. The next stocktake follows merge 59 and the next platform cadence merge 52.

- The cleanup stage retires no original C++, so that share has not moved from 21.94%. Fork C++ fell by 200 lines, all of it #736 probes. Tooling grew by 7,922 lines; 7,189 of them are the transcript review's three tools PRs (#942 to #944).
- #734 has 116 of 319 files finished, holding 76,906 lines, about 40% of the workset. Root's last session (9.5 hours) merged five file passes, which finished 17 files and about 10,000 lines. About 114,000 lines remain.
- Steering 35's audits compared every body changed by five file passes and draft #937 with the original and found no behaviour difference on reachable input. They found about 31 services written for one message, and other residue, in finished files ([list](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6834079997)). The analyzer reported none of it: of its roughly 50 rows on those files, 2 were real. The analyzer rows below are a candidate list, not a count of residue.
- #736's last transfer split at the file boundary into the train edge probe (#946) and the aircraft range probe (#950). The ScriptList sections stay native, and the end-state inventory is a table refresh, not a PR ([ruling](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6834080910)).
- #735 holds at 120 functions over seven parameters; 70 are FFI exports, which keep positional lists while their C++ callers remain.
- The candidate's instruction count rose 3.44% on Opus and 3.80% on Grok since merge 32, in road-vehicle path finding, where #926's `movement.rs` pass left the track-status procedures out of line. The anchor floor hides most of it from the projections ([speed](#speed)); #952 restores the inlining.
- The platform cadence passed at merge 47 (run 43319577655).
- No rollback has happened since merge 32, and no confirmed base regression is open. Full phase-3 planning is not triggered, and near-full reproduction is not met.

Reproduce the census with `python3 tools/port-metrics.py --whole-fork` (metric v2).

| Measured quantity | Merge 32 | Merge 47 |
| --- | ---: | ---: |
| Original C++ remaining | 221,957 | 221,955 |
| Fork source glue, including generated records | 32,947 | 32,915 |
| Compiled source and generated records C++ | 254,904 | 254,870 |
| Tracked tools C++ evidence | 15,016 | 14,855 |
| ABI-generated non-record C++ evidence | 5,358 | 5,353 |
| Measured fork C++ series | 275,278 | 275,078 |
| Original C++ retired, of 284,355 | 62,398 (21.94%) | 62,400 (21.94%) |
| Simulation C++ retired, of 61,148 | 41,231 (67.43%) | 41,233 (67.43%) |
| Historical regex exports / callback slots | 1,508 / 3,210 | 1,506 / 3,205 |
| Shared-accessor / rebound-callee / orphan-export baseline rows | 47 / 244 / 27 | 47 / 244 / 27 |
| Exports held only by uncalled members (baseline only shrinks) | 23 | 22 |
| Embedded C++ outside the census: oracle recipes | not measured | 280 |
| Rust / tooling code lines | 182,354 / 66,055 | 184,124 / 73,977 |

| Source area / phase relationship (merge 47) | Original | Retired | Remaining | Fork glue |
| --- | ---: | ---: | ---: | ---: |
| Simulation logic (phase 1) | 47,424 | 37,993 | 9,431 | 5,904 |
| Simulation storage (phase 2) | 13,724 | 3,240 | 10,484 | 1,384 |
| NewGRF, save/load, script API and network (phase 3) | 51,794 | 13,074 | 38,720 | 3,354 |
| Support, graphics and viewport (phase 4) | 32,602 | 980 | 31,622 | 548 |
| GUI and platform (phase 5) | 64,120 | 415 | 63,705 | 465 |
| Vendored (phase 6) | 55,148 | 2,317 | 52,831 | 0 |
| Data tables | 17,295 | 4,367 | 12,928 | 14 |
| Upstream tests and fork test glue | 2,248 | 14 | 2,234 | 4,787 |
| Boundary layer | 0 | 0 | 0 | 16,459 |

Coverage remains bounded by the existing saves, NewGRFs, scenarios and retained native fixtures; entry witnesses are not branch coverage. New gaps are on #156. #503 deep checks remain user-requested.

### Cleanup stream inventories

The comment, integer, wrapper, constant and literal rows are analyzer estimates from `tools/code-standards-inventory.py`. Their "in finished files" parts rise as files finish and are mostly rows a reviewer would dismiss. Root adds the next column at its stocktake ([selection](planning.md#selection)).

| Stream | Inventory | Merge 32 | Merge 47 |
| --- | --- | ---: | ---: |
| #734 | Workset files with a finished pass, of 319 | 99 | 116 |
| #734 | Workset lines in finished files, of 190,751 | 66,913 (35.1%) | 76,906 (40.3%) |
| #734 | Explanatory comment lines missing inside matched ported bodies | 2,250; 161 in finished files | 2,208; 205 |
| #734 | Bare-integer parameters and fields whose concept has a named type | 1,663; 171 in finished files | 1,640; 193 |
| #734 | Transparent wrapper types / named constants | 309 / 3,662; 58 / 716 in finished files | 318 / 3,703; 64 / 847 |
| #734 | Literal sites standing for a named value | 12; 0 in finished files | 12; 0 |
| #734 | #892's production assertion macro sites (`debug_assert*!` / `assert*!` / `unreachable!`) | about 172 in 63 files at `dfb5c74469` | 169 in 59 files (6 / 70 / 93), by a regex scan, since no tool counts them |
| #735 | Functions over seven parameters | 121 (70 FFI, 25 helpers, 26 commands) | 120 (70, 24, 26) |
| #735 | `too_many_arguments` ratchet rows / `allow` sites | 48 / 10 | 45 / 9 |
| #736 | Fork C++ evidence under `tools/` / `src/tests`, code lines | 15,016 / 4,787 | 14,855 / 4,787 |
| #738 | Rule corpus lines | 3,266 | 4,051, with 49 ADR files added |
| None | Ruff rows | 286 | 286 |

Counting commands:

- **#734:** reconcile the issue workset manifest with merged complete-file pass PRs, then count current files with `wc -l`; a deleted finished entry remains in the denominator, and mechanical caller edits do not finish a file. Run `python3 tools/code-standards-inventory.py [--head REV]` for the tree and with the finished paths for that subset.
- **#735:** use the tracked long-signature inventory counter, `migration/clippy-debt.json` (`too_many_arguments` keys) and `git grep 'allow(clippy::too_many_arguments' -- rust/`.
- **#736, the census baseline rows and the Clippy and Ruff rows:** from `python3 tools/port-metrics.py --whole-fork`, `python3 tools/orphan-exports.py --report-uncalled`, `wc -l` of the three baseline files, and the debt files (keys are `sites` entries, diagnostics the sum of their list lengths). Expanded exports: `growth_inventory` in `tools/port-metrics.py`.
- **#738:** `cat AGENTS.md $(git ls-files 'docs/*.md' 'docs/**/*.md' | grep -v -E '^docs/(roadmap.md|components/)') | wc -l`.

### Speed

The [merge-47 profile](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6838255532) on clean `63d8a83ef9` (code as `6d8a24a401`) is the latest. All seven canonical exit saves compared equal. The driver exits 1 on its anchor identity guard until a quiet pass re-anchors ([#275](https://github.com/fetch-laboratories/openttd-rust/issues/275)), so these projections are computed from the report's instruction counts and the retained anchors by the [formula](speed.md#instruction-projections). The profile's output is retained in locked `t3-b49bc889-dbbee229158ddc99`, the previous tip for attribution at the next profile. The merge-32 profile stays in locked `t3-656b5a85-5eedda484b0f05a1` until #952 merges, because #952 compares against it.

| Scenario | Original Ir | Candidate Ir (merge 47) | Change since merge 32 | Projection (merge 32) | Projection (merge 47) |
| --- | ---: | ---: | ---: | ---: | ---: |
| TGP | 2,265,528,237 | 2,792,808,122 | +0.12% | 1.173339x | 1.176162x |
| Opus | 21,773,343,583 | 28,430,119,665 | +3.44% | 1.345115x | 1.345115x |
| Grok | 20,969,727,163 | 27,241,415,237 | +3.80% | 1.346652x | 1.352896x |
| P1996 | 3,600,502,896 | 4,575,637,657 | +1.35% | 1.352193x | 1.352193x |
| P2000 | 6,049,922,238 | 7,506,357,557 | +0.96% | 1.444531x | 1.444531x |
| P2016 | 11,343,424,651 | 13,867,687,196 | +1.01% | 1.382593x | 1.382593x |
| Drawing (diagnostic) | 2,149,015,961 | 2,154,345,524 | -0.003% | 1.004635x | 1.004635x |

No policy projection rose 3% or entered its band: generation stays below its 1.235x band and 1.3x ceiling, and the highest play record is 1.444531x against the 1.52x band and 1.6x ceiling. No wall pass is due, and these instruction projections are not new wall-time measurements.

A projection does not move while a count is below its anchor's, so the table shows the counts' own change. Opus, P1996, P2000 and P2016 are on that floor; Grok has left it. Opus's count is 0.6% below its anchor's (28,611,669,801). Root reads the count column at each stocktake and selects a fix for a rise of 3% or more in it, as #952 was selected (provisional).

## Course

- Execution order: the cleanup stage, then phase 3 readers, then phase 2 storage with each pool's save/load, then phases 4, 5 and 6 ([goal and phases](planning.md#goal-and-phases), [ADR 0003](adr/0003-work-order-cleanup-readers-storage.md)).
- Yardstick: net fork C++ removed, counting C++ under `tools/` and `src/tests` ([cost decisions](planning.md#cost-decisions), [ADR 0022](adr/0022-progress-is-net-cpp-removed.md)).
- Limits: [implementer slots](team.md#slots) and the [stocktake cadence](root-workflow.md#cadences).

## Phases

Phase numbers name content; the execution order sets the sequence. Phase 1 (simulation logic) exited at integration 63. The last navigation's position table, by source area and the phase that removes it, is in this file at `78553df0a1`.

| Order | Stage | Size now | Exit (end state) | Status |
| ---: | --- | --- | --- | --- |
| 1 | Cleanup | At merge 47. #734: 116 of 319 workset files and 76,906 of 190,751 lines finished. #735: 120 long functions, 45 debt keys and 9 allows. #736: 14,855 counted tools C++ lines and 4,787 fork test lines, plus separately inventoried embedded C++. #737 and #738 are closed at their end states | #734: all 319 files finished and [closure](#cleanup-work-order) done; #735 closes with it. #736: no file the [classification](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6804817013) calls movable remains, as [steering 35 amended it](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6834080910); each remaining C++ file under `tools/` and `src/tests`, and any embedded C++, either checks the calling boundary itself or reaches a live native pool, reader or production caller, and names the owner or boundary whose retirement removes it ([evidence](porting.md#evidence)). The 65 pool and reader fixtures stay native until their owners retire them. Lint ratchets shrink per PR but set no exit, and the structural Clippy rows are outside it. Read each end state from the code and inventories, not from issue state | Current |
| 2 | Phase 3: readers (script API, NewGRF, network, content, admin, save/load outside pool chunks) | About 37,800 compiled lines: script API 13,252, network 10,628, NewGRF 5,184, and save/load 13,023 less the pool and map chunks. The [queued owners](#after-cleanup) estimate 4,600 to 7,000 net C++ removed | Readers run in Rust with evidence for their observable outputs. Every remaining body in these buckets has a named later owner: VM and binding shims phase 6, script and network GUI phase 5, pool chunks phase 2 | Next. Plan it in full before cleanup's last items are dispatched ([goal and phases](planning.md#goal-and-phases)) |
| 3 | Phase 2: storage, each pool with its save/load chunks, then the map | 11,899 compiled lines in storage, about 4,300 save/load chunk lines, the logic residue assigned to pool owners, and record field routes in the 16,759-line boundary layer | Rust owns every pool's records, slots, allocation, iteration, lifetime and save/load, then map allocation and remaining map bodies; no Rust-used field behind a native service | Queued; placement in the [reader and pool placement](design/reader-pool-placement.md) design |
| 4 | Support | About 32,200: support 21,191 and graphics and viewport 10,982, plus their data tables and startup | As in [planning](planning.md#goal-and-phases) | #3; planned when it becomes the next phase ([ADR 0005](adr/0005-plan-only-the-next-phase.md)) |
| 5 | GUI, drawing and platform | About 64,200: GUI 52,227 and platform 11,943, plus simulation and data-table drawing | As in [planning](planning.md#goal-and-phases) | As phase 4 |
| 6 | Vendored and final retirement | Vendored 52,831, plus whatever boundary, test and evidence C++ remains | Every original and fork-written C++ line retired and candidate native targets removed ([goal and phases](planning.md#goal-and-phases)) | As phase 4 |

Root's first actions on resume:
1. Follow the [latest steering actions](#latest-steering-actions) and the [resume checkpoint](#resume-checkpoint).
2. Drive each cleanup stream to its end state above.
3. Before the cleanup stage's last items are dispatched, plan phase 3 ([start-development](skills/start-development/SKILL.md) step 3).

## Cleanup work order

Each stream issue holds its batches, rules, proof and inventory. [Selection](planning.md#selection) ranks cleanup work by stream inventory. File ownership and file-set checks follow [team](team.md#ready-queue-and-pipelining); what a cleanup PR needs to merge is in [integration](root-workflow.md#integration).

Slots: both [implementer slots](team.md#slots) run #734/#735 file passes, on worksets whose file sets do not overlap. #946, #950, #952 and the interval's tools batch each take one slot while they run. The stage exits when the file passes do, so they come before anything that can wait. #738 is closed at its end state.

1. **#734/#735 file passes.** One PR per workset in the #734 manifest finishes every file it touches, applying the [code standards](porting.md#code-standards) to every line. Sizing, splits and single-file passes follow [sizing](planning.md#sizing); proof follows [cleanup proof](porting.md#cleanup-proof). Each pass reports its files' residue from `tools/code-standards-inventory.py`.

   Order, by fan-in (measured at `5af17025a3`). Delivered: the 03 remainder and its residue pass, and 23's remainder (#884).
   - (b) 13 Map/terrain (fan-in 90) with 23's type-bearing files. `map_func.rs`, `tile_map.rs`, the 13 leaves (#878) and 23's type-bearing files are finished; `tile_loop.rs`, `tilearea.rs`, `tree_cmd.rs` and `trees.rs` of workset 13, and 23's `native_test_support.rs` and `native_test_support/movement.rs`, are not yet in `migration/finished-files.txt`.
   - (c) 09 (59), 08 (58), 07 (52), 06 (45), 04 (38), 05 (27). The calls between 04 and 06 run both ways (06 to 04 in 2 files, 04 to 06 in 1), so strict fan-in governs.
   - (d) The rest by fan-in: 16 (67), 21 (62), 15 (42), 18 (37), 10 (33), 11 (28), 14 (27), 17 (26), 12 (25). The leaf worksets 22 (17), 20 (6) and 19 (1) go last. F's closure stays last.

   Twenty-four workset files exceed the single-file pass size; the largest are `station_cmd.rs` (4,868), `orders.rs` (4,841), `town.rs` (3,373), `train.rs` (3,121), `industry.rs` (2,874) and `company.rs` (2,698). #734 reports progress in lines finished as well as files.
2. **Tools batch.** One per stocktake interval ([selection](planning.md#selection)). This interval's is #945, dispatched after #946. New inventory visibility is not C++ deletion.
3. **#736 test evidence.** Three transfers remain: the authentication probe (#936, in the merge queue), the train edge probe (#946) and the aircraft range probe (#950), each oracle with an `oracle-capture.py` recipe. #950 projects to 10 to 13 changed lines for each net C++ line, above every delivered transfer, so root takes its cost decision at its first checkpoint: it continues, or the probe stays native under a named owner. Each follows the [movable rule](porting.md#tests-in-rust-and-c) and the oracle rule on #736, and its record claims only what its tests observe and names the layer each test observes. After #950 merges or is left native, the routine agent refreshes the classification table in #736's body, where each remaining file names its owner or boundary; that closes the stream's inventory without a PR ([ruling](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6834080910)).

   What stays native: the Catch2 files that delete whole (#931 closed, since its recipes outweighed the C++ it deleted), `src/tests/train.cpp` (its PBS and reservation cases retire with #725, its speed case with #656), and the ScriptList sections of `tools/migration/script-list-comparison.cpp`, which retire with that file when the script API reader takes the `ScriptList` adapter classes. The console fixtures and Catch2 files that reach live pools (65 files, 14,273 counted lines) retire inside the ports that retire their pools or readers. The 33 files and 3,909 lines of retained boundary evidence leave with their boundaries.
4. **#734 closure**, after the last workset pass: private wrapper fields, generator support, and removal of the raw adapters and aliases. Seventeen Rust files (2,944 lines; `test_tables.rs` is 1,735 of them) are in no workset. Each joins one before closure, the type files with F ([#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6767381933)). The end state and later stocktakes and steering measure with `tools/code-standards-inventory.py`. The closure residue sweep takes the recorded residue lists ([steering 34's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6789587702) and [steering 35's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6834079997)) unless a later pass touches the file; its macro sites are #892's. #892 rides remaining file passes, then that sweep and closure; no unrecorded production assertion macro site remains at closure, measured as #892 specifies.

   #734 also owns the `unwrap`/`expect` and release-overflow fidelity gaps recorded in #156: remaining file passes handle their selected files, and closure handles recorded residue in already-finished files. Compare each with the original's preconditions and observable failure behaviour; an occurrence alone is not a defect. This ownership does not expand #892's macro scope or add a census, lint rule or gate.
5. **#952 speed maintenance**, after #951 merges: restore the inlining of `movement.rs`'s track-status path, measured on Opus and Grok.
6. **#893 cleanup-exit evidence**, after closure and before the first phase-3 reader dispatch: run the coverage measurement, seeded-fault pilot and all-member deep check. Pause implementers for the two host measurements. Read the report and handle findings under [deep checks](deep-checks.md#policy) before dispatch. Phase-3 planning follows its normal trigger and does not wait for these results.

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
- #500 closes with #734's closure. #168 and #199 are closed: #565, #717 and #734 own what remained.
- #693 keeps the cancelled reconnect failure and two completed original/original runs that did not reproduce it; the cause is unresolved and its two-run authorization is exhausted. Diagnosis returns with #730's network owner after cleanup, or sooner if hangs exceed one rerun in ten Full requests at a stocktake (provisional).
- #503's coverage continuation runs last.
- #939 lists 35 rules that a script, hook or lint could hold instead of a doc. A candidate joins a [tools batch](planning.md#selection) when its rule was broken in the window just ended, with the count.
- #940 holds the docs cleanup's follow-ups. Root's rulings are recorded there: a worktree is protected only by its whole folder name, and a deep run ends with its session. Its small fixes ride a tools batch where they share its files.
- #860 lists the development-process changes the user approved: the merge queue ([ADR 0019](adr/0019-merge-queue.md)), the per-merge builds ([ADR 0018](adr/0018-builds-before-every-merge.md)), CI scenario comparison ([ADR 0020](adr/0020-scenario-comparisons-on-ci.md)), direct reviewer and implementer messages, and the wind-down-only checkpoint ([ADR 0026](adr/0026-roadmap-commit-timing.md)).
- #896 (paid CI minutes) stays with its author for the one-day weighted-minutes and network-failure follow-up. The simulation shards are back on Blacksmith; which jobs run on paid runners is the user's decision ([ADR 0009](adr/0009-paid-runner-assignment.md)).

Excluded, with issue placement and return conditions rechecked by steering 35 at merge 47; near-full reproduction and fork release-source publication triggers are not met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Latest steering actions

Steering 35 is the latest; its findings are in the message of the steering commit that follows `5ca69b39f4`. Its standing changes are rules now: a re-check's message and scope, and where a reviewer reads the brief ([review brief](review-brief.md#a-re-check)); one tools batch per stocktake interval ([selection](planning.md#selection)); and the workflow review's tool-call check ([CI administration](root-workflow.md#ci-administration)).

Root did all five on resuming (2026-11-12); the [resume checkpoint](#resume-checkpoint) has the state they left.

For root, on resume:
1. Resume #936 and #937 with a fresh implementer each, from the checkpoint rows below and steering 35's audit comments ([#936](https://github.com/fetch-laboratories/openttd-rust/pull/936#issuecomment-6834100123), [#937](https://github.com/fetch-laboratories/openttd-rust/pull/937#issuecomment-6834099575)). #936's two record findings block its merge; nothing blocks #937.
2. Start a planner for each [ready-queue](#ready-queue) entry now; all three wait on nothing.
3. Finish what the merge-44 stocktake still owes: the platform cadence, the instruction profile of the merged tip, `python3 tools/retrospective.py stocktake`, and #892's macro counts. "Where the fork stands" already has merge 47's census and inventories.
4. Bring the bodies of #734 and #736 up to date: 116 files finished, and #736's delivered batches with [steering 35's ruling](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6834080910).
5. Record the idle time of the #923 compile step, for the last item below.

Provisional, for steering 36 to judge:
- **Plans in issues.** How many implementers re-derived their plan's file set or size, and how many plans were wrong.
- **The tools batch.** Tools PRs merged in the interval, and the failed runs a batch would have prevented or caused.
- **Re-check scope.** Review stages per PR, and any behaviour defect that only a whole-PR re-check would have caught.
- **The residue gate.** It fired once in 11 PRs, falsely, so no false-failure threshold can be set yet.
- **Host load.** Under the waiting-slot rule three or more implementers worked at once for 226 of 376 minutes on 2026-11-11. Compare the compile step's loaded and idle times before proposing a change to [ADR 0010](adr/0010-two-implementers.md), which is the user's. Measured on resuming: the #923 compile step took 127 seconds (24 of 24 compiled) in #936's checkout with no build running and three planners starting, against 163 to 289 seconds loaded.

## Ready queue

Undispatched work orders ([team](team.md#ready-queue-and-pipelining)). Each has its own issue, whose body holds its plan once a planner has written it. Merges 48 to 51 were #951, #937, #956 and #936. The open component PRs are #955 (queued again after one removal), #959, #960, #961 and #962, with #958 dispatched and its PR to come: six, the [branch limit](root-workflow.md#integration).

1. #965: 9yr, the pass for `road_yapf.rs`, split from `ship_yapf.rs` at the file boundary. Waits on nothing.
2. #963: 8g, the file pass for `ground.rs` and `ground_vehicle.rs`, workset 08's shared callees. Planned; waits on #960 and #959, which rewrite the calls it replaces.
3. #964: 8r, the single-file pass for `road.rs`. Planned; waits on #963 and #965, its callees.

**Later.** #945, the interval's tools batch of ten items, waits on #955, which is #946's PR. In workset 09, the pass for `ship_yapf.rs` waits on #962, and the single-file pass for `rail_yapf.rs` on #958's PR ([order](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6811109267)). In workset 08, after #963 and #964: the single-file passes for `train.rs` (after #958 and #955, which edit it), `aircraft.rs` and `train_reservation.rs`, and one pass for `effect.rs`, `vehicle_effect.rs`, `ship_control.rs` and `disaster.rs`, which its planner splits if it must. A second slot with no entry ready takes the next workset in the [work order](#cleanup-work-order)'s fan-in order whose files no open pass finishes. Workset 23's `native_test_support.rs` and `native_test_support/movement.rs` get their own entry after #959 merges, since #959 edits the first. #950, the aircraft range probe, waits on #955 and follows #945.

Full phase-3 planning remains untriggered: 185 of #734's 319 files, in 19 worksets, are in no entry.

## Resume checkpoint

Root resumed on 2026-11-12 from the lenient wind-down at `93eedd81b2` (merge 44) and steering 35, and finished the merge-47 stocktake. This section is the state at that stocktake's commit; GitHub is right where it is newer.

Agent: /root | Model: claude-fable-5-2 | Reasoning effort: high

| Open PR | Head at the stocktake | State | Next action |
| --- | --- | --- | --- |
| #936, #736 authentication probe | `2073973e98` | In the merge queue. Steering 35's two record findings are fixed; `native` and Quick passed and the re-check accepted. Checkout `t3-bc6721f0-278565bae4d9b675`. | Its implementer reports the queue's outcome. |
| #937, 8a vehicle definition file passes | `dca497f982` | Review group 2 accepted. Group 1's four findings are fixed and await their re-check, which must also accept growth records for a net 4 callback slots. `native` passed on the earlier head `d05b0e9785` only. Checkout `t3-c294893f-86eae1faa0285870`. | `native` at the head, the group 1 re-check, then the queue. |
| #951, 9a pathfinding callees (#947) | `4f096a3307` | Ready for review: preflight passed, `native` requested, both review groups running. Checkout `t3-fb682001-d1c148065c5c6970`. | Fixture runs, review findings, then the queue. |

Implementation runs on `claude-opus-5-5`, medium, planning on `claude-fable-5-2`, medium, routine work on `claude-sonnet-5-5`, medium, and review on `gpt-6.1-sol`, high; at most two implementers work at once and six component branches stay unmerged.

Verified CI state at the wind-down, unchanged since: `CI_ON_DEMAND`, `CI_MERGE_QUEUE` and `MIGRATION_CCACHE` are `true`; `CI_LINUX_SIMULATION` is `blacksmith-4vcpu-ubuntu-2404`; `CI_LINUX_COMPARISONS`, `CI_LINUX_PLATFORM` and `CI_MACOS_HEAVY` stay unset. The hooks of #927 are installed in the shared clone (`python3 tools/git_hooks.py install`).

| Other work | State |
| --- | --- |
| Ready queue | #948 and #946 are planned and wait for a slot. #949 and #952 wait on #951 and have no plan yet. |
| Tools batch | #945 is this interval's batch, undispatched; it follows #946. |
| Platform cadence | Passed at merge 47 (run 43319577655). Next after merge 52. |
| Stocktake and instruction profile | Done at merge 47. Next after merge 59. Notes kept for steering 36: a reviewer started in a shared checkout left the main checkout on a detached head, and reviewers now start in their own worktree ([review](workflow.md#review)); a planner split two of three work orders at a file boundary (#947 into #949, #946 into #950), each needing a new issue from root. |
| Phase-3 planning | Not triggered. |

Flakes and limits carried forward:

- #693: this session saw three network-timing failures with no code cause, each cleared by one unchanged retry: a `network-protocol-loopback` hang on #916, the `network-protocol` hang that removed #932 (run 43147704245) and the reference-side `content-protocol` miss that removed #935 (run 43150720812). Earlier windows recorded 2 such failures in 42 simulation-tier runs, then none among 63 native and queue requests at the merge-20 stocktake; the windows are not combined into a rate. No source or oracle change and no new local network diagnosis is authorized by this checkpoint. Keep `.local/retained-network-693` and the linked artifacts; no candidate or base regression is inferred from a timeout.
- Metric scope completeness stays with #736's end-state inventory. No later phase is selected. #898 is a partial evidence transfer; native ownership, loader and error observations, pool and ABI identity and retained fixture consumers remain as its accepted report states.

Retained state for housekeeping:

- `python3 tools/housekeeping.py --apply` ran on resuming and removed 8.1 GB in 28 entries, none of them named below. The SSD has 59 GB free.
- A name below protects a worktree only when it is the whole folder name ([housekeeping](root-workflow.md#housekeeping)). The `t3-` names of eight hex digits in this list are whole folder names from an earlier host version.
- Keep each open PR's one checkout, named in the table above.
- Keep the locked profile checkouts `t3-b49bc889-dbbee229158ddc99` (merge 47) and `t3-656b5a85-5eedda484b0f05a1` (merge 32, until #952 merges). Preserve all current speed, deep and evidence anchors; the speed anchors live in the main checkout's `.local/speed-pass`.
- Six older profile checkouts were released on 2026-11-12 (#924); this file no longer names them, so the next housekeeping run removes them.
- The list below is carried forward unchanged as preservation guidance from earlier checkpoints; it does not assert that every old checkout still exists. `/steer` may reconcile stale references, preserving uncommitted or unpushed work and evidence.

- Keep the planning checkout `t3-20fec9ce` with its private plan, and origin `preserve/housekeeping-foundation-6ab9c20e5d`. Preserve every checkout with uncommitted or unpushed work.
- Keep `.local/worktrees/root-profile-125`, `.local/worktrees/root-profile-119` and `.local/worktrees/root-profile-113` with accepted reports, raw Callgrind and frozen runtimes. Keep `t3-a490295e` (profile 131, clean `305ac20c28`) with its HDD `.local` outputs. Profile 119 measures clean `4d3a1d1d41`, not integration 120.
- Keep main `.local/integration-records/`, `.local/speed-pass/` anchors, frozen runtimes and the retained simulation evidence.
- Keep `t3-f4cf7a0b` (one unique unpushed commit, `125ea2440a`). Leave the staged 11-08 trees `t3-03c1e2da`, `t3-1ddaadfd`, `t3-5575ebe4`, `t3-83082b90`, `t3-c68d914a` and `t3-e0361d8c` untouched.
- Leave dirty or staged trees untouched: `t3-3b491ddb` (five files), locked #565 WIP `t3-431c84b4`, locked native evidence `t3-d78d5f8b`, `t3-32cf4edc` (`tools/simulation/roads.py` staged) and `t3-e130d5a3` (`migration/saves/README.md` staged).
- Keep protected `.local/worktrees/root-docs-72` and locked `.local/worktrees/profile-after*`. Do not push stale root-docs guidance over current main. Keep the pinned original unchanged.
