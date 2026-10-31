/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_act0_aircraft.cpp NewGRF Action 0x00 handler for aircraft. */

#include "../stdafx.h"
#include "../debug.h"
#include "../newgrf_cargo.h"
#include "../newgrf_engine.h"
#include "../newgrf_sound.h"
#include "newgrf_bytereader.h"
#include "newgrf_internal_vehicle.h"
#include "newgrf_internal.h"

#include "../rust/vehicle_properties_ffi.h"
#include "../rust/newgrf_bytereader_services.h"

#include "../safeguards.h"

/**
 * Define properties for aircraft
 * @param first Local ID of the first vehicle.
 * @param last Local ID of the last vehicle.
 * @param prop The property to change.
 * @param buf The property value.
 * @return ChangeInfoResult.
 */
static ChangeInfoResult AircraftVehicleChangeInfo(uint first, uint last, int prop, ByteReader &buf)
{
	int32_t result = openttd_rust_vehicle_properties(&GetRustVehicleProperties(), &GetRustByteReaderServices(), _cur_gps.grffile, VEH_AIRCRAFT, first, last, prop, &buf);
	if (result < 0) RustGRFRethrowReadError();
	return static_cast<ChangeInfoResult>(result);
}

template <> ChangeInfoResult GrfChangeInfoHandler<GSF_AIRCRAFT>::Reserve(uint, uint, int, ByteReader &) { return CIR_UNHANDLED; }
template <> ChangeInfoResult GrfChangeInfoHandler<GSF_AIRCRAFT>::Activation(uint first, uint last, int prop, ByteReader &buf) { return AircraftVehicleChangeInfo(first, last, prop, buf); }
