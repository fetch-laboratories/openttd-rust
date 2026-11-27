/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_storage_ffi.h Serialized canonical NewGRF storage owner. */
#ifndef RUST_NEWGRF_STORAGE_FFI_H
#define RUST_NEWGRF_STORAGE_FFI_H
#include "service_defaults.hpp"
#include <cstdint>
#include <cstddef>
#include <type_traits>
struct BasePersistentStorageArray;
struct OpenTTDStorage16;
struct OpenTTDStorage256;
/** All entries hold the native game-state mutex (including generation/main).
 * Canonical arrays stay allocated until shell destruction. Save/load calls
 * return before SlObject/SlError; no Rust frame spans native file errors.
 * Services cannot yield or throw. No owner borrow survives a service call. */
struct OpenTTDStorageServices {
	bool (*can_allocate_item)() noexcept OPENTTD_SERVICE_DEFAULT(can_allocate_item);
	OpenTTDStorage256 *(*new_storage)(uint32_t, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(new_storage);
	uint32_t (*grfid)(const BasePersistentStorageArray *) noexcept OPENTTD_SERVICE_DEFAULT(grfid);
	uint8_t (*feature)(const BasePersistentStorageArray *) noexcept OPENTTD_SERVICE_DEFAULT(feature);
	uint32_t (*tile)(const BasePersistentStorageArray *) noexcept OPENTTD_SERVICE_DEFAULT(tile);
};
const OpenTTDStorageServices &GetRustStorageServices() noexcept;
extern "C" {
#define OPENTTD_STORAGE_API(N) \
OpenTTDStorage##N *openttd_rust_storage##N##_new(void *, const OpenTTDStorageServices *); \
void openttd_rust_storage##N##_delete(OpenTTDStorage##N *); \
int32_t *openttd_rust_storage##N##_data(OpenTTDStorage##N *); \
void openttd_rust_storage##N##_store(OpenTTDStorage##N *, uint32_t, int32_t); \
int32_t openttd_rust_storage##N##_get(const OpenTTDStorage##N *, uint32_t); \
void openttd_rust_storage##N##_clear(OpenTTDStorage##N *);
OPENTTD_STORAGE_API(16)
OPENTTD_STORAGE_API(256)
#undef OPENTTD_STORAGE_API
void openttd_rust_storage_switch_mode(uint8_t, bool);
void openttd_rust_storage_temp_store(uint32_t, int32_t);
int32_t openttd_rust_storage_temp_get(uint32_t);
}
#define OPENTTD_STORAGE_OVERLOADS(N) \
inline OpenTTDStorage##N *RustStorageNew(std::integral_constant<uint, N>, void *shell, const OpenTTDStorageServices *services) { return openttd_rust_storage##N##_new(shell, services); } \
inline void RustStorageDelete(OpenTTDStorage##N *s) { openttd_rust_storage##N##_delete(s); } \
inline int32_t *RustStorageData(OpenTTDStorage##N *s) { return openttd_rust_storage##N##_data(s); } \
inline void RustStorageStore(OpenTTDStorage##N *s, uint32_t p, int32_t v) { openttd_rust_storage##N##_store(s, p, v); } \
inline int32_t RustStorageGet(const OpenTTDStorage##N *s, uint32_t p) { return openttd_rust_storage##N##_get(s, p); } \
inline void RustStorageClear(OpenTTDStorage##N *s) { openttd_rust_storage##N##_clear(s); }
OPENTTD_STORAGE_OVERLOADS(16)
OPENTTD_STORAGE_OVERLOADS(256)
#undef OPENTTD_STORAGE_OVERLOADS
#endif /* RUST_NEWGRF_STORAGE_FFI_H */
