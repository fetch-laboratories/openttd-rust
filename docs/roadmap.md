# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-02, stocktake 64)

Agent: /root | Model: claude-opus-5-5 | Reasoning effort: xhigh

- Code is `5e414c5a64` (integration 64). This session integrated #648 (61, the cadence fixture fix), #645 (62, #641's packet owner), #640 (63, #559's command invocation, phase 1's exit) and #642 (64, #64's maintained crates and Rust network authentication). There were no rollbacks.
- Stocktake 64 ran `python3 tools/port-metrics.py --whole-fork` at `5e414c5a64`. Changes are since stocktake 62 at `bf2c4632a2`, 0.07 days earlier; stocktake 62 is in Git history (`f0cf0ede76`).
  - Retired: 55,822 of 284,355 original code lines (19.63%, +2,378). Vendored retired 2,317 (+1,890, Monocypher). Save/load retired 1,075 (0).
  - Compiled C++: 262,199 (92.21%, -2,331). Over stocktakes 60 to 64 (0.56 days) it fell 2,303 lines, about -4,100 per day; #642 supplied all of it.
  - Glue 33,666 (+47), with 1,425 exports (-12) and 4,062 callback slots (-2).
  - Rust 127,288 (-51, #642 deleted its earlier primitive ports) and tooling 53,247 (-93).
  - Simulation-source retirement: 40,407 of 61,148 (66.08%, +136).
- Baselines at `5e414c5a64`: 418 rebound-callee rows (-2), 126 shared-accessor rows (0) and 54 orphan exports (-1).
- Compiled C++ rose by 7 lines over integrations 61 to 63, then fell with #642. Excluding vendored code, the ownership ports still barely move it: #636's checkpoint 2 must delete the per-field inventory. Stocktake 66 judges that.
- Speed: the table is profile 64 at `5e414c5a64`, flat against profile 62 (-0.11% to +0.07% Ir). Every scenario stays 0.12% to 0.90% under its anchor count, so projections are unchanged. The drawing workload (#566) runs at 1.0027x reference Ir. All seven games compared equal, but the instruction harness now rejects the anchors: #642 changed `rust/openttd-kernels/Cargo.toml`, which is part of the build identity. Projections above use the speed formula by hand until a quiet speed pass re-anchors (recurring list).

| Scenario | Candidate Ir, profile 64 | Projection | Carried three-pass record |
| --- | ---: | ---: | ---: |
| TGP | 2,731,301,085 | 1.111722x | 1.154106x |
| Grok | 26,898,981,923 | 1.387147x | 1.385653x |
| Opus | 28,297,683,609 | 1.393949x | 1.394556x |
| P1996 | 4,542,468,375 | 1.336551x | 1.337764x |
| P2000 | 7,400,813,786 | 1.400577x | 1.401446x |
| P2016 | 13,400,911,478 | 1.522911x | 1.481910x |

- The platform cadence after integration 60 failed on a Linux Clang warning from #639's new `shared` field (run 41436192681); #648 fixed it and the rerun (run 41441266701) passed. The next cadence is after integration 65.
- Coverage: #156 holds the gaps, including #640's and #642's unreached branches, and #503 comes last.
- Unintegrated branches: five (#647, #636, #628, #649 and #575).

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
| 1. Simulation logic | About 2,250 lines of non-drawing bodies in the logic bucket, plus #559's command invocation (`command.cpp`, 151) and #69's tile areas (`tilearea.cpp`, 172) | Its open drafts, #559 and #69 integrate. Every other remaining non-drawing body belongs to the phase-2 owner of its file | Exited at integration 63 (#640, #559), after #69 (60) and the earlier drafts. No further phase-1 owner is planned |
| 2. Simulation storage | 11,007 in the storage bucket, plus the phase-1 bodies left in each pool's files | Every pool record and the map arrays are defined once in Rust, keep the original pool, lifetime and save behavior ([world-state design](design/world-state.md#transferring-shared-storage)), and are read in place by both languages. C++ calls Rust only for operations, and no per-field service or per-call-site reader remains for moved data. Each pool owner also ports the remaining non-drawing bodies in its files | PR-B (#607) first, then #570 with #566. Pools in order: vehicles (#570, then #598 and #599); stations and road stops; towns and industries; companies, groups, engines, link graph and the smaller pools; orders; then the map with the rail, road, water, tunnel, landscape and tile bodies |
| 3. Simulation data readers | 41,096: save/load 12,852, script 13,099, network 9,640, NewGRF 5,505 | These run in Rust; the Squirrel VM may stay C++ until phase 6 | #573, #574 and #575 are queued; #497 and #498 are parked (work order 5). They take every slot phases 1 and 2 leave free |
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
4. Fill every other free slot with phase 3 (#573, #574, #575; #497 and #498 are parked),
   ranked under [selection](planning.md#selection).
5. Size new owners to repay their review and Full ([selection](planning.md#selection)),
   and record the compiled-C++ rate at each stocktake (`/start-development`, step 4).
6. Collapse glue duplication where it lives longest: after PR-B, #622 takes
   the NewGRF specification and shared-operation bindings, about 332 of the
   526 rebound rows, which stay compiled until phases 3 to 5. The rest bind
   record fields, which go when phase-2 owners move those records, so do not
   tidy them first. Each stocktake records the baseline sizes.

## Twenty-seventh steering review (2026-11-02: a compiler defect at the boundary and the phase-2 template)

Agent: /root (Claude Code steering review) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host.

**What was audited.** Four independent read-only audits compared bodies against the pinned original, with no builds or games. The steering checked every claim acted on below against the source, and reproduced the compiler defect.
- #636 (#570 checkpoint 1) with the integrated #643 generator.
- #628 (#574 slices 1 to 3).
- #640 (#559), and the cause behind #644.
- #633 (#69, integration 60) and #645 (#641).

**Root's track.** The 11-01 evening session applied the twenty-sixth correction in its first turn. It integrated four PRs (57 to 60) with six Full runs: one was cancelled by a docs push during a Full, and #645's failed. #633 took three review rounds. The session wound down for the Claude session limit with stocktakes 58 and 60, profile 58 and the platform cadence owed. This review took stocktake 60.

**No reachable divergence in #628, #633, #636, #640 or #645**, except one abort from scripts (#646). Each PR's comment from this review separates what must change before integration from what can follow.

**1. The pinned rustc drops `zeroext` on `extern "C"` `bool` returns.**
- rustc 1.99.0 emits `define noundef i1` with no `zeroext` on a `bool` returned from a Rust-defined `extern "C"` function. Its upper seven bits are then unspecified, and GCC tests the whole byte. #640 hit it: false came back as `0x0a`, and town ratings diverged.
- 184 non-test Rust functions are exposed, 148 of them exports. A source scan cannot clear them, because which shapes come back unmasked depends on inlining.
- Parameters, C++ returns into Rust, `repr(C)` structs and out-pointers are masked and safe.
- Correction: #644 converts them to 0-or-1 `u8` returns and adds a quick-CI gate. It takes the first free slot with #646, and may go alone (work order 3).

**2. #570's checkpoint 1 is a sound core with four template faults.** Each is in the steering comment on #636.
- A new Rust-to-C++-to-Rust bounce through `SetNext`.
- One handle type that now means a record in some places and a shell in others.
- About 213 lines of per-field services retargeted to the record instead of deleted.
- "Retired" counts rewritten consumer lines. #636 reports 718 retired and +743 net glue, but its `src/` diff adds 1,124 lines and deletes 1,114, so compiled C++ barely moves until checkpoint 2 deletes the inventory. The issue's -500 to -1,100 net glue did not count rewritten lines; the audit estimates -150 to -450.
- Correction: phase-2 owners are ranked and costed by compiled C++ removed ([selection](planning.md#selection)). Before the #634 and #635 briefs, root decides from the post-#570 profile whether later records sit beside or inside their shells (work order 4).

**3. The wind-down checkpoint understated #645's failed Full.** Run 41366354465 failed `native / comparisons` and three annotation checks, not only the Emscripten check. The comparisons job never receives `build-rust/generated-abi`, which `packet_ffi.h` now includes. The resume action ("Full at that head, then merge") would have failed again. The fix packs that directory for every later #643 record.

**4. Issue placement.**
- #622's scope was finished, and this review closed it.
- #604's selection event, #570's generator, integrated as #643, so #604 is ready.
- #497's non-engine handlers were waiting on #622's shared table, which integrated in #639. The next planning round re-plans them.
- #64 returned under its recorded condition (root, 11-01), and #68 is closed. Their exclusion line is gone.
- The navigation's "first actions" under Phases are all complete or superseded by the work order.

No user direction is reversed. The restart prompt is unchanged.

| Twenty-sixth correction | Verdict and cost/result |
| --- | --- |
| 1. #570 first, then #69 and #559, then #629 | Applied in the next session's first turn. #629 integrated as 57 and #69 as 60. #570 reached checkpoint 1, and #559 passes its scenarios. Root also started the parked #497, which stopped before any code, and root corrected the roadmap (`92e23694cd`). No phase-1 or phase-2 owner waited while #628's slices ran. |

| Change since the twenty-sixth review | Verdict and next observation |
| --- | --- |
| Session-hold and started-slice rules dropped (user, `e68ea49b66`) | No hold recurred. |
| Commit in sensible chunks (user, `b0f8c88172`) | Applied. After #636's first WIP checkpoint, each open PR's commits are single coherent steps. Keep. |
| Twenty-fifth corrections still in force | Job IDs: children named theirs. Evidence: 175 tooling lines for 197 retired over integrations 57 to 60 (0.9 per line), still provisional. Roadmap commits: four for four integrations, and one (`8aef31dbdf`) cancelled #632's Full. |

New since this review, for the next steer to judge:
- Whether #636's checkpoint 2 deletes the per-field inventory and compiled C++ falls, and whether the bounce and handle items are fixed before integration.
- Whether #644's gate lands before more `bool` exports do.
- Whether the record-placement decision uses the post-#570 profile before #634 and #635 are briefed.
- Whether #628's next round names an end for each bounce listed in its steering comment.

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
changes and wind-down. #627's per-container typed leaves for STL
containers are a phase-3 necessity; they do not justify per-field leaves in
phase-2 records. A phase-2 owner's cost is its compiled-C++ change. AGENTS.md and its routing table carry the rules; issue
and PR bodies carry current actions and evidence.

## Recent completed work

Rows cover the period since the twenty-fifth steering review; earlier rows are
in their PRs and Git history. Metrics are `tools/port-metrics.py` code lines:
Rust / tooling / original C++ retired / net glue.

| Integration | Issues / PR | Merge commit | Metrics |
| --- | --- | --- | --- |
| 48 | #581 PR-B, Rust required and portable fallbacks retired / #607 | `92ef07aac4` | 1 / 106 / -812 / -813 (re-baseline, whole-fork step) |
| 49 | #199 batch: #569's map, movement and follower copies / #603 | `ec11d2ba4d` | 202 / 3 / 0 / +31 |
| 50 | #602 station spatial queries and rail trigger areas / #612 | `55e1c2c850` | 638 / 594 / 90 / -46 |
| 51 | Anchored instruction identity after the Rust-only build / #626 | `b34535f99c` | 0 / 56 / 0 / 0 |
| 52 | #615 clear-ground clearing and editor tree groups / #616 | `1daa611627` | 131 / 196 / 52 / +17 |
| 53 | #621 per-function inventory (#625) and #566 drawing workload (#623) / #630 | `5ecf4fbc57` | 0 / 409 / 0 / 0 |
| 54 | #617 snow-line state and map-edge water scans / #618 | `b29e8036f5` | 159 / 200 / 34 / +61 |
| 55 | #622 batch: five shared-operation families / #624 | `181a962caa` | see #624 / rebound rows 567 to 523 |
| 56 | #573 save/load codec slices 1 to 3a (#627) and #619 road-bit cleanup (#620) / #631 | `2d2cd78f53` | see stocktake 56: save/load retired +603 |
| 57 | #629 multiplayer comparison isolated from host loopback probes / #632 | `46a2f240d7` | 0 / 8 / 0 / 0 |
| 58 | #622 later batch: shared NewGRF spec and GRF-file bindings / #639 | `83c62ef441` | net glue -34 / rebound rows 523 to 420 |
| 59 | Shared-record declaration generator (#570, #641) / #643 | `2d4a063e33` | 48 / 236 / 0 / +8 |
| 60 | #69 tile-area geometry and the diagonal iterator / #633 | `b97004e599` | 529 / 5 / 197 / -27 |
| 61 | Platform cadence: landscape fixture `shared` initializer / #648 | `cdfc2de482` | 0 / 0 / 0 / 0 |
| 62 | #641 packet state and byte storage in a generated Rust record / #645 | `bf2c4632a2` | 718 / 175 / 52 / +80 |
| 63 | #559 command invocation lifecycle (phase 1's last owner) / #640 | `7940e700c2` | 403 / 287 / 136 / +115; rebound rows 420 to 418, orphan exports 55 to 54 |
| 64 | #64 maintained crypto crates and Rust network authentication / #642 | `5e414c5a64` | 2,027 / 480 / 2,242 / -68 |

## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and the disjoint-service rule; finishing review and integration
of an open branch comes first. A lower rank can run while a higher one waits.
Scopes live in issues; per-PR changes live in each PR's steering comment.

1. **#570 vehicle records (#636) to checkpoint 2.** It is phase 2's first owner and the template for every later pool.
   - First the four items in the twenty-seventh steering comment on #636: remove the `SetNext` bounce, switch handle identity with the first Rust field read, delete the retargeted per-field services with the design's inventory, and merge base, taking #643's generator.
   - Judge its cost by compiled C++ (original plus glue), not by its retired figure: rewritten consumer lines count on both sides. The audit estimates final net glue at -150 to -450, against the issue's -500 to -1,100.
   - Then, one at a time in the shared vehicle files: #598 vehicle routing decisions, then #599 vehicle base lifecycle. Remeasure #599's inventory after #570.
2. **Phase 1 has exited:** #559 integrated as 63 (#640). Every remaining non-drawing body moves with the phase-2 owner of its file; #621 lists them.
3. **Maintenance, one batch at a time** (one reviewer, one Full):
   - #647 (#644 boundary `bool` returns with #646 `TileArea::expand` overflow) is in review and integrates next. It overlaps #640 in eight files, so its base update over integration 63 needs a fresh scoped review unless every conflict keeps both sides ([review](workflow.md#review)). Its quick-CI gate then rejects new `bool` returns from Rust-defined `extern "C"` functions.
   - #604 shared settings access. It is ready, because its selection event, #570's declaration generator, integrated as #643. It also extends `tools/rebound-callees.py` to `.inc` tables (51 rows, moved from #639).
   - #199's open items, disjoint from #570: the `CargoFlow` bounce, #578's story-page bounce, #580's date parameters and the sweep's stale comments.
   - #565 item 2, after #570 integrates, because both change `movement.rs`. Delete the map fallbacks and convert the remaining rail, crossing, depot, stop, water and tunnel leaves, `tile_slope_z`, and world generation's per-tile services (issue body). Production net glue -300 to -600. It also deletes the native `bridge_above` and `rail_tile_type` services that #616 and #620 duplicated with Rust map readers (`road_cmd.rs:932`, `road_cmd.rs:1443`, `station_cmd.rs:447`).
   - Later: #168/#199/#500 completed-job publication and #501 probe typing.
4. **The next phase-2 pools after #570 integrates** (ranked by plan-after56, 2026-11-01). Each needs #570's generator, save descriptors and record lookup. Rank and judge each by compiled C++ removed ([selection](planning.md#selection)).
   - Before briefing #634 or #635, root decides whether later records sit beside their shells, as #570's do, or inside them. Each moved-field read beside the shell costs a dependent load. Read the cost from the merged-tip profile after #570 and #566's drawing workload. If the cost is material, record the decision in the [world-state design](design/world-state.md) before the briefs.
   - #634 stations and road stops: 450 to 650 retired, -300 to -800 net glue, about 110 to 160 station-field slots. It removes the per-station `SharedForEachStation` bounce and #612's two C++ calls per k-d tree coordinate (`station/spatial.rs:12`).
   - #635 towns, industries and subsidies: 350 to 550 retired, -150 to -450 net glue, about 40 to 70 field slots. Disjoint from #634; the two may run together.
   - Then orders and order lists (441 left; read on every vehicle tick), #637 engines (545, with #497's engine bindings), companies and groups (510), link graph (607), #638 the smaller pools (615, with #498's ObjectSpec payloads), and the map (3,988 storage) with the rail, road, water, tunnel, landscape and tile bodies. Pools without an issue get one and a design section when selected. Each moves its coupled save adapters with each record, generates native declarations, verifies the layout, and deletes the displaced readers, services and their baseline rows.
5. **Phase 3 fills every other free slot:**
   - #628 (#574 network protocol): the resume checkpoint, its status comment and the steering comment on #628 hold the next round. #645 (62) and #640 (63) have integrated, so its close, receive, send and command-post bounces adopt them now; it does not build a second packet or command path.
   - #575 script host, commands and events, with its unsaved-output evidence. Ready: #559 integrated as 63.
   - #573's next PR, after #570 integrates: slice 3b changes the `SaveLoad` base and the `saveload.h` macros that #570's vehicle descriptors use. Its brief keeps the save-descriptor macros unchanged so it can run beside #634 and #635; otherwise whichever starts second waits. Then slice 4, the corrupt-input and LoadCheck probes, and the native `CopyFromOldName` helper, as #627's status comment lists.
   - #497's non-engine spec handlers: their return condition, #622's shared `RustSpec*` table, integrated in #639. The next planning round re-plans them on that table. #497's `Engine::grf_prop` and `overrides` bindings stay with #637, and #498 with #638 (twenty-first steering, correction 2: no data behind setters).

   Size each owner to repay its review and Full ([selection](planning.md#selection)). Plan no Squirrel port.
6. **#503 coverage continuation last:** first the bounded clock-preparation and sanitized native-test-support build fixes, then the 119 unfinished traces with provenance checks. Do not recollect just to fill a slot.

Recurring:
- After every second integration (next after 66), take stock under `/start-development` step 4, including the compiled-C++ rate and the baseline sizes. Profile the merged tip with the pinned `LD_LIBRARY_PATH` from [speed](speed.md#instruction-projections).
- Run the platform cadence after every fifth integration; the next is after 65. Fix a red cadence before further integrations.
- A quiet speed pass is owed in the next quiet window ([speed passes](speed.md#speed-passes)): #642 changed `rust/openttd-kernels/Cargo.toml`, so `simulate --instructions` rejects the anchors' identity. It pauses builds and games for timed collection, so run it when no agent is building, at the latest before the post-#570 record-placement decision.
- Deep checks run only on the user's request (user, 2026-10-31).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions.

Excluded, with return conditions checked by this review:
- #507's NewGRF corpus (#525, #526) stays parked by the user (2026-10-28) until the user reselects it. #525's branch stays on origin.
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.

## Resume checkpoint (2026-11-02, stocktake 64)

Root is active in Claude Code (`claude-opus-5-5`, xhigh). Code is `5e414c5a64` (integration 64). `CI_ON_DEMAND` is `true`. Implementers and reviewers mostly run on Codex `gpt-6.1-sol` high; Claude Opus takes work when the Claude five-hour window has room, so reviewer and implementer families differ where possible. Each PR's status comment holds its next action.

| Issue / PR | Head | Next action | Running job or CI |
| --- | --- | --- | --- |
| #644, #646 / #647 boundary `bool` returns and `expand` overflow | `0f1ee72286` | Finish review (the reviewer fixed the gate), root spot-check, base update over integrations 63 and 64 with a scoped review of any non-trivial conflict, then Full and merge | Review by /root/bool_review |
| #604, #199 / #649 settings access batch | `01b3e64f93` | All seven carriers deleted, four #199 items, `.inc` rebound coverage and a shared-RNG fix. After validation: review (check the RNG fix's draw order), Full, merge | /root/settings_batch |
| #570 / #636 vehicle records | `b894512bb6` | Bounce removed, handle identity switched, identity and common reader services deleted. Next: ground and train records and the rest of the per-field inventory, checkpoint 2, then review | /root/vehicle_records_cp2 |
| #574 / #628 network protocol | `e773dbf2ed` | Close and send bounces removed, socket state canonical. Next: receive dispatch, the command-post bounce on #640, merge integration 64 and adopt #642's authentication owner, slice 4, then one round review | /root/network_round |
| #575 script host (new PR) | - | Implementation started on #640's command lifecycle | /root/script_host (Claude Opus, medium) |

PR worktrees: T3 task worktrees for each running agent, plus the earlier `t3-5aa8a17f` and `t3-fc38354a` (#628) and `t3-41b02f9e` (#636).

Keep unchanged: `t3-120ef117` (old #603), `t3-431c84b4` (cancelled #565 source with 58 dirty entries), `t3-d78d5f8b` (native fixture evidence), `.local/reference/openttd` at `14ec60f248` and the instruction-profile worktrees under the main checkout's `.local/worktrees/`.
