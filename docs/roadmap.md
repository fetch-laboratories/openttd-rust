# OpenTTD-Rust near-term roadmap

Root owns selection here; `AGENTS.md` and `docs/rust-migration.md` define process.
If an issue conflicts, follow this roadmap and report it to root. Keep this
forward-looking, about 200 lines; completed work is one row, with evidence in PRs.

## Where the fork stands (2026-10-21, code at `5e2f570eee`)

- Thirty-four ownership ports. Whole fork: Rust 79,796 / tooling 31,676 / glue 32,246 / original retired 46,102 of 470,612 C/C++ `src/` lines (9.80%). Simulation: 38,062 / 102,615 (37.09%).
- Since the fifteenth review, water #317, houses #269 and roads #290 integrated, retiring 2,667 lines. Seven maintenance PRs (#308, #323, #324, #326, #328, #332, #333) retired 12 lines.
- Speed: `SPEED_BUDGETS` still holds the #318 record. The last quiet six (#333 head) measured TGP 1.203, Opus 1.212, Grok 1.213, P1996 1.258, P2000 1.320 and P2016 1.371. The speed pass due since #269 closes when #322 records its anchors.
- Drafts: #322 speed guard, #313 objects, and #329 vehicle transactions (non-passing). #274 is a paused measurement branch.

## Sixteenth steering review (2026-10-21, counts per PR and closing the speed pass)

Independent Claude audits of #290, #269, #317, #333 and draft #313, plus a process audit of root's 2026-10-21 session (258 minutes, 40 child agents), found:

- **Quiet windows cost 27% of root's time and caught nothing.**
  - Six holds stopped all native work for about 71 minutes, and all ten measured heads passed.
  - Quiet P2016 spanned 1.350-1.439 across heads whose loop counts moved under 0.5%. #317 water, which barely runs in the play saves, measured 1.439. That spread comes from code layout, not code cost.
- **#322 grew to 3.7x its budget and caught no game defect.**
  - Five re-selections in 56 minutes took it from 150 to 560 tooling lines. R1 and R2 fixed defects in the tool itself, mostly hypothetical on this host.
  - The calibration's "factor-two underprediction" (houses: P2016 +4.27% wall at +0.29% loop Ir) is the same layout spread. Quiet sixes at heads that include houses measured 1.350-1.397.
- **The due speed pass was parked on #322 without cause.** Its profiles and attribution are done, and #326 and #332 came out of it; only the new record is missing.
- **The fifteenth review's speed figures were gross callback counts.** Aircraft (0.45G claimed) and the road fold (156M claimed) recovered 60M and 26M of net loop Ir. AGENTS.md already ranks net costs; that review's own numbers did not.
- **No reachable divergence in any of the five audited ports.**
  - Remaining callee copies and literals: `GetSlopeMaxZ` three times, `IsSlopeWithOneCornerRaised` three times, command flag constants four times. There is no shared Rust module for header helpers, and `shared-accessors.py` cannot see Rust-internal copies.
  - The house tile loop bounces Rust to C++ to Rust (`tile_loop.rs:166` through `town_cmd.cpp:497`).
  - #333 left five owner facades orphaned.
  - The road tick builds a command context on every vehicle part every tick, though only tram turns use it (`roadveh_cmd.cpp:1796`).
- **Slipped:** #281 (since 10-18), #274 (since 10-16), #247 (since 10-15), and #156, which no agent worked on. Two conditions were circular: ratio misses waited on the parked #274 views, and #293 waited on unscheduled #247.

