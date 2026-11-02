/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file command.cpp Handling of NewGRF road stops. */

#include "stdafx.h"
#include "debug.h"
#include "station_base.h"
#include "roadstop_base.h"
#include "newgrf_badge.h"
#include "newgrf_roadstop.h"
#include "newgrf_cargo.h"
#include "newgrf_roadtype.h"
#include "gfx_type.h"
#include "company_func.h"
#include "road.h"
#include "window_type.h"
#include "timer/timer_game_calendar.h"
#include "town.h"
#include "tile_cmd.h"
#include "viewport_func.h"
#include "newgrf_animation_base.h"
#include "rust/animation_binding.hpp"
#include "rust/services_station.h"
#include "newgrf_sound.h"

#include "table/strings.h"

#include "newgrf_class_func.h"

#include "safeguards.h"

template <>
std::span<const OpenTTDGRFClassDefault> RoadStopClass::Defaults()
{
	static const OpenTTDGRFClassDefault defaults[]{
		{ROADSTOP_CLASS_LABEL_DEFAULT, STR_STATION_CLASS_DFLT},
		{ROADSTOP_CLASS_LABEL_WAYPOINT, STR_STATION_CLASS_WAYP},
	};
	return defaults;
}


/* Instantiate RoadStopClass. */
template class NewGRFClass<RoadStopSpec, RoadStopClassID, ROADSTOP_CLASS_MAX>;


uint32_t RoadStopScopeResolver::GetRandomBits() const
{
	return openttd_rust_roadstop_random(&this->state, false, &GetRoadStopResolverServices());
}

uint32_t RoadStopScopeResolver::GetRandomTriggers() const
{
	return openttd_rust_roadstop_random(&this->state, true, &GetRoadStopResolverServices());
}

uint32_t RoadStopScopeResolver::GetVariable(uint8_t variable, [[maybe_unused]] uint32_t parameter, bool &available) const
{
	return openttd_rust_roadstop_variable(&this->state, &this->ro, this->ro.grffile, variable, parameter, &available, &GetRoadStopResolverServices());
}

RoadStopResolverObject::RoadStopResolverObject(const RoadStopSpec *roadstopspec, BaseStation *st, TileIndex tile, RoadType roadtype, StationType type, uint8_t view,
		CallbackID callback, uint32_t param1, uint32_t param2)
	: SpecializedResolverObject<StationRandomTriggers>(roadstopspec->grf_prop.grffile, callback, param1, param2), roadstop_scope(*this, st, roadstopspec, tile, roadtype, type, view)
{
	openttd_rust_roadstop_init(&this->roadstop_scope.state, &GetRoadStopResolverServices());
	this->root_spritegroup = this->roadstop_scope.state.root;
}

TownScopeResolver *RoadStopResolverObject::GetTown()
{
	return openttd_rust_roadstop_town(&this->roadstop_scope.state, this, &GetRoadStopResolverServices());
}

uint16_t GetRoadStopCallback(CallbackID callback, uint32_t param1, uint32_t param2, const RoadStopSpec *roadstopspec, BaseStation *st, TileIndex tile, RoadType roadtype, StationType type, uint8_t view, std::span<int32_t> regs100)
{
	return openttd_rust_roadstop_callback(callback, param1, param2, roadstopspec, st, tile.base(), roadtype, to_underlying(type), view, regs100.data(), regs100.size(), &GetRoadStopResolverServices());
}

/**
 * Draw representation of a road stop tile for GUI purposes.
 * @param x position x of image.
 * @param y position y of image.
 * @param image an int offset for the sprite.
 * @param roadtype the RoadType of the underlying road.
 * @param spec the RoadStop's spec.
 * @return true of the tile was drawn (allows for fallback to default graphics)
 */
