/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_act0_airports.cpp NewGRF Action 0x00 handler for airports. */

#include "../stdafx.h"
#include "../debug.h"
#include "../newgrf_airporttiles.h"
#include "../newgrf_airport.h"
#include "newgrf_bytereader.h"
#include "newgrf_internal.h"
#include "newgrf_stringmapping.h"

#include "../rust/airport_definitions_ffi.h"
#include "../rust/newgrf_bytereader_services.h"

#include "../safeguards.h"

/**
 * Define properties for airports
 * @param first Local ID of the first airport.
 * @param last Local ID of the last airport.
 * @param prop The property to change.
 * @param buf The property value.
 * @return ChangeInfoResult.
 */
static ChangeInfoResult AirportChangeInfo(uint first, uint last, int prop, ByteReader &buf)
{
	uint8_t result;
	if (!openttd_rust_airport_change_info(&GetAirportDefinitionServices(), _cur_gps.grffile, first, last, prop, &buf, &result)) RustGRFRethrowReadError();
	return static_cast<ChangeInfoResult>(result);
}

static ChangeInfoResult AirportTilesChangeInfo(uint first, uint last, int prop, ByteReader &buf)
{
	uint8_t result;
	if (!openttd_rust_airport_tile_change_info(&GetAirportDefinitionServices(), _cur_gps.grffile, first, last, prop, &buf, &result)) RustGRFRethrowReadError();
	return static_cast<ChangeInfoResult>(result);
}

template <> ChangeInfoResult GrfChangeInfoHandler<GSF_AIRPORTS>::Reserve(uint, uint, int, ByteReader &) { return CIR_UNHANDLED; }
template <> ChangeInfoResult GrfChangeInfoHandler<GSF_AIRPORTS>::Activation(uint first, uint last, int prop, ByteReader &buf) { return AirportChangeInfo(first, last, prop, buf); }

template <> ChangeInfoResult GrfChangeInfoHandler<GSF_AIRPORTTILES>::Reserve(uint, uint, int, ByteReader &) { return CIR_UNHANDLED; }
template <> ChangeInfoResult GrfChangeInfoHandler<GSF_AIRPORTTILES>::Activation(uint first, uint last, int prop, ByteReader &buf) { return AirportTilesChangeInfo(first, last, prop, buf); }
