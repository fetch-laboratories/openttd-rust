# Session

<!-- Add a new entry at the end of this file. -->

## Calendar and economy clocks (#527)

- Rust owns: ten clock fields, subscriptions, Gregorian/wallclock conversion, advancement and daily/monthly/yearly dispatch.
- C++ keeps: timer shells, vehicle, presentation and network services and scheduled-console exception frame.
- Limits: Gaps remain in historical save versions, full external-AI memory-limit failures or GUI/network scheduling.
- PRs: #539.

## World-generation lifecycle (#546)

- Rust owns: generation mode/dimensions, saved local company, done/abort callbacks, abort request, generating flag, setup, orchestration, rough/rocky terrain, warm-up, completion and cleanup.
- C++ keeps: map/pool allocation, presentation, resource scratch, company backup, callbacks and generation exception frames.
- Checks: Instruction profiling selects the Rust generation entry and the original reference body.
- Limits: Gaps remain in GUI scheduling, arbitrary scripts/NewGRFs, and unexercised exception combinations (#156).
- PRs: #547.

## Simulation-session lifecycle and pause control (#551)

- Rust owns: game mode, pending switch, pause bits, scan requests/callbacks, survey prompt, `StateGameLoop`, `GameLoop`, mode switching, `SafeLoad` recovery, intro/new-game/editor startup and completion, settings publication and `CmdPause`.
- C++ keeps: allocation/decoding, script VMs, networking, presentation, chrono samples, resource/profiling objects and exception frames.
- Checks: Instruction profiling selects Rust state-loop and generation entries for the candidate and C++ bodies for the reference.
- Limits: Calendar non-advancement and a scheduled-console exception have different outcomes. Gaps remain in external GUI/network scheduling, historical saves, and arbitrary script-error combinations (#156).
- PRs: #554.

## Goals, story pages and league commands (#572)

- Rust owns: destination/content validation, command test/execute, page/element ordering and deletion, goal mutations, league ratings/links, GS event eligibility and next-sort counters.
- C++ keeps: pools/records/strings, button packing/getters, windows, rendering, save/load and script VM execution.
- Limits: The port does not claim record-storage or presentation-pixel equivalence. Window presentation remains a #156 limit.
- PRs: [#578](https://github.com/fetch-laboratories/openttd-rust/pull/578).

## Simulation-setting transitions and setting commands (#587)

- Rust owns: through `settings_table.rs`, the simulation callbacks of `settings_table.cpp` (consists; company and client service-interval defaults, ranges and vehicle updates; train and road acceleration and slope caches; airport noise and catchment; road-side, dynamic-engine, height-limit and freeform-edge checks; freeform-edge map writes; timekeeping units and minutes per year, including the editor's link-graph, vehicle and economy-date shift) and the decisions of `CmdChangeSetting` and `CmdChangeCompanySetting`. Existing owners are direct Rust callees: train, ground vehicle, station, station command, engine loading, link-graph schedule, clocks and session. Map reads and writes use `map_func::World`, which gains field-sized stores, and the `tile_map`/`void_map` helpers.
- C++ keeps: opaque setting descriptors (lookup, validation and `IntSettingDesc::ChangeValue` are whole native operations, which reenter these callbacks); `VehicleDefaultSettings`, read and written in place under a pinned layout; ini-named facades, which keep the original bodies under `WITH_RUST` until #581; presentation.
- Limits: A ship or buoy on the northern border during a freeform-edge change is not reached. The service-interval refusal is unreachable through `ChangeValue`. The new-game forced-wallclock branch needs a configuration the range callback prevents (#156).
- PRs: [#594](https://github.com/fetch-laboratories/openttd-rust/pull/594).
