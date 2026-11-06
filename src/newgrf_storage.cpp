/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_storage.cpp Functionality related to the temporary and persistent storage arrays for NewGRFs. */

#include "stdafx.h"
#include "newgrf_storage.h"
#include "debug.h"

#include "safeguards.h"

PersistentStoragePool _persistent_storage_pool("PersistentStorage");

BasePersistentStorageArray::~BasePersistentStorageArray() = default;
void RustStorageDiscard(void *shell) noexcept
{
	auto *it = static_cast<BasePersistentStorageArray *>(shell);
	Debug(desync, 2, "warning: discarding persistent storage changes: Feature {}, GrfID {:08X}, Tile {}", it->Metadata().feature, std::byteswap(it->Metadata().grfid), it->Metadata().tile);
}
void BasePersistentStorageArray::SwitchMode(PersistentStorageMode mode, bool ignore_prev_mode)
{
	openttd_rust_storage_switch_mode(mode, ignore_prev_mode, RustAssertCondition, RustStorageDiscard);
}
