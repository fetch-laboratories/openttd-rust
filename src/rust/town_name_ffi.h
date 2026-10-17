/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file town_name_ffi.h Town name policy and borrowed rendering frames. */
#ifndef RUST_TOWN_NAME_FFI_H
#define RUST_TOWN_NAME_FFI_H
#include <string>
#include <set>
#include "town_ffi.h"
#include "byte_span_ffi.h"
#include "../townname_type.h"
struct Randomizer;
struct OpenTTDTownNames;
struct OpenTTDTownNameFrame { std::string candidate, other; };
/** Pool IDs cross as their uint16_t wire value; native PoolID classes stay native. */
/** Rendering owns its ordinary string allocation in these call-local native
 * objects. Candidate bytes survive other-name rendering; no view survives its
 * own next render. Only resolver services run here; they cannot unwind. */
struct OpenTTDTownNameServices {
	OpenTTDTownRef (*next)(uint) noexcept;
	OpenTTDByteSpan (*render)(OpenTTDTownNameFrame *, const TownNameParams *, uint32_t) noexcept;
	OpenTTDByteSpan (*render_other)(OpenTTDTownNameFrame *, const Town *) noexcept;
	size_t (*utf8_length)(const uint8_t *, size_t) noexcept;
	uint32_t (*random_next)(Randomizer *) noexcept;
};
const OpenTTDTownNameServices &GetRustTownNameServices() noexcept;
extern "C" {
bool openttd_rust_town_verify_name(const OpenTTDTownNameServices *, OpenTTDTownNameFrame *, uint32_t, const TownNameParams *, OpenTTDTownNames *);
bool openttd_rust_town_generate_name(const OpenTTDTownNameServices *, OpenTTDTownNameFrame *, Randomizer *, uint32_t *, const TownNameParams *, OpenTTDTownNames *);
OpenTTDTownNames *openttd_rust_town_names_new();
void openttd_rust_town_names_destroy(OpenTTDTownNames *);
void openttd_rust_town_names_insert(OpenTTDTownNames *, OpenTTDByteSpan);
void openttd_rust_town_names_clear(OpenTTDTownNames *);
}
#endif /* RUST_TOWN_NAME_FFI_H */
