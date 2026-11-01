/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file src/roadveh.h Road vehicle states */

#ifndef ROADVEH_H
#define ROADVEH_H

#include "ground_vehicle.hpp"
#include "engine_base.h"
#include "cargotype.h"
#include "track_func.h"
#include "road.h"
#include "road_map.h"
#include "newgrf_engine.h"
#include "rust/road_ffi.h"

struct RoadVehicle;

/** Road vehicle states */
enum RoadVehicleStates : uint8_t {
	/*
	 * Lower 4 bits are used for vehicle track direction. (Trackdirs)
	 * When in a road stop (bit 5 or bit 6 set) these bits give the
	 * track direction of the entry to the road stop.
	 * As the entry direction will always be a diagonal
	 * direction (X_NE, Y_SE, X_SW or Y_NW) only bits 0 and 3
	 * are needed to hold this direction. Bit 1 is then used to show
	 * that the vehicle is using the second road stop bay.
	 * Bit 2 is then used for drive-through stops to show the vehicle
	 * is stopping at this road stop.
	 */

	/* Numeric values */
	RVSB_IN_DEPOT                = 0xFE,                      ///< The vehicle is in a depot
	RVSB_WORMHOLE                = 0xFF,                      ///< The vehicle is in a tunnel and/or bridge

	/* Bit numbers */
	RVS_USING_SECOND_BAY         =    1,                      ///< Only used while in a road stop
	RVS_ENTERED_STOP             =    2,                      ///< Only set when a vehicle has entered the stop
	RVS_DRIVE_SIDE               =    4,                      ///< Only used when retrieving move data
	RVS_IN_ROAD_STOP             =    5,                      ///< The vehicle is in a road stop
	RVS_IN_DT_ROAD_STOP          =    6,                      ///< The vehicle is in a drive-through road stop

	/* Bit sets of the above specified bits */
	RVSB_IN_ROAD_STOP            = 1 << RVS_IN_ROAD_STOP,     ///< The vehicle is in a road stop
	RVSB_IN_ROAD_STOP_END        = RVSB_IN_ROAD_STOP + TRACKDIR_END,
	RVSB_IN_DT_ROAD_STOP         = 1 << RVS_IN_DT_ROAD_STOP,  ///< The vehicle is in a drive-through road stop
	RVSB_IN_DT_ROAD_STOP_END     = RVSB_IN_DT_ROAD_STOP + TRACKDIR_END,

	RVSB_DRIVE_SIDE              = 1 << RVS_DRIVE_SIDE,       ///< The vehicle is at the opposite side of the road

	RVSB_TRACKDIR_MASK           = 0x0F,                      ///< The mask used to extract track dirs
	RVSB_ROAD_STOP_TRACKDIR_MASK = 0x09,                      ///< Only bits 0 and 3 are used to encode the trackdir for road stops
};

/** State information about the Road Vehicle controller */
static const uint RDE_NEXT_TILE = 0x80; ///< We should enter the next tile
static const uint RDE_TURNED    = 0x40; ///< We just finished turning

/* Start frames for when a vehicle enters a tile/changes its state.
 * The start frame is different for vehicles that turned around or
 * are leaving the depot as the do not start at the edge of the tile.
 * For trams there are a few different start frames as there are two
 * places where trams can turn. */
static const uint RVC_DEFAULT_START_FRAME                =  0;
static const uint RVC_TURN_AROUND_START_FRAME            =  1;
static const uint RVC_DEPOT_START_FRAME                  =  6;
static const uint RVC_START_FRAME_AFTER_LONG_TRAM        = 21;
static const uint RVC_TURN_AROUND_START_FRAME_SHORT_TRAM = 16;
/* Stop frame for a vehicle in a drive-through stop */
static const uint RVC_DRIVE_THROUGH_STOP_FRAME           = 11;
static const uint RVC_DEPOT_STOP_FRAME                   = 11;

/** The number of ticks a vehicle has for overtaking. */
static const uint8_t RV_OVERTAKE_TIMEOUT = 35;

void RoadVehUpdateCache(RoadVehicle *v, bool same_length = false);
void GetRoadVehSpriteSize(EngineID engine, uint &width, uint &height, int &xoffs, int &yoffs, EngineImageType image_type);

/** Element of the RoadVehPathCache. */
struct RoadVehPathElement {
	Trackdir trackdir = INVALID_TRACKDIR; ///< Trackdir for this element.
	TileIndex tile = INVALID_TILE; ///< Tile for this element.

	constexpr RoadVehPathElement() {}
	constexpr RoadVehPathElement(Trackdir trackdir, TileIndex tile) : trackdir(trackdir), tile(tile) {}
};

using RoadVehPathCache = std::vector<RoadVehPathElement>;

/**
 * Buses, trucks and trams belong to this class.
 */
