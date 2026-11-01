/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Simulation callbacks of `settings_table.cpp` and the setting commands of `settings.cpp`.
//! Native setting descriptors stay opaque: lookup, validation and `IntSettingDesc::ChangeValue`
//! are whole native operations, which reenter these callbacks. Tables are immutable for the
//! process; no Rust borrow of game state spans a service call, and map reads and writes go
//! through an entry-local `World` that ends before any presentation call.
#![allow(unsafe_code)]
use crate::command_type::DoCommandFlag;
use crate::company_type::CompanyID;
use crate::map_func::{Map, World};
use crate::openttd::GameMode;
use crate::order_func::{
    DEF_SERVINT_DAYS_AIRCRAFT, DEF_SERVINT_DAYS_ROADVEH, DEF_SERVINT_DAYS_SHIPS,
    DEF_SERVINT_DAYS_TRAINS, DEF_SERVINT_MINUTES_AIRCRAFT, DEF_SERVINT_MINUTES_ROADVEH,
    DEF_SERVINT_MINUTES_SHIPS, DEF_SERVINT_MINUTES_TRAINS, DEF_SERVINT_PERCENT, MAX_SERVINT_DAYS,
    MAX_SERVINT_MINUTES, MAX_SERVINT_PERCENT, MIN_SERVINT_DAYS, MIN_SERVINT_MINUTES,
    MIN_SERVINT_PERCENT,
};
use crate::raw_field::{read_field, write_field};
use crate::services::{ByteSpan, CommandCost, EnvironmentServices};
use crate::strings_type::INVALID_STRING_ID;
use crate::tile_map::{set_tile_height, set_tile_type, tile_height_in};
use crate::tile_type::MP_WATER;
use crate::timer_game_calendar::{
    DEF_MINUTES_PER_YEAR, FROZEN_MINUTES_PER_YEAR, MAX_MINUTES_PER_YEAR,
};
use crate::vehicle::VehicleRef;
use crate::vehicle_type::{AM_ORIGINAL, VEH_AIRCRAFT, VEH_ROAD, VEH_SHIP, VEH_TRAIN, VehicleID};
use crate::window_type::{
    WC_BUILD_VEHICLE, WC_ENGINE_PREVIEW, WC_GAME_OPTIONS, WC_TOWN_VIEW, WC_VEHICLE_DETAILS,
};
use crate::witness::{self, hit};

/// Opaque native `SettingDesc`.
#[repr(C)]
pub struct SettingDesc {
    _private: [u8; 0],
}

/// Native `VehicleDefaultSettings`, read and written in place.
#[repr(C)]
#[allow(clippy::struct_field_names)]
pub struct VehicleDefaults {
    pub servint_ispercent: bool,
    pub servint_trains: u16,
    pub servint_roadveh: u16,
    pub servint_aircraft: u16,
    pub servint_ships: u16,
}

/// A range callback's `{min, max}` result.
#[repr(C)]
pub struct Range {
    pub min: i32,
    pub max: u32,
}

