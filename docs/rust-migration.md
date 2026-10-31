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

The driver creates the detached reference worktree and rejects a dirty source or wrong revision. Keep that worktree unchanged; updating the baseline is a separate task. See [workflow](workflow.md#branches-and-worktrees) for branches and PRs. A port does not by itself complete its subsystem.

## Build and verification

Native Linux verification needs a C++20 compiler, CMake, Ninja, Python 3.11 or newer, SDL2 development files, the libraries in `COMPILING.md`, OpenGFX and the Rust toolchain pinned in `rust-toolchain.toml`. Commercial assets are unnecessary. CI installs the same Rust toolchain. Node.js runs the CI-policy suite; an authenticated `gh` sends CI requests.

On Ubuntu with administrator access:

```sh
sudo apt-get install build-essential cmake ninja-build python3 pkg-config \
  libsdl2-dev liblzma-dev libpng-dev libcurl4-openssl-dev libfreetype-dev \
  libfontconfig-dev libharfbuzz-dev libicu-dev liblzo2-dev openttd-opengfx
```

On this Ubuntu 26.04 x86-64 host without administrator access, `python3 tools/bootstrap-local.py` downloads packages to `.local/downloads`, extracts them to `.local/deps`, and installs Rust in `.local/cargo` and `.local/rustup`. It changes no shell profiles or system packages. It depends on host libraries and package names; it is not a portable installer.

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
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

- `--jobs` defaults to `max(2, cores - round(1-minute load))`, read once at start. It also limits Cargo under CMake.
- Logs, test reports and source revision go to `.local/verification/<timestamp>/`. A pass establishes covered behavior, not whole-game equivalence.
- The candidate is `build-rust/openttd-rust`; Cargo artifacts are in `build-rust/cargo`. The reference is `.local/build-reference/openttd` in the main checkout.
- Bootstrapped dependencies give candidate binaries an RPATH, so `./build-rust/openttd-rust -X`, `./build-rust/openttd_test` and `ctest` in `build-rust` run from a plain shell. `-X` avoids global game folders; use separate development configuration and saves.
- Every fork build requires Rust. Native GNU/Linux x86-64 and ARM, macOS arm64, Emscripten and the scoped Windows MSVC mode below are supported. Other platforms, cross compilation and macOS universal or Intel are unsupported.

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
- CI caches use compatible OS, architecture, baseline, compiler, ccache and configuration keys (#79). Windows and macOS also cache vcpkg binaries and use ccache 4.14.1 without PCH; MSVC uses `/Z7` (CMP0141).

## Simulation comparison

`python3 tools/migration.py simulate` builds both games and runs `tools/simulate.py`. CI runs the default set after component comparisons. Add scenario dictionaries and prepare/check hooks to the component's module in `tools/simulation/`; `scenario_modules()` assembles them.

- Each scenario runs headless in both plain and `-d desync=3` modes under `.local/simulation/`. Desync mode saves an uncompressed snapshot every 32 economy days and changes cache and YAPF behavior. Both modes compare exit saves and full output; warnings, different checkpoint lists or end moments, and semantic differences fail.
- Default scenarios include regression saves with AIs, generated TGP and original maps, and `play-*` LLM road saves with manual distribution and cargodist. `--soak` adds scenarios, `--self` compares the reference to itself, and names filter the set. The driver runs `--jobs // 2` scenarios at once. See `migration/saves/README.md`.
- The #86 rail save requires train and ship movement and cargo delivery; `play-padhattan-ridge-2000-*` (#179) adds trains, ships, helicopters and a plane. Saved subsidies and PBS signals do not cover every multiplier or route branch. The aircraft fixture checks movement, cargo, flying and delivery; see `tools/aircraft-scenario-ai/README.md`.
- Community AI and arctic scenarios (#507) use pinned source downloads; one generated game and one short reload run by default, while ten-year temperate and arctic games use soak. See [input setup](../migration/community-ai/README.md). The same-NewGRF save remains unselected while #525 and #526 are user-parked.
- `MASKS` records reasons and hits. It masks random save ID, build revision, NewGRF version and the original's uninitialized `round_trip_time` (#83); a port touching the latter needs its own check. Record known port divergences in `KNOWN_FAILURES` by first divergence and issue, never as new `MASKS`.
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

Full validation requires native comparisons and simulation, Windows x64, Linux GCC SDL2, Emscripten, quick checks and annotation checks. It publishes a status only for the captured PR head after all succeed; skipped or cancelled jobs do not qualify. Protection also requires cheap checks, resolved conversations and a branch current with base. See [integration](workflow.md#integration) for the exact-head gate; never bypass it with `--admin`.

| Command | Purpose |
| --- | --- |
| `python3 tools/ci.py request 123 --profile rust` | Run the four Cargo checks and quick checks. |
| `python3 tools/ci.py request 123 --profile native` | Add Linux GCC SDL2 and full migration comparison. |
| `python3 tools/ci.py request 123 --profile platform` | Diagnose a suspected platform failure across the platform matrix. |
| `python3 tools/ci.py request 123 --profile full` | Request the only profile that satisfies the merge gate. |
| `python3 tools/ci.py wait .local/ci-requests/REQUEST.json` | Wait for a request. |
| `python3 tools/preflight.py --base origin/rust-migration` | Run the pinned commit checker and rebound-callee gate; `--hooks PATH` uses a clean pinned checkout offline. |

- The CLI refuses unmergeable or pending-mergeability PRs and changed heads. A changed head or advanced base needs a new Full. `CI_ON_DEMAND=true` enables requests after root installs and verifies the status; without it automatic Full remains the bootstrap.
- For a workflow-changing PR, root reviews its workflow, temporarily unsets `CI_ON_DEMAND`, and requests `--profile full --bootstrap` through `ci:full`. After merge, restore the variable to `true` and record both changes in the PR.
- Keep warm-cache Full within about 15 minutes (#233). Add parallel jobs or shards, not steps on one job's critical path. Keep caches under the repository limit.
- All profiles run quick checks including Ruff 0.16.8 and ty 0.0.92. Python conversion remains on #501. `rust-migration` protection disables ordinary force pushes and branch deletion; attributed independent review supplies the process gate with shared credentials.
- Protected pushes queue without cancelling a running validation; newer pending pushes replace older ones. Markdown-only pushes ordinarily start no workflows. Matching previously Full-tested non-Markdown trees refresh eligible compiler caches without repeating tests; incomplete inventories retain heavy validation. Post-merge checks do not authorize an untested combination.
- Full uses native comparisons and three simulation shards, with frozen runtimes and run-scoped artifacts. Reports and failing saves are artifacts; build inputs expire after one day. Native cache roles are trimmed to 750 MB, Linux flavours to 600 MB and macOS flavours to 650 MB; retention bounds GitHub cache use to 8 GiB under the 10 GiB limit.
- Runner variables are `CI_LINUX_HEAVY`, `CI_LINUX_COMPARISONS`, `CI_LINUX_SIMULATION`, `CI_LINUX_PLATFORM`, `CI_MACOS_HEAVY` and `CI_WINDOWS_HEAVY`. Unset values use `ubuntu-24.04`, `macos-latest` and `windows-2022` respectively; cached GCC flavours always use free `ubuntu-24.04`. Current Blacksmith settings use `blacksmith-8vcpu-ubuntu-2404` for heavy Linux, `blacksmith-4vcpu-ubuntu-2404` for other Linux jobs and `blacksmith-4vcpu-windows-2025` for Windows. Profile runner sizes with `blacksmith jobs list|steps|stats`, `jobs aggregate` and `jobs diagnose rightsize`; use `--job-name` for the latter two. Use an org token, never a person's `~/.blacksmith/credentials`.
- Run macOS, Windows x86, Linux Clang and Linux GCC Dedicated on `rust-migration` after every fifth integration (user, 2026-10-22 and 2026-10-26; #357, #514): `gh workflow run ci-build.yml --ref rust-migration`. Fix a red result before further integrations. An uncovered protected push also runs them.
- After two hosted-capacity cancellations of a macOS run, root runs `gh variable set CI_MACOS_HEAVY --body blacksmith-6vcpu-macos-26`, reruns, then `gh variable delete CI_MACOS_HEAVY` when it finishes.
- The cadence dispatch also compares Windows x64 and macOS arm64 Release against Linux for three manual play saves, `generate-tgp-128-1` and `aircraft-route-manual` (#509). Prepare with `python3 tools/simulate.py platform --reference-binary PATH --output DIR`; consume with `python3 tools/simulate.py platform --stored-reference DIR --candidate PATH --output OTHER`. Missing or mismatched inputs fail. The cases keep full tick intervals, normalize GLOG, set `linkgraph.recalc_time` to 9000 and move job joins to the initial economy date plus 4500 days. They use synchronous saves and identical OpenGFX; all checkpoints require joins after the fixed end date. This subset excludes link-graph joins, cargodist, regression AIs, custom console adapters, GUI scheduling and networking; it remains #509 acceptance work until root runs the changed workflow.

## Shared-code checks and port metrics

| Command | Purpose and limit |
| --- | --- |
| `python3 tools/shared-accessors.py --base-ref origin/rust-migration` | Enforce #249 in quick CI. `tools/shared-accessor-baseline.txt` can shrink, not gain exemptions. Macros, derived fields and compound callbacks need review. |
| `python3 tools/rebound-callees.py --base-ref origin/rust-migration` | Reject duplicate bindings and forwarded service fields in quick CI. `tools/rebound-callee-baseline.txt` can shrink, not grow. Macros, lambdas, wrappers, some templates and namespace resolution remain review work. |
| `python3 tools/orphan-exports.py --base-ref origin/rust-migration` | Check Rust exports and C++ declarations for production use. `tools/orphan-export-baseline.txt` can shrink, not grow. Macros and arbitrary conditional compilation need review; reviewers also check orphan `WITH_RUST` bodies. |
| `python3 tools/port-metrics.py` | Report code lines with Pygments 2.19.2, using `uv` if needed. Original retirement may be negative; removing fork glue does not retire original C++. |
| `python3 tools/port-metrics.py --whole-fork` | Report whole-fork and simulation retirement, per-area original, retired, remaining, glue and compiled C++ lines, Rust exports and boundary function pointers. |

The `tools/test-shared-accessors.py`, `tools/test-rebound-callees.py` and `tools/test-orphan-exports.py` tests exercise their checks. Original header helpers and constants live in shared Rust header modules (#261); the checker cannot detect Rust copies. See [porting](porting.md#callees-and-shared-accessors) for the rule.

Metrics count code lines, excluding comments, directives, blank lines and Python docstrings; a second line retains historical physical counts. The whole-fork report inventories source and `WITH_RUST` guards rather than preprocessing all build defines. Compiled C++ means remaining original plus compiled fork glue; it is not a claim of behavioral completion. Ownership ports retire more original C++ than net glue and tooling added, or explain why; conversions report net glue.

## Rust-only source sweep

`tools/rust-only.py` rejects `WITH_RUST` conditionals in `src/` in quick CI.
`WITH_RUST` stays defined by CMake for dual-build harnesses under `tools/`.
The pinned original remains the oracle; do not sweep its worktree or `tools/`.

Install unifdef 2.12 (Ubuntu/Debian: `sudo apt-get install unifdef`). On the
Ubuntu 26.04 host without administrator access, obtain package 2.12-1:

```sh
mkdir -p .local/unifdef
(cd .local/unifdef && apt-get download unifdef=2.12-1 && \
  dpkg-deb -x unifdef_2.12-1_amd64.deb extracted)
.local/unifdef/extracted/usr/bin/unifdef -V
python3 tools/rust-only.py --sweep --unifdef .local/unifdef/extracted/usr/bin/unifdef
```

The sweep invokes `unifdef -DWITH_RUST -m` on tracked files under `src/`
containing `WITH_RUST`. Exit status 1 means changed, not failure. Version 2.12
misparses C++ digit separators in `strings.cpp` and `station_cmd.cpp`; those two
files use the additional `-t` flag, preserving their bytes outside directives.
The two mixed `WITH_RUST && WITH_RUST_ABI_CHECKS` directives retain only the
ABI-check term; unifdef 2.12 leaves these unknown expressions unchanged.
A developer with unifdef on PATH can omit `--unifdef`.

Surviving drafts merge PR-B's first parent, then PR-B. Rerun the sweep on **all**
`src/` files the draft touches, including cleanly merged files. Pass the explicit
file paths after `--sweep`, or rerun across all tracked `src/` files as above.
The command refuses paths outside `src/`. Keep the reference unchanged and move
any fallback-reading fixture to the pinned baseline before sweeping it.
