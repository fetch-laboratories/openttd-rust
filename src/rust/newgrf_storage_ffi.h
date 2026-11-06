/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_storage_ffi.h Serialized canonical NewGRF storage owner. */
#ifndef RUST_NEWGRF_STORAGE_FFI_H
#define RUST_NEWGRF_STORAGE_FFI_H
#include "service_defaults.hpp"
#include <cstdint>
#include <cstddef>
#include <type_traits>
struct BasePersistentStorageArray;
struct PersistentStorage;
struct OpenTTDPersistentStorageMetadata;
struct OpenTTDPersistentStorageRecord;
struct Industry;
struct Station;
struct Town;
struct GRFFile;
struct OpenTTDSharedServices;
struct OpenTTDStorage16;
struct OpenTTDStorage256;
struct OpenTTDIndustryStorage { OpenTTDStorage256 *owner; bool valid; };
/** All entries hold the native game-state mutex (including generation/main).
 * Canonical arrays stay allocated until shell destruction. Save/load calls
 * return before SlObject/SlError; no Rust frame spans native file errors.
 * Services cannot yield or throw. No owner borrow survives a service call. */
struct OpenTTDStorageServices {
	const OpenTTDSharedServices *shared;
	void (*check)(bool) noexcept OPENTTD_SERVICE_DEFAULT(check);
	PersistentStorage *(*create)(uint32_t, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(create);
	void (*destroy)(PersistentStorage *) noexcept OPENTTD_SERVICE_DEFAULT(destroy);
	/** Null-in/null-out; otherwise resolve one live allocation under the game-state mutex. */
	OpenTTDPersistentStorageRecord *(*record)(const PersistentStorage *) noexcept OPENTTD_SERVICE_DEFAULT(record);
	OpenTTDIndustryStorage (*industry)(Industry *) noexcept OPENTTD_SERVICE_DEFAULT(industry);
	OpenTTDStorage256 *(*create_industry)(Industry *) noexcept OPENTTD_SERVICE_DEFAULT(create_industry);
	OpenTTDStorage256 *(*airport)(Station *) noexcept OPENTTD_SERVICE_DEFAULT(airport);
	OpenTTDStorage256 *(*create_airport)(Station *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(create_airport);
	size_t (*town_count)(const Town *) noexcept OPENTTD_SERVICE_DEFAULT(town_count);
	/** The index is valid in the original list; the returned record ends on drop/reset. */
	OpenTTDPersistentStorageRecord *(*town_storage)(const Town *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(town_storage);
	OpenTTDStorage256 *(*create_town)(Town *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(create_town);
	void (*append_town)(Town *, BasePersistentStorageArray *) noexcept OPENTTD_SERVICE_DEFAULT(append_town);
};
const OpenTTDStorageServices &GetRustStorageServices() noexcept;
/** Native shell association only; the caller retains the original scope gates. */
OpenTTDStorage256 *RustIndustryStorageOwner(Industry *) noexcept;
void RustStorageDiscard(void *) noexcept;
extern "C" {
#define OPENTTD_STORAGE_API(N) \
OpenTTDStorage##N *openttd_rust_storage##N##_new(void *, OpenTTDPersistentStorageMetadata **); \
void openttd_rust_storage##N##_delete(OpenTTDStorage##N *); \
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
}
#define OPENTTD_STORAGE_OVERLOADS(N) \
inline OpenTTDStorage##N *RustStorageNew(std::integral_constant<uint, N>, void *shell, OpenTTDPersistentStorageMetadata **metadata) { return openttd_rust_storage##N##_new(shell, metadata); } \
inline void RustStorageDelete(OpenTTDStorage##N *s) { openttd_rust_storage##N##_delete(s); } \
inline void RustStorageStore(OpenTTDStorage##N *s, uint32_t p, int32_t v, void (*check)(bool) noexcept) { openttd_rust_storage##N##_store(s, p, v, check); } \
inline int32_t RustStorageGet(const OpenTTDStorage##N *s, uint32_t p) { return openttd_rust_storage##N##_get(s, p); } \
inline void RustStorageClear(OpenTTDStorage##N *s) { openttd_rust_storage##N##_clear(s); }
OPENTTD_STORAGE_OVERLOADS(16)
OPENTTD_STORAGE_OVERLOADS(256)
#undef OPENTTD_STORAGE_OVERLOADS
#endif /* RUST_NEWGRF_STORAGE_FFI_H */
