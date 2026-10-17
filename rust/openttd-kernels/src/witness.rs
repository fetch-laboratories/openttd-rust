/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! One branch-witness facility for every port: named relaxed counters behind a
//! single enable flag. A hit is a Rust atomic load (and add when enabled); it
//! never crosses into C++. `src/rust/witness.cpp` enables it from
//! `OPENTTD_WITNESS` and writes `$HOME/branch-witnesses.json` at exit.
#![allow(unsafe_code)]
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[cfg(test)]
/// First train controller/reservation counter; indices follow the train order.
pub const TRAIN: usize = 0;
/// First ship controller counter.
#[cfg(test)]
pub const SHIP: usize = 31;
/// First ship YAPF counter; 0..13 are the search statistics.
#[cfg(test)]
pub const SHIP_YAPF: usize = 47;
/// First rail YAPF counter.
#[cfg(test)]
pub const RAIL: usize = 64;
/// First water-region map-query counter.
#[cfg(test)]
pub const WATER: usize = 87;

/// `train.tick_first` witness.
pub const TRAIN_TICK_FIRST: usize = 0;
/// `train.tick_second` witness.
pub const TRAIN_TICK_SECOND: usize = 1;
/// `train.reversal` witness.
pub const TRAIN_REVERSAL: usize = 2;
/// `train.wormhole_swap` witness.
pub const TRAIN_WORMHOLE_SWAP: usize = 3;
/// `train.unequal_before` witness.
pub const TRAIN_UNEQUAL_BEFORE: usize = 4;
/// `train.unequal_after` witness.
pub const TRAIN_UNEQUAL_AFTER: usize = 5;
/// `train.depot_start` witness.
pub const TRAIN_DEPOT_START: usize = 6;
/// `train.depot_service` witness.
pub const TRAIN_DEPOT_SERVICE: usize = 7;
/// `train.red_oneway` witness.
pub const TRAIN_RED_ONEWAY: usize = 8;
/// `train.red_twoway` witness.
pub const TRAIN_RED_TWOWAY: usize = 9;
/// `train.force_signal` witness.
pub const TRAIN_FORCE_SIGNAL: usize = 10;
/// `train.stuck_retry` witness.
pub const TRAIN_STUCK_RETRY: usize = 11;
/// `train.stuck_reverse` witness.
pub const TRAIN_STUCK_REVERSE: usize = 12;
/// `train.cross_bar` witness.
pub const TRAIN_CROSS_BAR: usize = 13;
/// `train.cross_unbar` witness.
pub const TRAIN_CROSS_UNBAR: usize = 14;
/// `train.collision` witness.
pub const TRAIN_COLLISION: usize = 15;
/// `train.crash_delete` witness.
pub const TRAIN_CRASH_DELETE: usize = 16;
/// `train.free_wagon_delete` witness.
pub const TRAIN_FREE_WAGON_DELETE: usize = 17;
/// `train.articulated_move` witness.
pub const TRAIN_ARTICULATED_MOVE: usize = 18;
/// `train.first_tile_rail` witness.
pub const TRAIN_FIRST_TILE_RAIL: usize = 19;
/// `train.wormhole_exit` witness.
pub const TRAIN_WORMHOLE_EXIT: usize = 20;
/// `train.depot_reentry` witness.
pub const TRAIN_DEPOT_REENTRY: usize = 21;
/// `train.controller_extension` witness.
pub const TRAIN_CONTROLLER_EXTENSION: usize = 22;
/// `train.extension_fail` witness.
pub const TRAIN_EXTENSION_FAIL: usize = 23;
/// `train.extension_rollback` witness.
pub const TRAIN_EXTENSION_ROLLBACK: usize = 24;
/// `train.extension_opposing_red` witness.
pub const TRAIN_EXTENSION_OPPOSING_RED: usize = 25;
/// `train.extension_restore` witness.
pub const TRAIN_EXTENSION_RESTORE: usize = 26;
/// `train.free_path` witness.
pub const TRAIN_FREE_PATH: usize = 27;
/// `train.choose_track` witness.
pub const TRAIN_CHOOSE_TRACK: usize = 28;
/// `train.order_lookahead` witness.
pub const TRAIN_ORDER_LOOKAHEAD: usize = 29;
/// `train.order_restore` witness.
pub const TRAIN_ORDER_RESTORE: usize = 30;
/// `ship.lock_up` witness.
pub const SHIP_LOCK_UP: usize = 31;
/// `ship.lock_down` witness.
pub const SHIP_LOCK_DOWN: usize = 32;
/// `ship.reverse` witness.
pub const SHIP_REVERSE: usize = 33;
/// `ship.rotate` witness.
pub const SHIP_ROTATE: usize = 34;
/// `ship.depot_leave` witness.
pub const SHIP_DEPOT_LEAVE: usize = 35;
/// `ship.auto_service` witness.
pub const SHIP_AUTO_SERVICE: usize = 36;
/// `ship.buoy` witness.
pub const SHIP_BUOY: usize = 37;
/// `ship.loading` witness.
pub const SHIP_LOADING: usize = 38;
/// `ship.water_change` witness.
pub const SHIP_WATER_CHANGE: usize = 39;
/// `ship.aqueduct` witness.
pub const SHIP_AQUEDUCT: usize = 40;
/// `ship.rotation_reload` witness.
pub const SHIP_ROTATION_RELOAD: usize = 41;
/// `ship.depot_search` witness.
pub const SHIP_DEPOT_SEARCH: usize = 42;
/// `ship.build` witness.
pub const SHIP_BUILD: usize = 43;
/// `ship.economy_day` witness.
pub const SHIP_ECONOMY_DAY: usize = 44;
/// `ship.path_cache` witness.
pub const SHIP_PATH_CACHE: usize = 45;
/// `ship.depot_enter` witness.
pub const SHIP_DEPOT_ENTER: usize = 46;
/// `ship_yapf.region_nodes` witness.
pub const SHIP_YAPF_REGION_NODES: usize = 47;
/// `ship_yapf.track_nodes` witness.
pub const SHIP_YAPF_TRACK_NODES: usize = 48;
/// `ship_yapf.retries` witness.
pub const SHIP_YAPF_RETRIES: usize = 49;
/// `ship_yapf.track_limits` witness.
pub const SHIP_YAPF_TRACK_LIMITS: usize = 50;
/// `ship_yapf.intermediate` witness.
pub const SHIP_YAPF_INTERMEDIATE: usize = 51;
/// `ship_yapf.cache_truncations` witness.
pub const SHIP_YAPF_CACHE_TRUNCATIONS: usize = 52;
/// `ship_yapf.final_region_clears` witness.
pub const SHIP_YAPF_FINAL_REGION_CLEARS: usize = 53;
/// `ship_yapf.lost` witness.
pub const SHIP_YAPF_LOST: usize = 54;
/// `ship_yapf.random_draws` witness.
pub const SHIP_YAPF_RANDOM_DRAWS: usize = 55;
/// `ship_yapf.reverse_origins` witness.
pub const SHIP_YAPF_REVERSE_ORIGINS: usize = 56;
/// `ship_yapf.blocked_fallbacks` witness.
pub const SHIP_YAPF_BLOCKED_FALLBACKS: usize = 57;
/// `ship_yapf.region_limits` witness.
pub const SHIP_YAPF_REGION_LIMITS: usize = 58;
/// `ship_yapf.alternate_docking` witness.
pub const SHIP_YAPF_ALTERNATE_DOCKING: usize = 59;
/// `ship_yapf.choose_calls` witness.
pub const SHIP_YAPF_CHOOSE_CALLS: usize = 60;
/// `ship_yapf.reverse_calls` witness.
pub const SHIP_YAPF_REVERSE_CALLS: usize = 61;
/// `ship_yapf.blocked_calls` witness.
pub const SHIP_YAPF_BLOCKED_CALLS: usize = 62;
/// `ship_yapf.reverse_chosen` witness.
pub const SHIP_YAPF_REVERSE_CHOSEN: usize = 63;
/// `rail.choose` witness.
pub const RAIL_CHOOSE: usize = 64;
/// `rail.reverse` witness.
pub const RAIL_REVERSE: usize = 65;
/// `rail.depot` witness.
pub const RAIL_DEPOT: usize = 66;
/// `rail.safe` witness.
pub const RAIL_SAFE: usize = 67;
/// `rail.cached` witness.
pub const RAIL_CACHED: usize = 68;
/// `rail.uncached` witness.
pub const RAIL_UNCACHED: usize = 69;
/// `rail.partial_paths` witness.
pub const RAIL_PARTIAL_PATHS: usize = 70;
/// `rail.bounded_depot` witness.
pub const RAIL_BOUNDED_DEPOT: usize = 71;
/// `rail.found` witness.
pub const RAIL_FOUND: usize = 72;
/// `rail.allow90` witness.
pub const RAIL_ALLOW90: usize = 73;
/// `rail.forbid90` witness.
pub const RAIL_FORBID90: usize = 74;
/// `rail.cache_flushes` witness.
pub const RAIL_CACHE_FLUSHES: usize = 75;
/// `rail.reserve_attempts` witness.
pub const RAIL_RESERVE_ATTEMPTS: usize = 76;
/// `rail.target_busy` witness.
pub const RAIL_TARGET_BUSY: usize = 77;
/// `rail.reserve_success` witness.
pub const RAIL_RESERVE_SUCCESS: usize = 78;
/// `rail.rollback` witness.
pub const RAIL_ROLLBACK: usize = 79;
/// `rail.invalidations` witness.
pub const RAIL_INVALIDATIONS: usize = 80;
/// `rail.reverse_chosen` witness.
pub const RAIL_REVERSE_CHOSEN: usize = 81;
/// `rail.depot_found` witness.
pub const RAIL_DEPOT_FOUND: usize = 82;
/// `rail.safe_found` witness.
pub const RAIL_SAFE_FOUND: usize = 83;
/// `rail.limit_stops` witness.
pub const RAIL_LIMIT_STOPS: usize = 84;
/// `rail.signals_red` witness.
pub const RAIL_SIGNALS_RED: usize = 85;
/// `rail.signals_restored` witness.
pub const RAIL_SIGNALS_RESTORED: usize = 86;
/// `water.tracks` witness.
pub const WATER_TRACKS: usize = 87;
/// `water.follows` witness.
pub const WATER_FOLLOWS: usize = 88;
/// `water.aqueducts` witness.
pub const WATER_AQUEDUCTS: usize = 89;
/// `water.rebuilds` witness.
pub const WATER_REBUILDS: usize = 90;

