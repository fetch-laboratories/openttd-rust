/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file vehicle_properties_services.hpp Native property storage and validation leaves. */
#include "vehicle_properties_ffi.h"
#include "engine_services.h"
#include "../newgrf_sound.h"
#include "../newgrf/newgrf_internal_vehicle.h"
const OpenTTDVehicleProperties &GetRustVehicleProperties() noexcept
{
	static const OpenTTDVehicleProperties services{
		.loading = &GetRustEngineLoadingServices(),
		.id = RustEngineID,
		.badges = RustEngineBadges,
		.sound = [](const GRFFile *file, uint16_t sound) noexcept { return GetNewGRFSoundID(file, sound); },
		.train_image_valid = IsValidNewGRFImageIndex<VEH_TRAIN>,
		.road_image_valid = IsValidNewGRFImageIndex<VEH_ROAD>,
		.ship_image_valid = IsValidNewGRFImageIndex<VEH_SHIP>,
		.aircraft_image_valid = IsValidNewGRFImageIndex<VEH_AIRCRAFT>,
		.uses_2cc = [](bool value) noexcept { _loaded_newgrf_features.has_2CC |= value; },
	};
	return services;
}

static_assert(AIR_HELI == 0 && EC_DIESEL == 1 && EC_ELECTRIC == 2 && EC_MAGLEV == 4);
static_assert(RAILVEH_SINGLEHEAD == 0 && RAILVEH_MULTIHEAD == 1 && RAILVEH_WAGON == 2);
static_assert(RAILTYPE_LABEL_RAIL == 0x5241494c && RAILTYPE_LABEL_ELECTRIC == 0x454c524c && RAILTYPE_LABEL_MONO == 0x4d4f4e4f && RAILTYPE_LABEL_MAGLEV == 0x4d474c56);
static_assert(to_underlying(EngineMiscFlag::Uses2CC) == 1);
