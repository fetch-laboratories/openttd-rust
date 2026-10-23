# OpenTTD-Rust near-term roadmap

Root owns selection here; `AGENTS.md` and `docs/rust-migration.md` define process.
If an issue conflicts, follow this roadmap and report it to root. Keep this
forward-looking, about 200 lines; completed work is one row, with evidence in PRs.

## Where the fork stands (2026-10-23, code at `ac7d2b60f2`)

- Fifty-two ownership ports. Whole fork: Rust 103,834 / tooling 36,694 / glue 37,169 / original retired 57,462 of 470,612 C/C++ `src/` lines (12.21%). Simulation: 49,264 / 102,615 (48.01%). The warning-only GUI cast in #382 is not ownership progress.
- Latest integration: #391 combines reviewed scheduler #380, live graph #383, occupancy #384 and monitors #388 on #390's routing base. Root spot-checked all source/CI fixes; corrected-head Full, strict protection and validated-tree equality passed before match-head merge. The first Full was cancelled after registry/portable-guard defects and a dependency timeout. No stale-base rerun, bootstrap or CI setting change.
- Speed: #390's post-fix instruction profile at `824ebd56c5` passed all six semantic/identity comparisons; play loop work fell 0.93–2.52%, TGP rose 0.144%. Matching quiet ratios are TGP 1.108913x, Opus 1.397978x, Grok 1.398183x, P1996 1.305639x, P2000 1.377750x, P2016 1.453124x. All 72 pairs/144 exits passed. Only P2016 remains in band; no ceiling exceeded. Opus/Grok/P1996 ranges separate from baseline; other small changes overlap spread.
- Genuine post-fix anchors/counts/identities integrated in #391. The due profile at frozen `ac7d2b60f2` passed all six semantic/identity checks: TGP 1.112461300x, Opus 1.473042194x, Grok 1.464168425x, P1996 1.368048397x, P2000 1.442737780x, P2016 1.516109672x. Opus/P2000/P2016 rose more than 3%. These projections stand and do not measure wall time. The quiet six was contaminated and incomplete: 66 zero exits, 33 raw pairs, 32 canonical comparisons; no new ratios or anchors and no retry. The external process was the user's wind-down session, not peers. On resume rerun the whole quiet six and attribute retained ac7-versus-824 profiles; interrupted preliminary attribution is not an accepted ranking. The >3% rise remains the next maintenance item, not a gate on other work. Ports have no individual speed gate.
- CI: due macOS arm64 Debug/Release passed at `824ebd56c5`; one integration since that run. CI maintenance observation is 4/10, recorded in integration notes. `CI_ON_DEMAND=true` is retained.
- Animation #389 passed independent fixing review at `cdc34942dd`; root spot-check and specific cost explanation are accepted. Reconciliation is saved at `2290e57998`; scoped review, affected checks, base update and Full remain. #392 and #393 have clean pushed unfinished checkpoints. All agents are stopped; no rollback is needed.

## Integration and wind-down correction (2026-10-22, takes precedence)

After #347/#353 passed separately but failed together, the user replaced the old unchanged-head merge rule. Final Full covers the exact current-base combination; strict protection blocks stale results. Keep implementation/review parallel and hold routine docs pushes during final validation.
Wind-down is bounded by actions: save the edit in hand, record review findings, start no new validation/repair and wait on no CI. Merge only already-reviewed/validated current combinations. A confirmed breaking merge with only docs after it is automatically removed with remote work preserved; AGENTS.md and the migration guide define recovery.
Implementation uses platform settings and existing documentation; no new controller or tooling is selected. The current explicit user wind-down limits override the normal loop.

Process audit follow-up (2026-10-22): #261/#365 setup/retry/failure reporting integrated via #368 at 376/400 lines with #361's ABI split; independent review/bootstrap Full passed and on-demand CI is restored. Small ready batches, pushed unfinished checkpoints and independent takeover remain required. Revisit Emscripten setup/merge-queue tooling after ten integration notes record stale Full reruns and waits; no new service, runner or local full suite is selected.

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
- Small maintenance work is batched: one PR, one reviewer and one Full run carry every ready cleanup, coverage, speed or tooling item. Eight of the last twelve integrations were separate small maintenance PRs, each paying the full pipeline.
- Agents run only the scenarios their change touches locally; the full default suite (38 minutes locally, repeated by Full in about 12) runs in CI.
- Local commands pass the free cores as `--jobs`, not a fixed 2. Last session they took 503 agent-minutes at 2 while half the cores sat idle for 171 of 261 minutes; 31 scenarios ran in 164 s at 2 and 44 s at 8, and the full suite in 10 minutes at 8.

