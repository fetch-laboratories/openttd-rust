# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and `docs/rust-migration.md` define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-10-28, code at `9e6d9d70e8`)

- 79 ownership ports. Since the twenty-first steering at `07c0d2e2e8`, seven
  owners and nineteen maintenance PRs integrated, retiring 4,237 more original
  lines, including 1,641 in the current simulation inventory. No confirmed code
  regression requires rollback.
- Whole-fork inventory: 72,102 of 470,612 original lines retired (15.32%),
  including 56,403 of 103,451 simulation lines (54.52%). Rust, tooling and glue
  total 129,122, 51,020 and 46,148 lines. These measure source retirement, not
  completed behavior. #527 added the omitted simulation clock files and refreshed
  the denominator; #518 did not expand that reviewed inventory.
- The identity-renewal quiet pass at `755609b8ea` completed all 144 games with
  no overlapping build, profile or game; maximum one-minute load was 1.42.
  All six semantic comparisons and compatible previous-tip identities passed.
  All six anchors integrated through #543. P2016 exceeds its 1.5x ceiling.
  Two matching-identity original profiles pass semantic checks but differ in
  presentation work driven by wall time; they remain unmatched to cached Ir.
  Instruction-site attribution estimates station and orders replacements at
  0.206% and 0.094% of P2016, only 0.301% combined after required work and
  replacement costs. Neither clears the existing thresholds; keep these as
  notes on #275. Loading and acceptance estimates now each narrowly clear 0.5%
  and together reach about 1.065% of the latest P2016 count, after required
  original work and replacement costs. Root selects that pair as one repair
  batch; register allocation remains unmeasured and can erase either margin.
  The separate immutable orders-table row stays an unselected note.
  Neither the batch's
  instruction reduction nor the notes establish isolated savings for #536.

| Scenario | Pre-batch `5d8f4c7c7f` under current anchors | Quiet anchor `755609b8ea` | Last profile `ba3095f768` |
| --- | --- | --- | --- |
| TGP | 1.2289x | 1.1117x | 1.1130x |
| Grok | 1.3903x | 1.3871x | 1.3960x |
| Opus | 1.3970x | 1.3939x | 1.4033x |
| P1996 | 1.3617x | 1.3366x | 1.3437x |
| P2000 | 1.4181x | 1.4006x | 1.4075x |
| P2016 | 1.5331x | 1.5229x | 1.5246x |

The historical `90f6ae311c` identities remain incompatible; its projections
under these anchors are unavailable. The compatible pre-batch report is retained.
The twenty-second-tip profile passed all six identities and canonical comparisons.
No projection
rose by 3% over the preceding compatible profile; no new quiet pass is triggered.
P2016 remains above its ceiling, with net-cost attribution active. The first
twentieth-tip attempt rejected changed host Brotli dependencies. Recovering the
two recorded library bytes restored all six quiet and current identities; the
compatible rerun passed. The failed attempt remains incompatible and uncredited.
- Coverage #518 integrated after exact-head Full passed. Its original-only
  report measured 84.96% of emitted lines and 53.97% of compiler branches;
  338 of 438 scenarios with zero unique lines remain investigation candidates,
  not proven redundant behavioral tests. The first #503 deep set had 519 equal
  of 520 scenarios; both games hit the same invalid terrain fixture assertion.
  #534 integrated through #538, repairing that fixture and the selected
  observation/helper gaps. The next soak had 459 of 520 equal and 61 adapter
  build failures after the live checkout advanced across #543, mixing old
  artifacts with new headers. No semantic differences were reported; this is
  invalid mixed-source evidence. The fixed isolated restart passed all 521
  soak scenarios and 8,396 snapshots in 5,551.9 seconds. Its full sanitized
  member is running; #548 integrated precise source-drift rejection. #550/#552
  repair an older road/rail boolean ABI defect and a shared house-fixture
  temporary lifetime. A rebuilt historical production crate reproduces the
  road defect; no recent integration rollback is justified. The
  changed-input original coverage recomputation is running on a fixed checkout
  containing #542/#544/#545.
- #535's 14-case sanitizer subset compared equal in 607.7 seconds. A real-game
  positive control captured five original animation-shift keys in 121.3 seconds
  with no candidate-only report. Fixing review is complete; the full sanitizer
  set's runtime remains unmeasured. #511 completed 40 valid mutants: 31
  detections and nine survivors, with two unsaved presentation limits and seven
  missing fault-sensitive subset inputs. The 41-minute study is closed; #156
  retains its scoped findings, and no recurring mutation controller is selected.
