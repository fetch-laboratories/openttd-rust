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
#ifdef WITH_RUST
#include "rust/train_state_ffi.h"
#endif

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

int GetTrainStopLocation(StationID station_id, TileIndex tile, const Train *v, int *station_ahead, int *station_length);

void GetTrainSpriteSize(EngineID engine, uint &width, uint &height, int &xoffs, int &yoffs, EngineImageType image_type);

bool TrainOnCrossing(TileIndex tile);
#ifndef WITH_RUST
void NormalizeTrainVehInDepot(const Train *u);
#endif

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

#ifdef WITH_RUST
/** The only C++ train-private cache is its shared NewGRF sprite pointer. */
struct TrainSpriteCache {
	const struct SpriteGroup *cached_override = nullptr;
};
#endif

/**
 * 'Train' is either a loco or a wagon.
 */
struct Train final : public GroundVehicle<Train, VEH_TRAIN> {
#ifdef WITH_RUST
	/* Canonical allocation destructs after PreDestructor, also on pool cleanup
	 * and indexed-load destruction. C++ retains only the sprite pointer cache. */
	std::unique_ptr<OpenTTDTrainState, decltype(&openttd_rust_train_state_destroy)> rust_state{openttd_rust_train_state_new(), openttd_rust_train_state_destroy};
	TrainSpriteCache tcache{};
#else
	VehicleRailFlags flags{};
	uint16_t crash_anim_pos = 0; ///< Crash animation counter.
	uint16_t wait_counter = 0; ///< Signal/stuck/forced-proceeding wait ticks.
	TrainCache tcache{};
	RailTypes compatible_railtypes{};
	RailTypes railtypes{};
	TrackBits track{};
	TrainForceProceeding force_proceed{};
#endif

	/* Link between the two ends of a multiheaded engine */
	Train *other_multiheaded_part = nullptr;

	/** We don't want GCC to zero our struct! It already is zeroed and has an index! */
	Train() : GroundVehicleBase() {}
	/** We want to 'destruct' the right class. */
	virtual ~Train() { this->PreDestructor(); }

#ifdef WITH_RUST
	OpenTTDTrainState *GetRustState() const { return this->rust_state.get(); }
#endif

	VehicleRailFlags GetTrainFlags() const
	{
#ifdef WITH_RUST
		return VehicleRailFlags(static_cast<uint16_t>(openttd_rust_train_state_get_flags(this->rust_state.get())));
#else
		return this->flags;
#endif
	}
	void SetTrainFlags(VehicleRailFlags value)
	{
#ifdef WITH_RUST
		openttd_rust_train_state_set_flags(this->rust_state.get(), value.base());
#else
		this->flags = value;
#endif
	}

	uint16_t GetCrashAnimPos() const
	{
#ifdef WITH_RUST
		return static_cast<uint16_t>(openttd_rust_train_state_get_crash_anim_pos(this->rust_state.get()));
#else
		return this->crash_anim_pos;
#endif
	}
	void SetCrashAnimPos(uint16_t value)
	{
#ifdef WITH_RUST
		openttd_rust_train_state_set_crash_anim_pos(this->rust_state.get(), value);
#else
		this->crash_anim_pos = value;
#endif
	}

	uint16_t GetWaitCounter() const
	{
#ifdef WITH_RUST
		return static_cast<uint16_t>(openttd_rust_train_state_get_wait_counter(this->rust_state.get()));
#else
		return this->wait_counter;
#endif
	}
	void SetWaitCounter(uint16_t value)
	{
#ifdef WITH_RUST
		openttd_rust_train_state_set_wait_counter(this->rust_state.get(), value);
#else
		this->wait_counter = value;
#endif
	}

	RailTypes GetCompatibleRailTypes() const
	{
#ifdef WITH_RUST
		return RailTypes(static_cast<uint64_t>(openttd_rust_train_state_get_compatible_railtypes(this->rust_state.get())));
#else
		return this->compatible_railtypes;
#endif
	}
	void SetCompatibleRailTypes(RailTypes value)
	{
#ifdef WITH_RUST
		openttd_rust_train_state_set_compatible_railtypes(this->rust_state.get(), value.base());
#else
		this->compatible_railtypes = value;
#endif
	}

	RailTypes GetRailTypes() const
	{
#ifdef WITH_RUST
		return RailTypes(static_cast<uint64_t>(openttd_rust_train_state_get_railtypes(this->rust_state.get())));
#else
		return this->railtypes;
#endif
	}
	void SetRailTypes(RailTypes value)
	{
#ifdef WITH_RUST
		openttd_rust_train_state_set_railtypes(this->rust_state.get(), value.base());
#else
		this->railtypes = value;
#endif
	}

