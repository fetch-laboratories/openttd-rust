# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-10, stocktake after the eighth merge)

Agent: /root/stocktake_merge_8 | Model: claude-sonnet-5-5 | Reasoning effort: high

**At the wind-down (root, 2026-11-10 17:14 UTC).** Code main is `03613dd0ac` (#879), the sixteenth merge through the merge queue: 16 since development resumed on 11-10, counting #866 at 08:32 UTC, and 8 since this stocktake (#872, #873, #871, #876, #878, #877, #882, #879). No queue entry was removed after the stocktake, and no merge was rolled back. Open: #883 and #884, both drafts, and #737's W branch without a PR ([resume checkpoint](#resume-checkpoint)). Measured at `db62aad692` (merge 15), with nothing measured at the wind-down: the fork C++ census is 275,958 (stocktake: 276,186), with 1,517 exports (1,536) and 3,269 callback slots (3,298); C++ embedded in tool files, outside the census, is 836 code lines in Python strings, 158 in oracle recipes and 21 original-body extraction sites (#882). Since the stocktake: #738 is at its end state and closed (corpus 3,228 lines); #737's batch V is delivered (#873, #876) with batch O's first PR (#872); #736's oracle compaction (#877: four oracles from 2,641,909 to 290,751 raw bytes) and scope follow-up (#882) are delivered; #734 gained the 01 to 03 residue (#871, with #857), entry 6b-1 (#878) and entry 15a (#879); the tools items #874, #875, #880 and #881 merged in #882. The figures below this paragraph are the stocktake's, at `c12561671f`.

**Since stocktake 143 (`da020b52c7`, 11-09 02:31 UTC).** Code main is `c12561671f` (#868), 11-10 11:35 UTC: the eighth merge through the merge queue (#866, #852, #837, #839, #869, #870, #853, #868), integration 155 in the old count. Open: #871 (draft), #872 and #873. The fork C++ census is 276,186, down 1,380 in 1.378 calendar days (−1,001 a day, commit timestamps; the 11-09 05:35 to 11-10 08:35 pause is inside it). The tests fell 828 lines (`src/tests` 5,615 to 4,787), `tools/` 384 (15,821 to 15,437) and ABI-generated non-record C++ 33. Exports are 1,697 expanded and 1,536 historical ([Phases](#phases) counts 1,539 at integration 146). Original C++ retired fell by 2 (−1.5 a day), in Simulation logic (38,329 to 38,327; remaining 9,095 to 9,097). Merged PRs that touched original-derived files since 143: #845 (`newgrf_station.cpp`), #851 (`effectvehicle.cpp`, `order_cmd.cpp`, `train_cmd.cpp`), #839 and #868 (`station_cmd.cpp`) and #869 (`cargopacket.h`). The figure is the metric's, reported as measured; no line was traced.

**Speed.** Profile 155 passed, with every canonical exit-save comparison equal and all six policy records below band and ceiling. TGP is 1.163070x (143: 1.230318x), as #843 predicted ([Speed](#speed)). No projection rose.

**Streams.** #734 finished 54 of 319 workset files (143: 41) and 46,082 of 183,789 lines (25.1%); worksets 01, 02 and 03 are finished (14, 15 and 14 files). #736 fell 828 lines in `src/tests` and 384 under `tools/`; #737's three baselines fell 17, 25 and 9 rows. #738's corpus rose 22 lines (3,229 to 3,251) with its item ready. #735's `allow` sites are unchanged at 11. The Clippy debt file is 54 keys because #858 stopped gating length, nesting and complexity rows: the same gate gives 59 keys at 143.

**Process since development resumed (11-10 08:35 UTC).**
- **Merged PRs.** 7 (#852, #837, #839, #869, #870, #853, #868). #866 merged at 08:32 UTC, before the resume. Created to merged: 28.2, 32.0, 31.6, 0.7, 0.8, 30.1 and 2.3 hours (median 28.2); the first, second, third and sixth were opened 11-09 and include the 27-hour pause. Accepted to merged, from the last review report on the merged head: 10, 22, 17, 18, 11, 15 and 16 minutes (median 16).
- **Merge queue.** 9 entries since 08:35 (10 with #866's): 7 merged and 2 removed, both #853. Of the 11 group runs, 7 passed and 4 failed:
  - 09:53 #853's own run: Windows x86 Clippy `unnecessary_box_returns` at `vehicle/scheduler.rs:998` (`new_pool() -> Box<PoolCore>`); Quick, native and the other platforms passed (run 42794535339). The lint depends on pointer width, and Windows x86 runs only in Full. Removed at 10:05.
  - 09:59 #837's and 10:04 #839's runs, both built on top of #853: E0560, `transactions::Services` has no field named `orders` (runs 42795338273 and 42795978873). #837 retired a slot that #853's test tables still set, which Git does not see as a conflict. The queue rebuilt both without #853 at 10:05, and both merged.
  - 10:33 #853's second run: the same E0560 once #837 was on base (run 42800171322). Its `platform` pre-check had passed at 10:14, before #837 merged. Removed at 10:52, 18 minutes after an error known within 2. #853's `9220c1e356` drops the stale initializer; it merged on its third entry.
- **Quick failures on PR heads.** 25 runs on the seven merged PRs since 08:35: 19 passed, 4 failed, 2 were cancelled. None of the four failed on the Clippy ratchet step: #839 and #868 failed Python complexity debt and lint policy, #869 Python formatting and lint, #853 Rust formatting, check and warnings. Over the five component PRs' whole lives (#852, #837, #839, #853, #868; 46 Quick runs on the seven PRs since 11-08 18:58): 14 failures, 10 on the Clippy ratchet step, all 11-09 02:20 to 05:35 UTC on #837 (2) and #839 (8), before #858 (11-09 20:52 UTC): 2.0 per component PR (steering 33: 1.8; provisional target below 0.5), 0 since #858.
- **Simulation-tier runs.** 30: 19 native requests (15 passed, 3 cancelled, 1 failed before the simulation shards) and 11 queue runs (7 passed, 4 failed as above, none in simulation). One ended on the network-protocol hang (#693): native request 42788129285 for #839 (09:08): shard 2 printed nothing after 09:20:09, was cancelled at 09:36:13, and three `openttd-network_protocol` processes were terminated at cleanup. No timeout message was logged, so the attribution is by signature. One in 30 (provisional threshold one in ten; 11-08/09 measured 2 in 26). The other two cancellations were superseded runs whose simulation shards stopped after 2 min 17 s and 37 s.
- **Branches.** 3 open PRs; 48 remote branches not merged into `rust-migration`: 21 upstream (`release/*`, `master`), 3 open-PR heads and 24 others. `issues.py`: 43 open, 0 unplaced, 1 excluded, 1 roadmap exclusion without an issue.

**Root's reading (root, 2026-11-10).** No course change: fork C++ fell, no scenario is near its ceiling and no item stalled across review rounds.
- **#738** is the one stream whose inventory rose with ready work: its last item was undispatched, and the 11-10 process rules added lines. Root dispatches it at this stocktake ([work order](#cleanup-work-order) item 5).
- **#735**'s `allow` sites held at 11 while its ratchet rows fell from 53 to 48. Its "functions over seven parameters" row has no tracked counter; #874 adds one to the inventory tool, folded into the next tools PR.
- **Clippy ratchet:** 0 failures in 25 Quick runs since #858, against steering 33's provisional target below 0.5 per component PR. The target is met; steering 34 confirms it.
- **#693:** 1 hang in 30 simulation-tier runs, below the one-in-ten threshold. No diagnosis is selected.
- **Metric correction to expect:** #872 deletes a wrapper in `src/station_cmd.cpp`, after which the metric lines 343 lines of that file up with the original again and counts them as original, not glue. "Original C++ retired" will read 343 lower at the next stocktake. That is a reclassification; the fork C++ census is unaffected.
- **Queue:** the two removals and the two process faults found on the first day (steered messages cancelling a running call, reviews held for CI) are on [#860](https://github.com/fetch-laboratories/openttd-rust/issues/860), with their fixes in `0ef341e341` and `1227a1becc`.

| Measured quantity | Integration 143 | Integration 155 (tip) |
| --- | ---: | ---: |
| Original C++ remaining | 221,654 | 221,656 |
| Fork source glue, including generated records | 34,616 | 33,651 |
| Compiled source and generated records C++ | 256,270 | 255,307 |
| Tracked tools C++ evidence | 15,821 | 15,437 |
| ABI-generated non-record C++ evidence | 5,475 | 5,442 |
| Measured fork C++ series (scope below) | 277,566 | 276,186 |
| Original C++ retired | 62,701 of 284,355 (22.05%) | 62,699 of 284,355 (22.05%) |
| Simulation C++ retired | 41,541 of 61,148 (67.94%) | 41,539 of 61,148 (67.93%) |
| Expanded Rust exports / callback fields | 1,708 / 3,374 | 1,697 / 3,340 |
| Historical regex exports / callback slots | 1,552 / 3,332 | 1,536 / 3,298 |
| Shared-accessor / rebound-callee / orphan-export baseline rows | 104 / 304 / 39 | 87 / 279 / 30 |
| Exports held only by uncalled members (`orphan-exports.py --report-uncalled`, not enforced) | – | 53 |
| Retained Clippy keys / diagnostics | 649 / 739 (59 / 88 under #858's gate) | 54 / 83 |
| Ruff keys / diagnostics | 287 / 287 | 287 / 287 |
| Raw GLOBAL Rust / tooling inventories (scope below) | 190,613 / 62,128 | 203,275 / 64,412 |
| Play-save speed ratio | No wall pass. Projections are 1.345x to 1.445x (P2000 highest), unchanged at the anchor floors | No wall pass. Play projections 1.345x to 1.445x (P2000 highest), unchanged at the anchor floors |

Fork-added C++ under `src/tests` (4,787 lines) is within source glue. The 6,064 ABI-generated lines are 622 record declarations within source glue plus the 5,442 evidence lines above. The series does not yet inventory extracted-original comparison fragments or C++ embedded in Python strings; that scope follow-up belongs to #736 and earns no deletion credit. Raw GLOBAL Rust grew 12,662 lines and tooling 2,284 since 143; the split between executable code and oracle data was not measured.

| Source area / phase relationship | Original | Retired | Remaining | Fork glue |
| --- | ---: | ---: | ---: | ---: |
| Simulation logic (phase 1) | 47,424 | 38,327 | 9,097 | 6,427 |
| Simulation storage (phase 2) | 13,724 | 3,212 | 10,512 | 1,386 |
| NewGRF, save/load, script API and network (phase 3) | 51,794 | 13,067 | 38,727 | 3,360 |
| Support, graphics and viewport (phase 4) | 32,602 | 980 | 31,622 | 551 |
| GUI and platform (phase 5) | 64,120 | 415 | 63,705 | 465 |
| Vendored (phase 6) | 55,148 | 2,317 | 52,831 | 0 |
| Data tables | 17,295 | 4,367 | 12,928 | 14 |
| Upstream tests and fork test glue | 2,248 | 14 | 2,234 | 4,787 |
| Boundary layer | 0 | 0 | 0 | 16,661 |

These are source-area subtotals, not ownership or phase-exit claims. Null-driver games, bounded NewGRF and historical-save corpora and retained native fixtures leave the limits recorded on #156; entry witnesses are not branch coverage. #503 deep checks remain user-requested. #156 gained one comment since 143, from steering 33's audits (none a demonstrated defect): #810 dropped the short-read view-at-cursor observation and the byte-strings and history sanitizer candidate runs; #825's `wait_twoway` and stuck-train news path have no witness; #814's `deliver_to_transfer` and `deliver_to_keep` are not distinguished, and station auto-refit and first-arrival visit-type bits have no witness; integrations 142 to 146 ran without 14 fork-only kept native cases, restored by #849 in #851.

### Cleanup stream inventories

Rows marked † are root's, updated at each stocktake ([selection](planning.md#selection)). The others are steering's. The comment, integer, wrapper, constant and literal rows now come from `tools/code-standards-inventory.py` (#854, #870); its 143 values were recounted at `da020b52c7` with the same tool, and at integration 129 it reproduced the overhaul's published figures exactly. Finished-file figures count the files in the † row above them.

| Stream | Inventory | Overhaul | Integration 129 | Integration 143 † | Integration 146 (steering 33) | Integration 155 † |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| #734 | Workset files with a finished pass † | 0 of 319 | 0 of 319 | 41 of 319 | 51 of 319, with #832's ten | 54 of 319 (#832's ten, #839, #852, #868) |
| #734 | Workset lines in finished files † | – | – | 32,080 of 176,930 (18.1%) | – | 46,082 of 183,789 (25.1%) |
| #734 | Explanatory comment lines missing inside matched ported bodies | 3,139 | 2,827 | 2,715; 129 in finished files | 2,693; 133 in finished files | 2,577; 152 in finished files |
| #734 | Bare-integer parameters and fields whose concept has a named type | 1,957 | 1,947 | 1,766; 118 in finished files | 1,772; 121 in finished files, many of them FFI or `repr(C)` boundaries that #355 allows | 1,695; 150 in finished files |
| #734 | Transparent wrapper types / named constants | 69 / 2,181 | 266 / 3,411 | 285 / 3,466; 37 / 332 in finished files | 287 / 3,482 | 294 / 3,538; 47 / 435 in finished files |
| #734 | Literal sites standing for a named value | – | 14 | 14; 0 in finished files | – | 12; 0 in finished files |
| #735 | Functions over seven parameters | 150 | 141 | – | 132 (70 FFI exports, 39 helpers, 23 commands) | – (no tracked counter; #874) |
| #735 | `too_many_arguments` ratchet rows / `allow` sites † | 71 / – | 63 / 19 | 53 / 11 | – | 48 / 11 |
| #736 | Fork C++ evidence under `tools/` / `src/tests`, code lines † | – | 16,869 / 5,684 | 15,821 / 5,615 | 15,642 / 5,279 ([Phases](#phases)) | 15,437 / 4,787 |
| #737 | Shared-accessor / rebound-callee / orphan-export rows † | 117 / 314 / 42 | 117 / 314 / 40 | 104 / 304 / 39 | – | 87 / 279 / 30 |
| #737 | Exports held only by uncalled members † | – | – | – | – | 53 |
| #738 | Rule corpus lines † | 3,312 | 3,362 | 3,229 | 3,231 before this review | 3,251 |
| None | Ruff rows | 296 | unchanged | 287 | – | 287 |

Counting commands:
- **#734:** from its issue's Worksets table and file lists. Lines are `wc -l` of each workset file under `rust/openttd-kernels/src` at the measured head. The issue's own "Finished" line is stale (33 files at integration 138): recount finished files from the pass PRs against the lists. At 155 they are workset 01 (14), 02 (15) and 03 (14), `tile_cmd.rs` and `tile_owner.rs` (13), `saveload.rs` (21), and `binaryheap.rs`, `bitmath_func.rs`, `container_func.rs`, `integer.rs`, `kdtree.rs`, `math.rs`, `pool.rs` and `random_func.rs` (23). The comment, integer, wrapper, constant and literal rows: `python3 tools/code-standards-inventory.py [--head REV]` for the tree, and `python3 tools/code-standards-inventory.py [--head REV] RUST_FILE...` with the finished files' paths for the finished-file figures.
- **#735:** from `migration/clippy-debt.json` (`too_many_arguments` keys) and `git grep 'allow(clippy::too_many_arguments' -- rust/`.
- **#736, #737 and the Clippy and Ruff rows:** from `python3 tools/port-metrics.py --whole-fork`, `python3 tools/orphan-exports.py --report-uncalled`, `wc -l` of the three baseline files, and the debt files (keys are `sites` entries, diagnostics the sum of their list lengths). Expanded exports: `growth_inventory` in `tools/port-metrics.py`.
- **#738:** `cat AGENTS.md $(git ls-files 'docs/*.md' 'docs/**/*.md' | grep -v -E '^docs/(roadmap.md|components/)') | wc -l`.

### Speed

Profile 155 on clean `c12561671f` ([#275](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6767370907)) used `simulate --instructions --jobs 2`; every anchor identity was accepted. All seven nonempty canonical exit-save comparisons pass, and all six policy records are below band and ceiling.

| Scenario | Original Ir | Candidate Ir (155) | Projection (155) | Projection (143) | Ir change since 143 |
| --- | ---: | ---: | ---: | ---: | ---: |
| TGP | 2,265,528,237 | 2,777,157,377 | 1.163070x | 1.230318x | −2.8132% |
| Opus | 21,773,343,583 | 27,450,175,408 | 1.345115x | 1.345115x | −0.4330% |
| Grok | 20,969,727,163 | 26,205,322,261 | 1.346652x | 1.346652x | −0.3979% |
| P1996 | 3,600,502,896 | 4,536,172,521 | 1.352193x | 1.352193x | −0.6155% |
| P2000 | 6,049,922,238 | 7,474,158,998 | 1.444531x | 1.444531x | −0.3095% |
| P2016 | 11,343,424,651 | 13,760,566,379 | 1.382593x | 1.382593x | +0.1171% |
| Drawing | 2,149,015,961 | 2,154,394,984 | 1.004635x | 1.004635x | +0.0032% |

TGP's Ir fell 80,389,299, the recovery profile 143 expected from #843 (80.6M, projection 1.230x to about 1.163x): it measures 1.163070x, 0.072x below its 1.235x band. No projection rose; the highest play projection, P2000's 1.444531x, is 0.075x below the 1.52x band. No record reached its band, so no speed pass is due. No anchor or history was reset.

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
| 1 | Cleanup | #734: 265 of 319 workset files and 137,707 of 183,789 lines unfinished (stocktake 155). #735's 48 `too_many_arguments` rows and 11 `allow` sites fold into those passes. #736: 15,437 counted C++ lines under `tools/` and 4,787 in `src/tests`, plus the `stations.py` string. #737: batch W (about 180 lines), batch V part 4, and batches I and O (root, 2026-11-10). #738: one item | #734: all 319 files finished and closure (work order item 6) done; #735 closes with it. #736: each remaining C++ file under `tools/` and `src/tests`, and any embedded C++, checks the calling boundary itself and names the boundary whose retirement removes it ([evidence](porting.md#evidence)). #737: batches W, V, I and O delivered and its three withheld scanner rows classified (I and O added by root on 2026-11-10, from #839's review and #856's scanner rule). #738: its last item applied. Lint ratchets shrink per PR but set no exit, and the structural Clippy rows are outside it. Read each end state from the code and inventories, not from issue state | Current. #736 and #737 were reopened at this navigation: #810's "Closes #736 batch U" and #814's "It does not close #737" closed them on merge |
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

   **Finished-file residue.** Steering 33's audits found raw calls, a kept bounce, helper copies and missing comments in finished files of worksets 01 to 03. Some arrived after the file was finished, through later passes' renames. The audit lists are on [#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6747509029). One residue PR fixes them (#871, open, with #857's assertions), ranked right after the 03 remainder (#839 and #868, both merged) and reviewed by function group. A finished file is one with no remaining finding of the [review brief](review-brief.md)'s every-line list.

   Split a workset only at a file boundary, and only when its diff would exceed about 2,000 changed lines. That size is provisional: the 33 vocabulary PRs averaged about 150 lines. Above about 1,000 changed lines, assign one reviewer per function group ([review](workflow.md#review)). Later PRs touch a finished file only for a mechanical shared-file edit or for a caller adopting a changed signature.

   Order (adopted 2026-11-08 from the overhaul thread's proposal, made at the user's request; it replaces closing passes for 01 to 09 and then the largest integer-site count first): pass a workset before its callers. A file passed after its callees adopts their typed forms in its one pass; one passed before them keeps raw calls and is touched again. Fan-in is the number of files outside the workset that reference it through `crate::` (measured at `5af17025a3`).
   - (a) Finish the 03 remainder, `station_cmd.rs` (#839) and `station_service.rs` (#868), then the finished-file residue PR (#871). The two passes have merged.
   - (b) 13 Map/terrain (fan-in 90; `map_func` alone 85), with 23's type-bearing files: `bitmath_func` (39), `random_func` (21), `pool`, `math`, `kdtree`, `binaryheap`, `integer` and `container_func`. 23's fan-in of 137 is mostly `game_state`, `debug` and `witness` (45 to 46 each), which gain nothing from types; they and `raw_field` stay in 23's remainder.
   - (c) 09 (59), 08 (58), 07 (52), 06 (45), 04 (38), 05 (27). The proposal put 04 before 06, but the calls between those two worksets run both ways (06 to 04 in 2 files, 04 to 06 in 1), so strict fan-in governs.
   - (d) The rest by fan-in: 16 (67), 21 (62), 15 (42), 18 (37), 10 (33), 11 (28), 14 (27), 17 (26), 12 (25), and 23's remainder. The leaf worksets 22 (17), 20 (6) and 19 (1) go last. F's closure stays last.

   Large files are not left for later. A file over about 1,500 lines gets its own single-file pass, placed by fan-in like everything else: before its workset's other files when they call it, and otherwise immediately after the workset pass that skipped it (root, 2026-11-09, after the planner showed that `station_cmd.rs` calls the station group and that nearly every 09 file calls `movement.rs`). Twenty-four workset files exceed that size; the largest are `station_cmd.rs` (4,868), `orders.rs` (4,841), `town.rs` (3,373), `train.rs` (3,121), `industry.rs` (2,874) and `company.rs` (2,698). A single file cannot split at a file boundary, so a single-file pass above 2,000 changed lines stays one PR with one reviewer per function group, as #819 did. The 03 remainder therefore becomes three passes: `station_cmd.rs`, `station_service.rs` (2,285), and `station.rs`, `station_map.rs`, `station_cmd/waypoint.rs` and `station_lifecycle.rs` together. #734 reports progress in lines finished as well as files.

   A pass's reviewer applies the review brief's every-line list to every line left in its files. Proof follows the [cleanup checklist](review-checklist.md#cleanup-prs). Skip the archive-equivalence attempt for a pass that changes signatures or adds argument structs: it cannot pass, and #801's attempt differed in 15 of 370 object groups.
2. **Tools maintenance.** Delivered: #855's Clippy ratchet (#858, `9b379f0d99`), #856 with #847 and #867 (#869: fork-only CTest name ratchet, instantiation-only orphan exports and the non-failing `--report-uncalled`), and #854's inventory tool (#870, `tools/code-standards-inventory.py`). Each pass reports its files' residue from that tool.

   Fold any later small tools item into the next tools PR. Waiting for it: #874 (a tracked counter for functions over seven parameters) and #736's scope follow-up (count `COMMAND_SOURCE` and the extracted-original fragments in `tools/port-metrics.py`).
3. **#857**: the native assertions dropped in the town, industry and station ports. All of its rows are folded into the residue PR (#871), which holds those files (root, 2026-11-10).
4. **#736 test evidence and #737 boundary debt**, ranked by C++ removed per PR, one PR per batch where the batch fits the review bound above. Fold any item under about 150 counted C++ lines into another item of its batch. Delivered: #736 U (#810), P (#820), the W town, landscape and house families (#829, #833, #840), F cargo (#844) and F vehicle scheduler and company economy (#853); #737 V (#814) with parts 1 and 2 (#825, #837), and batch O's first PR (#872, 21 chains). Open: #737 V part 3 (#873). Order (root, 2026-11-10, from the planner's rankings on [#737](https://github.com/fetch-laboratories/openttd-rust/issues/737#issuecomment-6767417698) and [#736](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6767452427)):
   - #737 V part 4, the last of batch V, with the list of the baseline rows left in V's files and each one's owner;
   - #736 oracle compaction: the 127 to 129 oracles and #844's income oracle, with their capture recipes, as one PR ([#736](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6747531876));
   - #736's families by rank, each as its files free: W session, objectives and signs, then W NewGRF loading and storage, then the others. A #736 family PR removes more counted C++ than any remaining #737 PR. T follows W and F;
   - #737 W with batch O's nine free chains folded in, then batch I (company infrastructure counters, one cross-table PR). Both interrupt the families at entry 6b-1's merge and merge before entry 7a opens, because neither can be open beside the map-core pass;
   - #737 batch O's last PR, after the #736 families that free its 18 chains. It turns `--report-uncalled` into the enforced rule, which counts calls from upstream tests;
   - #736's end-state inventory, the stream's last PR.

   Every #736 item follows the oracle rule on #736, and each family's first checkpoint reports the share of its lines that is behaviour and moves. Root re-checks source boundaries and live reservations before commissioning each item.
5. **#738 rules**: its last item follows #855, which changes the carries the brief and checklist describe. One docs agent applies it; root reviews and commits the docs-only change between integrations.
6. **#734 closure**, after the last workset pass: private wrapper fields, generator support, and removal of the raw adapters and aliases. Seventeen Rust files (2,944 lines; `test_tables.rs` is 1,735 of them) are in no workset. Each joins one before closure, the type files with F ([#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6767381933)). The end state and later stocktakes and steering measure with #854's tool (item 2).

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

## Recurring

- Merges, [one merged PR each](root-workflow.md#integration), count from the switch to the merge queue after integration 147.
- After every twelfth merge (user, 2026-11-08), take stock under `/start-development` step 4 and profile the merged tip in the background ([speed](speed.md#instruction-projections)). A running profile counts as one of the two implementers. The next stocktake and profile follow the twentieth merge.
- Run the platform cadence after every fifth merge; the next follows the thirteenth; the cadences after merges 3 and 8 were green. A new failure stops merges ([integration](root-workflow.md#integration)). Full runs Windows x86, Clang and GCC Dedicated itself (#860), so the cadence adds macOS and the platform comparison. Cadences 135 (Windows x86 oracle tests, fixed by #818) and 140 (`-Wmissing-field-initializers`, fixed by #831) each stopped integrations for about 75 minutes.
- Deep checks run only on the user's request (user, 2026-10-31).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions.
- #156 (coverage gaps), #275 (speed), #724 and #710 (host) stay open trackers. #500 is delivered by #734; #168 and #199 attach to #737 and the owners that touch those records. #693 retains the cancelled reconnect failure and two completed original/original runs that did not reproduce it; the cause remains unresolved and that two-run authorization is exhausted. Its loopback hang cost two Full reruns in root's 11-08/09 session (integrations 141 and 145); the implementer queues the PR again once ([integration](root-workflow.md#integration)). #736 P (#820) delivered without touching it, so diagnosis returns with #730's network owner after cleanup, or sooner if hangs exceed one rerun in ten Full requests at a stocktake (provisional; 2 in 26 measured). #503's coverage continuation runs last.

Excluded, with issue placement and return conditions rechecked on 2026-11-09; near-full reproduction and fork release-source publication triggers are not met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Ready queue

At most three undispatched work orders ([delegation](team.md#ready-queue-and-pipelining)). A dispatched entry leaves the queue. Refreshed on 2026-11-10 at `df91fdf1fc` (code `03613dd0ac`, merge 16), against #883, #884 and [W+O's reservation](https://github.com/fetch-laboratories/openttd-rust/issues/737#issuecomment-6772646463), including its unpushed scope. [Ranking, caller analysis and stream placement](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6779110729) live on #734. Use the existing two implementers and keep at most six unmerged component branches; earlier review fixes take priority.

Agent: /root/cleanup_ready_queue_refresh | Model: gpt-6-astra | Reasoning effort: high

1. **22, #734/#735 workset 22 Text/GUI, next on the file-pass slot after #884.**
   - **Files and size:** `byte_strings.rs`, `control_codes.rs`, `encoded.rs`, `sprites.rs`, `string_func.rs`, `string_services.rs`, `string_validation.rs`, `utf8.rs`, `viewport_func.rs`, `widget_parser.rs`: 2,854 current raw lines, one PR; prior estimate 980 to 1,767 changed lines.
   - **Dependencies and overlaps:** adopt #884's `SeparatorUsage` in `encoded.rs`. No pass file is held by #883, #884 or W+O. Reserve any changed string API callers, including `packet.rs` and `auth.rs`; serialize with 20p. Leaf fallback while W and I block map core.
2. **20p, #734/#735 workset 20 `packet.rs` single-file pass, fallback if 22 cannot start.**
   - **Files and size:** `packet.rs`, 1,968 current raw lines; prior site-based estimate about 640 to 1,150 changed lines. Caller scope: `auth.rs`, `network_auth.rs`, `network_client.rs`, `network_command.rs`, `network_connection.rs`, `network_content.rs`, `network_dispatch.rs`, `network_frame.rs`, `network_socket.rs`, `network_tcp.rs`, `test_tables.rs`; reserve required typed-call edits and internal auth helpers together.
   - **Dependencies and overlaps:** none of that set is held by the current PRs or W+O. Prefer 22 first when #884 merges, then adopt its string APIs. If 20p starts first, 22 waits for its merge because both can edit packet/auth callers. Keep upstream-test packet exports under #737's recorded exception. No map-core prerequisite; the other 17 network files follow their packet callee.
3. **I, #737 company infrastructure counters, next on the boundary slot after W+O merges.**
   - **Size:** one cross-table PR, nine callback slots; prior inventory about 30 to 35 gross counted C++ lines and 127 infrastructure calls including notifications that stay. Size replacement layout/access costs at the first checkpoint.
   - **Files:** Rust `company.rs`, `company_type.rs`, `station_cmd.rs`, `station_cmd/waypoint.rs`, `road_cmd.rs`, `tunnelbridge_cmd.rs`, `rail_cmd.rs`, `objects.rs`, `water.rs`, `landscape_cmd.rs`, `water_cmd.rs`, `test_tables.rs`; C++ `src/rail_cmd.cpp`, `src/rust/company_services.h`, `company_adapter.cpp`, `company_ffi.h`, `rail_command_ffi.h`, `ground_ffi.h`, `station_command_ffi.h`, `station_command_services.hpp`, `services.cpp`, plus its generated layout declaration inputs and boundary baselines.
   - **Dependencies and overlaps:** W+O holds station/world files and `services.cpp`. After its clean base update to `a571ff2df6`, #883 no longer changes `station_cmd.cpp` or `station_service.rs`, so W has no collision with that PR (root, 2026-11-10). I is disjoint from 22; its potential `test_tables.rs` overlap with 20p must serialize if both need edits, with I taking priority. I must merge before map core 7a opens. Keep window-dirty notifications; obtain reviewer acceptance for any generated-layout growth.

When W and I finish, 7a immediately outranks the leaf fallbacks: `map_func.rs` and `tile_map.rs`, with finished callers adopting typed forms, including `town.rs`'s six map-reader wrappers. Refresh the queue then; 9m still follows 7a. The #736 session family follows I on the boundary slot, then NewGRF loading/storage and the remaining ranked families. Full phase-3 planning is not triggered: cleanup still has undispatched work outside this queue.

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