/// Road tick entries.
pub const ROAD_TICKS: usize = 91;
/// Road calls into typed C++ services.
pub const ROAD_SERVICES: usize = 92;
/// Native traversal calls back into the road Rust visitor.
pub const ROAD_VISITORS: usize = 93;

/// Selected road controller entries, including reentrant helpers.
pub const ROAD_ENTRIES: usize = 94;

/// `station.rejected_delivery` branch.
pub const STATION_REJECTED_DELIVERY: usize = 95;
/// `station.stale_remove` branch.
pub const STATION_STALE_REMOVE: usize = 96;
/// `station.stale_refresh` branch.
pub const STATION_STALE_REFRESH: usize = 97;
/// `station.unload_cargo` branch.
pub const STATION_UNLOAD_CARGO: usize = 98;
/// `station.gradual_unload` branch.
pub const STATION_GRADUAL_UNLOAD: usize = 99;

/// `station.full_load_reserve` branch.
pub const STATION_FULL_LOAD_RESERVE: usize = 100;
/// `station.stale_restrict` branch.
pub const STATION_STALE_RESTRICT: usize = 101;
/// `station.stale_release` branch.
pub const STATION_STALE_RELEASE: usize = 102;

/// Primary aircraft ticks, including both event passes.
pub const AIRCRAFT_TICKS: usize = 103;
/// Aircraft calls into native services, including shared services.
pub const AIRCRAFT_SERVICES: usize = 104;
/// Native calls into aircraft controllers/helpers (excluding owner allocation).
pub const AIRCRAFT_ENTRIES: usize = 105;
/// Aircraft service station-pool resolutions.
pub const AIRCRAFT_STATIONS: usize = 106;
/// Bytes returned by aircraft observations (including record padding).
pub const AIRCRAFT_BYTES: usize = 107;
/// Cache helper ingress, also included in total entries.
pub const AIRCRAFT_CACHE_ENTRIES: usize = 108;
/// Ownerless flight helper ingress, also included in total entries.
pub const AIRCRAFT_DISASTER_ENTRIES: usize = 109;