**User decision (2026-10-21):** each PR is checked on projected instruction counts (#321 as built). Wall time is measured only in speed passes. This settles the policy question #322 returned to the user.

AGENTS.md now says:
- A PR does not answer for the layout spread. Count projections run alongside other agents with no quiet window.
- An integration that moves a projection into the 5% band triggers a pass; a pass's own wall measurement does not. Passes rank net costs.
- Root raises an infrastructure budget once; a second raise is a `question` issue. Infrastructure reviews fix only defects that can change a game comparison or a speed decision.
- Header helpers and constants are ported once into a shared Rust module named after the header.
- A Claude Code root keeps independent review on gpt-6.1-sol through T3 `delegate_task`.

Provisional, checked next review against #322's first anchors: the factor 2 (basis: the fifteenth review's 1.8x P2016 attribution; the calibration's residuals were layout) and the 6.6% layout spread (eight quiet P2016 sixes on 2026-10-21).

Actions, in order:
1. **#322 closes the speed pass.** Adopt the integrated base and refresh the six loop counts. One quiet wall pass at that head (four medians per scenario, line-tables-only builds) gives the first anchors and the new `SPEED_BUDGETS` record. No new tooling or calibration; HARD560 stands. Only an adoption that conflicts in `tools/` gets a scoped review. Then Full and integrate. After that, no PR takes a quiet window.
2. **#313:** apply its PR comment:
   - adopt #333 (direct owner dispatch with the outer context) and #328 (`command::nested`);
   - use one shared accessor each for recession and ownership;
   - restore the five portable originals under `#else`;
   - add a default witness for clearing an object back to water; then a scoped review, the count guard and Full.
3. **#329:** fix the null fleet list context first, then the road and aircraft constructors and articulated assembly. Tooling cap 500.
4. **#156 standing slot:** whenever fewer than five agents are busy, one works the gaps in the 2026-10-21 #156 comment, starting with ownership dispatch, which every owner now goes through.
5. **#261 R6** in a maintenance slot: create the shared header-helper modules, consolidate the copies, remove the house tile-loop bounce, delete the orphaned owner facades, and read `_price` through `crate::company::price`. The shared-accessor baseline shrinks.
6. **#275, ranked by net P2016 savings:** the per-tick road command context, the station per-slot reads, then the train reads.
7. Then Phase 2 from item 4 (#281).

## Earlier steering, still in force

- **Speed and the shared layer (fifteenth):** port callees as callees. Reviews compare callee by callee first, split by function group above about 1,000 retired lines. A shared service more than one draft needs lands once, as its own PR (#328, #333). #320's station fixes and #308 are integrated.
- **Merge churn and process (fourteenth):** #280 registries, #282 15% guard with 1.5x/1.3x ceilings (user decision), #283 receipt retirement and #284 generated ABI are integrated. Evidence is exact-head CI plus the PR's commands. Reviewers rerun only what their fixes touch. Drafts update from base only to integrate or to clear a blocking conflict. Root spot-checks fix commits. Commits that only fix warnings or record medians need no review. Each task keeps one status comment.
- **Core loops (thirteenth):** #257 and #255 integrated under the current guard. The vehicle raw-offset view is closed (#274). Command ports share one set of command checks. Duplicate wrappers found at review are deleted before integration.
- **Housekeeping (twelfth):** #259 H1-H4, #260 and #261 R1/R3-part/R4/R5 are done. R2 is unblocked now that #269 has integrated; H5 comes last. At most three maintenance PRs are in flight. They never count against the component cap or block an ownership port.
- **Direct shared services and boundaries:** direct shared services (#107/#168/#199) and one definition per shared accessor, enforced in quick CI by #249, whose baseline only shrinks. Use a fixed hasher for lookup-only YAPF maps; #284 generates named ABI layouts and fixture defaults.
- **Ownership:** prioritize simulation ownership. Keep state at the original's process scope.
- **Map storage:** #108 retains C++ map arrays with direct bundled services until #274's map view lands.
- **Harness evidence:** end moments (#154), default branch witnesses and reference probes (#198/#204/#210/#211), the speed guard (#155/#282/#321) and documented coverage gaps (#156).
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
| #256 | House lifecycle and tile simulation | #269 | `5ac2437aa5` | 1659 / 212 / 685 / 890 |
| #266 | Whole road command policy | #290 | `4e7753122e` | 2547 / 375 / 545 / 1265 |

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
| #261 R5 tile-loop cleanup and signal boundary | #324 | direct | `69d0365a9f` | 14 / 0 / 36 / 11; net glue -11 |
| #327 shared direct command lifecycle | #328 | direct | `764e1fa92d` | 45 / 0 / 0 / 0; net glue 0 |
| #325 aircraft movement observations | #326 | direct | `7b0a82cbd1` | 213 / 35 / 99 / 0; net glue +56 |
| #330 road tick prologue | #332 | direct | `4476646d19` | 22 / 36 / 67 / 0; net glue +36 |
| #331 shared ownership dispatch | #333 | direct | `5e2f570eee` | 143 / 90 / 122 / 1; net glue +101 |

Paused, not fallbacks: #64 curve family, #68 SHA-512/Ed25519, #69 tile areas.

## Phase 1: harness and speed maintenance

The default semantic harness runs in CI; regressions come first. Port differences
go in `KNOWN_FAILURES` with an issue, never in masks (`docs/rust-migration.md`).

1. **Shared-accessor check (#249).** A new per-port copy fails quick CI. The baseline of existing copies only shrinks; #259 H4 extends the check to orphaned exports and duplicate table bindings. Rust-internal helper copies are invisible to it; reviewers check them against the shared header modules (#261 R6).
2. **Speed guard and passes (#155, #282, #321, AGENTS.md).**
   - **Per PR:** projected loop counts against the anchors #322 records; no scenario more than 15% above its record and none over its ceiling (1.5x play saves, 1.3x generation). Until #322 lands, quiet wall medians. Misses get a callgrind fix round.
   - **Speed pass:** after every fourth ownership integration, or when an integration moves a projection within 5% of its ceiling (1.425x play, 1.235x generation). Its quiet six with loop counts replaces the anchors. #322's integration closes the pass due since #269. The next one falls due after four more ownership integrations.
   - **Redraw:** `-r 64x64` leaves the ratios unchanged. It makes whole-process counts timing-dependent, which is why only the game loop is counted.
3. **#156 coverage, one standing slot** (sixteenth review, action 4). Random and crash branches come first. These remain:
   - signal tbd/global propagation overflow and arbitrary mod layouts;
   - town exclusivity, unwanted months and authority refusals;
   - engine external NewGRF/retire_early and legacy save branches;
   - #218's NewGRF and game-script paths;
   - the #257, #268-#270 and #255 audit gaps (#156 comments, 2026-10-16 and 2026-10-18);
   - command scenarios with live vehicles on and toward affected tiles and at map edges, plus the #303, #255, #290 and #317 gaps (#156 comment, 2026-10-20);
   - ownership dispatch, water, house, object and road gaps (#156 comment, 2026-10-21).

## Phase 2: current work, in order

Integrate reviewed work first, in order of readiness, with at most six
unintegrated component branches. Maintenance (#261, #275) and #156 run in parallel.

1. **#322**, then **#313**, then **#329/#314** (sixteenth review, actions 1-3).
2. **#275 by net P2016 savings:** the per-tick road command context, the station per-slot reads, then the train reads. Retain the native movement and speed algorithms. TGP #312 trees stay ahead of clear observations on the 2026-10-21 net attribution (#275 comment); TGP is not the binding scenario.
3. **#281** vehicle maintenance direct calls, then the callee-copy check.
4. **#274 map view**, one shared PR.
5. **Next whole owners**, as WIP frees, each without draft service dependencies:
   - #315 rail construction and consist transactions, after #314;
   - #316 subsidies (glue plus tooling at most 450 lines);
   - #247 town commands, authority and deletion (unblocked: #269 is integrated);
   - #293 tunnel/bridge construction, removal and ownership (independent of #247: it reuses the town authority and rating owner #290 integrated).

   A shared service two of them need lands first, as its own PR. When fewer than two remain unstarted, plan the next owners from the largest unported simulation files: `vehicle.cpp` (2,568 lines), `station_cmd.cpp` (2,124), `train_cmd.cpp` (1,778) and `landscape.cpp` (1,618).
6. **#168 tail, largest first.** `order_cmd.cpp` and `economy.cpp` hold 65 of the 198 baseline copies.

New owners reuse integrated services, never draft copies. A Rust-allocated
state block with C++ reference aliases (#216, #248) is acceptable. Keep save
staging only for a field a `SaveLoad` descriptor cannot address through the alias.

## Resume checkpoint

Resumed 2026-10-21 by the sixteenth steering review. Root runs as Claude Opus 5.5
(xhigh) in Claude Code: implementation and planning on Claude models below xhigh,
and independent review on gpt-6.1-sol (high) through T3 `delegate_task`.

| Issue / PR | Head | Next step |
| --- | --- | --- |
| #321 / #322 | `535761dc9d` | Action 1 (`/root/speed_anchors_322`): adopt base, refresh counts, one quiet pass (four medians) as anchors; then root's workflow-bootstrap Full and integration. |
| #295 / #313 | `d6297bdeef` | Action 2 (`/root/object_adopt_313`): steering PR comment items 1-6; then scoped review, count guard, Full. |
| #314 / #329 | `de32f6f990` | Action 3 (`/root/vehicle_transactions_314_r2`): null fleet list context first; last passing head `da9f1c0464`. |
| #156 / #335 | `2a14a97a08` | Action 4: ownership-dispatch rows closed; review `/root/review_ownership_coverage_335_r1`, then Full. |
| #261 R6 / #334 | `58fb82abd3` | Action 5 (`/root/shared_header_helpers_261_r6`): shared header modules, house bounce, orphaned facades. |
| #274 | `162843acbd` | Paused measurement branch; map view stays in the work order. |

Only `/root/speed_anchors_322` takes a quiet window; other agents wait while `.local/quiet-window` exists. A stale marker with no live #322 task is deleted.

#64/#68/#69 are paused. Each open draft keeps one worktree and branch. Preserve the
pinned reference. Do not reapply effect helper `73ccd511fb`. Build and test with `--jobs 2`.

## Out of scope for the near term

More `src/3rdparty` ports, GUI rendering/layout and platform/toolchain expansion.
Deferred behavior improvements stay in `deferred-improvement` issues.

## Progress metric

Every port PR pastes `python3 tools/port-metrics.py`: Rust/tooling/glue added, original retired, fork glue removed and net glue.
`--whole-fork` gives totals and simulation share. Retire more original C++ than net glue plus tooling or explain why.
A port that misses this ratio gets a boundary fix (shared services) before deferral.
Judge game logic owned; data tables inflate retirement. Avoid utility kernels.