- #543 integrated reviewed clock ownership #539, disjoint effect/linkage
  maintenance #541 and measured anchors after exact-head Full. Its first Full
  exposed a history fixture definition conflicting with the new clock alias;
  the portable-only definition repair passed the existing comparison and Full.
  #544's reviewed random-command scenarios integrated after exact-head Full;
  #508 is closed. #542 also integrated after fixing review and exact-head Full; #506 is closed.
  #545 community AI/large-save evidence integrated after fixing review and
  exact-head bootstrap Full, preserving the integrated threaded/command tooling.
  C++ TSan found five shared Mesa identities and no candidate-only findings;
  Rust remains uninstrumented. #548 payment/deep/output maintenance integrated
  after fixing review, affected checks and exact-head Full. #547 complete
  world-generation lifecycle integrated as the twenty-second integration after
  fixing review, root spot-checks and exact-head Full. Three components remain:
  #551 session lifecycle, #552 sanitizer repairs and #553 delivery/industry
  maintenance. #549 multiplayer integrated as the twenty-third integration
  after fixing review and exact-head Full; #510 is closed. The newly selected
  loading/acceptance batch will be the fourth unintegrated component.
  Each integrates at its own tested head where shared files overlap.
- #529 closed after ten successful qualifying dependency installs. The
  fifteenth-integration platform cadence passed; #545 is the twentieth
  integration. Its cadence passed all five macOS semantic cases and other
  platform builds. Windows x64 lost redirected diagnostics through native
  console setup, causing strict log/witness failures; #548 carries the bounded
  repair. Its Full passed actual Windows file/pipe checks. The normal-cadence
  retry passed all five Windows semantic cases and 126 snapshots; all selected
  platform jobs passed. #509 is complete and closed. Job joins/cargodist,
  arbitrary NewGRFs and other platform configurations remain on #156.

## Twenty-second steering review (2026-10-27, observable evidence and clock ownership)

Agent: /root (Codex steering review) | Model: gpt-6-astra | Reasoning effort: xhigh.

Four independent Sol/high fidelity audits compared Action2 #493, vehicle
properties #495, house definitions #486 and catchment #522 against the pinned
original, covering 2,973 retired lines. Separate Sol/high process and retained
speed-evidence audits and Astra/high planning informed the corrections. Root
checked the load-bearing source, CI and measurement claims. This review ran no
build, simulation or wall measurement and changed no implementation code.

No confirmed reachable divergence was found. The main misses are observable
coverage and shared-helper/type adoption: a sentinel train speed is overwritten
before observation; house construction's callback mask has no rejecting callback
graph; catchment setup may place no house or industry yet still satisfy phase
labels. `Slide`, rectangle Width/Height and one industry-pointer callback still
violate shared-boundary rules. Action2's output-error, relative-random and layout
error gaps remain on #156; its audit found no new source defect.

Apply these corrections using the ranked work order below:

1. Finish #518 by updating its base and requesting final Full. Its Linux package
   timeout is infrastructure failure; the Windows-labelled annotation failure
   repeats that timeout because the checker scans the whole run. No coverage
   rewrite or new review solely for this failure. The PR now records the
   already-accepted 734-line tooling cost. Follow the existing scoped conflict
   review rule if the base update requires it.
2. Batch the three new observation gaps, the three helper/type repairs and
   #503's terrain fixture repair on #156, with #199/#500 owning their boundary
   items. Reuse the selected fixture worktree; obtain one fixing review and one
   Full for ready items. Require successful catchment setup and resulting
   nearby/delivery membership, observe the sentinel before overwriting it, and
   exercise file-loaded house rejection during construction. Replace raw sea
   and bridge height writes with original commands; check the affected default
   and soak seed, then read the next normal deep report. Do not shorten the run
   or restrict seeds to hide the fixture failure.
3. Resume #525 under #526's matching-source research and packaging scope. The
   publisher's Trains 0.3.0 tag supplies a concrete lead; the other two sets and
   complete correspondence remain unresolved. If complete matching sources
   cannot be obtained, select another community set with verifiable sources
   within #507. Preserve a real default case and long variant. This replaces
   the prior session's user-only distribution blocker; it does not reverse a
   user distribution policy or authorize a source offer on the user's behalf.
   The current 251-line tooling cost is accepted in the PR.
