# Simulation harness saves

Fixed inputs for `tools/simulate.py`. They are not expected results. The first group
below supplies the `play-*` scenarios: each is the final game of one episode in the
sibling play-openttd project: an LLM player built a road network on the same 128x128
map through an MCP server, using OpenTTD 15.1. The player's company in these episode
saves is an AI company driven by a bridge AI that is missing from the harness
runtime, so on load the idle dummy AI replaces it; AI-company rather than human-
company logic is exercised. No save has a GameScript or NewGRFs. Every episode save
was written paused; the scenario's `console` lines unpause it, and the harness checks
that their settings appear in the first snapshot.

| Save | Episode |
| --- | --- |
| `opus-55-167-002.sav` | 167-game-time-variance-opus-55-xhigh-episode-002 |
| `grok-159-001.sav` | 159-game-time-variance-grok-episode-001 |
| `astra-156-003.sav` | 156-game-time-variance-astra-episode-003 |
| `opus-55-165-002.sav` | 165-game-time-variance-opus-55-episode-002 |
| `opus-5-133-009.sav` | 133-game-time-variance-opus-episode-009 |
| `fable-152-004.sav` | 152-game-time-variance-fable-episode-004 |

These are road vehicles only.

## Hand-built multimodal save (#86)

`padhattan-ridge-1996.sav` was built by hand by the repository owner in
OpenTTD 15.3 (savegame version 362) on a 128x128 temperate map: one human
company (no AI, no GameScript, no NewGRFs), started 1990 and saved paused in
August 1996 with breakdowns on. It runs a two-train rail network with signals,
one ship route and road vehicles. The separate supplemental aircraft fixture below covers the
aircraft slice of #86. The `play-padhattan-ridge-1996-*` scenarios load it with manual
distribution and with cargodist. The fixture is distributed under the fork's
GPLv2 license. Original SHA-256:
`539c68014ba7d6ea65f21a474bfe934d82a2662d8da87c7a16271e57ab487a0a`.
The harness decompresses a temporary input and removes only optional GLOG
history, checking every other chunk remains byte-identical; output GLOG entries
and all other saved fields still compare. The committed original is unchanged.
Both trains and the existing ship must move, carry cargo and earn delivery
revenue between snapshots. This does not establish aircraft, level-crossing
or industry cargo-chain coverage.

## Later owner-built multimodal save (#179)

`padhattan-ridge-2000.sav` continues the same owner-built game, supplied as
`Padhattan Ridge Transport, Dec 27th, 2000.sav`. Its GLOG identifies OpenTTD
15.3 / save version 362. The paused 128x128 temperate game has one human
company, no AI, GameScript or NewGRFs, and breakdowns enabled. Its 19 road
vehicles, three trains (11 parts), three ships and four aircraft (11 parts;
three helicopters and one plane) include passenger/mail and industry cargo.
The four buoys, four PBS signals and two level crossings are saved inputs,
not evidence of every associated branch. SUBS contains an unawarded valuables
offer and a mail subsidy already awarded to company 0.

