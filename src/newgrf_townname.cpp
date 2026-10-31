/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file newgrf_townname.cpp
 * Implementation of  Action 0F "universal holder" structure and functions.
 * This file implements a linked-lists of townname generators,
 * holding everything that the newgrf action 0F will send over to OpenTTD.
 */

#include "stdafx.h"
#include "newgrf_townname.h"
#include "string_func.h"
#include "strings_internal.h"

#include "table/strings.h"

#include "safeguards.h"

#include "rust/byte_span_ffi.h"
#include "rust/services_ffi.h"

extern "C" void openttd_rust_grf_townname_generate(const OpenTTDSharedServices *, StringBuilder *builder, void (*append)(StringBuilder *, OpenTTDByteSpan) noexcept, uint32_t grfid, uint16_t gen, uint32_t seed);

void GRFTownNameGenerate(StringBuilder &builder, uint32_t grfid, uint16_t gen, uint32_t seed)
{
	openttd_rust_grf_townname_generate(&GetRustSharedServices(), &builder, [](StringBuilder *builder, OpenTTDByteSpan text) noexcept { *builder += std::string_view{reinterpret_cast<const char *>(text.data), text.size}; }, grfid, gen, seed);
}

StringID GetGRFTownNameName(uint16_t gen)
{
	auto names = GetGRFTownNameList();
	return gen < names.size() ? names[gen] : STR_UNDEFINED;
}
