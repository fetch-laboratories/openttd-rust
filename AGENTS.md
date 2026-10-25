# OpenTTD-Rust

This independent experimental fork incrementally replaces C++ with Rust while
preserving the original game's observable behavior. The original game supplies
the specification, including historical quirks. Record possible improvements in
fork GitHub issues for consideration after near-full Rust reproduction.

Read `docs/rust-migration.md` for setup, validation, and project process, and
`docs/roadmap.md` for current priorities. The roadmap decides what to work on
next; do not start work outside it without root re-selecting and updating it.
`migration/baseline.json` pins the original revision. `README.md` identifies the
fork; the remaining upstream documentation explains behavior and architecture.

## Migration method

- The goal is a game whose simulation runs in Rust. Prioritize game code: map
  generation, landscape and tile loops, towns, industries, vehicles, cargo,
  economy, link graph, pathfinding, and the commands that change them. Port
  support code only when a selected game component needs it. Vendored libraries
  (`src/3rdparty`), GUI rendering and platform code come last. Upstream
  unit-test coverage alone is not a reason to select a component.
- Prefer ownership ports: Rust owns the component's state and control flow, and
  C++ keeps only a thin facade plus callbacks for shared services (`Random`,
  pools, map access, progress). With `WITH_RUST` defined the original C++ body
  is not compiled; it remains in the `#else` of `#ifdef WITH_RUST` (or under
  `#ifndef WITH_RUST`) for portable builds. Do not extract an algorithmic
  fragment while C++ keeps the surrounding state and loop, unless the roadmap
  names it as a stepping stone.
- Keep the full game running throughout migration. Preserve networking, saves,
  mods, interface, and shared random-number behavior, including the exact
  sequence of random draws.
- Reproduce the original over its whole reachable input domain. Do not add
  guards, assertions, rejections, or fatal paths that the original lacks. Where
  the C++ wraps or truncates, use explicit `wrapping_*` operations or `as`
  casts; Rust overflow checks (enabled in release, with `panic = "abort"`) may
  only back up operations that cannot overflow in the original. This applies to
  new work; known divergences in existing ports are tracked as issues.
- Keep state at the original's scope. A C++ global or static becomes a
  process-wide Rust static, never `thread_local!`: GUI builds run input and
  single-player commands on the main thread and the game loop on `ottd:game`,
  serialized by the game-state mutex. The `-vnull` harness runs one thread and
  cannot see the difference.
- Call shared services directly. Rust calls `Random`, map and pool accessors and
  other shared services through `noexcept` C++ wrappers and keeps the original's
  control flow, so a port reads like the C++ body it replaces. Environmental
  failures are not simulation behavior: allocation failure, I/O errors in debug
  or log output, and paths that need developer-only defines such as
  `RANDOM_DEBUG`. An exception escaping a wrapper terminates. Return control to
  C++ (an action protocol) only where a C++ exception can unwind through the
  call during ordinary play (script VMs, save/load errors), and name each such
  service. Reentry alone is not a reason: end every Rust borrow before a
  callback that can reenter Rust or mutate its state, then call it directly.
- Port callees as callees. Where the original body calls a helper, a virtual
  method or a named constant, the Rust calls that same operation (its shared
  accessor, or the Rust port of that helper), not a hand-written equivalent. A
  raw field store in place of a setter, a hand-written bound in place of
  `TileAddWrap`, or a literal in place of `INVALID_RAILTYPE` diverges as soon as
  the helper does more than the copy. An inline helper or constant from an
  original header (`slope_func.h`, `map_func.h`, `tunnelbridge.h`, command
  flags, price indices) is ported once, into the shared Rust module named after
  that header, and every port imports it from there.
