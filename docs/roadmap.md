# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and `docs/rust-migration.md` define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-10-28, code at `3443388264`)

- 79 ownership ports. Since the twenty-first steering at `07c0d2e2e8`, seven
  owners and twenty-one maintenance PRs integrated, retiring 4,237 more original
  lines, including 1,641 in the current simulation inventory. No confirmed code
  regression requires rollback.
- Whole-fork inventory: 72,102 of 470,612 original lines retired (15.32%),
  including 56,403 of 103,451 simulation lines (54.52%). Rust, tooling and glue
  total 129,320, 51,037 and 46,173 lines. These measure source retirement, not
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
  Neither the batch's estimated reduction nor the notes establish isolated
  savings for #536.

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
  soak scenarios and 8,396 snapshots. The complete two-member run took 6.06
  hours; the sanitized member took 4.50 hours, with 442 of 446 cases passing.
  Its four failing cases were the older road/rail boolean ABI defect and shared
  house-fixture temporary lifetime, now repaired by reviewed #552 through #556.
  A historical production rebuild reproduces the road defect, so no recent
  integration rollback is justified. The shared original ScriptDate enum read
  in the Squirrel subsidy binding remains recorded on #156, without suppression.
  Root accepts the sanitizer member's measured cost and selects alternating
  soak/sanitizer rounds, each retaining threaded, TSan and multiplayer members.
  #503 owns a small explicit member selector and accurate partial-report metadata;
  prebuild ordinary probe dependencies before a sanitizer-only round.
  The changed-input original coverage run passed 259 of 449 cases before two
  600-second ferry snapshot timeouts; 188 cases have no outcome. Both ferries
  continued progressing. Earlier successful coverage used a 1,800-second bound
  on unchanged ferry inputs. Continue only the 190 unfinished cases at that bound,
  retaining complete traces and failed outputs. Final percentages remain unavailable.
- #535's 14-case sanitizer subset compared equal in 607.7 seconds. A real-game
  positive control captured five original animation-shift keys in 121.3 seconds
  with no candidate-only report. Fixing review is complete; the full sanitizer
  member's measured runtime and finding dispositions are complete; #505 is closed. #511 completed 40 valid mutants: 31
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
  fixing review, root spot-checks and exact-head Full. #549 multiplayer integrated
  as the twenty-third integration; #510 is closed. Reviewed, disjoint #552 and
  #553 integrated through #556 as the twenty-fourth integration after exact-head
  Full, including the native-only C++ linkage warning repair. Two components
  remain: #554 session lifecycle and #555 loading/acceptance maintenance.
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
| #550 four boolean wire returns and shared house fixture | #552 | #556 | `3443388264` | 16 / 3 / 58 / 0; net glue 50 |
| #168/#199/#500 payment delivery; #501 execution typing | #553 | #556 | `3443388264` | 291 / 49 / 81 / 0; net glue -25 |

## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and disjoint-service rule; completing review/integration takes
priority. A lower rank can run while a higher one waits. Scopes live in issues.

1. **Review and integrate #554, complete session lifecycle and pause control.**
   All selected semantic scenarios and final native failed-load checks pass.
   Rust owns complete loop/mode/load/startup/settings/scan/pause control and
   canonical process-wide state; preserve native aliases, chrono/resource and
   genuine exception frames, the atomic exit flag and profiling entries. Root
   accepts 593 original lines retired, 1,145 Rust, 445 net glue and 398 tooling
   as the necessary cost of this boundary and default saved/unsaved evidence.
   A fresh fixing reviewer compares every replaced body and caller before root's
   current-base Full. Complete command execution remains a later alternative,
   rather than a concurrent owner. NewGRFs remain user-parked.
2. **#275 loading/acceptance maintenance, draft #555.** Both repairs are pushed;
   Cargo, native and nine semantic cases pass. The final aircraft fixture check
   failed during compilation and returns to its owner before independent review.
   Root accepts 58 Rust and 30 tooling lines added, three glue lines removed and
   zero retirement; canonical helper changes and production-path fixture migration
   explain the cost. Preserve guarded initialization, fresh mutable observations,
   native assertion policy and direct Rust dispatch. The selected unbuilt pair
   estimates about 1.065% of P2016, with narrow margins that register allocation
   can erase. Inspect emitted code without a PR speed check. One fixing reviewer
   and current-base Full cover ready items; an unready item leaves the batch.
   Station tick, orders resolution, immutable orders-table field binding and road
   notes stay unselected. Only merged-tip measurement establishes savings.
