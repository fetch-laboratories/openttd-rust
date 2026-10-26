/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_act3_ffi.h Shared Action3 cargo translation diagnostics. */
#ifndef RUST_NEWGRF_ACT3_FFI_H
#define RUST_NEWGRF_ACT3_FFI_H
#include "service_defaults.hpp"
#include "../cargo_type.h"
#include "../newgrf.h"
struct GRFFile;
struct OpenTTDAct3CargoServices {
	void (*out_of_range)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(out_of_range);
	void (*not_available)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(not_available);
	void (*unsupported)(CargoLabel) noexcept OPENTTD_SERVICE_DEFAULT(unsupported);
	void (*mapped)(CargoLabel, CargoType) noexcept OPENTTD_SERVICE_DEFAULT(mapped);
};
const OpenTTDAct3CargoServices &GetAct3CargoServices() noexcept;
extern "C" CargoType openttd_rust_act3_translate_cargo(const OpenTTDAct3CargoServices *, GrfSpecFeature, uint8_t, const GRFFile *);
#endif