void DrawRoadStopTile(int x, int y, RoadType roadtype, const RoadStopSpec *spec, StationType type, int view)
{
	assert(roadtype != INVALID_ROADTYPE);
	assert(spec != nullptr);

	const RoadTypeInfo *rti = GetRoadTypeInfo(roadtype);
	RoadStopResolverObject object(spec, nullptr, INVALID_TILE, roadtype, type, view);
	const auto *group = object.Resolve<TileLayoutSpriteGroup>();
	if (group == nullptr) return;
	auto processor = group->ProcessRegisters(object, nullptr);
	auto dts = processor.GetLayout();

	PaletteID palette = GetCompanyPalette(_local_company);

	SpriteID image = dts.ground.sprite;
	PaletteID pal = dts.ground.pal;

	RoadStopDrawModes draw_mode;
	if (spec->flags.Test(RoadStopSpecFlag::DrawModeRegister)) {
		draw_mode = static_cast<RoadStopDrawModes>(object.GetRegister(0x100));
	} else {
		draw_mode = spec->draw_mode;
	}

	if (type == StationType::RoadWaypoint) {
		DrawSprite(SPR_ROAD_PAVED_STRAIGHT_X, PAL_NONE, x, y);
		if (draw_mode.Test(RoadStopDrawMode::WaypGround) && GB(image, 0, SPRITE_WIDTH) != 0) {
			DrawSprite(image, GroundSpritePaletteTransform(image, pal, palette), x, y);
		}
	} else if (GB(image, 0, SPRITE_WIDTH) != 0) {
		DrawSprite(image, GroundSpritePaletteTransform(image, pal, palette), x, y);
	}

	if (view >= 4) {
		/* Drive-through stop */
		uint sprite_offset = 5 - view;

		/* Road underlay takes precedence over tram */
		if (type == StationType::RoadWaypoint || draw_mode.Test(RoadStopDrawMode::Overlay)) {
			if (rti->UsesOverlay()) {
				SpriteID ground = GetCustomRoadSprite(rti, INVALID_TILE, ROTSG_GROUND);
				DrawSprite(ground + sprite_offset, PAL_NONE, x, y);

				SpriteID overlay = GetCustomRoadSprite(rti, INVALID_TILE, ROTSG_OVERLAY);
				if (overlay) DrawSprite(overlay + sprite_offset, PAL_NONE, x, y);
			} else if (RoadTypeIsTram(roadtype)) {
				DrawSprite(SPR_TRAMWAY_TRAM + sprite_offset, PAL_NONE, x, y);
			}
		}
	} else {
		/* Bay stop */
		if (draw_mode.Test(RoadStopDrawMode::Road) && rti->UsesOverlay()) {
			SpriteID ground = GetCustomRoadSprite(rti, INVALID_TILE, ROTSG_ROADSTOP);
			DrawSprite(ground + view, PAL_NONE, x, y);
		}
	}

	DrawCommonTileSeqInGUI(x, y, &dts, 0, 0, palette, true);
}

std::optional<SpriteLayoutProcessor> GetRoadStopLayout(TileInfo *ti, const RoadStopSpec *spec, BaseStation *st, StationType type, int view, std::span<int32_t> regs100)
{
	RoadStopResolverObject object(spec, st, ti->tile, INVALID_ROADTYPE, type, view);
	auto group = object.Resolve<TileLayoutSpriteGroup>();
	if (group == nullptr) return std::nullopt;
	for (uint i = 0; i < regs100.size(); ++i) {
		regs100[i] = object.GetRegister(0x100 + i);
	}
	return group->ProcessRegisters(object, nullptr);
}


/** Helper class for animation control. */
const OpenTTDRoadStopAnimation &GetRoadStopAnimationBinding() noexcept
{
	static const auto binding = MakeRustAnimationBindingDirect<RoadStopSpec, BaseStation, int>(
		[](uint32_t tile) noexcept { OpenTTDRoadStopAnimationEntry entry; openttd_rust_roadstop_animation_entry(tile, &entry); return entry; },
		[](const RoadStopSpec *s) noexcept -> OpenTTDAnimationInfo { return {s->animation.speed, s->animation.frames, s->animation.status == AnimationStatus::Looping, s->callback_mask.Test(RoadStopCallbackMask::AnimationSpeed), s->callback_mask.Test(RoadStopCallbackMask::AnimationNextFrame), s->flags.Test(RoadStopSpecFlag::Cb141RandomBits), true}; },
		openttd_rust_roadstop_animation_callback);
	return binding;
}

void AnimateRoadStopTile(TileIndex tile)
{
	openttd_rust_animate_roadstop(tile.base());
}

void TriggerRoadStopAnimation(BaseStation *st, TileIndex trigger_tile, StationAnimationTrigger trigger, CargoType cargo_type)
{
	openttd_rust_trigger_roadstop(st, trigger_tile.base(), to_underlying(trigger), cargo_type);
}

/**
 * Trigger road stop randomisation
 *
 * @param st the station being triggered
 * @param tile the exact tile of the station that should be triggered
 * @param trigger trigger type
 * @param cargo_type cargo type causing the trigger
 */
