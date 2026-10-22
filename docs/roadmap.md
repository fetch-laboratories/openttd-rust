# OpenTTD-Rust near-term roadmap

Root owns selection here; `AGENTS.md` and `docs/rust-migration.md` define process.
If an issue conflicts, follow this roadmap and report it to root. Keep this
forward-looking, about 200 lines; completed work is one row, with evidence in PRs.

## Where the fork stands (2026-10-22, code at `802ec7d6a4`)

- Thirty-eight ownership ports. Whole fork: Rust 88,388 / tooling 33,537 / glue 34,397 / original retired 49,636 of 470,612 C/C++ `src/` lines (10.55%). Simulation: 41,518 / 102,615 (40.46%).
- Wind-down integrated #329 vehicle transactions, #346 subsidies and #352 tunnel/bridge ownership, plus #345 train speed reads, #342 tram coverage. Each passed final-base counts and exact-head Full, including Windows linking.
- Speed: #322 wall anchors stand (TGP 1.215, Opus 1.204, Grok 1.191, P1996 1.250, P2000 1.305, P2016 1.356). #352's final six pass: TGP projects 1.222, the other five stay at their anchor floors. Four ownership integrations since the pass (#313/#329/#346/#352) make the next pass due; the user deferred it to the first resume step.
- CI: #352's protected push is green across native/platform/Quick; macOS remains on GitHub-hosted runners. #345's measured train speed-path saving is 19.27M net Ir; #341 closed without a saving. #351 passes final-base counts but stops as a draft on Full Rust formatting.
- Three component branches remain against #157's limit of six: #347, #353 and uncommitted #349. Two maintenance drafts (#340/#351) also remain. #342 closes the tram-turn gap; other #156 gaps remain. New ports use #334's shared header modules.

## Sixteenth steering review (2026-10-21, counts per PR and closing the speed pass)

Independent Claude audits of #290, #269, #317, #333 and draft #313, plus a process audit of root's 2026-10-21 session (258 minutes, 40 child agents), found:

- Six quiet holds cost 71 minutes (27%); all ten measured heads passed. P2016 wall ratios varied 1.350-1.439 with loop counts moving under 0.5%: code layout explains that spread.
- #322 grew from 150 to 560 tooling lines over five re-selections without finding a game defect. Its house calibration's +4.27% wall versus +0.29% Ir is the same layout spread; quiet heads including houses measured 1.350-1.397.
- The due speed pass was parked on #322 although profiles, attribution and followups #326/#332 were done; only its record remained.
- Gross callback counts overstated savings: aircraft recovered 60M net Ir, road fold 26M, versus 0.45G/156M claimed. Rank net costs.
- No reachable divergence in five audited ports. Remaining defects were copied header helpers/constants, the house Rust/C++/Rust bounce, five orphan owner facades and unconditional road command-context construction. `shared-accessors.py` cannot see Rust helper copies.
- #281/#274/#247/#156 had slipped; ratio fixes waiting on parked map views and #293 waiting on unscheduled #247 were circular.

