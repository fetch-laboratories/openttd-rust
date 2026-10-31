/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_act0_industries.cpp NewGRF Action 0x00 handler for industries and industrytiles. */

#include "../stdafx.h"
#include "../debug.h"
#include "../newgrf_cargo.h"
#include "../industry.h"
#include "../industrytype.h"
#include "../industry_map.h"
#include "../newgrf_industries.h"
#include "newgrf_bytereader.h"
#include "newgrf_internal.h"
#include "newgrf_stringmapping.h"

#include "table/strings.h"
#include "../table/build_industry.h"

#include "../rust/industry_property_ffi.h"
#include "../rust/newgrf_loading_ffi.h"
#include "../rust/newgrf_bytereader_services.h"

#include "../safeguards.h"

static ChangeInfoResult IndustrytilesChangeInfo(uint first, uint last, int prop, ByteReader &buf)
{
	GRFFile *file = _cur_gps.grffile;
	const OpenTTDIndustryPropertyContext context{file, file->grfid, file->grf_version, to_underlying(_settings_game.game_creation.landscape)};
	int32_t result;
	if (!openttd_rust_industry_tile_properties(first, last, prop, &buf, &context, &GetRustByteReaderServices(), &GetIndustryDefinitionServices(), &GetIndustryPropertyServices(), &GetRustGrfLoadingServices(), &result)) RustGRFRethrowReadError();
	return static_cast<ChangeInfoResult>(result);
}

static ChangeInfoResult IndustriesChangeInfo(uint first, uint last, int prop, ByteReader &buf)
{
	GRFFile *file = _cur_gps.grffile;
	const OpenTTDIndustryPropertyContext context{file, file->grfid, file->grf_version, to_underlying(_settings_game.game_creation.landscape)};
	int32_t result;
	if (!openttd_rust_industry_properties(first, last, prop, &buf, &context, &GetRustByteReaderServices(), &GetIndustryDefinitionServices(), &GetIndustryPropertyServices(), &GetRustGrfLoadingServices(), &result)) RustGRFRethrowReadError();
	return static_cast<ChangeInfoResult>(result);
}

template <> ChangeInfoResult GrfChangeInfoHandler<GSF_INDUSTRYTILES>::Reserve(uint, uint, int, ByteReader &) { return CIR_UNHANDLED; }
template <> ChangeInfoResult GrfChangeInfoHandler<GSF_INDUSTRYTILES>::Activation(uint first, uint last, int prop, ByteReader &buf) { return IndustrytilesChangeInfo(first, last, prop, buf); }

template <> ChangeInfoResult GrfChangeInfoHandler<GSF_INDUSTRIES>::Reserve(uint, uint, int, ByteReader &) { return CIR_UNHANDLED; }
template <> ChangeInfoResult GrfChangeInfoHandler<GSF_INDUSTRIES>::Activation(uint first, uint last, int prop, ByteReader &buf) { return IndustriesChangeInfo(first, last, prop, buf); }
