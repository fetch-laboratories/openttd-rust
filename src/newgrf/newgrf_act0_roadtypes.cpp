/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_act0_roadtypes.cpp NewGRF Action 0x00 handler for roadtypes. */

#include "../stdafx.h"
#include "../debug.h"
#include "../road.h"
#include "newgrf_bytereader.h"
#include "newgrf_internal.h"
#include "newgrf_stringmapping.h"

#include "../rust/type_definitions_ffi.h"
#include "../rust/newgrf_bytereader_services.h"

#include "../safeguards.h"

static const OpenTTDRoadProperties &GetRustRoadProperties() noexcept
{
	static const OpenTTDRoadProperties services = {
		.toolbar_caption = [](RoadTypeInfo *info, uint16_t value) noexcept { AddStringForMapping(GRFStringID{value}, &info->strings.toolbar_caption); },
		.menu_text = [](RoadTypeInfo *info, uint16_t value) noexcept { AddStringForMapping(GRFStringID{value}, &info->strings.menu_text); },
		.build_caption = [](RoadTypeInfo *info, uint16_t value) noexcept { AddStringForMapping(GRFStringID{value}, &info->strings.build_caption); },
		.replace_text = [](RoadTypeInfo *info, uint16_t value) noexcept { AddStringForMapping(GRFStringID{value}, &info->strings.replace_text); },
		.name = [](RoadTypeInfo *info, uint16_t value) noexcept { AddStringForMapping(GRFStringID{value}, &info->strings.name); },
		.cost_multiplier = [](RoadTypeInfo *info, uint16_t value) noexcept { info->cost_multiplier = value; },
		.max_speed = [](RoadTypeInfo *info, uint16_t value) noexcept { info->max_speed = value; },
		.map_colour = [](RoadTypeInfo *info, uint8_t value) noexcept { info->map_colour = PixelColour{value}; },
		.introduction_date = [](RoadTypeInfo *info, int32_t value) noexcept { info->introduction_date = TimerGameCalendar::Date(value); },
		.sorting_order = [](RoadTypeInfo *info, uint8_t value) noexcept { info->sorting_order = value; },
		.maintenance_multiplier = [](RoadTypeInfo *info, uint16_t value) noexcept { info->maintenance_multiplier = value; },
		.alternate_label = [](RoadTypeInfo *info, uint32_t value) noexcept { info->alternate_labels.insert(value); },
		.new_engine = [](RoadTypeInfo *info, uint16_t value) noexcept { AddStringForMapping(GRFStringID{value}, &info->strings.new_engine); },
		.flags = [](RoadTypeInfo *info, uint8_t value) noexcept { info->flags = static_cast<RoadTypeFlags>(value); },
		.powered = [](RoadTypeInfo *info, RoadType value) noexcept { info->powered_roadtypes.Set(value); },
		.required = [](RoadTypeInfo *info, RoadType value) noexcept { info->introduction_required_roadtypes.Set(value); },
		.introduces = [](RoadTypeInfo *info, RoadType value) noexcept { info->introduces_roadtypes.Set(value); },
		.badges = [](RoadTypeInfo *info) noexcept { return &info->badges; },
		.invalid_reserve = [](uint32_t last, size_t size) noexcept { GrfMsg(1, "RoadTypeReserveInfo: Road type {} is invalid, max {}, ignoring", last, size); },
		.invalid_change = [](uint32_t last, size_t size) noexcept { GrfMsg(1, "RoadTypeChangeInfo: Road type {} is invalid, max {}, ignoring", last, size); },
		.no_label = [](uint32_t id) noexcept { GrfMsg(1, "RoadTypeReserveInfo: Ignoring property 1D for road type {} because no label was set", id); },
		.reserve_mismatch = [](uint32_t id) noexcept { GrfMsg(1, "RoadTypeReserveInfo: Road type {} is invalid type (road/tram), ignoring", id); },
		.power_mismatch = [](uint8_t resolved, uint8_t current) noexcept { GrfMsg(1, "RoadTypeChangeInfo: Powered road type list: Road type {} road/tram type does not match road type {}, ignoring", resolved, current); },
	};
	return services;
}
static ChangeInfoResult RoadTypeChangeInfo(uint first, uint last, int prop, ByteReader &buf, RoadTramType rtt)
{
	GRFFile *file = _cur_gps.grffile;
	auto &map = (rtt == RTT_TRAM) ? file->tramtype_map : file->roadtype_map;
	OpenTTDTypeMap view{.data = reinterpret_cast<uint8_t *>(map.data()), .size = map.size()};
	int32_t result = openttd_rust_road_change(&GetRustRoadProperties(), &GetRustByteReaderServices(), view, first, last, prop, &buf, rtt);
	if (result < 0) RustGRFRethrowReadError();
	return static_cast<ChangeInfoResult>(result);
}
static ChangeInfoResult RoadTypeReserveInfo(uint first, uint last, int prop, ByteReader &buf, RoadTramType rtt)
{
	GRFFile *file = _cur_gps.grffile;
	auto &map = (rtt == RTT_TRAM) ? file->tramtype_map : file->roadtype_map;
	OpenTTDTypeMap view{.data = reinterpret_cast<uint8_t *>(map.data()), .size = map.size()};
	int32_t result = openttd_rust_road_reserve(&GetRustRoadProperties(), &GetRustByteReaderServices(), view, first, last, prop, &buf, rtt);
	if (result < 0) RustGRFRethrowReadError();
	return static_cast<ChangeInfoResult>(result);
}

template <> ChangeInfoResult GrfChangeInfoHandler<GSF_ROADTYPES>::Reserve(uint first, uint last, int prop, ByteReader &buf) { return RoadTypeReserveInfo(first, last, prop, buf, RTT_ROAD); }
template <> ChangeInfoResult GrfChangeInfoHandler<GSF_ROADTYPES>::Activation(uint first, uint last, int prop, ByteReader &buf) { return RoadTypeChangeInfo(first, last, prop, buf, RTT_ROAD); }

template <> ChangeInfoResult GrfChangeInfoHandler<GSF_TRAMTYPES>::Reserve(uint first, uint last, int prop, ByteReader &buf) { return RoadTypeReserveInfo(first, last, prop, buf, RTT_TRAM); }
template <> ChangeInfoResult GrfChangeInfoHandler<GSF_TRAMTYPES>::Activation(uint first, uint last, int prop, ByteReader &buf) { return RoadTypeChangeInfo(first, last, prop, buf, RTT_TRAM); }