The earlier unchanged-head merge permission is superseded above. AGENTS.md also says: a C++ class type crosses as its base type (#355); a local container the original builds stays a local `Vec`; review reports name orphaned WITH_RUST bodies, exports and declarations.

Provisional, checked next review: the 3% tip-rise threshold (basis: counts repeat to 0.04%; the largest ordinary per-PR move was 1.3%) and factor 2 (measured wall sensitivity 0.75-1.54 at `802ec7d6a4`, so it is conservative). The 6.6% layout spread is retired: a pass's median of four three-pair medians came within 1.4% of every projection.

The listed steering actions integrated through #358, #351, #340, #347, #353, #360 and #368. Current selections and remaining #156/#261 work appear below.

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
- **Tooling, WIP and CI:** proportionate tooling (#109) with a stated budget for infrastructure work; platform defaults over controllers (#233); six component branches at most; Full on demand (#171), strict final integration as corrected above, with no admin bypass for new code.
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
| #348 | Landscape clear, terraform and leveling | #353 | `33fc37e973` | 1289 / 189 / 301 / 447 |
| #354 | Road-stop bays and drive-through occupancy | #362 | `4075c50384` | 875 / 164 / 181 / 501 |
| #247 | Town commands, authority actions and deletion | #347 | `8c370210b4` | 977 / 270 / 298 / 589 |
| #315 | Rail construction and depot consist transactions | #360 | `eccfe3f240` | 1082 / 129 / 347 / 864; net glue 246 |
| #349 | Vehicle visits and route-link prediction | #359 via #368 | `4ddd8cfab4` | 1588 / 160 / 411 / 769; net glue 222 |
| #350 | Remaining generation and complete river construction | #364 via #379 | `dd31616e7a` | 1864 / 316 / 416 / 1214; net glue 316 |
| #372 | Ground-vehicle dynamics and canonical cache | #375 via #379 | `dd31616e7a` | 943 / 116 / 381 / 475; net glue 190 |
| #366 | Complete waypoint command policy | #369 via #382 | `032d0d150c` | 1093 / 142 / 194 / 484; net glue 139 |
| #367 | Complete PBS reservation service | #370 via #382 | `032d0d150c` | 1279 / 267 / 349 / 459; net glue 102 |
| #371 | Complete tile movement policy | #374 via #382 | `032d0d150c` | 2455 / 304 / 753 / 985; net glue 542 |
| #373 | Live link-graph scheduling and queues | #380 via #391 | `ac7d2b60f2` | 303 / 263 / 278 / 185; net glue 245 |
| #378 | Canonical live link-graph contents | #383 via #391 | `ac7d2b60f2` | 922 / 82 / 364 / 378; net glue 214 |
| #377 | Vehicle tile occupancy and queries | #384 via #391 | `ac7d2b60f2` | 1061 / 161 / 222 / 233; net glue 17 |
| #386 | Cargo-monitor maps and lifecycle | #388 via #391 | `ac7d2b60f2` | 371 / 240 / 157 / 196; net glue 117; acceptance basis 193 |

Harness/process #72/#84/#97/#88/#75/#90 and maintenance #209-#302 retain evidence in their PRs.
Rows below use original retirement and net glue (#223).

| Issues | Maintenance PRs | Integration | Commit | Metrics |
| --- | --- | --- | --- | --- |
| #355, #356, #357 batch and free-core `--jobs` | #358 | direct | `f141b6e91a` | 0 / 84 / 9 / 0; net glue +5 |
| #281 part 1 vehicle maintenance direct calls | #340 | direct | `9a991a1c23` | in PR |
| #274 shared assertion-faithful map reads | #351 | direct | `71d4f1d020` | in PR |
| #261 cleanup and selected CI maintenance | #361, #365 | #368 | `4ddd8cfab4` | in PRs; ABI tooling 172/180, CI changes 376/400 |
| #261 canonical station reservation setter | #376 | #379 | `dd31616e7a` | in PR; other drafts adopt this definition |
| #261 shared linear nearby-industry cursor | #381 | #382 | `032d0d150c` | 47 / 25 / 28 / 0; net glue 11; 100/100 added lines |
| #275, #356 typed routing and quiet anchors | #390, #391 anchors | direct | `824ebd56c5`, `ac7d2b60f2` | #390: 226 / 13 / 34 / 0; net glue 20; fixture 24/80 |

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

1. **#355, #356 and #357**: integrated as #358.
2. **#275 / #390 integrated:** post-fix counts and prescribed quiet remeasurement passed; genuine anchors integrated with #391. Only P2016 remains in band. The due #391 count passed; three projections rose more than 3%. The quiet attempt was contaminated; rerun it wholly on resume, then complete retained-profile attribution. No source fix is selected; other work has no speed gate. Residual pixel/track costs include required original work and ABI overhead. No broader raw-map/Vehicle permission or per-PR speed gate.
3. **#281** part 1 integrated (#340). Part 2: the callee-copy check and the four duplicate wrappers listed in the #340 r2 review.
4. **#274 map view:** integrated as #351, limited to the scheduler's clear/water/tree loops; C++ keeps the arrays and writes.
5. **Whole owners**, in readiness order; six component branches maximum:
   - #385 animation (#389): reviewed `cdc34942dd`; root accepts 349 implementation +81 native evidence +550 tooling versus unchanged 415 basis after boundary repairs. All caps remain 350/105/550; the one tooling raise is used, so further required growth is a user question issue. The 57 scaffold declarations earn no behavior credit. Actual sounds/setters/RNG, nonempty lifecycle/reload and native ANIT errors are default checks. Exact source helper order remains mandatory; dynamic same-TU station helper tracing is explicitly a #156 coverage limit under root's evidence re-selection. Preserve the saved base reconciliation; resume with fresh scoped conflict review, then affected checks, base update and Full.
   - #392 vehicle NewGRF resolver and canonical query cache: complete vehicle-specific scope/root/variable/callback/property/randomisation runtime and cache lifecycle. Native common Action2/specs/pools/rendering remain shared. First checkpoint proves canonical cache aliases/reuse, stack invocation state, direct owners and real 0x42/0x61, parent-override and trigger evidence. Planning budgets and full scope are on the issue; measure before broader evidence. Split review above about 1000 retired lines.
   - #393 NewGRF transactional storage: complete process registry/modes/registers, modern/legacy persistent payloads/backups and industry/airport/town creation policies. Native pool associations and save/error frames remain. First checkpoint proves canonical save aliases, all transitions/creation policies and nonempty reload. Planning costs can miss the ratio; no inherited waiver.
   - Integrated #373's separate explanation remains 185 retired versus 199 implementation and 309 evidence; #391 tooling 266 remains within 210/270/50 caps. Integrated #386 remains 113 implementation +4 native evidence +240 tooling versus unchanged 193 basis, within 120/8/240. No cross-owner offset or design waiver.

   New owners start independently from integrated `rust-migration`, without animation or other draft service dependencies. Needed shared services land once through root. Preserve canonical storage/typed leaves and #274's limited audited scope. Core owners receive measured boundary repair instead of abandonment; catchment and paused #69 are not selected.

6. **#168 tail, largest first.** `order_cmd.cpp` and `economy.cpp` hold 65 of the 198 baseline copies.

New owners reuse integrated services, never draft copies. A Rust-allocated
state block with C++ reference aliases (#216, #248) is acceptable. Keep save
staging only for a field a `SaveLoad` descriptor cannot address through the alias.

## Resume checkpoint

Paused at the user's request. All agents are closed and no local jobs remain. Preserve unfinished branches/worktrees and the retained speed evidence. Nothing is eligible to merge; no rollback is needed.

| Item | Saved head | Resume action | Jobs / CI at stop |
| --- | --- | --- | --- |
| #389 animation | `2290e57998` | Fresh scoped conflict review; then affected checks, base update and exact-head Full. | None. Quick green; no Full. Cargo4/maintenance passed; native build interrupted, ABI/scenarios pending. |
| #392 vehicle resolver | `48727d8fd1` | Finish unbuildable declaration/facade sketch and canonical cache/resolver checkpoint, then evidence/checks. | None. No checks or PR; automatic CI unchecked. |
| #393 transactional storage | `8b5e305555` | Finish payload/interfaces, creation policies and save aliases; then nonempty reload and checks. | None. No checks or PR; automatic CI unchecked. |
| #275 / #356 speed pass | frozen `ac7d2b60f2` | Rerun the whole quiet six; attribute retained ac7/824 profiles and select next maintenance. | None. Attempt contaminated/incomplete; no new ratios, anchors or retry. |

Session integrations: #360, #368, #379, #382, #390 and #391; rollbacks: none. Last game `ac7d2b60f2` passed Quick/Build/baseline and its combination Full; docs-only `851bbdd787`/`9ed430ce6c` Quick passed. CI for this final docs commit is pending/unknown; do not wait. #391 is merged and unwatched. CI_ON_DEMAND stays true; macOS is due after four more integrations. No new CI request or validation release is authorized during wind-down.

Preserve speed evidence checkouts `af1fea6a`, `bf961767`, `72a1223d`, `7ece2878`, `2a35b1c0` and the exact runtime. The failed attempt retains its partial reports/proofs. Keep unfinished worktrees/branches; cleanup is limited to verified merged work. Root uses gpt-6.1-sol ultra; implementation/reviews high, planning gpt-6-astra high. #281/#261 remain owned; #64/#68/#69 stay paused. Preserve the pinned reference; do not reapply `73ccd511fb`.

## Out of scope for the near term

More `src/3rdparty` ports, GUI rendering/layout and platform/toolchain expansion.
Deferred behavior improvements stay in `deferred-improvement` issues.

## Progress metric

Every port PR records `python3 tools/port-metrics.py`; `--whole-fork` gives totals and simulation share. Retire more original C++ than net glue plus tooling or explain why. A port that misses this ratio gets a boundary fix before deferral. Judge game logic owned; data tables inflate retirement. Avoid utility kernels.
