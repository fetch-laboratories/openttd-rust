# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place ([records](workflow.md#records)); `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands

Base is at merge 83 (`298e90e820`). The tables below are the merge-83 stocktake's (2026-11-24) beside merge 71 (`603724a3fd`), the stocktake before it; that interval took 21 running hours ([who decides](../AGENTS.md#who-decides)): 6 to merge 75, 6 of navigation, steering 37 and the hardware change, and 9 of the session of 2026-11-24, which merged 76 to 83. The census at merge 75 is in [phases](#phases). The next stocktake follows merge 95 and the next platform cadence merge 87.

- Fork C++ fell by 618 lines, 29 a running hour: 466 of source glue, 35 of original C++ retired and 129 generated, against 12 more under `tools/`. Rust grew by 4,159 lines and tooling by 3,381.
- **#734's lines left rose in merges 76 to 83.** They fell from 100,290 to 98,564 over the interval, 82 a running hour, but all of that fall came before merge 75 (98,024): since then they rose by 540, because one file finished (`vehicle.rs`, 462 lines) while unfinished files grew by what passes moved into them. Of the eight merges, two were the drafts carried over the pause (#1022, #1020), two the shared-service members steering 37 queued (#1028, #1033), one the speed anchors (#1024), two CI (#1030, #1034) and one a file pass (#1031). 143 of 319 files are finished, holding 110,401 lines, 57.9% of the 190,751-line denominator; the workset's files have grown to 208,965 lines. At 82 a running hour the 98,564 lines left are about 1,200 running hours of passes; that rate holds one file pass in eight merges and measures nothing of three slots of file passes, which the merge-95 stocktake measures. All three slots have run file passes since merge 83: #1035 and #1036 finish `vehicle/maintenance.rs` and `train.rs`, 4,609 lines, and worksets 07 and 08 hold a pass for each slot behind them ([ready queue](#ready-queue)).
- **Reviews took less than half the reports.** The six reviewed PRs of merges 77 to 83 had 2 to 4 review reports each, a median 3, against 7.8 for each PR in merges 66 to 75. The four PRs opened in the session merged 93, 103, 122 and 231 minutes after opening, against a median 3.0 hours. Root answered no disagreement between an implementer and a reviewer. Steering 38 audits what these reviews left.
- The boundary: 1,467 exports and 3,011 callback slots, 0 and 62 fewer. #735 is at 114 functions over seven parameters, 70 of them FFI exports.
- **#1026 is delivered** (#1034, merge 83), at the user's word ahead of the tools batch it was to lead. `enqueue` takes the newest passing requested run on any of the PR's own commits, so a fix that changes no operation, test or build file queues on Quick alone ([checks](workflow.md#checks)). Its planner traced most of Full's growth to GitHub's hosted runners installing from a mirror that stalls: installs over 2 minutes went from 0 of 562 in merges 48 to 59 to 155 of 566 in merges 66 to 75, and on 2026-11-24 four `native` jobs were lost to it before any build step. The apt fallback now changes host. The bootstrap Full on #1034 took 11.5 minutes against a median 21.4 for the Fulls of merges 76 to 80. Its acceptance is read over merges 84 to 95 ([trackers](#trackers-and-exclusions)).
- CI since the last stocktake: 18 requested `native` runs passed, 1 failed and 10 were cancelled by a moved head; 10 of 11 merge-queue runs passed. Both failures were on #1019, before merge 75: a suppression without its record and a `content-protocol` evidence miss (#693) in one, and the kernel-test crash that #1022 has since fixed in the other. No kernel-test crash recurred in merges 77 to 83, so #1012 is closed.
- The platform cadence was red at merge 77 and green at merges 80 and 82 (runs 45340425709 and 45356548608, no rerun and no fallback). The red was the hosted macOS image's newer CMake rejecting an argument in `cmake/PackageBundle.cmake`, a file identical to the pinned original's (#1029). That is no code regression, so no merge stopped; #1030 removed the argument.
- The user's allowances for the session of 2026-11-24, each ending with it: a third implementer slot; planners and implementers started after merge 82 at high effort; `gpt-6.2-sol` reviewers on the priority service tier.
- No rollback has happened since merge 32. Full phase-3 planning is not triggered, and near-full reproduction is not met.

Reproduce the census with `python3 tools/port-metrics.py --whole-fork` (metric version 3).

| Measured quantity | Merge 71 | Merge 83 |
| --- | ---: | ---: |
| Original C++ remaining | 221,612 | 221,577 |
| Fork source glue, including generated records | 31,701 | 31,235 |
| Compiled source and generated records C++ | 253,313 | 252,812 |
| Tracked tools C++ evidence | 14,477 | 14,489 |
| ABI-generated non-record C++ evidence | 5,168 | 5,039 |
| Measured fork C++ series | 272,958 | 272,340 |
| Original C++ retired, of 284,355 | 62,743 (22.07%) | 62,778 (22.08%) |
| Simulation C++ retired, of 61,148 | 41,461 (67.80%) | 41,487 (67.85%) |
| Historical regex exports / callback slots | 1,467 / 3,073 | 1,467 / 3,011 |
| Shared-accessor / rebound-callee / orphan-export baseline rows | 46 / 239 / 21 | 45 / 225 / 21 |
| Exports held only by uncalled members (baseline only shrinks) | 20 | 20 |
| Embedded C++ outside the census: oracle recipes | 302 | 302 |
| Rust / tooling code lines | 197,300 / 74,678 | 201,459 / 78,059 |

| Source area / phase relationship (merge 83) | Original | Retired | Remaining | Fork glue |
| --- | ---: | ---: | ---: | ---: |
| Simulation logic (phase 1) | 47,424 | 38,024 | 9,400 | 4,989 |
| Simulation storage (phase 2) | 13,724 | 3,463 | 10,261 | 1,380 |
| NewGRF, save/load, script API and network (phase 3) | 51,794 | 13,097 | 38,697 | 3,255 |
| Support, graphics and viewport (phase 4) | 32,602 | 1,073 | 31,529 | 539 |
| GUI and platform (phase 5) | 64,120 | 423 | 63,697 | 465 |
| Vendored (phase 6) | 55,148 | 2,317 | 52,831 | 0 |
| Data tables | 17,295 | 4,367 | 12,928 | 14 |
| Upstream tests and fork test glue | 2,248 | 14 | 2,234 | 4,857 |
| Boundary layer | 0 | 0 | 0 | 15,736 |

Coverage remains bounded by the existing saves, NewGRFs, scenarios and retained native fixtures; entry witnesses are not branch coverage. New gaps go on #156's lists, which are its sub-issues #1038, #1039, #1040, #1041, #1042 and #1043 since its body reached GitHub's length limit: this interval's passes added unbunching, road-stop, aircraft-fallback and train news paths that no scenario observes and that are source-reviewed only. #503 deep checks remain user-requested.

### Cleanup stream inventories

The comment, integer, wrapper, constant and literal rows are analyzer estimates from `tools/code-standards-inventory.py`. Their "in finished files" parts rise as files finish and are mostly rows a reviewer would dismiss, such as export parameters and raw adapters. Root adds the next column at its stocktake ([selection](planning.md#selection)).

| Stream | Inventory | Merge 71 | Merge 83 |
| --- | --- | ---: | ---: |
| #734 | Workset files with a finished pass, of 319 | 141 | 143 |
| #734 | Workset lines in finished files, of 190,751 | 105,163 (55.1%) | 110,401 (57.9%) |
| #734 | Lines left in unfinished workset files | 100,290 | 98,564 |
| #734 | Explanatory comment lines missing inside matched ported bodies | 2,033; 229 in finished files | 2,035; 237 |
| #734 | Bare-integer parameters and fields whose concept has a named type | 1,442; 339 in finished files | 1,397; 341 |
| #734 | Transparent wrapper types / named constants | 362 / 3,937; 97 / 1,267 in finished files | 367 / 4,047; 101 / 1,312 |
| #734 | Literal sites standing for a named value | 12; 0 in finished files | 12; 0 |
| #734 | #892's production assertion macro sites (`debug_assert*!` / `assert*!` / `unreachable!`) | 153 in 61 files (6 / 66 / 81) by the merge-83 scan; the merge-71 stocktake's own scan gave 157 in 62 files | 149 in 61 files (6 / 62 / 81) |
| #735 | Functions over seven parameters | 115 (70 FFI, 20 helpers, 25 commands) | 114 (70, 19, 25) |
| #735 | `too_many_arguments` ratchet rows / `allow` sites | 42 / 9 | 41 / 8 |
| #736 | Fork C++ evidence under `tools/` / `src/tests`, code lines | 14,477 / 4,853 | 14,489 / 4,857 |
| #738 | Rule corpus lines | 4,146 | 4,197 |
| None | Ruff rows | 283 | 283 |

Counting commands:

- **#734:** reconcile the issue workset manifest with merged complete-file pass PRs, then count current files with `wc -l`; a deleted finished entry remains in the denominator, and mechanical caller edits do not finish a file. The lines left are the `wc -l` of the workset files that `migration/finished-files.txt` does not list; their fall per running hour gives the stream's remainder. Run `python3 tools/code-standards-inventory.py [--head REV]` for the tree and with the finished paths for that subset. #892's sites have no tool: the scan covers `git ls-files 'rust/*.rs'` less test files, `cfg(test)` items, comments and literals, and each stocktake reruns it on the previous tip, since two scans' exclusions have differed.
- **#735:** use the tracked long-signature inventory counter, `migration/clippy-debt.json` (`too_many_arguments` keys) and `git grep 'allow(clippy::too_many_arguments' -- rust/`.
- **#736, the census baseline rows and the Clippy and Ruff rows:** from `python3 tools/port-metrics.py --whole-fork` (exports and callback slots are its "Boundary" line), `python3 tools/orphan-exports.py --report-uncalled`, `wc -l` of the three baseline files, and the debt files (keys are `sites` entries, diagnostics the sum of their list lengths). Expanded exports: `growth_inventory` in `tools/port-metrics.py`.
- **#738:** `cat AGENTS.md $(git ls-files 'docs/*.md' 'docs/**/*.md' | grep -v -E '^docs/(roadmap.md|components/)') | wc -l`.

### Speed

The merge-83 profile on clean `298e90e820` is the latest. All seven exit saves compared with no difference. The driver exited 1 on its anchor identity guard for every scenario: an unattended upgrade of the system's `libssh-4` on 2026-11-24 at 06:26Z changed the digest of one linked library, one day after #1024's quiet pass anchored the identity (#275). The library is not on the measured path, and the original's own counts moved by at most 0.04% against the pass, so these projections are computed from the report's instruction counts and the anchors by the [formula](speed.md#instruction-projections). The profile's output is retained in locked `.local/worktrees/root-profile-merge83`, the previous tip for attribution at the next profile; the merge-71 and merge-59 profile checkouts are released.

| Scenario | Original Ir | Candidate Ir (merge 83) | Change since merge 75 | Projection (merge 75, the anchor) | Projection (merge 83) |
| --- | ---: | ---: | ---: | ---: | ---: |
| TGP | 2,265,550,775 | 2,789,779,550 | +0.07% | 1.172488x | 1.174243x |
| Opus | 21,773,210,518 | 26,209,756,849 | +0.31% | 1.317818x | 1.325993x |
| Grok | 20,969,144,714 | 25,129,969,613 | +0.27% | 1.335597x | 1.342929x |
| P1996 | 3,600,042,113 | 4,427,082,350 | +0.05% | 1.319243x | 1.320627x |
| P2000 | 6,042,476,469 | 7,320,513,061 | −0.18% | 1.401373x | 1.401373x |
| P2016 | 11,345,807,488 | 13,548,302,804 | −0.40% | 1.343582x | 1.343582x |
| Drawing (diagnostic) | 2,149,057,585 | 2,154,445,719 | +0.01% | 1.004577x | 1.004738x |

No scenario rose by 3%, so nothing is attributed, and none is in its band: generation is at 1.174x against 1.235x and the highest play record, P2000, at 1.401x against 1.52x. Each row has one quiet pass on the current [hardware](rust-migration.md#hardware). #1031 (`vehicle.rs`) accounts for the play scenarios' rise: it measured +0.38% (Opus) and +0.33% (Grok) against its base. #1036 (`train.rs`) measures −0.41% and −0.44% and #1035 +0.08% and +0.07% against theirs. Until a quiet pass re-anchors, `simulate --instructions` exits 1 after recording its counts, and a pass compares raw candidate counts at base and head.

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

Slots: one [implementer slot](team.md#slots) always runs a #734/#735 file pass when one is ready, because the stage exits when the file passes do. The other takes, in this order: a defect that fails other PRs' runs, the interval's tools batch, a member of the shared-service family below, and otherwise a second file pass on a workset whose files do not overlap the first's. In merges 60 to 75 the six file passes had 22.8 of 56.8 slot-hours and made the whole 8,084-line fall in #734's lines left; the shared-service family had another 22.8, under which the lines left rose by 485. #738 is closed at its end state.

1. **Tools batch.** One per stocktake interval, dispatched within it and ranked by its items' measured cost ([selection](planning.md#selection)). #1013 merged as #1015 in the interval that started at merge 72, and #1026, which was to lead the next, merged on its own as #1034 at the user's word. The batch of the interval that starts at merge 84 is #1037: the PR template's "Review groups" section, #1023's dead offload code, a way for `housekeeping.py --apply` to leave one entry out (routine agents stopped before applying three times in four runs), #892's scan as a row of the inventory tool (three stocktakes wrote scans that disagree), the inventory at a pass's head as one command, and a check of field order in shared tables if its planner finds it smaller than the findings it replaces. None of them fails another PR's run, and #734's lines left rose in merges 76 to 83, so it ranks below the file passes while every slot has one ready. New inventory visibility is not C++ deletion.
2. **The shared-service family:** shared services, accessors and string IDs that several files need. Merged: news and script events (#982), windows and sound (#1001), errors (#1005), console and chat (#1008) and the string IDs (#995, #1006), 12,012 changed lines and −496 fork C++ in six PRs. Left: the framerate service (#1004, PR #1020 in review), `BaseStation::xy` (#1007), the remaining string-ID fields (#993), `Vehicle::Tick`'s dispatch (#1011, after the passes it waits on) and the shared crate (#998, with strgen's port in phase 4). The family is closed at these five: root opens a new member only with the counts [selection](planning.md#selection) asks for. Until a member merges, a pass keeps the carriers it would replace and names the work order ([design faults](porting.md#design-faults)).
3. **#734/#735 file passes.** One PR per workset in the #734 manifest finishes every file it touches, applying the [code standards](porting.md#code-standards) to every line. Sizing, splits and single-file passes follow [sizing](planning.md#sizing); proof follows [cleanup proof](porting.md#cleanup-proof). Each pass reports its files' residue from `tools/code-standards-inventory.py`.

   Order, by fan-in (measured at `5af17025a3`). Delivered: the 03 remainder and its residue pass, 23's remainder (#884), and 13 Map/terrain with 23's type-bearing files, except 23's `native_test_support.rs` and `native_test_support/movement.rs` (#973).
   - (c) 09 (59), 08 (58), 07 (52), 06 (45), 04 (38), 05 (27). Of 09, `rail_yapf.rs` (#976) and `ship_yapf.rs` (#977) are delivered; 08 is in the [ready queue](#ready-queue). The calls between 04 and 06 run both ways (06 to 04 in 2 files, 04 to 06 in 1), so strict fan-in governs.
   - (d) The rest by fan-in: 16 (67), 21 (62), 15 (42), 18 (37), 10 (33), 11 (28), 14 (27), 17 (26), 12 (25). The leaf worksets 22 (17), 20 (6) and 19 (1) go last. F's closure stays last.

   Twenty-four workset files exceed the single-file pass size; the largest are `station_cmd.rs` (4,868), `orders.rs` (4,841), `town.rs` (3,373), `train.rs` (3,121), `industry.rs` (2,874) and `company.rs` (2,698). #734 reports progress in lines finished as well as files.
4. **#736 test evidence.** No transfer remains. The aircraft range probe stays native under #656: #950 projected 10 to 13 changed lines for each net C++ line, above every delivered transfer, and the user approved closing it ([reason](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6860491029)). The classification table in #736's body names each remaining file's owner or boundary, as of 2026-11-14; that closes the stream's inventory without a PR ([ruling](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6834080910)).

   What stays native: `tools/migration/aircraft-range.cpp` with `finite_range_probe` (#656 retires it), the Catch2 files that delete whole (#931 closed, since its recipes outweighed the C++ it deleted), `src/tests/train.cpp` (its PBS and reservation cases retire with #725, its speed case with #656), and the ScriptList sections of `tools/migration/script-list-comparison.cpp`, which retire with that file when the script API reader takes the `ScriptList` adapter classes. The console fixtures and Catch2 files that reach live pools (65 files, 14,273 counted lines) retire inside the ports that retire their pools or readers. The 33 files and 3,909 lines of retained boundary evidence leave with their boundaries.
5. **#734 closure**, after the last workset pass: private wrapper fields, generator support, and removal of the raw adapters and aliases. Seventeen Rust files (2,944 lines; `test_tables.rs` is 1,735 of them) are in no workset. Each joins one before closure, the type files with F ([#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6767381933)). The end state and later stocktakes and steering measure with `tools/code-standards-inventory.py`. The closure residue sweep takes the recorded residue lists ([steering 34's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6789587702), [steering 35's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6834079997), [steering 36's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6847188216) and [steering 37's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-7119259564)) unless a later pass touches the file; its macro sites are #892's, and the three YAPF search-loop copies are #971's and stay. #892 rides remaining file passes, then that sweep and closure; no unrecorded production assertion macro site remains at closure, measured as #892 specifies.

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

- #156 (coverage gaps, whose lists are its sub-issues #1038, #1039, #1040, #1041, #1042 and #1043), #275 (speed), #724 and #710 (host) stay open trackers.
- #500 closes with #734's closure. #168 and #199 are closed: #565, #717 and #734 own what remained.
- #693: #1015's work traced the `network-protocol-loopback` stalls to a thread race in the original's `TCPConnecter`, which can stay in `Resolving` for ever when its resolver finishes first (#1016, a `deferred-improvement`: neither build changes). The harness bounds its waits and retries a session once on that signature while the candidate's copy of the file is identical to the pinned original's, counting each retry in the run's report; stocktakes read the rate there. The `content-protocol` evidence misses stay open on #693: that scenario runs without network debug output, so its logs cannot show the race. The allowance ends when #730 ports the connecter.
- #503's coverage continuation runs last.
- #1012 (one SIGSEGV in the kernel test binary on CI, 2026-11-18) is closed at the merge-83 stocktake: #1015's crash stacks placed the second crash as #1021's test race, #1022 fixed it at merge 77, and no kernel-test crash recurred in the runs of merges 77 to 83. A new crash opens a new issue with its stacks.
- #939 lists 35 rules that a script, hook or lint could hold instead of a doc. A candidate joins a [tools batch](planning.md#selection) when its rule was broken in the window just ended, with the count; [steering 37's counts](https://github.com/fetch-laboratories/openttd-rust/issues/939#issuecomment-7119259973) for merges 60 to 75 are there, with the Linux GCC cache split by runner image from #1026's plan, which root ranks at the merge-95 stocktake against Full's measured median and drops if Full is under 15 minutes without it.
- #1023 (dead hard-disk offload code in tools) is an item of #1037.
- #1026's acceptance is read at the merge-95 stocktake, over merges 84 to 95, as [root settled it](https://github.com/fetch-laboratories/openttd-rust/issues/1026#issuecomment-7125440240): for PRs whose last push needed no fresh `native`, bootstrap or reviewer, a median under 10 minutes from that push to queue entry with no requested run in it; for the others a median under 5 minutes from the later of that run's and that reviewer's end; a passing Full at a median of 15 minutes or less; and at most one PR removed from the queue for a defect in its own code. If Quick is then what a finished PR waits on, root opens a work order that splits it.
- #1032 (the economy date as a type through `delete_stale_links`) rides the pass of workset 21 that finishes `timer_game_economy.rs` and `linkgraph/live.rs`.
- #940 holds the docs cleanup's follow-ups. Root's rulings are recorded there: a worktree is protected only by its whole folder name, and a deep run ends with its session. Its small fixes ride a tools batch where they share its files.
- #860 lists the development-process changes the user approved: the merge queue ([ADR 0019](adr/0019-merge-queue.md)), the per-merge builds ([ADR 0018](adr/0018-builds-before-every-merge.md)), CI scenario comparison ([ADR 0020](adr/0020-scenario-comparisons-on-ci.md)), direct reviewer and implementer messages, and the wind-down-only checkpoint ([ADR 0026](adr/0026-roadmap-commit-timing.md)).
- #896 (paid CI minutes) stays with its author for the one-day weighted-minutes and network-failure follow-up. The simulation shards are back on Blacksmith; which jobs run on paid runners is the user's decision ([ADR 0009](adr/0009-paid-runner-assignment.md)).

Excluded, with issue placement and return conditions rechecked by steering 35 at merge 47; near-full reproduction and fork release-source publication triggers are not met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Latest steering actions

Steering 37 is the latest, at merge 75. It looked first at review and then surveyed merges 60 to 75; its findings are in the messages of the two steering commits that follow `aa7de6d8c7`. Its changes are rules now:

- A PR has two first reviewers whatever its size, one for behaviour and one for porting, and is not split into review groups. A PR with no game code has the behaviour reviewer alone ([review](workflow.md#review), [ADR 0051](adr/0051-review-whole-pr.md)).
- One re-check for the whole PR is the last review. No reviewer reads what the implementer fixes after it, and the queue no longer waits for every reviewer to accept.
- A finding that a rule, a ruling or the reviewed plan allows is answered in one line, not fixed. A point about text alone is not a finding ([review brief](review-brief.md#report)).
- Reads with no write, call or random draw between them may come in any order ([fidelity](porting.md#fidelity)).
- One slot always runs a file pass, and the shared-service family is closed at its five remaining members; a new cleanup work order of that kind needs counts ([work order](#cleanup-work-order), [selection](planning.md#selection)).
- An implementer starts no agent but its reviewers ([implementer brief](implementer-brief.md#following-the-plan)).

Of steering 36's provisional rules, plan review, rulings on recorded exceptions, the two kinds of reviewer and the tools batch stand. "For each group" is withdrawn, and so is root's rule that sent a fix made after the last stage to a fresh reviewer. Root's eleven other rule changes of merges 60 to 75 stand. The one that loosens fidelity-sensitive code, a tile's precondition proved once (#1019), was checked by two audits and holds.

For root, on resume:
1. Queue #1024 first. It is ready, needs no review and holds no slot; it records the quiet speed pass at merge 75.
2. #1020 and #1022 finish under the new review rules. #1020 has both reviews and their fixes: it gets one re-check of the fixing commits and then queues. #1022 gets its two reviews of the whole PR.
3. The first slot to free takes #984, the file pass that was ready and passed over three times on 2026-11-18. The other slot takes #1007 and then #993.
4. The plans already written list review groups. Ignore the lists; no plan is refreshed for that.
5. A PR body writes "Whole PR" under the template's "Review groups" heading until the next tools batch removes it. That batch takes #1026 first.

Provisional, for steering 38 to judge:
- **Whole-PR review.** Replayed on #1000 at its first reviewed head (5,806 changed lines), one behaviour and one porting reviewer found 17 and 23, in two runs, of the 28 serious and medium defects that the eight grouped reviewers found; on #996, 8 of 13. Both runs on #1000 also raised a design fault that none of the PR's 17 reports did and that is still in the merged file, the unbunching calls. Measure, per PR: reviewer sessions (7.9 in merges 66 to 75), review-fix commits and lines (13 and 889), and the implementer's active minutes on fixes (33). Audit the largest pass reviewed whole for what review left. Under every regime so far the audits found two to three minor departures in each 1,000 lines and no difference the game can reach: #976 (one reviewer for each group, 9 reports) 2.1, #988 (two for each group, 6 reports) 2.9, #1000 (two for each group, 17 reports) 2.6, and #1005 with #1019 (two for the whole PR, 3 reports each) 2.3 in each 1,000 changed lines.
- **The behaviour reviewer on a file pass.** On #1000, #996 and #1001 the behaviour reviewers found alone 13 of the 81 distinct first-stage defects, none of them serious; in the replays one added one or two medium findings to its porting partner's. On the tools PR #1015 it was the reverse: the behaviour reviewers alone would have found 22 of 28. No reviewer of either kind found a difference in game state in merges 60 to 75. Count each kind's own findings again: if the behaviour reviewer still adds this little, a file pass needs the porting reviewer only.
- **Fixes after the re-check.** Re-checks and later rounds found 64 defects in merges 60 to 75, 2 serious and 9 medium, all but 8 in lines a fix had just written or in the PR's text; the 26 reports after the re-checks found none serious. Count the fixes made after the re-check, and any defect that CI, a later pass or an audit finds in one.
- **Answered findings.** Ten findings were false, and at least four were fixed anyway (about 170 changed lines on #1000 and #996). Count findings answered against fixed, and the disagreements that reach root: root stepped in on review 12 times in merges 66 to 75.
- **Ports.** No port has been reviewed under these rules, and the rule that set review groups came from a port (#303). Before the first phase-3 port is dispatched, replay its review whole and in halves and compare.
- **Slot share.** Measure slot-hours by kind of work and the fall in #734's lines left for each running hour: 181 in merges 60 to 75, with file passes at 40% of slot time.
- **The wait after the last fix (#1026).** A median 56 minutes in merges 66 to 75, of which 19.5 is `native` at the final head and 22.0 the queue's Full.
- **Plan sizes.** Changed lines came out at a median 1.74 times the plan's estimate over 12 PRs (1.54 before), and #1015 at three times; a plan's Size does not yet predict a PR's.
- **The implementer limit** is the user's ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)). The measurements are with the user: the 6.4 hours at three on 2026-11-18 merged 2,128 changed lines an hour against 1,173 at two, on the old hardware, and one implementer also ran eight helpers of its own in that time.

## Ready queue

Undispatched work orders ([team](team.md#ready-queue-and-pipelining)). Each has its own issue, whose body holds its plan once a planner has written it. Root's status comment on #3 holds the queue's state between commits to this section.

Three slots are held, at the user's word of 2026-11-24 ("you can add a third implementer for the remainder of the session"); [ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)'s two return at the next resume. They hold #1035 (for #985, 7r-3, the pass for `vehicle/maintenance.rs`), past its first reviews; #1036 (for #1009, 8t, the single-file pass for `train.rs`), about to be reviewed; and #1027's first part (`effect.rs`), dispatched at merge 83 (`298e90e820`) when the stocktake's profile freed the slot. #1035 and #1036 share 19 files, so the second to merge takes a base update. Residue that passes left in files they did not finish is on #734: [merges 66 to 75](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6976329602), [#1020's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-7121869831), [#1031's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-7123409907) and [#1036's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-7128634649); two small porting points from #1000 wait on `residue/1000-porting-points` ([record](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6992168925)).

Worksets 07 and 08 run as three chains, one for each slot: `train.rs` and then #1010; `vehicle/maintenance.rs` and then #986, #987 and #1003; and #1027's four parts. Workset 06 (five files, 5,064 lines, of which `company.rs` is 3,897) gets its work orders when a chain has no entry for its slot.

1. #987: 7r-5, the pass for `vehicle/visits.rs`: about 2,300 changed lines and −90 fork C++. Planned. Waits on #1035's merge.
2. #986: 7r-4, the pass for `vehicle/scheduler.rs`: about 2,080 changed lines, with the day virtuals' dispatch moved into Rust. Planned. Waits on #1035's merge.
3. #1037: the tools batch of the interval that starts at merge 84 ([work order](#cleanup-work-order)); being planned at merge 83. Waits on nothing, and ranks below the file passes while every slot has one ready.

**Later.** #1027's parts 2 to 4 (`vehicle_effect.rs`, `ship_control.rs`, `disaster.rs`), each planned when the part before it has a PR ([root's answer](https://github.com/fetch-laboratories/openttd-rust/issues/1027#issuecomment-7122080934)); #1003 (`vehicle/occupancy.rs`, waiting on #986's PR, not yet planned) and #1010, 8tr, the single-file pass for `train_reservation.rs` (about 2,000 changed lines and −300; after #1009's PR, its plan refreshed and reviewed again at dispatch). Whichever of #1009 and #987 merges first moves the train case of `VehicleEnterDepot` into `vehicle/visits.rs`. #1011 moves `Vehicle::Tick`'s dispatch into Rust with the four controllers' accumulators; it waits on #986, #1009 and the `ship_control.rs` pass. After worksets 07 and 08 the fan-in order gives 06, 04 and 05. The train passes take their sites of #999 (order destination reads at the original's observation point).

#998 is the work order for one host-buildable crate below `openttd-kernels`, which removes the copies of three kernels ports that `openttd-strgen` keeps under [root's ruling](https://github.com/fetch-laboratories/openttd-rust/pull/995#issuecomment-6975789181). Root reconsidered it at this stocktake: no second build-time consumer has appeared and it removes no fork C++, so it stays behind the passes, selected no later than strgen's port in phase 4. #997 (the native assertion service reports neither the Rust site nor its predicate) is on #892's checklist and rides the closure residue sweep.

Full phase-3 planning remains untriggered: about 160 of #734's 319 files are in no entry.

## Resume checkpoint

**The session of 2026-11-24 is running; this section is rewritten at wind-down.** Root resumed at merge 75, merged eight PRs (merges 76 to 83) and took stock at merge 83. Between roadmap commits root's status comment on #3 holds the state: the open PRs, their heads and next actions, and the user's allowances for this session (three implementer slots, high effort for planners and implementers started after merge 82, the priority service tier for reviewers), each of which ends with the session. Where GitHub is newer than this section, GitHub is right.

Agent: /root | Model: claude-fable-5-2 | Reasoning effort: high

At the next resume the implementer limit is two ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)), and the [role table](team.md#roles-and-models) gives each role's model and effort again.

`CI_MACOS_HEAVY` is unset and `CI_ON_DEMAND` is `true`. The hooks of #927 are installed in the shared clone (`python3 tools/git_hooks.py install`).

Flakes and limits carried forward:

- #693: the `network-protocol-loopback` stalls are the original's `TCPConnecter` race (#1016); from merge 74 the harness retries a session once on that signature and counts the retry in the run's report. The `content-protocol` evidence misses (two on 2026-11-18, one reference side and one candidate side) are still unexplained. Keep `.local/retained-network-693` and the linked artifacts.
- Hosted macOS runners had no capacity in four runs on 2026-11-18 and 2026-11-19 and in none on 2026-11-24. `ci.py macos-fallback` has run only against a stubbed `gh`: after its first real use, check `gh variable list`.
- The profile driver exits 1 on its anchor identity guard until a quiet pass re-anchors (#275), and deletes `candidate-runtime/` after every run. A system library upgrade is enough to trip it: `libssh-4`'s on 2026-11-24 did, one day after #1024's pass ([speed](#speed)).
- GitHub's hosted Linux runners install from an EC2 mirror that stalls; #1034's fallback moves a stalled install to another host. A branch that forked before merge 83 runs the old action in a requested run and can still lose a job to the install step: rerun it.
- Root's status comment on #3 was the only carrier of state between roadmap commits; several child reports and one implementer's question did not reach root on 2026-11-18, and root read those results from the issues and PRs (#710).
- Metric scope completeness stays with #736's end-state inventory. No later phase is selected.

Retained state for housekeeping:

- `python3 tools/housekeeping.py --apply` ran once in the session of 2026-11-24, at its start (7.1G removed); two later dry runs listed only finished review checkouts and were not applied. `/` has 3.2T free on the new SSD.
- A name below protects a worktree only when it is the whole folder name ([housekeeping](root-workflow.md#housekeeping)). The `t3-` names of eight hex digits in this list are whole folder names from an earlier host version.
- Keep the locked merge-83 profile checkout `.local/worktrees/root-profile-merge83` until the next profile is recorded. Preserve all current speed, deep and evidence anchors; the speed anchors live in the main checkout's `.local/speed-pass`, and `20261123-hardware-f80cbaf` there is the pass the current anchors come from. `.local/stocktake-83/` holds the merge-83 stocktake's counts and scripts until the merge-95 stocktake.
- Keep `t3-f6bd08df`: three unpushed docs commits from 2026-11-10.
- Six older profile checkouts were released on 2026-11-12 (#924); this file no longer names them, so the next housekeeping run removes them.
- The list below is carried forward unchanged as preservation guidance from earlier checkpoints; it does not assert that every old checkout still exists. `/steer` may reconcile stale references, preserving uncommitted or unpushed work and evidence.

- Keep the planning checkout `t3-20fec9ce` with its private plan, and origin `preserve/housekeeping-foundation-6ab9c20e5d`. Preserve every checkout with uncommitted or unpushed work.
- Keep `.local/worktrees/root-profile-125`, `.local/worktrees/root-profile-119` and `.local/worktrees/root-profile-113` with accepted reports, raw Callgrind and frozen runtimes. Keep `t3-a490295e` (profile 131, clean `305ac20c28`) with its HDD `.local` outputs. Profile 119 measures clean `4d3a1d1d41`, not integration 120.
- Keep main `.local/integration-records/`, `.local/speed-pass/` anchors, frozen runtimes and the retained simulation evidence.
- Keep `t3-f4cf7a0b` (one unique unpushed commit, `125ea2440a`). Leave the staged 11-08 trees `t3-03c1e2da`, `t3-1ddaadfd`, `t3-5575ebe4`, `t3-83082b90`, `t3-c68d914a` and `t3-e0361d8c` untouched.
- Leave dirty or staged trees untouched: `t3-3b491ddb` (five files), locked #565 WIP `t3-431c84b4`, locked native evidence `t3-d78d5f8b`, `t3-32cf4edc` (`tools/simulation/roads.py` staged) and `t3-e130d5a3` (`migration/saves/README.md` staged).
- Keep protected `.local/worktrees/root-docs-72` and locked `.local/worktrees/profile-after*`. Do not push stale root-docs guidance over current main. Keep the pinned original unchanged.
