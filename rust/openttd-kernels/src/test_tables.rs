/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared test-only service tables (#736, root's ruling on entry 11).
//!
//! Plain data only: each table is a `const` struct literal, so a test copies it and sets
//! the live entries it needs explicitly. Every function entry is an `unreachable{N}` stub
//! for its arity, which may only panic. Data entries are inert zero, `false` or null
//! placeholders that no test may rely on; a test that reads one sets it. References name
//! the matching table, so `&'static` chains are valid. No builders, macros, traits,
//! defaults or behaviour. A later item adds only the tables it needs.
//! `crate::engine_capabilities::TEST_TABLE`, `crate::station_lifecycle::TEST_TABLE` and
//! `crate::cargo_registry::TEST_TABLE` stay in their modules for their private fields.

use std::ptr;

/// Every 0-argument entry; any call is a test error.
pub(crate) extern "C" fn unreachable0<R>() -> R {
    unreachable!("unexpected 0-argument service call")
}

/// Every 1-argument entry; any call is a test error.
pub(crate) extern "C" fn unreachable1<A, R>(_: A) -> R {
    unreachable!("unexpected 1-argument service call")
}

/// Every 2-argument entry; any call is a test error.
pub(crate) extern "C" fn unreachable2<A, B, R>(_: A, _: B) -> R {
    unreachable!("unexpected 2-argument service call")
}

/// Every 3-argument entry; any call is a test error.
pub(crate) extern "C" fn unreachable3<A, B, C, R>(_: A, _: B, _: C) -> R {
    unreachable!("unexpected 3-argument service call")
}

/// Every 4-argument entry; any call is a test error.
pub(crate) extern "C" fn unreachable4<A, B, C, D, R>(_: A, _: B, _: C, _: D) -> R {
    unreachable!("unexpected 4-argument service call")
}

/// Every 5-argument entry; any call is a test error.
pub(crate) extern "C" fn unreachable5<A, B, C, D, E, R>(_: A, _: B, _: C, _: D, _: E) -> R {
    unreachable!("unexpected 5-argument service call")
}

/// Every 6-argument entry; any call is a test error.
pub(crate) extern "C" fn unreachable6<A, B, C, D, E, F, R>(
    _: A,
    _: B,
    _: C,
    _: D,
    _: E,
    _: F,
) -> R {
    unreachable!("unexpected 6-argument service call")
}

/// Every 9-argument entry; any call is a test error.
pub(crate) extern "C" fn unreachable9<A, B, C, D, E, F, G, H, I, R>(
    _: A,
    _: B,
    _: C,
    _: D,
    _: E,
    _: F,
    _: G,
    _: H,
    _: I,
) -> R {
    unreachable!("unexpected 9-argument service call")
}

/// Every 10-argument entry; any call is a test error.
pub(crate) extern "C" fn unreachable10<A, B, C, D, E, F, G, H, I, J, R>(
    _: A,
    _: B,
    _: C,
    _: D,
    _: E,
    _: F,
    _: G,
    _: H,
    _: I,
    _: J,
) -> R {
    unreachable!("unexpected 10-argument service call")
}

/// Every 13-argument entry; any call is a test error.
pub(crate) extern "C" fn unreachable13<A, B, C, D, E, F, G, H, I, J, K, L, M, R>(
    _: A,
    _: B,
    _: C,
    _: D,
    _: E,
    _: F,
    _: G,
    _: H,
    _: I,
    _: J,
    _: K,
    _: L,
    _: M,
) -> R {
    unreachable!("unexpected 13-argument service call")
}

/// `crate::road_cmd::RoadCommandServices` with every entry unreachable.
pub(crate) const ROAD_COMMANDS: crate::road_cmd::RoadCommandServices =
    crate::road_cmd::RoadCommandServices {
        physics: &GROUND_VEHICLE,
        do_build: unreachable10,
        rail_commands: ptr::null(),
        town: ptr::null(),
        authority: ptr::null(),
        strings: crate::road_cmd::Strings {
            no_road: 0,
            no_tram: 0,
            junction: 0,
            remove_road: 0,
            no_suitable_road: 0,
            no_suitable_tram: 0,
            authority: 0,
        },
        nearest_tile: unreachable1,
        town_valid: unreachable1,
        town_edge: unreachable1,
        set_town: unreachable2,
        extra_dynamite: unreachable0,
        depot_slope: unreachable2,
        make_depot: unreachable5,
        depot_exit: unreachable2,
        next_effect: unreachable1,
        delete_effect: unreachable1,
        scratch_cost: unreachable1,
        conversion_scratch: unreachable1,
        include_vehicle: unreachable2,
        affected_count: unreachable1,
        affected_at: unreachable2,
    };

/// `crate::ground_vehicle::Services` with every entry unreachable.
pub(crate) const GROUND_VEHICLE: crate::ground_vehicle::Services =
    crate::ground_vehicle::Services {
        vehicle: &VEHICLE,
        road: &ROAD_LEAVES,
        assertions: false,
        resolve: unreachable1,
        weight_input: unreachable1,
        capabilities: &crate::engine_capabilities::TEST_TABLE,
        cargo_weight: unreachable2,
        train_cargo_weight: unreachable2,
        powered: unreachable2,
        rail_type: unreachable1,
        rail_acceleration: unreachable1,
        rail_speed: unreachable1,
        road_speed: unreachable1,
        steepness: unreachable1,
        display_max_speed: unreachable1,
        viewport: unreachable3,
        details_dirty: unreachable1,
        base_crash: unreachable2,
        depot_tile: unreachable2,
    };

/// `crate::vehicle::VehicleServices` with every entry unreachable.
pub(crate) const VEHICLE: crate::vehicle::VehicleServices = crate::vehicle::VehicleServices {
    assert_next: unreachable2,
    record_from_shell: unreachable1,
    tile_links: unreachable1,
    map_log_x: unreachable0,
    assertions: false,
    train_handle: unreachable1,
    waiting_for_unbunching: unreachable1,
    leave_unbunching_depot: unreachable1,
    reset_depot_unbunching: unreachable1,
    enter_depot: unreachable2,
    pathfinding_result: unreachable2,
    new_position: unreachable1,
    base_viewport: unreachable1,
    newgrf: unreachable0,
    property: unreachable4,
    callback: unreachable3,
    set_max_age: unreachable2,
    length_changed: unreachable1,
    play_vehicle_sound: unreachable2,
    play_sound: unreachable2,
    order_free: unreachable1,
    order_dummy: unreachable1,
    start_stop_dirty: unreachable1,
    destroy: unreachable1,
    set_dest_tile: unreachable2,
    engine: unreachable1,
    order_type: unreachable1,
    order_destination: unreachable1,
    order_nonstop: unreachable1,
    write_profit: unreachable2,
    write_economy_service: unreachable2,
    write_calendar_service: unreachable2,
    write_breakdowns: unreachable2,
    write_reliability: unreachable2,
    profit: unreachable1,
    make_dummy: unreachable1,
    check_orders_only: unreachable1,
    maintenance: ptr::null(),
    visits: unreachable0,
    orders: unreachable0,
    ship_get: unreachable1,
    first_sprite: unreachable1,
};

/// `crate::road::Leaves` with every entry unreachable.
pub(crate) const ROAD_LEAVES: crate::road::Leaves = crate::road::Leaves {
    vehicle: ptr::null(),
    ground: ptr::null(),
    shared: ptr::null(),
    max_depot_penalty: unreachable0,
    any_road_bits: unreachable3,
    is_road_depot_tile: unreachable1,
    depot_direction: unreachable1,
    disallowed_directions: unreachable1,
    is_bay_stop: unreachable1,
    bay_direction: unreachable1,
    is_station_road_stop: unreachable1,
    choose_track: unreachable4,
    find_depot: unreachable2,
    tram_build_read: unreachable2,
    disconnect: unreachable0,
    should_stop: unreachable2,
    arrival_news: unreachable4,
    depot_exit_dirty: unreachable1,
    crash_news: unreachable2,
    pay_running: unreachable2,
};

/// `crate::services::Services` with every entry unreachable.
pub(crate) const SHARED: crate::services::Services = crate::services::Services {
    all_compatible_railtypes: unreachable1,
    command_post: unreachable0,
    movement: ptr::null(),
    set_tile_owner: unreachable2,
    autoslope: unreachable0,
    bay_stop_direction: unreachable1,
    bridge_axis: unreachable1,
    bridge_speed: unreachable1,
    build_on_slopes: unreachable0,
    clear_square_services: crate::services::ClearSquareServices {
        docking: unreachable1,
        generating_world: unreachable0,
    },
    company_human: unreachable1,
    company_valid: unreachable1,
    context: ptr::null_mut(),
    cost_animation: unreachable4,
    crossing_road_bits: unreachable1,
    current_company: unreachable0,
    depot_index: unreachable1,
    disaster_sound: unreachable0,
    drive_through_axis: unreachable1,
    forbid_90_deg: unreachable0,
    free_train_reservation: unreachable1,
    has_road_type: unreachable1,
    has_water_ground: unreachable1,
    industry: unreachable3,
    is_bay_stop: unreachable1,
    is_bridge: unreachable1,
    is_drive_through_stop: unreachable1,
    is_plain_rail: unreachable1,
    is_road_depot: unreachable1,
    is_water_tile: unreachable1,
    map: ptr::null(),
    settings: ptr::null(),
    map_max_x: unreachable0,
    map_max_y: unreachable0,
    mark_bridge_dirty: unreachable1,
    mark_dirty: unreachable1,
    next_company: unreachable1,
    no_jetcrash: unreachable0,
    other_tunnel_end: unreachable1,
    plane_crashes: unreachable0,
    plane_speed: unreachable0,
    powered_roadtypes: unreachable1,
    random: unreachable1,
    road_bits: unreachable3,
    road_cost_multiplier: unreachable1,
    road_depot_direction: unreachable1,
    road_type: unreachable1,
    crossing_reserved: unreachable1,
    set_crossing_reserved: unreachable2,
    may_have_road: unreachable1,
    set_road_bits: unreachable3,
    set_road_type: unreachable3,
    set_road_owner: unreachable3,
    set_road_directions: unreachable2,
    road_powered: unreachable2,
    type_availability: unreachable0,
    pbs_services: unreachable1,
    service_at_helipad: unreachable0,
    set_tropic_zone: unreachable2,
    snow_line_support: crate::snow_line::Support {
        height: ptr::null(),
        destroy: unreachable1,
    },
    station_index: unreachable1,
    road_commands: unreachable0,
    kdtree_partition: unreachable2,
    station_commands: unreachable0,
    station_catchment: unreachable0,
    docking_tile: unreachable1,
    station_gfx: unreachable1,
    station_serve_neutral: unreachable0,
    station_type: unreachable1,
    storage_discard: unreachable1,
    tile_animation_set_frame: unreachable2,
    town_next_id: unreachable1,
    town_by_id: unreachable1,
    town_owner_by_id: unreachable1,
    utf8_length: unreachable2,
    vehicle_next: unreachable1,
    water_middle_lock: unreachable1,
    tile_max_z: unreachable1,
    tile_owner: unreachable1,
    tile_slope: unreachable1,
    tile_type: unreachable1,
    tile_valid: unreachable1,
    tile_z: unreachable1,
    town_index: unreachable1,
    town_owner: unreachable1,
    train_trackdir: unreachable1,
    tram_type: unreachable1,
    tropic_zone: unreachable1,
    try_path_reserve: unreachable3,
    tunnel_bridge_direction: unreachable1,
    tunnel_transport: unreachable1,
    update_level_crossing: unreachable2,
    recession: unreachable0,
    tick_counter: unreachable0,
    town_count: unreachable0,
    apply_foundation: unreachable2,
    clear_ground: unreachable1,
    has_water_class: unreachable1,
    make_shore: unreachable1,
    make_snow: unreachable2,
    play_sound: unreachable2,
    set_fence: unreachable3,
    tree_ground: unreachable1,
    water_services: unreachable0,
    for_each_station: unreachable2,
    road_stops: unreachable0,
    nearby_tile: unreachable2,
    nearby_tile_information: unreachable2,
    network_client_info: unreachable1,
    screen_dirty: unreachable0,
    string_data: unreachable2,
    terrain_type: unreachable1,
    copyable_badges: unreachable4,
    grf_file_version: unreachable1,
    grf_price_shift: unreachable2,
    grf_cargo_map: unreachable2,
    grf_boolean_callback: unreachable3,
    grf_file_at_or_null: unreachable1,
    grf_file_id: unreachable1,
    grf_file_name: unreachable1,
    grf_parameter: unreachable2,
    newgrf_specs: ptr::null(),
    resolver_grf_file: unreachable1,
    resolver_badges: unreachable4,
    resolver_grf_file_id: unreachable1,
    resolver_grf_version: unreachable1,
    show_error: unreachable2,
    not_reached: unreachable0,
};

/// `crate::rail_cmd::RailCommandServices` with every entry unreachable.
pub(crate) const RAIL_COMMANDS: crate::rail_cmd::RailCommandServices =
    crate::rail_cmd::RailCommandServices {
        shared: ptr::null(),
        rail: ptr::null(),
        cost: ptr::null(),
        command: ptr::null(),
        signal: ptr::null(),
        strings: crate::rail_cmd::Strings {
            impossible_combination: 0,
            already_built: 0,
            build_on_water: 0,
            sloped_wrong: 0,
            remove_signals_first: 0,
            road_works: 0,
            oneway_road: 0,
            crossing_rail: 0,
            crossing_road: 0,
            no_track: 0,
            owned_by: 0,
            flat_land: 0,
            demolish_bridge: 0,
            no_suitable_track: 0,
            no_signals: 0,
            owned_by_another: 0,
            remove_track: 0,
            demolish_building: 0,
            ship_in_way: 0,
        },
        compatible_rail: unreachable2,
        powered_rail: unreachable2,
        train_powered: unreachable2,
        rail_no_crossings: unreachable1,
        road_no_crossing: unreachable1,
        rail_build_cost: unreachable1,
        rail_clear_cost: unreachable1,
        rail_convert_cost: unreachable2,
        crossing_with_competitor: unreachable0,
        entrance_edge: unreachable4,
        build_rail: unreachable6,
        remove_rail: unreachable4,
        build_signal: unreachable13,
        remove_signal: unreachable4,
        convert_rail: unreachable6,
        rail_infrastructure: unreachable3,
        road_infrastructure: unreachable3,
        signal_infrastructure: unreachable2,
        parking: unreachable1,
        train_position: unreachable1,
        layout_changed: unreachable2,
        mark_crossings: unreachable2,
        crossing_removed: unreachable2,
        make_crossing: unreachable9,
        make_road: unreachable2,
        depot_available: unreachable0,
        new_depot: unreachable1,
        depot_name: unreachable1,
        delete_depot: unreachable1,
        depot_windows: unreachable1,
        close_inspect: unreachable1,
        make_shore: unreachable1,
        include_train: unreachable2,
        consist_changed: unreachable1,
    };

/// `crate::services::CommandCostServices` with every entry unreachable.
pub(crate) const COMMAND_COSTS: crate::services::CommandCostServices =
    crate::services::CommandCostServices {
        construct: unreachable1,
        destroy: unreachable1,
        zero: unreachable1,
        error: unreachable2,
        expense_money: unreachable3,
        error_param: unreachable3,
        add: unreachable2,
        move_cost: unreachable2,
        amount: unreachable2,
        success: unreachable1,
        error_message: unreachable1,
        money: unreachable1,
        expense: unreachable1,
        make_error: unreachable2,
        encode_owned_by: unreachable3,
        error_owner: unreachable2,
    };

/// `crate::town::Leaves` with every entry unreachable.
pub(crate) const TOWN_LEAVES: crate::town::Leaves = crate::town::Leaves {
    map_log_x: unreachable0,
    growth_rate: unreachable0,
    environment: crate::services::EnvironmentServices {
        editor: unreachable0,
        freeform_edges: unreachable0,
    },
    generating_world: unreachable0,
    allow_crossings: unreachable0,
    climate: unreachable0,
    road_flags: unreachable1,
    house_availability: unreachable1,
    house_placement: unreachable1,
    house_population: unreachable1,
    house_flags: unreachable1,
    house_extra_flags: unreachable1,
    house_callbacks: unreachable1,
    road_info: unreachable1,
    resolve_owner: unreachable1,
    dirty_authority: unreachable1,
    dirty_town: unreachable1,
    terraform_price: unreachable0,
    set_road_town: unreachable2,
    exchange_company: unreachable1,
    house_class: unreachable1,
    make_house: unreachable4,
    population_changed: unreachable1,
    house_count: unreachable0,
    road_type_error: unreachable1,
    next_town: unreachable1,
    build_bridge: unreachable5,
    build_tunnel: unreachable3,
    assert_clear_house: unreachable1,
    houses: ptr::null(),
};

/// `crate::town::house::HouseServices` with every entry unreachable.
pub(crate) const HOUSES: crate::town::house::HouseServices = crate::town::house::HouseServices {
    cost: ptr::null(),
    commands: ptr::null(),
    authority: ptr::null(),
    entry: unreachable1,
    loop_tile: unreachable1,
    house_type: unreachable1,
    completed: unreachable1,
    protected: unreachable1,
    lift: unreachable1,
    random: unreachable1,
    construction: unreachable1,
    cache: unreachable1,
    age: unreachable1,
    processing: unreachable1,
    clean_type: unreachable1,
    town: unreachable1,
    loop_spec: unreachable1,
    animation_spec: unreachable1,
    new_spec: unreachable1,
    production: unreachable1,
    sprite_groups: unreachable1,
    removal: unreachable1,
    acceptance: unreachable1,
    watched: unreachable1,
    inc_construction: unreachable1,
    reset_age: unreachable1,
    set_age: unreachable2,
    set_lift_destination: unreachable2,
    set_lift_position: unreachable2,
    halt_lift: unreachable1,
    dec_processing: unreachable1,
    set_processing: unreachable2,
    set_random_bits: unreachable2,
    set_random_triggers: unreachable2,
    set_type: unreachable2,
    delete_inspect: unreachable1,
    cargo_file: unreachable1,
    convert_8bit: unreachable3,
    scale_cargo: unreachable1,
    cargogen_mode: unreachable0,
    place_houses: unreachable0,
    errors: crate::town::house::HouseErrors {
        demolish: 0,
        protected: 0,
        refuses: 0,
        sloped: 0,
        bridge: 0,
        flat: 0,
        no_town: 0,
    },
};

/// `crate::services::CommandServices` with every entry unreachable.
pub(crate) const COMMANDS: crate::services::CommandServices = crate::services::CommandServices {
    command_flags: unreachable1,
    command_type: unreachable1,
    command_valid: unreachable1,
    finance: &FINANCE,
    infinite_money: unreachable0,
    money_error: unreachable2,
    infrastructure_dirty: unreachable1,
    landscape_commands: unreachable0,
    company: unreachable0,
    occupancy: unreachable0,
    shared: &SHARED,
    cost: &COMMAND_COSTS,
    rail_map: &RAIL_MAP,
    signals: &SIGNALS,
    shift_pressed: unreachable0,
    pause_level: unreachable0,
    last_build_coordinate: unreachable2,
    not_enough_cash: 0,
    paused_error: unreachable3,
    error_message: unreachable4,
    estimate: unreachable3,
};

/// `crate::company::FinanceServices` with every entry unreachable.
pub(crate) const FINANCE: crate::company::FinanceServices = crate::company::FinanceServices {
    current_company: unreachable0,
    set_current_company: unreachable1,
    networking: unreachable0,
    company_finances: unreachable1,
    invalidate_company_windows: unreachable1,
    give_money_message: unreachable2,
    money_animation: unreachable2,
};

/// `crate::services::RailMapServices` with every entry unreachable.
pub(crate) const RAIL_MAP: crate::services::RailMapServices = crate::services::RailMapServices {
    ground: ptr::null(),
    traversal: unreachable1,
    station_admission: unreachable3,
    wormhole_admission: unreachable2,
    set_signal_state: unreachable3,
    other_tunnel_bridge_end: unreachable1,
    invalid_rail_type: unreachable0,
    set_rail_type: unreachable2,
    set_track_bits: unreachable2,
    set_present_signals: unreachable2,
    set_signal_states: unreachable2,
    set_has_signals: unreachable2,
    set_signal_type: unreachable3,
    signal_variant: unreachable2,
    set_signal_variant: unreachable3,
    cycle_signal_side: unreachable2,
    rail_reservation: unreachable1,
    set_track_reservation: unreachable2,
    depot_reserved: unreachable1,
    set_depot_exit: unreachable2,
    make_rail_normal: unreachable4,
    make_rail_depot: unreachable5,
    wormhole_reserved: unreachable1,
};

/// `crate::signal::SignalServices` with every entry unreachable.
pub(crate) const SIGNALS: crate::signal::SignalServices = crate::signal::SignalServices {
    assertions: false,
    assert_condition: unreachable1,
    assert_wormhole_start: unreachable2,
    vehicles: ptr::null(),
    assert_company: unreachable1,
};

/// `crate::landscape_cmd::LandscapeServices` with every entry unreachable.
pub(crate) const LANDSCAPE: crate::landscape_cmd::LandscapeServices =
    crate::landscape_cmd::LandscapeServices {
        shared: ptr::null(),
        costs: ptr::null(),
        commands: ptr::null(),
        strings: crate::landscape_cmd::Strings {
            canal: 0,
            clear_limit: 0,
            water: 0,
            sea_level: 0,
            high: 0,
            edge: 0,
            bridge: 0,
            bridge_high: 0,
            tunnel: 0,
            terraform_limit: 0,
            levelled: 0,
        },
        tile_procs: ptr::null(),
        water: ptr::null(),
        on_water: unreachable1,
        coast: unreachable1,
        buoy: unreachable1,
        canal: unreachable1,
        max_bridge_height: unreachable0,
        freeform_edges: unreachable0,
        editor: unreachable0,
        exchange_generating: unreachable1,
        tunnel_in_way: unreachable2,
        mark_dirty_height: unreachable2,
        set_height: unreachable2,
        paused: unreachable0,
    };

/// `crate::landscape_cmd::TileServices` with every entry unreachable.
pub(crate) const LANDSCAPE_TILES: crate::landscape_cmd::TileServices =
    crate::landscape_cmd::TileServices {
        clear: unreachable3,
        terraform: unreachable5,
        owners: ptr::null(),
        trees: ptr::null(),
        industry_policy: ptr::null(),
        industry_costs: ptr::null(),
        industry_generation: ptr::null(),
    };

/// `crate::tile_owner::Services` with every entry unreachable.
pub(crate) const TILE_OWNERS: crate::tile_owner::Services = crate::tile_owner::Services {
    clear: unreachable3,
    house: unreachable3,
    industry: ptr::null(),
    objects: ptr::null(),
    rail: ptr::null(),
    shared: ptr::null(),
    station: ptr::null(),
    trees: unreachable3,
    tunnel_bridge: ptr::null(),
    void: unreachable3,
    water: ptr::null(),
};
/// `crate::ground::ClearServices` with every entry unreachable.
pub(crate) const CLEAR_GROUND: crate::ground::ClearServices = crate::ground::ClearServices {
    field_tile: unreachable1,
    density: unreachable1,
    counter: unreachable1,
    snow: unreachable1,
    surface: unreachable1,
    set_ground: unreachable3,
    set_density: unreachable2,
    set_counter: unreachable2,
    clear_snow: unreachable1,
    make_clear: unreachable3,
    fences: unreachable1,
    field: unreachable1,
    set_field: unreachable2,
};

/// `crate::water_regions::Leaves` with every entry unreachable.
pub(crate) const WATER_REGION_LEAVES: crate::water_regions::Leaves = crate::water_regions::Leaves {
    shared: ptr::null(),
    aqueduct: unreachable1,
    debug: unreachable3,
};

/// `crate::station::CatchmentServices` with every entry unreachable.
pub(crate) const CATCHMENT: crate::station::CatchmentServices = crate::station::CatchmentServices {
    shared: ptr::null(),
    airport: ptr::null(),
    industry_tile: ptr::null(),
    object: ptr::null(),
    town: ptr::null(),
    station: ptr::null(),
    visits: ptr::null(),
    station_count: unreachable0,
    station_if_valid: unreachable1,
    next_station: unreachable1,
    owner: unreachable1,
    station_owner: unreachable1,
    id: unreachable1,
    location: unreachable1,
    associated_industry: unreachable1,
    radius_facilities: unreachable1,
    modified_catchment: unreachable0,
    spread: unreachable0,
    industry_by_id: unreachable1,
    industry_id: unreachable1,
    industry_owner: unreachable1,
    industry_neutral: unreachable1,
    next_industry: unreachable1,
};

/// `crate::newgrf_roadstop::Services` with every entry unreachable.
pub(crate) const ROAD_STOPS: crate::newgrf_roadstop::Services = crate::newgrf_roadstop::Services {
    rebind: unreachable3,
    base_scope: unreachable3,
    build_date: unreachable1,
    by_tile: unreachable1,
    cached_cargo: unreachable1,
    clear: unreachable1,
    company: unreachable2,
    custom: unreachable1,
    facilities: unreachable1,
    frame: unreachable2,
    group: unreachable2,
    group_cargo: unreachable2,
    group_count: unreachable1,
    index: unreachable1,
    is_station: unreachable1,
    make_town: unreachable3,
    owner: unreachable1,
    random_bits: unreachable1,
    resize: unreachable2,
    resolve_callback: unreachable6,
    resolve_reseed: unreachable2,
    set_cached: unreachable3,
    set_random_bits: unreachable2,
    set_slot: unreachable5,
    set_tile_random: unreachable3,
    set_waiting: unreachable2,
    shared: ptr::null(),
    slot_grfid: unreachable2,
    slot_key: unreachable2,
    slot_spec: unreachable2,
    slot_vacancy: unreachable2,
    spec_cargo: unreachable1,
    spec_count: unreachable1,
    spec_triggers: unreachable1,
    station_id: unreachable1,
    tile: unreachable2,
    tile_count: unreachable1,
    tile_random: unreachable2,
    town: unreachable1,
    town_tile: unreachable1,
    waiting: unreachable1,
};

/// `crate::objects::ObjectServices` with every entry unreachable.
pub(crate) const OBJECTS: crate::objects::ObjectServices = crate::objects::ObjectServices {
    shared: ptr::null(),
    cost: ptr::null(),
    command: ptr::null(),
    company: ptr::null(),
    cargo: ptr::null(),
    town: ptr::null(),
    founding: ptr::null(),
    industry_tiles: ptr::null(),
    strings: crate::objects::Strings {
        too_many: 0,
        no_town: 0,
        edge_sub: 0,
        water: 0,
        edge: 0,
        on_water: 0,
        bridge: 0,
        flat: 0,
        own: 0,
        built: 0,
        limit: 0,
        hq: 0,
        object: 0,
    },
    spec_count: unreachable0,
    spec: unreachable1,
    callbacks: unreachable1,
    colour_policy: unreachable1,
    flags: unreachable1,
    footprint: unreachable1,
    clearance: unreachable1,
    area_shape: unreachable1,
    admission: unreachable1,
    generation: unreachable1,
    can_allocate: unreachable0,
    allocate: unreachable4,
    object_type: unreachable1,
    loop_policy: unreachable1,
    identity: unreachable1,
    location: unreachable1,
    next: unreachable1,
    set_colour: unreachable2,
    recolour: unreachable2,
    colour_error: unreachable2,
    make: unreachable5,
    delete_object: unreachable1,
    query: ptr::null(),
    resolver: ptr::null(),
    slope_error: unreachable3,
    build_price: unreachable1,
    clear_price: unreachable1,
    set_hq: unreachable2,
    statue_town: unreachable1,
    close_inspect: unreachable1,
    progress_set: unreachable2,
    progress_increase: unreachable1,
    normal: unreachable0,
    bulldozer: unreachable0,
    restoration: ptr::null(),
    set_owner: unreachable2,
    scale_size_1d: unreachable1,
    scale_cargo: unreachable1,
};

/// `crate::industry::tile::TileServices` with every entry unreachable.
pub(crate) const INDUSTRY_TILES: crate::industry::tile::TileServices =
    crate::industry::tile::TileServices {
        shared: ptr::null(),
        gfx: unreachable1,
        set_gfx: unreachable2,
        frame: unreachable1,
        set_frame: unreachable2,
        animation_loop: unreachable1,
        set_animation_loop: unreachable2,
        construction_counter: unreachable1,
        construction_stage: unreachable1,
        completed: unreachable1,
        set_completed: unreachable1,
        reset_construction: unreachable1,
        industry: unreachable1,
        industry_index: unreachable1,
        toffee_offset: unreachable1,
        on_water: unreachable1,
        transport: unreachable1,
        build_oilrig: unreachable1,
        pixel_max_z: unreachable1,
        callback: unreachable5,
    };

/// `crate::cargo_payment::Services` with every entry unreachable.
pub(crate) const CARGO: crate::cargo_payment::Services = crate::cargo_payment::Services {
    spec: unreachable2,
    registry: ptr::null(),
    station_company: unreachable1,
    station_town: unreachable1,
    resolver: ptr::null(),
    subsidies: unreachable0,
    industry_delivery: ptr::null(),
    detach_payment: unreachable1,
    finance: ptr::null(),
    vehicles: ptr::null(),
    local_company: unreachable0,
    expense: unreachable2,
    profit: unreachable1,
    shared: ptr::null(),
    feeder_animation: unreachable5,
    subsidy_multiplier: unreachable0,
    feeder_payment_share: unreachable0,
};
