/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_newgrf.h Canonical typed NewGRF result conversion leaves. */
#ifndef RUST_SERVICES_NEWGRF_H
#define RUST_SERVICES_NEWGRF_H
#include "../newgrf_commons.h"
#include "../command_type.h"
#include "../newgrf_debug.h"
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
	return ConvertBooleanCallback(spec->grf_prop.grffile, callback, result);
}
template <typename Spec, typename Handle, typename Result, typename Cost, StringID Error>
inline void RustLocationCallbackError(Cost *out, const Result *result, const Handle *handle) noexcept
{
	const Spec *spec = reinterpret_cast<const Spec *>(handle);
	*reinterpret_cast<CommandCost *>(out) = GetErrorMessageFromLocationCallbackResult(result->callback, result->registers, spec->grf_prop.grffile, Error);
}
template <GrfSpecFeature Feature, typename Id>
inline void RustCloseInspect(Id id) noexcept { DeleteNewGRFInspectWindow(Feature, id); }
#endif /* RUST_SERVICES_NEWGRF_H */
