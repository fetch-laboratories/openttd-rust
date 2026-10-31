/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_engine.cpp NewGRF handling of engines. */

#include "stdafx.h"
#include "debug.h"
#include "train.h"
#include "roadveh.h"
#include "company_func.h"
#include "newgrf_badge.h"
#include "newgrf_cargo.h"
#include "newgrf_spritegroup.h"
#include "timer/timer_game_calendar.h"
#include "vehicle_func.h"
#include "core/random_func.hpp"
#include "core/container_func.hpp"
#include "aircraft.h"
#include "station_base.h"
#include "company_base.h"
#include "newgrf_railtype.h"
#include "newgrf_roadtype.h"
#include "ship.h"

#include "rust/engine_loading_ffi.h"

#include "safeguards.h"

void SetWagonOverrideSprites(EngineID engine, CargoType cargo, const SpriteGroup *group, std::span<EngineID> engine_ids)
{
	Engine *e = Engine::Get(engine);

	assert(cargo < NUM_CARGO + 2); // Include SpriteGroupCargo::SG_DEFAULT and SpriteGroupCargo::SG_PURCHASE pseudo cargoes.

	WagonOverride *wo = &e->overrides.emplace_back();
	wo->group = group;
	wo->cargo = cargo;
	wo->engines.assign(engine_ids.begin(), engine_ids.end());
}

const SpriteGroup *GetWagonOverrideSpriteSet(EngineID engine, CargoType cargo, EngineID overriding_engine)
{
	return openttd_rust_vehicle_wagon_override(&GetRustVehicleResolverServices(), engine.base(), cargo, overriding_engine.base());
}

void SetCustomEngineSprites(EngineID engine, CargoType cargo, const SpriteGroup *group)
{
	Engine *e = Engine::Get(engine);

	if (e->grf_prop.GetSpriteGroup(cargo) != nullptr) {
		GrfMsg(6, "SetCustomEngineSprites: engine {} cargo {} already has group -- replacing", engine, cargo);
	}
	e->grf_prop.SetSpriteGroup(cargo, group);
}


/**
 * Tie a GRFFile entry to an engine, to allow us to retrieve GRF parameters
 * etc during a game.
 * @param engine Engine ID to tie the GRFFile to.
 * @param file   Pointer of GRFFile to tie.
 */
void SetEngineGRF(EngineID engine, const GRFFile *file)
{
	Engine *e = Engine::Get(engine);
	e->grf_prop.SetGRFFile(file);
}



/* virtual */ uint32_t VehicleScopeResolver::GetRandomBits() const
{
	return openttd_rust_vehicle_scope_random(&GetRustVehicleResolverServices(), &this->state, false);
}

/* virtual */ uint32_t VehicleScopeResolver::GetRandomTriggers() const
{
	return openttd_rust_vehicle_scope_random(&GetRustVehicleResolverServices(), &this->state, true);
}


/* virtual */ ScopeResolver *VehicleResolverObject::GetScope(VarSpriteGroupScope scope, uint8_t relative)
{
	switch (openttd_rust_vehicle_resolver_scope(&GetRustVehicleResolverServices(), &this->state, this, scope, relative)) {
		case VSG_SCOPE_SELF: return &this->self_scope;
		case VSG_SCOPE_PARENT: return &this->parent_scope;
		case VSG_SCOPE_RELATIVE: return &this->relative_scope;
		default: return ResolverObject::GetScope(scope, relative);
	}
}

/**
 * Determines the livery of an engine.
 *
 * This always uses dual company colours independent of GUI settings. So it is desync-safe.
 *
 * @param engine Engine type
 * @param v Vehicle, nullptr in purchase list.
 * @return Livery to use
 */

/* virtual */ uint32_t VehicleScopeResolver::GetVariable(uint8_t variable, [[maybe_unused]] uint32_t parameter, bool &available) const
{
	return openttd_rust_vehicle_scope_variable(&GetRustVehicleResolverServices(), &this->state, &this->ro, this->ro.grffile, this->ro.callback, variable, parameter, &available);
}


