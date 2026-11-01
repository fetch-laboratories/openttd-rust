/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_class_func.h Implementation of the NewGRF class' functions. */

#include "newgrf_class.h"

#include "table/strings.h"

#include "rust/object_resolver_ffi.h"
#include "rust/services_newgrf.h"

template <typename Tspec, typename Tindex, Tindex Tmax>
const OpenTTDGRFClassServices<Tspec> &NewGRFClass<Tspec, Tindex, Tmax>::GetServices()
{
	static const auto defaults = NewGRFClass::Defaults();
	static const OpenTTDGRFClassServices<Tspec> services{
		.class_index = [](const Tspec *spec) noexcept { return static_cast<uint16_t>(spec->class_index); },
		.set_index = [](Tspec *spec, uint16_t index) noexcept { spec->index = index; },
		.identity = RustSpecIdentity<Tspec>,
		.exhausted = [](uint16_t max) noexcept { GrfMsg(2, "ClassAllocate: already allocated {} classes, using default", max); },
		.object = []() noexcept -> const OpenTTDObjectServices * { if constexpr (std::is_same_v<Tspec, ObjectSpec>) return &GetRustObjectServices(); else return nullptr; }(),
		.defaults = defaults.data(),
		.default_count = defaults.size(),
		.maximum = Tmax,
		.empty = STR_EMPTY,
		.insert_empty = std::is_same_v<Tspec, StationSpec> || std::is_same_v<Tspec, RoadStopSpec>,
	};
	return services;
}

template <typename Tspec, typename Tindex, Tindex Tmax>
void NewGRFClass<Tspec, Tindex, Tmax>::Reset()
{
	Ops::reset(&NewGRFClass::GetServices());
}

template <typename Tspec, typename Tindex, Tindex Tmax>
Tindex NewGRFClass<Tspec, Tindex, Tmax>::Allocate(uint32_t global_id)
{
	return static_cast<Tindex>(Ops::allocate(global_id, Tmax, STR_EMPTY));
}