/// Engine services witness.
pub const ENGINE_SERVICES: usize = 110;
/// Engine bytes witness.
pub const ENGINE_BYTES: usize = 111;
/// Engine sync witness.
pub const ENGINE_SYNC: usize = 112;
/// Engine retire witness.
pub const ENGINE_RETIRE: usize = 113;
/// Engine `never_expire` witness.
pub const ENGINE_NEVER_EXPIRE: usize = 114;
/// Engine accept witness.
pub const ENGINE_ACCEPT: usize = 115;
/// Engine join witness.
pub const ENGINE_JOIN: usize = 116;
/// Engine offer witness.
pub const ENGINE_OFFER: usize = 117;
/// Engine penalty witness.
pub const ENGINE_PENALTY: usize = 118;
/// Engine cutoff witness.
pub const ENGINE_CUTOFF: usize = 119;
/// Engine visibility witness.
pub const ENGINE_VISIBILITY: usize = 120;
/// Engine control witness.
pub const ENGINE_CONTROL: usize = 121;
/// Engine rename witness.
pub const ENGINE_RENAME: usize = 122;

/// Engine native entry witness.
pub const ENGINE_ENTRIES: usize = 123;

/// Signal `updates` witness.
pub const SIGNAL_UPDATES: usize = 124;
/// Signal `tiles` witness.
pub const SIGNAL_TILES: usize = 125;
/// Signal `services` witness.
pub const SIGNAL_SERVICES: usize = 126;
/// Signal `tbu_overflow` witness.
pub const SIGNAL_TBU_OVERFLOW: usize = 127;
/// Signal `tbd_overflow` witness.
pub const SIGNAL_TBD_OVERFLOW: usize = 128;
/// Signal `global_overflow` witness.
pub const SIGNAL_GLOBAL_OVERFLOW: usize = 129;
/// Signal `train` witness.
pub const SIGNAL_TRAIN: usize = 130;
/// Signal `owner_barrier` witness.
pub const SIGNAL_OWNER_BARRIER: usize = 131;
/// Signal `depot` witness.
pub const SIGNAL_DEPOT: usize = 132;
/// Signal `split` witness.
pub const SIGNAL_SPLIT: usize = 133;
/// Signal `pbs` witness.
pub const SIGNAL_PBS: usize = 134;
/// Signal `multi_enter` witness.
pub const SIGNAL_MULTI_ENTER: usize = 135;
/// Signal `exit` witness.
pub const SIGNAL_EXIT: usize = 136;
/// Signal `green_exit` witness.
pub const SIGNAL_GREEN_EXIT: usize = 137;
/// Signal `station` witness.
pub const SIGNAL_STATION: usize = 138;
/// Signal `crossing` witness.
pub const SIGNAL_CROSSING: usize = 139;
/// Signal `wormhole` witness.
pub const SIGNAL_WORMHOLE: usize = 140;
/// Signal `reserved_skip` witness.
pub const SIGNAL_RESERVED_SKIP: usize = 141;
/// Signal `combo` witness.
pub const SIGNAL_COMBO: usize = 142;
/// Signal `propagate` witness.
pub const SIGNAL_PROPAGATE: usize = 143;
/// Signal `changed` witness.
pub const SIGNAL_CHANGED: usize = 144;
/// Signal `reset` witness.
pub const SIGNAL_RESET: usize = 145;
/// Signal `forced_flush` witness.
pub const SIGNAL_FORCED_FLUSH: usize = 146;

