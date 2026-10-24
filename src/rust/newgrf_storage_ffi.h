/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_storage_ffi.h Serialized canonical NewGRF storage owner. */
#ifndef RUST_NEWGRF_STORAGE_FFI_H
#define RUST_NEWGRF_STORAGE_FFI_H
#include "service_defaults.hpp"
#include <cstdint>
#include <cstddef>
#include <type_traits>
struct BasePersistentStorageArray;
struct Industry;
struct Station;
struct Town;
struct GRFFile;
struct OpenTTDStorage16;
struct OpenTTDStorage256;
struct OpenTTDStorageReference { OpenTTDStorage256 *owner; uint32_t grfid; };
struct OpenTTDIndustryStorage { OpenTTDStorage256 *owner; bool valid; };
/** All entries hold the native game-state mutex (including generation/main).
 * Canonical arrays stay allocated until shell destruction. Save/load calls
 * return before SlObject/SlError; no Rust frame spans native file errors.
 * Services cannot yield or throw. No owner borrow survives a service call. */
struct OpenTTDStorageServices {
	void (*check)(bool) noexcept OPENTTD_SERVICE_DEFAULT(check);
	uint32_t (*grfid)(const GRFFile *) noexcept OPENTTD_SERVICE_DEFAULT(grfid);
	OpenTTDIndustryStorage (*industry)(Industry *) noexcept OPENTTD_SERVICE_DEFAULT(industry);
	OpenTTDStorage256 *(*create_industry)(Industry *) noexcept OPENTTD_SERVICE_DEFAULT(create_industry);
	OpenTTDStorage256 *(*airport)(Station *) noexcept OPENTTD_SERVICE_DEFAULT(airport);
	OpenTTDStorage256 *(*create_airport)(Station *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(create_airport);
	size_t (*town_count)(const Town *) noexcept OPENTTD_SERVICE_DEFAULT(town_count);
	OpenTTDStorageReference (*town_storage)(const Town *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(town_storage);
	OpenTTDStorage256 *(*create_town)(Town *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(create_town);
	void (*append_town)(Town *, BasePersistentStorageArray *) noexcept OPENTTD_SERVICE_DEFAULT(append_town);
};
const OpenTTDStorageServices &GetRustStorageServices() noexcept;
extern "C" {
#define OPENTTD_STORAGE_API(N) \
OpenTTDStorage##N *openttd_rust_storage##N##_new(void *); \
void openttd_rust_storage##N##_delete(OpenTTDStorage##N *); \
int32_t *openttd_rust_storage##N##_data(OpenTTDStorage##N *); \
void openttd_rust_storage##N##_store(OpenTTDStorage##N *, uint32_t, int32_t, void (*)(bool) noexcept); \
int32_t openttd_rust_storage##N##_get(const OpenTTDStorage##N *, uint32_t); \
void openttd_rust_storage##N##_clear(OpenTTDStorage##N *);
OPENTTD_STORAGE_API(16)
OPENTTD_STORAGE_API(256)
#undef OPENTTD_STORAGE_API
void openttd_rust_storage_switch_mode(uint8_t, bool, void (*)(bool) noexcept, void (*)(void *) noexcept);
void openttd_rust_storage_temp_store(uint32_t, int32_t);
int32_t openttd_rust_storage_temp_get(uint32_t);
void openttd_rust_storage_temp_clear();
void openttd_rust_storage_industry(const OpenTTDStorageServices *, Industry *, uint32_t, int32_t);
void openttd_rust_storage_airport(const OpenTTDStorageServices *, Station *, const GRFFile *, uint32_t, int32_t);
void openttd_rust_storage_town(const OpenTTDStorageServices *, Town *, bool, const GRFFile *, uint32_t, int32_t);
uint32_t openttd_rust_storage_town_get(const OpenTTDStorageServices *, const Town *, const GRFFile *, uint32_t);
}
#define OPENTTD_STORAGE_OVERLOADS(N) \
inline OpenTTDStorage##N *RustStorageNew(std::integral_constant<uint, N>, void *shell) { return openttd_rust_storage##N##_new(shell); } \
inline void RustStorageDelete(OpenTTDStorage##N *s) { openttd_rust_storage##N##_delete(s); } \
inline int32_t *RustStorageData(OpenTTDStorage##N *s) { return openttd_rust_storage##N##_data(s); } \
inline void RustStorageStore(OpenTTDStorage##N *s, uint32_t p, int32_t v, void (*check)(bool) noexcept) { openttd_rust_storage##N##_store(s, p, v, check); } \
inline int32_t RustStorageGet(const OpenTTDStorage##N *s, uint32_t p) { return openttd_rust_storage##N##_get(s, p); } \
inline void RustStorageClear(OpenTTDStorage##N *s) { openttd_rust_storage##N##_clear(s); }
OPENTTD_STORAGE_OVERLOADS(16)
OPENTTD_STORAGE_OVERLOADS(256)
#undef OPENTTD_STORAGE_OVERLOADS
#endif /* RUST_NEWGRF_STORAGE_FFI_H */