/* virtual */ const SpriteGroup *VehicleResolverObject::ResolveReal(const RealSpriteGroup &group) const
{
	return openttd_rust_vehicle_resolve_real(&GetRustVehicleResolverServices(), &this->state, &group);
}

GrfSpecFeature VehicleResolverObject::GetFeature() const
{
	switch (Engine::Get(EngineID(this->self_scope.self_type))->type) {
		case VEH_TRAIN: return GSF_TRAINS;
		case VEH_ROAD: return GSF_ROADVEHICLES;
		case VEH_SHIP: return GSF_SHIPS;
		case VEH_AIRCRAFT: return GSF_AIRCRAFT;
		default: return GSF_INVALID;
	}
}

uint32_t VehicleResolverObject::GetDebugID() const
{
	return Engine::Get(EngineID(this->self_scope.self_type))->grf_prop.local_id;
}

/**
 * Get the grf file associated with an engine type.
 * @param engine_type Engine to query.
 * @return grf file associated with the engine.
 */
static const GRFFile *GetEngineGrfFile(EngineID engine_type)
{
	const Engine *e = Engine::Get(engine_type);
	return (e != nullptr) ? e->GetGRF() : nullptr;
}

/**
 * Resolver of a vehicle (chain).
 * @param engine_type Engine type
 * @param v %Vehicle being resolved.
 * @param wagon_override Application of wagon overrides.
 * @param rotor_in_gui Helicopter rotor is drawn in GUI.
 * @param callback Callback ID.
 * @param callback_param1 First parameter (var 10) of the callback.
 * @param callback_param2 Second parameter (var 18) of the callback.
 */
VehicleResolverObject::VehicleResolverObject(EngineID engine_type, const Vehicle *v, WagonOverride wagon_override, bool rotor_in_gui,
		CallbackID callback, uint32_t callback_param1, uint32_t callback_param2)
	: SpecializedResolverObject<VehicleRandomTriggers>(GetEngineGrfFile(engine_type), callback, callback_param1, callback_param2),
	state(this->owned_state), self_scope(*this, this->state.self_), parent_scope(*this, this->state.parent), relative_scope(*this, this->state.relative)
{
	openttd_rust_vehicle_resolver_init(&GetRustVehicleResolverServices(), &this->state, engine_type.base(), v, wagon_override, rotor_in_gui);
	this->root_spritegroup = this->state.root;
}

VehicleResolverObject::VehicleResolverObject(OpenTTDVehicleResolverState &state, EngineID engine, CallbackID callback, uint32_t p1, uint32_t p2)
	: SpecializedResolverObject<VehicleRandomTriggers>(GetEngineGrfFile(engine), callback, p1, p2),
	state(state), self_scope(*this, state.self_), parent_scope(*this, state.parent), relative_scope(*this, state.relative)
{
	this->root_spritegroup = state.root;
}

static void GetCustomEngineSprite(EngineID engine, const Vehicle *v, Direction direction, EngineImageType image_type, VehicleSpriteSeq *result)
{
	VehicleResolverObject object(engine, v, VehicleResolverObject::WO_CACHED, false, CBID_NO_CALLBACK);
	result->Clear();

	bool sprite_stack = EngInfo(engine)->misc_flags.Test(EngineMiscFlag::SpriteStack);
	uint max_stack = sprite_stack ? static_cast<uint>(std::size(result->seq)) : 1;
	for (uint stack = 0; stack < max_stack; ++stack) {
		object.callback_param1 = image_type | (stack << 8);
		const auto *group = object.Resolve<ResultSpriteGroup>();
		int32_t reg100 = sprite_stack ? object.GetRegister(0x100) : 0;
		if (group != nullptr && group->num_sprites != 0) {
			result->seq[result->count].sprite = group->sprite + (direction % group->num_sprites);
			result->seq[result->count].pal    = GB(reg100, 0, 16); // zero means default recolouring
			result->count++;
		}
		if (!HasBit(reg100, 31)) break;
	}
}

