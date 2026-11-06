# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-06, paused after integration 102)

Agent: /root | Model: gpt-6.1-sol | Reasoning effort: ultra

Development is paused at the user's request for a strategic overhaul and cleanup campaign. The checkpoint below records saved work; the separate overhaul thread will supply the new work order. No new stocktake, profile, review, repair round, build, simulation or CI request was started during wind-down.

#722 integrated as **101**, merge `31478f303ab5e3787989fb1b7ceb1c48c30b992f`, tree `b5cd1e99b02358e700cde5ef3f34963718f0cf1a`. Fresh source/fix reviews and exact-head Full [42302553842](https://github.com/fetch-laboratories/openttd-rust/actions/runs/42302553842) passed. Its finished ledger removes 852 compiled C++ lines and 973 total fork C++ lines; original retirement rises 380 and net glue falls 472. Actual Engine/Renew pools and named native consumers remain unfinished on #637/#497. [Acceptance and charged final-edit proofs](https://github.com/fetch-laboratories/openttd-rust/pull/722#issuecomment-6694392015) retain the raw census endpoint rather than claiming a second measurement.

#732 integrated as **102** during the authorized wind-down, merge `a0337694c4537381dcf00a404d79715cf51a5a5e`, tree `4c81b59630bc83c73fd42657b5d37a96f814695b`. Reviewed head `7f2225c5cfab1db63f15f7ecbcbda26a57636247` had no scoped blockers in review [5808055307](https://github.com/fetch-laboratories/openttd-rust/pull/732#pullrequestreview-5808055307), exact-head Full [42306952222](https://github.com/fetch-laboratories/openttd-rust/actions/runs/42306952222) and Validated tree passed, and its base was current `6fd364ffdb`. The merged tree equals that reviewed tree. Its completed census adds 2 compiled C++ lines and 4 total fork C++ lines for singular owned-byte assignment and Shared GRF price support. This is necessary support, with no ownership-completion, retirement or speed credit.

The finished metric-v2 records use implementation `f4e2c5b45420ba0cb633a034ab554ca99f08180d746be589e472fd00b553e396` and unchanged original `14ec60f248547d4d062a1160f0fc26d742319888`. No new measurement was made during wind-down.

| Measure | After 101 | After 102 | 102 change |
| --- | ---: | ---: | ---: |
| Compiled source and generated records C++ | 256,794 | 256,796 | +2 |
| Total fork C++ including evidence | 279,751 | 279,755 | +4 |
| Candidate upper inventory | 279,638 | 279,642 | +4 |
| Original C++ retired | 62,750 | 62,750 | 0 |
| Net glue C++ | 35,189 | 35,191 | +2 |
| Counted Rust | 141,676 | 141,722 | +46 |
| Tooling | 61,317 | 61,319 | +2 |
| C++ evidence | 22,957 | 22,959 | +2 |
| Boundary exports / callback slots | 1,563 / 3,377 | 1,563 / 3,377 | 0 / 0 |