/// Native leaves of the setting callbacks; existing owners are reached through their tables.
#[repr(C)]
pub struct Services {
    pub shared: &'static crate::services::Services,
    pub clock: &'static crate::timer_game_common::ClockServices,
    pub company: &'static crate::company::Services,
    pub transactions: &'static crate::vehicle_transactions::TransactionServices,
    pub refit: &'static crate::vehicle_transactions::refit::Services,
    pub objectives: &'static crate::objectives::Services,
    pub(crate) schedule: &'static crate::linkgraph_schedule::Services,
    pub engines: &'static crate::engine_loading::Services,
    pub environment: EnvironmentServices,
    /// `&_settings_client.company.vehicle`.
    pub client_vehicle_defaults: extern "C" fn() -> *mut VehicleDefaults,
    /// `&Company::Get(c)->settings.vehicle`.
    pub company_vehicle_defaults: extern "C" fn(CompanyID) -> *mut VehicleDefaults,
    pub set_service_interval: extern "C" fn(VehicleID, u16),
    pub set_service_interval_percent: extern "C" fn(VehicleID, bool),
    /// `_settings_newgame.economy.minutes_per_calendar_year`.
    pub newgame_minutes_per_year: extern "C" fn() -> u16,
    /// Store minutes per year in the new-game (true) or current game settings.
    pub set_minutes_per_year: extern "C" fn(bool, u16),
    /// `_settings_newgame.economy.timekeeping_units = TKU_WALLCLOCK`.
    pub set_newgame_wallclock: extern "C" fn(),
    /// `GetSettingFromName`.
    pub setting: extern "C" fn(ByteSpan) -> *const SettingDesc,
    /// `GetCompanySettingFromName`.
    pub company_setting: extern "C" fn(ByteSpan) -> *const SettingDesc,
    /// `SlIsObjectCurrentlyValid` for the descriptor's save versions.
    pub setting_current: extern "C" fn(*const SettingDesc) -> bool,
    pub setting_int: extern "C" fn(*const SettingDesc) -> bool,
    /// `IsEditable(true)`.
    pub setting_editable: extern "C" fn(*const SettingDesc) -> bool,
    /// `ChangeValue(&GetGameSettings(), value)`.
    pub change_game_setting: extern "C" fn(*const SettingDesc, i32),
    /// `ChangeValue(&Company::Get(c)->settings, value)`.
    pub change_company_setting: extern "C" fn(*const SettingDesc, CompanyID, i32),
    pub window_classes_dirty: extern "C" fn(u16),
    /// `ShowErrorMessage(GetEncodedString(id), {}, WL_ERROR)`.
    pub error_message: extern "C" fn(u32),
    pub edges_not_empty: u32,
    pub edges_not_water: u32,
    pub dynamic_engines_existing_vehicles: u32,
    pub too_high_mountain: u32,
}

/// `Vehicle::Iterate()` in pool order; each step ends before the loop body runs.
fn vehicles(s: &Services) -> impl Iterator<Item = VehicleRef> + '_ {
    let mut from = 0u32;
    std::iter::from_fn(move || {
        let v = (s.shared.vehicle_next)(from)?;
        from = (s.clock.vehicle_id)(v.0.as_ptr().cast()).wrapping_add(1);
        Some(v)
    })
}

fn vehicle_services(s: &Services) -> &crate::vehicle::VehicleServices {
    // SAFETY: The movement registry holds the process-lifetime vehicle table.
    unsafe { &*crate::movement::table(s.shared).vehicle }
}

fn train_leaves(s: &Services) -> *const crate::train::Leaves {
    crate::movement::table(s.shared).train
}

/// `Train::Iterate()` with `IsFrontEngine()` and `IsFreeWagon()` read at the source points.
fn trains(s: &Services, mut each: impl FnMut(VehicleRef, u8)) {
    let vs = vehicle_services(s);
    for v in vehicles(s) {
        if (vs.vehicle_type)(v) == VEH_TRAIN {
            each(v, (vs.subtype)(v));
        }
    }
}

fn menu() -> bool {
    crate::session::mode() == GameMode::MENU
}

/// The original's `vds` choice: client defaults in the menu or without a valid company.
fn vehicle_defaults(s: &Services) -> (*mut VehicleDefaults, Option<CompanyID>) {
    if menu() {
        return ((s.client_vehicle_defaults)(), None);
    }
    let company = CompanyID((s.shared.current_company)());
    if (s.shared.company_valid)(company.0) {
        ((s.company_vehicle_defaults)(company), Some(company))
    } else {
        ((s.client_vehicle_defaults)(), None)
    }
}

/// `Vehicle::Iterate()` filtered by `owner == company`, yielding index and type.
fn owned_vehicles(s: &Services, company: CompanyID, mut each: impl FnMut(VehicleID, u8)) {
    let mut from = 0;
    loop {
        let mut kind = 0;
        let id = (s.company.vehicle_next_owned)(company.0, from, &raw mut kind);
        if id == u32::MAX {
            break;
        }
        from = id.wrapping_add(1);
        each(VehicleID(id), kind);
    }
}

