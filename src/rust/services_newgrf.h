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
#include "../newgrf_commons.h"
#include "../newgrf_debug.h"
#include "../newgrf_house.h"
#include "../industrytype.h"

/* Borrowed GRF files remain native and live for the synchronous conversion.
 * A town shell is live for resolution (or null for a prototype); no Rust
 * owner borrow spans it. No C++ exception crosses Rust; environmental
 * failures terminate. */
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
inline void OpenTTDDeleteIndustryInspect(uint16_t industry) noexcept { RustCloseNewGRFInspect(GSF_INDUSTRIES, industry); }
inline bool OpenTTDIndustryTileBooleanCallback(const IndustryTileSpec *spec, uint16_t callback, uint16_t result) noexcept { return OpenTTDConvertBooleanCallback(spec->grf_prop.grffile, callback, result); }
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
