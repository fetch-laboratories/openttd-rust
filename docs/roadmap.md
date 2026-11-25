# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place ([records](workflow.md#records)); `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands

Base is at merge 89 (`17a38406ed`). The tables below are the merge-83 stocktake's (2026-11-24) beside merge 71 (`603724a3fd`), the stocktake before it, and the line under each gives what the 38th steering review measured at merge 89. Root's session of 2026-11-24 merged 76 to 89 and stopped at the user's request ([resume checkpoint](#resume-checkpoint)). The next stocktake follows merge 95 and the next platform cadence merge 92.

- **#734's lines left fell by 8,523 in merges 84 to 89**, from 98,564 to 90,041 in 8 running hours ([who decides](../AGENTS.md#who-decides)): 1,065 an hour, against 181 in merges 60 to 75 and 1,189 in merges 48 to 59. All six merges were file passes (`vehicle/maintenance.rs`, `effect.rs`, `train.rs`, `vehicle/visits.rs`, `train_reservation.rs` and `vehicle/scheduler.rs`), on three slots at the user's allowance. In merges 76 to 83 the lines left had risen by 540 over 9 running hours, with one file pass in eight merges. 149 of 319 files are finished. At 1,065 an hour the 90,041 lines are about 85 running hours of passes, and at the whole session's 499 about 180; six merges set the first rate, and the merge-95 stocktake measures it again.
- Fork C++ fell by 1,698 lines in merges 84 to 89, 212 a running hour, against 29 an hour over the merge-83 stocktake's interval.
- **Review under [ADR 0051](adr/0051-review-whole-pr.md) cost less than half and left no more behind.** The reviewed PRs of merges 77 to 89 had a mean 3.8 reviewer sessions against 7.9 in merges 66 to 75, review fixes of 14% of the lines first reviewed against 40%, and a median 132 active minutes from opening to merge against 182. Steering 38's audits compared all seven file passes of merges 82 to 89 with the original line by line: no difference the game can reach, and 0.7 to 2.1 minor departures in each 1,000 changed lines, where every earlier regime left 2.1 to 2.9. They are [on #734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-7143124114).
- The boundary: 1,454 exports and 2,817 callback slots at merge 89, 13 and 194 fewer than at merge 83. #735 is at 111 functions over seven parameters, 68 of them FFI exports.
- **#1026 is delivered** (#1034, merge 83): `enqueue` takes the newest passing requested run on any of the PR's own commits, and the apt fallback changes host when GitHub's mirror stalls. In merges 84 to 89 a PR merged a median 34 minutes after its last review report, against 63 before, and Full took a median 17.4 minutes against 22.0. Its acceptance is read over merges 84 to 95 ([trackers](#trackers-and-exclusions)).
- CI on 2026-11-24: four runs failed, two requested `native` runs (#1035 at its first head, #1036 after a review fix) and two platform builds at merge 77. No Full failed. One PR left the queue: #1036, on a conflict with #1044, which merged ahead of it.
- The platform cadence was red at merge 77 on the hosted macOS image's newer CMake (#1029, fixed by #1030; no code regression and no merge stop) and green at merges 80, 82 and 87. Merge 87's was the first real use of `ci.py macos-fallback`, which set and removed `CI_MACOS_HEAVY` as written.
- The user's allowances for the session of 2026-11-24 ended with it: a third implementer slot, planners and implementers at high effort, and `gpt-6.2-sol` reviewers on the priority service tier.
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

At merge 89 (`17a38406ed`): original C++ remaining 221,467; fork source glue 29,974; compiled C++ 251,441; tracked tools evidence 14,406; ABI-generated non-record evidence 4,795; measured fork C++ series 270,642; original C++ retired 62,888 (22.12%) and simulation C++ retired 41,597 (68.03%); exports and callback slots 1,454 and 2,817; shared-accessor, rebound-callee and orphan-export baseline rows 45, 217 and 21; Rust and tooling code lines 206,130 and 78,004.

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

At merge 89, by the same commands: 149 workset files finished, holding 124,081 lines; 90,041 lines left; 1,769 missing comment lines; 1,312 bare-integer sites; 373 wrappers and 4,090 named constants; 12 literal sites; 111 functions over seven parameters (68 FFI, 18 helpers, 25 commands); 41 `too_many_arguments` rows and 7 `allow` sites. The finished-file parts, #892's scan, the `src/tests` lines and the Ruff rows were not measured again.

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

No scenario rose by 3%, so nothing is attributed, and none is in its band: generation is at 1.174x against 1.235x and the highest play record, P2000, at 1.401x against 1.52x. Each row has one quiet pass on the current [hardware](rust-migration.md#hardware). The merged tip has not been profiled since merge 83, and the merge-95 stocktake profiles it. The passes since counted their own scenarios at base and head ([ADR 0052](adr/0052-speed-on-the-merged-branch-and-per-pass-counts.md)): #1031 +0.38% (Opus) and +0.33% (Grok), unexplained when it merged; #1035 +0.08% and +0.07%; #1036 −0.19% and −0.18% after its review fixes; #1050 +0.05% on P1996; and #1045 level with its base once a pool slot it looked up three times for each vehicle visit was read once. #1044 counted −0.46% on P2000 before its review fixes and was not counted again. The assertion-policy loads in the vehicle walks cost about 78 million instructions on Opus, 0.30% (#275). Until a quiet pass re-anchors, `simulate --instructions` exits 1 after recording its counts, and a pass compares raw candidate counts at base and head.

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

Slots: one [implementer slot](team.md#slots) always runs a #734/#735 file pass when one is ready, because the stage exits when the file passes do. With every slot on file passes the lines left fell by 1,065 a running hour in merges 84 to 89; with file passes at 43% of slot time they rose in merges 76 to 83. Another slot takes, in this order: a defect that fails other PRs' runs, a member of the shared-service family below, and otherwise a file pass of its own chain. Each slot works its own chain of passes, on worksets that do not call the other chains' ([sizing](planning.md#sizing)); the [ready queue](#ready-queue) names the chains. #738 is closed at its end state.

1. **Tools batch.** One per stocktake interval, dispatched within it and ranked by its items' measured cost ([selection](planning.md#selection)). The batch of the interval that starts at merge 84 is #1037, with a reviewed plan of about 700 changed lines, no game code and no workflow change: the PR template's "Review groups" section, #1023's dead offload code, `housekeeping.py --apply` made safe to run while reviewers work ([counts](https://github.com/fetch-laboratories/openttd-rust/issues/1037#issuecomment-7143125766)), #892's scan as a row of the inventory tool, the inventory at a pass's head as one command, and a check of field order in shared tables if its planner finds it smaller than the findings it replaces. It takes a slot before merge 95. New inventory visibility is not C++ deletion.
2. **The shared-service family:** shared services, accessors and string IDs that several files need. Merged: news and script events (#982), windows and sound (#1001), errors (#1005), console and chat (#1008), the string IDs (#995, #1006, #1033), the framerate service (#1020) and `BaseStation::xy` (#1028). Left: `Vehicle::Tick`'s dispatch (#1011, after the `ship_control.rs` pass) and the shared crate (#998, with strgen's port in phase 4). The family is closed at these two: root opens a new member only with the counts [selection](planning.md#selection) asks for. Until a member merges, a pass keeps the carriers it would replace and names the work order ([design faults](porting.md#design-faults)).
3. **#734/#735 file passes.** One PR per workset in the #734 manifest finishes every file it touches, applying the [code standards](porting.md#code-standards) to every line. Sizing, splits and single-file passes follow [sizing](planning.md#sizing); proof follows [cleanup proof](porting.md#cleanup-proof). Each pass reports its files' residue from `tools/code-standards-inventory.py`.

   Order, by fan-in (measured at `5af17025a3`). Delivered: the 03 remainder and its residue pass, 23's remainder (#884), 13 Map/terrain with 23's type-bearing files, except 23's `native_test_support.rs` and `native_test_support/movement.rs` (#973), and 09.
   - (c) 08 (58), 07 (52), 06 (45), 10 (33), 04 (38), 05 (27). Of 08, `vehicle_effect.rs` (#1051), `ship_control.rs` and `disaster.rs` are left, and of 07 `vehicle/occupancy.rs` (#1003). 06 has started (#1052, then #1048). Steering 38 moved 10 (rail, #1046: `rail.rs`, `rail_cmd.rs`, `rail_map.rs` and `track_func.rs`, 4,206 lines) ahead of 04 and 05: the finished train, reservation and pathfinder files call it, so their passes left it two one-call rail-type readers and 91 call sites of track helpers that drop an assertion, and it shares no file with workset 06. The calls between 04 and 06 run both ways (06 to 04 in 2 files, 04 to 06 in 1), so strict fan-in governs.
   - (d) The rest by fan-in: 16 (67), 21 (62), 15 (42), 18 (37), 11 (28), 14 (27), 17 (26), 12 (25). The leaf worksets 22 (17), 20 (6) and 19 (1) go last. F's closure stays last. Fan-in counts references, not their direction: at the merge-95 stocktake root counts, for each workset still to pass, the finished files that call it, and puts a workset that finished files call ahead of one they do not.

   The single-file passes still to come include `station_cmd.rs` (6,352 lines), `orders.rs` (5,879), `town.rs` (5,784), `company.rs` (3,910) and `industry.rs` (3,757). #734 reports progress in lines finished as well as files.
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

- #156 (coverage gaps, whose lists are its sub-issues #1038, #1039, #1040, #1041, #1042 and #1043), #275 (speed), #724 and #710 (host) stay open trackers. #710 has [the report lost at the pause of 2026-11-24](https://github.com/fetch-laboratories/openttd-rust/issues/710#issuecomment-7143126308).
- #500 closes with #734's closure. #168 and #199 are closed: #565, #717 and #734 own what remained.
- #693: #1015's work traced the `network-protocol-loopback` stalls to a thread race in the original's `TCPConnecter`, which can stay in `Resolving` for ever when its resolver finishes first (#1016, a `deferred-improvement`: neither build changes). The harness bounds its waits and retries a session once on that signature while the candidate's copy of the file is identical to the pinned original's, counting each retry in the run's report; stocktakes read the rate there. The `content-protocol` evidence misses stay open on #693: that scenario runs without network debug output, so its logs cannot show the race. The allowance ends when #730 ports the connecter.
- #503's coverage continuation runs last.
- #939 lists 35 rules that a script, hook or lint could hold instead of a doc. A candidate joins a [tools batch](planning.md#selection) when its rule was broken in the window just ended, with the count; [steering 37's counts](https://github.com/fetch-laboratories/openttd-rust/issues/939#issuecomment-7119259973) for merges 60 to 75 and [steering 38's](https://github.com/fetch-laboratories/openttd-rust/issues/939#issuecomment-7143125295) for merges 76 to 89 are there, with the Linux GCC cache split by runner image from #1026's plan, which root ranks at the merge-95 stocktake against Full's measured median and drops if Full is under 15 minutes without it.
- #1023 (dead hard-disk offload code in tools) is an item of #1037.
- #1026's acceptance is read at the merge-95 stocktake, over merges 84 to 95, as [root settled it](https://github.com/fetch-laboratories/openttd-rust/issues/1026#issuecomment-7125440240): for PRs whose last push needed no fresh `native`, bootstrap or reviewer, a median under 10 minutes from that push to queue entry with no requested run in it; for the others a median under 5 minutes from the later of that run's and that reviewer's end; a passing Full at a median of 15 minutes or less; and at most one PR removed from the queue for a defect in its own code. If Quick is then what a finished PR waits on, root opens a work order that splits it. Over merges 84 to 89 the second class entered the queue a median 0.75 minutes after its gate, Full took a median 17.4 minutes, and no PR left the queue for its own defect; no PR was in the first class, because every final push changed code or merged base and so ran `native` again. From steering 38 those pushes queue on Quick ([checks](workflow.md#checks)), which puts every PR in the first class.
- #1032 (the economy date as a type through `delete_stale_links`) rides the pass of workset 21 that finishes `timer_game_economy.rs` and `linkgraph/live.rs`.
- #940 holds the docs cleanup's follow-ups. Root's rulings are recorded there: a worktree is protected only by its whole folder name, and a deep run ends with its session. Its small fixes ride a tools batch where they share its files.
- #860 lists the development-process changes the user approved: the merge queue ([ADR 0019](adr/0019-merge-queue.md)), the per-merge builds ([ADR 0018](adr/0018-builds-before-every-merge.md)), CI scenario comparison ([ADR 0020](adr/0020-scenario-comparisons-on-ci.md)), direct reviewer and implementer messages, and the wind-down-only checkpoint ([ADR 0026](adr/0026-roadmap-commit-timing.md)).
- #896 (paid CI minutes) stays with its author for the one-day weighted-minutes and network-failure follow-up. The simulation shards are back on Blacksmith; which jobs run on paid runners is the user's decision ([ADR 0009](adr/0009-paid-runner-assignment.md)).

Excluded, with issue placement and return conditions rechecked by steering 38 at merge 89 (65 open issues, none unplaced); near-full reproduction and fork release-source publication triggers are not met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Latest steering actions

Steering 38 is the latest, at merge 89. It compared the seven file passes of merges 82 to 89 with the original, checked the 165 findings of the interval's review reports against the code, replayed #1036's review, and read root's session of 2026-11-24. Its findings are in the message of the steering commit that follows `17a38406ed`. Its changes are rules now:

- A PR has one `native` run, before its first review. Review fixes and base updates after it queue on Quick, and the queue's Full is their test ([checks](workflow.md#checks)).
- Wind-down waits on no child's report: root writes the checkpoint from GitHub, then does housekeeping ([wind-down](root-workflow.md#automatic-rollback-and-wind-down)).
- A plan names a test only with the fixture that produces it, and its Size counts tests, cleanup records and callers; a workset splits above about 5,000 changed lines ([planner brief](planner-brief.md#the-plan), [sizing](planning.md#sizing)).
- Concurrent passes run on worksets that do not call each other ([sizing](planning.md#sizing)), and after a base update a PR adopts the base's copy of anything both added ([integration](workflow.md#integration)).
- A pass on a hot path counts instructions at its base and head and explains a rise before review ([ADR 0052](adr/0052-speed-on-the-merged-branch-and-per-pass-counts.md), which replaces 0027 and records root's rule of 2026-11-24).
- A kept carrier may name the file pass that owns its replacement, where that pass's issue lists it ([design faults](porting.md#design-faults)); root ruled so on #1036 and #1047.
- A status comment holds the current state only ([records](workflow.md#records)).

Of steering 37's provisional rules, whole-PR review by two first reviewers, the one re-check as the last review, and unreviewed fixes after it stand ([where the fork stands](#where-the-fork-stands)). The behaviour reviewer stays on file passes: alone it raised 15 of the 79 distinct first-stage defects of eight passes and 9 of the 50 serious or medium ones, all of them evidence gaps or fatal paths the game cannot reach. Root's four other rule changes of the session stand: the bootstrap Full at the head, the background-job timeout, no stopping a job by a name pattern, and the speed attribution that ADR 0052 now records.

For root, on resume:
1. Start housekeeping with the routine agent as usual; wind-down's did not run and 126 worktrees stand. Rewrite the status comment on #3 to the current state.
2. Resume the two drafts from their status comments ([resume checkpoint](#resume-checkpoint)). #1051 has its `native` run, at `f15d991838`: after its fixes and its one re-check it queues on Quick.
3. Run the chains the [ready queue](#ready-queue) names, one for each slot. A third slot, if the user allows one, takes #1046.
4. Dispatch #1037 before merge 95.
5. The plans already written stand; none is refreshed for this steering.

Provisional, for steering 39 to judge:
- **One `native` run.** In merges 84 to 89 the nine `native` runs requested after a re-check all passed and no Full failed, at a median 15 minutes each before a Full of 17. Count the PRs that leave the queue on a failed job that `native` runs: at two in one stocktake interval, root restores the second run for fixes that change a body's operations.
- **Review of a large PR.** Replayed on #1036 at its first reviewed head (8,936 changed lines), the original pair found 23 of the 32 serious and medium defects that eight reviewers found between them; a second pair for the whole PR added 4, a pair for each half 7, and 7 of the eight reports' 52 distinct claims were false. None changes game state, the independent audit found a different seven, and the reviewers did not find fewer in the second half of the file. Extra reviewers keep finding medium porting departures at about two a session; the rule stays two reviewers whatever the size. Audit the first pass over 10,000 changed lines the same way.
- **Ports.** No port has been reviewed under these rules. Before the first phase-3 port is dispatched, replay its review whole and in halves and compare.
- **Plan sizes and evidence.** Changed lines came out at a median 1.82 times the plan's estimate over 12 PRs, and 22 first-stage findings said the code fell short of the plan, most of them tests; root cut planned tests from three plans. Measure both again under the new Size and Evidence rules.
- **Plan review.** Four first-stage findings faulted a plan decision that had passed plan review, two of them serious: each said no Rust form of a callee existed when one did (#1049, #1051).
- **Chains.** Four of the six passes of merges 84 to 89 needed a reviewed base update, and the file passes of the session 1.14 each, against 0.50 for each PR in merges 60 to 75; the six conflict reviews found nothing, and three of #1045's five leftovers came from one such merge. Count reviewed base updates for each pass, and the duplicates left at them.
- **Fixes that leave the code no better.** The audits found seven in seven passes: a change made and reverted, a second definition added to restore an assertion, a layer that does the same loads. Count them again.
- **Per-pass speed counts.** Three of five measured passes merged with an unexplained rise before the rule; count rises explained before review, and what the base and head counts cost a pass.
- **The implementer limit, effort and the service tier** are the user's ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md), [ADR 0011](adr/0011-model-and-effort-per-role.md)). The measurements are with the user: at three slots nine PRs and 35,942 changed lines merged in 9.9 hours, with the conflicts counted above; the three passes built at medium effort and the four at high left 0.7 to 1.3 and 1.2 to 2.1 departures in each 1,000 changed lines; and a first review took a median 4.7 minutes across both tiers, a re-check 3.6 on the default tier and 2.4 on priority.

## Ready queue

Undispatched work orders ([team](team.md#ready-queue-and-pipelining)). Each has its own issue, whose body holds its plan once a planner has written it. Root's status comment on #3 holds the queue's state between commits to this section.

Three slots for the session that began on 2026-11-24 at about 21:00Z, at the user's allowance; the standing limit is two ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)). At merge 93 (`fe9e973700`) they are held by #1055 (#1003, `vehicle/occupancy.rs`, in review), draft #1056 (#1048, `company.rs`) and #1027's third part (`ship_control.rs`, dispatched at merge 93). Each slot works one chain, and a pass planned on an open PR's head is dispatched when that PR merges. Residue that passes left in files they did not finish is on #734, one comment for each pass since [merges 66 to 75](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6976329602); two small porting points from #1000 wait on `residue/1000-porting-points` ([record](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6992168925)).

1. **#1046's second part**, the single-file pass for `rail_cmd.rs` (2,856 lines): part 1 merged as #1054 (merge 93). Its planner runs on `rust-migration`; it waits on nothing and takes the next slot to free. It retires #1054's callback-slot growth record.
2. **#1057: workset 04 (cargo)**, six files and 6,405 lines, of which `cargo_storage.rs` (2,892) has its own pass. It takes four PRs in the order `cargotype.rs` with `cargo_registry.rs`, `cargo_flow.rs`, `cargo_storage.rs`, then `cargo_payment.rs` with `cargomonitor.rs`. Part 1 (about 2,900 changed lines) is planned and reviewed on #1056's head, waits on #1056's merge and follows it in the company chain. Its growth record for the `CargoSpec` member slots names #1058, which owns `CargoSpec::array` in phase 2 with the smaller records of #638 and #498. Workset 05 (orders and fleet, with `orders.rs` at 5,879 lines) comes after it by the [work order](#cleanup-work-order).
3. **#1027's fourth part**, `disaster.rs` (1,446 lines): planned when part 3 has a PR; it waits on that PR's merge. #1011, which moves `Vehicle::Tick`'s dispatch into Rust with the four controllers' accumulators, also waits on part 3.

Workset 06 finishes with #1056, and workset 10 with #1046's second part. The next tools batch is picked at the merge-95 stocktake; #1037's merged as #1053 (merge 90).

The train passes take their sites of #999 (order destination reads at the original's observation point). #998 is the work order for one host-buildable crate below `openttd-kernels`, which removes the copies of three kernels ports that `openttd-strgen` keeps under [root's ruling](https://github.com/fetch-laboratories/openttd-rust/pull/995#issuecomment-6975789181); no second build-time consumer has appeared and it removes no fork C++, so it stays behind the passes, selected no later than strgen's port in phase 4. #997 (the native assertion service reports neither the Rust site nor its predicate) is on #892's checklist and rides the closure residue sweep.

Full phase-3 planning remains untriggered: most of the files still to pass are in no entry.

## Resume checkpoint

Paused at the user's request, 2026-11-24 about 17:20Z.

Root's wind-down stopped before its roadmap commit and housekeeping (#710); steering 38 wrote this section the same day from GitHub and the worktrees. No job is running, the merge queue is empty, and #1045 merged as merge 89 during the wind-down. Where GitHub is newer than this section, GitHub is right.

| Draft | Head | State | Next action |
| --- | --- | --- | --- |
| #1051 (#1027's second part, `vehicle_effect.rs`) | `ed9cea7654` | Both first reviews are in at `f15d991838`, where its `native` run passed after the pause (run 45444973730). The behaviour findings and porting findings 1, 2, 6 and 7 are fixed; the fix of 1 and 7 is checked in Rust only, with the C++ side and the ABI probe not built (`road.inc` changed). Porting findings 3, 4 and 5 are open | Fix 3 to 5; build, run the ABI probe and preflight; count instructions again at base and head (its base Callgrind output is in a scratchpad that a reboot clears); one re-check; queue on Quick |
| #1052 (#1047, 6a) | `390a26f973` | Clean; `verify` passed. No preflight, `native`, speed count or review. 34 cleanup records await the porting reviewer. 4,149 changed lines against a plan of 1,950, under the split size | `verify`, preflight, the speed count on both scenarios, the PR body, `native`, both reviewers |

Agent: /root (Claude Code steering review) | Model: claude-fable-5-2 | Reasoning effort: xhigh

The implementer limit is two ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)), and the [role table](team.md#roles-and-models) gives each role's model and effort.

`CI_MACOS_HEAVY` is unset and `CI_ON_DEMAND` is `true`. The hooks of #927 are installed in the shared clone (`python3 tools/git_hooks.py install`).

Flakes and limits carried forward:

- #693: the `network-protocol-loopback` stalls are the original's `TCPConnecter` race (#1016); the harness retries a session once on that signature and counts the retry in the run's report. The `content-protocol` evidence misses are still unexplained. Keep `.local/retained-network-693` and the linked artifacts.
- The profile driver exits 1 on its anchor identity guard until a quiet pass re-anchors (#275), and deletes `candidate-runtime/` after every run. A system library upgrade is enough to trip it: `libssh-4`'s on 2026-11-24 did, one day after #1024's pass ([speed](#speed)).
- A branch that forked before merge 83 runs the old apt action in a requested run and can still lose a job to GitHub's stalling mirror: rerun it.
- A child's last report can fail to reach root (#710): the last of the session of 2026-11-24 did.
- Metric scope completeness stays with #736's end-state inventory. No later phase is selected.

Retained state for housekeeping:

- 126 worktrees stand. `python3 tools/housekeeping.py --apply` last ran at merge 77; the merge-82 and merge-87 runs and wind-down's did not apply, and about 54G is removable. `/` has 3.2T free.
- Keep each open PR's checkout: `t3-c469d4ba-257ed73890023db5` (#1051) and `t3-07730374-7e8241eb876e496d` (#1052, with a scratch `.local/speed-base` inside it that holds no work). #1045's `t3-fb06aeac-6339778ecfcd170e` is merged and can go, and so can `t3-84c5d01a-0a453c83ab85d5ad`, whose one unpushed commit is a local planning merge of #1045's head.
- A name below protects a worktree only when it is the whole folder name ([housekeeping](root-workflow.md#housekeeping)). The `t3-` names of eight hex digits in this list are whole folder names from an earlier host version.
- Keep the locked merge-83 profile checkout `.local/worktrees/root-profile-merge83` until the next profile is recorded. Preserve all current speed, deep and evidence anchors; the speed anchors live in the main checkout's `.local/speed-pass`, and `20261123-hardware-f80cbaf` there is the pass the current anchors come from. `.local/stocktake-83/` holds the merge-83 stocktake's counts and scripts until the merge-95 stocktake, and `.local/steering-38/` the audits and replays of steering 38 until steering 39.
- Keep `t3-f6bd08df`: three unpushed docs commits from 2026-11-10. Keep `t3-f4cf7a0b` (one unique unpushed commit, `125ea2440a`).
- The rest is carried forward as preservation guidance from earlier checkpoints; steering 38 found each dirty tree it names still dirty and did not check the others.
  - Keep the planning checkout `t3-20fec9ce` with its private plan, and origin `preserve/housekeeping-foundation-6ab9c20e5d`. Preserve every checkout with uncommitted or unpushed work.
  - Keep `.local/worktrees/root-profile-125`, `.local/worktrees/root-profile-119` and `.local/worktrees/root-profile-113` with accepted reports, raw Callgrind and frozen runtimes. Keep `t3-a490295e` (profile 131, clean `305ac20c28`) with its HDD `.local` outputs. Profile 119 measures clean `4d3a1d1d41`, not integration 120.
  - Keep main `.local/integration-records/`, `.local/speed-pass/` anchors, frozen runtimes and the retained simulation evidence.
  - Leave dirty or staged trees untouched: `t3-03c1e2da`, `t3-1ddaadfd`, `t3-5575ebe4`, `t3-83082b90`, `t3-c68d914a`, `t3-e0361d8c`, `t3-3b491ddb`, locked #565 WIP `t3-431c84b4`, locked native evidence `t3-d78d5f8b`, `t3-32cf4edc` and `t3-e130d5a3`.
  - Keep protected `.local/worktrees/root-docs-72` and locked `.local/worktrees/profile-after*`. Do not push stale root-docs guidance over current main. Keep the pinned original unchanged.