fn update_all_service_interval(s: &Services, new_value: i32) {
    let (vds, company) = vehicle_defaults(s);
    let defaults = if new_value != 0 {
        [DEF_SERVINT_PERCENT; 4]
    } else if (s.clock.using_wallclock_units)(menu()) {
        [
            DEF_SERVINT_MINUTES_TRAINS,
            DEF_SERVINT_MINUTES_ROADVEH,
            DEF_SERVINT_MINUTES_AIRCRAFT,
            DEF_SERVINT_MINUTES_SHIPS,
        ]
    } else {
        [
            DEF_SERVINT_DAYS_TRAINS,
            DEF_SERVINT_DAYS_ROADVEH,
            DEF_SERVINT_DAYS_AIRCRAFT,
            DEF_SERVINT_DAYS_SHIPS,
        ]
    };
    write_field!(vds, servint_trains, defaults[0]);
    write_field!(vds, servint_roadveh, defaults[1]);
    write_field!(vds, servint_aircraft, defaults[2]);
    write_field!(vds, servint_ships, defaults[3]);

    if let Some(company) = company {
        hit(witness::SETTINGS_SERVICE_DEFAULTS);
        owned_vehicles(s, company, |id, kind| {
            if (s.company.vehicle_is_primary)(id.0)
                && !(s.company.vehicle_service_interval_is_custom)(id.0)
            {
                hit(witness::SETTINGS_SERVICE_VEHICLE);
                // `SetServiceInterval` takes the `int` of `CompanyServiceInterval` as uint16_t.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let interval = s.transactions.interval_default(company.0, kind) as u16;
                (s.set_service_interval)(id, interval);
                (s.set_service_interval_percent)(id, new_value != 0);
            }
        });
    }
    (s.window_classes_dirty)(WC_VEHICLE_DETAILS);
}

fn can_update_service_interval(s: &Services, new_value: i32) -> bool {
    let (vds, _) = vehicle_defaults(s);
    let interval = (s.transactions.interval_clamp)(new_value, read_field!(vds, servint_ispercent));
    new_value == 0 || i32::from(interval) == new_value
}

fn update_service_interval(s: &Services, kind: u8, new_value: i32) {
    if !menu() {
        let company = CompanyID((s.shared.current_company)());
        if (s.shared.company_valid)(company.0) {
            owned_vehicles(s, company, |id, vehicle_kind| {
                if vehicle_kind == kind
                    && (s.company.vehicle_is_primary)(id.0)
                    && !(s.company.vehicle_service_interval_is_custom)(id.0)
                {
                    hit(witness::SETTINGS_SERVICE_TYPE);
                    // `SetServiceInterval(uint16_t)` truncates the setting value.
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    (s.set_service_interval)(id, new_value as u16);
                }
            });
        }
    }
    (s.window_classes_dirty)(WC_VEHICLE_DETAILS);
}

fn default_service_interval(s: &Services, def: i32, kind: u8) -> i32 {
    let (vds, _) = vehicle_defaults(s);
    if read_field!(vds, servint_ispercent) {
        return i32::from(DEF_SERVINT_PERCENT);
    }
    if (s.clock.using_wallclock_units)(menu()) {
        return i32::from(match kind {
            VEH_TRAIN => DEF_SERVINT_MINUTES_TRAINS,
            VEH_ROAD => DEF_SERVINT_MINUTES_ROADVEH,
            VEH_AIRCRAFT => DEF_SERVINT_MINUTES_AIRCRAFT,
            VEH_SHIP => DEF_SERVINT_MINUTES_SHIPS,
            _ => unreachable!("NOT_REACHED"),
        });
    }
    def
}

fn service_interval_range(s: &Services) -> Range {
    let (vds, _) = vehicle_defaults(s);
    let (min, max) = if read_field!(vds, servint_ispercent) {
        (MIN_SERVINT_PERCENT, MAX_SERVINT_PERCENT)
    } else if (s.clock.using_wallclock_units)(menu()) {
        (MIN_SERVINT_MINUTES, MAX_SERVINT_MINUTES)
    } else {
        (MIN_SERVINT_DAYS, MAX_SERVINT_DAYS)
    };
    Range {
        min: i32::from(min),
        max: u32::from(max),
    }
}

