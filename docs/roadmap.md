# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-01, after integration 56)

Agent: /root | Model: claude-opus-5-5 | Reasoning effort: xhigh

- Code is `2d2cd78f53` (integration 56). This session integrated #616 (52), #625 with #623 through #630 (53), #618 (54), #624 (55), and #627 with #620 through #631 (56). There were no rollbacks.
- The stocktake at `2d2cd78f53` uses `python3 tools/port-metrics.py --whole-fork`. Changes are since stocktake 54 at `b29e8036f5`.
  - Retired: 53,195 of 284,355 original code lines (18.71%, +649). Save/load retired 1,075 (+603).
  - Compiled C++: 264,758 (93.11%, -379). Since stocktake 49 at `ec11d2ba4d`, 0.98 days earlier, it fell 518 lines.
  - Glue 33,598 (+270), with 1,415 exports (+35) and 4,172 callback slots (-6).
  - Rust 126,262 (+1,535) and tooling 53,054 (+242).
  - Simulation-source retirement: 40,074 of 61,148 (65.54%).
- #627 retired 603 save/load lines for +206 save/load glue. #624 removed 39 rebound rows.
- Baselines at `2d2cd78f53`: 523 rebound-callee rows (-39), 127 shared-accessor rows (0) and 55 orphan exports (0).
- Speed: profile 56 ran on merged `2d2cd78f53`, and every scenario is equal. Every play and generation Ir fell 0.1% to 0.4% since profile 54, so every projection still equals profile 46's. The drawing row is 2,024,515,373 Ir with no budget. The next profile is due after 58.

| Scenario | Candidate Ir, profile 56 | Projection | Carried three-pass record |
| --- | ---: | ---: | ---: |
| TGP | 2,729,609,930 | 1.111722x | 1.154106x |
| Grok | 26,828,480,584 | 1.387147x | 1.385653x |
| Opus | 28,216,022,308 | 1.393949x | 1.394556x |
| P1996 | 4,534,487,327 | 1.336551x | 1.337764x |
| P2000 | 7,384,638,935 | 1.400577x | 1.401446x |
| P2016 | 13,362,841,068 | 1.522911x | 1.481910x |

- The integration-55 platform cadence (run 41339144900) passed on every platform. The next is due after 60.
- Coverage is unchanged: #156 holds the gaps, and #503 comes last.
- Unintegrated branches: #628 only, held. The wrap-up course brought the count down from eight (user, 2026-11-01).

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

## Twenty-sixth steering review (2026-11-01: held phase-1 and phase-2 owners passed over)

Agent: /root (Claude Code steering review) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host.

**What was audited.** Four independent read-only audits ran, with no builds or games. The steering checked every claim acted on below against the source.
- Fidelity, line by line against the pinned original: #627's save/load codec slices 1 to 3a; #620, #616 and #618; #612 and #624.
- Targeted: readiness of the held owners #570, #559 and #69, and of #628 and #573's remaining slices.

**No reachable divergence in #612, #616, #618, #620, #624 or #627 from valid or old saves.**
- Random draws, widths, wrapping and side-effect order match. #612's partition bounce is gone: `RustKdtreePartition` is a stateless `nth_element` over keys Rust fills first.
- #612 reads each k-d tree coordinate through two C++ calls (`station/spatial.rs:12`), about four per visited node. The station pool owner removes them (work order 4).
- #616 and #620 added Rust map readers beside the native `bridge_above` and `rail_tile_type` services they duplicate. #565 item 2 deletes the services.
- #627: a corrupt table header's `NOT_REACHED` now names `services_assert.h` in its message, with the same text and exit. The `FixSCCEncoded` wrappers survive only for tests and keep two exports alive, and #573's next PR removes them. The old-save paths before `SLV_69` and 4.4 have no bundled save, so they go to #156.

