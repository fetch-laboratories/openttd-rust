# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-02, stocktake 64)

Agent: /root | Model: claude-opus-5-5 | Reasoning effort: xhigh

- Code is `6afdcbf390` (integration 65). The 11-02 session integrated #648 (61, the cadence fixture fix), #645 (62, #641's packet owner), #640 (63, #559's command invocation, phase 1's exit), #642 (64, #64's maintained crates and Rust network authentication) and #651 (65, #575's script host). There were no rollbacks.
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

- The platform cadence after integration 60 failed on a Linux Clang warning from #639's new `shared` field (run 41436192681); #648 fixed it and the rerun (run 41441266701) passed.
- **The platform cadence after integration 65 is red** (run 41466312349 at `6afdcbf390`, both attempts). macOS Debug (attempt 1) and macOS Release (attempt 2) fail `tools/check-rust-platform.py`: "rust-build.log: native libraries differ from the configure query"; the Windows and annotation failures cascade from it. Likely cause: #642's merge `5e414c5a64` (integration 64) added `libc` to `Cargo.lock` through `cpufeatures`, so the macOS Rust build now reports `-lc -lm -liconv -lSystem -lc -lm`, while the configure probe at `cmake/Rust.cmake` (around line 137) compiles a dependency-free staticlib. The job logs show the build's line; the probe's own line is only in `rust-native-libs.log`. #651's code merge follows `5e414c5a64`, so the bounded automatic rollback is unavailable. The repair is the first resume action; no integration happens before it.
- Coverage: #156 holds the gaps, including #640's and #642's unreached branches, and #503 comes last.
- Integration 65 (#651, #575's script host, `6afdcbf390`) followed stocktake 64: 354 retired, net glue +220, compiled C++ -134. Stocktake 66 has not run.
- Unintegrated branches: five (#647, #636, #628, #649 and #655).

## Phases

Agent: /root (Claude Code navigation) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host.

The goal is a complete Rust rewrite ([goal and phases](planning.md#goal-and-phases)). Each free
slot takes ready work from the earliest phase, and later phases fill the rest
(user decision, 2026-10-28). Position at code `6afdcbf390` (integration 65) in
code lines from `python3 tools/port-metrics.py --whole-fork`, with the change
since the previous navigation's position at `2a89fb023f` (integration 47) in
parentheses. Source areas are an accounting classification: they do not move
drawing or executable header logic into a different phase. "Retired" counts a
rewritten consumer line as retired and its replacement as glue, so compiled C++
(original plus glue) is the distance.

| Area | Original | Retired | Left | Glue added | Compiled C++ |
| --- | ---: | ---: | ---: | ---: | ---: |
| Simulation logic | 47,424 | 37,640 (+69) | 9,784 | 6,460 (-514) | 16,244 (-583) |
| Simulation storage | 13,724 | 2,767 (+50) | 10,957 | 1,313 (-22) | 12,270 (-72) |
| NewGRF | 10,785 | 5,201 (-79) | 5,584 | 349 (-79) | 5,933 (0) |
| Save/load | 13,388 | 1,075 (+539) | 12,313 | 727 (+142) | 13,040 (-397) |
| Script API | 14,908 | 1,202 (+324) | 13,706 | 542 (+89) | 14,248 (-235) |
| Network | 12,713 | 561 (+350) | 12,152 | 234 (+52) | 12,386 (-298) |
| Support | 21,617 | 930 (-52) | 20,687 | 452 (-61) | 21,139 (-9) |
| Graphics and viewport | 10,985 | 4 | 10,981 | 1 | 10,982 |
| GUI | 52,179 | 115 (-2) | 52,064 | 162 (-3) | 52,226 (-1) |
| Platform | 11,941 | 0 | 11,941 | 2 | 11,943 |
| Data tables | 17,295 | 4,364 | 12,931 | 0 | 12,931 |
| Vendored | 55,148 | 2,317 (+1,795) | 52,831 | 0 (-163) | 52,831 (-1,958) |
| Upstream tests | 2,248 | 0 | 2,248 | 5,292 (+174) | 7,540 (+174) |
| Boundary layer | 0 | 0 | 0 | 18,352 (+198) | 18,352 (+198) |
| **Whole fork** | **284,355** | **56,176 (19.76%, +2,994)** | **228,179** | **33,886 (-187)** | **262,065 (92.16%, -3,181)** |

The boundary has 1,443 Rust exports (+66) and 4,064 callback slots (-142).
Rust totals 127,673 code lines (+3,448) and tooling 53,611 (+2,301).
Simulation-source retirement is 40,407 of 61,148 (66.08%, +119). The duplicate
baselines at `5e414c5a64` hold 418 rebound-callee rows (581), 126
shared-accessor rows (128) and 54 orphan exports (55). #607 (integration 48)
re-baselined the count: its sweep removed 812 retired and 813 glue lines that
`WITH_RUST` fallbacks had counted on both sides, leaving compiled C++ unchanged.

| Period (steering and navigation commits) | Days | Retired | Per day | Compiled C++ change | Per day |
| --- | ---: | ---: | ---: | ---: | ---: |
| Fork start (`6ced7c097e`, 10-02) to 10-27 | 24.7 | 49,536 | 2,010 | -17,908 | -730 |
| 10-27 to the navigation at integration 47 (10-31) | 4.1 | 3,646 | 890 | -1,201 | -290 |
| Navigation to the twenty-sixth steering (11-01) | 1.12 | 13 (825 without #607) | 10 (730) | -488 | -430 |
| Twenty-sixth to twenty-seventh steering (11-02) | 0.59 | 197 | 330 | -256 | -430 |
| Twenty-seventh steering to now | 0.22 | 2,784 | | -2,437 (-479 without #642's vendored swap) | |
| **Since the last navigation** | **1.93** | **2,994** | **1,550** | **-3,181 (-1,223 without the swap)** | **-1,650 (-630)** |
| **Whole history** | **30.75** | **56,176** | **1,830** | **-22,290** | **-730** |

Days are calendar days. Integrations 48 to 65 landed in three root sessions
(10-31 19:47Z to 21:33Z, 11-01 16:42Z to 11-02 00:17Z, 11-02 12:00Z to 14:52Z);
the gaps were Claude session limits, user pauses and steering. What set the rate:
- #642 replaced bundled Monocypher with maintained crates: 1,958 of the
  3,181-line fall. Without it, compiled C++ fell 630 lines per day, twice the
  previous period's 290 but under the whole-history 730. At that rate the
  209,000 non-vendored compiled lines take about 330 days.
- The other ownership ports moved compiled C++ between +28 (#645) and -224
  (#633) each. Phase-3 owners delivered under their estimates: #575 planned
  1,000 to 1,350 retired and delivered 354 with +220 net glue (compiled -134).
- #570, phase 2's template, has not integrated: its checkpoint stands at
  compiled C++ -377.

Why the course changed:
1. Phase 2 could exit with C++ kept indefinitely. Its exit criterion here
   required "no per-field service ... for moved data", and
   [storage ports](porting.md#storage-ports) deleted only "replaced" services.
   #563's design keeps "Money/date fields and unselected vehicle fields in the
   shell" (`docs/design/world-state.md`, #563 records), yet Rust's maintenance
   owner reads reliability, age, value, profit and breakdown fields through
   `OpenTTDVehicleMaintenanceServices` and the shared vehicle table's
   `write_profit`, `write_reliability` and service-date slots. No later phase
   moves simulation fields, so those services and the shells would never
   retire. A pool is now moved only when no Rust code reaches any of its
   fields through a service, and #656 owns the vehicle remainder.
2. Planning ranked and costed owners by retired lines everywhere except phase 2
   ([selection](planning.md#selection)). Retired lines count rewritten lines
   and ignore glue, so #575's +220 net glue never counted against it. Every
   phase now ranks and decides cost by compiled C++ removed.
3. The planning trigger in `/start-development` (step 3) counted unstarted
   owners, including blocked ones. Phase 2's unstarted owners all wait for
   #570, and phase 3's queued #653 and #573 slice 3b wait for drafts, so
   planning would not run while no owner is ready and about 35,000 phase-3
   lines have none. It now counts ready owners.

| Phase | Left (code lines) | Exit criterion | Status |
| --- | --- | --- | --- |
| 1. Simulation logic | Its bucket's 9,784: about 3,780 drawing and description (phase 5), 490 startup in `openttd.cpp` (phase 4), 1,975 facade shells that retire with their callers, 540 file scope and about 3,000 non-drawing bodies | Every remaining non-drawing body belongs to the phase-2 owner of its file | Exited at integration 63 (#640, #559) |
| 2. Simulation storage | 10,957 in the storage bucket, the phase-1 bodies above, and the per-field services within the boundary layer's 18,352 glue lines and 4,064 callback slots | No Rust code reads or writes a pool or map field through a C++ service or per-call-site reader ([storage ports](porting.md#storage-ports)). Records and map arrays are defined once in Rust, keep the original pool, lifetime and save behavior ([world-state design](design/world-state.md#transferring-shared-storage)), and are read in place by both languages; C++ calls Rust only for operations. Fields only unported C++ touches stay in shells, each named with the phase that moves it. Each pool owner also ports the remaining non-drawing bodies in its files | Current. #570 (#636) is at checkpoint 2 (compiled C++ -377). Then #656, the remaining vehicle fields (it may join #598 and #599 as one owner); #634 with #635; orders; #637; companies and groups; link graph; #638; then the map with the rail, road, water, tunnel, landscape and tile bodies |
| 3. Simulation data readers | 39,962: save/load 12,313, script 12,782, network 9,283, NewGRF 5,584. Script and network GUI (3,793) are phase 5 | These run in Rust. The Squirrel VM and the C++ binding shims it calls may stay until phase 6 | Next, beside phase 2. #575 integrated (65). Open owners cover about 5,000: #573, #574 (#628), #652 (#655) and #653. About 35,000 have none: `afterload.cpp` and the old loader, chunk handlers, script API classes, content, admin and coordinator protocols, network core, and NewGRF configuration and actions |
| 4. Support | 45,100: support 20,687, graphics and viewport 10,981, startup 490, and data tables 12,931, which retire with their consumers | Not planned yet | Planned when phase 2 exits and phase 4 becomes the next phase (user direction, 2026-10-28) |
| 5. Interface | 71,570: GUI 52,064, platform 11,941, script and network GUI 3,793, and about 3,780 drawing lines in the logic bucket | Not planned yet | As above |
| 6. Vendored | 52,831. Only Squirrel (8,774) and md5, ICU and the social API (452) need a port or crate. nlohmann JSON (14,452), fmt (11,986), Catch2 (11,469) and the OpenGL headers (5,604) retire with their last C++ user | Not planned yet | As above |

Upstream unit tests (2,248) and fixture glue retire with the code they test.
Boundary-layer glue retires as its C++ callers do; phase 2 deletes its per-field part.

Root's first actions from this chart, after the resume checkpoint's cadence repair:
1. Run a planning round now: the corrected trigger fires, because no phase-2 or
   phase-3 owner is ready. Size phase 3's unowned 35,000 lines and rank owners
   by compiled C++ removed ([selection](planning.md#selection)), leaving the
   Squirrel VM and its binding shims. Place #656.
2. Finish #570's checkpoint 2, then #656 (alone or with #598 and #599, decided
   at planning). Every later pool owner's issue lists the shell fields Rust
   still reaches through services and moves them, as #634 and #635 already do,
   and names each field it leaves with the phase that moves it.
3. Record each owner's compiled-C++ estimate in its issue and its actual change
   at integration, and compare them at each stocktake.

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
| 65 | #575 script host, command context and event lifecycle / #651 | `6afdcbf390` | 386 / 364 / 354 / +220 |

## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and the disjoint-service rule; finishing review and integration
of an open branch comes first. A lower rank can run while a higher one waits.
Scopes live in issues; per-PR changes live in each PR's steering comment.

1. **#570 vehicle records (#636) to checkpoint 2.** It is phase 2's first owner and the template for every later pool.
   - First the four items in the twenty-seventh steering comment on #636: remove the `SetNext` bounce, switch handle identity with the first Rust field read, delete the retargeted per-field services with the design's inventory, and merge base, taking #643's generator.
   - Judge its cost by compiled C++ (original plus glue), not by its retired figure: rewritten consumer lines count on both sides. The audit estimates final net glue at -150 to -450, against the issue's -500 to -1,100.
   - Then #598 vehicle routing decisions and #599 vehicle base lifecycle as one owner in the shared vehicle files (plan-after64: #599 alone adds net glue). Remeasure the inventory after #570.
2. **Phase 1 has exited:** #559 integrated as 63 (#640). Every remaining non-drawing body moves with the phase-2 owner of its file; #621 lists them.
3. **Maintenance, one batch at a time** (one reviewer, one Full):
   - #647 (#644 boundary `bool` returns with #646 `TileArea::expand` overflow) is in review and integrates next. It overlaps #640 in eight files, so its base update over integration 63 needs a fresh scoped review unless every conflict keeps both sides ([review](workflow.md#review)). Its quick-CI gate then rejects new `bool` returns from Rust-defined `extern "C"` functions.
   - #649 (#604 settings access with #199's disjoint items and the `.inc` rebound coverage) is reviewed. It integrates after #651; its base update overlaps #651 in `src/rust/services.cpp` and `tools/migration/windows-abi.cpp`. Its inherited session `bool` returns are converted by #647 or by #647's conflict resolution, whichever integrates second (root's comment on #649).
   - #565 item 2, after #570 integrates, because both change `movement.rs`. Delete the map fallbacks and convert the remaining rail, crossing, depot, stop, water and tunnel leaves, `tile_slope_z`, and world generation's per-tile services (issue body), with #168's unfinished world-generation items. Production net glue -300 to -600. It also deletes the native `bridge_above` and `rail_tile_type` services that #616 and #620 duplicated with Rust map readers (`road_cmd.rs:932`, `road_cmd.rs:1443`, `station_cmd.rs:447`).
   - Later: #501 probe typing. #168/#199/#500's completed-job publication moves to the link-graph pool owner.
4. **The next phase-2 pools after #570 integrates** (ranked by plan-after56, 2026-11-01). Each needs #570's generator, save descriptors and record lookup. Rank and judge each by compiled C++ removed ([selection](planning.md#selection)).
   - Before briefing #634 or #635, root decides whether later records sit beside their shells, as #570's do, or inside them. Each moved-field read beside the shell costs a dependent load. Read the cost from the merged-tip profile after #570 and #566's drawing workload. If the cost is material, record the decision in the [world-state design](design/world-state.md) before the briefs.
   - #634 stations and road stops: 450 to 650 retired, -300 to -800 net glue, about 110 to 160 station-field slots. It removes the per-station `SharedForEachStation` bounce and #612's two C++ calls per k-d tree coordinate (`station/spatial.rs:12`).
   - #635 towns, industries and subsidies: 350 to 550 retired, -150 to -450 net glue, about 40 to 70 field slots. Disjoint from #634; the two may run together.
   - Then orders and order lists (441 left; read on every vehicle tick), #637 engines (545, with #497's engine bindings), companies and groups (510), link graph (607), #638 the smaller pools (615, with #498's ObjectSpec payloads), and the map (3,988 storage) with the rail, road, water, tunnel, landscape and tile bodies. Pools without an issue get one and a design section when selected. Each moves its coupled save adapters with each record, generates native declarations, verifies the layout, and deletes the displaced readers, services and their baseline rows.
5. **Phase 3 fills every other free slot:**
   - #628 (#574 network protocol): the resume checkpoint, its status comment and the steering comment on #628 hold the next round. #645 (62) and #640 (63) have integrated, so its close, receive, send and command-post bounces adopt them now; it does not build a second packet or command path.
   - #573's next PR, after #570 integrates: slice 3b changes the `SaveLoad` base and the `saveload.h` macros that #570's vehicle descriptors use. Its brief keeps the save-descriptor macros unchanged so it can run beside #634 and #635; otherwise whichever starts second waits. Then slice 4, the corrupt-input and LoadCheck probes, and the native `CopyFromOldName` helper, as #627's status comment lists.
   - #652 (#655, in implementation): GRF text and NewGRF Actions 3, 4 and 13, absorbing #497's re-planned Action 3 (plan-after64, comment on #3). Compiled C++ about -750 to -850. Formatting-time GRF string code (about 170 lines) stays C++ as phase 4. `Engine::grf_prop` and `overrides` stay with #637, and #498 with #638.
   - #653 industry and industry-tile specifications, after #647 and #649 integrate (both edit the industry files). Compiled C++ about -270 to -400; the spec records' vector fields are decided at its first checkpoint.

   Size each owner to repay its review and Full ([selection](planning.md#selection)). Plan no Squirrel port.
6. **#503 coverage continuation last:** first the bounded clock-preparation and sanitized native-test-support build fixes, then the 119 unfinished traces with provenance checks. Do not recollect just to fill a slot.

Recurring:
- After every second integration (next after 66), take stock under `/start-development` step 4, including the compiled-C++ rate and the baseline sizes. Profile the merged tip with the pinned `LD_LIBRARY_PATH` from [speed](speed.md#instruction-projections).
- Run the platform cadence after every fifth integration. The cadence after 65 is run 41466312349; the next is after 70. Fix a red cadence before further integrations.
- A quiet speed pass is owed in the next quiet window ([speed passes](speed.md#speed-passes)): #642 changed `rust/openttd-kernels/Cargo.toml`, so `simulate --instructions` rejects the anchors' identity. It pauses builds and games for timed collection, so run it when no agent is building, at the latest before the post-#570 record-placement decision.
- Deep checks run only on the user's request (user, 2026-10-31).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions.

Excluded, with return conditions checked by this review:
- #507's NewGRF corpus (#525, #526) stays parked by the user (2026-10-28) until the user reselects it. #525's branch stays on origin.
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.
- #654 asks the user whether to reselect #507's corpus for the NewGRF loading ports; root recommends keeping it parked until the Action 0 owners.

## Resume checkpoint (2026-11-02, wind-down after integration 65)

Paused at the user's request: wind-down at 16:20Z. Code is `6afdcbf390` (integration 65). `CI_ON_DEMAND` is `true`. No open head has a requested or passing Full. Each PR's status comment holds its detail; #654 awaits the user. Agents were stopped or told to stop after recording state; no local job result below is known unless stated.

Due first on resume, in order:
1. **Repair the red platform cadence** (run 41466312349) before any integration. Confirm the cause from the macOS `rust-native-libs.log` (evidence artifact or a local configure for `aarch64-apple-darwin`), then make the configure probe and the build agree on native libraries now that `libc` is a dependency, in a small root PR with Full and a platform run. Rerun the cadence on the merged fix.
2. Stocktake 66 (owed after integration 65's successor; take it at the next even integration), the quiet speed pass (recurring list) and the next planning round stay owed.

| Issue / PR | Head | Next action | Running job or CI |
| --- | --- | --- | --- |
| #644, #646 / #647 boundary `bool` returns and `expand` overflow | `9e0d5af362` | Base merge over integrations 61 to 65 done and validated (77 Cargo tests, 332 CTests, packet and auth comparisons, 16 simulations, cheap CI). /root/bool_review2 (Claude) was told to post its report and stop; it found gaps in the gate. Next: a fix round on its findings, a scoped re-review, Full, then merge after the cadence repair | Review report: check #647. No Full |
| #604, #199 / #649 settings access batch | `ba07c85bb1` | Full at `20eaf59ebb` failed `native / build` (generated-ABI include; fixed in `c48cdcd89e` and `128c1b9c96`); Full at `128c1b9c96` failed six scenarios. Cause: `SessionFixture::Run` advanced the company tick cursor after `19520dd248` deleted the editor stub. Fixture fix `ba07c85bb1`: 4 of 6 scenarios equal; the family run (56 `landscape-*`, `stations-commands-*`, `stations-cargo-*` scenarios including both `-reload` variants) result unknown. Next: the family run (rerun if unknown), the fixture unit tests including the `company_economy` cursor check, a scoped review of the fix, Full at the exact head, merge | Quick checks green at `ba07c85bb1`. No Full |
| #570 / #636 vehicle records | `6191d5f9b0` | Bounce removed, handle identity switched, ground and train records moved; build and 236 of 236 CTests passed; compiled C++ -377. Session 73637 left no result. Next: rerun the train-edge comparison and the 17 focused train and fleet scenarios, then finish checkpoint 2 (the remaining train, reservation, road and mixed-consumer per-field services and the design inventory), then review | None |
| #574 / #628 network protocol | `2711496991` | All four steering faults fixed (close, send, receive and command-post bounces; #645, #640 and #642 adopted); 33 obsolete exports retired; fixture clock fix pushed. Validation session 51484 result unknown. Next: rerun the scoped validation at `2711496991`, finish slice 4 if incomplete, then one round review (Claude) | None |
| #652 / #655 GRF text and NewGRF Actions 3, 4 and 13 | `54c44c68c5` | Text owner, translation and all three action handlers in; net glue +113 against the +50 estimate. Session 87540 failed 0 of 2: `newgrf-loading-bindings` and `-reload` do not compile the reference side of `tools/simulation/newgrf-loading-console.cpp` (a `CargoLabel` passed where `GetAiPurchaseCallbackResult` takes a `CargoType`). Next: fix that fixture, rerun the two scenarios, take metrics, then review by function group | None |

PR worktrees (preserve): #647 `t3-c4634554`, `t3-17a0c009` and the cancelled reviewer's `t3-2efde48a`; #649 `t3-13298c8f`, `t3-6e7160e6` and `t3-b4561a0d`; #636 `t3-c67aada9` and `t3-41b02f9e`; #628 `t3-eb8e9c59`, `t3-1e4ffc20`, `t3-3208e14e`, `t3-5aa8a17f`, `t3-fc38354a` and `t3-e0d0c168`; #655 `t3-986b5854`. The merged and finished review checkouts are T3-locked; housekeeping keeps them until T3 releases them.

Keep unchanged: `t3-120ef117` (old #603), `t3-431c84b4` (cancelled #565 source with 58 dirty entries), `t3-d78d5f8b` (native fixture evidence), `.local/reference/openttd` at `14ec60f248` and the instruction-profile worktrees under the main checkout's `.local/worktrees/`.
