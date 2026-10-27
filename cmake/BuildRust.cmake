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
execute_process(COMMAND "${CARGO}" rustc --package openttd-kernels --release --locked
    --target "${TARGET}" --manifest-path "${SOURCE}/Cargo.toml" ${FEATURE_ARGS} -- --print=native-static-libs
    WORKING_DIRECTORY "${SOURCE}" OUTPUT_VARIABLE OUTPUT ERROR_VARIABLE ERROR RESULT_VARIABLE RESULT)
file(WRITE "${LOG}" "${CLEAN_OUTPUT}${CLEAN_ERROR}${OUTPUT}${ERROR}")
if(NOT RESULT EQUAL 0)
    message(FATAL_ERROR "Rust archive build failed: ${OUTPUT}${ERROR}")
endif()
message(STATUS "${OUTPUT}${ERROR}")
file(WRITE "${PREVIOUS}" "${CONFIGURATION_HASH}")
