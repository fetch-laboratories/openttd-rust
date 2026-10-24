# OpenTTD-Rust near-term roadmap

Root owns selection here; `AGENTS.md` and `docs/rust-migration.md` define process.
If an issue conflicts, follow this roadmap and report it to root. Keep this
forward-looking, about 200 lines; completed work is one row, with evidence in PRs.

## Where the fork stands (2026-10-24, code at `103ee7cff8`)

- Sixty-three ownership ports. Original retirement:62,169 /470,612 C/C++ src lines (13.21%); simulation53,345 /102,615 (51.99%). Whole-fork Rust/tooling/glue:114,035 /41,163 /40,566. Source-line measures include comments/data, not completed behavior percentages.
- Since the seventeenth steering:25 owners and12,533 original lines retired. #444 integrated reviewed airport428/generic436/object438 with statue441 and whole quiet156915 anchors443, plus the19-line concrete-animation ABI test repair. Exact-current-base Full passed; no regression/rollback.
- Latest103ee merged-tip profile passed6/6 semantic/end/log/identity comparisons. SAME156915-anchor projections: TGP1.403021x, Opus1.406031x, Grok1.411097x, P1996 1.368669x, P2000 1.421772x, P2016 1.501579x. TGP work+13.181%/projection+26.362%, P1996 projection+3.892%, P2000+3.025% trigger retained per-function/PR attribution. These are projections, not wall results; prior156915 counts remain preserved.
- Quiet103ee whole pass:144 exits/72 equal pairs, four interleaved three-pair medians, no contaminated samples. TGP1.209077x, Opus1.419874x, Grok1.414493x, P1996 1.343712x, P2000 1.412774x, P2016 1.483781x. All below ceilings; onlyP2016 remains in band. Workers resumed before assessment. Whole anchors448 are ready; re-anchoring does not repair or erase growth. Retained-only attribution selects the next bounded maintenance repair; ready owners proceed while it is prepared.
- Reviewed442 aircraft counted/plain specialization and four direct ProcessOrders routes adopted actual103ee and passed affected checks; root spot-check passed. Growth/P2016 account is posted on #275: TGP ambient/containing-loop frontier361.148M Ir after prior-work credit; explicit residuals and original raw/cache-5,691,628 remain. Tiny436 ambient gate and airport animation dispatcher are repair candidates, not promised savings. No repair PR started.
- Exact-current-base Full remains strict:44 requests since steering,24 successful,12 failed,8 cancelled. #444's first Full repeated a stale ABI regression-test assumption and was cancelled; the reviewed test-only repair passed fresh Full. macOS Debug/Release passed at8b3; next macOS after four further integrations. CI_ON_DEMAND true, broader infrastructure unselected.
- Paused at the user's request. Five drafts remain: ready442/445/448 and incomplete446/447. None has Full at its current head/current base, so nothing qualifies to merge. All known task jobs exited, subagents closed; no rollback or confirmed main regression. Cleanup111; protected inputs and user tar files remain. Effect-emission/cargo-lifecycle proposals stay unselected.

## Eighteenth steering review (2026-10-23, remove accidental gates and close shared-layer debt)

Four fresh Codex gpt-6.1-sol/high fidelity audits covered the largest new ports:
#364 generation (1,214 retired), #374 movement (985), #360 rail transactions (864)
and #359 visits (769). A fifth audited process and blockers. These share root's
model family at the user's request; the Opus tasks were stopped. Root verified
load-bearing findings against source, retained reports and transcripts.

