# CI

Requesting and reading CI runs, and what costs money. Local builds and checks are in [migration tools](rust-migration.md); the cadence and bootstrap procedure are in [CI administration](root-workflow.md#ci-administration).

## Workflows

| File | Role |
| --- | --- |
| `ci-quick.yml` | Quick: the Cargo checks and the [cleanup gates](rust-migration.md#quick-cleanup-checks). Reports `Quick validation`. |
| `ci-request.yml` | Requested runs and the merge queue; publishes `Full validation` and `Validated tree`. |
| `rust-migration.yml` | Native build, component comparisons and simulation shards. |
| `ci-build.yml` | Platform builds, `Rust tests (32-bit)`, annotation checks and the platform comparison. |
| `ci-policy.yml`, `ci-invalidate.yml` | Whether a push needs heavy jobs; setting or carrying `Full validation` on PR pushes. |

## Jobs and profiles

Full is the merge gate; its builds are set by [ADR 0018](adr/0018-builds-before-every-merge.md). The job conditions in `ci-build.yml` and `ci-request.yml` are the source for this table.

| Job | `rust` | `native` | `platform` | `full` and merge queue | Cadence dispatch |
| --- | --- | --- | --- | --- | --- |
| Quick | yes | yes | yes | yes | no |
| Cargo checks job | yes | no | no | no | no |
| Native comparisons and simulation | no | yes | no | yes | no |
| Linux GCC SDL2 | no | yes | yes | yes | no |
| Linux Clang | no | yes | yes | yes | yes |
| Linux GCC Dedicated | no | no | yes | yes | yes |
| `Rust tests (32-bit)` | no | yes | yes | yes | yes |
| Emscripten | no | no | yes | yes | no |
| Windows x64 | no | no | yes | yes | yes |
| macOS arm64 Debug and Release | no | no | yes | no | yes |
| Platform comparison | no | no | no | no | yes |
| Annotation checks | no | yes | yes | yes | yes |

- No job builds or tests 32-bit Windows ([ADR 0007](adr/0007-no-32-bit-windows.md)). `Rust tests (32-bit)` runs the workspace tests for `i686-unknown-linux-gnu`; it builds no game.
- The cadence dispatch is `gh workflow run ci-build.yml --ref rust-migration`.
- `Full validation` is published only after every selected job succeeds.
- Keep a warm-cache Full within about 15 minutes, because every merge waits on it: add parallel jobs or shards, not steps on one job's critical path.
- A draft PR runs no Quick until it is marked ready or a profile is requested.
- A PR whose merge commit changes only Quick's checker scripts, the Clippy and Ruff debt files, `migration/lints/clippy.toml` and inert Markdown runs Quick alone in Full.

## Requesting CI

| Command | Purpose |
| --- | --- |
| `python3 tools/ci.py request PR --profile native --wait` | Request a run for the PR's current head and wait for it, started as one background job; also `rust`, `platform` or `full`, and `--head SHA` to refuse another head. |
| `python3 tools/ci.py wait PR` | Resume waiting on the newest request at the PR's head (or a request file). Exit 2 means still running at the timeout (default 5,400 seconds), not failed; it prints the command to resume. |
| `python3 tools/ci.py log RUN` | Read a failed run: each failed job's failing step, error lines and tail, while other jobs still run. Takes a run or job URL, a request file or `--pr N`; `--grep RE`, `--job NAME`, `--full`. Logs are cached under `.local/ci-logs/`; `wait` and `queue-wait` print the same summary on failure. |
| `python3 tools/ci.py state PR` | Read a PR's state in one call: head, mergeability, Quick and the newest requested run per profile at head, queue position, review reports (agent, disposition, reviewed SHA) and what `enqueue`'s current checks say; `--json` for scripts. `status PR` is the same; `status REQUEST.json` prints a request's state once. |
| `python3 tools/ci.py ready PR` | Mark a draft PR ready once a preflight stamp covers its head. |
| `python3 tools/ci.py enqueue PR` | Queue a PR ([integration](workflow.md#integration)). |
| `python3 tools/ci.py queue-wait PR` | Wait, as a background job, until a queued PR merges or leaves the queue. |
| `python3 tools/ci.py failures --since DATE` | Count failed runs. |

- Which profile to request, and when, is in [checks](workflow.md#checks).
- `request --profile full` is an optional pre-check; the queue's Full is the merge gate. A failing pre-check holds the PR out of the queue until its next push.

## Merge queue

Every merge goes through GitHub's merge queue ([ADR 0019](adr/0019-merge-queue.md)).

| Setting | Value |
| --- | --- |
| Required checks on `rust-migration` | `Commit checker`, `Script missing mode enforcement`, `Unused strings`, `Quick validation`, `Full validation` |
| Queue ruleset | `rust-migration merge queue`: merge method `MERGE`, check timeout 90 minutes, at most 5 entries, grouping `ALLGREEN` |

- A pushed PR head's `Full validation` is set successful with the description `Full runs in the merge queue`, a placeholder so the PR can enter the queue; it is not a run.
- A PR that adds, changes, renames or deletes a file under `.github/workflows/` stays pending: it needs a reviewed bootstrap Full first ([CI administration](root-workflow.md#ci-administration)).
- The queue runs Full on the group commit (base, the PRs ahead and this PR) and merges it.

### Carried validation

When a new commit's tree, outside inert Markdown, matches one that already passed Full, the result is copied instead of run again: on a PR push within seconds, provided the new head contains the current base, and in a rebuilt merge group from the earlier group run of the same PR head. A carry waits up to 45 minutes for that earlier run, so the queue's check timeout must exceed 45 minutes plus a Full.

## Protected pushes

- A Markdown-only push starts no build; Quick still runs.
- A push whose non-Markdown tree already passed Full only refreshes compiler caches; any other push runs the heavy jobs.

## Runners and variables

Who decides which jobs run on paid Blacksmith runners is in [ADR 0009](adr/0009-paid-runner-assignment.md); a change that raises paid CI cost needs the user's approval first ([ADR 0008](adr/0008-paid-ci-cost-needs-approval.md)). The repository is public, so GitHub-hosted minutes are free and Blacksmith buys speed only.

| Variable | Selects | Current value | When unset |
| --- | --- | --- | --- |
| `CI_LINUX_HEAVY` | Native build and platform-reference runner | `blacksmith-8vcpu-ubuntu-2404` | `ubuntu-24.04` |
| `CI_LINUX_SIMULATION` | Simulation shard runner | `blacksmith-4vcpu-ubuntu-2404` | `ubuntu-24.04` |
| `CI_LINUX_COMPARISONS` | Component comparison runner | unset | `ubuntu-24.04` |
| `CI_LINUX_PLATFORM` | Linux Clang runner | unset | `ubuntu-24.04` |
| `CI_WINDOWS_HEAVY` | Windows runner | `blacksmith-4vcpu-windows-2025` | `windows-2022` |
| `CI_MACOS_HEAVY` | macOS runner | unset | `macos-latest` |
| `CI_MERGE_QUEUE` | [Merge-queue](#merge-queue) behaviour; set it together with the ruleset | `true` | queue behaviour off |
| `CI_ON_DEMAND` | Requested CI: PR pushes skip automatic heavy jobs and `ci:full` label requests are refused | `true` | every PR push runs the heavy jobs |
| `MIGRATION_CCACHE` | Nothing: no workflow or tool reads it | `true` | no effect |

- Values read from `gh variable list`; check it before relying on a value.
- Profile runner sizes with `blacksmith jobs list|steps|stats`, `jobs aggregate` and `jobs diagnose rightsize`. Use an org token, not a person's `~/.blacksmith/credentials`.

## Caches

- `ci-cache.yml` bounds total cache use to 8 GiB under GitHub's 10 GiB limit.
- Requested dispatches share the `rust-migration` ref's cache-write scope with protected pushes. That is accepted only while every PR comes from the fork's own agents; before the fork accepts outside PRs, enforce candidate cache-write separation by ref or service policy.

## Native validation

`rust-migration.yml` builds once, then runs the component comparisons and three simulation shards in parallel. Reports and failing saves are artifacts.

## Platform comparison

The cadence dispatch compares Windows x64 and macOS arm64 Release against Linux for a subset of saves (`tools/simulate.py platform`).
