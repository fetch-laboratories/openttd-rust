/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_act0_houses.cpp NewGRF Action 0x00 handler for houses. */

#include "../stdafx.h"
#include "../debug.h"
#include "../town.h"
#include "../newgrf_cargo.h"
#include "../newgrf_house.h"
#include "newgrf_bytereader.h"
#include "newgrf_internal.h"
#include "newgrf_stringmapping.h"

#include "table/strings.h"

#include "../rust/house_definition_services.hpp"
#include "../rust/newgrf_bytereader_services.h"

#include "../safeguards.h"

static ChangeInfoResult TownHouseChangeInfo(uint first, uint last, int prop, ByteReader &buf)
{
	uint8_t result = openttd_rust_house_definition_property(&GetHouseDefinitionServices(), &GetRustByteReaderServices(), &GetRustGrfLoadingServices(), _cur_gps.grffile, first, last, prop, &buf);
	if (result == UINT8_MAX) RustGRFRethrowReadError();
	return static_cast<ChangeInfoResult>(result);
}

template <> ChangeInfoResult GrfChangeInfoHandler<GSF_HOUSES>::Reserve(uint, uint, int, ByteReader &) { return CIR_UNHANDLED; }
template <> ChangeInfoResult GrfChangeInfoHandler<GSF_HOUSES>::Activation(uint first, uint last, int prop, ByteReader &buf) { return TownHouseChangeInfo(first, last, prop, buf); }