fn update_consists(s: &Services) {
    let vs = vehicle_services(s);
    trains(s, |v, subtype| {
        if crate::vehicle_base::is_front_engine(true, subtype)
            || crate::ground_vehicle::is_free_wagon(subtype)
        {
            hit(witness::SETTINGS_CONSISTS);
            // SAFETY: Process-lifetime tables; the live train shell is resolved per call.
            unsafe {
                crate::train::openttd_rust_train_consist_changed(
                    (vs.train_handle)(v.0.as_ptr()),
                    crate::train::CCF_TRACK,
                    train_leaves(s),
                    s.shared,
                );
            }
        }
    });
    (s.objectives.window_classes)(WC_BUILD_VEHICLE, 0);
}

fn acceleration_windows(s: &Services) {
    (s.window_classes_dirty)(WC_ENGINE_PREVIEW);
    (s.objectives.window_classes)(WC_BUILD_VEHICLE, 0);
    (s.window_classes_dirty)(WC_VEHICLE_DETAILS);
}

fn train_acceleration_model_changed(s: &Services) {
    let vs = vehicle_services(s);
    trains(s, |v, subtype| {
        if crate::vehicle_base::is_front_engine(true, subtype) {
            hit(witness::SETTINGS_TRAIN_ACCELERATION);
            let id = (vs.train_handle)(v.0.as_ptr());
            // SAFETY: Process-lifetime tables; the live train shell is resolved per call.
            let limit = unsafe {
                crate::train::openttd_rust_train_curve_limit(id, train_leaves(s), s.shared)
            };
            id.set_cached_max_curve_speed(u64::from(limit));
            // SAFETY: As above.
            unsafe {
                crate::train::openttd_rust_train_update_acceleration(id, train_leaves(s), s.shared);
            }
        }
    });
    acceleration_windows(s);
}

fn physics(s: &Services) -> &crate::ground_vehicle::Services {
    // SAFETY: The movement registry holds the process-lifetime train leaves.
    unsafe { (*train_leaves(s)).physics }
}

fn train_slope_steepness_changed(s: &Services) {
    trains(s, |v, subtype| {
        if crate::vehicle_base::is_front_engine(true, subtype) {
            hit(witness::SETTINGS_SLOPE);
            crate::ground_vehicle::cargo_changed(physics(s), v);
        }
    });
}

/// `RoadVehicle::Iterate()` front engines.
fn road_front_engines(s: &Services, mut each: impl FnMut(VehicleRef)) {
    let vs = vehicle_services(s);
    for v in vehicles(s) {
        if (vs.vehicle_type)(v) == VEH_ROAD
            && crate::vehicle_base::is_front_engine(true, (vs.subtype)(v))
        {
            each(v);
        }
    }
}

fn road_acceleration_model_changed(s: &Services) {
    if (s.refit.road_acceleration)() != AM_ORIGINAL {
        road_front_engines(s, |v| {
            hit(witness::SETTINGS_ROAD_ACCELERATION);
            crate::ground_vehicle::cargo_changed(physics(s), v);
        });
    }
    acceleration_windows(s);
}

fn road_slope_steepness_changed(s: &Services) {
    road_front_engines(s, |v| {
        hit(witness::SETTINGS_SLOPE);
        crate::ground_vehicle::cargo_changed(physics(s), v);
    });
}

fn difficulty_noise_change(s: &Services) {
    if crate::session::mode() == GameMode::NORMAL {
        hit(witness::SETTINGS_NOISE);
        let commands = (s.shared.station_commands)();
        // SAFETY: The shared registry returns the process-lifetime station command table.
        unsafe { crate::station_cmd::openttd_rust_station_update_airport_noise(commands) };
        // SAFETY: As above.
        if (unsafe { &*commands }.noise_enabled)() {
            (s.objectives.window_classes)(WC_TOWN_VIEW, 0);
        }
    }
}

fn check_road_side(s: &Services) -> bool {
    let vs = vehicle_services(s);
    let allowed = menu()
        || !crate::road_cmd::road_vehicles_are_built(vehicles(s).map(|v| (vs.vehicle_type)(v)));
    if !allowed {
        hit(witness::SETTINGS_ROAD_SIDE_REFUSED);
    }
    allowed
}

fn refuse(s: &Services, message: u32) -> bool {
    (s.error_message)(message);
    false
}

