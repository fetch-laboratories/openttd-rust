/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file train.h Base for the train class. */

#ifndef TRAIN_H
#define TRAIN_H

#include "core/enum_type.hpp"

#include "newgrf_engine.h"
#include "cargotype.h"
#include "rail.h"
#include "engine_base.h"
#include "rail_map.h"
#include "ground_vehicle.hpp"
#include "rust/train_state_ffi.h"

struct Train;

/** Rail vehicle flags. */
enum VehicleRailFlag : uint8_t {
	Reversing = 0, ///< Train is slowing down to reverse.
	PoweredWagon = 3, ///< Wagon is powered.
	Flipped = 4, ///< Reverse the visible direction of the vehicle.

	AllowedOnNormalRail = 6, ///< Electric train engine is allowed to run on normal rail. */
	Reversed = 7, ///< Used for vehicle var 0xFE bit 8 (toggled each time the train is reversed, accurate for first vehicle only).
	Stuck = 8, ///< Train can't get a path reservation.
	LeavingStation = 9, ///< Train is just leaving a station.
};
using VehicleRailFlags = EnumBitSet<VehicleRailFlag, uint16_t>;

/** Modes for ignoring signals. */
enum TrainForceProceeding : uint8_t {
	TFP_NONE   = 0,    ///< Normal operation.
	TFP_STUCK  = 1,    ///< Proceed till next signal, but ignore being stuck till then. This includes force leaving depots.
	TFP_SIGNAL = 2,    ///< Ignore next signal, after the signal ignore being stuck.
};

/** Flags for Train::ConsistChanged */
enum class ConsistChangeFlag : uint8_t {
	Length, ///< Allow vehicles to change length.
	Capacity, ///< Allow vehicles to change capacity.
};

using ConsistChangeFlags = EnumBitSet<ConsistChangeFlag, uint8_t>;

static constexpr ConsistChangeFlags CCF_TRACK{}; ///< Valid changes while vehicle is driving, and possibly changing tracks.
static constexpr ConsistChangeFlags CCF_LOADUNLOAD{}; ///< Valid changes while vehicle is loading/unloading.
static constexpr ConsistChangeFlags CCF_AUTOREFIT{ConsistChangeFlag::Capacity}; ///< Valid changes for autorefitting in stations.
static constexpr ConsistChangeFlags CCF_REFIT{ConsistChangeFlag::Length, ConsistChangeFlag::Capacity}; ///< Valid changes for refitting in a depot.
static constexpr ConsistChangeFlags CCF_ARRANGE{ConsistChangeFlag::Length, ConsistChangeFlag::Capacity}; ///< Valid changes for arranging the consist in a depot.
static constexpr ConsistChangeFlags CCF_SAVELOAD{ConsistChangeFlag::Length}; ///< Valid changes when loading a savegame. (Everything that is not stored in the save.)

uint8_t FreightWagonMult(CargoType cargo);

void CheckTrainsLengths();

void FreeTrainTrackReservation(const Train *v);
bool TryPathReserve(Train *v, bool mark_as_stuck = false, bool first_tile_okay = false);


void GetTrainSpriteSize(EngineID engine, uint &width, uint &height, int &xoffs, int &yoffs, EngineImageType image_type);

bool TrainOnCrossing(TileIndex tile);

/** Variables that are cached to improve performance and such */
struct TrainCache {
	/* Cached wagon override spritegroup */
	const struct SpriteGroup *cached_override = nullptr;

	/* cached values, recalculated on load and each time a vehicle is added to/removed from the consist. */
	bool cached_tilt = false; ///< train can tilt; feature provides a bonus in curves
	uint8_t user_def_data = 0; ///< Cached property 0x25. Can be set by Callback 0x36.

	int16_t cached_curve_speed_mod = 0; ///< curve speed modifier of the entire train
	uint16_t cached_max_curve_speed = 0; ///< max consist speed limited by curves

	auto operator<=>(const TrainCache &) const = default;
};

/** The only C++ train-private cache is its shared NewGRF sprite pointer. */
struct TrainSpriteCache {
	const struct SpriteGroup *cached_override = nullptr;
};

/**
 * 'Train' is either a loco or a wagon.
 */
struct Train final : public GroundVehicle<Train, VEH_TRAIN> {
	/* Canonical allocation destructs after PreDestructor, also on pool cleanup
	 * and indexed-load destruction. C++ retains only the sprite pointer cache. */
	std::unique_ptr<OpenTTDTrainState, decltype(&openttd_rust_train_state_destroy)> rust_state{openttd_rust_train_state_new(), openttd_rust_train_state_destroy};
	TrainSpriteCache tcache{};