	TrackBits GetTrack() const
	{
#ifdef WITH_RUST
		return static_cast<TrackBits>(openttd_rust_train_state_get_track(this->rust_state.get()));
#else
		return this->track;
#endif
	}
	void SetTrack(TrackBits value)
	{
#ifdef WITH_RUST
		openttd_rust_train_state_set_track(this->rust_state.get(), value);
#else
		this->track = value;
#endif
	}

	TrainForceProceeding GetForceProceed() const
	{
#ifdef WITH_RUST
		return static_cast<TrainForceProceeding>(openttd_rust_train_state_get_force_proceed(this->rust_state.get()));
#else
		return this->force_proceed;
#endif
	}
	void SetForceProceed(TrainForceProceeding value)
	{
#ifdef WITH_RUST
		openttd_rust_train_state_set_force_proceed(this->rust_state.get(), value);
#else
		this->force_proceed = value;
#endif
	}

	bool GetCachedTilt() const
	{
#ifdef WITH_RUST
		return static_cast<bool>(openttd_rust_train_state_get_cached_tilt(this->rust_state.get()));
#else
		return this->tcache.cached_tilt;
#endif
	}

	uint8_t GetUserDefData() const
	{
#ifdef WITH_RUST
		return static_cast<uint8_t>(openttd_rust_train_state_get_user_def_data(this->rust_state.get()));
#else
		return this->tcache.user_def_data;
#endif
	}

	int16_t GetCachedCurveSpeedMod() const
	{
#ifdef WITH_RUST
		return static_cast<int16_t>(static_cast<uint16_t>(openttd_rust_train_state_get_cached_curve_speed_mod(this->rust_state.get())));
#else
		return this->tcache.cached_curve_speed_mod;
#endif
	}

	uint16_t GetCachedMaxCurveSpeed() const
	{
#ifdef WITH_RUST
		return static_cast<uint16_t>(openttd_rust_train_state_get_cached_max_curve_speed(this->rust_state.get()));
#else
		return this->tcache.cached_max_curve_speed;
#endif
	}
	void SetCachedMaxCurveSpeed(uint16_t value)
	{
#ifdef WITH_RUST
		openttd_rust_train_state_set_cached_max_curve_speed(this->rust_state.get(), value);
#else
		this->tcache.cached_max_curve_speed = value;
#endif
	}

	void SetTrainFlag(VehicleRailFlag flag, bool value = true) { this->SetTrainFlags(this->GetTrainFlags().Set(flag, value)); }
	void ResetTrainFlag(VehicleRailFlag flag) { this->SetTrainFlags(this->GetTrainFlags().Reset(flag)); }
	void FlipTrainFlag(VehicleRailFlag flag) { this->SetTrainFlags(this->GetTrainFlags().Flip(flag)); }

	/** Copied diagnostic view; never a second authority for private cache state. */
	TrainCache CopyTrainCache() const
	{
		return {this->tcache.cached_override, this->GetCachedTilt(), this->GetUserDefData(), this->GetCachedCurveSpeedMod(), this->GetCachedMaxCurveSpeed()};
	}

#ifdef WITH_RUST
	uint RustDoUpdateSpeed(uint accel, int min_speed, int max_speed) { return this->DoUpdateSpeed(accel, min_speed, max_speed); }
#endif

	friend struct GroundVehicle<Train, VEH_TRAIN>; // GroundVehicle needs to use the acceleration functions defined at Train.

	void MarkDirty() override;
	void UpdateDeltaXY() override;
	ExpensesType GetExpenseType(bool income) const override { return income ? EXPENSES_TRAIN_REVENUE : EXPENSES_TRAIN_RUN; }
	void PlayLeaveStationSound(bool force = false) const override;
	bool IsPrimaryVehicle() const override { return this->IsFrontEngine(); }
	void GetImage(Direction direction, EngineImageType image_type, VehicleSpriteSeq *result) const override;
	int GetDisplaySpeed() const override { return this->gcache.last_speed; }
	int GetDisplayMaxSpeed() const override { return this->vcache.cached_max_speed; }
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

	uint16_t GetCurveSpeedLimit() const;

	void ConsistChanged(ConsistChangeFlags allowed_changes);

	int UpdateSpeed();

	void UpdateAcceleration();

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
#ifdef WITH_RUST
		return openttd_rust_ground_acceleration_type(&GetRustGroundPhysicsServices(), this);
#else
		return GetRailTypeInfo(GetRailType(this->tile))->acceleration_type;
#endif
	}