4. Finish the bounded #275 tile-loop batch and renew the anchors. Updated tree,
   house and water excess totals about 1.32% of the P2016 loop, not 1.6%; none
   is promised savings. The tree checkpoint has no measured credit. Keep
   speed checks on the merged branch, and do not hold identity renewal for
   speculative extra repairs. Schedule the quiet collection before launching a
   deep run when both are due; the existing lock could otherwise wait through
   the measured 99-minute set. No blocked collection was observed this period.
   No new process controller or lower threshold is selected.
5. Use #511 to test whether covered wrong results escape observation. The
   current observation holes make this more useful than deleting scenarios
   merely because they add zero unique source lines. Target the issue's bounded
   mutation sample using #518 and these audits; retain behaviorally distinct
   cases. Broader random commands remain selected after this and sanitizer work.
6. #527 selects calendar/economy clock ownership. Rust owns canonical clock
   state, specialized subscriptions, conversion and complete advancement/event
   dispatch; generic realtime/window/tick timers stay out. Preserve one daily
   callback snapshot, period/pointer ordering, callback mutation and timeout
   semantics, and call existing Rust owners directly. Native save/consumer
   aliases need explicit layout and lifetime proof. The issue defines reusable
   harness evidence and estimates 440-460 retired lines, 250-400 gross glue and
   200-300 support lines; judge the first concrete boundary under the ordinary
   single cost disposition. Include these simulation timer files in the shared
   inventory without expanding #518's reviewed scope. No clock branch has begun.
7. Keep placement and artifacts current. #515 is closed; #168's disaster action
   is complete; #503 already runs. #500's next pointer repair is selected in the
   maintenance batch, and the next #168/#199/#500 slice is selected at the
   stocktake immediately after that batch integrates. Historical mentions do
   not schedule remaining work. Preserve active pinned speed dependencies;
   superseded coverage output can be removed after its failure evidence is
   recorded. The current free space provides no reason for aggressive cleanup.

#522's final `wrapping_add` repair was faithful but changed arithmetic after the
last independent review. The steering audit covered it; no retrospective review
round is needed. AGENTS.md clarifies that CI-discovered behavior fixes need a
fresh scoped review, unlike warning/build repairs. It also records the user's
Opus-only implementation rule under Claude roots, the narrow retained road
track-status descriptor authorization, active-evidence cleanup protection and
meaningful issue placement. The restart prompt remains unchanged.

| Twenty-first correction | Verdict and cost/result |
| --- | --- |
| One ranked order across ports, tests and maintenance | Applied: five owners and seven maintenance integrations. Completed actions remained active; this review replaces them. |
| #497/#498 remain parked until selected owners need Rust storage | Applied: no new branches; return conditions remain unmet. No extra storage port is selected. |
| Disjoint unintegrated services | Improved: six base-update review rounds, only #499 found a source defect. Shared helper adoption still needs the small batch above. |
| Integrate overlapping components separately | Applied: six owner Full requests for five owners, including #522's retry. No evidence shows solo Fulls prevented combination failures; retain the rule and watch its cost. |
| Production caller/orphan inventory and fixture grep | Useful: reviews found Action2, airport, disaster and catchment interface defects. #522's final arithmetic review slipped; no audited production orphan remains. |
| In-band speed cadence and retained profiles | Applied: no repeated quiet collections; the provisional 3% projection trigger detected #517. Keep 1.5%, 0.5% and 1% credit thresholds based on the prior 3.3% wall spread versus 1.13% credited repairs. |
| Audited ground-vehicle accessors | Implemented but no net saving established. #517/#523 required three Full requests and 45.4 elapsed minutes, including cancellation. Keep no separate credit for retained road reads and require measured value before extending them. |
| Worktree/docs hygiene | Five active worktrees, no limit breach; roadmap grew only 370 to 378 lines. Deleting the active pinned runtime caused a recovery task and identity failures. Preserve current dependencies, remove only superseded output. |
| Testing before speculative support ports | Coverage and deep runners delivered; coverage needed four attempts, final 3.64 hours. Deep cost 390 tooling lines versus about 100 estimated and found a fixture defect in 99 minutes. Corpus review found a concrete source-packaging problem at 251 lines. Finish these results rather than commission replacement tools. |

