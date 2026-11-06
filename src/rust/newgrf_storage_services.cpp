/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_storage_services.cpp Native pool identity and feature associations. */
#include "../stdafx.h"
#include "../newgrf_storage.h"
#include "../station_base.h"
#include "services_newgrf.h"
#include "../safeguards.h"

static PersistentStorage *StorageCreate(uint32_t grfid, uint8_t feature, uint32_t tile) noexcept
{
	assert(PersistentStorage::CanAllocateItem());
	return new PersistentStorage(grfid, static_cast<GrfSpecFeature>(feature), TileIndex{tile});
}
static void StorageDestroy(PersistentStorage *storage) noexcept { delete storage; }
static OpenTTDStorageReference StorageReference(const PersistentStorage *storage) noexcept
{
	return {storage->rust_storage, storage->grfid};
}
static OpenTTDStorage256 *StorageAirport(Station *station) noexcept { return station->airport.psa == nullptr ? nullptr : station->airport.psa->rust_storage; }
static OpenTTDStorage256 *StorageCreateAirport(Station *station, uint32_t grfid) noexcept
{
	assert(PersistentStorage::CanAllocateItem());
	station->airport.psa = new PersistentStorage(grfid, GSF_AIRPORTS, station->airport.tile);
	return station->airport.psa->rust_storage;
}
const OpenTTDStorageServices &GetRustStorageServices() noexcept
{
	static const OpenTTDStorageServices services{
		.shared = &GetRustSharedServices(),
		.check = RustAssertCondition,
		.create = StorageCreate,
		.destroy = StorageDestroy,
		.reference = StorageReference,
		.airport = StorageAirport,
		.create_airport = StorageCreateAirport,
	};
	return services;
}
#include "service_table_check.hpp"
#include "abi-visit-OpenTTDStorageServices.hpp"
bool CheckAbi_storage() { return AbiServicesComplete(GetRustStorageServices()); }
