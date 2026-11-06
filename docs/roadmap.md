# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and the docs in its routing table define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-11-06, stocktake 88)

Agent: /root | Model: claude-opus-5-5 | Reasoning effort: xhigh.

- Code is `5168023c13` (integration 88), green on push: CI - Build, Quick validation and Migration baseline. Since stocktake 86, two integrations landed:
  - #702 (87): `[[maybe_unused]]` on `VehicleRecordAssertNext`, with no behavior change. The platform cadence rerun at #702 (run 42146030209) passed, which cleared integration 85's red cadence.
  - #703 (88): #692 (#653, industry specs) and #628 (#574, network game protocol), batched. The `abi.rs`/`generate-abi.py` conflict was resolved with the generator's `=> "std::array<...>"` native-type marker. Three network FFI headers gained `#include "../newgrf_badge_type.h"`. Full passed at `aa9c93a4a0`.
- **Paused at the user's request** (wind-down, 2026-11-06), after integration 88. #705's review requested changes, so integration 89 did not happen. The next platform cadence follows integration 90.
- `python3 tools/port-metrics.py --whole-fork --head origin/rust-migration` at `5168023c13`, run in #704's checkout, with changes against stocktake 86 at `ef51e41b80`:
  - Retired: 61,415 of 284,355 original code lines (21.60%, +2,082).
  - Compiled original plus glue: **258,221** (90.81%, **-537**, all from #703).
  - Glue: 35,281 (+1,545). Exports 1,542 (+33). Callback slots 3,636 (+113). #628's network protocol boundary is most of the rise.
  - Rust: 138,236 (+5,941). Tooling: 58,669 (+681).
  - Simulation-source retirement: 41,139 of 61,148 (67.28%).
- Not re-measured at this stocktake: the phase buckets and the rebound-callee, shared-accessor and orphan-export baselines (376 / 120 / 43 at stocktake 86).
- **Speed.** The merged-tip profile for integrations 87 and 88 is owed; it was not started during the wind-down. The latest merged-tip profile is integration 85's (`7fd398da1f`, report `.local/simulation/20261106T010514Z-2542270`), where all 7 scenarios were equal.
  - **#698's TGP:** its re-profile at `79bc4eaa94`, which contains base `5168023c13`, measured 2,791,869,839 Ir, projection 1.1754x. That is 1.3% below integration 85's TGP. There is no integration-88 base profile to compare against yet.

| Scenario | Anchor ratio | Candidate Ir at `7fd398da1f` | Projection |
| --- | ---: | ---: | ---: |
| TGP | 1.160107x | 2,829,532,773 | 1.206883x |
| Opus | 1.345115x | 27,104,173,493 | 1.345115x (Ir -5.3%) |
| Grok | 1.346652x | 25,835,584,034 | 1.346652x (Ir -4.9%) |
| P1996 | 1.352194x | 4,498,538,863 | 1.352193x (Ir -5.2%) |
| P2000 | 1.444531x | 7,427,017,102 | 1.444531x (Ir -5.2%) |
| P2016 | 1.382593x | 13,584,077,535 | 1.382593x (Ir -4.3%) |
| Drawing | 1.004635x | 2,154,532,194 | 1.004635x |

- Coverage: unchanged. No coverage run or deep check was performed.
- Unintegrated components, five in all, under the six-branch limit: #705 (tools maintenance), #700 (#634), #704 (#635), #698 (#565 item 2) and #706 (#667). The [resume checkpoint](#resume-checkpoint-2026-11-06-stocktake-88) gives each one's head and next action.
- `ty`: #705 converts `tools/test-generated-abi.py`, the last #501 file, and sets `ty.toml` to all of `tools/`; its review passed `ty` over all 111 files. Until #705 merges, that file stays on the `ANN` ignore list.
- Issue placement: 34 open, zero unplaced. On #686, the user has raised Codex's per-session subagent limit to 12 (see the checkpoint).
- **Process lessons:**
  - **Base before Full:** a Full tests the PR merged with its own merge base. #703's first Full tested `54e55a8e6c`, so it re-ran. Update a branch to current base immediately before requesting Full.
  - **Orphan audits:** #700's batch 4 deleted two wrappers that callers in their own defining header still used; the file-mention count missed them. Count callers inside the defining header too, and build before committing a deletion batch.
  - **Housekeeping:** `housekeeping.py` keeps worktrees named in the roadmap, but not `.local/simulation` runs. `--apply` would have removed five profiles this checkpoint retains. Until the tool honours references for runs, remove runs by hand.
  - **Disk:** housekeeping's 12-hour activity window kept 33 merged, clean worktrees, about 90 GB of build output, until the SSD filled. At each stocktake, delete merged, clean worktrees whatever their recent activity.

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

## Twenty-ninth steering review (2026-11-05: new host and the test harness)

Agent: /root (Claude Code steering review) | Model: claude-opus-5-5 | Reasoning effort: xhigh.

**What was audited.** Four independent read-only audits by Claude Opus at high effort, with no builds or games, because this host had no compiler:
- the Python harness's duplication;
- its type annotations;
- its structure and growth;
- #649's fidelity.

The steering checked every claim acted on below against the source.

**Root's track.**
- **Root.** One Codex session ran on 11-03 from 04:52Z to 06:25Z, then wound down at the user's request. It integrated #665 (68) and the cadence passed.
- **User-directed sessions.** #660 (66) and #662 (67) integrated from user-directed Claude sessions. All three integrations were infrastructure, so retired original and compiled C++ are unchanged since integration 65.
- **The move.** On 11-05 the user moved the project to a new host and removed 36 finished worktrees by hand.

**Findings and corrections.**

1. **The new host cannot build.**
   - It runs Ubuntu 24.04 with no C or C++ compiler, linker, `make`, `ninja` or `valgrind`, and agents cannot run `sudo`.
   - Output built on the old Ubuntu 26.04 host needs glibc 2.43, and the host has 2.39.
   - The driver puts `.local/deps`, which held 26.04 packages, ahead of system paths.

   Steering moved that output (17 GB) to the HDD, and pruned 36 records of worktrees whose folders were gone. Correction: the user installs the apt packages ([migration tools](rust-migration.md#build-and-verification)). The first quiet pass on this host re-anchors every scenario and restarts the three-pass record ([speed](speed.md#instruction-projections)).
2. **The harness is mostly per-component scenario code, not duplication** (the user's question, 2026-11-05).
   - **Size.** `tools/` holds 53,827 code lines:
     - 38,418 physical lines of Python in 103 files: 31 scenario modules hold 19,142 lines, tests of tools 7,438, comparison drivers 2,323, and infrastructure the rest;
     - about 19,700 lines of C++: 13,452 in console adapters relinked into both games, and 5,943 in comparison and ABI fixtures;
     - 1,299 lines of Squirrel.

     Over the whole fork that is 0.96 tooling lines per original line retired. The rate was 3.8 from 10-27 to 10-31, driven by infrastructure, and has been 0.31 since.
   - **Duplication.** 0.71% exact, by jscpd once its 1,000-line file cap is lifted; about 2 to 3% counting renamed copies and the same job written several ways. About 400 to 450 lines are removable mechanically, and 400 to 600 more with judgment. No dead code was confirmed.
   - **Annotations.**
     - 41.7% of functions and 33% of function code are fully annotated; 59 files are ANN-ignored and 38 are in `ty`.
     - New files follow the rule, but about 30% of new functions in ignored files are unannotated.
     - Scenarios, runs and results flow as untyped dicts through an unannotated `core.py`, so `ty` proves less than its pass suggests.
     - At the current pace #501 finishes around March 2027. Its first planned step, `core.py` and `migration.py`, never happened.
   - **Structure.** The code is split by component, with one private reach-in. But it has a 37-module import cycle through `core` and the driver, five second copies of `migration.py`, hooks dispatched to every module through `getattr`, and functions of 268 and 437 lines.
   - **Verdict: no broad refactor.** The size tracks ports, and the six drafts touch 72 `tools/` files. Corrections:
     1. Type checking goes first (user, 2026-11-05): #673's typing PR, with #501's free cleanup, then #501's mechanical batches until the ignore list is empty ([work order](#work-order) item 2).
     2. #672 guards the oracle.
     3. #673's second PR consolidates helpers.
     4. New scenario code uses the shared helpers ([evidence](porting.md#evidence)).
3. **The oracle can weaken silently.** Four gaps:
   - a removed scenario passes Full, because `validate_plan` checks the candidate's own list;
   - a deleted or empty comparison passes;
   - masks can grow;
   - a misspelled scenario key skips a check.

   Correction: #672, batched with #671.
4. **Housekeeping kept 36 finished worktrees.** With the rest, they held 149 GB.
   - Locked trees whose PRs merged are always kept, and so are pushed review checkouts.
   - Wind-down preserved every review checkout.
   - On the 220 GB SSD, with each worktree's `build-rust` on it, this fills the disk.

   Correction: #671. Wind-down now preserves only each open PR's checkout and trees with uncommitted or unpushed work ([wind-down](workflow.md#automatic-rollback-and-wind-down)).
5. **#649 has no reachable divergence.** It integrates as reviewed. Its surviving settings-catalog copies, carriers and small leftovers join #658, and its two coverage gaps are on #156 ([steering comment](https://github.com/fetch-laboratories/openttd-rust/pull/649#issuecomment-6651456306)).
6. **#655 was held behind unstarted #657.** #655 is reviewed, while #657 waits for #647 and #649. Correction: #655 integrates in readiness order. #657 converts the per-message services on base when it is built, #655's included. If #657 lands first, #655 adopts it at its base update.

No user direction is reversed. On 11-05 the user asked to change the project as little as possible while moving it. That bound only that session, and the move is done.

| Twenty-eighth correction | Verdict and cost/result |
| --- | --- |
| 1. Children check their job; root checks idle children at each wake | Applied. No stall was recorded in the 1.5-hour 11-03 session. |
| 2. #628 makes its bounded repair before review | Applied. Compiled C++ is -92 before #657 and about -264 after, against -240 to -360 expected. Its caller-inventory review is unfinished. |
| 3. #657 ports `Debug` and `GrfMsg` once | Slipped. The design is done but nothing is built, behind #647, #649 and the pause. Finding 6 removes #655's wait on it. |
| 4. #658 counts `.inc` glue | Applied inside #649. The +445 re-baseline was verified exactly, with no double count. |
| 5. The platform profile runs before Full for Cargo, toolchain or cmake changes | Applied to #665. The cadence was green at 68. |
| 6. #636 finishes checkpoint 2 near -900 to -1,250 | Unfinished. The worker was cancelled at wind-down, job `23352`'s result is unread, and the head conflicts with base. |
| 7. #655 removes its two bounces | Applied. The ACTIONS and TEXT reviews are clear, and `invalid_engine` is gone at `68bcc7d8d2`. |
| 8. #647's resume round is scoped | Applied. Reviews are clear at `5d809ff395`; Full has been pending since 11-03. |

| Change since the twenty-eighth review | Verdict and next observation |
| --- | --- |
| #660 carried Full validation and #662 Emscripten cache (user-directed, 11-02, answering "CI and merging is a huge bottleneck") | Untested: no integration candidate has run Full since. Watch Fulls per integration and Full wall time. |
| Codex subagent limit raised to eight (user-directed) | Untested. |
| `.local` output on the HDD behind symlinks (user-directed, 11-05) | Applied. Worktree links resolve and the job logs survived. Watch tool code that resolves `.local` paths. |

New since this review, for the next steer to judge:
- Whether the first builds on this host and the first Full from it agree, and what the new anchors measure.
- Whether typing takes the first freed slots: #673's typing PR, then #501's batches, and how fast the ignore list empties.
- Whether #671 and #672 follow, and whether a guard fires on a draft.
- Whether #673 keeps the scenario inventory and digests identical, and how many `ty` diagnostics remain over `tools/`.
- Whether housekeeping after #671 keeps T3 worktrees to open PRs plus unpushed work.

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
recording its cause. Before briefing #634 and #635, root decides record
placement from the post-#570 profile. Per-message `Debug` and `GrfMsg` services are design faults
once #657 integrates. A change to Cargo, the toolchain or cmake runs the
platform profile once before Full. AGENTS.md and its routing table carry the
rules; issue and PR bodies carry current actions and evidence.

## Recent completed work

Rows cover the period since the twenty-seventh steering review; earlier rows are
in their PRs and Git history. Metrics are `tools/port-metrics.py` code lines:
Rust / tooling / original C++ retired / net glue.

| Integration | Issues / PR | Merge commit | Metrics |
| --- | --- | --- | --- |
| 61 | Platform cadence: landscape fixture `shared` initializer / #648 | `cdfc2de482` | 0 / 0 / 0 / 0 |
| 62 | #641 packet state and byte storage in a generated Rust record / #645 | `bf2c4632a2` | 718 / 175 / 52 / +80 |
| 63 | #559 command invocation lifecycle (phase 1's last owner) / #640 | `7940e700c2` | 403 / 287 / 136 / +115; rebound rows 420 to 418, orphan exports 55 to 54 |
| 64 | #64 maintained crypto crates and Rust network authentication / #642 | `5e414c5a64` | 2,027 / 480 / 2,242 / -68 |
| 65 | #575 script host, command context and event lifecycle / #651 | `6afdcbf390` | 386 / 364 / 354 / +220 |
| 66 | #659 carried Full validation across pushes that keep the tested non-Markdown tree / #660 (CI only; the user allowed it ahead of the cadence repair) | `f47064fc57` | 0 / tooling only / 0 / 0 |
| 67 | #661 Emscripten compiler cache / #662 (CI only; the user allowed it ahead of the cadence repair) | `d93afabee9` | 0 / tooling only / 0 / 0 |
| 68 | #664 native-library query from the actual Rust crate / #665 | `e7d12e1508` | 0 / 0 / 0 / 0 (CMake-only; outside the tooling metric) |
| 69 | #604 canonical settings access with #199's disjoint items / #649 | `e76c94b306` | 437 / 118 / 453 / +394; compiled C++ -59, reported apart from the +445 `.inc` re-baseline; callback slots 4,064 to 4,026 |
| 70 | #507 pinned NewGRF sets and arctic corpus scenarios / #670 (the same-NewGRF larger save remains) | `a114e1f8a7` | 0 / tooling only / 0 / 0 |
| 71 | #671 housekeeping of pushed and stale-locked worktrees, #672 harness inventory guards, the instruction-cache cross-disk fix / #675 (workflow change; bootstrap Full) | `64bbc057bf` | 0 / 436 / 0 / 0 |
| 72 | #644 `u8` boolean returns across the boundary, #646 `TileArea::expand` int wrapping / #647 | `3c30fb8129` | 870 / 242 / 0 / +173 at its reviewed head (+167 compiled on base): inline bool wrappers; orphan exports 54 to 48 |
| 73 | #673 typed `Scenario`, `Run` and `Result` records and owner-only hook dispatch / #674 | `02393552fd` | 0 / tooling only / 0 / 0 |
| 74 | #501 first typing batch: six simulation scenario modules / #677 | `566b87c35d` | 0 / tooling only / 0 / 0 |
| 75 | #652 GRF text and NewGRF Actions 3, 4 and 13 / #655 | `da7faa8d0b` | compiled C++ **-648** (262,618 to 261,970; 645 reported before review); original retired +761; exports 1,443 to 1,447, callback slots 4,026 to 4,044 |
| 76 | #501 typing batch 2: thirteen comparison tools / #680 | `9260dc5148` | 0 / tooling only / 0 / 0 |
| 77 | This host's quiet speed pass: stale-anchor acceptance, reference identities per role, one collection per bundle and the new anchors / #676 | `80d778897a` | 0 / tooling only / 0 / 0 |
| 78 | #501 typing batch 3: landscape, companies, industries and signals scenario modules / #681 | `5493d11679` | 0 / tooling only / 0 / 0 |
| 79 | #501 typing batch 4: CI and platform tooling / #682 | `54eeaee3a6` | 0 / tooling only / 0 / 0 |
| 80 | #657 Debug and GrfMsg in Rust once, #658 auth and script-host leftovers with `.inc` glue counted / #679; state-loop profile entry / #683; batched as #685 | `a2e417114b` | compiled C++ **-414** (261,970 to 261,556); callback slots 4,044 to 3,860; exports 1,447 to 1,446; rebound rows 418 to 410; shared-accessor rows +10 (`.inc` coverage re-baseline) |
| 81 | #507's fifty-year same-NewGRF save / #684; #501 typing batches 5 and 6 / #688, #689; batched as #690 with root's combined-typing fix | `7c067ad699` | 0 / tooling only / 0 / 0; #507 closed |
| 82 | #570 shared vehicle movement and consist records with direct Rust Tick / #636 | `2e6958801e` | compiled C++ **-1,952** (261,556 to 259,604); callback slots 3,860 to 3,524; exports 1,446 to 1,426; rebound rows 410 to 376, shared-accessor 136 to 120, orphan exports 48 to 43; original retired +760 |
| 83 | #666 tile, map, rail and road script APIs (first PR) / #691 | `c9de8d52ca` | compiled C++ **-716** (259,604 to 258,888); exports +68, callback slots -1; original retired +946 |
| 84 | #501 typing batches 7 and 8 / #694, #695; batched as #696 | `6e686c2b9e` | 0 / tooling only / 0 / 0 |
| 85 | #666 bridge, tunnel, marine and list script APIs (second PR) / #697 | `7fd398da1f` | compiled C++ **-130** (258,888 to 258,758); exports +15; #666 closed |
| 86 | #699 per-commit Windows vcpkg binary cache / #701 (workflow change; bootstrap Full) | `ef51e41b80` | 0 / CI only / 0 / 0 |

## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and the disjoint-service rule; finishing review and integration
of an open branch comes first. A lower rank can run while a higher one waits.
Scopes live in issues; per-PR changes live in each PR's steering comment.

0. **Correct C++ distance accounting (#707)** in the next tools maintenance batch after #705's existing scope. Preserve the user's typing-first priority. Count native tooling and generated ABI evidence separately from the original oracle; this creates no gate for ownership PRs.
1. **Finish the open branches in readiness order.** The [integration-88 checkpoint](#resume-checkpoint-2026-11-06-stocktake-88) holds current heads and concrete next actions for #705, #700, #704, #698 and #706. Integration 85's cadence is repaired; #692 and #628 integrated together as 88, clearing their dependency events. Children check `.local/deep.lock.holders` before a native build, link or game, and pass `--jobs 3`. Rebuild an open PR's worktree only when its work resumes, after pulling it to the PR head. Briefs for PRs that change the Rust/C++ boundary require a CI `native` profile run before review and Full (stocktake 83's process lesson), not a local full set.
2. **Type checking first (user, 2026-11-05).** Typing takes each slot an integration frees, ahead of the harness batch and new owners.
   - Batches 1 to 8 integrated (#677, #680, #681, #682, #688, #689, #694 and #695 as #696).
   - #705 converts the last file, `tools/test-generated-abi.py`, after the integrated #628/#692 changes. Finish its review fixes from the checkpoint.
   - Done when the ANN ignore list is empty and `ty.toml` covers all of `tools/`.
   - **#673's consolidation PR** follows typing. Inventory, scenario digests and comparison results must stay identical. It is the only harness refactor; no broader rewrite.
3. **Maintenance and measurement.**
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
- After every second integration (next after 88), take stock under `/start-development` step 4, including the compiled-C++ rate and the baseline sizes. Profile the merged tip in the background ([speed](speed.md#instruction-projections)).
- Run the platform cadence after every fifth integration; the next is after 90. Integration 85's failure was repaired by #702 and its rerun passed. A new failure stops integrations. After two hosted-capacity macOS cancellations, use `CI_MACOS_HEAVY` as documented, and delete it afterwards.
- This host's first quiet pass integrated with #676 (77). Further passes follow the [speed policy](speed.md#policy) triggers.
- Deep checks run only on the user's request (user, 2026-10-31).
- Preserve anchors, raw traces, frozen runtimes and failed outputs.
- At each stocktake, root checks issue placement and exclusion conditions.

Excluded, with return conditions reconsidered on 2026-11-05; neither trigger is met:
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until near-full reproduction under #513's original-default setting policy. #513 needs implementation when the first such improvement is accepted.
- #650 (offline vendoring of crates.io dependencies) is a `deferred-improvement`, reconsidered after near-full reproduction or when the fork publishes release source packages.

## Resume checkpoint (2026-11-06, stocktake 88)

**Paused at the user's request** (wind-down, 2026-11-06, lenient action scope), after integration 88.

Root ran in Claude Code (`claude-opus-5-5`, xhigh). Code is `5168023c13`, green on push. `CI_ON_DEMAND` is `true` and `CI_MACOS_HEAVY` is unset. No rollback was eligible.

The next cadence and stocktake follow integration 90. The merged-tip profile for integration 88 is owed. Navigation has reconciled the phase chart and work order with this checkpoint; the requested steering review follows before development resumes.

**Subagent limit (user, 2026-11-06):** the user raised Codex's per-session subagent limit to 12, in `~/.codex/config.toml` and in the main checkout's `.codex/config.toml`. Codex rejects values below 1, so the limit cannot be removed. The main checkout's change is uncommitted on purpose: leave it, and neither revert nor commit it. Worktrees still check out a limit of 8 until a PR changes the file.

Navigation fast-forwarded the main checkout to `bc6c727c48` and makes its docs-only commit there, preserving the config edit. The previous root's docs checkout is `.local/worktrees/root-docs-72`.

No job or CI run is running at the pause. Every open PR's work is pushed, and each worktree is clean apart from its `.local` output. The [#705 review](https://github.com/fetch-laboratories/openttd-rust/pull/705#issuecomment-6675882700) is posted, and its checkout `t3-452f5347` is removed.

| Issue / PR | Head | State at pause | Next action on resume | Checkout |
| --- | --- | --- | --- | --- |
| #501, #678, #693 / #705 tools maintenance | `464f66c33a` | Native CI 42153621626 green. The [review](https://github.com/fetch-laboratories/openttd-rust/pull/705#issuecomment-6675882700) requested changes: two P2 findings. The reviewer's local `network-protocol-loopback` never ran: its candidate link failed with "No space left on device" when the root SSD was full. Re-run it on resume. | Fix finding 1, the exact `SendError(): error=7` one-use match, and finding 2, the close before authorization, as [handed off](https://github.com/fetch-laboratories/openttd-rust/pull/705#issuecomment-6675945551). Then base update, a scoped review of the fixes, Full, merge. | `t3-34d0fa9c` |
| #634 / #700 station records | `6e0ec19dc6` (WIP) | Batch 4 does not compile: aircraft leaves in `src/rust/services_station.h` still call the deleted `RustStationAirportRotation` and `RustStationAirportType`. Last good is `500463d213`: CTest 236/236 and the scenarios equal. [Status](https://github.com/fetch-laboratories/openttd-rust/pull/700#issuecomment-6675897921). | 1. Merge integration 88.<br>2. Fix batch 4's build.<br>3. Finish checkpoint 2: the aircraft leaf slots, with record-backed fake stations in `aircraft-range.cpp` and the Windows ABI aircraft probe; the ship and company slots keyed by station ID; the area services and three facade owner pointers; the k-d tree read; the new lifetime, mid-visit delete-and-allocate and #566 drawing evidence.<br>4. Native CI.<br>5. The checkpoint 2 report and port-metrics. | `t3-98010525` |
| #635 / #704 town records | `e5097dd424` (WIP) | Not rebuilt since its last fix, which removed three needless `unsafe` blocks. Native CI 42154615345 at `fae36124db` failed in `native / build` and the Linux annotations, cause unconfirmed. Push checks at `e5097dd424` are green, including CI - Build and Quick validation. -24 compiled so far. [Status](https://github.com/fetch-laboratories/openttd-rust/pull/704#issuecomment-6675927407). | Rebuild, then fix leftovers until local `verify` passes; that completes checkpoint 1. Then native CI, port-metrics against base and the checkpoint 1 report. | `t3-19613657` |
| #565 item 2 / #698 map leaves | `79bc4eaa94` (draft) | Contains base `5168023c13`. Native CI 42152096539 green; local verify passed. TGP 2,791,869,839 Ir. Net glue -198, against the -300 to -600 plan. [Status](https://github.com/fetch-laboratories/openttd-rust/pull/698#issuecomment-6675915139). | Re-profile TGP against an integration-88 base profile, then mark ready, review and Full. | `t3-c3bf9002` |
| #667 / #706 content protocol | `c013663ed7` (WIP draft) | The native build, Clippy, fmt, three unit tests, ruff and ty passed. Quick validation failed at this WIP head. Not yet run: the `content-protocol` scenario, preflight, CTest, native CI and port-metrics. | Continue to the first checkpoint:<br>1. the inventory and the boundary, against the 500 to 700 estimate;<br>2. the metadata and dependency comparison with the original;<br>3. the scenario, run locally;<br>4. preflight and native CI;<br>5. port-metrics.<br>Add its two #156 notes: libstdc++ multimap order, and the null-parent abort. | `t3-9803b4f2` |

Preserve the following:
- the open PR checkouts above, each only while its PR is open;
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
