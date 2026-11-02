/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file grf_bindings_ffi.h Native operations on retained NewGRF binding payloads. */
#ifndef RUST_GRF_BINDINGS_FFI_H
#define RUST_GRF_BINDINGS_FFI_H
#include "service_defaults.hpp"
#include "../newgrf.h"
struct OpenTTDEngine;
struct OpenTTDGrfBindingServices {
	StringID *(*badge_name)(GRFFile *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(badge_name);
	StringID *(*engine_string)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(engine_string);
	void (*new_name)(uint16_t, const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(new_name);
};
const OpenTTDGrfBindingServices &GetRustGrfBindingServices() noexcept;
#endif
