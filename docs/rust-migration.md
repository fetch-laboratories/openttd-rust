# OpenTTD-Rust migration tools

Building, local checks and the comparison harness. CI is in [CI](ci.md).

## Repository and reference

| Item | Location |
| --- | --- |
| Fork and `origin` | https://github.com/fetch-laboratories/openttd-rust |
| `upstream` | https://github.com/OpenTTD/OpenTTD.git |
| Integration branch | `rust-migration` |
| Original | OpenTTD 15.3, `14ec60f248547d4d062a1160f0fc26d742319888`, pinned in `migration/baseline.json` |
| Shared reference worktree and build | `.local/reference/openttd` and `.local/build-reference` in the main checkout |
| Candidate build | `build-rust` in each worktree; the game is `build-rust/openttd-rust` |
| Rust crates | `rust/openttd-kernels`; `rust/openttd-strgen`, the master language file's string-ID assignment |

- Read the original from any worktree with `python3 tools/migration.py original show FILE[:A-B]` or `original grep PATTERN [PATH ...]`, which run `git show <baseline commit>:src/FILE` and `git grep PATTERN <baseline commit> -- src/`; `original path` prints the reference worktree after checking it is clean and at the pin.
- Direct CMake builds need the pinned commit object for the native-test oracle headers: `git fetch --no-tags --depth=1 origin 14ec60f248547d4d062a1160f0fc26d742319888`.

## Prerequisites and host

Native Linux builds need a C++20 compiler, CMake, Ninja, Python 3.11 or newer, the libraries in `COMPILING.md`, OpenGFX and the Rust toolchain pinned in `rust-toolchain.toml`. Node.js runs `tools/test-ci.py`; an authenticated `gh` sends CI requests.

```sh
sudo apt-get install build-essential cmake ninja-build python3 pkg-config \
  libsdl2-dev liblzma-dev libpng-dev libcurl4-openssl-dev libfreetype-dev \
  libfontconfig-dev libharfbuzz-dev libicu-dev liblzo2-dev openttd-opengfx
```

- This host runs Ubuntu 24.04 x86-64 with these packages plus `valgrind`, `ccache`, `unifdef` and `gdb`. Keep `.local/deps` absent here: the driver puts it ahead of the system on every compiler, library and pkg-config path.
- `tools/bootstrap-local.py --install` is for an Ubuntu 26.04 host without administrator access. Do not run it here.

### Hardware

