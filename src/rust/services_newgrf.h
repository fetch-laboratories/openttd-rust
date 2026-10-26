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
#include "newgrf_class_ffi.h"
#include "../newgrf.h"
#include "../newgrf_commons.h"
#include "../newgrf_debug.h"
#include "../newgrf_sound.h"
#include "../debug.h"
#include "../newgrf_house.h"
#include "../house.h"
#include "../town_map.h"
#include "../newgrf_spritegroup.h"
#include "../industrytype.h"
#include "../newgrf_badge.h"
#include "../newgrf_railtype.h"
#include "../newgrf_roadtype.h"
#include "../town.h"
#include "../newgrf_config.h"
#include "../newgrf/newgrf_internal.h"
#include "../newgrf/newgrf_stringmapping.h"
#include "../rev.h"
inline void RustGRFPlayTileSound(const GRFFile *file, uint16_t sound, uint32_t tile) noexcept { PlayTileSound(file, sound, TileIndex{tile}); }
inline uint16_t RustIndustrySubstitute(uint8_t industry) noexcept { return GetIndustrySpec(industry)->grf_prop.subst_id; }
inline int32_t RustHouseAge(uint32_t tile) noexcept { return GetHouseAge(TileIndex{tile}).base(); }
inline uint8_t RustHouseRandomBits(uint32_t tile) noexcept { return GetHouseRandomBits(TileIndex{tile}); }
inline uint8_t RustHouseRandomTriggers(uint32_t tile) noexcept { return GetHouseRandomTriggers(TileIndex{tile}).base(); }
inline const HouseSpec *RustHouseSpec(uint16_t house) noexcept { return HouseSpec::Get(house); }
inline uint16_t RustTownHouseClass(const HouseSpec *spec) noexcept { return spec->class_id; }
inline uint16_t RustTownHouseClass(uint16_t house) noexcept { return RustTownHouseClass(RustHouseSpec(house)); }

inline void RustUnhandledScopeVariable(uint8_t variable) noexcept { Debug(grf, 1, "Unhandled scope variable 0x{:X}", variable); }

/* These synchronous field reads borrow live native objects. Callers retain
 * the original non-null resolver/file preconditions; nothing is cached. */
inline const GRFFile *RustResolverGRFFile(const ResolverObject *resolver) noexcept { return resolver->grffile; }
inline uint32_t RustGRFFileID(const GRFFile *file) noexcept { return file->grfid; }
inline GRFFile *RustGRFFileByID(uint32_t id) noexcept { return GetFileByGRFID(id); }
inline RailType RustGRFRailTypeByLabel(RailTypeLabel label) noexcept { return GetRailTypeByLabel(label); }
inline RoadType RustGRFRoadTypeByLabel(RoadTypeLabel label) noexcept { return GetRoadTypeByLabel(label); }
inline uint32_t RustResolverGRFFileID(const ResolverObject *resolver) noexcept { return RustGRFFileID(RustResolverGRFFile(resolver)); }
inline uint8_t RustGRFFileVersion(const GRFFile *file) noexcept { return file->grf_version; }
inline uint8_t RustResolverGRFVersion(const ResolverObject *resolver) noexcept { return RustGRFFileVersion(RustResolverGRFFile(resolver)); }
inline bool RustResolverGRFVersion8(const ResolverObject *resolver) noexcept { return RustResolverGRFVersion(resolver) >= 8; }
template <typename Spec>
inline const GRFFile *RustSpecGRFFile(const Spec *spec) noexcept { return spec->grf_prop.grffile; }
template <typename Spec, typename Handle>
inline const GRFFile *RustSpecHandleGRFFile(const Handle *spec) noexcept { return RustSpecGRFFile(reinterpret_cast<const Spec *>(spec)); }
template <typename Spec>
inline void RustSpecPlayTileSound(const Spec *spec, uint8_t sound, uint32_t tile) noexcept { RustGRFPlayTileSound(RustSpecGRFFile(spec), sound, tile); }
template <typename Spec>
inline void RustSpecSetGRFFile(Spec *spec, const GRFFile *file) noexcept { spec->grf_prop.SetGRFFile(file); }
template <typename Spec>
inline void RustSpecSetCargoGroup(Spec *spec, CargoType cargo, const SpriteGroup *group) noexcept { spec->grf_prop.SetSpriteGroup(cargo, group); }
template <typename Spec>
inline void RustSpecSetLocalID(Spec *spec, uint16_t id) noexcept { spec->grf_prop.local_id = id; }
template <typename Spec>
inline void RustSpecRegisterName(Spec *spec, uint16_t source) noexcept { AddStringForMapping(GRFStringID{source}, &spec->name); }
template <typename Spec, void (*Publish)(const Spec *, StringID)>
inline void RustSpecRegisterClassName(Spec *spec, uint16_t source) noexcept { AddStringForMapping(GRFStringID{source}, [spec](StringID name) { Publish(spec, name); }); }
template <typename Spec>
inline bool RustSpecHasGRFFile(const Spec *spec) noexcept { return spec->grf_prop.HasGrfFile(); }
inline const GRFFile *RustHouseGrf(uint16_t house) noexcept { return RustSpecGRFFile(RustHouseSpec(house)); }
template <typename Spec, typename Handle = Spec>
inline uint8_t RustSpecGRFVersion(const Handle *handle) noexcept { return RustGRFFileVersion(RustSpecGRFFile(reinterpret_cast<const Spec *>(handle))); }
template <typename Spec>
inline bool RustSpecGRFVersion8(const Spec *spec) noexcept { return RustSpecGRFVersion<Spec>(spec) >= 8; }
/* Container reads retain the native index preconditions and full size_t width.
 * Selection and narrowing belong to the caller; these leaves allocate nothing. */
