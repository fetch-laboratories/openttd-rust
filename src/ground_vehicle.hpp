/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ground_vehicle.hpp Base class and functions for all vehicles that move through ground. */

#ifndef GROUND_VEHICLE_HPP
#define GROUND_VEHICLE_HPP

#include "vehicle_base.h"
#include "vehicle_gui.h"
#include "landscape.h"
#include "window_func.h"

#include "widgets/vehicle_widget.h"
#include "rust/ground_vehicle_ffi.h"

/** What is the status of our acceleration? */
enum AccelStatus : uint8_t {
	AS_ACCEL, ///< We want to go faster, if possible of course.
	AS_BRAKE, ///< We want to stop.
};

/**
 * Cached, frequently calculated values.
 * All of these values except cached_slope_resistance are set only for the first part of a vehicle.
 */
struct GroundVehicleCache {
	/* Cached acceleration values, recalculated when the cargo on a vehicle changes (in addition to the conditions below) */
	uint32_t cached_weight = 0; ///< Total weight of the consist (valid only for the first engine).
	uint32_t cached_slope_resistance = 0; ///< Resistance caused by weight when this vehicle part is at a slope.
	uint32_t cached_max_te = 0; ///< Maximum tractive effort of consist (valid only for the first engine).
	uint16_t cached_axle_resistance = 0; ///< Resistance caused by the axles of the vehicle (valid only for the first engine).

	/* Cached acceleration values, recalculated on load and each time a vehicle is added to/removed from the consist. */
	uint16_t cached_max_track_speed = 0; ///< Maximum consist speed (in internal units) limited by track type (valid only for the first engine).
	uint32_t cached_power = 0; ///< Total power of the consist (valid only for the first engine).
	uint32_t cached_air_drag = 0; ///< Air drag coefficient of the vehicle (valid only for the first engine).

	/* Cached NewGRF values, recalculated on load and each time a vehicle is added to/removed from the consist. */
	uint16_t cached_total_length = 0; ///< Length of the whole vehicle (valid only for the first engine).
	EngineID first_engine = EngineID::Invalid(); ///< Cached EngineID of the front vehicle. EngineID::Invalid() for the front vehicle itself.
	uint8_t cached_veh_length = 0; ///< Length of this vehicle in units of 1/VEHICLE_LENGTH of normal length. It is cached because this can be set by a callback.

	/* Cached UI information. */
	uint16_t last_speed = 0; ///< The last speed we did display, so we only have to redraw when this changes.

	auto operator<=>(const GroundVehicleCache &) const = default;
};

/** One canonical Rust allocation, shared by stable native reference aliases.
 * Construction (including indexed loading) and destruction follow the shell.
 * SaveLoad and legacy loaders address gv_flags through the alias, unchanged. */
struct OpenTTDGroundState { GroundVehicleCache cache; uint16_t flags = 0; };
extern "C" OpenTTDGroundState *openttd_rust_ground_new();
extern "C" void openttd_rust_ground_destroy(OpenTTDGroundState *);
struct GroundStateOwner {
	OpenTTDGroundState *state;
	GroundStateOwner() : state(openttd_rust_ground_new()) { new (this->state) OpenTTDGroundState(); }
	GroundStateOwner(const GroundStateOwner &) = delete;
	GroundStateOwner &operator=(const GroundStateOwner &) = delete;
	~GroundStateOwner() { openttd_rust_ground_destroy(this->state); }
};

/** Ground vehicle flags. */
enum GroundVehicleFlags : uint8_t {
	GVF_GOINGUP_BIT              = 0,  ///< Vehicle is currently going uphill. (Cached track information for acceleration)
	GVF_GOINGDOWN_BIT            = 1,  ///< Vehicle is currently going downhill. (Cached track information for acceleration)
	GVF_SUPPRESS_IMPLICIT_ORDERS = 2,  ///< Disable insertion and removal of automatic orders until the vehicle completes the real order.
};

/**
 * Base class for all vehicles that move through ground.
 *
 * Child classes must define all of the following functions.
 * These functions are not defined as pure virtual functions at this class to improve performance.
 *
 * virtual uint16_t      GetPower() const = 0;
 * virtual uint16_t      GetPoweredPartPower(const T *head) const = 0;
 * virtual uint16_t      GetWeight() const = 0;
 * virtual uint8_t        GetTractiveEffort() const = 0;
 * virtual uint8_t        GetAirDrag() const = 0;
 * virtual uint8_t        GetAirDragArea() const = 0;
 * virtual AccelStatus GetAccelerationStatus() const = 0;
 * virtual uint16_t      GetCurrentSpeed() const = 0;
 * virtual uint32_t      GetRollingFriction() const = 0;
 * virtual int         GetAccelerationType() const = 0;
 * virtual int32_t       GetSlopeSteepness() const = 0;
 * virtual int         GetDisplayMaxSpeed() const = 0;
 * virtual uint16_t      GetMaxTrackSpeed() const = 0;
 * virtual bool        TileMayHaveSlopedTrack() const = 0;
 */
template <class T, VehicleType Type>
struct GroundVehicle : public SpecializedVehicle<T, Type> {
	GroundStateOwner rust_ground;
	GroundVehicleCache &gcache = this->rust_ground.state->cache;
	uint16_t &gv_flags = this->rust_ground.state->flags;

	typedef GroundVehicle<T, Type> GroundVehicleBase; ///< Our type

