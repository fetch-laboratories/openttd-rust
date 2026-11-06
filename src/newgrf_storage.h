/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_storage.h Functionality related to the temporary and persistent storage arrays for NewGRFs. */

#ifndef NEWGRF_STORAGE_H
#define NEWGRF_STORAGE_H

#include "rust/pool.hpp"
#include <array>
#include "newgrf.h"
#include "tile_type.h"
#include "rust/newgrf_storage_ffi.h"
#include "rust/services_assert.h"

/**
 * Mode switches to the behaviour of persistent storage array.
 */
enum PersistentStorageMode : uint8_t {
	PSM_ENTER_GAMELOOP,   ///< Enter the gameloop, changes will be permanent.
	PSM_LEAVE_GAMELOOP,   ///< Leave the gameloop, changes will be temporary.
	PSM_ENTER_COMMAND,    ///< Enter command scope, changes will be permanent.
	PSM_LEAVE_COMMAND,    ///< Leave command scope, revert to previous mode.
	PSM_ENTER_TESTMODE,   ///< Enter command test mode, changes will be temporary.
	PSM_LEAVE_TESTMODE,   ///< Leave command test mode, revert to previous mode.
};

using PersistentStorageID = PoolID<uint32_t, struct PersistentStorageIDTag, 0xFF000, 0xFFFFF>;
struct PersistentStorage;
#include "abi-record-storage.h"

/**
 * Base class for all persistent NewGRF storage arrays. Nothing fancy, only here
 * so we have a generalised access to the virtual methods.
 */
struct BasePersistentStorageArray {
protected:
	OpenTTDPersistentStorageMetadata *metadata = nullptr; ///< Canonical fixed-data prefix; initialized by the array owner.
public:
	OpenTTDPersistentStorageMetadata &Metadata() { return *this->metadata; }
	const OpenTTDPersistentStorageMetadata &Metadata() const { return *this->metadata; }

	virtual ~BasePersistentStorageArray();

	static void SwitchMode(PersistentStorageMode mode, bool ignore_prev_mode = false);

};

/**
 * Class for persistent storage of data.
 * On #ClearChanges that data is either reverted or saved.
 * @tparam TYPE the type of variable to store.
 * @tparam SIZE the size of the array.
 */
template <typename TYPE, uint SIZE>
struct PersistentStorageArray : BasePersistentStorageArray {
	using StorageType = std::array<TYPE, SIZE>;

	static_assert(std::is_same_v<TYPE, int32_t> && (SIZE == 16 || SIZE == 256));
	using RustStorage = std::conditional_t<SIZE == 16, OpenTTDStorage16, OpenTTDStorage256>;
	RustStorage *rust_storage = RustStorageNew(std::integral_constant<uint, SIZE>{}, static_cast<BasePersistentStorageArray *>(this), &this->metadata);
	using RecordType = std::conditional_t<SIZE == 16, OpenTTDLegacyPersistentStorageRecord, OpenTTDPersistentStorageRecord>;
	/** Metadata is the first field of the generated fixed-data prefix. No view survives owner drop. */
	RecordType &Record() { static_assert(offsetof(RecordType, metadata) == 0); return *reinterpret_cast<RecordType *>(this->metadata); }
	const RecordType &Record() const { static_assert(offsetof(RecordType, metadata) == 0); return *reinterpret_cast<const RecordType *>(this->metadata); }

	PersistentStorageArray() = default;
	PersistentStorageArray(const PersistentStorageArray &) = delete;
	PersistentStorageArray &operator=(const PersistentStorageArray &) = delete;
	~PersistentStorageArray() override { RustStorageDelete(this->rust_storage); }
	void StoreValue(uint pos, int32_t value) { RustStorageStore(this->rust_storage, pos, value, RustAssertCondition); }
	TYPE GetValue(uint pos) const { return RustStorageGet(this->rust_storage, pos); }
	void ClearChanges() { RustStorageClear(this->rust_storage); }
};


/**
 * Class for temporary storage of data.
 * On #ClearChanges that data is always zero-ed.
 * @tparam TYPE the type of variable to store.
 * @tparam SIZE the size of the array.
 */
template <typename TYPE, uint SIZE>
struct TemporaryStorageArray {
	using StorageType = std::array<TYPE, SIZE>;
	using StorageInitType = std::array<uint16_t, SIZE>;

	static_assert(std::is_same_v<TYPE, int32_t> && SIZE == 0x110);
	void StoreValue(uint pos, int32_t value) { openttd_rust_storage_temp_store(pos, value); }
	TYPE GetValue(uint pos) const { return openttd_rust_storage_temp_get(pos); }
	void ClearChanges() { openttd_rust_storage_temp_clear(); }
};


typedef PersistentStorageArray<int32_t, 16> OldPersistentStorage;

using PersistentStoragePool = RustPool<PersistentStorage, PersistentStorageID, 1>;

extern PersistentStoragePool _persistent_storage_pool;

/**
 * Class for pooled persistent storage of data.
 */
struct PersistentStorage : PersistentStorageArray<int32_t, 256>, PersistentStoragePool::PoolItem<&_persistent_storage_pool> {
	/** We don't want GCC to zero our struct! It already is zeroed and has an index! */
	PersistentStorage(const uint32_t new_grfid, GrfSpecFeature feature, TileIndex tile)
	{
		this->Metadata().grfid = new_grfid;
		this->Metadata().feature = feature;
		this->Metadata().tile = tile;
		/* Both bases now exist: legacy arrays never enter this indexed path. */
		this->Record().shell = this;
		this->Record().index = this->index;
		_persistent_storage_pool.BindRecord(this->index.base(), this, &this->Record());
	}
	~PersistentStorage() override { _persistent_storage_pool.RetireRecord(this->index.base(), this, &this->Record()); }
};

static_assert(std::tuple_size_v<std::remove_reference_t<decltype(OldPersistentStorage::RecordType::storage)>> <= std::tuple_size_v<std::remove_reference_t<decltype(PersistentStorage::RecordType::storage)>>);

#endif /* NEWGRF_STORAGE_H */
