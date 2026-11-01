/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file disaster_vehicle.h All disaster vehicles. */

#ifndef DISASTER_VEHICLE_H
#define DISASTER_VEHICLE_H

#include "vehicle_base.h"
#include "rust/disaster_ffi.h"
#include <memory>

/** Different sub types of disaster vehicles. */
enum DisasterSubType : uint8_t {
	ST_ZEPPELINER,               ///< Zeppelin, crashes at airports.
	ST_ZEPPELINER_SHADOW,        ///< Shadow of the zeppelin.
	ST_SMALL_UFO,                ///< Small UFO, tries to find a road vehicle to destroy.
	ST_SMALL_UFO_SHADOW,         ///< Shadow of small UFO
	ST_AIRPLANE,                 ///< Airplane destroying an oil refinery
	ST_AIRPLANE_SHADOW,          ///< Shadow of airplane
	ST_HELICOPTER,               ///< Helicopter destroying a factory.
	ST_HELICOPTER_SHADOW,        ///< Shadow of helicopter.
	ST_HELICOPTER_ROTORS,        ///< Rotors of helicopter.
	ST_BIG_UFO,                  ///< Big UFO, finds a piece of railroad to "park" on
	ST_BIG_UFO_SHADOW,           ///< Shadow of the big UFO
	ST_BIG_UFO_DESTROYER,        ///< Aircraft the will bomb the big UFO
	ST_BIG_UFO_DESTROYER_SHADOW, ///< Shadow of the aircraft.
	ST_SMALL_SUBMARINE,          ///< Small submarine, pops up in the oceans but doesn't do anything
	ST_BIG_SUBMARINE,            ///< Big submarine, pops up in the oceans but doesn't do anything
};

/**
 * Disasters, like submarines, skyrangers and their shadows, belong to this class.
 */
struct DisasterVehicle final : public SpecializedVehicle<DisasterVehicle, VEH_DISASTER> {
	struct RustStateDeleter {
		void operator()(OpenTTDDisasterState *state) const { openttd_rust_disaster_state_destroy(state); }
	};
	std::unique_ptr<OpenTTDDisasterState, RustStateDeleter> rust_state{openttd_rust_disaster_state_create(VehicleID::Invalid().base())};
	SpriteID &ImageOverride() { return this->rust_state->image_override; }
	uint32_t &DestroyerTarget() { return this->rust_state->target; }
	uint8_t &Flags() { return this->rust_state->flags; }
	uint16_t &State() { return this->rust_state->state; }

	/** For use by saveload. */
	DisasterVehicle() : SpecializedVehicleBase() {}
	DisasterVehicle(int x, int y, Direction direction, DisasterSubType subtype, VehicleID big_ufo_destroyer_target = VehicleID::Invalid());
	/** We want to 'destruct' the right class. */
	virtual ~DisasterVehicle() = default;

	void UpdateDeltaXY() override;
	bool Tick() override;
};

#endif /* DISASTER_VEHICLE_H */
