# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place ([records](workflow.md#records)); `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands

Measured at merge 59 (`f24c05dbe9`); the other column is merge 47 (`6d8a24a401`), the stocktake before it. A routine agent took the census, the inventories and the instruction profile. Root's half of this stocktake is still owed ([resume checkpoint](#resume-checkpoint)). The next stocktake follows merge 71 and the next platform cadence merge 62.

- Fork C++ fell by 787 lines: 376 compiled, 361 under `tools/` and 50 generated. The cleanup stage retires almost no original C++, so that share moved only from 21.94% to 21.96%. Rust grew by 6,172 lines and tooling fell by 444.
- #734 has 133 of 319 files finished, holding 92,475 lines. That is 48.5% of the 190,751-line denominator; the workset's files have grown to 198,098 lines, of which it is 46.7%. Seven more finished files, 612 lines, are in no workset.
- The boundary shrank: 1,501 exports and 3,178 callback slots, 5 and 27 fewer.
- #735 is at 118 functions over seven parameters; 69 are FFI exports, which keep positional lists while their C++ callers remain.
- #736 delivered the authentication and train edge probes. The aircraft range probe stays native under #656, and #950 is closed ([reason](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6860491029)), so the stream has no transfer left.
- The instruction counts that rose 3.4% and 3.8% by merge 47 are back at their merge-32 level on Opus and Grok ([speed](#speed)). Generation's count has risen 0.97% since merge 32, and its projection is 0.039x below its band.
- Every file pass so far raised news and script events through one C++ leaf per message, a design fault no pass could fix alone. #974 builds the shared services and converts the 29 leaves in finished files. About 58 error, console, window and sound leaves in finished files have the same shape; #979 owns them.
- Steering 36's audits compared the bodies of seven passes merged in this interval, and of draft #976, with the original and found no behaviour difference on reachable input. The departures from the code standards they found are [residue on #734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6847188216).
- Of the 1,782 minutes the interval's 12 PRs spent between dispatch and merge, implementation took 31% and CI, base updates and the merge queue 56%. Three defects the undispatched tools batch holds caused about 300 of those minutes and 9 failed runs ([#945](https://github.com/fetch-laboratories/openttd-rust/issues/945#issuecomment-6847190436)).
- The platform cadence passed at merges 52 and 57 (runs 43350842579 and 43362926059).
- No rollback has happened since merge 32, and no confirmed base regression is open. Full phase-3 planning is not triggered, and near-full reproduction is not met.

Reproduce the census with `python3 tools/port-metrics.py --whole-fork` (metric v2).

| Measured quantity | Merge 47 | Merge 59 |
| --- | ---: | ---: |
| Original C++ remaining | 221,955 | 221,908 |
| Fork source glue, including generated records | 32,915 | 32,586 |
| Compiled source and generated records C++ | 254,870 | 254,494 |
| Tracked tools C++ evidence | 14,855 | 14,494 |
| ABI-generated non-record C++ evidence | 5,353 | 5,303 |
| Measured fork C++ series | 275,078 | 274,291 |
| Original C++ retired, of 284,355 | 62,400 (21.94%) | 62,447 (21.96%) |
| Simulation C++ retired, of 61,148 | 41,233 (67.43%) | 41,279 (67.51%) |
| Historical regex exports / callback slots | 1,506 / 3,205 | 1,501 / 3,178 |
| Shared-accessor / rebound-callee / orphan-export baseline rows | 47 / 244 / 27 | 47 / 244 / 26 |
| Exports held only by uncalled members (baseline only shrinks) | 22 | 20 |
| Embedded C++ outside the census: oracle recipes | 280 | 302 |
| Rust / tooling code lines | 184,124 / 73,977 | 190,296 / 73,533 |

| Source area / phase relationship (merge 59) | Original | Retired | Remaining | Fork glue |
| --- | ---: | ---: | ---: | ---: |
| Simulation logic (phase 1) | 47,424 | 38,037 | 9,387 | 5,766 |
| Simulation storage (phase 2) | 13,724 | 3,242 | 10,482 | 1,384 |
| NewGRF, save/load, script API and network (phase 3) | 51,794 | 13,075 | 38,719 | 3,350 |
| Support, graphics and viewport (phase 4) | 32,602 | 980 | 31,622 | 548 |
| GUI and platform (phase 5) | 64,120 | 415 | 63,705 | 465 |
| Vendored (phase 6) | 55,148 | 2,317 | 52,831 | 0 |
| Data tables | 17,295 | 4,367 | 12,928 | 14 |
| Upstream tests and fork test glue | 2,248 | 14 | 2,234 | 4,770 |
| Boundary layer | 0 | 0 | 0 | 16,289 |

Coverage remains bounded by the existing saves, NewGRFs, scenarios and retained native fixtures; entry witnesses are not branch coverage. New gaps are on #156. #503 deep checks remain user-requested.

### Cleanup stream inventories

The comment, integer, wrapper, constant and literal rows are analyzer estimates from `tools/code-standards-inventory.py`. Their "in finished files" parts rise as files finish and are mostly rows a reviewer would dismiss, such as export parameters and raw adapters. Root adds the next column at its stocktake ([selection](planning.md#selection)).

| Stream | Inventory | Merge 47 | Merge 59 |
| --- | --- | ---: | ---: |
| #734 | Workset files with a finished pass, of 319 | 116 | 133 |
| #734 | Workset lines in finished files, of 190,751 | 76,906 (40.3%) | 92,475 (48.5%) |
| #734 | Explanatory comment lines missing inside matched ported bodies | 2,208; 205 in finished files | 2,057; 217 |
| #734 | Bare-integer parameters and fields whose concept has a named type | 1,640; 193 in finished files | 1,525; 298 |
| #734 | Transparent wrapper types / named constants | 318 / 3,703; 64 / 847 in finished files | 338 / 3,831; 84 / 1,134 |
| #734 | Literal sites standing for a named value | 12; 0 in finished files | 12; 0 |
| #734 | #892's production assertion macro sites (`debug_assert*!` / `assert*!` / `unreachable!`) | 169 in 59 files (6 / 70 / 93) | 176 in 65 files (6 / 73 / 97); the two scans' exclusions differ, so this is not a like-for-like change |
| #735 | Functions over seven parameters | 120 (70 FFI, 24 helpers, 26 commands) | 118 (69, 24, 25) |
| #735 | `too_many_arguments` ratchet rows / `allow` sites | 45 / 9 | 44 / 9 |
| #736 | Fork C++ evidence under `tools/` / `src/tests`, code lines | 14,855 / 4,787 | 14,494 / 4,770 |
| #738 | Rule corpus lines | 4,051, with 49 ADR files added | 4,066 |
| None | Ruff rows | 286 | 283 |

Counting commands:

- **#734:** reconcile the issue workset manifest with merged complete-file pass PRs, then count current files with `wc -l`; a deleted finished entry remains in the denominator, and mechanical caller edits do not finish a file. Run `python3 tools/code-standards-inventory.py [--head REV]` for the tree and with the finished paths for that subset. #892's sites have no tool: the merge-59 scan covers `git ls-files 'rust/*.rs'` less test files, `cfg(test)` items, comments and literals.
- **#735:** use the tracked long-signature inventory counter, `migration/clippy-debt.json` (`too_many_arguments` keys) and `git grep 'allow(clippy::too_many_arguments' -- rust/`.
- **#736, the census baseline rows and the Clippy and Ruff rows:** from `python3 tools/port-metrics.py --whole-fork` (exports and callback slots are its "Boundary" line), `python3 tools/orphan-exports.py --report-uncalled`, `wc -l` of the three baseline files, and the debt files (keys are `sites` entries, diagnostics the sum of their list lengths). Expanded exports: `growth_inventory` in `tools/port-metrics.py`.
- **#738:** `cat AGENTS.md $(git ls-files 'docs/*.md' 'docs/**/*.md' | grep -v -E '^docs/(roadmap.md|components/)') | wc -l`.

### Speed

The [merge-59 profile](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6845964937) on clean `f24c05dbe9` is the latest. All seven canonical exit saves compared equal. The driver exits 1 on its anchor identity guard until a quiet pass re-anchors ([#275](https://github.com/fetch-laboratories/openttd-rust/issues/275)), so these projections are computed from the report's instruction counts and the retained anchors by the [formula](speed.md#instruction-projections). The profile's output is retained in locked `t3-7739ec11-3073cbaa77b28444`, the previous tip for attribution at the next profile.

| Scenario | Original Ir | Candidate Ir (merge 59) | Change since merge 47 | Change since merge 32 | Projection (merge 47) | Projection (merge 59) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| TGP | 2,265,528,237 | 2,816,449,297 | +0.85% | +0.97% | 1.176162x | 1.195938x |
| Opus | 21,773,343,583 | 27,495,561,894 | -3.29% | +0.04% | 1.345115x | 1.345115x |
| Grok | 20,969,727,163 | 26,258,846,750 | -3.61% | +0.06% | 1.352896x | 1.346652x |
| P1996 | 3,600,502,896 | 4,545,633,958 | -0.66% | +0.68% | 1.352193x | 1.352193x |
| P2000 | 6,049,922,238 | 7,472,835,968 | -0.45% | +0.51% | 1.444531x | 1.444531x |
| P2016 | 11,347,936,448 | 13,741,768,668 | -0.91% | +0.09% | 1.382593x | 1.382593x |
| Drawing (diagnostic) | 2,149,065,748 | 2,154,415,240 | +0.003% | -0.000% | 1.004635x | 1.004635x |

No count rose 3% and no projection entered its band: generation is at 1.195938x against its 1.235x band and 1.3x ceiling, and the highest play record is 1.444531x against the 1.52x band and 1.6x ceiling. No wall pass is due, and these instruction projections are not new wall-time measurements.

A projection does not move while a count is below its anchor's, so the table shows the counts' own change. Every record but generation is on that floor. Root reads the count columns at each stocktake and selects a fix for a rise of 3% or more (provisional); #952 was the first, and with #969's map reads in place it brought the road search under Opus from 13.49 to 12.55 billion instructions. Generation is the one count still climbing, 0.85% in this interval; the profile did not attribute it, since nothing crossed 3%.

## Course

- Execution order: the cleanup stage, then phase 3 readers, then phase 2 storage with each pool's save/load, then phases 4, 5 and 6 ([goal and phases](planning.md#goal-and-phases), [ADR 0003](adr/0003-work-order-cleanup-readers-storage.md)).
- Yardstick: net fork C++ removed, counting C++ under `tools/` and `src/tests` ([cost decisions](planning.md#cost-decisions), [ADR 0022](adr/0022-progress-is-net-cpp-removed.md)).
- Limits: [implementer slots](team.md#slots) and the [stocktake cadence](root-workflow.md#cadences).

## Phases

Agent: /root (Claude Code navigation) | Model: claude-fable-5-2 | Reasoning effort: 30 (host-reported number)

Phase numbers name content; the execution order sets the sequence. Phase 1 (simulation logic) exited at integration 63.

Position at merge 59 (`f24c05dbe9`), 2026-11-13. Counts are code lines from `python3 tools/port-metrics.py --whole-fork` (metric version 2): comments, blanks and preprocessor lines are excluded, and the scope is `src/` plus generated records for all platforms, not one executable. Parentheses compare the last navigation, `78553df0a1` (integration 146), recounted with the same metric; the recount matches its published figures. Areas are accounting buckets; the last column names what removes each.

| Area | Original | Retired (change) | Remaining (change) | Fork glue (change) | Compiled C++ (change) | Removed by |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Simulation logic | 47,424 | 38,037 (−292) | 9,387 (+292) | 5,766 (−692) | 15,153 (−400) | Phase 1 has exited; its residue goes with its consumers: drawing functions in phase 5, startup (`openttd.cpp`) in phase 4, facades and other bodies with their phase-2 owners |
| Simulation storage | 13,724 | 3,242 (+30) | 10,482 (−30) | 1,384 (−3) | 11,866 (−33) | Phase 2 |
| Save/load | 13,388 | 1,435 (0) | 11,953 (0) | 1,070 (0) | 13,023 (0) | Phase 3, except about 4,300 lines of pool and map chunks and their compat tables, which go with phase 2 |
| Script API | 14,908 | 2,406 (0) | 12,502 (0) | 750 (0) | 13,252 (0) | Phase 3; VM and binding shims in phase 6 |
| Network | 12,713 | 3,260 (+7) | 9,453 (−7) | 1,158 (−10) | 10,611 (−17) | Phase 3 |
| NewGRF | 10,785 | 5,974 (+1) | 4,811 (−1) | 372 (0) | 5,183 (−1) | Phase 3 |
| Support | 21,617 | 973 (0) | 20,644 (0) | 544 (−3) | 21,188 (−3) | Phase 4 |
| Graphics and viewport | 10,985 | 7 (0) | 10,978 (0) | 4 (0) | 10,982 (0) | Phase 4 |
| Data tables | 17,295 | 4,367 (0) | 12,928 (0) | 14 (0) | 12,942 (0) | Each table with its last C++ consumer, in phases 2 to 5 |
| GUI | 52,179 | 415 (0) | 51,764 (0) | 463 (0) | 52,227 (0) | Phase 5 |
| Platform | 11,941 | 0 (0) | 11,941 (0) | 2 (0) | 11,943 (0) | Phase 5 |
| Vendored | 55,148 | 2,317 (0) | 52,831 (0) | 0 (0) | 52,831 (0) | Phase 6 |
| Upstream tests and fork test glue | 2,248 | 14 (0) | 2,234 (0) | 4,770 (−509) | 7,004 (−509) | Fork tests with the pools and readers they reach ([ADR 0023](adr/0023-cpp-tests-stay-until-code-is-ported.md)). Upstream tests go with the code they test; Catch2 goes when the last C++ test leaves |
| Boundary layer | 0 | 0 (0) | 0 (0) | 16,289 (−470) | 16,289 (−470) | Each side's owner; the final exit |
| **Source total** | **284,355** | **62,447 (21.96%, −254)** | **221,908 (+254)** | **32,586 (−1,687)** | **254,494 (−1,433)** | |
| C++ under `tools/` | | | | 14,494 (−1,148) | 14,494 (−1,148) | With the pools, readers and boundaries each file reaches (#736's classification) |
| ABI-generated non-record C++ | | | | 5,303 (−172) | 5,303 (−172) | With the records and layouts it checks |
| **Fork C++ census** | | | | | **274,291 (−2,753)** | |

Inventory correction, apart from retirement: no original C++ came back. Version 2 diffs each file against the pin with git's default algorithm, whose alignment shifts when a heavily edited file changes, so it overstates retired and glue by the same amount: 591 lines at the last navigation and 176 now (#978). Under a minimal diff, retired is 62,271 now against 62,110 then, a change of +161 where version 2 shows −254. Compiled C++ and the census are the same under both; read those for distance until #978 lands.

Of the 19,797 evidence lines, 18,154 are candidate or shared, none is reference-only and 1,643 are unresolved, which the census counts as candidate. Outside the census are 836 raw lines of C++ in `COMMAND_SOURCE` (`tools/simulation/stations.py`) and 302 lines of oracle recipes, which compile only against the pinned original. The boundary has 1,501 exports (−38) and 3,178 callback slots (−154). Rust has 190,296 code lines (−1,603) and tooling 73,533 (+11,637).

Rates use commit timestamps between code endpoints, in calendar days. The days include the user's pauses, so these rates do not compare periods; the next navigation recounts them per running hour ([who decides](../AGENTS.md#who-decides)):

| Period | Days | Original retired | Per day | Fork C++ census change | Per day |
| --- | ---: | ---: | ---: | ---: | ---: |
| Fork start `6ced7c097e` to the pause before cleanup, `e800717540` | 35.05 | +62,750 | 1,790 | −4,600 | −131 |
| Cleanup to the last navigation's code, `61c6d2c598` (integration 146) | 2.21 | −49 | −22 | −2,711 | −1,229 |
| To steering 34, `3a2d52b870` | 2.11 | −303 | −143 | −1,509 | −714 |
| To steering 35, `5b0b8427b8` (merge 47) | 1.52 | +2 | 1 | −457 | −300 |
| To merge 59 | 0.40 | +47 | 117 | −787 | −1,967 |
| **Since the last navigation** | **4.04** | **−254** | **−63** | **−2,753** | **−682** |
| **Cleanup stage so far** | **6.24** | **−303** | **−49** | **−5,464** | **−876** |
| **Whole history** | **41.29** | **+62,447** | **1,512** | **−10,064** | **−244** |

What set the rate:
- The cleanup stage retires no original C++ by design ([ADR 0004](adr/0004-cleanup-before-porting.md)); the movement in that column is the diff noise of #978. Ownership ports retired 1,250 to 2,160 original lines a day before it. At that pace the 221,908 lines left are about 150 days of porting, with no measured pace yet for the GUI, platform or vendored code.
- #734 finished 92 files and 60,395 lines since the stocktake at integration 143, just before the last navigation: about 15,000 lines a day. 105,623 lines remain at the files' current sizes, about seven more days at that pace, before closure and #893.
- The census fell at half the previous navigation's pace. Between steerings 34 and 35, 8 of the 17 merges in root's session were tools or CI changes and tooling grew by 8,314 lines; steering 35 moved that work to one batch per stocktake interval, and the 12 merges after it cut the census at 1,967 lines a day.

| Order | Stage | Size now | Exit (end state) | Status |
| ---: | --- | --- | --- | --- |
| 1 | Cleanup | #734: 133 of 319 workset files and 92,475 of 190,751 lines finished; the files now hold 198,098 lines. #735: 118 long functions, 44 debt keys and 9 allows. #736: 14,494 counted tools C++ lines and 4,770 fork test lines, plus separately inventoried embedded C++. #737 and #738 are closed at their end states | #734: all 319 files finished and [closure](#cleanup-work-order) done; #735 closes with it. #736: no file the [classification](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6804817013) calls movable remains, as [steering 35 amended it](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6834080910); each remaining C++ file under `tools/` and `src/tests`, and any embedded C++, either checks the calling boundary itself or reaches a live native pool, reader or production caller, and names the owner or boundary whose retirement removes it ([evidence](porting.md#evidence)). The 65 pool and reader fixtures stay native until their owners retire them. Lint ratchets shrink per PR but set no exit, and the structural Clippy rows are outside it. Read each end state from the code and inventories, not from issue state | Current |
| 2 | Phase 3: readers (script API, NewGRF, network, content, admin, save/load outside pool chunks) | About 37,800 compiled lines: script API 13,252, network 10,611, NewGRF 5,183, and save/load 13,023 less the pool and map chunks. The [queued owners](#after-cleanup) estimate 4,600 to 7,000 net C++ removed, so about 31,000 to 33,000 lines have no owner yet | Readers run in Rust with evidence for their observable outputs. Every remaining body in these buckets has a named later owner: VM and binding shims phase 6, script and network GUI phase 5, pool chunks phase 2 | Next. Plan it in full before cleanup's last items are dispatched ([goal and phases](planning.md#goal-and-phases)), and no sooner: the user declined planning it during cleanup as far too early |
| 3 | Phase 2: storage, each pool with its save/load chunks, then the map | 11,866 compiled lines in storage, about 4,300 save/load chunk lines, the logic residue assigned to pool owners, and record field routes in the 16,289-line boundary layer | Rust owns every pool's records, slots, allocation, iteration, lifetime and save/load, then map allocation and remaining map bodies; no Rust-used field behind a native service | Queued; placement in the [reader and pool placement](design/reader-pool-placement.md) design |
| 4 | Support | About 32,200: support 21,188 and graphics and viewport 10,982, plus their data tables and startup | As in [planning](planning.md#goal-and-phases) | #3; planned when it becomes the next phase ([ADR 0005](adr/0005-plan-only-the-next-phase.md)) |
| 5 | GUI, drawing and platform | About 64,200: GUI 52,227 and platform 11,943, plus simulation and data-table drawing | As in [planning](planning.md#goal-and-phases) | As phase 4 |
| 6 | Vendored and final retirement | Vendored 52,831, plus whatever boundary, test and evidence C++ remains | Every original and fork-written C++ line retired and candidate native targets removed ([goal and phases](planning.md#goal-and-phases)) | As phase 4 |

Why the course changed at this navigation (cited at `80f50ba64c`):
1. `/steer` step 2 said "progress counts original C++ retired, not glue removed" (`skills/steer/SKILL.md:38`), against the yardstick of [ADR 0022](adr/0022-progress-is-net-cpp-removed.md). It now reads the fork C++ census.
2. The retired count moved by hundreds of lines with diff alignment and showed retirement running backwards (#978).

Root's first actions on resume:
1. Follow the [latest steering actions](#latest-steering-actions) and the [resume checkpoint](#resume-checkpoint).
2. Add #978 to the tools batch (#945).
3. Drive each cleanup stream to its end state above.
4. Before the cleanup stage's last items are dispatched, plan phase 3 ([start-development](skills/start-development/SKILL.md) step 3).

## Cleanup work order

Each stream issue holds its batches, rules, proof and inventory. [Selection](planning.md#selection) ranks cleanup work by stream inventory. File ownership and file-set checks follow [team](team.md#ready-queue-and-pipelining); what a cleanup PR needs to merge is in [integration](root-workflow.md#integration).

Slots: the [implementer slots](team.md#slots) run #734/#735 file passes, on worksets whose file sets do not overlap. #945, #974 and #979 each take one slot while they run. The stage exits when the file passes do, so they come before anything that can wait; #945 and the shared services do not wait, because every pass pays for their absence. #738 is closed at its end state.

1. **Tools batch.** One per stocktake interval, dispatched within it and ranked by its items' measured cost ([selection](planning.md#selection)). This interval's is #945, which goes first because three of its items cost about 300 PR-minutes and 9 failed runs in the last interval; with it go #978 (the census's diff algorithm) and the two defects [steering 36 measured](https://github.com/fetch-laboratories/openttd-rust/issues/945#issuecomment-6847190436). New inventory visibility is not C++ deletion.
2. **Shared services for one-call-site leaves.** #974 (news and script events, planned), then #979 (error, console, window and sound leaves), which waits on #974 for its pattern and on #945 for its list. Until each merges, a pass keeps the leaves it would replace and names the work order ([design faults](porting.md#design-faults)).
3. **#734/#735 file passes.** One PR per workset in the #734 manifest finishes every file it touches, applying the [code standards](porting.md#code-standards) to every line. Sizing, splits and single-file passes follow [sizing](planning.md#sizing); proof follows [cleanup proof](porting.md#cleanup-proof). Each pass reports its files' residue from `tools/code-standards-inventory.py`.

   Order, by fan-in (measured at `5af17025a3`). Delivered: the 03 remainder and its residue pass, 23's remainder (#884), and 13 Map/terrain with 23's type-bearing files, except 23's `native_test_support.rs` and `native_test_support/movement.rs` (#973).
   - (c) 09 (59), 08 (58), 07 (52), 06 (45), 04 (38), 05 (27). Of 09, `rail_yapf.rs` (#976) and `ship_yapf.rs` (#977) are open; 08 is in the [ready queue](#ready-queue). The calls between 04 and 06 run both ways (06 to 04 in 2 files, 04 to 06 in 1), so strict fan-in governs.
   - (d) The rest by fan-in: 16 (67), 21 (62), 15 (42), 18 (37), 10 (33), 11 (28), 14 (27), 17 (26), 12 (25). The leaf worksets 22 (17), 20 (6) and 19 (1) go last. F's closure stays last.

   Twenty-four workset files exceed the single-file pass size; the largest are `station_cmd.rs` (4,868), `orders.rs` (4,841), `town.rs` (3,373), `train.rs` (3,121), `industry.rs` (2,874) and `company.rs` (2,698). #734 reports progress in lines finished as well as files.
4. **#736 test evidence.** No transfer remains. The aircraft range probe stays native under #656: #950 projected 10 to 13 changed lines for each net C++ line, above every delivered transfer, and the user approved closing it ([reason](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6860491029)). At resume the routine agent refreshes the classification table in #736's body, where each remaining file names its owner or boundary; that closes the stream's inventory without a PR ([ruling](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6834080910)).

   What stays native: `tools/migration/aircraft-range.cpp` with `finite_range_probe` (#656 retires it), the Catch2 files that delete whole (#931 closed, since its recipes outweighed the C++ it deleted), `src/tests/train.cpp` (its PBS and reservation cases retire with #725, its speed case with #656), and the ScriptList sections of `tools/migration/script-list-comparison.cpp`, which retire with that file when the script API reader takes the `ScriptList` adapter classes. The console fixtures and Catch2 files that reach live pools (65 files, 14,273 counted lines) retire inside the ports that retire their pools or readers. The 33 files and 3,909 lines of retained boundary evidence leave with their boundaries.
5. **#734 closure**, after the last workset pass: private wrapper fields, generator support, and removal of the raw adapters and aliases. Seventeen Rust files (2,944 lines; `test_tables.rs` is 1,735 of them) are in no workset. Each joins one before closure, the type files with F ([#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6767381933)). The end state and later stocktakes and steering measure with `tools/code-standards-inventory.py`. The closure residue sweep takes the recorded residue lists ([steering 34's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6789587702), [steering 35's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6834079997) and [steering 36's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6847188216)) unless a later pass touches the file; its macro sites are #892's, and the three YAPF search-loop copies are #971's and stay. #892 rides remaining file passes, then that sweep and closure; no unrecorded production assertion macro site remains at closure, measured as #892 specifies.

   #734 also owns the `unwrap`/`expect` and release-overflow fidelity gaps recorded in #156: remaining file passes handle their selected files, and closure handles recorded residue in already-finished files. Compare each with the original's preconditions and observable failure behaviour; an occurrence alone is not a defect. This ownership does not expand #892's macro scope or add a census, lint rule or gate.
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

Then storage, each pool with its save/load chunks, redone from current base with each closed draft's branch as reference (the branches are kept on origin, and each PR records its head and state): #656/#598/#599 vehicle (#714), #634 station/road-stop/cargo (#700), #638/#498 small pools (#723), #635 town/industry/subsidy (#704), #637/#497 remaining engine pools, #717 attachment, #725 YAPF with #971's shared search loop ([moved out of cleanup](https://github.com/fetch-laboratories/openttd-rust/issues/971#issuecomment-6860497422)), #906 orders, #907 companies and groups, then the map (#565's remainder). #716 tracks the all-pools closure. Each pool brief follows the [reader and pool placement](design/reader-pool-placement.md) design.

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

Steering 36 is the latest; its findings are in the message of the steering commit that follows `c80ace14b8`. Its standing changes are rules now:

- The tools batch is dispatched within its interval, ranked by its items' measured cost ([selection](planning.md#selection)). The last one was not dispatched at all.
- A plan grants no exception to porting and prescribes no commit split ([planner brief](planner-brief.md#the-plan)).
- A plan is reviewed before dispatch, by a reviewer the planner starts and the PR does not reuse ([plan review](planner-brief.md#plan-review), [review brief](review-brief.md#a-plan-review)). A PR's reviewer rules on each exception the PR body records ([review brief](review-brief.md#what-to-examine-in-your-group)).
- Three porting rulings: a pass keeps a one-call-site carrier only for a shared-service work order ([design faults](porting.md#design-faults)); a map service with no Rust port stays for #565 ([map access](porting.md#map-access)); an export that serves only a kept native fixture leaves with it ([callees](porting.md#callees-and-shared-accessors)).

For root, on resume:
1. Take #976 to the queue; nothing blocks it. Its failed Quick step had a cause that is not in the PR: the head moved during the requested run. Run `python3 tools/ci.py ready 976`, request `native` at `2b2fa63782`, push nothing to the branch until the run's Quick job has finished, and queue on a pass ([audit comment](https://github.com/fetch-laboratories/openttd-rust/pull/976#issuecomment-6847188596)).
2. The five plans already written (#945, #974, #963, #973, #964) predate plan review, and the base has moved under them. Before dispatching each, start a planner for it again: it refreshes the plan and has it reviewed.
3. Finish root's half of the merge-59 stocktake ([resume checkpoint](#resume-checkpoint)). In it, add #978 and items 12 and 13 of [steering 36's comment](https://github.com/fetch-laboratories/openttd-rust/issues/945#issuecomment-6847190436) to #945, and have its planner say whether the batch still fits one PR.
4. Slots are strict now ([slots](team.md#slots)): the implementer that takes #976 through CI holds one until it merges. Dispatch #945 to the other, before any new file pass, then #974. File passes take the slots after them: #963, then a fresh implementer for #977 from [its notes](https://github.com/fetch-laboratories/openttd-rust/pull/977#issuecomment-6847189014).
5. Start a planner for #979 once #974 has merged and #945's one-call-site listing exists.
6. Bring #734's body to 133 files and link the three residue lists.

Provisional, for steering 37 to judge:
- **Plan review.** Of the 22 departures the audits found in merges 48 to 59 that no reviewer raised, about 10 were plan decisions, 5 were exceptions the implementer recorded in the PR body and 7 were in neither. The PR reviewers had the rules, the code and the original's lines on screen; they checked behaviour and took the plan as settled. Replayed on this interval's eight plans, a plan reviewer raised 37 findings in about four minutes a plan and caught 8 of the 13 known faulty decisions and facts, among them the false "no in-place Rust form exists" of #958 and the leaf that cost #972 a rework round; all 8 further findings checked against the code were true. It caught none of the 8 known defects that only a build shows (commit splits, size, file sets). Measure, per plan: findings, how many the planner accepted, and audit departures that trace to a reviewed plan.
- **Recorded exceptions.** Count the exceptions reviewers ruled on and the findings that came of them; the rule is untested.
- **The tools batch.** Failed runs by cause, and the share of PR-minutes outside implementation, after #945 merges; and whether each later batch was dispatched in its interval.
- **Plans.** Plans cost about 9% of implementer minutes and their file sets were accurate (10 of 12 PRs had at most 4% unplanned lines), so plans in issues stand. Count the plan decisions an audit or reviewer finds contrary to porting (six in this window) and compare estimates with actual changed lines (median 1.54 times).
- **Re-check scope** stands: no re-check raised a behaviour defect outside its fixing commits, and the four sessions beyond two stages were forced by the exact-head acceptance, item 13 on #945.
- **Root's docs pushes.** Ten of 22 landed on a queued PR and rebuilt 13 merge-group runs before `b4e7ca3d73`, none after. Root made 13 ready-queue commits in 8.3 hours; count both again.
- **Implementer slots.** The user made the strict limit of two standing ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)): a waiting implementer keeps its slot. Under the earlier rule 3.0 implementers were building or editing on average and the compile step ran a median 234 seconds against 136 to 140 idle. Measure changed lines merged per hour (3,396 before the limit, 3,660 in the 2.4 hours after it), the compile step's time and how long a slot sits idle behind CI, so that the user can judge the limit on more than one session.

## Ready queue

Undispatched work orders ([team](team.md#ready-queue-and-pipelining)). Each has its own issue, whose body holds its plan once a planner has written it. The open component PRs are two drafts, each resumed by a fresh implementer from its checkout and status comment: #976 (for #968, `rail_yapf.rs`, checkout `t3-9332b642-2e3cbe422377f934`, reviews accepted, CI left) and #977 (for #970, `ship_yapf.rs`, checkout `t3-0442faa5-4501bd863733960a`, with its two measurement worktrees `speed-970-base` and `speed-970-fresh`). Root's status comment on #3 holds the queue's state between commits to this section.

1. #945: the interval's tools batch. Planned; root adds three items first ([steering actions](#latest-steering-actions)). Waits on nothing, and goes before any new file pass.
2. #974: shared typed news and script-event services in place of one C++ leaf per message, with the 29 leaves in finished files converted. Planned; waits on nothing.
3. #963: 8g, the file pass for `ground.rs` and `ground_vehicle.rs`, workset 08's shared callees. Planned; waits on nothing.

**Later.** #973, 23n, the pass for `native_test_support.rs` and `native_test_support/movement.rs`, is planned and waits on #963, so that it removes C++ and adds none. #964, 8r, the single-file pass for `road.rs`, is planned and waits on #963 and #974. #975 deletes the C++ free functions the extended orphan check finds uncalled, about 35 by the planner's scan, and waits on #945. #979 replaces the other one-call-site leaves in finished files and waits on #974 and #945. In workset 08, after #963 and #964: the single-file passes for `train.rs` and `train_reservation.rs`, and one pass for `effect.rs`, `vehicle_effect.rs`, `ship_control.rs` and `disaster.rs`, which its planner splits if it must. A second slot with no entry ready takes the next workset in the [work order](#cleanup-work-order)'s fan-in order whose files no open pass finishes.

Full phase-3 planning remains untriggered: 180 of #734's 319 files, in 19 worksets, are in no entry.

## Resume checkpoint

**Paused at the user's request (lenient wind-down, 2026-11-13).**

Root resumed at merge 47 on 2026-11-12 and stopped at merge 59 (`f24c05dbe9`), with no rollback and no confirmed base regression. The merge queue is empty. No implementation, repair, review, build, simulation, CI request or retry is authorized until resume. Where GitHub is newer than this section, GitHub is right. Steering 36 rewrote the [ready queue](#ready-queue) and #976's row below at merge 59.

Agent: /root | Model: claude-fable-5-2 | Reasoning effort: high

| Remaining PR | Pushed head | State when root stopped | Next action |
| --- | --- | --- | --- |
| #976, 9c `rail_yapf.rs` pass (draft) | `2b2fa63782` | All three review groups accepted, and a re-check accepted `2b2fa63782`. Preflight passed 16 of 16 at this head. `native` has not run at this head. Checkout `t3-9332b642-2e3cbe422377f934`, clean and pushed. | [Steering action 1](#latest-steering-actions): mark it ready, request `native` at `2b2fa63782`, push nothing while its Quick job runs, then queue. |
| #977, 9ys `ship_yapf.rs` pass (draft) | `96d942d6de` | Code written; nothing built or run natively, no CI, no review. It conflicts with base and carries copies of the shared ports #969 merged. Checkout `t3-0442faa5-4501bd863733960a`; measurement worktrees `speed-970-base` and `speed-970-fresh`, the second with one uncommitted file. | A fresh implementer starts from the PR's status comment: base update, native build and tests, the speed runs that also decide the retained scratch, preflight, review. |

Implementation runs on `claude-opus-5-5`, medium, planning on `claude-fable-5-2`, medium, routine work on `claude-sonnet-5-5`, medium, and review on `gpt-6.1-sol`, high.

CI variables were not rechecked this session, and no workflow change or bootstrap Full ran. The hooks of #927 are installed in the shared clone (`python3 tools/git_hooks.py install`).

| Other work | State |
| --- | --- |
| Ready queue | As the [ready queue](#ready-queue) above has it. Root's status comment on #3 has the session's notes. |
| Tools batch | #945 was not dispatched in its interval. It is the batch of the interval that starts at merge 60 and goes first on its measured cost; the other candidates on #939 wait for the one after. |
| Platform cadence | Passed at merges 52 and 57. Next after merge 62. |
| Stocktake | Measured at merge 59. Root's half is owed on resume: rerun `issues.py` and recheck the exclusion conditions; bring #734's body to 133 files; and the three checks in root's notes on #3 (whether the two rule changes of 2026-11-13 were used beyond the PRs that prompted them, base updates made without a conflict since `fc5010d279`, and whether #945's added items still fail runs). |
| Phase-3 planning | Not triggered. |

Flakes and limits carried forward:

- #693: this session saw three network-timing failures with no code cause, each cleared by one unchanged retry: a `network-protocol-loopback` hang on #916, the `network-protocol` hang that removed #932 (run 43147704245) and the reference-side `content-protocol` miss that removed #935 (run 43150720812). Earlier windows recorded 2 such failures in 42 simulation-tier runs, then none among 63 native and queue requests at the merge-20 stocktake; the windows are not combined into a rate. No source or oracle change and no new local network diagnosis is authorized by this checkpoint. Keep `.local/retained-network-693` and the linked artifacts; no candidate or base regression is inferred from a timeout.
- #693, this session: one `network-protocol-loopback` hang, on #961's `native` run 43347645519, cleared by one rerun of the failed shard.
- Two background jobs were killed by a signal with no sender identified on 2026-11-13 (#724), and the merge-59 profile's first run ended on its own agent's 30-minute limit and was rerun once.
- Metric scope completeness stays with #736's end-state inventory. No later phase is selected. #898 is a partial evidence transfer; native ownership, loader and error observations, pool and ABI identity and retained fixture consumers remain as its accepted report states.

Retained state for housekeeping:

- `python3 tools/housekeeping.py --apply` last ran after merge 57, and root removed 18 review and merged checkouts by hand at this wind-down, each clean and pushed. The SSD has 49 GB free.
- A name below protects a worktree only when it is the whole folder name ([housekeeping](root-workflow.md#housekeeping)). The `t3-` names of eight hex digits in this list are whole folder names from an earlier host version.
- Keep each open PR's one checkout and #977's two measurement worktrees, named in the table above.
- Keep the locked merge-59 profile checkout `t3-7739ec11-3073cbaa77b28444`. The merge-47 and merge-32 profile checkouts are released: this file no longer names them, and root unlocks them on resume so that housekeeping removes them. Preserve all current speed, deep and evidence anchors; the speed anchors live in the main checkout's `.local/speed-pass`.
- Six older profile checkouts were released on 2026-11-12 (#924); this file no longer names them, so the next housekeeping run removes them.
- The list below is carried forward unchanged as preservation guidance from earlier checkpoints; it does not assert that every old checkout still exists. `/steer` may reconcile stale references, preserving uncommitted or unpushed work and evidence.

- Keep the planning checkout `t3-20fec9ce` with its private plan, and origin `preserve/housekeeping-foundation-6ab9c20e5d`. Preserve every checkout with uncommitted or unpushed work.
- Keep `.local/worktrees/root-profile-125`, `.local/worktrees/root-profile-119` and `.local/worktrees/root-profile-113` with accepted reports, raw Callgrind and frozen runtimes. Keep `t3-a490295e` (profile 131, clean `305ac20c28`) with its HDD `.local` outputs. Profile 119 measures clean `4d3a1d1d41`, not integration 120.
- Keep main `.local/integration-records/`, `.local/speed-pass/` anchors, frozen runtimes and the retained simulation evidence.
- Keep `t3-f4cf7a0b` (one unique unpushed commit, `125ea2440a`). Leave the staged 11-08 trees `t3-03c1e2da`, `t3-1ddaadfd`, `t3-5575ebe4`, `t3-83082b90`, `t3-c68d914a` and `t3-e0361d8c` untouched.
- Leave dirty or staged trees untouched: `t3-3b491ddb` (five files), locked #565 WIP `t3-431c84b4`, locked native evidence `t3-d78d5f8b`, `t3-32cf4edc` (`tools/simulation/roads.py` staged) and `t3-e130d5a3` (`migration/saves/README.md` staged).
- Keep protected `.local/worktrees/root-docs-72` and locked `.local/worktrees/profile-after*`. Do not push stale root-docs guidance over current main. Keep the pinned original unchanged.
