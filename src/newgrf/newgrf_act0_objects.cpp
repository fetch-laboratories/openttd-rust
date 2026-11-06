/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_act0_objects.cpp NewGRF Action 0x00 handler for objects. */

#include "../stdafx.h"
#include "../debug.h"
#include "../newgrf_object.h"
#include "newgrf_bytereader.h"
#include "newgrf_internal.h"
#include "newgrf_stringmapping.h"

#include "../safeguards.h"

#include "../rust/newgrf_bytereader_services.h"
static void MapObjectString(uint16_t source, ObjectSpec *spec, bool class_name) noexcept
{
	if (class_name) {
		AddStringForMapping(GRFStringID{source}, [spec](StringID str) { ObjectClass::Get(spec->class_index)->name = str; });
	} else {
		AddStringForMapping(GRFStringID{source}, &spec->name);
	}
}
static ChangeInfoResult ObjectChangeInfo(uint first, uint last, int prop, ByteReader &buf)
{
	int32_t result;
	if (!openttd_rust_object_properties(&_cur_gps.grffile->objectspec, first, last, prop, &buf, &GetRustByteReaderServices(), MapObjectString, &_loaded_newgrf_features.has_2CC, &result)) RustGRFRethrowReadError();
	return static_cast<ChangeInfoResult>(result);
}

template <> ChangeInfoResult GrfChangeInfoHandler<GSF_OBJECTS>::Reserve(uint, uint, int, ByteReader &) { return CIR_UNHANDLED; }
template <> ChangeInfoResult GrfChangeInfoHandler<GSF_OBJECTS>::Activation(uint first, uint last, int prop, ByteReader &buf) { return ObjectChangeInfo(first, last, prop, buf); }
