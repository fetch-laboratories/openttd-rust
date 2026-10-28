# Seeded command scenarios (#508)

`simulation/commands.py` uses the ordinary scenario AI installer and core game,
log, checkpoint and exit-save comparisons. Company zero retains its existing
assets; preparation changes its AI flag, money and AIPL configuration. Generated
inputs come from a one-tick reference game. Both roles receive the same input.

The script builds a small road depot and bus with two depot orders on a flat
patch. Its Park-Miller PRNG never calls the game's Random. Initial infrastructure,
vehicle locations and map corners anchor random nearby coordinates; out-of-map
coordinates become an invalid tile instead of wrapping a row. Six interleaved
families get equal command counts. A short opening sequence guarantees successful
and failed activity, including order MOVE, SKIP and an actual flags change. Each
command records its index, API operation, argument carrier, return value and
`AIError.GetLastErrorString()` on failure. The report retains every outcome and
per-operation success/error-string counts. Missing or incomplete logs, wrong RNG
completion, absent family success/failure and missing order activity fail.

The source audit uses `src/script/api/script_{rail,road,order,vehicle,tile}.cpp`:
rail track/depot/signal/conversion, road build/remove/depot/conversion, bus/truck
and rail stations, order insertion/move/skip/flags/removal/conditional insertion,
vehicle build/clone/start-stop/reverse/depot/refit, and terraform/level/clear all
use actual script APIs. Script precondition failures are recorded separately by
their error string. One API call can issue several native commands (long road,
flags changes); the unit counted here is an API operation. Script errors collapse
some native StringIDs; equal error strings do not establish native error precedence.

The default play-save case issues 360 commands. `--soak` selects seeds 1, 48271
and 1234567 on both that play save and a 128 by 128 TGP map, with 12,000 commands
per case (2,000 per family). The existing deep soak member discovers these cases.

```sh
python3 tools/migration.py simulate commands-play-1 --jobs 2
python3 tools/migration.py simulate --soak commands --jobs 2
python3 tools/test-command-scenarios.py
```

This does not expose raw command flags, deity/editor/network clients, bankruptcy
or water-owner execution, arbitrary native-invalid enum/ID inputs filtered by the
API, pool exhaustion or NewGRF-specific command branches. Random family presence
alone closes no #156 branch gap. The ordinary comparator retains its existing
round-trip-time mask; this scenario adds no masks, retries or accepted failures.
