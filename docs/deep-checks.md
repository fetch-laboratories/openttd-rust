# Deep checks

## Policy

Deep checks (#503) run only on user request (user, 2026-10-31). Root runs the requested members, all by default, on the integrated tip in the background at idle CPU and I/O priority, surviving session end; read the report before the next integration after completion. Never restart an interrupted run without a request.

A failure alone does not block integration. Bisect integrations since the last clean deep check: a newly introduced divergence is a code regression under the [rollback rule](workflow.md#automatic-rollback-and-wind-down); an older one becomes a #156 fidelity item or a new issue.

## Running

```sh
python3 tools/migration.py deep [SELECTOR...]
```

| Member | Scope |
| --- | --- |
| `simulate --soak` | 520 scenarios versus 445 by default |
| `simulate --sanitize` | ASan and UBSan games (#505) |
| Threaded and C++ TSan | Threaded comparisons (#506) |
| `simulate --multiplayer` | Dedicated server and SDL client (#510) |

- Run on a clean, fixed isolated checkout on the developer host, never GitHub Actions, at `nice -n 19` and `ionice -c 3`. Each member uses the ordinary driver and a 7200-second per-game timeout.
- `.local/deep/<time>-<commit>/report.json` records the commit, member exit status, wall and summed scenario seconds, failed scenarios and first differences; logs accompany it. The soak report's `seconds` is wall time; summed scenario seconds divided by `--jobs // 2` approximates it.
- HEAD and cleanliness are checked before and after each member; drift fails and stops the set. Selectors apply to every member and mark the report partial: a partial report is a smoke check, not a deep check. Planned members are on #503.
- Clone-wide exclusive `deep.lock` (`LOCK_NAME` in `tools/deep.py`) covers the whole deep run and a speed pass's `collect` from launch preparation through timed collection. A deep run finding it held exits 0 as skipped; collection waits and prints holders.

## Sanitized games

| Command | Purpose and limits |
| --- | --- |
| `python3 tools/migration.py simulate --threaded` | #506: unchanged road Opus and Padhattan 2000 inputs, SDL offscreen and native graph workers. Tests game and main-thread occupied-road queries, saturated landscaping credits, distinct threads and calendar phase. |
| `python3 tools/migration.py build --tsan` or `python3 tools/migration.py simulate --tsan --threaded` | #506: separate `build-reference-tsan` and `build-rust-tsan`, C++ `-fsanitize=thread -fno-omit-frame-pointer`. No Rust instrumentation; incompatible with `--sanitize`. |
| `python3 tools/migration.py simulate --multiplayer` | #510: host deep tier only; original dedicated server and SDL offscreen fork client run the Opus save, then swap roles. |
| `python3 tools/migration.py build --sanitize` or `python3 tools/migration.py simulate --sanitize` | #505: separate `build-reference-sanitized` and `build-rust-sanitized`; C++ ASan/UBSan and Rust ASan via pinned toolchain and `RUSTC_BOOTSTRAP=1`. Rust has no UBSan and its precompiled standard library is uninstrumented. |

- Threaded games compare snapshots, RNG and complete combined output with existing masks and GLOG normalization; each input also has same-build pairs in plain and desync modes. Missing saves or markers fail. The next-month callback saves during calendar advancement, not at end of tick; `scrollto instant` and `32bpp-anim` avoid camera and startup blitter timing differences.
- TSan classifies reference-only, candidate-only and both-role findings separately; unknown reports fail. Reference-only findings can reflect accesses moved to uninstrumented Rust. The fixed inputs do not establish arbitrary SDL determinism, interactive RNG equality or Rust race freedom. Full reports remain with saves.
- Multiplayer substitutes reference revision constants only in the candidate fixture's generated revision object: unmodified builds reject the join. This does not establish release-version or arbitrary NewGRF interoperability. It checks sync, real threads, stop saves, RNG and progress, exits and network errors, and compares equivalent-role saves including AIPL. It exercises spectator networking and server unpause, not client construction commands; logs, commands, identity and failures are retained.
- Sanitizer games disable leak detection because `RLIMIT_NPROC=0` prevents LeakSanitizer's thread; UBSan suppressions are in `tools/simulation/ubsan-suppressions.txt`. Reports common to both roles are recorded as original behaviour; candidate-only reports fail the scenario, reference-only reports are recorded. A suppression names reference undefined behaviour in the same scenario and its original site.
- Preparation games are sanitized but their reports are not compared; ASan stops at its first report. Reference-gap probes still link the ordinary `build-rust` archive, so a sanitized run needs a current ordinary candidate build (provided by the deep soak member). `--instructions`, `--benchmark` and `--frozen-runtimes` are refused.
