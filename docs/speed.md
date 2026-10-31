# Speed

## Policy

Speed is checked on the merged branch; PRs have no speed check (user, 2026-10-22).
Ceiling and band decisions use the mean of a scenario's last three quiet-pass ratios,
carried forward by the projection; never use one pass alone.

| Scenario | Ceiling | Band starts |
| --- | --- | --- |
| Play saves | 1.6x (user, 2026-10-28) | 1.52x |
| Generation | 1.3x (#282) | 1.235x |

| Event | Action |
| --- | --- |
| Every second integration | Root profiles the integrated tip in the background and records six projections in the roadmap's "Where the fork stands". |
| Projection rises over 3% since previous tip | Attribute the rise per function to the intervening PRs; fix forward in the next maintenance PR. A ready ownership PR proceeds meanwhile. |
| Record first reaches its band | Run a speed pass. |
| Scenario remains in band | Run another pass only when its projection rises at least 3% over its anchor or a repair batch crediting at least 1.5% of its loop count integrates; otherwise the projection is the record. |
| A pass has a measured ratio in band | Profile that scenario, attribute net costs, make maintenance PRs for the largest two or three, and re-measure. Otherwise the pass ends. |

- Three-pass record is provisional: P2016's five passes ranged 1.453x to 1.523x, mean 1.486x, while loop count moved about 1.3%.
- The 3% rise trigger is provisional.
- The repeat-pass rule is provisional: the three 2026-10-26 P2016 passes spread 3.3%; six repairs credited 1.13% of loop count.

## Instruction projections

`R0 * (1 + 2 * max(0, I / I0 - 1))`, where `R0` and `I0` are the anchor's wall ratio and instruction count and `I` is the tip's count. Compare tips under the same anchor; after re-anchoring recompute the previous tip from its retained count. Counts do not depend on host load.

Factor 2 is provisional: measured wall sensitivity to loop Ir ranges from 0.75 (TGP) to 1.54 (P2016) (#356). Memory stalls, excluded work and kernel/I/O costs are outside the proxy.

```sh
python3 tools/migration.py simulate --instructions --jobs 2
```

- Profiles six scenarios with witnesses off and `-r 64x64`; canonical exit states and full logs are compared, including on reference-count cache hits.
- Exact Callgrind selectors: `StateGameLoop()` and, for generation, `_GenerateWorld()`. Never use a wildcard: it toggles off collection in the nested link-graph pause helper. Loading, saving and early generation setup are excluded.
- The driver verifies release `line-tables-only` debug information. It caches only reference Ir; profile times do not enter `plain_speed`.
- The current anchors require the main checkout's pinned zlib and lzma runtime. Prefix profiles and speed passes with `LD_LIBRARY_PATH=<main>/.local/tip-profile-anchored-runtime-zlib-20261023/runtime/usr/lib/x86_64-linux-gnu`.

## Wall timing

`plain_speed` (#155) reports paired process wall seconds, candidate/reference ratios and their median. A ratio is null for missing exit saves, failed runs, different end moments or semantic differences; timing never changes pass/fail.

```sh
python3 tools/migration.py simulate play-opus-55-167-002-manual --benchmark 3 --jobs 2
```

- Use the same build settings, profiling counters off and the pinned manual-distribution save. The driver builds with two jobs and runs one scenario worker.
- Benchmark mode runs plain pairs, compares exit saves and full logs, and uses #154's serial offline thread-failure fallback (`execution` in the report). Its total times include startup, loading, serial link-graph work and save I/O; they are not threaded frame latency.
- Keep the final pair, runtimes, HOME/XDG directories and libraries for `perf record` replay using `plain_commands` and `tools/simulation/game_launcher.py`. Profile separately from timing.
- The coordination lock does not reserve a quiet host. Ordinary timeout polling can add about 50 ms. Compile load moves P2016 medians 2-7%; concurrent games move them about 10% either way.

## Speed passes

- From a clean checkout of the head to anchor, build and profile with `simulate --instructions`; run `python3 tools/simulate.py speed prepare DIR --profile REPORT`, then `python3 tools/simulate.py speed collect DIR`. Preparation freezes runtimes and proves identity without games; collection runs 144 interleaved timed games and prints `SPEED_PASS` rows.
- Measure six wall ratios on a quiet host as the median of four interleaved three-pair medians; record ratios and loop counts as new anchors. Wall medians are taken only in passes: a loaded median is not a speed result.
- Pause builds, links, profiles and other games only for timed collection; prepare runtimes beforehand and release the hold when the last timed game exits. Parse, attribute and report afterwards. See [deep checks](deep-checks.md#running) for the shared lock.
- Record process and load observations. An unrelated process crossing an uncalibrated CPU threshold alone does not reject a collection; exclude genuinely contaminated samples from anchors and record why.
- Attribute boundary cost as overhead beyond the original's inline work, ranking net costs rather than callback counts. Candidates under 0.5% of loop count remain notes on #275; batch candidates until they credit 1% together (provisional).
- When net glue is spread widely, rank the ownership or storage port removing hot crossings above another repair batch; stop attribution at the selected thresholds. P2016 on 2026-10-28: glue was 99% of the gap and its top ten glue functions held 25%.
- Compare inclusive costs under `StateGameLoop()`, `_GenerateWorld()`, or their Rust replacement, against the same scenario's retained `.local/speed-reference-profiles/` profile; the reference Ir cache has no per-function data. Keep candidate `callgrind.out` under `.local/simulation/` until the next profile. Keep no C++ frame solely for attribution.

## Anchors

An anchor is a `SPEED_ANCHORS` record in `tools/simulation/instructions.py`: `{head, ratio, Ir, identity}` for a quiet pass; a separate `integrated` record holds its head and Ir. Empty anchors permit collection only, not a passing guard.

`SPEED_PASS` keeps the last three quiet-pass ratios, oldest first; collection prints the shifted history. `projected_ratio` uses the latest anchor, while ceiling and band decisions use the three-pass mean carried forward by the projection.

- Quiet passes replace the whole anchor and refresh `integrated` together; never reset anchor Ir alone.
- PRs never edit `SPEED_ANCHORS` or `SPEED_INTEGRATED` (user, 2026-10-22).
