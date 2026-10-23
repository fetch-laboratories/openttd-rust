# OpenTTD-Rust near-term roadmap

Root owns selection here; `AGENTS.md` and `docs/rust-migration.md` define process.
If an issue conflicts, follow this roadmap and report it to root. Keep this
forward-looking, about 200 lines; completed work is one row, with evidence in PRs.

## Where the fork stands (2026-10-23, code at `4ddd8cfab4`)

- Forty-three ownership ports. Whole fork: Rust 94,876 / tooling 34,817 / glue 35,264 / original retired 52,852 of 470,612 C/C++ `src/` lines (11.23%). Simulation: 44,716 / 102,615 (43.58%).
- Latest integrations: #360 rail construction, then #368 combining visits #359, cleanup #361 and CI #365. Both used current-base Full and strict protection; #368 restored verified `CI_ON_DEMAND=true`. The linear nearby-industry repair remains on #261.
- Speed: merged-tip profile at `4ddd8cfab4` passed all six comparisons/identities with the verified anchored runtime (#356): TGP 1.213575x, Opus 1.231172x, Grok 1.236176x, P1996 1.257340x, P2000 1.336737x, P2016 1.371234x. Largest rise since the accepted pre-rail tip: 1.04%; none in its speed-pass band. Anchors remain unchanged; next profile after two integrations.
- CI: the last due macOS Debug/Release run passed; two integrations since it, next due after three more. Bootstrap also tested both macOS configurations. #368 fixed observed Windows ABI-discovery/UTF-8 errors and one unused facade; its final runner dependency timeout passed a failed-jobs retry without moving head/base.
- Component branches: five reviewed owners (#364/#369/#370/#374/#375) and unfinished scheduler #380 (six of six). #379 combines generation, ground dynamics and shared station reservation #376; scoped review is complete, final current-base Full next. Waypoint/PBS/movement have completed #368 base reconciliation and await the integrated setter. #373 retains its owned boundary/linkage repair; #377/#378 start as slots free.

## Integration and wind-down correction (2026-10-22, takes precedence)

User authorized replacing the rules after #347 and #353 passed separately but
failed together. Keep automatic rollback; replace the time deadline with action limits.

- Final Full tests the combination to integrate; GitHub strict/up-to-date protection blocks old passing results after base moves. Update when preparing final validation; batch ready, reviewed work to avoid serial runs. Implementation and review stay parallel.
- Keep focused local checks and the full suite in CI; no extra local integration build gate. Avoid routine docs pushes during final validation, since they advance base too.
- Wind-down finishes the current edit/review and records its state, merges only eligible already-tested combinations, and starts no new CI, retries or repairs. Pending checks are handed off; no deadline or time estimate.
- A confirmed breaking merge with only docs after it is reset automatically without asking, with work preserved remotely and docs corrected. The tracking issue reopens; a replacement PR follows on resume. AGENTS.md and the migration guide define recovery and exceptions.
- Implementation uses platform settings and five existing documentation files; tooling budget: zero new code, workflows or controllers. Earlier unchanged-head merge permission below is superseded.

## Process audit follow-up (2026-10-22)

- **CI maintenance #261/#365 integrated via #368:** bounded setup/retries, early failed-child reporting and conservative coverage fallback; 376/400 changed non-documentation lines. Independent review and bootstrap Full passed; on-demand CI is restored. #361 remained a separate scope in the same tested combination.
- **Process changes now:** use small ready integration batches; save unfinished checkpoints before long checks or handoff; require independent execution for any unattended provider takeover. AGENTS.md and the migration guide define these rules. No new schedule or verified fallback is established by this documentation.
- **Existing conflict reduction:** finish #261 / #361's shared ABI split through its pending review and validation; no new issue or repeated unchanged-component review.
- **Deferred optimization:** root revisits Emscripten image setup and merge-queue tooling after the selected CI fixes land and ten integrations record stale Full reruns and ready-work waits in existing integration notes. No new service, runner upgrade or extra local full suite is selected.

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

Actions, in order (apply first; ports keep running alongside):
1. **#355, #356 and #357 as one batch PR,** before any other integration: the MSVC return fix and compile-time return-type check; the identity fix and the quiet anchors the tip profiles compare against (no pass is due); and macOS out of the pre-merge Full; and the driver's `--jobs` default set to the free cores (AGENTS.md). Until that lands, use the Blacksmith fallback on capacity cancellations.
2. **Integrate ready drafts** under the corrected integration rule above: #351 after #356 (rustfmt, then Full; no extension), #340 (two deletions, then review), #347 (PR comment items), #353 (fresh review).
3. **Ports continue:** #349 to its first passing milestone, #315, then owner selection (Phase 2 item 5).
4. **Batches:** the #261 cleanup list (2026-10-22 comments) as one batch PR, including the per-component split of `abi.rs`/`windows-abi.cpp`, the most frequent base-update conflict; the **#156** slot batches the new gaps.

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

Harness/process #72/#84/#97/#88/#75/#90 and maintenance #209-#302 retain evidence in their PRs.
Rows below use original retirement and net glue (#223).

| Issues | Maintenance PRs | Integration | Commit | Metrics |
| --- | --- | --- | --- | --- |
| #355, #356, #357 batch and free-core `--jobs` | #358 | direct | `f141b6e91a` | 0 / 84 / 9 / 0; net glue +5 |
| #281 part 1 vehicle maintenance direct calls | #340 | direct | `9a991a1c23` | in PR |
| #274 shared assertion-faithful map reads | #351 | direct | `71d4f1d020` | in PR |
| #261 cleanup and selected CI maintenance | #361, #365 | #368 | `4ddd8cfab4` | in PRs; ABI tooling 172/180, CI changes 376/400 |

Earlier maintenance (#307-#345) retains its rows' evidence in those PRs.

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

1. **#355, #356 and #357**: integrated as #358.
2. **#275** has no standing stream. When a pass or a tip rise opens speed work, rank it by net cost (#275 comment, 2026-10-22: aircraft, road-controller tick reads, scheduler carrier, orders).
3. **#281** part 1 integrated (#340). Part 2: the callee-copy check and the four duplicate wrappers listed in the #340 r2 review.
4. **#274 map view:** integrated as #351, limited to the scheduler's clear/water/tree loops; C++ keeps the arrays and writes.
5. **Next whole owners**, as WIP frees, each without draft service dependencies:
   - #350 generation/rivers (#364) and #372 ground dynamics/cache (#375): reconciled and reviewed in #379; final Full next;
   - #366 waypoints (#369) and #367 PBS (#370): reviews complete; adopt #376 after it integrates;
   - #371 tile movement (#374): source/base fixing reviews complete; later combination Full;
   - #373 live link-graph scheduling/queues (#380): early checkpoint fails ratio and native-generator linkage; fresh Astra boundary design selects the same-owner repair before evidence expands;
   - #377 vehicle tile occupancy/index and queries: selected after scheduling;
   - #378 live link-graph contents/mutation lifecycle: selected after occupancy.

   A shared service needed by two drafts lands once as its own PR (#376), then drafts adopt the integrated definition. Fresh Astra plans selected these whole owners without draft dependencies. Canonical map/pool storage and typed services remain; #274 grants no new raw access. Tight retirement boundaries get early measured checkpoints and an owned boundary fix if needed. #377 excludes viewport indexing; #378 excludes visits, scheduling and existing graph computation.

6. **#168 tail, largest first.** `order_cmd.cpp` and `economy.cpp` hold 65 of the 198 baseline copies.

New owners reuse integrated services, never draft copies. A Rust-allocated
state block with C++ reference aliases (#216, #248) is acceptable. Keep save
staging only for a field a `SaveLoad` descriptor cannot address through the alias.

## Resume checkpoint

Resumed 2026-10-23. Root is Codex (gpt-6.1-sol, ultra); implementation and independent reviews use gpt-6.1-sol high, owner planning uses gpt-6-astra high. Seventeenth-review actions 1-2 are done. Strict/up-to-date protection is confirmed enabled.

| Issue / PR | Head | Next step |
| --- | --- | --- |
| #350 / #372 / #261, batch #379 | `3fb2f988c4` | Frozen #364 `81f8720ec8`, #375 `30b9257466`, #376 `b0657f8bbf` preserved; scoped review and root fix spot-check complete. Update from this docs base, then final Full. |
| #366 / #369 | `0e85aebefa` | Base reconciliation reviewed; root price-shift/ABI spot-check passed. Adopt integrated #376, then concrete combination Full. |
| #367 / #370 | `f00356144a` | Base reconciliation reviewed; root spot-check passed. Adopt #376 and preserve movement's callback deletions in combination. |
| #371 / #374 | `fa2a503894` | Source/base reviews and root spot-check complete; preserve canonical foundation and direct Orders/visits routes in later combination. |
| #373 / #380 | `7295bec4d1` | Unfinished: 179 retired / 175 net glue / 0 tooling; direct game externs fail native generator linking; orphan audit remains. Astra boundary design active; no broad evidence started. |
| #377 / #378 | selected | Occupancy/index, then live graph contents as slots free. Exact scopes/evidence/planning budgets are on issues. |

#368 merged its exact validated combination; member PRs are merged and their worktrees removed. CI_ON_DEMAND is restored true. The due frozen-tip profile passed; no >3% rise or speed-pass band was reached. Complete routine docs before the next Full, then keep base stable. #368 needed platform fixes and one runner retry, with no stale-base Full rerun; integration notes carry the ten-integration observation.
Each open draft keeps one worktree; no unmerged component is adopted as another draft's service dependency. #281 part 2 and #261 R7/linear traversal remain on their issues. #64/#68/#69 stay paused. Preserve the pinned reference; do not reapply effect helper `73ccd511fb`.

## Out of scope for the near term

More `src/3rdparty` ports, GUI rendering/layout and platform/toolchain expansion.
Deferred behavior improvements stay in `deferred-improvement` issues.

## Progress metric

Every port PR records `python3 tools/port-metrics.py`; `--whole-fork` gives totals and simulation share. Retire more original C++ than net glue plus tooling or explain why. A port that misses this ratio gets a boundary fix before deferral. Judge game logic owned; data tables inflate retirement. Avoid utility kernels.
