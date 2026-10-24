# OpenTTD-Rust near-term roadmap

Root owns selection here; `AGENTS.md` and `docs/rust-migration.md` define process.
If an issue conflicts, follow this roadmap and report it to root. Keep this
forward-looking, about 200 lines; completed work is one row, with evidence in PRs.

## Where the fork stands (2026-10-24, code at `8b3dd5ebef`)

- Sixty ownership ports. Original retirement:61,606 /470,612 C/C++ src lines (13.09%); simulation52,782 /102,615 (51.44%). Whole-fork Rust/tooling/glue:112,487 /40,407 /39,939. Source-line measures include comments/data, not completed behavior percentages.
- Since the seventeenth steering:22 owners and11,970 original lines retired. #437 integrated common Action2 #425 and property-boundary coverage #435 after a warning-only retry; previous #434 integrated house/town and reviewed maintenance. No regression or rollback.
- Latest completed profile at156915 passed6/6 semantic/end/log/identity comparisons. Under SAME quiet09eda anchors both tips' projections remain at floors: TGP1.138746x, Opus1.428848x, Grok1.416843x, P1996 1.321550x, P2000 1.381829x, P2016 1.463012x; counts fell0.238-1.663%, no >3% rise. This profile precedes common437; the next merged-tip profile is due after one more integration.
- New quiet156915 whole pass completed144 exits/72 equal pairs, four interleaved three-pair medians: TGP1.110315x, Opus1.396654x, Grok1.401286x, P1996 1.317390x, P2000 1.380024x, P2016 1.460536x. All below ceilings, onlyP2016 in band. Proposed complete anchors/baseline are ready maintenance; both09eda/156915 recomputed under them show no rise. No causal savings claim; collection hold ended before assessment. No hold is active.
- Current retained-only P2016 net attribution conserves3,064,970,050 excess Ir, preserving unexplained original raw/cache-5,691,628. Root selected aircraft counted/plain specialization and four direct controller-to-orders routes. Repaired track/deferred-order/occupancy families are not recycled; necessary scheduler/pixel costs have no further bounded defect established. Ready owners proceed while repairs are prepared.
- Exact-current-base Full remains strict:42 requests since steering,23 successful,12 failed,7 cancelled. #437's only failed attempt was one MSVC assertion-only unused parameter; all builds/native/shards passed, then the warning fix passed fresh Full. Due macOS Debug/Release both passed at8b3; next macOS is due after five more integrations. CI_ON_DEMAND stays true, broader infrastructure unselected.
- Six component branches: reviewed airport428, generic436 and object438; reviewed statue441; whole quiet156915 anchors443; speed source repairs442. Next439 engine cargo-capability/articulated queries, then440 rail/road introduction/admission are root-selected from fresh Astra research and source checks, queued after430. Cleanup count99; protected speed inputs remain until consumed.

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

The 3% trigger and factor 2 remain provisional: observed reference count drift is
about 0.055%, far below this tip's 2.17-2.68% play-work growth. At 032d the factor-2
projections exceeded quiet play ratios by 9.55-19.36%; keep using quiet passes to
decide actual ceiling proximity, not treating projections as wall results.
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
| #261/#281/#156 source, checker, coverage; #275 repairs/shared leaf | #396, #401, #402 | #403 | `6a2416df4e` | 301 / 399 / 91 / 4; net glue 40 |
| #275 complete quiet-ac7 six anchors | #404 | #406 | `77b8e663f2` | 0 / 8 / 0 / 0; net glue 0 |
| #156 structure coverage; #261 shared resolver metadata | #405, #408 | #412 | `6bb78266d2` | 0 / 136 / 40 / 0; net glue 28 |
| #261 shared metadata; #156 bribe coverage and frozen industry loader | #413, #414; #415 repair | #415 | `268c7c140d` | in PRs; combined 3120 / 1099 / 979 / 1592, net glue 723 |
| #261 goods/base scope; #275 anchors and repairs; #156 affordability/PBS observation | #419, #420, #421; #423 repair | #423 | `09eda66766` | in PRs; combined with #411: 1297 / 439 / 350 / 561, net glue 329 |
| #261 station/spec metadata; #156 vehicle real groups; MSVC animation return | #424, #426; #427 repair | #427 | `445ce4f30c` | in PRs; combined with #422: 880 / 305 / 362 / 369, net glue 286 |
| #156 wagon overrides; #275 track/orders and quiet anchors | #431, #432, #433 | #434 | `1569158485` | combined with #416: 966 / 308 / 303 / 430, net glue 209 |
| #156 property sign/failure boundaries; MSVC assertion-only warning | #435; #437 repair | #437 | `8b3dd5ebef` | combined with425:1120 /290 /520 /458, net glue479 |

