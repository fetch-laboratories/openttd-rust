# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-06, stocktake 94; development through 95)

Agent: /root | Model: gpt-6.1-sol | Reasoning effort: ultra

- Code is `5391a8ee49` (integration 95): #720's one native pool facade and
  first vehicle allocation adoption passed both source reviews and exact-head
  Full at `d0b8655b9c`; the merged tree is identical. Local corrected-head evidence
  also passed eleven games/64 snapshots and seven affected native tests. Full
  vehicle fields/controllers/lifecycle/direct Tick remain #714; generic Pool is
  still instantiated elsewhere. The measured +181 C++ support cost is accepted.
- The census below remains **integration 94**, code `36a3681ee0`; the next census
  is after 96. #706/#667 completed in 94 after review and exact-head Full;
  #718's shared core integrated as 93. No behavior rollback was eligible.
- The same-implementation metric-version-2 census is **280,684 total fork C++
  lines**, down 214 from stocktake 92 at `55eff6fd40`. Historical original-plus-glue
  C++ is **257,612** (down 431); separate evidence C++ is **23,072** (up 217:
  tracked 17,304 and generated 5,768). The conservative candidate upper bound is
  **280,571**, including 2,602 unresolved evidence lines; reference-only evidence
  remains 113. These source inventories do not claim that every line is compiled
  into one executable.
- Original retirement is **62,091 of 284,355 (21.84%)**, up 676, all in network.
  By source-area accounting buckets: phase 1 logic 38,215; phase 2 storage 2,924;
  phase 3 readers 12,950; phase 4 support/graphics/data 5,326; phase 5 GUI/platform
  345; phase 6 vendored 2,317; upstream tests 14 separately. These are accounting
  buckets, not completed ownership. Simulation-source retirement remains
  **41,139 of 61,148 (67.28%)**. No coverage or deep run was performed.
- From `55eff6fd40` (10:25:48 UTC) to `36a3681ee0` (12:25:20 UTC), source C++
  fell 431 in 0.08301 calendar days: about **5,192 lines/day**; original retirement
  rose 676, about **8,144/day**. These short commit-timestamp intervals are not
  forecasts. #706's source reduction is 451 against its 500–700 estimate; the
  single shared TCP leaves, declarations and controlled protocol evidence explain
  the accepted cost. #718 adds 20 generated C++ lines, producing the combined
  431-line fall. Evidence growth remains visible in the smaller total-fork fall.
- Rust is **140,523** code lines (+1,889); tooling **60,915** (+622); glue
  **35,348** (+245). Exports are **1,570** (+28), callback slots **3,622** (+43).
  Rebound/shared-accessor/orphan baseline rows remain **351 / 119 / 43**.
  The higher boundary inventory includes new content and shared-pool interfaces;
  it is not crossing-retirement credit. Census endpoints use one implementation
  (`f4e2c5b45420…`); prior inventory corrections receive no deletion credit.
- Six component branches remain: **#700, #704, #714, #719, #721 and #722**.
  #637/#497 engine/renewal now runs in #722 using its prepared source brief.
  #719 has added actual graph/job allocation and typed descriptor access over the
  integrated facade; its single index-free shell-membership helper preserves real
  stack/copy records without reading uninitialized indices. Native evidence and
  source review are pending. #721's two source reviews have no findings; its
  measured map-reader follow-up is validating. Unintegrated code is not counted
  above. Records and attachments alone do not complete pool ownership.
- Housekeeping removed four clean completed checkouts (11.4 GB) after 92,
  two (2.6 GB) after 93, four (4.1 GB) after 94 and three (193.5 MB) after 95.
  Active, dirty, locked and
  retained evidence work remains protected. Issue placement finds **32 open,
  zero unplaced**; the deferred-improvement return conditions remain unmet.
- #636's shared-record/data-boundary work removed 1,952 C++ lines. Its direct
  Tick dispatch, routing, lifecycle and pool completion remain in combined
  #656/#598/#599, now #714. No duplicate retirement is claimed.
