add_test_files(
    alternating_iterator.cpp
    alternating_iterator_state.cpp
    abi_layout.hpp
    bitmath_func.cpp
    company_economy.cpp
    cargo_registry.cpp
    cargo_payment.cpp
    cargo_payment_stubs.hpp
    fleet.cpp
    follower_probe.hpp
    newgrf_loading.cpp
    engine.cpp
    effectvehicle.cpp
    effect_protocol.hpp
    enum_over_optimisation.cpp
    flatset_type.cpp
    industry.cpp
    history_func.cpp
    landscape_partial_pixel_z.cpp
    linkgraph_schedule.cpp
    landscape_tile_loop.cpp
    landscape_protocol.hpp
    math_func.cpp
    mock_environment.h
    mock_fontcache.h
    mock_spritecache.cpp
    mock_spritecache.h
    newgrf_bytereader.cpp
    orders.cpp
    rail_yapf.cpp
    rail_yapf_protocol.hpp
    road_yapf.cpp
    roadvehicle.cpp
    vehicle_scheduler.cpp
    session.cpp
    service_tables.cpp
    station_service.cpp
    spiral_iterator_state.cpp
    string_builder.cpp
    string_consumer.cpp
    string_consumer_bytes.cpp
    string_inplace.cpp
    string_func.cpp
    test_main.cpp
    test_network_crypto.cpp
    test_script_admin.cpp
    test_window_desc.cpp
    town_growth.cpp
    town_protocol.hpp
    townname.cpp
    train.cpp
    ship_yapf.cpp
    ship_yapf_protocol.hpp
    water_regions.cpp
    water_regions_protocol.hpp
    world_generation_probe.hpp
    tilearea.cpp
    utf8.cpp
)

# Test oracles come from the immutable baseline, not fork fallback bodies.
file(STRINGS "${CMAKE_SOURCE_DIR}/migration/baseline.json" baseline_line REGEX "\"commit\"")
string(REGEX REPLACE ".*\"commit\": \"([0-9a-f]+)\".*" "\\1" baseline_commit "${baseline_line}")
find_package(Git REQUIRED)
file(MAKE_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/reference")
foreach(header table/roadveh_movement.h pathfinder/yapf/yapf_node_road.hpp)
    get_filename_component(name "${header}" NAME)
    execute_process(
        COMMAND "${GIT_EXECUTABLE}" show "${baseline_commit}:src/${header}"
        WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
        OUTPUT_FILE "${CMAKE_CURRENT_BINARY_DIR}/reference/reference-${name}"
        RESULT_VARIABLE oracle_result
    )
    if(NOT oracle_result EQUAL 0)
        message(FATAL_ERROR "Fetch migration/baseline.json commit before configuring native tests")
    endif()
endforeach()
target_include_directories(openttd_test PRIVATE
    "${CMAKE_CURRENT_BINARY_DIR}/reference"
    "${CMAKE_SOURCE_DIR}/src/pathfinder/yapf"
)
