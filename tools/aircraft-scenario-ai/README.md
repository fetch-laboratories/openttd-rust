# Supplemental aircraft fixture (#86)

Agent: /root/aircraft_fixture_86 | Model: gpt-6.1-sol | Reasoning effort: high

The owner authorized this supplemental fixture in #111 and on #86 on
2026-10-04. The GPLv2 setup AI runs only in an isolated copy of the unchanged
pinned OpenTTD 15.3 reference runtime. It generates a 128x128 temperate map
(seed 86, 1980), borrows through the ordinary loan command, and builds a city
and metropolitan airport near two passenger-accepting towns. A small plane
and a helicopter carry passengers and mail between them. Towns and engines
are scanned in deterministic list order; construction uses unchanged AI APIs.

`migration/saves/aircraft-route.json` records the baseline and script
hashes, complete generation settings, vehicle IDs, airport tiles,
setup marker, raw/normalized hashes and save moment. The uncompressed save
is under 1 MB. Preparation removes only optional input GLOG history and checks
every other chunk byte-for-byte. No funding or vehicle field is patched.
The random DATE save ID may differ when regenerating this input.

The comparison runtime omits the setup AI. Each run must log its replacement
by the idle dummy AI and must emit no setup marker. There is no GameScript.
`aircraft.py` requires both aircraft to move in plain and desync runs; desync
checkpoints additionally require loaded cargo, positive delivery revenue,
visits to both airports and flying state. Manual/cargodist runs compare every
saved chunk and log. Four-year soak runs retain these witnesses. This is a
transport corpus; it does not establish all airport layouts, crashes, NewGRFs,
legacy aircraft saves or complete controller equivalence.

```sh
python3 tools/migration.py build --jobs 2
python3 tools/simulate.py --prepare-aircraft-save
python3 tools/migration.py simulate aircraft --self --jobs 2
python3 tools/migration.py simulate aircraft --jobs 2
python3 tools/migration.py simulate aircraft --self --soak --jobs 2
python3 tools/migration.py simulate aircraft --soak --jobs 2
```

A scratch negative probe changes `uint spd = v->acceleration * 77;` to
`uint spd = v->acceleration * 76;` in the candidate's `src/aircraft_cmd.cpp`.
Run `python3 tools/migration.py simulate aircraft-route-manual --jobs 2`;
it must report semantic differences at the first desync checkpoint. Restore
the original source and rerun the clean aircraft pair before committing.
Never mutate the pinned reference or change fixture fields to fit this probe.