void GetCustomVehicleSprite(const Vehicle *v, Direction direction, EngineImageType image_type, VehicleSpriteSeq *result)
{
	GetCustomEngineSprite(v->engine_type, v, direction, image_type, result);
}

void GetCustomVehicleIcon(EngineID engine, Direction direction, EngineImageType image_type, VehicleSpriteSeq *result)
{
	GetCustomEngineSprite(engine, nullptr, direction, image_type, result);
}

static void GetRotorOverrideSprite(EngineID engine, const struct Aircraft *v, EngineImageType image_type, VehicleSpriteSeq *result)
{
	const Engine *e = Engine::Get(engine);

	/* Only valid for helicopters */
	assert(e->type == VEH_AIRCRAFT);
	assert(!(e->VehInfo<AircraftVehicleInfo>().subtype & AIR_CTOL));

	/* We differ from TTDPatch by resolving the sprite using the primary vehicle 'v', and not using the rotor vehicle 'v->Next()->Next()'.
	 * TTDPatch copies some variables between the vehicles each time, to somehow synchronize the rotor vehicle with the primary vehicle.
	 * We use 'rotor_in_gui' to replicate when the variables differ.
	 * But some other variables like 'rotor state' and 'rotor speed' are not available in OpenTTD, while they are in TTDPatch. */
	bool rotor_in_gui = image_type != EIT_ON_MAP;
	VehicleResolverObject object(engine, v, VehicleResolverObject::WO_SELF, rotor_in_gui, CBID_NO_CALLBACK);
	result->Clear();
	uint rotor_pos = v == nullptr || rotor_in_gui ? 0 : v->Next()->Next()->state;

	bool sprite_stack = e->info.misc_flags.Test(EngineMiscFlag::SpriteStack);
	uint max_stack = sprite_stack ? static_cast<uint>(std::size(result->seq)) : 1;
	for (uint stack = 0; stack < max_stack; ++stack) {
		object.callback_param1 = image_type | (stack << 8);
		const auto *group = object.Resolve<ResultSpriteGroup>();
		int32_t reg100 = sprite_stack ? object.GetRegister(0x100) : 0;
		if (group != nullptr && group->num_sprites != 0) {
			result->seq[result->count].sprite = group->sprite + (rotor_pos % group->num_sprites);
			result->seq[result->count].pal    = GB(reg100, 0, 16); // zero means default recolouring
			result->count++;
		}
		if (!HasBit(reg100, 31)) break;
	}
}

void GetCustomRotorSprite(const struct Aircraft *v, EngineImageType image_type, VehicleSpriteSeq *result)
{
	GetRotorOverrideSprite(v->engine_type, v, image_type, result);
}

void GetCustomRotorIcon(EngineID engine, EngineImageType image_type, VehicleSpriteSeq *result)
{
	GetRotorOverrideSprite(engine, nullptr, image_type, result);
}

/**
 * Check if a wagon is currently using a wagon override
 * @param v The wagon to check
 * @return true if it is using an override, false otherwise
 */
bool UsesWagonOverride(const Vehicle *v)
{
	assert(v->type == VEH_TRAIN);
	return Train::From(v)->tcache.cached_override != nullptr;
}

/**
 * Evaluate a newgrf callback for vehicles
 * @param callback The callback to evaluate
 * @param param1   First parameter of the callback
 * @param param2   Second parameter of the callback
 * @param engine   Engine type of the vehicle to evaluate the callback for
 * @param v        The vehicle to evaluate the callback for, or nullptr if it doesn't exist yet
 * @param[out] regs100 Additional result values from registers 100+
 * @return The value the callback returned, or CALLBACK_FAILED if it failed
 */
