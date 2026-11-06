/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file grf_bindings_ffi.h Native operations on retained NewGRF binding payloads. */
#ifndef RUST_GRF_BINDINGS_FFI_H
#define RUST_GRF_BINDINGS_FFI_H
#include "service_defaults.hpp"
#include "../newgrf.h"
struct OpenTTDEngine;
struct ObjectSpec;
#include "../engine_type.h"
struct OpenTTDGrfBindingServices {
	bool (*badge_bind)(GRFFile *, uint16_t, uint8_t, const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(badge_bind);
	StringID * (*badge_name)(GRFFile *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(badge_name);
	void (*canal_bind)(uint8_t, const GRFFile *, const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(canal_bind);
	void (*cargo_bind)(uint8_t, const GRFFile *, const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(cargo_bind);
	void (*mark_feature)(GRFFile *, GrfSpecFeature) noexcept OPENTTD_SERVICE_DEFAULT(mark_feature);
	ObjectSpec * (*object_spec)(GRFFile *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(object_spec);
	void (*rail_bind)(RailType, uint8_t, const GRFFile *, const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(rail_bind);
	void (*road_bind)(RoadType, uint8_t, const GRFFile *, const SpriteGroup *) noexcept OPENTTD_SERVICE_DEFAULT(road_bind);
};
const OpenTTDGrfBindingServices &GetRustGrfBindingServices() noexcept;
#endif
