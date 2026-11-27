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
OpenTTDStorage16 *openttd_rust_storage16_new(void *, const OpenTTDStorageServices *);
void openttd_rust_storage16_delete(OpenTTDStorage16 *);
int32_t *openttd_rust_storage16_data(OpenTTDStorage16 *);
void openttd_rust_storage16_store(OpenTTDStorage16 *, uint32_t, int32_t);
int32_t openttd_rust_storage16_get(const OpenTTDStorage16 *, uint32_t);
void openttd_rust_storage16_clear(OpenTTDStorage16 *);
OpenTTDStorage256 *openttd_rust_storage256_new(void *, const OpenTTDStorageServices *);
void openttd_rust_storage256_delete(OpenTTDStorage256 *);
int32_t *openttd_rust_storage256_data(OpenTTDStorage256 *);
void openttd_rust_storage256_store(OpenTTDStorage256 *, uint32_t, int32_t);
int32_t openttd_rust_storage256_get(const OpenTTDStorage256 *, uint32_t);
void openttd_rust_storage256_clear(OpenTTDStorage256 *);
void openttd_rust_storage_switch_mode(uint8_t, bool);
void openttd_rust_storage_temp_store(uint32_t, int32_t);
int32_t openttd_rust_storage_temp_get(uint32_t);
}
inline OpenTTDStorage16 *RustStorageNew(std::integral_constant<uint, 16>, void *shell, const OpenTTDStorageServices *services) { return openttd_rust_storage16_new(shell, services); }
inline void RustStorageDelete(OpenTTDStorage16 *s) { openttd_rust_storage16_delete(s); }
inline int32_t *RustStorageData(OpenTTDStorage16 *s) { return openttd_rust_storage16_data(s); }
inline void RustStorageStore(OpenTTDStorage16 *s, uint32_t p, int32_t v) { openttd_rust_storage16_store(s, p, v); }
inline int32_t RustStorageGet(const OpenTTDStorage16 *s, uint32_t p) { return openttd_rust_storage16_get(s, p); }
inline void RustStorageClear(OpenTTDStorage16 *s) { openttd_rust_storage16_clear(s); }
inline OpenTTDStorage256 *RustStorageNew(std::integral_constant<uint, 256>, void *shell, const OpenTTDStorageServices *services) { return openttd_rust_storage256_new(shell, services); }
inline void RustStorageDelete(OpenTTDStorage256 *s) { openttd_rust_storage256_delete(s); }
inline int32_t *RustStorageData(OpenTTDStorage256 *s) { return openttd_rust_storage256_data(s); }
inline void RustStorageStore(OpenTTDStorage256 *s, uint32_t p, int32_t v) { openttd_rust_storage256_store(s, p, v); }
inline int32_t RustStorageGet(const OpenTTDStorage256 *s, uint32_t p) { return openttd_rust_storage256_get(s, p); }
inline void RustStorageClear(OpenTTDStorage256 *s) { openttd_rust_storage256_clear(s); }
#endif /* RUST_NEWGRF_STORAGE_FFI_H */
