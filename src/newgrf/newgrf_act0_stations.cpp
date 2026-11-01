/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_act0_stations.cpp NewGRF Action 0x00 handler for stations. */

#include "../stdafx.h"
#include "../debug.h"
#include "../newgrf_station.h"
#include "newgrf_bytereader.h"
#include "newgrf_internal.h"
#include "newgrf_stringmapping.h"

#include "../rust/station_definitions_ffi.h"
#include "../rust/newgrf_act2_ffi.h"
#include "../rust/newgrf_loading_ffi.h"
#include "../rust/newgrf_bytereader_services.h"

#include "../safeguards.h"

/** The maximum amount of stations a single GRF is allowed to add */

/**
 * Define properties for stations
 * @param first Local ID of the first station.
 * @param last Local ID of the last station.
 * @param prop The property to change.
 * @param buf The property value.
 * @return ChangeInfoResult.
 */
static ChangeInfoResult StationChangeInfo(uint first, uint last, int prop, ByteReader &buf)
{
	OpenTTDStationSequence sequence;
	uint8_t result;
	if (!openttd_rust_station_change_info(&GetStationDefinitionServices(), &GetRustAction2Services(), &GetRustGrfLoadingServices(), _cur_gps.grffile, first, last, prop, &buf, &sequence, &result)) RustGRFRethrowReadError();
	return static_cast<ChangeInfoResult>(result);
}

template <> ChangeInfoResult GrfChangeInfoHandler<GSF_STATIONS>::Reserve(uint, uint, int, ByteReader &) { return CIR_UNHANDLED; }
template <> ChangeInfoResult GrfChangeInfoHandler<GSF_STATIONS>::Activation(uint first, uint last, int prop, ByteReader &buf) { return StationChangeInfo(first, last, prop, buf); }
