# CI

Read this when you request or interpret a CI run, or change a workflow, runner or CI variable. Local builds and checks are in [migration tools](rust-migration.md). Preflight, stamps and checkpoint pushes are in [workflow](workflow.md#commits-and-pushes); queueing a PR is in [integration](root-workflow.md#integration); the cadence and the bootstrap procedure are in [CI administration](root-workflow.md#ci-administration).

## Workflows

| File | Role |
| --- | --- |
| `ci-quick.yml` | Quick: the four Cargo checks and the [cleanup gates](rust-migration.md#quick-cleanup-checks). Reports `Quick validation`. |
| `ci-request.yml` | Requested runs and the merge queue: plans the run, calls the others, publishes `Full validation` and `Validated tree`. |
| `rust-migration.yml` | Native validation: the native build, component comparisons and simulation shards. |
| `ci-build.yml` | Platform builds: Linux flavours, Emscripten, macOS, Windows, `Rust tests (32-bit)`, annotation checks and the platform comparison. |
| `ci-linux.yml`, `ci-macos.yml`, `ci-windows.yml`, `ci-emscripten.yml` | One platform build each, called by `ci-build.yml`. |
| `ci-policy.yml` | Decides whether a push needs heavy jobs or is already covered by an earlier Full. |
| `ci-invalidate.yml` | Sets, carries or skips `Full validation` on each PR push. |
| `ci-cache.yml` | Bounds total GitHub cache use. |
| `commit-checker.yml`, `unused-strings.yml`, `script-missing-mode-enforcement.yml` | Inherited cheap checks. |

## Jobs and profiles

A profile is the scope of a requested run. Full is the merge gate; the set of builds it runs is decided in [ADR 0018](adr/0018-builds-before-every-merge.md).

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

- The source is the job conditions in `ci-build.yml` and `ci-request.yml`; check there before relying on a cell.
- No job builds or tests 32-bit Windows ([ADR 0007](adr/0007-no-32-bit-windows.md)). `Rust tests (32-bit)` runs Clippy and `cargo test --workspace` for `i686-unknown-linux-gnu` on a free runner, after requiring a 32-bit pointer width. It is a test proxy for the crate's pointer-width behaviour, not a game build, MSVC linkage or Emscripten runtime check.
- `native` builds Clang so its annotation check sees Clang-only C++ test warnings before the queue, and runs `Rust tests (32-bit)` so a 32-bit-only Clippy finding fails before the queue.
- The cadence dispatch is `gh workflow run ci-build.yml --ref rust-migration`; when it runs is in [CI administration](root-workflow.md#ci-administration).
- A Full publishes `Full validation` only for the captured PR head, and only after every selected job succeeds; a skipped or cancelled job does not qualify.
- Keep a warm-cache Full within about 15 minutes, because every merge waits on it: add parallel jobs or shards, not steps on one job's critical path. Keep caches under the repository limit.

## Quick

- Quick runs on pushes to a ready PR, on protected pushes, and inside every requested run that is not [carried](#carried-validation); a draft PR runs none until it is marked ready or a profile is requested. Its steps are in `ci-quick.yml`; preflight runs the same steps locally.
- Quick uses no local action, because on a pull request it checks out the PR head and an older branch lacks a newer local action. It holds an inline copy of the `apt-install` script; `tools/test-ci.py` keeps the copy identical.
- A requested profile's Quick takes the tested merge commit's first parent as its base, not the PR's recorded base SHA, which can predate later merges into base and would count their changes as the PR's. Quick's own pull-request run tests the PR head against the event's base SHA.

### Checker-only PRs

- A checker-only PR's merge commit changes only Quick's checker scripts, the Clippy and Ruff debt files, `migration/lints/clippy.toml` and inert Markdown. The plan in `ci-request.yml` lists the paths and decides from the merge commit and its first parent, failing closed.
- No native or platform job reads those files, so its Full runs Quick alone and publishes no `Validated tree` marker; the post-merge run still builds the merged tree. A checker-only merge group never carries.

### Inert Markdown

- Inert Markdown is `AGENTS.md` and `docs/**.md`, except the docs CMake packages for the in-game viewer (the `builtDocs` list in `ci-policy.yml`). Every other Markdown file, root-level `README.md` included, is a build input.
- The checker-only plan, carried validation and the post-merge Markdown-only skip share this rule; `tools/test-ci.py` checks the list against CMake. When a push may skip preflight is in [workflow](workflow.md#commits-and-pushes).

## Requesting CI

| Command | Purpose |
| --- | --- |
| `python3 tools/ci.py request PR --profile rust\|native\|platform\|full` | Request a run for the PR's current head; see the table above. It writes a request file under `.local/ci-requests/`. `--wait` waits for it; `--bootstrap` and `--unstamped` are covered in [CI administration](root-workflow.md#ci-administration) and [checks](workflow.md#checks). |
| `python3 tools/ci.py wait .local/ci-requests/REQUEST.json` | Wait for a request to finish; prints its resume command on timeout. |
| `python3 tools/ci.py status .local/ci-requests/REQUEST.json` | Print a request's state once. |
| `python3 tools/ci.py ready PR` | Mark a draft PR ready once a preflight stamp covers its head. |
| `python3 tools/ci.py enqueue PR` | Queue a PR; its gates and `--retry` are in [integration](root-workflow.md#integration). |
| `python3 tools/ci.py queue-wait PR` | Wait, as a background job, until a queued PR merges or leaves the queue; exit codes are in [integration](root-workflow.md#integration). |
| `python3 tools/ci.py failures --since DATE` | Count failed runs, leaving out `control/` branches (the negative controls); `--controls` counts them too. |

- Which profile to request, and when, is in [checks](workflow.md#checks).
- `request --profile full` is an optional pre-check of a PR head; the queue's Full is the merge gate. It ignores the queue status on the head, and its pending and final statuses replace it, so a failing pre-check holds the PR out of the queue until its next push.

## Merge queue

Every merge goes through GitHub's merge queue ([ADR 0019](adr/0019-merge-queue.md)).

| Setting | Value |
| --- | --- |
| Required checks on `rust-migration` | `Commit checker`, `Script missing mode enforcement`, `Unused strings`, `Quick validation`, `Full validation` |
| Other protection | Resolved conversations required; the branch need not be current with base; force pushes and deletion disabled |
| Queue ruleset | `rust-migration merge queue`: merge method `MERGE`, check timeout 90 minutes, at most 5 entries built and merged, grouping `ALLGREEN` |
| `CI_MERGE_QUEUE` | `true`; set it together with the ruleset. Unset, the queue-specific behaviour below is off. |

- **Queue status on PR heads.** The invalidation workflow marks a pushed PR head's `Full validation` successful with the description `Full runs in the merge queue`, so the PR can enter the queue without its own Full. A carried result takes precedence.
- **Workflow changes.** The head stays pending when the PR adds, changes, renames or deletes a file under `.github/workflows/`, or when its file list cannot be read in full (3,000 files or more). The queue runs the group commit's own workflow definitions with write permissions, so such a PR needs a reviewed bootstrap Full first ([CI administration](root-workflow.md#ci-administration)). Neither the plan nor `tools/ci.py` accepts an ordinary requested Full for it.
- **Group run.** The `merge_group` event runs Full on the group commit: the base, the PRs queued ahead and this PR. Quick takes the group commit's first parent as its base, so a second queued PR's gates and commit check count only its own changes. The acceptance gates read the queued PR's comments through the group commit's second parent, which the plan requires to be the PR's current head.
- **Required cheap checks.** `Commit checker`, `Unused strings`, `Script missing mode enforcement` and `Quick validation` also trigger on merge groups and skip there, which counts as passed; Full's own Quick runs them on the group commit.
- **Merge.** `Full validation` and `Validated tree` go on the group commit, which the queue merges. The post-merge policy reads them on the pushed commit, then on merged PR heads, and trusts only statuses `github-actions[bot]` created, so a queue merge only refreshes caches. A failed plan fails the group at once instead of waiting for the queue's timeout.

### Carried validation

Carried validation copies an earlier passing Full to a new commit whose non-Markdown tree is unchanged, instead of running Full again. The tree comparison is the complete-inventory rule of the post-merge cache refresh.

- **PR pushes.** When a push keeps the tree, outside inert Markdown, that the previous head passed Full on, the invalidation workflow copies that `Full validation` and `Validated tree` to the new head within seconds. Examples: an update from base over docs-only commits, or a Markdown-only push. The new head must contain the current base, so the merged tree is the tested one. Any other push, a previous head without a same-run passing result, or a failed lookup leaves the status pending.
- **Merge groups.** A direct push to the base, or a PR removed ahead, rebuilds each queued group and starts a new run; the earlier run continues to its end, and merge-group runs never cancel one another.
- The new run's plan lists earlier merge-group runs of `ci-request.yml` for the same PR, oldest first, and takes the first whose group commit merges the same PR head with the same non-Markdown tree. It waits for that run if it is still going.
- If that run passed, with trusted `Full validation` and `Validated tree` statuses for the same tree inventory, and the result names the run that built that tree, the new run skips Quick, native and platform and publishes that result with the building run's URL. The building run is that run or, through a chain of carries, an older successful merge-group run for the same PR head whose group tree is the marker's tree.
- A failed lookup, a failed or cancelled earlier run, or one still going after 45 minutes means a normal run. The queue's check timeout must therefore exceed 45 minutes plus a Full.

## Protected pushes

- Protected pushes queue without cancelling a running validation; a newer pending push replaces an older one.
- A push that changes only Markdown (any `**.md`) starts no build: `ci-build.yml` and `rust-migration.yml` ignore it. Quick still runs.
- A push whose non-Markdown tree already passed Full (covered) only refreshes eligible compiler caches, without repeating tests. An uncovered push, or one whose inventory is incomplete, runs the heavy jobs, macOS included. Post-merge checks never authorize an untested combination.

## Native validation

- `rust-migration.yml` builds once, then runs the component comparisons and three simulation shards in parallel, from frozen runtimes and run-scoped artifacts. The shards run `tools/ci_native.py shard`, not `migration.py simulate`.
- Reports and failing saves are artifacts; build inputs expire after one day.

## Runners and variables

Who decides which jobs run on paid Blacksmith runners is in [ADR 0009](adr/0009-paid-runner-assignment.md); a change that raises paid CI cost needs the user's approval first ([ADR 0008](adr/0008-paid-ci-cost-needs-approval.md)). The repository is public, so GitHub-hosted minutes are free and Blacksmith buys speed only.

| Variable | Selects | Current value | When unset |
| --- | --- | --- | --- |
| `CI_LINUX_HEAVY` | Native build and platform-reference runner | `blacksmith-8vcpu-ubuntu-2404` | `ubuntu-24.04` |
| `CI_LINUX_SIMULATION` | Simulation shard runner | `blacksmith-4vcpu-ubuntu-2404` | `ubuntu-24.04` |
| `CI_LINUX_COMPARISONS` | Component comparison runner | unset | `ubuntu-24.04` |
| `CI_LINUX_PLATFORM` | Linux Clang runner | unset | `ubuntu-24.04` |
| `CI_WINDOWS_HEAVY` | Windows runner | `blacksmith-4vcpu-windows-2025` | `windows-2022` |
| `CI_MACOS_HEAVY` | macOS runner; the capacity fallback is in [CI administration](root-workflow.md#ci-administration) | unset | `macos-latest` |
| `CI_MERGE_QUEUE` | [Merge-queue](#merge-queue) behaviour | `true` | queue behaviour off |
| `CI_ON_DEMAND` | Requested CI: PR pushes skip automatic heavy jobs, the invalidation workflow runs, and `ci:full` label requests are refused | `true` | every PR push runs the heavy jobs; a `ci:full` label starts a bootstrap Full |
| `MIGRATION_CCACHE` | Nothing: no workflow or tool reads it | `true` | — |

- Values read from `gh variable list`. Check it before relying on a value; this table does not update itself.
- The cached GCC flavours, Linux GCC SDL2 and GCC Dedicated, always use free `ubuntu-24.04` (`ci-linux.yml`).
- Profile runner sizes with `blacksmith jobs list|steps|stats`, `jobs aggregate` and `jobs diagnose rightsize` (`--job-name` for the last two). Use an org token, never a person's `~/.blacksmith/credentials`.

## Caches

The local caching rule is in [compiler cache](rust-migration.md#compiler-cache).

- CI cache keys cover OS, architecture, baseline, compiler, ccache and configuration.
- Cache roles are trimmed to 750 MB native, 600 MB per Linux flavour, 650 MB per macOS flavour and 500 MB for Emscripten; `ci-cache.yml` bounds total use to 8 GiB under GitHub's 10 GiB limit.
- Windows and macOS also cache vcpkg binaries and use ccache without PCH (version pinned in their workflows). Windows vcpkg keys are per commit and restore the newest prefix match; a job saves only when it stored new archives. After a successful build it prunes to the installed ABIs when every ABI file can be hashed and matched to an archive; otherwise it warns and saves without pruning.
- MSVC uses `/Z7` (CMP0141). Emscripten builds without PCH and wraps emcc's clang with the container's ccache through `EM_COMPILER_WRAPPER`.
- Requested runs use a `candidate` cache namespace that protected runs' restore keys exclude. Key names do not limit a job's cache-write authority: requested dispatches share the `rust-migration` ref scope with protected pushes. That is accepted only while every PR comes from the fork's own agents; before the fork accepts outside PRs, enforce candidate cache-write separation by ref or service policy.

## System packages

- Linux jobs install apt packages through `.github/actions/apt-install`, directly or through `native-environment`; Quick holds its inline copy. The first two attempts are bounded, and a failed or stalled attempt retries without the Azure mirror.
- The Emscripten container installs `curl` with a bare `apt-get` before Rust. Windows and macOS jobs install no apt packages.

## Platform comparison

- The cadence dispatch compares Windows x64 and macOS arm64 Release against Linux for three manual play saves, `generate-tgp-128-1` and `aircraft-route-manual`.
- Prepare with `python3 tools/simulate.py platform --reference-binary PATH --output DIR`; consume with `python3 tools/simulate.py platform --stored-reference DIR --candidate PATH --output OTHER`. Missing or mismatched inputs fail.
- The cases keep full tick intervals, normalize GLOG, set `linkgraph.recalc_time` to 9000 and move job joins to the initial economy date plus 4500 days. They use synchronous saves and identical OpenGFX, and every checkpoint requires joins after the fixed end date.
- The subset excludes link-graph joins, cargodist, regression AIs, custom console adapters, GUI scheduling and networking. It remains #509 acceptance work until root runs the changed workflow.