struct RoadVehicle final : public GroundVehicle<RoadVehicle, VEH_ROAD> {
	/* The shell owns one canonical allocation; members destruct after PreDestructor,
	 * including its early return during pool cleanup and indexed-load destruction. */
	std::unique_ptr<OpenTTDRoadState, decltype(&openttd_rust_road_destroy)> rust_state{openttd_rust_road_new(), openttd_rust_road_destroy};

	RoadType roadtype = INVALID_ROADTYPE; ///< NOSAVE: Roadtype of this vehicle.
	VehicleID disaster_vehicle = VehicleID::Invalid(); ///< NOSAVE: Disaster vehicle targetting this vehicle.
	RoadTypes compatible_roadtypes{}; ///< NOSAVE: Roadtypes this consist is powered on.

	/** We don't want GCC to zero our struct! It already is zeroed and has an index! */
	RoadVehicle() : GroundVehicleBase() {}
	/** We want to 'destruct' the right class. */
	virtual ~RoadVehicle() { this->PreDestructor(); }

	OpenTTDRoadState *GetRustState() const { return this->rust_state.get(); }

	uint8_t GetState() const
	{
		return static_cast<uint8_t>(openttd_rust_road_get(this->rust_state.get(), 0));
	}
	void SetState(uint8_t value)
	{
		openttd_rust_road_set(this->rust_state.get(), 0, value);
	}

	uint8_t GetFrame() const
	{
		return static_cast<uint8_t>(openttd_rust_road_get(this->rust_state.get(), 1));
	}
	void SetFrame(uint8_t value)
	{
		openttd_rust_road_set(this->rust_state.get(), 1, value);
	}

	uint16_t GetBlockedCounter() const
	{
		return static_cast<uint16_t>(openttd_rust_road_get(this->rust_state.get(), 2));
	}
	void SetBlockedCounter(uint16_t value)
	{
		openttd_rust_road_set(this->rust_state.get(), 2, value);
	}

	uint8_t GetOvertaking() const
	{
		return static_cast<uint8_t>(openttd_rust_road_get(this->rust_state.get(), 3));
	}
	void SetOvertaking(uint8_t value)
	{
		openttd_rust_road_set(this->rust_state.get(), 3, value);
	}

	uint8_t GetOvertakingCounter() const
	{
		return static_cast<uint8_t>(openttd_rust_road_get(this->rust_state.get(), 4));
	}
	void SetOvertakingCounter(uint8_t value)
	{
		openttd_rust_road_set(this->rust_state.get(), 4, value);
	}

	uint16_t GetCrashedCounter() const
	{
		return static_cast<uint16_t>(openttd_rust_road_get(this->rust_state.get(), 5));
	}
	void SetCrashedCounter(uint16_t value)
	{
		openttd_rust_road_set(this->rust_state.get(), 5, value);
	}

	uint8_t GetReverseCounter() const
	{
		return static_cast<uint8_t>(openttd_rust_road_get(this->rust_state.get(), 6));
	}
	void SetReverseCounter(uint8_t value)
	{
		openttd_rust_road_set(this->rust_state.get(), 6, value);
	}

	RoadVehPathCache CopyPath() const
	{
		RoadVehPathCache path;
		const size_t size = this->PathSize();
		path.reserve(size);
		for (size_t i = 0; i < size; ++i) {
			const auto element = openttd_rust_road_path_get(this->rust_state.get(), i);
			path.emplace_back(static_cast<Trackdir>(element.trackdir), TileIndex(element.tile));
		}
		return path;
	}
	void ReplacePath(const RoadVehPathCache &path)
	{
		std::vector<OpenTTDRoadPathElement> elements;
		elements.reserve(path.size());
		for (const auto &element : path) elements.push_back({static_cast<uint8_t>(element.trackdir), element.tile.base()});
		openttd_rust_road_path_replace(this->rust_state.get(), elements.data(), elements.size());
	}
	void ClearPath()
	{
		openttd_rust_road_path_clear(this->rust_state.get());
	}
	size_t PathSize() const
	{
		return openttd_rust_road_path_size(this->rust_state.get());
	}
	RoadVehPathElement PathBack() const
	{
		const auto element = openttd_rust_road_path_get(this->rust_state.get(), this->PathSize() - 1);
		return {static_cast<Trackdir>(element.trackdir), TileIndex(element.tile)};
	}
	void PathPush(Trackdir trackdir, TileIndex tile)
	{
		openttd_rust_road_path_push(this->rust_state.get(), {static_cast<uint8_t>(trackdir), tile.base()});
	}
	void PathPop()
	{
		openttd_rust_road_path_pop(this->rust_state.get());
	}

	friend struct GroundVehicle<RoadVehicle, VEH_ROAD>; // GroundVehicle needs to use the acceleration functions defined at RoadVehicle.