inline size_t RustRealLoadingCount(const RealSpriteGroup *group) noexcept { return group->loading.size(); }
inline size_t RustRealLoadedCount(const RealSpriteGroup *group) noexcept { return group->loaded.size(); }
inline const SpriteGroup *RustRealLoading(const RealSpriteGroup *group, size_t index) noexcept { return group->loading[index]; }
inline const SpriteGroup *RustRealLoaded(const RealSpriteGroup *group, size_t index) noexcept { return group->loaded[index]; }
template <typename Spec>
inline bool RustSpecHasSpriteGroups(const Spec *spec) noexcept { return spec->grf_prop.HasSpriteGroups(); }
inline bool RustHouseHasSpriteGroups(uint16_t house) noexcept { return RustSpecHasSpriteGroups(RustHouseSpec(house)); }

inline uint32_t RustGRFParameter(const GRFFile *file, uint32_t parameter) noexcept { return file->GetParam(parameter); }
inline uint8_t RustGRFPalette(const GRFFile *file) noexcept { return GetGRFConfig(file->grfid)->palette & GRFP_USE_MASK; }
inline int32_t RustGRFTrainPitch(const GRFFile *file) noexcept { return file->traininfo_vehicle_pitch; }
inline uint32_t RustGRFTrainWidth(const GRFFile *file) noexcept { return file->traininfo_vehicle_width; }
inline uint8_t RustGRFMiscBits() noexcept
{
	extern GrfMiscBits _misc_grf_features;
	assert(!_misc_grf_features.Test(GrfMiscBit::TrainWidth32Pixels));
	return _misc_grf_features.base();
}
inline uint32_t RustNewGRFVersion() noexcept { return _openttd_newgrf_version; }

/* Keep the native map wrapping, station-axis handling and signed-offset default. */
inline uint32_t RustNearbyTileWithAxis(uint8_t parameter, uint32_t tile, bool signed_offsets, Axis axis) noexcept { return GetNearbyTile(parameter, TileIndex(tile), signed_offsets, axis).base(); }
inline uint32_t RustNearbyTile(uint8_t parameter, uint32_t tile, bool signed_offsets) noexcept { return RustNearbyTileWithAxis(parameter, tile, signed_offsets, INVALID_AXIS); }
inline uint32_t RustSignedNearbyTile(uint8_t parameter, uint32_t tile) noexcept { return RustNearbyTile(parameter, tile, true); }
inline uint32_t RustNearbyStationTile(uint8_t parameter, uint32_t tile, Axis axis) noexcept { return RustNearbyTileWithAxis(parameter, tile, true, axis); }
inline uint32_t RustNearbyTileInformation(uint32_t tile, bool version8) noexcept { return GetNearbyTileInformation(TileIndex(tile), version8); }
inline uint32_t RustTerrainType(uint32_t tile) noexcept { return GetTerrainType(TileIndex(tile)); }
inline HouseZone RustTownRadiusGroup(const Town *town, uint32_t tile) noexcept { return GetTownRadiusGroup(town, TileIndex(tile)); }
template <typename Spec>
inline uint32_t RustSpecBadges(const ResolverObject *resolver, const Spec *spec, uint32_t parameter) noexcept { return GetBadgeVariableResult(*RustResolverGRFFile(resolver), spec->badges, parameter); }
inline uint32_t RustHouseBadges(const ResolverObject *resolver, uint16_t house, uint32_t parameter) noexcept { return RustSpecBadges(resolver, RustHouseSpec(house), parameter); }

/* Preserve native default-livery and nullable-file translation behavior. */
inline uint32_t RustCompanyInfo(uint8_t owner, const Livery *livery) noexcept { return GetCompanyInfo(CompanyID(owner), livery); }
inline uint8_t RustReverseRailTypeTranslation(RailType railtype, const GRFFile *file) noexcept { return GetReverseRailTypeTranslation(railtype, file); }
/* A spec's file can differ from its caller resolver's file. */
template <typename Spec>
inline uint8_t RustSpecReverseRailTypeTranslation(RailType railtype, const Spec *spec) noexcept { return RustReverseRailTypeTranslation(railtype, RustSpecGRFFile(spec)); }


