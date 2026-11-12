# Deep checks

## Policy

- Deep checks run only on the user's request, plus all members once at cleanup exit and once at reader-phase (phase 3) exit ([ADR 0017](adr/0017-deep-checks-on-request.md)).
- Root runs the requested members, all by default, on the integrated tip in the background, and reads the report before the next integration after it completes.
- Do not restart an interrupted run without a request.
- A failure alone does not block integration. Bisect integrations since the last clean deep check: a newly introduced divergence is a code regression under the [rollback rule](root-workflow.md#automatic-rollback-and-wind-down); an older one becomes a #156 fidelity item or a new issue.

## Running

```sh
python3 tools/migration.py deep [SELECTOR...]
```

- Members: `simulate --soak`, `simulate --sanitize` (ASan and UBSan games and sanitized Cargo tests), threaded games with C++ TSan, and `simulate --multiplayer` (dedicated server and SDL client).
- Run from a clean checkout of the tip that nothing changes during the run. The report is `.local/deep/<time>-<commit>/report.json`, with logs beside it.
- Selectors mark the report partial: a partial report is a smoke check, not a deep check.
- A deep run and a speed pass's `collect` share `deep.lock`. A deep run that finds it held exits 0 as skipped, which is not a pass.

## Sanitized games

| Command | Limits |
| --- | --- |
| `python3 tools/migration.py simulate --threaded` | Unchanged road Opus and Padhattan 2000 inputs. |
| `python3 tools/migration.py build --tsan` or `python3 tools/migration.py simulate --tsan --threaded` | Rust is not instrumented, so reference-only findings can reflect accesses moved to Rust. |
| `python3 tools/migration.py simulate --multiplayer` | Exercises spectator networking and server unpause, not client construction commands, release-version or arbitrary NewGRF interoperability. |
| `python3 tools/migration.py build --sanitize` or `python3 tools/migration.py simulate --sanitize` | Linux only. The precompiled standard library is uninstrumented. |

- The threaded inputs do not establish arbitrary SDL determinism, interactive RNG equality or Rust race freedom.
- Sanitizer reports common to both roles are recorded as original behaviour; candidate-only reports fail the scenario; reference-only reports are recorded. A UBSan suppression in `tools/simulation/ubsan-suppressions.txt` names reference undefined behaviour in the same scenario and its original site.