protected: // These functions should not be called outside acceleration code.

	/**
	 * Allows to know the power value that this vehicle will use.
	 * @return Power value from the engine in HP, or zero if the vehicle is not powered.
	 */
	inline uint16_t GetPower() const
	{
#ifdef WITH_RUST
		return openttd_rust_ground_power(&GetRustGroundPhysicsServices(), this);
#else
		/* Power is not added for articulated parts */
		if (!this->IsArticulatedPart() && HasPowerOnRail(this->GetRailTypes(), GetRailType(this->tile))) {
			uint16_t power = GetVehicleProperty(this, PROP_TRAIN_POWER, RailVehInfo(this->engine_type)->power);
			/* Halve power for multiheaded parts */
			if (this->IsMultiheaded()) power /= 2;
			return power;
		}

		return 0;
#endif
	}

	/**
	 * Returns a value if this articulated part is powered.
	 * @return Power value from the articulated part in HP, or zero if it is not powered.
	 */
	inline uint16_t GetPoweredPartPower(const Train *head) const
	{
#ifdef WITH_RUST
		return openttd_rust_ground_powered_part(&GetRustGroundPhysicsServices(), this, head);
#else
		/* For powered wagons the engine defines the type of engine (i.e. railtype) */
		if (this->GetTrainFlags().Test(VehicleRailFlag::PoweredWagon) && HasPowerOnRail(head->GetRailTypes(), GetRailType(this->tile))) {
			return RailVehInfo(this->gcache.first_engine)->pow_wag_power;
		}

		return 0;
#endif
	}

	/**
	 * Allows to know the weight value that this vehicle will use.
	 * @return Weight value from the engine in tonnes.
	 */
	inline uint16_t GetWeight() const
	{
#ifdef WITH_RUST
		return openttd_rust_ground_weight(&GetRustGroundPhysicsServices(), this, false);
#else
		uint16_t weight = CargoSpec::Get(this->cargo_type)->WeightOfNUnitsInTrain(this->cargo.StoredCount());

		/* Vehicle weight is not added for articulated parts. */
		if (!this->IsArticulatedPart()) {
			weight += GetVehicleProperty(this, PROP_TRAIN_WEIGHT, RailVehInfo(this->engine_type)->weight);
		}

		/* Powered wagons have extra weight added. */
		if (this->GetTrainFlags().Test(VehicleRailFlag::PoweredWagon)) {
			weight += RailVehInfo(this->gcache.first_engine)->pow_wag_weight;
		}

		return weight;
#endif
	}

	/**
	 * Calculates the weight value that this vehicle will have when fully loaded with its current cargo.
	 * @return Weight value in tonnes.
	 */
	uint16_t GetMaxWeight() const override;

	/**
	 * Allows to know the tractive effort value that this vehicle will use.
	 * @return Tractive effort value from the engine.
	 */
	inline uint8_t GetTractiveEffort() const
	{
#ifdef WITH_RUST
		return openttd_rust_ground_tractive_effort(&GetRustGroundPhysicsServices(), this);
#else
		return GetVehicleProperty(this, PROP_TRAIN_TRACTIVE_EFFORT, RailVehInfo(this->engine_type)->tractive_effort);
#endif
	}

	/**
	 * Gets the area used for calculating air drag.
	 * @return Area of the engine in m^2.
	 */
	inline uint8_t GetAirDragArea() const
	{
#ifdef WITH_RUST
		return openttd_rust_ground_air_drag_area(&GetRustGroundPhysicsServices(), this);
#else
		/* Air drag is higher in tunnels due to the limited cross-section. */
		return (this->GetTrack() == TRACK_BIT_WORMHOLE && this->vehstatus.Test(VehState::Hidden)) ? 28 : 14;
#endif
	}

	/**
	 * Gets the air drag coefficient of this vehicle.
	 * @return Air drag value from the engine.
	 */
	inline uint8_t GetAirDrag() const
	{
#ifdef WITH_RUST
		return openttd_rust_ground_air_drag(&GetRustGroundPhysicsServices(), this);
#else
		return RailVehInfo(this->engine_type)->air_drag;
#endif
	}

	/**
	 * Checks the current acceleration status of this vehicle.
	 * @return Acceleration status.
	 */
	inline AccelStatus GetAccelerationStatus() const
	{
#ifdef WITH_RUST
		return static_cast<AccelStatus>(openttd_rust_ground_acceleration_status(&GetRustGroundPhysicsServices(), this));
#else
		return this->vehstatus.Test(VehState::Stopped) || this->GetTrainFlags().Any({VehicleRailFlag::Reversing, VehicleRailFlag::Stuck}) ? AS_BRAKE : AS_ACCEL;
#endif
	}

	/**
	 * Calculates the current speed of this vehicle.
	 * @return Current speed in km/h-ish.
	 */
	inline uint16_t GetCurrentSpeed() const
	{
#ifdef WITH_RUST
		return openttd_rust_ground_current_speed(&GetRustGroundPhysicsServices(), this);
#else
		return this->cur_speed;
#endif
	}

	/**
	 * Returns the rolling friction coefficient of this vehicle.
	 * @return Rolling friction coefficient in [1e-4].
	 */
	inline uint32_t GetRollingFriction() const
	{
#ifdef WITH_RUST
		return openttd_rust_ground_rolling_friction(&GetRustGroundPhysicsServices(), this);
#else
		/* Rolling friction for steel on steel is between 0.1% and 0.2%.
		 * The friction coefficient increases with speed in a way that
		 * it doubles at 512 km/h, triples at 1024 km/h and so on. */
		return 15 * (512 + this->GetCurrentSpeed()) / 512;
#endif
	}

	/**
	 * Returns the slope steepness used by this vehicle.
	 * @return Slope steepness used by the vehicle.
	 */
	inline uint32_t GetSlopeSteepness() const
	{
#ifdef WITH_RUST
		return openttd_rust_ground_slope_steepness(&GetRustGroundPhysicsServices(), this);
#else
		return _settings_game.vehicle.train_slope_steepness;
#endif
	}

	/**
	 * Gets the maximum speed allowed by the track for this vehicle.
	 * @return Maximum speed allowed.
	 */
	inline uint16_t GetMaxTrackSpeed() const
	{
#ifdef WITH_RUST
		return openttd_rust_ground_max_track_speed(&GetRustGroundPhysicsServices(), this);
#else
		return GetRailTypeInfo(GetRailType(this->tile))->max_speed;
#endif
	}

	/**
	 * Returns the curve speed modifier of this vehicle.
	 * @return Current curve speed modifier, in fixed-point binary representation with 8 fractional bits.
	 */
	inline int16_t GetCurveSpeedModifier() const
	{
		return GetVehicleProperty(this, PROP_TRAIN_CURVE_SPEED_MOD, RailVehInfo(this->engine_type)->curve_speed_mod, true);
	}

	/**
	 * Checks if the vehicle is at a tile that can be sloped.
	 * @return True if the tile can be sloped.
	 */
	inline bool TileMayHaveSlopedTrack() const
	{
#ifdef WITH_RUST
		return openttd_rust_ground_sloped_track(&GetRustGroundPhysicsServices(), this);
#else
		/* Any track that isn't TRACK_BIT_X or TRACK_BIT_Y cannot be sloped. */
		return this->GetTrack() == TRACK_BIT_X || this->GetTrack() == TRACK_BIT_Y;
#endif
	}

	/**
	 * Trains can always use the faster algorithm because they
	 * have always the same direction as the track under them.
	 * @return false
	 */
	inline bool HasToUseGetSlopePixelZ()
	{
		return false;
	}
};