- **Speed:** integration 94 passed all seven canonical state/full log/stdout
  comparisons, equal end moments and drawing pixels at 8065 by 4183. Retained
  92-to-94 candidate state/log comparisons also passed. All prepared/current
  identities match the unchanged anchors, including selected Callgrind/core 3.26.
  All six projections below remain below their bands. The largest new rise is
  TGP's 0.2020%, so no new attribution trigger, quiet pass or re-anchor is
  indicated. Grok's earlier 3.4800% rise at 92 remains with #721 maintenance.
  This is no ownership PR gate. Existing threaded/GUI, arbitrary-GRF and
  whole-game limits remain on #156.

| Scenario | Original Ir | Candidate Ir (94) | Accepted projection (94) |
| --- | ---: | ---: | ---: |
| TGP | 2,265,528,237 | 2,812,367,493 | 1.192524x |
| Opus | 21,773,343,583 | 28,950,378,859 | 1.376962x |
| Grok | 20,969,727,163 | 27,669,829,947 | 1.395351x |
| P1996 | 3,600,502,896 | 4,699,117,427 | 1.352193x |
| P2000 | 6,049,922,238 | 7,712,164,520 | 1.444531x |
| P2016 | 11,343,424,651 | 14,075,811,339 | 1.382593x |
| Drawing | 2,149,015,961 | 2,154,636,809 | 1.004732x |

The [integration-94 profile and exact prior comparison](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6684817660)
uses the same anchors, selected tool/core and runtime identities as 92. P1996,
P2000 and P2016 remain below anchor counts, so their projection floors hold.
Drawing's projection is distinct from its raw Ir ratio. No new function savings
credit is inferred from this profile, and profiling elapsed time is not wall timing.

The [integration-92 attribution](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6682901535)
still selects #721: query/trackdirs combined self rose 667M Ir, follower self
449M and crossing predicate 175M on Grok. These are forward-maintenance targets,
not promised recoverable savings; inclusive attribution rows must not be added.
The first #721 checkpoint saved 0.119% of Grok Ir; its subsequent assertion-handler
copy repair awaits remeasurement. Anchors and speed-pass history are unchanged.

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
| **src/ + generated record total** | **284,355** | **61,415 (+5,239)** | **222,940 (-5,239)** | **35,281 (+950)** | **258,221 (-4,289)** |
| C++ evidence source in tools/ | 0 | 0 | 0 | 17,139 (+101) | 17,139 (+101) * |
| Generated ABI visitors/layout checks | 0 | 0 | 0 | 5,785 (-467) | 5,785 (-467) * |

The first total is the `src/` plus generated-record series in `port-metrics.py --whole-fork`: 21.60% original retirement
(+1.84 percentage points), with 90.81% of the original-sized C++ inventory
remaining (-1.51 points). A rewritten native consumer counts as retired original
plus new glue and does not reduce C++. This source/guard inventory covers all
platforms, not the preprocessed contents of one executable. Source area names
are accounting buckets, not phase boundaries.

