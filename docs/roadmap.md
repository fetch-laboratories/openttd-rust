# OpenTTD-Rust near-term roadmap

Root owns selection here; `AGENTS.md` and `docs/rust-migration.md` define process.
If an issue conflicts, follow this roadmap and report it to root. Keep this
forward-looking, about 200 lines; completed work is one row, with evidence in PRs.

## Where the fork stands (2026-10-18, `83bd72774e`)

- Twenty-nine ownership ports. Whole fork: Rust 64,134 / tooling 31,966 / glue 28,250 / original retired 40,145 of 470,612 `src/` lines (8.53%). Simulation: 32,218 / 102,615 (31.40%).
- This round integrated #286 registries and #285 speed guard: no original retirement, Rust net -418, tooling +206 and glue +15. Ownership progress is unchanged.
- Three component branches: #269 houses (891 retired) and #255 landscape (846), adopting the integrated registry base; #266/#290 road commands starts from it. Three maintenance PRs are #287, #289 and the urgent #292 protected-branch cleanup repair; #284 resumes preserved implementation without opening a fourth PR.
- First speed record (#285, median of three): TGP 1.146 (ceiling 1.3), play saves 1.21-1.37 (ceiling 1.5). The 15% guard is integrated; no result triggers an early speed pass.
- Coverage (#156): vehicle deletion/pool-growth assertions, NewGRF/legacy branches and the unintegrated house/landscape gaps remain open.
- The largest remaining simulation C++: `station_cmd.cpp` 4,424 lines, `road_cmd.cpp` 2,669, `vehicle.cpp` 2,573, `tunnelbridge_cmd.cpp` 2,135, `town_cmd.cpp` 1,929, `train_cmd.cpp` 1,778, `landscape.cpp` 1,665, `rail_cmd.cpp` 1,531, `water_cmd.cpp` 1,471.

## Fourteenth steering review (2026-10-18, merge churn and honest exceptions)

Independent audits of #270, #257, #269 and #255 and a fresh profile found **one reachable divergence** (#269) and these patterns:

- **Merge churn is the largest cost.** In the last session, 9 of 17 agent tasks on component PRs were base updates or conflict reviews. Every conflict was an append collision: witness IDs from one global sequence, include lines, probe calls, service-table tails.
- **#269 R1 regressed fidelity.** The construction-stage trigger reads the untranslated house ID, so a placed house that a NewGRF overrides skips a `Random()` draw. All four earlier #269 findings are fixed.
- **#255's speed exception was overstated.** The exception is gone now; these design faults still block:
  - Most of the cited 275M `RustClearSurface` is reads the original also makes inline.
  - The final head added two rail trampolines after the timing.
  - Four Rust-to-C++-to-Rust bounces into Rust-owned code remain.
  - Glue is 1,048 against an estimate of 350-500.
- **Copies still land.**
  - #257 added about ten copies of existing wrappers, and train, ship and aircraft still reach the retired maintenance bodies through C++ (3 crossings instead of 1, unseen by the Opus gate).
  - #270 copied the tree area iterator.
  - The reviewer grep rule did not catch these.
- **The speed gap is about 85% boundary cost.** Apart from aircraft and orders, Rust bodies cost about what the retired C++ did.
  - Road YAPF reads the whole tile record up to three times per tile (809M Ir on Opus).
  - Effects and disasters copy the shared table per entry, and ship YAPF allocates per search.
  - The planned road C-layout block would save about 1M Ir.
- **#274's map view clears its gate,** but at 1,549M against the native loop's 1,853M it beats inline C++, so its saving is not yet attributed.

**Speed policy (user decision, 2026-10-18).** The 3% ratchet blocked on noise. Padhattan 2000/2016 noise is about 2%, and today's caps fail an unchanged build most of the time. Speed gating took about 20-30% of subagent time, and the real finds came from periodic profiles. AGENTS.md now has a 15% per-PR guard, ceilings of 1.5x (play saves) and 1.3x (generation), no per-port CPU budgets, and a speed pass after every fourth ownership integration. #282 changes the harness.

**Process changes (user decision, 2026-10-18).** 87% of ready-to-merge time passed with no root session running, 22 of 43 review rounds found nothing or only evidence items, and root's re-verification of fix commits rejected nothing. AGENTS.md now makes exact-head CI plus the PR's commands the evidence, with no receipts; #283 retires that tooling and stops Markdown commits from forcing post-merge runs. Reviewers rerun only what their fixes touch. Drafts update from base only to integrate or to clear a blocking conflict. Root spot-checks fix commits. Commits that only fix warnings or platform errors, or only record medians, need no review. Platform CI starts when implementation completes. Each task keeps one status comment. Tooling budgets allow about 25% over. Hand-written ABI lists were involved in 46 of 97 conflicted merges, and #284 generates them and gives fixtures aborting default slots.

Actions, in order:
1. **#280, #282, then #284** (maintenance PRs): per-component witness blocks and an ordering convention for shared append lists; the speed guard fields and the first measured record; generated ABI layout lists and default fixture slots. AGENTS.md now lets a base update that keeps both sides' lines skip review.
2. **#269:** fix the override divergence and the `RustHouseTown` copy, add a probe that places an overridden house, adopt #280, then review, Full and integrate (PR comment).
3. **#255:** the six must-change items in its PR comment (canonical rail leaves, no bounces, orphan exports, Windows warnings, the speed guard at the final head instead of the old exception, glue explained or cut), then review, Full and integration.
4. **#275 in its revised order** (issue comment): road YAPF tile reads and the other cheap rule fixes, then `GetCurrentMaxSpeed`, then trees and industry `next_tick`, then folding the road per-tick C++ into Rust; aircraft last. This is the current speed pass's fix list.
5. **#281, two maintenance PRs:** direct vehicle maintenance calls with wrapper deletion, then the callee-copy check in quick CI.
6. **#274 map view** as one shared PR after #255 integrates, with the attribution in the #274 comment.
7. **#266 road commands starts once #280 integrates**, without waiting for #269 or #255. Then #267 and #247.

## Earlier steering, still in force

- **Core loops (thirteenth):** #257 integrated; #255 integrates under the 15% guard once its design faults are fixed. The vehicle raw-offset view is closed (#274). Command ports share one set of command checks. Duplicate wrappers found at review are deleted before integration.
- **Housekeeping (twelfth):** #259 H1-H4, #260 and #261 R1/R3-part/R4 are done. R2 follows #269, and H5 comes last. At most three maintenance PRs are in flight. They never count against the component cap or block an ownership port.
- **Direct shared services and boundaries:** direct shared services (#107/#168/#199) and one definition per shared accessor, enforced in quick CI by #249, whose baseline only shrinks. Use a fixed hasher for lookup-only YAPF maps; ABI blocks stay per component until #284 removes numeric IDs.
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

Paused, not fallbacks: #64 curve family, #68 SHA-512/Ed25519, #69 tile areas.

## Phase 1: harness and speed maintenance

The default semantic harness runs in CI; regressions come first. Port differences
go in `KNOWN_FAILURES` with an issue, never in masks (`docs/rust-migration.md`).

1. **Shared-accessor check (#249).** A new per-port copy fails quick CI. The baseline of existing copies only shrinks; #259 H4 extends the check to orphaned exports and duplicate table bindings.
2. **Speed guard and passes (#155, #282, AGENTS.md).**
   - **Record:** `SPEED_BUDGETS` in `tools/simulation/roads.py` holds current `rust-migration` ratios. Each integrating PR records its medians, and each speed pass re-measures all six on a quiet host.
   - **Per PR:** no scenario more than 15% slower than its record and none over its ceiling (1.5x play saves, 1.3x generation). Misses get a callgrind fix round.
   - **Speed pass:** after every fourth ownership integration, or when a scenario is within 5% of its ceiling (1.425x play, 1.235x generation). The 2026-10-18 profile at `600dca4e27` is the current pass (#275). Count the next four from there; #269 and #255 count.
   - **Redraw inflation:** the null driver redraws whenever 1 ms has elapsed, so a slower candidate draws more frames, which inflates play-save ratios. Say so in a PR when it decides a miss.
3. **#156 coverage, standing capacity.** Random and crash branches come first. These remain:
   - signal tbd/global propagation overflow and arbitrary mod layouts;
   - town exclusivity, unwanted months and authority refusals;
   - engine external NewGRF/retire_early and legacy save branches;
   - #218's NewGRF and game-script paths;
   - the #268, #269 and #270 audit gaps (#156 comment, 2026-10-16);
   - the #257, #270, #269 and #255 audit gaps (#156 comment, 2026-10-18), first the #257 deletion and pool-growth assertions.

## Phase 2: current work, in order

Integrate reviewed work first, in order of readiness, with at most six
unintegrated component branches. Maintenance (#259, #261, #275) and #156 run in parallel.

1. Repair protected Quick with reviewed **#291/#292** before further integration. **#283/#287** passed exact-head bootstrap Full; CI_ON_DEMAND is restored to true. Observe its real docs-only postmerge skip after integration. **#284 generated ABI layouts** resumes preserved work after a provider-capacity failure. #280 and #282 are integrated above.
2. **Drafts:** #269 houses, then #255 landscape, each with the must-change items from its 2026-10-18 PR comment, adopting #280 on its base update.
3. **#275 speed, revised order (2026-10-18 comment):**
   - road YAPF single tile read and speed-limit/height folding (#288/#289, fresh review and Full); the effect/disaster table copy; ship YAPF scratch; `constinit` service tables;
   - road `GetCurrentMaxSpeed`;
   - trees facade and industry `next_tick` (TGP has 1% headroom);
   - fold the road per-tick C++ (tick-front, `DoUpdateSpeed`, incline, crash check) into Rust;
   - aircraft last. The road C-layout block is dropped as a speed item.
4. **#281** vehicle maintenance direct calls, then the callee-copy check.
5. **#274 map view**, one shared PR after #255 integrates.
6. **Commands:**
   - #266 road commands is active from the integrated #280 base, reusing #270's command checks and crossing services;
   - #267 station construction on the #125 owner;
   - #247 town commands, authority and deletion.

   Tunnel/bridge follows once #247 and #212 settle.
7. **#168 tail, largest first.** `order_cmd.cpp` and `economy.cpp` hold 65 of the 198 baseline copies. Train, ship, effect and cargo-packet C-layout blocks wait for nothing.

New owners reuse integrated services, never draft copies. A Rust-allocated
state block with C++ reference aliases (#216, #248) is acceptable. Keep save
staging only for a field a `SaveLoad` descriptor cannot address through the alias.

## Resume checkpoint

Root is gpt-6.1-sol, xhigh; implementation and review use gpt-6.1-sol, high.
Three component branches are unintegrated. Protected Quick failed temporary-clone
cleanup after its assertions passed; #292 repairs it before unrelated merges.
CI_ON_DEMAND=true; #287 bootstrap Full passed. Keep at most three maintenance PRs.

| Issue / PR | Branch, head | State and next step |
| --- | --- | --- |
| #291 / #292 | `fix-registry-maintenance-291`, `7d0a69899d` | Fresh review has no findings; exact-head Full requested. Integrate, then observe protected Quick recovery before other merges. |
| #284 | `generated-abi-284` | Same unfinished implementation resumed after provider-capacity failure; dirty work preserved. 650 handwritten support/test lines plus mechanical conversions; push milestone before opening a draft when a maintenance PR slot frees. |
| #283 / #287 | `t3/retire-evidence-receipts`, `ecbd157836` | Four review findings fixed and root-checked; 337/350 tooling. Conflict-free base update retains speed tests. Full passed; held for #292 and protected Quick recovery, then actual docs-only postmerge skip. |
| #256 / #269 | `house-lifecycle-256`, `bc691107d2` | Registry adoption complete; fresh review of steering fixes/probes and conflict resolution assigned, Full requested. Platform requested; 45 scenarios/439 snapshots and final six medians pass locally, P2016 1.353. |
| #212 / #255 | `landscape-owner-212`, `cc6e1172e5` | Registry/base adoption active; six steering fixes and prior 141 scenarios/980 snapshots pass. Platform requested; fresh review/final-head speed/Full follow. |
| #266 / #290 | `road-commands-266`, `6b89dcd222` | Ownership implementation active from integrated `83bd72774e`; 200-300 tooling lines. |
| #288 / #289 (#275) | `road-yapf-reads-288`, `45a4a1adfe` | Fresh fixing review and exact-head Full requested. Cargo/native/20 road scenarios and all six speed guards pass; 216/400 added implementation/test/ABI lines, no new Python tooling. |
| #281 | none | Follows the revised #275 fix order when a maintenance slot frees. |
| #274 | `measurement-274-map-dd68ca46`, `162843acbd` | Shared map-view PR after #255; retain attribution and #282's shared profile-lock/concurrency acceptance. |
| #64 / #66 | remote `issue/64-curve-family`, `aab5211801` | #66 closed unmerged; #64 and older paused work stay untouched. |

Ten finished worktrees were removed after checking clean, pushed heads. Both open
drafts now use one fresh task checkout each; their old checkouts were removed.
Temporary review branches that `git branch -d` refused remain until integration.

ABI blocks (check only consumed layouts; #284 deletes this registry):
- train through 385, tree additions 386-389, fleet 390-399
- company 400-429, town 430-459, road 460-499
- station/loading 500-559, industry 560-619 (572 is the shared ByteSpan), CommandCost 620-629
- town lifecycle 630-669, aircraft 670-739
- vehicle 740-779, landscape 780-819, signals 820-859, engines 860-919
- house lifecycle 920-979, rail commands 980-1019, road commands 1020-1059, station construction 1060-1119

Preserve the pinned reference. Do not reapply effect helper `73ccd511fb`.
Build and test with `--jobs 2`; standalone fixture games share the benchmark lock.

## Out of scope for the near term

More `src/3rdparty` ports, GUI rendering/layout and platform/toolchain expansion.
Deferred behavior improvements stay in `deferred-improvement` issues.

## Progress metric

Every port PR pastes `python3 tools/port-metrics.py`: Rust/tooling/glue added, original retired, fork glue removed and net glue.
`--whole-fork` gives totals and simulation share. Retire more original C++ than net glue plus tooling or explain why.
A port that misses this ratio gets a boundary fix (shared services, #274 views) before deferral.
Judge game logic owned; data tables inflate retirement. Avoid utility kernels.
