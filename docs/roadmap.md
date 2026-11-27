# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place ([records](workflow.md#records)); `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands

Base is at merge 119 (`0f05ba2ef1`, #1098), root's stocktake in the session that began 2026-11-26 19:43Z. Its outputs and scripts are in `.local/stocktake-119/`. The interval is merges 108 to 119, 18 running hours ([who decides](../AGENTS.md#who-decides)), of which five (08Z to 11Z and 18Z on 2026-11-26) had no implementer: the fathom fixes #1092 and #1093 and #896's #1094 landed in them. The next stocktake follows merge 131, and the platform cadence was green at merges 114 and 119.

- **#734 has 186 of 318 workset files finished and 61,257 lines left**, from 74,355 at merge 107 and 73,029 at merge 110: 13,098 fewer in the interval, 728 a running hour, and about 1,008 in the thirteen hours an implementer ran, against 754 in the session before, 789 in merges 90 to 96 and 947 in merges 84 to 89. At 728 to 1,008 an hour the 61,257 lines are about 61 to 84 running hours of passes, before the 57 unfinished files in no workset (14,814 lines), closure and #893. Worksets 04, 06, 07, 08 and 10 are finished; 05 has four files left, two of them in #1099, and 15 has twelve of nineteen.
- **Fork C++ is at 267,818 lines**, 1,255 fewer than at merge 107, 70 a running hour. Original C++ retired is 63,336 lines, 22.27%. The boundary has 2,548 callback slots, 81 fewer. Its textual export count rose from 1,438 to 1,458 at one merge, #1096, which wrote out by hand the class exports a macro had generated, since lint policy refuses an `unsafe_code` allow inside a macro body; the expanded count (`growth_inventory`) fell from 1,582 to 1,567. #735 is at 91 functions over seven parameters, from 101, and #892 has 89 run-time assertion macro sites in 41 files, from 119.
- **The session's passes** (merges 115 to 119) are #1084 (`orders.rs` with `order_func.rs`, 17,066 changed lines), #1089 (`cargo_storage.rs`, 9,253), #1096 (#1064's first part, 4,039 against a plan of 2,400), #1097 (`cargo_payment.rs` with `cargomonitor.rs`, 2,836 against 3,900) and #1098 (#1064's second part, 4,902 against 3,700). The three dispatched in the session took 114, 63 and 77 minutes from the PR's opening to its merge.
- **Review:** #1084 was reviewed whole by two reviewers and one re-check on `gpt-6.2-sol`, after its eight void reports; its behaviour review found `gb_u16` returning 0 for a full 16-bit read, fixed before merge. #1096 took three scoped records reviews, the third on root's answer to `ci.py reviewer` (#1091's line reached root as built); each round's findings were one registry resolved more than once in a body, not where `unsafe` sits, and each fix voided the acceptance of the items it changed. It had two scoped reviews after its re-check, which is #1090's limit and not over it. Root's dispatches since carry that fact, and #1097 and #1098 took three reports each.
- **A slot in trouble** reached root three times, each by the implementer's own message: #1089 before its reviews (the two drafts merged without a textual conflict and did not compile together, so it waited for #1084), #1084 at `enqueue` (#1100's second item) and #1096's third scoped review.
- **CI:** a passing queue Full took a median 12.51 minutes over the interval's twelve, and 11.00 over merges 115 to 119, after #1095 kept apt's downloads and had the queue's run save the compiler caches. Each covered push since found its commit's entries, built nothing on Windows and ran its Linux build job in 35 to 41 seconds ([#896's record](https://github.com/fetch-laboratories/openttd-rust/issues/896#issuecomment-7207548059)). Three merges left the queue once, none for its own code: #1092 on an apt stall of 30 minutes, #1095 on a `content-protocol` evidence miss (#693) and #1089 on a GitHub server error in the annotation checks (#1100's first item).
- **Speed:** every play scenario runs fewer instructions than at merge 107 and generation 0.11% more, all under their anchors ([speed](#speed)).
- The limit is two implementers ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)). The merge-119 profile held one slot for 37 minutes.
- No rollback has happened since merge 32. Full phase-3 planning is not triggered, and near-full reproduction is not met. `python3 docs/skills/steer/issues.py` ends `69 open: 1 unplaced, 1 excluded, 0 roadmap exclusions without an issue`; the unplaced one is #1100, which this stocktake opened and [places](#cleanup-work-order).
- **Unmerged branches on origin:** 55 at merge 119, listed in `.local/stocktake-119/12-branches-prs.txt`; #1099 is the one with an open PR. Among the branches kept with no PR are `preserve/housekeeping-foundation-6ab9c20e5d` and `fathom/1082` (`52e69b22ae`, the one docs commit of the closed #1082, which #1090 reads).

Reproduce the census with `python3 tools/port-metrics.py --whole-fork` (metric version 3).

| Measured quantity | Merge 107 | Merge 119 | Change per running hour (18) |
| --- | ---: | ---: | ---: |
| Original C++ remaining | 221,283 | 221,019 | −15 |
| Fork source glue, including generated records | 28,842 | 27,975 | −48 |
| Compiled source and generated records C++ | 250,125 | 248,994 | −63 |
| Tracked tools C++ evidence | 14,399 | 14,395 | −0.2 |
| ABI-generated non-record C++ evidence | 4,549 | 4,429 | −7 |
| Measured fork C++ series | 269,073 | 267,818 | −70 |
| Original C++ retired, of 284,355 | 63,072 (22.18%) | 63,336 (22.27%) | +15 |
| Simulation C++ retired, of 61,148 | 41,770 (68.31%) | 41,960 (68.62%) | +11 |
| Historical regex exports / callback slots | 1,438 / 2,629 | 1,458 / 2,548 | +1 / −4.5 |
| Shared-accessor / rebound-callee / orphan-export baseline rows | 42 / 192 / 17 | 27 / 181 / 17 | |
| Exports held only by uncalled members (baseline only shrinks) | 19 | 19 | |
| Uncalled C++ free functions in #1077's baseline | 34 | 0 | |
| Embedded C++ outside the census: oracle recipes | 302 | 302 | |
| Rust / tooling code lines | 212,731 / 81,786 | 216,300 / 83,726 | +198 / +108 |

| Source area / phase relationship (merge 119) | Original | Retired | Remaining | Fork glue |
| --- | ---: | ---: | ---: | ---: |
| Simulation logic (phase 1) | 47,424 | 38,239 | 9,185 | 4,098 |
| Simulation storage (phase 2) | 13,724 | 3,721 | 10,003 | 1,334 |
| NewGRF, save/load, script API and network (phase 3) | 51,794 | 13,100 | 38,694 | 3,187 |
| Support, graphics and viewport (phase 4) | 32,602 | 1,130 | 31,472 | 538 |
| GUI and platform (phase 5) | 64,120 | 424 | 63,696 | 466 |
| Vendored (phase 6) | 55,148 | 2,317 | 52,831 | 0 |
| Data tables | 17,295 | 4,391 | 12,904 | 14 |
| Upstream tests and fork test glue | 2,248 | 14 | 2,234 | 4,735 |
| Boundary layer | 0 | 0 | 0 | 13,603 |

Coverage remains bounded by the existing saves, NewGRFs, scenarios and retained native fixtures; entry witnesses are not branch coverage. New gaps go on #156's lists, which are its sub-issues #1038, #1039, #1040, #1041, #1042 and #1043: this interval's passes added to #1041 `NewGRFClass::Allocate`'s full-registry branch, the Action 0 handlers' `last > size` exit and the road 0x1D alternate-label store, and to #1042 a refit at a station that never held the cargo, aircraft loading through a part other than the first, the script API's walk of waiting cargo, the exclusive-supplier branch of `DeliverGoodsToIndustry` and `CmdAddSharedVehicleGroup`. No retained profile loads a NewGRF, so #1098's lookup by label, up to 256 slot calls for a label that is not there, is unmeasured. #503 deep checks remain user-requested.

### Cleanup stream inventories

The comment, integer, wrapper, constant, literal and assertion rows are analyzer estimates from `tools/code-standards-inventory.py`, given as "tree; in finished files". Their "in finished files" parts rise as files finish and are mostly rows a reviewer would dismiss, such as export parameters and raw adapters. Root adds the next column at its stocktake ([selection](planning.md#selection)).

| Stream | Inventory | Merge 107 | Merge 119 |
| --- | --- | ---: | ---: |
| #734 | Workset files with a finished pass | 173 of 318 | 186 of 318 |
| #734 | Workset lines in finished files | 146,256 | 166,485 |
| #734 | Lines left in unfinished workset files | 74,355 | 61,257 |
| #734 | Explanatory comment lines missing inside matched ported bodies | 1,439; 251 | 1,153; 267 |
| #734 | Bare-integer parameters and fields whose concept has a named type | 1,164; 390 | 1,141; 416 |
| #734 | Transparent wrapper types / named constants | 391 / 4,144; 148 / 1,779 | 398 / 4,182; 153 / 1,909 |
| #734 | Literal sites standing for a named value | 0; 0 | 0; 0 |
| #734 | #892's production assertion macro sites at run time (`debug_assert*!` / `assert*!` / `unreachable!`), with compile-time `assert!` beside them | 119 in 44 files (6 / 43 / 70); 37 in 16 (0 / 14 / 23). Compile-time 15; 13 | 89 in 41 files (6 / 30 / 53); 37 in 16 (0 / 14 / 23). Compile-time 18; 15 |
| #735 | Functions over seven parameters | 101 (67 FFI, 16 helpers, 18 commands) | 91 (61, 12, 18) |
| #735 | `too_many_arguments` ratchet rows / `allow` sites | 31 / 6 | 28 / 6 |
| #736 | Fork C++ evidence under `tools/` / `src/tests`, code lines | 14,399 / 4,731 | 14,395 / 4,735 |
| #738 | Rule corpus lines | 4,352 | 4,362 |
| None | Ruff rows | 282 | 282 |

The workset count is one command since #1065 (`docs/worksets.md`): 318 entries, none without a file. 15 finished files (7,277 lines) and 57 unfinished Rust files (14,814 lines) are in no workset; `.local/stocktake-119/4-worksets.txt` lists them, and [closure](#cleanup-work-order) places them. The lines left are the measure ([selection](planning.md#selection)).

Counting commands:

- **#734:** reconcile the issue workset manifest with merged complete-file pass PRs, then count current files with `wc -l`; a deleted finished entry remains in the denominator, and mechanical caller edits do not finish a file. The lines left are the `wc -l` of the workset files that `migration/finished-files.txt` does not list; their fall per running hour gives the stream's remainder. Run `python3 tools/code-standards-inventory.py [--head REV]` for the tree and with the finished paths for that subset; its `Assertion macros` line gives #892's sites, run-time and compile-time apart. The scripts of the merge-95 count are in `.local/stocktake-95/`.
- **#735:** use the tracked long-signature inventory counter, `migration/clippy-debt.json` (`too_many_arguments` keys) and `git grep 'allow(clippy::too_many_arguments' -- rust/`.
- **#736, the census baseline rows and the Clippy and Ruff rows:** from `python3 tools/port-metrics.py --whole-fork` (exports and callback slots are its "Boundary" line), `python3 tools/orphan-exports.py --report-uncalled`, `wc -l` of the three baseline files, and the debt files (keys are `sites` entries, diagnostics the sum of their list lengths). Expanded exports: `growth_inventory` in `tools/port-metrics.py`.
- **#738:** `cat AGENTS.md $(git ls-files 'docs/*.md' 'docs/**/*.md' | grep -v -E '^docs/(roadmap.md|components/)') | wc -l`.

### Speed

The merge-119 profile on clean `0f05ba2ef1` is the latest. All seven exit saves compared with no difference. The driver exited 1 on its anchor identity guard for every scenario, as it has since an unattended upgrade of the system's `libssh-4` on 2026-11-24 changed the digest of one linked library (#275), so these projections are computed from the report's instruction counts and the anchors by the [formula](speed.md#instruction-projections). The profile's output is retained in locked `.local/worktrees/root-profile-merge119`, the previous tip for attribution at the next profile; the merge-107 profile checkout `.local/worktrees/root-profile-merge107` is released.

| Scenario | Original Ir | Candidate Ir (merge 119) | Change since merge 107 | Candidate / original (merge 107) | Candidate / original (merge 119) | Projection (merge 119) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| TGP | 2,265,531,288 | 2,774,374,964 | +0.11% | 1.2232 | 1.2246 | 1.172488x |
| Opus | 21,773,118,666 | 25,873,377,165 | −0.44% | 1.1936 | 1.1883 | 1.317818x |
| Grok | 20,969,239,076 | 24,848,778,822 | −0.41% | 1.1898 | 1.1850 | 1.335597x |
| P1996 | 3,601,649,037 | 4,388,570,315 | −0.20% | 1.2215 | 1.2185 | 1.319243x |
| P2000 | 6,069,351,558 | 7,205,567,642 | −0.44% | 1.1977 | 1.1872 | 1.401373x |
| P2016 | 11,346,764,572 | 13,407,729,354 | −0.35% | 1.1858 | 1.1816 | 1.343582x |
| Drawing (diagnostic) | 2,148,975,596 | 2,154,608,804 | −0.00% | 1.0026 | 1.0026 | 1.004890x |

Every play scenario and generation runs fewer instructions than its anchor, so each projection is its anchor's ratio and none is in its band: generation is at 1.172x against 1.235x and the highest play record, P2000, at 1.401x against 1.52x. No projection rose by 3%, so [speed policy](speed.md#policy) asks for no attribution. Generation rose by 3.1 million instructions since merge 107 and is 0.48% under its anchor, after 12.3 million between merges 95 and 107; the passes that counted it measured −0.26% (#1096) and −0.002% (#1098), so the rise is in merges that did not count that scenario, and the next profile attributes it if it rises again. The passes counted their own scenarios at base and head ([ADR 0054](adr/0054-per-pass-counts-run-beside-review.md)): #1084 −0.88% on Opus; #1089 +0.10% on P2016, attributed in its PR body to restored assertions and code layout; #1097 −0.15% on P2016. Until a quiet pass re-anchors, `simulate --instructions` exits 1 after recording its counts, and a pass compares raw candidate counts at base and head.

The original's counts moved between the two profiles, where the merge-107 profile's equalled merge 95's to the instruction: from −91,852 (Opus) to +1,606,924 (P1996) and +26,875,089 (P2000, +0.44%). An unattended upgrade at 06:41Z on 2026-11-26 replaced system libraries (`libexpat1`, `libelf1` and others) between them; which one moved the counts is not traced. The candidate's change since merge 107 carries the same shift, so the candidate-to-original column is the comparison across this pair, and a pass's base and head counts taken on one day are not affected.

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

1. **Tools batch.** One per stocktake interval, dispatched within it and ranked by its items' measured cost ([selection](planning.md#selection)). #1063's batch merged as #1065 (merge 102). The batch of the interval that starts at merge 108 was #1085, merged as #1088 (merge 110): it deleted the uncalled C++ free functions that no open pass holds, and the push hook now refuses the base while the merge queue is not empty. The batch of the interval that starts at merge 120 is #1100: annotation checks that survive one GitHub API error, which removed #1089 from the queue once, and a requested run that failed in setup before any build, on which `enqueue` refused #1084 until root had one shard rerun. Its measured cost is one removal and one message to root in twelve merges, below a file pass's, so it is dispatched within the interval to a slot whose chain waits on a merge or a plan. New inventory visibility is not C++ deletion.
2. **The shared-service family:** shared services, accessors and string IDs that several files need. Merged: news and script events (#982), windows and sound (#1001), errors (#1005), console and chat (#1008), the string IDs (#995, #1006, #1033), the framerate service (#1020) and `BaseStation::xy` (#1028). Left: `Vehicle::Tick`'s dispatch (#1011, after the `ship_control.rs` pass) and the shared crate (#998, with strgen's port in phase 4). The family is closed at these two: root opens a new member only with the counts [selection](planning.md#selection) asks for. Until a member merges, a pass keeps the carriers it would replace and names the work order ([design faults](porting.md#design-faults)).
3. **#734/#735 file passes.** One PR per workset in the #734 manifest finishes every file it touches, applying the [code standards](porting.md#code-standards) to every line. Sizing, splits and single-file passes follow [sizing](planning.md#sizing); proof follows [cleanup proof](porting.md#cleanup-proof). Each pass reports its files' residue from `tools/code-standards-inventory.py`.

   Order, by fan-in (measured at `5af17025a3`). Delivered: the 03 remainder and its residue pass, 23's remainder (#884), 13 Map/terrain with 23's type-bearing files, except 23's `native_test_support.rs` and `native_test_support/movement.rs` (#973), and 09.
   - (c) 08, 07, 06, 10, 04, 05. Worksets 04, 06, 07, 08 and 10 are finished: #1057 closed with #1097 (merge 118). 05 is #1060, in three PRs: `orders.rs` with `order_func.rs` merged as #1084, `fleet.rs` with `vehiclelist.rs` is #1099, and `fleet/transactions.rs` with `fleet/pending.rs` follows it.
   - (d) The rest by fan-in ([sizing](planning.md#sizing)), the files outside a workset that reference it, counted at merge 95 (`.local/stocktake-95/11-fanin-output.txt`): 16 (89, #1062), 21 (75), 15 (51, #1064), 18 (47), 17 (37), 11 (36), 14 (30), 12 (29). Workset 19 (2) goes last of them, and F's closure stays last of all. Steering 39 put this order back: root's merge-95 order counted only the finished files that reference each workset, which moved 18 ahead of 15 and 12 ahead of 14. A finished caller is edited whenever its callee's workset passes, so it ranks no workset over another; the callers the order spares are the unfinished ones, and they give this same order. Counted again at merge 107 (`.local/stocktake-107/11-fanin-output.txt`), the order holds: 16 at 96, 21 at 78, 15 at 52, 18 at 48, 17 at 39, 11 at 37, and 14 and 12 at 32. Counted again at merge 119 (`.local/stocktake-119/11-fanin-output.txt`): 16 at 100, 21 at 80, 15 at 50, 18 at 49, 17 at 39, 11 at 38, 12 at 33 and 14 at 32, so 12 now goes before 14 and the rest holds. The NewGRF chain's slot interleaves 15 and 16 as [root's answers on #1064](https://github.com/fetch-laboratories/openttd-rust/issues/1064#issuecomment-7173604267) order them; the other slot takes 21 (#1101) after 05. Root counts again at the merge-131 stocktake.

   The unfinished files over 1,500 lines, each with a single-file pass, are `objects.rs` (2,696), `road_cmd.rs` (2,581), `newgrf_vehicle.rs` (2,129), `fleet.rs` (1,838, #1099), `newgrf_loading.rs` (1,832), `tunnelbridge_cmd.rs` (1,653), `landscape_generation.rs` (1,573) and `services.rs` (1,669, with F); `test_tables.rs` (2,249) is in no workset. #734 reports progress in lines finished as well as files.
4. **#736 test evidence.** No transfer remains. The aircraft range probe stays native under #656: #950 projected 10 to 13 changed lines for each net C++ line, above every delivered transfer, and the user approved closing it ([reason](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6860491029)). The classification table in #736's body names each remaining file's owner or boundary, as of 2026-11-14; that closes the stream's inventory without a PR ([ruling](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6834080910)).

   What stays native: `tools/migration/aircraft-range.cpp` with `finite_range_probe` (#656 retires it), the Catch2 files that delete whole (#931 closed, since its recipes outweighed the C++ it deleted), `src/tests/train.cpp` (its PBS and reservation cases retire with #725, its speed case with #656), and the ScriptList sections of `tools/migration/script-list-comparison.cpp`, which retire with that file when the script API reader takes the `ScriptList` adapter classes. The console fixtures and Catch2 files that reach live pools (65 files, 14,273 counted lines) retire inside the ports that retire their pools or readers. The 33 files and 3,909 lines of retained boundary evidence leave with their boundaries.
5. **#734 closure**, after the last workset pass: private wrapper fields, generator support, and removal of the raw adapters and aliases. At merge 119, 57 unfinished Rust files (14,814 lines; `test_tables.rs` is 2,249 of them) are in no workset, most of them test modules that passes moved out of their files; `.local/stocktake-119/4-worksets.txt` lists them. Each joins one before closure, the type files with F ([#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6767381933)). The end state and later stocktakes and steering measure with `tools/code-standards-inventory.py`. The closure residue sweep takes the recorded residue lists ([steering 34's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6789587702), [steering 35's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6834079997), [steering 36's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6847188216) and [steering 37's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-7119259564)) unless a later pass touches the file; its macro sites are #892's, and the three YAPF search-loop copies are #971's and stay. #892 rides remaining file passes, then that sweep and closure; no unrecorded production assertion macro site remains at closure, measured as #892 specifies.

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
- #1067 tracks the [fathom](skills/fathom/SKILL.md) cases, one checklist line for each, costliest first. Open: #1090 (reviews of where `unsafe` sits) and #1091 (a slot in trouble reaches root only through the user), both with their fixes landed and their numbers to read, #1070 (a new worktree's first build), #1072 (`native` failures before review) and #1073 (findings that could be settled before review). Steering reads each landed fix's number and closes its case or sends it back.
- #939 lists 35 rules that a script, hook or lint could hold instead of a doc. A candidate joins a [tools batch](planning.md#selection) when its rule was broken in the window just ended, with the count; [steering 40's counts](https://github.com/fetch-laboratories/openttd-rust/issues/939#issuecomment-7187322964) for merges 98 to 110 are the latest. #1092 now holds the costliest, a reviewer on a model that is not the review row's; the next is a full local simulation with no scenario named (68 slot-minutes on #1084).
- #1077 (the C++ free functions that no code calls): its baseline is empty since #1084 deleted the last ten rows. It stays open for [one function the gate passed](https://github.com/fetch-laboratories/openttd-rust/issues/1077#issuecomment-7187265117).
- #1032 (the economy date as a type through `delete_stale_links`) rides the pass of workset 21 (#1101) that finishes `timer_game_economy.rs` and `linkgraph/live.rs`.
- #940 holds the docs cleanup's follow-ups. Root's rulings are recorded there: a worktree is protected only by its whole folder name, and a deep run ends with its session. Its small fixes ride a tools batch where they share its files.
- #860 lists the development-process changes the user approved: the merge queue ([ADR 0019](adr/0019-merge-queue.md)), the per-merge builds ([ADR 0018](adr/0018-builds-before-every-merge.md)), CI scenario comparison ([ADR 0020](adr/0020-scenario-comparisons-on-ci.md)), direct reviewer and implementer messages, and the wind-down-only checkpoint ([ADR 0026](adr/0026-roadmap-commit-timing.md)).
- #896 (paid CI minutes): #1094 and #1095 merged the changes the user approved on 2026-11-26, and `CI_QUEUE_SAVES_CACHES` is `true`; [root's reading of merges 115 to 119](https://github.com/fetch-laboratories/openttd-rust/issues/896#issuecomment-7207548059) is its first evidence. It stays with its author for the one-day weighted-minutes and network-failure follow-up. The simulation shards are back on Blacksmith; which jobs run on paid runners is the user's decision ([ADR 0009](adr/0009-paid-runner-assignment.md)).

Excluded, with issue placement and return conditions read by steering 40 at merge 110 (69 open issues, none unplaced); near-full reproduction and fork release-source publication triggers are not met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Latest steering actions

Steering 40 is the latest, at merge 110. It timed every minute of the eleven implementers' slots in root's session of 2026-11-25, checked the 107 findings of its 34 review reports, compared #1066 and #1086 as merged with the original, read #1066's eleven reports round by round, and audited #1084 and root's 49 wakes. Its findings are in the message of the steering commit that follows `44e67df099`. Its changes are these.

- **Cases** (#1067). Two were new, and both fixes have landed from fathom agents' own threads ([ADR 0053](adr/0053-a-fathom-takes-no-slot.md)). #1069 and #1071 are closed on their numbers, #1070 and #1072 hold so far and are read again, and #1073 is sent back and not started. Root puts none of them in a tools batch and makes no rule for them.
  - **#1090, reviews of where `unsafe` sits** (`24541493b1` and #1093, merge 112). The original's preconditions are a module's contract, and whether a function relying only on it is a safe `fn` or an `unsafe fn` is no finding in either direction ([where `unsafe` sits](porting.md#where-unsafe-sits)). An `unsafe_code` acceptance holds through a fix that changes only the item's comments.
  - **#1091, a slot in trouble that reaches root only through the user** (#1092, merge 111). `python3 tools/ci.py reviewer PR KIND` prints each reviewer's start with the review row's model and holds the review limits: at a third scoped review that is not a base update's it prints a line for root, and root answers with one more review or with residue ([review loop](team.md#review-loop)). `ci.py report` refuses a report off the review row. A waiting child's last line does not reach root ([spawning](team.md#spawning)).
- **Root's spawn message is the fixed message** ([spawning](team.md#spawning)): the child's model and effort are in it, and root adds neither "name model and effort to any reviewer you start", which two implementers read as their own model, nor a line about scoped reviews, which the command now holds.
- **#1084 is reviewed whole**, by two reviewers and one re-check ([ADR 0051](adr/0051-review-whole-pr.md)). Root's porting groups are withdrawn ([steering's comments](https://github.com/fetch-laboratories/openttd-rust/pull/1084#issuecomment-7190133736)).
- **One `native` run a PR** is no longer provisional ([checks](workflow.md#checks)): 13 of 16 PRs kept to it over two sessions, and the one queue removal on a job `native` runs came from the merge itself.

Steering 40's two stops have ended with the fixes. The implementer's message before a second scoped review is replaced by the command's limit. Root's read of `ci.py state` at each wake ended when #1092 merged, as its line said, and [ADR 0012](adr/0012-root-decides-implementers-review-and-merge.md) stands as written: root watches no PR.

For root, on resume:
1. Start housekeeping with the routine agent. Rewrite the status comment on #3 to the current state.
2. Resume the two drafts from the [resume checkpoint](#resume-checkpoint), #1084 first, each with steering's comments on it ([#1084](https://github.com/fetch-laboratories/openttd-rust/pull/1084#issuecomment-7190133736), [#1089](https://github.com/fetch-laboratories/openttd-rust/pull/1089#issuecomment-7190134253)). #1084 takes the base before its reviewers start: its branch has neither `ci.py reviewer` nor #1093's acceptance rule. #1089 takes the base that holds #1084 before its first review.
3. Dispatch from the [ready queue](#ready-queue) in its order, on two slots.
4. Run the platform cadence after merge 114 and take stock after merge 119. In the stocktake, write the last line of `issues.py` as it prints, and no sentence about the exclusions' return conditions unless each was read: the stocktakes at merges 95 and 107 both say they were read, and neither read one.

Provisional, for steering 41 to judge:
- **The cases.** Read #1070's numbers at the merge-119 stocktake and #1072's once 20 PRs have requested `native`. Read #1090's on #1084 first: more than two scoped porting reviews after a re-check on a PR, or more than five on the next ten, is its failure, and so is any finding that asks to mark a function `unsafe` for a precondition the original has. Read #1091's over the PRs merged after `539a1eac80` with `python3 tools/ci.py reviewer PR --check`: a review past the limits with no answer from root, a void report, or a slot in trouble that the user finds before root does is its failure. It does not cover a slot idle on a lost wake or busy on a long local job, such as #1084's 68-minute simulation. The baseline is this session: of 1,559 slot-minutes editing took 31%, preflight 20.5%, builds 15% and base updates 10%.
- **Large passes.** Compare #1084 as merged with the original, and read each first reviewer's line of bodies compared against its roughly 418 changed functions. The void whole-PR pair found 0.48 findings in each 1,000 lines, where #1056's first stage found 1.09.
- **Reviews of a conflicted base update.** Five over two sessions found nothing. At ten with none, the rule goes ([integration](workflow.md#integration)).
- **Ports.** No port has been reviewed under these rules; the first three of phase 3 are compared with the original ([steer](skills/steer/SKILL.md)).
- **Plan sizes.** The session's passes came in at 0.99 to 1.90 times their plans. The excess is `unsafe_code` records (#1090) and, on #1086, 2,635 lines of merged branches' record files that the pass deleted.
- **The implementer limit, effort and the service tier** are the user's ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md), [ADR 0011](adr/0011-model-and-effort-per-role.md)). With three or four implementers a full preflight took a median 387 seconds over 29 runs, against 265 for a run alone, and preflight was 27.5% of slot time against 14.5% with two.

## Ready queue

Undispatched work orders ([team](team.md#ready-queue-and-pipelining)). Each has its own issue, whose body holds its plan once a planner has written it. Root's status comment on #3 holds the queue's state between commits to this section.

Two slots ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)). Each slot works one chain, and a pass planned on an open PR's head is dispatched when that PR merges. Residue that passes left in files they did not finish is on #734, one comment for each pass since [merges 66 to 75](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6976329602) and [steering 40's list](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-7187322409); two small porting points from #1000 wait on `residue/1000-porting-points` ([record](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6992168925)).

In flight at merge 119 (`0f05ba2ef1`), one chain a slot:

- **Orders chain:** #1099 (#1060's second part, `fleet.rs` with `vehiclelist.rs`), in review.
- **NewGRF chain:** #1064's third part (`newgrf_bytereader.rs`, `newgrf_storage.rs`, `newgrf_specs.rs` and `newgrf_translation.rs`, planned at about 4,600 changed lines), dispatched after the merge-119 profile.

Waiting on a merge, with a planner running:

1. **#1060's third part** (`fleet/transactions.rs` with `fleet/pending.rs`; it closes #1060) follows #1099 and is planned on its head. It takes `TransferCargo`'s final consist update, which #1089 left at its caller.
2. **#1101, workset 21** (Session/commands/clocks: 15 unfinished files, 6,699 lines, more than one PR) follows #1060's third part on the orders chain's slot. #1032 rides the pass that finishes `timer_game_economy.rs` and `linkgraph/live.rs`.

Waiting on a merge, not yet planned:

3. **#1064's fourth part** (`newgrf_spritegroup.rs` with the resolver frames) follows the third, whose `newgrf_storage.rs` is its callee, and gets its planner when the third has a PR. It goes before #1062's parts 2 to 4.
4. **#1062's parts 2 to 4** (workset 16, NewGRF domains; `newgrf_vehicle.rs`, 2,129 lines, has its own pass): they follow #1064's fourth part and move the 26 `GRFFile` member slots of three finished definition files to the handle ([root's second answers](https://github.com/fetch-laboratories/openttd-rust/issues/1062#issuecomment-7154092829)). #1064's parts 5 to 7 follow them.

Waiting on a slot whose chain waits: **#1100**, the tools batch of the interval that starts at merge 120 ([cleanup work order](#cleanup-work-order)); it has no plan yet.

Root plans the next entry when a dispatch leaves a chain without one ([team](team.md#ready-queue-and-pipelining)).

The train passes take their sites of #999 (order destination reads at the original's observation point). #998 is the work order for one host-buildable crate below `openttd-kernels`, which removes the copies of three kernels ports that `openttd-strgen` keeps under [root's ruling](https://github.com/fetch-laboratories/openttd-rust/pull/995#issuecomment-6975789181); no second build-time consumer has appeared and it removes no fork C++, so it stays behind the passes, selected no later than strgen's port in phase 4. #997 (the native assertion service reports neither the Rust site nor its predicate) is on #892's checklist and rides the closure residue sweep.

Full phase-3 planning remains untriggered: most of the files still to pass are in no entry.

## Resume checkpoint

Running. This section is the state at the merge-119 stocktake, 2026-11-27 about 02:30Z; root's status comment on #3 holds the state between roadmap commits, and where GitHub is newer than this section, GitHub is right.

Base is merge 119 (`0f05ba2ef1`); its Quick and builds passed, the platform cadence at merge 119 was green, the merge queue is empty, no merge stop is in force and no rollback happened. The [ready queue](#ready-queue) names what each slot holds and what follows. The next platform cadence follows merge 124 and the next stocktake merge 131.

Root's rulings of the session are on their PRs: #1095's workflow review and bootstrap Full; #1089's wait for #1084, on a trial merge that did not compile; one rerun of the failed shard of #1084's `native` run where steering's comment said to rerun nothing, since `enqueue` reads the newest requested run (#1100's second item); and #1096's third scoped records review.

Agent: /root | Model: claude-fable-5-2 | Reasoning effort: high

The implementer limit is two ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)), and the [role table](team.md#roles-and-models) gives each role's model and effort.

`CI_MACOS_HEAVY` is unset, `CI_ON_DEMAND` is `true` and `CI_QUEUE_SAVES_CACHES` is `true`. The hooks of #927 are installed in the shared clone (`python3 tools/git_hooks.py install`).

Flakes and limits carried forward:

- #693: the `network-protocol-loopback` stalls are the original's `TCPConnecter` race (#1016); the harness retries a session once on that signature and counts the retry in the run's report. The `content-protocol` evidence misses are still unexplained. Keep `.local/retained-network-693` and the linked artifacts.
- The profile driver exits 1 on its anchor identity guard until a quiet pass re-anchors (#275), and deletes `candidate-runtime/` after every run. A system library upgrade is enough to trip it: `libssh-4`'s on 2026-11-24 did, one day after #1024's pass ([speed](#speed)).
- A branch that forked before merge 83 runs the old apt action in a requested run and can still lose a job to GitHub's stalling mirror: rerun it.
- A child's last report can fail to reach root (#710): the last of the session of 2026-11-24 did. A `gpt-6.2-sol` reviewer can stop on a host approval prompt that only the user can answer, under a full-access runtime; its parent sees only silence (#710).
- An implementer started at medium effort signs `Reasoning effort: high`: the host does not report the effort to a child, so root's spawn message names the model and effort to sign.
- A GitHub API error in one annotation check fails the three checks after it and the queue run with them (#1100); until that batch merges, queue such a removal again once with `enqueue --retry`.
- The original's instruction counts moved between the merge-107 and merge-119 profiles after an unattended system upgrade ([speed](#speed)); compare a pass's base and head counts taken on one day.
- The push hook refuses a push to `rust-migration` while the merge queue holds an entry (#1088).
- Metric scope completeness stays with #736's end-state inventory. No later phase is selected.

Retained state for housekeeping:

- `python3 tools/housekeeping.py --apply` ran at the session's start and with the platform cadence at merges 114 and 119; `/` has 3.2T free.
- An open PR's checkout is kept by the tool. The merged PRs' checkouts and the pushed review, re-check and plan checkouts can go.
- A name below protects a worktree only when it is the whole folder name ([housekeeping](root-workflow.md#housekeeping)). The `t3-` names of eight hex digits in this list are whole folder names from an earlier host version.
- Keep the locked merge-119 profile checkout `.local/worktrees/root-profile-merge119` until the next profile is recorded; the merge-107 one is released. Preserve all current speed, deep and evidence anchors; the speed anchors live in the main checkout's `.local/speed-pass`, and `20261123-hardware-f80cbaf` there is the pass the current anchors come from. `.local/stocktake-119/` holds the merge-119 stocktake's counts and scripts until the merge-131 stocktake, and `.local/stocktake-107/` is released. `.local/steering-40/` holds steering 40's audits, which #1090 and #1091 cite, `.local/steering-39/` steering 39's, which #1070, #1072 and #1073 cite, and `.local/steering-38/` the findings-table definitions they use: keep all three until the cases close.
- Keep `t3-f6bd08df`: three unpushed docs commits from 2026-11-10. Keep `t3-f4cf7a0b` (one unique unpushed commit, `125ea2440a`).
- The rest is carried forward as preservation guidance from earlier checkpoints; steering 38 found each dirty tree it names still dirty and did not check the others.
  - Keep the planning checkout `t3-20fec9ce` with its private plan, and origin `preserve/housekeeping-foundation-6ab9c20e5d`. Preserve every checkout with uncommitted or unpushed work.
  - Keep `.local/worktrees/root-profile-125`, `.local/worktrees/root-profile-119` and `.local/worktrees/root-profile-113` with accepted reports, raw Callgrind and frozen runtimes. Keep `t3-a490295e` (profile 131, clean `305ac20c28`) with its HDD `.local` outputs. Profile 119 measures clean `4d3a1d1d41`, not integration 120.
  - Keep main `.local/integration-records/`, `.local/speed-pass/` anchors, frozen runtimes and the retained simulation evidence.
  - Leave dirty or staged trees untouched: `t3-03c1e2da`, `t3-1ddaadfd`, `t3-5575ebe4`, `t3-83082b90`, `t3-c68d914a`, `t3-e0361d8c`, `t3-3b491ddb`, locked #565 WIP `t3-431c84b4`, locked native evidence `t3-d78d5f8b`, `t3-32cf4edc` and `t3-e130d5a3`.
  - Keep protected `.local/worktrees/root-docs-72` and locked `.local/worktrees/profile-after*`. Do not push stale root-docs guidance over current main. Keep the pinned original unchanged.
