/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file engine_loading_ffi.h Canonical engine loading state and synchronous native leaves. */
#ifndef RUST_ENGINE_LOADING_FFI_H
#define RUST_ENGINE_LOADING_FFI_H
#include "../engine_base.h"
struct OpenTTDEngineMappingView { const EngineIDMapping *data; size_t size; };
extern "C" {
OpenTTDEngineMappingView openttd_rust_engine_mappings(uint8_t);
void openttd_rust_engine_mapping_clear();
void openttd_rust_engine_mapping_defaults(const uint8_t *);
uint16_t openttd_rust_engine_mapping_get(uint8_t, uint16_t, uint32_t);
uint16_t openttd_rust_engine_mapping_use(uint8_t, uint16_t, uint32_t, bool);
void openttd_rust_engine_mapping_set(uint8_t, uint16_t, uint32_t, uint8_t, uint16_t);
void openttd_rust_engine_override_set(uint32_t, uint32_t);
bool openttd_rust_engine_override_get(uint32_t, uint32_t *);
void openttd_rust_engine_override_clear();
}
#endif