3. **#503 bounded evidence recovery and deep rotation.** Continue the 190 unfinished
   original-coverage cases on fixed source using the retained binary, traces and
   existing measurement/aggregation helpers, with the previously successful
   1,800-second bound. Preserve the failed attempt and collect a separate final
   report only after all 449 traces exist. No full-corpus restart or replacement
   coverage runner is selected. The completed deep set exceeds about six hours;
   select a small tracked member flag, expected 30–60 tooling lines, retaining
   the default full set and marking rotation explicitly in reports. Alternate
   soak versus sanitizer rounds with threaded/TSan/multiplayer in each. Sanitizer
   probes require the ordinary candidate archive before that round starts.
   Review and integrate this bounded infrastructure before the pending rotated
   twenty-fourth-integration run; do not overlap timed speed collection.
4. **Recurring platform cadence.** #509 is closed after macOS and Windows passed
   all five full-duration prepared cases. Keep strict comparisons and #156's
   bounded limits. The next normal cadence follows the twenty-fifth integration.
5. **#156 selective observation; #501 and remaining boundary maintenance.** #553
   completed delivery/industry selectors and validation_execution.py typing.
   #168/#199/#500 and #501 remain open for incremental conversion, with their
   next bounded slices selected at #551's integration stocktake. Now that #552
   is integrated, add TownCanGrowRoad inputs with differing road types and observe
   actual growth/RNG. Preserve #511's other input and presentation limits;
   further mutation studies remain selective.

Recurring: profile the integrated twenty-fourth tip now, retaining the pinned
runtime and complete six-case identities. No rise over 3% or new quiet-pass
trigger was established by the twenty-second profile. Read finished deep reports
before integration. The prior deep set is terminal and classified; the next run
awaits the selected bounded rotation support. Keep incompatible projections
unavailable and preserve active anchors, traces and runtime dependencies. Root
runs issues.py at stocktake and checks each current action and exclusion condition.
A fresh planning agent ranks the next owner and remaining maintenance; selection
remains with root. The previous corpus result measures reachability, not equivalence.

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

Codex root integrated reviewed #552/#553 through #556 as the twenty-fourth
integration. #550 and #505 are complete; no confirmed recent regression requires
rollback. The two remaining components are #554 and #555. Active anchors depend
on the pinned zlib/lzma/Brotli runtime; incompatible reports remain uncredited.

| Item | Current state | Next action |
| --- | --- | --- |
| #509 and #529 | CI_ON_DEMAND is true; twenty-fourth integration complete. Windows/macOS semantic cadence passed; both issues are closed. | Retain the normal fifth-integration cadence and #156's actual limits. |
| #275 | Twenty-second-tip profile passes; P2016 projects 1.5246x. #555 implements the selected pair; final fixture compilation needs repair. | Profile the twenty-fourth tip; finish #555 validation and fixing review. No PR speed check or new quiet collection. |
| #505/#503/#550 | Completed deep set took 6.06 hours; all four sanitized failures map to #552, now integrated. Shared original ScriptDate report is retained. #505/#550 are closed. Coverage has 259 complete traces and 190 unfinished cases. | Continue bounded coverage at 1,800 seconds; implement explicit member rotation before the next deep run. Preserve fixed checkouts and failures. |
| #511 | Completed and closed: 31 detections, nine classified survivors, zero invalid/inconclusive; source/build restored. | Use #156's scoped findings; further samples are selective. |
| #168/#199/#500 and #501 | #553 delivery/industry and execution typing integrated through reviewed, Full-tested #556. Trackers remain open. | Select the next bounded conversions at #551 integration stocktake. |
| #546/#551 | #546 is closed. #554 session implementation and focused evidence pass; necessary cost is accepted. | Fresh fixing review, root spot-check, base update and exact-head Full. |
| #506 | #542 integrated after fixing review and exact-head Full; issue closed. | Future deep runs include native/TSan members; preserve #156's uninstrumented Rust and remaining lifecycle limits. |
| #508 | #544 integrated after exact-head Full; issue closed. | Keep histograms and scoped #156 limits; six soak variants join the next current-source deep run. |
| #507 | #545 integrated after exact-head bootstrap Full; community/large vanilla evidence complete. | Keep NewGRFs user-parked; issue remains open for that scope. |
| #510 | #549 integrated after fixing review and exact-head Full; issue closed. | Future deep runs include the member. Retain #156's spectator/handshake/network limits. |
| #525/#526 | Parked by the user; draft #525 closed at ccc28e2ed0. | None until user reselects it. |
Selection: root; planning agents use gpt-6-astra, high. The start-development
skill remains open-ended and unchanged; completing this order triggers planning.