fn check_freeform_edges(s: &Services, new_value: i32) -> bool {
    if menu() {
        return true;
    }
    let map = s.shared.map();
    if new_value != 0 {
        let vs = vehicle_services(s);
        for v in vehicles(s) {
            if (vs.vehicle_type)(v) != VEH_SHIP {
                continue;
            }
            let tile = (vs.tile)(v);
            if map.tile_x(tile) == 0 || map.tile_y(tile) == 0 {
                hit(witness::SETTINGS_EDGES_OCCUPIED);
                return refuse(s, s.edges_not_empty);
            }
        }
        // SAFETY: The shared registry returns the process-lifetime catchment table.
        let catchment = unsafe { &*(s.shared.station_catchment)() };
        // SAFETY: As above, for its station services.
        let stations = unsafe { &*catchment.station };
        for id in 0..(stations.pool_size)() {
            // BaseStation pool IDs are bounded by their u16 representation.
            #[allow(clippy::cast_possible_truncation)]
            let st = (stations.get)(id as u16);
            if st.is_null() || !(stations.in_use)(st) {
                continue;
            }
            let xy = (catchment.location)(st);
            if map.tile_x(xy) == 0 || map.tile_y(xy) == 0 {
                hit(witness::SETTINGS_EDGES_OCCUPIED);
                return refuse(s, s.edges_not_empty);
            }
        }
        return true;
    }
    let w = World::new(s.shared);
    let max_x = map.max_x();
    let max_y = Map::max_y(w.size_y());
    let water = |t: u32| w.kind(t) == MP_WATER;
    let edges_land = (0..max_x).any(|i| tile_height_in(&w, map.tile_xy(i, 1)) != 0)
        || (1..max_x).any(|i| {
            !water(map.tile_xy(i, max_y - 1)) || tile_height_in(&w, map.tile_xy(1, max_y)) != 0
        })
        || (0..max_y).any(|i| tile_height_in(&w, map.tile_xy(1, i)) != 0)
        || (1..max_y).any(|i| {
            !water(map.tile_xy(max_x - 1, i)) || tile_height_in(&w, map.tile_xy(max_x, i)) != 0
        });
    if edges_land {
        hit(witness::SETTINGS_EDGES_NOT_WATER);
        return refuse(s, s.edges_not_water);
    }
    true
}

fn update_freeform_edges(s: &Services, new_value: i32) {
    if menu() {
        return;
    }
    let w = World::new(s.shared);
    let map = w.dimensions;
    let freeform = || (s.environment.freeform_edges)();
    if new_value != 0 {
        hit(witness::SETTINGS_EDGES_VOID);
        for x in 0..map.size_x() {
            crate::void_map::make_void(&w, map.tile_xy(x, 0), freeform);
        }
        for y in 0..w.size_y() {
            crate::void_map::make_void(&w, map.tile_xy(0, y), freeform);
        }
    } else {
        hit(witness::SETTINGS_EDGES_WATER);
        for i in 0..map.max_x() {
            set_tile_height(&w, map.tile_xy(i, 0), 0);
            set_tile_type(&w, map.tile_xy(i, 0), MP_WATER, freeform);
        }
        for i in 0..Map::max_y(w.size_y()) {
            set_tile_height(&w, map.tile_xy(0, i), 0);
            set_tile_type(&w, map.tile_xy(0, i), MP_WATER, freeform);
        }
    }
    (s.company.shared.screen_dirty)();
}

fn check_dynamic_engines(s: &Services) -> bool {
    if menu() {
        return true;
    }
    if !crate::engine_loading::openttd_rust_engine_reset_config(s.engines) {
        hit(witness::SETTINGS_DYNAMIC_REFUSED);
        return refuse(s, s.dynamic_engines_existing_vehicles);
    }
    true
}

fn check_max_height_level(s: &Services, new_value: i32) -> bool {
    let mode = crate::session::mode();
    if mode == GameMode::NORMAL {
        return false;
    }
    if mode != GameMode::EDITOR {
        return true;
    }
    let w = World::new(s.shared);
    // `(int32_t)TileHeight(t)` of a byte height.
    #[allow(clippy::cast_possible_wrap)]
    let too_high = Map::iterate(w.types.count()).any(|t| tile_height_in(&w, t) as i32 > new_value);
    if too_high {
        hit(witness::SETTINGS_HEIGHT_REFUSED);
        return refuse(s, s.too_high_mountain);
    }
    true
}

