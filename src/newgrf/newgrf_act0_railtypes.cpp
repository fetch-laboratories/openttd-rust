/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_act0_railtypes.cpp NewGRF Action 0x00 handler for railtypes. */

#include "../stdafx.h"
#include "../debug.h"
#include "../rail.h"
#include "newgrf_bytereader.h"
#include "newgrf_internal.h"
#include "newgrf_stringmapping.h"

#include "../rust/type_definitions_ffi.h"
#include "../rust/services_rail.h"
#include "../rust/newgrf_bytereader_services.h"

#include "../safeguards.h"

static const OpenTTDRailProperties &GetRustRailProperties() noexcept
{
	static const OpenTTDRailProperties services = {
		.toolbar_caption = [](RailTypeInfo *info, uint16_t value) noexcept { AddStringForMapping(GRFStringID{value}, &info->strings.toolbar_caption); },
		.menu_text = [](RailTypeInfo *info, uint16_t value) noexcept { AddStringForMapping(GRFStringID{value}, &info->strings.menu_text); },
		.build_caption = [](RailTypeInfo *info, uint16_t value) noexcept { AddStringForMapping(GRFStringID{value}, &info->strings.build_caption); },
		.replace_text = [](RailTypeInfo *info, uint16_t value) noexcept { AddStringForMapping(GRFStringID{value}, &info->strings.replace_text); },
		.name = [](RailTypeInfo *info, uint16_t value) noexcept { AddStringForMapping(GRFStringID{value}, &info->strings.name); },
		.cost_multiplier = RustSetRailCostMultiplier,
		.max_speed = [](RailTypeInfo *info, uint16_t value) noexcept { info->max_speed = value; },
		.map_colour = [](RailTypeInfo *info, uint8_t value) noexcept { info->map_colour = PixelColour{value}; },
		.introduction_date = [](RailTypeInfo *info, int32_t value) noexcept { info->introduction_date = TimerGameCalendar::Date(value); },
		.sorting_order = [](RailTypeInfo *info, uint8_t value) noexcept { info->sorting_order = value; },
		.maintenance_multiplier = [](RailTypeInfo *info, uint16_t value) noexcept { info->maintenance_multiplier = value; },
		.alternate_label = [](RailTypeInfo *info, uint32_t value) noexcept { info->alternate_labels.insert(value); },
		.new_engine = [](RailTypeInfo *info, uint16_t value) noexcept { AddStringForMapping(GRFStringID{value}, &info->strings.new_loco); },
		.flags = [](RailTypeInfo *info, uint8_t value) noexcept { info->flags = static_cast<RailTypeFlags>(value); },
		.powered = [](RailTypeInfo *info, RailType value) noexcept { info->powered_railtypes.Set(value); },
		.required = [](RailTypeInfo *info, RailType value) noexcept { info->introduction_required_railtypes.Set(value); },
		.introduces = [](RailTypeInfo *info, RailType value) noexcept { info->introduces_railtypes.Set(value); },
		.compatible = [](RailTypeInfo *info, RailType value) noexcept { info->compatible_railtypes.Set(value); },
		.curve_speed = [](RailTypeInfo *info, uint8_t value) noexcept { info->curve_speed = value; },
		.fallback = [](RailTypeInfo *info, uint8_t value) noexcept { info->fallback_railtype = value; },
		.acceleration = [](RailTypeInfo *info, uint8_t value) noexcept { info->acceleration_type = static_cast<VehicleAccelerationModel>(value); },
		.badges = [](RailTypeInfo *info) noexcept { return &info->badges; },
		.invalid_reserve = [](uint32_t last, size_t size) noexcept { GrfMsg(1, "RailTypeReserveInfo: Rail type {} is invalid, max {}, ignoring", last, size); },
		.invalid_change = [](uint32_t last, size_t size) noexcept { GrfMsg(1, "RailTypeChangeInfo: Rail type {} is invalid, max {}, ignoring", last, size); },
		.no_label = [](uint32_t id) noexcept { GrfMsg(1, "RailTypeReserveInfo: Ignoring property 1D for rail type {} because no label was set", id); },
	};
	return services;
}
static ChangeInfoResult RailTypeChangeInfo(uint first, uint last, int prop, ByteReader &buf)
{
	GRFFile *file = _cur_gps.grffile;
	auto &map = file->railtype_map;
	OpenTTDTypeMap view{.data = reinterpret_cast<uint8_t *>(map.data()), .size = map.size()};
	int32_t result = openttd_rust_rail_change(&GetRustRailProperties(), &GetRustByteReaderServices(), view, file->grf_version, first, last, prop, &buf);
	if (result < 0) RustGRFRethrowReadError();
	return static_cast<ChangeInfoResult>(result);
}
static ChangeInfoResult RailTypeReserveInfo(uint first, uint last, int prop, ByteReader &buf)
{
	GRFFile *file = _cur_gps.grffile;
	auto &map = file->railtype_map;
	OpenTTDTypeMap view{.data = reinterpret_cast<uint8_t *>(map.data()), .size = map.size()};
	int32_t result = openttd_rust_rail_reserve(&GetRustRailProperties(), &GetRustByteReaderServices(), view, first, last, prop, &buf);
	if (result < 0) RustGRFRethrowReadError();
	return static_cast<ChangeInfoResult>(result);
}

template <> ChangeInfoResult GrfChangeInfoHandler<GSF_RAILTYPES>::Reserve(uint first, uint last, int prop, ByteReader &buf) { return RailTypeReserveInfo(first, last, prop, buf); }
template <> ChangeInfoResult GrfChangeInfoHandler<GSF_RAILTYPES>::Activation(uint first, uint last, int prop, ByteReader &buf) { return RailTypeChangeInfo(first, last, prop, buf); }