/// Bytes returned by signal-facing typed native services.
pub const SIGNAL_BYTES: usize = 147;

/// Signal boundary `entries` counter.
pub const SIGNAL_ENTRIES: usize = 148;
/// Signal boundary `context_acquisitions` counter.
pub const SIGNAL_CONTEXT_ACQUISITIONS: usize = 149;
/// Signal boundary `context_owner_reads` counter.
pub const SIGNAL_CONTEXT_OWNER_READS: usize = 150;
/// Signal boundary `occupancy_accessors` counter.
pub const SIGNAL_OCCUPANCY_ACCESSORS: usize = 151;
/// Signal boundary `context_bytes` counter.
pub const SIGNAL_CONTEXT_BYTES: usize = 152;
/// Signal boundary `classification_bytes` counter.
pub const SIGNAL_CLASSIFICATION_BYTES: usize = 153;

/// Signal multiple exit detection.
pub const SIGNAL_MULTI_EXIT: usize = 154;
/// Signal multiple green exit detection.
pub const SIGNAL_MULTI_GREEN: usize = 155;
/// Signal combo red rule with multiple exits.
pub const SIGNAL_COMBO_RED: usize = 156;
/// Signal empty neighbour buffer continue.
pub const SIGNAL_NEIGHBOUR_CONTINUE: usize = 157;