fn station_catchment_changed(s: &Services) {
    hit(witness::SETTINGS_CATCHMENT);
    // SAFETY: The shared registry returns the process-lifetime catchment table.
    crate::station::recompute_catchment_for_all(&unsafe {
        crate::station::Ctx::new((s.shared.station_catchment)())
    });
    (s.company.shared.screen_dirty)();
}

fn change_timekeeping_units(s: &Services) {
    if !read_field!((s.client_vehicle_defaults)(), servint_ispercent) {
        update_all_service_interval(s, 0);
    }
    if !(s.clock.using_wallclock_units)(true) {
        #[allow(clippy::cast_possible_truncation)]
        (s.set_minutes_per_year)(true, DEF_MINUTES_PER_YEAR as u16);
    }
    (s.objectives.window_classes)(WC_GAME_OPTIONS, 0);

    if crate::session::mode() == GameMode::EDITOR {
        hit(witness::SETTINGS_TIMEKEEPING_EDITOR);
        let wallclock = (s.clock.using_wallclock_units)(false);
        let (date, fraction) = if wallclock {
            (
                crate::timer_game_economy::ymd_to_date(1, 0, 1, wallclock),
                0,
            )
        } else {
            (
                crate::timer_game_common::calendar_date().0,
                crate::timer_game_common::calendar_fraction(),
            )
        };
        let interval = || date.wrapping_sub(crate::timer_game_common::economy_date().0);
        crate::linkgraph_schedule::openttd_rust_schedule_shift(s.schedule, interval());
        for v in vehicles(s) {
            (s.clock.vehicle_shift_dates)(v, interval());
        }
        // `TimerGameEconomy::SetDate` asserts the fraction.
        crate::movement::native_assert(
            s.shared,
            i32::from(fraction) < crate::timer_game_tick::DAY_TICKS,
        );
        crate::timer_game_economy::openttd_rust_economy_set_date(
            date,
            fraction,
            (s.clock.using_wallclock_units)(false),
        );
    }
}

fn change_minutes_per_year(s: &Services, new_value: i32) {
    // The setting is never below the default except for 0, a frozen calendar.
    #[allow(clippy::cast_possible_wrap)]
    if new_value < DEF_MINUTES_PER_YEAR as i32 {
        hit(witness::SETTINGS_MINUTES_CLAMPED);
        // From 0, an increase to 1 jumps to the default.
        let clamped = if new_value == 1 {
            DEF_MINUTES_PER_YEAR
        } else {
            FROZEN_MINUTES_PER_YEAR
        };
        #[allow(clippy::cast_possible_truncation)]
        (s.set_minutes_per_year)(menu(), clamped as u16);
    }
    if menu()
        && u32::from((s.newgame_minutes_per_year)()) != DEF_MINUTES_PER_YEAR
        && !(s.clock.using_wallclock_units)(true)
    {
        (s.set_newgame_wallclock)();
        change_timekeeping_units(s);
    }
}

#[allow(clippy::cast_possible_wrap)]
fn minutes_per_year_range(s: &Services) -> Range {
    if (s.clock.using_wallclock_units)(menu()) {
        return Range {
            min: FROZEN_MINUTES_PER_YEAR as i32,
            max: MAX_MINUTES_PER_YEAR,
        };
    }
    Range {
        min: DEF_MINUTES_PER_YEAR as i32,
        max: DEF_MINUTES_PER_YEAR,
    }
}

/// `CmdChangeSetting` and `CmdChangeCompanySetting`: `company` selects the company table.
fn change_setting(
    s: &Services,
    cost: *mut CommandCost,
    flags: u16,
    name: ByteSpan,
    value: i32,
    company: bool,
) {
    let refuse = || {
        hit(witness::SETTINGS_COMMAND_REFUSED);
        (s.transactions.cost.error)(cost, INVALID_STRING_ID);
    };
    if name.size == 0 {
        return refuse();
    }
    let sd = if company {
        (s.company_setting)(name)
    } else {
        (s.setting)(name)
    };
    if sd.is_null() {
        return refuse();
    }
    if !company && !(s.setting_current)(sd) {
        return refuse();
    }
    if !(s.setting_int)(sd) {
        return refuse();
    }
    if !company && !(s.setting_editable)(sd) {
        return refuse();
    }
    if flags & DoCommandFlag::EXECUTE != 0 {
        if company {
            hit(witness::SETTINGS_COMPANY_COMMAND);
            (s.change_company_setting)(sd, CompanyID((s.shared.current_company)()), value);
        } else {
            hit(witness::SETTINGS_COMMAND);
            (s.change_game_setting)(sd, value);
        }
    }
}

