# OpenTTD-Rust near-term roadmap

Root owns selection here; `AGENTS.md` and `docs/rust-migration.md` define process.
If an issue conflicts, follow this roadmap and report it to root. Keep this
forward-looking, about 200 lines; completed work is one row, with evidence in PRs.

## Where the fork stands (2026-10-20, code at `4c4c5a0eab`)

- Thirty-one ownership ports. Whole fork: Rust 74,681 / tooling 30,620 / glue 30,850 / original retired 43,423 of 470,612 `src/` lines (9.23%). Simulation: 35,473 / 102,615 (34.57%).
- This development session integrated #309 scheduler deletion/growth coverage, #298 generated ABI/defaults, #303 station command ownership and #319 road speed/effect plus #318 constant-table maintenance after review and exact-head Full. Station landed canonical command, clear-square and restoration services first.
- Paused at the user's request after the final wind-down gates. Remaining drafts keep their branches/worktrees and recorded blockers; no new owners or planning started. #269/#308 and older paused work are unchanged.
- Current speed record (#318): TGP 1.187; Opus 1.231, Grok 1.203, P1996 1.256, P2000 1.321, P2016 1.388. Protected-branch CI is green; CI_ON_DEMAND=true.
- Coverage (#156): #309 adds deletion/pool-growth assertions. Station, water, road and object review gaps remain recorded there; passing scenarios do not establish complete equivalence.
- Integrate reviewed work before starting selected next owners. The work order below is unchanged; the resume checkpoint names each draft's next gate.

## Fourteenth steering review (2026-10-18, merge churn and honest exceptions)

Audits found merge churn, duplicate leaves, house-override fidelity and landscape bounces; component PRs retain evidence.
The selected order remains below. #269 blocks on speed; #255's design/speed gates passed.

**Speed policy (user decision, 2026-10-18).** The 3% ratchet blocked on noise. Padhattan 2000/2016 noise is about 2%, and today's caps fail an unchanged build most of the time. Speed gating took about 20-30% of subagent time, and the real finds came from periodic profiles. AGENTS.md now has a 15% per-PR guard, ceilings of 1.5x (play saves) and 1.3x (generation), no per-port CPU budgets, and a speed pass after every fourth ownership integration. #282 changes the harness.

**Process changes (user decision, 2026-10-18).** 87% of ready-to-merge time passed with no root session running, 22 of 43 review rounds found nothing or only evidence items, and root's re-verification of fix commits rejected nothing. AGENTS.md now makes exact-head CI plus the PR's commands the evidence, with no receipts; #283 retires that tooling and stops Markdown commits from forcing post-merge runs. Reviewers rerun only what their fixes touch. Drafts update from base only to integrate or to clear a blocking conflict. Root spot-checks fix commits. Commits that only fix warnings or platform errors, or only record medians, need no review. Platform CI runs with the pre-merge full validation (#304). Each task keeps one status comment. Tooling budgets allow about 25% over. Hand-written ABI lists were involved in 46 of 97 conflicted merges, and #284 generates them and gives fixtures aborting default slots.

Actions, in order:
1. **#280, #282, then #284** (maintenance PRs): per-component witness blocks and an ordering convention for shared append lists; the speed guard fields and the first measured record; generated ABI layout lists and default fixture slots. AGENTS.md now lets a base update that keeps both sides' lines skip review.
2. **#269:** fidelity/copy/probe fixes and steering reviews passed; use the completed #275 attribution for the owned speed fix round, then review, Full and integration.
3. **#255:** integrated as `377b374579` after the six must-change items, fresh reviews, ordered quiet repeats and Full passed.
4. **#275 in its revised order** (issue comment): road YAPF tile reads and the other cheap rule fixes, then `GetCurrentMaxSpeed`, then trees and industry `next_tick`, then folding the road per-tick C++ into Rust; aircraft last. This is the current speed pass's fix list.
5. **#281, two maintenance PRs:** direct vehicle maintenance calls with wrapper deletion, then the callee-copy check in quick CI.
6. **#274 map view** as one shared PR after #255 integrates, with the attribution in the #274 comment.
7. **#266 road commands starts once #280 integrates**, without waiting for #269 or #255. Then #267 and #247.

## Earlier steering, still in force

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

Harness and process: #72 harness (#85), #84 play saves (#87), #97 provenance
freeze (#100), #88 Ruff (#92), #75 partial-pixel fidelity (#91), #90 world-state
design (`docs/design/world-state.md`). Maintenance PRs #209-#264 (shared
services, CI throughput, disk hygiene, caps, #249 enforcement, ScriptList overflow, dead FFI) keep their
evidence in their PRs; rows below use original retirement and net glue (#223).

| Issues | Maintenance PRs | Integration | Commit | Metrics |
| --- | --- | --- | --- | --- |
| #261 R1 (tests, one ABI layout table) | #271 | direct | `15e0ae1390` | 3 / 171 / 189 / 0; net glue -405 |
| #156 (no-city towns, stale-link restrict/release) | #272 | direct | `6ffbf8f302` | 99 / 20 / 0 / 0; net glue 0 |
| #261 R4 (direct save aliases) | #268 | direct | `5b3c666698` | 7 / 4 / 35 / -44; net glue -204 |
| #261 R3 part (train/orders forwarders and unsafe contracts) | #273 | direct | `bdab12a31b` | 1571 / 0 / 0 / 0; Rust net -532, net glue 0 |
| #276 (#275 native inline ClampTo) | #277 | direct | `08e5372c8f` | 0 / 7 / 0 / 0; Rust net -44, net glue -33 |
| #278 (#275 retained road YAPF scratch) | #279 | direct | `35d872ca3c` | 86 / 11 / 0 / 0; net glue 0 |
| #280 (component registries and append sections) | #286 | direct | `762a56477f` | 633 / 232 / 226 / 0; Rust net -418, net glue +15 |
| #282 (15% speed guard and fixed ceilings) | #285 | direct | `83bd72774e` | 0 / 49 / 0 / 0; net glue 0 |
| #291 (temporary clone cleanup) | #292 | direct | `c93cf6701e` | 0 / 15 / 0 / 0; net glue 0 |
| #283 (receipt tooling retirement) | #287 | direct | `bb2f242ddc` | 0 / 337 / 0 / 0; tooling net -1904 |
| #288 (#275 road YAPF observations) | #289 | direct | `62622e60d3` | 53 / 4 / 200 / 0; net glue +126 |
| #296 (typed Markdown classification) | #297 | direct | `629262509e` | 0 / 50 / 0 / 0; tooling net +49 |
| #301 (shared benchmark coordination) | #302 | direct | `e2e3aa074f` | 0 / 34 / 0 / 0; net glue 0 |
| #299 (borrow effect/disaster tables) | #300 | direct | `363502a424` | 70 / 7 / 6 / 0; net glue +4 |
| #307 (#156 scheduler deletion/pool growth) | #309 | direct | `ad58d65c10` | 0 / 7 / 74 / 0; tests only |
| #284 (generated ABI and typed defaults) | #298 | direct | `79dea34092` | 4541 / 287 / 2650 / 0; net glue +196 |
| #311 (#275 road speed/effect boundary) | #319 | direct | `08053fda50` | 88 / 35 / 101 / 5; net glue +90 |
| #310 (#275 immutable road service tables) | #318 | direct | `4c4c5a0eab` | 0 / 27 / 312 / 0; net glue +7 |

Paused, not fallbacks: #64 curve family, #68 SHA-512/Ed25519, #69 tile areas.

## Phase 1: harness and speed maintenance

The default semantic harness runs in CI; regressions come first. Port differences
go in `KNOWN_FAILURES` with an issue, never in masks (`docs/rust-migration.md`).

1. **Shared-accessor check (#249).** A new per-port copy fails quick CI. The baseline of existing copies only shrinks; #259 H4 extends the check to orphaned exports and duplicate table bindings.
2. **Speed guard and passes (#155, #282, AGENTS.md).**
   - **Coordination #301/#302 integrated:** timed games share the normal lock; the bounded real-process regression runs in default checks.
   - **Record:** `SPEED_BUDGETS` in `tools/simulation/roads.py` holds current `rust-migration` ratios. Each integrating PR records its medians, and each speed pass re-measures all six on a quiet host.
   - **Per PR:** no scenario more than 15% slower than its record and none over its ceiling (1.5x play saves, 1.3x generation). Misses get a callgrind fix round.
   - **Speed pass:** after every fourth ownership integration, or when a scenario is within 5% of its ceiling (1.425x play, 1.235x generation). Latest #275 attribution covers integrated `62622e60d3` and house failures. #255 and #303 are the two ownership integrations since that profile; remaining fixes retain their order.
   - **Redraw inflation:** the null driver redraws whenever 1 ms has elapsed, so a slower candidate draws more frames, which inflates play-save ratios. Say so in a PR when it decides a miss.
3. **#156 coverage, standing capacity.** Random and crash branches come first. These remain:
   - signal tbd/global propagation overflow and arbitrary mod layouts;
   - town exclusivity, unwanted months and authority refusals;
   - engine external NewGRF/retire_early and legacy save branches;
   - #218's NewGRF and game-script paths;
   - the #268, #269 and #270 audit gaps (#156 comment, 2026-10-16);
   - the #257, #270, #269 and #255 audit gaps (#156 comment, 2026-10-18); #309 covers the selected deletion and pool-growth assertions.

## Phase 2: current work, in order

Integrate reviewed work first, in order of readiness, with at most six
unintegrated component branches. Maintenance (#259, #261, #275) and #156 run in parallel.

1. **#291/#292, #283/#287, #288/#289, #296/#297, #301/#302, #307/#309 and #284/#298 are integrated.** #267/#303, #311/#319 and #310/#318 also integrated. CI_ON_DEMAND=true.
2. **Paused drafts:** retain each recorded blocker; #269/#308 get no additional fix round during wind-down. Priorities are unchanged.
3. **#275 speed, revised order (2026-10-18 comment):**
   - road YAPF tile reads (#288/#289) and effect/disaster borrowing (#299/#300) integrated; six-scenario attribution is posted on #275. Ship scratch #306/#308 remains speed-blocked; #310/#318 integrated constant initialization of the actual road service tables;
   - #311/#319 integrated the lean road `GetCurrentMaxSpeed`/visual-effect boundary;
   - #312 narrows remaining tree observations (#255 already removed the main facade/settings rebuild), then industry `next_tick`;
   - fold the road per-tick C++ (tick-front, `DoUpdateSpeed`, incline, crash check) into Rust;
   - aircraft last. The road C-layout block is dropped as a speed item.
4. **#281** vehicle maintenance direct calls, then the callee-copy check.
5. **#274 map view**, one shared PR after #255 integrates.
6. **Commands:**
   - #266/#290 road commands: shared command/finance/price/clearing fixes pass; remaining town/tram native-to-Rust build bounces need direct outer-entry cost/scratch context, actual-base adoption and fresh review before speed/Full;
   - #267/#303 station construction is integrated with direct shared command/clearing/restoration support;
   - #247 town commands, authority and deletion, after #256/#269 integrates;
   - #293 tunnel/bridge construction, removal and ownership, once #247 and #212 settle;
   - #294/#317 water construction/restoration: fixes, medians and frozen-worker packaging repair pass; material conflicts with integrated #303 require shared-service/ABI adoption and scoped review, then affected evidence/speed/Full;
   - #295/#313 object construction, generation and HQ lifecycle: shared command/scratch/price/restoration milestone is pushed. Retirement remains 79 lines short of strict net-glue-plus-tooling; root must resolve that boundary gap, then actual-base ABI/default adoption and final gates. No waiver.

   Fresh Astra planning selected #293-#295. Reuse canonical services; none waits on draft wrappers. Object generation's cancellation services need return-before-rethrow handling.
   Next whole owners: **#314** common vehicle transactions/nonrail construction/articulated assembly; **#315** rail construction/consist transactions; **#316** subsidy state and award/expiry. They start in that order after active work, without draft service dependencies; subsidy glue+tooling targets <=450 lines.
7. **#168 tail, largest first.** `order_cmd.cpp` and `economy.cpp` hold 65 of the 198 baseline copies. Train, ship, effect and cargo-packet C-layout blocks wait for nothing.

New owners reuse integrated services, never draft copies. A Rust-allocated
state block with C++ reference aliases (#216, #248) is acceptable. Keep save
staging only for a field a `SaveLoad` descriptor cannot address through the alias.

## Resume checkpoint

Paused at the user's request on 2026-10-20. Root: gpt-6.1-sol, xhigh;
implementation and fresh reviews: gpt-6.1-sol, high. No new work starts until resumed.

| Issue / PR | Head | Blocker and next step |
| --- | --- | --- |
| #256 / #269 | `64f133b059` | P2016 1.558 fails 1.5 ceiling; source review/checks pass. Retain the existing #275 speed disposition; no new fix round in wind-down. |
| #266 / #290 | `17bc99cd5a` | Town/tram build owner bounces and actual-base conflicts. Add direct outer-entry context, adopt shared services and obtain fresh fixing review, then medians/Full. |
| #306 / #308 | `9675566fb9` | P2016 1.618 fails ceiling; source review/checks pass. Retain existing speed disposition; no new fix round in wind-down. |
| #294 / #317 | `5410774ff7` | Review/medians and frozen-worker repair pass; Full lost its Emscripten runner. Adopt #303 shared services across material conflicts, obtain scoped review and rerun final gates. |
| #295 / #313 | `b64cfe4cf0` | Passing command/scratch/price/restoration milestone; tooling325, strict retirement gap79. Root boundary disposition, actual-base ABI/default adoption, medians, fresh review and Full remain. Earlier356-case default evidence is a dirty development tree, not this head. |
| #274 | `162843acbd` | Paused measurement branch; shared map view/attribution remain in the existing work order. No new PR. |
| #64 / #66 | `aab5211801` | #66 closed unmerged; #64 and older paused work remain untouched. |

#68/#69 are unchanged. #247/#293 retain their prerequisites; #281 follows #275.
Merged PR worktrees/branches are removed; stopped drafts retain one worktree and
branch, including any uncommitted work. Only recorded ad hoc output is cleaned;
shared builds, caches, toolchains, reference, dependencies and locks are retained.
Preserve the pinned reference. Do not reapply effect helper `73ccd511fb`.
Build/test with `--jobs 2`; standalone fixture games share the benchmark lock.

## Out of scope for the near term

More `src/3rdparty` ports, GUI rendering/layout and platform/toolchain expansion.
Deferred behavior improvements stay in `deferred-improvement` issues.

## Progress metric

Every port PR pastes `python3 tools/port-metrics.py`: Rust/tooling/glue added, original retired, fork glue removed and net glue.
`--whole-fork` gives totals and simulation share. Retire more original C++ than net glue plus tooling or explain why.
A port that misses this ratio gets a boundary fix (shared services, #274 views) before deferral.
Judge game logic owned; data tables inflate retirement. Avoid utility kernels.
