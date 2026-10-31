# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-10-31, paused after integration 47)

Agent: /root (Claude Code steering review) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host.

- Root stopped at the user's request at 17:00 UTC. Code is `2a89fb023f` (integration 47); every later commit is docs only. Integrations 31 to 47 are listed under "Recent completed work". There were no rollbacks.
- The whole-fork position at `2a89fb023f` is the table in "Phases": 53,182 of 284,355 original code lines retired (18.70%), compiled C++ 265,246 (93.28%), glue 34,073, 1,377 exports and 4,206 callback slots. The duplicate baselines hold 581 rebound-callee rows, 128 shared-accessor rows and 55 orphan exports. The last root stocktake was after integration 46; the next is due after 48.
- When #607 integrates, original retirement will drop by about 815 code lines and glue by about 816, through diff matching, while compiled C++ stays the same. Record that as a re-baseline, not a regression (#607 comment).
- Speed: profile 46 ran on merged `a60ef2e616`. Every carried record is below its band, so no pass is due. P2016's single-anchor projection does not replace its carried record. The next profile is due after 48.

| Scenario | Candidate Ir, profile 46 | Projection | Carried three-pass record |
| --- | ---: | ---: | ---: |
| TGP | 2,715,619,270 | 1.111722x | 1.154106x |
| Grok | 26,869,118,791 | 1.387147x | 1.385653x |
| Opus | 28,273,410,089 | 1.393949x | 1.394556x |
| P1996 | 4,537,889,943 | 1.336551x | 1.337764x |
| P2000 | 7,385,372,083 | 1.400577x | 1.401446x |
| P2016 | 13,361,928,672 | 1.522911x | 1.481910x |

- The integration-45 platform cadence passed on every platform; the next is due after 50.
- Coverage: the original-coverage measurement retains 330 of 449 traces, which is not a completed coverage result. #156 holds the observation gaps, including this review's. #503's continuation comes last. Deep checks run only on the user's request.
- Six drafts are open, the branch limit: #603, #607, #612, #616, #618 and #620. Their next actions are in the checkpoint.

## Phases

Agent: /root (Claude Code navigation) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host.