## Phase 1: selected maintenance and coverage

The semantic harness remains the evidence base. #261/#281 own source-boundary
repairs, #156 owns missing branches, and #275 owns the measured speed work.
Their current selections and readiness order follow below; no separate gate or
new tooling stream is introduced. Keep one worker closing the coverage backlog.

## Phase 2: current work, in order of readiness

Integrate ready reviewed work first. Keep at most six component branches.

1. **Ready owner batch428/436/438:** whole and actual8b fixing reviews passed. Root spot-checked strict/nullable lookups, canonical state pointers, direct common/scope/variable/random/PSA dispatch and shared house/frame composition. Combine all three with reviewed maintenance, review substantive combination resolutions, then strict current-base Full. No renewed cost gate or future speed-base wait.
2. **#156/#275 ready maintenance:** statue441 and whole quiet156915 anchors443 join that concrete owner batch. Complete measured baseline stays together; no new median review or per-PR speed run.
3. **Speed source repairs442:** exactly counted/plain aircraft specialization and four direct ProcessOrders routes using canonical ownership. Focused checks are active, then fresh fixing review and next ready maintenance combination. Preserve raw/cache discrepancy and rank net costs; exposure is not promised savings.
4. **Next439 then440:** whole engine cargo-capability/articulated-query runtime first, paired rail/road introduction/admission second. Root source-checks confirm native bodies over existing Rust state. Scope/evidence/checkpoints are in the issues; start independently from integrated main only when capacity frees, with one concrete necessary-cost/bounded-repair disposition each.

Shared operations land once and drafts adopt them at final validation. Core owners
receive concrete repairs or necessary-cost explanations, not circular deferral.
Catchment, #64/#68/#69, vendored/GUI/platform expansion remain unselected. Finishing
the queued owners triggers fresh planning; it does not end development.

## Resume checkpoint

Development active; common437 merged and due macOS passed. Ready NewGRF batch in preparation; no timing hold.

| Item | Saved head | Resume action | Jobs / CI |
| --- | --- | --- | --- |
| #428 airport | `f099371635` | Final fixing commits spot-checked; combine reviewed runtime. | Cargo4/build/ABI and eight affected defaults pass. |
| #436 generic | `cb6ba96e0b` | Direct common adoption and observer fix spot-checked; combine. | Cargo4/build/ABI and seven affected defaults pass. |
| #438 object | `9a76a9707f` | Whole review/common context adoption spot-checked; combine. | Cargo4/build/ABI and eight selected defaults pass. |
| #441 statue / #443 quiet anchors | `17b808c0b7` / `433f453614` | Ready fragments included with owners. | Coverage reviewed; median-only record exempt; combination Full pending. |
| #442 speed repairs | `5700049e8d` | Aircraft witness/direct orders; finish checks, fresh review. | Cargo4 passed; native/semantic pipeline active; no speed run. |
| #439 / #440 | selected | Engine capabilities/articulated queries, then rail/road introduction/admission. | Queued, not started. |

Protected profiles/quiet inputs stay until #275 consumers finish; pinned reference
stays unchanged. RootSol xhigh, implementation/reviewSol high, planningAstra high.
CI_ON_DEMAND true; next macOS after five integrations. Old wind-down messages bind
only their stopped session. Exact-head CI/commands remain in PRs, not this table.

## Progress metric

Every port PR records `python3 tools/port-metrics.py`; `--whole-fork` gives totals
and simulation share. Retire more original C++ than net glue plus tooling or
explain the necessary cost after boundary review. Data tables inflate retirement;
judge game logic owned. AGENTS.md governs cost disposition and evidence scope.
