# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place ([records](workflow.md#records)); `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands

Base is at merge 120 (`4f3f8f1620`, #1099), where root's session of 2026-11-26 19:43Z to 2026-11-27 03:50Z (8.12 hours) stopped at the user's request ([resume checkpoint](#resume-checkpoint)). Whole-fork census and stream inventory figures below are steering 41's merge-120 readings at `4f3f8f1620` beside root's merge-119 stocktake (`0f05ba2ef1`, `.local/stocktake-119/`). The merge 108 to 119 interval was 18 running hours ([who decides](../AGENTS.md#who-decides)), of which five had no implementer (the fathom fixes #1092 and #1093 and #896's #1094 landed in them). The next stocktake follows merge 131, and the platform cadence was green at merges 114 and 119.

- **#734 has 188 of 318 workset files finished and 59,465 lines left** at merge 120, after #1099 finished `fleet.rs` and `vehiclelist.rs` (from 61,257 at merge 119, 73,029 at merge 110 and 74,355 at merge 107). In merges 108 to 119 the lines left fell by 13,098 (728 a running hour; about 1,008 in the 13 hours an implementer ran, against 754 in the session before). At 728 to 1,008 an hour, the 59,465 lines left are about 59 to 82 running hours of passes before the 58 unfinished files in no workset (14,819 lines), closure and #893. Of group (c), worksets 04, 06, 07, 08 and 10 are finished and 05 has two files left (`fleet/transactions.rs` and `fleet/pending.rs`, #1060 part 3); 15 has twelve of nineteen left, four of them in #1102.
- **Fork C++ is at 267,681 lines at merge 120** (267,818 at merge 119; 1,392 fewer than merge 107's 269,073). Original C++ retired is 63,363 lines, 22.28% (simulation 41,987 of 61,148, 68.66%). The boundary has 2,518 callback slots (111 fewer than merge 107) and 1,454 textual exports (up from 1,438 at #1096, which expanded macro-generated exports by hand; expanded exports fell from 1,582 to 1,567 by merge 119). #735 is at 90 functions over seven parameters (from 101), #892 has 88 run-time assertion macro sites in 40 files (from 119), shared-accessor baseline fell from 42 to 21, and #1077's uncalled-free-function baseline is at 0.
- **The session's merged passes** (merges 115 to 120) are #1084 (`orders.rs` with `order_func.rs`, 17,066 changed lines), #1089 (`cargo_storage.rs`, 9,253), #1096 (#1064 part 1, 4,039), #1097 (`cargo_payment.rs` with `cargomonitor.rs`, 2,836), #1098 (#1064 part 2, 4,902) and #1099 (#1060 part 2, `fleet.rs` with `vehiclelist.rs`, 13,628 raw / 8,315 pass-own after excluding 5,313 deleted lines of six earlier PRs' cleanup-record files). Across the session's 8 implementer slots (924.4 slot-minutes), editing share jumped from 31.1% to 47.3%, local builds fell from 15.2% to 8.1%, base updates fell from 10.2% to 3.5% and blocked speed waits fell from 2.9% to 0.5%; fixing review findings took 31.1% and preflight 17.4%.
- **Review and fidelity:** #1090's rule (`24541493b1`) and #1093 dropped original-precondition `unsafe` findings from 27 to 0 across all 33 valid reports while keeping all 6 real Rust-aliasing catches; `FIX@S3` fell from 8.3% to 2.6%. #1091's `ci.py reviewer` held 0 post-fix `VOID` reports and caught #1096's third scoped review (answered by root in 24 s). Steering 41's line-by-line fidelity audits of #1084 and #1099 found **0 reachable behaviour divergences** (`0.00 / 1k` on both) and 4 minor departures left behind on each (`0.23 / 1k` on #1084; `0.29 / 1k` total / `0.48 / 1k` pass-own on #1099).
- **CI and spend (#896):** after #1094 and #1095 (with `CI_QUEUE_SAVES_CACHES=true`), all six covered merges (115 to 120) hit queue-saved caches on push (`native / build` 35–41 s, `windows-saved` 14–16 s, Windows skipped), median queue Full fell to 11.00 minutes, and measured Blacksmith spend dropped **52% per merge** from `$1.460` to **`$0.707`** (`$0` paid macOS; [steering 41's measurement](https://github.com/fetch-laboratories/openttd-rust/issues/896#issuecomment-7213331262)).
- **Speed:** every play scenario runs fewer instructions than at merge 107 and generation 0.11% more, all under their anchors ([speed](#speed)).
- The limit is two implementers ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md); 0.0 minutes at 3+ implementers, 98.3% at two including the 37.8-minute merge-119 profile).
- No rollback has happened since merge 32. Full phase-3 planning is not triggered, and near-full reproduction is not met. `python3 docs/skills/steer/issues.py` ends `70 open: 0 unplaced, 1 excluded, 0 roadmap exclusions without an issue`.
- **Unmerged branches on origin:** 56 at merge 120 (55 at merge 119 in `.local/stocktake-119/12-branches-prs.txt`; #1099 merged and deleted, #1102 and `t3/fleet-transactions-pending-port` added). Kept with no PR: `preserve/housekeeping-foundation-6ab9c20e5d` and `fathom/1082` (`52e69b22ae`).

Reproduce the census with `python3 tools/port-metrics.py --whole-fork` (metric version 3).

| Measured quantity | Merge 107 | Merge 119 | Merge 120 | Change per running hour (m108–119, 18 h) |
| --- | ---: | ---: | ---: | ---: |
| Original C++ remaining | 221,283 | 221,019 | 220,992 | −15 |
| Fork source glue, including generated records | 28,842 | 27,975 | 27,894 | −48 |
| Compiled source and generated records C++ | 250,125 | 248,994 | 248,886 | −63 |
| Tracked tools C++ evidence | 14,399 | 14,395 | 14,395 | −0.2 |
| ABI-generated non-record C++ evidence | 4,549 | 4,429 | 4,400 | −7 |
| Measured fork C++ series | 269,073 | 267,818 | 267,681 | −70 |
| Original C++ retired, of 284,355 | 63,072 (22.18%) | 63,336 (22.27%) | 63,363 (22.28%) | +15 |
| Simulation C++ retired, of 61,148 | 41,770 (68.31%) | 41,960 (68.62%) | 41,987 (68.66%) | +11 |
| Historical regex exports / callback slots | 1,438 / 2,629 | 1,458 / 2,548 | 1,454 / 2,518 | +1 / −4.5 |
| Shared-accessor / rebound-callee / orphan-export baseline rows | 42 / 192 / 17 | 27 / 181 / 17 | 21 / 181 / 17 | |
| Exports held only by uncalled members (baseline only shrinks) | 19 | 19 | 19 | |
| Uncalled C++ free functions in #1077's baseline | 34 | 0 | 0 | |
| Embedded C++ outside the census: oracle recipes | 302 | 302 | 302 | |
| Rust / tooling code lines | 212,731 / 81,786 | 216,300 / 83,726 | 218,059 / 83,720 | +198 / +108 |

| Source area / phase relationship (merge 120) | Original | Retired | Remaining | Fork glue |
| --- | ---: | ---: | ---: | ---: |
| Simulation logic (phase 1) | 47,424 | 38,253 | 9,171 | 4,103 |
| Simulation storage (phase 2) | 13,724 | 3,734 | 9,990 | 1,331 |
| NewGRF, save/load, script API and network (phase 3) | 51,794 | 13,100 | 38,694 | 3,187 |
| Support, graphics and viewport (phase 4) | 32,602 | 1,130 | 31,472 | 538 |
| GUI and platform (phase 5) | 64,120 | 424 | 63,696 | 466 |
| Vendored (phase 6) | 55,148 | 2,317 | 52,831 | 0 |
| Data tables | 17,295 | 4,391 | 12,904 | 14 |
| Upstream tests and fork test glue | 2,248 | 14 | 2,234 | 4,735 |
| Boundary layer | 0 | 0 | 0 | 13,520 |

Coverage remains bounded by the existing saves, NewGRFs, scenarios and retained native fixtures; entry witnesses are not branch coverage. New gaps go on #156's lists, which are its sub-issues #1038, #1039, #1040, #1041, #1042 and #1043: this interval's passes added to #1041 `NewGRFClass::Allocate`'s full-registry branch, the Action 0 handlers' `last > size` exit and the road 0x1D alternate-label store, and to #1042 a refit at a station that never held the cargo, aircraft loading through a part other than the first, the script API's walk of waiting cargo and the exclusive-supplier branch of `DeliverGoodsToIndustry` (`CmdAddSharedVehicleGroup`'s execution path was already recorded on #1042 before the session). No retained profile loads a NewGRF, so #1098's lookup by label, up to 256 slot calls for a label that is not there, is unmeasured. #503 deep checks remain user-requested.

### Cleanup stream inventories

The comment, integer, wrapper, constant, literal and assertion rows are analyzer estimates from `tools/code-standards-inventory.py`, given as "tree; in finished files". Their "in finished files" parts rise as files finish and are mostly rows a reviewer would dismiss, such as export parameters and raw adapters. Root adds the next column at its stocktake ([selection](planning.md#selection)).

| Stream | Inventory | Merge 107 | Merge 119 | Merge 120 |
| --- | --- | ---: | ---: | ---: |
| #734 | Workset files with a finished pass | 173 of 318 | 186 of 318 | 188 of 318 |
| #734 | Workset lines in finished files | 146,256 | 166,485 | 169,289 |
| #734 | Lines left in unfinished workset files | 74,355 | 61,257 | 59,465 |
| #734 | Explanatory comment lines missing inside matched ported bodies | 1,439; 251 | 1,153; 267 | 1,124; 267 |
| #734 | Bare-integer parameters and fields whose concept has a named type | 1,164; 390 | 1,141; 416 | 1,138; 437 |
| #734 | Transparent wrapper types / named constants | 391 / 4,144; 148 / 1,779 | 398 / 4,182; 153 / 1,909 | 399 / 4,180; 154 / 1,924 |
| #734 | Literal sites standing for a named value | 0; 0 | 0; 0 | 0; 0 |
| #734 | #892's production assertion macro sites at run time (`debug_assert*!` / `assert*!` / `unreachable!`), with compile-time `assert!` beside them | 119 in 44 files (6 / 43 / 70); 37 in 16 (0 / 14 / 23). Compile-time 15; 13 | 89 in 41 files (6 / 30 / 53); 37 in 16 (0 / 14 / 23). Compile-time 18; 15 | 88 in 40 files (6 / 30 / 52). Compile-time 18 |
| #735 | Functions over seven parameters | 101 (67 FFI, 16 helpers, 18 commands) | 91 (61, 12, 18) | 90 (61, 11, 18) |
| #735 | `too_many_arguments` ratchet rows / `allow` sites | 31 / 6 | 28 / 6 | 28 / 6 |
| #736 | Fork C++ evidence under `tools/` / `src/tests`, code lines | 14,399 / 4,731 | 14,395 / 4,735 | 14,395 / 4,735 |
| #738 | Rule corpus lines | 4,352 | 4,362 | 4,362 |
| None | Ruff rows | 282 | 282 | 282 |

The workset count is one command since #1065 (`docs/worksets.md`): 318 entries, none without a file. At merge 120, 16 finished files (8,737 lines) and 58 unfinished Rust files (14,819 lines) are in no workset; `.local/stocktake-119/4-worksets.txt` lists the merge-119 set, and [closure](#cleanup-work-order) places them. The lines left are the measure ([selection](planning.md#selection)).

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

The original's P2000 count moved by `+26,875,089` (+0.44%) between the merge-107 and merge-119 profiles (while the other six stayed within 0.05%). Steering 41 traced it ([#275](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-7213330372)): an unattended upgrade of six linked libraries at 06:40Z on 2026-11-26 relinked `.local/build-reference/openttd` and invalidated merge 83's cached P2000 reference result; re-running P2000 live under CPU contention ran slower (`140.5 s` vs `89.7 s`), and `VideoDriver_Null::MainLoop` calls `UpdateWindows()` reading real wall-clock `steady_clock`, flipping `TimerWindow` from 8 to 10 ticks and keeping a road-vehicle `NewsWindow` mini-viewport open longer. Both reference and candidate show this bimodal `±0.45%` wall-clock noise on P2000; it is not a library or game regression.

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

1. **Tools batch.** One per stocktake interval, dispatched within it and ranked by its items' measured cost ([selection](planning.md#selection)). #1063's batch merged as #1065 (merge 102). The batch of merges 108 to 119 was #1085, merged as #1088 (merge 110). The batch of the interval starting at merge 120 is **#1100**, expanded by steering 41 to six ranked items needing no diagnosis: (1) refuse bare `migration.py simulate` without `--all` (#939 top item: 68 slot-minutes on #1084); (2) stop `tools/cleanup_contract.py:173` from auto-deleting other merged PRs' cleanup-record files inside unrelated file passes (#1099 +5,313 lines, #1086 +2,635); (3) annotation checks surviving one GitHub API error (#1089 queue removal, 15 min + `$0.368`); (4) `enqueue` handling of pre-build setup-failed requested runs (#1084, +10.6 min); (5) align 32-bit Clippy flags in `ci-build.yml:255` with `tools/migration.py` (#1098, 6.3 min + `$0.176`, sharing item 3's bootstrap Full); and (6) `review_start.py` refusing `records` before `recheck` posts (#1096). Plan and dispatch #1100 within merges 120 to 131 on a slot whose chain waits or ahead of lower-cost work. New inventory visibility is not C++ deletion.
2. **The shared-service family:** shared services, accessors and string IDs that several files need. Merged: news and script events (#982), windows and sound (#1001), errors (#1005), console and chat (#1008), the string IDs (#995, #1006, #1033), the framerate service (#1020) and `BaseStation::xy` (#1028). Left: `Vehicle::Tick`'s dispatch (#1011, after the `ship_control.rs` pass) and the shared crate (#998, with strgen's port in phase 4). The family is closed at these two: root opens a new member only with the counts [selection](planning.md#selection) asks for. Until a member merges, a pass keeps the carriers it would replace and names the work order ([design faults](porting.md#design-faults)).
3. **#734/#735 file passes.** One PR per workset in the #734 manifest finishes every file it touches, applying the [code standards](porting.md#code-standards) to every line. Sizing, splits and single-file passes follow [sizing](planning.md#sizing); proof follows [cleanup proof](porting.md#cleanup-proof). Each pass reports its files' residue from `tools/code-standards-inventory.py`.

   Order, by fan-in (measured at `5af17025a3`). Delivered: the 03 remainder and its residue pass, 23's remainder (#884), 13 Map/terrain with 23's type-bearing files, except 23's `native_test_support.rs` and `native_test_support/movement.rs` (#973), and 09.
   - (c) 08, 07, 06, 10, 04, 05. Worksets 04, 06, 07, 08 and 10 are finished: #1057 closed with #1097 (merge 118). 05 is #1060, in three PRs: `orders.rs` with `order_func.rs` merged as #1084 (merge 115), `fleet.rs` with `vehiclelist.rs` merged as #1099 (merge 120), and `fleet/transactions.rs` with `fleet/pending.rs` follows it (in progress at checkpoint `2b4d5eed8a`).
   - (d) The rest by fan-in ([sizing](planning.md#sizing)), the files outside a workset that reference it, counted at merge 95 (`.local/stocktake-95/11-fanin-output.txt`): 16 (89, #1062), 21 (75), 15 (51, #1064), 18 (47), 17 (37), 11 (36), 14 (30), 12 (29). Workset 19 (2) goes last of them, and F's closure stays last of all. Steering 39 put this order back: root's merge-95 order counted only the finished files that reference each workset, which moved 18 ahead of 15 and 12 ahead of 14. A finished caller is edited whenever its callee's workset passes, so it ranks no workset over another; the callers the order spares are the unfinished ones, and they give this same order. Counted again at merge 107 (`.local/stocktake-107/11-fanin-output.txt`), the order holds: 16 at 96, 21 at 78, 15 at 52, 18 at 48, 17 at 39, 11 at 37, and 14 and 12 at 32. Counted again at merge 119 (`.local/stocktake-119/11-fanin-output.txt`): 16 at 100, 21 at 80, 15 at 50, 18 at 49, 17 at 39, 11 at 38, 12 at 33 and 14 at 32, so 12 now goes before 14 and the rest holds. The NewGRF chain's slot interleaves 15 and 16 as [root's answers on #1064](https://github.com/fetch-laboratories/openttd-rust/issues/1064#issuecomment-7173604267) and [steering 41's placement of part 4b](https://github.com/fetch-laboratories/openttd-rust/issues/1064#issuecomment-7213332719) order them; the other slot takes 21 (#1101) after 05. Root counts again at the merge-131 stocktake.

   The unfinished files over 1,500 lines, each with a single-file pass, are `objects.rs` (2,696), `road_cmd.rs` (2,581), `newgrf_vehicle.rs` (2,128), `newgrf_loading.rs` (1,832), `tunnelbridge_cmd.rs` (1,653), `landscape_generation.rs` (1,573) and `services.rs` (1,669, with F); `test_tables.rs` (2,220) is in no workset. #734 reports progress in lines finished as well as files.
4. **#736 test evidence.** No transfer remains. The aircraft range probe stays native under #656: #950 projected 10 to 13 changed lines for each net C++ line, above every delivered transfer, and the user approved closing it ([reason](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6860491029)). The classification table in #736's body names each remaining file's owner or boundary, as of 2026-11-14; that closes the stream's inventory without a PR ([ruling](https://github.com/fetch-laboratories/openttd-rust/issues/736#issuecomment-6834080910)).

   What stays native: `tools/migration/aircraft-range.cpp` with `finite_range_probe` (#656 retires it), the Catch2 files that delete whole (#931 closed, since its recipes outweighed the C++ it deleted), `src/tests/train.cpp` (its PBS and reservation cases retire with #725, its speed case with #656), and the ScriptList sections of `tools/migration/script-list-comparison.cpp`, which retire with that file when the script API reader takes the `ScriptList` adapter classes. The console fixtures and Catch2 files that reach live pools (65 files, 14,273 counted lines) retire inside the ports that retire their pools or readers. The 33 files and 3,909 lines of retained boundary evidence leave with their boundaries.
5. **#734 closure**, after the last workset pass: private wrapper fields, generator support, and removal of the raw adapters and aliases. At merge 120, 58 unfinished Rust files (14,819 lines; `test_tables.rs` is 2,220 of them) are in no workset, most of them test modules that passes moved out of their files; `.local/stocktake-119/4-worksets.txt` lists the merge-119 set. Each joins one before closure, the type files with F ([#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6767381933)). The end state and later stocktakes and steering measure with `tools/code-standards-inventory.py`. The closure residue sweep takes the recorded residue lists ([steering 34's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6789587702), [steering 35's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6834079997), [steering 36's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6847188216), [steering 37's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-7119259564), [steering 40's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-7187322409) and [steering 41's](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-7213328774)) unless a later pass touches the file; its macro sites are #892's, and the three YAPF search-loop copies are #971's and stay. #892 rides remaining file passes, then that sweep and closure; no unrecorded production assertion macro site remains at closure, measured as #892 specifies.

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

- #156 (coverage gaps, whose lists are its sub-issues #1038, #1039, #1040, #1041, #1042 and #1043), #275 (speed), #724 and #710 (host) stay open trackers. #710 holds [steering 39's](https://github.com/fetch-laboratories/openttd-rust/issues/710#issuecomment-7158594687), [steering 40's](https://github.com/fetch-laboratories/openttd-rust/issues/710#issuecomment-7187324041) and [steering 41's host measurements](https://github.com/fetch-laboratories/openttd-rust/issues/710#issuecomment-7213331947) (0 lost wakes and 0 approval prompts in merges 115 to 120).
- #1058 owns `CargoSpec::array`, its defaults and its publication in phase 2, queued with the smaller records of #638 and #498; workset 04's growth record for the `CargoSpec` member slots names it.
- #500 closes with #734's closure. #168 and #199 are closed: #565, #717 and #734 own what remained.
- #693: #1015's work traced the `network-protocol-loopback` stalls to a thread race in the original's `TCPConnecter`, which can stay in `Resolving` for ever when its resolver finishes first (#1016, a `deferred-improvement`: neither build changes). The harness bounds its waits and retries a session once on that signature while the candidate's copy of the file is identical to the pinned original's, counting each retry in the run's report; stocktakes read the rate there. The `content-protocol` evidence misses stay open on #693: that scenario runs without network debug output, so its logs cannot show the race. The allowance ends when #730 ports the connecter.
- #503's coverage continuation runs last.
- #1067 tracks the [fathom](skills/fathom/SKILL.md) cases, one checklist line for each, costliest first. Closed at steering 41: #1070 (cold builds / table headers) and #1091 (a slot in trouble). Open: **#1073** (settleable pre-review findings, restarted by steering 41 with a fathom agent), **#1103** (preflight failure rate and 268 s Python step, opened and started by steering 41), **#1104** (run-time-compiled scenario probes failing `native`, opened not started), **#1090** (reviews of where `unsafe` sits, holds strongly across 6 of 10 passes) and **#1072** (`native` freeze-step failures and `ready-wait`, holds across 18 of 20 PRs). Steering reads each landed fix's number and closes its case or sends it back.
- #939 lists rules that a script, hook or lint could hold instead of a doc. A candidate joins a [tools batch](planning.md#selection) when its rule was broken in the window just ended, with the count; [steering 41's counts](https://github.com/fetch-laboratories/openttd-rust/issues/939#issuecomment-7213329612) for merges 111 to 120 plus #1102 are the latest (Steering 40's top remaining item, bare `simulate` with no scenario, moved into #1100).
- #1077 (the C++ free functions that no code calls): its baseline is empty since #1084 deleted the last ten rows. It stays open for [one function the gate passed](https://github.com/fetch-laboratories/openttd-rust/issues/1077#issuecomment-7187265117).
- #1032 (the economy date as a type through `delete_stale_links`) rides #1101's part 2 (the linkgraph files: `linkgraph/live.rs`, `linkgraph.rs`, `linkgraph/job.rs`), per [root's split ruling](https://github.com/fetch-laboratories/openttd-rust/issues/1101#issuecomment-7208053608).
- #940 holds the docs cleanup's follow-ups. Root's rulings are recorded there: a worktree is protected only by its whole folder name, and a deep run ends with its session. Its small fixes ride a tools batch where they share its files.
- #860 lists the development-process changes the user approved: the merge queue ([ADR 0019](adr/0019-merge-queue.md)), the per-merge builds ([ADR 0018](adr/0018-builds-before-every-merge.md)), CI scenario comparison ([ADR 0020](adr/0020-scenario-comparisons-on-ci.md)), direct reviewer and implementer messages, and the wind-down-only checkpoint ([ADR 0026](adr/0026-roadmap-commit-timing.md)).
- #896 (paid CI minutes): #1094 and #1095 merged the changes the user approved on 2026-11-26, and `CI_QUEUE_SAVES_CACHES` is `true`; [root's reading of merges 115 to 119](https://github.com/fetch-laboratories/openttd-rust/issues/896#issuecomment-7207548059) and [steering 41's Blacksmith spend measurement](https://github.com/fetch-laboratories/openttd-rust/issues/896#issuecomment-7213331262) (`$1.460` → `$0.707` per merge, merge 120 cache hit verified) confirm the reduction. The simulation shards are on Blacksmith; which jobs run on paid runners is the user's decision ([ADR 0009](adr/0009-paid-runner-assignment.md)).

Excluded, with exclusion return conditions last read by steering 40 at merge 110 and current issue placement read by steering 41 at merge 120 (`70 open: 0 unplaced, 1 excluded, 0 roadmap exclusions without an issue`); near-full reproduction and fork release-source publication triggers are not met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Latest steering actions

Steering 41 is the latest, at merge 120 (`4f3f8f1620`). It timed every minute of the 8 implementer slots in root's 8.12-hour session (`84ea9335`, merges 115 to 120 plus #1102 and #1060 part 3), verified all 143 findings across 33 valid review reports (merges 111 to 120 plus #1102), audited all 12 unreviewed post-recheck commits, audited root's 27 turns, merge-119 stocktake and 7 plans, and compared #1084 and #1099 line by line against the pinned original. Its findings are in the message of the steering 41 commit on `rust-migration`. Earlier standing actions from steering 40 now live in their owning docs ([root-workflow](root-workflow.md#cadences) now holds verbatim `issues.py` output, honest exclusion claims and checking #939 at stocktakes; [team](team.md#spawning) gives planners the same non-doc-repeating `Facts` clause as implementers).

- **Cases** (#1067). Two closed on their numbers; two held and stay open; two fathom agents launched on `claude-opus-5-5` high ([ADR 0053](adr/0053-a-fathom-takes-no-slot.md)); one new case opened not started. Root puts none of the open cases in a tools batch and makes no rule for them.
  - **Closed:** **#1070** (cold builds / table headers: `--reach` 21, 14, 18, 11 of 574 at merge 119 against fail `> 50`; 6-merge median 204.5; build share 15.2% → 8.1%) and **#1091** (slot in trouble: 0 post-fix `VOID` reports, 0 unapproved `Past the limits` on 9 merged PRs + #1102, 0 user-found stalls).
  - **Started now:** **#1073** (settleable pre-review findings, restarted: `FIX` = 31.1% of slot time, 85.3% file-pass serious/medium settleable, K1 repeated resolution 4 → 17 rows driving both post-recheck scoped cycles) and **#1103** (local preflight: 17.4% of slot time at 2 slots, 40% failure rate burning 92.3 of 217.0 job-minutes while 36 s Rust failures wait 251–571 s, Python step at 268.4 s).
  - **Opened, not started:** **#1104** (run-time-compiled scenario probes such as `aircraft-range.cpp` failing `native` after passing preflight — 3rd recurrence on #1102 after #1035 and #1087).
  - **Holding, stay open:** **#1090** (3 post-recheck scoped reviews on 6 merged passes, max 2; 0 original-precondition `unsafe` findings; 4 passes left in 10-PR bar) and **#1072** (0 freeze-step failures on 18 PRs, 2 left; `ready-wait` passed at 2 of 10 / 2 of 20; adapter step 229.6 s < 240 s).
- **Tools batch #1100 updated** with four proven items needing no diagnosis: `migration.py simulate` bare-run guard (#939, 68 min on #1084), stopping `tools/cleanup_contract.py:173` from auto-deleting other merged PRs' cleanup-record files into file passes (#1099 +5,313 lines), aligning `ci-build.yml:255` 32-bit Clippy flags with `migration.py` (#1098, sharing #1100's bootstrap Full), and `review_start.py` refusing `records` before `recheck` posts (#1096).
- **Work order & plans settled:** #1064 part 4b (7 industry carriers, ~2,000 lines) goes directly after part 4a and before #1062 parts 2 to 4 ([#1064](https://github.com/fetch-laboratories/openttd-rust/issues/1064#issuecomment-7213332719)); all 6 findings of part 4a's second plan review are verified true and must be applied and checked against `#1102`'s base before dispatch. Root's stale `get_asserted` directive on #1060 part 3 is retired (`fleet.rs:1460` already holds it in merge 120).
- **Recorded elsewhere:** #1084 and #1099 closure residue on [#734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-7213328774), P2000 `steady_clock` bimodal jitter on [#275](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-7213330372), Blacksmith spend (`$0.707/merge`) on [#896](https://github.com/fetch-laboratories/openttd-rust/issues/896#issuecomment-7213331262), rule counts on [#939](https://github.com/fetch-laboratories/openttd-rust/issues/939#issuecomment-7213329612), host counts on [#710](https://github.com/fetch-laboratories/openttd-rust/issues/710#issuecomment-7213331947).

For root, on resume:
1. Start housekeeping with the routine agent and refresh #3's status comment.
2. Enqueue **#1102** immediately: its `native` run `45935847797` passed at `0bb80ebbfc` after wind-down, `Quick` `45935780444` is green, and `0bb80ebbfc` is audited sound.
3. Resume **#1060 part 3** on `t3/fleet-transactions-pending-port` (`2b4d5eed8a`) from [its status comment](https://github.com/fetch-laboratories/openttd-rust/issues/1060#issuecomment-7209290879) — do not re-issue the obsolete `fleet.rs` `get_asserted` directive.
4. Apply the six verified blockers from [Plan Review 2](https://github.com/fetch-laboratories/openttd-rust/issues/1064#issuecomment-7209311621) to **#1064 part 4a** (verifying every newly written signature, table link and fixture symbol against the repo), re-read at the base holding #1102, and dispatch 4a after #1102; plan and dispatch **part 4b** directly after 4a and before #1062 parts 2 to 4.
5. Dispatch **#1101 part 1** after #1060 part 3, plan and dispatch **#1100** within merges 120 to 131, run the platform cadence after merge 124, and take stock after merge 131.

Provisional, for steering 42 to judge:
- **The cases.** Read #1090 once 10 passes adding `unsafe_code` records merge after `4ade3941e5` (4 left); read #1072 once 20 PRs have requested `native` after #1079 (2 left); read the landed fixes' numbers for #1073 and #1103; track whether #1104 recurs again. Baseline is steering 41: of 924.4 slot-minutes, editing took 47.3%, `FIX` 31.1%, preflight 17.4%, builds 8.1%, base updates 3.5%, queue 9.9%, blocked speed 0.5%.
- **Codex reviewer truncation and `ci.py report` bypass.** On #1084, `gpt-6.2-sol` read >90% of `orders.rs` in 6.6–8.1 min with 0 compactions, but ~10k-token per-call truncation silently dropped slices (5,445 tok of `Leaves`, 84% of `order_base.h`) while both reviewers still listed them on "Bodies compared"; 10 of 33 Codex reviewers bypassed `ci.py report` via MCP `pull_request_respond`; and 2 findings (`1084-R1-5`, `1097-R2-1`) were verified false. Watch whether truncation or false findings cause regressions on large passes.
- **Per-pass speed recounts (ADR 0054).** #1099 did not recount P2016 after 6 review-fix commits edited per-tick-reachable `fleet/pending.rs::drain` and other P2016 paths; watch compliance on passes whose fixes touch measured hot paths.
- **Reviews of a conflicted base update.** Still 5 of 10 (0 occurred in merges 111 to 120). At 10 with none, the rule goes ([integration](workflow.md#integration)).
- **Ports.** No phase-3 port has been reviewed under these rules; compare the first three of phase 3 line by line with the original ([steer](skills/steer/SKILL.md)).
- **The implementer limit, effort and paid runners** remain the user's ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md), [ADR 0011](adr/0011-model-and-effort-per-role.md), [ADR 0009](adr/0009-paid-runner-assignment.md)); all three held cleanly in merges 115 to 120.

## Ready queue

Undispatched work orders ([team](team.md#ready-queue-and-pipelining)). Each has its own issue, whose body holds its plan once a planner has written it. Root's status comment on #3 holds the queue's state between commits to this section.

Two slots ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)). Each slot works one chain, and a pass planned on an open PR's head is dispatched when that PR merges. Residue that passes left in files they did not finish is on #734, one comment for each pass since [merges 66 to 75](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6976329602), [steering 40's list](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-7187322409) and [steering 41's list](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-7213328774); two small porting points from #1000 wait on `residue/1000-porting-points` ([record](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6992168925)).

In flight at merge 120 (`4f3f8f1620`), paused at wind-down, one chain a slot:

- **NewGRF chain:** **#1102** (#1064's third part: `newgrf_bytereader.rs`, `newgrf_storage.rs`, `newgrf_specs.rs`, `newgrf_translation.rs`), ready at `0bb80ebbfc` with `Quick` and `native` (`45935847797`) green — enqueue on resume.
- **Orders chain:** **#1060's third part** (`fleet/transactions.rs` with `fleet/pending.rs`; closes #1060), mid-port checkpoint `2b4d5eed8a` on `t3/fleet-transactions-pending-port` (takes `TransferCargo`'s final consist update left by #1089).

Planned and waiting on a merge:

1. **#1101, workset 21 — part 1** (the four clock files, planned with [root's split ruling](https://github.com/fetch-laboratories/openttd-rust/issues/1101#issuecomment-7208053608)) follows #1060's third part on the orders chain's slot. Part 2 (the linkgraph files) carries and closes #1032.
2. **#1064's fourth part, first PR (4a: `newgrf_spritegroup.rs` with the resolver frames)** follows #1102 on the NewGRF slot, once [Plan Review 2](https://github.com/fetch-laboratories/openttd-rust/issues/1064#issuecomment-7209311621)'s six verified blockers are applied and re-checked at #1102's landed base.

Waiting on a merge, not yet planned:

3. **#1064's part 4b** (the seven industry and industry-tile carriers, ~2,000 lines, planned at 4a's head) goes directly after part 4a and before #1062's parts 2 to 4 ([steering 41's placement](https://github.com/fetch-laboratories/openttd-rust/issues/1064#issuecomment-7213332719)).
4. **#1062's parts 2 to 4** (workset 16, NewGRF domains; `newgrf_vehicle.rs`, 2,128 lines, has its own pass): follow #1064's part 4b and move the 26 `GRFFile` member slots of three finished definition files to the handle ([root's second answers](https://github.com/fetch-laboratories/openttd-rust/issues/1062#issuecomment-7154092829)). #1064's parts 5 to 7 follow them.

Tools batch for merges 120 to 131: **#1100** (six ranked items, [cleanup work order](#cleanup-work-order)), no plan yet; plan and dispatch within the interval.

Root plans the next entry when a dispatch leaves a chain without one ([team](team.md#ready-queue-and-pipelining)).

The train passes take their sites of #999 (order destination reads at the original's observation point). #998 is the work order for one host-buildable crate below `openttd-kernels`, which removes the copies of three kernels ports that `openttd-strgen` keeps under [root's ruling](https://github.com/fetch-laboratories/openttd-rust/pull/995#issuecomment-6975789181); no second build-time consumer has appeared and it removes no fork C++, so it stays behind the passes, selected no later than strgen's port in phase 4. #997 (the native assertion service reports neither the Rust site nor its predicate) is on #892's checklist and rides the closure residue sweep.

Full phase-3 planning remains untriggered: most of the files still to pass are in no entry.

## Resume checkpoint

Paused at the user's request, 2026-11-27 about 03:50Z.

Root sent each child its stop and wrote this section from GitHub without waiting for their reports. Base is merge 120 (`4f3f8f1620`); its Quick, `Migration baseline validation` and `CI - Build` passed, the platform cadence was last green at merge 119, the merge queue is empty, no merge stop is in force and no rollback happened. Where GitHub is newer than this section, GitHub is right.

| Item | Head | Next action | Running jobs or CI at the pause |
| --- | --- | --- | --- |
| #1102 (#1064's third part: `newgrf_bytereader.rs`, `newgrf_storage.rs`, `newgrf_specs.rs`, `newgrf_translation.rs`), `/root/newgrf-1064-3` | `0bb80ebbfc`, ready, not queued | Its three review reports are in, `0bb80ebbfc` is audited sound, preflight passed 17 of 17, and `native` `45935847797` passed at 03:59Z (`enqueue checks: pass`). **Enqueue immediately on resume.** Its checkout is `t3-5baa26e2-7e725fbe05d322fb`, with its `.local/speed-base` | Quick 45935780444 passed; `native` 45935847797 passed after wind-down |
| #1060's third part (`fleet/transactions.rs` with `fleet/pending.rs`), `/root/fleet-1060-3`, no PR | `2b4d5eed8a` on `t3/fleet-transactions-pending-port`, a checkpoint from `4f3f8f1620` | Mid-port and unbuilt: resume from [its status comment](https://github.com/fetch-laboratories/openttd-rust/issues/1060#issuecomment-7209290879) and the branch, then build, write the two Evidence tests, preflight, open the draft PR, request `native` and start both first reviews. (`fleet.rs:1460` already holds `get_asserted` at merge 120; the obsolete directive is retired.) Its checkout is `t3-232f1c58-c57b57acc5bfed6e` | None |
| #1064's fourth part, first PR (4a: `newgrf_spritegroup.rs` with the resolver frames), plan by `/root/plan-1064-4` | In #1064's body, planned at `c555ebb592`, about 5,300 changed lines | [Plan Review 2](https://github.com/fetch-laboratories/openttd-rust/issues/1064#issuecomment-7209311621) posted six verified blockers. Apply all six (verifying newly written signatures, table links and fixture symbols against the repo), re-read at the base holding #1102, then dispatch after #1102. Steering 41 placed **part 4b** (seven industry carriers, ~2,000 lines, planned at 4a's head) directly after 4a and before #1062's part 2 | Plan Review 2 completed at 03:50Z |

Not started, and root's next actions after them:
- Dispatch #1101's first part (the four clock files, planned, with [root's answer](https://github.com/fetch-laboratories/openttd-rust/issues/1101#issuecomment-7208053608)) after #1060's third part; #1032 rides #1101's part 2.
- #1100 (the six-item tools batch), #1064's part 4b plan and #1062's parts 2 to 4, as the [ready queue](#ready-queue) orders them.
- The platform cadence after merge 124 and the stocktake after merge 131.

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
- P2000 Callgrind counts carry bimodal `±0.45%` wall-clock load noise (`VideoDriver_Null::MainLoop` → `UpdateWindows()` reading `steady_clock`, [#275](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-7213330372)); compare a pass's base and head counts taken under similar load on one day.
- The push hook refuses a push to `rust-migration` while the merge queue holds an entry (#1088).
- Metric scope completeness stays with #736's end-state inventory. No later phase is selected.

Retained state for housekeeping:

- `python3 tools/housekeeping.py --apply` ran at the session's start, with the platform cadences at merges 114 and 119, and after wind-down; `/` has 3.3T free.
- Keep `t3-5baa26e2-7e725fbe05d322fb` (#1102) and `t3-232f1c58-c57b57acc5bfed6e` (#1060's third part, no PR). The merged PRs' checkouts and the pushed review, re-check and plan checkouts can go.
- A name below protects a worktree only when it is the whole folder name ([housekeeping](root-workflow.md#housekeeping)). The `t3-` names of eight hex digits in this list are whole folder names from an earlier host version.
- Keep the locked merge-119 profile checkout `.local/worktrees/root-profile-merge119` until the next profile is recorded; the merge-107 one is released. Preserve all current speed, deep and evidence anchors; the speed anchors live in the main checkout's `.local/speed-pass`, and `20261123-hardware-f80cbaf` there is the pass the current anchors come from. `.local/stocktake-119/` holds the merge-119 stocktake's counts and scripts until the merge-131 stocktake, and `.local/stocktake-107/` is released. Keep `.local/steering-41/` (steering 41's audits, cited by #1072, #1073, #1090, #1103, #1104), `.local/steering-40/` (cited by #1090), `.local/steering-39/` (cited by #1072, #1073) and `.local/steering-38/` (the findings-table definitions they use) until those cases close.
- Keep `t3-f6bd08df`: three unpushed docs commits from 2026-11-10. Keep `t3-f4cf7a0b` (one unique unpushed commit, `125ea2440a`).
- The rest is carried forward as preservation guidance from earlier checkpoints; steering 38 found each dirty tree it names still dirty and did not check the others.
  - Keep the planning checkout `t3-20fec9ce` with its private plan, and origin `preserve/housekeeping-foundation-6ab9c20e5d`. Preserve every checkout with uncommitted or unpushed work.
  - Keep `.local/worktrees/root-profile-125`, `.local/worktrees/root-profile-119` and `.local/worktrees/root-profile-113` with accepted reports, raw Callgrind and frozen runtimes. Keep `t3-a490295e` (profile 131, clean `305ac20c28`) with its HDD `.local` outputs. Profile 119 measures clean `4d3a1d1d41`, not integration 120.
  - Keep main `.local/integration-records/`, `.local/speed-pass/` anchors, frozen runtimes and the retained simulation evidence.
  - Leave dirty or staged trees untouched: `t3-03c1e2da`, `t3-1ddaadfd`, `t3-5575ebe4`, `t3-83082b90`, `t3-c68d914a`, `t3-e0361d8c`, `t3-3b491ddb`, locked #565 WIP `t3-431c84b4`, locked native evidence `t3-d78d5f8b`, `t3-32cf4edc` and `t3-e130d5a3`.
  - Keep protected `.local/worktrees/root-docs-72` and locked `.local/worktrees/profile-after*`. Do not push stale root-docs guidance over current main. Keep the pinned original unchanged.