Original retirement is 62,750/284,355 (22.07%); simulation retirement is 41,550/61,148 (67.95%). Raw records, exact provenance, reviews, failed outputs and merge decisions remain in main `.local/integration-records/engine722/`, `101/` and `102/`. The last full stocktake remains [100](https://github.com/fetch-laboratories/openttd-rust/issues/3#issuecomment-6694740716); no stocktake or profile 102 was started.

Main is green at the saved code tip. Platform recovery [42304912050](https://github.com/fetch-laboratories/openttd-rust/actions/runs/42304912050) passed on attempt 2 for integration 101. Attempt 1 failed dependency downloads before compilation; it was a runner failure, not a confirmed code regression. No rollback is eligible, no rollback was performed, and no integration hold remains. Integration 102's post-merge Quick [42310883006](https://github.com/fetch-laboratories/openttd-rust/actions/runs/42310883006), baseline validation [42310883064](https://github.com/fetch-laboratories/openttd-rust/actions/runs/42310883064) and CI Build [42310883061](https://github.com/fetch-laboratories/openttd-rust/actions/runs/42310883061) passed. No post-merge CI wait was started.

[Profile 100 remains accepted](https://github.com/fetch-laboratories/openttd-rust/issues/275#issuecomment-6695636889) at clean `dbbb502073d3b6c1280cb17832baeb5a85a5038a`: seven canonical and seven retained-98 comparisons passed full state, normalized logs, byte stdout, end moments and drawing. The `27de6f8c18` anchors, identities and frozen runtimes remain unchanged. These are combined 99/100 instruction results, with no isolated PR cost or wall-speed claim. All accepted projections remain below their bands with no rise or quiet-pass trigger. The table is historical profile-100 evidence, not a measurement of 101/102.

| Scenario | Original Ir | Candidate Ir (100) | Accepted projection (100) |
| --- | ---: | ---: | ---: |
| TGP | 2,265,528,237 | 2,782,269,348 | 1.167346x |
| Opus | 21,773,343,583 | 27,916,561,893 | 1.345115x |
| Grok | 20,969,727,163 | 26,626,873,969 | 1.346652x |
| P1996 | 3,600,502,896 | 4,593,990,046 | 1.352193x |
| P2000 | 6,049,922,238 | 7,583,890,696 | 1.444531x |
| P2016 | 11,343,424,651 | 13,931,397,338 | 1.382593x |
| Drawing | 2,149,015,961 | 2,154,521,531 | 1.004635x |

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
   - The merged #706/#720/#721 evidence is recorded on each PR; release their completed evidence trees through ordinary housekeeping checks.
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
| 97 | Host signal witness #724/#727 | `4cc6ee4b20` | Independent source review and exact-head Full/tree match. Four signal/process tests; future long jobs use the integrated wrapper. C++ and Rust unchanged; no live sender identified yet. |
| 98 | Graph/job indexed pool checkpoint #719 | `9867694d02` | Both steering fixes/source review/focused/native and exact Full/tree pass. Source C++-153, fork-151; actual station publication remains717. |
| 99 | Shared nearby StationList #728 | `8faa77f296` | Source review/exact Full/tree pass. Source C+++6, evidence+1; necessary shared support, no completion/retirement credit. |
| 100 | Const nearby StationList views #729 | `dbbb502073` | Two qualifiers, same descriptor/lifetime. Source review/exact Full/tree pass; counters unchanged. Profile100 accepted; cadence initializer repair integrates in101. |
| 101 | Engine records/GRF/direct consumers #722 | `31478f303a` | Scoped reviews and exact Full42302553842/tree pass. Compiled C++−852, fork−973, original retired+380/net glue−472; callback slots−229. Actual pools/native consumers remain637/497. Main platform recovery42304912050 runs. |

## Work order

The retrospective repairs (#709, #705's findings, #707, #708, #710's tools) integrated as 89 to 91. Root resumed in Codex at 08:28 on 2026-11-06. The #710 host investigation is complete. Its only user choice is T3's "Auto-settle on merge" setting; root identifies #724's kill sender itself.

There is one ranked list. Each free slot takes the highest unblocked item that fits the six-branch limit and the disjoint-service rule. Finishing review and integration of an open branch comes first, and a lower rank can run while a higher one waits. Scopes live in issues; per-PR changes live in each PR's steering comment.

0. **Development is active through integration 101.** Stocktake100 and accepted profile100 are recorded above; Engine101 contains the reviewed cadence initializer repair, with main platform recovery running. Apply steering 31's actions first.
1. **Bank converging owners, then finish them** ([integration](workflow.md#integration)). The [resume checkpoint](#resume-checkpoint-2026-11-06-development-through-integration-101) holds heads and next actions.
   - Integrate a checkpoint at -100 or better once it meets the integration rule, and continue the owner in a fresh PR on its issue.
   - #719 banked its reviewed checkpoint in98; the actual publication remainder stays on717.
   - #722 is banked as101 with resolved findings and exact Full/tree match, measured -852; actual pools and named native consumers remain a fresh637/497 PR after the selected shared-bytes support.
   - #704 takes its cost decision before review.

   Within an owner, field moves and service deletions come before pool adoption ([storage ports](porting.md#storage-ports)). Integrated #718's core and #720's facade are the single shared pool definition, and owners add no duplicate primitives. Children inspect `.local/deep.lock.holders` before native work, use `--jobs 3`, and start long jobs with `exec` ([team](team.md#spawning)). Focused checks and diagnostic subsets address concrete failures. No universal native profile precedes review and Full.
2. **Maintenance and measurement.**
   - **Shared StationList support (#728/#729), complete in99/100.** One generated row/NativeVec descriptor and const inline readers serve634/635. Their attachment placement remains owner work; no parallel descriptor or repeated credit. The freed slot selects730.
   - **#726, steering-31 batch.** It covers the env export, #706's shared constants, signal helper copies and debug-only `add_by_diag_dir`. Its TCP receive/send tail is assigned once to #730 and excluded from this batch. Add #156's script-sweep high-bit repair when ready. Use one PR, one reviewer and one Full.
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
   | 6 | #717 graph/jobs (#719, integrated 98) | 450 to 800 (unsourced, set before the placement decision) | -153 at integrating head69868838a8 | 20 to 60, the #700 attachment step | Both steering fixes and the unused-binder finding are resolved; the reviewed checkpoint passed exact-head Full and integrated. Actual attachment/publication follows in a fresh #717 PR after #700's StationGoodsData closure. |
   | 7 | #725 YAPF remainder | 450 to 650 provisional | - | - | 683 original lines are kept alive by the rail desync dump and native fixtures. Ready once no open draft touches `src/pathfinder/yapf/`. |
   | 8 | Orders/order lists/backups | 350 to 650 provisional | - | - | Select after the vehicle owner's BaseConsist changes integrate. Include current fields, actual pools and field-service round trips. |
   | 9 | Companies/groups | 300 to 600 provisional | - | - | Select after vehicle unit-number ownership settles. Include settings, finance/statistics aliases, names/identity and remaining Rust-used state. |
   | 10 | Map storage and remaining map bodies | 2,000 to 3,200 provisional | - | - | Requires every actual pool owner complete, including the network-client/admin/SpriteGroup policy below, plus #565 item 2. Own map allocation/reset and the remaining tile bodies. |

   "Measured" is net compiled C++ against each draft's merge base, with the metric at implementation `f4e2c5b45420…` (steering 31). Remaining bounds are notes from the [stocktake-96 assessment](https://github.com/fetch-laboratories/openttd-rust/issues/3#issuecomment-6688072876), not replacements for the estimates. A large shortfall gets the [cost decision](planning.md#cost-decisions), not a new estimate. Do not expand a finished scope or delete independent evidence to hit a number. Effort ranks do not serialize disjoint work or override readiness-ordered integration. Every owner includes allocation and lifetime, exact shell dispositions, coupled save adapters and displaced field routes. No new phase-1 owner is selected.

   **One shared policy.** Pooled production shells bind and retire by index ([world-state](design/world-state.md#shared-pool-policy-716)). #719 removed the unused fixture-driven binder in 98; #704 carries the exact deletion, its borrowed constructor skips publication and its pooled constructor uses the index. An actual nonpooled production need must establish its publication point before adding support, with no parallel API or uninitialized-index read. #719, #722 and #723 reconcile with that one definition at their necessary updates. StationGoodsData is a real record, container and lifetime prerequisite across #700's consumers; do not extract only its names or create a second layout.

5. **Phase 3 fills slots phase 2 leaves free.** After earlier-phase ready work and
   branch capacity permit, use this accepted reader order:

   | Rank | Owner | Net C++ estimate | Selection event / closure |
   | ---: | --- | ---: | --- |
   | 1 | #669 afterload/legacy | 1,800 to 2,600 | Conditional: establish a complete canonical-write route using existing owner operations before selection. No blanket wait for every pool or all #573, and no replacement setter layer. Preserve malformed/historical/partial-load evidence. |
   | 2 | #669 network core/discovery/resource | 700 to 1,100 | Selected as #730 from integrated #628/#574 interfaces. Own both client pools and TCP/packet/command/resource lifetime; establish nonempty queue/resource observations. First coherent checkpoint110–240 (central170); original700–1,100 remains. |
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
- After every second integration, take stock under `/start-development` step 4, including the compiled-C++ rate and the baseline sizes. Stocktake100 and profile100 are complete and accepted; the next stocktake/profile follows102. Profile the merged tip in the background ([speed](speed.md#instruction-projections)).
- Run the platform cadence after every fifth integration; integration 95's cadence passed at `5391a8ee49` ([run](https://github.com/fetch-laboratories/openttd-rust/actions/runs/42225369211)); cadence100's actual builds/tests passed but annotations failed; repair validation is pending, then next after105. Integration 85's failure was repaired by #702 and its rerun passed. A new failure stops integrations. After two hosted-capacity macOS cancellations, use `CI_MACOS_HEAVY` as documented, and delete it afterwards.
- This host's first quiet pass integrated with #676 (77). Further passes follow the [speed policy](speed.md#policy) triggers.
- Deep checks run only on the user's request (user, 2026-10-31).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions.

Excluded, with return conditions reconsidered on 2026-11-06; neither trigger is met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Resume checkpoint (2026-11-06, paused after integration 102)

Paused at the user's request. Root is Codex (`gpt-6.1-sol`, ultra). Code main is `a0337694c4537381dcf00a404d79715cf51a5a5e`, tree `4c81b59630bc83c73fd42657b5d37a96f814695b`; this docs-only checkpoint follows it. The separate strategic-overhaul thread owns the new work order. The earlier work-order and recurring sections above are preserved unchanged; they do not authorize continued work during this pause. Later CI or job notifications do not restart development.

All four source delegates were canceled after saving their work. Root's three merge helpers and two trace/sequence helpers are complete with no edits pending. Every active draft has one retained checkout. Their working trees are clean, including the locally ahead ObjectSpec checkout. The user's uncommitted `.codex/config.toml` is unchanged and excluded from this commit. No VehicleServices.engine / Base.Engine rewiring edit was started.

| Draft PR | Saved head and checkout | Validation and CI at checkpoint | Recorded next action, subject to the new work order |
| --- | --- | --- | --- |
| [#700 station](https://github.com/fetch-laboratories/openttd-rust/pull/700) | Pushed/local `0256b0bbdb8016089dc1c5b05a88982ea24f513b`; `t3-085f01f5` | Existing job 6635 finished its Cargo checks and both builds; all 12 focused cases passed with 70 equal snapshots. No job process remains. Its final wrapper status and the planned 23 selected native-test result are unverified. Quick and Commit checker passed; Full pending. | Recover the remaining exact-head native result and review the saved station checkpoint. Foreign Industry/Town/PSA and virtual-dispatch seams remain open. |
| [#704 town/industry](https://github.com/fetch-laboratories/openttd-rust/pull/704) | Pushed/local `60757916b4448c1a52932de6a246cc64ad732441`; `t3-8cd4fc03` | Saved 15-file unfinished edit. Preflight passed; existing short job 1376 failed Clippy on unused `Commands::industry` and two while-let loops. Removed `OpenTTDIndustryTown` still has native news callers; native validation is absent for this head. Quick/Commit checker passed, Full pending; those checks do not clear the known local failures. No live job. | Finish the bounded foreign-consumer edit and its known errors before a fresh review. The steering-31 +64 compiled-C++ decision at `e2089ea8e7` remains recorded separately from the older measured +36 and original 500–1,000 estimate. |
| [#714 vehicle](https://github.com/fetch-laboratories/openttd-rust/pull/714) | Pushed/local `20c5f1bb99e2a8a23790d065b8a761af3997b093`; `.local/worktrees/root-review-714` | Exact-head Cargo job 4483 passed fmt/check/Clippy and 106 tests. Native/runtime validation is pending. Quick/Commit checker passed, Full pending. The latest mergeability snapshot is conflicting after 102; no base update was attempted. No live job. | Native/runtime validation of the pushed head, with the base conflict recorded for reconciliation. A1/A2 and the engine field route stay open; no new repair round began. |
| [#723 small pools/ObjectSpec](https://github.com/fetch-laboratories/openttd-rust/pull/723) | Local `2ec78fb676d12faa10b74c503dc7dea8fb64f9a9`; pushed `a26709dc2c26b2011438ff8a7d629824e6d1f3fa`; `t3-c19acd32` | Local inventory/base/fixture/source commits are preserved unpushed. Pushed a267 passed Cargo, both builds and 97 reference / 244 candidate CTests. Seven existing Object cases passed 19 snapshots. Definition retry 66028 exited 1 on unnamed-class 0 versus original STR_EMPTY 1; local 2ec fixes allocation from actual metadata. Job 90494 was stopped (exit 130): its four Cargo stages passed, native build/scenarios incomplete; coordinator and children confirmed absent. Pushed-head Quick failed, Commit checker passed, Full pending; branch conflicts. | Adopt integrated Shared.grf_price_shift, retire ObjectServices' local binding, then push. No duplicate-binding bypass or 102 adoption was performed during wind-down. |
| [#731 network](https://github.com/fetch-laboratories/openttd-rust/pull/731) | Pushed/local `655d8f8990d38930689c38506777ca9a6ca2df62`; `t3-eb96953a` | Root preserved 18 dirty files from the interrupted Game/client/command lifetime edit in this unvalidated draft commit. Ordinary push preflight passed; no Cargo/native validation of 655. Quick failed, Commit checker passed, Full pending. Prior 595 passed Cargo/native 97/243; loopback job 85545 exited 0 at prior `89aa2a1666ed4b6d44ba644af681c76410cea7d1`. No live job. | Finish the saved client-pool and command-queue lifetime work; adopt 102's shared helper at the next necessary base update. |

The table records CI snapshots, not a fresh validation request. All five PRs remain drafts with Full pending; none is accepted by this checkpoint. Root watches were removed. #722 and #732 are merged, source-reviewed and exact-head Full validated; no work remains on those PRs. The completed #719/#728/#729 remain integrated as 98/99/100.

Prior evidence stays scoped to its head: #704 job 71619 passed ownership-dispatch (one snapshot, 124 chunks, 2,294 elements) and all eight pending Town/Industry/Cargo/ABI CTests at `417f6f4689`, completing eight focused cases/31 snapshots across original and recovery. Its same-tick Industry deletion observation independently saw closing IDs 0/2 removed and successors 1/3 advance to history masks 2/2 in both games. Neither result validates unfinished 607. #714's old `6f3c5b5d8b` retains 19 cases/73 snapshots and byte-identical drawing/Ir evidence, without validating 20c's subsequent edits. ObjectSpec's original-side count assertion was corrected only in the new fixture: saved mappings reserve IDs 5–9, so actual publication through 12 produces size 13. The existing original/input/oracle remained unchanged.

Existing root jobs 33288 (finished census), 4483 (Cargo) and 86612 (platform watch) completed successfully. Station's existing simulation driver report `.local/verification/20261106T225442.926486Z/report.json` and game report `.local/simulation/20261106T231444Z-1395222/report.json` now pass at 0256; its earlier Cargo report is `.local/verification/20261106T225326.786663Z/report.json`. The planned native result remains unknown; absence of a process is not a pass. No identified validation process remains in the five retained PR checkouts. ObjectSpec 90494 is stopped, not passed. No CI wait remains. Automatic checks from this docs checkpoint may be pending or unknown when root stops; do not wait on them.

Cleanup removed 22 clean, unlocked, inactive worktrees using plain `git worktree remove`, after checking live processes and pushed/merged state. The merged review trees removed were `t3-1271d206`, `t3-1746136e`, `t3-2d07d8f5`, `t3-366ac40b`, `t3-62f6417e`, `t3-725cd3c1`, `t3-8287a3fe`, `t3-85887512`, `t3-98f13860`, `t3-bb7b109b`, `t3-d8457d82`, `t3-e60b216a`, `t3-fb6deab4` and `root-review-719`. Older pushed copies removed were `t3-3bee912b`, `t3-96c71e86`, `t3-a2c01c93`, `t3-a4c94f5a`, `t3-98010525` and `t3-19613657`. `root-review-704` was removed after verifying its a2414acc patch was already applied as b0f7df42 in #704. `root-shared-bytes-grf` was removed after integration 102. No dirty, locked, profile or release-list tree was removed.

Retain the following unchanged:

- The five PR checkouts in the table; main; and the original `.local/reference/openttd` at `14ec60f248547d4d062a1160f0fc26d742319888`.
- `t3-8ed2642d` (profile/preparation 96), `t3-62945724` (accepted 98) and `t3-a6181dc7` (accepted 100), with full comparisons and frozen runtimes.
- Release-list trees `t3-44649657` (profile 92), `t3-b7a95131` (profile 94) and `t3-96affac3` (steering-30 audit); leave their disposition to the overhaul.
- Dirty/staged trees `t3-3b491ddb` (five files), `t3-431c84b4` (locked #565 work), `t3-d78d5f8b` (locked native evidence), `t3-32cf4edc` (`tools/simulation/roads.py` staged) and `t3-e130d5a3` (`migration/saves/README.md` staged).
- Protected `.local/worktrees/root-docs-72` and the overhaul's locked `.local/worktrees/overhaul`; never push stale root-docs guidance over current main.
- Locked legacy profile trees `.local/worktrees/profile-after18-544`, `profile-after20-545`, `profile-after26-558`, `profile-after36-595`, `profile-after38-576`, `profile-after40-594`, `profile-after42-605`, `profile-after44-609`, `profile-after46-606` and `profile-after68-664`.
- Main `.local/integration-records/`, `.local/speed-pass/` anchors/frozen runtimes, and the old vehicle owner's raw 6f3 evidence through `.local/worktrees/root-review-714/.local/historical-owner-6f3-evidence`.
- Main `.local/simulation/20261102T135838Z-2198633` (64), `20261105T091320Z-509137` (anchor), `20261105T185141Z-1237553` (80), `20261105T215010Z-1842682` (82), `20261106T010514Z-2542270` (85), `20261106T080938Z-3339040` (91) and `20261106T004411Z-2474065` (base TGP).
- Incomplete evidence `.local/simulation/20261105T110506Z-908834` and `.local/verification/20261105T110302.850651Z`; neither was removed.

Exact heads, cleanup decisions, process/CI snapshots and the retained-worktree list are saved under main `.local/winddown-20261106-*`. No disk stocktake was taken for this pause. Host #724's previously recorded sender limitation remains unchanged; no new host investigation occurred. This is the single wind-down roadmap checkpoint. Further docs changes and work selection belong to the overhaul after root stops.
