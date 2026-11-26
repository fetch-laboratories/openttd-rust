# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place ([records](workflow.md#records)); `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands

Base is at merge 110 (`b9b6de7c49`), where root's session that began 2026-11-25 19:48Z stopped at the user's request ([resume checkpoint](#resume-checkpoint)). The bullets are steering 40's counts at merge 110. The tables are root's stocktake at merge 107, with its outputs and scripts in `.local/stocktake-107/`. The next stocktake follows merge 119, and the platform cadence ran at merges 101, 102, 107 and 109.

- **#734 has 174 of 318 workset files finished and 73,029 lines left**, from 82,156 at merge 97: 9,127 fewer in the session's twelve running hours ([who decides](../AGENTS.md#who-decides)), 754 an hour, against 789 in merges 90 to 96 and 947 in merges 84 to 89. The fall was 975 an hour in the first eight hours and 330 in the last four, when one slot held #1084 and the other took #1087 and #1088, which finish no file. At 750 to 975 an hour the 73,029 lines are about 75 to 97 running hours of passes, before the 57 unfinished files in no workset (14,845 lines), closure and #893. Worksets 06, 07, 08 and 10 are finished; of group (c), 04 has three files left and 05 all six.
- **Fork C++ is at 268,860 lines**, 1,021 fewer than at merge 97. Original C++ retired is 63,202 lines, 22.23%. The boundary has 1,434 exports and 2,626 callback slots. #735 is unchanged at 101 functions over seven parameters, and #892 has 100 run-time assertion macro sites in 43 files, from 119 at merge 107.
- **Slot time**, 1,559 slot-minutes on eleven PRs: editing 31%, local preflight 20.5% (14.5% with two implementers and 27.5% with three or four), builds 15%, base updates 10%, the queue 9.5% and speed counts 2.9%. Fixing review findings took 25% with its builds and preflights: 158 minutes for first reviews, 48 for re-checks and 129 for scoped reviews after a re-check.
- **Review:** 34 reports held 107 findings, none false of the 64 checked. One was serious and fixed before merge: #1081 indexed with a check where the original reads an empty vector. 29 are about where `unsafe` sits and change nothing the game does; #1066's seven scoped reviews after its re-check are of that kind (#1090). As merged, #1066 keeps five minor departures, 0.80 in each 1,000 changed lines, and #1086 two, 0.33, against 0.33 for #1056 and 0.7 to 2.1 for the seven passes before it. None is a difference the game can reach.
- **A slot in trouble** reached root through the user or by chance (#1091): #1066's seven scoped reviews, and on #1084 a 68-minute full local simulation and eight reviews on the implementer's own model.
- **CI:** Quick took 4.3 to 6.5 minutes as three parallel jobs, and a passing Full a median 14.85 minutes over the thirteen since merge 96. A full local preflight took a median 302 seconds over the session's 62 runs, of which 18 failed. #1066 left the queue once, when its merge group no longer compiled against #1080. `Migration baseline validation` failed at merge 110 before compiling, in apt.
- **Speed:** every play scenario runs fewer instructions than at merge 95 and generation 0.45% more, all under their anchors ([speed](#speed)).
- The limit is two implementers ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)). At the user's word four ran for 34 minutes and three for 78, from 19:57Z on 2026-11-25, to their merges.
- No rollback has happened since merge 32. Full phase-3 planning is not triggered, and near-full reproduction is not met. 69 issues are open and none is unplaced (`python3 docs/skills/steer/issues.py`).
- **Unmerged branches on origin:** 54 at merge 107, listed in `.local/stocktake-107/12-branches-prs.txt`; #1084 and #1089 are the open PRs. Among the branches kept with no PR are `preserve/housekeeping-foundation-6ab9c20e5d` and `fathom/1082` (`52e69b22ae`, the one docs commit of the closed #1082, which #1090 reads).

Reproduce the census with `python3 tools/port-metrics.py --whole-fork` (metric version 3).

| Measured quantity | Merge 97 | Merge 107 | Change per running hour (17) |
| --- | ---: | ---: | ---: |
| Original C++ remaining | 221,356 | 221,283 | −4 |
| Fork source glue, including generated records | 29,470 | 28,842 | −37 |
| Compiled source and generated records C++ | 250,826 | 250,125 | −41 |
| Tracked tools C++ evidence | 14,389 | 14,399 | +1 |
| ABI-generated non-record C++ evidence | 4,666 | 4,549 | −7 |
| Measured fork C++ series | 269,881 | 269,073 | −48 |
| Original C++ retired, of 284,355 | 62,999 (22.16%) | 63,072 (22.18%) | +4 |
| Simulation C++ retired, of 61,148 | 41,708 (68.21%) | 41,770 (68.31%) | +4 |
| Historical regex exports / callback slots | 1,443 / 2,712 | 1,438 / 2,629 | −0.3 / −5 |
| Shared-accessor / rebound-callee / orphan-export baseline rows | 45 / 203 / 21 | 42 / 192 / 17 | |
| Exports held only by uncalled members (baseline only shrinks) | 19 | 19 | |
| Uncalled C++ free functions in #1077's baseline | | 34 | |
| Embedded C++ outside the census: oracle recipes | 302 | 302 | |
| Rust / tooling code lines | 208,285 / 78,998 | 212,731 / 81,786 | +262 / +164 |

| Source area / phase relationship (merge 107) | Original | Retired | Remaining | Fork glue |
| --- | ---: | ---: | ---: | ---: |
| Simulation logic (phase 1) | 47,424 | 38,212 | 9,212 | 4,557 |
| Simulation storage (phase 2) | 13,724 | 3,558 | 10,166 | 1,362 |
| NewGRF, save/load, script API and network (phase 3) | 51,794 | 13,099 | 38,695 | 3,257 |
| Support, graphics and viewport (phase 4) | 32,602 | 1,081 | 31,521 | 538 |
| GUI and platform (phase 5) | 64,120 | 424 | 63,696 | 466 |
| Vendored (phase 6) | 55,148 | 2,317 | 52,831 | 0 |
| Data tables | 17,295 | 4,367 | 12,928 | 14 |
| Upstream tests and fork test glue | 2,248 | 14 | 2,234 | 4,731 |
| Boundary layer | 0 | 0 | 0 | 13,917 |

Coverage remains bounded by the existing saves, NewGRFs, scenarios and retained native fixtures; entry witnesses are not branch coverage. New gaps go on #156's lists, which are its sub-issues #1038, #1039, #1040, #1041, #1042 and #1043: this interval's passes added Action D target `0x9E` to #1041, the `GetActiveCargoLabel` `NOT_REACHED` arm to #1040, and to #1042 the house tile no scenario clears through a disaster. #503 deep checks remain user-requested.

### Cleanup stream inventories

The comment, integer, wrapper, constant, literal and assertion rows are analyzer estimates from `tools/code-standards-inventory.py`, given as "tree; in finished files". Their "in finished files" parts rise as files finish and are mostly rows a reviewer would dismiss, such as export parameters and raw adapters. Root adds the next column at its stocktake ([selection](planning.md#selection)).

| Stream | Inventory | Merge 97 | Merge 107 |
| --- | --- | ---: | ---: |
| #734 | Workset files with a finished pass | 160 of 319 | 173 of 318 |
| #734 | Workset lines in finished files | 135,138 | 146,256 |
| #734 | Lines left in unfinished workset files | 82,156 | 74,355 |
| #734 | Explanatory comment lines missing inside matched ported bodies | 1,537; 250 | 1,439; 251 |
| #734 | Bare-integer parameters and fields whose concept has a named type | 1,183; 372 | 1,164; 390 |
| #734 | Transparent wrapper types / named constants | 381 / 4,081; 118 / 1,470 | 391 / 4,144; 148 / 1,779 |
| #734 | Literal sites standing for a named value | 12; 0 | 0; 0 |
| #734 | #892's production assertion macro sites at run time (`debug_assert*!` / `assert*!` / `unreachable!`), with compile-time `assert!` beside them | 123 in 46 files (6 / 43 / 74); 37 in 16 (0 / 14 / 23). Compile-time 15 | 119 in 44 files (6 / 43 / 70); 37 in 16 (0 / 14 / 23). Compile-time 15; 13 |
| #735 | Functions over seven parameters | 101 (67 FFI, 16 helpers, 18 commands) | 101 (67, 16, 18) |
| #735 | `too_many_arguments` ratchet rows / `allow` sites | 32 / 6 | 31 / 6 |
| #736 | Fork C++ evidence under `tools/` / `src/tests`, code lines | 14,389 / 4,793 | 14,399 / 4,731 |
| #738 | Rule corpus lines | 4,281 | 4,352 |
| None | Ruff rows | 283 | 282 |

The workset count is one command since #1065 (`docs/worksets.md`): 318 entries, none without a file. 14 finished files (6,232 lines) and 57 unfinished Rust files (14,847 lines) are in no workset; `.local/stocktake-107/4-worksets.txt` lists them, and [closure](#cleanup-work-order) places them. The lines left are the measure ([selection](planning.md#selection)).

Counting commands:

- **#734:** reconcile the issue workset manifest with merged complete-file pass PRs, then count current files with `wc -l`; a deleted finished entry remains in the denominator, and mechanical caller edits do not finish a file. The lines left are the `wc -l` of the workset files that `migration/finished-files.txt` does not list; their fall per running hour gives the stream's remainder. Run `python3 tools/code-standards-inventory.py [--head REV]` for the tree and with the finished paths for that subset; its `Assertion macros` line gives #892's sites, run-time and compile-time apart. The scripts of the merge-95 count are in `.local/stocktake-95/`.
- **#735:** use the tracked long-signature inventory counter, `migration/clippy-debt.json` (`too_many_arguments` keys) and `git grep 'allow(clippy::too_many_arguments' -- rust/`.
- **#736, the census baseline rows and the Clippy and Ruff rows:** from `python3 tools/port-metrics.py --whole-fork` (exports and callback slots are its "Boundary" line), `python3 tools/orphan-exports.py --report-uncalled`, `wc -l` of the three baseline files, and the debt files (keys are `sites` entries, diagnostics the sum of their list lengths). Expanded exports: `growth_inventory` in `tools/port-metrics.py`.
- **#738:** `cat AGENTS.md $(git ls-files 'docs/*.md' 'docs/**/*.md' | grep -v -E '^docs/(roadmap.md|components/)') | wc -l`.

### Speed

The merge-107 profile on clean `969478514d` is the latest. All seven exit saves compared with no difference. The driver exited 1 on its anchor identity guard for every scenario, as it has since an unattended upgrade of the system's `libssh-4` on 2026-11-24 changed the digest of one linked library (#275); the library is not on the measured path, so these projections are computed from the report's instruction counts and the anchors by the [formula](speed.md#instruction-projections). The profile's output is retained in locked `.local/worktrees/root-profile-merge107`, the previous tip for attribution at the next profile; the merge-95 profile checkout `.local/worktrees/root-profile-merge95` is released.

| Scenario | Original Ir | Candidate Ir (merge 107) | Change since merge 95 | Projection (merge 95) | Projection (merge 107) |
| --- | ---: | ---: | ---: | ---: | ---: |
| TGP | 2,265,550,775 | 2,771,249,331 | +0.45% | 1.172488x | 1.172488x |
| Opus | 21,773,210,518 | 25,988,868,308 | −0.25% | 1.317818x | 1.317818x |
| Grok | 20,969,144,714 | 24,949,842,098 | −0.24% | 1.335597x | 1.335597x |
| P1996 | 3,600,042,113 | 4,397,434,150 | −0.35% | 1.319243x | 1.319243x |
| P2000 | 6,042,476,469 | 7,237,325,989 | −0.60% | 1.401373x | 1.401373x |
| P2016 | 11,345,807,488 | 13,454,146,983 | −0.04% | 1.343582x | 1.343582x |
| Drawing (diagnostic) | 2,149,057,585 | 2,154,618,685 | +0.02% | 1.004577x | 1.004899x |

Every play scenario and generation runs fewer instructions than its anchor, so each projection is its anchor's ratio and none is in its band: generation is at 1.172x against 1.235x and the highest play record, P2000, at 1.401x against 1.52x. Generation rose by 12.3 million instructions since merge 95 and is 0.59% under its anchor; no projection rose by 3%, so [speed policy](speed.md#policy) asks for no attribution, and the pass that counted that scenario (#1080) measured −0.07% on it. The next profile attributes it if it rises again. The passes counted their own scenarios at base and head ([ADR 0054](adr/0054-per-pass-counts-run-beside-review.md)): #1056 −0.27% on P1996 and +0.09% on P2016 at its final head, with its earlier rises attributed in its PR body; #1081 −0.06% and #1066 −0.13% on P2016; #1083 −0.25% on Opus and −0.38% on P2016, where its plan estimated 100 to 280 million fewer instructions on Opus and it gave 65 million. Until a quiet pass re-anchors, `simulate --instructions` exits 1 after recording its counts, and a pass compares raw candidate counts at base and head. The original's counts equal the merge-95 profile's to the instruction.

## Course

- Execution order: the cleanup stage, then phase 3 readers, then phase 2 storage with each pool's save/load, then phases 4, 5 and 6 ([goal and phases](planning.md#goal-and-phases), [ADR 0003](adr/0003-work-order-cleanup-readers-storage.md)).
- Yardstick: net fork C++ removed, counting C++ under `tools/` and `src/tests` ([cost decisions](planning.md#cost-decisions), [ADR 0022](adr/0022-progress-is-net-cpp-removed.md)).
- Limits: [implementer slots](team.md#slots) and the [stocktake cadence](root-workflow.md#cadences).

## Phases

Agent: /root (Claude Code navigation) | Model: claude-fable-5-2 | Reasoning effort: 80 (host-reported number)

Phase numbers name content; the execution order sets the sequence. Phase 1 (simulation logic) exited at integration 63.

Position at merge 75 (`00494f6c23`), measured on 2026-11-23. Counts are code lines from `python3 tools/port-metrics.py --whole-fork` (metric version 3): comments, blanks and preprocessor lines are excluded, and the scope is `src/` plus generated records for all platforms, not one executable. Parentheses compare the last navigation, `c80ace14b8` (merge 59, `f24c05dbe9`), recounted with the same metric; the recount matches its published figures. Areas are accounting buckets; the last column names what removes each.

| Area | Original | Retired (change) | Remaining (change) | Fork glue (change) | Compiled C++ (change) | Removed by |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Simulation logic | 47,424 | 38,015 (+154) | 9,409 (−154) | 5,070 (−520) | 14,479 (−674) | Phase 1 has exited; its residue goes with its consumers: drawing functions in phase 5, startup (`openttd.cpp`) in phase 4, facades and other bodies with their phase-2 owners |
| Simulation storage | 13,724 | 3,463 (+221) | 10,261 (−221) | 1,380 (−4) | 11,641 (−225) | Phase 2 |
| Save/load | 13,388 | 1,440 (+5) | 11,948 (−5) | 1,070 (0) | 13,018 (−5) | Phase 3, except about 4,300 lines of pool and map chunks and their compat tables, which go with phase 2 |
| Script API | 14,908 | 2,399 (−7) | 12,509 (+7) | 750 (0) | 13,259 (+7) | Phase 3; VM and binding shims in phase 6 |
| Network | 12,713 | 3,272 (+12) | 9,441 (−12) | 1,075 (−83) | 10,516 (−95) | Phase 3 |
| NewGRF | 10,785 | 5,986 (+12) | 4,799 (−12) | 368 (−4) | 5,167 (−16) | Phase 3 |
| Support | 21,617 | 1,054 (+81) | 20,563 (−81) | 539 (−5) | 21,102 (−86) | Phase 4 |
| Graphics and viewport | 10,985 | 19 (+12) | 10,966 (−12) | 4 (0) | 10,970 (−12) | Phase 4 |
| Data tables | 17,295 | 4,367 (0) | 12,928 (0) | 14 (0) | 12,942 (0) | Each table with its last C++ consumer, in phases 2 to 5 |
| GUI | 52,179 | 415 (0) | 51,764 (0) | 463 (0) | 52,227 (0) | Phase 5 |
| Platform | 11,941 | 8 (+8) | 11,933 (−8) | 2 (0) | 11,935 (−8) | Phase 5 |
| Vendored | 55,148 | 2,317 (0) | 52,831 (0) | 0 (0) | 52,831 (0) | Phase 6 |
| Upstream tests and fork test glue | 2,248 | 14 (0) | 2,234 (0) | 4,857 (+87) | 7,091 (+87) | Fork tests with the pools and readers they reach ([ADR 0023](adr/0023-cpp-tests-stay-until-code-is-ported.md)). Upstream tests go with the code they test; Catch2 goes when the last C++ test leaves |
| Boundary layer | 0 | 0 (0) | 0 (0) | 15,919 (−370) | 15,919 (−370) | Each side's owner; the final exit |
| **Source total** | **284,355** | **62,769 (22.07%, +498)** | **221,586 (−498)** | **31,511 (−899)** | **253,097 (−1,397)** | |
| C++ under `tools/` | | | | 14,473 (−21) | 14,473 (−21) | With the pools, readers and boundaries each file reaches (#736's classification) |
| ABI-generated non-record C++ | | | | 5,130 (−173) | 5,130 (−173) | With the records and layouts it checks |
| **Fork C++ census** | | | | | **272,700 (−1,591)** | |

Inventory correction, apart from retirement: no original C++ came back. The script API row shows 7 lines fewer retired although no file under `src/script`, `src/ai` or `src/game` changed between the endpoints: the metric now counts the constructor of `ScriptRailTypeList`, whose file #666 deleted, as remaining. That is a counting effect, and its cause is not traced. The rates table below is recounted with version 3 at every endpoint.

Of the 19,603 evidence lines, 17,976 are candidate or shared, none is reference-only and 1,627 are unresolved, which the census counts as candidate. Outside the census are 836 raw lines of C++ in Python strings and 302 lines of oracle recipes, which compile only against the pinned original. The boundary has 1,467 exports (−34) and 3,039 callback slots (−139). Rust has 200,156 code lines (+9,860) and tooling 77,314 (+3,781).

Rates are per running hour ([who decides](../AGENTS.md#who-decides)), counted between code endpoints from `git log --all --format=%at`. An hour on a boundary counts in both periods, so the periods sum to a few hours more than the totals. Every period ran before the [hardware change](rust-migration.md#hardware) of 2026-11-23, since the last code commit is from 2026-11-19; the next navigation's first period spans it. The last column is the lines left in #734's unfinished workset files, with their fall per running hour.

| Period | Running hours | Original retired | Per hour | Fork C++ census change | Per hour | #734 lines left (fall per hour) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Fork start `6ced7c097e` to the stop before cleanup, `e800717540` | 339 | +62,150 | 183 | −4,600 | −14 | |
| Cleanup to `61c6d2c598` (integration 146) | 32 | −40 | −1 | −2,711 | −85 | |
| To steering 34, `3a2d52b870` | 39 | +109 | 3 | −1,509 | −39 | |
| To steering 35, `5b0b8427b8` (merge 47) | 23 | +2 | 0 | −457 | −20 | 116,327 |
| To merge 59, `f24c05dbe9` (the last navigation; steering 36 followed) | 9 | +50 | 6 | −787 | −87 | 105,623 (1,189) |
| To merge 71, `603724a3fd` (the stocktake) | 36 | +472 | 13 | −1,333 | −37 | 100,290 (148) |
| To merge 75, `00494f6c23` | 6 | +26 | 4 | −258 | −43 | 98,024 (378) |
| **Since the last navigation** | **42** | **+498** | **12** | **−1,591** | **−38** | **−7,599 (181)** |
| **Cleanup stage so far** | **139** | **+619** | **4** | **−7,055** | **−51** | |
| **Whole history** | **478** | **+62,769** | **131** | **−11,655** | **−24** | |

What set the rate:
- The cleanup stage retires no original C++ by design ([ADR 0004](adr/0004-cleanup-before-porting.md)). It has run 139 of the fork's 478 running hours. The 619 lines retired in it are forwarding bodies that design-fault repairs removed.
- #734's pace fell to about a sixth after merge 59: the lines left fell by 1,189 a running hour in merges 48 to 59 and by 181 in merges 60 to 75. Merges per session hour fell from 1.6 to about 0.6. Three measured changes account for it, each by about a factor of two:
  - **Fewer PRs in flight.** On average 4.0 PRs were open in the session of merges 48 to 59, then 2.7 and 2.2 in the sessions of 2026-11-14 and 2026-11-18, under the limit of two implementers ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md), the user's).
  - **Each PR takes longer.** The median time from a PR's opening to its merge rose from 1.6 hours to 2.3 in merges 60 to 65 and 3.0 in merges 66 to 75. Review reports for each PR rose from 3.2 to 8.5 and 7.8, the share of reports with findings from 38% to 49% and 88%, and review-fix commits for each PR from 1.8 to 4.7 and 11.8. Two reviewers for each group came with steering 36, after merge 59, and the `gpt-6.2-sol` reviewer at merge 66.
  - **Fewer merges were file passes.** Nine of 12 before and 6 of 16 after. The other 10 were shared services, string IDs, two tools batches, #994 and the tile-loop repair, which are cleanup work that leaves the lines-left count where it was.

  Twelve of the 41 distinct running hours also had only Markdown commits.

  Steering 37 audited the review load named here; its result and rule changes are under [latest steering actions](#latest-steering-actions).
- At 181 lines a running hour the 98,024 lines left are about 540 running hours of passes, before closure and #893; at the 378 of merges 72 to 75 they are about 260. The fork's whole history is 478 running hours. The last navigation's "about seven more days" used the pace of merges 48 to 59.
- The merge-71 stocktake projected that remainder from finished lines per running hour, which also count the growth of files already finished: it gave 285 running hours where the lines left gave 680. [Selection](planning.md#selection) now measures the lines left.
- Ownership ports retired 183 original lines a running hour before the cleanup, at which the 221,586 lines left are about 1,210 running hours. That is not a forecast: it produced the code the cleanup is redoing, no port has been made under the current review rules, and the GUI, platform and vendored code have no measured pace.
- ADR 0049's compile times predate the hardware change and do not compare with later ones ([hardware](rust-migration.md#hardware)); a later measurement of the limit starts from the new hardware.

| Order | Stage | Size now | Exit (end state) | Status |
| ---: | --- | --- | --- | --- |
| 1 | Cleanup | #734: 142 of 319 workset files finished; 176 files and 98,024 lines are left, about 540 running hours at the pace since merge 59. #735: 115 long functions, 42 debt keys and 9 allows at merge 71. #736: 14,473 counted tools C++ lines and 4,857 fork test lines, plus separately inventoried embedded C++. #737 and #738 are closed at their end states | #734: all 319 files finished and [closure](#cleanup-work-order) done; #735 closes with it. #736: no file the [classification](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6804817013) calls movable remains, as [steering 35 amended it](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6834080910); each remaining C++ file under `tools/` and `src/tests`, and any embedded C++, either checks the calling boundary itself or reaches a live native pool, reader or production caller, and names the owner or boundary whose retirement removes it ([evidence](porting.md#evidence)). The 65 pool and reader fixtures stay native until their owners retire them. Lint ratchets shrink per PR but set no exit, and the structural Clippy rows are outside it. Read each end state from the code and inventories, not from issue state | Current |
| 2 | Phase 3: readers (script API, NewGRF, network, content, admin, save/load outside pool chunks) | About 37,700 compiled lines: script API 13,259, network 10,516, NewGRF 5,167, and save/load 13,018 less the pool and map chunks. The [queued owners](#after-cleanup) estimate 4,600 to 7,000 net C++ removed, so about 31,000 to 33,000 lines have no owner yet | Readers run in Rust with evidence for their observable outputs. Every remaining body in these buckets has a named later owner: VM and binding shims phase 6, script and network GUI phase 5, pool chunks phase 2 | Next. Plan it in full before cleanup's last items are dispatched ([goal and phases](planning.md#goal-and-phases)), and no sooner: the user declined planning it during cleanup as far too early. |
| 3 | Phase 2: storage, each pool with its save/load chunks, then the map | 11,641 compiled lines in storage, about 4,300 save/load chunk lines, the logic residue assigned to pool owners, and record field routes in the 15,919-line boundary layer | Rust owns every pool's records, slots, allocation, iteration, lifetime and save/load, then map allocation and remaining map bodies; no Rust-used field behind a native service | Queued; placement in the [reader and pool placement](design/reader-pool-placement.md) design |
| 4 | Support | About 32,100: support 21,102 and graphics and viewport 10,970, plus their data tables and startup | As in [planning](planning.md#goal-and-phases) | #3; planned when it becomes the next phase ([ADR 0005](adr/0005-plan-only-the-next-phase.md)) |
| 5 | GUI, drawing and platform | About 64,200: GUI 52,227 and platform 11,935, plus simulation and data-table drawing | As in [planning](planning.md#goal-and-phases) | As phase 4 |
| 6 | Vendored and final retirement | Vendored 52,831, plus whatever boundary, test and evidence C++ remains | Every original and fork-written C++ line retired and candidate native targets removed ([goal and phases](planning.md#goal-and-phases)) | As phase 4 |

Verdict at this navigation (cited at `ca752b114a`):
1. **Goal: holds.** `AGENTS.md:3-4`, [goal and phases](planning.md#goal-and-phases) and `/start-development` name the same end: a Rust-only game with every original and fork-written C++ line retired.
2. **Plan: holds.** Every remaining line has a phase that removes it, and phase 3 is planned in full before the cleanup's last items are dispatched.
3. **Rules: hold.** ADR 0004 keeps original retirement near zero through the cleanup by design. That cost is known and accepted, and the user confirmed it at this navigation.
4. **Metrics: one failed and is corrected.** `docs/roadmap.md:13` and `docs/planning.md:27` measured #734 by files and lines finished, which more than halved its projected remainder.
5. **Rate: the cleanup's own pace is the finding.** It fell to about a sixth after merge 59, from the three measured changes above.
6. **Autonomy: holds.** `docs/roadmap.md:13` left the cleanup's pace to steering 37 unmeasured; this navigation measured its causes and leaves steering 37 the audit of the review load.

Root's first actions on resume:
1. Follow the [resume checkpoint](#resume-checkpoint) and the [latest steering actions](#latest-steering-actions), and continue the cleanup. Plan phase 3 at its recorded trigger.
2. At each stocktake, record the lines left in #734's unfinished files and their fall per running hour, and project the stage's remainder from that ([selection](planning.md#selection)).

## Cleanup work order

Each stream issue holds its batches, rules, proof and inventory. [Selection](planning.md#selection) ranks cleanup work by stream inventory. File ownership and file-set checks follow [team](team.md#ready-queue-and-pipelining); what a cleanup PR needs to merge is in [integration](root-workflow.md#integration).

Slots: one [implementer slot](team.md#slots) always runs a #734/#735 file pass when one is ready, because the stage exits when the file passes do. With every slot on file passes the lines left fell by 947 a running hour in merges 84 to 89 and by 789 in merges 90 to 96; with file passes at 43% of slot time they rose in merges 76 to 83. Another slot takes, in this order: a defect that fails other PRs' runs, a member of the shared-service family below, and otherwise a file pass of its own chain. Each slot works its own chain of passes, on worksets that do not call the other chains' ([sizing](planning.md#sizing)); the [ready queue](#ready-queue) names the chains. #738 is closed at its end state.

1. **Tools batch.** One per stocktake interval, dispatched within it and ranked by its items' measured cost ([selection](planning.md#selection)). #1063's batch merged as #1065 (merge 102). The batch of the interval that starts at merge 108 was #1085, merged as #1088 (merge 110): it deleted the uncalled C++ free functions that no open pass holds, and the push hook now refuses the base while the merge queue is not empty. New inventory visibility is not C++ deletion.
2. **The shared-service family:** shared services, accessors and string IDs that several files need. Merged: news and script events (#982), windows and sound (#1001), errors (#1005), console and chat (#1008), the string IDs (#995, #1006, #1033), the framerate service (#1020) and `BaseStation::xy` (#1028). Left: `Vehicle::Tick`'s dispatch (#1011, after the `ship_control.rs` pass) and the shared crate (#998, with strgen's port in phase 4). The family is closed at these two: root opens a new member only with the counts [selection](planning.md#selection) asks for. Until a member merges, a pass keeps the carriers it would replace and names the work order ([design faults](porting.md#design-faults)).
3. **#734/#735 file passes.** One PR per workset in the #734 manifest finishes every file it touches, applying the [code standards](porting.md#code-standards) to every line. Sizing, splits and single-file passes follow [sizing](planning.md#sizing); proof follows [cleanup proof](porting.md#cleanup-proof). Each pass reports its files' residue from `tools/code-standards-inventory.py`.

   Order, by fan-in (measured at `5af17025a3`). Delivered: the 03 remainder and its residue pass, 23's remainder (#884), 13 Map/terrain with 23's type-bearing files, except 23's `native_test_support.rs` and `native_test_support/movement.rs` (#973), and 09.
   - (c) 08, 07, 06, 10, 04, 05. Worksets 06, 07, 08 and 10 are finished. 04 is #1057, in four PRs: `cargo_storage.rs` is the draft #1089, and `cargo_payment.rs` with `cargomonitor.rs` follows it. 05 is #1060, in three: `orders.rs` with `order_func.rs` is the draft #1084.
   - (d) The rest by fan-in ([sizing](planning.md#sizing)), the files outside a workset that reference it, counted at merge 95 (`.local/stocktake-95/11-fanin-output.txt`): 16 (89, #1062), 21 (75), 15 (51, #1064), 18 (47), 17 (37), 11 (36), 14 (30), 12 (29). Workset 19 (2) goes last of them, and F's closure stays last of all. Steering 39 put this order back: root's merge-95 order counted only the finished files that reference each workset, which moved 18 ahead of 15 and 12 ahead of 14. A finished caller is edited whenever its callee's workset passes, so it ranks no workset over another; the callers the order spares are the unfinished ones, and they give this same order. Counted again at merge 107 (`.local/stocktake-107/11-fanin-output.txt`), the order holds: 16 at 96, 21 at 78, 15 at 52, 18 at 48, 17 at 39, 11 at 37, and 14 and 12 at 32. Root counts again at the merge-119 stocktake.

   The unfinished files over 1,500 lines, each with a single-file pass, are `orders.rs` (5,850, #1084), `cargo_storage.rs` (2,892, #1089), `objects.rs` (2,674), `road_cmd.rs` (2,581), `newgrf_vehicle.rs` (2,141), `newgrf_loading.rs` (1,839), `fleet.rs` (1,790), `tunnelbridge_cmd.rs` (1,653), `landscape_generation.rs` (1,573) and `services.rs` (1,543, with F); `test_tables.rs` (2,375) is in no workset. #734 reports progress in lines finished as well as files.
4. **#736 test evidence.** No transfer remains. The aircraft range probe stays native under #656: #950 projected 10 to 13 changed lines for each net C++ line, above every delivered transfer, and the user approved closing it ([reason](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6860491029)). The classification table in #736's body names each remaining file's owner or boundary, as of 2026-11-14; that closes the stream's inventory without a PR ([ruling](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6834080910)).

   What stays native: `tools/migration/aircraft-range.cpp` with `finite_range_probe` (#656 retires it), the Catch2 files that delete whole (#931 closed, since its recipes outweighed the C++ it deleted), `src/tests/train.cpp` (its PBS and reservation cases retire with #725, its speed case with #656), and the ScriptList sections of `tools/migration/script-list-comparison.cpp`, which retire with that file when the script API reader takes the `ScriptList` adapter classes. The console fixtures and Catch2 files that reach live pools (65 files, 14,273 counted lines) retire inside the ports that retire their pools or readers. The 33 files and 3,909 lines of retained boundary evidence leave with their boundaries.
5. **#734 closure**, after the last workset pass: private wrapper fields, generator support, and removal of the raw adapters and aliases. At merge 95, 56 unfinished Rust files (14,268 lines; `test_tables.rs` is 2,375 of them) are in no workset, most of them test modules that passes moved out of their files; #1063's item 5 lists them. Each joins one before closure, the type files with F ([#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6767381933)). The end state and later stocktakes and steering measure with `tools/code-standards-inventory.py`. The closure residue sweep takes the recorded residue lists ([steering 34's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6789587702), [steering 35's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6834079997), [steering 36's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6847188216) and [steering 37's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-7119259564)) unless a later pass touches the file; its macro sites are #892's, and the three YAPF search-loop copies are #971's and stay. #892 rides remaining file passes, then that sweep and closure; no unrecorded production assertion macro site remains at closure, measured as #892 specifies.

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

- #156 (coverage gaps, whose lists are its sub-issues #1038, #1039, #1040, #1041, #1042 and #1043), #275 (speed), #724 and #710 (host) stay open trackers. #710 has [the report lost at the pause of 2026-11-24](https://github.com/fetch-laboratories/openttd-rust/issues/710#issuecomment-7143126308) and [a reviewer that stopped on a host approval prompt](https://github.com/fetch-laboratories/openttd-rust/issues/710#issuecomment-7149805139) under a full-access runtime; [steering 39 measured both sessions' host failures](https://github.com/fetch-laboratories/openttd-rust/issues/710#issuecomment-7158594687).
- #1058 owns `CargoSpec::array`, its defaults and its publication in phase 2, queued with the smaller records of #638 and #498; workset 04's growth record for the `CargoSpec` member slots names it.
- #500 closes with #734's closure. #168 and #199 are closed: #565, #717 and #734 own what remained.
- #693: #1015's work traced the `network-protocol-loopback` stalls to a thread race in the original's `TCPConnecter`, which can stay in `Resolving` for ever when its resolver finishes first (#1016, a `deferred-improvement`: neither build changes). The harness bounds its waits and retries a session once on that signature while the candidate's copy of the file is identical to the pinned original's, counting each retry in the run's report; stocktakes read the rate there. The `content-protocol` evidence misses stay open on #693: that scenario runs without network debug output, so its logs cannot show the race. The allowance ends when #730 ports the connecter.
- #503's coverage continuation runs last.
- #1067 tracks the [fathom](skills/fathom/SKILL.md) cases, one checklist line for each, costliest first. Open: #1090 (reviews of where `unsafe` sits), #1091 (a slot in trouble reaches root only through the user), #1070 (a new worktree's first build), #1072 (`native` failures before review) and #1073 (findings that could be settled before review). Steering reads each landed fix's number and closes its case or sends it back.
- #939 lists 35 rules that a script, hook or lint could hold instead of a doc. A candidate joins a [tools batch](planning.md#selection) when its rule was broken in the window just ended, with the count; [steering 40's counts](https://github.com/fetch-laboratories/openttd-rust/issues/939#issuecomment-7187322964) for merges 98 to 110 are the latest, and the two that cost most are a reviewer on a model that is not the review row's (2 PRs, about 119 slot-minutes) and a full local simulation with no scenario named (68).
- #1026's acceptance, as [root settled it](https://github.com/fetch-laboratories/openttd-rust/issues/1026#issuecomment-7125440240), read over merges 84 to 95: the five PRs whose last push needed no fresh gate entered the queue a median 10.35 minutes after that push with no requested run in the gap, against under 10; the seven others a median 0.78 minutes after their later gate, against under 5; a passing Full took a median 16.11 minutes, against 15 or less, and 14.54 over merges 90 to 95; and no PR left the queue for a defect in its own code, against at most one. Quick is what a finished PR now waits on, so #1063's item 1 splits or reorders it, and #1026 closes when that item is delivered or dropped.
- #1077 (the C++ free functions that no code calls) stays open for its last ten baseline rows, which #1084 deletes, and for [one function the gate passed](https://github.com/fetch-laboratories/openttd-rust/issues/1077#issuecomment-7187265117).
- #1032 (the economy date as a type through `delete_stale_links`) rides the pass of workset 21 that finishes `timer_game_economy.rs` and `linkgraph/live.rs`.
- #940 holds the docs cleanup's follow-ups. Root's rulings are recorded there: a worktree is protected only by its whole folder name, and a deep run ends with its session. Its small fixes ride a tools batch where they share its files.
- #860 lists the development-process changes the user approved: the merge queue ([ADR 0019](adr/0019-merge-queue.md)), the per-merge builds ([ADR 0018](adr/0018-builds-before-every-merge.md)), CI scenario comparison ([ADR 0020](adr/0020-scenario-comparisons-on-ci.md)), direct reviewer and implementer messages, and the wind-down-only checkpoint ([ADR 0026](adr/0026-roadmap-commit-timing.md)).
- #896 (paid CI minutes) stays with its author for the one-day weighted-minutes and network-failure follow-up. The simulation shards are back on Blacksmith; which jobs run on paid runners is the user's decision ([ADR 0009](adr/0009-paid-runner-assignment.md)).

Excluded, with issue placement and return conditions read by steering 40 at merge 110 (69 open issues, none unplaced); near-full reproduction and fork release-source publication triggers are not met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Latest steering actions

Steering 40 is the latest, at merge 110. It timed every minute of the eleven implementers' slots in root's session of 2026-11-25, checked the 107 findings of its 34 review reports, compared #1066 and #1086 as merged with the original, read #1066's eleven reports round by round, and audited #1084 and root's 49 wakes. Its findings are in the message of the steering commit that follows `44e67df099`. Its changes are these.

- **Cases** (#1067). Two are new, each with a fathom agent in a thread of its own ([ADR 0053](adr/0053-a-fathom-takes-no-slot.md)): #1090, reviews of where `unsafe` sits, and #1091, a slot in trouble that reaches root only through the user. #1090's rule has landed (`24541493b1`, [where `unsafe` sits](porting.md#where-unsafe-sits)): the original's preconditions are a module's contract, and whether a function relying only on it is safe or `unsafe` is no finding. Its tool half, #1093, is in review as the fathom's own PR. #1091's agent is running. #1069 and #1071 are closed on their numbers. #1070 and #1072 hold so far and are read again. #1073 is sent back and not started. Root puts none of them in a tools batch and makes no rule for them.
- **A reviewer's model** is named in the fixed message that starts it, and the brief tells an implementer to name it in the `delegate_task` call ([spawning](team.md#spawning), [starting reviewers](implementer-brief.md#starting-reviewers)). Root's spawn message is the fixed message: the child's model and effort are in it, and root no longer adds "name model and effort to any reviewer you start", which two implementers read as their own model.
- **#1084 is reviewed whole**, by two reviewers and one re-check ([ADR 0051](adr/0051-review-whole-pr.md)). Root's porting groups are withdrawn ([steering's comment](https://github.com/fetch-laboratories/openttd-rust/pull/1084#issuecomment-7187321442)).
- **One `native` run a PR** is no longer provisional ([checks](workflow.md#checks)): 13 of 16 PRs kept to it over two sessions, and the one queue removal on a job `native` runs came from the merge itself.

Each of these is provisional and ends as it says:
- **Before a second scoped review after a re-check, the implementer messages root** (#1090). It ends when #1084 has merged with at most two scoped reviews after its re-check, the failure line of [#1090's number](https://github.com/fetch-laboratories/openttd-rust/issues/1090#issuecomment-7188306924); root then drops the line from its spawn messages. If #1084 passes two, the line stays and the case goes back. Root adds this line to each implementer's spawn message: "Before you start a second scoped review after your re-check, message root in one line: the PR, the reports so far and what the next review would read." Root answers within the turn: where the finding is not a difference the game can reach, the implementer records it in the PR body as residue for #734 and queues; otherwise it fixes and reviews once more.
- **Root reads `python3 tools/ci.py state PR` for each open PR at a wake it already has**, and starts no wake for it (#1091). It ends when #1091's fix lands, or sooner at the user's word. The read touches [ADR 0012](adr/0012-root-decides-implementers-review-and-merge.md), which is the user's, and the user has been asked whether it stays. It does not show a reviewer's model.

For root, on resume:
1. Start housekeeping with the routine agent. Rewrite the status comment on #3 to the current state.
2. Resume the two drafts from the [resume checkpoint](#resume-checkpoint), #1084 first, each with steering's comment on it ([#1084](https://github.com/fetch-laboratories/openttd-rust/pull/1084#issuecomment-7187321442), [#1089](https://github.com/fetch-laboratories/openttd-rust/pull/1089#issuecomment-7187321879)). #1089 takes the base that holds #1084 before its first review.
3. Dispatch from the [ready queue](#ready-queue) in its order, on two slots.
4. Run the platform cadence after merge 114 and take stock after merge 119. In the stocktake, write the last line of `issues.py` as it prints, and no sentence about the exclusions' return conditions unless each was read: the stocktakes at merges 95 and 107 both say they were read, and neither read one.
5. A fathom agent's commit may change a doc a running implementer follows. It sends root the commit; root fetches and applies it.

Provisional, for steering 41 to judge:
- **The cases.** Read #1070's numbers at the merge-119 stocktake and #1072's once 20 PRs have requested `native`. Read #1090's on #1084 first (scoped porting reviews after a re-check: more than two on a PR, or more than five on the next ten, is its failure; and any finding that asks to mark a function `unsafe` for a precondition the original has), and #1091's once its fix lands. The baseline is this session: of 1,559 slot-minutes editing took 31%, preflight 20.5%, builds 15% and base updates 10%.
- **Large passes.** Compare #1084 as merged with the original, and read each first reviewer's line of bodies compared against its roughly 418 changed functions. The void whole-PR pair found 0.48 findings in each 1,000 lines, where #1056's first stage found 1.09.
- **Reviewers' model.** Count the reports whose model is not the review row's. #939 has the gate that would refuse one.
- **Reviews of a conflicted base update.** Five over two sessions found nothing. At ten with none, the rule goes ([integration](workflow.md#integration)).
- **Ports.** No port has been reviewed under these rules; the first three of phase 3 are compared with the original ([steer](skills/steer/SKILL.md)).
- **Plan sizes.** The session's passes came in at 0.99 to 1.90 times their plans. The excess is `unsafe_code` records (#1090) and, on #1086, 2,635 lines of merged branches' record files that the pass deleted.
- **The implementer limit, effort and the service tier** are the user's ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md), [ADR 0011](adr/0011-model-and-effort-per-role.md)). With three or four implementers a full preflight took a median 387 seconds over 29 runs, against 265 for a run alone, and preflight was 27.5% of slot time against 14.5% with two.

## Ready queue

Undispatched work orders ([team](team.md#ready-queue-and-pipelining)). Each has its own issue, whose body holds its plan once a planner has written it. Root's status comment on #3 holds the queue's state between commits to this section.

Two slots ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)). Each slot works one chain, and a pass planned on an open PR's head is dispatched when that PR merges. Residue that passes left in files they did not finish is on #734, one comment for each pass since [merges 66 to 75](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6976329602) and [steering 40's list](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-7187322409); two small porting points from #1000 wait on `residue/1000-porting-points` ([record](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6992168925)).

In flight, both in the [resume checkpoint](#resume-checkpoint): #1084 (#1060's first part, `orders.rs` with `order_func.rs`) and #1089 (#1057's third part, `cargo_storage.rs`). They share eleven files.

Waiting on nothing but a slot:

1. **#1064's first part** (workset 15, NewGRF loading: `newgrf_class.rs`, `override_manager.rs`, `newgrf_properties.rs` and `newgrf_internal.rs`, about 2,400 changed lines): planned and reviewed, with [root's answers](https://github.com/fetch-laboratories/openttd-rust/issues/1064#issuecomment-7173604267). The workset (19 files, 8,694 lines) takes seven PRs, and its fourth part (`newgrf_spritegroup.rs` with the resolver frames) goes before #1062's parts 2 to 4.

Waiting on a merge:

2. **#1062's parts 2 to 4** (workset 16, NewGRF domains; `newgrf_vehicle.rs`, 2,141 lines, has its own pass): they follow #1064's fourth part and move the 26 `GRFFile` member slots of three finished definition files to the handle ([root's second answers](https://github.com/fetch-laboratories/openttd-rust/issues/1062#issuecomment-7154092829)).
3. **#1057's fourth part** (`cargo_payment.rs` with `cargomonitor.rs`) follows #1089. Its growth record for the `CargoSpec` member slots names #1058.
4. **#1060's parts 2 and 3** (`fleet.rs` with `vehiclelist.rs`, then `fleet/transactions.rs` with `fleet/pending.rs`) follow #1084; the third also follows #1089.

Root plans the next entry when a dispatch leaves a chain without one ([team](team.md#ready-queue-and-pipelining)).

The train passes take their sites of #999 (order destination reads at the original's observation point). #998 is the work order for one host-buildable crate below `openttd-kernels`, which removes the copies of three kernels ports that `openttd-strgen` keeps under [root's ruling](https://github.com/fetch-laboratories/openttd-rust/pull/995#issuecomment-6975789181); no second build-time consumer has appeared and it removes no fork C++, so it stays behind the passes, selected no later than strgen's port in phase 4. #997 (the native assertion service reports neither the Rust site nor its predicate) is on #892's checklist and rides the closure residue sweep.

Full phase-3 planning remains untriggered: most of the files still to pass are in no entry.

## Resume checkpoint

Paused at the user's request, 2026-11-26 about 07:50Z.

Root sent each child its stop and wrote this section from GitHub without waiting for their reports. Base is merge 110 (`b9b6de7c49`); its Quick and build passed, the merge queue is empty, no merge stop is in force and no rollback happened. Where GitHub is newer than this section, GitHub is right.

| Draft | Head | State | Next action | Running at the pause |
| --- | --- | --- | --- | --- |
| #1084 (#1060's first part, `orders.rs` with `order_func.rs`) | `2e47f18990` | Draft again, with merges 109 and 110 taken; preflight and Quick passed at the head. Its eight review reports are void: their reviewers ran on `claude-opus-5-5`, the implementer's own model, where the [role table](team.md#roles-and-models) gives review to `gpt-6.2-sol` at high ([root's comment](https://github.com/fetch-laboratories/openttd-rust/pull/1084#issuecomment-7185379159)). The fixes made for them stand | As [steering 40's comment](https://github.com/fetch-laboratories/openttd-rust/pull/1084#issuecomment-7187321442) says: one behaviour and one porting review of the whole PR, then one re-check as the last review, all on `gpt-6.2-sol` at high; no further `native`; then ready and the queue. It deletes #1077's last ten baseline rows | `native` run 45776538246 failed in native-environment setup after the pause; its cause is not read |
| #1089 (#1057's third part, `cargo_storage.rs`) | `42f8c20829` | Mid-port and unbuilt, pushed as a checkpoint: at the stop it was removing the C++ side (`cargopacket.h`, `station_service_ffi.h`, the `GetCargoTile` virtual and `src/tests/station_service.cpp`), and its last `cargo check` did not compile. Its status comment is the record. No review, preflight or CI | Finish the port and take the base that holds #1084 before its first review ([steering 40's comment](https://github.com/fetch-laboratories/openttd-rust/pull/1089#issuecomment-7187321879)); then preflight, `native` and both first reviews on `gpt-6.2-sol` at high | None |

With two slots, both drafts resume. Not started, and root's next actions after them:
- Dispatch #1064's first part to the next free slot, then the [ready queue](#ready-queue): #1062's parts 2 to 4 and the rest of worksets 04 and 05, each planned when its chain needs it.
- The platform cadence after merge 114 and the stocktake after merge 119.

Root's rulings of the session are on their issues and PRs: the order of the NewGRF chain and the owner of the kept `ResolverObject` classes (#1064); #1074's cost at its first draft (#1074); and the tools batch's deletions (#1085). Its rulings on #1084's porting groups are withdrawn ([latest steering actions](#latest-steering-actions)).

Agent: /root | Model: claude-fable-5-2 | Reasoning effort: high

The implementer limit is two ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)), and the [role table](team.md#roles-and-models) gives each role's model and effort.

`CI_MACOS_HEAVY` is unset and `CI_ON_DEMAND` is `true`. The hooks of #927 are installed in the shared clone (`python3 tools/git_hooks.py install`).

Flakes and limits carried forward:

- #693: the `network-protocol-loopback` stalls are the original's `TCPConnecter` race (#1016); the harness retries a session once on that signature and counts the retry in the run's report. The `content-protocol` evidence misses are still unexplained. Keep `.local/retained-network-693` and the linked artifacts.
- The profile driver exits 1 on its anchor identity guard until a quiet pass re-anchors (#275), and deletes `candidate-runtime/` after every run. A system library upgrade is enough to trip it: `libssh-4`'s on 2026-11-24 did, one day after #1024's pass ([speed](#speed)).
- A branch that forked before merge 83 runs the old apt action in a requested run and can still lose a job to GitHub's stalling mirror: rerun it.
- A child's last report can fail to reach root (#710): the last of the session of 2026-11-24 did. A `gpt-6.2-sol` reviewer can stop on a host approval prompt that only the user can answer, under a full-access runtime; its parent sees only silence (#710).
- An implementer started at medium effort signs `Reasoning effort: high`: the host does not report the effort to a child, so root's spawn message names the model and effort to sign.
- `Migration baseline validation` failed at merge 110 (run 45774564231) in its build job before compiling: apt could not fetch a package from `azure.archive.ubuntu.com`. That is a runner failure, not a code regression, and it was not rerun.
- The push hook refuses a push to `rust-migration` while the merge queue holds an entry (#1088).
- Metric scope completeness stays with #736's end-state inventory. No later phase is selected.

Retained state for housekeeping:

- `python3 tools/housekeeping.py --apply` ran at the session's start and with the platform cadence at merges 102, 107 and 109, and after this commit; `/` has 3.3T free.
- Keep each open draft's checkout: `t3-6fbecfdc-5be8177bc143ce7f` (#1084) and `t3-d09df7ab-894d4e88a62c4099` (`cargo_storage.rs`, #1057's third part). The merged PRs' checkouts and the pushed review, re-check and plan checkouts can go.
- A name below protects a worktree only when it is the whole folder name ([housekeeping](root-workflow.md#housekeeping)). The `t3-` names of eight hex digits in this list are whole folder names from an earlier host version.
- Keep the locked merge-107 profile checkout `.local/worktrees/root-profile-merge107` until the next profile is recorded; the merge-95 one is released. Preserve all current speed, deep and evidence anchors; the speed anchors live in the main checkout's `.local/speed-pass`, and `20261123-hardware-f80cbaf` there is the pass the current anchors come from. `.local/stocktake-107/` holds the merge-107 stocktake's counts and scripts until the merge-119 stocktake, and `.local/stocktake-95/` is released. `.local/steering-40/` holds steering 40's audits, which #1090 and #1091 cite, `.local/steering-39/` steering 39's, which #1070, #1072 and #1073 cite, and `.local/steering-38/` the findings-table definitions they use: keep all three until the cases close.
- Keep `t3-f6bd08df`: three unpushed docs commits from 2026-11-10. Keep `t3-f4cf7a0b` (one unique unpushed commit, `125ea2440a`).
- The rest is carried forward as preservation guidance from earlier checkpoints; steering 38 found each dirty tree it names still dirty and did not check the others.
  - Keep the planning checkout `t3-20fec9ce` with its private plan, and origin `preserve/housekeeping-foundation-6ab9c20e5d`. Preserve every checkout with uncommitted or unpushed work.
  - Keep `.local/worktrees/root-profile-125`, `.local/worktrees/root-profile-119` and `.local/worktrees/root-profile-113` with accepted reports, raw Callgrind and frozen runtimes. Keep `t3-a490295e` (profile 131, clean `305ac20c28`) with its HDD `.local` outputs. Profile 119 measures clean `4d3a1d1d41`, not integration 120.
  - Keep main `.local/integration-records/`, `.local/speed-pass/` anchors, frozen runtimes and the retained simulation evidence.
  - Leave dirty or staged trees untouched: `t3-03c1e2da`, `t3-1ddaadfd`, `t3-5575ebe4`, `t3-83082b90`, `t3-c68d914a`, `t3-e0361d8c`, `t3-3b491ddb`, locked #565 WIP `t3-431c84b4`, locked native evidence `t3-d78d5f8b`, `t3-32cf4edc` and `t3-e130d5a3`.
  - Keep protected `.local/worktrees/root-docs-72` and locked `.local/worktrees/profile-after*`. Do not push stale root-docs guidance over current main. Keep the pinned original unchanged.
