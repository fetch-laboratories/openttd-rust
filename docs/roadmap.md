# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-11, stocktake after merge 20)

Agent: /root | Model: gpt-6-astra | Reasoning effort: xhigh

**Measured endpoints.** `c12561671f` (merge 8, integration 155) to `65c62aeaa7` (#885, merge 20, integration 167), 2026-11-10 11:35:57–23:59:00 UTC. The same-version census spans 0.501204 calendar days: fork C++ fell 432 code lines (862/day), source glue fell 660, generated non-record evidence fell 75, tracked tools C++ rose 2 and fork test glue fell 1. Original retirement reads 301 lower because #872 reclassified 343 station-command lines from glue to original; this is not restored code or a behavior regression. The source-area table below reports the measured totals without turning that classification into retirement credit.

**Delivered.** The twelve merged PRs are #872, #873, #871, #876, #878, #877, #882, #879, #883, #884, #886 and #885. They finish the 01–03 residue and assertion repairs, further map/vehicle/support/text passes, V and W boundary batches, thirty O chains in total, oracle compaction and the tools/scope follow-up. #738's final docs item is delivered. No phase exit or whole-file completion is inferred from a boundary-only edit.

**Root's reading.** Continue cleanup and the callee-first file order. #734 has 92 of 319 manifest entries finished; #737's baselines and boundary shrink. #736's counted `tools/` plus `src/tests` inventory did not fall (20,224 to 20,225), despite the oracle-data compaction. The boundary slot therefore returns directly to the ranked #736 evidence families after the already-dispatched I batch; no further #737 batch goes ahead of them. Their first checkpoint identifies the behavior that actually moves, and actual source overlaps with 7a/9m are reserved or serialized. The measured speed projections trigger no wall pass. Full phase-3 planning remains premature: 227 file-pass entries, closure, evidence families and O's remaining chains are not all dispatched or Ready.

**Process in the endpoint interval.** Created-time API windows returned 634, 380 and 395 runs, each below the 1,000 cap, with 1,409 distinct IDs. Twenty queue workflow runs finished: nineteen passed; #885's first queue run failed Clang initializer/unused-variable annotations, fixed before retry. Forty-three native dispatches finished: twenty passed, sixteen cancelled and seven failed. These are workflow runs, not counts of PRs or accepted changes. For the twelve merged PRs, the median last report matching the merged head to merge was 22.31 minutes; matching report text alone does not establish acceptance.

- The 103 repository-wide Quick PR runs were 36 passed, 33 failed and 34 cancelled. Failed-step classifications: five force-warn Rust ratchet, eight Python policy, four boundary growth, one archive proof and fifteen Rust checks. All five ratchet failures belong to #871. Across eleven integrated component PRs (excluding tools-only #882), that is 0.455 per PR, below the provisional 0.5 target. It is interval-bounded, not directly comparable to earlier whole-PR-lifetime rates.
- The bounded #693 review found zero hang signatures among 63 native-or-queue requests. Thirty-eight reached simulation shards; one stopped during package installation, so 37 reached games. All fourteen queue runs with shards passed them. Inspected non-success native shard logs show one harness failure, two cancellations after progress and one installation cancellation. This excludes platform/bootstrap workflows and does not prove every request exercised the network scenario or every scenario log was audited. No earlier diagnosis is selected from this interval.
- After the endpoint, #887's mixed original-server/candidate-client reconnect hang was [recorded separately](https://github.com/fetch-laboratories/openttd-rust/issues/693#issuecomment-6783491542); its single unchanged-head failed-job retry passed. The cause remains unknown, and the two-run original/original authorization remains exhausted.

At report preparation #887 and I/#888 are the two open component PRs. The collector's earlier snapshot contained 47 remote refs not merged into the fixed endpoint, including upstream and historical refs; it is not an active-work count. Issue placement is 40 open, zero unplaced, #513 conditionally excluded, and one issue-less historical pause row. The Ready queue explicitly names the remaining network files' #734/#735 owner and return condition. Neither near-full reproduction nor the release-source trigger is met.

Measurements are retained in `t3-9bfeabfd-b8f486fd2881369f/.local/merge20-facts.txt` and its named census, inventory, CI and latency artifacts. Reproduce the endpoint census with `python3 tools/retrospective.py stocktake c12561671f 65c62aeaa7 --json`; scope is metric v2, implementation `3c413e4da0199fc6af536a704cc083318ced6c6ec75ed245ab3631229a28d379`. Counts are inventories, not ownership or phase-exit claims.

| Measured quantity | Merge 8 (integration 155) | Merge 20 (integration 167) |
| --- | ---: | ---: |
| Original C++ remaining | 221,656 | 221,957 |
| Fork source glue, including generated records | 33,651 | 32,991 |
| Compiled source and generated records C++ | 255,307 | 254,948 |
| Tracked tools C++ evidence | 15,437 | 15,439 |
| ABI-generated non-record C++ evidence | 5,442 | 5,367 |
| Measured fork C++ series | 276,186 | 275,754 |
| Original C++ retired | 62,699 of 284,355 (22.05%) | 62,398 of 284,355 (21.94%) |
| Simulation C++ retired | 41,539 of 61,148 (67.93%) | 41,231 of 61,148 (67.43%) |
| Expanded Rust exports / callback fields | 1,697 / 3,340 | 1,664 / 3,261 |
| Historical regex exports / callback slots | 1,536 / 3,298 | 1,508 / 3,219 |
| Shared-accessor / rebound-callee / orphan-export baseline rows | 87 / 279 / 30 | 48 / 245 / 27 |
| Exports held only by uncalled members (not enforced) | 53 | 23 |
| Retained Clippy keys / diagnostics | 54 / 83 | 54 / 83 |
| Ruff keys / diagnostics | 287 / 287 | 287 / 287 |
| Raw GLOBAL Rust / tooling inventories | 203,275 / 64,412 | 175,632 / 65,718 |
| Play-save speed ratio | No wall pass; projections 1.345x–1.445x | No wall pass due; projections 1.345x–1.445x |

Fork-added `src/tests` C++ is included in source glue (4,787 to 4,786); generated record declarations also remain inside that series. The separate embedded inventory is 836 code lines in Python strings, 158 in oracle recipes and 21 original-body extraction sites. Its prior same-tool values were 836, zero and 20: recipe/extraction additions are scope changes, not deletion credit, and embedded identities are not silently added to the census. Raw Rust fell 27,643 lines, including #877 oracle compaction; raw tooling rose 1,306. These raw inventories include data and do not measure executable-code growth alone. The #736 end-state classification remains outstanding.

| Source area / phase relationship | Original | Retired | Remaining | Fork glue |
| --- | ---: | ---: | ---: | ---: |
| Simulation logic (phase 1) | 47,424 | 37,992 | 9,432 | 5,913 |
| Simulation storage (phase 2) | 13,724 | 3,239 | 10,485 | 1,385 |
| NewGRF, save/load, script API and network (phase 3) | 51,794 | 13,074 | 38,720 | 3,354 |
| Support, graphics and viewport (phase 4) | 32,602 | 980 | 31,622 | 548 |
| GUI and platform (phase 5) | 64,120 | 415 | 63,705 | 465 |
| Vendored (phase 6) | 55,148 | 2,317 | 52,831 | 0 |
| Data tables | 17,295 | 4,367 | 12,928 | 14 |
| Upstream tests and fork test glue | 2,248 | 14 | 2,234 | 4,786 |
| Boundary layer | 0 | 0 | 0 | 16,526 |

Coverage remains bounded by the existing saves, NewGRFs, scenarios and retained native fixtures; entry witnesses are not branch coverage. Three interval #156 reports qualify [subsidy edge/assertion cases](https://github.com/fetch-laboratories/openttd-rust/issues/156#issuecomment-6767494506), [invalid-kind and allocation paths in vehicle construction](https://github.com/fetch-laboratories/openttd-rust/issues/156#issuecomment-6779349307), and [empty StringConsumer search with assertions disabled](https://github.com/fetch-laboratories/openttd-rust/issues/156#issuecomment-6779547670). #884 repaired the latter with a focused regression; this does not close unrelated existing gaps. #503 deep checks remain user-requested.

### Cleanup stream inventories

Historical steering columns are retained; root adds the merge-20 measurements here at the stocktake ([selection](planning.md#selection)). The comment, integer, wrapper, constant and literal rows are analyzer estimates from `tools/code-standards-inventory.py`, not automatic source findings. Finished-file figures use the same manifest subset: 92 entries, 91 existing files plus deleted `tree_cmd.rs`. At merge 20 the analyzer matched 1,633 bodies overall and 655 in that subset. FFI and record base integers may be permitted by #355; apparent missing comments still require source matching.

| Stream | Inventory | Overhaul | Integration 129 | Integration 143 † | Integration 146 (steering 33) | Integration 155 † | Merge 20 (integration 167) † |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| #734 | Workset files with a finished pass † | 0 of 319 | 0 of 319 | 41 of 319 | 51 of 319, with #832's ten | 54 of 319 (#832's ten, #839, #852, #868) | 92 of 319 |
| #734 | Workset lines in finished files † | – | – | 32,080 of 176,930 (18.1%) | – | 46,082 of 183,789 (25.1%) | 60,266 of 187,833 (32.1%) |
| #734 | Explanatory comment lines missing inside matched ported bodies | 3,139 | 2,827 | 2,715; 129 in finished files | 2,693; 133 in finished files | 2,577; 152 in finished files | 2,304; 136 in finished files |
| #734 | Bare-integer parameters and fields whose concept has a named type | 1,957 | 1,947 | 1,766; 118 in finished files | 1,772; 121 in finished files, many of them FFI or `repr(C)` boundaries that #355 allows | 1,695; 150 in finished files | 1,664; 170 in finished files |
| #734 | Transparent wrapper types / named constants | 69 / 2,181 | 266 / 3,411 | 285 / 3,466; 37 / 332 in finished files | 287 / 3,482 | 294 / 3,538; 47 / 435 in finished files | 303 / 3,662; 53 / 688 in finished files |
| #734 | Literal sites standing for a named value | – | 14 | 14; 0 in finished files | – | 12; 0 in finished files | 12; 0 in finished files |
| #735 | Functions over seven parameters | 150 | 141 | – | 132 (70 FFI exports, 39 helpers, 23 commands) | – (no tracked counter; #874) | 121 (70 FFI, 25 helpers, 26 commands) |
| #735 | `too_many_arguments` ratchet rows / `allow` sites † | 71 / – | 63 / 19 | 53 / 11 | – | 48 / 11 | 48 / 11 |
| #736 | Fork C++ evidence under `tools/` / `src/tests`, code lines † | – | 16,869 / 5,684 | 15,821 / 5,615 | 15,642 / 5,279 ([Phases](#phases)) | 15,437 / 4,787 | 15,439 / 4,786 |
| #737 | Shared-accessor / rebound-callee / orphan-export rows † | 117 / 314 / 42 | 117 / 314 / 40 | 104 / 304 / 39 | – | 87 / 279 / 30 | 48 / 245 / 27 |
| #737 | Exports held only by uncalled members † | – | – | – | – | 53 | 23 |
| #738 | Rule corpus lines † | 3,312 | 3,362 | 3,229 | 3,231 before this review | 3,251 | 3,237 |
| None | Ruff rows | 296 | unchanged | 287 | – | 287 | 287 |

Counting commands:

- **#734:** reconcile the issue workset manifest with merged complete-file pass PRs, then count current files with `wc -l`; a deleted finished entry remains in the denominator, and mechanical caller edits do not finish a file. The merge-20 artifact `merge20-finished-files.txt` records the 91 extant paths behind 92 finished entries. Run `python3 tools/code-standards-inventory.py [--head REV]` for the tree and with those paths for the finished subset. Current line totals change when shared callers change, so merge 19's 60,295/187,808 is not substituted for merge 20's 60,266/187,833.
- **#735:** use the tracked long-signature inventory counter (#874/#882), `migration/clippy-debt.json` (`too_many_arguments` keys) and `git grep 'allow(clippy::too_many_arguments' -- rust/`.
- **#736, #737 and the Clippy and Ruff rows:** from `python3 tools/port-metrics.py --whole-fork`, `python3 tools/orphan-exports.py --report-uncalled`, `wc -l` of the three baseline files, and the debt files (keys are `sites` entries, diagnostics the sum of their list lengths). Expanded exports: `growth_inventory` in `tools/port-metrics.py`.
- **#738:** `cat AGENTS.md $(git ls-files 'docs/*.md' 'docs/**/*.md' | grep -v -E '^docs/(roadmap.md|components/)') | wc -l`.

### Speed

[Merge-20 profile](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6783388958) on clean `65c62aeaa7` used `simulate --instructions --jobs 2`. All seven canonical exit saves and full logs match, and post-build identity preparation accepted the current host/tool/runtime identities. No anchor changed. Retained artifacts are in locked `t3-9bfeabfd-b8f486fd2881369f/.local/simulation/20261111T002248Z-186703/` and `.local/speed-pass/merge20-identity/`; the previous profile remains in locked `t3-ee66f61c`.

| Scenario | Original Ir | Candidate Ir (merge 20) | Projection (merge 8) | Projection (merge 20) |
| --- | ---: | ---: | ---: | ---: |
| TGP | 2,265,528,237 | 2,774,994,754 | 1.163070x | 1.161260x |
| Opus | 21,773,343,583 | 27,411,470,613 | 1.345115x | 1.345115x |
| Grok | 20,969,727,163 | 26,174,670,001 | 1.346652x | 1.346652x |
| P1996 | 3,600,502,896 | 4,505,830,818 | 1.352193x | 1.352193x |
| P2000 | 6,049,922,238 | 7,408,317,235 | 1.444531x | 1.444531x |
| P2016 | 11,343,424,651 | 13,711,755,287 | 1.382593x | 1.382593x |
| Drawing (diagnostic) | 2,149,015,961 | 2,154,439,219 | 1.004635x | 1.004635x |

No policy projection rose 3% or entered its band. Generation remains below 1.235x/1.3x band/ceiling; play remains below 1.52x/1.6x. Drawing has no policy ceiling. No wall pass is due, and these instruction projections are not new wall-time measurements.

## Course (user, 2026-11-06)

- **Execution order:** the cleanup stage, then phase 3 readers, then phase 2 storage with each pool's save/load, then phases 4, 5 and 6 ([planning](planning.md#goal-and-phases)).
- **Yardstick:** net fork C++ removed, counting C++ under `tools/` and `src/tests`. Rust exports plus callback slots fall or hold; growth needs a reason the reviewer accepts ([cost decisions](planning.md#cost-decisions)).
- **Limits:** at most two implementation agents (user, 2026-11-10; [team](team.md#roles-and-models)); stocktake and profile every twelfth merge (user, 2026-11-08; every sixth from 2026-11-06, and stocktake 137 is skipped).
- **Enforcement:** checks enforce mechanical rules on every push (#733); reviewers work from the [review checklist](review-checklist.md); implementers read the [implementer brief](implementer-brief.md) and their work order.
- **Process (user, 2026-11-10):** #860 lists the changes from the development-process review: a merge queue, Windows x86, Clang and GCC Dedicated in Full, cached CI packages, one Clippy run, scenario comparison on CI, direct reviewer and implementer messages, and a checkpoint written at wind-down only.
- **Sizes, never durations:** plans, issues and briefs size work in counts (files, lines, call sites), never in time (user, 2026-11-06).

## Phases

Agent: /root (Claude Code navigation) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host

Phase numbers name content; the execution order above sets the sequence. Phase 1 (simulation logic) exited at integration 63.

Position at code `61c6d2c598` (integration 146), docs `89b6807754`, 2026-11-09. Counts are code lines from `python3 tools/port-metrics.py --whole-fork` (metric version 2, implementation `79b7d225f6`): comments, blanks and preprocessor lines are excluded, and the scope is `src/` plus generated records for all platforms, not one executable. Parentheses compare the last navigation, `2d233f628b` (integration 88), recounted with the same metric. The recount matches its published figures, so there is no inventory correction. Areas are accounting buckets; the last column names what removes each.

| Area | Original | Retired (change) | Remaining (change) | Fork glue (change) | Compiled C++ (change) | Removed by |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Simulation logic | 47,424 | 38,329 (+114) | 9,095 (−114) | 6,458 (−172) | 15,553 (−286) | Phase 1 has exited; its residue goes with its consumers: about 2,400 lines of drawing functions in phase 5, startup (`openttd.cpp`) in phase 4, and 1,984 facade lines and other bodies with their phase-2 owners |
| Simulation storage | 13,724 | 3,212 (+288) | 10,512 (−288) | 1,387 (+16) | 11,899 (−272) | Phase 2 |
| Save/load | 13,388 | 1,435 (+100) | 11,953 (−100) | 1,070 (+88) | 13,023 (−12) | Phase 3, except about 4,300 lines of pool and map chunks and their compat tables, which go with phase 2 |
| Script API | 14,908 | 2,406 (+12) | 12,502 (−12) | 750 (+12) | 13,252 (0) | Phase 3; VM and binding shims in phase 6 |
| Network | 12,713 | 3,253 (+676) | 9,460 (−676) | 1,168 (+126) | 10,628 (−550) | Phase 3 |
| NewGRF | 10,785 | 5,973 (+5) | 4,812 (−5) | 372 (+1) | 5,184 (−4) | Phase 3 |
| Support | 21,617 | 973 (+20) | 20,644 (−20) | 547 (+77) | 21,191 (+57) | Phase 4 |
| Graphics and viewport | 10,985 | 7 (0) | 10,978 (0) | 4 (0) | 10,982 (0) | Phase 4 |
| Data tables | 17,295 | 4,367 (+1) | 12,928 (−1) | 14 (+1) | 12,942 (0) | Each table with its last C++ consumer, in phases 2 to 5. The largest are `town_land.h` (2,206), `build_industry.h` (1,468) and `sprites.h` (1,278) |
| GUI | 52,179 | 415 (+70) | 51,764 (−70) | 463 (+71) | 52,227 (+1) | Phase 5 |
| Platform | 11,941 | 0 (0) | 11,941 (0) | 2 (0) | 11,943 (0) | Phase 5 |
| Vendored | 55,148 | 2,317 (0) | 52,831 (0) | 0 (0) | 52,831 (0) | Phase 6 |
| Upstream tests and fork test glue | 2,248 | 14 (0) | 2,234 (0) | 5,279 (−406) | 7,513 (−406) | Fork tests in cleanup (#736). Upstream tests go with the code they test; Catch2 goes when the last C++ test leaves |
| Boundary layer | 0 | 0 (0) | 0 (0) | 16,759 (−822) | 16,759 (−822) | Each side's owner; the final exit |
| **Source total** | **284,355** | **62,701 (22.05%, +1,286)** | **221,654 (−1,286)** | **34,273 (−1,008)** | **255,927 (−2,294)** | |
| C++ under `tools/` | | | | 15,642 (−1,497) | 15,642 (−1,497) | Cleanup (#736), except named boundary checks |
| ABI-generated non-record C++ | | | | 5,475 (−310) | 5,475 (−310) | With the records and layouts it checks |
| **Fork C++ census** | | | | | **277,044 (−4,101)** | |

Of the 21,117 evidence lines, 19,465 are candidate or shared, none is reference-only and 1,652 are unresolved, which the census counts as candidate. Outside the census are the 936 raw lines of C++ in `COMMAND_SOURCE` (`tools/simulation/stations.py`), compiled into fixture games, and the reference-only extracted-original comparison fragments; #736 owns both (batch T and its scope follow-up). The boundary has 1,539 exports (−3) and 3,332 callback slots (−304). Rust has 191,899 code lines (+53,663), of which about 37,000 are oracle text added since the last navigation (`*-oracle.txt` under `rust/`); tooling has 61,896 (+3,227).

Rates use commit timestamps through each endpoint, in calendar days, not agent time:

| Period | Days | Original retired | Per day | Fork C++ census change | Per day |
| --- | ---: | ---: | ---: | ---: | ---: |
| Fork start `6ced7c097e` to navigation `8fb4499105` | 25.93 | +50,111 | 1,933 | +3,023 | +117 |
| To navigation `c05fdd45e6` | 4.83 | +6,065 | 1,256 | −1,578 | −327 |
| To navigation `2d233f628b` (integration 88) | 3.47 | +5,239 | 1,509 | −4,655 | −1,341 |
| To steering 31, `220e138f8c` | 0.54 | +735 | 1,358 | −277 | −512 |
| To the pause before cleanup, `e800717540` (integration 102) | 0.28 | +600 | 2,157 | −1,113 | −4,001 |
| To steering 32, `0cb2f766b9` (integration 129) | 0.78 | −40 | −51 | −796 | −1,018 |
| To integration 131, `305ac20c28` (paused for steering 32 and the overhaul evaluation) | 1.04 | 0 | 0 | 0 | 0 |
| To now (integration 146) | 0.43 | −9 | −21 | −1,915 | −4,476 |
| **Since the last navigation** | **3.07** | **+1,286** | **419** | **−4,101** | **−1,335** |
| **Cleanup stage so far (integrations 103 to 146)** | **2.25** | **−49** | **−22** | **−2,711** | **−1,204** |
| **Whole history** | **37.30** | **+62,701** | **1,681** | **−7,311** | **−196** |

What set the rate:
- Ownership ports retired 1,250 to 2,160 original lines a day through integration 102. The cleanup stage retires none by design (user, 2026-11-06). Its net −49 follows glue edits in original-derived files; root traced the −9 at integration 138 to that reclassification, not restored original code.
- The cleanup still cut the fork C++ census by 2,711 lines, close to the pace of the porting period before it. #736 supplied most of it: C++ under `tools/` fell 1,795 lines, and compiled C++, including `src/tests`, fell 869.
- The 33 vocabulary PRs (integrations 109 to 126) and the 1.04-day pause for steering 32 took the stage's first half. Once file passes began at integration 132, #734 finished 41 files and 32,080 lines in 12 integrations (stocktake 143).

| Order | Stage | Size now | Exit (end state) | Status |
| ---: | --- | --- | --- | --- |
| 1 | Cleanup | #734: 227 of 319 entries and 127,567 of 187,833 raw lines unfinished (merge 20). #735: 121 long functions, 48 debt keys and 11 allows. #736: 15,439 counted tools C++ lines and 4,786 fork test lines, plus separately inventoried embedded C++. #737: I and O's 18 evidence-held chains remain; W/V and the three scanner classifications are delivered, with five network exports retained for #730. #738 is delivered | #734: all 319 files finished and closure (work order item 6) done; #735 closes with it. #736: each remaining C++ file under `tools/` and `src/tests`, and any embedded C++, checks the calling boundary itself and names the boundary whose retirement removes it ([evidence](porting.md#evidence)). #737: batches W, V, I and O delivered and its three withheld scanner rows classified (I and O added by root on 2026-11-10, from #839's review and #856's scanner rule). #738: its last item applied. Lint ratchets shrink per PR but set no exit, and the structural Clippy rows are outside it. Read each end state from the code and inventories, not from issue state | Current. #736 and #737 were reopened at this navigation: #810's "Closes #736 batch U" and #814's "It does not close #737" closed them on merge |
| 2 | Phase 3: readers (script API, NewGRF, network, content, admin, save/load outside pool chunks) | About 37,800 compiled lines: script API 13,252, network 10,628, NewGRF 5,184, and save/load 13,023 less the pool and map chunks. The [queued owners](#after-cleanup), planned at integration 67, estimate 4,600 to 7,000 net C++ removed | Readers run in Rust with evidence for their observable outputs. Every remaining body in these buckets has a named later owner: VM and binding shims phase 6, script and network GUI phase 5, pool chunks phase 2 | Next. Plan it in full before cleanup's last items are dispatched ([goal and phases](planning.md#goal-and-phases)) |
| 3 | Phase 2: storage, each pool with its save/load chunks, then the map | 11,899 compiled lines in storage, about 4,300 save/load chunk lines, the logic residue assigned to pool owners, and record field routes in the 16,759-line boundary layer | Rust owns every pool's records, slots, allocation, iteration, lifetime and save/load, then map allocation and remaining map bodies; no Rust-used field behind a native service | Queued; placement in [accepted placement](#accepted-reader-and-pool-placement-2026-11-03-after-integration-67) |
| 4 | Support | About 32,200: support 21,191 and graphics and viewport 10,982, plus their data tables and startup | As in [planning](planning.md#goal-and-phases) | #3; plan deferred until adjacent (user, 2026-10-28) |
| 5 | GUI, drawing and platform | About 64,200: GUI 52,227 and platform 11,943, plus simulation and data-table drawing | As in [planning](planning.md#goal-and-phases) | As phase 4 |
| 6 | Vendored and final retirement | Vendored 52,831, plus whatever boundary, test and evidence C++ remains | Every original and fork-written C++ line retired and candidate native targets removed ([goal and phases](planning.md#goal-and-phases)) | As phase 4 |

Why the course changed at this navigation (cited at `89b6807754`):
1. The cleanup exit (`roadmap.md:124`) named no end state. "Lint and growth baselines shrinking" is a trend, and "streams done" could be read from issue state. GitHub had closed #736 and #737 on merges, leaving about 21,000 test C++ lines and two #737 batches. The exit now names end states, both issues are reopened, and the PR template no longer opens with `Closes #<issue>`.
2. Nothing planned phase 3 in full before cleanup ends: `start-development` step 3 planned only more cleanup. The queued owners cover about a sixth of phase 3's buckets. Planning and step 3 now plan it first.
3. Goal restatements had drifted. `AGENTS.md:4` retired only fork C++ "glue", against the 11-06 yardstick that counts tools and tests; `start-development` said "simulation first" and stocktakes "every sixth integration". All three now match the user's decisions.

Root's first actions on resume:
1. Follow the [resume checkpoint](#resume-checkpoint).
2. Drive each cleanup stream to its end state above. Check that #736's and #737's bodies list their remaining batches; record delivered batches there.
3. Once every remaining cleanup item is dispatched or in the ready queue, spawn the phase-3 planner ([start-development](skills/start-development/SKILL.md) step 3).

## Cleanup work order

Each stream issue holds its batches, rules, proof and inventory. [Selection](planning.md#selection) ranks cleanup work by stream inventory. A file belongs to at most one open PR across all streams: check open PRs' file sets before starting. Files whose edits merge mechanically are exempt, including `lib.rs` module lists, `migration/*-debt.json`, `migration/harness-inventory.json` and generated registries; resolve those edits when updating from base. Cleanup PRs merge on their stream's proof, review and the merge queue's Full, growing neither fork C++ nor the boundary ([integration](root-workflow.md#integration)).

Slots (two implementers; user, 2026-11-10): while ready worksets remain, one slot runs #734/#735 file passes and the other runs items 2 to 4 in order. A running profile counts as one of the two. #738 runs as one docs agent outside the implementer count.

1. **#734/#735 file passes.** One PR per workset in the #734 manifest finishes every file it touches, in one pass:
   - the original's wrapper types in internal signatures, return types and accessors. FFI exports, service tables and generated declarations keep base integers (#355);
   - named constants wherever a literal stands for a named value;
   - the original's explanatory comments at the matching branches;
   - #735's argument and context structs for the functions defined in those files;
   - the original's shared helpers, called where the original calls them. Where the helper has no Rust port yet, the pass adds it once in its header module (`Town::Iterate`, `Map::SizeY`, `ClampTo`, `CompanyMask`).

   **Bodies behind FFI exports** (steering 33, resolving root's [#735 deferral](https://github.com/fetch-laboratories/openttd-rust/issues/735#issuecomment-6726055806)). While its C++ caller remains, an FFI export keeps its positional base-integer list, so cleanup adds no generated declaration or C++ facade code. The command or helper body behind it moves into a Rust-internal function that takes the original's types and its argument or context struct, and the export becomes a shim that builds them, as #827's waypoint commands do. Rust callers call that typed function, never an `openttd_rust_*` export.

   A typed accessor replaces its raw form. A temporary raw adapter, delegating to the typed accessor, serves only callers in files no pass has finished yet; each adopts the typed form in its own file's pass. The pass converts callers in already-finished files to the typed form itself, as mechanical edits listed in its body. The narrow allowance's `_raw` renames apply only to held files that are not finished. An adapter retires with its last caller, or at #734 closure.

   **Finished-file residue.** Steering 33's audits found raw calls, a kept bounce, helper copies and missing comments in finished files of worksets 01 to 03. Some arrived after the file was finished, through later passes' renames. The audit lists are on [#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6747509029). The residue PR #871 delivered them with #857's assertion fixes at `94679169c9`, after the merged 03 remainder (#839 and #868), with review by function group. A finished file is one with no remaining finding of the [review brief](review-brief.md)'s every-line list.

   Split a workset only at a file boundary, and only when its diff would exceed about 2,000 changed lines. That size is provisional: the 33 vocabulary PRs averaged about 150 lines. Above about 1,000 changed lines, assign one reviewer per function group ([review](workflow.md#review)). Later PRs touch a finished file only for a mechanical shared-file edit or for a caller adopting a changed signature.

   Order (adopted 2026-11-08 from the overhaul thread's proposal, made at the user's request; it replaces closing passes for 01 to 09 and then the largest integer-site count first): pass a workset before its callers. A file passed after its callees adopts their typed forms in its one pass; one passed before them keeps raw calls and is touched again. Fan-in is the number of files outside the workset that reference it through `crate::` (measured at `5af17025a3`).
   - (a) Delivered: the 03 remainder, `station_cmd.rs` (#839) and `station_service.rs` (#868), followed by the finished-file residue #871.
   - (b) 13 Map/terrain (fan-in 90; `map_func` alone 85), with 23's type-bearing files: `bitmath_func` (39), `random_func` (21), `pool`, `math`, `kdtree`, `binaryheap`, `integer` and `container_func`. 23's fan-in of 137 is mostly `game_state`, `debug` and `witness` (45 to 46 each), which gain nothing from types; they and `raw_field` stay in 23's remainder.
   - (c) 09 (59), 08 (58), 07 (52), 06 (45), 04 (38), 05 (27). The proposal put 04 before 06, but the calls between those two worksets run both ways (06 to 04 in 2 files, 04 to 06 in 1), so strict fan-in governs.
   - (d) The rest by fan-in: 16 (67), 21 (62), 15 (42), 18 (37), 10 (33), 11 (28), 14 (27), 17 (26), 12 (25), and 23's remainder. The leaf worksets 22 (17), 20 (6) and 19 (1) go last. F's closure stays last.

   Large files are not left for later. A file over about 1,500 lines gets its own single-file pass, placed by fan-in like everything else: before its workset's other files when they call it, and otherwise immediately after the workset pass that skipped it (root, 2026-11-09, after the planner showed that `station_cmd.rs` calls the station group and that nearly every 09 file calls `movement.rs`). Twenty-four workset files exceed that size; the largest are `station_cmd.rs` (4,868), `orders.rs` (4,841), `town.rs` (3,373), `train.rs` (3,121), `industry.rs` (2,874) and `company.rs` (2,698). A single file cannot split at a file boundary, so a single-file pass above 2,000 changed lines stays one PR with one reviewer per function group, as #819 did. The 03 remainder therefore becomes three passes: `station_cmd.rs`, `station_service.rs` (2,285), and `station.rs`, `station_map.rs`, `station_cmd/waypoint.rs` and `station_lifecycle.rs` together. #734 reports progress in lines finished as well as files.

   A pass's reviewer applies the review brief's every-line list to every line left in its files. Proof follows the [cleanup checklist](review-checklist.md#cleanup-prs). Skip the archive-equivalence attempt for a pass that changes signatures or adds argument structs: it cannot pass, and #801's attempt differed in 15 of 370 object groups.
2. **Tools maintenance.** Delivered: #855's Clippy ratchet (#858, `9b379f0d99`), #856 with #847 and #867 (#869: fork-only CTest name ratchet, instantiation-only orphan exports and the non-failing `--report-uncalled`), and #854's inventory tool (#870, `tools/code-standards-inventory.py`). Each pass reports its files' residue from that tool.

   Fold #891 into the next tools PRs; sanitizer items 1 to 3 merge before #734 closure. Fold later small tools items into the next tools PR. #882 delivered #874's long-function counter, #875/#880/#881 and #736's embedded/extracted-original scope follow-up. The remaining #736 end-state classification still names each retained evidence boundary; new inventory visibility is not C++ deletion.
3. **#857 delivered** in #871: the native assertions dropped in the town, industry and station ports were repaired with the residue pass.
4. **#736 test evidence and #737 boundary debt**, ranked by C++ removed per PR, one PR per batch where the batch fits the review bound above. Fold any item under about 150 counted C++ lines into another item of its batch. Delivered: #736 U (#810), P (#820), the W town, landscape and house families (#829, #833, #840), F cargo (#844) and F vehicle scheduler and company economy (#853); #737 V (#814, #825, #837, #873, #876), O's first PR (#872, 21 chains), and W with nine more O chains (#885). #736 oracle compaction (#877) and scope follow-up (#882) are delivered. Remaining order (root, 2026-11-10, from the planner's rankings on [#737](https://github.com/fetch-laboratories/openttd-rust/issues/737#issuecomment-6767417698) and [#736](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6767452427)):
   - #737 I (company infrastructure counters, one cross-table PR) is dispatched as #888 after W. It must merge before entry 7a opens;
   - return the boundary slot directly to #736's ranked families after I: W session, objectives and signs, then W NewGRF loading and storage, then the others. The merge-20 test/evidence C++ inventory did not fall, so no further #737 batch precedes these ready families. Reserve separate test modules or serialize actual source overlap with map/movement passes. Each family's first checkpoint distinguishes its total evidence footprint from behavior that can move. T follows W and F;
   - #737 batch O's last PR, after the #736 families that free its 18 chains. It turns `--report-uncalled` into the enforced rule, which counts calls from upstream tests;
   - #736's end-state inventory, the stream's last PR.

   Every #736 item follows the oracle rule on #736, and each family's first checkpoint reports the share of its lines that is behaviour and moves. Root re-checks source boundaries and live reservations before commissioning each item.
5. **#738 rules delivered** at `c49bd95076`; the stream is closed at its end state. Keep the rule corpus measured, and consolidate later necessary process changes without reopening completed scope merely to fill a slot.
6. **#734 closure**, after the last workset pass: private wrapper fields, generator support, and removal of the raw adapters and aliases. Seventeen Rust files (2,944 lines; `test_tables.rs` is 1,735 of them) are in no workset. Each joins one before closure, the type files with F ([#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6767381933)). The end state and later stocktakes and steering measure with #854's tool (item 2). #892 rides remaining file passes, then a finished-file residue sweep and closure; no unrecorded production assertion macro site remains at closure, measured as #892 specifies.

   #734 also owns the `unwrap`/`expect` and release-overflow fidelity gaps recorded in #156: remaining file passes handle their selected files, and closure handles recorded residue in already-finished files. Compare each with the original's preconditions and observable failure behaviour; an occurrence alone is not a defect. This ownership ruling does not expand #892's macro scope or add a census, lint rule or gate.

7. **#893 cleanup-exit evidence**, after closure and before the first phase-3 reader dispatch: run the coverage measurement, seeded-fault pilot and all-member deep check. Pause implementers for the two host measurements. Read the report and handle findings under [deep checks](deep-checks.md#policy) before dispatch. Phase-3 planning follows its normal trigger and does not wait for these results.

Corrections adopted on 2026-11-07 under [#746](https://github.com/fetch-laboratories/openttd-rust/issues/746), still in force:

- #734's proof uses the [cleanup checklist](review-checklist.md#cleanup-prs): unchanged operations and order, plus the merge queue's Full when compiled code differs. Per-PR Callgrind profiles and disassembly accounting are not required.
- #747 is integrated through #751/#752, and #855 narrows it further: the Clippy ratchet keeps parameter count and safety documentation only. Remove unnecessary existing cast suppressions on later base updates.
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

## Thirty-third steering review (2026-11-09)

Agent: /root (Claude Code steering review) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host

Root was paused at the user's wind-down. This review covers root's Claude Code session from 11-08 18:41 to 11-09 05:35 UTC: integrations 130 to 146, with #851 and four drafts open, after navigation `78553df0a1`. Fresh audits checked:
- the town passes (#807, #812, #819);
- the industry and station passes (#811, #816, #821, #828, #813, #827, #834);
- #737 V and the TGP fix (#814, #825, #843);
- the #736 transfers;
- process cost;
- steering's inventory rows.

Findings:

- **Delivery is real, and fidelity holds.**
  - No audit found a reachable divergence introduced since steering 32.
  - Passes finished 51 files by 146. Missing original comments fell from 2,827 to 2,693, bare-integer sites from 1,947 to 1,772, and functions over seven parameters from 141 to 132 ([inventories](#cleanup-stream-inventories)).
  - The measurement reproduced integration 129's figures exactly.
- **"Finished" files keep residue, and some regress (debt recurrence).**
  - The analyzer still counts 133 missing comment lines and 121 bare-integer sites in the 51 finished files. The 41 files finished by 143 held 258 and 294 at 129, and 129 and 118 at 143.
  - The audits confirmed calls to raw forms where typed ones exist: `subsidy.rs:162` calls `town::position_raw`, and `station.rs:996` calls `add_*_cargo_raw`, which #832 renamed in after #827 finished the file.
  - #828 kept a Rust-to-C++-to-Rust bounce (`industry.rs:1486`).
  - Helpers are hand-written: RandomTile at `transaction.rs:1046`, `GetBuildCost` at `waypoint.rs:691`, seven `Map::SizeY` copies and nine `Town::Iterate` loops.
  - [#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6747509029) has the lists. These got through four ways:
    - The review brief checked the adapters a pass defines, not calls to raw forms. Its every-line list omitted the checklist's boundary design faults.
    - Nothing asked for code standards on changed lines outside file passes. #814 and #825 dropped the original's comments on rewritten branches and kept a literal `0` for `AM_ORIGINAL`.
    - The narrow allowance's `_raw` renames had no step back to the typed form.
    - Under root's #735 ruling, FFI exports keep raw lists, and Rust code calls them directly. `ChangeTownRating` has no typed Rust form, and the industry and subsidy command bodies sit inside their exports.
- **The Clippy ratchet is the largest process cost and caught no defect.**
  - 64 Quick runs failed in the session, 46 of them on the ratchet: 1.8 per integrated component PR.
  - 8 were new debt reshaped to fit a threshold, and 8 were accepted rows voided by a later head.
  - 4 were a tooling defect: Quick compares the PR head with the moved base tip (`ci-quick.yml:47`).
  - 26 were renames, re-keys or lowered rows left unrecorded. `excessive_nesting` keys hash the block's tokens, so any edit re-keys the row.
  - Lint records took about 16% of implementer messages and 12.5% of reviewer tool calls. At the pause, one module row was all that blocked integration 147.
- **Kept native tests can vanish unnoticed.**
  - #829 and #833 removed 14 kept C++ cases from the build at integrations 142 and 144; #849 (in #851) guards that route.
  - Full checks only names the reference build has (`evidence_replacements.py:181`), so a fork-only case can be deleted or renamed with no record.
  - Otherwise the #736 transfers carried their observations. #810 lost two (short and empty view-at-cursor reads, and two sanitizer runs), and #853 loses three.
- **#736's oracle rule slipped.**
  - #844's income oracle is 1,981 bytes per retired line against the 1,500 budget, and the PR doesn't report the miss.
  - The 127–129 oracles (2.07 MB) were never compacted, because "the next PR that touches those tests" never matched.
- **#737 left an orphan chain and copies.**
  - `VehicleCargoList::Reassign` and its export survive with no caller. Explicit instantiations hide it from the orphan scan.
  - #825 shares station visit-type, facilities and depot-penalty readers but keeps the aircraft, ship and road copies.
- **The roadmap became a log.**
  - It grew from 339 to 590 lines in 23 roadmap commits.
  - The ready queue held 12 entries in 138 lines against the user's "two or three".
  - Root recorded dispatches and integrations in it.
- **Delegation moved root's work but not the rate.**
  - There were 9 integrations before midnight and 8 after.
  - Agent messages rose from 9% to 36% of root's tool calls, and 51 notifications needed no action.
  - Slot occupancy rose with the cap (2 to 4), not with delegation.
  - Profile 143 ran beside four implementers, over the cap.
- **Assert-build divergences predate this period** and remain (#857). The Rust `CmdIndustrySetProduction` path skips `UsesOriginalEconomy`'s assertion. `road_stop.rs` uses `debug_assert!` for nine native assertions. A crafted save's out-of-range town layout grows a grid where the original reaches `NOT_REACHED`.

Actions:

1. **Finished means finished.**
   - [Work order](#cleanup-work-order) item 1:
     - passes add missing shared ports;
     - command and helper bodies move out of FFI exports into typed internal functions, and Rust callers stop calling exports. This resolves root's #735 deferral and adds no C++;
     - a pass converts finished files' calls to the callees it types, and `_raw` adapters serve only unfinished files.
   - The [review brief](review-brief.md) adds those findings, the boundary design faults, and code standards on every changed line of every PR. The [implementer brief](implementer-brief.md) carries the code rules.
   - One residue PR fixes the audit lists for worksets 01 to 03, right after the 03 remainder.
2. **#855 corrects the ratchet** (user, 11-09: a wrong decision gets corrected, not deferred). It drops function length, nesting and cognitive complexity, which measure the original's own structure, and with them the 11-06 approval of those families. It keeps parameter count and safety documentation, fails only on growth against the merge base, and pairs unambiguous renames itself. Then #856 (fork-only CTest names and instantiation-only orphans), #847 and #854. At stocktake 155, root reports ratchet failures per integrated component PR beside today's 1.8. Below 0.5 is a provisional target for steering 34; a miss gets a second tools round from the failure classes, not a deferral.
3. **#857**, one fidelity PR, runs next in work order item 3.
4. **#736 and #737 follow-ups.**
   - #853 restores its three observations before integration ([PR comment](https://github.com/fetch-laboratories/openttd-rust/pull/853#issuecomment-6747520518)).
   - #851 reports #844's oracle miss in its body; nothing else blocks it ([PR comment](https://github.com/fetch-laboratories/openttd-rust/pull/851#issuecomment-6747514237)).
   - Oracle compaction becomes an explicit item 4 entry.
   - The #737 follow-ups go to V part 2 or 3 ([#737](https://github.com/fetch-laboratories/openttd-rust/issues/737#issuecomment-6747525414)).
   - New coverage gaps are on [#156](https://github.com/fetch-laboratories/openttd-rust/issues/156#issuecomment-6747532937).
5. **Bound the roadmap.**
   - The [ready queue](#ready-queue) holds at most three undispatched entries. A dispatched entry leaves the queue, and its state lives on its PR.
   - The planner's analysis of later entries lives on the stream issue; the earlier one moved to [#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6747537417).
   - Root commits the roadmap only at stocktakes, course changes and wind-down; the planner's queue refresh is the one exception ([team](team.md#ready-queue-and-pipelining)).
6. **One user decision is corrected.** The 11-06 approval to keep the length, nesting and complexity Clippy families is reversed by #855, on the measured cost above. The delegation model, the four-implementer cap, report-only reviewers and the twelfth-integration stocktake stand. The user extended the delegation model to Codex on 2026-11-10; both providers use report-only reviewers, implementer fixes and the same ready-queue and pipelining rules ([team](team.md#delegation-user-2026-11-08)).

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

Steering 32 (2026-11-07), still in force:
- cleanup work is ranked and reported by stream inventory;
- #734/#735 passes finish their files, with two implementer slots;
- #806's declared carries replace rename, fidelity and renewal contracts;
- #736 oracles store each distinct output once, under #736's budget;
- review reports and integration notes carry findings, fixing commits and the disposition, and a base update records only its applicable condition.

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
| 132–146 | #734/#735 file passes: workset 01 (#807, #812, #819), 02 (#811, #816, #821, #828), 03 (#813, #827, #834) and 13/23 part a (#832). #736 U #810, P #820, W #829/#833/#840. #737 V #814. Maintenance #809 (#726), #808 (#806), #826, #818, #831, #842, #838 | 51 of 319 workset files finished, with residue listed on #734. Fork C++ −1,915 from 129 to 146 (Phases); exports 1,552 → 1,539. No rollback. |
| — | #858 Clippy ratchet for #855 (`9b379f0d99`) | Clippy debt rows 649 → 59: length, nesting and complexity retired; growth-only against the merge base. |
| 147 | #851: #849 coverage fix, #846 #835 timing tests, #825 #737 V part 1, #844 #736 F cargo, #843, #850 (`489f4ea161`) | Restores the 14 kept CTest cases that integrations 142 and 144 dropped (coverage regression; #849 guards the route). #844's income oracle misses #736's budget: 1,981 bytes per counted line against 1,500, left for oracle compaction. |
| — | #859 checker-only Full and unchanged growth acceptances (`ba7bc3fd44`) | A PR changing only Quick's checker scripts, their debt files and inert Markdown runs Quick alone for Full (about 8 minutes, not 22). Growth acceptances hold while the added source is unchanged. Inert Markdown narrowed to `AGENTS.md` and `docs/**.md` minus CMake-packaged docs, for carry and post-merge skips too. |
| Merges 1–8 (queue, 11-10) | #866 preflight parity (`048109075c`); #734 passes #852 `saveload.rs`, #839 `station_cmd.rs` and #868 `station_service.rs` (`c12561671f`); #737 V part 2 #837; #736 F vehicle scheduler and company economy #853; tools #869 (#856, #847, #867) and #870 (#854) | 54 of 319 workset files finished, worksets 01 to 03 complete. Fork C++ census 277,044 → 276,186 from 146; historical exports 1,539 → 1,536; callback slots 3,332 → 3,298. No rollback; #853 left the queue twice before merging. |
| Merges 9–20 (queue, 11-10) | #737 O/V/W (#872, #873, #876, #885); residue/assertions #871; file passes #878, #879, #883, #884, #886; oracle compaction #877; tools/scope #882 | 92 of 319 entries finished. Same-version fork C++ −432; shared/rebound/orphan baselines 87/279/30 → 48/245/27; thirty uncalled chains removed. #738 docs item delivered separately. #885's Clang queue finding was fixed before its successful retry. |

## Recurring

- Merges, [one merged PR each](root-workflow.md#integration), count from the switch to the merge queue after integration 147.
- After every twelfth merge (user, 2026-11-08), take stock under `/start-development` step 4 and profile the merged tip in the background ([speed](speed.md#instruction-projections)). A running profile counts as one of the two implementers. Merge-20 stocktake/profile are complete; the next follow merge 32.
- Run the platform cadence after every fifth merge; the next follows merge 23. Merge-18 cadence `42915509900` passed. A new failure stops merges ([integration](root-workflow.md#integration)). Full runs Windows x86, Clang and GCC Dedicated itself (#860), so the cadence adds macOS and the platform comparison. Cadences 135 (Windows x86 oracle tests, fixed by #818) and 140 (`-Wmissing-field-initializers`, fixed by #831) each stopped integrations for about 75 minutes.
- All-member deep checks are requested at cleanup exit and phase-3 exit; other deep runs remain on request ([policy](deep-checks.md#policy)).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions and reports #892's remaining production macro counts beside the #734 inventory.
- #156 (coverage gaps), #275 (speed), #724 and #710 (host) stay open trackers. #500 is delivered by #734; #168 and #199 attach to #737 and the owners that touch those records. #693 retains the cancelled reconnect failure and two completed original/original runs that did not reproduce it; the cause remains unresolved and that two-run authorization is exhausted. Its loopback hang cost two Full reruns in root's 11-08/09 session (integrations 141 and 145); the implementer queues the PR again once ([integration](root-workflow.md#integration)). #736 P (#820) delivered without touching it, so diagnosis returns with #730's network owner after cleanup, or sooner if hangs exceed one rerun in ten Full requests at a stocktake (provisional; 2 in 26 measured). #503's coverage continuation runs last.

Excluded, with issue placement and return conditions rechecked at merge 20 (2026-11-11); near-full reproduction and fork release-source publication triggers are not met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Ready queue

At most three undispatched work orders ([delegation](team.md#ready-queue-and-pipelining)). Refreshed at `6fc785a80d8` (merge 22): [7a](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6785467943) and [W-s](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6785468251) are dispatched and leave Ready. Keep two implementers and at most six unmerged component branches; earlier review fixes take priority. The two entries below unlock through that dispatched work, on their respective slots; [ranking and source fences](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6785546279).

Agent: /root/cleanup_ready_queue_refresh_22 | Model: gpt-6-astra | Reasoning effort: high

1. **9m, #734/#735 movement file passes, after dispatched 7a merges.**
   - **Scope and size:** complete `movement.rs`, `movement/tests.rs` and `signal_func.rs`: 1,836 raw lines, with the prior provisional 748–1,348 changed-line bound to refine at the first checkpoint. This shared callee precedes the rest of workset 09 and its movement callers; leaf network passes do not displace it once released. Include #892's assertion class in the existing every-line pass.
   - **Reservation and holds:** required typed conversions among 68 external referencing files; 47 of those also directly reference the two map modules. Wait for 7a's merge rather than overlap its broad caller rewrite. #889's actual 13-file diff is disjoint from this direct caller set, but its wider reservation holds `network_client.rs`, `network_connection.rs` and `network_frame.rs` until merge or explicit release. I's former holds are released.
   - **Shared hooks:** W-s owns `signs_cmd.rs`'s first test-module hook; serialize and explicitly release that parent file before 9m converts its two `movement::pixel_z` calls. Separate test bodies do not release parent edits. Coordinate any required `newgrf_text.rs` / `newgrf_townname.rs` caller edits with the NewGRF evidence entry below before either edits their parent hooks. No dependence on an undispatched full-file pass is implied.
2. **W NewGRF loading and storage, #736 evidence, after dispatched W-s on the evidence slot.**
   - **Scope and size:** the four-file 836 gross counted-line footprint and corrected 400–490 potentially moving estimate remain provisional ([checkpoint correction](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6771502984)). Classify actual transferred behavior and retained observations at the first checkpoint; no new deletion credit follows from these sizes. No further #737 batch precedes this family.
   - **Reservation:** `tools/simulation/{newgrf-loading-console.cpp,storage-console.inc,newgrf_loading.py,towns.py}`, the affected include in `towns-console.cpp`, `src/tests/{newgrf_loading.cpp,newgrf_bytereader.cpp}` and required registration/replacement records. Use separate Rust test bodies under `newgrf_loading`, `newgrf_storage` and `newgrf_act2`; extend to `newgrf_text`, `newgrf_townname` and `snow_line` only for their recorded observations, reserving the minimal parent hooks. The core destinations are outside 7a's and 9m's direct caller sets; the text/name parents are 9m callers and need explicit serialization. Shared registration and `test_tables.rs` edits follow W-s's release; preserve #889-held network interfaces.
   - **Retained boundaries:** keep native ByteReader/badge failures, native ABI and alias identity until their named owners retire them. Preserve storage-console consumers in `towns.py`/`towns-console.cpp`, PSAC load and native resolver observations; determine the `Inspect`/SpriteGroup boundary before claiming a moving share. A Rust substitute alone does not replace native identity or exception evidence.

**Active source fence:** W-s owns the first minimal `objectives.rs` hook, then releases the file directly to 7a for mechanical map caller edits; additional parent edits need renewed coordination. I has released test support. #889's bounded `container_func.rs` include helper, `crypto.rs` derive deletion and `string_type.rs` alias removal confer no additional full-file selection.

**Later selection:** the rest of workset 09 follows 9m; the twelve remaining network files stay #734/#735 workset 20, returning after 20t under callee-first order and released map/movement priority. The ranked #736 families follow NewGRF; T waits for W/F, O's final 18 chains wait for their evidence families, and end-state classification stays last. No third entry is promoted through an undispatched predecessor.

**New issue placement (user-directed evidence review, 2026-11-11):** #890 belongs to a docs agent outside the implementer count now, before the phase-3 planner; #891 belongs to work-order item 2's next tools PRs, its sanitizer items before #734 closure; #892 rides file passes and closure with a final finished-file residue sweep, without its own slot; #893 belongs after closure and before the first phase-3 dispatch, with exit coverage, the fault pilot and the standing deep-run request. None preempts cleanup. Root must synchronize the broader work order, Recurring and deep-check guidance; this Ready-only refresh dispatches none of them.

Full phase-3 planning remains untriggered: many file passes, F/closure, evidence families and O's last batch remain outside dispatched work and Ready. Root still owns the next platform cadence at merge 23 and stocktake at merge 32.

## Resume checkpoint

Agent: /root | Model: claude-fable-5-2 | Reasoning effort: xhigh

**Paused at the user's request (wind-down, 2026-11-10 17:14 UTC).** No implementer, reviewer or planner runs; both implementers' tasks were cancelled after their checkpoints were pushed. Code main is `03613dd0ac` (#879, merge 16), and the docs tip is this commit. No rollback was performed, and the merge queue is empty. Base `03613dd0ac` is green: Quick, Migration baseline and CI - Build (run 42861359447) passed. The platform cadence after merge 13 is green (run 42849903006; its macOS Debug job passed on a rerun with `CI_MACOS_HEAVY`, which is deleted again). `CI_ON_DEMAND` and `CI_MERGE_QUEUE` are `true`. One CI run was pending at the pause: #884's Quick, run 42863173744, started by the wind-down push. On resume the [delegation](team.md#delegation-user-2026-11-08) model applies as before, with models and effort from [team](team.md#roles-and-models).

| Open work | Head | State, CI and jobs at pause | Next action on resume |
| --- | --- | --- | --- |
| #883, entry 15b: `rail.rs`, `articulated.rs`, `build.rs`, `clone.rs` and `construct.rs` | `03bab12ce5`, draft, pushed | About 950 changed lines, built on #879's branch. Local verify and preflight (14 of 14) passed at this head, and Quick is green. Its body is complete. No `native` request and no review. It predates #879's merge, so it needs a base update. No job runs. | Merge base and compile both `cargo check` variants, push, mark ready. Then root assigns its reviewers and the implementer requests `native` once. |
| #884, entry 23r: the workset 23 remainder, twelve files | `82216cceb8`, draft, pushed at the wind-down | Its verify (`buhbzylfe`) passed at this head at 17:16 UTC (report `20261110T170401.303151Z`). Quick at this head was still running at the pause (run 42863173744), its result unknown. No preflight at this head, no full body, no `native` request and no review. No local job runs. | Read Quick's result. Preflight, write the body, request `native`, mark ready; one reviewer. |
| #737 W with batch O's nine chains, branch `cleanup/737-w-world-stations`, no PR | `8925627431763fe0728315518fafc89b77fc3131`, pushed | Town (`749e337f01`), industry (`2f5bb939e8`) and station (`b895493f54`) groups are built and tested. The session group (`8925627431`) is a checkpoint: its build, `b91ydyskv`, ended with the implementer's task at 17:16 UTC after 631 seconds with no result, and the shared-accessor scanner ran in the same command, so its result is unseen too. 26 of W's 31 shared-accessor deletions are done, with 16 catalog addresses added. Not started: the 14 forwarded rows, the three scanner rows, O's nine chains and the PR. | Run the session group's build and both scanners. Merge base before the PR and compile both variants: #879 changed `station_cmd.cpp` and `station_service.rs`. Then the forwarded rows, the scanner rows and O's nine chains, then open the PR; its reviewer rules on about 20 catalog addresses as growth. |

W's reservation and re-read manifest are on [#737](https://github.com/fetch-laboratories/openttd-rust/issues/737#issuecomment-6772646463). Root's ruling on W's five session rows that are not reads: they stay, each with a named owner in the PR. `publish_settings` goes with settings storage (#581), `company_settings` and `company_begin` with the company record in phase 2 (#716), and `reset_game_config` and `reset_ai_config` with the script reader owner in phase 3 (#669).

| Other work | State | Next action |
| --- | --- | --- |
| Ready-queue entries 22, 7a and 9m | Undispatched | As the [ready queue](#ready-queue) records. Entry 22 follows #884 on the file-pass slot. Entry 7a opens on root's word, after #737's W and I PRs merge. |
| #737 batch I, one cross-table PR | Not started | After W merges, on the batch slot. Batch O's last PR waits on the #736 families that free its 18 chains. |
| #736 W session, objectives and signs | Not started; its files are free | After W's PR is in review. W NewGRF loading and storage follows; it is reserved on #736 with its [first checkpoint](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6771502984), and its branch holds no commit. |
| Platform cadence | Green after merge 13 | Due after merge 18. macOS was cancelled for hosted capacity twice on 11-10; apply the `CI_MACOS_HEAVY` step again on the next cancellation ([CI administration](root-workflow.md#ci-administration)). |
| Stocktake and profile | Last at merge 8 | Due after merge 20. Report #872's 343-line glue-to-original reclassification, and refresh work order items 2 and 5, which #882 and #738's closure delivered. |

Neither the cadence nor the stocktake gates the resume.

Flakes and limits carried forward:

- #693: 2 network-timing failures in 42 runs of the simulation tier since the queue switch (a loopback hang on #839 and a `content-protocol` evidence miss in the reference role on #877), below the one-in-ten threshold. The two-run original/original authorization remains exhausted, and no repair, retry or new diagnostic is selected. Keep `.local/retained-network-693` with the failed Full artifact, inconclusive diagnostics, passing traces and bounded original/original evidence.
- A flaky shard can be rerun with `gh run rerun RUN --failed` on a branch that contains `db62aad692` (#881); a branch on older tools needs a fresh `native` request.
- A PR that changes `.github/workflows/` needs root's bootstrap Full before it queues ([CI administration](root-workflow.md#ci-administration)).
- Metric scope completeness stays with #736's end-state inventory. No later phase is selected.
- Pass briefs add `cargo check -p openttd-kernels --features native-test-support --all-targets --locked`: `rust-checks` does not compile that feature, and #807's first Full failed on such a caller. A root acceptance comment uses the plain `Agent: /root | Model: ... | Reasoning effort: ...` form, which the lint gates parse. #710 records six missed Codex reviewer resumes on 11-08; root still checks a reviewer's named job when its turn ends on one.

Retained state for housekeeping:

- Keep the two implementer checkouts: `t3-8bdcce4f` (#883 and #884; its `.local/verification` holds #884's passing verify) and `t3-43581c15` (W; `.local/logs/w-manifest.md` is not in Git). Both are clean and pushed.
- Keep `t3-ee66f61c`, locked: profile 155's outputs (merge 8, `c12561671f`) for #275, until the next profile is recorded.
- `tools/housekeeping.py` found nothing removable at the wind-down. It kept three merged, clean checkouts only for a task active within 12 hours or an earlier roadmap reference: the #738 docs agent's, the planner's and #850's. This checkpoint does not name them, so a later run may remove them.
- Keep the planning checkout `t3-20fec9ce` with its private plan, and origin `preserve/housekeeping-foundation-6ab9c20e5d`. Preserve every checkout with uncommitted or unpushed work.
- Keep `t3-8ed2642d` (profile 96), `t3-62945724` (98) and `t3-a6181dc7` (100) with their comparisons and frozen runtimes. Keep `.local/worktrees/root-profile-125`, `.local/worktrees/root-profile-119` and `.local/worktrees/root-profile-113` with accepted reports, raw Callgrind and frozen runtimes. Keep `t3-a490295e` (profile 131, clean `305ac20c28`) and `t3-d3ff7564` (profile 143, clean `da020b52c7`) with their HDD `.local` outputs. Profile 119 measures clean `4d3a1d1d41`, not integration 120.
- Keep main `.local/integration-records/`, `.local/speed-pass/` anchors, frozen runtimes and the retained simulation evidence.
- Keep `t3-f4cf7a0b` (one unique unpushed commit, `125ea2440a`). Leave the staged 11-08 trees `t3-03c1e2da`, `t3-1ddaadfd`, `t3-5575ebe4`, `t3-83082b90`, `t3-c68d914a` and `t3-e0361d8c` untouched.
- Leave dirty or staged trees untouched: `t3-3b491ddb` (five files), locked #565 WIP `t3-431c84b4`, locked native evidence `t3-d78d5f8b`, `t3-32cf4edc` (`tools/simulation/roads.py` staged) and `t3-e130d5a3` (`migration/saves/README.md` staged).
- Keep protected `.local/worktrees/root-docs-72` and locked `.local/worktrees/profile-after*`. Never push stale root-docs guidance over current main. Keep the pinned original and main's uncommitted user-owned `.codex/config.toml` unchanged.
