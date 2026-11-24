# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place ([records](workflow.md#records)); `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands

Base is at merge 75 (`00494f6c23`). The census, the inventories and the instruction profile below are the merge-71 measurements (`603724a3fd`), taken on 2026-11-18 by routine agents, and the latest; the other column is merge 59 (`f24c05dbe9`), the stocktake before it. That interval took 36 running hours ([who decides](../AGENTS.md#who-decides)). Since merge 71 only the census and #734's files have been measured on the merged tip, at merge 75 ([phases](#phases)): the next stocktake follows merge 83 and the next platform cadence merge 77.

- Fork C++ fell by 1,333 lines, 37 a running hour: 709 of source glue, 472 of original C++ retired, 135 generated and 17 under `tools/`. Rust grew by 7,004 lines and tooling by 1,145.
- **Map generation's instruction count rose 24.6%** between the two profiles, the only scenario that moved, and its projection is 1.775x against the 1.3x ceiling ([speed](#speed)). Every exit save still compares equal. All of it was in the per-tick tile loop, from #988's rewrite of the clear tile loop (merge 62), whose plan measured one play scenario only. #1019 repaired it at merge 75: the scenario is 1.0% below its merge-59 count at that PR's head, a projection of 1.171885x, which the merged tip's next profile has still to confirm; a plan now measures the scenario where its hot body costs most ([planner brief](planner-brief.md#the-plan)).
- #734 has 141 of 319 files finished, holding 105,163 lines. That is 55.1% of the 190,751-line denominator; the workset's files have grown to 205,453 lines, of which it is 51.2%. Nine more finished files, 876 lines, are in no workset. Eight files finished in twelve merges: six of the twelve were shared services, the string IDs and tools, and the passes that did merge each carried the repair of the design faults in its files. The lines left in unfinished files fell from 105,623 to 100,290, 148 a running hour; finished lines rose by 352 a running hour only because finished files also grow. At 148 the 100,290 lines left are about 680 running hours of passes, and at the 181 measured through merge 75 the 98,024 then left are about 540 ([phases](#phases)). Why the pace fell is measured in [phases](#phases).
- The boundary shrank: 1,467 exports and 3,073 callback slots, 34 and 105 fewer.
- #735 is at 115 functions over seven parameters; 70 are FFI exports, which keep positional lists while their C++ callers remain.
- #736 has no transfer left. Fork test glue under `src/tests` grew by 83 lines, from fixture inputs that passes repaired under accepted growth records.
- The one-call-site leaves in finished files are gone for news and script events (#982), windows and sound (#1001) and errors (#1005), with console and chat in review (#1008); Rust has every `STR_*` constant from one generated table (#995), which took the seven string-ID records #982 added and thirteen more (#1006) with it.
- Reviews have run on `gpt-6.2-sol` since merge 66, at the user's word. Its re-checks raise new findings: four of the seven PRs reviewed on it made production or fixture changes after their last stage, and [review](workflow.md#review) now sends those to one scoped behaviour reviewer (provisional).
- The user raised the implementer limit to three on 2026-11-18 from about 15:30Z and returned it to two at about 22:20Z; merges up to 67 ran under two, and merges 68 to 71 under three.
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
| #734 | Lines left in unfinished workset files | 105,623 | 100,290 |
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

- **#734:** reconcile the issue workset manifest with merged complete-file pass PRs, then count current files with `wc -l`; a deleted finished entry remains in the denominator, and mechanical caller edits do not finish a file. The lines left are the `wc -l` of the workset files that `migration/finished-files.txt` does not list; their fall per running hour gives the stream's remainder. Run `python3 tools/code-standards-inventory.py [--head REV]` for the tree and with the finished paths for that subset. #892's sites have no tool: the scan covers `git ls-files 'rust/*.rs'` less test files, `cfg(test)` items, comments and literals, and each stocktake reruns it on the previous tip, since two scans' exclusions have differed.
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

**Repaired at merge 75:** #1019 (for #1014) brought `generate-tgp-256-1` to 2,787,696,015 instructions at its head, 1.0% below merge 59, with all exit saves equal and the six other scenarios 0.0% to 2.0% lower than its base; by the formula that is a projection of 1.171885x, under the 1.235x band. The table below stays as the merge-71 profile measured it; the next stocktake's profile confirms the repair on the merged tip. Generation crossed both its band (1.235x) and its ceiling (1.3x) in one interval. Under [speed policy](speed.md#policy) the rise is attributed per function to the intervening merges and fixed forward; the projection doubles the count's rise and is not a wall-time measurement, so the record's wall pass waits for the fix. The play records are on their floors: no play count is above its anchor's, and the highest play record is 1.444531x against the 1.52x band and 1.6x ceiling. P1996 rose 2.09%, under the 3% line; the next profile reads it again.

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

  For steering 37, the review load:
  - Audit whether the findings of merges 66 to 75 are true, and which of them a pass should settle before its review.
  - Of the 129 review reports on merges 60 to 75, 103 fit the two stages of [ADR 0025](adr/0025-two-review-stages.md), which are two reviewers and one re-check for each group, and 26 came after them. By reviewer name about half of the 26 fit an exception that [review](workflow.md#review) names and about half are a further round, on #976, #982, #995, #1000, #1001, #1008 and #1015. Read those reports.
  - The group size in [review](workflow.md#review) has never been measured. Steering 15 set "about 1,000 original lines" for ports on 2026-10-20 (`dfdf1079ee`), after one reviewer covered #303's 6,985 lines and missed three reachable divergences. Steering 32 carried the number to the changed lines of a cleanup pass (`0cb2f766b9`), and steering 36 gave each group two reviewers (`28bdf3cb42`), on a measurement of reviewer effort taken on `gpt-6.1-sol`. The product, eight first reviewers on a four-group PR, was never decided as a number. A single-file pass does not split, so every large file left gets groups: #1009 plans five.
  - The user, on 2026-11-24, having not been asked when the rule was made: "I don't think there's really any reason to have 8 separate reviewers on a single PR." It is a working rule, not an ADR.
  - Test it on runs that exist: for #1000, #996 and #982, count the true, distinct findings that each group's reviewers raised. If a few groups carry nearly all of them, raise the size or end grouping for cleanup passes; if most groups found real defects, keep it and say so with the counts.
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
- #693: #1015's work traced the `network-protocol-loopback` stalls to a thread race in the original's `TCPConnecter`, which can stay in `Resolving` for ever when its resolver finishes first (#1016, a `deferred-improvement`: neither build changes). The harness bounds its waits and retries a session once on that signature while the candidate's copy of the file is identical to the pinned original's, counting each retry in the run's report; stocktakes read the rate there. The `content-protocol` evidence misses stay open on #693: that scenario runs without network debug output, so its logs cannot show the race. The allowance ends when #730 ports the connecter.
- #503's coverage continuation runs last.
- #1012 records one SIGSEGV in the kernel test binary on CI (#1000's `native` run 44379930797, 2026-11-18) that 50 debugger runs each on that head and on the base did not reproduce. It is unplaced, not a base regression. A second crash, in #1019's merge-group run, was placed by #1015's crash stacks as #1021's test race, whose two commits were already in the first run's binary; the first log cannot confirm it was the same. #1012 closes at the merge-83 stocktake if no crash recurs after #1021's fix.
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

The implementer limit is two again ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)): the user raised it to three on 2026-11-18 from about 15:30Z and returned it to two at about 22:20Z, without stopping any implementer then running. Merged since the stocktake: #1000 (merge 72, `60beebfa21`, #964: `road.rs`, net fork C++ −124; two small porting points wait on `residue/1000-porting-points`, [recorded on #734](https://github.com/fetch-laboratories/openttd-rust/issues/734#issuecomment-6992168925)), #1008 (merge 73, `c7af962916`, #991: console and chat leaves, −70), #1015 (merge 74, `4a9c972310`, #1013: the tools batch) and #1019 (merge 75, `00494f6c23`, #1014: the tile loop's cost). The platform cadence passed at merge 72 (run 44406217846); the next follows merge 77. The two slots hold #1020 (for #1004, the shared framerate service), in review, and #1021 (a race between three industry news tests over one process-wide table, which crashed the kernel tests on CI twice; planned and reviewed twice, about 180 changed lines of test code), dispatched at merge 75 because it fails other PRs' runs.

What #1015 changed for every PR: a growth record or lint acceptance other than `unsafe_code` holds by the record's text across heads, and `ci.py enqueue` names any acceptance missing at the head before it queues; `ci.py` waits for Quick, waits out a run by `--run`, and retries a request refused after a push, during GitHub's mergeability check or on a 5xx; CI prints every thread's stack when a Rust test binary dies by signal, which placed #1021; and `ci.py macos-fallback` runs the paid-runner fallback of [CI administration](root-workflow.md#ci-administration) as one command (a SIGKILL can leave `CI_MACOS_HEAVY` set; `gh variable list` shows it).

1. #984: 7r-2, the single-file pass for `vehicle.rs`, next in workset 07 (vehicle ownership): about 2,050 changed lines and −40 fork C++. Planned and reviewed at `457208977e`, with [root's answer](https://github.com/fetch-laboratories/openttd-rust/issues/984#issuecomment-6978558930) in the plan. Waits on nothing; #1009 and #985 wait on its PR.
2. #1007: one shared accessor for `BaseStation::xy`: about 200 changed lines and −11 fork C++. Planned and reviewed at `5075f87cad`. Waits on nothing.
3. #993: the 107 string-ID fields left in 25 tables: about 1,000 changed lines and −163 fork C++. Planned and reviewed at `8869d55587`. Waits on nothing.

**Later.** Waiting on #984's PR: #1009, 8t, the single-file pass for `train.rs` (about 5,800 changed lines and −390 fork C++, five review groups; planned at high effort against #1000's head), and #985, 7r-3, the pass for `vehicle/maintenance.rs` (about 2,800 changed lines and −240; [root's answer](https://github.com/fetch-laboratories/openttd-rust/issues/985#issuecomment-6978554257)). After those: #986 (`vehicle/scheduler.rs`, about 2,080 changed lines, with the day virtuals' dispatch moved into Rust; after #1004, #984 and #985), #1003 (`vehicle/occupancy.rs`, waiting on #986's PR, not yet planned), #987 (`vehicle/visits.rs`, about 2,300 changed lines and −90; after #984 and #985) and #1010, 8tr, the single-file pass for `train_reservation.rs` (about 2,000 changed lines and −300; after #1009's PR, its plan refreshed and reviewed again at dispatch). Whichever of #1009 and #987 merges first moves the train case of `VehicleEnterDepot` into `vehicle/visits.rs`. #1011 moves `Vehicle::Tick`'s dispatch into Rust with the four controllers' accumulators; it waits on #1004, #986, #1009 and the `ship_control.rs` pass. The rest of workset 08 is one pass for `effect.rs`, `vehicle_effect.rs`, `ship_control.rs` and `disaster.rs`, which its planner splits if it must; it has no issue yet. After worksets 07 and 08 the fan-in order gives 06, 04 and 05. The train passes take their sites of #999 (order destination reads at the original's observation point).

#998 is the work order for one host-buildable crate below `openttd-kernels`, which removes the copies of three kernels ports that `openttd-strgen` keeps under [root's ruling](https://github.com/fetch-laboratories/openttd-rust/pull/995#issuecomment-6975789181). Root reconsidered it at this stocktake: no second build-time consumer has appeared and it removes no fork C++, so it stays behind the passes, selected no later than strgen's port in phase 4. #997 (the native assertion service reports neither the Rust site nor its predicate) is on #892's checklist and rides the closure residue sweep.

Full phase-3 planning remains untriggered: about 160 of #734's 319 files are in no entry.

## Resume checkpoint

**Paused at the user's request (2026-11-19, about 01:50Z), at merge 75 (`00494f6c23`).**

Root resumed at merge 65 on 2026-11-18, merged ten PRs (merges 66 to 75) and took stock at merge 71. No rollback happened and no confirmed base regression is open: the base's Quick, build and baseline runs passed at merge 75. The merge queue is empty. No implementer, planner or reviewer is running; root stopped both implementers after their checkpoint pushes. No implementation, repair, review, build, simulation, CI request or retry is authorized until resume. Where GitHub is newer than this section, GitHub is right.

Agent: /root | Model: claude-fable-5-2 | Reasoning effort: high

| Open PR | Pushed head | State | Local jobs left running, not awaited | CI | Next action at resume |
| --- | --- | --- | --- | --- | --- |
| #1020, for #1004, the shared framerate service | `e203d2cb41`, a checkpoint push; draft | Both reviews are in and their fixes are committed: the porting fixes (`2a1213aa17`), a base merge that takes #1019, the tile-loop fix root ruled into this PR (`5bf8f13712`) and one native fixture's map input (`e203d2cb41`). No re-check has run | `bk9u1ji4o` (verify, preflight, then the head's instruction counts) in `t3-73b6f65a-4508fb87886c846c`; `b4f6zuecx` (base counts at `2d8bef1686`) in that checkout's scratchpad `base` worktree. Its status comment names their logs and reports | Pending or unknown at the checkpoint head. The last `native` run, 44430778604, passed at `8f818d8d3b`, an older head | Read those counts or rerun them (stop for root if `generate-tgp-256-1` or a play scenario is more than 0.5% up against a base with #1019); then verify and preflight if unfinished, one re-check of the fixing commits with both review reports, `native`, the six new `unsafe` acceptances and three growth records, and the queue |
| #1022, for #1021, the test race over process-wide tables | `9503a1423d`, a checkpoint push; draft | Five commits, the plan's whole scope; no review has started | `b0q1e2lja` (preflight), which failed only on `cargo fmt` before the format commit | Pending or unknown; no CI was requested | Run preflight at the head, then the behaviour and porting reviews, `native` and the queue. Until it merges, any PR's kernel tests can crash on this race (#1012) |

On resume the implementer limit is two ([ADR 0049](adr/0049-two-implementers-waiting-keeps-slot.md)); the user's three-slot and high-effort allowances of 2026-11-18 ended that day. Root starts one fresh implementer for each open PR from its status comment, since both hold slots, and dispatches nothing else until one merges; the [ready queue](#ready-queue) then gives #984, #1007 and #993.

Implementation runs on `claude-opus-5-5`, medium, planning on `claude-fable-5-2`, medium, routine work on `claude-sonnet-5-5`, medium, and review on `gpt-6.2-sol`, high.

`CI_MACOS_HEAVY` is unset. The hooks of #927 are installed in the shared clone (`python3 tools/git_hooks.py install`).

| Other work | State |
| --- | --- |
| Ready queue | As the [ready queue](#ready-queue) above has it, except that its two slot-holders are the draft checkpoints in the table above. Root's status comment on #3 has the session's notes. |
| Tools batch | #1013 merged as #1015 (merge 74), the batch of the interval that starts at merge 72. Candidates for the next are on #939, each with its count. |
| Platform cadence | Passed at merge 72 (run 44406217846). Next after merge 77. |
| Stocktake | Measured at merge 71. Next after merge 83; it profiles the merged tip, which confirms or refutes generation's repair, and closes #1012 if no crash has recurred since #1022 merged. |
| Phase-3 planning | Not triggered. |

Flakes and limits carried forward:

- #693: the `network-protocol-loopback` stalls are the original's `TCPConnecter` race (#1016); from merge 74 the harness retries a session once on that signature and counts the retry in the run's report. The `content-protocol` evidence misses (two on 2026-11-18, one reference side and one candidate side) are still unexplained. Keep `.local/retained-network-693` and the linked artifacts.
- Hosted macOS runners had no capacity in four runs on 2026-11-18 and 2026-11-19: two finished on the paid fallback, one on a rerun and one was superseded by a new head. `ci.py macos-fallback` has run only against a stubbed `gh`: after its first real use, check `gh variable list`.
- The profile driver exits 1 on its anchor identity guard until a quiet pass re-anchors (#275), and deletes `candidate-runtime/` after every run.
- Root's status comment on #3 was the only carrier of state between roadmap commits; several child reports and one implementer's question did not reach root on 2026-11-18, and root read those results from the issues and PRs (#710).
- Metric scope completeness stays with #736's end-state inventory. No later phase is selected.

Retained state for housekeeping:

- `python3 tools/housekeeping.py --apply` ran four times in the session of 2026-11-18 and again at this wind-down; root's status comment on #3 has each run's result. Three implementers with their reviewers' checkouts took the SSD from 66 GB free to 18 GB in about twelve hours.
- A name below protects a worktree only when it is the whole folder name ([housekeeping](root-workflow.md#housekeeping)). The `t3-` names of eight hex digits in this list are whole folder names from an earlier host version.
- Keep the open PRs' checkouts: `t3-73b6f65a-4508fb87886c846c` (#1020), with the scratchpad `base` worktree its status comment names until its counts are recorded, and `t3-8b132336-39dac24543a85a08` (#1022).
- Keep the locked merge-71 profile checkout `t3-612e96b6-a878391da496d7f7` and the locked merge-59 one, `t3-7739ec11-3073cbaa77b28444`, until the next profile is recorded. Preserve all current speed, deep and evidence anchors; the speed anchors live in the main checkout's `.local/speed-pass`.
- Keep `t3-f6bd08df`: three unpushed docs commits from 2026-11-10.
- Six older profile checkouts were released on 2026-11-12 (#924); this file no longer names them, so the next housekeeping run removes them.
- The list below is carried forward unchanged as preservation guidance from earlier checkpoints; it does not assert that every old checkout still exists. `/steer` may reconcile stale references, preserving uncommitted or unpushed work and evidence.

- Keep the planning checkout `t3-20fec9ce` with its private plan, and origin `preserve/housekeeping-foundation-6ab9c20e5d`. Preserve every checkout with uncommitted or unpushed work.
- Keep `.local/worktrees/root-profile-125`, `.local/worktrees/root-profile-119` and `.local/worktrees/root-profile-113` with accepted reports, raw Callgrind and frozen runtimes. Keep `t3-a490295e` (profile 131, clean `305ac20c28`) with its HDD `.local` outputs. Profile 119 measures clean `4d3a1d1d41`, not integration 120.
- Keep main `.local/integration-records/`, `.local/speed-pass/` anchors, frozen runtimes and the retained simulation evidence.
- Keep `t3-f4cf7a0b` (one unique unpushed commit, `125ea2440a`). Leave the staged 11-08 trees `t3-03c1e2da`, `t3-1ddaadfd`, `t3-5575ebe4`, `t3-83082b90`, `t3-c68d914a` and `t3-e0361d8c` untouched.
- Leave dirty or staged trees untouched: `t3-3b491ddb` (five files), locked #565 WIP `t3-431c84b4`, locked native evidence `t3-d78d5f8b`, `t3-32cf4edc` (`tools/simulation/roads.py` staged) and `t3-e130d5a3` (`migration/saves/README.md` staged).
- Keep protected `.local/worktrees/root-docs-72` and locked `.local/worktrees/profile-after*`. Do not push stale root-docs guidance over current main. Keep the pinned original unchanged.
