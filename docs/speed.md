# Speed

## Policy

- Speed is measured on the merged branch at each stocktake, not per PR ([ADR 0027](adr/0027-speed-measured-on-merged-branch.md)). PRs do not edit `SPEED_ANCHORS` or `SPEED_INTEGRATED`.
- Ceiling and band decisions use the mean of a scenario's last three speed-pass ratios on this host, carried forward by the projection. Until a scenario has three passes here, use the mean of those it has.

| Scenario | Ceiling ([ADR 0014](adr/0014-speed-ceilings.md)) | Band starts |
| --- | --- | --- |
| Play saves | 1.6x | 1.52x |
| Generation | 1.3x | 1.235x |

| Event | Action |
| --- | --- |
| Each stocktake ([ADR 0013](adr/0013-stocktake-every-12-merges.md)) | Root profiles the merged tip in the background and records six projections in the roadmap's "Where the fork stands". |
| Projection rises over 3% since previous tip | Attribute the rise per function to the intervening PRs; fix forward in the next maintenance PR. A ready ownership PR proceeds meanwhile. |
| Record first reaches its band | Run a speed pass. |
| Scenario remains in band | Run another pass only when its projection rises at least 3% over its anchor or a repair batch crediting at least 1.5% of its loop count integrates; otherwise the projection is the record. |
| A pass has a measured ratio in band | Profile that scenario, attribute net costs, make maintenance PRs for the largest two or three, and re-measure. Otherwise the pass ends. |

The three-pass record and the 3% and 1.5% thresholds are provisional.

## Instruction projections

`R0 * (1 + 2 * max(0, I / I0 - 1))`, where `R0` and `I0` are the anchor's wall ratio and instruction count and `I` is the tip's count. Compare tips under the same anchor; after re-anchoring, recompute the previous tip from its retained count. Factor 2 is provisional.

```sh
python3 tools/migration.py simulate --instructions --jobs 2
```

- Before expensive profiling, `python3 tools/simulate.py speed prepare DIR` checks existing instruction builds' identity without games.
- Anchors carry this host's and toolchain's identity, and `simulate --instructions` rejects a profile under another's. `speed prepare` accepts a profile whose only failure is that mismatch; the next speed pass replaces the anchors.

## Wall timing

```sh
python3 tools/migration.py simulate play-opus-55-167-002-manual --benchmark 3 --jobs 2
```

Benchmark totals include startup, loading, serial link-graph work and save I/O; they are not frame latency, and a median taken outside a speed pass is not a speed result. To profile wall time, replay the kept final pair with `perf record` using `plain_commands` and `tools/simulation/game_launcher.py`, separately from timing.

## Speed passes

- From a clean checkout of the head to anchor, build and profile with `simulate --instructions`, then run `python3 tools/simulate.py speed prepare DIR --profile REPORT` and `python3 tools/simulate.py speed collect DIR` on a quiet host. A rejected collection needs a freshly prepared directory. Paste the printed `SPEED_HEAD` and `SPEED_PASS` into `tools/simulation/instructions.py`.
- `collect` holds builds and games only while its timed games run ([shared lock](deep-checks.md#running)); ready owners proceed meanwhile.
- An unrelated process crossing an uncalibrated CPU threshold alone does not reject a collection; exclude genuinely contaminated samples from anchors and record why.
- Attribute boundary cost as overhead beyond the original's inline work, ranking net costs rather than callback counts. Candidates under 0.5% of loop count remain notes on #275; batch candidates until they credit 1% together (provisional).
- When net glue is spread widely, rank the ownership or storage port removing hot crossings above another repair batch.
- Compare inclusive costs under `StateGameLoop()`, `_GenerateWorld()` or their Rust replacement against the same scenario's retained `.local/speed-reference-profiles/` profile. Keep candidate `callgrind.out` under `.local/simulation/` until the next profile. Keep no C++ frame solely for attribution.
