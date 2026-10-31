# Ported components

## Entry format

Entries state what Rust owns, what C++ keeps, open limits and the PR. They
record scope and limits only; code comments and the PR hold mechanics. Review
status, witness counts and costs stay in the PR. Add new entries at the end of
their area file.

## Every port

Unless an entry says otherwise:

- With `OPTION_RUST=ON` (`WITH_RUST`), the C++ facade calls Rust. Otherwise,
  the original bodies compile until #581 removes them and makes Rust the only
  build.
- Rust panics and allocation failures abort (`panic = "abort"` in both
  profiles). The C ABI never unwinds. No C++ exception crosses a Rust frame.
  Identical resource-exhaustion timing is not claimed.
- Borrowed byte spans hold initialized, readable bytes in one live allocation.
  Their length is at most `PTRDIFF_MAX`; they stay immutable during the call.
  Empty spans may be null, and read-only spans may overlap. Rust retains no
  pointer after return.
- Unchanged upstream tests cover each port first. Named comparisons cover
  only listed gaps. C++ sanitizers do not instrument Rust accesses. The checks
  establish neither whole-game equivalence nor cryptographic security or
  complete secret erasure.
- Every port runs `python3 tools/migration.py verify`. #156 tracks branches
  the harness does not reach; entries name the kinds of gaps.

## Areas

| File | What it covers |
| --- | --- |
| [support.md](support.md) | Support libraries and text |
| [network-script.md](network-script.md) | Network and scripts |
| [landscape.md](landscape.md) | Landscape, water and map structures |
| [infrastructure.md](infrastructure.md) | Rail, road, waypoints and animation |
| [newgrf.md](newgrf.md) | NewGRF loading, definitions, resolvers and callbacks |
| [vehicles.md](vehicles.md) | Vehicles, orders, movement and pathfinding |
| [economy.md](economy.md) | Stations, cargo, companies and link graphs |
| [towns-industries.md](towns-industries.md) | Towns, industries and subsidies |
| [session.md](session.md) | Clocks, world generation, sessions, goals and story pages |