void TriggerRoadStopRandomisation(BaseStation *st, TileIndex tile, StationRandomTrigger trigger, CargoType cargo_type)
{
	openttd_rust_roadstop_randomise(st, tile.base(), to_underlying(trigger), cargo_type, &GetRoadStopResolverServices());
}

/**
 * Checks if there's any new stations by a specific RoadStopType
 * @param rs the RoadStopType to check.
 * @param roadtype the RoadType to check.
 * @return true if there was any new RoadStopSpec's found for the given RoadStopType and RoadType, else false.
 */
bool GetIfNewStopsByType(RoadStopType rs, RoadType roadtype)
{
	for (const auto &cls : RoadStopClass::Classes()) {
		/* Ignore the waypoint class. */
		if (IsWaypointClass(cls)) continue;
		/* Ignore the default class with only the default station. */
		if (cls.Index() == ROADSTOP_CLASS_DFLT && cls.GetSpecCount() == 1) continue;
		if (GetIfClassHasNewStopsByType(&cls, rs, roadtype)) return true;
	}
	return false;
}

/**
 * Checks if the given RoadStopClass has any specs assigned to it, compatible with the given RoadStopType.
 * @param roadstopclass the RoadStopClass to check.
 * @param rs the RoadStopType to check.
 * @param roadtype the RoadType to check.
 * @return true if the RoadStopSpec has any specs compatible with the given RoadStopType and RoadType.
 */
bool GetIfClassHasNewStopsByType(const RoadStopClass *roadstopclass, RoadStopType rs, RoadType roadtype)
{
	for (const auto spec : roadstopclass->Specs()) {
		if (GetIfStopIsForType(spec, rs, roadtype)) return true;
	}
	return false;
}

/**
 * Checks if the given RoadStopSpec is compatible with the given RoadStopType.
 * @param roadstopspec the RoadStopSpec to check.
 * @param rs the RoadStopType to check.
 * @param roadtype the RoadType to check.
 * @return true if the RoadStopSpec is compatible with the given RoadStopType and RoadType.
 */
bool GetIfStopIsForType(const RoadStopSpec *roadstopspec, RoadStopType rs, RoadType roadtype)
{
	/* The roadstopspec is nullptr, must be the default station, always return true. */
	if (roadstopspec == nullptr) return true;

	if (roadstopspec->flags.Test(RoadStopSpecFlag::RoadOnly) && !RoadTypeIsRoad(roadtype)) return false;
	if (roadstopspec->flags.Test(RoadStopSpecFlag::TramOnly) && !RoadTypeIsTram(roadtype)) return false;

	if (roadstopspec->stop_type == ROADSTOPTYPE_ALL) return true;

	switch (rs) {
		case RoadStopType::Bus:
			if (roadstopspec->stop_type == ROADSTOPTYPE_PASSENGER) return true;
			break;

		case RoadStopType::Truck:
			if (roadstopspec->stop_type == ROADSTOPTYPE_FREIGHT) return true;
			break;

		default:
			NOT_REACHED();
	}
	return false;
}

const RoadStopSpec *GetRoadStopSpec(TileIndex t)
{
	return openttd_rust_roadstop_spec(t.base(), &GetRoadStopResolverServices());
}

/**
 * Allocate a RoadStopSpec to a Station. This is called once per build operation.
 * @param spec RoadStopSpec to allocate.
 * @param st Station to allocate it to.
 * @return Index within the Station's road stop spec list, or std::nullopt if the allocation failed.
 */
std::optional<uint8_t> AllocateSpecToRoadStop(const RoadStopSpec *spec, BaseStation *st)
{
	int16_t index = openttd_rust_roadstop_allocate(spec, st, &GetRoadStopResolverServices());
	return index < 0 ? std::nullopt : std::make_optional(static_cast<uint8_t>(index));
}

/**
 * Assign a previously allocated RoadStopSpec specindex to a Station.
 * @param spec RoadStopSpec to assign..
 * @param st Station to allocate it to.
 * @param specindex Spec index of allocation.
 */
void AssignSpecToRoadStop(const RoadStopSpec *spec, BaseStation *st, uint8_t specindex)
{
	openttd_rust_roadstop_assign(spec, st, specindex, &GetRoadStopResolverServices());
}

/**
 * Deallocate a RoadStopSpec from a Station. Called when removing a single roadstop tile.
 * @param st Station to work with.
 * @param specindex Index of the custom roadstop within the Station's roadstop spec list.
 */
void DeallocateSpecFromRoadStop(BaseStation *st, uint8_t specindex)
{
	openttd_rust_roadstop_deallocate(st, specindex, &GetRoadStopResolverServices());
}