- No reachable game-state divergence established. Autorefit restoration allocates a fresh Vec where original assignment reuses capacity. Strict graph lookup was replaced with GetIfValid; ordinary dangling-ID reachability is unproved. Named helper copies and duplicate native leaves remain. Fixing checklist: [#261](https://github.com/fetch-laboratories/openttd-rust/issues/261#issuecomment-6370620352).
- Animation's evidence caught a real airport RNG/SB defect before integration. Keep its accepted package. Nine animation tasks (three planning/inventory), six scheduler tasks and four monitor tasks repeatedly tried to force ratios before accepting necessary boundaries. The infrastructure budget rule had been extended to ordinary owner evidence; AGENTS.md corrects that scope.
- A ready #387 Full waited about 90 minutes for future speed maintenance. Quiet leases covered about 33 minutes for roughly 7.5 minutes of collection. The private 0.25-core cutoff aborted 66 games when an unrelated process used 0.267856 core; no calibration established timing contamination. AGENTS.md restores readiness order and confines holds to actual measurement.
- The speed report compared ac7 against 824's OLD-anchor projection. Recomputing both from the current anchor gives play rises of 4.33-5.37%, not the three previously reported. #275 is the authoritative correction and work list.

Standing corrections from reviews sixteen/seventeen and the later user correction:

| Correction | Verdict and cost versus result |
| --- | --- |
| Merged-tip profiles; no per-PR speed; band-only passes | Applied; identified real cost growth and selected #390. Nonblocking interpretation slipped as above. |
| Strict tested combinations and batching | Applied after the #347/#353 failure; later failed combinations stayed off main. Keep it. 27 Full requests still cost six failed/six cancelled attempts. |
| macOS every fifth integration; focused local checks/free-core jobs | Applied; both due latest-session macOS runs passed without capacity cancellation; no duplicate full local suite found. |
| Infrastructure budgets; proportionate owner evidence | Harmful reinterpretation into three owner ledgers; repeated planning stopped at the same eventual cost explanation. Scope corrected in AGENTS.md. |
| Header helpers, base-type returns, local containers and orphan review | MSVC/scratch fixes landed and reviews removed orphans; helper copies and one allocation slipped through. #261/#281 fixes are selected. |
| #156 standing capacity and #281 tail | Slipped; no dedicated backlog task in the latest session. Assign maintenance/coverage workers before another evidence planner. |
| Clean-base review exemption; stock-take docs | Slipped around clean reconciliation (#380 had an empty remerge diff but another review). Six integrations produced 16 docs commits; use existing exemption and selection/stock-take cadence. |
| Checkpoints, action-based wind-down and rollback | Applied: three unfinished checkpoints preserved; no new CI wait. No eligible rollback in the latest session. |

Disposition: #403 integrated the selected source/checker and coverage fixes plus
three repairs from the completed #275 net attribution. The full quiet-six retry
is complete. #389's probes and base conflict resolution passed fresh scoped
review and integrated exact-current-base Full. Standing #156 work and #259 cleanup
continue. No new profiling controller, raw-map/Vehicle permission or owner cost
ledger was introduced. Its [accepted disposition](https://github.com/fetch-laboratories/openttd-rust/pull/389#issuecomment-6370622209)
remains settled; ready owners do not wait for future speed maintenance.

The 3% trigger/factor2 remain provisional; quiet passes determine ceiling proximity and raw/cache drift stays explicit on #275.
No user speed/CI/wind-down policy is reversed. The removed gates were agent
interpretations. The requested whole quiet-six retry is now complete. The steer skill
uses the active host's native subagents, whether Codex or Claude Code;
the start-development prompt is unchanged.

## Earlier steering, still in force

Ownership and original observable behavior come first; C++ retains canonical map
arrays except the explicitly audited #274 read descriptor. Preserve original
callees, shared helpers/services, process scope and RNG. No per-call allocation
absent from the original, opcode dispatch, whole-record copies or owner bounces.
Evidence is default semantic checks plus exact-combination CI, with gaps on #156
and original retirement excluding glue (#223). Small maintenance is batched;
reviewers fix source findings and rerun only affected checks. Keep six component
branches maximum, strict protection, no admin merge, automatic bounded rollback,
and action-based wind-down. AGENTS.md is the authoritative rule text.

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
| #385 | Complete animated tiles and NewGRF animation | #389 via #406 | `77b8e663f2` | 1154 / 633 / 594 / 472; net glue 429; acceptance basis 415 |
| #393 | NewGRF transactional storage and feature policies | #400 via #412 | `6bb78266d2` | 399 / 221 / 249 / 258; net glue 247 |
| #392 | Complete vehicle NewGRF resolver/cache runtime | #397 via #415 | `268c7c140d` | 2047 / 442 / 599 / 1076; net glue 378 |
| #398 | Industry/industry-tile NewGRF resolver runtime | #407 via #415 | `268c7c140d` | 1076 / 436 / 364 / 516; net glue 329 |
| #399 | Complete station/waypoint NewGRF resolver runtime | #411 via #423 | `09eda66766` | 1207 / 261 / 267 / 561; net glue 254 |
| #410 | Road-stop resolver and spec associations | #422 via #427 | `445ce4f30c` | 876 / 187 / 319 / 369; net glue 285 |
| #409 | House/shared-town NewGRF resolver runtime | #416 via #434 | `1569158485` | 927 / 215 / 303 / 430; net glue 209 |
| #417 | Complete common Action2 execution and invocation | #425 via #437 | `8b3dd5ebef` | 1120 /213 /520 /458; net glue479 |
| #418 | Airport/airport-tile runtime and live spec selection | #428 via #444 | `103ee7cff8` | 1087 /245 /496 /264; net glue337 |
| #429 | Generic callback registry, AI and ambient runtime | #436 via #444 | `103ee7cff8` | 349 /188 /164 /248; net glue138 |
| #430 | Object invocation and complete eligibility | #438 via #444 | `103ee7cff8` | 334 /178 /182 /51; net glue152 |

Maintenance rows use original retirement and net glue (#223); evidence stays in PRs.

| Issues | Maintenance PRs | Integration | Commit | Metrics |
| --- | --- | --- | --- | --- |
| #355, #356, #357 batch and free-core `--jobs` | #358 | direct | `f141b6e91a` | 0 / 84 / 9 / 0; net glue +5 |
| #281 part 1 vehicle maintenance direct calls | #340 | direct | `9a991a1c23` | in PR |
| #274 shared assertion-faithful map reads | #351 | direct | `71d4f1d020` | in PR |
| #261 cleanup and selected CI maintenance | #361, #365 | #368 | `4ddd8cfab4` | in PRs; ABI tooling 172/180, CI changes 376/400 |
| #261 canonical station reservation setter | #376 | #379 | `dd31616e7a` | in PR; other drafts adopt this definition |
| #261 shared linear nearby-industry cursor | #381 | #382 | `032d0d150c` | 47 / 25 / 28 / 0; net glue 11; 100/100 added lines |
| #275, #356 typed routing and quiet anchors | #390, #391 anchors | direct | `824ebd56c5`, `ac7d2b60f2` | #390: 226 / 13 / 34 / 0; net glue 20; fixture 24/80 |
| #394 CI idle-time trims | #395 | direct | `cb4a01e86f` | 0 / 10 / 0 / 0; net glue 0 |
| #261/#281/#156 source, checker, coverage; #275 repairs/shared leaf | #396, #401, #402 | #403 | `6a2416df4e` | 301 / 399 / 91 / 4; net glue 40 |
| #275 complete quiet-ac7 six anchors | #404 | #406 | `77b8e663f2` | 0 / 8 / 0 / 0; net glue 0 |
| #156 structure coverage; #261 shared resolver metadata | #405, #408 | #412 | `6bb78266d2` | 0 / 136 / 40 / 0; net glue 28 |
| #261 shared metadata; #156 bribe coverage and frozen industry loader | #413, #414; #415 repair | #415 | `268c7c140d` | in PRs; combined 3120 / 1099 / 979 / 1592, net glue 723 |
| #261 goods/base scope; #275 anchors and repairs; #156 affordability/PBS observation | #419, #420, #421; #423 repair | #423 | `09eda66766` | in PRs; combined with #411: 1297 / 439 / 350 / 561, net glue 329 |
| #261 station/spec metadata; #156 vehicle real groups; MSVC animation return | #424, #426; #427 repair | #427 | `445ce4f30c` | in PRs; combined with #422: 880 / 305 / 362 / 369, net glue 286 |
| #156 wagon overrides; #275 track/orders and quiet anchors | #431, #432, #433 | #434 | `1569158485` | combined with #416: 966 / 308 / 303 / 430, net glue 209 |
| #156 property sign/failure boundaries; MSVC assertion-only warning | #435; #437 repair | #437 | `8b3dd5ebef` | combined with425:1120 /290 /520 /458, net glue479 |
| #156 statue search; #275 whole anchors; concrete animation ABI test | #441, #443; #444 repair | #444 | `103ee7cff8` | combined owners:1768 /800 /840 /563, net glue627 |

## Phase 1: selected maintenance and coverage

The semantic harness remains the evidence base. #261/#281 own boundary repairs,
#156 missing branches and #275 measured speed; resume coverage within the component limit.

## Phase 2: current work, in order of readiness

Integrate ready reviewed work first. Keep at most six component branches.

1. **First action on resume:** prepare the ready current-base maintenance batch442/445/448 and its one Full. Do not assemble, request CI or merge during this pause. Current reviewed heads are retained; final-base update and exact-combination validation remain required.
2. **Due #275 growth repair:** use the posted conserved103ee/156915 growth and current P2016 net account. The tiny per-call436 ambient gate and shared airport animation dispatch are candidates; no repair PR exists. Keep it due as the next maintenance item, preserve raw/cache discrepancy and explicit residuals, and do not recycle442's reviewed repairs or claim all exposure recoverable.
3. **Unfinished owners446/447:**446 next investigates nullable clone/refit-cost handling after the capability fixture crash;447 validates the saved per-actor TownID fixture repair. First paired cost checkpoints, remaining evidence/audits, fresh review and Full are pending. No fix/retry continues during wind-down.
4. **Unselected next-owner proposals:** whole vehicle effect-emission/cache policy and cargo registry/translation/callback lifecycle. Read-only planning is complete; neither is selected or started. Root selects only after resume, preserving native sorting/loader phases and existing-owner credit.

Shared operations land once. Catchment, #64/#68/#69 and vendored/GUI/platform
expansion stay unselected; no circular deferral or future maintenance-base gate.

## Resume checkpoint

Paused at the user's request.

| Draft / item | Saved head | Next action on resume | Jobs / CI |
| --- | --- | --- | --- |
| #442 speed repairs | `af29f9521a` | Reviewed/spot-checked; ready batch with445/448, current-base update and one Full. | No task jobs; no Full; push CI unknown. |
| #445 order-flag coverage | `0486abc627` | Reviewed, no findings; ready batch and exact-combination Full. | No task jobs; no Full; push CI unknown. |
| #446 engine/articulated (#439) | `e96e45c240` | Investigate nullable refit-cost clone path; finish paired evidence/cost checkpoint and review. | Build/Cargo4/native pass; simulate1/3, capability/reload crash. No jobs; push CI unknown. |
| #447 type admission (#440) | `c3fdd2227d` | Validate saved TownID fixture repair; finish paired evidence/cost checkpoint and review. | Last assessed pair failed; later retry exited unread. No jobs; no Full; push CI unknown. |
| #448 whole quiet anchors | `01bbc69578` | Median-only record exempt review; ready batch and exact-combination Full. | No jobs; harness19/Ruff/preflight pass; no Full; push CI unknown. |
| Due #275 growth repair | account6386132219 | Next maintenance: per-call ambient gate/airport animation dispatch candidates; root selects bounded fixes. | Analysis posted; agent closed; no repair PR or job. |
| Unselected proposals | planning complete | Effect-emission/cache, then cargo registry/translation lifecycle; no selection or start during pause. | Planner closed; no jobs. |

#444 Full passed;103ee push Quick/Build/baseline green at the user's15:49Z check.
Final docs CI pending/unknown, not waited on. No new CI request, task job or hold.
Protected profile/quiet inputs and tar files stay; next macOS after four integrations.

## Progress metric

PRs record `tools/port-metrics.py`; whole-fork source totals do not measure behavioral
completion. Decide necessary boundary/evidence cost once under AGENTS.md.
