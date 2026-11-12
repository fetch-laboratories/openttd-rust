# Decision records

Readers: every agent, before changing a rule that links here or reversing a past choice; the user, when asked to approve one.

An ADR (architecture decision record) is a short file that says what was decided, why, and who may change it. The rule docs hold the working detail: commands, thresholds and procedures. An ADR holds the reason, so that a later reader can judge a change without digging through history.

## When to write one

Write an ADR for a decision that meets all three tests:
- it is hard or costly to reverse, in money, agent-hours or disk;
- a later reader would question it without the context;
- it came from a real trade-off, with a cost on each side.

An ordinary working rule does not get one. It lives in its rule doc, and agents may change it there.

## Who may change one

Each ADR's front matter has a `may change:` line.

- **`user`**: the agent asks the user first. It says plainly that it is reversing a recorded decision, names the ADR by number and header, and gives the reason and the cost.
- **`agents`**: an agent may change it by writing a replacement ADR with a measured reason.

A change that raises paid CI cost needs the user's approval under [ADR 0008](0008-paid-ci-cost-needs-approval.md), whoever owns the ADR. A change that lowers it does not.

`may change:` records who owns a decision from now on. It does not claim who made it; each ADR's body says what the record shows.

## How the user sorts ownership

A decision is the user's if it sets the goal or scope, spends money, depends on this host, or is legal. Everything else belongs to the agents. Agents run the project; the user guides it.

## How to change one

1. Write a new ADR with the next free number. Never reuse a number.
2. Give it `status: accepted` and say what it replaces and why, with the measured reason.
3. In the old ADR, change only the status line, to `status: superseded by ADR NNNN`. The old file stays.
4. Update the rule docs that link to the old ADR.

## How to refer to one

By number and header, with a link to the file, for example [ADR 0010](0010-two-implementers.md), "At most two implementers work at once". Never refer to a decision by its date, such as "the 11-10 decision".

## The records

### The user's

| ADR | Header |
| --- | --- |
| [0001](0001-full-rust-rewrite.md) | The whole game is rewritten in Rust, and the Rust build is the only build |
| [0002](0002-original-game-is-the-specification.md) | The original game defines correct behaviour; improvements wait |
| [0003](0003-work-order-cleanup-readers-storage.md) | Work order: cleanup, then readers, then storage |
| [0004](0004-cleanup-before-porting.md) | Porting stays paused until the cleanup is finished |
| [0005](0005-plan-only-the-next-phase.md) | Agents plan only the next phase |
| [0006](0006-bundled-libraries-and-squirrel.md) | Bundled libraries are replaced by Rust libraries where they match; Squirrel is decided later |
| [0007](0007-no-32-bit-windows.md) | 32-bit Windows is not supported |
| [0008](0008-paid-ci-cost-needs-approval.md) | A change that raises paid CI cost needs the user's approval |
| [0009](0009-paid-runner-assignment.md) | The user decides which CI jobs run on paid runners |
| [0010](0010-two-implementers.md) | At most two implementers work at once |
| [0011](0011-model-and-effort-per-role.md) | Each agent role has a fixed model and effort level |
| [0012](0012-root-decides-implementers-review-and-merge.md) | Root chooses work and settles questions; implementers run their own reviews and merges |
| [0013](0013-stocktake-every-12-merges.md) | Root takes stock after every 12 merges |
| [0014](0014-speed-ceilings.md) | The Rust game may cost at most 1.6 times the original in play and 1.3 times in map generation |
| [0015](0015-third-party-test-content.md) | Other people's test content is never committed or uploaded |
| [0016](0016-automatic-rollback.md) | Root rolls back a merge that breaks the main branch without asking |
| [0017](0017-deep-checks-on-request.md) | Deep checks run only on the user's request, plus at the end of cleanup and of the reader phase |

### The agents'

ADRs 0018 to 0020 also fall under [ADR 0008](0008-paid-ci-cost-needs-approval.md): a change that raises paid CI cost needs the user's approval.

| ADR | Header |
| --- | --- |
| [0018](0018-builds-before-every-merge.md) | Clang and the GCC dedicated server build run before every merge; macOS runs after every fifth merge |
| [0019](0019-merge-queue.md) | Every merge goes through the merge queue |
| [0020](0020-scenario-comparisons-on-ci.md) | Scenario comparisons run on CI, not on this machine |
| [0021](0021-integer-wrappers-not-rust-enums.md) | Rust wraps the original's integer types instead of using Rust enums |
| [0022](0022-progress-is-net-cpp-removed.md) | Progress is measured by net lines of C++ removed |
| [0023](0023-cpp-tests-stay-until-code-is-ported.md) | C++ tests of C++ code stay in C++ until that code is ported |
| [0024](0024-lint-ratchet-scope.md) | The lint check enforces only parameter count, safety documentation and production assertions |
| [0025](0025-two-review-stages.md) | A pull request gets at most two review stages |
| [0026](0026-roadmap-commit-timing.md) | Root commits the roadmap only at a stocktake, a change of course or wind-down |
| [0027](0027-speed-measured-on-merged-branch.md) | Speed is measured on the merged branch, not on each pull request |