/// `town.services` witness.
pub const TOWN_SERVICES: usize = 158;
/// `town.bytes` witness.
pub const TOWN_BYTES: usize = 159;
/// `town.placement` witness.
pub const TOWN_PLACEMENT: usize = 160;
/// `town.growth` witness.
pub const TOWN_GROWTH: usize = 161;
/// `town.generated` witness.
pub const TOWN_GENERATED: usize = 162;
/// `town.coast` witness.
pub const TOWN_COAST: usize = 163;
/// `town.manual` witness.
pub const TOWN_MANUAL: usize = 164;
/// `town.deity` witness.
pub const TOWN_DEITY: usize = 165;
/// `town.editor` witness.
pub const TOWN_EDITOR: usize = 166;
/// `town.invalid` witness.
pub const TOWN_INVALID: usize = 167;
/// `town.duplicate` witness.
pub const TOWN_DUPLICATE: usize = 168;
/// `town.money` witness.
pub const TOWN_MONEY: usize = 169;
/// `town.zero_growth` witness.
pub const TOWN_ZERO_GROWTH: usize = 170;
/// `town.retry` witness.
pub const TOWN_RETRY: usize = 171;
/// `town.rename` witness.
pub const TOWN_RENAME: usize = 172;
/// `town.name_retry` witness.
pub const TOWN_NAME_RETRY: usize = 173;
/// `town.name_exhausted` witness.
pub const TOWN_NAME_EXHAUSTED: usize = 174;
/// `town.name_duplicate` witness.
pub const TOWN_NAME_DUPLICATE: usize = 175;
/// `town.fallback` witness.
pub const TOWN_FALLBACK: usize = 176;
/// `town.cancelled` witness.
pub const TOWN_CANCELLED: usize = 177;
/// `town.grid` witness.
pub const TOWN_GRID: usize = 178;
/// `town.name_set` witness.
pub const TOWN_NAME_SET: usize = 179;
/// `town.entries` witness.
pub const TOWN_ENTRIES: usize = 180;
/// `town.no_cities` witness: generation with `larger_towns == 0`.
pub const TOWN_NO_CITIES: usize = 181;