	/** Native facade over the canonical NOSAVE multihead record link. */
	Train *GetOtherMultiheadedPart() const { return static_cast<Train *>(VehicleFromRecord(this->rust_state->other_multiheaded_part)); }
	void SetOtherMultiheadedPart(Train *part) { this->rust_state->other_multiheaded_part = part == nullptr ? nullptr : part->record; }

	/** We don't want GCC to zero our struct! It already is zeroed and has an index! */
	Train() : GroundVehicleBase() { this->record->train = this->rust_state.get(); }
	/** We want to 'destruct' the right class. */
	virtual ~Train() { this->PreDestructor(); }

	OpenTTDTrainState *GetRustState() const { return this->rust_state.get(); }

	VehicleRailFlags GetTrainFlags() const
	{
		return VehicleRailFlags(this->rust_state->flags);
	}
	void SetTrainFlags(VehicleRailFlags value)
	{
		this->rust_state->flags = value.base();
	}

	uint16_t GetCrashAnimPos() const
	{
		return this->rust_state->crash_anim_pos;
	}
	void SetCrashAnimPos(uint16_t value)
	{
		this->rust_state->crash_anim_pos = value;
	}

	uint16_t GetWaitCounter() const
	{
		return this->rust_state->wait_counter;
	}
	void SetWaitCounter(uint16_t value)
	{
		this->rust_state->wait_counter = value;
	}

	RailTypes GetCompatibleRailTypes() const
	{
		return RailTypes(this->rust_state->compatible_railtypes);
	}
	void SetCompatibleRailTypes(RailTypes value)
	{
		this->rust_state->compatible_railtypes = value.base();
	}

	RailTypes GetRailTypes() const
	{
		return RailTypes(this->rust_state->railtypes);
	}
	void SetRailTypes(RailTypes value)
	{
		this->rust_state->railtypes = value.base();
	}

	TrackBits GetTrack() const
	{
		return static_cast<TrackBits>(this->rust_state->track);
	}
	void SetTrack(TrackBits value)
	{
		this->rust_state->track = value;
	}

	TrainForceProceeding GetForceProceed() const
	{
		return static_cast<TrainForceProceeding>(this->rust_state->force_proceed);
	}
	void SetForceProceed(TrainForceProceeding value)
	{
		this->rust_state->force_proceed = value;
	}

	bool GetCachedTilt() const
	{
		return this->rust_state->cached_tilt;
	}

	uint8_t GetUserDefData() const
	{
		return this->rust_state->user_def_data;
	}

	int16_t GetCachedCurveSpeedMod() const
	{
		return this->rust_state->cached_curve_speed_mod;
	}

	uint16_t GetCachedMaxCurveSpeed() const
	{
		return this->rust_state->cached_max_curve_speed;
	}

	void SetTrainFlag(VehicleRailFlag flag, bool value = true) { this->SetTrainFlags(this->GetTrainFlags().Set(flag, value)); }
	void ResetTrainFlag(VehicleRailFlag flag) { this->SetTrainFlags(this->GetTrainFlags().Reset(flag)); }
	void FlipTrainFlag(VehicleRailFlag flag) { this->SetTrainFlags(this->GetTrainFlags().Flip(flag)); }

	/** Copied diagnostic view of the private cache state. */
	TrainCache CopyTrainCache() const
	{
		return {this->tcache.cached_override, this->GetCachedTilt(), this->GetUserDefData(), this->GetCachedCurveSpeedMod(), this->GetCachedMaxCurveSpeed()};
	}


	friend struct GroundVehicle<Train, VEH_TRAIN>; // GroundVehicle needs to use the acceleration functions defined at Train.

	void MarkDirty() override;
	void UpdateDeltaXY() override;
	ExpensesType GetExpenseType(bool income) const override { return income ? EXPENSES_TRAIN_REVENUE : EXPENSES_TRAIN_RUN; }
	void PlayLeaveStationSound(bool force = false) const override;
	bool IsPrimaryVehicle() const override { return this->IsFrontEngine(); }
	void GetImage(Direction direction, EngineImageType image_type, VehicleSpriteSeq *result) const override;
	int GetDisplaySpeed() const override { return this->gcache.last_speed; }
	int GetDisplayMaxSpeed() const override { return this->Record().cached_max_speed; }
	Money GetRunningCost() const override;
	int GetCursorImageOffset() const;
	int GetDisplayImageWidth(Point *offset = nullptr) const;
	bool IsInDepot() const override { return this->GetTrack() == TRACK_BIT_DEPOT; }
	bool Tick() override;
	void OnNewCalendarDay() override;
	void OnNewEconomyDay() override;
	uint Crash(bool flooded = false) override;
	Trackdir GetVehicleTrackdir() const override;
	TileIndex GetOrderStationLocation(StationID station) override;
	ClosestDepot FindClosestDepot() override;