Earlier corrections were checked as well: #462 recovery remains integrated in
#463; collector repairs and serialized-thread/tree-ambient witnesses remain;
#446/#447's bounded cost decisions are complete; owners progressed alongside
speed work; clean-checkout checks, unchanged reference and process-wide state
contracts remain in force. No new quiet pass tested the collector this period.
The provenance rule slipped on the pinned runtime. Evidence-content rules kept
component entries compact, but stale issue/PR decisions needed the corrections
above. The period used 47 distinct delegated tasks (24 Codex, 23 Claude), plus
one retry under the same request ID; elapsed CI minutes are not agent hours.

| User-directed rule since earlier steering | Verdict and next observation |
| --- | --- |
| Action-bounded wind-down and reread/send workflow | Applied: Codex stopped in about eight minutes under explicit lenient exceptions; Claude in about two. No unauthorized new work found. |
| Content rules replace roadmap/evidence caps | Applied; modest growth retained necessary facts. Staleness, not line count, caused the remaining confusion. |
| Root decides tooling overruns without user escalation | Decisions were made; #518/#525 descriptions lagged and are corrected. No new cost approval gate. |
| Python annotations, typed records, Ruff ANN and ty | Applied: reviews caught missing annotations/fixed-shape dictionaries. #501 continues incrementally. |
| Testing queue and platform cadence | Applied: Full jobs fell from 20 to 17; cadence exposed macOS annotations. The cancelled #521 and #522 runs had already exposed signature and arithmetic defects; #517's cancellation merely superseded it. |
| Every open issue placed and exclusions rechecked | Partial: textual checker accepted completed/history mentions for #500, parked #498 and finished #515. Explicit current actions and return conditions now accompany each; no larger issue controller. |
| Deep alongside every second integration; exclusive speed lock | One completed set establishes cost and a fixture failure; no observed priority inversion. Keep the lock and coordinate quiet collection first when both are due. Coverage recomputation remains scheduled only when scenario inputs change under #503. |
| Steer evaluates rule changes | Applied by this review; no user testing or timing threshold is reversed. |
| Sonnet writes no code under Claude root | Applied prospectively: two workers stopped and restarted on Opus after the instruction. No full subsequent session yet; next steer checks authorship and cost under this rule. |

## Earlier steering, still in force

Ownership, original behavior, process scope, named callees, typed shared services
and exact random-draw order come first. Preserve default semantic evidence,
record gaps on #156 and keep retirement distinct from glue. No hot allocation,
opcode dispatch, whole-record copies or owner bounces. Descriptor reads stay
within `docs/design/world-state.md`. Ready owners proceed beside speed work;
only timed collection holds builds/games. Compare one compatible anchor; factor 2
and the speed thresholds remain provisional. Six branches, batched maintenance,
fixing reviews, strict exact-combination Full, bounded rollback and action-based
wind-down remain. Recover confirmed regressions before further integrations.
AGENTS.md carries the rules; issue and PR bodies carry current actions/evidence.

## Recent completed work

The earlier 72 ownership ports and maintenance batches are recorded in their PRs
and Git history. These rows cover the period since the previous steering.
Metrics are `tools/port-metrics.py`: Rust / tooling / gross C++ glue / original
C++ retired; net glue is separate.

| Issue | Component | PR | Commit | Metrics |
| --- | --- | --- | --- | --- |
| #485 | Action2 graph construction and publication | #493 | `5e39efe901` | 1206 / 300 / 195 / 708; net glue 159 |
| #491 | Vehicle properties and deferred purchase ordering | #495 | `3b3382ce1c` | 1148 / 338 / 367 / 1031; net glue 290 |
| #477 | House definition loading and publication | #486 | `fc3bf4b20b` | 714 / 372 / 221 / 532; net glue 193 |
| #492 | Airport and airport-tile definitions | #499 | `c04219b0d8` | 590 / 297 / 311 / 336; net glue 251 |
| #516 | Station catchment, rectangles and nearby lists | #522 | `4c73c991e9` | 2216 / 315 / 461 / 702; net glue 204 |
| #527 | Calendar/economy clocks and subscriptions | #539/#543 | `74cea95cf8` | 820 / 505 / 571 / 519; net glue 482 |
| #546 | Complete world-generation lifecycle | #547 | `591282249e` | 746 / 293 / 422 / 365; net glue 344 |

