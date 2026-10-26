/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file override_manager_ffi.h Canonical NewGRF identity and override storage. */
#ifndef RUST_OVERRIDE_MANAGER_FFI_H
#define RUST_OVERRIDE_MANAGER_FFI_H
#include <cstdint>
struct EntityIDMapping;
struct OpenTTDOverrideManager;
struct OpenTTDOverrideView {
	EntityIDMapping *mappings;
	uint16_t *entities;
	uint32_t *grfids;
};
extern "C" {
OpenTTDOverrideManager *openttd_rust_override_new(uint16_t, uint16_t, uint16_t);
OpenTTDOverrideManager *openttd_rust_override_clone(const OpenTTDOverrideManager *);
void openttd_rust_override_assign(OpenTTDOverrideManager *, const OpenTTDOverrideManager *);
void openttd_rust_override_delete(OpenTTDOverrideManager *);
OpenTTDOverrideView openttd_rust_override_view(OpenTTDOverrideManager *);
void openttd_rust_override_add(OpenTTDOverrideManager *, uint16_t, uint32_t, uint32_t);
void openttd_rust_override_reset_mapping(OpenTTDOverrideManager *);
void openttd_rust_override_reset(OpenTTDOverrideManager *);
uint16_t openttd_rust_override_get(const OpenTTDOverrideManager *, uint16_t, uint32_t);
uint32_t openttd_rust_override_grfid(const OpenTTDOverrideManager *, uint16_t);
uint16_t openttd_rust_override_substitute(const OpenTTDOverrideManager *, uint16_t);
uint16_t openttd_rust_override_allocate(OpenTTDOverrideManager *, uint16_t, uint32_t, uint16_t, void *, uint16_t, bool (*)(void *, uint16_t) noexcept);
void openttd_rust_airport_register_manager(OpenTTDOverrideManager *);
void openttd_rust_airport_tile_register_manager(OpenTTDOverrideManager *);
void openttd_rust_industry_register_manager(OpenTTDOverrideManager *);
void openttd_rust_industry_tile_register_manager(OpenTTDOverrideManager *);
uint16_t openttd_rust_industry_definition_id(const OpenTTDOverrideManager *, uint16_t, uint32_t);
}
#endif
