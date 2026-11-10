/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file town_name_ffi.h Town name policy and borrowed rendering frames. */
#ifndef RUST_TOWN_NAME_FFI_H
#define RUST_TOWN_NAME_FFI_H
#include "service_defaults.hpp"
#include <string>
#include <set>
#include "town_ffi.h"
#include "services_ffi.h"
#include "byte_span_ffi.h"
#include "../townname_type.h"
struct Randomizer;
struct OpenTTDTownNames;
struct OpenTTDTownNameFrame { std::string candidate, other; };
/** Rendering owns its ordinary string allocation in these call-local native
 * objects. Candidate bytes survive other-name rendering; no view survives its
 * own next render. Only resolver services run here; they cannot unwind. */
struct OpenTTDTownNameServices {
	OpenTTDTownRef (*next)(uint) noexcept OPENTTD_SERVICE_DEFAULT(next);
	OpenTTDByteSpan (*render)(OpenTTDTownNameFrame *, const TownNameParams *, uint32_t, bool other) noexcept OPENTTD_SERVICE_DEFAULT(render);
	const OpenTTDSharedServices *shared;
	uint32_t (*random_next)(Randomizer *) noexcept OPENTTD_SERVICE_DEFAULT(random_next);
};
const OpenTTDTownNameServices &GetRustTownNameServices() noexcept;
extern "C" {
uint8_t openttd_rust_town_verify_name_abi(const OpenTTDTownNameServices *, OpenTTDTownNameFrame *, uint32_t, const TownNameParams *, OpenTTDTownNames *);
extern "C++" inline bool openttd_rust_town_verify_name(const OpenTTDTownNameServices *a0, OpenTTDTownNameFrame *a1, uint32_t a2, const TownNameParams *a3, OpenTTDTownNames *a4) { return openttd_rust_town_verify_name_abi(a0, a1, a2, a3, a4) != 0; }
uint8_t openttd_rust_town_generate_name_abi(const OpenTTDTownNameServices *, OpenTTDTownNameFrame *, Randomizer *, uint32_t *, const TownNameParams *, OpenTTDTownNames *);
extern "C++" inline bool openttd_rust_town_generate_name(const OpenTTDTownNameServices *a0, OpenTTDTownNameFrame *a1, Randomizer *a2, uint32_t *a3, const TownNameParams *a4, OpenTTDTownNames *a5) { return openttd_rust_town_generate_name_abi(a0, a1, a2, a3, a4, a5) != 0; }
OpenTTDTownNames *openttd_rust_town_names_new();
void openttd_rust_town_names_destroy(OpenTTDTownNames *);
void openttd_rust_town_names_insert(OpenTTDTownNames *, OpenTTDByteSpan);
void openttd_rust_town_names_clear(OpenTTDTownNames *);
}
#endif /* RUST_TOWN_NAME_FFI_H */
