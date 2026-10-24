# OpenTTD-Rust near-term roadmap

Root owns selection here; `AGENTS.md` and `docs/rust-migration.md` define process.
If an issue conflicts, follow this roadmap and report it to root. Keep this
forward-looking, about 200 lines; completed work is one row, with evidence in PRs.

## Where the fork stands (2026-10-24, code at `cb4a01e86f`)

- Fifty-two ownership ports. Original retirement: 57,462 / 470,612 C/C++ src lines (12.21%); simulation 49,264 / 102,615 (48.01%). Whole-fork Rust/tooling/glue: 103,834 / 36,703 / 37,169. These are source-line measures, including comments/data, not completed behavior percentages.
- Since the seventeenth steering: 14 owners, 7,826 original lines retired, 2,772 net glue and 4,612 tooling lines added. Latest #391 integrates scheduler #380, live graph #383, occupancy #384 and monitors #388. The previous migration session integrated #360/#368/#379/#382/#390/#391; #395 subsequently trimmed CI setup/export idle time. No rollback.
- Current tip projections, using the 824 quiet anchors: TGP 1.112461x; Opus 1.473042x; Grok 1.464168x; P1996 1.368048x; P2000 1.442738x; P2016 1.516110x. All five play rises exceed 3% when BOTH tips use those anchors (#275). These are not wall measurements. The last complete quiet pass was at 824: P2016 1.453124x, the only measured scenario still in band. The ac7 attempt is incomplete and supplies no new anchors.
- Integrated code and current docs Quick are green; final combinations passed Full. Since steering: 28 Full requests, 15 successful, seven failed and six cancelled. Strict current-base protection is enabled. macOS is due after three more integrations; CI maintenance observation is 5/10. CI_ON_DEMAND stays true.
- Six unintegrated components/batches: animation #389 needs Full-failure repairs; #396 and shared prerequisite #401 are reviewed; speed fixes #402 are in review; storage #400 is reviewed pending shared-leaf adoption; resolver #397 continues whole-owner completion. Protected speed inputs remain; completed task/fragment checkouts were safely removed under #259. Next owners #398/#399 are selected but wait for the six-branch limit to fall.

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

Root's next actions:

1. Resume #389's substantive conflict review and affected checks, then current-base Full when ready. Its accepted whole-owner cost/evidence is settled; [PR disposition](https://github.com/fetch-laboratories/openttd-rust/pull/389#issuecomment-6370622209) removes the future speed-base dependency.
2. Assign #261/#281 source maintenance and one #156 coverage worker alongside #392/#393. Batch ready items once; do not wait for an unfinished checker or expand into a new framework. Issues contain the concrete scope and evidence.
3. Complete #275 attribution and the requested full quiet-six retry under the corrected hold rule. Fix demonstrated boundary cost forward while owners continue. No new profiling controller or raw-map/Vehicle permission.
4. Apply existing #259 cleanup to completed task checkouts; retain the explicitly protected speed inputs and unfinished owners. No host database changes or forced deletion.

The 3% trigger and factor 2 remain provisional: observed reference count drift is
about 0.055%, far below this tip's 2.17-2.68% play-work growth. At 032d the factor-2
projections exceeded quiet play ratios by 9.55-19.36%; keep using quiet passes to
decide actual ceiling proximity, not treating projections as wall results.
No user speed/CI/wind-down policy is reversed. The removed gates were agent
interpretations. The requested whole quiet-six retry remains. The steer skill
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
| #394 CI idle-time trims | #395 | direct | `cb4a01e86f` | 0 / 10 / 0 / 0; net glue 0 |

## Phase 1: selected maintenance and coverage

The semantic harness remains the evidence base. #261/#281 own source-boundary
repairs, #156 owns missing branches, and #275 owns the measured speed work.
Their current selections and readiness order follow below; no separate gate or
new tooling stream is introduced. Keep one worker closing the coverage backlog.

## Phase 2: current work, in order of readiness

Integrate ready reviewed work first. Keep at most six component branches; the
three owners and three maintenance/shared branches fill the six-branch limit.

1. **Ready maintenance #396/#401/#402:** source/coverage/checker batch #396 and canonical GRFFile-ID prerequisite #401 are reviewed; #402 carries the three attributed speed repairs and receives fresh fixing review. Batch ready reviewed heads on current base for one Full; the speed fix is the next maintenance integration. Root adopts the shared leaf into active owners only after integration.
2. **Animation #385 / PR #389:** whole-owner and conflict reviews passed; build/native188 and eight affected scenarios passed. First Full exposed stale industry/town probes and rail/PBS checks. Two scoped repair workers retain canonical owners/oracles, then root combines fixes for one fresh repair review and Full. No helper-trace redesign, cost ledger or future speed-base gate.
3. **Speed #275 and standing coverage #156:** retained ac7/824 net attribution selected #402; whole quiet-six retry is prepared outside its hold and remains to collect. Keep six coherent anchors and compare both tips under one anchor. The three selected coverage gaps are implemented in #396; resume backlog closure after integration. No per-PR speed check or new framework.
4. **#392 / PR #397 vehicle resolver/cache:** canonical cache/runtime first checkpoint passes; fresh completion worker ports native variable/0x61/purchase/subtype/aircraft control and remaining owner bounces, then finishes selected trigger/register/nonempty reload checks. Root accepts necessary cache/evaluator boundaries at the checkpoint; no new cost planner. Split final review above about 1,000 retired lines.
5. **#393 / PR #400 storage:** complete payload/process/creation owner and nonempty PSAC evidence pass independent review. Adopt integrated #401, remove the duplicate GRFID leaf, then current-base combination Full. Root accepted the necessary boundary/evidence cost; native associations/save-error frames remain.
6. **#259 lifecycle cleanup:** remove verified completed review/fix checkouts, retaining pending speed inputs until consumed. Broader infrastructure is unselected; revisit CI tooling only after the ten-integration observation already chosen.
7. **#398 industry and industry-tile NewGRF resolver:** own both feature runtimes, stack contexts, variables/roots/town selection, type/nearby policy and callback/probability/refusal routes. Reuse industry state directly; common Action2/specs remain native and storage stays #393. First checkpoint requires actual var67/68 and production/refusal callbacks in default industry scenarios.
8. **#399 rail-station/waypoint NewGRF resolver:** own scope/root/cargo policy, six caches for each resolver lifetime, platform/continuation queries, slope callbacks and randomisation. Direct cargo/movement routes and exact reseed RNG are the first checkpoint; animation stays #389. No draft-service dependency. Issues hold the interface/evidence plans; root selects a fresh worker when capacity frees.

New owners start from integrated rust-migration, never draft services. Root lands
a shared service once, and other drafts adopt it when preparing final validation.
Core owners receive concrete boundary repair or a necessary-cost explanation,
not circular deferral. Catchment, #64/#68/#69, vendored libraries, GUI rendering
and platform expansion remain unselected. Plan the next whole simulation owners
when the existing restart loop calls for it; completing this list does not stop work.

## Resume checkpoint

Development is active. Animation's first Full failed before integration; main is
unchanged. Ready maintenance/shared work proceeds while two scoped animation
repairs and resolver completion run. No Full is currently running.

| Item | Saved head | Resume action | Jobs / CI |
| --- | --- | --- | --- |
| #389 animation | `98c7fdb1e0` | Combine industry/town and rail/PBS repairs, fresh scoped review, Full. | Repair workers active; first Full failed. |
| #396 source/coverage/checker | `a8f83a312d` | Reviewed; root spot-checked scalar-narrowing fix. Batch current-base Full. | All affected checks pass. |
| #401 shared GRFFile ID | `e7058ac276` | Reviewed; batch ready prerequisite, then drafts adopt. | Build/native/object checks pass. |
| #402 speed repairs | `ace1f2cfbb` | Fresh fixing review, next maintenance combination. | Build/Cargo4/native/15 scenarios pass. |
| #397 resolver | `1b641104b7` | Fresh worker completes variable/helper control and selected evidence. | First-checkpoint checks pass. |
| #400 storage | `13fe137ba3` | Review passed; adopt #401/current base and Full. | No local jobs. |
| #275 quiet retry | frozen `ac7d2b60f2` | Prepared; collect whole six only during actual timed hold. | No timed collection or hold. |
| #398 / #399 next owners | selected | Industry resolver, then station/waypoint; start after integrations free branches. | Not started. |

Preserve speed checkouts `af1fea6a`, `bf961767`, `72a1223d`, `7ece2878`, `2a35b1c0`
and their exact runtime until #275 consumes them. Keep the pinned reference
unchanged. Current root is gpt-6.1-sol xhigh; implementation/review workers use
Sol high and planning uses Astra high. CI_ON_DEMAND stays true; macOS is due
after three integrations. Old wind-down messages bind only the stopped session.

## Progress metric

Every port PR records `python3 tools/port-metrics.py`; `--whole-fork` gives totals
and simulation share. Retire more original C++ than net glue plus tooling or
explain the necessary cost after boundary review. Data tables inflate retirement;
judge game logic owned. AGENTS.md governs cost disposition and evidence scope.