	/**
	 * The constructor at SpecializedVehicle must be called.
	 */
	GroundVehicle() : SpecializedVehicle<T, Type>() {}

	void PowerChanged();
	void CargoChanged();
	int GetAcceleration() const;
	bool IsChainInDepot() const override;

	/**
	 * Common code executed for crashed ground vehicles
	 * @param flooded was this vehicle flooded?
	 * @return number of victims
	 */
	uint Crash(bool flooded) override
	{
		return openttd_rust_ground_crash(&GetRustGroundPhysicsServices(), this, flooded);
	}

	/**
	 * Calculates the total slope resistance for this vehicle.
	 * @return Slope resistance.
	 */

	/**
	 * Updates vehicle's Z position and inclination.
	 * Used when the vehicle entered given tile.
	 * @pre The vehicle has to be at (or near to) a border of the tile,
	 *      directed towards tile centre
	 */

	/**
	 * Updates vehicle's Z position.
	 * Inclination can't change in the middle of a tile.
	 * The faster code is used for trains and road vehicles unless they are
	 * reversing on a sloped tile.
	 */

	/**
	 * Checks if the vehicle is in a slope and sets the required flags in that case.
	 * @param new_tile True if the vehicle reached a new tile.
	 * @param update_delta Indicates to also update the delta.
	 * @return Old height of the vehicle.
	 */

	/**
	 * Set front engine state.
	 */
	inline void SetFrontEngine() { SetBit(this->Record().subtype, GVSF_FRONT); }

	/**
	 * Remove the front engine state.
	 */
	inline void ClearFrontEngine() { ClrBit(this->Record().subtype, GVSF_FRONT); }

	/**
	 * Set a vehicle to be an articulated part.
	 */
	inline void SetArticulatedPart() { SetBit(this->Record().subtype, GVSF_ARTICULATED_PART); }

	/**
	 * Clear a vehicle from being an articulated part.
	 */
	inline void ClearArticulatedPart() { ClrBit(this->Record().subtype, GVSF_ARTICULATED_PART); }

	/**
	 * Set a vehicle to be a wagon.
	 */
	inline void SetWagon() { SetBit(this->Record().subtype, GVSF_WAGON); }

	/**
	 * Clear wagon property.
	 */
	inline void ClearWagon() { ClrBit(this->Record().subtype, GVSF_WAGON); }

	/**
	 * Set engine status.
	 */
	inline void SetEngine() { SetBit(this->Record().subtype, GVSF_ENGINE); }

	/**
	 * Clear engine status.
	 */
	inline void ClearEngine() { ClrBit(this->Record().subtype, GVSF_ENGINE); }

	/**
	 * Set a vehicle as a free wagon.
	 */
	inline void SetFreeWagon() { SetBit(this->Record().subtype, GVSF_FREE_WAGON); }

	/**
	 * Clear a vehicle from being a free wagon.
	 */
	inline void ClearFreeWagon() { ClrBit(this->Record().subtype, GVSF_FREE_WAGON); }

	/**
	 * Set a vehicle as a multiheaded engine.
	 */
	inline void SetMultiheaded() { SetBit(this->Record().subtype, GVSF_MULTIHEADED); }

	/**
	 * Clear multiheaded engine property.
	 */
	inline void ClearMultiheaded() { ClrBit(this->Record().subtype, GVSF_MULTIHEADED); }

	/**
	 * Check if the vehicle is a free wagon (got no engine in front of it).
	 * @return Returns true if the vehicle is a free wagon.
	 */
	inline bool IsFreeWagon() const { return HasBit(this->Record().subtype, GVSF_FREE_WAGON); }

	/**
	 * Check if a vehicle is an engine (can be first in a consist).
	 * @return Returns true if vehicle is an engine.
	 */
	inline bool IsEngine() const { return HasBit(this->Record().subtype, GVSF_ENGINE); }

	/**
	 * Check if a vehicle is a wagon.
	 * @return Returns true if vehicle is a wagon.
	 */
	inline bool IsWagon() const { return HasBit(this->Record().subtype, GVSF_WAGON); }

	/**
	 * Check if the vehicle is a multiheaded engine.
	 * @return Returns true if the vehicle is a multiheaded engine.
	 */
	inline bool IsMultiheaded() const { return HasBit(this->Record().subtype, GVSF_MULTIHEADED); }

	/**
	 * Tell if we are dealing with the rear end of a multiheaded engine.
	 * @return True if the engine is the rear part of a dualheaded engine.
	 */
	inline bool IsRearDualheaded() const { return this->IsMultiheaded() && !this->IsEngine(); }

	/**
	 * Update the GUI variant of the current speed of the vehicle.
	 * Also mark the widget dirty when that is needed, i.e. when
	 * the speed of this vehicle has changed.
	 */

protected:
	/**
	 * Update the speed of the vehicle.
	 *
	 * It updates the cur_speed and subspeed variables depending on the state
	 * of the vehicle; in this case the current acceleration, minimum and
	 * maximum speeds of the vehicle. It returns the distance that that the
	 * vehicle can drive this tick. #Vehicle::GetAdvanceDistance() determines
	 * the distance to drive before moving a step on the map.
	 * @param accel     The acceleration we would like to give this vehicle.
	 * @param min_speed The minimum speed here, in vehicle specific units.
	 * @param max_speed The maximum speed here, in vehicle specific units.
	 * @return Distance to drive.
	 */
};

#endif /* GROUND_VEHICLE_HPP */