- Keep the boundary cheap and typed. Entries are plain synchronous calls with
  no async, future, task or mailbox machinery, and crossing the boundary
  allocates nothing per call. Where the original builds a local container, the
  port uses a local `Vec`, not process-wide unsafe scratch. Each service is its
  own typed `noexcept` function, not an opcode switch or positional array. Each
  entry resolves its owner once and reads, once, only the fields it uses:
  neither a whole-record view nor a call per field. Services return their real
  types on both sides and tables use designated initializers. A C++ class type
  (`PoolID`, `StrongType`) crosses as its base type, because MSVC returns class
  types through a hidden pointer (#355). Existing ports convert under #168 and
  #199. Allocation on a hot path or per call where the original has none,
  opcode dispatch, whole-record copies and a Rust-to-C++-to-Rust bounce into
  code Rust owns are design faults: they are fixed before integration whatever
  the timing, and root grants no exceptions.
- A shared accessor (vehicle field, map predicate, setting, constant) has one
  typed definition used by every port. `tools/shared-accessors.py` rejects new
  copies in quick CI (#249). It cannot see Rust service-table fields, macros or
  compound callbacks, so a reviewer greps the callee of every new table-bound
  wrapper (such as `CheckOwnership` or `GetRoadOwner`) across `src/` and reports
  each existing wrapper of it as a finding. Command ports share one set of
  command-check services. A PR that adds a shared accessor deletes the
  integrated copies, and a PR that removes a caller deletes the export,
  declaration or callback field it orphans.
- The selected #274 shared map descriptor is limited to audited scheduler clear,
  water and tree loops (`docs/design/world-state.md`). C++ retains canonical
  arrays and writes. Shared header helpers preserve native assertions; raw
  field loads end before callbacks and descriptor lifetime requires a proved
  no-reset/no-yield interval. Other entries keep typed services until audited.
- Deferral names an owned issue whose result can lift it; a re-selection
  condition that waits on another deferred or unscheduled item is circular. A
  core loop (tile loop, vehicle tick) that misses the retirement ratio gets a
  boundary fix, not abandonment.
- Evidence for game-logic ports is the semantic simulation harness (`python3
  tools/migration.py simulate`, #72) plus the existing tests; a new game-logic
  port integrates only after its harness scenarios exercise the component.
  Fidelity fixes to existing ports (such as #75) are not new ports and do not
  wait for it.
  Extend the harness's scenarios rather than writing a per-component comparison
  tool. Add a narrow comparison against unchanged reference bodies only for a
  concrete gap the harness cannot reach, and name that gap in the PR. Any check
  a doc or PR cites, whether a branch witness or a reference probe, runs in
  default CI and fails when missing; an opt-in profile or a `__main__`-only
  probe is not evidence.
- Keep the pinned reference worktree unchanged. Never change candidate behavior
  and expected results together merely to make checks pass. Existing test
  success establishes covered behavior, not complete game equivalence.
- Compare semantic state. Screenshots alone and compressed-save byte equality
  cannot establish equivalent simulation. Retain small failures and explain
  discrepancies before accepting changes.
- Record changes, checks and remaining limits in PRs. Migration documentation
  records design decisions and open limits, not evidence or review history.
  Scaffolding does not complete a subsystem.

## Speed

- Speed is checked on the merged branch; PRs run no speed check of their own
  (user decision, 2026-10-22). The ceilings are 1.5x the original for play saves
  and 1.3x for generation (#282).
- A projection is the anchor's measured ratio times (1 + 2 x the growth of the
  game loop's callgrind instruction count over the anchor), from
  `SPEED_ANCHORS`; counts do not depend on host load. After every second
  integration root runs `python3 tools/migration.py simulate --instructions` on
  the integrated tip in the background, alongside other work, and records the
  six projections in the roadmap's "Where the fork stands". Compare tips under
  the same anchor; after re-anchoring, recompute the previous tip from its
  retained count.
- If a projection rose more than 3% (provisional) since the previous tip
  profile, root attributes the rise per function to the PRs since and fixes it
  forward; that fix is the next maintenance PR to integrate. A ready ownership
  PR proceeds meanwhile; it does not wait for a future maintenance base.
- A speed pass runs only when a tip projection comes within 5% of its ceiling
  (1.425x play, 1.235x generation). It re-measures the six wall ratios on a
  quiet host (no builds, links or other timed games) as the median of four
  interleaved three-pair medians, and records them with their loop counts as the
  new anchors. If no measured ratio is in its band, the pass ends there.
  Otherwise it profiles that scenario with callgrind, counts boundary cost as
  the overhead beyond what the original spends inline on the same work, ranks
  those net costs (never gross callback counts), turns the two or three largest
  into maintenance PRs, then re-measures.
- A pass runs alongside other work: only its timed collection pauses builds,
  links, profiles and other games. Prepare runtimes before the hold and release
  it when the last timed game exits; parsing, attribution and reports run after
  release. Record process and load observations. An unrelated process crossing
  an uncalibrated CPU threshold is not, by itself, grounds to abort or reject a
  collection; keep genuinely contaminated samples out of anchors and record why.
  Wall medians are taken only in passes; one taken under load is not a speed
  result.

## Evidence budget

Evidence must be checkable, not exhaustive prose. Spend effort on code and on
checks that run; do not restate them in paragraphs.

- Issue: scope, affected interfaces, evidence plan and acceptance criteria;
  about 60 lines at most, leaving design detail to the implementation.
- PR description: what moved, what stays in C++, exact commands, known limits,
  and the output of `python3 tools/port-metrics.py` (the roadmap progress
  metric); about 60 lines at most. Integration PRs link the component PRs
  instead of re-describing them. `.github/PULL_REQUEST_TEMPLATE.md` and the
  migration issue form follow this budget.
- Component entry in `docs/rust-migration.md`: about 25 lines at most, saying
  what Rust owns, what stays in C++, open limits and the PR. Review status,
  witness counts and costs stay in the PR.
- Roadmap: forward-looking, about 200 lines at most. A completed item becomes
  one table row (issue, PR, commit, metrics); its evidence stays in the PR. The
  resume checkpoint is a short status table, not a log: CI run IDs, executable
  hashes, probe values and per-PR plans go in the PR or issue.
- Evidence is the exact-head CI runs plus the commands recorded in the PR.
  Comments and docs carry no receipt bundles, executable hashes or `.local/`
  paths, which nobody else can check. Tooling that a rule depends on, such as
  the speed-pass collector, is tracked in `tools/`; no script is copied from one
  `.local/` run to the next.
- Harness tooling for a port goes in that component's scenario module. Tooling
  is code to maintain; keep it proportionate to the C++ the port retires.
- Decide an ownership port's cost at its first concrete boundary or evidence
  checkpoint, with the ordinary retirement, net-glue and tooling metric: a
  bounded repair, or the necessary cost with a specific explanation. Do not turn
  estimates into separate implementation, native-test and tooling approval
  gates, or keep replanning solely to force a line ratio; source and boundary
  defects still require fixes. A new evidence requirement names the observable
  behavior existing checks cannot distinguish; tracing pure helpers only to
  reproduce the source call sequence is not required.
- Infrastructure work (CI, caches, evidence plumbing, not scenario or test
  support within an ownership port) states a tooling budget in its issue and
  prefers platform defaults and settings (LRU cache eviction, concurrency
  groups) over custom controllers. Root may accept a draft up to about 25% over
  its budget and may raise a budget once; a second raise is a `question` issue
  for the user. Its review fixes only defects that can change a game comparison
  or a speed decision on this host or in CI; hypothetical ones go in the PR's
  limits.
- Review report: reviewed commit, findings with their fixing commits (or why one
  went to root), and one line each naming the functions or files compared
  against the original body, any new callback that duplicates a shared accessor,
  and any WITH_RUST body, export or declaration the PR leaves without a caller
  (each one is a finding). Do not narrate further when there are no findings.
  Branches the harness does not reach go in the coverage tracker (#156), not
  only in the disposition.
- One PR per ownership port, targeting `rust-migration` directly. Small
  maintenance (cleanups, coverage additions, speed fixes, tooling and fidelity
  fixes) is batched: one PR carries every ready item, listed as a checklist on
  its tracking issue (#156, #275, #449-#452) rather than one issue each, with
  one reviewer and one Full run. An item that fails review or CI leaves the
  batch for the next one rather than holding it. A fix that a draft or a live
  crash needs may go alone. Batch ready, reviewed components in one integration
  PR when that avoids serial Full runs.

## Agent team and review

The root agent (`gpt-6.1-sol`, xhigh effort) orchestrates and delegates heavily,
owns component selection and integration, and spot-checks delegated diffs.
Target concurrency is six agents: the root plus five subagents, matching
`.codex/config.toml`. Respect the running host's actual limit; this document
cannot raise a session limit.

Every spawn must specify a model and reasoning effort rather than inherit them,
and every subagent runs at lower effort than root's xhigh:

- `gpt-6.1-sol`: most implementation and analysis, and all independent review;
  high effort unless a clearly bounded task justifies lower.
- `gpt-6-luna`: bounded mechanical work; low or medium effort.
- `gpt-6-astra`: all delegated planning at high effort.

Every agent-authored GitHub issue, PR, comment and review report must identify
the agent, exact model and reasoning effort, including artifacts authored by
root, for example `Agent: /root/implementation | Model: gpt-6.1-sol | Reasoning
effort: high`. A user-directed session outside Codex (for example Claude Code)
names its exact model and states its effort as reported by its host. When such a
session runs root, it keeps the same structure with its host's models: subagents
at lower effort than root, and a fresh reviewer per round. A Claude Code root
runs implementation and planning on its host's models and keeps independent
review on `gpt-6.1-sol` (high) through T3 `delegate_task`, so a reviewer's model
family differs from the implementer's.

Spawn a fresh agent for each task and name it after that task; its name is its
attribution. Do not reassign a finished agent to unrelated work. Each review
round, including a re-review, gets a fresh reviewer that reads the PR's earlier
attributed review reports. Close each agent when its task ends. At the host's
slot limit, root works through its own queue (integration, spot-checking fix
commits, roadmap) until a slot frees. Root leaves host state alone: no archiving
threads or editing host databases to free slots.

Sessions can end without warning. Implementation agents push their branch at
every passing milestone, and before a long check, handoff or expected usage
limit push a coherent unfinished checkpoint without waiting for validation,
keeping its PR draft with what is incomplete, which checks are pending or
failed, and the next action. A checkpoint is not integration evidence. Each
task keeps one status comment on its PR (or on its issue before the PR), edited
in place when it starts, reaches a milestone or stops. A PR's other comments
are review reports and root's integration note. A tracker issue's body is its
current list, edited when an item opens or closes; a task with a PR does not
also post its status on the tracker. A review that stops before recording
findings and a disposition counts as no review.

For substantive changes:

1. Create a fork issue with scope, affected interfaces, evidence plan and
   acceptance criteria. Give each task a clear owner.
2. Use an isolated branch and worktree based on `rust-migration`. The
   implementation agent opens a draft PR targeting `rust-migration`, linking the
   issue and recording exact validation commands and limitations. The pre-merge
   Full covers the platform matrix except macOS, which runs on `rust-migration`
   after every fifth integration (user decision, 2026-10-22; #357). After two
   hosted-capacity cancellations of a macOS run, root sets `CI_MACOS_HEAVY` to
   `blacksmith-6vcpu-macos-26` for the rerun and deletes it afterwards. Request
   `--profile platform` only to diagnose a suspected platform failure.
3. A separate reviewer examines the final commit and checks evidence. Fidelity
   comes first: it compares each replaced original body callee by callee before
   checking boundary rules. A port that retires more than about 1,000 original
   lines gets one reviewer per function group, in parallel. The reviewer commits
   each fix to the PR branch, reruns only the checks its fixes affect, and lists
   each finding with its fixing commit; a review without fixes reruns nothing.
   Root spot-checks the fix commits before integration. Findings that need a
   scope or policy decision go to root unfixed. Commits that only fix CI
   warnings or platform errors, or only record speed medians, need no new
   review: the implementer or root makes them and CI checks them. Updating a
   reviewed PR from its base needs no new review when CI passes and the update
   either has no conflicts in `src/`, `rust/`, `tools/` or `.github/`, or
   resolves each one by keeping both sides' lines unchanged (`git show
   --remerge-diff` removes only conflict markers; root records this on the PR).
   Otherwise a fresh reviewer checks and fixes only the conflict resolution.
4. Root integrates only the combination Full tested. Update a PR from base when
   preparing its final Full or resolving a blocking conflict, not continuously.
   Before the final Full, update the PR (or integration PR) from current
   `origin/rust-migration`, request `python3 tools/ci.py request PR --profile
   full` for that exact head, and wait for it. Only a successful Full at that
   exact head publishes the required `Full validation` status. Under GitHub's
   strict protection, a later push or a base move needs a new one; a clean
   textual merge, local build, older passing result or post-merge CI is no
   substitute. Root serializes final integration, uses `gh pr merge
   --match-head-commit` with the reviewed SHA, and never bypasses this gate. A
   failure leaves the shared branch unchanged. After integration, record
   completion and remove the worktree as described under Standards.

Keep at most six unintegrated component branches, counting drafts and WIP
(#157). Integrating reviewed work comes before starting a new component, and a
component starts from integrated `rust-migration`, not from an unmerged
component branch, unless the roadmap names the dependency. A shared accessor
or service is never such a dependency: the first PR to integrate adds it to the
shared layer in its final typed form, and other drafts adopt it when they
update from base. A shared service or shared-path fix (command protocol,
clearing, prices, restoration) that more than one draft needs is built once, as
root's own small PR from `rust-migration`; no draft builds a parallel copy.
Integrate drafts in order of readiness: none waits for another draft's
services, and a reviewed PR gains no new scope before it integrates.

Do not self-approve or imply that agents sharing GitHub credentials are
independent GitHub accounts. With shared credentials, publish an explicitly
attributed review report naming agent, exact model, reasoning effort, reviewed
commit, findings and disposition. Platform approval requires a reviewer with
separate credentials. The user authorizes ordinary issues, PRs and review
reports on this fork. Do not contact upstream maintainers or submit upstream
changes. This fork welcomes agent-generated work; upstream contribution
policies apply to upstream submissions.

### Automatic rollback and wind-down

A confirmed code regression on `rust-migration` stops further integrations.
When the breaking merge has only docs commits after it (or none), root removes
it automatically, without asking, using the rollback procedure in
`docs/rust-migration.md`. Preserve its work remotely, restore the merge's first
parent plus corrected docs, and report afterwards. This recovery is explicitly
authorized even during wind-down; it needs no fix PR or new CI wait. A runner
or API failure alone is not a code regression. If later code prevents this
bounded rollback, preserve the state and handle recovery before further
integrations; during wind-down, hand it off instead of starting repairs.

Wind-down is bounded by actions, not time estimates. Finish an edit already
underway and save its state, even if incomplete; finish an active review by
recording findings. Start no new implementation, review, repair round, build,
simulation, CI request or retry. Do not wait on CI. Merge only already-reviewed,
already-validated combinations that still meet step 4; a required base update
leaves the PR for resume. Perform an eligible automatic rollback, record each
unfinished item's next action and running jobs, report, and stop. Pending or
later CI results do not restart wind-down. Preserve unfinished worktrees and
branches for resume; cleanup applies only to verified merged work. The user's
explicit action scope overrides these defaults.

## Standards and commands

C/C++ uses tabs; Rust uses `cargo fmt`. The inherited commit checker examines
every commit's diff and title: fix style in the introducing commit, and use
supported prefixes such as `Add:`, `Change:`, `Fix:`, `Doc:` or `Update:`,
including on integration merges (`Merge:` is rejected). Use ASCII without tabs
in commit messages, including their bodies. Python tools pass
`uvx --from ruff==0.16.8 ruff check tools/` and
`uvx --from ruff==0.16.8 ruff format --check tools/`. Before pushing, run
`python3 tools/preflight.py --base origin/rust-migration`.

Comments say what the code cannot: a quirk of the original, an outside
constraint (MSVC ABI, random-draw order) or a measurement, in one line where
possible. Name the original callee or file; cite an issue only for an open gap.
Code carries no review defense, history or process words ("not a second X",
"no longer", "reviewed", dates). State a safety or lifetime contract once per
module or type; call sites note only what differs. Docs, issues, comments and
commit messages use full sentences, a space after every colon and between a
word and a number ("totals 99", not "totals99"), and "X of Y", not "X /Y".

A test fails when the behavior it names breaks: it runs production code against
an independent expectation (the original, a fixture or a table). Tests do not
match substrings of source, workflow or documentation text (parse it when its
structure is the behavior, as for workflow permissions), count incidental
structure, or check results at fixed historical commits, and `cfg(test)` never
changes a production layout or body. Judge redundancy by the behavior a test
can distinguish, not by how directly its assertions follow the implementation:
serialized-thread state tests must distinguish process-wide from thread-local
storage. Prefer one table-driven test to copies.

Rust changes require reproducible `cargo fmt --all -- --check`,
`cargo check --workspace --all-targets --locked`,
`cargo clippy --workspace --all-targets --locked -- -D warnings` and
`cargo test --workspace --locked`; the migration driver and CI enforce them.
Keep unsafe/FFI boundaries narrow and document safety, ownership, lifetimes and
panic behavior. Make overflow and integer-conversion behavior explicit and
faithful to the original. Avoid unrelated C++ warning cleanup or blanket
warning changes.

`docs/rust-migration.md` documents the driver (`python3 tools/migration.py
verify|build|tools|rust-checks|simulate`; `build` skips tests),
`tools/run-comparisons.py` (it picks up every `tools/*-comparison.py`), the
shared reference checkout and ccache. `--jobs` defaults to the free cores (8
minus the 1-minute load average, at least 2); `simulate` runs `jobs // 2`
scenarios at once. Cache build outputs freely when the key covers what
determines them; never cache test results. Builds, dependencies, toolchains and
reference worktrees belong in ignored locations.

Iterate locally and push reviewable batches. Locally, an agent runs the build,
the Cargo checks and the simulation scenarios its change touches; it does not
run the full default `simulate` suite (38 minutes locally), which the
exact-head Full runs in about 12. Add no second full local suite or pre-merge
local build gate. Ordinary PR pushes run only cheap checks, so code pushes do
not use `[skip ci]`; `python3 tools/ci.py request PR --profile
rust|native|platform` runs diagnostic subsets. CI wall time is development time
(#233): keep warm-cache Full within about 15 minutes by adding parallel jobs or
shards, never steps on one job's critical path, and keep caches under the
repository limit. Avoid docs pushes during final validation: they advance base.

A worktree and its local branch exist only while their PR is open, or before
the PR while the roadmap's work order holds the issue; an open PR keeps one.
Root removes a review or update task's own worktree once its commits are
pushed, and a merged PR's worktree and branch with `git worktree remove PATH`
and `git branch -d BRANCH` (both refuse dirty, locked or unmerged state);
GitHub deletes the merged head branch. When
work stops for good (closed, superseded, or paused with no next step), close
its draft PR, keep the branch on origin, record its head on the issue, and
remove the worktree and local branch without archiving or copying anything
out. `.local/`
holds only caches and rebuildable output: delete ad hoc output (CI artifacts,
probe runs, logs, superseded runtimes) once the PR or issue records it. Establish
file provenance before calling it user-owned; untracked files may be outputs
left by earlier agents and follow the same cleanup rule.

Docs-only changes (only `docs/` or `*.md` files, such as roadmap updates and
skill edits under `docs/skills/`) are committed directly to `rust-migration` as
a single commit, without a PR. Any agent, including a user-directed Claude Code
session, pushes its docs-only change as soon as it is done, without asking the
user first. Everything else (code, tools, workflows, saves) goes through a PR
that passes CI before it merges; never merge past CI with `--admin`. Preserve
upstream copyright notices, credits and GPLv2.