| Issues | Maintenance PRs | Integration | Commit | Metrics |
| --- | --- | --- | --- | --- |
| #514 cadence split; #501 gate configuration | #502 | direct | `1fae5d498a` | 0 / 118 / 0 / 0; net glue 0 |
| Cadence annotation warnings; orphan cargo-label export | #521 | direct | `cf8e561d61` | 0 / 0 / 5 / 28; net glue -4 |
| #275 ground-vehicle accessor audit | #517 | direct | `7274b59099` | 536 / 19 / 42 / 0; net glue 34 |
| #515 orphans and accessors; #500 first ID types; #506 quick check | #519 | direct | `fbf9f1cc5a` | 328 / 82 / 144 / 13; net glue 66 |
| #275 recovery of the post-#517 loop rise | #523 | direct | `4481a442f6` | 65 / 0 / 0 / 0; net glue -2 |
| #168 disaster typed services and direct calls | #520 | direct | `be18afafda` | 1168 / 118 / 323 / 3; net glue 111 |
| #503 deep runner and mutual exclusion | #524 | direct | `35202d9eda` | 0 / 390 / 0 / 0; net glue 0 |
| #528 host RPATH and `migration.py env`; #529 apt retry; #531 `ci.py wait`; #532 housekeeping | #533 | direct | `f0a576839d` | 0 / 727 / 0 / 0; net glue 0 |
| #504 original simulation coverage | #518 | direct | `3b75d5c5fb` | 0 / 743 / 0 / 0; net glue 0 |
| #156/#503 observations and terrain; #199/#500 helpers/types | #534 | #538 | `db624a6f00` | 17 / 96 / 4 / 0; net glue 0 |
| #505 sanitizer game comparison and report identity | #535 | #538 | `db624a6f00` | 0 / 551 / 0 / 0; net glue 0 |
| #275/#312 tile ambient observations and orphan removal | #536 | #538 | `db624a6f00` | 151 / 17 / 4 / 0; net glue -1 |
| #509 prepared platform semantic comparison | #540 | direct | `c5b522f1eb` | 0 / 402 / 0 / 0; net glue 0 |
| #537 testing-off linkage; #168 typed effect mutations | #541 | #543 | `74cea95cf8` | 78 / 0 / 49 / 0; net glue 28 |
| #508 seeded random-command semantic scenarios | #544 | direct | `d26b9447bc` | 0 / 501 / 0 / 0; net glue 0 |
| #506 threaded semantic and C++ TSan evidence | #542 | direct | `18fb574219` | 0 / 933 / 0 / 0; net glue 0 |
| #507 community AI and larger vanilla save | #545 | direct | `a297bed748` | 0 / 440 / 0 / 0; net glue 0 |
| #168/#199/#500 payment boundaries; #503 evidence; #509 Windows diagnostics | #548 | direct | `467b89bcbd` | 113 / 378 / 140 / 0; net glue 68 |
| #510 mixed multiplayer semantic comparison | #549 | direct | `9e6d9d70e8` | 0 / 669 / 0 / 0; net glue 0 |

## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and disjoint-service rule; completing review/integration takes
priority. A lower rank can run while a higher one waits. Scopes live in issues.

1. **Review and integrate #552 and #553.** #549 is integrated. #552 repairs
   four demonstrated road/rail availability/validation wire returns and the
   shared house-fixture lifetime. Logical APIs and service callbacks stay bool;
   the four exports use canonical byte carriers under pinned Rust 1.99.
   Cargo, 2,331 native assertions and affected ordinary/sanitized cases pass;
   unchanged house evidence is retained. Necessary cost remains accepted:
   16 Rust, 3 tooling, 50 net glue and zero retirement, with 48 native test
   lines. #553's delivery/industry and typing checks also pass; root accepts
   its 291 Rust, 49 tooling, minus 25 net glue and zero retirement. Give each
   a fresh fixing review, then integrate in readiness order through separate
   current-base Full gates where files overlap. NewGRFs remain user-parked.