/// `rail_cmd.entries` witness.
pub const RAIL_CMD_ENTRIES: usize = 182;
/// `rail_cmd.build_rail` witness.
pub const RAIL_CMD_BUILD_RAIL: usize = 183;
/// `rail_cmd.build_not_plain` witness.
pub const RAIL_CMD_BUILD_NOT_PLAIN: usize = 184;
/// `rail_cmd.build_remove_signals` witness.
pub const RAIL_CMD_BUILD_REMOVE_SIGNALS: usize = 185;
/// `rail_cmd.build_convert` witness.
pub const RAIL_CMD_BUILD_CONVERT: usize = 186;
/// `rail_cmd.build_crossing` witness.
pub const RAIL_CMD_BUILD_CROSSING: usize = 187;
/// `rail_cmd.build_clear` witness.
pub const RAIL_CMD_BUILD_CLEAR: usize = 188;
/// `rail_cmd.build_half_water` witness.
pub const RAIL_CMD_BUILD_HALF_WATER: usize = 189;
/// `rail_cmd.remove_rail` witness.
pub const RAIL_CMD_REMOVE_RAIL: usize = 190;
/// `rail_cmd.remove_crossing` witness.
pub const RAIL_CMD_REMOVE_CROSSING: usize = 191;
/// `rail_cmd.remove_shore` witness.
pub const RAIL_CMD_REMOVE_SHORE: usize = 192;
/// `rail_cmd.remove_signal_charge` witness.
pub const RAIL_CMD_REMOVE_SIGNAL_CHARGE: usize = 193;
/// `rail_cmd.remove_diagonal_crossing` witness.
pub const RAIL_CMD_REMOVE_DIAGONAL_CROSSING: usize = 194;
/// `rail_cmd.drag_build` witness.
pub const RAIL_CMD_DRAG_BUILD: usize = 195;
/// `rail_cmd.drag_remove` witness.
pub const RAIL_CMD_DRAG_REMOVE: usize = 196;
/// `rail_cmd.drag_obstacle` witness.
pub const RAIL_CMD_DRAG_OBSTACLE: usize = 197;
/// `rail_cmd.depot_build` witness.
pub const RAIL_CMD_DEPOT_BUILD: usize = 198;
/// `rail_cmd.depot_rotate` witness.
pub const RAIL_CMD_DEPOT_ROTATE: usize = 199;
/// `rail_cmd.signal_build` witness.
pub const RAIL_CMD_SIGNAL_BUILD: usize = 200;
/// `rail_cmd.signal_convert` witness.
pub const RAIL_CMD_SIGNAL_CONVERT: usize = 201;
/// `rail_cmd.signal_variant` witness.
pub const RAIL_CMD_SIGNAL_VARIANT: usize = 202;
/// `rail_cmd.signal_cycle_type` witness.
pub const RAIL_CMD_SIGNAL_CYCLE_TYPE: usize = 203;
/// `rail_cmd.signal_cycle_side` witness.
pub const RAIL_CMD_SIGNAL_CYCLE_SIDE: usize = 204;
/// `rail_cmd.signal_copy` witness.
pub const RAIL_CMD_SIGNAL_COPY: usize = 205;
/// `rail_cmd.signal_pbs_state` witness.
pub const RAIL_CMD_SIGNAL_PBS_STATE: usize = 206;
/// `rail_cmd.signal_reserve` witness.
pub const RAIL_CMD_SIGNAL_RESERVE: usize = 207;
/// `rail_cmd.signal_drag` witness.
pub const RAIL_CMD_SIGNAL_DRAG: usize = 208;
/// `rail_cmd.signal_drag_remove` witness.
pub const RAIL_CMD_SIGNAL_DRAG_REMOVE: usize = 209;
/// `rail_cmd.signal_minimise` witness.
pub const RAIL_CMD_SIGNAL_MINIMISE: usize = 210;
/// `rail_cmd.signal_autofill` witness.
pub const RAIL_CMD_SIGNAL_AUTOFILL: usize = 211;
/// `rail_cmd.signal_wormhole` witness.
pub const RAIL_CMD_SIGNAL_WORMHOLE: usize = 212;
/// `rail_cmd.signal_remove` witness.
pub const RAIL_CMD_SIGNAL_REMOVE: usize = 213;
/// `rail_cmd.signal_remove_path_end` witness.
pub const RAIL_CMD_SIGNAL_REMOVE_PATH_END: usize = 214;
/// `rail_cmd.convert_rail` witness.
pub const RAIL_CMD_CONVERT_RAIL: usize = 215;
/// `rail_cmd.convert_depot` witness.
pub const RAIL_CMD_CONVERT_DEPOT: usize = 216;
/// `rail_cmd.convert_wormhole` witness.
pub const RAIL_CMD_CONVERT_WORMHOLE: usize = 217;
/// `rail_cmd.convert_station_crossing` witness.
pub const RAIL_CMD_CONVERT_STATION_CROSSING: usize = 218;
/// `rail_cmd.convert_reroute` witness.
pub const RAIL_CMD_CONVERT_REROUTE: usize = 219;
/// `rail_cmd.clear_rail` witness.
pub const RAIL_CMD_CLEAR_RAIL: usize = 220;
/// `rail_cmd.clear_water` witness.
pub const RAIL_CMD_CLEAR_WATER: usize = 221;
/// `rail_cmd.clear_depot` witness.
pub const RAIL_CMD_CLEAR_DEPOT: usize = 222;
/// `rail_cmd.owner_transfer` witness.
pub const RAIL_CMD_OWNER_TRANSFER: usize = 223;
/// `rail_cmd.owner_clear` witness.
pub const RAIL_CMD_OWNER_CLEAR: usize = 224;
/// `rail_cmd.terraform_corner` witness.
pub const RAIL_CMD_TERRAFORM_CORNER: usize = 225;
/// `rail_cmd.terraform_depot` witness.
pub const RAIL_CMD_TERRAFORM_DEPOT: usize = 226;
/// `rail_cmd.build_above_clear` witness.
pub const RAIL_CMD_BUILD_ABOVE_CLEAR: usize = 227;

