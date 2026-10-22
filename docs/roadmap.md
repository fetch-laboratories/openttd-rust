# OpenTTD-Rust near-term roadmap

Root owns selection here; `AGENTS.md` and `docs/rust-migration.md` define process.
If an issue conflicts, follow this roadmap and report it to root. Keep this
forward-looking, about 200 lines; completed work is one row, with evidence in PRs.

## Where the fork stands (2026-10-22, code at `802ec7d6a4`)

- Thirty-eight ownership ports. Whole fork: Rust 88,388 / tooling 33,537 / glue 34,397 / original retired 49,636 of 470,612 C/C++ `src/` lines (10.55%). Simulation: 41,518 / 102,615 (40.46%), up from 37.09% at the sixteenth review.
- Since then, twelve integrations, four of them ownership ports (#313 objects, #329 vehicle transactions, #346 subsidies, #352 tunnels/bridges). Retired 3,534 lines against 2,151 net glue plus 1,960 tooling.
- Speed: a quiet six at this code (#356) matches every projection within -0.7% to +1.4% (TGP 1.214, Opus 1.199, Grok 1.202, P1996 1.254, P2000 1.323, P2016 1.359). No scenario is in its 5% band. TGP is nearest its ceiling in relative terms: before #351 it is 0.8% loop Ir from its band.
- CI on `rust-migration` is green. On 2026-10-21, hosted macOS arm64 capacity cancelled 19 Full attempts; macOS moves to every fifth integration (#357).
- Component branches: #347, #353 and uncommitted #349 (three of six). Maintenance drafts: #340, #351.

## Seventeenth steering review (2026-10-22, speed on the merged branch, base-type returns, macOS every fifth merge)

Independent Claude Opus audits of integrated #329, #346 and #352 and draft #347, a speed audit (fresh profiles plus a quiet six) and a process audit of both root sessions since the sixteenth review (Claude: 193 active minutes, 3 integrations; Codex: 261 minutes, 9 integrations) found:

- No game-state divergence on Linux or macOS in any audited port. One Windows defect: the subsidy `local_company` service returns `CompanyID` by value. MSVC returns it through a hidden pointer, while Rust reads a byte in AL (#355). Windows CI runs no scenario, and the ABI checks cover layouts, not return types.
- Per-PR speed checks caught nothing. No ordinary PR moved a projection more than 1.3%, against a 15% guard. Recording the result on one shared line forced every queued PR to remerge, recount and rerun Full after each integration: about 20 serial minutes per PR, and 16 of 24 base merges superseded.
- Four of the twelve integrations were speed work (#322, #339, #336, #345), saving about 0.4% of P2016. Quiet medians at this code match every projection within 1.4%, and all scenarios sit 6-10% below their ceilings. `simulate --instructions` fails identity in the main checkout (#356).
- Hosted macOS capacity cost the Claude root about 105 minutes and paused four PRs; macOS caught no defect.
- Of 24 review rounds, 12 had findings: duplicate copies and evidence holes, never a divergence. Reviews missed orphaned glue in all three audited integrations (#261 comment). #329 added three process-wide unsafe scratch pools where the original uses a local vector.
- The wind-down skill restated process steps and stopped PRs on fixes that need no review. Root then made the due speed pass a gate on all resume work. The skill is fixed.

**User decisions (2026-10-22):**
- PRs run no speed check. Root profiles the merged branch after every second integration and fixes forward any rise of more than 3%. This replaces the 2026-10-21 per-PR count decision and the 15% per-PR guard.
- Speed passes and speed fixes happen only when a scenario comes within 5% of its 1.5x/1.3x ceiling. #275 has no standing stream.
- macOS is tested after every fifth integration, not per PR, with a Blacksmith fallback after two hosted-capacity cancellations (#357).

AGENTS.md also says: a PR whose exact-head Full passed and that merges cleanly integrates without a base update; a C++ class type crosses as its base type (#355); a local container the original builds stays a local `Vec`; review reports name orphaned WITH_RUST bodies, exports and declarations.

Provisional, checked next review: the 3% tip-rise threshold (basis: counts repeat to 0.04%; the largest ordinary per-PR move was 1.3%) and factor 2 (measured wall sensitivity 0.75-1.54 at `802ec7d6a4`, so it is conservative). The 6.6% layout spread is retired: a pass's median of four three-pair medians came within 1.4% of every projection.

Actions, in order (apply first; ports keep running alongside):
1. **#355:** the MSVC return fix and the compile-time return-type check, before any other integration.
2. **#356:** the identity fix and the quiet anchors; the tip profiles compare against them. No pass is due: nothing is near a ceiling.
3. **#357:** macOS out of the pre-merge Full. Until it lands, use the Blacksmith fallback on capacity cancellations.
4. **Integrate ready drafts,** without base updates unless a conflict blocks: #351 after #356 (rustfmt, then Full; no extension), #340 (two deletions, then review), #347 (PR comment items), #353 (fresh review).
5. **Ports continue:** #349 to its first passing milestone, #315, then owner selection (Phase 2 item 5).
6. **#261:** the 2026-10-22 cleanup list as one maintenance PR. The **#156** standing slot takes the new gaps.

## Earlier steering, still in force

- **Shared helper layer (sixteenth):** wall time only in passes, ranked by net cost (per-PR counts are superseded, 2026-10-22). Infrastructure budgets are raised once. Header helpers are ported once (#334). A Claude Code root keeps review on gpt-6.1-sol.
- **Speed and the shared layer (fifteenth):** port callees as callees. Reviews compare callee by callee first, split by function group above about 1,000 retired lines. A shared service more than one draft needs lands once, as its own PR (#328, #333).
- **Merge churn and process (fourteenth):** #282 1.5x/1.3x ceilings (user decision). Evidence is exact-head CI plus the PR's commands. Reviewers rerun only what their fixes touch. Root spot-checks fix commits. Commits that only fix warnings or record medians need no review. Each task keeps one status comment.
- **Core loops (thirteenth):** the vehicle raw-offset view is closed (#274). Command ports share one set of command checks. Duplicate wrappers found at review are deleted before integration.
- **Housekeeping (twelfth):** #261 R2 is unblocked; H5 comes last. At most three maintenance PRs are in flight. They never count against the component cap or block an ownership port.
- **Direct shared services and boundaries:** direct shared services (#107/#168/#199) and one definition per shared accessor, enforced in quick CI by #249, whose baseline only shrinks. #284 generates named ABI layouts and fixture defaults.
- **Ownership:** prioritize simulation ownership. Keep state at the original's process scope.
- **Map storage:** #108 retains C++ map arrays; #351's descriptor is limited to the audited scheduler loops (AGENTS.md).
- **Harness evidence:** end moments (#154), default branch witnesses and reference probes (#198/#204/#210/#211), the speed guard (#155/#282/#321) and documented coverage gaps (#156).
- **Tooling, WIP and CI:** proportionate tooling (#109) with a stated budget for infrastructure work; platform defaults over controllers (#233); six component branches at most; exact-head Full on demand (#171), validated concurrently, with no admin bypass.
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
   - **Merged branch:** after every second integration, `simulate --instructions` on the tip, projected from `SPEED_ANCHORS` and recorded in "Where the fork stands". A rise of more than 3% since the previous tip profile gets a fix-forward maintenance PR. PRs run no speed check.
   - **Speed pass:** only when a projection comes within 5% of its ceiling (1.425x play, 1.235x generation). It re-anchors from a quiet six first and opens fix PRs only if a measured ratio is in its band. Passes never block ports.
   - **Redraw:** `-r 64x64` leaves the ratios unchanged. It makes whole-process counts timing-dependent, which is why only the game loop is counted.
3. **#156 coverage, one standing slot.** Random and crash branches come first. These remain:
   - signal tbd/global propagation overflow and arbitrary mod layouts;
   - town exclusivity, unwanted months and authority refusals (until #347 integrates);
   - engine external NewGRF/retire_early and legacy save branches;
   - #218's NewGRF and game-script paths;
   - the #257, #268-#270 and #255 audit gaps (#156 comments, 2026-10-16 and 2026-10-18);
   - command scenarios with live vehicles on and toward affected tiles and at map edges, plus the #303, #255, #290 and #317 gaps (#156 comment, 2026-10-20);
   - ownership dispatch, water, house, object and road gaps (#156 comment, 2026-10-21);
   - vehicle transaction, subsidy, tunnel/bridge and town command gaps (#156 comment, 2026-10-22).

## Phase 2: current work, in order

Integrate reviewed work first, in order of readiness, with at most six
unintegrated component branches. Maintenance (#261, #281) and #156 run in parallel.

1. **#355, #356, #357** (seventeenth review, actions 1-3).
2. **#275** has no standing stream. When a pass or a tip rise opens speed work, rank it by net cost (#275 comment, 2026-10-22: aircraft, road-controller tick reads, scheduler carrier, orders).
3. **#281** vehicle maintenance direct calls: #340 deletes the orphan aircraft reliability wrapper and the duplicate engine accessor, then gets a fresh review. The callee-copy check follows.
4. **#274 map view (#351):** limited to the scheduler's clear/water/tree loops; C++ keeps the arrays and writes. Remaining: rustfmt and exact-head Full after #356. No extension for TGP speed.
5. **Next whole owners**, as WIP frees, each without draft service dependencies:
   - #247 town commands (#347): the PR comment items, then review;
   - #348 landscape clearing, terraforming and leveling (#353): fresh review;
   - #349 vehicle station/depot visits with LinkRefresher and IncreaseStats: first passing milestone;
   - #315 rail construction and consist transactions (unblocked: #329 is integrated).

   A shared service that two owners need lands first, as its own PR. #315 is the only selected owner not yet started, so selection is due. Root decides on the amended #350 (generation and rivers; plan on #3, 2026-10-22) and #354 (road-stop occupancy) when a slot frees. Generation owners land after #351 so its TGP saving gives them headroom.
6. **#168 tail, largest first.** `order_cmd.cpp` and `economy.cpp` hold 65 of the 198 baseline copies.

New owners reuse integrated services, never draft copies. A Rust-allocated
state block with C++ reference aliases (#216, #248) is acceptable. Keep save
staging only for a field a `SaveLoad` descriptor cannot address through the alias.

## Resume checkpoint

Paused at the user's request; resume with the seventeenth review's actions. Root is gpt-6.1-sol (xhigh).

| Issue / PR | Head | Next step |
| --- | --- | --- |
| #355 | none | Small fix PR from `rust-migration`, first. |
| #356 | none | Identity fix plus the quiet anchors from the issue. |
| #357 | none | macOS out of the pre-merge Full; every fifth integration; Blacksmith fallback. |
| #281 / #340 | `0c8a5e15f3` | Two deletions from the conflict review, fresh review, counts, Full. |
| #247 / #347 | `134a40b026` | PR comment items (subsidy direct call, shared object iterator, `InvalidateWindowData`, base merge), then review, counts, Full. |
| #274 / #351 | `edc8b95bdc` | rustfmt the merged `lib.rs`, then exact-head Full after #356. |
| #348 / #353 | `6d10b6c900` | Fresh independent review, then semantic, count and Full gates. |
| #349 (no PR) | `bc30c0fc25` + dirty WIP | Finish visit bindings/callers to a first passing commit and draft PR. State is on #349. |

All retained drafts and WIP keep their worktrees and branches: three component branches of six.
#261 R7 and #281 part 2 inputs remain on their issues. #64/#68/#69 stay paused.
Preserve the pinned reference; do not reapply effect helper `73ccd511fb`.
Build and test with `--jobs 2`.

## Out of scope for the near term

More `src/3rdparty` ports, GUI rendering/layout and platform/toolchain expansion.
Deferred behavior improvements stay in `deferred-improvement` issues.

## Progress metric

Every port PR records `python3 tools/port-metrics.py`; `--whole-fork` gives totals and simulation share. Retire more original C++ than net glue plus tooling or explain why. A port that misses this ratio gets a boundary fix before deferral. Judge game logic owned; data tables inflate retirement. Avoid utility kernels.
