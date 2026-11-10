# OpenTTD-Rust migration tools

This is the migration tool reference. See [AGENTS.md](../AGENTS.md) for rules and the [component index](components/README.md) for ports.

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

Direct CMake builds need the pinned commit object for native-test oracle headers:
`git fetch --no-tags --depth=1 origin 14ec60f248547d4d062a1160f0fc26d742319888`.
CI fetches that commit before configuring.

The driver creates the detached reference worktree and rejects a dirty source or wrong revision. Updating the baseline is a separate task. See [workflow](workflow.md#branches-and-worktrees) for branches and PRs.

## Build and verification

Native Linux verification needs a C++20 compiler, CMake, Ninja, Python 3.11 or newer, SDL2 development files, the libraries in `COMPILING.md`, OpenGFX and the Rust toolchain pinned in `rust-toolchain.toml`. Commercial assets are unnecessary. CI installs the same Rust toolchain. Node.js runs the CI-policy suite; an authenticated `gh` sends CI requests.

On Ubuntu with administrator access:

```sh
sudo apt-get install build-essential cmake ninja-build python3 pkg-config \
  libsdl2-dev liblzma-dev libpng-dev libcurl4-openssl-dev libfreetype-dev \
  libfontconfig-dev libharfbuzz-dev libicu-dev liblzo2-dev openttd-opengfx
```

This host runs Ubuntu 24.04 x86-64, the release CI's `ubuntu-24.04` runners use, with the packages above plus `valgrind`, `ccache`, `unifdef` and `gdb` from apt (moved 2026-11-05). Keep `.local/deps` absent here: the driver puts it ahead of system packages on every compiler, library and pkg-config path. `python3 tools/bootstrap-local.py` is for an Ubuntu 26.04 host without administrator access; it extracts that release's packages to `.local/deps` and installs Rust in `.local/cargo` and `.local/rustup`, changing no shell profile or system package. Output built on the previous Ubuntu 26.04 host needs glibc 2.43; it is kept unused at `/mnt/storage/openttd-rust/old-host-ubuntu2604-20261105/`.

| Command | What it does |
| --- | --- |
| `python3 tools/migration.py rust-checks` | Runs the four Cargo checks without building the reference. |
| `python3 tools/migration.py build` | Builds both games without tests. |
| `python3 tools/migration.py verify` | Runs Cargo checks, builds both graphical executables and runs both CTest suites. It fails for empty test inventories, missing candidate test names or invalid Rust target metadata. |
| `python3 tools/migration.py tools` | Builds native Rust generators in `.local/build-tools-rust` with `OPTION_TOOLS_ONLY=ON`. |
| `python3 tools/run-comparisons.py` | Runs every `tools/*-comparison.py` in parallel and prints case inventories. |
| `eval "$(python3 tools/migration.py env)"` | Sets the driver environment for Cargo and binaries run outside the driver. |

The four Cargo checks are:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings \
  --force-warn clippy::too_many_arguments --force-warn clippy::missing_safety_doc
cargo test --workspace --locked
```

- `--jobs` defaults to `max(2, cores - round(1-minute load))`, read once at start. It also limits Cargo under CMake.
- Logs, test reports and source revision go to `.local/verification/<timestamp>/`. A pass establishes covered behavior, not whole-game equivalence.
- The candidate is `build-rust/openttd-rust`; Cargo artifacts are in `build-rust/cargo`. The reference is `.local/build-reference/openttd` in the main checkout.
- Bootstrapped dependencies give candidate binaries an RPATH, so `./build-rust/openttd-rust -X`, `./build-rust/openttd_test` and `ctest` in `build-rust` run from a plain shell. `-X` avoids global game folders; use separate development configuration and saves.
- Every fork build requires Rust. Native GNU/Linux x86-64 and ARM, macOS arm64, Emscripten and the scoped Windows MSVC mode below are supported. Other platforms, cross compilation and macOS universal or Intel are unsupported.

### Quick cleanup checks

Ordinary PR pushes and protected-branch pushes run the four Cargo checks above in `ci-quick.yml`, followed by the cleanup gates below. These checks preserve the original's casts and control flow; a faithful truncation must not become a checked conversion to reduce debt.

| Command | Failure distinguished |
| --- | --- |
| `python3 tools/lint-ratchet.py clippy --base-ref origin/rust-migration` | Growth against the merge base for two force-warn Clippy lints: a new item over its threshold, or more or larger diagnostics on an item. Improvements never fail. |
| `python3 tools/lint-ratchet.py ruff --base-ref origin/rust-migration` | The same ratchet for C901 and PLR0911/12/13/15 over `tools/`, with `--ignore-noqa`. |
| `uvx --from ruff==0.16.8 ruff check tools/`, `uvx --from ruff==0.16.8 ruff format --check tools/`, `uvx ty@0.0.92 check`, then every `tools/test-*.py` | Quick's Python step 8, in that order. A PR that changes `tools/` runs all four locally before it is marked ready. CI stops at the first failure, so a `ty` error hides failures in the test scripts (#826). |
| `python3 tools/lint-policy.py --base-ref origin/rust-migration` | Weaker selections, broader exclusions/per-file ignores, changed thresholds/tool versions, and new Rust allow/expect or Python noqa sites. Unclassified policy edits fail for review. |
| `python3 tools/growth-ratchet.py --base-ref origin/rust-migration` | Independent growth in tools C++, fork-added src/tests C++, Rust exports and callback fields. Generated records and other outside-src C++ have separate counters so moves cannot exempt growth. |
| `python3 tools/refactor-proof.py --base-ref origin/rust-migration` | Claimed release-archive equivalence fails on changed code, relocation targets/addends, read-only data, symbol offsets or section alignment. |
| `python3 tools/test-cleanup-checks.py` | Seeded violations distinguish each gate, including all four Cargo commands and separate code/data/relocation changes. |

- Lint baselines are `migration/clippy-debt.json` and `migration/ruff-debt.json`. Their provenance identifies the input checkpoint; generated baseline data is accounted separately from tooling. Keys name the file, the innermost named item and the lint, with diagnostic multiplicity and measured magnitude. Cargo's library/test duplicate emissions count once. The check compares the head with the debt file at the merge base, never with a base tip that has moved since; without a common ancestor it stops. Retired rows and lower magnitudes pass without a debt-file edit, and a base row whose item no longer exists allows nothing new. `--write` records the measured debt whenever nothing grew. Another retired site cannot purchase a new violation. `--initialize` is an explicit first-inventory operation, never a CI update command. Compiler/linter upgrades require an explicit inventory migration.
- Clippy uses Rust 1.99.0 with `too_many_arguments` and `missing_safety_doc` under `--force-warn` (#747, narrowed by #855). Function length, nesting and cognitive complexity are not collected: a faithful port keeps the original's function structure, and those rows caught no defect (steering 33). Cast and per-block unsafe lints are outside this selection; ports preserve the original's integer operations and document module/type safety contracts. The gate's Clippy run force-warns both lints, which stay warnings under `-D warnings`, and the ratchet repeats that exact command in the same build directory, so Cargo replays the gate's diagnostics and checks the crate once (#860). A hand-run `cargo clippy` with other arguments makes the next gate run check it again. The 7-argument threshold lives in `migration/lints/clippy.toml`, which the driver sets as `CLIPPY_CONF_DIR`. Ruff is pinned to 0.16.8 and measured in isolated mode so project ignores cannot hide debt.
- Growth uses one current `port-metrics.py` implementation and pinned-original baseline for both endpoints. Its cost is lexing: each text is lexed once in a run, and code-line results are kept in the main checkout's `.local/growth-inputs/code-lines-*.json`, keyed by lexer and text, in a file named after the lexing module's own hash and the Pygments version (#860). On the idle host the ratchet fell from 126 seconds to 81 on a first run and 37 after it; a CI run has no kept results and takes the first-run path. Source counters reuse its code-line inventory. The pinned Rust compiler expands both archived inputs with test syntax enabled, and a pinned Rust syntax tree counts unmangled/export_name functions and callback field types, resolving aliases. Generic associated types are counted conservatively if any implementation is a callback. This is a syntax census, not a complete monomorphized ABI graph; Full still checks native layout and calls. Historical regex boundary totals are reported separately as a scope correction. Archive extractions and expansion outputs use `.local`; they are not Git worktrees.
- The archive proof runs when the PR includes a `cleanup-acceptance` JSON block with `{"kind":"refactor-proof","method":"archive","renames":{}}`. It builds both captured revisions in release mode with the same toolchain and remapped source root, then compares allocated code/read-only sections and `.data.rel.ro`, including relocation targets and symbol bindings. It preserves object grouping so local relocation targets retain their defining object's data, while ignoring archive member filenames and non-allocated metadata. `--build` locally builds both inputs even without a PR claim. Direct local comparison uses `--base-archive PATH --head-archive PATH`; `--renames PATH` supplies a local old-to-new raw-symbol map. CI accepts renames only through the reviewed PR contract below. Proof is not a behavioral coverage claim for unexercised branches.
- A necessary new lint exception first adds only its exact item/site row to the debt file. Its PR proposal has `kind: lint`, `tool: clippy` or `ruff`, `site`, exact `before` and `after` magnitude lists, and nonempty `operation`, `source_contract`, `reason`, `retiring_owner` and `retirement_condition`. It needs independent acceptance in the growth format, bound to item text as below. The check prints the accepted operation; it does not automatically refresh any baseline.
- A necessary Rust item suppression uses `kind: suppression`, `tool: rust`, the exact policy `site`, integer `before` and `after` occurrence counts, and the same operation/source/reason/owner/condition fields and independent review. This permits original enumerator spelling or a faithful native cast without suppressing unreviewed sites. Module-wide attributes, lint groups, new Python noqa and weaker configuration remain rejected even with a review record. A retained original-vocabulary exception states that it lasts while that API is reproduced; it is not a debt-baseline reset.
- A rename or move that keeps a lint row or suppression site declares a carry instead of a contract: a PR-body `cleanup-acceptance` record `{"kind":"carry","tool":"clippy"|"ruff"|"rust"|"python","from":"<base key>","to":"<head key>"}`. The check requires `from` at base and not at head, `to` new at head, the same lint or rule (and, for suppressions, no broadening to module scope), and head magnitudes and multiplicity or count no greater than `from`'s. Carries may cross files, need no review or head binding, and are copied unchanged into integration PRs. The Clippy and Ruff ratchets need no carry for an evident rename or move: the old item existed at base and is gone at head, the new item did not exist at base, the pairing is one-to-one within the same lint (by item name, then within one file), and the new row does not grow. The check prints each pair as `carried:`, and candidates for the rest. A renamed row's allowance moves to its new key, so the change records the rename in the debt file (`--write`), and the old row may not stay.
- An item-scoped `allow` or `expect` of `non_upper_case_globals`, `clippy::cast_possible_truncation`, `clippy::cast_possible_wrap` or `clippy::cast_sign_loss` needs no contract when its attribute has a nonempty `reason = "..."` naming the original definition or cast it preserves. One attribute on the `impl` block of a wrapper's associated constants counts as item scope.
- A reviewed lint or suppression contract binds to its site's item text at the reviewed head: the acceptance stays valid at later heads, base updates and integration PRs while that item is unchanged. The acceptance block may sit in the reviewer's ordinary review report. An integration PR copies the component's proposal and names the component PRs in `{"kind":"reviews","pulls":[<numbers>]}`; acceptances there by that PR's own author do not count. A growth-counter contract for generated records or tools, tests or outside C++ stays valid at later heads while the source of every added key is byte-identical: the generated record text, or the file. Exports and callback slots keep exact-head binding.
- A growth exception is a PR-body `cleanup-acceptance` JSON block containing `kind: growth`, the exact `counter`, `old`, `new` and `added` identity/count mapping printed by the check, plus nonempty `reason`, `existing_alternatives`, `retiring_owner` and `retirement_condition`. An independent attributed review comment must repeat that exact proposal under `{"head":"<tested PR SHA>","accept":[<proposal>]}` in its own `cleanup-acceptance` fence. Rename proposals use the same acceptance format. Agent paths distinguish independence because credentials are shared. Stale heads, changed payloads and self-acceptance fail. Accepted exceptions remain visible in the check result; no global growth baseline resets. Local exception checks and CI require read-only GitHub API access through `gh`/`GH_TOKEN`.

### Generated ABI probes and fixture defaults (#284)

- Add ABI declarations to `layouts!` in the owning Rust module. Use `rust_field => cpp_field` for aliases and `width field: RustType` for a native-width check. Do not add a second C++ field list or numeric type ID.
- Fixtures specify callbacks they use; other callback defaults abort. Native tests check production instances, and `tools/test-generated-abi.py` checks generated lists and visitors. Cross-language signature assertions are not generated.

## Original simulation coverage

Run `python3 tools/migration.py coverage --jobs 4` to measure the default scenarios against the pinned original on Linux. It builds with GCC coverage in `.local/build-reference-coverage`; pinned gcovr 8.4 writes JSON and a Markdown summary to `.local/coverage`.

- Results include preparation, snapshot and plain games, per-file and per-function lines and branches, and unique original lines per scenario. The `-O0` measurement includes compiler exception edges; files without emitted code are listed separately.
- Re-measure after scenario inputs change. The action runs outside CI and is not equivalence or integration evidence. Failed reference runs retain diagnostics and fail the action.
- `--timeout` changes the per-game limit; the default is 600 seconds.

## Compiler cache, shared reference and CI time

Cache build outputs when the key covers what determines them; never cache test results. Put builds, dependencies, toolchains and reference worktrees in ignored locations.

- Worktrees share the main checkout's reference, build, bootstrapped dependencies and `.local/compiler-cache/`. No symlinks are needed.
- Install ccache or use `tools/bootstrap-local.py`. `migration/ccache.conf` uses compiler-content checks, direct mode and no sloppiness. `--no-ccache` enables an ordinary PCH build; `--ccache-bypass` keeps no-PCH flags without reuse.
- The driver skips CMake configure when the cache has all requested `-D` values; Ninja still responds to CMake input changes. For changed configure-time environment such as a dependency prefix, use a fresh build directory or configure manually.
- CI caches use compatible OS, architecture, baseline, compiler, ccache and configuration keys (#79). Windows and macOS also cache vcpkg binaries and use ccache 4.14.1 without PCH. Windows vcpkg keys are per commit and restore the newest prefix match; a job saves only when it stored new archives. After a successful build it prunes to the installed ABIs when every ABI file can be hashed and matched to an archive; otherwise it warns and saves without pruning. Requested runs use a `candidate` namespace that protected runs' restore keys exclude (#699). Key names do not restrict a job's cache-write authority; requested dispatches share the `rust-migration` ref scope with protected pushes. That is accepted only because every PR here is written by the fork's own agents (#701). Before the fork accepts outside PRs, enforce candidate cache-write separation by ref or service policy. MSVC uses `/Z7` (CMP0141). Emscripten builds without PCH and wraps emcc's clang with the container's ccache through `EM_COMPILER_WRAPPER`.

## Output disk

This host has a 220 GB SSD and a 1 TB HDD mounted at `/mnt/storage` (`/etc/fstab` label `storage`, `nofail`). Write-once output lives on the HDD behind symlinks. Caches, toolchains, the pinned original and its builds, and each worktree's `build-rust` stay on the SSD for build speed.

| Path | Symlink target |
| --- | --- |
| Main checkout `.local/` `simulation`, `verification`, `coverage`, `deep`, `speed-pass`, `worktrees` | `/mnt/storage/openttd-rust/main-local/<name>` |
| Each other worktree's `.local` | `/mnt/storage/openttd-rust/worktree-local/<worktree>-<id>/local` |

- `.git/hooks/post-checkout` calls `.git/hooks/offload-local hook`, which links the `.local` of each new worktree, T3's included. Worktrees inside the main `.local` keep a real `.local`: `.local/worktrees` is already on the HDD, and the pinned original stays unchanged. The hook always exits 0, so it never fails `git worktree add`; without the HDD the new worktree keeps `.local` on the SSD. Both scripts are untracked and exist only on this host.
- Removing a worktree removes only its link. Each time the hook links a new worktree, it deletes HDD output whose worktree has been gone for 24 hours. `offload-local sweep` lists that output; `--apply` deletes it now.
- `offload-local migrate` moves a worktree's real `.local` onto the HDD, for example after `git worktree add --no-checkout` skipped the hook, and recreates a missing main-checkout link. It is idempotent. The scripts log to `/mnt/storage/openttd-rust/offload.log`.
- If the HDD is unmounted, the main checkout's links dangle and runs fail instead of filling the SSD. Mount it and rerun.
- In tool code, `.resolve()` can take a `.local` path off the checkout's tree, so compute `relative_to(ROOT)` from the unresolved path. `Path.rglob` follows a symlinked starting folder but not symlinked subfolders, which is why worktrees link `.local` whole.

## Simulation comparison

`python3 tools/migration.py simulate` builds both games and runs `tools/simulate.py`. CI runs the default set after component comparisons. Add scenario dictionaries and prepare/check hooks to the component's module in `tools/simulation/`; `scenario_modules()` assembles them. A module's hooks run only for the scenarios it defines; a hook that must also act on another module's scenarios is named in `SHARED_HOOKS` in `core.py`.

- Each scenario runs headless in both plain and `-d desync=3` modes under `.local/simulation/`. Desync mode saves an uncompressed snapshot every 32 economy days and changes cache and YAPF behavior. Both modes compare exit saves and full output; warnings, different checkpoint lists or end moments, and semantic differences fail.
- Default scenarios include regression saves with AIs, generated TGP and original maps, and `play-*` LLM road saves with manual distribution and cargodist. `--soak` adds scenarios, `--self` compares the reference to itself, and names filter the set. The driver runs `--jobs // 2` scenarios at once. See `migration/saves/README.md`.
- The #86 rail save requires train and ship movement and cargo delivery; `play-padhattan-ridge-2000-*` (#179) adds trains, ships, helicopters and a plane. Saved subsidies and PBS signals do not cover every multiplier or route branch. The aircraft fixture checks movement, cargo, flying and delivery; see `tools/aircraft-scenario-ai/README.md`.
- Community AI and arctic scenarios (#507) use pinned source downloads; one generated game and one short reload run by default, while ten-year temperate and arctic games use soak. See [input setup](../migration/community-ai/README.md). NewGRF sets (#507) use the same pinned, uncommitted downloads ([evidence](porting.md#evidence)): `newgrf-sets-arctic` runs by default and `newgrf-sets-arctic-long` is soak-only. The fifty-year same-NewGRF save reloads in `community-newgrf-large-reload` by default and `community-newgrf-large-reload-long` in soak.
- `MASKS` records reasons and hits. It masks random save ID, build revision, NewGRF version and the original's uninitialized `round_trip_time` (#83); a port touching the latter needs its own check. Record known port divergences in `KNOWN_FAILURES` by first divergence and issue, never as new `MASKS`.
- `migration/harness-inventory.json` records scenario names, each comparison driver's case lines, `MASKS` and `KNOWN_FAILURES`. Quick CI fails when the tree differs from it, a mask is added or widened against base, a new known failure names no open issue, or no tool reads a scenario key; `python3 tools/harness_inventory.py --write` records the tree. `run-comparisons.py` fails on no, zero or fewer cases than recorded; `--record` writes the printed lines. A removal passes only once the PR records it there.
- Named witnesses (#198) fail when a required branch is not reached. `OPENTTD_WITNESS` writes `branch-witnesses.json`; scenario modules set it and check the required counters. Missing files or zero counts fail.
- Linux games run as an unprivileged user with child-only `RLIMIT_NPROC=0`; original thread-failure paths compute link graphs and save synchronously. A probe rejects hosts that bypass the limit. There are no timing retries or truncated logs; active GameScripts remain outside the corpus. The cadence subset below uses native workers.
- Runtime trees are copied before games. Keep the candidate build idle during its initial copy. `--candidate` uses data beside that executable. Reports, including initial `candidate_commit` and `candidate_status` (#97), go to `.local/simulation/<time>-<pid>/report.json`.

See [speed](speed.md) for benchmarks and profiles and [deep checks](deep-checks.md) for soak, sanitizer, threaded and multiplayer runs.

## Native macOS arm64 Rust linkage

- CMake checks the pinned Rust host against the C++ platform, architecture and 64-bit pointers. macOS requires `CMAKE_OSX_ARCHITECTURES=arm64`, deployment minimum 11.0 or newer, and matching `SDKROOT` and `MACOSX_DEPLOYMENT_TARGET` for Rust. Intel packaging is separate work.
- Emscripten requires `wasm32-unknown-emscripten` std as well as host std. Use emsdk 4.0.23 and `-fwasm-exceptions` for C++ compilation and linking. CI runs `openttd_rust_abi` with emsdk node 22.16.0.
- The archive is `<build>/cargo/<validated-target>/release/libopenttd_kernels.a`; `rust_configuration(build)` and `rust_archive(build)` in `tools/migration.py` locate it. Imported `HOST_BINARY_DIR` tools remain host executables.
- macOS Debug uses `OPTION_USE_ASSERTS`; Release defines `NDEBUG`. Both run Cargo checks, nonempty CTest and scripted regressions, and fresh native tools. `tools/check-rust-platform.py` checks linkage, flags and archive rebuilds. Linux results do not establish Darwin support.

## Native Windows MSVC Rust linkage

- #33 supports VS 2022 MSVC, x86 or x64, single-config Ninja, `RelWithDebInfo`, `OPTION_USE_ASSERTS=ON` and static release CRT. Install the target std for `i686-pc-windows-msvc` or `x86_64-pc-windows-msvc`; the C++ architecture selects the target, not the Rust host.
- `WindowsRust.cmake` selects `MultiThreaded` and `-C target-feature=+crt-static`. The archive is `<build>/cargo/<validated-target>/release/openttd_kernels.lib`.
- Debug or dynamic CRT, other build types, disabled assertions, non-MSVC, multi-config or non-Ninja, ARM, UWP, MinGW, cross-OS and external `HOST_BINARY_DIR` remain unsupported (#3). Conflicting runtime flags, target overrides, mismatch suppression or missing target std fail configuration.
- Windows game and test builds use `NDEBUG` and `WITH_ASSERT`; generators use `NDEBUG` alone. Exports use `extern "C"` (cdecl on i686). Encoded descriptors need unaligned reads on MSVC i686.
- `tools/check-rust-platform.py` checks `/MT`, native library arguments and unsupported configuration refusals. CTest's `rust-abi-layout-and-calls` checks layouts, calls and ownership paths; signed negative compare arguments are outside the defined C domain (#26). Fresh tools-only builds run both generators.

## Continuous integration

Full validation requires native comparisons and simulation, Windows x64 and x86, Linux GCC SDL2, GCC Dedicated and Clang, Emscripten, quick checks and annotation checks. Windows x86, GCC Dedicated and Clang returned to Full on 2026-11-10 (#860, reversing #514): each red cadence from 11-05 to 11-09 came from one of them, and two stopped integrations for 134 minutes. A covered protected push refreshes their compiler caches with the others. It publishes a status only for the captured PR head after all succeed; skipped or cancelled jobs do not qualify. The exception is a checker-only PR, whose merge commit changes only these files:
  - Quick's checker scripts: `lint-ratchet`, `lint-policy`, `growth-ratchet`, `refactor-proof`, `baseline_ratchet`, `cleanup_contract` and `test-cleanup-checks`;
  - the Clippy and Ruff debt files;
  - `migration/lints/clippy.toml`;
  - inert Markdown.

  The plan compares the captured merge commit with its own first parent, not the PR's file list or a base that may have moved. It fails closed when GitHub's comparison picks another merge base or lists 300 or more files. No native or platform job reads those files, so its Full runs Quick alone, in about 8 minutes instead of 22. It publishes `Full validation` without a `Validated tree` marker, so the post-merge run still builds the merged tree. Protection also requires cheap checks, resolved conversations and a branch current with base. See [integration](root-workflow.md#integration) for the exact-head gate.
- Inert Markdown is `AGENTS.md` and `docs/**.md`, except the docs CMake packages for the in-game viewer. Root-level Markdown such as `README.md`, and every other Markdown file, is a build input. The checker-only plan, carried validation and the post-merge Markdown-only skip share this rule; `tools/test-ci.py` checks it against CMake.

| Command | Purpose |
| --- | --- |
| `python3 tools/ci.py request 123 --profile rust` | Run the four Cargo checks and quick checks. |
| `python3 tools/ci.py request 123 --profile native` | Add Linux GCC SDL2 and full migration comparison. |
| `python3 tools/ci.py request 123 --profile platform` | Diagnose a suspected platform failure across the platform matrix. |
| `python3 tools/ci.py request 123 --profile full` | Request the only profile that satisfies the merge gate. |
| `python3 tools/ci.py wait .local/ci-requests/REQUEST.json` | Wait for a request. |
| `python3 tools/preflight.py --base origin/rust-migration` | Run the pinned commit checker and rebound-callee gate; `--hooks PATH` uses a clean pinned checkout offline. |

- The CLI refuses unmergeable or pending-mergeability PRs and changed heads. A changed head or advanced base needs a new Full, except when the head's status was carried (next bullet). A Full request for a head that already passed dispatches nothing. An implementer requests `rust` or `native` for its own PR ([checks](workflow.md#checks)), the integrator requests Full, and root administers workflow bootstraps ([CI administration](root-workflow.md#ci-administration)).
- Keep warm-cache Full within about 15 minutes (#233). Add parallel jobs or shards, not steps on one job's critical path. Keep caches under the repository limit.
- All profiles run quick checks. `rust-migration` protection disables ordinary force pushes and branch deletion.
- Carried validation (#659): when a PR push keeps the tree outside inert Markdown that its previous head passed Full on, the invalidation workflow copies that `Full validation` and `Validated tree` to the new head within seconds, instead of setting it pending. Examples are an update from base over docs-only commits and a Markdown-only push to the PR. The new head must contain the current base, so the merged tree is the one that was tested. The tree comparison is the same complete-inventory rule as the post-merge cache refresh. Any other push, a previous head without a same-run passing result, or a failed lookup leaves the status pending.
- Protected pushes queue without cancelling a running validation; newer pending pushes replace older ones. Markdown-only pushes ordinarily start no workflows. Matching previously Full-tested non-Markdown trees refresh eligible compiler caches without repeating tests; incomplete inventories retain heavy validation. Post-merge checks do not authorize an untested combination.
- Full uses native comparisons and three simulation shards, with frozen runtimes and run-scoped artifacts. Reports and failing saves are artifacts; build inputs expire after one day. Native cache roles are trimmed to 750 MB, Linux flavours to 600 MB, macOS flavours to 650 MB and Emscripten to 500 MB; retention bounds GitHub cache use to 8 GiB under the 10 GiB limit.
- Every job installs system packages through `.github/actions/apt-install`: each attempt is bounded, and a failed or stalled attempt retries without the Azure mirror. Azure trickled at about 40 kB/s (#858's Full spent 6 minutes on 46 MB) and Canonical's archive timed out (#833, #841).
- Runner variables are `CI_LINUX_HEAVY`, `CI_LINUX_COMPARISONS`, `CI_LINUX_SIMULATION`, `CI_LINUX_PLATFORM`, `CI_MACOS_HEAVY` and `CI_WINDOWS_HEAVY`. Unset values use `ubuntu-24.04`, `macos-latest` and `windows-2022` respectively; cached GCC flavours always use free `ubuntu-24.04`. Current Blacksmith settings use `blacksmith-8vcpu-ubuntu-2404` for heavy Linux, `blacksmith-4vcpu-ubuntu-2404` for other Linux jobs and `blacksmith-4vcpu-windows-2025` for Windows. Profile runner sizes with `blacksmith jobs list|steps|stats`, `jobs aggregate` and `jobs diagnose rightsize`; use `--job-name` for the latter two. Use an org token, never a person's `~/.blacksmith/credentials`.
- The [platform cadence](root-workflow.md#ci-administration) dispatch compares Windows x64 and macOS arm64 Release against Linux for three manual play saves, `generate-tgp-128-1` and `aircraft-route-manual` (#509). Prepare with `python3 tools/simulate.py platform --reference-binary PATH --output DIR`; consume with `python3 tools/simulate.py platform --stored-reference DIR --candidate PATH --output OTHER`. Missing or mismatched inputs fail. The cases keep full tick intervals, normalize GLOG, set `linkgraph.recalc_time` to 9000 and move job joins to the initial economy date plus 4500 days. They use synchronous saves and identical OpenGFX; all checkpoints require joins after the fixed end date. This subset excludes link-graph joins, cargodist, regression AIs, custom console adapters, GUI scheduling and networking; it remains #509 acceptance work until root runs the changed workflow.

## Shared-code checks and port metrics

| Command | Purpose and limit |
| --- | --- |
| `python3 tools/shared-accessors.py --base-ref origin/rust-migration` | Enforce #249 in quick CI. `tools/shared-accessor-baseline.txt` can shrink, not gain exemptions. Macros, derived fields and compound callbacks need review. |
| `python3 tools/rebound-callees.py --base-ref origin/rust-migration` | Reject duplicate bindings and forwarded service fields in quick CI. `tools/rebound-callee-baseline.txt` can shrink, not grow. Macros, lambdas, wrappers, some templates and namespace resolution remain review work. |
| `python3 tools/rust-only.py` | Reject `WITH_RUST` conditionals in `src/` in quick CI. CMake still defines `WITH_RUST` for dual-build harnesses under `tools/`. |
| `python3 tools/ffi-bool-returns.py` | Reject Rust-defined C `bool` returns, including callbacks, macro templates and visible simple aliases (#644). Rust-only `cfg(test)` service mocks are excluded. |
| `python3 tools/orphan-exports.py --base-ref origin/rust-migration` | Check Rust exports and C++ declarations for production use. `tools/orphan-export-baseline.txt` can shrink, not grow. Macros and arbitrary conditional compilation need review; reviewers also check orphan `WITH_RUST` bodies. |
| `python3 tools/port-metrics.py` | Report code lines with Pygments 2.19.2, using `uv` if needed. Original retirement may be negative; removing fork glue does not retire original C++. |
| `python3 tools/port-metrics.py --whole-fork` | Report source-area and simulation retirement, C++ evidence/dependency inventory, Rust exports and boundary function pointers. |
| `python3 tools/port-metrics.py --functions [AREA ...]` | Add the whole-fork report's remaining original code lines per baseline file and top-level function, plus file scope. Rows sum to each area's remaining count. `facade` marks rows whose remaining lines are only a signature and braces. Function detection is a brace heuristic; macro-headed blocks show as `?`. |

The `tools/test-ffi-bool-returns.py`, `tools/test-shared-accessors.py`, `tools/test-rebound-callees.py`, `tools/test-orphan-exports.py` and `tools/test-port-metrics.py` tests exercise their checks. No checker detects Rust copies of original header helpers and constants ([porting](porting.md#callees-and-shared-accessors)).

Metrics count code lines, excluding comments, directives, blank lines and Python docstrings; a second line retains historical physical counts. `compiled_cpp` and the area totals retain the comparable `src/` plus generated-record inventory and the 284,355-line original denominator. `evidence_cpp` adds tracked C++ outside `src/` and non-record fragments from the ABI generator, each source identity once. `fork_cpp` is their combined source census, not a preprocessed executable size. Records remain only in the area series. Version 2 does not inventory extracted-original comparison fragments or C++ embedded in Python strings; their volume is unknown and belongs to #736's scope-completeness follow-up. The current `fork_cpp` series and `candidate_cpp_upper_bound` are limited to the listed inventory, not bounds over that unlisted support. These are coverage gaps, not exemptions from final C++ retirement; report any later coverage correction separately from deletion. Historical `--head` runs archive each revision's generator and Rust/header/probe inputs; mutable build output is never read. Raw `rust` and `tooling` totals classify by directory prefix, so frozen oracle text under `rust/` and native support under `tools/` also contribute. Report executable `.rs`/`.py` changes and data footprint separately: #802's 7,007 raw Rust additions contain 6,739 oracle-data lines and 268 implementation lines. Preserve the collected values; a future classification correction is not new code or C++ retirement.

Evidence rows identify direct Rust/dual-build dependencies as candidate/shared. The pinned-original Monocypher vector generator's documented reference-only compile recipe places it separately; fixtures with reference-like names can still exercise candidate Rust. Other sources stay unresolved and count toward `candidate_cpp_upper_bound`. This conservative bound includes all source-area C++ and unresolved evidence; it is not an exact build graph.

JSON records the metric version, implementation hash, head, baseline and scope. Recount historical endpoints with the same implementation and report inventory corrections separately from deletion. No line metric claims behavioral completion.
