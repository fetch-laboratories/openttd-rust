# Road controller coverage (#156)

Agent: /root/coverage_gaps_156_resume | Model: gpt-6.1-sol | Reasoning effort: high
Flooding: /root/road_flooding_coverage_156 | Model: gpt-6.1-sol | Reasoning effort: high
Crash expiry: /root | Model: gpt-6-astra | Reasoning effort: xhigh

`setup.nut` loads the committed `water-ferry.sav` in an isolated pinned-reference
runtime and constructs one crossing through `AIRoad.BuildRoad`. The first legal
owned, flat, straight rail tile is 10005. The save retains the existing map,
vehicles and company; only optional input GLOG history is removed. Its JSON
record lists the source, scripts, construction marker and
raw/normalized hashes. No map bytes are patched. The scripts retain the source
AI name `WaterScenes` so the existing company runs them without an AIPL edit.

The comparison installs `main.nut`, a read-only crash-event observer. Typed
scenario input edits place existing visible stopped train 17 and loaded road
vehicle 21 on the command-built crossing, with matching XY/Z and normal road
state. Loaded link jobs are postponed as in the existing road scenarios.
Both modes require the vehicle to crash, its crash counter to advance and exactly
one event with vehicle/site/reason/victims `21 10005 1 3`. All saved chunks and logs
compare. The event witnesses the RoadCrashNews call; viewport/news pixels and
sound output are not observed.

`roads-crash-before-expiry` and `roads-crash-expiry` reuse that input and observer
for 2238 and 2239 ticks. The unchanged original retains the single crashed head
at counter 2239 / vehicle tick 63 (the uint8 tick wraps) at the first endpoint,
then deletes pool entry 21 on the next tick, a 32-tick boundary. Both modes
require the original collision event and DATE ticks 18018/18019; no crash counter
or active-crash flag is seeded. These cases cover single-head wreck deletion;
articulated tail deletion, surviving-head handling and road-stop cleanup remain
open in #156. Reproduce with `python3 tools/migration.py simulate roads-crash
--jobs 2`, adding `--self` for reference determinism.

The existing player save supplies the no-destination case. Clear head 3's orders
and destination while retaining its cached route. At junction 10815, the original
chooses road state 10, leaves the cache intact and ends with DATE random seeds
`15484628, 1889006306`; these frozen results are required in both modes. Removing
the Rust site's draw instead chooses state 5 and seeds `2227148571, 73632321`.
The unchanged original advances 280 draws from the input seeds; this mutation
advances 279. No per-site counter or modified reference is used.

Reproduction (retain or move any previous preparation directory first):

```sh
python3 tools/migration.py build --jobs 2
PYTHONPATH=tools python3 -m simulation.roads --prepare-crossing
PYTHONPATH=tools python3 -m simulation.roads --prepare-flooding
python3 tools/migration.py simulate roads-no-destination roads-level-crossing --self --jobs 2
python3 tools/migration.py simulate roads --jobs 2
```

Scratch negatives: save `rust/openttd-kernels/src/road.rs`, apply exactly one
replacement below, run the named scenario with `python3 tools/migration.py
simulate <scenario> --jobs 2`, require failure, and restore before the next probe:

| Scenario | Original Rust expression | Scratch replacement | Required failure |
| --- | --- | --- | --- |
| `roads-no-destination` | `u64::from(self.random()) * u64::from(tracks.count_ones())` | `u64::from(0_u32) * u64::from(tracks.count_ones())` | track/shared RNG witness; DATE seeds differ |
| `roads-level-crossing` | `self.tile(IS_CROSSING, uid, u.tile) != 0` | `self.tile(IS_CROSSING, uid, u.tile) == 0` | crossing did not crash the road vehicle |
| `roads-level-crossing` | `self.q(CRASH_NEWS, id, victims, 0);` | `if victims == u32::MAX { self.q(CRASH_NEWS, id, victims, 0); }` | crash event missing while vehicle still crashes |
| `roads-flooding` | `if flooded { 2000 } else { 1 }` | `if flooded { 1 } else { 1 }` | flooded crash countdown witness; counter and shared RNG differ |
| `roads-crash-expiry` | `counter >= 2220 && self.read(id).tick & 31 == 0` | `counter >= 65535 && self.read(id).tick & 31 == 0` | crashed head survived its deletion tick; VEHS differs |

After restoring, rerun `python3 tools/migration.py simulate roads --jobs 2`.
The flooding fixture uses the same reference constructor and existing bus 21.
Typed preparation stops the bus to retain its 18 passengers and postpones loaded
link jobs by 32 days. Fifteen `BuildCanal` commands cover a 5x3 sea rectangle;
three `RemoveCanal` commands clear its interior before `BuildRoadFull` creates
road tiles 1200--1202. The twelve remaining passive canals prevent flooding.
The comparison positions the stopped, visible bus at sea level on tile 1201.
Its AI removes canal 945 once, then observes events. Ordinary water tile loops
flood the breach and call `RoadVehicle::Crash(true)`; no crash is directly invoked.
Both modes require event `21 1201 5 14`, counter 2032 and DATE RNG seeds
`1209897734, 2054087316` at tick 16080 after 300 ticks. Victims are randomized by
the unchanged `Vehicle::Crash`; the 18 passengers remain in the saved cargo.
The countdown mutation saves counter 33 and seeds `1153993592, 739546886`, with
direction 2 instead of 1, and fails the crash witness. Restore before final checks.
The crashed bus blocks road clearing at this checkpoint. This covers the flooded
counter initialization, rather than eventual wreck deletion or natural traffic
entering a flooding tile. All saved fields/logs compare with the existing masks.

These cases cover three road gaps. Other #156 branches, arbitrary maps/NewGRFs,
legacy saves, news rendering and audio remain unexercised by these cases.
