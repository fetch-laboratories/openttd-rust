/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ship.h Base for ships. */

#ifndef SHIP_H
#define SHIP_H

#include "vehicle_base.h"
#include "water_map.h"

void GetShipSpriteSize(EngineID engine, uint &width, uint &height, int &xoffs, int &yoffs, EngineImageType image_type);

/** Element of the ShipPathCache. */
struct ShipPathElement {
	Trackdir trackdir = INVALID_TRACKDIR; ///< Trackdir for this element.

	constexpr ShipPathElement() {}
	constexpr ShipPathElement(Trackdir trackdir) : trackdir(trackdir) {}
};

#include "rust/ship_yapf_ffi.h"
#include "rust/ship_control_ffi.h"
/** A view of the cached path that the ship's Rust owner holds, for save/load. */
class ShipPathCache {
	OpenTTDShipPath *owner;
public:
	explicit ShipPathCache(OpenTTDShipPath *owner) : owner(owner) {}
	ShipPathCache(const ShipPathCache &) = delete;
	ShipPathCache &operator=(const ShipPathCache &) = delete;
	size_t size() const { return openttd_rust_ship_path_size(this->owner); }
	ShipPathElement at(size_t index) const { return static_cast<Trackdir>(openttd_rust_ship_path_get(this->owner, index)); }
	void set(size_t index, ShipPathElement element) { openttd_rust_ship_path_set(this->owner, index, element.trackdir); }
	void push_back(ShipPathElement element) { openttd_rust_ship_path_push(this->owner, element.trackdir); }
};

/**
 * All ships have this type.
 */
struct Ship final : public SpecializedVehicle<Ship, VEH_SHIP> {
	std::unique_ptr<OpenTTDShipState, decltype(&openttd_rust_ship_state_destroy)> rust_state{openttd_rust_ship_state_new(), openttd_rust_ship_state_destroy};
	ShipPathCache path{openttd_rust_ship_state_path(rust_state.get())}; ///< Cached path.
	OpenTTDShipState *GetRustState() const { return this->rust_state.get(); }
	TrackBits GetState() const
	{
		return static_cast<TrackBits>(openttd_rust_ship_get_state(this->GetRustState()));
	}
	void SetState(TrackBits value)
	{
		openttd_rust_ship_set_state(this->GetRustState(), value);
	}
	Direction GetRotation() const
	{
		return static_cast<Direction>(openttd_rust_ship_get_rotation(this->GetRustState()));
	}
	void SetRotation(Direction value)
	{
		openttd_rust_ship_set_rotation(this->GetRustState(), value);
	}
	int16_t GetRotationX() const
	{
		return static_cast<int16_t>(openttd_rust_ship_get_rotation_x(this->GetRustState()));
	}
	void SetRotationX(int16_t value)
	{
		openttd_rust_ship_set_rotation_x(this->GetRustState(), value);
	}
	int16_t GetRotationY() const
	{
		return static_cast<int16_t>(openttd_rust_ship_get_rotation_y(this->GetRustState()));
	}
	void SetRotationY(int16_t value)
	{
		openttd_rust_ship_set_rotation_y(this->GetRustState(), value);
	}


	/** We don't want GCC to zero our struct! It already is zeroed and has an index! */
	Ship() : SpecializedVehicleBase() { this->record->ship = this->rust_state.get(); }
	/** We want to 'destruct' the right class. */
	virtual ~Ship() { this->PreDestructor(); }

	void MarkDirty() override;
	void UpdateDeltaXY() override;
	ExpensesType GetExpenseType(bool income) const override { return income ? EXPENSES_SHIP_REVENUE : EXPENSES_SHIP_RUN; }
	void PlayLeaveStationSound(bool force = false) const override;
	bool IsPrimaryVehicle() const override { return true; }
	void GetImage(Direction direction, EngineImageType image_type, VehicleSpriteSeq *result) const override;
	int GetDisplaySpeed() const override { return this->Record().cur_speed / 2; }
	int GetDisplayMaxSpeed() const override { return this->Record().cached_max_speed / 2; }
	int GetCurrentMaxSpeed() const override { return std::min<int>(this->Record().cached_max_speed, this->current_order.GetMaxSpeed() * 2); }
	Money GetRunningCost() const override;
	bool IsInDepot() const override { return this->GetState() == TRACK_BIT_DEPOT; }
	bool Tick() override;
	TileIndex GetOrderStationLocation(StationID station) override;
	ClosestDepot FindClosestDepot() override;
	void UpdateCache();
	void SetDestTile(TileIndex tile) override;
};

/** Nested save staging exists only during save/load. */
class ShipStateScope {
	static inline ShipStateScope *active = nullptr;
	ShipStateScope *previous;
	Ship *ship;
	bool loading;
	uint8_t state, rotation;
public:
	ShipStateScope(Ship *s, bool loading) : previous(active), ship(s), loading(loading), state(s->GetState()), rotation(s->GetRotation()) { active = this; }
	~ShipStateScope()
	{
		if (this->loading) { this->ship->SetState(static_cast<TrackBits>(state)); this->ship->SetRotation(static_cast<Direction>(rotation)); }
		active = this->previous;
	}
	static uint8_t &State() { return active->state; }
	static uint8_t &Rotation() { return active->rotation; }
};
#endif /* SHIP_H */