/// Counter names, grouped by the prefix the harness selects.
pub const NAMES: [&str; 228] = [
    "train.tick_first",
    "train.tick_second",
    "train.reversal",
    "train.wormhole_swap",
    "train.unequal_before",
    "train.unequal_after",
    "train.depot_start",
    "train.depot_service",
    "train.red_oneway",
    "train.red_twoway",
    "train.force_signal",
    "train.stuck_retry",
    "train.stuck_reverse",
    "train.cross_bar",
    "train.cross_unbar",
    "train.collision",
    "train.crash_delete",
    "train.free_wagon_delete",
    "train.articulated_move",
    "train.first_tile_rail",
    "train.wormhole_exit",
    "train.depot_reentry",
    "train.controller_extension",
    "train.extension_fail",
    "train.extension_rollback",
    "train.extension_opposing_red",
    "train.extension_restore",
    "train.free_path",
    "train.choose_track",
    "train.order_lookahead",
    "train.order_restore",
    "ship.lock_up",
    "ship.lock_down",
    "ship.reverse",
    "ship.rotate",
    "ship.depot_leave",
    "ship.auto_service",
    "ship.buoy",
    "ship.loading",
    "ship.water_change",
    "ship.aqueduct",
    "ship.rotation_reload",
    "ship.depot_search",
    "ship.build",
    "ship.economy_day",
    "ship.path_cache",
    "ship.depot_enter",
    "ship_yapf.region_nodes",
    "ship_yapf.track_nodes",
    "ship_yapf.retries",
    "ship_yapf.track_limits",
    "ship_yapf.intermediate",
    "ship_yapf.cache_truncations",
    "ship_yapf.final_region_clears",
    "ship_yapf.lost",
    "ship_yapf.random_draws",
    "ship_yapf.reverse_origins",
    "ship_yapf.blocked_fallbacks",
    "ship_yapf.region_limits",
    "ship_yapf.alternate_docking",
    "ship_yapf.choose_calls",
    "ship_yapf.reverse_calls",
    "ship_yapf.blocked_calls",
    "ship_yapf.reverse_chosen",
    "rail.choose",
    "rail.reverse",
    "rail.depot",
    "rail.safe",
    "rail.cached",
    "rail.uncached",
    "rail.partial_paths",
    "rail.bounded_depot",
    "rail.found",
    "rail.allow90",
    "rail.forbid90",
    "rail.cache_flushes",
    "rail.reserve_attempts",
    "rail.target_busy",
    "rail.reserve_success",
    "rail.rollback",
    "rail.invalidations",
    "rail.reverse_chosen",
    "rail.depot_found",
    "rail.safe_found",
    "rail.limit_stops",
    "rail.signals_red",
    "rail.signals_restored",
    "water.tracks",
    "water.follows",
    "water.aqueducts",
    "water.rebuilds",
    "road.ticks",
    "road.services",
    "road.visitors",
    "road.entries",
    "station.rejected_delivery",
    "station.stale_remove",
    "station.stale_refresh",
    "station.unload_cargo",
    "station.gradual_unload",
    "station.full_load_reserve",
    "station.stale_restrict",
    "station.stale_release",
    "aircraft.ticks",
    "aircraft.services",
    "aircraft.entries",
    "aircraft.station_resolutions",
    "aircraft.observation_bytes",
    "aircraft.cache_entries",
    "aircraft.disaster_entries",
    "engine.services",
    "engine.bytes",
    "engine.sync",
    "engine.retire",
    "engine.never_expire",
    "engine.accept",
    "engine.join",
    "engine.offer",
    "engine.penalty",
    "engine.cutoff",
    "engine.visibility",
    "engine.control",
    "engine.rename",
    "engine.entries",
    "signal.updates",
    "signal.tiles",
    "signal.services",
    "signal.tbu_overflow",
    "signal.tbd_overflow",
    "signal.global_overflow",
    "signal.train",
    "signal.owner_barrier",
    "signal.depot",
    "signal.split",
    "signal.pbs",
    "signal.multi_enter",
    "signal.exit",
    "signal.green_exit",
    "signal.station",
    "signal.crossing",
    "signal.wormhole",
    "signal.reserved_skip",
    "signal.combo",
    "signal.propagate",
    "signal.changed",
    "signal.reset",
    "signal.forced_flush",
    "signal.bytes",
    "signal.entries",
    "signal.context_acquisitions",
    "signal.context_owner_reads",
    "signal.occupancy_accessors",
    "signal.context_bytes",
    "signal.classification_bytes",
    "signal.multi_exit",
    "signal.multi_green",
    "signal.combo_red",
    "signal.neighbour_continue",
    "town.services",
    "town.bytes",
    "town.placement",
    "town.growth",
    "town.generated",
    "town.coast",
    "town.manual",
    "town.deity",
    "town.editor",
    "town.invalid",
    "town.duplicate",
    "town.money",
    "town.zero_growth",
    "town.retry",
    "town.rename",
    "town.name_retry",
    "town.name_exhausted",
    "town.name_duplicate",
    "town.fallback",
    "town.cancelled",
    "town.grid",
    "town.name_set",
    "town.entries",
    "town.no_cities",
    "rail_cmd.entries",
    "rail_cmd.build_rail",
    "rail_cmd.build_not_plain",
    "rail_cmd.build_remove_signals",
    "rail_cmd.build_convert",
    "rail_cmd.build_crossing",
    "rail_cmd.build_clear",
    "rail_cmd.build_half_water",
    "rail_cmd.remove_rail",
    "rail_cmd.remove_crossing",
    "rail_cmd.remove_shore",
    "rail_cmd.remove_signal_charge",
    "rail_cmd.remove_diagonal_crossing",
    "rail_cmd.drag_build",
    "rail_cmd.drag_remove",
    "rail_cmd.drag_obstacle",
    "rail_cmd.depot_build",
    "rail_cmd.depot_rotate",
    "rail_cmd.signal_build",
    "rail_cmd.signal_convert",
    "rail_cmd.signal_variant",
    "rail_cmd.signal_cycle_type",
    "rail_cmd.signal_cycle_side",
    "rail_cmd.signal_copy",
    "rail_cmd.signal_pbs_state",
    "rail_cmd.signal_reserve",
    "rail_cmd.signal_drag",
    "rail_cmd.signal_drag_remove",
    "rail_cmd.signal_minimise",
    "rail_cmd.signal_autofill",
    "rail_cmd.signal_wormhole",
    "rail_cmd.signal_remove",
    "rail_cmd.signal_remove_path_end",
    "rail_cmd.convert_rail",
    "rail_cmd.convert_depot",
    "rail_cmd.convert_wormhole",
    "rail_cmd.convert_station_crossing",
    "rail_cmd.convert_reroute",
    "rail_cmd.clear_rail",
    "rail_cmd.clear_water",
    "rail_cmd.clear_depot",
    "rail_cmd.owner_transfer",
    "rail_cmd.owner_clear",
    "rail_cmd.terraform_corner",
    "rail_cmd.terraform_depot",
    "rail_cmd.build_above_clear",
];