/**
 * Update the cached animation trigger bitmask for a station.
 * @param st Station to update.
 */
void RoadStopUpdateCachedTriggers(BaseStation *st)
{
	openttd_rust_roadstop_cache(st, &GetRoadStopResolverServices());
}

RoadStopResolverObject::RoadStopResolverObject(const OpenTTDRoadStopResolverState &state, CallbackID callback, uint32_t param1, uint32_t param2)
	: SpecializedResolverObject<StationRandomTriggers>(state.spec->grf_prop.grffile, callback, param1, param2), roadstop_scope(*this, state)
{
	this->root_spritegroup = state.root;
}
ScopeResolver *RoadStopResolverObject::GetScope(VarSpriteGroupScope scope, uint8_t relative)
{
	return openttd_rust_roadstop_scope(&this->roadstop_scope.state, this, &this->roadstop_scope, scope, relative, &GetRoadStopResolverServices());
}
const OpenTTDRoadStopResolverServices &GetRoadStopResolverServices() noexcept
{
	static_assert(to_underlying(HouseZone::TownEdge) == 0);
	static_assert(CargoGRFFileProps::SG_DEFAULT_NA == NUM_CARGO + 2);
	static_assert(CBID_STATION_AVAILABILITY == 0x13);
	static const OpenTTDRoadStopResolverServices services{
		.rebind = RustRoadStopRebindSpec,
		.base_scope = RustResolverBaseScope,
		.build_date = RustStationBuildDate<const BaseStation>,
		.by_tile = RustBaseStationByTile,
		.cached_cargo = RustRoadStopCachedCargo,
		.clear = RustRoadStopSpecClear,
		.company = RustCompanyInfo,
		.custom = RustCustomRoadStopSpecIndex,
		.facilities = RustStationFacilities,
		.frame = RustRoadStopAnimationFrame,
		.group = RustSpecCargoSpriteGroup<RoadStopSpec>,
		.group_cargo = RustSpecCargoGroupCargo<RoadStopSpec>,
		.group_count = RustSpecCargoGroupCount<RoadStopSpec>,
		.index = RustCustomRoadStopIndex,
		.is_station = RustStationIsExpected<BaseStation *>,
		.make_town = [](ResolverObject *ro, Town *town, bool readonly) noexcept { return &static_cast<RoadStopResolverObject *>(ro)->town_scope.emplace(*ro, town, readonly); },
		.owner = RustStationOwnerByte<BaseStation>,
		.random_bits = RustStationRandomBits<const BaseStation>,
		.resize = RustRoadStopSpecResize,
		.resolve_callback = [](const OpenTTDRoadStopResolverState *state, uint16_t cb, uint32_t p1, uint32_t p2, int32_t *regs, size_t size) noexcept { RoadStopResolverObject object(*state, static_cast<CallbackID>(cb), p1, p2); return object.ResolveCallback({regs, size}); },
		.resolve_reseed = [](const OpenTTDRoadStopResolverState *state, uint8_t waiting) noexcept -> OpenTTDRoadStopReseed { RoadStopResolverObject object(*state); object.SetWaitingRandomTriggers(StationRandomTriggers(waiting)); object.ResolveRerandomisation(); return {object.GetReseedSum(), object.GetUsedRandomTriggers().base()}; },
		.set_cached = RustRoadStopSetCachedTriggers,
		.set_random_bits = RustStationSetRandomBits,
		.set_slot = RustRoadStopSetSpecSlot,
		.set_tile_random = RustRoadStopSetRandomBits,
		.set_waiting = RustStationSetWaitingTriggers,
		.shared = &GetRustSharedServices(),
		.slot_grfid = RustRoadStopSlotGRFID,
		.slot_key = RustRoadStopSlotKey,
		.slot_spec = RustRoadStopSlotSpec,
		.slot_vacancy = RustRoadStopSlotVacancy,
		.spec_cargo = RustRoadStopCargoTriggers,
		.spec_count = RustRoadStopSpecCount,
		.spec_triggers = RustRoadStopSpecTriggers,
		.station_id = RustBaseStationID<BaseStation>,
		.tile = RustRoadStopDataTile,
		.tile_count = RustRoadStopTileDataCount,
		.tile_random = RustRoadStopRandomBits,
		.town = OpenTTDBaseStationTown,
		.town_tile = OpenTTDTownTile,
		.waiting = RustStationWaitingTriggers<const BaseStation>,
	};
	return services;
}
