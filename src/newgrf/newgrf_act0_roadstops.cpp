/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_act0_roadstops.cpp NewGRF Action 0x00 handler for roadstops. */

#include "../stdafx.h"
#include "../debug.h"
#include "../newgrf_engine.h"
#include "../newgrf_roadstop.h"
#include "newgrf_bytereader.h"
#include "newgrf_internal.h"
#include "newgrf_stringmapping.h"

#include "../rust/station_definitions_ffi.h"
#include "../rust/newgrf_bytereader_services.h"

#include "../safeguards.h"

/**
 * Ignore properties for roadstops
 * @param prop The property to ignore.
 * @param buf The property value.
 * @return ChangeInfoResult.
 */

static ChangeInfoResult RoadStopChangeInfo(uint first, uint last, int prop, ByteReader &buf)
{
	uint8_t result;
	if (!openttd_rust_roadstop_change_info(&GetStationDefinitionServices(), _cur_gps.grffile, first, last, prop, &buf, &result)) RustGRFRethrowReadError();
	return static_cast<ChangeInfoResult>(result);
}

template <> ChangeInfoResult GrfChangeInfoHandler<GSF_ROADSTOPS>::Reserve(uint, uint, int, ByteReader &) { return CIR_UNHANDLED; }
template <> ChangeInfoResult GrfChangeInfoHandler<GSF_ROADSTOPS>::Activation(uint first, uint last, int prop, ByteReader &buf) { return RoadStopChangeInfo(first, last, prop, buf); }
