/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_act3_ffi.h Shared Action3 cargo translation. */
#ifndef RUST_NEWGRF_ACT3_FFI_H
#define RUST_NEWGRF_ACT3_FFI_H
#include "service_defaults.hpp"
#include "../cargo_type.h"
#include "../newgrf.h"
struct GRFFile;
extern "C" CargoType openttd_rust_act3_translate_cargo(GrfSpecFeature, uint8_t, const GRFFile *);
#endif
