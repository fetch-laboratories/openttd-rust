# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-06, stocktake 96, steering 31)

Agent: /root (Claude Code steering review) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host

- **Code:** `01dcc7419f` (integration 96). Integrations 89 to 91 were the user-directed retrospective repairs. Integrations 92 to 96 were root's.
  - #698/#713: map leaves and the harness.
  - #718: the shared Rust pool core.
  - #706: content protocol.
  - #720: the native pool facade and vehicle allocation.
  - #721: map-reader speed maintenance.
- **Census (metric version 2, implementation `f4e2c5b45420…`):**
  - Original-plus-glue C++: **257,793**, down only 428 since navigation measured 258,221 at integration 88. #720's +181 shared-support cost is not yet repaid.
  - Evidence C++: 23,075 (tracked 17,305, generated 5,770).
  - Fork source census: 280,868. Candidate upper bound: 280,755.
  - Original retirement: **62,150 of 284,355 (21.86%)**. Simulation-source retirement: 41,141 of 61,148 (67.28%).
  - Rust 140,611; tooling 60,979; glue 35,588.
  - Exports 1,580; callback slots 3,622.
  - Rebound, shared-accessor and orphan baselines: 351 / 119 / 43.
- **Unintegrated owners:** the six drafts hold **-664** compiled C++ lines, none integrated. Per-owner figures, against their issue estimates, are in the [work order](#work-order).
- **CI:** since steering 30, 8 of 8 Full requests passed, and no native diagnostic was requested. Reviews start 0 to 8 minutes after a PR is ready and take 3 to 8 minutes.
- **Host:** 16 host SIGTERM kills of child jobs are tracked in #724.
- **Disk:** the SSD has 29 GB free (87% used), down from 78 GB at about 07:00. Of that, 69 GB is T3 worktrees; see steering 31, action 5.
- **Issues:** 35 open and none unplaced. This review added #724, #725 and #726. The deferred-improvement return conditions remain unmet.
- **Speed:** profile 96 is accepted at fixed clean `0ac8f70316`, the same code as `01dcc7419f` ([report](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6687384050)).
  - All seven canonical comparisons and all seven retained 94-to-96 candidate comparisons pass, and drawing is byte-identical.
  - Identities match the unchanged anchors at `27de6f8c18`.
  - All six projections are below their bands. Opus, Grok and the Padhattan saves are below their anchor counts, so their projection floors hold.
  - From 94 to 96, Ir fell by TGP 1.20%, Opus 3.84%, Grok 4.11%, P1996 1.98%, P2000 1.80%, P2016 1.65% and drawing 0.01%. These are combined 95 and 96 results, not per-symbol credit.
  - There is no rise trigger, quiet pass or PR speed gate.

| Scenario | Original Ir | Candidate Ir (96) | Accepted projection (96) |
| --- | ---: | ---: | ---: |
| TGP | 2,265,528,237 | 2,778,703,077 | 1.164363x |
| Opus | 21,773,343,583 | 27,839,081,434 | 1.345115x |
| Grok | 20,969,727,163 | 26,533,974,564 | 1.346652x |
| P1996 | 3,600,502,896 | 4,606,165,416 | 1.352193x |
| P2000 | 6,049,922,238 | 7,573,398,605 | 1.444531x |
| P2016 | 11,343,424,651 | 13,843,760,315 | 1.382593x |
| Drawing | 2,149,015,961 | 2,154,471,694 | 1.004635x |

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
| 3. Simulation data readers | 38,860 C++ across save/load, script, network and NewGRF, excluding 3,793 script/network GUI lines; feature records also appear in simulation buckets | Save/load, API results, protocols and NewGRF behavior run in Rust with evidence for their observable outputs. Retained VM/binding shims have phase 6 as their removal event; platform and formatting leaves have phases 4/5 | Runs beside phase 2. #652, #653, #574 and #666 integrated. #667 is integrated; #668, #573 continuation and the ready #669 owners supply the next work. Network/admin/SpriteGroup pool policy is coupled here and completes before final map allocation. Recount their estimates when selected. |
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

## Thirty-first steering review (2026-11-06: bank converging owners, host kills)

Agent: /root (Claude Code steering review) | Model: claude-opus-5-5 | Reasoning effort: not reported to the agent by host

**Scope.** This review covers the time since `7c4a448327`:
- the user-directed retrospective session, which made integrations 89 to 91 and added two discoverable skills;
- the Codex root from 08:28, which made integrations 92 to 96 and opened six owner drafts.

Root kept running during the review under the new notice, freeze and release protocol ([steer skill](skills/steer/SKILL.md)). Six fresh, read-only Opus audits ran:
- source fidelity of #706, #718/#720, #719 and #721, plus a sample of #698;
- process friction;
- phase-2 retirement.

The reviewer verified every finding acted on. These are scoped comparisons, not certification.

**Findings.**

- **Fidelity: no reachable divergence in the four audited ports.**
  - #719 has two rule violations that must be fixed before integration: an orphaned copy export with dead glue, and opcode dispatch in `ReadCargoFlowJob` ([PR comment](https://github.com/fetch-laboratories/openttd-rust/pull/719#issuecomment-6690327405)).
  - Integrated vehicle code reads `refit_cap` from Rust through a C++ service that reads the Rust record (`src/rust/vehicle_services.cpp:96`). The fix is in #714's scope ([PR comment](https://github.com/fetch-laboratories/openttd-rust/pull/714#issuecomment-6690328030)).
  - Shared-callee copies and a debug-only service are in batch #726. Unobserved branches are on #156.
- **Retirement: drafts accumulate instead of integrating.** Each of the six drafts has been open for 1.5 to 8 hours, and together they hold -664 unintegrated lines.
  - Deleting field services converges. It shrank the boundary layer by 203 lines in #700, 157 in #714 and 158 in #722.
  - Pool adoption accumulates: +99 lines in #704 and +181 in #720. Native fixtures grew by +388 lines across the drafts, and exports by +16.
  - Root's stocktake-96 "source-backed bounds" replaced the issue estimates after the results came in. This loosened a rule without user direction, and the roadmap stopped showing the shortfall. #704, for example, can remove at most 86 lines against its 500 to 1,000 estimate.
- **Process: CI is no longer the cost; children's jobs are.**
  - Host kills (#724) cost about 95 build-minutes, a duplicate verify, about 27 agent turns and six pidfd or select waiter sessions.
  - Six drafts ran 37 whole-fork recounts (about 228 wall-minutes) and made 272 status edits. The rules call for two recounts per PR, and root's briefs asked for the rest.
  - Five missed resumes left children idle for 11 to 25 minutes each.
- **Unowned C++.** 683 original YAPF lines have had no owner since phase 1 exited (#725).
- **Disk.** Retained trees hold about 18 GB: superseded profiles 92 and 94, the steering-30 audit, and evidence for merged PRs #706, #720 and #721. Two old record branches hold another 7 GB.

**Actions.**

1. **Bank converging checkpoints** ([integration](workflow.md#integration)). Integrate an owner's checkpoint once it removes at least 100 compiled C++ lines net, leaves no moved field behind a service, has no boundary design fault and passes review and exact-head Full. The owner continues in a fresh PR on the same issue.
   - #719 goes first, at -112, after its two PR-comment fixes and a scoped review. Its station attachment step stays on #717, behind #700's StationGoodsData closure.
   - #722 follows at its next passing checkpoint; it measures -255 now.
   - #714 and #700 integrate at their next qualifying checkpoints.
   - #704 is at +64. Take the cost decision first ([PR comment](https://github.com/fetch-laboratories/openttd-rust/pull/704#issuecomment-6690328583)).
   - Readiness order and one-at-a-time integration of overlapping PRs still apply.
2. **Move fields before adopting pools, and keep estimates next to measurements.**
   - A field moves in the same change that deletes its Rust-called services. No service is routed through `Record()` as an intermediate step ([storage ports](porting.md#storage-ports)).
   - Pool adoption follows an owner's deletions.
   - The work-order table again shows each issue estimate beside the measured change ([cost decisions](planning.md#cost-decisions)).
   - Pooled production shells bind by index, not by scan ([world-state](design/world-state.md#shared-pool-policy-716)).
3. **Two recounts per PR; status edits at milestones only.** Run the whole-fork metric at the first concrete boundary and at the integrating head. Briefs ask for no other recounts. Status comments change at milestones only ([planning](planning.md#cost-decisions), [workflow](workflow.md#records)).
4. **Host kills (#724; no user action).** Start long jobs with `exec`. A wrapper exit 143 with no logged failure means a host kill: read the log, then rerun once incrementally. Add no waiters or duplicate runs ([team](team.md#spawning)). T3's protocol log rules out a T3-ordered kill, so the sender is still unknown. Root adds the small signal-witness wrapper described in #724, alone as a fix needed by live drafts. The first kill it witnesses names the sender. Root adds each new kill time to #724.
5. **Disk.**
   - Profile 96 is accepted on #275, so release the superseded profile trees `t3-44649657` (92) and `t3-b7a95131` (94), and the steering-30 audit tree `t3-96affac3`.
   - Release the merged-PR evidence trees `t3-e897547d` (#706), `t3-57a288c6` (#720) and `t3-44e4f3b0` (#721) once their PRs hold that evidence.
   - Check the old record trees `t3-19613657` (`town-635-records`) and `t3-98010525` (`station-634-records`). Push any unpushed head and record it on its issue, then remove the tree.
   - Use housekeeping's ordinary checks throughout. Keep profile 96, the anchors and the pinned runtimes.
6. **Placements.**
   - #725 enters the phase-2 queue.
   - #726 is the next small maintenance batch. It takes the script-sweep high-bit repair from #156, which slipped from steering 30, when that item is ready.

**Standing corrections and intervening changes.**

| Correction or change | Verdict, cost and next observation |
| --- | --- |
| #709 fatal-reporting recovery first (review 30) | Applied: integrated as 89 through #711 with a negative control. Cost: one cancelled Full and one passing Full. |
| #636 claim correction (review 30) | Applied. #714 carries direct Tick dispatch, and the claims are scoped. |
| One final Full tier; native is not a prerequisite (review 30) | Applied and effective. 8 of 8 Fulls passed with no native requests, against 30 Full and 20 native requests in the previous period. Keep it. |
| Housekeeping #708 (review 30) | Applied in #712 (integration 91), removing 3 or 4 trees per integration. The preserve list kept growing and the SSD is at 87% again. See action 5. |
| Completed-job recovery (reviews 28 and 30) | Applied but costly: five idle gaps, 3 of 8 recovery messages sent to children that had already resumed, and 44 process scans by root. Host kills are a separate cause (action 4). |
| #156 evidence bounds (review 30) | Recorded. The script-sweep repair and the network-seam observation slipped because no batch ran. They join the next batch (action 6). |
| Retrospective skills and fact tools (user, 11-06) | Applied. #717/#719 used `brief-storage-owner`, and this review used the `retrospective.py` facts. Watch whether evidence-contract reviews keep adding native fixtures. |
| Twelve subagent slots (user, 11-06) | Applied. Turnaround is fast. The six-branch limit binds, not the slot count, and the open drafts' size is the problem. |
| Stocktake-96 accepted bounds (root) | Reversed by action 2. The bounds stay as remaining-work notes beside the estimates. |
| Per-checkpoint recounts in briefs (root) | Outside the rules: 37 recounts. See action 3. |

**Issue verdicts.** The survey found 32 open issues and none unplaced.
- New: #724 (root identifies the kill sender), #725 (phase-2 queue) and #726 (maintenance batch).
- #710 stays open. Its one user choice is whether to turn off T3's "Auto-settle on merge" sidebar setting, which detached a thread with live jobs. #724 is a separate mechanism.
- The exclusions are unchanged and their return conditions unmet: #12, #26, #41, #67, #71 and #83 under #513, and #650.
- #693 stays queued for a selected evidence and tools batch.
- No issue closes.

No user direction is reversed.

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
rules; issue and PR bodies carry current actions and evidence. Native fatal paths keep the native error
box and crash reason (#709). Native diagnostic subsets address a named failure
or uncovered fixture; exact-head Full is the final simulation tier. A completion
claim names only the delivered scope (#636). Release a quiet-host hold when the
timed games end; a rejected peripheral host action does not end the assignment.

## Recent completed work

Individual evidence and review reports stay in the linked PRs. Metrics below
are actual compiled-C++ reductions where stated, not sums of rewritten lines.

| Integration | Work | Integration commit | Result |
| --- | --- | --- | --- |
| 69–88 | Settings, corpus, typing, shared records, GRF text #655, logging #679, vehicle records #636, script APIs #691/#697, CI repairs, industry specs #692 and game protocol #628 | `e76c94b306` through `5168023c13` | Compiled C++ −4,289 on the consistent series (navigation at `bc6c727c48`); #636 alone −1,952. Rows are in the thirtieth steering commit `7c4a448327`. |
| 89 | Native fatal reporting #709, repair #711 | `6426473252` | Production-path fatal probe and unrepaired-binary negative control; original/glue C++ unchanged. |
| 90 | Typing, TSan startup and network diagnostics #705 | `8fc7620ce5` | All-tools typing and precise one-use network allowances; platform workflow passed; C++ unchanged. |
| 91 | Accounting, housekeeping, identity and fact tools #712 | `561ec04c42` | Versioned census, ordinary cleanup and compatible profile guard; no ownership-retirement credit. |
| 92 | Map leaves #698 and harness consolidation #713, combined #715 | `f5f16377ce` | Historical C++ -178; total fork C++ -247; callback slots -57; original retirement unchanged. |
| 93 | Shared Rust pool core #716/#718 | `d5cb6a424e` | Coupled support; Rust +682, generated C++ +20; no production pool or retirement claim. |
| 94 | Content metadata/dependency/download state #706/#667 | `36a3681ee0` | Exact-head Full/tree match; combined 93–94 source C++ -431, total fork -214, original retirement +676. Seven profile comparisons equal, all projections below bands. |
| 95 | Shared native pool facade and first vehicle allocation #720 | `5391a8ee49` | Both source reviews and exact-head Full/tree match; eleven local games/64 snapshots and seven affected native tests pass. Measured +181 C++ coupled support, no full-vehicle or generic deletion credit. Platform cadence passed. |
| 96 | Map-reader maintenance #275/#721 | `01dcc7419f` | Both source reviews and exact-head Full/tree match. PR Grok Ir -3.966178% versus retained 92; Rust +77 added/net +27, C++/tooling zero. Combined 95–96 source C++ +181, fork +184; profile 96 passes seven comparisons and all six projections below bands. |

## Work order

The retrospective repairs (#709, #705's findings, #707, #708, #710's tools) integrated as 89 to 91. Root resumed in Codex at 08:28 on 2026-11-06. The #710 host investigation is complete. Its only user choice is T3's "Auto-settle on merge" setting; root identifies #724's kill sender itself.

There is one ranked list. Each free slot takes the highest unblocked item that fits the six-branch limit and the disjoint-service rule. Finishing review and integration of an open branch comes first, and a lower rank can run while a higher one waits. Scopes live in issues; per-PR changes live in each PR's steering comment.

0. **Development is active through integration 96.** Census and profile 96 are recorded above. Apply steering 31's actions first.
1. **Bank converging owners, then finish them** ([integration](workflow.md#integration)). The [resume checkpoint](#resume-checkpoint-2026-11-06-development-through-integration-96) holds heads and next actions.
   - Integrate a checkpoint at -100 or better once it meets the integration rule, and continue the owner in a fresh PR on its issue.
   - #719 comes first, after its two PR-comment fixes.
   - #722 follows at its next passing checkpoint.
   - #704 takes its cost decision before review.

   Within an owner, field moves and service deletions come before pool adoption ([storage ports](porting.md#storage-ports)). Integrated #718's core and #720's facade are the single shared pool definition, and owners add no duplicate primitives. Children inspect `.local/deep.lock.holders` before native work, use `--jobs 3`, and start long jobs with `exec` ([team](team.md#spawning)). Focused checks and diagnostic subsets address concrete failures. No universal native profile precedes review and Full.
2. **Maintenance and measurement.**
   - **#726, steering-31 batch.** It covers the env export, #706's shared constants and TCP receive tail, the signal helper copies and the debug-only `add_by_diag_dir` service. Add #156's script-sweep high-bit repair when ready. Use one PR, one reviewer and one Full.
   - **#156.** Establish the nonempty network execution-queue seam before choosing its observation. Threaded cancellation stays with the socket/resource owner. Steering 31's gaps are on its list.
   - **#275.** Forward maintenance for the integration-92 Grok rise is complete in #721. Whole-loop savings are credited once.
   - **#565.** #698's map-leaf checkpoint integrated in 92. Its town synthetic-slope fixture and seven services stay with #635. Train, ship and reservation consumers belong to combined #656, and the remaining map bodies and allocation belong to the map owner.
   - **#500 and #168/#199** attach to the owner that touches those records. Completed-job publication stays with #717.
   - **#693** stays unproved, queued for a selected evidence/tools batch that keeps a loaded-base or original/original reproduction. Add no blind retry, mask or synchronization change.
   - **#686** is resolved: the subagent caps are lifted and Codex has 12 slots.
3. **Typing (#501) and harness consolidation (#673) are complete** in 90 and 92. No broader harness rewrite is selected.
4. **Ranked phase-2 queue (#570 integrated as 82).** Placement is decided: later records sit **beside** their shells ([world-state design](design/world-state.md#later-pool-record-placement-634-635-decided-2026-11-05-after-integration-82)). Briefs state its two conditions: no out-of-line record-to-shell call, and no stored references. Cargo-packet co-allocation is decided at #634's pool-completion checkpoint.

   | Rank | Owner (PR) | Issue estimate | Measured at head | Remaining bound (stocktake 96) | Coupled scope / readiness |
   | ---: | --- | ---: | ---: | --- | --- |
   | 1 | Combined #656/#598/#599 vehicle (#714) | 750 to 1,250 | -148 | 150 to 300 for the named next groups. The bound omits #714's Rust-to-C++ field routes such as `refit_cap` | Native Crash/pathfinding/lifecycle/position/unit-number policy, mixed maintenance and six-controller direct dispatch, with Consist records and all consumers together. Preserve timing, scratch, survival, Money/date/RNG behavior. Its base predates #720; recount after the next base update. |
   | 2 | #637/#497 engine/renewal (#722) | 400 to 750 | -255 | 180 to 350 | Delete the already-moved EngineInfo/payload readers and carriers, with sorted GRF bindings, ordered overrides/nested IDs/badges and the actual pools. Preserve monostate/wrong-variant failure and ENGN nonpooled/move behavior. Integrate the next passing checkpoint. |
   | 3 | #634 station/road-stop/cargo (#700) | 800 to 1,400 | -168 | 100 to 250 | Names, cache and spec-reference vectors, native field routes, and Station/RoadStop/CargoPacket/CargoPayment policy. Keep a single StationGoodsData allocation and lifetime closure. Definition payloads stay with #669. CargoPacket must not use the identity scan. |
   | 4 | #638/#498 small pools/ObjectSpec (#723) | 300 to 550 | -45 | 150 to 300 | Depot/Sign routes, objective records, seven more pools and ObjectSpec defaults/parser/publication. Only `Sign::sign` remains phase 5. |
   | 5 | #635 town/industry/subsidy (#704) | 500 to 1,000 | +64 | 50 to 150 | First, the cost decision: delete the services, carriers and adapters the moved records displaced (`industry_ffi.h`, `industry_adapter.hpp`). Preserve owned versus borrowed Industry lifetimes and the live-ID guard. |
   | 6 | #717 graph/jobs (#719) | 450 to 800 (unsourced, set before the placement decision) | -112 | 20 to 60, the #700 attachment step | Fix the orphaned copy export and the opcode dispatch, then integrate. The attachment step stays on #717, behind #700's StationGoodsData closure. |
   | 7 | #725 YAPF remainder | 450 to 650 provisional | - | - | 683 original lines are kept alive by the rail desync dump and native fixtures. Ready once no open draft touches `src/pathfinder/yapf/`. |
   | 8 | Orders/order lists/backups | 350 to 650 provisional | - | - | Select after the vehicle owner's BaseConsist changes integrate. Include current fields, actual pools and field-service round trips. |
   | 9 | Companies/groups | 300 to 600 provisional | - | - | Select after vehicle unit-number ownership settles. Include settings, finance/statistics aliases, names/identity and remaining Rust-used state. |
   | 10 | Map storage and remaining map bodies | 2,000 to 3,200 provisional | - | - | Requires every actual pool owner complete, including the network-client/admin/SpriteGroup policy below, plus #565 item 2. Own map allocation/reset and the remaining tile bodies. |

   "Measured" is net compiled C++ against each draft's merge base, with the metric at implementation `f4e2c5b45420…` (steering 31). Remaining bounds are notes from the [stocktake-96 assessment](https://github.com/fetch-laboratories/openttd-rust/issues/3#issuecomment-6688072876), not replacements for the estimates. A large shortfall gets the [cost decision](planning.md#cost-decisions), not a new estimate. Do not expand a finished scope or delete independent evidence to hit a number. Effort ranks do not serialize disjoint work or override readiness-ordered integration. Every owner includes allocation and lifetime, exact shell dispositions, coupled save adapters and displaced field routes. No new phase-1 owner is selected.

   **One shared helper.** #704's Industry adoption promotes the tested shell-identity `BindRecordIfOwned` into the existing `pool.hpp`, with no parallel API and no uninitialized-index read. Pooled production shells bind and retire by index ([world-state](design/world-state.md#shared-pool-policy-716)). #719, #722 and #723 reconcile with that one definition at their necessary updates. StationGoodsData is a real record, container and lifetime prerequisite across #700's consumers; do not extract only its names or create a second layout.

5. **Phase 3 fills slots phase 2 leaves free.** After earlier-phase ready work and
   branch capacity permit, use this accepted reader order:

   | Rank | Owner | Net C++ estimate | Selection event / closure |
   | ---: | --- | ---: | --- |
   | 1 | #669 afterload/legacy | 1,800 to 2,600 | Conditional: establish a complete canonical-write route using existing owner operations before selection. No blanket wait for every pool or all #573, and no replacement setter layer. Preserve malformed/historical/partial-load evidence. |
   | 2 | #669 network core/discovery/resource | 700 to 1,100 | Ready integrated #628/#574 interfaces. Include TCP queue/partial receive, command destruction, authorization/discovery and NetworkClientInfoPool/NetworkClientSocketPool policy; observe nonempty queue copy and resource disposition. |
   | 3 | #669 GRF configuration/common publication/actions | 500 to 800 for bounded delivery | Ready #655/#628 interfaces. Own parameters/Action14/configuration/publication and remaining SpriteGroupPool lifetime. Exclude #637/#638 payload credit; preserve partial parser/reset order. |
   | 4 | #573 continuation | 350 to 550 | Ready with descriptor macro syntax preserved while record owners adapt saves. Complete 3b/4, corrupt input/LoadCheck, CopyFromOldName and displaced test wrappers, excluding integrated slices. |
   | 5 | #668 coordinator/STUN/TURN | 400 to 600 | Ready. Own token/negotiation/timeout/fallback/disposition with controlled packets/time and integrated socket/connect interfaces; no unrelated draft-service wait. |

   Admin follows at 350 to 550 net, including NetworkAdminSocketPool policy.
   Chunk families wait on stable descriptor interfaces and their own records,
   not unrelated pool families. Script discovery/configuration/AI/Game remains
   500 to 800 provisional with company coordination. World/economy, vehicle/order
   and story/text/event groups retain their actual record dependencies on #669.
   HouseSpec's old #628 prerequisite is clear; remaining grouped definitions and
   SpriteGroup lifetime stay explicit. #653 completed in 88; #622 is an integrated
   interface. #666 infrastructure APIs and #667 content protocol are complete.

   **All-pools closure before map:** NetworkClientInfoPool and client socket policy
   belong to #669 network core; admin socket policy to #669 admin; SpriteGroupPool
   to #669 GRF publication/lifetime. They can run beside phase 2 and satisfy this
   prerequisite. Read-only native iterator compatibility remains with its named
   phase-3 readers, phase-4 support and phase-5 GUI consumers until final candidate
   C++ retirement; it does not retain mutable pool authority or earn template
   deletion credit. #716 tracks this closure. Preserve phase-6 VM/binding shims.

6. **#503 coverage continuation last:** first the bounded clock-preparation and sanitized native-test-support build fixes, then the 119 unfinished traces with provenance checks. Do not recollect merely to fill a slot. Deep execution remains user-only.

Recurring:
- After every second integration, take stock under `/start-development` step 4, including the compiled-C++ rate and the baseline sizes. Integration 96's census/profile are complete; the next stocktake/profile follows 98. Profile the merged tip in the background ([speed](speed.md#instruction-projections)).
- Run the platform cadence after every fifth integration; integration 95's cadence passed at `5391a8ee49` ([run](https://github.com/fetch-laboratories/openttd-rust/actions/runs/42225369211)); next after 100. Integration 85's failure was repaired by #702 and its rerun passed. A new failure stops integrations. After two hosted-capacity macOS cancellations, use `CI_MACOS_HEAVY` as documented, and delete it afterwards.
- This host's first quiet pass integrated with #676 (77). Further passes follow the [speed policy](speed.md#policy) triggers.
- Deep checks run only on the user's request (user, 2026-10-31).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions.

Excluded, with return conditions reconsidered on 2026-11-06; neither trigger is met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Resume checkpoint (2026-11-06, development through integration 96)

Root is Codex (`gpt-6.1-sol`, ultra, Fast off), with fresh Sol high independent
reviews. Code is `01dcc7419f`: #721 integrated as 96 after both reviews and
exact-head Full at `c66dab3395`, with the same tree. The one native facade/schema
and actual vehicle allocation support integrated as 95; #718 core is 93 and
#706/#667 completed as 94. No rollback is eligible. `CI_ON_DEMAND` remains true.
The platform cadence passed at the exact 95 code in
[run 42225369211](https://github.com/fetch-laboratories/openttd-rust/actions/runs/42225369211);
the integration hold is clear and the next platform cadence follows 100.

The integration-96 census is complete above: source C++ increased 181, which
triggers the recorded retirement-focused course action and the accepted one-off planning assessment. Profile 96 is accepted with seven equal comparisons and all six projections below bands; no quiet-pass trigger applies. #721 completed the selected forward maintenance for
the earlier Grok rise; the accepted combined profile is recorded above.
The freed branch slot now holds #638/#498 small-pool ownership in #723.
#719's first graph/job pool adoption
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
| #634 / #700 station records | `f0dd14cdd0` | Road-stop/waypoint and reload variants pass; prior `3c851` Cargo/object/controlled-animation cases pass. Actual e716 verify 100/97/241 passed. Spec-reference vector work continues without changing GoodsData offsets. [Status](https://github.com/fetch-laboratories/openttd-rust/pull/700#issuecomment-6680736375). | Complete typed rail/road mapping vectors, remaining names/cache/routes and four pools, keeping source partial-load ordering. Preserve the real attachment boundary for #719; fresh review/Full at readiness. | `t3-085f01f5` |
| #635 / #704 town records | `e2089ea8e7` | Actual IndustryPool adoption promotes the exact tested shell-identity helper; owned/borrowed construction stays distinct. Prior `09cc` verify 102/97/242 passed. New native verify runs; current cost is source C++ +64/fork +79 versus 95, exports -1/slots -26. [Status](https://github.com/fetch-laboratories/openttd-rust/pull/704#issuecomment-6675927407). | Complete new evidence and direct Industry routes/containers. Independent source review of this checkpoint starts now, including native lifetime/evidence cost, live-ID guard and resolver observation. Whole owner still unfinished. | `t3-8cd4fc03` |
| #656/#598/#599 / #714 vehicle owner | `f908eacccc` | Prior d73d Consist/typed-pool delivery passes 98 Rust/candidate 241/eight games/33 snapshots/ABI. Unit/group bitmap storage now Rust-owned in Company members; preflight passes and Rust checks run. [Status](https://github.com/fetch-laboratories/openttd-rust/pull/714#issuecomment-6680888070). | Convert old unit adapters to direct canonical bitmap owners; finish native policy/maintenance/lifecycle/direct Tick. Preserve Company copy/move/drop, source sentinel/no-claim, settings/statistics and reservation timing. | `t3-a4c94f5a` |
| #717 / #719 link graph/jobs | `f0f67cf819` | Source review, 102 Rust/97+242 native tests and nine games/77 snapshots pass; each reload retains 47 scheduler observations. Completed census: source/fork C++ -112, exports -13, slots -16 versus 95; native fixture/generated/helper costs included. [Status](https://github.com/fetch-laboratories/openttd-rust/pull/719#issuecomment-6683281680). | Finish one #700 attachment/publication remainder using existing station World/Links, live distribution settings and canonical pool descriptors. Preserve join/apply/retire/drop and noninvalidating reroute lifetimes. Root promotes coherent shared support when needed; no duplicate helper. | `t3-cf46f29b` |
| #637/#497 / #722 engine/renewal | `8ecca3675f` | Canonical record handles/direct Action0 and spec consumers retire 80 slots/seven carriers/66 native functions; six rebound rows removed. Rust/ABI/preflight pass, source cost -255 includes generated records. Native verify runs. [Status](https://github.com/fetch-laboratories/openttd-rust/pull/722#issuecomment-6686237802). | Review this fixed checkpoint while native evidence runs; complete sorted GRF bindings/ordered overrides/nested IDs/badges/routes/pools. Preserve active tag/wrong-variant, moves and partial publication. | `t3-c376a5e0` |
| #638/#498 / #723 small pools/ObjectSpec | `4ac82e0245` | Prior `42af` verify 97/242 and signs/town focused 2/2 with five snapshots pass; source -43/fork -47 accepted as initial cost. Depot/Sign non-drawing policy moved to Rust; objective-pool/encoded-view work continues. [Status](https://github.com/fetch-laboratories/openttd-rust/pull/723#issuecomment-6687670929). | Finish objective records, lifetime/field routes and remaining pools/ObjectSpec. Native input Span deduplicates Town/Industry helpers; reconcile one shared hunk with #704 and preserve partial save writes. | `t3-c19acd32` |

Preserve the following:
- the active open-PR checkouts above, each while its PR is open; old paused copies remain subject to ordinary clean/pushed-head housekeeping;
- `t3-8ed2642d`, completed integration-96 profile/preparation and cross-tip state/log evidence;
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

Release after the checks in steering 31 action 5. Each item stays protected while it is listed here; remove its line when you remove the tree:
- the superseded profile trees `t3-44649657` (92) and `t3-b7a95131` (94), and the steering-30 audit tree `t3-96affac3`. Profile 96 is accepted on #275;
- the merged-PR evidence trees `t3-e897547d` (#706), `t3-57a288c6` (#720) and the #721 measurement tree `t3-44e4f3b0`, once their PRs hold the evidence;
- the old record trees `t3-19613657` (`town-635-records`) and `t3-98010525` (`station-634-records`). Push any unpushed head and record it on its issue first.

The wind-down's housekeeping removed only three verification runs of integrated commits. It kept two runs without a report, `.local/simulation/20261105T110506Z-908834` (770 MB) and `.local/verification/20261105T110302.850651Z`, for review on resume.

**Disk (2026-11-06, steering 31):** after the morning's manual cleanup left 78 GB free, the SSD was back to 29 GB free (87% used) by 16:30. T3 worktrees hold 69 GB, and builds stay on the SSD. The release list above frees about 25 GB. `.local/reference/openttd` and the active checkouts stay.
