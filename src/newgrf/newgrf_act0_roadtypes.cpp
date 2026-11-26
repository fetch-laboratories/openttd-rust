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

#include "../rust/type_definitions_ffi.h"
#include "../rust/newgrf_bytereader_services.h"

#include "../safeguards.h"

static ChangeInfoResult RoadTypeChangeInfo(uint first, uint last, int prop, ByteReader &buf, RoadTramType rtt)
{
	int32_t result = openttd_rust_road_change(&GetRustByteReaderServices(), _cur_gps.grffile, first, last, prop, &buf, rtt);
	if (result < 0) RustGRFRethrowReadError();
	return static_cast<ChangeInfoResult>(result);
}
static ChangeInfoResult RoadTypeReserveInfo(uint first, uint last, int prop, ByteReader &buf, RoadTramType rtt)
{
	int32_t result = openttd_rust_road_reserve(&GetRustByteReaderServices(), _cur_gps.grffile, first, last, prop, &buf, rtt);
	if (result < 0) RustGRFRethrowReadError();
	return static_cast<ChangeInfoResult>(result);
}

template <> ChangeInfoResult GrfChangeInfoHandler<GSF_ROADTYPES>::Reserve(uint first, uint last, int prop, ByteReader &buf) { return RoadTypeReserveInfo(first, last, prop, buf, RTT_ROAD); }
template <> ChangeInfoResult GrfChangeInfoHandler<GSF_ROADTYPES>::Activation(uint first, uint last, int prop, ByteReader &buf) { return RoadTypeChangeInfo(first, last, prop, buf, RTT_ROAD); }

template <> ChangeInfoResult GrfChangeInfoHandler<GSF_TRAMTYPES>::Reserve(uint first, uint last, int prop, ByteReader &buf) { return RoadTypeReserveInfo(first, last, prop, buf, RTT_TRAM); }
template <> ChangeInfoResult GrfChangeInfoHandler<GSF_TRAMTYPES>::Activation(uint first, uint last, int prop, ByteReader &buf) { return RoadTypeChangeInfo(first, last, prop, buf, RTT_TRAM); }
