/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file services_saveload.h Typed native pool references for the Rust codec. */
#ifndef RUST_SERVICES_SAVELOAD_H
#define RUST_SERVICES_SAVELOAD_H

#include "environment_services.h"
#include "services_autoreplace.h"
#include "services_linkgraph.h"
#include "services_station.h"
#include "vehicle_services.h"

/** Index of a pool item, for ReferenceToInt. */
template <typename T>
inline uint32_t RustSaveReferenceIndex(const void *obj) noexcept
{
	if constexpr (std::is_same_v<T, Vehicle>) {
		return RustVehicleIndexOpaque(obj);
	} else if constexpr (std::is_same_v<T, Station>) {
		return RustBaseStationID(static_cast<const Station *>(obj));
	} else if constexpr (std::is_same_v<T, Town>) {
		return RustTownAuthorityIndex(static_cast<const OpenTTDTownHandle *>(obj));
	} else if constexpr (std::is_same_v<T, EngineRenew>) {
		return RustEngineRenewIndex(obj);
	} else {
		return static_cast<const T *>(obj)->index.base();
	}
}

/** Pool item at an index, or nullptr when the index is invalid, for IntToReference. */
template <typename T>
inline void *RustSaveReferenceGet(size_t index) noexcept
{
	if (!T::IsValidID(index)) return nullptr;
	/* Validate the full size_t index before narrowing it for shared accessors. */
	if constexpr (std::is_same_v<T, Station>) {
		return RustStationGet<Station *>(static_cast<uint16_t>(index));
	} else if constexpr (std::is_same_v<T, Town>) {
		return OpenTTDTownShellByID(static_cast<uint32_t>(index));
	} else if constexpr (std::is_same_v<T, LinkGraph>) {
		return RustLinkGraphGet(static_cast<uint16_t>(index));
	} else {
		return T::Get(index);
	}
}

#endif /* RUST_SERVICES_SAVELOAD_H */