**1. The held owners were passed over.** This is the user's focus.
- The 10-31 session scope said "Leave #616, #618, #620, #559, #69 and #570 untouched" because they did not fit that usage window. It did not say why.
- When the user widened the budget, root kept the hold "since you didn't lift that". It filled the room with #621, #622, then phase-3 #573 and #574 (`97c01a0df8`).
- The 11-01 wrap-up integrated #616, #618 and #620. Its work order then ranked "Continue the started owners" (#622's later batch, #573's slices 3b and 4, #628) above #559, #69 and #570. That contradicts [selection](planning.md#selection) (earliest phase first, user 2026-10-28) and the navigation's "Start #570 when PR-B integrates".
- Cost: #570, phase 2's only ready owner, had no branch for about 25 hours after #607 integrated, across eight integrations. Phase 3 gained #627's 603 retired save/load lines. #628 holds a slot at slice 1 of 5, blocked on #629.
- Root cause: a one-off session hold whose reason went unstated, which the wrap-up's work order then carried forward. It adds no rule: the user judged it rare (user, 2026-11-01).

**2. The held owners' briefs are stale.**
- #570's issue and design put the vehicle reference adapters in `saveload.cpp`; #627 moved reference conversion to `saveload.rs`.
- #573's slice 3b changes the `SaveLoad` base and the `saveload.h` macros that #570's vehicle descriptors use, so the two cannot run together.
- #559's body still carries the 23rd review's exclusion and the portable bodies #607 removed. #628 already calls a native `IsCommandAllowedWhilePaused` leaf that #559 would port.
- #69 is still labelled `paused` and cites removed bases and a C++ fallback. #522 already owns bitmap storage.

Apply these corrections through the work order:

1. **#570 first, then #69 and #559, then #629** (work order 1 to 3), under the existing earliest-phase [selection](planning.md#selection). Root rewrites #69's and #559's bodies before their briefs. #573's next PR waits for #570, and #565 item 2 waits for #570 to leave `movement.rs`.

Template watch: #627's per-container typed leaves for STL containers are a phase-3 necessity. They do not justify per-field leaves in phase-2 records, which #570 defines once in Rust.

No user direction is reversed. The 10-31 hold and burn approvals were session-scoped and have expired. The restart prompt is unchanged.

| Twenty-fifth correction | Verdict and cost/result |
| --- | --- |
| 1. Agents run their own long jobs | Applied, and harmful in part. After it, 8 of 11 child turns that claimed a running job had started none. That cost about an hour on 11-01 and lost #623's and #625's reviews on 10-31. Root traced it at the user's request and fixed it in `ae3fd45dff`: a job ID before ending a turn, short checks in the foreground, and root checks children that name none. Keep. |
| 2. #607 first, in one base update | Applied: integration 48 (`92ef07aac4`) in the scoped session, with no further proof. |
| 3. Evidence names its behavior | No measurable effect. Integrations 50, 52, 54 and 56 added about 1,230 tooling lines for about 825 retired, about 1.5 per line. #627 ran at 0.4. The small owners carry the ratio: #612 594 for 90, #616 196 for 52, #618 200 for 34. Keep it provisional; phase 1's close stops the small owners. |
| 4. Fault routing | Applied: #612's bounce was fixed before integration (`37a10145c6`), and the audit confirms it. |
| 5. Roadmap commits only at stocktakes, course changes and wind-down | Applied: 9 roadmap commits for integrations 48 to 56, 1.0 per integration against 2.7. |

| Change since the twenty-fifth review | Verdict and next observation |
| --- | --- |
| Job-ID rule and root check (`ae3fd45dff`, user-directed investigation) | Applied in the wrap-up. Watch for child turns ending without a named job. |
| Navigation of 2026-10-31 | #607 landed first and #566 integrated in #623. Phase 1 gained no new owner, but #559 and #69 did not start (finding 1). Stocktakes record the compiled-C++ rate: -518 lines in 0.98 days to stocktake 56. |

