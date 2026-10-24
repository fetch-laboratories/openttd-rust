/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_newgrf.h Shared NewGRF leaves for synchronous Rust services. */
#ifndef RUST_SERVICES_NEWGRF_H
#define RUST_SERVICES_NEWGRF_H
#include "../command_type.h"
#include "../newgrf.h"
#include "../newgrf_commons.h"
#include "../newgrf_debug.h"
#include "../newgrf_house.h"
#include "../house.h"
#include "../newgrf_spritegroup.h"
#include "../industrytype.h"
#include "../newgrf_badge.h"
#include "../newgrf_railtype.h"
#include "../town.h"
inline const HouseSpec *RustHouseSpec(uint16_t house) noexcept { return HouseSpec::Get(house); }

/* These synchronous field reads borrow live native objects. Callers retain
 * the original non-null resolver/file preconditions; nothing is cached. */
inline uint32_t RustGRFFileID(const GRFFile *file) noexcept { return file->grfid; }
inline uint32_t RustResolverGRFFileID(const ResolverObject *resolver) noexcept { return RustGRFFileID(resolver->grffile); }
inline uint8_t RustGRFFileVersion(const GRFFile *file) noexcept { return file->grf_version; }
inline uint8_t RustResolverGRFVersion(const ResolverObject *resolver) noexcept { return RustGRFFileVersion(resolver->grffile); }
inline bool RustResolverGRFVersion8(const ResolverObject *resolver) noexcept { return RustResolverGRFVersion(resolver) >= 8; }
template <typename Spec, typename Handle = Spec>
inline uint8_t RustSpecGRFVersion(const Handle *handle) noexcept { return RustGRFFileVersion(reinterpret_cast<const Spec *>(handle)->grf_prop.grffile); }
template <typename Spec>
inline bool RustSpecGRFVersion8(const Spec *spec) noexcept { return RustSpecGRFVersion<Spec>(spec) >= 8; }
/* GetRegister retains its native register-range precondition (reg < 0x110). */
inline int32_t RustResolverRegister(const ResolverObject *resolver, uint16_t reg) noexcept { return resolver->GetRegister(reg); }
inline int32_t RustResolverRegister100(const ResolverObject *resolver) noexcept { return RustResolverRegister(resolver, 0x100); }

/* Keep the native map wrapping, station-axis handling and signed-offset default. */
inline uint32_t RustNearbyTileWithAxis(uint8_t parameter, uint32_t tile, bool signed_offsets, Axis axis) noexcept { return GetNearbyTile(parameter, TileIndex(tile), signed_offsets, axis).base(); }
inline uint32_t RustNearbyTile(uint8_t parameter, uint32_t tile, bool signed_offsets) noexcept { return RustNearbyTileWithAxis(parameter, tile, signed_offsets, INVALID_AXIS); }
inline uint32_t RustSignedNearbyTile(uint8_t parameter, uint32_t tile) noexcept { return RustNearbyTile(parameter, tile, true); }
inline uint32_t RustNearbyStationTile(uint8_t parameter, uint32_t tile, Axis axis) noexcept { return RustNearbyTileWithAxis(parameter, tile, true, axis); }
inline uint32_t RustNearbyTileInformation(uint32_t tile, bool version8) noexcept { return GetNearbyTileInformation(TileIndex(tile), version8); }
inline uint32_t RustTerrainType(uint32_t tile) noexcept { return GetTerrainType(TileIndex(tile)); }
inline HouseZone RustTownRadiusGroup(const Town *town, uint32_t tile) noexcept { return GetTownRadiusGroup(town, TileIndex(tile)); }
template <typename Spec>
inline uint32_t RustSpecBadges(const ResolverObject *resolver, const Spec *spec, uint32_t parameter) noexcept { return GetBadgeVariableResult(*resolver->grffile, spec->badges, parameter); }

