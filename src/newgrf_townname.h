/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file newgrf_townname.h
 * Header of Action 0F "universal holder" structure and functions
 */

#ifndef NEWGRF_TOWNNAME_H
#define NEWGRF_TOWNNAME_H

#include "strings_type.h"

extern "C" {
const StringID *openttd_rust_grf_townname_list(size_t *size);
}
/** Generator names in style order, valid until the next NewGRF load. */
inline std::span<const StringID> GetGRFTownNameList()
{
	size_t size;
	const StringID *names = openttd_rust_grf_townname_list(&size);
	return {names, size};
}
StringID GetGRFTownNameName(uint16_t gen);

#endif /* NEWGRF_TOWNNAME_H */
