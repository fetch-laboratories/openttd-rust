# Water-region and ship scenarios (#104 / ship slice of #86)

Agent: /root/water_regions_104 | Model: gpt-6.1-sol | Reasoning effort: high
Original fixture preparation: /root | Model: gpt-6-astra | Reasoning effort: xhigh

These GPLv2 scripts build through unchanged game APIs. `setup.nut` replaces
StationList only in an isolated reference runtime. The two saves are inputs,
with JSON manifests recording script/input hashes, optional funding,
and normalization. Only setup-AI identity is renamed and input GLOG removed;
other bytes remain unchanged. Ferry runs use the idle dummy AI. Structure runs
install WaterScenes with frozen scripts and per-run parameters. Route/reload
markers observe both ends without changing world state; mutation/depot markers
assert command success, newly emitted lost events, recovery and depot arrival.

Build/prepare/check:

```sh
python3 tools/migration.py build --jobs 2
python3 tools/simulate.py --prepare-water-save ferry
python3 tools/simulate.py --prepare-water-save structures
python3 tools/migration.py simulate water --self --jobs 2
python3 tools/migration.py simulate water --jobs 2
python3 tools/migration.py simulate water --soak --jobs 2
```

Both modes compare all saved fields and logs; ferry snapshots require loaded
cargo and a positive payment. Structure commands construct a lock, canals, two
isolated water patches and a cross-region aqueduct. Input/reload witnesses retain
live ship path records. The native visitor probe covers cache mutations inside
callbacks, which the ordinary ship AI cannot trigger during YAPF traversal.

Witnessed candidate runs (not `--benchmark`) also count map-query crossings:
the `water.*` counters of `branch-witnesses.json` are copied into the harness
report; the original is unchanged. Summarize the emitted report with:

```sh
python3 tools/migration.py simulate water --soak --jobs 2
PYTHONPATH=tools python3 -m simulation.ships <report.json>
```

The summary counts tracks/follower/aqueduct calls, cold cache rebuilds, returned
scalar payload upper bounds (2/5/4 respectively), and reference/candidate seconds.
Failed follower queries return only the four-byte tile sentinel; their bridge
byte is unwritten, so the five-byte term deliberately reports a maximum.
Warm queries use Rust-owned cached labels without map crossings. Counts include
startup and all simulation work; elapsed times include startup/save output.

Negative connectivity probe (temporary candidate edit; restore before committing):

```sh
cp src/pathfinder/water_regions.cpp /tmp/water-regions-original.cpp
python3 - <<'PYTHON'
from pathlib import Path
p = Path("src/pathfinder/water_regions.cpp")
s = p.read_text().replace("\tCFollowTrackWater ft;", "\tif (tile == 48809) return INVALID_TILE.base();\n\tCFollowTrackWater ft;", 1)
p.write_text(s)
PYTHON
python3 tools/migration.py simulate water-structures-route --jobs 2 # must fail
cp /tmp/water-regions-original.cpp src/pathfinder/water_regions.cpp
python3 tools/migration.py build --jobs 2
```

This removes the source fixture's west aqueduct ramp from candidate region
connectivity. It must break the route witness and/or produce semantic differences.
Ship YAPF ownership reuses this corpus. Every water scenario enables ship
controller and YAPF branch witnesses for a distinct candidate and fails when a
profile is missing or a required branch is zero (not in `--self`/`--benchmark`):

```sh
python3 tools/migration.py simulate water --soak --jobs 2
PYTHONPATH=tools python3 -m simulation.ships <report.json>
```

Ferries require intermediate goals, alternate docking, cache truncation/final
clearing and reversals; structures require retries and lost/random recovery.
The 5,120-node track and map-derived region limits are exercised with injected
native graph leaves and unchanged CYapfBaseT timing checks. These synthetic
fixtures do not claim reachable-map coverage.

Negative cache probe: temporarily replace Rust's `if path[0] == end` with
`if !cache.0.is_empty() || path[0] == end`, run the paired ferry scenario and require
VEHS path differences, then restore and rerun it. The reference and masks remain
unchanged. #86's remaining aircraft corpus is separate work.
