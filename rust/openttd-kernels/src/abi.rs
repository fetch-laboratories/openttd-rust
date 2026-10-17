/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Bounded cross-language layout metadata for the first native 32-bit ABI audit.

use std::mem::{align_of, offset_of, size_of};

macro_rules! layout {
    ($type:ty, $item:expr; $($field:ident),+) => {
        match $item {
            0 => size_of::<$type>(),
            1 => align_of::<$type>(),






        _ => [$(offset_of!($type, $field)),+]
                .get(usize::from($item) - 2)
                .copied()
                .unwrap_or(usize::MAX),
        }
    };
}

// Keep ABI IDs in one registry so duplicate match arms remain compiler-checked.
#[allow(clippy::too_many_lines)]
pub fn layout(type_id: u16, item: u8) -> usize {
    match type_id {
        860 => {
            layout!(crate::engine::Lifecycle, item; company_avail, company_hidden, preview_asked, intro_date, age, reliability, reliability_spd_dec, reliability_start, reliability_max, reliability_final, duration_phase_1, duration_phase_2, duration_phase_3, flags, preview_company, preview_wait)
        }
        861 => layout!(crate::engine::EngineRef, item; shell, state, id, kind),
        862 => layout!(crate::engine::SyncVariant, item; parent, sync),
        863 => layout!(crate::engine::DisplayVariant, item; parent, display),
        864 => layout!(crate::engine::PreviewVariant, item; parent, join),
        865 => layout!(crate::engine::Reliability, item; base_life, retire_early),
        866 => {
            layout!(crate::engine::Startup, item; base_intro, grfid, decay_speed, base_life, climate)
        }
        867 => {
            layout!(crate::engine::SeedConfig, item; base_intro, lifelength, decay_speed, retire_early)
        }
        868 => layout!(crate::engine::AgingYear, item; base_intro, lifelength),
        869 => layout!(crate::engine::Ymd, item; year, month, day),
        870 => layout!(crate::engine::StartSettings, item; generation_seed, starting_year, date),
        871 => layout!(crate::engine::Company, item; shell, id),
        872 => layout!(crate::engine::CompanyPreview, item; blocked, performance),
        873 => layout!(crate::engine::Vehicle, item; shell, id, owner, kind),
        874 => layout!(crate::engine::VehicleCargo, item; engine, cargo),
        875 => layout!(crate::services::RandomSeeds, item; random, interactive),
        876 => layout!(crate::services::RandomServices, item; save, restore, set_seed),
        877 => {
            layout!(crate::engine::Services, item; shared, rng, next, resolve, sync_variant, display_variant, preview_variant, extra_flags, engine_id, reliability, startup, seed_config, base_intro, aging_year, calendar_year, never_expire, start_settings, train_limit, road_limit, ship_limit, aircraft_limit, ai_train_disabled, ai_road_disabled, ai_ship_disabled, ai_aircraft_disabled, date_to_ymd, ymd_to_date, enabled, climate, rail_kind, clear_last_variant, refresh_type, refresh_lists, company_next, company_preview, vehicle_next, vehicle_cargo, can_carry, refit_mask, update_railtypes, update_roadtypes, date_company_types, company_valid, compatible_railtypes, powered_roadtypes, company_railtypes, company_roadtypes, company_engine_count, block_preview, introduces_railtypes, introduces_roadtypes, set_railtypes, set_roadtypes, current_company, local_company, toolbar_enable, toolbar_release, close_preview, invalidate_preview, offer_preview, available_event, available_news, no_engines, screen_dirty, utf8_length)
        }
        831 => {
            layout!(crate::signal::SetTrace, item; duplicate_count, duplicate_pop, full_failed, overflow_full, first_after_remove, refill_succeeded, refill_pop, drained_count, overflow_drained, overflow_reset)
        }
        830 => match item {
            6 => {
                let _: fn(&crate::signal::Context) -> &std::ffi::c_uint = |context| &context.log_x;
                std::mem::size_of::<std::ffi::c_uint>()
            }
            _ => layout!(crate::signal::Context, item; shared, rail, native, log_x),
        },
        824 => layout!(crate::services::RailTraversal, item; depot, tracks),
        825 => {
            layout!(crate::services::RailSignalPresence, item; signals, track, kind, forward, reverse)
        }
        826 => layout!(crate::services::RailSignalUpdate, item; kind, reservations),
        827 => layout!(crate::services::RailStationAdmission, item; rail, owner, axis, blocked),
        828 => layout!(crate::services::RailCrossingAdmission, item; crossing, owner, axis),
        829 => layout!(crate::services::RailWormholeAdmission, item; owner, transport, direction),
        822 => {
            layout!(crate::signal::Snapshot, item; tbu_count, tbd_count, global_count, overflow, last_owner, tbu, tbd, global)
        }
        823 => layout!(crate::signal::SignalItem, item; tile, dir),
        820 => {
            layout!(crate::signal::SignalServices, item; assertions, assert_condition, assert_wormhole_start, train_on_tile, train_on_tracks, overflow_tbu, overflow_tbd, overflow_global, assert_company)
        }
        821 => {
            layout!(crate::services::RailMapServices, item; traversal, signal_presence, signal_update, station_admission, crossing_admission, wormhole_admission, is_rail_depot, rail_depot_direction, has_signal_trackdir, signal_state, set_signal_state, other_tunnel_bridge_end, rail_track_status)
        }
        445 => layout!(crate::town::YearlyServices, item; completed, age, set_age),
        444 => layout!(crate::town::SuppliedEntry, item; entry, created),
        443 => {
            layout!(crate::town::AuthorityServices, item; council_tolerance, bulldozer, authority_distance, town_index, refusal)
        }
        621 => layout!(crate::services::CommandServices, item; landscape_clear),
        620 => {
            layout!(crate::services::CommandCostServices, item; zero, error, expense_money, error_param, add, move_cost, amount, success, error_message, money)
        }
        210..=214 | 400..=417 => crate::company::abi_layout(type_id, item),
        320 => layout!(crate::ship_control::Position, item; x, y, old_tile, new_tile),
        321 => {
            layout!(crate::ship_control::Leaves, item; depot_dir, depot_axis, is_depot, depot_index, wait_unbunch, chain_depot, servint, needs_service, max_distance, tile_valid, tile_type, water_class, lock_middle, lock_dir, tile_min_z, tile_max_z, track_status, offset, diag_between, dist_square, dist_manhattan, docking, dock, dock_water, station, industry_station, oilrig, station_use, station_xy, station_contains, station_dock, station_visits, visit_set, arrival, service, leave_unbunch, path_result, order_free, order_dummy, order_depot, order_leave, order_increment, timetable, position, start_dirty, depot_dirty, depot_invalidate, ships_dirty, details_dirty, age, economy_age, decrease_value, check_breakdown, check_orders, running_cost, cost_divisor, pay_running, speed_default, age_default, speed_frac, speed_property, age_property, update_visual, cache_invalidate, capacity, sprite_direction, tile_x, tile_y, build_flag, build_random, new_position, exit_dir, track_direction, tracks_reach, busy_tile, path_size, path_back, path_pop, path_clear, enter_tile, enter_depot, process_orders, loading, begin_loading, breakdown, viewport, base_viewport, visual, cache, play_sound, yapf_reverse, yapf_choose, update_delta, build_owner, build_z, build_properties, build_dates, build_acceleration, build_prototype, build_interval_percent, build_capacity, set_tile, set_x, set_y, set_z, set_direction, set_speed, set_tick, set_running, set_day, set_order_time, set_progress, set_last_station, set_hidden, set_max_speed, set_cargo_age, set_dest, tile, dest, x, y, z, direction, speed, tick, running, day, order_time, progress, status, owner, last_station, order_destination, order_type, order_max_speed, acceleration, max_speed, state_owner, patch, neighbours, next_depot)
        }
        322 => layout!(crate::ship_control::ReverseResult, item; reverse, trackdir),
        323 => layout!(crate::ship_control::Depot, item; id, tile, owner, ship),
        324 => layout!(crate::ship_control::TrackResult, item; track, found),
        325 => layout!(crate::ship_control::DepotResult, item; tile, id, valid),
        0 => {
            layout!(crate::IntegerResult, item; value_bits, length, error_offset, error_length, error_kind)
        }
        1 => layout!(crate::Utf8Encoded, item; bytes, length),
        2 => layout!(crate::Utf8Decoded, item; length, codepoint),
        3 => layout!(crate::LittleEndian, item; bytes),
        4 => layout!(crate::FormattedInteger, item; bytes, length),
        5 => layout!(crate::AlternatingState, item; position, next_after, current_after),
        6 => layout!(crate::AlternatingStep, item; state, movement),
        7 => layout!(crate::ConsumerBound, item; length, position, shortfall),
        8 => layout!(crate::ConsumerByte, item; value_bits, length),
        9 => layout!(crate::ConsumerMatch, item; length, matched),
        10 => layout!(crate::ConsumerSeparator, item; result_length, consumed_length),
        11 => layout!(crate::EncodedDescriptor, item; value, bytes, length, kind),
        12 => {
            layout!(crate::EncodedView, item; bytes, length, diagnostic_count, assertion_offset, assertion_length, assertion_codepoint, apply, assertion_kind)
        }
        13 => layout!(crate::EncodedDiagnostic, item; offset, length, tail_length, kind),
        14 => {
            layout!(crate::SpiralState, item; max_radius, extent, cur_radius, position, x, y, direction)
        }
        15 => layout!(crate::byte_strings::TrimResult, item; offset, length),
        16 => {
            layout!(crate::HistoryDescriptor, item; child, periods, records, first, last, division, total_division, child_periods, child_division)
        }
        17 => layout!(crate::HistoryStep, item; kind, count, first, last, target, token, value),
        18 => {
            layout!(crate::station_cargo::Collector, item; amount, previous, last_key, other, origin, selector, finalized)
        }
        19..=21 => crate::crypto_primitives::abi_layout(u8::try_from(type_id).unwrap(), item),
        22 => crate::blake2b::abi_layout(item),
        23 => layout!(crate::packet::State, item; limit, position),
        24 => layout!(crate::packet::Frame, item; message, payload),
        25 => crate::x25519::abi_layout(item),
        26 => layout!(crate::string_validation::Step, item; consumed, output, stopped),
        27 => layout!(crate::string_validation::Write, item; position, accepted),
        29 => layout!(crate::script_list::control::Input, item; a, b, flag, kind),
        30 => layout!(crate::script_list::control::Action, item; a, kind),
        34 => {
            layout!(crate::linkgraph::InputNode, item; supply, demand, station, x, y, edge_begin, edge_count)
        }
        35 => layout!(crate::linkgraph::InputEdge, item; capacity, travel_time, dest),
        36 => {
            layout!(crate::linkgraph::Settings, item; accuracy, demand_distance, demand_size, saturation, distribution, express, map_max_x, map_max_y, runtime)
        }
        37 => {
            layout!(crate::linkgraph::OutputShare, item; node, origin, via, cumulative, unrestricted, has_share)
        }
        38 => {
            layout!(crate::trees::TreeServices, item; plant_observation, tree_observation, tree_count, clear_observation, snow_line, sin, cos, make_tree, set_ground_density, add_count, add_growth, set_growth, make_clear, make_shore, make_snow, clear_neighbour_flooding, tile_loop_water, ambient, play_sound, progress, progress_total, clear_square, town_rating, command_begin, command_tile, command_next, command_debit, landscape_clear)
        }
        28 => {
            layout!(crate::trees::PlantObservation, item; tile_type, bridge, ground, density, snow, coast, zone)
        }
        31 => {
            layout!(crate::trees::TreeObservation, item; ground, density, species, count, growth, zone)
        }
        386 => layout!(crate::trees::ClearObservation, item; count, species),
        32 => {
            layout!(crate::trees::LoopSettings, item; tick_counter, size_x, climate, extra, ambient)
        }
        33 => layout!(crate::trees::TickSettings, item; tick_counter, size, climate, extra),
        43 => {
            layout!(crate::trees::GenerateSettings, item; size, size_x, size_y, climate, placer, height_limit, editor, freeform_edges)
        }
        57 => {
            layout!(crate::trees::CommandSettings, item; build_price, size, climate, editor, execute, company_valid)
        }
        58 => layout!(crate::trees::CommandResult, item; cost, status, message),
        59 => layout!(crate::trees::LandscapeClear, item; cost, failed),
        39 => {
            layout!(crate::effect::View, item; x, y, z, sprite, progress, spritenum, subtype, ambient)
        }
        40 | 83 | 232 | 235 | 502 => 0, // Retired effect/road observations and capacity subset.
        41 => {
            layout!(crate::effect::Leaves, item; observe, write, viewport, sound, animated)
        }
        42 => {
            layout!(crate::services::Services, item; context, random, industry, tile_type, bridge_above, tropic_zone, tile_z, tile_slope, mark_dirty, set_tropic_zone, tile_owner, station_index, depot_index, is_level_crossing, tunnel_bridge_direction, bridge_speed, is_water_tile, company_valid, next_company, current_company, closest_town, town_owner, tile_valid, distance_from_edge, tile_max_z, tile_height, bridge_axis, has_water_ground, is_sea, is_plain_rail, is_bridge, tunnel_transport, other_tunnel_end, is_road_depot, road_depot_direction, is_normal_road, disallowed_road_directions, is_bay_stop, bay_stop_direction, is_drive_through_stop, drive_through_axis, is_any_road_stop, road_type, has_road_type, road_owner, town_index, road_bits, snow_line, calendar_year, map_log_x, map_max_x, map_max_y, plane_speed, no_jetcrash, plane_crashes, service_at_helipad, disaster_sound, economy_date, calendar_date, slope)
        }
        // ABI reservations: train 380-385; tree additions 386-389; fleet 390-399; company 400-429; town 430-459; shared vehicle/road 460-499; station 500-559; industry 560-619; shared CommandCost 620-629 (town-owned). Trees retain existing low IDs.
        461 => layout!(crate::vehicle::NewPosition, item; x, y, tile),
        462 => layout!(crate::vehicle::Breakdown, item; broken, status),
        460 => {
            layout!(crate::vehicle::VehicleServices, item; write_counters, handle_breakdown, waiting_for_unbunching, leave_unbunching_depot, reset_depot_unbunching, service_in_depot, needs_automatic_servicing, enter_depot, pathfinding_result, new_position, move_then_status, base_viewport, property, length_callback, unknown_length_result, length_changed, play_start_sound, play_sound, large_explosion, order_free, order_dummy, start_stop_dirty, destroy, age, economy_age, decrease_value, check_breakdown, check_orders, write_day, write_direction, write_speed, last_station, write_last_station, write_dest, write_progress, write_max_speed, previous_tile, detach_last, next, engine_id, engine_for_length, write_tick, direction, status, tile, progress, dest_tile, order_type, order_destination, order_nonstop, subtype, x, y, z, tick, owner, speed, subspeed, running, order_time, vehicle_type, write_x, write_y, write_z, write_tile, write_status, write_subspeed, write_breakdown_counter, write_running, write_order_time, write_profit, write_economy_service, write_calendar_service, write_breakdowns, write_reliability, write_cargo_age, coordinates, xy, speed_inputs, direction_progress, tick_status, day_cost, make_dummy, loading, breakdown, check_orders_only, write_coordinates, altitude_inputs, maintenance)
        }
        464 => layout!(crate::vehicle::Moved, item; old_z, z, cur_speed, max_track_speed),
        465 => layout!(crate::vehicle::Speed, item; distance, cur_speed),
        466 => layout!(crate::vehicle::LastSpeed, item; status),
        467 => layout!(crate::vehicle::Crash, item; victims, front),
        463 => {
            layout!(crate::vehicle::GroundServices, item; cache_begin, move_incline, update_viewport_then_next, turn, do_update_speed, set_last_speed, acceleration, crash, first_cargo_changed, go_to_depot_service, write_first_engine, write_length, cached_length, has_articulated)
        }
        80 => layout!(crate::road::PathElement, item; trackdir, tile),
        81 => {
            layout!(crate::road::Entry, item; acceleration_model, road_side, queue, map_log_x)
        }
        215 => {
            layout!(crate::road::TickRead, item; tick, front)
        }
        216 => {
            layout!(crate::road::MaxSpeedRead, item; next, next_state, tile, max_track_speed, order_max_speed, direction, status)
        }
        217 => layout!(crate::road::SpeedRead, item; max, x, y, z, acceleration, length, tram),
        218 => layout!(crate::road::SpeedPart, item; next, next_state, tile, direction, status),
        219 => {
            layout!(crate::road::Step, item; tile, x, y, z, cur_speed, direction, length, front, tram)
        }
        220 => layout!(crate::road::CrashPart, item; next, next_state, tile, z, crossing),
        221 => layout!(crate::road::Entered, item; tile, cur_speed, flags, direction, order_type),
        222 => {
            layout!(crate::road::Candidate, item; vehicle, state, first, first_state, index, x, y, z, first_speed, direction)
        }
        223 => layout!(crate::road::TileVehicle, item; vehicle, first),
        224 => layout!(crate::road::PartRead, item; next, next_state, tile, z, tram),
        225 => layout!(crate::road::PathRead, item; dest, owner, articulated, bus),
        226 => layout!(crate::road::StopRead, item; owner, bus),
        227 => {
            layout!(crate::road::OvertakeRead, item; tile, cur_speed, direction, status)
        }
        228 => layout!(crate::road::RoadTypes, item; before, after),
        468 => layout!(crate::vehicle::PreviousTile, item; tile, exists),
        230 => layout!(crate::road::TrackChoice, item; trackdir, found),
        231 => layout!(crate::road::DepotResult, item; tile, length),
        233 => layout!(crate::road::ArrivalRead, item; had_vehicle_of_type, bus),
        234 => layout!(crate::road::SoundRead, item; sound, tick),
        473 => layout!(crate::vehicle::ViewportNext, item; next, final_progress),
        471 => layout!(crate::road::Link, item; next, next_state),
        472 => layout!(crate::road::LinkTile, item; link, tile),
        470 => layout!(crate::road::FrontTickRead, item; movement, order_time, running, status),
        469 => layout!(crate::vehicle::EngineForLength, item; engine, grf_version),
        236 => layout!(crate::road::CostRead, item; engine, cost_class, cost_factor),
        237 => layout!(crate::road::ServiceRead, item; tile, servint, cur_speed),
        238 => {
            layout!(crate::road::TurnRead, item; tile, breakdown_ctr, direction, order_type, status)
        }
        239 => layout!(crate::road::TrackdirRead, item; tile, direction, status),
        82 => {
            layout!(crate::road::Leaves, item; vehicle, ground, shared, tick_front, max_depot_penalty, process_orders_then_loading, visual_then_speed, max_speed_read, speed_part, step, crash_part, part, link, link_tile, enter_tile, visit_close, visit_trains, visit_tile, has_road, track_status, any_road_bits, tram_bits, is_road_depot_tile, depot_direction, is_normal_road, has_road_works, disallowed_directions, is_bay_stop, bay_direction, is_drive_through, is_station_road_stop, stop_type, has_free_bay, continuation, move_tile, path_read, choose_track, find_depot, can_build_tram, disconnect, stop_read, should_stop, stop_leave, stop_entrance, stop_entrance_busy, arrival_read, arrival_news, begin_loading_at, overtake_read, sound_read, depot_exit, crash_news, shorten_factor, default_max_speed, update_cargo_age, cost_read, price, pay_running, service_read, turn_read, trackdir_read)
        }
        44 => layout!(crate::disaster::State, item; image_override, target, state, flags),
        45 => layout!(crate::disaster::Action, item; kind, id, other, a, b, c, d),
        46 => layout!(crate::water_regions::Patch, item; x, y, label),
        47 => layout!(crate::water_regions::Snapshot, item; edges, labels, patches, aqueducts),
        48 => layout!(crate::water_regions::Leaves, item; tracks, follow, aqueduct, debug),
        49 => {
            layout!(crate::cargo_payment::Spec, item; payment, valid, callback, periods1, periods2)
        }
        50 => {
            layout!(crate::cargo_payment::Fields, item; front, route_profit, visual_profit, visual_transfer)
        }
        60 => layout!(crate::cargo_payment::Payment, item; fields, station),
        51 => {
            layout!(crate::cargo_payment::Services, item; spec, callback, nearby_industry, station_read, industry_read, industry_write, refuses, accept, statistics, monitor, subsidised, industry_effect, vehicle_read, settle, feeder, setting)
        }
        52 => {
            layout!(crate::ship_yapf::Input, item; map_x, map_y, tile, dest_tile, curve90, curve45, max_speed, dest_dirs, reverse_dirs, trackdir, ocean_frac, canal_frac, station, unit_number)
        }
        53 => {
            layout!(crate::ship_yapf::Leaves, item; destination, follow, tile, patch, visit_new, visit_next, visit_destroy, debug)
        }
        54 => layout!(crate::ship_yapf::Follow, item; tile, skipped, dirs, followed),
        55 => layout!(crate::ship_yapf::Tile, item; ships, docking, sea, lock_middle, destination),
        56 => layout!(crate::ship_yapf::Result, item; direction, found, origin, stats),
        630 => layout!(crate::town::TownRef, item; shell, state, id),
        631 => {
            layout!(crate::town::names::NameServices, item; next, render, render_other, utf8_length, random_next)
        }
        632 => layout!(crate::town::founding::Metadata, item; grfid, kind),
        633 => {
            layout!(crate::town::founding::FoundingServices, item; shared, growth, names, cost, allocate, resolve, can_allocate, close_to_town, flat, clear_ground, tree_ground, water_distance, scale_size, scale_land, found_setting, layout_setting, initial_city_size, town_density, custom_number, larger_towns, parameters, metadata, initialize_counts, insert_town, cargo_effect_exists, prepare_rename, name_changed, update_virt, directory_rebuild, directory_resort, update_noise, rebuild_tree, delete_town, assert_random_context, assert_allocation, initialize_cost, sufficient_money, money_deficit, set_result, exchange_generating, nearest_town_roads, founded_news, founded_events, clear_station_names, clear_industry_names, update_station_coords, cached_name, set_progress, increase_progress, no_town_error, town_count, random, edge_error, close_error, unsuitable_error, unique_error, too_many_error, no_space_error)
        }
        432 => {
            layout!(crate::town::SimulationFields, item; valid_history, xy, num_houses, population, radius, goal, ratings, received, noise_reached, time_until_rebuild, statues, have_ratings, unwanted, exclusivity, exclusive_counter, larger_town, layout, townnamegrfid, townnametype, townnameparts)
        }
        433 => layout!(crate::town::Received, item; old_max, new_max, old_act, new_act),
        434 => layout!(crate::town::SuppliedHistory, item; production, transported),
        435 => layout!(crate::town::Supplied, item; cargo, history),
        439 => layout!(crate::town::RoadChoice, item; introduction_date, max_speed, town_build),
        440 => {
            layout!(crate::town::HouseAvailability, item; zones, override_id, class_id, probability, enabled)
        }
        441 => layout!(crate::town::HousePlacement, item; min_year, max_year, flags, extra_flags),
        442 => layout!(crate::town::HouseConstruction, item; extra_flags, population),
        438 => layout!(crate::town::StationActivity, item; time_since_load, time_since_unload),
        437 => layout!(crate::town::GrowthFields, item; counter, rate, funding, road, flags),
        436 => layout!(crate::town::Slots, item; data, size),
        430 => layout!(crate::town::CommandResult, item; cost, success),
        431 => {
            layout!(crate::town::Leaves, item; map_log_x, build_on_slopes, growth_rate, snow_line, editor, generating_world, allow_crossings, climate, highest_snow_line, calendar_year, allow_roads, road_flags, house_availability, house_placement, house_construction, house_flags, house_extra_flags, house_allows_callback, road_info, state, stations, station_activity_owner, station_owner, dirty_authority, dirty_town, foundation_slope, terraform_price, clean_road_bits, set_road_town, exchange_company, house_class, make_house, add_animation, population_changed, nearby_stations, house_count, update_max_pass, next_town, road_mask, build_road, clear_land, terraform_land, build_bridge, build_tunnel, clear_house_land, allow_construction, construction_animation)
        }
        90 => {
            layout!(crate::rail_yapf::Settings, item; max_nodes, firstred, firstred_exit, lastred, lastred_exit, station, slope, curve45, curve90, depot_reverse, crossing, lookahead, p0, p1, p2, pbs_cross, pbs_station, pbs_back, doubleslip, longer, longer_tile, shorter, shorter_tile, firstred_eol)
        }
        91 => {
            layout!(crate::rail_yapf::Tile, item; flags, other_end, station, railtype, tracks, reserved, station_track, tunnel_dir, uphill, flat_ramp, signal_along, signal_against, signal_green, signal_type, oneway)
        }
        92 => {
            layout!(crate::rail_yapf::Follow, item; tile, skipped, min_speed, max_speed, dirs, followed, error, station)
        }
        93 => {
            layout!(crate::rail_yapf::Leaves, item; train, tile, follow, safe, free, compatible_station, platform_length, closest_station, destination_dirs, origin, write, output, debug)
        }
        94 => {
            layout!(crate::rail_yapf::Input, item; context, settings, map_x, tile, max_cost, desync, kind, td, override_railtype, forbid90, reserve)
        }
        95 => {
            layout!(crate::rail_yapf::Step, item; tile, destination, target_tile, best_length, action, td, found, reverse, value, target_td, target_okay)
        }
        97 => {
            layout!(crate::rail_yapf::Train, item; compatible, all_compatible, tile, rear_tile, virtual_tile, rear_virtual_tile, dest_tile, length, speed, order_destination, td, rear_td, wormhole, rear_wormhole, order, nearest_depot, complex_waypoint)
        }
        100 => {
            layout!(crate::road_yapf::Input, item; map_x, map_y, tile, dest_tile, max_nodes, slope, crossing, stop, occupied, bay, curve, display_speed, order_destination, order_speed, order_type, bus, articulated, trackdir)
        }
        101 => {
            layout!(crate::road_yapf::Tile, item; occupied, length, station, r#type, station_type, depot, depot_dir, crossing, waypoint, drive_through, continuation, busy_bays)
        }
        102 => {
            layout!(crate::road_yapf::Follow, item; tile, skipped, max_speed, min_speed, dirs, followed)
        }
        103 => {
            layout!(crate::road_yapf::Area, item; tile, width, height, valid, stop, drive_through, next)
        }
        104 => layout!(crate::road_yapf::Leaves, item; tile, follow, tracks, height, closest, area),
        105 => {
            layout!(crate::road_yapf::Result, item; tile, cost, direction, found, rounds, open, closed, calcs, distance)
        }
        260 => {
            layout!(crate::orders::Order, item; kind, flags, destination, refit, wait, travel, speed)
        }
        261 => {
            layout!(crate::orders::Consist, item; time, lateness, start, last_departure, next_departure, round_trip, real, implicit, flags)
        }
        262 => layout!(crate::orders::VehicleOrders, item; current, orders, next, previous),
        263 => layout!(crate::orders::List, item; manual, vehicles, first, timetable, total),
        264 => layout!(crate::orders::Backup, item; user, tile, group, clone),
        265 => {
            layout!(crate::orders::Leaves, item; vehicle, consist, list, vector, backup, backup_vector, backup_consist, first_vehicle, last_station, ownerless_station, vehicle_type, vehicle_status, tick_counter, primary_vehicle, vehicle_ownership, ticks_per_second, unit_number, economy_date, economy_fraction, maximum_date, timetable_year_limit, stopped_or_crashed, allocate_list, suppress_implicit, shared_window, vehicle_id, percent_filled, reliability, engine_reliability, display_speed, age_years, needs_service, remaining_years, airport_tile, base_station_tile, station_tile, depot_tile, distance, station_location, destination_tile, aircraft_flying, target_airport, waypoint_tile, at_station, tile_station, ship_station_tile, valid_station, station_owner, can_use_station, owner_check, station_error, has_hangar, valid_depot, depot_owner, rail_depot, road_depot, ship_depot, valid_waypoint, waypoint_facilities, waypoint_owner, list_capacity, next_backup, next_vehicle, aircraft_range, aircraft_range_square, bus, backup_capacity, create_backup, networking, network_server, network_client, server_client, default_group, vehicle_tile, vehicle_group, unique_backup_name, backup_hangar, review_setting, local_owner, day_counter, fast_aircraft, short_strip, no_jet_crash, append_station, invalidate_station_list, command_error, timetable_dirty, invalidate_order, vehicle_dirty, delete_order_news, suppress_implicit_write, invalidate_vehicle_list, close_shared_window, invalidate_shared_window, last_station_write, dirty_vehicle_windows, capture_backup_metadata, clear_backup_name, restore_backup_metadata, order_news, debug_list, assert_departure_range, delete_list, leave_station, reverse_train, next_airport, set_destination, closest_depot, share_command, group_command, delete_backup, clear_backup_gui, clear_backup_post, missing_aircraft_orders, change_timetable_command)
        }
        266 => layout!(crate::orders::Closest, item; tile, destination, reverse, found),
        515 => layout!(crate::station_service::LoadingStation, item; native, base, state),
        500 => layout!(crate::station_service::RatingCounts, item; available, destinations),
        501 => layout!(crate::station_service::StationExclusivity, item; counter, exclusive),
        503 => layout!(crate::station_service::CargoCapacity, item; owner, capacity, cargo),
        504 => {
            layout!(crate::station_service::CargoReservation, item; mask, reserved, capacity, cargo)
        }
        505 => layout!(crate::station_service::CargoStored, item; stored, capacity),
        506 => {
            layout!(crate::station_service::DeliveryRemaining, item; remaining, deliver, capacity)
        }
        507 => {
            layout!(crate::station_service::LoadEngine, item; engine, amount)
        }
        508 => layout!(crate::station_service::LoadingWait, item; lateness, wait),
        509 => layout!(crate::station_service::OrderDestination, item; destination, is_station),
        510 => layout!(crate::station_service::TruncatedSource, item; amount, source, present),
        511 => layout!(crate::cargo_storage::Source, item; id, source_type),
        512 => {
            layout!(crate::station_service::LoadingControl, item; ticks, vehicle_type, unload_type, load_type, refit_cargo, refit, full_load, stop, gradual, improved)
        }
        513 => {
            layout!(crate::station_service::LoadingPart, item; capacity, cargo, vehicle_type, unloading)
        }
        514 => {
            layout!(crate::station_service::LoadingStats, item; stored, load_count, speed, age, capacity, cargo)
        }
        110 => {
            layout!(crate::station_service::CargoFields, item; max_waiting_cargo, status, time_since_pickup, rating, last_speed, last_age, amount_fract)
        }
        111 => {
            layout!(crate::station_service::Fields, item; always_accepted, delete_ctr, time_since_load, time_since_unload, last_vehicle_type)
        }
        112 => {
            layout!(crate::station_service::World, item; station, station_owner, station_exclusivity, station_next, rating_counts, get, owner, pool_size, tick_counter, editor, facilities, in_use, is_station, local_owner, rectangle_empty, town_statue, select_goods, rating_cheat, available, cargo_valid, rating_callback_enabled, passenger_cargo, packet_available, rating_redraw, set_demand, acceptance_news, acceptance_redraw, delete_station, acceptance_animation, watched_animation, append_packet, update_supply, station_list_invalidation, new_cargo_animation, waiting_redraw, rating_callback, with_tiles, tile_next, accept_tile, acceptance_amounts, with_truncation, truncate_next, shared)
        }
        113 => {
            layout!(crate::station_service::Edge, item; destination, last_update, distance)
        }
        114 => {
            layout!(crate::station_service::Links, item; graph, automatic_distribution, node, edge_count, date, station_id, edge_last_update, last_compression, edge, edge_unrestricted, edge_restricted, delete_flows, restrict_edge, restrict_flows, release_edge, remove_edge, compress, order_list, order_pool_size, order_count, order_destination, order_vehicle, next_vehicle, next_shared, stopped_in_depot, last_service, cargo_type, refresh, reroute)
        }
        115 => {
            layout!(crate::station_service::Loading, item; control, part, set_refit_capacity_and_stats, wait_observation, load_engine, grf_version, engine_load_callback, engine_no_multiplier, delivery_remaining, cargo_stored, cargo_reservation, cargo_capacity, vehicle_type, cargo_type, cargo_cap, stored_count, unload_count, deliver_count, transfer_count, load_count, unload_type, has_orders, load_unload_ticks, status, has_articulated_part, is_articulated_part, is_rear_dualheaded, is_normal_aircraft, is_multiheaded, cargo_multiplier, cached_total_length, full_load_order, vehicle_owner, passenger_cargo, station_length_penalty, current_company, vehicle, total_count, order_time, refit_mask, set_cargo_unloading, set_loading_finished, set_stop, set_load_ticks, set_speed, subtract_profit, unknown_load_result, refresh_capacity, vehicle_dirty, update_indicator, set_current_company, next_vehicle, next_articulated, other_multiheaded, first_engine_part, first_vehicle, previous_vehicle, with_next_stations, stage, return_cargo, unload_cargo, load_cargo, reserve_cargo, deliver_to_transfer, deliver_to_keep, has_cargo_for, platform_length, vehicle_tile, station_index, station_cargo_total, load_property, load_callback, get_payment, create_payment, delete_payment, flush_delivery, vehicle_new_cargo, cargo_taken, train_loads, road_loads, vehicle_empty, station_dirty, test_refit, execute_refit)
        }

        586 => layout!(crate::industry::LocationResult, item; callback, registers),
        579 => layout!(crate::industry::ClearArea, item; first_tile, area),
        580 => layout!(crate::industry::ClearAreas, item; data, size),
        581 => layout!(crate::industry::Allocation, item; shell, owner, id),
        582 => layout!(crate::industry::ConflictKinds, item; kinds),
        583 => {
            layout!(crate::industry::ConstructionErrors, item; site, forest, positioned, desert, rainforest, lowland, one_per_town, town, population, near_town, conflict, too_many)
        }
        584 => {
            layout!(crate::industry::TransactionServices, item; enabled, layout_count, layout, raw, raw_setting, check_proc, conflicting, multiple_per_town, closest_town, town, population, town_position, industry, highest_snow_line, edge_distance, refinery_limit, tile_height, has_water_class, ensure_no_vehicle, set_company, temporary_shell, resolve_location, resolve_shape, location_error, shape_error, tile_grf_version, shape_callback, terragenesis, ignore_restrictions, platform, terraform, can_allocate, allocate, prospecting_chance, local_company, prospect_no_place, prospect_unlucky, construction_cost)
        }
        585 => {
            layout!(crate::industry::TransactionContext, item; map, builder, production, generation, construction, lifecycle, tiles, tile_policy, commands, costs, transaction, errors, clear_areas, restore_clear_areas)
        }
        578 => {
            layout!(crate::industry::TilePolicyServices, item; current_company, magic_bulldozer, object_in_way, slopes_refused, close_ai, close_game, destroy, removal_cost, autoslope_enabled, tile_max_z, autoslope_callback, callback_boolean, foundation_cost, landscape_clear)
        }
        577 => {
            layout!(crate::industry::IndustryCostServices, item; zero, error, expense_money, success)
        }
        576 => {
            layout!(crate::industry::IndustryCommandServices, item; deity, get, owner, valid_company, invalidate_view)
        }
        575 => {
            layout!(crate::industry::TileAcceptance, item; cargo, amount, accepts_all, accepts_callback, amount_callback)
        }
        573 => layout!(crate::industry::TileIndustry, item; shell, owner),
        574 => {
            layout!(crate::industry::TileServices, item; gfx, set_gfx, frame, set_frame, animation_loop, set_animation_loop, delete_animated, tick_counter, ambient, animate_new, construction_counter, construction_stage, completed, set_completed, reset_construction, industry, industry_index, toffee_offset, on_water, water_loop, transport, anim_production, anim_next, build_oilrig, pixel_max_z, create_effect, create_effect_above, set_effect_substate, spec, acceptance, callback, cargo_translation)
        }
        572 => layout!(crate::services::ByteSpan, item; data, size),
        568 => {
            layout!(crate::industry::CargoLists, item; produced, accepted)
        }
        569 => {
            layout!(crate::industry::LayoutTile, item; x, y, gfx)
        }
        570 => {
            layout!(crate::industry::ConstructionRequest, item; layout, town, layout_index, tile, size_x, size_y, id, random_bits, kind, founder)
        }
        571 => {
            layout!(crate::industry::ConstructionServices, item; cargo_lists, callback_mask, set_town, calendar_date, generating, callback, callback_error, grf_version, cargo_translation, layout_size, layout_tile, water_class, clear_tile, make_industry, set_construction_counter, set_construction_stage, translated_gfx, animation_enabled, add_animated, build_window_dirty, serve_neutral, insert_near_station, clear_near_industries, insert_neutral_industry, stations_around, add_delivery)
        }
        566 => layout!(crate::industry::GenerationInfo, item; appear, enabled, layouts),
        567 => {
            layout!(crate::industry::GenerationServices, item; spec, spec_behaviour, info, coastal, editor, density, custom_number, scale_by_size_1d, scale_by_land_proportion, set_progress, increase_progress, missing_warning)
        }
        560 => {
            layout!(crate::industry::Metadata, item; tile, width, height, construction_date, random, kind, owner, random_colour, founder, construction_type, selected_layout, exclusive_supplier, exclusive_consumer)
        }
        561 => layout!(crate::industry::CleanupTile, item; industry, kind, oilrig),
        562 => layout!(crate::industry::Reseed, item; own, parent, used),
        563 => layout!(crate::industry::RandomisationSpec, item; gfx, enabled),
        564 => layout!(crate::industry::AnimationSpec, item; spec, triggers),
        565 => {
            layout!(crate::industry::LifecycleServices, item; neutral_station, cleanup_tile, delete_tile_inspect, make_water, delete_oilrig, clear_docking, industry_field, disassociate_field, release_disasters, delete_storage, delete_news, close_view, close_production, delete_inspect, delete_subsidies, invalidate_cargo, next_near_station, remove_delivery, belongs, randomisation_spec, random_triggers, set_random_triggers, resolve_reseed, random_bits, set_random_bits, animation_spec, change_frame)
        }
        130 => {
            layout!(crate::industry::Fields, item; valid_history, last_prod_year, counter, prod_level, was_cargo_delivered, ctlflags)
        }
        131 => {
            layout!(crate::industry::BuildFields, item; probability, min_number, target_count, max_wait, wait_count)
        }
        132 => {
            layout!(crate::industry::BuilderFields, item; builddata, wanted_inds, daily_counter, daily_increment, sound_tile, sound_ctr)
        }
        133 => layout!(crate::industry::Slots, item; data, size),
        134 => layout!(crate::industry::Produced, item; cargo, waiting, rate, history),
        135 => {
            layout!(crate::industry::Accepted, item; cargo, waiting, accumulated_waiting, last_accepted, history)
        }
        136 => {
            layout!(crate::industry::Observation, item; owner, behaviour, callbacks, life, original, kind, up_text, down_text, closure_text)
        }
        137 => {
            layout!(crate::industry::Services, item; shared, next_tick, sound_count, behaviour, location, industry_sound, play_sound, special_effect, tick_trigger, production_callback, scale_cargo, move_goods, tile_add_wrap, farm_tile, snow_line, make_field, fence_wanted, set_fence, tile_completed, harvest, observe, production_rate, change_callback, custom_text, news, rate_news, callback_error, set_dirty, destroy, advertise, create, random_industry, get, type_count, total, type_info, probability_callback, scale_by_map_size, company_none, restore_company, directory_dirty, recession, economy_month, economy_year, days_since_last_month, landscape, economy_type, passengers, fund_only, calendar_year, deity)
        }
        138 => {
            layout!(crate::industry::ProductionResult, item; subtract, add, again, cargo_input, cargo_output, version, num_input, num_output, present)
        }
        139 => layout!(crate::industry::Map, item; size_x, size_y, landscape),
        140 => {
            layout!(crate::industry::TickRecord, item; counter, map, interval, ambient, editor)
        }
        141 => layout!(crate::industry::Entry, item; industry, owner, id, callbacks),
        142 => layout!(crate::industry::Location, item; tile, width, height),
        143 => layout!(crate::industry::FarmTile, item; kind, snow, ground, grown),
        144 => layout!(crate::industry::Change, item; result, reg),
        145 => {
            layout!(crate::industry::TypeInfo, item; behaviour, enabled, layouts, appear)
        }

        180 => {
            layout!(crate::aircraft::State, item; cached_max_range_sqr, cached_max_range, cache_padding, crashed_counter, targetairport, pos, previous_pos, state, last_direction, number_consecutive_turns, turn_counter, flags)
        }
        181 => layout!(crate::aircraft::Vehicle, item; handle, state, id),
        182 => layout!(crate::aircraft::Node, item; next, blocks, position, next_position, heading),
        183 => layout!(crate::aircraft::Moving, item; x, y, flags, direction),

        185 => {
            layout!(crate::aircraft::Leaves, item; hangar_candidate, geometry, layout_blocks, layout_heading, layout_next_position, layout_next, layout_route, vehicle, shared, next, station, airport_tile, station_tile, rotation, airport_type, station_owner, has_hangar, airport_fta, airport_blocks, had_vehicle, dummy_airport, airport_elements, helipads, airport_flags, airport_delta_z, node, fta, moving, engine_speed, engine_subtype, engine_sound, engine_reliability, airport_entry, direction_towards, hangar_height, terminal_count, hangar_exit, can_use_station, service_interval, chain_in_depot, nearest_depot_order, part_of_orders, next_station, next_aircraft, update_position, rotor_image, copy_sprite, position_viewport, create_effect, truncate_cargo, crash_news, station_rating, landing_rating, dirty_depot, first_arrival, begin_loading, dirty_details, update_delta, touchdown_animation, destination_too_far, delete_range_news, service_order, running_cost, subtract_cost, dirty_lists, next_stopping_station, remove_depot_orders, assert_flying, invalid_movement, invalid_position, invalid_scheme, unreachable, cargo_age_property, range_property, touchdown_sound, rotor_image_if_changed, update_rotor_image, process_orders, vehicle_crash, send_to_depot, fta_next_position, block_node, route_node, block_choice)
        }
        186 => layout!(crate::aircraft::BlockNode, item; blocks, position, next_position),
        187 => layout!(crate::aircraft::RouteNode, item; next, next_position, heading),
        188 => layout!(crate::aircraft::BlockChoice, item; next, blocks, heading),
        340 => layout!(crate::train::Handle, item; shell,owner),
        341 => layout!(crate::train::TrainConsistChangedRead, item; engine,front),
        342 => layout!(crate::train::TrainConsistChanged1Read, item; engine,engine_part),
        343 => layout!(crate::train::TrainConsistChanged2Read, item; cargo_cap),
        344 => layout!(crate::train::TrainCurveLimitRead, item; next,direction),
        345 => {
            layout!(crate::train::TrainStopLocationRead, item; length,total_length,order_destination,order)
        }
        346 => layout!(crate::train::TrainCurrentMaxSpeedRead, item; tile,max_track_speed,speed),
        347 => layout!(crate::train::TrainCurrentMaxSpeed6Read, item; next,tile,status),
        348 => layout!(crate::train::TrainUpdateAccelerationRead, item; power,weight),
        349 => layout!(crate::train::TrainUpdateSpeedRead, item; status,acceleration),
        350 => layout!(crate::train::TrainTrackdirRead, item; tile,direction,status),
        351 => layout!(crate::train::TrainCanLeaveRead, item; tile,direction),
        353 => layout!(crate::train::TrainCrossingApproachRead, item; status,front),
        354 => layout!(crate::train::TrainNextOffsetRead, item; next,length),
        355 => layout!(crate::train::TrainAfterSwapRead, item; tile,x,y),
        356 => layout!(crate::train::TrainReverseSwapRead, item; tile,x,y,z,direction,status),
        357 => layout!(crate::train::TrainApproachingEndRead, item; x,y,length,speed,direction),
        358 => layout!(crate::train::TrainLineEndsRead, item; tile,speed,breakdown),
        359 => layout!(crate::train::TrainSpeedZRead, item; z,max_track_speed,speed),
        360 => {
            layout!(crate::train::TrainMoveVehicleRead, item; tile,x,y,direction,front,articulated)
        }
        361 => layout!(crate::train::TrainMoveVehicle20Read, item; speed,status,front),
        362 => layout!(crate::train::TrainCollisionOneRead, item; first,x,y,z,length,owner),
        363 => layout!(crate::train::TrainCollisionOne22Read, item; x,y,z,length,owner),
        364 => layout!(crate::train::TrainDeleteLastRead, item; tile,owner),
        365 => layout!(crate::train::TrainStayDepotRead, item; tile,power),
        366 => layout!(crate::train::TrainLocoRead, item; speed,status,order),
        367 => layout!(crate::train::TrainLoco26Read, item; tile,order_destination,order,nonstop),
        368 => layout!(crate::train::TrainTickRead, item; speed,status,running,front,free_wagon),
        369 => layout!(crate::train::TrainNeedsServiceRead, item; order_destination,order),
        370 => layout!(crate::train::TrainNextForceRead, item; tile,status),
        371 => layout!(crate::train::TrainReverseCommandRead, item; status,breakdown,front),
        372 => {
            layout!(crate::train::Leaves, item; read_first,read_next,read_previous,read_next_unit,read_last,read_tile,read_dest,read_order_time,read_length,read_total_length,read_max_track_speed,read_speed,read_gv_flags,read_refit_cap,read_last_station,read_direction,read_status,read_tick,read_running,read_day,read_progress,read_order,read_front,read_articulated,read_multiheaded,read_owner,read_vis_effect,read_consist_changed,read_consist_changed_1,read_consist_changed_2,read_curve_limit,read_stop_location,read_current_max_speed,read_current_max_speed_6,read_update_acceleration,read_update_speed,read_trackdir,read_can_leave,read_crossing_approach,read_next_offset,read_after_swap,read_reverse_swap,read_approaching_end,read_line_ends,read_speed_z,read_move_vehicle,read_move_vehicle_20,read_collision_one,read_collision_one_22,read_delete_last,read_stay_depot,read_loco,read_loco_26,read_tick_state,read_needs_service,read_next_force,read_reverse_command,write_tile,write_dest,write_x,write_y,write_z,write_direction,write_speed,write_tick,write_running,write_day,write_order_time,write_progress,write_subspeed,write_gv_flags,write_acceleration,write_length,write_total_length,write_first_engine,write_max_speed,write_cargo_cap,write_refit_cap,write_cargo_age,write_last_station,write_colourmap,write_status,acceleration,acc_model,acc_type,advance_distance,age,all_powered,ambient_sound,arrival_news,arrival_triggers,axis_diag,backoff,base_viewport,begin_loading,bridge_speed,cache_override,callback_length,capacity,capacity_error,cargo_age_default,cargo_changed,chain_depot,check_breakdown,check_next,check_orders,check_reverse,choose_track,clear_reservation,compatible_rail_owner,consist_windows,cost_class,cost_default,cost_divisor,count_chain,crash_event,crash_ground,crash_news,crash_rating,crash_sound,crossing_barred,crossing_rail_axis,crossing_reserved,crossing_road_axis,crossing_sound,curve_advantage,curve_mod,day_ticks,decrease_value,delete_vehicle,depot_dir,depot_dirty,depot_index,depot_track,depot_window,diag_axis,diag_between,diag_reaches_tracks,diag_trackdir,dirty_tile,dir_diag,disaster_sound,disconnect,economy_age,engine_power,enter_depot,enter_tile,find_depot,first_track,free_reservation,grf_version,handle_breakdown,has_depot_res,has_reserved,has_signal,has_signals,has_signal_td,hide_fill,inclination,invalidate_grf,invalid_price,is_bridge,is_crossing,is_depot,is_plain_rail,is_railway,is_station,is_station_any,is_tunnelbridge,large_explosion,last_speed,leave_sound,leave_station,leave_unbunch,length_callback,length_changed,length_default,length_error,loading,local_company,lost_warn,map_size,max_depot_penalty,needs_service,no_90,oneway_blocking,order_depot_service,order_dummy,order_free,order_max_speed,order_stop,other_end,pay_running,pbs_signal_type,platform_ahead,platform_length,position,pow_wag_power)
        }
        373 => layout!(crate::train_reservation::TrainReservationFreeRead, item; tile),
        374 => layout!(crate::train_reservation::TrainReservationFree1Read, item; tile,next),
        375 => {
            layout!(crate::train_reservation::TrainReservationNewRead, item; dest,last_station,order_index,suppress)
        }
        376 => {
            layout!(crate::train_reservation::TrainReservationChooseRead, item; tile,dest,destination,order)
        }
        377 => layout!(crate::train_reservation::TrainReservationChoose4Read, item; order,nearest),
        378 => {
            layout!(crate::train_reservation::TrainReservationCheckNextRead, item; tile,dest,destination,order,num_orders)
        }
        379 => {
            layout!(crate::train_reservation::Follow, item; old_tile,new_tile,skipped,dirs,old_td,exitdir,tunnel,bridge,station,error)
        }
        380 => layout!(crate::train_reservation::Pbs, item; tile,other,td,okay),
        381 => {
            layout!(crate::train_reservation::Search, item; track,tile,final_dest,td,found,okay)
        }
        382 => {
            layout!(crate::train_reservation::Leaves, item; read_tile,read_next,read_last_station,read_direction,read_order,read_num_orders,read_order_index,read_free,read_free_1,read_new,read_choose,read_choose_4,read_check_next,all_compat,backoff,bits_track,blocking,check_reverse,compat_station,conditional,copy_order,cross_dirs,cross_tracks,depot_dir,depot_reserved,diag_reach_dirs,diag_track,enter_td,exit_dir,free,green,has_pbs,has_reserved,has_signal,increment_order,is_bridge,is_depot,is_pbs,is_plain,is_railway,is_station,is_tunnel,is_waypoint,line_reverse,mark_bridge,mark_tile,needs_service,oneway,order_service,order_stop,order_type,other_end,overlap,path_result,rail90,reach_dirs,reach_tracks,reserved,reserve_paths,restore_order,safe,save_order,service,set_depot,set_depot_dest,set_platform,set_signal,set_tunnel,show_res,signal_context,vehicle,start_stop,station,station_train,station_xy,stuck,tile_add,tile_offset,trackdir,track_status,try_track,tunnel_dir,tunnel_free,unreserve,write_dest,write_last,write_suppress,follow,origin,pathfind,safe_track,process_orders,update_order_dest)
        }
        383 => layout!(crate::train::Depot, item; tile,length),
        384 => {
            layout!(crate::train::Leaves, item; price,process_orders,property,railveh_wagon,rail_tilt,rail_type,rail_types,reserve_paths,reserve_under,reset_unbunch,reverse_at_signals,reverse_single_blocked,reverse_windows,running_windows,service,servint,set_depot_res,set_next,set_platform_res,set_signal_state,set_tunnel_res,show_effect,show_reservation,signal_context,signal_has_pbs,signal_pbs,signal_type,sigseg_full,sigseg_pbs,small_explosion,speed_default,start_stop_dirty,station,station_axis,station_compatible,station_dest,stopped_in_depot,stop_location,stuck_news,suppress_implicit,ticks_leave_depot,tile_add_diag,tile_offset_axis,tile_offset_diag,tile_owner,tile_rail_type,tile_virt,trackdir_exit,trackdir_reaches,track_bits,track_crosses,track_direction,track_status,train_list,train_visit,truncate_cargo,try_path,try_reserve,tunnel_dir,unreserve,update_delta,update_speed,user_default,veh_exit_dir,viewport,view_window,visit_type,vis_effect,wagon_override,wagon_speed_limits,wait_oneway,wait_pbs,wait_twoway,wait_unbunch,write_crossing_bar,write_crossing_res,write_visit_type,visit_tile,visit_near)
        }
        385 => layout!(crate::train_reservation::Choice, item; track,reserved),
        280 => {
            layout!(crate::cargo_storage::Packet, item; feeder_share, source_xy, count, periods_in_transit, first_station, next_hop, source_id, travelled_x, travelled_y, source_type, in_vehicle)
        }
        281 => {
            layout!(crate::cargo_storage::Fields, item; cargo_periods_in_transit, feeder_share, count, reserved_count, action_counts)
        }
        282 => {
            layout!(crate::cargo_storage::Services, item; can_allocate, create, packet, destroy, random, coordinate, flow, pay, origin, flow_owner, random_draw, packet_next)
        }
        300 => layout!(crate::cargo_flow::Share, item; cumulative, station, found),
        301 => layout!(crate::cargo_flow::Origin, item; flow, origin, found),
        302 => {
            layout!(crate::cargo_flow::Services, item; context, read, job_flows, live_flows, reroute, finish)
        }
        310 => {
            layout!(crate::cargo_storage::Vehicle, item; list, capacity, cargo, train, articulated)
        }
        311 => {
            layout!(crate::cargo_storage::CapacityServices, item; read, next_part, last_engine_part, other_multiheaded_part, cargo)
        }
        390 => {
            layout!(crate::fleet::GroupFields, item; owner, vehicle_type, flags, livery, parent, number)
        }
        391 => {
            layout!(crate::fleet::StatsFields, item; profit, profit_min_age, vehicles, vehicles_min_age, defined, finished)
        }
        392 => layout!(crate::fleet::Renew, item; from, to, next, group, when_old),
        393 => {
            layout!(crate::fleet::GroupServices, item; group, next_group, next_company, next_vehicle, vehicle, vehicle_id, vehicle_type, vehicle_owner, vehicle_group, vehicle_engine, profit, old_enough, primary, countable, ground, front, next_part, first_shared, next_shared, set_membership, invalidate_cache, viewport, stats, head, renew_state, next_renew, renew_id, engine_type, current_company, buildable_type, can_allocate, allocate, use_number, release_number, company_livery, keep_length, delete_group, invalid_parent, clear_backup, remove_rule, remove_vehicles, delete_child, add_to_group, utf8_length, list_dirty, list_set_dirty, colour_dirty, replace_dirty, replace_invalidate, alter_dirty, vehicle_dirty, depot_dirty, close_replace, screen_dirty, list_generate, list_push, list_size, list_at, renew_allocate, renew_can_allocate, renew_delete, recursion_error)
        }
        394 => {
            layout!(crate::fleet::transactions::Services, item; cost_zero, cost_vehicles, cost_error, cost_add, cost_move, cost_amount, success, error, money, ownership, rear, articulated, crashed, stopped, chain_depot, first, next_unit, prev_unit, length, flipped, cargo_type, can_carry, stopped_in_depot, max_length, check, needs_renew, engine_valid, company_valid, engine_buildable, rail_compatible, road_powered, wagon, tram, plane, refit_mask, refit_masks, vehicle_cargo, default_cargo, orders, order_count, order_count_id, order_at, order_refit, order_auto, order_cargo, local, refit_news, build, refit, subtype, reverse_probability, reverse, start_stop, move_vehicle, sell, clone_order, copy_group, copy_configuration, viewports, view_window, news, transfer_cargo, capacity, event, save_rng, restore_rng, rule_window, assertions, unavailable, too_long, too_long_replacement, nothing)
        }
        395 => {
            layout!(crate::fleet::transactions::Costs, item; result, replace, build, copy, temporary, seeds)
        }
        396 => {
            layout!(crate::fleet::pending::Services, item; set_current, restart, x, y, z, reserve, subtract, command, animation, length_news, failed_news, cash, limit)
        }
        670 => layout!(crate::vehicle::Coordinates, item; x, y, z),
        671 => layout!(crate::vehicle::XY, item; x, y),
        672 => {
            layout!(crate::vehicle::SpeedInputs, item; speed, max_speed, acceleration, status, subspeed)
        }
        673 => layout!(crate::vehicle::DirectionProgress, item; direction, progress),
        674 => layout!(crate::vehicle::TickStatus, item; tick, status),
        675 => layout!(crate::vehicle::DayCost, item; profit, running),
        676 => layout!(crate::aircraft::Geometry, item; tile, rotation, width, height, airport),
        677 => layout!(crate::aircraft::LayoutRoute, item; node, next, next_position, heading),
        678 => layout!(crate::vehicle::AltitudeInputs, item; max_speed, direction),
        679 => layout!(crate::aircraft::HangarCandidate, item; owner, airport, hangar),
        740 => layout!(crate::vehicle::scheduler::PoolCursor, item; slots, slots_len, first_unused),
        741 => {
            layout!(crate::vehicle::scheduler::PostTick, item; vehicle_type, period, counter, motion, speed, front_type, subtype, tick, front_status, status)
        }
        743 => layout!(crate::vehicle::scheduler::DayCallback, item; day, has_engine),
        748 => layout!(crate::vehicle::scheduler::StationLookup, item; station, id),
        749 => {
            layout!(crate::vehicle::scheduler::SchedulerServices, item; pool, assertions, tick, calendar_day, economy_day, day_callback_read, day_callback, randomise_callback, first_dirty, unknown_day_callback, cargo_owner, running_sound, periodic_sound, next_station, with_loading_timer, reset_accumulators)
        }
        750 => {
            layout!(crate::vehicle::scheduler::Tables, item; scheduler, cargo, station, loading, pending, transaction, group)
        }
        751 => {
            layout!(crate::vehicle::maintenance::Renewal, item; age, max_age, months, enabled, kind, subtype)
        }
        752 => {
            layout!(crate::vehicle::maintenance::Reset, item; economy_date, calendar_date, reliability, chance, breakdowns)
        }
        753 => {
            layout!(crate::vehicle::maintenance::ResetWrite, item; economy_date, calendar_date, reliability, chance, since)
        }
        754 => {
            layout!(crate::vehicle::maintenance::Interval, item; company, last_service, now, interval, reliability, percent, wallclock, no_service, breakdowns)
        }
        755 => layout!(crate::vehicle::maintenance::Replacement, item; engine, when_old),
        756 => layout!(crate::vehicle::maintenance::Budget, item; needed, available),
        757 => layout!(crate::vehicle::maintenance::ReplacementCost, item; cost, available),
        758 => layout!(crate::vehicle::maintenance::Automatic, item; has_depot_order, order_type),
        759 => {
            layout!(crate::vehicle::maintenance::CheckBreakdown, item; reliability, decay, no_service, breakdowns)
        }
        760 => {
            layout!(crate::vehicle::maintenance::BreakdownGuard, item; speed, counter, status, breakdowns, menu)
        }
        761 => {
            layout!(crate::vehicle::maintenance::BreakdownTrigger, item; reliability, chance, kind, breakdowns)
        }
        762 => layout!(crate::vehicle::maintenance::BreakdownProbe, item; counter, status),
        763 => layout!(crate::vehicle::maintenance::BreakdownStart, item; since, kind),
        764 => layout!(crate::vehicle::maintenance::BreakdownActive, item; kind, tick, delay),
        765 => layout!(crate::vehicle::maintenance::BreakdownCountdown, item; order_type, counter),
        766 => layout!(crate::vehicle::maintenance::BreakdownSoundState, item; kind, landscape),
        767 => layout!(crate::vehicle::maintenance::BreakdownSmoke, item; shell, delay),
        768 => layout!(crate::vehicle::maintenance::Age, item; age, max_age, decay),
        769 => {
            layout!(crate::vehicle::maintenance::Warning, item; enabled, has_previous, local, status)
        }
        770 => layout!(crate::vehicle::maintenance::WarningCompany, item; company, renewal),
        771 => {
            layout!(crate::vehicle::maintenance::Services, item; shared, transaction, group, renewal, reset, reset_write, has_engine, interval, engine_reliability, budget, replacement, replacement_available, replacement_cost, automatic, depot_service, check_breakdown, breakdown_guard, breakdown_trigger, chance_write, trigger, breakdown_counter, breakdown_probe, breakdown_start, breakdown_start_write, breakdown_active, breakdown_countdown, breakdown_sound_state, breakdown_delay_write, breakdown_sound, smoke_disabled, smoke, smoke_animation, aircraft_broken, mark_dirty, view_dirty, details_dirty, age, age_write, decay_write, warning, warning_company, engine_available_any, replacement_active, old_news, economy_age, economy_age_write, reached_min_age, value, value_write, max_calendar_date, max_economy_date, profit_min_age, anniversaries, before_zero, news_getting_old, news_very_old, news_very_old_and)
        }
        772 => match item {
            0 => std::mem::size_of::<crate::services::CalendarDate>(),
            1 => std::mem::align_of::<crate::services::CalendarDate>(),
            2 => std::mem::offset_of!(crate::services::CalendarDate, 0),
            _ => usize::MAX,
        },
        773 => match item {
            0 => std::mem::size_of::<crate::services::EconomyDate>(),
            1 => std::mem::align_of::<crate::services::EconomyDate>(),
            2 => std::mem::offset_of!(crate::services::EconomyDate, 0),
            _ => usize::MAX,
        },
        774 => match item {
            0 => std::mem::size_of::<crate::vehicle::OrderType>(),
            1 => std::mem::align_of::<crate::vehicle::OrderType>(),
            _ => usize::MAX,
        },

        775 => match item {
            0 => std::mem::size_of::<crate::services::LandscapeType>(),
            1 => std::mem::align_of::<crate::services::LandscapeType>(),
            _ => usize::MAX,
        },

        _ => usize::MAX,
    }
}
