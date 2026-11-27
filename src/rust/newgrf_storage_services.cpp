/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_storage_services.cpp Native storage pool and shell members. */
#include "../stdafx.h"
#include "../newgrf_storage.h"
#include "../safeguards.h"

const OpenTTDStorageServices &GetRustStorageServices() noexcept
{
	static const OpenTTDStorageServices services{
		.can_allocate_item = []() noexcept { return PersistentStorage::CanAllocateItem(); },
		.new_storage = [](uint32_t grfid, uint8_t feature, uint32_t tile) noexcept { return (new PersistentStorage(grfid, static_cast<GrfSpecFeature>(feature), TileIndex{tile}))->rust_storage; },
		.grfid = [](const BasePersistentStorageArray *storage) noexcept { return storage->grfid; },
		.feature = [](const BasePersistentStorageArray *storage) noexcept -> uint8_t { return storage->feature; },
		.tile = [](const BasePersistentStorageArray *storage) noexcept { return storage->tile.base(); },
	};
	return services;
}
#include "service_table_check.hpp"
#include "abi-visit-OpenTTDStorageServices.hpp"
bool CheckAbi_storage() { return AbiServicesComplete(GetRustStorageServices()); }