- Since 2026-11-23 the host has an Intel i7-4790K with four cores and eight threads at up to 4.4 GHz, 32 GB of DDR3-2133, and a 4 TB SSD that holds the system and all output ([output disk](#output-disk)). The GTX 970 graphics card plays no part: games run with the null or SDL dummy video driver and draw in software.
- From 2026-11-05, when the project moved to this machine, until 2026-11-23, the same processor ran at up to 4.0 GHz with DDR3-1333, on a 220 GB SSD with output on the hard disk. Wall times, compile-step times, speed-pass wall ratios and per-hour pace from before 2026-11-23 do not compare with later ones. Instruction counts do: they do not depend on clock, memory or disk speed.

## Build and verification

| Command | Use |
| --- | --- |
| `python3 tools/migration.py rust-checks` | The four Cargo checks (fmt, check, clippy, test), without building the reference. Run Clippy only this way, so the ratchet's replay of the same command stays cached. |
| `python3 tools/migration.py build` | Build both games without tests. |
| `python3 tools/migration.py verify` | Cargo checks, both games and both CTest suites. Logs go to `.local/verification/<timestamp>/`. |
| `python3 tools/migration.py tools` | Build the native Rust generators. |
| `python3 tools/migration.py simulate`, `coverage`, `deep` | See [simulation comparison](#simulation-comparison), [coverage](#original-simulation-coverage) and [deep checks](deep-checks.md). |
| `python3 tools/run-comparisons.py` | Run every `tools/*-comparison.py`: the component comparisons, not the simulation scenarios. |
| `python3 tools/migration.py exec -- CMD ...` | Run one command, such as `cargo test -p openttd-kernels`, in the driver environment. |
| `python3 tools/migration.py fmt` | Format the Rust workspace and the Python Quick checks, at the pinned versions; `--check` only reports, `--fix` also applies Ruff's lint fixes. It stages nothing. |
| `eval "$(python3 tools/migration.py env)"` | Set the driver environment in a shell that runs many commands. |
| `python3 tools/git_hooks.py install` | Install the pre-push, pre-commit and commit-msg hooks. |

- Run the candidate with `./build-rust/openttd-rust -X`, which avoids the global game folders.
- `rust-checks` runs Linux test binaries through `tools/crash_stacks.py` as Cargo's runner. A binary killed by a core-writing signal (SIGSEGV, SIGABRT and the like) gets every thread's stack from its core with `gdb`, printed whole in the failed step's output and kept in its log folder under `crash-stacks/`. On CI the script points `kernel.core_pattern` at its own folder and installs `gdb` after a crash; locally it reads a core only where the host's pattern writes a file (not apport), and otherwise prints the command that reruns the binary under `gdb`. The 32-bit, Windows and macOS test steps call Cargo directly and are not covered.
- `table::strings` is generated by `openttd-kernels`' build script, which derives each `STR_*` ID from `src/lang/english.txt` with `openttd-strgen`. Every CMake build runs `openttd-strgen` against the `table/strings.h` its C++ strgen wrote and fails on any differing name or value before linking the game or its native tests.
- Supported targets: native GNU/Linux x86-64 (aarch64 builds but no CI job tests it), macOS arm64, Emscripten and the scoped [Windows MSVC mode](#windows-msvc). Unsupported: 32-bit Windows ([ADR 0007](adr/0007-no-32-bit-windows.md)), other platforms, cross compilation, macOS universal or Intel.

### Quick cleanup checks

[Quick](ci.md#jobs-and-profiles) runs the Cargo checks and these gates; `python3 tools/preflight.py` runs them all locally, against the merge base of `--base` (default `origin/rust-migration`) and the head. Run one gate with `--step NAME`, which runs the Quick steps whose name contains NAME (`--list` names them) with Quick's pinned versions and base. `python3 tools/preflight.py --fast` is the seconds-long check before a commit: the pre-push hook's fast tier and Quick's Ruff and ty lines, with no stamp. Preserve the original's casts and control flow: a faithful truncation must not become a checked conversion to reduce debt.

| Command | Fails on |
| --- | --- |
| `python3 tools/lint-ratchet.py clippy --base-ref origin/rust-migration` | More or larger force-warned Clippy diagnostics than the merge base's `migration/clippy-debt.json`. |
| `python3 tools/lint-ratchet.py ruff --base-ref origin/rust-migration` | The same for C901 and PLR0911/12/13/15 over `tools/`, against `migration/ruff-debt.json`. |
| `python3 tools/lint-policy.py --base-ref origin/rust-migration` | Weaker lint configuration, new Rust `allow`/`expect` or Python `noqa` sites. |
| `python3 tools/growth-ratchet.py --base-ref origin/rust-migration` | Growth in tools C++, fork-added `src/` test C++, Rust exports and callback fields. |
| `python3 tools/refactor-proof.py --base-ref origin/rust-migration` | A claimed release-archive equivalence that does not hold. |
| `python3 tools/code-standards-inventory.py --gate origin/rust-migration` | A file in `migration/finished-files.txt` gaining a raw adapter or export call, an internal `execute: bool` parameter, a `GameState<Vec<...>>` scratch pool or a command over seven parameters. Each file pass adds its files to the list. |

- Function length, nesting and cognitive complexity are not measured ([ADR 0024](adr/0024-lint-ratchet-scope.md)).
- When a ratchet reports `rename not recorded`, or lint policy prints a ready carry, run the same command with `--write`.

#### Cleanup records

A change's carries and exception proposals go in `migration/cleanup-records/<branch with / as ->.json`, a JSON list of records, not in the PR body. The checks name any malformed field.

- **Carry**: a rename or move that keeps a lint row or suppression site. `{"kind":"carry","tool":"clippy"|"ruff"|"rust"|"python","from":"<base key>","to":"<head key>"}`; it needs no review. The tools print ready carries.
- **Lint exception** (`kind: lint`) or **suppression** (`kind: suppression`): add only the exact item or site row, and give `tool`, `site`, `operation`, `source_contract`, `reason`, `retiring_owner`, `retirement_condition`, and `before` and `after` (lint: the item's magnitude lists; suppression: occurrence counts). Module-wide attributes, lint groups, new `noqa` and weaker configuration are not accepted.
- **Cast reason**: an item-scoped `allow` or `expect` of `non_upper_case_globals` or the Clippy `cast_possible_truncation`, `cast_possible_wrap` or `cast_sign_loss` lints needs no record when its `reason = "..."` names the original definition or cast it preserves.
- **Growth exception** (`kind: growth`): the `counter` and `added` mapping the check prints, plus `reason`, `existing_alternatives`, `retiring_owner` and `retirement_condition`.
- **Archive proof**: `{"kind":"refactor-proof","method":"archive","renames":{}}`. A proof is not behavioural coverage of unexercised branches. A proof with renames needs an acceptance at the tested head itself; the carry rule below covers growth, suppressions and lint exceptions only.
- **Acceptance** (growth, lint exceptions and suppressions): the independent reviewer's attributed report repeats each proposal as `{"head":"<full SHA it reviewed>","accept":[<proposal>]}` in a `cleanup-acceptance` fence; a block whose `head` is not a full 40-digit SHA fails the gates. An acceptance matches a proposal on every field but its wording (`reason`, `retiring_owner`, `retirement_condition`, `existing_alternatives`, `source_contract`), so rewording keeps it, and it holds at any later head whose record is unchanged: an edit to the excepted file or item keeps it, and [review](workflow.md#review) covers that edit. A change to `counter` or `added`, or to a suppression's or lint exception's `tool`, `site`, `operation`, `before` or `after`, needs a new acceptance. The exception is a suppression of `unsafe_code`: its acceptance holds at its `head`, or later only while the item's text is unchanged, since an accepted `unsafe` is an argument about one body.
- **Pending review**: a growth, suppression or lint exception with a record but no acceptance prints `pending review`. It passes preflight, PR runs and requested runs, and fails in the merge group. The gates read the event from `--event`, then `GITHUB_EVENT_NAME`, then default to `pull_request`. A site or growth with no record fails on every event. `ci.py enqueue` refuses while any record lacks its acceptance, naming each one.

### Generated ABI probes and fixture defaults

- Add ABI declarations to `layouts!` in the owning Rust module, with `rust_field => cpp_field` for aliases and `width field: RustType` for a native-width check. Add no second C++ field list or numeric type ID.
- Fixtures specify the callbacks they use; other callback defaults abort.

## Original simulation coverage

`python3 tools/migration.py coverage --jobs 4` measures the default scenarios' coverage of the pinned original into `.local/coverage`. Re-measure after scenario inputs change. It is neither equivalence nor integration evidence. `--timeout SECONDS` sets the per-game limit (default 600).

## Compiler cache

Cache build outputs only when the key covers everything that determines them; do not cache test results. Worktrees share the main checkout's reference, reference build and `.local/compiler-cache/`. After a configure-time environment change, such as a dependency prefix, use a fresh build directory: the driver skips CMake configure when the cached `-D` values match.

## Output disk

All output lives on the SSD in ordinary folders, and each worktree's `.local` is a real folder ([ADR 0050](adr/0050-output-on-the-ssd.md)). No Git hook makes links.

- Until 2026-11-23 output lived on the hard disk behind symlinks, and records from then embed `/mnt/storage/openttd-rust/` paths. That folder now links to `~/storage/openttd-rust`, where `main-local` links to the main checkout's `.local` and each live `worktree-local/<key>/local` to its worktree's `.local`, so old paths still resolve while the hard disk is mounted. Write no new path through them.
- `~/storage/openttd-rust` also keeps what had no live owner: output of removed worktrees, `old-host-ubuntu2604-20261105` from the previous host, `overhaul-20261106`, and the old offload script and log.
- The checkouts in `.local/worktrees/root-*` were made on the hard disk, so their build files name `/mnt/storage` paths. The `root-profile-*` ones are kept as evidence; do not rebuild in them.

## Simulation comparison

`python3 tools/migration.py simulate` builds both games and runs `tools/simulate.py`. Scenario comparisons run on CI, not on this host; run one locally only by name, to diagnose a failure ([ADR 0020](adr/0020-scenario-comparisons-on-ci.md)).

- Add scenarios and their prepare and check hooks to the component's module in `tools/simulation/`; `tools/simulation/registry.py` assembles them, and a hook for other modules' scenarios goes in its `SHARED_HOOKS`.
- `MASKS` (`tools/simulation/savegame.py`) masks the random save ID, build revision, NewGRF version and the original's uninitialized `round_trip_time`; a port touching the last needs its own check. Record a known port divergence in `KNOWN_FAILURES` by first divergence and an open issue, not as a new mask.
- After changing scenarios, comparison cases, `MASKS` or `KNOWN_FAILURES`, run `python3 tools/harness_inventory.py --write`; after changing comparison cases, `python3 tools/run-comparisons.py --record`. Quick fails on an unrecorded change.
- `network-protocol-loopback` bounds each session wait at 120 seconds (or the game timeout when smaller). A client stuck in the original's `TCPConnecter` race (#693, #1016) fails that wait with `TCPConnecter stuck in Resolving (#693)`, and the session runs once more, under three conditions: `src/network/core/tcp_connect.cpp` and `src/thread.h` equal the pinned original's, checked at run time; at most one retry per scenario run, across both server roles; and each retry prints a `::warning::` annotation and sets `network_protocol_retries` in the shard report. A second stall fails. `content-protocol` has no retry.
- Community AI, arctic and NewGRF scenarios use pinned downloads ([input setup](../migration/community-ai/README.md), [evidence](porting.md#evidence)). Saves are described in `migration/saves/README.md`.

See [speed](speed.md) for benchmarks and [deep checks](deep-checks.md) for soak, sanitizer, threaded and multiplayer runs.

## Platform linkage

`rust_archive(build)` in `tools/migration.py` locates the Rust archive. `tools/check-rust-platform.py` checks linkage and refusals of unsupported configurations.

### macOS

- Requires `CMAKE_OSX_ARCHITECTURES=arm64`, a deployment minimum of 11.0 or newer, and matching `SDKROOT` and `MACOSX_DEPLOYMENT_TARGET` for Rust. Linux results do not establish Darwin support.

### Emscripten

- Requires the `wasm32-unknown-emscripten` std, emsdk 4.0.23 and `-fwasm-exceptions`.

### Windows MSVC

- Supported: VS 2022 MSVC, x64, single-config Ninja, `RelWithDebInfo`, `OPTION_USE_ASSERTS=ON` and the static release CRT, with the `x86_64-pc-windows-msvc` std.
- CMake refuses every other configuration, naming the requirement.

## Shared-code checks and port metrics

Quick runs all but `port-metrics.py`, against `--base-ref origin/rust-migration` where they take one. Each baseline file only shrinks.

| Command | Checks |
| --- | --- |
| `python3 tools/shared-accessors.py` | The [shared-accessor rule](porting.md#callees-and-shared-accessors); macros, derived fields and compound callbacks need review. |
| `python3 tools/rebound-callees.py` | Duplicate bindings and forwarded service fields; macros, lambdas, wrappers, some templates and namespace resolution need review. |
| `python3 tools/rust-only.py` | `WITH_RUST` conditionals in `src/`. |
| `python3 tools/ffi-bool-returns.py` | Rust-defined C `bool` returns. |
| `python3 tools/orphan-exports.py` | Rust exports and C++ declarations without production use; tests and harnesses do not count. An export used only by uncalled C++ members needs a row in `tools/uncalled-export-baseline.txt`, and one used only by uncalled free functions a row in `tools/uncalled-function-baseline.txt`, each naming the owner issue that retires it. `--report-uncalled` also lists uncalled free functions and those only tests or tools call. |
| `python3 tools/port-metrics.py` | Code-line counts and original retirement; `--whole-fork` adds source areas, C++ evidence and exports; `--functions [AREA ...]` adds remaining original lines per file and function. |

- No checker detects Rust copies of original header helpers and constants ([porting](porting.md#callees-and-shared-accessors)).
- Metric scope: version 3 aligns each file with the pin by the minimal diff (`--diff-algorithm=minimal`); retired and glue from version 2's default diff are not comparable with version 3's, while compiled C++ is unchanged. Neither version counts extracted-original comparison fragments or C++ embedded in Python strings, so `fork_cpp` and `candidate_cpp_upper_bound` cover only the listed inventory. Report a later coverage correction separately from deletion. Raw `rust` totals include oracle text under `rust/`; report implementation and oracle data separately. No line metric claims behavioural completion.
