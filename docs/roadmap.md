# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place ([records](workflow.md#records)); `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands

Base is at merge 71 (`603724a3fd`), measured there on 2026-11-18 by routine agents; the other column is merge 59 (`f24c05dbe9`), the stocktake before it. The interval took 36 running hours ([who decides](../AGENTS.md#who-decides)). The next stocktake follows merge 83 and the next platform cadence merge 72.

- Fork C++ fell by 1,333 lines, 37 a running hour: 709 of source glue, 472 of original C++ retired, 135 generated and 17 under `tools/`. Rust grew by 7,004 lines and tooling by 1,145.
- **Map generation's instruction count rose 24.6%** between the two profiles, the only scenario that moved, and its projection is 1.775x against the 1.3x ceiling ([speed](#speed)). Every exit save still compares equal. It is being attributed per function to the twelve merges; its fix takes the first slot that frees once it is placed, and no wall pass runs before that.
- #734 has 141 of 319 files finished, holding 105,163 lines. That is 55.1% of the 190,751-line denominator; the workset's files have grown to 205,453 lines, of which it is 51.2%. Nine more finished files, 876 lines, are in no workset. Eight files finished in twelve merges: six of the twelve were shared services, the string IDs and tools, and the passes that did merge each carried the repair of the design faults in its files. At this interval's 352 finished lines a running hour, the 100,290 lines left are about 285 running hours of passes; steering 37 judges whether that is the course.
- The boundary shrank: 1,467 exports and 3,073 callback slots, 34 and 105 fewer.
- #735 is at 115 functions over seven parameters; 70 are FFI exports, which keep positional lists while their C++ callers remain.
- #736 has no transfer left. Fork test glue under `src/tests` grew by 83 lines, from fixture inputs that passes repaired under accepted growth records.
- The one-call-site leaves in finished files are gone for news and script events (#982), windows and sound (#1001) and errors (#1005), with console and chat in review (#1008); Rust has every `STR_*` constant from one generated table (#995), which took the seven string-ID records #982 added and thirteen more (#1006) with it.
- Reviews have run on `gpt-6.2-sol` since merge 66, at the user's word. Its re-checks raise new findings: four of the seven PRs reviewed on it made production or fixture changes after their last stage, and [review](workflow.md#review) now sends those to one scoped behaviour reviewer (provisional).
- The user raised the implementer limit to three for the session of 2026-11-18, from about 15:30Z; merges up to 67 ran under two.
- Of 39 `native` runs in the interval 5 failed and 11 were cancelled by a moved head; of 16 merge-queue runs 2 failed and 2 were cancelled. Six failures had no code cause and were network scenarios: #1013's planner traced five loopback stalls to a thread race in the original's `TCPConnecter::CheckActivity`, in a file both builds compile unchanged ([#693](https://github.com/fetch-laboratories/openttd-rust/issues/693)). One SIGSEGV in the kernel tests on CI is unexplained (#1012).
- The platform cadence passed at merges 62 and 67 (runs 43566394953 and 44309078613); hosted macOS had no runner in three runs, each finished on the paid fallback that [CI administration](root-workflow.md#ci-administration) prescribes.
- No rollback has happened since merge 32. Full phase-3 planning is not triggered, and near-full reproduction is not met.

Reproduce the census with `python3 tools/port-metrics.py --whole-fork` (metric version 3).

| Measured quantity | Merge 59 | Merge 71 |
| --- | ---: | ---: |
| Original C++ remaining | 222,084 | 221,612 |
| Fork source glue, including generated records | 32,410 | 31,701 |
| Compiled source and generated records C++ | 254,494 | 253,313 |
| Tracked tools C++ evidence | 14,494 | 14,477 |
| ABI-generated non-record C++ evidence | 5,303 | 5,168 |
| Measured fork C++ series | 274,291 | 272,958 |
| Original C++ retired, of 284,355 | 62,271 (21.90%) | 62,743 (22.07%) |
| Simulation C++ retired, of 61,148 | 41,103 (67.22%) | 41,461 (67.80%) |
| Historical regex exports / callback slots | 1,501 / 3,178 | 1,467 / 3,073 |
| Shared-accessor / rebound-callee / orphan-export baseline rows | 47 / 244 / 26 | 46 / 239 / 21 |
| Exports held only by uncalled members (baseline only shrinks) | 20 | 20 |
| Embedded C++ outside the census: oracle recipes | 302 | 302 |
| Rust / tooling code lines | 190,296 / 73,533 | 197,300 / 74,678 |

| Source area / phase relationship (merge 71) | Original | Retired | Remaining | Fork glue |
| --- | ---: | ---: | ---: | ---: |
| Simulation logic (phase 1) | 47,424 | 38,000 | 9,424 | 5,085 |
| Simulation storage (phase 2) | 13,724 | 3,461 | 10,263 | 1,379 |
| NewGRF, save/load, script API and network (phase 3) | 51,794 | 13,090 | 38,704 | 3,331 |
| Support, graphics and viewport (phase 4) | 32,602 | 1,071 | 31,531 | 541 |
| GUI and platform (phase 5) | 64,120 | 423 | 63,697 | 465 |
| Vendored (phase 6) | 55,148 | 2,317 | 52,831 | 0 |
| Data tables | 17,295 | 4,367 | 12,928 | 14 |
| Upstream tests and fork test glue | 2,248 | 14 | 2,234 | 4,853 |
| Boundary layer | 0 | 0 | 0 | 16,033 |

Coverage remains bounded by the existing saves, NewGRFs, scenarios and retained native fixtures; entry witnesses are not branch coverage. New gaps are on #156: this interval's shared services added window, sound, error, console and chat sites that no scenario observes and that are source-reviewed only. #503 deep checks remain user-requested.

### Cleanup stream inventories

The comment, integer, wrapper, constant and literal rows are analyzer estimates from `tools/code-standards-inventory.py`. Their "in finished files" parts rise as files finish and are mostly rows a reviewer would dismiss, such as export parameters and raw adapters. Root adds the next column at its stocktake ([selection](planning.md#selection)).

| Stream | Inventory | Merge 59 | Merge 71 |
| --- | --- | ---: | ---: |
| #734 | Workset files with a finished pass, of 319 | 133 | 141 |
| #734 | Workset lines in finished files, of 190,751 | 92,475 (48.5%) | 105,163 (55.1%) |
| #734 | Explanatory comment lines missing inside matched ported bodies | 2,057; 217 in finished files | 2,033; 229 |
| #734 | Bare-integer parameters and fields whose concept has a named type | 1,525; 298 in finished files | 1,442; 339 |
| #734 | Transparent wrapper types / named constants | 338 / 3,831; 84 / 1,134 in finished files | 362 / 3,937; 97 / 1,267 |
| #734 | Literal sites standing for a named value | 12; 0 in finished files | 12; 0 |
| #734 | #892's production assertion macro sites (`debug_assert*!` / `assert*!` / `unreachable!`) | 165 (6 / 73 / 86) by the merge-71 scan; the merge-59 stocktake's own scan gave 176 in 65 files | 157 in 62 files (6 / 70 / 81) |
| #735 | Functions over seven parameters | 118 (69 FFI, 24 helpers, 25 commands) | 115 (70, 20, 25) |
| #735 | `too_many_arguments` ratchet rows / `allow` sites | 44 / 9 | 42 / 9 |
| #736 | Fork C++ evidence under `tools/` / `src/tests`, code lines | 14,494 / 4,770 | 14,477 / 4,853 |
| #738 | Rule corpus lines | 4,066 | 4,146 |
| None | Ruff rows | 283 | 283 |

Counting commands:

- **#734:** reconcile the issue workset manifest with merged complete-file pass PRs, then count current files with `wc -l`; a deleted finished entry remains in the denominator, and mechanical caller edits do not finish a file. Run `python3 tools/code-standards-inventory.py [--head REV]` for the tree and with the finished paths for that subset. #892's sites have no tool: the scan covers `git ls-files 'rust/*.rs'` less test files, `cfg(test)` items, comments and literals, and each stocktake reruns it on the previous tip, since two scans' exclusions have differed.
- **#735:** use the tracked long-signature inventory counter, `migration/clippy-debt.json` (`too_many_arguments` keys) and `git grep 'allow(clippy::too_many_arguments' -- rust/`.
- **#736, the census baseline rows and the Clippy and Ruff rows:** from `python3 tools/port-metrics.py --whole-fork` (exports and callback slots are its "Boundary" line), `python3 tools/orphan-exports.py --report-uncalled`, `wc -l` of the three baseline files, and the debt files (keys are `sites` entries, diagnostics the sum of their list lengths). Expanded exports: `growth_inventory` in `tools/port-metrics.py`.
- **#738:** `cat AGENTS.md $(git ls-files 'docs/*.md' 'docs/**/*.md' | grep -v -E '^docs/(roadmap.md|components/)') | wc -l`.

### Speed

The merge-71 profile on clean `8869d55587` (docs commits over merge `603724a3fd`) is the latest. All seven canonical exit saves compared equal. The driver exits 1 on its anchor identity guard until a quiet pass re-anchors ([#275](https://github.com/fetch-laboratories/openttd-rust/issues/275)), so these projections are computed from the report's instruction counts and the retained anchors by the [formula](speed.md#instruction-projections). The profile's output is retained in locked `t3-612e96b6-a878391da496d7f7`, the previous tip for attribution at the next profile; the merge-59 profile's `t3-7739ec11-3073cbaa77b28444` stays locked until generation's rise is attributed.

| Scenario | Original Ir | Candidate Ir (merge 71) | Change since merge 59 | Projection (merge 59) | Projection (merge 71) |
| --- | ---: | ---: | ---: | ---: | ---: |
| TGP | 2,265,550,541 | 3,509,053,182 | +24.59% | 1.195938x | 1.775323x |
| Opus | 21,773,080,679 | 27,299,548,829 | -0.71% | 1.345115x | 1.345115x |
| Grok | 20,969,186,256 | 26,097,533,933 | -0.61% | 1.346652x | 1.346652x |
| P1996 | 3,599,266,428 | 4,640,604,505 | +2.09% | 1.352193x | 1.352193x |
| P2000 | 6,042,768,925 | 7,566,851,463 | +1.26% | 1.444531x | 1.444531x |
| P2016 | 11,340,951,303 | 13,852,178,127 | +0.80% | 1.382593x | 1.382593x |
| Drawing (diagnostic) | 2,149,071,227 | 2,154,377,465 | -0.00% | 1.004635x | 1.004635x |

Generation crossed both its band (1.235x) and its ceiling (1.3x) in one interval. Under [speed policy](speed.md#policy) the rise is attributed per function to the intervening merges and fixed forward; the projection doubles the count's rise and is not a wall-time measurement, so the record's wall pass waits for the fix. The play records are on their floors: no play count is above its anchor's, and the highest play record is 1.444531x against the 1.52x band and 1.6x ceiling. P1996 rose 2.09%, under the 3% line; the next profile reads it again.

## Course

- Execution order: the cleanup stage, then phase 3 readers, then phase 2 storage with each pool's save/load, then phases 4, 5 and 6 ([goal and phases](planning.md#goal-and-phases), [ADR 0003](adr/0003-work-order-cleanup-readers-storage.md)).
- Yardstick: net fork C++ removed, counting C++ under `tools/` and `src/tests` ([cost decisions](planning.md#cost-decisions), [ADR 0022](adr/0022-progress-is-net-cpp-removed.md)).
- Limits: [implementer slots](team.md#slots) and the [stocktake cadence](root-workflow.md#cadences).

## Phases

Agent: /root (Claude Code navigation) | Model: claude-fable-5-2 | Reasoning effort: 30 (host-reported number)

Phase numbers name content; the execution order sets the sequence. Phase 1 (simulation logic) exited at integration 63.

Position at merge 59 (`f24c05dbe9`), 2026-11-13. Counts are code lines from `python3 tools/port-metrics.py --whole-fork` (metric version 3, recounted at both endpoints after #978 merged): comments, blanks and preprocessor lines are excluded, and the scope is `src/` plus generated records for all platforms, not one executable. Parentheses compare the last navigation, `78553df0a1` (integration 146), recounted with the same metric; the recount matches its published figures. Areas are accounting buckets; the last column names what removes each.

| Area | Original | Retired (change) | Remaining (change) | Fork glue (change) | Compiled C++ (change) | Removed by |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Simulation logic | 47,424 | 37,861 (+123) | 9,563 (−123) | 5,590 (−277) | 15,153 (−400) | Phase 1 has exited; its residue goes with its consumers: drawing functions in phase 5, startup (`openttd.cpp`) in phase 4, facades and other bodies with their phase-2 owners |
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
| **Source total** | **284,355** | **62,271 (21.90%, +161)** | **222,084 (−161)** | **32,410 (−1,272)** | **254,494 (−1,433)** | |
| C++ under `tools/` | | | | 14,494 (−1,148) | 14,494 (−1,148) | With the pools, readers and boundaries each file reaches (#736's classification) |
| ABI-generated non-record C++ | | | | 5,303 (−172) | 5,303 (−172) | With the records and layouts it checks |
| **Fork C++ census** | | | | | **274,291 (−2,753)** | |

Inventory correction, apart from retirement: no original C++ came back. Version 2 diffed each file against the pin with git's default algorithm, whose alignment shifts when a heavily edited file changes, so it overstated retired and glue by the same amount: 591 lines at the last navigation and 176 at merge 59, and it showed retirement of −254 over a period in which it was +161. Version 3 uses the minimal diff, and the table above is recounted with it; compiled C++ and the census are the same under both. The rates table below still holds version 2's retired figures at endpoints not recounted, and the next navigation recounts it.

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
- The cleanup stage retires no original C++ by design ([ADR 0004](adr/0004-cleanup-before-porting.md)); the movement in that column is the diff noise of #978. Ownership ports retired 1,250 to 2,160 original lines a day before it. At that pace the 222,084 lines left are about 150 days of porting, with no measured pace yet for the GUI, platform or vendored code.
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

1. **Tools batch.** One per stocktake interval, dispatched within it and ranked by its items' measured cost ([selection](planning.md#selection)). #945 merged as #981 in the last interval. This interval's is #1013, dispatched at the stocktake: the network scenario stalls of #693, acceptances that a moved head voids, `ci.py`'s waits and retries, a backtrace when a test binary dies on CI (#1012) and the macOS fallback as one command. New inventory visibility is not C++ deletion.
2. **Shared services and accessors that several files need.** Merged: news and script events (#982), windows and sound (#1001), errors (#1005), the string IDs (#995, #1006). In review: console and chat (#1008). Planned: the framerate service (#1004), `BaseStation::xy` (#1007) and the remaining string-ID fields (#993). Until each merges, a pass keeps the carriers it would replace and names the work order ([design faults](porting.md#design-faults)).
3. **#734/#735 file passes.** One PR per workset in the #734 manifest finishes every file it touches, applying the [code standards](porting.md#code-standards) to every line. Sizing, splits and single-file passes follow [sizing](planning.md#sizing); proof follows [cleanup proof](porting.md#cleanup-proof). Each pass reports its files' residue from `tools/code-standards-inventory.py`.

   Order, by fan-in (measured at `5af17025a3`). Delivered: the 03 remainder and its residue pass, 23's remainder (#884), and 13 Map/terrain with 23's type-bearing files, except 23's `native_test_support.rs` and `native_test_support/movement.rs` (#973).
   - (c) 09 (59), 08 (58), 07 (52), 06 (45), 04 (38), 05 (27). Of 09, `rail_yapf.rs` (#976) and `ship_yapf.rs` (#977) are delivered; 08 is in the [ready queue](#ready-queue). The calls between 04 and 06 run both ways (06 to 04 in 2 files, 04 to 06 in 1), so strict fan-in governs.
   - (d) The rest by fan-in: 16 (67), 21 (62), 15 (42), 18 (37), 10 (33), 11 (28), 14 (27), 17 (26), 12 (25). The leaf worksets 22 (17), 20 (6) and 19 (1) go last. F's closure stays last.

   Twenty-four workset files exceed the single-file pass size; the largest are `station_cmd.rs` (4,868), `orders.rs` (4,841), `town.rs` (3,373), `train.rs` (3,121), `industry.rs` (2,874) and `company.rs` (2,698). #734 reports progress in lines finished as well as files.
4. **#736 test evidence.** No transfer remains. The aircraft range probe stays native under #656: #950 projected 10 to 13 changed lines for each net C++ line, above every delivered transfer, and the user approved closing it ([reason](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6860491029)). The classification table in #736's body names each remaining file's owner or boundary, as of 2026-11-14; that closes the stream's inventory without a PR ([ruling](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6834080910)).

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
- #1012 records one SIGSEGV in the kernel test binary on CI (#1000's `native` run 44379930797, 2026-11-18) that 50 debugger runs each on that head and on the base did not reproduce. It is unplaced, not a base regression. A second occurrence on any PR stops merges until its cause is found; until then it has no owner beyond that trigger.
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
- A plan is reviewed before dispatch, by a reviewer the planner starts and the PR does not reuse ([plan review](planner-brief.md#plan-review), [review brief](review-brief.md#a-plan-review)). A PR's reviewers rule on each exception the PR body records ([behaviour review](review-brief.md#a-behaviour-review), [porting review](review-brief.md#a-porting-review)).
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
- **Two reviewers per group.** Each PR group now gets a behaviour reviewer and a porting reviewer, started with different messages ([starting reviewers](implementer-brief.md#starting-reviewers)). The one-reviewer brief gave a method only for comparing operations with the original, so its reviewers did not check how a body reaches what it uses: a Rust port bypassed for a C++ service, a record resolved once per field, a duplicate accessor. A reviewer spends about 2.2 to 2.9 minutes and 12 to 15 tool calls whatever its brief asks, so adding those checks to one brief displaced the behaviour checks (behaviour findings fell from 5.5 to 0 of 10). Replayed on eight groups of #956, #966, #969, #972 and #976, two runs each with `gpt-6.1-sol`: the original reviewers found 0 of 10 known porting departures; the old brief found 4.5 of 20 across both runs; the porting reviewer found 10 of 20, and on #956 group 2 found duplicate service bindings that 14 earlier runs accepted. The behaviour reviewer scored 3 of 10 against a behaviour key built from the old brief's own findings, short of the bar of 5 set beforehand, but raised about 8 distinct true behaviour defects across its runs against the old brief's 6, among them dropped assertions in `chance16_i` and `GetNextStoppingStation` and airport dimensions narrowed to `u8`. No brief caught a misleading comment, a flag set returned as a raw integer or an adapter missing its `_raw` suffix. Measure, per PR: behaviour and porting findings separately, groups each kind accepted with nothing, and audit departures that both reviewers accepted.
- **Recorded exceptions.** Count the exceptions reviewers ruled on and the findings that came of them; the rule is untested.
- **The tools batch.** Failed runs by cause, and the share of PR-minutes outside implementation, after #945 merges; and whether each later batch was dispatched in its interval.
- **Plans.** Plans cost about 9% of implementer minutes and their file sets were accurate (10 of 12 PRs had at most 4% unplanned lines), so plans in issues stand. Count the plan decisions an audit or reviewer finds contrary to porting (six in this window) and compare estimates with actual changed lines (median 1.54 times).
- **Re-check scope** stands: no re-check raised a behaviour defect outside its fixing commits, and the four sessions beyond two stages were forced by the exact-head acceptance, item 13 on #945.
- **Root's docs pushes.** Ten of 22 landed on a queued PR and rebuilt 13 merge-group runs before `b4e7ca3d73`, none after. Root made 13 ready-queue commits in 8.3 hours; count both again.
- **Implementer slots.** The user made the strict limit of two standing ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)): a waiting implementer keeps its slot. Under the earlier rule 3.0 implementers were building or editing on average and the compile step ran a median 234 seconds against 136 to 140 idle. Measure changed lines merged per hour (3,396 before the limit, 3,660 in the 2.4 hours after it), the compile step's time and how long a slot sits idle behind CI, so that the user can judge the limit on more than one session.

## Ready queue

Undispatched work orders ([team](team.md#ready-queue-and-pipelining)). Each has its own issue, whose body holds its plan once a planner has written it. Root's status comment on #3 holds the queue's state between commits to this section.

Root resumed on 2026-11-18 at merge 65 and took stock at merge 71. Merged in that session: #996 (merge 66, `b25a2347a9`, #983: `vehicle_base.rs` and `vehicle_func.rs`, net glue −3), #995 (merge 67, `9a66db7c7e`, #989: the generated `crate::table::strings`, net fork C++ −89), #1001 (merge 68, `9a00e0cf2e`, #979: window and sound leaves, −125), #1002 (merge 69, `48b5eddcf6`, #973: `native_test_support.rs`, no C++ added), #1005 (merge 70, `7d78b2c2b7`, #990: error leaves, −58) and #1006 (merge 71, `603724a3fd`, #992: thirteen `*Strings` records, −229). Residue that passes left in files they did not finish is [on #734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6976329602).

The user raised the implementer limit to three for the rest of this session; [ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)'s standing limit of two is unchanged. The three slots hold #1000 (for #964, 8r, `road.rs`; net fork C++ −124), reviewed, with `native` and one acceptance review running at its final head; #1008 (for #991, the console and `NetworkTextMessage` leaves; −70), reviewed and queueing behind #1000; and #1013 (the tools batch), dispatched at the stocktake.

1. **The fix for map generation's instruction rise** ([speed](#speed)), once /root/attribute-tgp-71 has placed it; root opens its issue from that report. It takes the first slot that frees.
2. #1004: the shared framerate service, nine of eleven timing sites: about 670 changed lines and −6 fork C++. Planned and reviewed twice at `2e10fffd9b`, with [root's answers](https://github.com/fetch-laboratories/openttd-rust/issues/1004#issuecomment-6989526426) in the plan. Waits on nothing; it goes before #986.
3. #984: 7r-2, the single-file pass for `vehicle.rs`, next in workset 07 (vehicle ownership): about 2,050 changed lines and −40 fork C++. Planned and reviewed at `457208977e`, with [root's answer](https://github.com/fetch-laboratories/openttd-rust/issues/984#issuecomment-6978558930) in the plan. Waits on #1000, which finishes `road.rs` under it.

**Later.** Planned, reviewed and waiting for a slot: #1007, one shared accessor for `BaseStation::xy` (about 200 changed lines, −11 fork C++), and #993, the 107 string-ID fields left in 25 tables (about 1,000 changed lines, −163). Waiting on #984's PR: #1009, 8t, the single-file pass for `train.rs` (about 5,800 changed lines and −390 fork C++, five review groups; planned at high effort against #1000's head), and #985, 7r-3, the pass for `vehicle/maintenance.rs` (about 2,800 changed lines and −240; [root's answer](https://github.com/fetch-laboratories/openttd-rust/issues/985#issuecomment-6978554257)). After those: #986 (`vehicle/scheduler.rs`, about 2,080 changed lines, with the day virtuals' dispatch moved into Rust; after #1004, #984 and #985), #1003 (`vehicle/occupancy.rs`, waiting on #986's PR, not yet planned), #987 (`vehicle/visits.rs`, about 2,300 changed lines and −90; after #984 and #985) and #1010, 8tr, the single-file pass for `train_reservation.rs` (about 2,000 changed lines and −300; after #1009's PR, its plan refreshed and reviewed again at dispatch). Whichever of #1009 and #987 merges first moves the train case of `VehicleEnterDepot` into `vehicle/visits.rs`. #1011 moves `Vehicle::Tick`'s dispatch into Rust with the four controllers' accumulators; it waits on #1004, #986, #1009 and the `ship_control.rs` pass. The rest of workset 08 is one pass for `effect.rs`, `vehicle_effect.rs`, `ship_control.rs` and `disaster.rs`, which its planner splits if it must; it has no issue yet. After worksets 07 and 08 the fan-in order gives 06, 04 and 05. The train passes take their sites of #999 (order destination reads at the original's observation point).

#998 is the work order for one host-buildable crate below `openttd-kernels`, which removes the copies of three kernels ports that `openttd-strgen` keeps under [root's ruling](https://github.com/fetch-laboratories/openttd-rust/pull/995#issuecomment-6975789181). Root reconsidered it at this stocktake: no second build-time consumer has appeared and it removes no fork C++, so it stays behind the passes, selected no later than strgen's port in phase 4. #997 (the native assertion service reports neither the Rust site nor its predicate) is on #892's checklist and rides the closure residue sweep.

Full phase-3 planning remains untriggered: about 160 of #734's 319 files are in no entry.

## Resume checkpoint

**Paused at the user's request (2026-11-15), after the two drafts the last checkpoint held were merged.**

Root resumed at merge 63 on 2026-11-15 with the user's scope of finishing and merging #994 and #977 and then winding down, and stopped at merge 65 (`d8265f9308`), with no rollback and no confirmed base regression. No PR is open, the merge queue is empty, both slots are free and no implementer or reviewer is running. Nothing was dispatched from the [ready queue](#ready-queue) and no planning ran. No implementation, repair, review, build, simulation, CI request or retry is authorized until resume. Where GitHub is newer than this section, GitHub is right.

Agent: /root | Model: claude-fable-5-2 | Reasoning effort: high

| Merged this session | Merge | What it left |
| --- | --- | --- |
| #994, for #975, uncalled C++ free functions | 64, `f139c117b2` | The uncalled-function report still lists 70 functions: 62 with callers the scan cannot see, five track and road helpers that only the rails and signals comparison adapters call, and three ship-region functions. The 62 false reports are a [candidate on #939](https://github.com/fetch-laboratories/openttd-rust/issues/939). |
| #977, for #970, `ship_yapf.rs` pass | 65, `d8265f9308` | `yapf_ship.cpp` keeps its two entry functions until `ship_control.rs`'s pass. `road_yapf.rs` and `rail_yapf.rs` lack the same-list duplicate-key assertion that ship now has; [#971 carries it](https://github.com/fetch-laboratories/openttd-rust/issues/971#issuecomment-6902158343). |

On resume, root dispatches the ready queue's first two entries, #989 and #983, to the two slots; each plan was reviewed at merge 63, so each implementer is told the base has moved by #994 and #977.

Implementation runs on `claude-opus-5-5`, medium, planning on `claude-fable-5-2`, medium, routine work on `claude-sonnet-5-5`, medium, and review on `gpt-6.1-sol`, high.

CI variables were not rechecked this session, and no workflow change or bootstrap Full ran. The hooks of #927 are installed in the shared clone (`python3 tools/git_hooks.py install`).

| Other work | State |
| --- | --- |
| Ready queue | As the [ready queue](#ready-queue) above has it. Root's status comment on #3 has the session's notes. |
| Tools batch | #945 merged as #981 (merge 61), the batch of the interval that starts at merge 60. The candidates root recorded on #939 on 2026-11-14 and [2026-11-15](https://github.com/fetch-laboratories/openttd-rust/issues/939#issuecomment-6902158633), each with its count, wait for the batch picked at the merge-71 stocktake. |
| Platform cadence | Passed at merge 62 (run 43566394953). Next after merge 67. |
| Stocktake | Measured at merge 59, and root's half is done. Next after merge 71; it counts the news and event leaves converted since #982 against its figure of about 5.9 C++ lines a leaf. |
| Phase-3 planning | Not triggered. |

Flakes and limits carried forward:

- #693: this session saw three network-timing failures with no code cause, each cleared by one unchanged retry: a `network-protocol-loopback` hang on #916, the `network-protocol` hang that removed #932 (run 43147704245) and the reference-side `content-protocol` miss that removed #935 (run 43150720812). Earlier windows recorded 2 such failures in 42 simulation-tier runs, then none among 63 native and queue requests at the merge-20 stocktake; the windows are not combined into a rate. No source or oracle change and no new local network diagnosis is authorized by this checkpoint. Keep `.local/retained-network-693` and the linked artifacts; no candidate or base regression is inferred from a timeout.
- #693, session of 2026-11-13: one `network-protocol-loopback` hang, on #961's `native` run 43347645519, cleared by one rerun of the failed shard.
- #693, session of 2026-11-14: one `network-protocol-loopback` hang, on #988's `native` run at `ee3d2c335a`, cleared by one re-request; about eleven `native` requests ran in the session.
- #693, session of 2026-11-15: one `network-protocol-loopback` hang, in #994's first merge-queue run, cleared by one `enqueue --retry`; about six `native` requests and three queue runs ran in the session.
- Two background jobs were killed by a signal with no sender identified on 2026-11-13 (#724), and the merge-59 profile's first run ended on its own agent's 30-minute limit and was rerun once.
- Metric scope completeness stays with #736's end-state inventory. No later phase is selected. #898 is a partial evidence transfer; native ownership, loader and error observations, pool and ABI identity and retained fixture consumers remain as its accepted report states.

Retained state for housekeeping:

- `python3 tools/housekeeping.py --apply` ran at this session's start, removing 19 worktrees (7.6 GB) and leaving the SSD 53 GB free, and runs again at this wind-down for the two merged PRs' checkouts; root's status comment on #3 has that run's result.
- A name below protects a worktree only when it is the whole folder name ([housekeeping](root-workflow.md#housekeeping)). The `t3-` names of eight hex digits in this list are whole folder names from an earlier host version.
- No PR is open, so no PR checkout is kept. #977's two measurement worktrees are removed; its PR body records their figures.
- Keep the locked merge-59 profile checkout `t3-7739ec11-3073cbaa77b28444`. Preserve all current speed, deep and evidence anchors; the speed anchors live in the main checkout's `.local/speed-pass`.
- Six older profile checkouts were released on 2026-11-12 (#924); this file no longer names them, so the next housekeeping run removes them.
- The list below is carried forward unchanged as preservation guidance from earlier checkpoints; it does not assert that every old checkout still exists. `/steer` may reconcile stale references, preserving uncommitted or unpushed work and evidence.

- Keep the planning checkout `t3-20fec9ce` with its private plan, and origin `preserve/housekeeping-foundation-6ab9c20e5d`. Preserve every checkout with uncommitted or unpushed work.
- Keep `.local/worktrees/root-profile-125`, `.local/worktrees/root-profile-119` and `.local/worktrees/root-profile-113` with accepted reports, raw Callgrind and frozen runtimes. Keep `t3-a490295e` (profile 131, clean `305ac20c28`) with its HDD `.local` outputs. Profile 119 measures clean `4d3a1d1d41`, not integration 120.
- Keep main `.local/integration-records/`, `.local/speed-pass/` anchors, frozen runtimes and the retained simulation evidence.
- Keep `t3-f4cf7a0b` (one unique unpushed commit, `125ea2440a`). Leave the staged 11-08 trees `t3-03c1e2da`, `t3-1ddaadfd`, `t3-5575ebe4`, `t3-83082b90`, `t3-c68d914a` and `t3-e0361d8c` untouched.
- Leave dirty or staged trees untouched: `t3-3b491ddb` (five files), locked #565 WIP `t3-431c84b4`, locked native evidence `t3-d78d5f8b`, `t3-32cf4edc` (`tools/simulation/roads.py` staged) and `t3-e130d5a3` (`migration/saves/README.md` staged).
- Keep protected `.local/worktrees/root-docs-72` and locked `.local/worktrees/profile-after*`. Do not push stale root-docs guidance over current main. Keep the pinned original unchanged.
