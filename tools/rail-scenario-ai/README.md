# Rail controller inputs (#130)

The existing owner's Padhattan save remains unchanged. `simulation/rails.py`
normalizes its optional GLOG history, then declares company zero as an AI and
installs this frozen script through the existing simulation runner. Both roles
receive identical prepared input bytes and script bytes.

The reversal, servicing and crossing cases issue ordinary game commands. The
crossing is built across the existing straight rail at tile 4183; the observer
records actual train entry/exit, and snapshots require both barred/open states.

The collision case copies the second consist's position/direction/track from
the first reference consist. It injects no crash state: real collision detection
must emit both train crash events, save crashed trains, and delete both consists.

The reservation case translates both real consists along their existing line,
parks the second and copies the source TRACK_Y reservation onto occupied tiles.
The AI skips the first train's loading order through the ordinary command and
builds PBS signals. The first train remains blocked; every checkpoint must show
that the corridor's partially extended reservations were cleared. Its
`-reverse` variant also sets `pf.reverse_at_signals`, so the blocked train
reverses after waiting. The candidate's `extension_fail` and `extension_rollback`
must be positive. These are controller extension branches, distinct from YAPF's
search rollback.

Every rail scenario sets `OPENTTD_WITNESS` for a distinct candidate and fails
when its `branch-witnesses.json` is missing or a `train.*` branch in
`rails.TRAIN_BRANCHES` is zero; the reference, `--self` and `--benchmark` runs
do not count branches.

```sh
python3 tools/migration.py simulate rail-controller --self --jobs 2
python3 tools/migration.py simulate rail-controller --jobs 2
```

The inputs do not witness articulated/unequal-length trains, bridge/tunnel
reversal, or opposing PBS green-to-red restoration. The supplied save has no
custom train NewGRFs, and the built opposing PBS signal remains red.

The Rust test `train::evidence_tests` covers the concrete stock-engine gap with
20,000 variable-length curve cases and 20,000 reversal pose/flag cases, against
the pinned original's results frozen in `train/edge-oracle.txt`
(`python3 tools/oracle-capture.py train_edge`). Its world services only record
ordering. It does not establish realistic NewGRF construction, articulated
follower movement, spacing advances, or bridge entry.
