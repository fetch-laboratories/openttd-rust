/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file effectvehicle_base.h Base class for all effect vehicles. */

#ifndef EFFECTVEHICLE_BASE_H
#define EFFECTVEHICLE_BASE_H

#include "vehicle_base.h"
#include "transparency.h"
#include "rust/effect_ffi.h"

/**
 * A special vehicle is one of the following:
 *  - smoke
 *  - electric sparks for trains
 *  - explosions
 *  - bulldozer (road works)
 *  - bubbles (industry)
 */
struct EffectVehicle final : public SpecializedVehicle<EffectVehicle, VEH_EFFECT> {
	std::unique_ptr<OpenTTDEffectState, decltype(&openttd_rust_effect_destroy)> rust_state{openttd_rust_effect_new(), openttd_rust_effect_destroy};

	/** We don't want GCC to zero our struct! It already is zeroed and has an index! */
	EffectVehicle() : SpecializedVehicleBase() {}
	/** We want to 'destruct' the right class. */
	virtual ~EffectVehicle() = default;

	uint16_t GetAnimationState() const
	{
		return openttd_rust_effect_get_animation(this->rust_state.get());
	}
	uint8_t GetAnimationSubstate() const
	{
		return openttd_rust_effect_get_substate(this->rust_state.get());
	}
	void SetAnimationState(uint16_t state)
	{
		openttd_rust_effect_set_animation(this->rust_state.get(), state);
	}
	void SetAnimationSubstate(uint8_t state)
	{
		openttd_rust_effect_set_substate(this->rust_state.get(), state);
	}

	void UpdateDeltaXY() override;
	bool Tick() override;
	TransparencyOption GetTransparencyOption() const;
};

/** Stack-only serialization staging; nested boundaries restore their predecessor.
 * Private bytes remain owned by Rust. Partial loads are committed on unwind too. */
class EffectVehicleAnimationScope {
	static inline EffectVehicleAnimationScope *active = nullptr;
	EffectVehicleAnimationScope *previous;
	EffectVehicle *vehicle;
	bool loading;
	uint16_t animation_state;
	uint8_t animation_substate;
public:
	EffectVehicleAnimationScope(EffectVehicle *v, bool loading) : previous(active), vehicle(v), loading(loading), animation_state(v->GetAnimationState()), animation_substate(v->GetAnimationSubstate()) { active = this; }
	~EffectVehicleAnimationScope()
	{
		if (this->loading) {
			this->vehicle->SetAnimationState(this->animation_state);
			this->vehicle->SetAnimationSubstate(this->animation_substate);
		}
		active = this->previous;
	}
	static uint16_t &State() { return active->animation_state; }
	static uint8_t &Substate() { return active->animation_substate; }
};
#endif /* EFFECTVEHICLE_BASE_H */