uint16_t GetVehicleCallback(CallbackID callback, uint32_t param1, uint32_t param2, EngineID engine, const Vehicle *v, std::span<int32_t> regs100)
{
	return openttd_rust_vehicle_callback(&GetRustVehicleResolverServices(), callback, param1, param2, engine.base(), v, nullptr, false, regs100.data(), regs100.size());
}

/**
 * Evaluate a newgrf callback for vehicles with a different vehicle for parent scope.
 * @param callback The callback to evaluate
 * @param param1   First parameter of the callback
 * @param param2   Second parameter of the callback
 * @param engine   Engine type of the vehicle to evaluate the callback for
 * @param v        The vehicle to evaluate the callback for, or nullptr if it doesn't exist yet
 * @param parent   The vehicle to use for parent scope
 * @param[out] regs100 Additional result values from registers 100+
 * @return The value the callback returned, or CALLBACK_FAILED if it failed
 */
uint16_t GetVehicleCallbackParent(CallbackID callback, uint32_t param1, uint32_t param2, EngineID engine, const Vehicle *v, const Vehicle *parent, std::span<int32_t> regs100)
{
	return openttd_rust_vehicle_callback(&GetRustVehicleResolverServices(), callback, param1, param2, engine.base(), v, parent, true, regs100.data(), regs100.size());
}


/* Callback 36 handlers */
int GetVehicleProperty(const Vehicle *v, PropertyID property, int orig_value, bool is_signed)
{
	return GetEngineProperty(v->engine_type, property, orig_value, v, is_signed);
}


int GetEngineProperty(EngineID engine, PropertyID property, int orig_value, const Vehicle *v, bool is_signed)
{
	return openttd_rust_vehicle_property(&GetRustVehicleResolverServices(), engine.base(), property, orig_value, v, is_signed);
}

/**
 * Test for vehicle build probability type.
 * @param v Vehicle whose build probability to test.
 * @param type Build probability type to test for.
 * @returns True or false depending on the probability result, or std::nullopt if the callback failed.
 */
std::optional<bool> TestVehicleBuildProbability(Vehicle *v, EngineID engine, BuildProbabilityType type)
{
	int8_t result = openttd_rust_vehicle_build_probability(&GetRustVehicleResolverServices(), v, engine.base(), to_underlying(type));
	return result < 0 ? std::nullopt : std::optional<bool>(result != 0);
}


void TriggerVehicleRandomisation(Vehicle *v, VehicleRandomTrigger trigger)
{
	openttd_rust_vehicle_randomise(&GetRustVehicleResolverServices(), v, to_underlying(trigger));
}

/* Functions for changing the order of vehicle purchase lists */


/**
 * Comparator function to sort engines via scope-GRFID and local ID.
 * @param a left side
 * @param b right side
 * @return comparison result
 */
static bool EnginePreSort(const EngineID &a, const EngineID &b)
{
	const Engine &engine_a = *Engine::Get(a);
	const Engine &engine_b = *Engine::Get(b);

	/* 1. Sort by engine type */
	if (engine_a.type != engine_b.type) return static_cast<int>(engine_a.type) < static_cast<int>(engine_b.type);

	/* 2. Sort by scope-GRFID */
	if (engine_a.grf_prop.grfid != engine_b.grf_prop.grfid) return engine_a.grf_prop.grfid < engine_b.grf_prop.grfid;

	/* 3. Sort by local ID */
	return static_cast<int>(engine_a.grf_prop.local_id) < static_cast<int>(engine_b.grf_prop.local_id);
}

void RustVehicleOrderSort(uint16_t *data, size_t size) noexcept
{
	std::ranges::sort(std::span(data, size), [](uint16_t a, uint16_t b) { return EnginePreSort(EngineID{a}, EngineID{b}); });
}


/**
 * Fill the grf_cache of the given vehicle.
 * @param v The vehicle to fill the cache for.
 */
void FillNewGRFVehicleCache(const Vehicle *v)
{
	openttd_rust_vehicle_grf_fill(&GetRustVehicleResolverServices(), v);
}

#include "rust/vehicle_services_newgrf.h"