	void ReserveTrackUnderConsist() const;


	void ConsistChanged(ConsistChangeFlags allowed_changes);

	int UpdateSpeed();

	int GetCurrentMaxSpeed() const override;

	/**
	 * Get the next real (non-articulated part and non rear part of dualheaded engine) vehicle in the consist.
	 * @return Next vehicle in the consist.
	 */
	inline Train *GetNextUnit() const
	{
		Train *v = this->GetNextVehicle();
		if (v != nullptr && v->IsRearDualheaded()) v = v->GetNextVehicle();

		return v;
	}

	/**
	 * Get the previous real (non-articulated part and non rear part of dualheaded engine) vehicle in the consist.
	 * @return Previous vehicle in the consist.
	 */
	inline Train *GetPrevUnit()
	{
		Train *v = this->GetPrevVehicle();
		if (v != nullptr && v->IsRearDualheaded()) v = v->GetPrevVehicle();

		return v;
	}

	/**
	 * Calculate the offset from this vehicle's center to the following center taking the vehicle lengths into account.
	 * @return Offset from center to center.
	 */
	int CalcNextVehicleOffset() const
	{
		/* For vehicles with odd lengths the part before the center will be one unit
		 * longer than the part after the center. This means we have to round up the
		 * length of the next vehicle but may not round the length of the current
		 * vehicle. */
		return this->gcache.cached_veh_length / 2 + (this->Next() != nullptr ? this->Next()->gcache.cached_veh_length + 1 : 0) / 2;
	}

	/**
	 * Allows to know the acceleration type of a vehicle.
	 * @return Acceleration type of the vehicle.
	 */
	inline VehicleAccelerationModel GetAccelerationType() const
	{
		return openttd_rust_ground_acceleration_type(&GetRustGroundPhysicsServices(), this->record);
	}

protected: // These functions should not be called outside acceleration code.

	/**
	 * Allows to know the power value that this vehicle will use.
	 * @return Power value from the engine in HP, or zero if the vehicle is not powered.
	 */

	/**
	 * Returns a value if this articulated part is powered.
	 * @return Power value from the articulated part in HP, or zero if it is not powered.
	 */

	/**
	 * Allows to know the weight value that this vehicle will use.
	 * @return Weight value from the engine in tonnes.
	 */

	/**
	 * Calculates the weight value that this vehicle will have when fully loaded with its current cargo.
	 * @return Weight value in tonnes.
	 */
	uint16_t GetMaxWeight() const override;

	/**
	 * Allows to know the tractive effort value that this vehicle will use.
	 * @return Tractive effort value from the engine.
	 */

	/**
	 * Gets the area used for calculating air drag.
	 * @return Area of the engine in m^2.
	 */

	/**
	 * Gets the air drag coefficient of this vehicle.
	 * @return Air drag value from the engine.
	 */

	/**
	 * Checks the current acceleration status of this vehicle.
	 * @return Acceleration status.
	 */

	/**
	 * Calculates the current speed of this vehicle.
	 * @return Current speed in km/h-ish.
	 */

	/**
	 * Returns the rolling friction coefficient of this vehicle.
	 * @return Rolling friction coefficient in [1e-4].
	 */

	/**
	 * Returns the slope steepness used by this vehicle.
	 * @return Slope steepness used by the vehicle.
	 */

	/**
	 * Gets the maximum speed allowed by the track for this vehicle.
	 * @return Maximum speed allowed.
	 */

	/**
	 * Returns the curve speed modifier of this vehicle.
	 * @return Current curve speed modifier, in fixed-point binary representation with 8 fractional bits.
	 */
	inline int16_t GetCurveSpeedModifier() const
	{
		return GetVehicleProperty(this, PROP_TRAIN_CURVE_SPEED_MOD, RailVehInfo(this->GetEngineType())->curve_speed_mod, true);
	}

	/**
	 * Checks if the vehicle is at a tile that can be sloped.
	 * @return True if the tile can be sloped.
	 */

	/**
	 * Trains can always use the faster algorithm because they
	 * have always the same direction as the track under them.
	 * @return false
	 */
};

#endif /* TRAIN_H */
