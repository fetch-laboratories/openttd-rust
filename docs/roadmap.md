# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place ([records](workflow.md#records)); `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands

Base is at merge 107 (`969478514d`). Root's session that began 2026-11-25 19:48Z merged 102 to 107: #1065 (the tools batch), #1056 (`company.rs`), #1080 (#1062's first part), #1081 (#1057's first part), #1066 (`disaster.rs`) and #1083 (#1011). Merges 98 to 101 are the fathom PRs #1075, #1076, #1078 and #1079, merged outside it. The figures below are root's stocktake at merge 107, with the outputs and scripts in `.local/stocktake-107/`; the next stocktake follows merge 119, and the platform cadence was run at merges 101, 102 and 107.

- **#734 has 173 of 318 workset files finished and 74,355 lines left**, by `docs/worksets.md` and the count #1065 made one command. The lines left fell by 7,801 since merge 97, all of it in the session's eight running hours ([who decides](../AGENTS.md#who-decides)): 975 an hour, against 789 in merges 90 to 96 and 947 in merges 84 to 89. The session ran four implementers for its first two hours at the user's allowance and two after. At 790 to 975 an hour the 74,355 lines are about 76 to 94 running hours of passes, before the 57 unfinished files in no workset (14,847 lines), closure and #893. Worksets 06, 07, 08 and 10 are finished; of group (c), 04 has four files left and 05 all six.
- **Fork C++ is at 269,073 lines**, 808 fewer than at merge 97, in 17 running hours of which nine held only the fathom PRs and docs (48 an hour; 101 an hour over the session's eight). Original C++ retired is 63,072 lines, 22.18%, 73 more.
- **The boundary:** 1,438 exports and 2,629 callback slots, 5 and 83 fewer than at merge 97. #735 is unchanged at 101 functions over seven parameters, 67 of them FFI exports.
- **Review:** the six PRs had 3, 5, 6, 3, 11 and 3 review reports. #1066's eleven are its two first reviews, its re-check, one base-update review and seven scoped porting reviews of fixes to `unsafe_code` items after the re-check, each of which found the next caller out; rounds 4 to 7 were in `company.rs` and `aircraft.rs`, and none found a difference the game can reach. #1056 had one such scoped review and #1080 two. From its re-check to its first queue entry #1066 took 143 minutes. Root did not see it: the implementer never stopped to wait, so it sent no report, and root read no open PR's state between wakes. Root now reads `python3 tools/ci.py state` for each open PR at every wake. Steering 40 has the counts; root opened and then closed #1082 for it, and makes no rule.
- **CI in merges 102 to 107:** Quick took 4.3 to 6.5 minutes as three parallel jobs (#1065). A PR entered the queue a median 6.1 minutes after its last push, #1065 itself 25.7 with its bootstrap Full. A passing Full took a median 14.85 minutes over the thirteen since merge 96. #1066 left the queue once, when its merge group no longer compiled against #1080, and its second Full was started again by two docs commits root pushed while it was queued, against [the push rule](workflow.md#change-types). No Full failed on a PR's own defect otherwise. The platform cadence was green at merges 101 and 102; at 101 GitHub cancelled both macOS jobs twice for hosted capacity and the paid fallback ran them ([record](https://github.com/fetch-laboratories/openttd-rust/issues/896#issuecomment-7173939526)).
- **Speed:** every play scenario runs fewer instructions than at merge 95 and generation 0.45% more, all under their anchors ([speed](#speed)).
- The user's allowance of more than two implementers ran from about 19:57Z to 20:00Z on 2026-11-25; the four started in it ran to their merges. The limit is two ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)).
- No rollback has happened since merge 32. Full phase-3 planning is not triggered, and near-full reproduction is not met. The exclusions' return conditions were read at this stocktake and none is met: 71 issues are open and none is unplaced.
- **Unmerged branches on origin:** 54. One has an open PR, the draft #1084, the only work in flight. 19 have a closed PR, kept as the [cleanup rule](root-workflow.md#cleanup) says. 34 have none: upstream's `master` and release branches, and fork branches kept for recovery, measurement and old stocktakes, among them `preserve/housekeeping-foundation-6ab9c20e5d` and `fathom/1082` (`52e69b22ae`, one docs commit of the closed #1082). `.local/stocktake-107/12-branches-prs.txt` lists them.

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

1. **Tools batch.** One per stocktake interval, dispatched within it and ranked by its items' measured cost ([selection](planning.md#selection)). #1063's batch merged as #1065 (merge 102). The batch of the interval that starts at merge 108 is #1085: #1077's uncalled C++ free functions, and a push guard for the base while the merge queue is not empty. New inventory visibility is not C++ deletion.
2. **The shared-service family:** shared services, accessors and string IDs that several files need. Merged: news and script events (#982), windows and sound (#1001), errors (#1005), console and chat (#1008), the string IDs (#995, #1006, #1033), the framerate service (#1020) and `BaseStation::xy` (#1028). Left: `Vehicle::Tick`'s dispatch (#1011, after the `ship_control.rs` pass) and the shared crate (#998, with strgen's port in phase 4). The family is closed at these two: root opens a new member only with the counts [selection](planning.md#selection) asks for. Until a member merges, a pass keeps the carriers it would replace and names the work order ([design faults](porting.md#design-faults)).
3. **#734/#735 file passes.** One PR per workset in the #734 manifest finishes every file it touches, applying the [code standards](porting.md#code-standards) to every line. Sizing, splits and single-file passes follow [sizing](planning.md#sizing); proof follows [cleanup proof](porting.md#cleanup-proof). Each pass reports its files' residue from `tools/code-standards-inventory.py`.

   Order, by fan-in (measured at `5af17025a3`). Delivered: the 03 remainder and its residue pass, 23's remainder (#884), 13 Map/terrain with 23's type-bearing files, except 23's `native_test_support.rs` and `native_test_support/movement.rs` (#973), and 09.
   - (c) 08, 07, 06, 10, 04, 05. Worksets 07 and 10 are finished (#1055, and #1054 with #1061). Of 08, `disaster.rs` is left (draft #1066), and of 06 `company.rs` (draft #1056). 04 is #1057, in four PRs, and 05 is #1060, in three.
   - (d) The rest by fan-in ([sizing](planning.md#sizing)), the files outside a workset that reference it, counted at merge 95 (`.local/stocktake-95/11-fanin-output.txt`): 16 (89, #1062), 21 (75), 15 (51, #1064), 18 (47), 17 (37), 11 (36), 14 (30), 12 (29). Workset 19 (2) goes last of them, and F's closure stays last of all. Steering 39 put this order back: root's merge-95 order counted only the finished files that reference each workset, which moved 18 ahead of 15 and 12 ahead of 14. A finished caller is edited whenever its callee's workset passes, so it ranks no workset over another; the callers the order spares are the unfinished ones, and they give this same order. Counted again at merge 107 (`.local/stocktake-107/11-fanin-output.txt`), the order holds: 16 at 96, 21 at 78, 15 at 52, 18 at 48, 17 at 39, 11 at 37, and 14 and 12 at 32. Root counts again at the merge-119 stocktake.

   The unfinished files over 1,500 lines, each with a single-file pass, are `orders.rs` (5,850), `company.rs` (3,901, #1056), `cargo_storage.rs` (2,892), `objects.rs` (2,674), `road_cmd.rs` (2,581), `newgrf_vehicle.rs` (2,141), `newgrf_loading.rs` (1,839), `fleet.rs` (1,790), `tunnelbridge_cmd.rs` (1,653), `landscape_generation.rs` (1,573) and `services.rs` (1,543, with F); `test_tables.rs` (2,375) is in no workset. #734 reports progress in lines finished as well as files.
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
- #1067 tracks the [fathom](skills/fathom/SKILL.md) cases, one checklist line for each: #1069 (preflight), #1070 (a new worktree's first build), #1072 (`native` failures before review), #1071 (per-pass speed counts) and #1073 (findings that could be settled before review). Steering reads each landed fix's number and closes its case or sends it back.
- #939 lists 35 rules that a script, hook or lint could hold instead of a doc. A candidate joins a [tools batch](planning.md#selection) when its rule was broken in the window just ended, with the count; [steering 37's counts](https://github.com/fetch-laboratories/openttd-rust/issues/939#issuecomment-7119259973) for merges 60 to 75 [steering 38's](https://github.com/fetch-laboratories/openttd-rust/issues/939#issuecomment-7143125295) for merges 76 to 89 and [steering 39's](https://github.com/fetch-laboratories/openttd-rust/issues/939#issuecomment-7158594211) for merges 90 to 96 are there, with the Linux GCC cache split by runner image from #1026's plan, which root left out of #1063: Full's median was 14.54 minutes over merges 90 to 95 without it. Root read it again at the merge-107 stocktake and dropped it: a passing Full took a median 14.85 minutes over the thirteen since merge 96.
- #1026's acceptance, as [root settled it](https://github.com/fetch-laboratories/openttd-rust/issues/1026#issuecomment-7125440240), read over merges 84 to 95: the five PRs whose last push needed no fresh gate entered the queue a median 10.35 minutes after that push with no requested run in the gap, against under 10; the seven others a median 0.78 minutes after their later gate, against under 5; a passing Full took a median 16.11 minutes, against 15 or less, and 14.54 over merges 90 to 95; and no PR left the queue for a defect in its own code, against at most one. Quick is what a finished PR now waits on, so #1063's item 1 splits or reorders it, and #1026 closes when that item is delivered or dropped.
- #1074 (the tables C++ fills for Rust, moved out of headers that core headers include) came from the fathom case #1070 and is a work order of its own, ranked by its measured cost: a median 377 of 575 objects compiled again after each port merge. It is being planned, and goes to a slot between passes, since it edits headers every pass's callers touch. #1077 (the 34 C++ free functions that no code calls, from #1073) is item 1 of the tools batch #1085. Until that merges, #1078's gate refuses a new uncalled free function, and a pass that edits a file holding one of the 34 deletes it there and drops its baseline row.
- #1032 (the economy date as a type through `delete_stale_links`) rides the pass of workset 21 that finishes `timer_game_economy.rs` and `linkgraph/live.rs`.
- #940 holds the docs cleanup's follow-ups. Root's rulings are recorded there: a worktree is protected only by its whole folder name, and a deep run ends with its session. Its small fixes ride a tools batch where they share its files.
- #860 lists the development-process changes the user approved: the merge queue ([ADR 0019](adr/0019-merge-queue.md)), the per-merge builds ([ADR 0018](adr/0018-builds-before-every-merge.md)), CI scenario comparison ([ADR 0020](adr/0020-scenario-comparisons-on-ci.md)), direct reviewer and implementer messages, and the wind-down-only checkpoint ([ADR 0026](adr/0026-roadmap-commit-timing.md)).
- #896 (paid CI minutes) stays with its author for the one-day weighted-minutes and network-failure follow-up. The simulation shards are back on Blacksmith; which jobs run on paid runners is the user's decision ([ADR 0009](adr/0009-paid-runner-assignment.md)).

Excluded, with issue placement and return conditions rechecked by steering 39 at merge 97 (72 open issues, none unplaced); near-full reproduction and fork release-source publication triggers are not met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Latest steering actions

Steering 39 is the latest, at merge 97. It measured where the slot time of root's session of 2026-11-24 went, checked the 127 findings of its 29 review reports against the code, compared #1056 with the original at its head, and audited the session's plans and root's rulings. Its findings are in the message of the steering commit that follows `34f8b00c2b`. It wrote no new rule: what needs a cause found is a [fathom](skills/fathom/SKILL.md) case, and its changes are these.

- **Cases, costliest first** (#1067): #1069, local preflight, 26% of slot time; #1070, a new worktree's first build; #1072, `native` failures before review; #1071, per-pass speed counts; #1073, findings that could be settled before review. Fathom agents are started for #1069 and #1070, in threads of their own, and land their own fixes ([ADR 0053](adr/0053-a-fathom-takes-no-slot.md)). Root puts none of the five in a tools batch and makes no rule for them meanwhile.
- **The ready queue** holds each chain's next entry, and root refills it when no entry could go to the slot that frees next ([team](team.md#ready-queue-and-pipelining)). The cap of three left a slot empty for 21 minutes while every entry waited on another PR.
- **Group (d)** of the [work order](#cleanup-work-order) is back in [sizing](planning.md#sizing)'s fan-in order.
- **`orders.rs` stays one PR** ([measurement on #1060](https://github.com/fetch-laboratories/openttd-rust/issues/1060#issuecomment-7158574265)), and sizing records that a pass of 15,000 changed lines left no more behind.

For root, on resume:
1. Start housekeeping with the routine agent, and with it the platform cadence due at merge 97. Rewrite the status comment on #3 to the current state.
2. Resume the three drafts from the [resume checkpoint](#resume-checkpoint), #1056 first. [Steering 39's comment on it](https://github.com/fetch-laboratories/openttd-rust/pull/1056#issuecomment-7158573708) says what blocks and what can follow: it gets no third re-check and no further `native` run.
3. #1065 takes its base update for #1068, then root's workflow review and the bootstrap Full. A fathom agent's PR may touch its files (`tools/preflight.py`, `tools/ci.py`, `tools/migration.py`); whichever merges second takes the base update.
4. Dispatch from the [ready queue](#ready-queue) in its order, on two slots unless the user allows a third.
5. At the merge-107 stocktake, read each exclusion's return condition and record the unmerged branches: the merge-95 stocktake's text claims the first and has neither.

Provisional, for steering 40 to judge:
- **The cases.** Read the number each landed fix names. The baseline is merges 90 to 96: of 1,618 slot-minutes editing took 28%, preflight 26%, builds 11% and speed counts 9%.
- **One `native` run.** Seven of eight PRs kept to it and no PR left the queue on a job `native` runs. At two such removals in one stocktake interval, root restores the second run for fixes that change a body's operations.
- **The re-check as the last review.** #1056 ran two re-checks and two `native` runs after its passing one, and planned a third re-check: its implementer applied the line that says when a re-check is due and not the next, which makes it the last. The other seven PRs kept to one. At a second PR it is a case.
- **Large passes.** #1056 at 15,141 changed lines left 0.33 departures in each 1,000, and its first reviewers found half as much for each line in the same four minutes. `orders.rs` is planned at 13,000 to 20,000: read its first-stage findings for each 1,000 lines and its fix rounds beside #1056's.
- **Ports.** No port has been reviewed under these rules; the first three of phase 3 are compared with the original ([steer](skills/steer/SKILL.md)).
- **Plan sizes.** Under steering 38's Size rule two merged passes came in at 1.27 and 0.86 times their plans and two open ones stand at 1.64 and 1.55. Count again over merges 97 to 107, with what the excess is made of: on #1056 it was tests (4,517 lines against 1,900 planned) and the cleanup records of an item-scoped `unsafe_code`.
- **Chains.** At three slots base updates took 105 slot-minutes on five PRs; two needed a scoped review, which found nothing, one PR left the queue on a conflict, and no accessor stayed duplicated. Count again when a third slot runs.
- **`ci.py report` for reviewers** (#1065). One reviewer session of 40 stopped on a host approval prompt, for 6.7 minutes, and the other 39 posted through the host's tools. Count the reviewers that stop on a prompt once every report goes through the command.
- **The implementer limit, effort and the service tier** are the user's ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md), [ADR 0011](adr/0011-model-and-effort-per-role.md)). The measurements are with the user: at three slots seven PRs and 25,940 changed lines merged in 9.6 hours with three more drafts open, and no session has run two slots on the current hardware to compare; plans written at high effort took 2.4 times the planner minutes and 2.5 times the output tokens of the one written at medium, with as many plan-review findings; and reviewers took 3 to 5 minutes on the priority tier, where waits on reviewers are 3% of slot time.

## Ready queue

Undispatched work orders ([team](team.md#ready-queue-and-pipelining)). Each has its own issue, whose body holds its plan once a planner has written it. Root's status comment on #3 holds the queue's state between commits to this section.

Two slots ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)); the session of 2026-11-24 ran three at the user's allowance, and its three drafts are in the [resume checkpoint](#resume-checkpoint), which says which two resume first. Each slot works one chain, and a pass planned on an open PR's head is dispatched when that PR merges. Residue that passes left in files they did not finish is on #734, one comment for each pass since [merges 66 to 75](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6976329602); two small porting points from #1000 wait on `residue/1000-porting-points` ([record](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6992168925)).

Waiting on nothing but a slot:

1. **#1062: workset 16 (NewGRF domains)**, 14 files and 7,489 lines, of which `newgrf_vehicle.rs` (2,141) has its own pass. It is first of group (d) by the merge-95 count and takes four PRs; the first (`newgrf.rs`, `newgrf_generic.rs` and seven small files, about 3,750 changed lines before the `GRFFile` handle [root added to it](https://github.com/fetch-laboratories/openttd-rust/issues/1062#issuecomment-7153888902)) is planned on `rust-migration` and reviewed twice, with [root's second answers](https://github.com/fetch-laboratories/openttd-rust/issues/1062#issuecomment-7154092829): parts 2 and 3 move the 26 `GRFFile` member slots of three finished definition files to the handle. Its first part merged as #1080 (merge 104); parts 2 to 4 follow #1064's fourth part ([root's answers on #1064](https://github.com/fetch-laboratories/openttd-rust/issues/1064#issuecomment-7173604267)). Its kept resolver frame names #1064, which root opened for workset 15 (NewGRF loading, 19 files and 8,694 lines), queued in group (d) after workset 21.

2. **#1064's first part** (workset 15, NewGRF loading: `newgrf_class.rs`, `override_manager.rs`, `newgrf_properties.rs` and `newgrf_internal.rs`, about 2,400 changed lines): planned and reviewed, with [root's answers](https://github.com/fetch-laboratories/openttd-rust/issues/1064#issuecomment-7173604267). The workset takes seven PRs. It follows #1062's first part in the NewGRF chain, and its fourth part (`newgrf_spritegroup.rs` with the resolver frames) goes before #1062's parts 2 to 4 where #1057's first part has merged by then.

Planned and reviewed, waiting on a merge:

3. **#1057's first part** (workset 04, cargo: `cargotype.rs` with `cargo_registry.rs`, about 2,900 changed lines): waits on #1056 and follows it in the company chain. The workset takes four PRs, then `cargo_flow.rs`, `cargo_storage.rs`, and `cargo_payment.rs` with `cargomonitor.rs`, one after another. Its growth record for the `CargoSpec` member slots names #1058.
4. **#1027's fourth part**, `disaster.rs`: dispatched at merge 96 and paused as a checkpoint ([resume checkpoint](#resume-checkpoint)). **#1011**, which moves `Vehicle::Tick`'s dispatch into Rust with the four controllers' accumulators, is planned and reviewed with no open question: about 950 changed lines for seven sites in seven finished files, about 85 net fork C++ lines and an estimated 100 to 280 million fewer instructions on Opus, which root accepts as its cost. It waits on #1056 and `disaster.rs` and goes ahead of `orders.rs` where both are ready.
5. **#1060's first part** (workset 05, orders and fleet: `orders.rs` with `order_func.rs`, about 15,000 changed lines in one PR by [root's answer](https://github.com/fetch-laboratories/openttd-rust/issues/1060#issuecomment-7151245362)): waits on #1056 and #1059, and takes the first slot free after both. The workset takes three PRs, then `fleet.rs` with `vehiclelist.rs`, and `fleet/transactions.rs` with `fleet/pending.rs`, which follows workset 04's `cargo_storage.rs`.

Merged in the session of 2026-11-25: #1065 (#1063's batch, merge 102), #1056 (`company.rs`, 103), #1080 (#1062's first part, 104), #1081 (#1057's first part, 105), #1066 (`disaster.rs`, 106), #1083 (#1011, 107) and #1086 (#1057's second part, `cargo_flow.rs`, 108). Running: #1060's first part (draft #1084), and #1074, dispatched at merge 108 with [root's answers](https://github.com/fetch-laboratories/openttd-rust/issues/1074#issuecomment-7181626846). Waiting on nothing but a slot, in this order: the tools batch #1085 (planned and reviewed, with [root's answers](https://github.com/fetch-laboratories/openttd-rust/issues/1085#issuecomment-7181661972)); #1057's third part (`cargo_storage.rs`, planned and reviewed twice with no open question, about 8,650 changed lines in one PR, which also removes the loading bounce in `station_service.rs` and writes two `cargo_payment.rs` functions; it edits eleven files #1084 edits, so the second to merge takes the base update); then #1064's first part. Root plans the next entry when a dispatch leaves a chain without one ([team](team.md#ready-queue-and-pipelining)). A plan written on #1056's head (#1057, #1060) is read again at the base its pass starts from.

The train passes take their sites of #999 (order destination reads at the original's observation point). #998 is the work order for one host-buildable crate below `openttd-kernels`, which removes the copies of three kernels ports that `openttd-strgen` keeps under [root's ruling](https://github.com/fetch-laboratories/openttd-rust/pull/995#issuecomment-6975789181); no second build-time consumer has appeared and it removes no fork C++, so it stays behind the passes, selected no later than strgen's port in phase 4. #997 (the native assertion service reports neither the Rust site nor its predicate) is on #892's checklist and rides the closure residue sweep.

Full phase-3 planning remains untriggered: most of the files still to pass are in no entry.

## Resume checkpoint

Paused at the user's request, 2026-11-25 about 06:35Z.

Root sent each child its stop and wrote this section from GitHub and the worktrees without waiting for their reports. Base is merge 96 (`d50ec9f6b0`) and green there; the merge queue is empty and no merge stop is in force. Where GitHub is newer than this section, GitHub is right.

Steering 39 brought the table below up to GitHub's state at merge 97 (#1056's size and finished profile, and #1066), and its actions are under [latest steering actions](#latest-steering-actions).

Merged since the pause, outside root's session and at the user's request: #1068, merge 97 (`3ee5207400`), through the queue's Full. It makes docs-only a rule about what reads a file, not its type ([change types](workflow.md#change-types)). #1065 now conflicts with base in `tools/test-ci.py` and takes a [base update](workflow.md#integration) before its preflight; #1056 still merges cleanly. The same day's docs commits `93b4b97bbe` to `6c9cd3d399` add the [fathom](skills/fathom/SKILL.md) skill with its tracker #1067, change what [steer](skills/steer/SKILL.md) and [navigate](skills/navigate/SKILL.md) do, and remove process-review.

| Draft | Head | State | Next action | Running at the pause |
| --- | --- | --- | --- | --- |
| #1056 (#1048, `company.rs`) | `cfc422f74c` | Draft again. Both first reviews and two re-checks are in; `native` and preflight passed at `1eccaba6cf` (run 45559506214). `cfc422f74c` fixes the second re-check's one finding (the second vehicle fetch in `cmd_change_service_int`), a checkpoint with no preflight. 15,141 changed lines against a plan of 9,250 | The attribution of the +0.32% (P1996) and +0.33% (P2016) rises in the PR body, with the body's export and function counts; preflight at the head; `ci.py ready --wait`; queue on Quick. No further re-check, though its status comment plans one, and no further `native`: the re-check is a PR's last review ([ADR 0025](adr/0025-two-review-stages.md)), and this PR has had two. It merges with base cleanly. [Steering 39's comment](https://github.com/fetch-laboratories/openttd-rust/pull/1056#issuecomment-7158573708) lists four minor departures it may fix in that push or record as residue | None: the instruction profile of `1eccaba6cf` finished, and its counts are in the status comment |
| #1065 (#1063, tools batch) | `c197510c67` | Draft again. The behaviour review's nine findings and the re-check's four remaining ones (1, 2, 6 and 10) are fixed and pushed; `native` passed at `958b5c4c38` (run 45558078917), where the split Quick took 4 minutes 51 seconds from its first job's start; preflight passed at `a28e0c85ae` and has not run on the last two commits | Preflight, a stamped push, `ci.py ready --wait`. No further re-check, for the same reason. Then root reviews the workflow, the routine agent runs the bootstrap Full on the head, whose passing conditions #1063's Evidence lists, and it queues | None |
| #1066 (#1027's fourth part, `disaster.rs`) | `2affda2fb8` | Draft, mid-port and unbuilt: 6,308 changed lines so far against a plan of 4,200. Its status comment is the record | Finish the port, then preflight, `native` and both first reviews | None |

With two slots, #1056 and #1065 resume first, since each is within one push of the queue and #1056 opens #1057, #1060 and #1011; `disaster.rs` takes the first slot either frees. With a third slot all three resume at once. Then the [ready queue](#ready-queue).

Root's rulings of the session are on their issues: one PR for `company.rs` and its nested service-interval observation moved to a Rust test (#1048, PR #1056); #1058 as the owner of `CargoSpec::array` (#1057); where `disaster.rs`'s clear test records (#1027); one PR for `orders.rs` (#1060); the `GRFFile` handle in workset 16's first part and #1064 as the owner of the resolver frames (#1062); and the tools batch's manifest and denominators (#1063).

Agent: /root | Model: claude-fable-5-2 | Reasoning effort: high

The implementer limit is two ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)), and the [role table](team.md#roles-and-models) gives each role's model and effort.

`CI_MACOS_HEAVY` is unset and `CI_ON_DEMAND` is `true`. The hooks of #927 are installed in the shared clone (`python3 tools/git_hooks.py install`).

Flakes and limits carried forward:

- #693: the `network-protocol-loopback` stalls are the original's `TCPConnecter` race (#1016); the harness retries a session once on that signature and counts the retry in the run's report. The `content-protocol` evidence misses are still unexplained. Keep `.local/retained-network-693` and the linked artifacts.
- The profile driver exits 1 on its anchor identity guard until a quiet pass re-anchors (#275), and deletes `candidate-runtime/` after every run. A system library upgrade is enough to trip it: `libssh-4`'s on 2026-11-24 did, one day after #1024's pass ([speed](#speed)).
- A branch that forked before merge 83 runs the old apt action in a requested run and can still lose a job to GitHub's stalling mirror: rerun it.
- A child's last report can fail to reach root (#710): the last of the session of 2026-11-24 did. A `gpt-6.2-sol` reviewer can stop on a host approval prompt that only the user can answer, under a full-access runtime; its parent sees only silence (#710).
- An implementer started at medium effort signs `Reasoning effort: high`: the host does not report the effort to a child, so root's spawn message names the model and effort to sign.
- Metric scope completeness stays with #736's end-state inventory. No later phase is selected.

Retained state for housekeeping:

- 97 worktrees stand. `python3 tools/housekeeping.py --apply` ran at the session's start (69 worktrees, 66.8G), with the platform cadence at merge 92 (16, 3.0G) and after this commit; `/` has 3.2T free.
- Keep each open draft's checkout: `t3-706094fc-8aa26582d6ba4ced` (`disaster.rs`, #1066). The merged #1056, #1059, #1061 and #1065 checkouts and the pushed review, re-check and plan checkouts can go.
- A name below protects a worktree only when it is the whole folder name ([housekeeping](root-workflow.md#housekeeping)). The `t3-` names of eight hex digits in this list are whole folder names from an earlier host version.
- Keep the locked merge-107 profile checkout `.local/worktrees/root-profile-merge107` until the next profile is recorded; the merge-95 one is released. Preserve all current speed, deep and evidence anchors; the speed anchors live in the main checkout's `.local/speed-pass`, and `20261123-hardware-f80cbaf` there is the pass the current anchors come from. `.local/stocktake-107/` holds the merge-107 stocktake's counts and scripts until the merge-119 stocktake, and `.local/stocktake-95/` is released. `.local/steering-39/` holds steering 39's audits, which the fathom cases cite, and `.local/steering-38/` the findings-table definitions they use: keep both until the cases close.
- Keep `t3-f6bd08df`: three unpushed docs commits from 2026-11-10. Keep `t3-f4cf7a0b` (one unique unpushed commit, `125ea2440a`).
- The rest is carried forward as preservation guidance from earlier checkpoints; steering 38 found each dirty tree it names still dirty and did not check the others.
  - Keep the planning checkout `t3-20fec9ce` with its private plan, and origin `preserve/housekeeping-foundation-6ab9c20e5d`. Preserve every checkout with uncommitted or unpushed work.
  - Keep `.local/worktrees/root-profile-125`, `.local/worktrees/root-profile-119` and `.local/worktrees/root-profile-113` with accepted reports, raw Callgrind and frozen runtimes. Keep `t3-a490295e` (profile 131, clean `305ac20c28`) with its HDD `.local` outputs. Profile 119 measures clean `4d3a1d1d41`, not integration 120.
  - Keep main `.local/integration-records/`, `.local/speed-pass/` anchors, frozen runtimes and the retained simulation evidence.
  - Leave dirty or staged trees untouched: `t3-03c1e2da`, `t3-1ddaadfd`, `t3-5575ebe4`, `t3-83082b90`, `t3-c68d914a`, `t3-e0361d8c`, `t3-3b491ddb`, locked #565 WIP `t3-431c84b4`, locked native evidence `t3-d78d5f8b`, `t3-32cf4edc` and `t3-e130d5a3`.
  - Keep protected `.local/worktrees/root-docs-72` and locked `.local/worktrees/profile-after*`. Do not push stale root-docs guidance over current main. Keep the pinned original unchanged.