	void MarkDirty() override;
	void UpdateDeltaXY() override;
	ExpensesType GetExpenseType(bool income) const override { return income ? EXPENSES_ROADVEH_REVENUE : EXPENSES_ROADVEH_RUN; }
	bool IsPrimaryVehicle() const override { return this->IsFrontEngine(); }
	void GetImage(Direction direction, EngineImageType image_type, VehicleSpriteSeq *result) const override;
	int GetDisplaySpeed() const override { return this->gcache.last_speed / 2; }
	int GetDisplayMaxSpeed() const override { return this->Record().cached_max_speed / 2; }
	Money GetRunningCost() const override;
	int GetDisplayImageWidth(Point *offset = nullptr) const;
	bool IsInDepot() const override { return this->GetState() == RVSB_IN_DEPOT; }
	bool Tick() override;
	void OnNewCalendarDay() override;
	void OnNewEconomyDay() override;
	uint Crash(bool flooded = false) override;
	Trackdir GetVehicleTrackdir() const override;
	TileIndex GetOrderStationLocation(StationID station) override;
	ClosestDepot FindClosestDepot() override;

	bool IsBus() const;

	int GetCurrentMaxSpeed() const override;
	int UpdateSpeed();
	void SetDestTile(TileIndex tile) override;

protected: // These functions should not be called outside acceleration code.

	/**
	 * Allows to know the power value that this vehicle will use.
	 * @return Power value from the engine in HP, or zero if the vehicle is not powered.
	 */

	/**
	 * Returns a value if this articulated part is powered.
	 * @return Zero, because road vehicles don't have powered parts.
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
	 * Allows to know the acceleration type of a vehicle.
	 * @return \c VehicleAccelerationModel::Normal, road vehicles always use a normal acceleration method.
	 */

	/**
	 * Returns the slope steepness used by this vehicle.
	 * @return Slope steepness used by the vehicle.
	 */

	/**
	 * Gets the maximum speed allowed by the track for this vehicle.
	 * @return Since roads don't limit road vehicle speed, it returns always zero.
	 */

	/**
	 * Checks if the vehicle is at a tile that can be sloped.
	 * @return True if the tile can be sloped.
	 */

	/**
	 * Road vehicles have to use GetSlopePixelZ() to compute their height
	 * if they are reversing because in that case, their direction
	 * is not parallel with the road. It is safe to return \c true
	 * even if it is not reversing.
	 * @return are we (possibly) reversing?
	 */
};

/** Call-scoped serialization staging. No Rust frame spans save/load, callbacks or
 * reference fixups. Nested handlers restore the previous scope; partial loads
 * commit on unwind, matching direct field/vector mutation in the portable build. */
class RoadVehicleStateScope {
	static inline RoadVehicleStateScope *active = nullptr;
	RoadVehicleStateScope *previous;
	RoadVehicle *vehicle;
	bool loading;
	uint8_t state;
	uint8_t frame;
	uint16_t blocked_ctr;
	uint8_t overtaking;
	uint8_t overtaking_ctr;
	uint16_t crashed_ctr;
	uint8_t reverse_ctr;
	RoadVehPathCache path;
	std::vector<Trackdir> path_td;
	std::vector<TileIndex> path_tile;
public:
	RoadVehicleStateScope(RoadVehicle *v, bool loading) : previous(active), vehicle(v), loading(loading),
		state(v->GetState()),
		frame(v->GetFrame()),
		blocked_ctr(v->GetBlockedCounter()),
		overtaking(v->GetOvertaking()),
		overtaking_ctr(v->GetOvertakingCounter()),
		crashed_ctr(v->GetCrashedCounter()),
		reverse_ctr(v->GetReverseCounter()),
		path(v->CopyPath()) { active = this; }
	~RoadVehicleStateScope()
	{
		if (this->loading) {
			this->vehicle->SetState(this->state);
			this->vehicle->SetFrame(this->frame);
			this->vehicle->SetBlockedCounter(this->blocked_ctr);
			this->vehicle->SetOvertaking(this->overtaking);
			this->vehicle->SetOvertakingCounter(this->overtaking_ctr);
			this->vehicle->SetCrashedCounter(this->crashed_ctr);
			this->vehicle->SetReverseCounter(this->reverse_ctr);
			this->vehicle->ReplacePath(this->path);
		}
		active = this->previous;
	}
	static uint8_t &State() { return active->state; }
	static uint8_t &Frame() { return active->frame; }
	static uint16_t &BlockedCounter() { return active->blocked_ctr; }
	static uint8_t &Overtaking() { return active->overtaking; }
	static uint8_t &OvertakingCounter() { return active->overtaking_ctr; }
	static uint16_t &CrashedCounter() { return active->crashed_ctr; }
	static uint8_t &ReverseCounter() { return active->reverse_ctr; }
	static RoadVehPathCache &Path() { return active->path; }
	static std::vector<Trackdir> &PathTrackdirs() { return active->path_td; }
	static std::vector<TileIndex> &PathTiles() { return active->path_tile; }
};

#endif /* ROADVEH_H */