New since this review, for the next steer to judge:
- Whether #570 starts in the first session, and whether #69 and #559 start after their rewrites.
- Whether a later slice of #573 or #628 takes a slot while a phase-1 or phase-2 owner is ready.
- Compiled C++ per day once #570 runs, against -518 per day.

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
NewGRF corpus stays excluded with the return condition below. Every agent runs
its own long jobs and ends a turn only on a named job ID, and root checks any
child that ends a turn without one. A base update that meets a review condition
needs only CI. Each new scenario, fixture or checker names the behavior existing
checks cannot distinguish; this rule is provisional, on a basis of 1.5 tooling
lines per retired line. Roadmap commits happen only at stocktakes, course
changes and wind-down. AGENTS.md and its routing table carry the rules; issue
and PR bodies carry current actions and evidence.

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
| 48 | #581 PR-B, Rust required and portable fallbacks retired / #607 | `92ef07aac4` | 1 / 106 / -812 / -813 (re-baseline, whole-fork step) |
| 49 | #199 batch: #569's map, movement and follower copies / #603 | `ec11d2ba4d` | 202 / 3 / 0 / +31 |
| 50 | #602 station spatial queries and rail trigger areas / #612 | `55e1c2c850` | 638 / 594 / 90 / -46 |
| 51 | Anchored instruction identity after the Rust-only build / #626 | `b34535f99c` | 0 / 56 / 0 / 0 |
| 52 | #615 clear-ground clearing and editor tree groups / #616 | `1daa611627` | 131 / 196 / 52 / +17 |
| 53 | #621 per-function inventory (#625) and #566 drawing workload (#623) / #630 | `5ecf4fbc57` | 0 / 409 / 0 / 0 |
| 54 | #617 snow-line state and map-edge water scans / #618 | `b29e8036f5` | 159 / 200 / 34 / +61 |
| 55 | #622 batch: five shared-operation families / #624 | `181a962caa` | see #624 / rebound rows 567 to 523 |
| 56 | #573 save/load codec slices 1 to 3a (#627) and #619 road-bit cleanup (#620) / #631 | `2d2cd78f53` | see stocktake 56: save/load retired +603 |

## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and the disjoint-service rule; finishing review and integration
of an open branch comes first. A lower rank can run while a higher one waits.
Scopes live in issues.

The 10-31 session hold on #559, #69 and #570 has expired, and the wrap-up course
(user, 2026-11-01) is complete. Forward work resumes here.

1. **#570 shared vehicle records starts in the first free slot.** It is phase 2's first owner. It has been ready since #607 (48) and #623 (53) integrated.
   - Its brief names `rust/openttd-kernels/src/saveload.rs` as the home of reference conversion since #627; the issue and design still say `saveload.cpp`.
   - #570 follows the [world-state design](design/world-state.md). It deletes the replaced readers, the direct Tick bounces and their baseline rows, and makes `tools/port-metrics.py` count the C++ declarations it generates. Estimated net glue is -500 to -1,100; its 0.86% instruction credit is a model.
   - Then, one at a time in the shared vehicle files: #598 vehicle routing decisions, then #599 vehicle base lifecycle. Remeasure #599's inventory after #570. The airport automata follow #570's declaration generator.
2. **#69 tile areas and #559 command invocation start in the next free slots.** They are phase 1's last owners. Root rewrites each issue body before its brief:
   - #69: drop the `paused` label and the C++ fallback and #65/#62 bases that #607 and later work removed. Bitmap storage is already Rust-owned (#522). Scope `tilearea.cpp` (177) including its diagonal areas, and the `tilearea_type.h` bodies. `script_tilelist.cpp` stays with phase 3. The inline iterators in C++ loops must not become one crossing per tile.
   - #559: replace the 23rd review's exclusion text. Its planned 320 to 420 retired lines against 500 to 900 net glue get the necessary-cost or bounded-repair decision at the first boundary checkpoint ([cost decisions](planning.md#cost-decisions)). #628 already calls a native `IsCommandAllowedWhilePaused` leaf from Rust; #559 owns it once and #628 adopts that owner.

   No further phase-1 owner is planned. Every other remaining non-drawing body moves with the phase-2 owner of its file, including plan-after47's link-graph job lifecycle, elrail transition and land count; #621 lists them.
