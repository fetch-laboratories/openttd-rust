file(SHA256 "${CONFIGURATION}" CONFIGURATION_HASH)
set(PREVIOUS "${CONFIGURATION}.successful")
set(PREVIOUS_HASH "")
if(EXISTS "${PREVIOUS}")
    file(READ "${PREVIOUS}" PREVIOUS_HASH)
endif()
if(NOT CONFIGURATION_HASH STREQUAL PREVIOUS_HASH)
    # Cargo can otherwise reuse an archive after an SDK/environment-only change.
    execute_process(COMMAND "${CARGO}" clean --package openttd-kernels --release
        --target "${TARGET}" --manifest-path "${SOURCE}/Cargo.toml"
        WORKING_DIRECTORY "${SOURCE}" OUTPUT_VARIABLE CLEAN_OUTPUT
        ERROR_VARIABLE CLEAN_ERROR RESULT_VARIABLE CLEAN_RESULT)
    if(NOT CLEAN_RESULT EQUAL 0)
        message(FATAL_ERROR "Cannot invalidate changed Rust configuration: ${CLEAN_OUTPUT}${CLEAN_ERROR}")
    endif()
endif()
set(FEATURE_ARGS "")
if(FEATURES)
    list(APPEND FEATURE_ARGS --features "${FEATURES}")
endif()
# JSON messages name the build script's OUT_DIR, also when Cargo reuses the archive; compiler
# messages, native-static-libs among them, stay rendered on stderr.
execute_process(COMMAND "${CARGO}" rustc --package openttd-kernels --release --locked
    --message-format=json-render-diagnostics
    --target "${TARGET}" --manifest-path "${SOURCE}/Cargo.toml" ${FEATURE_ARGS} -- --print=native-static-libs
    WORKING_DIRECTORY "${SOURCE}" OUTPUT_VARIABLE OUTPUT ERROR_VARIABLE ERROR RESULT_VARIABLE RESULT)
file(WRITE "${LOG}" "${CLEAN_OUTPUT}${CLEAN_ERROR}${OUTPUT}${ERROR}")
if(NOT RESULT EQUAL 0)
    message(FATAL_ERROR "Rust archive build failed: ${OUTPUT}${ERROR}")
endif()
message(STATUS "${ERROR}")
if(TABLE_STRINGS)
    # Publish the table::strings source this archive compiled for the string-ID check, on every
    # run, so a missing copy is recreated even when Cargo reuses the archive.
    string(REGEX MATCH "\"reason\":\"build-script-executed\",\"package_id\":\"[^\"]*openttd-kernels[^\"]*\"[^\n]*\"out_dir\":\"([^\"]*)\""
        OUT_DIR_MATCH "${OUTPUT}")
    if(NOT OUT_DIR_MATCH)
        message(FATAL_ERROR "Cargo reported no openttd-kernels build-script OUT_DIR")
    endif()
    string(REPLACE "\\\\" "/" OUT_DIR "${CMAKE_MATCH_1}")
    execute_process(COMMAND "${CMAKE_COMMAND}" -E copy "${OUT_DIR}/strings.rs" "${TABLE_STRINGS}"
        RESULT_VARIABLE COPY_RESULT)
    if(NOT COPY_RESULT EQUAL 0)
        message(FATAL_ERROR "Cannot copy ${OUT_DIR}/strings.rs to ${TABLE_STRINGS}")
    endif()
endif()
file(WRITE "${PREVIOUS}" "${CONFIGURATION_HASH}")