// One export per native callback name; each takes the process-lifetime table.
macro_rules! callbacks {
    ($($export:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)? => $body:expr;)*) => {$(
        #[unsafe(no_mangle)]
        pub extern "C" fn $export(s: &Services, $($arg: $ty),*) $(-> $ret)? {
            $body(s, $($arg),*)
        }
    )*};
}
callbacks! {
    openttd_rust_settings_update_consists() => update_consists;
    openttd_rust_settings_update_all_service_interval(new_value: i32) => update_all_service_interval;
    openttd_rust_settings_can_update_service_interval(new_value: i32) -> bool => can_update_service_interval;
    openttd_rust_settings_update_service_interval(kind: u8, new_value: i32) => update_service_interval;
    openttd_rust_settings_default_service_interval(def: i32, kind: u8) -> i32 => default_service_interval;
    openttd_rust_settings_service_interval_range() -> Range => service_interval_range;
    openttd_rust_settings_train_acceleration_model_changed() => train_acceleration_model_changed;
    openttd_rust_settings_train_slope_steepness_changed() => train_slope_steepness_changed;
    openttd_rust_settings_road_acceleration_model_changed() => road_acceleration_model_changed;
    openttd_rust_settings_road_slope_steepness_changed() => road_slope_steepness_changed;
    openttd_rust_settings_difficulty_noise_change() => difficulty_noise_change;
    openttd_rust_settings_check_road_side() -> bool => check_road_side;
    openttd_rust_settings_check_freeform_edges(new_value: i32) -> bool => check_freeform_edges;
    openttd_rust_settings_update_freeform_edges(new_value: i32) => update_freeform_edges;
    openttd_rust_settings_check_dynamic_engines() -> bool => check_dynamic_engines;
    openttd_rust_settings_check_max_height_level(new_value: i32) -> bool => check_max_height_level;
    openttd_rust_settings_station_catchment_changed() => station_catchment_changed;
    openttd_rust_settings_change_timekeeping_units() => change_timekeeping_units;
    openttd_rust_settings_change_minutes_per_year(new_value: i32) => change_minutes_per_year;
    openttd_rust_settings_minutes_per_year_range() -> Range => minutes_per_year_range;
}

/// `CanChangeTimetableMode`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_settings_can_change_timetable_mode(s: &Services) -> bool {
    !(s.clock.using_wallclock_units)(false)
}

/// # Safety
/// `cost` is the facade's live native `CommandCost`; `name` denotes live bytes for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cmd_change_setting(
    s: &Services,
    cost: *mut CommandCost,
    flags: u16,
    name: ByteSpan,
    value: i32,
) {
    change_setting(s, cost, flags, name, value, false);
}

/// # Safety
/// As for `openttd_rust_cmd_change_setting`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cmd_change_company_setting(
    s: &Services,
    cost: *mut CommandCost,
    flags: u16,
    name: ByteSpan,
    value: i32,
) {
    change_setting(s, cost, flags, name, value, true);
}

crate::abi::layouts! {
    fn abi_layout;
    "VehicleDefaultSettings" ["settings"] => VehicleDefaults {
        servint_ispercent, servint_trains, servint_roadveh, servint_aircraft, servint_ships,
    };
    "OpenTTDSettingRange" ["settings"] => Range { min, max, };
    "OpenTTDSettingServices" ["settings"] => Services {
        shared, clock, company, transactions, refit, objectives, schedule, engines, environment,
        client_vehicle_defaults, company_vehicle_defaults, set_service_interval,
        set_service_interval_percent, newgame_minutes_per_year,
        set_minutes_per_year, set_newgame_wallclock, setting, company_setting, setting_current,
        setting_int, setting_editable, change_game_setting, change_company_setting,
        window_classes_dirty, error_message, edges_not_empty,
        edges_not_water, dynamic_engines_existing_vehicles, too_high_mountain,
    };
}