#ifdef WITH_RUST
/** Call-scoped scalar serialization staging. No Rust frame spans save/load or
 * reference fixups. Nested handlers restore the prior scope; partial loads
 * commit on unwind, matching direct mutation in the original field descriptors. */
class TrainStateScope {
	static inline TrainStateScope *active = nullptr;
	TrainStateScope *previous;
	Train *vehicle;
	bool loading;
	VehicleRailFlags flags;
	uint16_t crash_anim_pos;
	uint16_t wait_counter;
	TrackBits track;
	TrainForceProceeding force_proceed;
public:
	TrainStateScope(Train *v, bool loading) : previous(active), vehicle(v), loading(loading),
		flags(v->GetTrainFlags()), crash_anim_pos(v->GetCrashAnimPos()),
		wait_counter(v->GetWaitCounter()), track(v->GetTrack()),
		force_proceed(v->GetForceProceed()) { active = this; }
	~TrainStateScope()
	{
		if (this->loading) {
			this->vehicle->SetTrainFlags(this->flags);
			this->vehicle->SetCrashAnimPos(this->crash_anim_pos);
			this->vehicle->SetWaitCounter(this->wait_counter);
			this->vehicle->SetTrack(this->track);
			this->vehicle->SetForceProceed(this->force_proceed);
		}
		active = this->previous;
	}
	static VehicleRailFlags &Flags() { return active->flags; }
	static uint16_t &CrashAnimPos() { return active->crash_anim_pos; }
	static uint16_t &WaitCounter() { return active->wait_counter; }
	static TrackBits &Track() { return active->track; }
	static TrainForceProceeding &ForceProceed() { return active->force_proceed; }
};
#endif

#endif /* TRAIN_H */