static ENABLED: AtomicBool = AtomicBool::new(false);
static COUNTS: [AtomicU64; NAMES.len()] = [const { AtomicU64::new(0) }; NAMES.len()];

/// Whether counters are on; lets a site skip work that only feeds counters.
#[inline]
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Add `amount` to counter `index` when witnesses are enabled.
#[inline]
pub fn add(index: usize, amount: u64) {
    if ENABLED.load(Ordering::Relaxed) {
        COUNTS[index].fetch_add(amount, Ordering::Relaxed);
    }
}

/// Count one hit of branch `index`.
#[inline]
pub fn hit(index: usize) {
    add(index, 1);
}

/// Turn every counter on; called once at static initialization.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_witness_enable() {
    ENABLED.store(true, Ordering::Relaxed);
}

/// Name of counter `index` (UTF-8, not NUL-terminated, static lifetime) and its
/// length; null past the last counter.
///
/// # Safety
/// `length` is a writable `usize` for this call only.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_witness_name(index: usize, length: *mut usize) -> *const u8 {
    NAMES.get(index).map_or(std::ptr::null(), |name| {
        // SAFETY: Caller supplies a writable output for this call.
        unsafe { length.write(name.len()) };
        name.as_ptr()
    })
}

/// Current count of counter `index`, which must be below the name count.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_witness_count(index: usize) -> u64 {
    COUNTS[index].load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn groups_start_at_their_prefix() {
        fn group(name: &str) -> &str {
            name.split_once('.').map_or(name, |(group, _)| group)
        }
        for (base, prefix) in [
            (TRAIN, "train."),
            (SHIP, "ship."),
            (SHIP_YAPF, "ship_yapf."),
            (RAIL, "rail."),
            (WATER, "water."),
        ] {
            assert!(NAMES[base].starts_with(prefix));
            assert!(base == 0 || !NAMES[base - 1].starts_with(prefix));
        }
        for (index, name) in NAMES.iter().enumerate().skip(1) {
            if group(name) != group(NAMES[index - 1]) {
                assert!(!NAMES[..index].iter().any(|n| group(n) == group(name)));
            }
        }
        let mut length = 0;
        assert!(unsafe { openttd_rust_witness_name(NAMES.len(), &raw mut length) }.is_null());
    }
}