The two starred rows are the separate `evidence_cpp` source census:
22,924 C++ evidence lines (-366). They mix candidate, shared and reference-only
support; **do not add all of them to candidate build distance**. The combined
fork source census is 281,145 lines (-4,655), and reference-only fork adapters
remain retirement work. #707 reports direct candidate/shared dependencies, reference-only support and
conservatively included unresolved sources, with all generated ABI checks. Counted with the same
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
1. Follow the current resume checkpoint for the active owners.
   Repairs through integration 91 and development integration 92 are integrated.
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
| Typing first, #673 then #501 (user after review 29) | Complete through #705 at 90, including its two diagnostic fixes. No broad harness rewrite. |
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
| 73–74, 76–79, 81, 84 | Shared typed records #674; typing batches #677/#680/#681/#682/#688/#689/#694/#695; host anchors #676; corpus #684 | `02393552fd` through `6e686c2b9e` | Eight typing batches and #673's record/dispatch step integrated; final typing file followed through #705 at 90. #507 closed. |
| 75 | GRF text/actions #655 | `da7faa8d0b` | Original retired +761; compiled C++ −648. |
| 80 | Shared logging and residuals #679, state-loop profile #683, combined #685 | `a2e417114b` | Compiled C++ −414; callback slots −184. |
| 82 | Vehicle shared-record/data-boundary checkpoint #636 | `2e6958801e` | Original retired +760; compiled C++ −1,952; slots −336. Direct Tick remains in #656. |
| 83, 85 | Infrastructure script APIs #691/#697 | `c9de8d52ca`, `7fd398da1f` | Compiled C++ −716 and −130. #709 repairs the reachable fatal-reporting path. |
| 86–87 | Windows vcpkg cache #701; release assertion parameters #702 | `ef51e41b80`, `bde3ce114c` | Cadence repaired; no claimed ownership retirement. |
| 88 | Industry specs #692 and game protocol #628, combined #703 | `5168023c13` | Original retired +2,082; compiled C++ −537; glue +1,545. |
| 89 | Native fatal reporting #709, repair #711 | `6426473252` | Production-path fatal probe and unrepaired-binary negative control; original/glue C++ unchanged. |
| 90 | Typing, TSan startup and network diagnostics #705 | `8fc7620ce5` | All-tools typing and precise one-use network allowances; platform workflow passed; C++ unchanged. |
| 91 | Accounting, housekeeping, identity and fact tools #712 | `561ec04c42` | Versioned census, ordinary cleanup and compatible profile guard; no ownership-retirement credit. |
| 92 | Map leaves #698 and harness consolidation #713, combined #715 | `f5f16377ce` | Historical C++ -178; total fork C++ -247; callback slots -57; original retirement unchanged. |
| 93 | Shared Rust pool core #716/#718 | `d5cb6a424e` | Coupled support; Rust +682, generated C++ +20; no production pool or retirement claim. |
| 94 | Content metadata/dependency/download state #706/#667 | `36a3681ee0` | Exact-head Full/tree match; combined 93–94 source C++ -431, total fork -214, original retirement +676. Seven profile comparisons equal, all projections below bands. |
| 95 | Shared native pool facade and first vehicle allocation #720 | `5391a8ee49` | Both source reviews and exact-head Full/tree match; eleven local games/64 snapshots and seven affected native tests pass. Measured +181 C++ coupled support, no full-vehicle or generic deletion credit. Platform cadence running. |

## Work order