3. **One maintenance batch at a time** (one reviewer, one Full):
   - #629 first: isolate the multiplayer comparison from other loopback connections on port 41051. It fails on base too, and #628 needs it as its per-slice check.
   - #622's later batch: the shared typed `RustSpec*` table (root's answers on #624), then `RustGRFFileID`, `RustGRFCargoMap`, `RustGRFFileAtOrNull` and the `RustResolverGRF*` callees, and extending `tools/rebound-callees.py` to `.inc` service tables.
   - #199's open items, disjoint from #570: the `CargoFlow` bounce, #578's story-page bounce, #580's date parameters and the sweep's stale comments.
   - #565 item 2, after #570 integrates, because both change `movement.rs`. Delete the map fallbacks and convert the remaining rail, crossing, depot, stop, water and tunnel leaves, `tile_slope_z`, and world generation's per-tile services (issue body). Production net glue -300 to -600. It also deletes the native `bridge_above` and `rail_tile_type` services that #616 and #620 duplicated with Rust map readers (`road_cmd.rs:932`, `road_cmd.rs:1443`, `station_cmd.rs:447`).
   - Later: #604 shared settings access, after #570's generator; then #168/#199/#500 completed-job publication and #501 probe typing.
4. **The next phase-2 pool after #570 integrates**, in the order of "Phases": stations and road stops (1,604 storage lines, about 600 callback slots; it removes #612's two C++ calls per k-d tree coordinate read in `station/spatial.rs:12`); towns and industries (1,295, about 344); companies, groups, engines, link graph and the smaller pools (952, about 286); orders and order lists (390, about 111); then the map (3,988) with the rail, road, water, tunnel, landscape and tile bodies. Each pool gets an issue and a design section when selected. It moves its coupled save adapters with each record, generates native declarations, verifies the layout, and deletes the displaced readers, services and their baseline rows.
5. **Phase 3 fills every other free slot:**
   - #628 (#574 network protocol) resumes from slice 2 once #629 integrates. Read its status comment first.
   - #575 script host, commands and events, with its unsaved-output evidence.
   - #573's next PR, after #570 integrates: slice 3b changes the `SaveLoad` base and the `saveload.h` macros that #570's vehicle descriptors use. Then slice 4, the corrupt-input and LoadCheck probes, and the native `CopyFromOldName` helper, as #627's status comment lists.
   - Then #497 with its engine and feature binding payloads, and #498 with its ObjectSpec payloads.

   Size each owner to repay its review and Full ([selection](planning.md#selection)). Plan no Squirrel port.
6. **#503 coverage continuation last:** first the bounded clock-preparation and sanitized native-test-support build fixes, then the 119 unfinished traces with provenance checks. Do not recollect just to fill a slot.

Recurring:
- After every second integration (next after 58), take stock under `/start-development` step 4, including the compiled-C++ rate and the baseline sizes. Profile the merged tip with the pinned `LD_LIBRARY_PATH` from [speed](speed.md#instruction-projections).
- Run the platform cadence after every fifth integration (next after 60). Fix a red cadence before further integrations.
- Deep checks run only on the user's request (user, 2026-10-31).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions.

Excluded, with return conditions checked by this review:
- #507's NewGRF corpus (#525, #526) stays parked by the user (2026-10-28) until the user reselects it. #525's branch stays on origin.
- #64 and #68 become maintained Rust crates that reproduce the game's observable cryptography. They return when phase 3's network ports reach authentication, or with phase 6, whichever comes first.
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.

## Resume checkpoint (2026-11-01, wrap-up complete after integration 56)

Code is `2d2cd78f53`. `CI_ON_DEMAND` is `true`. No agent is running. Read #628's status comment before resuming it.

| Issue / PR | State at this commit | Next action | Checkout |
| --- | --- | --- | --- |
| #574 / #628 | Slice 1 of 5 at `77c0b8295e`, held | Slice 2 after #629 integrates (work order 3 and 5) | `t3-1e4ffc20` |

Since 2026-11-01, AGENTS.md requires a named job ID before an agent ends its turn on a background job, and short checks run in the foreground. Codex children had been ending turns on jobs they never started (`ae3fd45dff`).

Keep unchanged: `t3-120ef117` (old #603), `t3-431c84b4` (cancelled #565 source with 58 dirty entries), `t3-d78d5f8b` (native fixture evidence), `.local/reference/openttd` at `14ec60f248` and the instruction-profile worktrees under the main checkout's `.local/worktrees/`. The merged #603, #607, #612, #616, #618, #626 and #630 checkouts and the finished review checkouts are T3-locked. Housekeeping keeps them until T3 releases them.