2. **#275 loading/acceptance repair batch.** Root selects the two qualifying
   estimates: loading 73,011,340 Ir and acceptance 70,874,325 Ir, about 1.065%
   together at the latest P2016 count. Loading keeps an immutable table address
   caller-local across ProcessOrders, reads owner/type fresh in a direct Rust
   inline guard, and resolves visits only on OT_LOADING/OT_DUMMY slow paths.
   Preserve guarded initialization and mutable-state reacquisition; remove the
   displaced loading slot/export/declaration. Acceptance inlines the existing
   canonical bitmap/orthogonal helpers and resolves the same native assertion
   service only for false conditions. Preserve native NDEBUG policy, fresh
   membership after cargo callbacks, geometry/arithmetic/bounds, empty/default
   cursors and short borrows. No raw descriptor, second cursor or copied bitmap.
   Existing orders/visits, station/catchment/iterator and P2016 semantics supply
   evidence. Inspect emitted code during ordinary implementation, with no PR
   instruction/wall speed check. Plan zero retirement, 30–100 net Rust lines,
   neutral or shrinking glue and up to 60 support lines; judge the first draft.
   One fixing review and Full cover ready items; an unready item leaves it.
   Actual register allocation can erase the narrow margins. Station tick,
   orders resolution, immutable orders-table binding and road notes remain
   unselected. Only merged-tip measurement establishes savings.
   #312's implemented tree narrowing is complete, without isolated speed credit.
3. **#168/#199/#500 delivery and industry boundaries.** #548 completed the
   payment lifecycle/settlement/income and #503 evidence protection. Its
   integration selects the next bounded maintenance batch: replace station_read,
   industry_read/write, accept, statistics and industry_effect in CargoPayment
   deliver/trigger with named typed operations. Reuse existing station,
   industry, company and town owners and canonical Source/IndustryID/ShellHandle.
   Preserve observation points, saturation, callback ordering/reentry and pool
   semantics; remove displaced selectors, wrappers and fixture bindings. No
   whole-fork conversion, new raw-map permission or speed credit is selected.
   Carry the existing Cargo/native/economy/subsidy evidence. Root decides cost
   at the first draft; plan zero retirement, 150–300 Rust lines changed,
   30–100 net glue and 50–100 native/tooling support lines.
4. **#551 complete simulation-session lifecycle and pause control.** #547's
   integration selects this owner alongside the disjoint delivery/industry
   maintenance. Move StateGameLoop, GameLoop, SwitchToMode, SafeLoad,
   intro/new-game/editor startup and completion, settings publication,
   NewGRF-scan requests and CmdPause control. Rust owns canonical mode,
   pending switch, pause/scan-request state and private persistent state;
   preserve native aliases, chrono/resource semantics and genuine exception
   frames. Keep the native atomic exit flag and exact profiling entries.
   Compose existing Rust owners directly and remove displaced callbacks;
   release state before load/reset, progress yields, network reentry and
   callbacks. No new raw-map access. Default evidence must distinguish normal,
   paused, modal/editor, restart/reload and failed-load partial effects through
   actual entries, plus unsaved thread/alias and exception observations.
   Planning ranges: 1,000–1,500 Rust, 450–650 net glue and 250–400 support;
   root decides actual retirement/glue/tooling cost at the first concrete
   checkpoint. Complete command execution remains an alternative after this
   owner's checkpoint, not a concurrent selection.
5. **#505 ASan/UBSan.** Tooling is integrated. Read the first full sanitized deep
   member, distinguish original failures from candidate-only faults and decide
   the measured runtime budget.
6. **Recurring platform semantic cadence.** #509 is complete: macOS passed
   at the twentieth cadence, and Windows passed all five full-duration cases
   on the integrated #548 repair. Keep the prepared no-join inputs, strict
   comparisons and #156's bounded limits. The next ordinary cadence is due
   after the twenty-fifth integration; no per-PR semantic platform job is added.
7. **#501 incremental typing; #156 selective observation work.** Select the
   bounded validation_execution.py annotations and typed holder records for
   the delivery batch if ready; preserve subprocess/locking behavior and its
   existing tests, add ty scope and remove the ANN exemption. Unready typing
   leaves the batch. After #552's road-type validation repair integrates, add
   TownCanGrowRoad inputs with differing road types, observing resulting
   growth/RNG. Preserve #511's other input and presentation gaps; further
   mutation samples remain selective.