/* Native frame address only; Rust retains common/feature scope selection. */
inline ScopeResolver *RustResolverBaseScope(ResolverObject *resolver, uint8_t, uint8_t) noexcept { return &resolver->default_scope; }
/* Optional facade construction only; successful parent selection belongs to the
 * Rust feature owner. The town/storage implementation is shared by all features. */
template <typename Resolver>
inline TownScopeResolver *RustResolverTownScope(Resolver *resolver, const Town *town, bool readonly) noexcept
{
	if (!resolver->town_scope.has_value()) resolver->town_scope.emplace(*resolver, const_cast<Town *>(town), readonly);
	return &*resolver->town_scope;
}
template <typename Resolver>
inline ScopeResolver *RustResolverBaseScopeFor(Resolver *resolver, uint8_t scope, uint8_t relative) noexcept { return RustResolverBaseScope(resolver, scope, relative); }

inline uint16_t RustResolverCallback(const ResolverObject *resolver) noexcept { return resolver->callback; }
template <typename Spec> inline uint16_t RustSpecOverride(const Spec *spec) noexcept { return spec->grf_prop.override_id; }
template <typename Spec> inline uint16_t RustSpecSubstitute(const Spec *spec) noexcept { return spec->grf_prop.subst_id; }
template <typename Spec> inline uint32_t RustSpecGRFID(const Spec *spec) noexcept { return spec->grf_prop.grfid; }
template <typename Spec> inline uint16_t RustSpecLocalID(const Spec *spec) noexcept { return spec->grf_prop.local_id; }
template <typename Spec> inline OpenTTDGRFClassIdentity RustSpecIdentity(const Spec *spec) noexcept { return {RustSpecGRFID(spec), RustSpecLocalID(spec)}; }
template <typename Spec> inline const SpriteGroup *RustSpecSpriteGroup(const Spec *spec, bool live) noexcept { return spec->grf_prop.GetSpriteGroup(live); }
inline const SpriteGroup *RustHouseSpriteGroup(uint16_t house, bool live) noexcept { return RustSpecSpriteGroup(RustHouseSpec(house), live); }
inline uint16_t RustHouseLocalID(uint16_t house) noexcept { return RustSpecLocalID(RustHouseSpec(house)); }

template <typename Spec> inline size_t RustSpecCargoGroupCount(const Spec *spec) noexcept { return spec->grf_prop.spritegroups.size(); }
template <typename Spec> inline uint8_t RustSpecCargoGroupCargo(const Spec *spec, size_t index) noexcept { return spec->grf_prop.spritegroups[index].first; }
template <typename Spec> inline const SpriteGroup *RustSpecCargoSpriteGroup(const Spec *spec, uint8_t cargo) noexcept { return spec->grf_prop.GetSpriteGroup(cargo); }
inline uint8_t RustReverseRoadTypeTranslation(RoadType roadtype, const GRFFile *file) noexcept { return GetReverseRoadTypeTranslation(roadtype, file); }

/* Borrowed GRF files remain native and live for the synchronous conversion.
 * A town shell is live for resolution (or null for a prototype); no Rust
 * owner borrow spans it. No C++ exception crosses Rust; environmental
 * failures terminate. */
inline bool OpenTTDConvertBooleanCallback(const GRFFile *grf, uint16_t callback, uint16_t result) noexcept { return ConvertBooleanCallback(grf, callback, result); }
inline bool OpenTTDConvert8bitBooleanCallback(const GRFFile *grf, uint16_t callback, uint16_t result) noexcept { return Convert8bitBooleanCallback(grf, callback, result); }
inline bool RustHouseConvertBoolean(uint16_t house, uint16_t callback, uint16_t result) noexcept { return OpenTTDConvertBooleanCallback(RustHouseGrf(house), callback, result); }
inline bool RustHouseConvert8bitBoolean(uint16_t house, uint16_t callback, uint16_t result) noexcept { return OpenTTDConvert8bitBooleanCallback(RustHouseGrf(house), callback, result); }
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
	return OpenTTDConvertBooleanCallback(RustSpecGRFFile(spec), callback, result);
}
template <typename Spec, typename Handle, typename Result, typename Cost, StringID Error>
inline void RustLocationCallbackError(Cost *out, const Result *result, const Handle *handle) noexcept
{
	const Spec *spec = reinterpret_cast<const Spec *>(handle);
	*reinterpret_cast<CommandCost *>(out) = GetErrorMessageFromLocationCallbackResult(result->callback, result->registers, RustSpecGRFFile(spec), Error);
}
template <GrfSpecFeature Feature, typename Id>
inline void RustCloseInspect(Id id) noexcept { RustCloseNewGRFInspect(Feature, id); }
#endif /* RUST_SERVICES_NEWGRF_H */