The original bytes are distributed under the fork's GPLv2 license, with the
same owner's permission as the earlier fixture. SHA-256:
`f7ccdf6b1a6d85e732d0b2d5c954f5eedea4c20ace1da6036e5c4698e230b3e0`.
Temporary normalization uses the same byte-checked decompression/GLOG removal
as 1996; the committed original and earlier fixture remain unchanged.
`play-padhattan-ridge-2000-manual` and `-cargodist` run for two years by default,
six with `--soak`, requiring all train/ship/aircraft heads to move, carry cargo
and earn delivery revenue; aircraft must also reach flying state. Cargodist
checkpoints require crossing 4668 to be barred and released. Other crossing
states and subsidy offers/awards are recorded, without proving live crossing
audio, every PBS route, buoy traversal or a particular subsidy multiplier.
Remaining branch gaps are tracked in [#156](https://github.com/fetch-laboratories/openttd-rust/issues/156).

## Later owner-built structures save (#183)

`padhattan-ridge-2006.sav` continues the owner's game, supplied as
`Padhattan Ridge Transport, Sep 21st, 2006.sav`. GLOG identifies OpenTTD 15.3 /
save version 362. The paused 128x128 temperate game has one human company,
no AI, GameScript or NewGRFs, and breakdowns enabled. The original 47,940 bytes
are distributed under the fork's GPLv2 license with the owner's permission.
SHA-256: `603ce345ef3e50a3e49ef747dc811c2956dcfd34619db4acb10c2b67d2773067`.
Normalization decompresses OTTX to OTTN and removes only GLOG, byte-checking
all 61 remaining chunks; this original and both earlier owner saves stay unchanged.

Inventory is separate from use: 27 road vehicles, four trains (14 parts), four
ships, seven aircraft (19 parts), 42 monorail railway tiles (41 plain, one depot),
two monorail tunnels, five road bridges, one clear canal tile and two locks (six parts). There is no
rail bridge or aqueduct. The two older passenger trains remain stationary in the
reference run; the default movement/cargo/delivery checks select active trains
37/43, road vehicle 45, all four ships and all seven aircraft instead.
`play-padhattan-ridge-2006-manual` and `-cargodist` run two years, six with
`--soak`. Periodic checkpoints require monorail/tunnel wormhole, road bridge
wormhole and lock poses. These show occupancy, not every complete traversal.

`rail-owner-2006-canal`, `-lock-lowering` and `-lock-exit` reload the reference's
naturally reached ship-65 upper-lock pose at tile12064 / y1515 / z8 after a
47,000-tick warmup. Only GLOG is removed from that save. At 30/120/240 ticks,
both plain/desync modes require respectively tile12192/y1529/z8,
tile12448/y1560/z3 and tile12576/y1579/z0, comparing every semantic chunk.
This witnesses canal movement, lowering and lower-part exit. Upward lock
movement, road tunnels, rail bridges, tunnel reversal, arbitrary structures,
stationary passenger routes and random/crash branches remain in [#156](https://github.com/fetch-laboratories/openttd-rust/issues/156).

```sh
python3 tools/migration.py simulate play-padhattan-ridge-2006 rail-owner-2006 --self --jobs 2
python3 tools/migration.py simulate play-padhattan-ridge-2006 rail-owner-2006 --jobs 2
python3 tools/migration.py simulate play-padhattan-ridge-2006 --soak --jobs 2
```

## Later owner-built electrified save (#221)

`padhattan-ridge-2016.sav` continues the same game, supplied as
`Padhattan Ridge Transport, May 26th, 2016.sav` (53,108 bytes; GLOG: OpenTTD
15.3 / save version 362; same company, settings and license terms as #183).
SHA-256: `16d78671c5665e7d59cb2ac0edbf8698cf2ff03c5a50ebd4c5b1c9b32a1a19c9`.
Normalization removes only GLOG (61 chunks byte-checked); older saves are unchanged.

Inventory: 24 road vehicles, three trains (16 parts), five ships, 13 aircraft
(34 parts); 126 rail (eight signal, two depot), 42 monorail (one depot) and,
new since 2006, 41 electrified tiles (one depot); two monorail tunnels, six road
bridges, two crossings, one canal tile, two locks and six ship depots. There is
no rail bridge, road tunnel or aqueduct. The two older passenger trains are gone.
In the reference, aircraft 26, 48 and 57 crash (only 48 with cargodist) and are
deleted; every other head moves, carries cargo and earns delivery revenue,
including the nine-part electric train 5 and new ship 70. `HEADS_2016` requires
these in `play-padhattan-ridge-2016-manual`/`-cargodist` (two years, six with
`--soak`), with the #183 monorail/tunnel/road-bridge/lock occupancy checks and
the `TRAIN_BRANCHES` witnesses both distributions reach. Occupancy is sampled
every 32 days, so the new road bridge (heads 15037/15041), tunnel entry at tile
1217 and the canal tile (cargodist only) are not required; branch gaps stay in
[#156](https://github.com/fetch-laboratories/openttd-rust/issues/156).

```sh
python3 tools/migration.py simulate play-padhattan-ridge-2016 --self --jobs 2
python3 tools/migration.py simulate play-padhattan-ridge-2016 --jobs 2
python3 tools/migration.py simulate play-padhattan-ridge-2016 --soak --jobs 2
```

## Reference-built supplemental aircraft save (#86)

`aircraft-route.sav` is a separate 128x128 temperate save generated by the
pinned original with the GPLv2 committed deterministic setup AI in
[`tools/aircraft-scenario-ai`](../../tools/aircraft-scenario-ai/README.md).
It runs a small plane and helicopter between a city and metropolitan airport,
with passengers and mail, no GameScript or NewGRFs. Comparison loads omit
the setup AI and require the idle dummy AI replacement. The JSON manifest
records input/script hashes and generation settings; the linked README
gives exact reproduction and comparison commands and coverage limits.
Only optional input GLOG history is removed; every other chunk is unchanged.
The original owner-built and road saves retain their original bytes.

## Aircraft landing RNG witnesses (#156)

Agent: /root/aircraft_landing_rng_156 | Model: gpt-6.1-sol | Reasoning effort: high

`aircraft-controller-landing-*` reuses the existing eight-plane controller setup
and 411-tick reference reload. Plane 2 naturally reaches state16/position34,
speed293/target0 with passenger and mail cargo; the target station also has both.
Typed DATE seeds are the only field edits; optional GLOG is removed. The event
case additionally restores the setup's exact AIPL configuration for a passive
observer. Preparation records record input hashes/assignments; the harness
freezes the unchanged-reference runtime and scenario AI files.

With crashes enabled (setting1), seeds `(2443390976,1012692424)` produce the
third shared draw21 and crash at threshold equality; `(2443382784,1011643848)`
produce22 and survive; `(2443390968,1012691400)` produce`0x400015` and crash,
requiring the low22-bit mask. Setting0 survives after three shared draws,
versus five for the enabled draw22 survivor (the equality crash uses six).
One-tick witnesses require original DATE RNG, status8->138/counter0->3 on crash,
empty plane/mail and station cargo, block mask3328, rating0 for goods0/2/5
(nonzero status) and rating1 otherwise. Survivors retain cargo and ratings.
The 16-tick passive observer requires exactly `LANDING-EVENT 2 10064 3 10`
and counter93 in both plain/desync modes; every semantic chunk/log is compared.

```sh
python3 tools/migration.py simulate aircraft-controller-landing --self --jobs 2
python3 tools/migration.py simulate aircraft aircraft-controller --jobs 2
python3 tools/migration.py verify --jobs 2
```

Sensitivity: in the candidate only, change `maybe_crash`'s `> prob` to `>= prob`,
then separately replace only its `self.random()` with `0`. Rebuild and run the
landing selection above; each must fail. Restore exact Rust source and rerun
the suite. Never alter reference bodies or expectations to fit a mutation.
Short-strip fast jets, cheat/setting2 probability, flood crashes, finite-range
NewGRF sequences, historical saves and arbitrary airport layouts remain limits.

## Road service-routing RNG witness (#156)

`roads-service-retain` and `roads-service-refresh` reuse `opus-55-167-002.sav`.
The existing blocked vehicle 37 keeps its pose, speed and empty path cache.
Declared typed edits set its current order to automatic non-stop service
(type 66, flags 1), depot 5 / tile 3091, and last service date 727700.
The refresh case sets both DATE RNG words to 8; retain keeps the original RNG.
As in the other short road windows, link-job join dates move forward 32 days;
only optional input GLOG history is removed. The record lists every assignment.

At 37 ticks the unchanged original's `CheckIfRoadVehNeedsService` has run once:
day counter 31 becomes 32 and blocked counter 185 becomes 222. The order stays
at depot 5 with ground flags 0 on retention, or changes to closest depot 2 /
tile 6206 with implicit-order suppression flag 4 on refresh. DATE RNG words are
respectively `(2876180153, 3044549951)` and `(2072642750, 3270530758)`.
Both plain/desync modes require these states and compare all semantic chunks.

```sh
python3 tools/migration.py simulate roads-service --jobs 2
python3 tools/migration.py simulate roads-service --self --jobs 2
python3 tools/migration.py verify --jobs 2
uvx --from ruff==0.16.8 ruff check tools/
uvx --from ruff==0.16.8 ruff format --check tools/
```

Negative probe: replace only `self.random()` in `service()`'s
`chance16_i(1, 20, self.random())` with `0`, then run the focused pair above.
The probe changes retain to depot 2 / tile 6206 / ground flags 4; refresh RNG
becomes `(3680812379, 2696267631)`. Both fail their witnesses. Restore that
scratch change and rerun the pair.
This covers one stock blocked vehicle, two RNG outcomes and ordinary daily
service dispatch; unblocked traffic, articulation/trams and the other #156
coverage domains remain open. It changes no production behavior or reference source.

## Reference-built road crossing (#156)

`road-crossing.sav` adds one level crossing to the existing water-ferry fixture
through unchanged reference game commands. Its JSON record and
[`tools/road-scenario-ai`](../../tools/road-scenario-ai/README.md) record preparation
and reproduction. Comparison uses typed vehicle-position inputs and a read-only
AI crash-event observer; it changes no map bytes and compares every saved chunk.

`road-flooding.sav` uses the same constructor to enclose three sea-level road tiles
with twelve canals. Its record lists the source, typed stopped-bus/link-job
preparation and unchanged-reference commands. During comparison a single legal
canal removal lets ordinary sea flooding crash the bus; position edits preserve
its existing 18 passengers. The road scenario README records the fixed event,
counter/RNG checkpoint and negative probe. No map bytes are patched.

## Community-built arctic save (#507)

`community-arctic-1970.sav` is a 256x256 arctic game generated by the unchanged
pinned original after twenty years of community AI play, starting in 1950 with
seed 507. SimpleAI 14, AdmiralAI 25, trAIns 2.1 and ShipAI 11 remain configured
in AI company slots 0 through 3. AdmiralAI uses its supported `use_trains=0`
setting; the other scripts retain their normal transport choices. There are no
NewGRFs or GameScript. The fixture is distributed under GPLv2-compatible terms;
complete third-party source packages remain pinned downloads, uncommitted.
SHA-256: `b3714b1ce9a7a78f5c87a734bf41e9ddd71448efbbceddcf965ca55349ccb9be`.

The sibling JSON records the original revision, generation configuration,
command, source-package manifest hash and end moment. The
[community input README](../community-ai/README.md) records exact source,
version, license and archive hashes, dependencies and reproduction commands.
`community-ai-large-reload` installs these exact packages and compares full
semantic snapshots and ordinary end states, retaining opaque saved AI data.
Only optional GLOG history is normalized; no simulation field is patched.
The same-NewGRF long-game variant remains a limit of #507 while #525/#526 are
user-parked.