**User decision (2026-10-21):** each PR is checked on projected instruction counts (#321 as built). Wall time is measured only in speed passes. This settles the policy question #322 returned to the user.

AGENTS.md now says:
- A PR does not answer for the layout spread. Count projections run alongside other agents with no quiet window.
- An integration that moves a projection into the 5% band triggers a pass; a pass's own wall measurement does not. Passes rank net costs.
- Root raises an infrastructure budget once; a second raise is a `question` issue. Infrastructure reviews fix only defects that can change a game comparison or a speed decision.
- Header helpers and constants are ported once into a shared Rust module named after the header.
- A Claude Code root keeps independent review on gpt-6.1-sol through T3 `delegate_task`.

Provisional, checked next review against #322's first anchors: the factor 2 (basis: the fifteenth review's 1.8x P2016 attribution; the calibration's residuals were layout) and the 6.6% layout spread (eight quiet P2016 sixes on 2026-10-21).

Remaining steering actions (completed #322/#313/#334 are table rows below):
1. **#329:** integrated; completion and metrics are below.
2. **#156 standing slot:** whenever fewer than five agents are busy, work the recorded gaps; ownership dispatch and tram turns are covered; other recorded gaps remain.
3. **#275 by net P2016 savings:** road context and train speed reads integrated; station draft closed.
4. Then Phase 2 from #281 onward.

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
| #295 | Object lifecycle, commands and NewGRF policy | #313 | `883ce69671` | 2474 / 429 / 585 / 928 |
| #314 | Vehicle transactions and non-rail construction | #329 | `43257e3744` | 3231 / 206 / 1200 / 1312 |
| #316 | Subsidy state and route lifecycle | #346 | `7e83fe925e` | 915 / 177 / 277 / 430 |
| #293 | Tunnel/bridge commands and ownership | #352 | `802ec7d6a4` | 1862 / 178 / 325 / 854 |

Harness/process #72/#84/#97/#88/#75/#90 and maintenance #209-#302 retain evidence in their PRs.
Rows below use original retirement and net glue (#223).

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
| #321 instruction-count speed guard and first anchors | #322 | direct | `f6cddca557` | 0 / 576 / 0 / 0; net glue 0 |
| #156 ownership-dispatch coverage | #335 | direct | `caadbabc38` | 0 / 173 / 0 / 0; tests only |
| #261 R6 shared header modules | #334 | direct | `18dbc15675` | 858 / 7 / 64 / 10; net glue +10 |
| #338 instruction identity and rolling record | #339 | direct | `c4efe7bc72` | 0 / 30 / 0 / 0; net glue 0 |
| #156 road/water command coverage | #337 | direct | `9da106b61f` | 0 / 128 / 0 / 0; tests only |
| #275 lazy tram command context | #336 | direct | `43f8d27304` | 17 / 2 / 23 / 0; net glue +11 |
| #344 train speed observations | #345 | direct | `fc72e5cc1a` | 4 / 1 / 71 / 0; net glue +59 |
| #156 tram-turn coverage | #342 | direct | `6017f98f3d` | 7 / 71 / 0 / 0; tests only |

Paused, not fallbacks: #64 curve family, #68 SHA-512/Ed25519, #69 tile areas.

## Phase 1: harness and speed maintenance

The default semantic harness runs in CI; regressions come first. Port differences
go in `KNOWN_FAILURES` with an issue, never in masks (`docs/rust-migration.md`).

1. **Shared-accessor check (#249).** A new per-port copy fails quick CI. The baseline of existing copies only shrinks; #259 H4 extends the check to orphaned exports and duplicate table bindings. Rust-internal helper copies are invisible to it; reviewers check them against the shared header modules (#261 R6).
2. **Speed guard and passes (#155, #282, #321, AGENTS.md).**
   - **Per PR:** `simulate --instructions` loop counts projected from `SPEED_ANCHORS`; no scenario more than 15% above the integrated projection and none over its ceiling (1.5x play saves, 1.3x generation). Misses get a callgrind fix round.
   - **Speed pass:** after every fourth ownership integration, or when an integration moves a projection within 5% of its ceiling (1.425x play, 1.235x generation). Its quiet six with loop counts replaces the anchors. #322 closed the pass due since #269; four owners have since integrated. The next pass is due and deferred to resume by the user.
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

1. **#329/#314** integrated after #313 (sixteenth review, actions 2-3).
2. **#275 by net P2016 savings:** #336 integrated; station #341 closed without a saving; integrated train #345 replaces only the speed read6 bundle with existing services at original branches. Its measured speed-path saving passes; no raw train view or movement-reader change. TGP #312 trees remain ahead of clear observations in the net attribution; TGP is not binding.
3. **#281** vehicle maintenance direct calls: #340 paused after conflict review found two boundary defects; then the callee-copy check.
4. **#274 map view**, #351 paused on Full formatting, one shared maintenance PR: extend the canonical descriptor and adopt assertion-faithful shared helpers in the scheduler's clear/water/tree loops. C++ retains arrays and writes. Same-base TGP attribution and the six-count guard decide acceptance; historical results alone do not. Budget: 800 handwritten support/glue/test/tooling lines, including at most 150 validation/tooling; no new runner or vehicle view.
5. **Next whole owners**, as WIP frees, each without draft service dependencies:
   - #315 rail construction and consist transactions, after #314;
   - #316 subsidies integrated in #346 (support418 <430 retired, cap450);
   - #247 town commands, authority and deletion (unblocked: #269 is integrated);
   - #293 tunnel/bridge construction, removal and ownership integrated in #352;
   - #348 landscape clearing, terraforming and leveling: #353 milestone paused (support target400, tooling cap200);
   - #349 vehicle station/depot visits with LinkRefresher and IncreaseStats (support target560, tooling cap275).

   A shared service two need lands first, as its own PR. The next-owner plan is on #3 (2026-10-21): #348/#349 use integrated services independently of #274. #350 generation/rivers remains proposed; root must accept its exact native sea-set iteration/lifetime boundary before selecting it. Plan again when fewer than two selected owners remain unstarted. The finished #350/#354 plan on #3 is input only; neither proposal is selected. No new planning during the pause.
6. **#168 tail, largest first.** `order_cmd.cpp` and `economy.cpp` hold 65 of the 198 baseline copies.

New owners reuse integrated services, never draft copies. A Rust-allocated
state block with C++ reference aliases (#216, #248) is acceptable. Keep save
staging only for a field a `SaveLoad` descriptor cannot address through the alias.

## Resume checkpoint

Paused at the user's request. **First resume step: the due speed pass**, before further integrations or component starts. The work order above is unchanged; #350/#354 remain proposal inputs only. Root is gpt-6.1-sol (xhigh).

| Issue / PR | Head | Next step |
| --- | --- | --- |
| #281 / #340 | `0c8a5e15f3` | Draft: fix the orphan aircraft reliability wrapper and duplicate engine accessor found by conflict review; fresh review, six counts, record and Full. |
| #247 / #347 | `134a40b026` | Untouched draft: adopt integrated subsidies, then fresh full source review and remaining integration gates. |
| #274 / #351 | `edc8b95bdc` | Draft: Full fails merged lib.rs module ordering. Native build/default semantics/final-base six counts pass. Format, refresh evidence as needed, then exact-head Full; additions789/800. |
| #348 / #353 | `6d10b6c900` | Draft milestone: native verify/Cargo4/preflight and Quick pass. Fresh independent review next session, then remaining semantic/count/Full gates. |
| #349 (no PR) | `bc30c0fc25` + dirty WIP | Stopped before a passing milestone. Finish visit bindings/callers; Cargo4/preflight, first passing commit/push and draft PR. State is on #349. |

All retained drafts/WIP keep their worktrees and branches: three component branches against #157's limit of six; five unintegrated branches total.
Merged worktrees/local branches, the verified stale measurement branch,
retired preflight output and main-checkout verification runs before today were removed.
#261 R7 and #281 part 2 inputs remain on their issues. #64/#68/#69 stay paused.
Preserve the pinned reference; do not reapply effect helper `73ccd511fb`.
Build and test with `--jobs 2`.

## Out of scope for the near term

More `src/3rdparty` ports, GUI rendering/layout and platform/toolchain expansion.
Deferred behavior improvements stay in `deferred-improvement` issues.

## Progress metric

Every port PR records `python3 tools/port-metrics.py`; `--whole-fork` gives totals and simulation share. Retire more original C++ than net glue plus tooling or explain why. A port that misses this ratio gets a boundary fix before deferral. Judge game logic owned; data tables inflate retirement. Avoid utility kernels.
