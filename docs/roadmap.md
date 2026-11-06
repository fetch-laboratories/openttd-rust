# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-06, steering after stocktake 88)

Agent: /root (Codex steering review) | Model: gpt-6-astra | Reasoning effort: xhigh

- Code remains `5168023c13` (integration 88); navigation is `2d233f628b`.
  Development is paused. **Before further integrations, repair #709's confirmed
  native fatal-reporting regression.** Later code prevents bounded rollback.
- The steering recount matches navigation: 61,415 of 284,355 original code lines
  retired (21.60%), +5,239 since steering 29. The measured original-plus-glue
  inventory is 258,221, down 4,289 on the consistent recount. Its scope and the
  separately counted evidence C++ remain in Phases; #707 corrects the tool.
- Rust is 138,236 lines; tooling 58,669; glue 35,281; exports 1,542; callback
  slots 3,636. Current rebound/shared-accessor/orphan baseline rows are
  **371 / 119 / 43**. Simulation-source retirement is 41,139 of 61,148 (67.28%),
  not whole-game completion. No coverage or deep run was performed.
- Five component branches remain: #705, #700, #704, #698 and #706. The
  six-component bound is unchanged. #705 finishes typing across all 111 Python
  files; its diagnostic fixes and validation remain. The checkpoint records
  verified blockers rather than inferring readiness from green push checks.
- #636's shared-record/data-boundary work removed 1,952 C++ lines. Direct Rust
  Tick dispatch remains in #656 with routing, lifecycle and pool completion;
  the historical integration title does not establish that checkpoint.
- **Speed:** six fresh integration-88 candidate/reference state and log
  comparisons are equal with matched Callgrind 3.26. Candidate configuration
  identity still rejects their projections: CMake INTERNAL `prefix_result`
  differs from the retained build. #275 owns a bounded identity correction;
  anchors stay unchanged. The first seven-scenario harness run also compared
  equal, but its system 3.22 selection and configuration identity were rejected.
