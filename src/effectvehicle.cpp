/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file effectvehicle.cpp Implementation of everything generic to vehicles. */

#include "stdafx.h"
#include "landscape.h"
#include "core/random_func.hpp"
#include "industry_map.h"
#include "vehicle_func.h"
#include "sound_func.h"
#include "animated_tile_func.h"
#include "effectvehicle_func.h"
#include "effectvehicle_base.h"

#include "rust/vehicle_services.h"
#include "safeguards.h"



static void OPENTTD_RUST_CALL EffectViewport(void *context) noexcept
{
	/* Effects skip UpdatePosition; UpdateViewport has no NewGRF/script callbacks. */
	static_cast<EffectVehicle *>(context)->UpdatePositionAndViewport();
}

static void OPENTTD_RUST_CALL EffectSound(void *context, uint8_t success) noexcept
{
	SndPlayVehicleFx(success == 0 ? SND_2F_BUBBLE_GENERATOR_FAIL : SND_31_BUBBLE_GENERATOR_SUCCESS, static_cast<EffectVehicle *>(context));
}

static const OpenTTDEffectLeaves &EffectServices()
{
	static const OpenTTDEffectLeaves leaves{
		.vehicles = &GetRustVehicleServices(),
		.sprite_sequence = RustOpaqueVehicleInvoke<RustWriteVehicleSpriteSequence, uint32_t>,
		.sprite = RustOpaqueVehicleInvoke<RustWriteVehicleFirstSprite, uint32_t>,
		.spritenum = RustOpaqueVehicleInvoke<RustWriteVehicleSprite, uint8_t>,
		.viewport = EffectViewport,
		.sound = EffectSound,
	};
	return leaves;
}

extern "C" const OpenTTDEffectCreationServices *GetRustEffectCreationServices() noexcept
{
	static const OpenTTDEffectCreationServices services{
		.effect = &EffectServices(),
		.shared = &GetRustSharedServices(),
		.allocate = []() noexcept -> OpenTTDEffectAllocation { auto *v = new EffectVehicle(); return {v, RustVehicle(v), v->rust_state.get()}; },
		.owner = [](void *shell) noexcept { return static_cast<EffectVehicle *>(shell)->rust_state.get(); },

	};
	return &services;
}

static bool RunEffect(EffectVehicle *v)
{
	if (openttd_rust_effect_run(v->rust_state.get(), RustVehicle(v), &EffectServices(), &GetRustSharedServices()) != 0) return true;
	/* Expiry returns before destroying the Rust owner. Vehicle::~Vehicle and
	 * Pool::FreeItem/PostDestructor run wholly outside any Rust frame/borrow. */
	delete v;
	return false;
}

/**
 * Create an effect vehicle at a particular location.
 * @param x The x location on the map.
 * @param y The y location on the map.
 * @param z The z location on the map.
 * @param type The type of effect vehicle.
 * @return The effect vehicle.
 */
EffectVehicle *CreateEffectVehicle(int x, int y, int z, EffectVehicleType type)
{
	openttd_rust_effect_bind(GetRustEffectCreationServices());
	return reinterpret_cast<EffectVehicle *>(openttd_rust_effect_create(x, y, z, type));
}

/**
 * Create an effect vehicle above a particular location.
 * @param x The x location on the map.
 * @param y The y location on the map.
 * @param z The offset from the ground.
 * @param type The type of effect vehicle.
 * @return The effect vehicle.
 */
EffectVehicle *CreateEffectVehicleAbove(int x, int y, int z, EffectVehicleType type)
{
	openttd_rust_effect_bind(GetRustEffectCreationServices());
	return reinterpret_cast<EffectVehicle *>(openttd_rust_effect_create_above(x, y, z, type));
}

/**
 * Create an effect vehicle above a particular vehicle.
 * @param v The vehicle to base the position on.
 * @param x The x offset to the vehicle.
 * @param y The y offset to the vehicle.
 * @param z The z offset to the vehicle.
 * @param type The type of effect vehicle.
 * @return The effect vehicle.
 */
EffectVehicle *CreateEffectVehicleRel(const Vehicle *v, int x, int y, int z, EffectVehicleType type)
{
	openttd_rust_effect_bind(GetRustEffectCreationServices());
	return reinterpret_cast<EffectVehicle *>(openttd_rust_effect_create_rel(RustVehicle(v), x, y, z, type));
}

bool EffectVehicle::Tick()
{
	return RunEffect(this);
}

void EffectVehicle::UpdateDeltaXY()
{
	this->bounds = {{}, {1, 1, 1}, {}};
}

/**
 * Determines the transparency option affecting the effect.
 * @return Transparency option, or TO_INVALID if none.
 */
TransparencyOption EffectVehicle::GetTransparencyOption() const
{
	static const std::array<TransparencyOption, EV_END> transparency{TO_INDUSTRIES, TO_INVALID, TO_INVALID, TO_INVALID, TO_INVALID, TO_INVALID, TO_INVALID, TO_INVALID, TO_INVALID, TO_INDUSTRIES, TO_INVALID, TO_INDUSTRIES};
	return transparency[this->Record().subtype];
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDEffectLeaves.hpp"
#include "abi-visit-OpenTTDEffectCreationServices.hpp"
bool CheckAbi_effectvehicle() { return AbiServicesComplete(EffectServices()) && AbiServicesComplete(*GetRustEffectCreationServices()); }