/* Preserve native default-livery and nullable-file translation behavior. */
inline uint32_t RustCompanyInfo(uint8_t owner, const Livery *livery) noexcept { return GetCompanyInfo(CompanyID(owner), livery); }
inline uint8_t RustReverseRailTypeTranslation(RailType railtype, const GRFFile *file) noexcept { return GetReverseRailTypeTranslation(railtype, file); }

/* Borrowed GRF files remain native and live for the synchronous conversion.
 * A town shell is live for resolution (or null for a prototype); no Rust
 * owner borrow spans it. No C++ exception crosses Rust; environmental
 * failures terminate. */
inline uint8_t RustGRFCargoMap(const GRFFile *grf, CargoType cargo) noexcept { return grf->cargo_map[cargo]; }
inline bool OpenTTDConvertBooleanCallback(const GRFFile *grf, uint16_t callback, uint16_t result) noexcept { return ConvertBooleanCallback(grf, callback, result); }
inline bool OpenTTDConvert8bitBooleanCallback(const GRFFile *grf, uint16_t callback, uint16_t result) noexcept { return Convert8bitBooleanCallback(grf, callback, result); }
inline uint16_t OpenTTDHouseCallback(CallbackID callback, uint32_t param1, uint32_t param2, HouseID house, Town *town, TileIndex tile, bool not_yet_constructed, uint8_t initial_random = 0) noexcept
{
	return GetHouseCallback(callback, param1, param2, house, town, tile, {}, not_yet_constructed, initial_random);
}
inline void RustCloseNewGRFInspect(GrfSpecFeature feature, uint32_t index) noexcept { DeleteNewGRFInspectWindow(feature, index); }
/* Typed adapters retain the existing per-component table signatures. */
inline void OpenTTDDeleteRailInspect(uint32_t tile) noexcept { RustCloseNewGRFInspect(GSF_RAILTYPES, tile); }
inline void OpenTTDDeleteHouseInspect(uint32_t tile) noexcept { RustCloseNewGRFInspect(GSF_HOUSES, tile); }
inline void OpenTTDDeleteIndustryTileInspect(uint32_t tile) noexcept { RustCloseNewGRFInspect(GSF_INDUSTRYTILES, tile); }
inline void OpenTTDDeleteTownInspect(uint16_t town) noexcept { RustCloseNewGRFInspect(GSF_FAKE_TOWNS, town); }
inline void OpenTTDDeleteIndustryInspect(uint16_t industry) noexcept { RustCloseNewGRFInspect(GSF_INDUSTRIES, industry); }
inline void RustUnknownCallbackResult(uint32_t grfid, uint16_t callback, uint16_t result) noexcept { ErrorUnknownCallbackResult(grfid, callback, result); }
template <typename Spec, typename Handle>
inline void RustSpecCallbackError(const Handle *handle, uint16_t callback, uint16_t result) noexcept
{
	const Spec *spec = reinterpret_cast<const Spec *>(handle);
	RustUnknownCallbackResult(spec->grf_prop.grfid, callback, result);
}
template <uint16_t Callback, typename Spec>
inline void RustFixedSpecCallbackError(const Spec *spec, uint16_t result) noexcept { RustSpecCallbackError<Spec, Spec>(spec, Callback, result); }
template <typename Spec>
inline bool RustSpecBooleanCallback(const Spec *spec, uint16_t callback, uint16_t result) noexcept
{
	return OpenTTDConvertBooleanCallback(spec->grf_prop.grffile, callback, result);
}
template <typename Spec, typename Handle, typename Result, typename Cost, StringID Error>
inline void RustLocationCallbackError(Cost *out, const Result *result, const Handle *handle) noexcept
{
	const Spec *spec = reinterpret_cast<const Spec *>(handle);
	*reinterpret_cast<CommandCost *>(out) = GetErrorMessageFromLocationCallbackResult(result->callback, result->registers, spec->grf_prop.grffile, Error);
}
template <GrfSpecFeature Feature, typename Id>
inline void RustCloseInspect(Id id) noexcept { RustCloseNewGRFInspect(Feature, id); }
#endif /* RUST_SERVICES_NEWGRF_H */