The goal is a complete Rust rewrite ([goal and phases](planning.md#goal-and-phases)). Each free
slot takes ready work from the earliest phase, and later phases fill the rest
(user decision, 2026-10-28). Position at code `2a89fb023f` (integration 47) in
code lines from `python3 tools/port-metrics.py --whole-fork`, with the change
since the previous navigation's inventory at `06a644505a` in parentheses.
Source areas are an accounting classification: they do not move drawing or
executable header logic into a different phase.

| Area | Original | Retired | Left | Glue added | Compiled C++ |
| --- | ---: | ---: | ---: | ---: | ---: |
| Simulation logic | 47,424 | 37,571 (+1,827) | 9,853 | 6,974 (+276) | 16,827 (-1,551) |
| Simulation storage | 13,724 | 2,717 (+74) | 11,007 | 1,335 (+23) | 12,342 (-51) |
| NewGRF | 10,785 | 5,280 (+62) | 5,505 | 428 (+5) | 5,933 (-57) |
| Save/load | 13,388 | 536 | 12,852 | 585 | 13,437 |
| Script API | 14,908 | 878 | 14,030 | 453 | 14,483 |
| Network | 12,713 | 211 | 12,502 | 182 | 12,684 |
| Support | 21,617 | 982 (+317) | 20,635 | 513 (+89) | 21,148 (-228) |
| Graphics and viewport | 10,985 | 4 | 10,981 | 1 | 10,982 |
| GUI | 52,179 | 117 | 52,062 | 165 | 52,227 |
| Platform | 11,941 | 0 | 11,941 | 2 | 11,943 |
| Data tables | 17,295 | 4,364 | 12,931 | 0 | 12,931 |
| Vendored | 55,148 | 522 | 54,626 | 163 | 54,789 |
| Upstream tests | 2,248 | 0 | 2,248 | 5,118 (+435) | 7,366 (+435) |
| Boundary layer | 0 | 0 | 0 | 18,154 (+657) | 18,154 (+657) |
| **Whole fork** | **284,355** | **53,182 (18.70%, +2,280)** | **231,173** | **34,073 (+1,485)** | **265,246 (93.28%, -795)** |

The boundary has 1,377 Rust exports (+80) and 4,206 callback slots (+132).
Rust totals 124,225 code lines (+6,446) and tooling 51,310 (+3,636).
Simulation-source retirement is 40,288 of 61,148 (65.89%). The duplicate
baselines hold 581 rebound-callee rows (585 when the gate landed), 128
shared-accessor rows (206 at the start) and 55 orphan exports (62).

| Period (steering and navigation commits) | Days | Retired | Per day | Compiled C++ change | Per day |
| --- | ---: | ---: | ---: | ---: | ---: |
| Fork start to 10-09 | 7.0 | 23,244 | 3,330 | -10,321 | -1,480 |
| 10-09 to 10-21 | 12.1 | 8,971 | 740 | +1,408 | +120 |
| 10-21 to 10-27 | 6.4 | 17,321 | 2,720 | -8,995 | -1,410 |
| 10-27 to the first navigation (10-28) | 1.2 | 575 | 470 | +92 | +80 |
| First to second navigation | 0.45 | 791 | 1,760 | -498 | -1,110 |
| Second navigation to now | 2.4 | 2,280 | 950 | -795 | -330 |
| **Whole history** | **29.6** | **53,182** | **1,800** | **-19,109** | **-650** |

Days are calendar days, including pauses; root ran for about one day of the
last 2.4. Three things set the last period's rate:
- Retirement per integration fell from about 390 lines (integrations 29 to 32)
  to about 140 (39 to 47), and the four drafts now open hold 34 to 88 each.
- Glue rose 65% as fast as retirement: the boundary layer grew 657 lines and
  native test fixtures 435. So compiled C++ fell 795 lines while 2,280 original
  lines retired. Moving records in phase 2 deletes most per-field services,
  and phase 2 has not started.
- Tooling grew 1.6 lines per retired line.

Why the course changed:
1. Phase 1's exit criterion and the planning trigger in `/start-development`
   (fewer than three unstarted owners in the earliest unfinished phase) kept
   every slot on phase 1 after its large owners ran out. Of the 9,853 lines left
   in its bucket, about 4,700 are drawing and description (phase 5) and 510 are
   startup (phase 4). About 1,700 are three- and four-line facade shells
   (signature and braces) that retire with their C++ callers. About 2,250 are
   other non-drawing bodies in about 200 functions across 51 files; the rest
   are file-scope declarations (about 350), `command.cpp` (151, #559) and
   `tilearea.cpp` (172, #69). #621 makes this inventory a tool. Planning cut
   owners of 11 to 88 lines from that tail. Meanwhile phase-3 owners sized at
   1,000 to 2,900 retired lines each (#573, #574, #575) have been queued since
   10-29. Every owner pays
   the same issue, review and exact-head Full cost. Phase 1 now closes with its
   open drafts and the two excluded items whose return condition is its exit.
   Its remaining bodies move with the phase-2 owner of their files.
2. #581 PR-B (#607) was the previous navigation's first action and is phase 2's
   only prerequisite, yet it is still a draft. Integration in readiness order
   let eight integrations pass it on 10-31. Each moved base and forced a new
   sweep and deviation proof, and the checkpoint then queued it behind #603 as
   well. A sweep across `src/` now integrates before other `src/` changes
   ([integration](workflow.md#integration)).
3. #566's drawing evidence gates #570's integration, but the work order started
   it only after PR-B, which it does not need. It starts now.

| Phase | Left | Exit criterion | Status |
| --- | --- | --- | --- |
| 1. Simulation logic | About 2,250 lines of non-drawing bodies in the logic bucket, plus #559's command invocation (`command.cpp`, 151) and #69's tile areas (`tilearea.cpp`, 172) | Its open drafts, #559 and #69 integrate. Every other remaining non-drawing body belongs to the phase-2 owner of its file | Closing. #612, #616, #618 and #619 are in draft. #559 and #69 return now: their recorded condition is phase 1's exit. No further phase-1 owner is planned |
| 2. Simulation storage | 11,007 in the storage bucket, plus the phase-1 bodies left in each pool's files | Every pool record and the map arrays are defined once in Rust, keep the original pool, lifetime and save behavior ([world-state design](design/world-state.md#transferring-shared-storage)), and are read in place by both languages. C++ calls Rust only for operations, and no per-field service or per-call-site reader remains for moved data. Each pool owner also ports the remaining non-drawing bodies in its files | PR-B (#607) first, then #570 with #566. Pools in order: vehicles (#570, then #598 and #599); stations and road stops; towns and industries; companies, groups, engines, link graph and the smaller pools; orders; then the map with the rail, road, water, tunnel, landscape and tile bodies |
| 3. Simulation data readers | 41,096: save/load 12,852, script 13,099, network 9,640, NewGRF 5,505 | These run in Rust; the Squirrel VM may stay C++ until phase 6 | #573, #574, #575, #497 and #498 are queued and ready. They take every slot phases 1 and 2 leave free |
| 4. Support | 31,616 support and graphics lines, plus startup in `openttd.cpp` (510); data tables (12,931) retire with their consumers | Not planned yet | Not planned until it is the next phase (user direction, 2026-10-28) |
| 5. Interface | 67,796 GUI and platform lines including script and network GUI, plus about 4,700 drawing and description lines in the logic bucket | Not planned yet | As above |
| 6. Vendored | 54,626 | Not planned yet | As above |

Upstream unit tests (2,248) and fixture glue retire with the code they test.
Boundary-layer glue retires as its C++ callers do.

Root's first actions from this chart. They override the plan-after47 input's
#603-before-#607 order and its phase-1 rankings:
1. #581 PR-B (#607) integrates before any other `src/` change, #603 included:
   finish its current-base sweep, replay and deviation proof, then bootstrap
   Full. Other drafts keep implementing and reviewing, then adopt it at their
   base update. #621 changes only `tools/` and may integrate meanwhile.
2. Start #566 in the next free slot. Start #570 when PR-B integrates.
3. Finish #612, #616, #618 and #619, and select #559 and #69 as phase 1's last
   owners. The plan-after47 phase-1 candidates (link-graph job lifecycle, elrail
   transition, land count) join the phase-2 owners of their files.
4. Fill every other free slot with phase 3 (#573, #574, #575, #497, #498),
   ranked under [selection](planning.md#selection).
5. Size new owners to repay their review and Full ([selection](planning.md#selection)),
   and record the compiled-C++ rate at each stocktake (`/start-development`, step 4).
6. Collapse glue duplication where it lives longest: after PR-B, #622 takes
   the NewGRF specification and shared-operation bindings, about 332 of the
   526 rebound rows, which stay compiled until phases 3 to 5. The rest bind
   record fields, which go when phase-2 owners move those records, so do not
   tidy them first. Each stocktake records the baseline sizes.

## Twenty-fifth steering review (2026-10-31: the long-job relay, repeated sweep proofs and evidence that names nothing)

Agent: /root (Claude Code steering review) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host.

**What was audited.** Six independent read-only audits ran, with no project builds or games. The steering checked every claim acted on below against the source.
- Fidelity, line by line against the pinned original:
  - #607, the sweep, which integrates next;
  - #588: #578's goals, story pages and league tables, and #580's clock batch;
  - #594's setting transitions;
  - #593's heightmap and town-data import.
- Targeted: the shared k-d tree (#608 integrated, #612 in draft), and the process cost of integrations 39 to 47.

**No reachable divergence on main.**
- Random draws, widths, saturation and side-effect order match in every audited body.
- #608's Rust town tree visits in a different order from the native tree. Every town consumer calls only `nearest`, which ignores order.
- One case is compiler-dependent and goes to #156 with the coverage gaps: the counter-clockwise town-data float conversion, which is undefined behavior in the original.

**1. Root relayed its children's long jobs.**
- Codex children briefed before 10:31 were woken by their own `exec_command` jobs: 49 completions, including industry601's verify and metric runs.
- From 10:41, root's briefs told children to hand long jobs to root. The reason was one child's claim that Codex children get no wakeup.
- Root then ran about 43 of its children's jobs; 21 of them exited nonzero. Each took at least two root turns, about 86 of its 370 messages.
- station602 sat idle for 71 minutes across seven relays.
- Root cause: AGENTS.md named only Claude Code's mechanism.

**2. #607 repeated a full proof at every base update and never reached Full.**
- On 10-31 it had six base merges. Each resolved conflicts, reswept, replayed 1,810 paths and re-proved eight deviations. Five also recounted metrics, and three compiled or reran tests.
- The workflow's condition needs only a rerun of the sweep.
- The audit found that the saved head matches a clean re-sweep of its merge base, and base has since moved only in docs. One docs-only merge and a bootstrap Full integrate it.

**3. Evidence plans listed coverage matrices instead of naming what existing checks miss.**
- [Cost decisions](planning.md#cost-decisions) require the naming.
- #613's plan listed every vehicle kind, climate and factor extreme. It estimated 60 to 120 tooling lines and ended at 352.
- Integrations 39 to 47 added 1,858 tooling lines and about 315 fixture lines for 1,241 lines retired.
- #612's visit-order check compares the same `nth_element` on both sides, so it matches by construction.
- Root accepted each plan at its first checkpoint.

**4. Design faults the reviews missed.**
- #612's partition calls Rust on every comparison, and that Rust code calls C++ for station coordinates (PR comment).
- #578's story-page deletion bounces from Rust through the C++ destructor back into Rust (#199).
- World generation keeps per-tile height, inner-tile and void services. #593 and #594 touched that code and left them (#565 item 2).
- #593 added carriers, each serving one call site, for libpng, nlohmann and BMP data. About 45 of its 278 "retired" lines survive as that glue.

**5. Roadmap commits moved base 24 times in ten hours.**
- Root committed 24 roadmap updates to base, besides the pause, after almost every integration and selection.
- Each push moved base under PRs in final validation. The nine integrated PRs needed 23 base merges between them, and three of the four Full failures came from combining with base.

Apply these corrections through the work order below:

1. **Long jobs** ([AGENTS.md](../AGENTS.md#every-agent), [team](team.md#spawning)). In Codex, root or child, an agent starts a long job with `exec_command` and ends its turn. No agent detaches a job or hands one to root, and root's briefs drop the relay.
2. **#607 first, in one base update** (PR comment, work order 1).
   - Merge the docs-only base under review condition (a), let the cheap checks pass, run the bootstrap Full, and merge.
   - If `src/` moves first, regenerate it with the recorded sweep and the stored deviation patch instead of resolving conflicts.
   - A base update that meets a review condition needs only CI ([review](workflow.md#review)).
3. **Evidence names its behavior, checked at review** ([review](workflow.md#review)).
   - For each new scenario, fixture or checker, the reviewer names the behavior that existing checks cannot distinguish. An item with none is deleted.
   - Provisional. Basis: 1.5 tooling lines per retired line in integrations 39 to 47.
4. **Fault routing.** Fix #612's bounce before it integrates. #199 and #565 carry the integrated faults, and #156 the gaps.
5. **Roadmap commits only at stocktakes, course changes and wind-down** (`/start-development`, work loop step 2). Selections and integrations go on their issues and PRs, and resume also reads each open PR's status comment.

Template watch:
- Phase 2's station-pool and map ports do not carry #612's raw visit-order evidence. They change neither the tree nor its split.
- Phase-3 readers that use libraries staying native, such as zlib, LZMA or sockets, should not repeat #593's carriers, each serving one call site.

No user direction is reversed. The restart prompt changed: work loop step 2, and resume step 5.

| Twenty-fourth correction | Verdict and cost/result |
| --- | --- |
| 1. Finish the five drafts with the audit fixes | Applied: #578 integrated in #588 (31), then #579 (32), #576 (38) and #568 (39). #568 took 63 hours from issue to merge, with nine base merges. Its review missed #578's story-page bounce, now on #199. |
| 2. #581 in three PRs | PR-A (#583, 33) and PR-E (#591, 35, on emsdk 4.0.23) applied. PR-B (#607) slipped: started on 10-31, then six base merges, each with a full proof, and no Full (finding 2). |
| 3. The map view costs a load | Applied in #606 (46): one `World` type, loads and native assertions, with production net glue -4 and fixtures +16. Item 2's deletions (-300 to -600) have not started; generation's services joined its scope. |
| 4. #582 tooling batch | Applied (#590, 34). The rebound baseline fell only from 585 to 581 rows, and #612's review still fixed a duplicate binding. Housekeeping's cwd fix works (T3 worktrees fell from 82 to 41 GB), but finished review and merged checkouts stay locked. |
| 5. #199 batch after #582 | Slipped: #603 is reviewed and validated but waits behind base and #607. |
| 6. Speed after integration 30 | Applied and effective: profile 30 found Grok +13.7% and Opus +12.0%, and #606's repairs brought them to 1.387x and 1.394x at profile 46. |

| Change since the twenty-fourth review | Verdict and next observation |
| --- | --- |
| Opus writes code at medium effort (user, 2026-10-30) | Applied in the Claude Code root sessions; the Codex root used gpt-6.1-sol high. No defect traced to it. |
| Deep checks only on the user's request (user, 2026-10-31) | Applied: the killed rotation was not restarted. Its two worktrees (64 GB) remain. |
| Guidance routed from AGENTS.md to task docs (#597, user-directed) | Applied. Its background rule, which named Claude Code only, is finding 1. |
| Navigation of 2026-10-31 (three commits) | Untested; no root session since. Watch whether #607 lands before other `src/` changes, whether #566 starts, whether phase 1 closes with #559 and #69 and no new owners, and whether stocktakes record the compiled-C++ rate. |

New since this review, for the next steer to judge:
- Whether Codex children run their own long jobs, and how many root turns each integration takes.
- Whether #607 integrates after one base update.
- Tooling and fixture lines per retired line, against 1.5 for integrations 39 to 47, and whether a defect is ever found only by evidence the review rule would delete.
- Roadmap pushes per integration, against about 2.7 for integrations 39 to 47.

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
only on the user's request; active speed dependencies are preserved. The parked
NewGRF corpus stays excluded with the return condition below. AGENTS.md and its
routing table carry the rules; issue and PR bodies carry current actions and
evidence.

## Recent completed work

Rows cover the period since the twenty-fourth steering review; earlier rows are
in their PRs and Git history. Metrics are `tools/port-metrics.py` code lines:
Rust / tooling / original C++ retired / net glue.

| Integration | Issues / PR | Merge commit | Metrics |
| --- | --- | --- | --- |
| 31 | #572 goals, story and league (#578), #168 clock batch (#580), #589 / #588 | `44cb5f49f5` | 1,133 / 376 / 388 / +264 |
| 32 | #561 company identity, faces, colours and liveries / #579 | `0de459cbe1` | 995 / 205 / 373 / +315 |
| 33 | #581 PR-A, Rust by default and on Linux CI / #583 | `d531622d32` | 0 / 0 / 0 / 0 (CI and CMake) |
| 34 | #582 rebound-callee gate and housekeeping cwd fix / #590 | `29b65dd186` | 0 / 964 / 0 / 0 |
| 35 | #581 PR-E, Emscripten links Rust on emsdk 4.0.23 / #591 | `69f02851be` | 0 / 21 / 0 / 0 |
| 36 | Clang missing-initializer fixes in `src/tests` / #595 | `063e7b28f1` | 0 / 0 / 0 / +18 |
| 37 | #584 heightmap input and town-data import / #593 | `b74888c4bc` | 554 / 389 / 278 / +193 |
| 38 | #565 movement terrain and water entries in place / #576 | `e48846d8fd` | 546 / 61 / 0 / +15 |
| 39 | #562 station naming, tile loop and lifecycle / #568 | `4733594d75` | 983 / 248 / 295 / +70 |
| 40 | #587 simulation-setting transitions and commands / #594 | `b01e7fd5c8` | 726 / 180 / 322 / +167 |
| 41 | #585 NewGRF town-name state, generation and ActionF / #596 | `a2429cda1f` | 393 / 209 / 190 / +37 |
| 42 | #586 sign commands and depot naming / #605 | `73c5a4797c` | 230 / 260 / 76 / +128 |
| 43 | #600 town spatial index and policy / #608 | `5345e29939` | 699 / 112 / 120 / -16 |
| 44 | #601 industry servicing and policy / #609 | `16e0235d0d` | 309 / 158 / 105 / +37 |
| 45 | #610 company control and ownership checks / #611 | `1dd6d32544` | 393 / 181 / 70 / +171 |
| 46 | #565 canonical map/movement loads and speed repair / #606 | `a60ef2e616` | 524 / 158 / 0 / +12 |
| 47 | #613 remaining engine runtime decisions / #614 | `2a89fb023f` | 316 / 352 / 63 / +76 |

## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and the disjoint-service rule; finishing review and integration
comes first. A lower rank can run while a higher one waits. Scopes live in issues.

First: **#621**, the per-function remaining-code inventory in
`tools/port-metrics.py` (navigation, 80 to 150 tooling lines). It changes only
`tools/`, so it may integrate while #607 holds `src/`. Planners use it to size
owners and phases.

1. **#607 (#581 PR-B) integrates before any other `src/` change** (Phases, action 1). The steps are in its PR comment of 2026-10-31: merge the docs-only base, let the cheap checks pass, run the bootstrap Full, merge, and restore `CI_ON_DEMAND`. Other drafts keep implementing and reviewing. At their base update they rerun the recorded sweep over the `src/` files they touch.
2. **Finish the open drafts after #607**, in readiness order. No reviewed draft gains scope, but a fix the audits found is not new scope.
   - #603 (#199 batch): base update, then Full. Do not expand it into session or `CargoFlow` work.
   - #612 (#602 station spatial queries): fix the partition bounce (PR comment), get a scoped review, then base update and Full. Keep the approved native STL primitive and the native `GetTileArea` selectors.
   - #616 (#615 clear ground and editor tree groups): repair the Full failure at `src/tests/town_growth.cpp:20`, which comes from combining with integration 47. A behavior fix gets a scoped review. Then Full.
   - #618 (#617 snow line and water borders): fresh full fixing review, then Full.
   - #620 (#619 road bits and crossings): continue the implementation.

   Then come **#559**, command invocation, and **#69**, tile areas: phase 1's last owners. No further phase-1 owner is planned. Every other remaining non-drawing body moves with the phase-2 owner of its file, including plan-after47's link-graph job lifecycle, elrail transition and land count; #621 lists them.
3. **#566 drawing workload now; #570 shared vehicle records after #607.**
   - #566 takes the next free slot, since it does not need #607. It records reference and candidate pixels and drawing Ir before storage integrates (150 to 250 tooling lines).
   - #570 follows the [world-state design](design/world-state.md). It deletes the replaced readers, the direct Tick bounces and their baseline rows, and makes `tools/port-metrics.py` count the C++ declarations it generates. Estimated net glue is -500 to -1,100; its 0.86% instruction credit is a model.
   - Then, one at a time in the shared vehicle files: #598 vehicle routing decisions, then #599 vehicle base lifecycle. Remeasure #599's inventory after #570. The airport automata follow #570's declaration generator.
4. **One maintenance batch at a time** (one reviewer, one Full):
   - #622 after #607: it collapses duplicate NewGRF specification and shared-operation bindings, about 332 of the rebound rows. Estimated net glue -200 to -500.
   - #565 item 2: delete the map fallbacks and convert the remaining rail, crossing, depot, stop, water and tunnel leaves, `tile_slope_z`, and world generation's per-tile services (issue body). Production net glue -300 to -600.
   - #199's open items: #569's copies (in #603), the `CargoFlow` bounce, #578's story-page bounce, #580's date parameters and the sweep's stale comments.
   - Later: #604 shared settings access, after #607 and #570's generator; then #168/#199/#500 completed-job publication and #501 probe typing.
5. **The next phase-2 pool after #570 integrates**, in the order of "Phases": stations and road stops (1,604 storage lines, about 600 callback slots); towns and industries (1,295, about 344); companies, groups, engines, link graph and the smaller pools (952, about 286); orders and order lists (390, about 111); then the map (3,988) with the rail, road, water, tunnel, landscape and tile bodies. Each pool gets an issue and a design section when selected. It moves its coupled save adapters with each record, generates native declarations, verifies the layout, and deletes the displaced readers, services and their baseline rows.
6. **Phase 3 fills every other free slot:**
   - #573 codec and chunk execution, #574 game network protocol and queues, and #575 script host, commands and events, each with its unsaved-output evidence;
   - then #497 with its engine and feature binding payloads, and #498 with its ObjectSpec payloads.

   Size each owner to repay its review and Full ([selection](planning.md#selection)). Plan no Squirrel port.
7. **#503 coverage continuation last:** first the bounded clock-preparation and sanitized native-test-support build fixes, then the 119 unfinished traces with provenance checks. Do not recollect just to fill a slot.

Recurring:
- After every second integration (next after 48), take stock under `/start-development` step 4, including the compiled-C++ rate and the baseline sizes. Profile the merged tip with the pinned `LD_LIBRARY_PATH` from [speed](speed.md#instruction-projections).
- Run the platform cadence after every fifth integration (next after 50). Fix a red cadence before further integrations.
- Deep checks run only on the user's request (user, 2026-10-31).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions.

Excluded, with return conditions checked by this review:
- #507's NewGRF corpus (#525, #526) stays parked by the user (2026-10-28) until the user reselects it. #525's branch stays on origin.
- #64 and #68 become maintained Rust crates that reproduce the game's observable cryptography. They return when phase 3's network ports reach authentication, or with phase 6, whichever comes first.
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.

## Resume checkpoint (2026-10-31, paused after integration 47)

Paused at the user's request; resume only on the user's instruction. No root
job, build, simulation or CI wait is running. Base Quick passed at the pause
commit. `CI_ON_DEMAND` is unchanged. The six drafts' Quick and commit checks
passed at their saved heads. None has a current-head Full: #616's failed, and
the others show only pending placeholders.

| Issue / draft | Saved head | Next action | Checkout |
| --- | --- | --- | --- |
| #581 / #607 | `8bb6678cea` | Work order 1: docs-only base merge, cheap checks, bootstrap Full, merge | `t3-4076556a` |
| #199 / #603 | `f2731de370` | After #607: base update, then Full | `t3-d31fc312` |
| #602 / #612 | `a5bd61f93d` | Fix the partition bounce and get a scoped review; after #607: base update (it conflicts now), then Full | `t3-56a3afc8` |
| #615 / #616 | `b4021e34a1` | Repair the Full failure at `town_growth.cpp:20` (native, Linux GCC SDL2 and Windows x64) with a scoped review; after #607: Full | `t3-68700e40` |
| #617 / #618 | `45e1350c79` | Fresh full fixing review; after #607: base update, then Full | `t3-0d996c45` |
| #619 / #620 | `1afd6fe028` | Continue the implementation; compilation, behavior, cost and review are incomplete | `t3-90e92030` |

Keep these unchanged as well: `t3-120ef117` (old #603 at `451ebd778e`),
`t3-431c84b4` (cancelled #565 source with 58 dirty entries), `t3-d78d5f8b`
(native fixture evidence at `51ade2d021`), `.local/reference/openttd` at
`14ec60f248`, the instruction-profile worktrees and the deep evidence. `t3-`
paths are under `/home/fetch/.t3/worktrees/openttd-rust/`; profile and deep
worktrees are under the main checkout's `.local/worktrees/`. On resume, remove
the checkouts of finished reviews, merged #614 and the completed planner under
[cleanup](workflow.md#cleanup).