- Exact #698 at `79bc4eaa94` remeasures TGP at 2,791,048,267 Ir, with an accepted
  **1.174690x** projection. The diagnostic same-tool delta from 88 is −0.8754%;
  it does not establish an accepted base projection. No wall pass was run and
  no new policy-band trigger was established. Counts, attribution and limits
  are on [#275](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6677008877).

| Scenario | Fresh original Ir | Candidate Ir (88) | Last accepted projection (85, not 88) |
| --- | ---: | ---: | ---: |
| TGP | 2,265,549,108 | 2,815,697,949 | 1.206883x |
| Opus | 21,773,123,114 | 27,180,362,480 | 1.345115x |
| Grok | 20,969,239,210 | 25,907,367,430 | 1.346652x |
| P1996 | 3,600,184,286 | 4,505,587,789 | 1.352193x |
| P2000 | 6,063,062,862 | 7,443,766,699 | 1.444531x |
| P2016 | 11,348,111,721 | 13,599,751,552 | 1.382593x |
| Drawing | Not remeasured with matched tool | — | 1.004635x |

## Phases

Agent: /root (Codex navigation) | Model: gpt-6-astra | Reasoning effort: xhigh

The destination remains OpenTTD as a Rust program: every original C++ line
retired, the game behaving as the original does, and root's agents getting
there on their own. The phase order and earliest-ready selection remain the
user's 2026-10-28 decisions. Position is `bc6c727c48`, code `5168023c13`
(integration 88), at 2026-11-06 03:30 UTC. Parentheses compare the last navigation,
`c05fdd45e6` (code `6afdcbf390`, integration 65), using today's metric at both
revisions. Comments, blanks and preprocessor lines are excluded.

| Area | Original | Retired (change) | Original remaining (change) | Fork C++ (change) | C++ inventory (change) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Simulation logic | 47,424 | 38,215 (+575) | 9,209 (-575) | 6,630 (+170) | 15,839 (-405) |
| Simulation storage | 13,724 | 2,924 (+157) | 10,800 (-157) | 1,371 (+58) | 12,171 (-99) |
| NewGRF | 10,785 | 5,968 (+767) | 4,817 (-767) | 371 (+22) | 5,188 (-745) |
| Save/load | 13,388 | 1,335 (+260) | 12,053 (-260) | 982 (+255) | 13,035 (-5) |
| Script API | 14,908 | 2,394 (+1,192) | 12,514 (-1,192) | 738 (+196) | 13,252 (-996) |
| Network | 12,713 | 2,577 (+2,016) | 10,136 (-2,016) | 1,042 (+808) | 11,178 (-1,208) |
| Support | 21,617 | 953 (+23) | 20,664 (-23) | 470 (+18) | 21,134 (-5) |
| Graphics and viewport | 10,985 | 7 (+3) | 10,978 (-3) | 4 (+3) | 10,982 (0) |
| GUI | 52,179 | 345 (+230) | 51,834 (-230) | 392 (+230) | 52,226 (0) |
| Platform | 11,941 | 0 (0) | 11,941 (0) | 2 (0) | 11,943 (0) |
| Data tables | 17,295 | 4,366 (+2) | 12,929 (-2) | 13 (+13) | 12,942 (+11) |
| Vendored | 55,148 | 2,317 (0) | 52,831 (0) | 0 (0) | 52,831 (0) |
| Upstream tests | 2,248 | 14 (+14) | 2,234 (-14) | 5,685 (+393) | 7,919 (+379) |
| Boundary layer | 0 | 0 (0) | 0 (0) | 17,581 (-1,216) | 17,581 (-1,216) |
| **Measured source total** | **284,355** | **61,415 (+5,239)** | **222,940 (-5,239)** | **35,281 (+950)** | **258,221 (-4,289)** |
| C++ evidence source in tools/ | 0 | 0 | 0 | 17,139 (+101) | 17,139 (+101) * |
| Generated ABI visitors/layout checks | 0 | 0 | 0 | 5,785 (-467) | 5,785 (-467) * |

The first total is `port-metrics.py --whole-fork`: 21.60% original retirement
(+1.84 percentage points), with 90.81% of the original-sized C++ inventory
remaining (-1.51 points). A rewritten native consumer counts as retired original
plus new glue and does not reduce C++. This source/guard inventory covers all
platforms, not the preprocessed contents of one executable. Source area names
are accounting buckets, not phase boundaries.

The two starred rows complete the known source census outside that metric:
22,924 C++ evidence lines (-366). They mix candidate, shared and reference-only
support; **do not add all of them to candidate build distance**. The combined
fork source census is 281,145 lines (-4,655), and reference-only fork adapters
remain retirement work. #707 will split those dependencies and include all
generated ABI checks. Counted with the same
pinned lexer over archived tracked sources and each revision's generator,
including `src/rust/` for visitors. Generated record headers (313, formerly 60)
are already in the first total. The pinned original has no C++ source outside
`src/` with the metric's source suffixes.

The last navigation published 262,065 C++ lines; today's recount gives 262,510.
That +445 is #658's `.inc` inventory correction, not a regression. Compared
with the old printed number the fall is 3,844; the consistent series falls
**4,289**. Boundary exports are 1,542 (+99), callback slots 3,636 (-428), Rust
138,236 (+10,563), and tooling 58,669 (+5,058). Simulation-source retirement is
41,139 of 61,148 (67.28%, +732); it is not whole-game completion.

| Steering period (commit timestamps) | Days | Original retired | Retired/day | C++ inventory change | C++ removed/day |
| --- | ---: | ---: | ---: | ---: | ---: |
| `08b5a14907` to `fe58621b17` | 1.086 | +13 | 12 | -490 | 451 |
| `fe58621b17` to `e82d0d473a` | 0.594 | +197 | 332 | -257 | 433 |
| `e82d0d473a` to `a728d50781` | 0.273 | +2,784 | 10,182 | -2,437 | 8,913 |
| `a728d50781` to `95967b2aec` | 2.537 | +0 | 0 | +0 | 0 |
| `95967b2aec` to `bc6c727c48` | 0.871 | +5,239 | 6,015 | -4,289 | 4,925 |
| **Since navigation `c05fdd45e6`** | **3.454** | **+5,239** | **1,517** | **-4,289** | **1,242** |
| **Fork start `6ced7c097e` to position** | **34.213** | **+61,415** | **1,795** | **-26,134** | **764** |

Days are calendar days through the position commit, not active agent time.
Against the preceding navigation interval recounted consistently, retirement
rate changed from 1,539 to 1,517/day (-22) and C++ removal from 1,637 to
1,242/day (-395). Whole-history rates changed from 1,826 to 1,795 retired/day
(-31) and 710 to 764 C++ removed/day (+54). The earlier interval included the
1,958-line Monocypher replacement; this one has no vendored reduction. The
11-02 to 11-05 zero reflects the user pause, host move and infrastructure-only
integrations 66 to 68. The final 0.871-day burst is not a sustainable forecast.

The broad vehicle record port (#636, integration 82) supplied 1,952 of the
4,289-line reduction and removed 336 callback slots. Reader and logging owners
supplied most of the rest; support, graphics, GUI and platform buckets barely
moved. The result supports finishing pool ownership and grouped readers. At
the calendar rate, even the measured 258,221-line remainder is about 208 days
of work; this linear extrapolation excludes changing phase difficulty and the
unclassified evidence dependency. More agent slots are useful only if root
turns completed work into prompt review and integration; #686's latency study
found 29% idle time and 18% waiting for root to arrange reviews, with overlapping
per-PR waits, not 47% of calendar time attributable to a slot cap.

Navigation verdicts (causing text is cited at `bc6c727c48`):

| Part | Verdict and correction |
| --- | --- |
| Goal | Holds: `AGENTS.md:3-4` requires original and glue retirement. Planning now states the autonomous destination and the candidate/reference distinction explicitly. |
| Plan | Incomplete exits: `planning.md:16` allowed reassignment without a final no-C++ exit; `porting.md:26` could call a record port a completed pool. Require Rust pool bookkeeping/allocation/lifetime ownership, account for every residual at phase exit, and remove candidate native targets at the final exit. Later-phase planning still waits until adjacent. |
| Rules | Contradiction: `planning.md:29` and the accepted-reader brief parked a whole owner behind a shared service, while `workflow.md` Integration says root supplies it once independently. Use that integration rule; retain real state/lifetime dependencies and one integrated service definition. |
| Metrics | Incomplete build distance: `tools/port-metrics.py:392-395,446-447` omits the starred C++ rows. #707 is the bounded tool correction, first in maintenance priority after the user's outstanding typing work. No new gate. |
| Rate | Improving C++ retirement in ownership work; calendar pauses and host/infrastructure work dominate the flat interval. Keep compiled-C++ cost decisions and fidelity gates. This review does not infer per-agent productivity from subday merge bursts. |
| Autonomy | The ready-work planning trigger holds; phase exit now also triggers the next plan. Remove obsolete eight/five-child limits in `team.md:8,23` and `wind-down/SKILL.md:11`, applying the user's 12-subagent Codex limit. Host-aware orchestration replaces the unsupported background-native instruction in `steer/SKILL.md:59`, while retaining missed-job recovery and the outlet for user questions. |

Phase sizes below are source envelopes, not additive savings estimates. Shared
boundaries and deferred consumers overlap them; the position table is the
additive accounting. Plans for phases 4 to 6 remain deferred under the user's
next-phase-only rule; their exit outcomes describe the fixed destination.

| Phase | Current size | Exit outcome | Status |
| --- | --- | --- | --- |
| 1. Simulation logic | 9,209 original lines / 15,839 C++ in its accounting bucket, including drawing, startup, facade shells and bodies assigned to pool/map owners | Every remaining simulation body has a phase-2 owner; drawing and startup move with their later consumer phases | Exited at integration 63. No new phase-1 owners. |
| 2. Simulation storage | 10,800 original / 12,171 C++ in storage; carried logic above and field routes within the 17,581-line boundary layer | Rust owns every pool's records, slots, allocation/free/reset/iteration and lifetime policy, then map allocation and remaining map bodies. No Rust-used field remains behind a native service. Exact C++-only shell fields have a later consumer phase | Current. #570 integrated as 82; #634 and #635 are active. Combined #656/#598/#599, link graph/jobs, #637, orders, companies, #638 and finally map remain in the ranked queue. |
| 3. Simulation data readers | 38,860 C++ across save/load, script, network and NewGRF, excluding 3,793 script/network GUI lines; feature records also appear in simulation buckets | Save/load, API results, protocols and NewGRF behavior run in Rust with evidence for their observable outputs. Retained VM/binding shims have phase 6 as their removal event; platform and formatting leaves have phases 4/5 | Runs beside phase 2. #652, #653, #574 and #666 integrated. #667 is active; #668, #573 continuation and the ready #669 owners supply the next work. Recount their estimates when selected. |
| 4. Support | 45,058 C++ in support, graphics/viewport and data tables, plus startup in the logic bucket | Support behavior and tables are Rust-owned; remaining presentation/backend bodies are explicitly assigned to phase 5 and dependency adapters to phase 6 | Plan when phase 2 exits and this becomes the next phase. |
| 5. Interface | 64,169 C++ in GUI/platform, plus 3,793 reader GUI lines and simulation drawing | GUI, drawing, input and supported platform behavior run in Rust; remaining dependency adapters belong to phase 6 | Outcome fixed; implementation plan deferred until adjacent. |
| 6. Vendored and final retirement | 52,831 vendored C++; remaining shells, declarations, fixtures and adapters in the other rows retire with their last consumers | Retire all remaining original and fork C++, including generated glue and evidence adapters, and remove candidate native targets. Preserve the pinned original separately. Replace or port libraries by observed behavior; the user's Squirrel decision is taken when this phase reaches it | Outcome fixed; implementation plan deferred until adjacent. |

Root's first actions on development resume:
1. Follow the integration-88 checkpoint for the five open branches. Complete
   #705's typing and evidence repairs; add #707 to the following tools batch.
   Keep readiness-ordered reviews and the six-component WIP bound while using
   the host's available agent slots. No new speed or metric gate.
2. Finish #634/#635 as complete pool owners, not just record declarations.
   Select combined #656/#598/#599 and other ready phase-2 owners in the accepted
   order; #668 and #669 readers fill remaining slots. Root supplies any shared
   service once under the integration rule instead of waiting for an unrelated
   full owner.
3. At each phase exit, place its residual C++ and plan the newly adjacent phase.
   At stocktakes compare measured reduction with each owner's estimate, retain
   the separate evidence-source inventory, and keep the user's question outlet.

## Accepted reader and pool placement (2026-11-03, after integration 67)

Agent: /root/place-after67-reader-plan | Model: gpt-6.1-sol | Reasoning effort: high

Root accepted these ownership boundaries. The current reader envelope is in
[Phases](#phases); #669 holds the remaining owners and their selection events,
not an additive retirement promise. #653's industry definitions are integrated;
specification files classified under simulation storage/logic add work outside
the NewGRF bucket. Retain the VM, registration glue, stack conversions and
exception/suspension shims until their phase-6 owner. Executable API header
methods belong to their API owners; whole headers and new forwarding methods
are not savings.

Pool briefs include allocation bookkeeping (`data`, `used_bitmap`, `first_free`,
`first_unused`, `items`, assertion credit `checked`, `cleaning` and applicable
allocation-cache state), allocation/indexed allocation, reuse, iteration,
reset/cleanup and construction publication/destruction hooks. The generic
native pool template earns no repeated deletion credit while any C++ pool
instantiates it. Every Rust-used name/container/cache field moves in phase 2;
only exact original-C++-only fields may await a named later owner. Record lookup,
coupled save descriptors and generated declarations follow integrated #570.

Exact remaining shell dispositions constrain briefs:

| Boundary | Phase-2 ownership and exact later-owned remainder |
| --- | --- |
| Vehicle/type shells | Combined #656/#598/#599 takes maintenance, cargo/identity/loading, resolver fields, names and Rust-used sprite/cache fields; existing controller/cargo/order/visual/occupancy owners stay canonical. `coord`, `bounds`, viewport hash links and `fill_percent_te_id` belong to phase-5 drawing/viewport; `old_orders` belongs to #669 legacy conversion. Identity/vtable/owner-pointer scaffolding retires with final callers. |
| Stations/road stops/cargo | #634 takes names and Rust-used caches, spec-reference vectors, custom road-stop data, catchment/loading/nearby-list facades, GoodsEntry cargo/flow attachments and CargoPacket/CargoPayment pool completion. Link-graph owner takes flow/completed-job publication. `sign` belongs to phase 5; spec definitions belong to #669. |
| Town/industry/subsidy | #635 takes town `psa_list`, Rust-used `cached_name`/caches, industry `psa`, town/station links and subsidy flags; existing Rust name/cargo/count/nearby owners stay canonical. Town `cache.sign`/`show_zone` belong to phase 5. Industry `cached_name` may await phase-4 formatting only without a Rust field route; presentation facade storage retires with phase-4 string callers. Persistent-storage payload/pool is #638; industry definitions #653 and HouseSpec #669 after #628. |
| Engine | #637/#497 takes info, vehicle-specific info, original image/type/list position, GRF/override/badge payloads, lifecycle and name access after adopting #655. Only `display_flags`/`display_last_variant` await phase-5 build-list GUI. |
| Orders/lists/backups | All current fields and pool state move in phase 2; `old_order_index` and `OldOrderSaveLoadItem::{index,next,order}` belong to #669 legacy conversion. |
| Company/group | Phase 2 takes settings, finance/statistics aliases, identity, names/livery and free-unit/free-group state used by Rust. AI instance/info/config payload belongs to #669 script orchestration; authorization-list payload to #669 network remainder. Rust-used links move with the record. Only `Group::folded` awaits phase-5 GUI. |
| LinkGraph/LinkGraphJob | Graph/settings snapshots, join/abort state, annotations, flows and publication move in phase 2, including #168/#199/#500 completed-job publication. A native thread handle may remain behind a named join/start operation until platform threading moves; no field-callback exception follows. |
| Small pools | #638 includes objects, depots, signs, goals, story pages/elements, league tables/elements and persistent storage, plus #498 payloads and Rust-used strings. Only `Sign::sign` awaits phase-5 drawing/viewport. |

All issue estimates are net compiled C++ removed, including retained/generated
declarations, facades and fixtures. Rewritten consumers receive no reduction
credit. #658's **+445 code-line .inc re-baseline** is reported separately from
its small cleanup savings and #657's deletion savings. Reader evidence compares
API return values/errors/command results/enumeration and protocol packets,
ordering, timeout/error/connection decisions; world-state equality alone misses
these observations. Reader owners use integrated interfaces only. Root supplies
a shared service once under [integration](workflow.md#integration); a service
need does not park the whole owner behind an unrelated draft. Real state and
lifetime dependencies retain their selection events.

## Thirtieth steering review (2026-11-06: fidelity, completion and turnaround)

Agent: /root (Codex steering review) | Model: gpt-6-astra | Reasoning effort: xhigh

**Scope.** Since `95967b2aec`, root integrated 69 through 88 in one Claude
Opus 5.5 session, then stopped at the user's wind-down. Navigation at
`2d233f628b` owns the unchanged course above. Four fresh Sol/high source audits
covered network #628, script #691/#697, NewGRF #655 and vehicle #636; targeted
audits covered process/CI and speed. Root verified each finding acted on.
These are scoped comparisons, not certification of every changed body or ABI.

**Actions.**

1. **Recover the native fatal-reporting path (#709) before further integrations.**
   `AIRoad.CanBuildConnectedRoadParts(SLOPE_STEEP, [], north, south)` reaches
   Rust `unreachable!` instead of native `NOT_REACHED`. Both abort; the regression
   is the missing native error box/crash reason, not a new crash. #691 introduced
   this path; later code through integration 88 rules out the bounded automatic
   rollback. Use a small repair PR with scoped review and exact-head Full.
   Other owners may implement and review meanwhile. No production code changed
   in steering.
2. **Correct #636's completion claim.** Its 1,952-line C++ reduction is real,
   but `VehicleSchedulerTick`, virtual Tick facades, `OpenTTDVehiclePostTick`
   and `RunEffect` remain. It completed shared-record/data-boundary work, not
   direct Rust Tick dispatch. Keep checkpoint 3 in combined #656/#598/#599.
   #656 now also names the missed consist-cache invalidation bounce,
   `TrainSuppressImplicit` field write and encountered-record `train_handle`
   crossing. They preserve inspected behavior but violate boundary rules;
   remove them with that already selected owner, without duplicate credit.
3. **Restore one final full simulation tier.** The root-added universal native
   profile prerequisite replaced an expensive local full suite, but still
   duplicated Full. Native remains a diagnostic for a named uncovered fixture
   or failure, not a universal prerequisite to review. Exact-head Full,
   independent review and the user's platform gates remain. Update base and
   freeze membership before requesting Full. #698 proceeds to review without
   waiting for a PR speed comparison; speed work follows the merged-branch
   policy. Its shortfall against the retirement estimate needs an explanation,
   not scope added solely to hit a number.
4. **Repair housekeeping (#708), then use it.** The 12-hour activity fallback
   kept known merged T3 worktrees; 33 clean merged trees were removed manually
   after a link failed with 335 MB free. The SSD then had 78 GB free. Separately,
   run cleanup ignores explicitly retained older profiles. Keep ambiguous/live,
   dirty, unpushed and referenced work; exempt positively completed work from
   the activity fallback and protect retained run references. Until repaired,
   check the dry run against retained evidence and remove only verified eligible
   entries. This is a bounded tools batch beside #707 after #705.
5. **Use the existing completed-job recovery rule.** #679's timing finished at
   11:17:43; its reviewer ended nine seconds later still calling it running.
   Subsequent root wakes kept the native-work hold until user recovery at 15:14.
   This explains almost four reviewer-hours and overlapping held work, not a
   slot-cap bottleneck. Release quiet-host holds when games end, before report
   preparation. A rejected peripheral host action does not cancel the ongoing
   assignment. The restart prompt now makes those handoffs explicit. Codex
   background registration is provider-specific; `t3_background_wait` is Claude
   only here. Recover an idle child from its job/report on independent wakes,
   without polling a running delegate or introducing a scheduler.
6. **Bound the evidence claims (#156).** No new ordinary-build divergence was
   found in #628/#655 or the vehicle bodies compared. The script tile sweep's
   modulo-2^30 fingerprint loses high-bit/sentinel distinctions; repair its
   observations in the next evidence batch. Network commands start after
   MAP_DONE, so the current replay check does not prove nonempty execution-queue
   copying at save start. Establish reachability at that seam before choosing
   its bounded observation. Real threaded map cancellation, arbitrary
   GUI/game contention, legacy vehicle saves and NewGRF exhaustion/malformed
   combinations remain limits; no new deep run is selected.

**Speed verdict.** #636's 80→82 game-source interval reduces Opus by 1.285G
and P2016 by 573M Ir, while TGP grows 8.9M. Renamed/inlined work is not deleted
work. The new 88→#698 TGP total falls 24.65M Ir after offsetting Rust work;
no finer exact #692/#628 decomposition is available. Fresh reference captures
vary by up to 0.2172% despite matching identities and equal states/logs, so
instruction counts are not a one-instruction noise-free oracle. No new threshold
is imposed. #275's narrow configuration-identity follow-up joins the tools batch;
real build/tool/input changes must still invalidate anchors. All measurement
jobs finished; preserve their outputs until a replacement is recorded.

**Cost and standing corrections.** GitHub records 30 Full requests (24 success,
3 failure, 3 cancelled) and 20 native requests (9 success, 9 failure, 2 cancelled)
in this period. These are requests, not 50 independent integrations. #703's
successful stale-base Full took 13.9 minutes and had to be replaced. #692 and
#697's successful native runs repeated the simulation tier before Full. Fixture
failures were real; the correction removes duplication, not the final gate.
The #686 study records two carried-validation successes. No bypass was found.

| Standing correction / intervening change | Verdict, cost and next observation |
| --- | --- |
| New-host rebuild and first re-anchor (review 29, #676) | Applied; the 11-05 build/CTest and first host pass succeeded. Old-host runtimes remain unusable. The speed verdict above qualifies current projections. |
| Typing first, #673 then #501 (user after review 29) | Applied: core records and eight batches integrated; #705 holds the final file. Finish its two diagnostic fixes, without a broad harness rewrite. |
| #672 oracle guards (review 29) | Applied through #675. They guard inventory/case/mask loss, not whole-game fidelity. Do not turn an inventory update into permission to weaken observations. |
| #673 bounded helper consolidation (review 29) | Queued after typing; no unstarted rewrite is counted as progress. Keep digests, inventory and results unchanged. |
| #671 cleanup and wind-down preservation (review 29) | Slipped despite tool use; SSD exhaustion and manual cleanup recur. #708 fixes the specific decisions, not a new disk gate. |
| Idle-job recovery (review 28) | Slipped: almost four reviewer-hours plus overlapping hold. No capacity increase substitutes for reading completed jobs. |
| #628 bounded cost repair (review 28) | Applied and integrated; source audit found no new ordinary-build divergence. Socket/codec retirement and threaded evidence remain assigned. |
| #657 shared logging and #658 `.inc` accounting (review 28) | Applied through #679/#685; +445 inventory re-baseline is separate from deletion. #707 addresses the different evidence-source omission found by navigation. |
| #636 checkpoint 2 and placement decision (reviews 28/29) | Applied at −1,952 compiled C++; direct-dispatch completion was overstated. Beside placement is decided with bounded measurements; watch cargo allocation and hops at the existing pool checkpoint. |
| #655 bounce repair and readiness order (reviews 28/29) | Applied; it integrated before the shared logging owner. Source audit clear on the compared ordinary paths, with #156 limits retained. |
| #647 scoped repair and canonical-byte returns (reviews 28/29) | Applied; integrated as 72. #706's three new bool exports slipped, and Quick caught them. Fix that draft before review; keep the guard. |
| Platform-before-Full for build inputs and cadence (review 28) | Applied; #702 repaired integration 85's red cadence. Exact-head Full and cadence remain; next cadence is 90. |
| Carried Full, Windows/Emscripten caches (user-directed) | Applied; two recorded carries, #701 cache repair integrated. Stale-base and membership changes still wasted Fulls; enforce the existing freeze. |
| HDD symlinks (user-directed) | Applied; cross-device tool failures repaired. Builds remain on SSD, so output offloading does not replace worktree cleanup. |
| Native-before-review requirement (root, `04c2e4fa5b`) | Harmful duplication; removed as a universal requirement. Diagnostic subsets remain available. |
| TSan startup handling (#678) | Correctly narrowed in #705 to reject FATAL startup failures; a clean TSan run need not produce a report. Do not require nonexistent clean logs. |
| Twelve subagents and navigation phase/shared-service corrections | User capacity preserved, prospective results untested. No post-navigation development has run. Watch turnaround, complete pool allocation and residual accounting at existing checkpoints. |

**Issue verdicts.** No open issue was unplaced at survey. #500's obsolete
selection event was replaced with adoption by current record owners. #513 stays
excluded until the first accepted improvement. #12/#26/#41/#67/#71/#83 propose
changes to original behavior or undefined cases: retain near-full-reproduction
return conditions, including direct evidence for any touched masked field.
#650 returns at near-full reproduction or release-source publication; neither
has occurred. The issue-less exclusion matches are the user pause and future
phase policy, not omitted components. #693 remains open: matching source order
suggests a timing mechanism, but there is no retained original/original
reproduction establishing its cause. No user direction is reversed.

## Earlier steering, still in force

Ownership, original behavior, process scope, named callees, typed shared services
and exact random-draw order come first. Preserve default semantic evidence,
record gaps on #156 and keep retirement distinct from glue. No hot allocation,
opcode dispatch, whole-record copies or owner bounces; no C++ frame is kept only
for attribution. Rust reaches the map by a load through `map_func::World`, with
native assertions and no C++ fallback, and descriptor reads stay within
`docs/design/world-state.md`. The rebound-callee, shared-accessor and
orphan-export baselines only shrink. An owner does not port support code whose
data stays in C++ behind one service per field. Ready owners proceed beside
speed work; only timed collection holds builds and games. Ceiling and band
decisions use the three-pass record; the projection factor and speed thresholds
remain provisional, and 0.1% to 0.3% speed candidates get no attribution
research. Testing is maintenance: scenarios arrive with the ports that need
them. Six branches, batched maintenance, fixing reviews, strict exact-combination
Full, bounded rollback and action-based wind-down remain. A historical mention
is not placement; CI-found behavior fixes get a scoped review; deep checks run
only on the user's request; active speed dependencies are preserved. Third-party
test inputs stay out of Git and CI artifacts ([evidence](porting.md#evidence)). Every agent runs
its own long jobs and ends a turn only on a named job ID; root checks any child
that ends a turn without one, and any child idle on a job that has exited. A
base update that meets a review condition needs only CI. Each new scenario,
fixture or checker names the behavior existing checks cannot distinguish; this
rule is provisional, on a basis of 1.5 tooling lines per retired line. Roadmap
commits happen only at stocktakes, course changes and wind-down. #627's
per-container typed leaves for STL containers are a phase-3 necessity; they do
not justify per-field leaves in phase-2 records. Every owner's cost is its
compiled-C++ change, decided before review. Rust-defined `extern "C"` functions
return 0-or-1 `u8`, not `bool` (#644). Read a failed Full to its last job before
recording its cause. The decided beside placement and its revisit conditions live in the world-state design. Per-message `Debug` and `GrfMsg` services are design faults
after #657 integrated. A change to Cargo, the toolchain or cmake runs the
platform profile once before Full. AGENTS.md and its routing table carry the
rules; issue and PR bodies carry current actions and evidence.

## Recent completed work

Individual evidence and review reports stay in the linked PRs. Metrics below
are actual compiled-C++ reductions where stated, not sums of rewritten lines.

| Integration | Work | Integration commit | Result |
| --- | --- | --- | --- |
| 69–72 | Canonical settings #649; NewGRF corpus #670; guards/housekeeping #675; bool/tile-area repair #647 | `e76c94b306` through `3c30fb8129` | #649 compiled C++ −59 apart from the +445 inventory correction; #647 normalized byte returns and retired six orphan exports. |
| 73–74, 76–79, 81, 84 | Shared typed records #674; typing batches #677/#680/#681/#682/#688/#689/#694/#695; host anchors #676; corpus #684 | `02393552fd` through `6e686c2b9e` | Eight typing batches and #673's record/dispatch step integrated; #705 contains the last file. #507 closed. |
| 75 | GRF text/actions #655 | `da7faa8d0b` | Original retired +761; compiled C++ −648. |
| 80 | Shared logging and residuals #679, state-loop profile #683, combined #685 | `a2e417114b` | Compiled C++ −414; callback slots −184. |
| 82 | Vehicle shared-record/data-boundary checkpoint #636 | `2e6958801e` | Original retired +760; compiled C++ −1,952; slots −336. Direct Tick remains in #656. |
| 83, 85 | Infrastructure script APIs #691/#697 | `c9de8d52ca`, `7fd398da1f` | Compiled C++ −716 and −130. #709 repairs the reachable fatal-reporting path. |
| 86–87 | Windows vcpkg cache #701; release assertion parameters #702 | `ef51e41b80`, `bde3ce114c` | Cadence repaired; no claimed ownership retirement. |
| 88 | Industry specs #692 and game protocol #628, combined #703 | `5168023c13` | Original retired +2,082; compiled C++ −537; glue +1,545. |

## Work order

**User-directed retrospective repairs (2026-11-06, #710).** This session may
complete #709, the two review findings on #705, and the tools batch covering
#707, #708, #275's identity correction and #710's small fact collectors. Create
the two discoverable port-review and storage-brief skills, reusing the existing
guidance. Opus 5.5 reviews these changes. Check existing host completion support
before proposing a host repair; do not edit host databases. Establish no
cadence, schedules or automatic supervisor. Other ownership work remains paused.

Recovery #709 integrated as **89**, `6426473252`, through #711 after focused
scenarios, an unrepaired-binary negative control, Opus 5.5 review and exact-head
Full. The two #705 fixes are also Opus-reviewed; update that PR from the repaired
base and run its final Full next. The skills are installed and Opus-reviewed in
`39aa4dc2db`. #710 retains the host-completion limitation and the tools work.

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and the disjoint-service rule; finishing review and integration
of an open branch comes first. A lower rank can run while a higher one waits.
Scopes live in issues; per-PR changes live in each PR's steering comment.

0. **Recovery #709 is complete** through #711 at integration 89. Preserve the user's typing-first priority by integrating #705 next, then complete the selected retrospective tools batch. Other ownership work remains paused under this session's scope.
1. **Finish the open branches in readiness order.** The [integration-88 checkpoint](#resume-checkpoint-2026-11-06-stocktake-88) holds current heads and concrete next actions for #705, #700, #704, #698 and #706. Integration 85's cadence is repaired; #692 and #628 integrated together as 88, clearing their dependency events. Children check `.local/deep.lock.holders` before a native build, link or game, and pass `--jobs 3`. Rebuild an open PR's worktree only when its work resumes, after pulling it to the PR head. Use focused local checks and diagnostic CI subsets for concrete failures; no universal native profile precedes review and Full ([checks](workflow.md#checks)). #698 does not wait for PR speed evidence.
2. **Type checking first (user, 2026-11-05).** Typing takes each slot an integration frees, ahead of the harness batch and new owners.
   - Batches 1 to 8 integrated (#677, #680, #681, #682, #688, #689, #694 and #695 as #696).
   - #705 converts the last file, `tools/test-generated-abi.py`, after the integrated #628/#692 changes. Finish its review fixes from the checkpoint.
   - Done when the ANN ignore list is empty and `ty.toml` covers all of `tools/`.
   - **#673's consolidation PR** follows typing and the #707/#708/#275 tools batch. Inventory, scenario digests and comparison results must stay identical. It is the only harness refactor; no broader rewrite.
3. **Maintenance and measurement.**
   - **#707, #708, #275's identity correction and #710** join the next tools batch after #705: complete C++ evidence accounting, repair finished-worktree/retained-profile cleanup, resolve non-behavioral configuration identity, and compose existing PR/CI/metric data into small read-only readiness, latency and stocktake commands. No new metric or ownership gate.
   - **#156**: repair script sweep high-bit observations in the next evidence batch; establish the nonempty network execution-queue seam before choosing its observation. Threaded cancellation remains with the socket/resource owner or user-requested threaded work.
   - **#686 (the user's request):** measured and its capacity decision resolved on 2026-11-06: the repository's lower subagent caps are lifted and Codex has 12 slots. The next steering review checks review/resume turnaround under that limit. #699's cache repair integrated as #701 and its issue is closed.
   - **TGP Ir attribution**: done on #275. The `tile_loop::run` gate repair went into #698. The vehicle record↔shell hops (0.44% to 0.51%) went to #656.
   - **#565 item 2** (#698, work order item 1) with #168's unfinished world-generation leaves:
     - remove map fallbacks;
     - remove the remaining rail, crossing, depot, stop, water and tunnel leaves;
     - remove `tile_slope_z` and the per-tile world-generation services;
     - delete the duplicated `bridge_above` and `rail_tile_type`;
     - retire the map-leaf callbacks in #691's overlap table (recorded on #565) by reusing #691's helpers.

     Estimated compiled C++ removed: 300 to 600. It completes the phase-2 boundary and creates no phase-1 owner.
   - #500 typing adoption and the #168/#199 residual boundary cleanup attach to the owner touching those records. Completed-job publication stays with link graph and jobs.
   - **#678** (TSan startup aborts counted as clean runs) and **#693** (the mixed multiplayer diagnostic's late sync under host load) fold into the next tools maintenance PR, with that PR's reviewer and Full. Neither may change the original's frame and sync behavior or weaken an evidence check.
4. **Ranked phase-2 queue (#570 integrated as 82).** Placement is decided: later records sit **beside** their shells ([world-state design](design/world-state.md#later-pool-record-placement-634-635-decided-2026-11-05-after-integration-82)). Briefs state its two conditions: no out-of-line record↔shell call, and no stored references. Cargo-packet co-allocation is decided at #634's pool-completion checkpoint.

   | Rank | Owner | Estimated compiled C++ removed | Selection event / coupled scope |
   | ---: | --- | ---: | --- |
   | 1 | #634 station/road-stop/cargo attachments | 800 to 1,400 | #570 and placement decision; include CargoPacket/CargoPayment pool management, station vectors/names/caches, stored references, 110 to 160 field slots, `SharedForEachStation` and spatial callbacks. Coordinate flow publication with link graph. |
   | 2 | Combined #656 + #598 + #599 vehicle owner | 750 to 1,250 | #570 integrated and remainder remeasured, including checkpoint 3/`SetDestTile`; residual records, names/sprite caches, maintenance, routing, lifecycle and pool allocation in one ownership PR. No duplicate #570 credit. |
   | 3 | #635 town/industry/subsidy | 500 to 1,000 | #570 and placement decision; 40 to 70 slots, mirrors/references, town coordinates, `psa_list` and remaining Rust-used caches. Disjoint from #634. |
   | 4 | Link graph/jobs | 450 to 800 | #570 and integrated #628 schedule changes; graph/job storage, station flow publication and #168/#199/#500 completed-job publication. |
   | 5 | #637 engine/renewal with #497 | 400 to 750 | #570; adopt integrated #655 before deleting engine publication/name leaves; include GRF/override and vehicle-specific spec payloads. |
   | 6 | Orders/order lists/backups | 350 to 650 | Combined vehicle owner's BaseConsist changes integrated; current fields, native pool management and field-service round trips into Rust order owners. |
   | 7 | Companies/groups | 300 to 600 | Vehicle unit-number ownership settled; adopt integrated network/company changes; include settings, finance/statistics aliases, names/identity and remaining Rust-used state. |
   | 8 | #638 small pools with #498 | 300 to 550 | #570; explicitly objects, depots, signs, goals, story pages/elements, league tables/elements, persistent storage and ObjectSpec payloads. |
   | 9 | Map storage and remaining map bodies | 2,000 to 3,200 | All simulation pools complete and #565 item 2 integrated; allocation/reset and remaining rail/road/water/tunnel/landscape/tile bodies. Pools-before-map ranks this last despite size. |

   Pools without an issue get one and a design section when selected. Every brief includes the accepted allocation/lifetime and exact shell dispositions above. No new phase-1 owners. Coupled save adapters, generated native declarations/layout checks and displaced field routes belong to each record owner.
5. **Phase 3 fills slots phase 2 leaves free**, after finishing the open owners:
   - **#666 infrastructure script APIs** integrated as 83 and 85 and the issue is closed. Its remaining `ScriptInfrastructure` goes to companies/groups, station tile lists to #634 and industry tile lists to #635.
   - **Active #667 content/download protocol (#706)**, 500 to 700 from 923 source lines. Own metadata, dependency and download state; native HTTP/file/decompression operations remain. Compare packets, dependency order, cancellation, completion and defined malformed-input behavior locally.
   - **READY #668 coordinator/STUN/TURN**, 400 to 600 from 912 source lines. Own token/negotiation state using integrated packet handling and native socket/connect leaves. Compare packets, fallback order, controlled-time timeouts and connection disposition.
   - **#669 reader-remainder tracker** holds the named queued owners, ranges and selection events: chunk schemas/handlers; afterload/legacy; network core/discovery and remaining protocol payloads; admin; NewGRF configuration/common publication/actions; grouped world/economy, vehicle/order and story/text/list/event APIs; script discovery/configuration and AI/Game orchestration; remaining feature definitions. Activate its highest-ranked ready owner by expected net reduction, without double phase-2 credit.
   - **#573 continuation**, 600 to 1,000 provisional, after #570: preserve save-descriptor macros so slice 3b can run beside #634/#635, otherwise the second waits. Finish slice 4, corrupt-input/LoadCheck probes and native `CopyFromOldName`; remeasure excluding completed slices.
   - **#653 industry/industry-tile definitions** integrated in 88: records with Rust `Vec` containers behind a `RustVector<T>` view. HouseSpec's #628 dependency is cleared under #669; station/road-stop/airport definitions are grouped there. #622 is completed.

   These owners use integrated interfaces. Root supplies a shared service once under [integration](workflow.md#integration); retain genuine state/lifetime dependencies. Keep VM/binding surfaces until phase 6, and size owners to repay review/Full ([selection](planning.md#selection)).
6. **#503 coverage continuation last:** first the bounded clock-preparation and sanitized native-test-support build fixes, then the 119 unfinished traces with provenance checks. Do not recollect merely to fill a slot. Deep execution remains user-only.

Recurring:
- After every second integration (next after 90), take stock under `/start-development` step 4, including the compiled-C++ rate and the baseline sizes. Profile the merged tip in the background ([speed](speed.md#instruction-projections)).
- Run the platform cadence after every fifth integration; the next is after 90. Integration 85's failure was repaired by #702 and its rerun passed. A new failure stops integrations. After two hosted-capacity macOS cancellations, use `CI_MACOS_HEAVY` as documented, and delete it afterwards.
- This host's first quiet pass integrated with #676 (77). Further passes follow the [speed policy](speed.md#policy) triggers.
- Deep checks run only on the user's request (user, 2026-10-31).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions.

Excluded, with return conditions reconsidered on 2026-11-06; neither trigger is met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Resume checkpoint (2026-11-06, stocktake 88)

**Paused at the user's request** after integration 88. On development resume, #709 recovery precedes further integrations; review and implementation may continue within the six-branch bound.

Root ran in Claude Code (`claude-opus-5-5`, xhigh). Code is `5168023c13`, green on push. `CI_ON_DEMAND` is `true` and `CI_MACOS_HEAVY` is unset. No rollback was eligible.

The next cadence and stocktake follow integration 90. Steering adds #709 recovery before further integrations and corrects the next actions below. The integration-88 speed audit is complete with the projection-identity limit recorded above; no development restart was requested.

**Subagent limit (user, 2026-11-06):** the user raised Codex's per-session subagent limit to 12, in `~/.codex/config.toml` and in the main checkout's `.codex/config.toml`. Codex rejects values below 1, so the limit cannot be removed. The main checkout's change is uncommitted on purpose: leave it, and neither revert nor commit it. Worktrees still check out a limit of 8 until a PR changes the file.

The main checkout includes navigation `2d233f628b` and preserves the config edit. The previous root's docs checkout is `.local/worktrees/root-docs-72`; do not push its stale checkpoint over current guidance.

No development job or CI run was running at the pause. All steering measurement jobs also finished; they did not restart development. Every open PR's work is pushed, and each worktree is clean apart from its `.local` output. The [#705 review](https://github.com/fetch-laboratories/openttd-rust/pull/705#issuecomment-6675882700) is posted, and its checkout `t3-452f5347` is removed.

| Issue / PR | Head | State at pause | Next action on resume | Checkout |
| --- | --- | --- | --- | --- |
| #501, #678, #693 / #705 tools maintenance | `464f66c33a` | Native CI 42153621626 green. The [review](https://github.com/fetch-laboratories/openttd-rust/pull/705#issuecomment-6675882700) requested changes: two P2 findings. The reviewer's local `network-protocol-loopback` never ran: its candidate link failed with "No space left on device" when the root SSD was full. Re-run it on resume. | Fix finding 1, the exact `SendError(): error=7` one-use match, and finding 2, the close before authorization, as [handed off](https://github.com/fetch-laboratories/openttd-rust/pull/705#issuecomment-6675945551). Then base update, a scoped review of the fixes, Full, merge. | `t3-34d0fa9c` |
| #634 / #700 station records | `6e0ec19dc6` (WIP) | Batch 4 does not compile: aircraft leaves in `src/rust/services_station.h` still call the deleted `RustStationAirportRotation` and `RustStationAirportType`. Last good is `500463d213`: CTest 236/236 and the scenarios equal. [Status](https://github.com/fetch-laboratories/openttd-rust/pull/700#issuecomment-6675897921). | 1. Merge integration 88.<br>2. Fix batch 4's build.<br>3. Finish checkpoint 2: the aircraft leaf slots, with record-backed fake stations in `aircraft-range.cpp` and the Windows ABI aircraft probe; the ship and company slots keyed by station ID; the area services and three facade owner pointers; the k-d tree read; the new lifetime, mid-visit delete-and-allocate and #566 drawing evidence.<br>4. Focused validation, the checkpoint 2 report and port-metrics; diagnostic native CI only for a named failure.<br>5. Independent review and Full after #709 recovery. | `t3-98010525` |
| #635 / #704 town records | `e5097dd424` (WIP) | Not rebuilt since its last fix, which removed three needless `unsafe` blocks. Native CI 42154615345 at `fae36124db` failed on the same three unnecessary `unsafe` blocks removed by this head; the fix is not yet validated. Push checks at `e5097dd424` are green, including CI - Build and Quick validation. -24 compiled so far. [Status](https://github.com/fetch-laboratories/openttd-rust/pull/704#issuecomment-6675927407). | Validate the current head with local `verify` and focused scenarios; finish checkpoint 1 and its cost report. Use diagnostic native CI only for a concrete remaining failure, then review and Full. | `t3-19613657` |
| #565 item 2 / #698 map leaves | `79bc4eaa94` (draft) | Contains base `5168023c13`. Native CI 42152096539 green; local verify passed. TGP 2,791,869,839 Ir. Net glue -198, against the -300 to -600 plan. [Status](https://github.com/fetch-laboratories/openttd-rust/pull/698#issuecomment-6675915139). | Mark ready for independent source review; explain the estimate shortfall. Speed comparison proceeds separately. After #709 recovery, base update and exact-head Full. | `t3-c3bf9002` |
| #667 / #706 content protocol | `c013663ed7` (WIP draft) | The native build, Clippy, fmt, three unit tests, ruff and ty passed. Quick validation rejects three C-ABI `bool` returns in `network_content.rs` and `tcp_content_type.rs`; fix them before review. Not yet run: the `content-protocol` scenario, preflight, CTest, native CI and port-metrics. | Continue to the first checkpoint:<br>1. the inventory and the boundary, against the 500 to 700 estimate;<br>2. the metadata and dependency comparison with the original;<br>3. the scenario, run locally;<br>4. the bool-return guard, preflight and focused checks; diagnostic CI only if needed;<br>5. port-metrics.<br>Add its two #156 notes: libstdc++ multimap order, and the null-parent abort. | `t3-9803b4f2` |

Preserve the following:
- the open PR checkouts above, each only while its PR is open;
- `t3-96affac3`, the steering speed audit checkout and its `.local/steering30-speed/` and `.local/simulation/` evidence, until #275 records a replacement;
- `t3-3b491ddb`, with five dirty files;
- `t3-431c84b4`, locked, with #565's earlier 58 dirty files;
- `t3-d78d5f8b`, locked, holding the native-fixture evidence;
- `t3-32cf4edc`, with `tools/simulation/roads.py` staged on the merged `typing-501-batch6`;
- `t3-e130d5a3`, with `migration/saves/README.md` staged on `corpus-507`;
- the locked `.local/worktrees/profile-after68-664` and `.local/worktrees/root-docs-72`;
- the unchanged pinned original;
- the retained profiles under `.local/simulation/`:
  - profile 64's run, `20261102T135838Z-2198633`;
  - the anchor-head profile, `20261105T091320Z-509137`;
  - integration 80's, `20261105T185141Z-1237553`;
  - integration 82's, `20261105T215010Z-1842682`;
  - integration 85's, `20261106T010514Z-2542270`;
  - the base TGP profile at `04c2e4fa5b`, `20261106T004411Z-2474065`;
- this host's speed-pass bundle and anchors under `.local/speed-pass/`.

The wind-down's housekeeping removed only three verification runs of integrated commits. It kept two runs without a report, `.local/simulation/20261105T110506Z-908834` (770 MB) and `.local/verification/20261105T110302.850651Z`, for review on resume.

**Disk (2026-11-06):** the root SSD filled to 335 MB free, which failed #705's loopback link. At the user's request, root then deleted 33 worktrees whose HEAD is in `origin/rust-migration`, each clean, unlocked and unused, together with their merged local branches. That left 78 GB free (63% used). `.local/reference/openttd` and every checkout listed above were kept. The deleted worktrees' HDD `.local` targets under `/mnt/storage/openttd-rust/worktree-local/` remain.