**User-directed retrospective repairs (2026-11-06, #710) are complete.** #709,
#705's two findings, #707, #708, #275's identity correction and #710's fact
collectors are integrated. The discoverable port-review and storage-brief skills
reuse existing guidance and are installed. Opus 5.5 reviewed all changes. No
cadence, schedule or supervisor was established. The user authorized a new
Sol ultra root thread, Fast off, invoking `start-development` to resume migration.

Recovery #709 integrated as **89**, `6426473252`, through #711 after focused
scenarios, an unrepaired-binary negative control, Opus 5.5 review and exact-head
Full. #705 integrated as **90**, `8fc7620ce5`, after its scoped Opus review and
exact-head Full. The skills are installed and Opus-reviewed in
`39aa4dc2db`. #712 integrated as **91**, `561ec04c42`, after Opus review and
exact-head Full. #710 records the completed separate Opus 5.5 xhigh host investigation and its
supported-repair prerequisites. The user reported early completion delivery and
auto-settle detachment mechanisms; no host repair or setting change was made.
Continue verified-job recovery, without duplicating the investigation, adding
a polling supervisor or modifying host state.

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and the disjoint-service rule; finishing review and integration
of an open branch comes first. A lower rank can run while a higher one waits.
Scopes live in issues; per-PR changes live in each PR's steering comment.

0. **Development is active through integration 95.** Retrospective repairs and the separate host investigation are complete; supported host repairs still require user direction. Integration 94's stocktake and profile are recorded above. Integration 92's Grok attribution selected #721 forward maintenance; no new cadence or speed gate exists.
1. **Finish the open branches in readiness order.** The [resume checkpoint](#resume-checkpoint-2026-11-06-development-through-integration-95) holds current heads and next actions for the open owners. Integrated **#716/#718 shared Rust pool core** supplies the single allocation/bookkeeping definition for #634/#635/#656 and later selected pools; it is coupled support, not another phase-1 owner or a completed pool. Owners continue their disjoint work without duplicate primitives. Children inspect `.local/deep.lock.holders` before native work and use `--jobs 3`. Focused checks and diagnostic subsets address concrete failures; no universal native profile precedes review and Full.
2. **Typing and bounded harness consolidation are complete.** #705 completed #501, and #713 completed #673 in integration 92. The ANN ignore list is empty, `ty.toml` covers `tools/`, and the inventory/digest/comparison observations are preserved. No broader harness rewrite is selected.
3. **Maintenance and measurement.**
   - **#707, #708, #275's identity correction and #710's helpers** integrated as #712 at 91. Cleanup, stocktake and merged-tip profiling are complete. The separate host investigation is complete and documented on #710; no host change or new metric/ownership gate is selected.
   - **#156**: repair script sweep high-bit observations in the next evidence batch; establish the nonempty network execution-queue seam before choosing its observation. Threaded cancellation remains with the socket/resource owner or user-requested threaded work.
   - **#686 (the user's request):** measured and its capacity decision resolved on 2026-11-06: the repository's lower subagent caps are lifted and Codex has 12 slots. The next steering review checks review/resume turnaround under that limit. #699's cache repair integrated as #701 and its issue is closed.
   - **#275 integration-92 forward maintenance (#721 active):** selected a bounded batch for movement query/road-trackdir grouping, follower self work and the level-crossing predicate after source/native-inline comparison. Preserve behavior/RNG and measure actual savings; do not sum nested inclusive costs. Grok's projection-rise trigger is met, all bands remain clear, and ready ownership work proceeds.
   - **TGP Ir attribution**: done on #275. The `tile_loop::run` gate repair went into #698. The vehicle record↔shell hops (0.44% to 0.51%) went to #656.
   - **#565 map-leaf checkpoint #698 integrated in 92.** Its fallbacks, generation leaves, duplicated bridge/rail helpers and named #691 overlaps are retired within the reviewed scope. The final historical C++ reduction is 178 against the 300 to 600 estimate; required independent fixture observations explain the accepted cost. The retained town synthetic-slope fixture and seven services stay on #565 with #635; train/ship/reservation consumers belong to combined #656, and the remaining map bodies/allocation stay with the final map owner. This creates no phase-1 owner and does not claim the whole map boundary complete.
   - #500 typing adoption and the #168/#199 residual boundary cleanup attach to the owner touching those records. Completed-job publication stays with link graph and jobs.
   - **#678** (TSan startup aborts counted as clean runs) is complete through #705. **#693** (the mixed multiplayer diagnostic's late sync under host load) remains unproved; investigate it in a future selected tools batch without changing the original's frame/sync behavior or weakening an evidence check.
4. **Ranked phase-2 queue (#570 integrated as 82).** Placement is decided: later records sit **beside** their shells ([world-state design](design/world-state.md#later-pool-record-placement-634-635-decided-2026-11-05-after-integration-82)). Briefs state its two conditions: no out-of-line record↔shell call, and no stored references. Cargo-packet co-allocation is decided at #634's pool-completion checkpoint.

   | Rank | Owner | Estimated compiled C++ removed | Selection event / coupled scope |
   | ---: | --- | ---: | --- |
   | 1 | #634 station/road-stop/cargo attachments | 800 to 1,400 | #570 and placement decision; include CargoPacket/CargoPayment pool management, station vectors/names/caches, stored references, 110 to 160 field slots, `SharedForEachStation` and spatial callbacks. Coordinate flow publication with link graph. |
   | 2 | Active combined #656 + #598 + #599 vehicle owner (#714) | 750 to 1,250 | #570 integrated and remainder remeasured, including checkpoint 3/`SetDestTile`; residual records, names/sprite caches, maintenance, routing, lifecycle and pool allocation in one ownership PR. No duplicate #570 credit. |
   | 3 | #635 town/industry/subsidy | 500 to 1,000 | #570 and placement decision; 40 to 70 slots, mirrors/references, town coordinates, `psa_list` and remaining Rust-used caches. Disjoint from #634. |
   | 4 | Selected #717 link graph/jobs | 450 to 800 | #570 and integrated #628 schedule changes; graph/job storage, station flow publication and #168/#199/#500 completed-job publication. |
   | 5 | Selected #637 engine/renewal with #497 (#722) | 400 to 750 | Started from integrated 95 with prepared source/lifetime brief and world-state design. Adopt #655 and the one shared pool facade; include GRF/override and vehicle-specific payloads. |
   | 6 | Orders/order lists/backups | 350 to 650 | Combined vehicle owner's BaseConsist changes integrated; current fields, native pool management and field-service round trips into Rust order owners. |
   | 7 | Companies/groups | 300 to 600 | Vehicle unit-number ownership settled; adopt integrated network/company changes; include settings, finance/statistics aliases, names/identity and remaining Rust-used state. |
   | 8 | #638 small pools with #498 | 300 to 550 | #570; explicitly objects, depots, signs, goals, story pages/elements, league tables/elements, persistent storage and ObjectSpec payloads. |
   | 9 | Map storage and remaining map bodies | 2,000 to 3,200 | All simulation pools complete and #565 item 2 integrated; allocation/reset and remaining rail/road/water/tunnel/landscape/tile bodies. Pools-before-map ranks this last despite size. |

   Pools without an issue get one and a design section when selected. Every brief includes the accepted allocation/lifetime and exact shell dispositions above. No new phase-1 owners. Coupled save adapters, generated native declarations/layout checks and displaced field routes belong to each record owner.
5. **Phase 3 fills slots phase 2 leaves free**, after finishing the open owners:
   - **#666 infrastructure script APIs** integrated as 83 and 85 and the issue is closed. Its remaining `ScriptInfrastructure` goes to companies/groups, station tile lists to #634 and industry tile lists to #635.
   - **#667 content/download protocol (#706)** integrated as 94 and is closed. Rust owns the selected metadata/dependency/download state; native HTTP/file/decompression and GUI/format leaves retain their named later owners. Controlled protocol observations and exact-head Full passed.
   - **READY #668 coordinator/STUN/TURN**, 400 to 600 from 912 source lines. Own token/negotiation state using integrated packet handling and native socket/connect leaves. Compare packets, fallback order, controlled-time timeouts and connection disposition.
   - **#669 reader-remainder tracker** holds the named queued owners, ranges and selection events: chunk schemas/handlers; afterload/legacy; network core/discovery and remaining protocol payloads; admin; NewGRF configuration/common publication/actions; grouped world/economy, vehicle/order and story/text/list/event APIs; script discovery/configuration and AI/Game orchestration; remaining feature definitions. Activate its highest-ranked ready owner by expected net reduction, without double phase-2 credit.
   - **#573 continuation**, 600 to 1,000 provisional, after #570: preserve save-descriptor macros so slice 3b can run beside #634/#635, otherwise the second waits. Finish slice 4, corrupt-input/LoadCheck probes and native `CopyFromOldName`; remeasure excluding completed slices.
   - **#653 industry/industry-tile definitions** integrated in 88: records with Rust `Vec` containers behind a `RustVector<T>` view. HouseSpec's #628 dependency is cleared under #669; station/road-stop/airport definitions are grouped there. #622 is completed.

   These owners use integrated interfaces. Root supplies a shared service once under [integration](workflow.md#integration); retain genuine state/lifetime dependencies. Keep VM/binding surfaces until phase 6, and size owners to repay review/Full ([selection](planning.md#selection)).
6. **#503 coverage continuation last:** first the bounded clock-preparation and sanitized native-test-support build fixes, then the 119 unfinished traces with provenance checks. Do not recollect merely to fill a slot. Deep execution remains user-only.

Recurring:
- After every second integration, take stock under `/start-development` step 4, including the compiled-C++ rate and the baseline sizes. Integration 94's stocktake/profile are recorded above; forward maintenance for the earlier integration-92 Grok rise remains selected. Next stocktake/profile after 96. Profile the merged tip in the background ([speed](speed.md#instruction-projections)).
- Run the platform cadence after every fifth integration; integration 95's cadence is running at `5391a8ee49` ([run](https://github.com/fetch-laboratories/openttd-rust/actions/runs/42225369211)); no further integration until it passes, then next after 100. Integration 85's failure was repaired by #702 and its rerun passed. A new failure stops integrations. After two hosted-capacity macOS cancellations, use `CI_MACOS_HEAVY` as documented, and delete it afterwards.
- This host's first quiet pass integrated with #676 (77). Further passes follow the [speed policy](speed.md#policy) triggers.
- Deep checks run only on the user's request (user, 2026-10-31).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions.

Excluded, with return conditions reconsidered on 2026-11-06; neither trigger is met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Resume checkpoint (2026-11-06, development through integration 95)

Root is Codex (`gpt-6.1-sol`, ultra, Fast off), with fresh Sol high independent
reviews. Code is `5391a8ee49`: #720 integrated as 95 after both reviews and
exact-head Full at `d0b8655b9c`, with the same tree. The one native facade/schema
and actual vehicle allocation support are integrated; #718 core is 93 and
#706/#667 completed as 94. No rollback is eligible. `CI_ON_DEMAND` remains true.
The due platform cadence runs at the exact 95 code; further integrations wait
for [run 42225369211](https://github.com/fetch-laboratories/openttd-rust/actions/runs/42225369211).

The integration-94 census and seven-scenario profile are complete above, with
all six projections below bands and no new rise trigger. #721 remains the
selected forward maintenance for the earlier Grok rise. The freed branch slot
now holds #637/#497 engine ownership (#722). #719's first graph/job pool adoption
adds the one in-place index-free binding helper; stack/copy/borrowed ownership
must not read uninitialized indices or retire caller-owned records. Root will
promote earliest coherent shared adoption support when needed, without a hard
wait on a full unrelated owner or a parallel definition.

**Subagent limit (user, 2026-11-06):** Codex has 12 subagent slots. The main
checkout's uncommitted `.codex/config.toml` belongs to the user; do not commit or
revert it. Worktrees may retain their checked-in lower limit.

The old root docs checkout `.local/worktrees/root-docs-72` is stale and locked;
never push its checkpoint over current guidance. The separate #710 investigation
is complete with supported-repair prerequisites recorded. Do not duplicate it or
modify host settings/databases without user direction.

| Issue / PR | Pushed head | Current state | Next action | Active checkout |
| --- | --- | --- | --- | --- |
| #634 / #700 station records | `6ef06215d8` | Verify 99 Rust/241 candidate and 34 focused scenarios/170 snapshots passed, including actual final-owner fixture and four reloads. Drawing image/Ir running. C++ -102, slots -107 versus `c9d4adfe51`. [Status](https://github.com/fetch-laboratories/openttd-rust/pull/700#issuecomment-6680736375). | Finish checkpoint 2 names/vectors/field routes and actual pools, adopting integrated 95 only at the necessary API update. Keep one StationGoodsData; send concrete no-slot cases for the reserved helper. Fresh review/Full at readiness. | `t3-085f01f5` |
| #635 / #704 town records | `bbcd616070` | Verify passed 94 Rust/97 reference/240 candidate; affected town/industry scenarios running. Same-version cost C++ -88, slots -23, exports -1 versus `55eff6fd40`. Growth/reload and full original probe passed at `728c9a26f2`. [Status](https://github.com/fetch-laboratories/openttd-rust/pull/704#issuecomment-6675927407). | Continue fields/containers and actual pool adoption over integrated 95. Preserve borrowed and owned stack Industry semantics without index reads/slot publication. Final review covers live-ID guard, probe adaptations and native string identity. | `t3-8cd4fc03` |
| #667 / #706 content protocol | integrated `36a3681ee0` | Exact reviewed `36f0bff41c` Full passed; merged tree identical. #667 closed. [Completion](https://github.com/fetch-laboratories/openttd-rust/pull/706#issuecomment-6684815661). | Stocktake/profile 94 complete; retain controlled protocol traces and #156 limits. Root update/review checkouts cleaned. | original `t3-e897547d` retained evidence |
| #656/#598/#599 / #714 vehicle owner | `2f87f0fa6a` | Fourteen money/date/reliability/maintenance fields and forty slots retired; native compile found ambiguous Money/int64 subtraction, fixed with typed Money operand. Verification/twelve focused cases/ABI rerun active. Prior `5c515136ae` probes and aircraft cases passed. [Status](https://github.com/fetch-laboratories/openttd-rust/pull/714#issuecomment-6680888070). | Finish mixed carriers, remaining fields/containers, typed IDs and lifecycle/direct Tick. Preserve saturation, date/save widths and gates; reconcile the integrated 95 facade once without reverting direct tile links. No complete-owner/generic deletion credit. | `t3-a4c94f5a` |
| #716 / #718 shared pool core | integrated `d5cb6a424e` | Reviewed exact-head Full; 98 tests and source-derived lifecycle traces. Core adds 20 generated C++ lines with no retirement. | Other owners reuse this core and #720's single in-place schema/facade after integration. | completed core checkouts cleaned |
| #716 / #720 native facade and vehicle allocation | integrated `5391a8ee49` | Both source reviews/exact-head Full passed at `d0b8655b9c`, merged tree identical. Local `fa3242e536` eleven games/64 snapshots and seven native tests passed; accepted measured +181 C++ support cost. | Existing core/schema/facade is shared by all selected owners. Index-free binding extension belongs to first #719 adoption; owning-member retire differs from borrowed view teardown. Keep #156 limits explicit. | completed; local evidence `t3-57a288c6` retained |
| #717 / #719 link graph/jobs | `f0f67cf819` | Actual Graph/Job pool binding/retirement and typed lookup/iteration pushed over 95; Cargo/Clippy/102 Rust tests/eight ABI tests/preflight passed. Native verify and first independent source review running. Prior `9495d2ee84` source C++ -219, evidence +1, total fork -218; not current-pool cost. [Status](https://github.com/fetch-laboratories/openttd-rust/pull/719#issuecomment-6683281680). | Finish native evidence and the one #700 attachment/publication remainder. Review covers worker entry, staged record references, shared canonical scheduler slots, static descriptor member address and stack/copy isolation. Preserve audited cargo-only reroute lifetime contract. | `t3-cf46f29b` |
| #637/#497 / #722 engine/renewal | `d206740816` | Inventory on #637; first generated lifecycle/name/renewal records and native alias removal pushed. Cargo/Clippy/101 tests/eight ABI/preflight/audits pass; native verify and first cost recount running. [Status](https://github.com/fetch-laboratories/openttd-rust/pull/722#issuecomment-6686237802). | Complete EngineInfo/four payloads/GRF containers/direct consumers/field routes and actual pools. Preserve default/monostate/nonpooled ENGN/move semantics and save identity; no duplicate helper or premature owner claim. | dedicated T3 task from 95 |
| #275 / #721 forward maintenance | `d26fe66593` | Both source reviews have no findings. Prior `f92e03de2a` native97/240/eight scenarios passed and Grok Ir fell 0.119%. New handler-in-place reader optimization preserves raw/production assertion expectations; native/focused/measurement/final metric pipeline active. [Status](https://github.com/fetch-laboratories/openttd-rust/pull/721#issuecomment-6684937572). | Accept only fixed-head final counts/cost, keep no material-saving claim from the first small checkpoint. Root current-base Full at readiness, after platform95 passes. No new speed gate or anchor change. | `t3-44e4f3b0` |
| #275 profile after 94 | fixed `36a3681ee0` | Complete: seven state/log/stdout comparisons and drawing pixels equal; cross-tip candidate saves/logs also equal. All six projections below bands, no new attribution/quiet-pass trigger. [Report](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6684817660). | Preserve runtime identities, counts/projections and raw evidence for ordinary profile96. No jobs remain. | retained `t3-b7a95131` |

Preserve the following:
- the active open-PR checkouts above, each while its PR is open; old paused copies remain subject to ordinary clean/pushed-head housekeeping;
- `t3-b7a95131`, completed integration-94 profile/preparation/Callgrind and cross-tip state/log evidence;
- `t3-57a288c6`, completed integration-95 local pool-adoption evidence while #716 records the shared-helper follow-up;
- `t3-e897547d`, merged #706 controlled protocol traces and comparison evidence;
- `t3-44649657`, completed integration-92 profile/preparation/Callgrind evidence referenced by #275, retained for forward attribution;
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
  - integration 91's, `20261106T080938Z-3339040`;
  - the base TGP profile at `04c2e4fa5b`, `20261106T004411Z-2474065`;
- this host's speed-pass bundle and anchors under `.local/speed-pass/`.

The wind-down's housekeeping removed only three verification runs of integrated commits. It kept two runs without a report, `.local/simulation/20261105T110506Z-908834` (770 MB) and `.local/verification/20261105T110302.850651Z`, for review on resume.

**Disk (2026-11-06):** the root SSD filled to 335 MB free, which failed #705's loopback link. At the user's request, root then deleted 33 worktrees whose HEAD is in `origin/rust-migration`, each clean, unlocked and unused, together with their merged local branches. That left 78 GB free (63% used). `.local/reference/openttd` and every checkout listed above were kept. The deleted worktrees' HDD `.local` targets under `/mnt/storage/openttd-rust/worktree-local/` remain.
