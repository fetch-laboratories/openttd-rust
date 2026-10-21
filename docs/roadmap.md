# OpenTTD-Rust near-term roadmap

Root owns selection here; `AGENTS.md` and `docs/rust-migration.md` define process.
If an issue conflicts, follow this roadmap and report it to root. Keep this
forward-looking, about 200 lines; completed work is one row, with evidence in PRs.

## Where the fork stands (2026-10-20, code at `067e3c3b93`)

- Thirty-two ownership ports. Whole fork: Rust 75,597 / tooling 30,974 / glue 31,046 / original retired 43,935 of 470,612 `src/` lines (9.34%). Simulation: 35,985 / 102,615 (35.07%).
- Since the fourteenth review: #255 landscape (846 retired) and #303 stations (2,427), plus #285/#286/#287/#298 and the #275 fixes #289/#300/#318/#319. Water #317 adds 512 retired; station fidelity #323 adds default witnesses. Retirement +3,790; glue plus tooling +2,038.
- Speed record (#318): TGP 1.187, Opus 1.231, Grok 1.203, P1996 1.256, P2000 1.321, P2016 1.388. Four quiet steering medians per scenario confirm it within 1%. CI is green; CI_ON_DEMAND=true.
- Ship scratch #308, station fidelity #323 and water #317 are integrated after review, quiet speed and exact-head Full. Three component drafts remain: #269 houses awaits validation; #313 completes object policy and #290 adopts shared services. Guard #322 and #261 R5 are maintenance. The latest quiet ratios pass; #156 gaps remain open. After #269, the fourth ownership integration since the profile makes a speed pass due.

## Fifteenth steering review (2026-10-20, honest speed and one shared layer)

Independent audits of #303, #255, #290 and #317, plus a profile and noise study, found:

- **The speed guard measured host load.** #269 and #308 were parked on P2016 1.558-1.677 and Grok 1.405-1.493, timed while agents compiled. Quiet reruns at their exact heads pass every limit: #308 P2016 1.354-1.382, #269 1.390/1.414. Root's own #275 attribution had shown houses add 0.34% loop Ir. Compile load moves P2016 medians 2-7%, concurrent games about 10% either way, and single samples 25%; game-loop callgrind counts move under 0.04%.
- **Three reachable divergences in integrated #303** (verified at source, #320):
  - Road-stop removal stores `dest_tile` raw instead of calling `SetDestTile`, leaving a stale road-vehicle path.
  - A hand-written bound accepts the south-east edge tile that `TileAddWrap` rejects.
  - It tests `0x3f` where the original tests `INVALID_RAILTYPE`.

  One 35-minute review covered 6,985 lines and checked boundaries, and the station scenario has no vehicles and avoids map edges. Like #269 R1, #290's slope units and #303's tram predicate, each of the three hand-writes an original helper, virtual or constant.
- **Shared services were built four times.** Root gave the same command/finance/clearing fix to #290, #303, #313 and #317 in parallel. #303 landed first, and the other three now conflict in 24-38 files, mostly add/add duplicates.
- **#275 misses the binding scenario.** On P2016, the scenario nearest its ceiling, aircraft is 36% of the gap (about 0.45G of it per-field reads), road 29%, trains 13% and the station tick 8%; neither trains nor the station tick is on #275. On TGP the clear-tile reads are 66% of the gap and also unlisted (#275 comment).
- **Leftovers:**
  - #255 left five tile-loop facades and two exports compiled but never called, plus a flooding-to-signal bounce (#261 R5).
  - #290 has three more bounces than root recorded (PR comment).
  - #317 has no divergence; its adoption list is in a PR comment.

AGENTS.md now says:
- Per-PR speed uses game-loop instruction counts projected onto the recorded ratio (#321), and wall medians count only on a quiet host.
- Callees are ported as callees.
- Reviews compare callee by callee first, split by function group above about 1,000 retired lines.
- A shared service that more than one draft needs lands once, as its own PR.

Actions, in order:
1. **#308:** record the quiet medians from its PR comment, run exact-head Full and integrate.
2. **#320 and #321 in parallel.** #320 is the station fidelity fix, with default witnesses for a vehicle en route to a removed stop and for map-edge builds and removals. #321 is the instruction-count guard. Until #321 lands, wall medians count only with no builds, links or other timed games running.
3. **#269:** update from base, run a scoped conflict review if needed, take quiet medians, run Full and integrate. No new fix round.
4. **#317, then #313, then #290:** each adopts #303's shared layer and deletes its own copies (PR comments). #290 also removes its five bounces. #313 reassesses its retirement ratio after its duplicate glue is gone.
5. **#275 in the corrected order:** aircraft per-field reads, the road per-tick fold, station-tick per-slot reads, then train per-field reads. For TGP, the clear-tile reads come before #312 trees.
6. **#261 R5** in a maintenance slot.

## Earlier steering, still in force

- **Merge churn and process (fourteenth):** #280 registries, #282 15% guard with 1.5x/1.3x ceilings (user decision), #283 receipt retirement and #284 generated ABI are integrated. Evidence is exact-head CI plus the PR's commands. Reviewers rerun only what their fixes touch. Drafts update from base only to integrate or to clear a blocking conflict. Root spot-checks fix commits. Commits that only fix warnings or record medians need no review. Each task keeps one status comment.
- **Core loops (thirteenth):** #257 and #255 integrated under the current guard. The vehicle raw-offset view is closed (#274). Command ports share one set of command checks. Duplicate wrappers found at review are deleted before integration.
- **Housekeeping (twelfth):** #259 H1-H4, #260 and #261 R1/R3-part/R4 are done. R2 follows #269, and H5 comes last. At most three maintenance PRs are in flight. They never count against the component cap or block an ownership port.
- **Direct shared services and boundaries:** direct shared services (#107/#168/#199) and one definition per shared accessor, enforced in quick CI by #249, whose baseline only shrinks. Use a fixed hasher for lookup-only YAPF maps; #284 generates named ABI layouts and fixture defaults.
- **Ownership:** prioritize simulation ownership. Keep state at the original's process scope.
- **Map storage:** #108 retains C++ map arrays with direct bundled services until #274's map view lands.
- **Harness evidence:** end moments (#154), default branch witnesses and reference probes (#198/#204/#210/#211), the speed guard (#155/#282) and documented coverage gaps (#156).
- **Tooling and WIP:** proportionate tooling (#109) and a stated budget for infrastructure work. Prefer platform defaults to controllers (#233). The WIP cap is six component branches.
- **CI and review:** full CI is exact-head and on demand (#171), with concurrent validation (#233) and no admin bypass. Fresh reviewers fix their findings.
- **Metrics:** original retirement excludes removed fork glue (#223).

## Completed ownership ports

Metrics are `tools/port-metrics.py`: Rust / tooling / C++ glue / C++ retired.

| Issue | Component | PR | Commit | Metrics |
| --- | --- | --- | --- | --- |
| #73 | TGP terrain generation | #94 | `014968bd38` | 658 / 85 / 84 / 991 |
| #74 | Link graph job computation | #95 | `7a2858c469` | 1040 / 159 / 135 / 1120 |
| #96 | Built-in town names (retired count is mostly data tables) | #98 | `ff3648ba09` | 3296 / 241 / 122 / 3848 |
| #99 | Tree generation, simulation, planting | #102 | `602b5a60ff` | 1288 / 561 / 247 / 816 |
| #101 | Effect vehicles | #105 | `f6a0d66b63` | 494 / 553 / 412 / 548 |
| #104 | Water-region cache and graph service | #114 | `9732d8bfa2` | 459 / 610 / 273 / 407 |
| #103 | Disaster scheduling, vehicles and event control | #118 | `d5edfcb4c5` | 1404 / 795 / 306 / 968 |
| #117 | Cargo payment and delivery | #123 via #134 | `adbe063372` | 396 / 618 / 370 / 315 |
| #119 | Ship YAPF and canonical path cache | #126 via #134 | `adbe063372` | 881 / 54 / 506 / 646 |
| #120 | Town growth and private state | #127 via #134 | `adbe063372` | 1430 / 668 / 625 / 1548 |
| #121 | Road controller/private state (includes 1475 movement-data lines) | #128 via #134 | `adbe063372` | 3159 / 471 / 1188 / 2842 |
| #122 | Rail YAPF search, cache and reservation | #133 via #141 | `ea80a746a3` | 1704 / 144 / 595 / 1517 |
| #124 | Road YAPF search and path construction | #135 via #141 | `ea80a746a3` | 663 / 55 / 329 / 532 |
| #125 | Station cargo-service control and state | #142 | `ba920555ca` | 1516 / 241 / 1022 / 1332 |
| #130 | Rail controller, reservation and private state | #143 via #166 | `dace87c9b1` | 4087 / 613 / 1660 / 2585 |
| #129 | Industry production, histories and builder state | #144 via #166 | `dace87c9b1` | 1508 / 474 / 678 / 932 |
| #136 | Aircraft controller and airport blocks | #145 | `f0e0b3d712` | 1773 / 764 / 380 / 1880 |
| #137 | Company finance, economy and lifecycle | #149 | `dbce82a309` | 1607 / 702 / 1052 / 1394 |
| #139 | Cargo storage and movement | #151 via #184 | `2a76d5f287` | 2838 / 460 / 973 / 2445 |
| #138 | Orders lifecycle and commands | #176 via #184 | `2a76d5f287` | 4554 / 519 / 1752 / 2518 |
| #146 | Ship controller and private state | #152 via #184 | `2a76d5f287` | 1101 / 270 / 893 / 721 |
| #147 | Fleet groups and autoreplace | #197 | `a7a6e6650a` | 2686 / 577 / 1199 / 1454 |
| #150 | Industry construction and lifecycle | #218 | `648d44e940` | 3066 / 701 / 1433 / 1736 |
| #148 | Town canonical state and periodic policy (narrow scope) | #216 | `59385bce59` | 1848 / 694 / 1418 / 340 |
| #220 | Engine lifecycle and canonical design state | #248 | `d2115d1c3d` | 1218 / 369 / 833 / 579 |
| #219 | Rail signal blocks and fixed work sets | #245 | `9487f3079f` | 1229 / 812 / 445 / 646 |
| #246 | Town founding, generation and names | #258 | `09bb0b4df8` | 1123 / 321 / 470 / 492 |
| #265 | Rail track, depot and signal commands | #270 | `0cb20cec28` | 2965 / 301 / 740 / 1578 |
| #213 | Vehicle scheduling and base maintenance | #257 | `8bfcbd3abd` | 1833 / 375 / 1160 / 388 |
| #212 | Landscape scheduling, ground evolution and flooding | #255 | `377b374579` | 2065 / 553 / 1095 / 846 |
| #267 | Station construction and removal commands | #303 | `ca2b181d88` | 5233 / 333 / 1388 / 2427 |
| #294 | Water construction, clearing and restoration | #317 | `067e3c3b93` | 871 / 249 / 228 / 512 |

Harness and process: #72 harness (#85), #84 play saves (#87), #97 provenance
freeze (#100), #88 Ruff (#92), #75 partial-pixel fidelity (#91), #90 world-state
design (`docs/design/world-state.md`). Maintenance PRs #209-#302 (shared
services, CI throughput and cost, disk hygiene, #249 enforcement, #280 registries,
#282 speed guard, #283 receipt retirement, #275 road YAPF/scratch/table fixes)
keep their evidence in their PRs. Rows below use original retirement and net glue (#223).

| Issues | Maintenance PRs | Integration | Commit | Metrics |
| --- | --- | --- | --- | --- |
| #307 (#156 scheduler deletion/pool growth) | #309 | direct | `ad58d65c10` | 0 / 7 / 74 / 0; tests only |
| #284 (generated ABI and typed defaults) | #298 | direct | `79dea34092` | 4541 / 287 / 2650 / 0; net glue +196 |
| #311 (#275 road speed/effect boundary) | #319 | direct | `08053fda50` | 88 / 35 / 101 / 5; net glue +90 |
| #310 (#275 immutable road service tables) | #318 | direct | `4c4c5a0eab` | 0 / 27 / 312 / 0; net glue +7 |
| #306 (#275 ship search scratch) | #308 | direct | `e996032875` | 108 / 0 / 0 / 0; net glue 0 |
| #320 station command fidelity | #323 | direct | `bc25c51d44` | 60 / 118 / 34 / 0; net glue -5 |

Paused, not fallbacks: #64 curve family, #68 SHA-512/Ed25519, #69 tile areas.

## Phase 1: harness and speed maintenance

The default semantic harness runs in CI; regressions come first. Port differences
go in `KNOWN_FAILURES` with an issue, never in masks (`docs/rust-migration.md`).

1. **Shared-accessor check (#249).** A new per-port copy fails quick CI. The baseline of existing copies only shrinks; #259 H4 extends the check to orphaned exports and duplicate table bindings.
2. **Speed guard and passes (#155, #282, #321, AGENTS.md).**
   - **Record:** `SPEED_BUDGETS` in `tools/simulation/roads.py` holds current `rust-migration` ratios; #321 adds fixed measured anchors and separately rolling integrated instruction projections. Root reselected 560 added tooling/test/workflow lines for R2 after the real profile build exposed valid minimal DWARF rejection and broken frozen profiling; 505 preceded these fixes. No new controller.
   - **Per PR:** no scenario's projected ratio more than 15% above its record and none over its ceiling (1.5x play saves, 1.3x generation). Until #321 lands, use quiet wall medians. Misses get a callgrind fix round.
   - **Speed pass:** after every fourth ownership integration (#255 and #303 count two since the #275 profile), or when a projection comes within 5% of its ceiling (1.425x play, 1.235x generation). The fifteenth steering profile on #275 is the current attribution.
   - **Redraw:** `-r 64x64` leaves the ratios unchanged, so redraw does not inflate them. It does make whole-process counts timing-dependent, which is why only the game loop is counted.
3. **#156 coverage, standing capacity.** Random and crash branches come first. These remain:
   - signal tbd/global propagation overflow and arbitrary mod layouts;
   - town exclusivity, unwanted months and authority refusals;
   - engine external NewGRF/retire_early and legacy save branches;
   - #218's NewGRF and game-script paths;
   - the #257, #268-#270 and #255 audit gaps (#156 comments, 2026-10-16 and 2026-10-18);
   - command scenarios with live vehicles on and toward affected tiles and at map edges, plus the #303, #255, #290 and #317 gaps (#156 comment, 2026-10-20).

## Phase 2: current work, in order

Integrate reviewed work first, in order of readiness, with at most six
unintegrated component branches. Maintenance (#261, #275, #321) and #156 run in parallel.

1. **Unblock and fix:** #308, #320/#323 and #317 are integrated. Complete #321/#322 while #269 finishes validation and integrates, as in the fifteenth actions.
2. **Command drafts, in order of readiness:** #313 objects, #290 road. Each adopts the integrated shared layer and builds no parallel copy. #313's remaining 57-line retirement gap gets a selected object NewGRF query/animation ownership extension (#295), about 250 retired for at most 100 net glue+75 tooling; tooling 331 is the starting budget. No waiver or deferred dependency.
3. **#275 corrected order:** #325 aircraft movement observations (typed call-scoped addresses, net glue at most 120 and tooling 100), road per-tick fold, station-tick per-slot reads, train reads; TGP clear-tile reads, then #312 trees and industry `next_tick`. Ship scratch is #308.
4. **#281** vehicle maintenance direct calls, then the callee-copy check.
5. **#274 map view**, one shared PR.
6. **Next whole owners**, as WIP frees, each without draft service dependencies:
   - #314 common vehicle transactions, nonrail construction and articulated assembly;
   - #315 rail construction and consist transactions;
   - #316 subsidies (glue plus tooling at most 450 lines);
   - #247 town commands, authority and deletion, after #269 integrates;
   - #293 tunnel/bridge construction, removal and ownership, once #247 settles.

   A shared service two of them need lands first, as its own PR.
7. **#168 tail, largest first.** `order_cmd.cpp` and `economy.cpp` hold 65 of the 198 baseline copies.

New owners reuse integrated services, never draft copies. A Rust-allocated
state block with C++ reference aliases (#216, #248) is acceptable. Keep save
staging only for a field a `SaveLoad` descriptor cannot address through the alias.

## Resume checkpoint

Resumed on 2026-10-20 with the fifteenth steering actions. Root on this host:
gpt-6.1-sol, ultra; implementation and reviews: gpt-6.1-sol, high. Source reviews
are complete; root validates ready heads while road/object and calibration work proceed.

| Issue / PR | Head | Next step |
| --- | --- | --- |
| #320 / #323 | `bc25c51d44` | Integrated after review, quiet six and exact-head Full; worktree removed. |
| #321 / #322 | `e0b80cd745` | R2 fixes real DWARF and frozen-run blockers within 560 additions; profiles/calibration/quiet anchors follow. |
| #256 / #269 | `330d710e7b` | Scoped review fixed three clear-square bounces; root spot-check passes. Quiet six pass; exact-head Full requested. |
| #294 / #317 | `067e3c3b93` | Integrated after scoped review, four uncontaminated quiet sets and Full; worktree removed. |
| #295 / #313 | `4bc35b6d4d` | Query/animation extension passes early metrics and Cargo/native checks; default probes and semantics precede fresh split review. |
| #266 / #290 | `17bc99cd5a` | New agent adopts canonical services and removes five bounces, then gates. |
| #261 R5 / #324 | `45124e12e1` | Implementation checks pass; fresh fixing review precedes quiet speed and Full. |
| #325 | selected | Aircraft controller/position address contract selected; implement from integrated base, preserve mutation witnesses. |
| #274 | `162843acbd` | Paused measurement branch; map view stays in the work order. |
| #64 / #66 | `aab5211801` | #66 closed unmerged; #64 and older paused work untouched. |

#68/#69 are unchanged. Each open draft keeps one worktree and branch. Preserve the
pinned reference. Do not reapply effect helper `73ccd511fb`. Build/test with `--jobs 2`.

## Out of scope for the near term

More `src/3rdparty` ports, GUI rendering/layout and platform/toolchain expansion.
Deferred behavior improvements stay in `deferred-improvement` issues.

## Progress metric

Every port PR pastes `python3 tools/port-metrics.py`: Rust/tooling/glue added, original retired, fork glue removed and net glue.
`--whole-fork` gives totals and simulation share. Retire more original C++ than net glue plus tooling or explain why.
A port that misses this ratio gets a boundary fix (shared services, #274 views) before deferral.
Judge game logic owned; data tables inflate retirement. Avoid utility kernels.
