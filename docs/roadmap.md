# OpenTTD-Rust near-term roadmap

Root selects here; `AGENTS.md` and `docs/rust-migration.md` define process. Follow
this roadmap over conflicting issues; it is the current state of umbrella issue
#3. Update it in place; `/steer` prunes it. Evidence stays in PRs.

## Where the fork stands (2026-10-27, code at `35202d9eda`)

- 77 ownership ports. Since the twenty-first steering at `07c0d2e2e8`, five
  owners and seven maintenance PRs integrated, retiring 3,353 more original
  lines, including 784 in the current simulation inventory. No rollback and no
  new confirmed reachable divergence in this steering review.
- Whole-fork inventory: 71,218 of 470,612 original lines retired (15.13%),
  including 55,546 of 102,615 simulation lines (54.13%). Rust, tooling and glue
  total 127,585, 44,872 and 45,227 lines. These include comments and data and
  measure retirement, not completed behavior. The simulation inventory omits
  `src/timer/`; #527 adds the selected clock sources and refreshes its denominator.
- Speed: #517 raised play loop counts by about 2.25% to 4.05%. #523 recovered
  P2016 to +0.11337% against `c04219b0d8` at its measured commit `1813a11c79`.
  The later retained baseline `90f6ae311c` is +0.81531% against `c04219b0d8`;
  this later change is not attributed by the steering audit. #517/#523 earned
  no demonstrated aggregate optimization credit. The last valid projections,
  at `c04219b0d8`, are TGP 1.1788x, Grok 1.3948x, Opus 1.4018x, P1996
  1.3354x, P2000 1.3934x and P2016 1.4695x. Current build identity differs from
  the f492 anchors; renew them before publishing new projections. Details: #275.
- Deep: #524 implemented the runner. The first 520-case soak set completed in
  5,958.6 seconds; 519 compared equal and both games hit the same invalid
  terrain fixture assertion in `town-commands`. Repair scope is on #503.
  Coverage #518 measured 84.96% of emitted lines and 53.97% of compiler branches
  in the current original simulation-source inventory; those figures exclude
  timers. Its 338 of 438 scenarios with zero unique lines are investigation
  candidates, not proven redundant behavioral tests.
- CI: the period contained 16 Full requests: 12 passed, three cancelled and one
  failed, totalling 198.9 elapsed minutes including queue/finalization, not billed
  runner-minutes. #518's failure and the latest post-merge failure were Linux
  dependency-download timeouts before compilation. The cadence at `4481a442f6`
  passed. Five selected unfinished worktrees and two draft PRs remain, within
  the six-branch limit. #512's first defect tally is complete and closed.

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

## Work order

One ranked list. Each free slot takes the highest unblocked item that fits the
six-branch limit and disjoint-service rule; completing review/integration takes
priority. A lower rank can run while a higher one waits. Scopes live in issues.

1. **#504 / #518 coverage integration.** The completed report can guide the
   rest; update base and retry final Full after the infrastructure failure.
2. **#156 observation and #503 fixture repairs, with #199/#500 helpers/types.**
   One maintenance batch makes existing claimed evidence distinguish wrong
   behavior and restores valid deep terrain. Reuse the fixture worktree.
3. **#275 / #312 tile-loop batch, followed by identity-required re-anchoring.**
   There is measured excess and limited headroom; actual savings are unknown.
4. **#511 targeted mutation study.** Measure observation quality before removing
   scenarios or expanding random input. Use #518 and the new observation gaps.
5. **#505 ASan/UBSan.** Continue the existing checkpoint; compare both games
   and distinguish original failures from candidate-only boundary faults.
6. **#527 calendar/economy clocks.** Recurring simulation state and dispatch
   ownership with existing evidence infrastructure, no external prerequisite.
7. **#509 platform semantic subset.** Concrete MSVC defect history and an
   existing cadence make this more useful than speculative corpus expansion.
8. **#506 threaded execution and TSan.** The quick guard is integrated; complete
   deterministic stopping and prove the specified thread-local mutation fails.
9. **#508 seeded random-command AI.** Target measured unexecuted command
   branches and compare command outcomes as well as terminal state.
10. **#507 remaining community AI and large-map corpus**, then **#510 mixed
    multiplayer feasibility**. Broaden integration evidence after the cheaper
    concrete gaps above; neither depends on the deferred owners.
11. **#501 typing and remaining #168/#199/#500 debt.** Carry annotations on
    touched #505/#527 tooling. At the first stocktake after item 2 integrates,
    select the next concrete selector, observation or ID conversion slice from
    these trackers using measured cost or a source defect; do not launch a
    whole-fork sweep. Disaster conversion is complete.

Recurring: instruction profiles and deep checks after every second integration,
subject to the existing running-deep skip and quiet collection lock. Read completed
reports before integration. Until the identity-required pass, keep raw counts and
label incompatible projections unavailable. Retain active anchors and their
runtime dependencies. #503 also owns adding changed-input coverage recomputation
after #518 integrates and the later deep members as their issues complete.
Run `issues.py` at start/planning/stocktake and check actual current placement,
not just a textual mention. After twelve integrations since the previous steer,
the next platform cadence is the fifteenth; #512's first tally is finished.

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
  New GUI/platform ownership under #3 follows simulation; #509 validation is
  selected now and is not excluded by that policy.
- #12, #26, #41, #67, #71 and #83 are behavior improvements deferred until
  near-full reproduction under #513's original-default setting policy.
  #513 needs implementation when the first such improvement is accepted.

## Resume checkpoint

The Claude Code root (2026-10-28) integrated #533 and wound down at the user's
request; review agents were allowed to finish. No confirmed code regression
requires rollback. Fetch the remote PR head before resuming a local checkout;
older T3 worktrees of #534, #535 and #536 are behind their remote heads.

| Item | Current state | Next action |
| --- | --- | --- |
| #529 follow-up | #533 integrated; the apt retry has run in one Full. | Record the Install dependencies step duration over the next ten Linux jobs on #529. |
| #518 | Updated from base at `2992371c70`; one `tools/migration.py` conflict kept both `coverage` and `env`; `/root/review518-conflict` reviews the resolution. | Read that report, then request exact-head Full and integrate. Preserve final report through integration. |
| #156/#503 batch | Draft PR #534; town-commands default and soak pass; `/root/issue503-156-fixture-validate` checkpoints its scenario results and the house-rejection item at wind-down. | Read the PR status comment, finish the listed checks and items, then fresh review and Full. Decide the statue fixture's direct height writes. |
| #525/#526 | Parked by the user; draft closed at `ccc28e2ed0`. | None until the user reselects it. |
| #275/#312 | PR #536 at `173dd67b77`; 80 of 80 trees/landscape/water scenarios equal; `/root/review536-tile-loop` fidelity review. | Spot-check review fixes, update base, Full, integrate; measure speed on the merged tip. |
| #505 | Draft PR #535; `/root/issue505-sanitizers` checkpoints the sanitized subset at wind-down. | Read the PR status comment and continue from it. |
| Quiet pass | Existing anchors incompatible. | After bounded tile batch, before a new deep launch when both are due; prepare dependencies before the hold. |
| #527 | Issue selected, no branch. | Start from integrated base when a slot and rank permit; assess first concrete ownership boundary. |
| Jobs | No root-launched jobs remain. One integration (#533) since the #524 deep check. | Profile and deep check after the next integration. |

Selection: root; planning agents use gpt-6-astra, high. The start-development
skill remains open-ended and unchanged; completing this order triggers planning.
