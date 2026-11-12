# OpenTTD-Rust migration tools

Read this when you build the games, run local checks or use the comparison harness. CI is in [CI](ci.md), rules are in [AGENTS.md](../AGENTS.md) and ports are in the [component index](components/README.md).

## Repository and reference

| Item | Location |
| --- | --- |
| Fork and `origin` | https://github.com/fetch-laboratories/openttd-rust |
| `upstream` | https://github.com/OpenTTD/OpenTTD.git |
| Integration branch | `rust-migration` |
| Original | OpenTTD 15.3, `14ec60f248547d4d062a1160f0fc26d742319888`, pinned in `migration/baseline.json` |
| Shared reference worktree | `.local/reference/openttd` in the main checkout |
| Shared reference build | `.local/build-reference` in the main checkout |
| Candidate build | `build-rust` in each worktree |
| Rust crate | `rust/openttd-kernels`, linked into the game, tests and native generators |

- Direct CMake builds need the pinned commit object for the native-test oracle headers: `git fetch --no-tags --depth=1 origin 14ec60f248547d4d062a1160f0fc26d742319888`. CI fetches it before configuring.
- The driver creates the detached reference worktree and rejects a dirty source or a wrong revision. Updating the baseline is a separate task.
- Branches and PRs: [workflow](workflow.md#branches-and-worktrees).

## Prerequisites and host

Native Linux verification needs a C++20 compiler, CMake, Ninja, Python 3.11 or newer, SDL2 development files, the libraries in `COMPILING.md`, OpenGFX and the Rust toolchain pinned in `rust-toolchain.toml` (CI installs the same one). Commercial assets are unnecessary. Node.js runs `tools/test-ci.py`; an authenticated `gh` sends CI requests.

On Ubuntu with administrator access:

```sh
sudo apt-get install build-essential cmake ninja-build python3 pkg-config \
  libsdl2-dev liblzma-dev libpng-dev libcurl4-openssl-dev libfreetype-dev \
  libfontconfig-dev libharfbuzz-dev libicu-dev liblzo2-dev openttd-opengfx
```

- This host runs Ubuntu 24.04 x86-64, as CI's `ubuntu-24.04` runners do, with the packages above plus `valgrind`, `ccache`, `unifdef` and `gdb` from apt.
- Keep `.local/deps` absent here: the driver puts it ahead of the system on every compiler, library and pkg-config path.
- `tools/bootstrap-local.py` is for an Ubuntu 26.04 host without administrator access. It extracts that release's packages to `.local/deps` and installs Rust in `.local/cargo` and `.local/rustup`, changing no shell profile or system package. It takes no arguments and starts installing at once, even when given `--help`.
- Output built on the earlier Ubuntu 26.04 host needs glibc 2.43; it is kept unused at `/mnt/storage/openttd-rust/old-host-ubuntu2604-20261105/`.

## Build and verification

| Command | What it does |
| --- | --- |
| `python3 tools/migration.py rust-checks` | Runs the four Cargo checks without building the reference. |
| `python3 tools/migration.py build` | Builds both games without tests. |
| `python3 tools/migration.py verify` | Runs the Cargo checks, builds both graphical executables and runs both CTest suites. Fails on an empty test inventory, missing candidate test names or invalid Rust target metadata. |
| `python3 tools/migration.py tools` | Builds the native Rust generators in `.local/build-tools-rust` with `OPTION_TOOLS_ONLY=ON`. |
| `python3 tools/migration.py simulate`, `coverage`, `deep` | See [simulation comparison](#simulation-comparison), [coverage](#original-simulation-coverage) and [deep checks](deep-checks.md). |
| `python3 tools/run-comparisons.py` | Runs every `tools/*-comparison.py` in parallel and prints case inventories. |
| `eval "$(python3 tools/migration.py env)"` | Sets the driver environment for Cargo and binaries run outside the driver. |
| `python3 tools/git_hooks.py install` | Writes the pre-push and pre-commit shims into the clone's shared hooks folder; [workflow](workflow.md#commits-and-pushes) says what they enforce. |

- The four Cargo checks are `cargo fmt`, `cargo check`, `cargo clippy` and `cargo test`, defined in `command_rust_checks` and `clippy_command` in `tools/migration.py`. Run Clippy through the driver: the ratchet repeats the gate's exact command so Cargo replays its result, and a hand-run `cargo clippy` with other arguments makes the next gate check the crate again.
- `--jobs` defaults to `max(2, cores - round(1-minute load))`, read once at start. It also limits Cargo under CMake.
- Logs, test reports and the source revision go to `.local/verification/<timestamp>/`. A pass establishes covered behaviour, not whole-game equivalence.
- The candidate is `build-rust/openttd-rust`; Cargo output is in `build-rust/cargo`. The reference is `.local/build-reference/openttd` in the main checkout.
- Bootstrapped dependencies give candidate binaries an RPATH, so `./build-rust/openttd-rust -X`, `./build-rust/openttd_test` and `ctest` in `build-rust` run from a plain shell. `-X` avoids the global game folders; use a separate development configuration and saves.
- Every fork build requires Rust. Supported: native GNU/Linux x86-64, macOS arm64, Emscripten and the scoped [Windows MSVC mode](#windows-msvc). The build system also accepts native GNU/Linux `aarch64-unknown-linux-gnu`, which no CI job exercises.
- Unsupported: 32-bit Windows ([ADR 0007](adr/0007-no-32-bit-windows.md)), other platforms, cross compilation, macOS universal or Intel. CI runs the Rust workspace tests for `i686-unknown-linux-gnu` as a [32-bit test proxy](ci.md#jobs-and-profiles); no game is built for that target.

### Quick cleanup checks

[Quick](ci.md#quick) runs the four Cargo checks and then these gates. `python3 tools/preflight.py --base origin/rust-migration` runs all of them locally; [workflow](workflow.md#commits-and-pushes) says when. The gates preserve the original's casts and control flow: a faithful truncation must not become a checked conversion to reduce debt.

| Command | Failure distinguished |
| --- | --- |
| `python3 tools/lint-ratchet.py clippy --base-ref origin/rust-migration` | Growth against the merge base for the force-warned Clippy lints: a new item over its threshold, or more or larger diagnostics on an item. Improvements never fail. |
| `python3 tools/lint-ratchet.py ruff --base-ref origin/rust-migration` | The same ratchet for C901 and PLR0911/12/13/15 over `tools/`. |
| Quick's Python step in `ci-quick.yml` (Ruff, `ty`, then every `tools/test-*.py`) | CI stops at the first failure, so a `ty` error hides failures in the test scripts; preflight runs every step. |
| `python3 tools/lint-policy.py --base-ref origin/rust-migration` | Weaker selections, broader exclusions or per-file ignores, changed thresholds or tool versions, and new Rust `allow`/`expect` or Python `noqa` sites. Unclassified policy edits fail for review. |
| `python3 tools/growth-ratchet.py --base-ref origin/rust-migration` | Independent growth in tools C++, fork-added `src/` tests C++, Rust exports and callback fields. Generated records and other C++ outside `src/` have separate counters, so a move cannot exempt growth. |
| `python3 tools/refactor-proof.py --base-ref origin/rust-migration` | A claimed release-archive equivalence fails on changed code, relocation targets or addends, read-only data, symbol offsets or section alignment. |
| `python3 tools/code-standards-inventory.py --gate origin/rust-migration` | A file in `migration/finished-files.txt` gaining a raw adapter or export call outside export shims and tests, an internal `execute: bool` parameter, a `GameState<Vec<...>>` scratch pool or a command over seven parameters. It counts each class per file against the merge base, so a move or rename passes; a listed file missing at head fails. Each file pass adds its files to the list; edits to it merge mechanically. |
| `python3 tools/test-cleanup-checks.py` | Seeded violations show that each gate catches its own case, including all four Cargo commands and separate code, data and relocation changes. |

The harness inventory gate and the shared-code gates are under [simulation comparison](#simulation-comparison) and [shared-code checks](#shared-code-checks-and-port-metrics).

#### Lint debt

- Baselines are `migration/clippy-debt.json` and `migration/ruff-debt.json`. A key names the file, the innermost named item and the lint, with diagnostic multiplicity and measured magnitude. The ratchet compares the head with the debt file at the merge base.
- `--write` records the measured debt whenever nothing grew. `--initialize` is an explicit first inventory, never a CI update. A compiler or linter upgrade needs an explicit inventory migration.
- Clippy uses the pinned Rust and force-warns the lints in `CLIPPY_DEBT_LINTS` (`tools/migration.py`): `too_many_arguments`, `missing_safety_doc` and `disallowed_macros`. Force-warned lints stay warnings under `-D warnings`, so debt never fails the Cargo check itself. The run also compiles `native-test-support` callers.
- `disallowed_macros` counts production `assert*!`, `debug_assert*!`, `panic!` and `unreachable!` sites, listed in `migration/lints/clippy.toml`, from the non-test library unit, which the ratchet replays with `--lib`; tests assert legitimately.
- A lint family that a change adds to `CLIPPY_DEBT_LINTS` starts from that change's recorded rows, its reviewed first inventory, written with `--initialize`. Every family the merge base already counts keeps the base allowance.
- Function length, nesting and cognitive complexity are not collected, because a faithful port keeps the original's function structure ([ADR 0024](adr/0024-lint-ratchet-scope.md)). Cast and per-block `unsafe` lints are outside the selection: ports preserve the original's integer operations and document module and type safety contracts.
- The 7-argument threshold is in `migration/lints/clippy.toml`; the driver sets that folder as `CLIPPY_CONF_DIR`.
- Ruff is pinned in `ci-quick.yml` and measured in isolated mode with `--ignore-noqa`, so project ignores cannot hide debt.

#### Growth counting

- Growth uses the one current `tools/port-metrics.py` implementation and the pinned-original baseline for both endpoints.
- Its cost is lexing. Each text is lexed once per run; code-line results are kept in the main checkout's `.local/growth-inputs/code-lines-*.json`, keyed by lexer and text, in a file named after the lexing module's hash and the Pygments version. A CI run has no kept results. Source counters reuse the code-line inventory.
- The pinned Rust compiler expands both archived inputs with test syntax enabled, and a pinned Rust syntax tree counts unmangled and `export_name` functions and callback field types, resolving aliases. A generic associated type counts as a callback if any implementation is one.
- This is a syntax census, not a complete monomorphized ABI graph; Full still checks native layout and calls. Historical regex boundary totals are reported separately as a scope correction.
- Archive extractions and expansion outputs go under `.local`; they are not Git worktrees.

#### Cleanup records

- A change's carries and exception proposals live in its branch's file, `migration/cleanup-records/<branch with / as ->.json`, a JSON list of records, never in the PR body. The checks read the working tree.
- A file unchanged since the merge base is inert. A new or changed file counts each record it lacked there, so a later branch may repeat a merged record.
- Lint policy validates every record file on each run, so preflight and Quick give the same result on the same tree, and carries need no network.
- One file per branch keeps two open PRs' records from conflicting. A merged PR's file is inert at every later base; any later `--write` deletes it, and identical deletions in two PRs merge cleanly.
- A malformed record or acceptance block fails in one line naming the file or comment, the record and the field.

#### Archive proof

- The proof runs when the cleanup records hold `{"kind":"refactor-proof","method":"archive","renames":{}}`. It builds both revisions in release mode with the same toolchain and remapped source root.
- It compares allocated code and read-only sections and `.data.rel.ro`, including relocation targets and symbol bindings. It keeps object grouping, so local relocation targets keep their defining object's data, and ignores archive member file names and non-allocated metadata.
- `--build` builds both inputs locally without a PR claim. `--base-archive PATH --head-archive PATH` compares two archives directly; `--renames PATH` supplies a local old-to-new raw-symbol map. CI accepts renames only through a reviewed proposal, as for growth below.
- A proof is not behavioural coverage of unexercised branches.

#### Exceptions, carries and acceptance

- **Lint exception.** A necessary new lint exception first adds only its exact item or site row to the debt file. Its proposal has `kind: lint`, `tool: clippy` or `ruff`, and exact `before` and `after` magnitude lists.
- **Suppression.** A necessary Rust item suppression's proposal has `kind: suppression`, `tool: rust`, and integer `before` and `after` occurrence counts.
- Both give the exact `site` and nonempty `operation`, `source_contract`, `reason`, `retiring_owner` and `retirement_condition`, and need independent acceptance in the growth format below.
- `tools/lint-policy.py` accepts no proposal for a module-wide attribute, a lint group, a new Python `noqa` or a weaker configuration. A retained original-vocabulary exception states that it lasts while that API is reproduced; it is not a debt-baseline reset.
- **Cast reason.** An item-scoped `allow` or `expect` of `non_upper_case_globals`, `clippy::cast_possible_truncation`, `clippy::cast_possible_wrap` or `clippy::cast_sign_loss` needs no proposal when it has a nonempty `reason = "..."` naming the original definition or cast it preserves. One attribute on the `impl` block of a wrapper's associated constants counts as item scope.
- **Carry.** A rename or move that keeps a lint row or suppression site declares a carry, not a proposal: `{"kind":"carry","tool":"clippy"|"ruff"|"rust"|"python","from":"<base key>","to":"<head key>"}`. Carries may cross files and need no review or head binding.
- The checks reject a carry whose keys do not match base and head, whose lint or scope differs or whose magnitude grows, print why, and print a ready record for each one-to-one pair. `python3 tools/lint-policy.py --write` records lint policy's ready carries and checks again. The Clippy and Ruff ratchets pair an evident one-to-one rename or move themselves and print `carried:`; when they report `rename not recorded`, run `--write`.
- **Growth exception.** A cleanup record with `kind: growth`, the exact `counter` and the `added` identity-to-count mapping the check prints, plus nonempty `reason`, `existing_alternatives`, `retiring_owner` and `retirement_condition`. Rename proposals use the same acceptance format.
- **Acceptance.** An independent attributed review comment repeats the exact proposal as `{"head":"<tested PR SHA>","accept":[<proposal>]}` in its own `cleanup-acceptance` fence; it may sit in the ordinary review report. The check rejects a changed payload and the PR author's own acceptance.
- A growth acceptance binds to the PR's own added identities and counts, not to the counter's `old` and `new` totals, which the check ignores. Another PR's merge that moves those totals leaves it valid, and a base update needs no re-acceptance.
- A lint or suppression acceptance binds to its site's item text at the reviewed head and stays valid at later heads and base updates while that item is unchanged. A growth acceptance for generated records, tools, tests or other outside C++ stays valid while the source of every added key is byte-identical: the generated record text, or the file. Exports and callback slots keep exact-head binding.
- Growth whose proposal awaits acceptance reports `pending review`. It passes on every event but `merge_group`, where it fails, so work waiting for review is no failed Quick. Growth without a proposal always fails.
- Checking an acceptance needs read-only GitHub API access through `gh` or `GH_TOKEN`; without it, acceptances count as absent.

### Generated ABI probes and fixture defaults

- Add ABI declarations to `layouts!` in the owning Rust module. Use `rust_field => cpp_field` for aliases and `width field: RustType` for a native-width check. Add no second C++ field list or numeric type ID.
- Fixtures specify the callbacks they use; other callback defaults abort. Native tests check production instances, and `tools/test-generated-abi.py` checks generated lists and visitors. Cross-language signature assertions are not generated.

## Original simulation coverage

`python3 tools/migration.py coverage --jobs 4` measures the default scenarios against the pinned original on Linux. It builds with GCC coverage in `.local/build-reference-coverage`; pinned gcovr writes JSON and a Markdown summary to `.local/coverage`.

- Results include preparation, snapshot and plain games, per-file and per-function lines and branches, and unique original lines per scenario. The `-O0` measurement includes compiler exception edges; files without emitted code are listed separately.
- Re-measure after scenario inputs change. The action runs outside CI and is neither equivalence nor integration evidence. A failed reference run keeps its diagnostics and fails the action.
- `--timeout SECONDS` sets the per-game limit (default 600). `migration.py --help` does not list it: the driver forwards it to `tools/simulation/coverage.py`.

## Compiler cache

Cache build outputs only when the key covers everything that determines them; never cache test results. Put builds, dependencies, toolchains and reference worktrees in ignored locations. CI caches are in [CI](ci.md#caches).

- Worktrees share the main checkout's reference, reference build, bootstrapped dependencies and `.local/compiler-cache/` without symlinks.
- Install ccache or use `tools/bootstrap-local.py`. `migration/ccache.conf` uses compiler-content checks, direct mode and no sloppiness. `--no-ccache` makes an ordinary PCH build; `--ccache-bypass` keeps the no-PCH flags without reuse.
- The driver skips CMake configure when the cache already has every requested `-D` value; Ninja still reacts to CMake input changes. For a changed configure-time environment, such as a dependency prefix, use a fresh build directory or configure by hand.

## Output disk

This host has a 220 GB SSD and a 1 TB HDD mounted at `/mnt/storage` (`/etc/fstab` label `storage`, `nofail`). Write-once output lives on the HDD behind symlinks. Caches, toolchains, the pinned original and its builds, and each worktree's `build-rust` stay on the SSD for build speed.

| Path | Symlink target |
| --- | --- |
| Main checkout `.local/` `simulation`, `verification`, `coverage`, `deep`, `speed-pass`, `worktrees` | `/mnt/storage/openttd-rust/main-local/<name>` |
| Each other worktree's `.local` | `/mnt/storage/openttd-rust/worktree-local/<worktree>-<id>/local` |

- `.git/hooks/post-checkout` calls `.git/hooks/offload-local hook`, which links the `.local` of each new worktree, T3's included. Both scripts are untracked and exist only on this host.
- Worktrees inside the main `.local` keep a real `.local`: `.local/worktrees` is already on the HDD, and the pinned original stays unchanged.
- The hook always exits 0, so it never fails `git worktree add`; without the HDD the new worktree keeps `.local` on the SSD.
- Removing a worktree removes only its link. Each time the hook links a new worktree, it deletes HDD output whose worktree has been gone for 24 hours. `offload-local sweep` lists that output; `--apply` deletes it now.
- `offload-local migrate` moves a worktree's real `.local` onto the HDD, for example after `git worktree add --no-checkout` skipped the hook, and recreates a missing main-checkout link. It is idempotent. The scripts log to `/mnt/storage/openttd-rust/offload.log`.
- If the HDD is unmounted, the main checkout's links dangle and runs fail instead of filling the SSD. Mount it and rerun.
- In tool code, `.resolve()` can take a `.local` path off the checkout's tree, so compute `relative_to(ROOT)` from the unresolved path. `Path.rglob` follows a symlinked starting folder but not symlinked subfolders, which is why worktrees link `.local` whole.
- Stale worktrees and ad hoc `.local` output: `tools/housekeeping.py` ([housekeeping](root-workflow.md#housekeeping)).

## Simulation comparison

`python3 tools/migration.py simulate` builds both games and runs `tools/simulate.py`. Scenario comparisons run on CI, not on this host; run one locally only by name, to diagnose a failure ([ADR 0020](adr/0020-scenario-comparisons-on-ci.md), [checks](workflow.md#checks)). CI runs the default set in [native validation](ci.md#native-validation).

- Add scenario dictionaries and prepare and check hooks to the component's module in `tools/simulation/`; `scenario_modules()` in `tools/simulation/registry.py` assembles them. A module's hooks run only for its own scenarios; a hook that must also act on another module's scenarios goes in `SHARED_HOOKS` in the same file.
- Each scenario runs headless in plain and `-d desync=3` modes under `.local/simulation/`. Desync mode saves an uncompressed snapshot every 32 economy days and changes cache and YAPF behaviour. Both modes compare exit saves and full output; warnings, different checkpoint lists or end moments, and semantic differences fail.
- Default scenarios include regression saves with AIs, generated TGP and original maps, and `play-*` LLM road saves with manual distribution and cargodist. `--soak` adds scenarios, `--self` compares the reference with itself, and names filter the set. The driver runs `--jobs // 2` scenarios at once. Saves are described in `migration/saves/README.md`.
- The hand-built rail save `padhattan-ridge-1996.sav` requires train and ship movement and cargo delivery; `play-padhattan-ridge-2000-*` adds trains, ships, helicopters and a plane. Saved subsidies and PBS signals do not cover every multiplier or route branch. The aircraft fixture checks movement, cargo, flying and delivery; see `tools/aircraft-scenario-ai/README.md`.
- Community AI and arctic scenarios use pinned source downloads ([input setup](../migration/community-ai/README.md)); one generated game and one short reload run by default, and ten-year temperate and arctic games run in soak. NewGRF sets use the same pinned, uncommitted downloads ([evidence](porting.md#evidence)): `newgrf-sets-arctic` runs by default and `newgrf-sets-arctic-long` in soak. The fifty-year same-NewGRF save reloads in `community-newgrf-large-reload` by default and `community-newgrf-large-reload-long` in soak.
- `MASKS` (`tools/simulation/savegame.py`) records reasons and hits. It masks the random save ID, build revision, NewGRF version and the original's uninitialized `round_trip_time`; a port touching the last needs its own check. Record a known port divergence in `KNOWN_FAILURES` by first divergence and issue, never as a new mask.
- `migration/harness-inventory.json` records scenario names, each comparison driver's case lines, `MASKS` and `KNOWN_FAILURES`. Quick fails when the tree differs from it, a mask is added or widened against base, a new known failure names no open issue, or no tool reads a scenario key. `python3 tools/harness_inventory.py --write` records the tree.
- `run-comparisons.py` fails on no, zero or fewer cases than recorded; `--record` writes the printed lines. A removal passes only once the PR records it there.
- Named witnesses fail when a required branch is not reached. `OPENTTD_WITNESS` writes `branch-witnesses.json`; scenario modules set it and check the required counters. Missing files or zero counts fail.
- Linux games run as an unprivileged user with a child-only `RLIMIT_NPROC=0`, so the original's thread-failure paths compute link graphs and save synchronously. A probe rejects hosts that bypass the limit. There are no timing retries or truncated logs; active GameScripts are outside the corpus. The [platform comparison](ci.md#platform-comparison) uses native workers.
- Runtime trees are copied before games; keep the candidate build idle during its first copy. `--candidate` uses the data beside that executable. Reports, including the initial `candidate_commit` and `candidate_status`, go to `.local/simulation/<time>-<pid>/report.json`.

See [speed](speed.md) for benchmarks and profiles and [deep checks](deep-checks.md) for soak, sanitizer, threaded and multiplayer runs.

## Platform linkage

CMake checks the pinned Rust host against the C++ platform, architecture and 64-bit pointers. The Rust archive is `<build>/cargo/<validated-target>/release/libopenttd_kernels.a` (`openttd_kernels.lib` on Windows); `rust_configuration(build)` and `rust_archive(build)` in `tools/migration.py` locate it. Imported `HOST_BINARY_DIR` tools remain host executables.

### Linux and macOS

- macOS requires `CMAKE_OSX_ARCHITECTURES=arm64`, a deployment minimum of 11.0 or newer, and matching `SDKROOT` and `MACOSX_DEPLOYMENT_TARGET` for Rust. Intel packaging is separate work.
- Linux GCC SDL2 builds CMake's default Debug with `OPTION_USE_ASSERTS`, as macOS Debug does, so a `_DEBUG`-only assertion fails before the queue. Without that option `cmake/Options.cmake` adds `NDEBUG`, which compiles every `assert` out; GCC Dedicated and Clang keep that configuration.
- macOS Release defines `NDEBUG`. Both macOS builds run the Cargo checks, a nonempty CTest, scripted regressions and fresh native tools. `tools/check-rust-platform.py` checks linkage, flags and archive rebuilds. Linux results do not establish Darwin support.

### Emscripten

- Requires the `wasm32-unknown-emscripten` std as well as the host std. Use emsdk 4.0.23 and `-fwasm-exceptions` for C++ compiling and linking.
- CI runs `openttd_rust_abi` under emsdk's node, which `emcmake` sets as the CTest emulator.

### Windows MSVC

- Supported: VS 2022 MSVC, x64, single-config Ninja, `RelWithDebInfo`, `OPTION_USE_ASSERTS=ON` and the static release CRT. Install the target std for `x86_64-pc-windows-msvc`. A 32-bit Windows configuration fails with an unsupported-target error.
- `cmake/WindowsRust.cmake` selects `MultiThreaded` and `-C target-feature=+crt-static`.
- Debug or dynamic CRT, other build types, disabled assertions, non-MSVC, multi-config or non-Ninja generators, ARM, UWP, MinGW, cross-OS and external `HOST_BINARY_DIR` are unsupported. Conflicting runtime flags, target overrides, mismatch suppression or a missing target std fail configuration.
- Windows game and test builds use `NDEBUG` and `WITH_ASSERT`; generators use `NDEBUG` alone. Exports use `extern "C"`. Encoded descriptors keep unaligned reads, which 32-bit targets still need.
- `tools/check-rust-platform.py` checks `/MT`, native library arguments and refusals of unsupported configurations. CTest's `rust-abi-layout-and-calls` checks layouts, calls and ownership paths; signed negative compare arguments are outside the defined C domain. Fresh tools-only builds run both generators.

## Continuous integration

CI has its own reference: [CI](ci.md).

## Shared-code checks and port metrics

| Command | Purpose and limit |
| --- | --- |
| `python3 tools/shared-accessors.py --base-ref origin/rust-migration` | Enforces the [shared-accessor rule](porting.md#callees-and-shared-accessors) in Quick. `tools/shared-accessor-baseline.txt` can shrink, never gain exemptions. Macros, derived fields and compound callbacks need review. |
| `python3 tools/rebound-callees.py --base-ref origin/rust-migration` | Rejects duplicate bindings and forwarded service fields in Quick. `tools/rebound-callee-baseline.txt` can shrink, never grow. Macros, lambdas, wrappers, some templates and namespace resolution remain review work. |
| `python3 tools/rust-only.py` | Rejects `WITH_RUST` conditionals in `src/` in Quick. CMake still defines `WITH_RUST` for dual-build harnesses under `tools/`. |
| `python3 tools/ffi-bool-returns.py` | Rejects Rust-defined C `bool` returns, including callbacks, macro templates and visible simple aliases. Rust-only `cfg(test)` service mocks are excluded. |
| `python3 tools/orphan-exports.py --base-ref origin/rust-migration` | Checks Rust exports and C++ declarations for production use. `tools/orphan-export-baseline.txt` can shrink, never grow. Tests and harnesses are not production users. An export whose only references sit in uncalled C++ members needs a row in `tools/uncalled-export-baseline.txt`, which also only shrinks; each row names the owner issue that retires the calling fixture, and that owner removes the row. Macros and arbitrary conditional compilation need review. |
| `python3 tools/port-metrics.py` | Reports code lines with Pygments (pinned in `tools/code_lines.py`), using `uv` if needed. Original retirement may be negative; removing fork glue does not retire original C++. |
| `python3 tools/port-metrics.py --whole-fork` | Reports source-area and simulation retirement, the C++ evidence and dependency inventory, Rust exports and boundary function pointers. |
| `python3 tools/port-metrics.py --functions [AREA ...]` | Adds the remaining original code lines per baseline file and top-level function, plus file scope; implies `--whole-fork`. Rows sum to each area's remaining count. `facade` marks rows whose remaining lines are only a signature and braces. Function detection is a brace heuristic; macro-headed blocks show as `?`. |

- `python3 tools/orphan-exports.py --report-uncalled`, with no other argument, lists exports whose only references sit in uncalled members. The script has no `--help`: given one, it runs the whole check.
- `tools/test-ffi-bool-returns.py`, `test-shared-accessors.py`, `test-rebound-callees.py`, `test-orphan-exports.py` and `test-port-metrics.py` exercise these checks.
- No checker detects Rust copies of original header helpers and constants ([porting](porting.md#callees-and-shared-accessors)).

### Port metric definitions

- Metrics count code lines, excluding comments, directives, blank lines and Python docstrings. A second line keeps the historical physical counts.
- `compiled_cpp` and the area totals keep the comparable `src/` plus generated-record inventory and the 284,355-line original denominator. Records stay only in the area series.
- `evidence_cpp` adds tracked C++ outside `src/` and non-record fragments from the ABI generator, each source identity once. `fork_cpp` is their combined source census, not a preprocessed executable size.
- Version 2 does not inventory extracted-original comparison fragments or C++ embedded in Python strings; their volume is unknown and belongs to #736's scope-completeness follow-up. `fork_cpp` and `candidate_cpp_upper_bound` cover only the listed inventory. These are coverage gaps, not exemptions from final C++ retirement; report a later coverage correction separately from deletion.
- Evidence rows mark direct Rust or dual-build dependencies as candidate or shared. The pinned-original Monocypher vector generator's reference-only compile recipe places it separately; fixtures with reference-like names can still exercise candidate Rust. Other sources stay unresolved and count toward `candidate_cpp_upper_bound`, a conservative bound over all source-area C++ and unresolved evidence, not an exact build graph.
- Raw `rust` and `tooling` totals classify by directory prefix, so frozen oracle text under `rust/` and native support under `tools/` count there too. Report executable `.rs` and `.py` changes separately from data footprint.
- Historical `--head` runs archive each revision's generator and Rust, header and probe inputs; mutable build output is never read. Recount historical endpoints with the same implementation, and report inventory corrections separately from deletion. Preserve collected values: a later classification correction is neither new code nor C++ retirement.
- JSON records the metric version, implementation hash, head, baseline and scope. No line metric claims behavioural completion.