Recurring: instruction profiles and deep checks after every second integration,
subject to the existing running-deep skip and quiet collection lock. Read completed
reports before integration. Keep incompatible projections unavailable. Retain active anchors and their
runtime dependencies. #503 owns the single changed-input coverage recomputation
after #542/#544/#545 integrate, along with their applicable deep membership.
Run `issues.py` at start/planning/stocktake and check actual current placement,
not just a textual mention. The twentieth-integration cadence and retry passed
after #548's diagnostic repair. #512's first tally is finished. Launch the
next instruction profile after the twenty-fourth integration; the fixed-source
twenty-second profile is complete. Skip another deep run while the fixed-source
sanitizer member is still active.

Excluded, with return conditions checked by this review:
- #507's NewGRF corpus (#525, #526) is parked by the user (2026-10-28) until
  the user reselects it. The resume plan on #507 keeps the sets uncommitted,
  fetched by pinned hash into `.local/` and cached in CI; #525's branch stays
  on origin.
- #497 Action3 and #498 object definitions remain parked until a selected
  simulation component needs their data in Rust; then replan for owned storage.
  A free slot alone is not that event, and #527 needs neither.
- #69's remaining tile-list/support work is deferred until a selected simulation
  component needs it; catchment's rectangle/bitmap part is complete in #522.
- #64 and #68 are vendored cryptography, deferred until after simulation ownership.
  New GUI/platform ownership under #3 follows simulation; #509's completed
  validation and recurring cadence are not excluded by that policy.
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until
  near-full reproduction under #513's original-default setting policy.
  #513 needs implementation when the first such improvement is accepted.

## Resume checkpoint

Codex root integrated #548 payment/evidence maintenance, #547 complete world
generation and #549 multiplayer evidence. #509 and #510 are closed. The older
road/rail ABI and house-fixture sanitizer faults are assigned to #552. Fetch
remote heads before resuming historical branches. Active anchors depend on the
pinned zlib/lzma/Brotli runtime; incompatible historical reports stay uncredited.

| Item | Current state | Next action |
| --- | --- | --- |
| #509 and #529 | CI_ON_DEMAND is true; twenty-third integration complete. Windows/macOS semantic cadence passed; both issues are closed. | Retain the normal fifth-integration cadence and #156's actual limits. |
| #275 | Twenty-second-tip profile passes all six comparisons/identities; P2016 projects 1.5246x. Loading/acceptance estimate about 1.065% is selected as one batch. | Implement the bounded pair; no PR speed check or new quiet collection. Next instruction profile is due after the twenty-fourth integration. |
| #505/#503/#550 | Fixed-source soak passed 521 cases/8,396 snapshots; full sanitizer member and changed-input original coverage remain active. #552's four-export and house repairs pass focused evidence. | Read final reports, give #552 fresh fixing review and Full; skip overlapping deep launch. Preserve active checkouts. |
| #511 | Completed and closed: 31 detections, nine classified survivors, zero invalid/inconclusive; source/build restored. | Use #156's scoped findings; further samples are selective. |
| #168/#199/#500 and #501 | #553 delivery/industry and bounded typing checks pass; necessary cost is accepted. | Give the final implementation head fresh fixing review and current-base Full. |
| #546/#551 | #547 integrated after fixing review and exact-head Full; #546 is closed. Session lifecycle/pause implementation is assigned. | Judge its first concrete retirement/glue/tooling cost; preserve direct ownership and named exception frames. |
| #506 | #542 integrated after fixing review and exact-head Full; issue closed. | Future deep runs include native/TSan members; preserve #156's uninstrumented Rust and remaining lifecycle limits. |
| #508 | #544 integrated after exact-head Full; issue closed. | Keep histograms and scoped #156 limits; six soak variants join the next current-source deep run. |
| #507 | #545 integrated after exact-head bootstrap Full; community/large vanilla evidence complete. | Keep NewGRFs user-parked; issue remains open for that scope. |
| #510 | #549 integrated after fixing review and exact-head Full; issue closed. | Future deep runs include the member. Retain #156's spectator/handshake/network limits. |
| #525/#526 | Parked by the user; draft #525 closed at ccc28e2ed0. | None until user reselects it. |
Selection: root; planning agents use gpt-6-astra, high. The start-development
skill remains open-ended and unchanged; completing this order triggers planning.
