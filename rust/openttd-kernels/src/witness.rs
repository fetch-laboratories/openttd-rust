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

// Component blocks are alphabetical. Local indices and names live together;
// adding a block never requires editing another component or an array length.
macro_rules! witness_blocks {
    ($($component:ident { $($counter:ident = $local:literal => $name:literal,)+ })+) => {
        const BLOCKS: &[&[&str]] = &[$(&[$($name),+]),+];
        #[derive(Clone, Copy)]
        #[allow(non_camel_case_types)]
        enum Block { $($component),+ }
        $( $(
            #[doc = concat!("`", $name, "`")]
            pub const $counter: usize = block_base(Block::$component) + $local;
        )+ )+
        /// Counter names in the same order as the component blocks.
        pub const NAMES: &[&str] = &[$($($name),+),+];
        const _: () = { $($(assert!($counter < NAMES.len());)+)+ };
        #[cfg(test)]
        const INDICES: &[usize] = &[$($($counter),+),+];
    };
}

const fn block_base(block: Block) -> usize {
    let mut base = 0;
    let mut index = 0;
    while index < block as usize {
        base += BLOCKS[index].len();
        index += 1;
    }
    base
}

witness_blocks! {
    aircraft {
        AIRCRAFT_TICKS = 0 => "aircraft.ticks",
        AIRCRAFT_SERVICES = 1 => "aircraft.services",
        AIRCRAFT_ENTRIES = 2 => "aircraft.entries",
        AIRCRAFT_STATIONS = 3 => "aircraft.station_resolutions",
        AIRCRAFT_BYTES = 4 => "aircraft.observation_bytes",
        AIRCRAFT_CACHE_ENTRIES = 5 => "aircraft.cache_entries",
        AIRCRAFT_DISASTER_ENTRIES = 6 => "aircraft.disaster_entries",
    }
    engine {
        ENGINE_SERVICES = 0 => "engine.services",
        ENGINE_BYTES = 1 => "engine.bytes",
        ENGINE_SYNC = 2 => "engine.sync",
        ENGINE_RETIRE = 3 => "engine.retire",
        ENGINE_NEVER_EXPIRE = 4 => "engine.never_expire",
        ENGINE_ACCEPT = 5 => "engine.accept",
        ENGINE_JOIN = 6 => "engine.join",
        ENGINE_OFFER = 7 => "engine.offer",
        ENGINE_PENALTY = 8 => "engine.penalty",
        ENGINE_CUTOFF = 9 => "engine.cutoff",
        ENGINE_VISIBILITY = 10 => "engine.visibility",
        ENGINE_CONTROL = 11 => "engine.control",
        ENGINE_RENAME = 12 => "engine.rename",
        ENGINE_ENTRIES = 13 => "engine.entries",
    }
    // Registry: house (#256).
    house {
        HOUSE = 0 => "house.stage",
        HOUSE_COMPLETE = 1 => "house.complete",
        HOUSE_LIFT_START = 2 => "house.lift_start",
        HOUSE_LIFT_STOP = 3 => "house.lift_stop",
        HOUSE_ORIGINAL = 4 => "house.original",
        HOUSE_BINOMIAL = 5 => "house.binomial",
        HOUSE_RECESSION = 6 => "house.recession",
        HOUSE_REBUILD = 7 => "house.rebuild",
        HOUSE_REPLACE = 8 => "house.replace",
        HOUSE_CLEAR_AUTO = 9 => "house.clear_auto",
        HOUSE_CLEAR_REFUSED = 10 => "house.clear_refused",
        HOUSE_CLEAR_EXECUTE = 11 => "house.clear_execute",
        HOUSE_AUTOSLOPE = 12 => "house.autoslope",
        HOUSE_PLACE = 13 => "house.place",
        HOUSE_PLACE_ERROR = 14 => "house.place_error",
        HOUSE_STATION_REMOVED = 15 => "house.station_removed",
        HOUSE_PROTECTED = 16 => "house.protected",
    }

    // Registry: landscape (#212).
    landscape {
        LANDSCAPE = 0 => "landscape.snow_start",
        LANDSCAPE_SNOW_CHANGE = 1 => "landscape.snow_change",
        LANDSCAPE_SNOW_CLEAR = 2 => "landscape.snow_clear",
        LANDSCAPE_DESERT = 3 => "landscape.desert",
        LANDSCAPE_FIELD_FENCE = 4 => "landscape.field_fence",
        LANDSCAPE_FIELD_CYCLE = 5 => "landscape.field_cycle",
        LANDSCAPE_FIELD_ORPHAN = 6 => "landscape.field_orphan",
        LANDSCAPE_RAIL_GROUND = 7 => "landscape.rail_ground",
        LANDSCAPE_ROADSIDE = 8 => "landscape.roadside",
        LANDSCAPE_ROAD_WORK_START = 9 => "landscape.road_work_start",
        LANDSCAPE_ROAD_WORK_END = 10 => "landscape.road_work_end",
        LANDSCAPE_ROAD_REMOVE = 11 => "landscape.road_remove",
        LANDSCAPE_TUNNEL_GROUND = 12 => "landscape.tunnel_ground",
        LANDSCAPE_FLOOD_SUCCESS = 13 => "landscape.flood_success",
        LANDSCAPE_FLOOD_FAIL = 14 => "landscape.flood_fail",
        LANDSCAPE_DRY_COAST = 15 => "landscape.dry_coast",
        LANDSCAPE_DRY_RAIL = 16 => "landscape.dry_rail",
        LANDSCAPE_DRY_TREE = 17 => "landscape.dry_tree",
        LANDSCAPE_WAKEUP = 18 => "landscape.wakeup",
        LANDSCAPE_HALF_RAIL_REMOVE = 19 => "landscape.half_rail_remove",
        LANDSCAPE_HALF_RAIL = 20 => "landscape.half_rail",
        LANDSCAPE_BRIDGE_START = 21 => "landscape.bridge_start",
        LANDSCAPE_BRIDGE_END = 22 => "landscape.bridge_end",
        LANDSCAPE_AIRPORT_MEMBER = 23 => "landscape.airport_member",
        LANDSCAPE_VEHICLE_CRASH = 24 => "landscape.vehicle_crash",
        LANDSCAPE_CRASHED_SKIP = 25 => "landscape.crashed_skip",
    }

    rail {
        RAIL_CHOOSE = 0 => "rail.choose",
        RAIL_REVERSE = 1 => "rail.reverse",
        RAIL_DEPOT = 2 => "rail.depot",
        RAIL_SAFE = 3 => "rail.safe",
        RAIL_CACHED = 4 => "rail.cached",
        RAIL_UNCACHED = 5 => "rail.uncached",
        RAIL_PARTIAL_PATHS = 6 => "rail.partial_paths",
        RAIL_BOUNDED_DEPOT = 7 => "rail.bounded_depot",
        RAIL_FOUND = 8 => "rail.found",
        RAIL_ALLOW90 = 9 => "rail.allow90",
        RAIL_FORBID90 = 10 => "rail.forbid90",
        RAIL_CACHE_FLUSHES = 11 => "rail.cache_flushes",
        RAIL_RESERVE_ATTEMPTS = 12 => "rail.reserve_attempts",
        RAIL_TARGET_BUSY = 13 => "rail.target_busy",
        RAIL_RESERVE_SUCCESS = 14 => "rail.reserve_success",
        RAIL_ROLLBACK = 15 => "rail.rollback",
        RAIL_INVALIDATIONS = 16 => "rail.invalidations",
        RAIL_REVERSE_CHOSEN = 17 => "rail.reverse_chosen",
        RAIL_DEPOT_FOUND = 18 => "rail.depot_found",
        RAIL_SAFE_FOUND = 19 => "rail.safe_found",
        RAIL_LIMIT_STOPS = 20 => "rail.limit_stops",
        RAIL_SIGNALS_RED = 21 => "rail.signals_red",
        RAIL_SIGNALS_RESTORED = 22 => "rail.signals_restored",
    }
    rail_cmd {
        RAIL_CMD_ENTRIES = 0 => "rail_cmd.entries",
        RAIL_CMD_BUILD_RAIL = 1 => "rail_cmd.build_rail",
        RAIL_CMD_BUILD_NOT_PLAIN = 2 => "rail_cmd.build_not_plain",
        RAIL_CMD_BUILD_REMOVE_SIGNALS = 3 => "rail_cmd.build_remove_signals",
        RAIL_CMD_BUILD_CONVERT = 4 => "rail_cmd.build_convert",
        RAIL_CMD_BUILD_CROSSING = 5 => "rail_cmd.build_crossing",
        RAIL_CMD_BUILD_CLEAR = 6 => "rail_cmd.build_clear",
        RAIL_CMD_BUILD_HALF_WATER = 7 => "rail_cmd.build_half_water",
        RAIL_CMD_REMOVE_RAIL = 8 => "rail_cmd.remove_rail",
        RAIL_CMD_REMOVE_CROSSING = 9 => "rail_cmd.remove_crossing",
        RAIL_CMD_REMOVE_SHORE = 10 => "rail_cmd.remove_shore",
        RAIL_CMD_REMOVE_SIGNAL_CHARGE = 11 => "rail_cmd.remove_signal_charge",
        RAIL_CMD_REMOVE_DIAGONAL_CROSSING = 12 => "rail_cmd.remove_diagonal_crossing",
        RAIL_CMD_DRAG_BUILD = 13 => "rail_cmd.drag_build",
        RAIL_CMD_DRAG_REMOVE = 14 => "rail_cmd.drag_remove",
        RAIL_CMD_DRAG_OBSTACLE = 15 => "rail_cmd.drag_obstacle",
        RAIL_CMD_DEPOT_BUILD = 16 => "rail_cmd.depot_build",
        RAIL_CMD_DEPOT_ROTATE = 17 => "rail_cmd.depot_rotate",
        RAIL_CMD_SIGNAL_BUILD = 18 => "rail_cmd.signal_build",
        RAIL_CMD_SIGNAL_CONVERT = 19 => "rail_cmd.signal_convert",
        RAIL_CMD_SIGNAL_VARIANT = 20 => "rail_cmd.signal_variant",
        RAIL_CMD_SIGNAL_CYCLE_TYPE = 21 => "rail_cmd.signal_cycle_type",
        RAIL_CMD_SIGNAL_CYCLE_SIDE = 22 => "rail_cmd.signal_cycle_side",
        RAIL_CMD_SIGNAL_COPY = 23 => "rail_cmd.signal_copy",
        RAIL_CMD_SIGNAL_PBS_STATE = 24 => "rail_cmd.signal_pbs_state",
        RAIL_CMD_SIGNAL_RESERVE = 25 => "rail_cmd.signal_reserve",
        RAIL_CMD_SIGNAL_DRAG = 26 => "rail_cmd.signal_drag",
        RAIL_CMD_SIGNAL_DRAG_REMOVE = 27 => "rail_cmd.signal_drag_remove",
        RAIL_CMD_SIGNAL_MINIMISE = 28 => "rail_cmd.signal_minimise",
        RAIL_CMD_SIGNAL_AUTOFILL = 29 => "rail_cmd.signal_autofill",
        RAIL_CMD_SIGNAL_WORMHOLE = 30 => "rail_cmd.signal_wormhole",
        RAIL_CMD_SIGNAL_REMOVE = 31 => "rail_cmd.signal_remove",
        RAIL_CMD_SIGNAL_REMOVE_PATH_END = 32 => "rail_cmd.signal_remove_path_end",
        RAIL_CMD_CONVERT_RAIL = 33 => "rail_cmd.convert_rail",
        RAIL_CMD_CONVERT_DEPOT = 34 => "rail_cmd.convert_depot",
        RAIL_CMD_CONVERT_WORMHOLE = 35 => "rail_cmd.convert_wormhole",
        RAIL_CMD_CONVERT_STATION_CROSSING = 36 => "rail_cmd.convert_station_crossing",
        RAIL_CMD_CONVERT_REROUTE = 37 => "rail_cmd.convert_reroute",
        RAIL_CMD_CLEAR_RAIL = 38 => "rail_cmd.clear_rail",
        RAIL_CMD_CLEAR_WATER = 39 => "rail_cmd.clear_water",
        RAIL_CMD_CLEAR_DEPOT = 40 => "rail_cmd.clear_depot",
        RAIL_CMD_OWNER_TRANSFER = 41 => "rail_cmd.owner_transfer",
        RAIL_CMD_OWNER_CLEAR = 42 => "rail_cmd.owner_clear",
        RAIL_CMD_TERRAFORM_CORNER = 43 => "rail_cmd.terraform_corner",
        RAIL_CMD_TERRAFORM_DEPOT = 44 => "rail_cmd.terraform_depot",
        RAIL_CMD_BUILD_ABOVE_CLEAR = 45 => "rail_cmd.build_above_clear",
    }
    road {
        ROAD_TICKS = 0 => "road.ticks",
        ROAD_SERVICES = 1 => "road.services",
        ROAD_VISITORS = 2 => "road.visitors",
        ROAD_ENTRIES = 3 => "road.entries",
    }
    road_commands {
        ROAD_COMMANDS_ENTRIES = 0 => "road_commands.entries",
        ROAD_COMMANDS_TOWN_REMOVE = 1 => "road_commands.town_remove",
        ROAD_COMMANDS_REMOVE = 2 => "road_commands.remove",
        ROAD_COMMANDS_REMOVE_WORMHOLE = 3 => "road_commands.remove_wormhole",
        ROAD_COMMANDS_REMOVE_STOP = 4 => "road_commands.remove_stop",
        ROAD_COMMANDS_REMOVE_CROSSING = 5 => "road_commands.remove_crossing",
        ROAD_COMMANDS_SLOPE = 6 => "road_commands.slope",
        ROAD_COMMANDS_BUILD = 7 => "road_commands.build",
        ROAD_COMMANDS_ONEWAY = 8 => "road_commands.oneway",
        ROAD_COMMANDS_BUILD_CROSSING = 9 => "road_commands.build_crossing",
        ROAD_COMMANDS_BUILD_CLEAR = 10 => "road_commands.build_clear",
        ROAD_COMMANDS_BUILD_WORMHOLE = 11 => "road_commands.build_wormhole",
        ROAD_COMMANDS_BUILD_STOP = 12 => "road_commands.build_stop",
        ROAD_COMMANDS_BUILD_LONG = 13 => "road_commands.build_long",
        ROAD_COMMANDS_DRAG_OBSTACLE = 14 => "road_commands.drag_obstacle",
        ROAD_COMMANDS_REMOVE_LONG = 15 => "road_commands.remove_long",
        ROAD_COMMANDS_REMOVE_MONEY = 16 => "road_commands.remove_money",
        ROAD_COMMANDS_DEPOT_BUILD = 17 => "road_commands.depot_build",
        ROAD_COMMANDS_DEPOT_ROTATE = 18 => "road_commands.depot_rotate",
        ROAD_COMMANDS_DEPOT_REMOVE = 19 => "road_commands.depot_remove",
        ROAD_COMMANDS_CLEAR = 20 => "road_commands.clear",
        ROAD_COMMANDS_OWNER = 21 => "road_commands.owner",
        ROAD_COMMANDS_TERRAFORM = 22 => "road_commands.terraform",
        ROAD_COMMANDS_AUTOSLOPE = 23 => "road_commands.autoslope",
        ROAD_COMMANDS_CONVERT = 24 => "road_commands.convert",
        ROAD_COMMANDS_CONVERT_TILE = 25 => "road_commands.convert_tile",
        ROAD_COMMANDS_CONVERT_WORMHOLE = 26 => "road_commands.convert_wormhole",
        ROAD_COMMANDS_BUILD_ABOVE = 27 => "road_commands.build_above",
        ROAD_COMMANDS_FLOOD_WORKS = 28 => "road_commands.flood_works",
        ROAD_COMMANDS_NON_ROAD = 29 => "road_commands.non_road",
    }
    ship {
        SHIP_LOCK_UP = 0 => "ship.lock_up",
        SHIP_LOCK_DOWN = 1 => "ship.lock_down",
        SHIP_REVERSE = 2 => "ship.reverse",
        SHIP_ROTATE = 3 => "ship.rotate",
        SHIP_DEPOT_LEAVE = 4 => "ship.depot_leave",
        SHIP_AUTO_SERVICE = 5 => "ship.auto_service",
        SHIP_BUOY = 6 => "ship.buoy",
        SHIP_LOADING = 7 => "ship.loading",
        SHIP_WATER_CHANGE = 8 => "ship.water_change",
        SHIP_AQUEDUCT = 9 => "ship.aqueduct",
        SHIP_ROTATION_RELOAD = 10 => "ship.rotation_reload",
        SHIP_DEPOT_SEARCH = 11 => "ship.depot_search",
        SHIP_BUILD = 12 => "ship.build",
        SHIP_ECONOMY_DAY = 13 => "ship.economy_day",
        SHIP_PATH_CACHE = 14 => "ship.path_cache",
        SHIP_DEPOT_ENTER = 15 => "ship.depot_enter",
    }
    ship_yapf {
        SHIP_YAPF_REGION_NODES = 0 => "ship_yapf.region_nodes",
        SHIP_YAPF_TRACK_NODES = 1 => "ship_yapf.track_nodes",
        SHIP_YAPF_RETRIES = 2 => "ship_yapf.retries",
        SHIP_YAPF_TRACK_LIMITS = 3 => "ship_yapf.track_limits",
        SHIP_YAPF_INTERMEDIATE = 4 => "ship_yapf.intermediate",
        SHIP_YAPF_CACHE_TRUNCATIONS = 5 => "ship_yapf.cache_truncations",
        SHIP_YAPF_FINAL_REGION_CLEARS = 6 => "ship_yapf.final_region_clears",
        SHIP_YAPF_LOST = 7 => "ship_yapf.lost",
        SHIP_YAPF_RANDOM_DRAWS = 8 => "ship_yapf.random_draws",
        SHIP_YAPF_REVERSE_ORIGINS = 9 => "ship_yapf.reverse_origins",
        SHIP_YAPF_BLOCKED_FALLBACKS = 10 => "ship_yapf.blocked_fallbacks",
        SHIP_YAPF_REGION_LIMITS = 11 => "ship_yapf.region_limits",
        SHIP_YAPF_ALTERNATE_DOCKING = 12 => "ship_yapf.alternate_docking",
        SHIP_YAPF_CHOOSE_CALLS = 13 => "ship_yapf.choose_calls",
        SHIP_YAPF_REVERSE_CALLS = 14 => "ship_yapf.reverse_calls",
        SHIP_YAPF_BLOCKED_CALLS = 15 => "ship_yapf.blocked_calls",
        SHIP_YAPF_REVERSE_CHOSEN = 16 => "ship_yapf.reverse_chosen",
    }
    signal {
        SIGNAL_UPDATES = 0 => "signal.updates",
        SIGNAL_TILES = 1 => "signal.tiles",
        SIGNAL_SERVICES = 2 => "signal.services",
        SIGNAL_TBU_OVERFLOW = 3 => "signal.tbu_overflow",
        SIGNAL_TBD_OVERFLOW = 4 => "signal.tbd_overflow",
        SIGNAL_GLOBAL_OVERFLOW = 5 => "signal.global_overflow",
        SIGNAL_TRAIN = 6 => "signal.train",
        SIGNAL_OWNER_BARRIER = 7 => "signal.owner_barrier",
        SIGNAL_DEPOT = 8 => "signal.depot",
        SIGNAL_SPLIT = 9 => "signal.split",
        SIGNAL_PBS = 10 => "signal.pbs",
        SIGNAL_MULTI_ENTER = 11 => "signal.multi_enter",
        SIGNAL_EXIT = 12 => "signal.exit",
        SIGNAL_GREEN_EXIT = 13 => "signal.green_exit",
        SIGNAL_STATION = 14 => "signal.station",
        SIGNAL_CROSSING = 15 => "signal.crossing",
        SIGNAL_WORMHOLE = 16 => "signal.wormhole",
        SIGNAL_RESERVED_SKIP = 17 => "signal.reserved_skip",
        SIGNAL_COMBO = 18 => "signal.combo",
        SIGNAL_PROPAGATE = 19 => "signal.propagate",
        SIGNAL_CHANGED = 20 => "signal.changed",
        SIGNAL_RESET = 21 => "signal.reset",
        SIGNAL_FORCED_FLUSH = 22 => "signal.forced_flush",
        SIGNAL_BYTES = 23 => "signal.bytes",
        SIGNAL_ENTRIES = 24 => "signal.entries",
        SIGNAL_CONTEXT_ACQUISITIONS = 25 => "signal.context_acquisitions",
        SIGNAL_CONTEXT_OWNER_READS = 26 => "signal.context_owner_reads",
        SIGNAL_OCCUPANCY_ACCESSORS = 27 => "signal.occupancy_accessors",
        SIGNAL_CONTEXT_BYTES = 28 => "signal.context_bytes",
        SIGNAL_CLASSIFICATION_BYTES = 29 => "signal.classification_bytes",
        SIGNAL_MULTI_EXIT = 30 => "signal.multi_exit",
        SIGNAL_MULTI_GREEN = 31 => "signal.multi_green",
        SIGNAL_COMBO_RED = 32 => "signal.combo_red",
        SIGNAL_NEIGHBOUR_CONTINUE = 33 => "signal.neighbour_continue",
    }
    station {
        STATION_REJECTED_DELIVERY = 0 => "station.rejected_delivery",
        STATION_STALE_REMOVE = 1 => "station.stale_remove",
        STATION_STALE_REFRESH = 2 => "station.stale_refresh",
        STATION_UNLOAD_CARGO = 3 => "station.unload_cargo",
        STATION_GRADUAL_UNLOAD = 4 => "station.gradual_unload",
        STATION_FULL_LOAD_RESERVE = 5 => "station.full_load_reserve",
        STATION_STALE_RESTRICT = 6 => "station.stale_restrict",
        STATION_STALE_RELEASE = 7 => "station.stale_release",
    }
    station_command {
        STATION_CMD_BUILD_RAIL = 0 => "station_command.build_rail",
        STATION_CMD_BUILD_ROAD = 1 => "station_command.build_road",
        STATION_CMD_BUILD_AIRPORT = 2 => "station_command.build_airport",
        STATION_CMD_REMOVE_AIRPORT = 3 => "station_command.remove_airport",
        STATION_CMD_BUILD_DOCK = 4 => "station_command.build_dock",
        STATION_CMD_REMOVE_DOCK = 5 => "station_command.remove_dock",
        STATION_CMD_BUILD_OILRIG = 6 => "station_command.build_oilrig",
        STATION_CMD_DELETE_OILRIG = 7 => "station_command.delete_oilrig",
        STATION_CMD_CHECK_DOCKING = 8 => "station_command.check_docking",
        STATION_CMD_RENAME = 9 => "station_command.rename",
        STATION_CMD_MOVE_NAME = 10 => "station_command.move_name",
        STATION_CMD_CHANGE_OWNER = 11 => "station_command.change_owner",
        STATION_CMD_CLEAR = 12 => "station_command.clear",
        STATION_CMD_TERRAFORM = 13 => "station_command.terraform",
        STATION_CMD_REMOVE_ROAD_TILE = 14 => "station_command.remove_road_tile",
        STATION_CMD_REMOVE_RAIL_AREA = 15 => "station_command.remove_rail_area",
        STATION_CMD_JOIN = 16 => "station_command.join",
        STATION_CMD_REUSE = 17 => "station_command.reuse",
        STATION_CMD_PARTIAL_RAIL = 18 => "station_command.partial_rail",
        STATION_CMD_BUILD_ABOVE = 19 => "station_command.build_above",
    }
    subsidy {
        SUBSIDY_CREATE = 0 => "subsidy.create",
        SUBSIDY_AWARD = 1 => "subsidy.award",
        SUBSIDY_EXPIRE = 2 => "subsidy.expire",
    }
    town {
        TOWN_SERVICES = 0 => "town.services",
        TOWN_BYTES = 1 => "town.bytes",
        TOWN_PLACEMENT = 2 => "town.placement",
        TOWN_GROWTH = 3 => "town.growth",
        TOWN_GENERATED = 4 => "town.generated",
        TOWN_COAST = 5 => "town.coast",
        TOWN_MANUAL = 6 => "town.manual",
        TOWN_DEITY = 7 => "town.deity",
        TOWN_EDITOR = 8 => "town.editor",
        TOWN_INVALID = 9 => "town.invalid",
        TOWN_DUPLICATE = 10 => "town.duplicate",
        TOWN_MONEY = 11 => "town.money",
        TOWN_ZERO_GROWTH = 12 => "town.zero_growth",
        TOWN_RETRY = 13 => "town.retry",
        TOWN_RENAME = 14 => "town.rename",
        TOWN_NAME_RETRY = 15 => "town.name_retry",
        TOWN_NAME_EXHAUSTED = 16 => "town.name_exhausted",
        TOWN_NAME_DUPLICATE = 17 => "town.name_duplicate",
        TOWN_FALLBACK = 18 => "town.fallback",
        TOWN_CANCELLED = 19 => "town.cancelled",
        TOWN_GRID = 20 => "town.grid",
        TOWN_NAME_SET = 21 => "town.name_set",
        TOWN_ENTRIES = 22 => "town.entries",
        TOWN_NO_CITIES = 23 => "town.no_cities",
    }
    train {
        TRAIN_TICK_FIRST = 0 => "train.tick_first",
        TRAIN_TICK_SECOND = 1 => "train.tick_second",
        TRAIN_REVERSAL = 2 => "train.reversal",
        TRAIN_WORMHOLE_SWAP = 3 => "train.wormhole_swap",
        TRAIN_UNEQUAL_BEFORE = 4 => "train.unequal_before",
        TRAIN_UNEQUAL_AFTER = 5 => "train.unequal_after",
        TRAIN_DEPOT_START = 6 => "train.depot_start",
        TRAIN_DEPOT_SERVICE = 7 => "train.depot_service",
        TRAIN_RED_ONEWAY = 8 => "train.red_oneway",
        TRAIN_RED_TWOWAY = 9 => "train.red_twoway",
        TRAIN_FORCE_SIGNAL = 10 => "train.force_signal",
        TRAIN_STUCK_RETRY = 11 => "train.stuck_retry",
        TRAIN_STUCK_REVERSE = 12 => "train.stuck_reverse",
        TRAIN_CROSS_BAR = 13 => "train.cross_bar",
        TRAIN_CROSS_UNBAR = 14 => "train.cross_unbar",
        TRAIN_COLLISION = 15 => "train.collision",
        TRAIN_CRASH_DELETE = 16 => "train.crash_delete",
        TRAIN_FREE_WAGON_DELETE = 17 => "train.free_wagon_delete",
        TRAIN_ARTICULATED_MOVE = 18 => "train.articulated_move",
        TRAIN_FIRST_TILE_RAIL = 19 => "train.first_tile_rail",
        TRAIN_WORMHOLE_EXIT = 20 => "train.wormhole_exit",
        TRAIN_DEPOT_REENTRY = 21 => "train.depot_reentry",
        TRAIN_CONTROLLER_EXTENSION = 22 => "train.controller_extension",
        TRAIN_EXTENSION_FAIL = 23 => "train.extension_fail",
        TRAIN_EXTENSION_ROLLBACK = 24 => "train.extension_rollback",
        TRAIN_EXTENSION_OPPOSING_RED = 25 => "train.extension_opposing_red",
        TRAIN_EXTENSION_RESTORE = 26 => "train.extension_restore",
        TRAIN_FREE_PATH = 27 => "train.free_path",
        TRAIN_CHOOSE_TRACK = 28 => "train.choose_track",
        TRAIN_ORDER_LOOKAHEAD = 29 => "train.order_lookahead",
        TRAIN_ORDER_RESTORE = 30 => "train.order_restore",
    }
    vehicle {
        VEHICLE_CALLBACK32 = 0 => "vehicle.callback32",
        VEHICLE_CARGO_AGES = 1 => "vehicle.cargo_ages",
        VEHICLE_CARGO_OWNER = 2 => "vehicle.cargo_owner",
        VEHICLE_CROSSINGS = 3 => "vehicle.crossings",
        VEHICLE_DAYS = 4 => "vehicle.days",
        VEHICLE_DELETED = 5 => "vehicle.deleted",
        VEHICLE_DESCRIPTOR_BYTES = 6 => "vehicle.descriptor_bytes",
        VEHICLE_MOTION = 7 => "vehicle.motion",
        VEHICLE_OBSERVED_BYTES = 8 => "vehicle.observed_bytes",
        VEHICLE_OUTER_VISITS = 9 => "vehicle.outer_visits",
        VEHICLE_PACKET_LOOKUPS = 10 => "vehicle.packet_lookups",
        VEHICLE_PERIODIC_SOUND = 11 => "vehicle.periodic_sound",
        VEHICLE_POOL_GROWTH = 12 => "vehicle.pool_growth",
        VEHICLE_POST_TICK = 13 => "vehicle.post_tick",
        VEHICLE_RUNNING_SOUND = 14 => "vehicle.running_sound",
        VEHICLE_SCALAR_READ_BYTES = 15 => "vehicle.scalar_read_bytes",
        VEHICLE_SCALAR_WRITE_BYTES = 16 => "vehicle.scalar_write_bytes",
        VEHICLE_STATIONS = 17 => "vehicle.stations",
        VEHICLE_VISITS = 18 => "vehicle.visits.train",
        VEHICLE_VISITS_ROAD = 19 => "vehicle.visits.road",
        VEHICLE_VISITS_SHIP = 20 => "vehicle.visits.ship",
        VEHICLE_VISITS_AIRCRAFT = 21 => "vehicle.visits.aircraft",
        VEHICLE_VISITS_EFFECT = 22 => "vehicle.visits.effect",
        VEHICLE_VISITS_DISASTER = 23 => "vehicle.visits.disaster",
        VEHICLE_VISITS_END = 24 => "vehicle.visits.end",
        VEHICLE_VISITS_INVALID = 25 => "vehicle.visits.invalid",
    }
    vehicle_maintenance {
        VEHICLE_MAINT_AGE = 0 => "vehicle_maintenance.age",
        VEHICLE_MAINT_AIRCRAFT = 1 => "vehicle_maintenance.aircraft",
        VEHICLE_MAINT_BYTES = 2 => "vehicle_maintenance.bytes",
        VEHICLE_MAINT_CHECK = 3 => "vehicle_maintenance.check",
        VEHICLE_MAINT_COUNTDOWN = 4 => "vehicle_maintenance.countdown",
        VEHICLE_MAINT_CROSSINGS = 5 => "vehicle_maintenance.crossings",
        VEHICLE_MAINT_DAYS = 6 => "vehicle_maintenance.days",
        VEHICLE_MAINT_DECAY = 7 => "vehicle_maintenance.decay",
        VEHICLE_MAINT_DELAY = 8 => "vehicle_maintenance.delay",
        VEHICLE_MAINT_ENDED = 9 => "vehicle_maintenance.ended",
        VEHICLE_MAINT_LOADING = 10 => "vehicle_maintenance.loading",
        VEHICLE_MAINT_MAX_AGE = 11 => "vehicle_maintenance.max_age",
        VEHICLE_MAINT_MINUTES = 12 => "vehicle_maintenance.minutes",
        VEHICLE_MAINT_OLD_NEWS = 13 => "vehicle_maintenance.old_news",
        VEHICLE_MAINT_PERCENT = 14 => "vehicle_maintenance.percent",
        VEHICLE_MAINT_PROFIT_AGE = 15 => "vehicle_maintenance.profit_age",
        VEHICLE_MAINT_RENEW = 16 => "vehicle_maintenance.renew",
        VEHICLE_MAINT_REPLACE = 17 => "vehicle_maintenance.replace",
        VEHICLE_MAINT_RESET = 18 => "vehicle_maintenance.reset",
        VEHICLE_MAINT_RESET_REDUCED = 19 => "vehicle_maintenance.reset_reduced",
        VEHICLE_MAINT_SERVICE = 20 => "vehicle_maintenance.service",
        VEHICLE_MAINT_SMOKE = 21 => "vehicle_maintenance.smoke",
        VEHICLE_MAINT_STARTED = 22 => "vehicle_maintenance.started",
        VEHICLE_MAINT_TRIGGER = 23 => "vehicle_maintenance.trigger",
        VEHICLE_MAINT_VALUE = 24 => "vehicle_maintenance.value",
        VEHICLE_MAINT_ENTRIES = 25 => "vehicle_maintenance.entries",
    }
    water {
        WATER_TRACKS = 0 => "water.tracks",
        WATER_FOLLOWS = 1 => "water.follows",
        WATER_AQUEDUCTS = 2 => "water.aqueducts",
        WATER_REBUILDS = 3 => "water.rebuilds",
    }
    water_cmd {
        WATER_CMD_RESTORE = 0 => "water_cmd.restore",
        WATER_CMD_DEPOT = 1 => "water_cmd.depot",
        WATER_CMD_REMOVE_DEPOT = 2 => "water_cmd.remove_depot",
        WATER_CMD_FORCE = 3 => "water_cmd.force",
        WATER_CMD_LOCK = 4 => "water_cmd.lock",
        WATER_CMD_REMOVE_LOCK = 5 => "water_cmd.remove_lock",
        WATER_CMD_CANAL = 6 => "water_cmd.canal",
        WATER_CMD_SKIP = 7 => "water_cmd.skip",
        WATER_CMD_CLEAR = 8 => "water_cmd.clear",
        WATER_CMD_OWNER = 9 => "water_cmd.owner",
        WATER_CMD_DESERT = 10 => "water_cmd.desert",
    }

}

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
    fn blocks_do_not_overlap() {
        // Every local index is consecutive inside its block; all blocks together
        // cover the dense counter array exactly once, with no overlaps or holes.
        assert_eq!(INDICES, (0..NAMES.len()).collect::<Vec<_>>());
        let prefixes: Vec<_> = BLOCKS
            .iter()
            .map(|block| block[0].split_once('.').unwrap().0)
            .collect();
        assert!(prefixes.windows(2).all(|pair| pair[0] < pair[1]));
        for (block, prefix) in BLOCKS.iter().zip(prefixes) {
            assert!(
                block
                    .iter()
                    .all(|name| name.split_once('.').unwrap().0 == prefix)
            );
        }
    }

    #[test]
    fn names_are_unique_and_exported() {
        let unique: std::collections::BTreeSet<_> = NAMES.iter().collect();
        assert_eq!(unique.len(), NAMES.len());
        for (index, name) in NAMES.iter().enumerate() {
            let mut length = 0;
            let pointer = unsafe { openttd_rust_witness_name(index, &raw mut length) };
            assert_eq!(length, name.len());
            // SAFETY: The export supplies static readable bytes of this length.
            assert_eq!(
                unsafe { std::slice::from_raw_parts(pointer, length) },
                name.as_bytes()
            );
        }
        let mut length = 0;
        assert!(unsafe { openttd_rust_witness_name(NAMES.len(), &raw mut length) }.is_null());
    }
}
