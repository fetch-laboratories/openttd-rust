/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_station.cpp Functions for dealing with station classes and custom stations. */

#include "stdafx.h"
#include "debug.h"
#include "station_base.h"
#include "waypoint_base.h"
#include "roadstop_base.h"
#include "newgrf_badge.h"
#include "newgrf_cargo.h"
#include "newgrf_station.h"
#include "newgrf_spritegroup.h"
#include "newgrf_sound.h"
#include "newgrf_railtype.h"
#include "town.h"
#include "newgrf_town.h"
#include "company_func.h"
#include "tunnelbridge_map.h"
#include "newgrf_animation_base.h"
#include "rust/animation_binding.hpp"
#include "rust/services_station.h"
#include "rust/station_resolver_ffi.h"
#include "rust/services_rail.h"
#include "rust/services_map.h"
#include "rust/services_cargo.h"
#include "timer/timer_game_calendar.h"

#include "table/strings.h"

#include "newgrf_class_func.h"

#include "safeguards.h"



static_assert(INITIAL_STATION_RATING == 175 && HVOT_WAYPOINT == (1 << 6));
static_assert(ATP_TTDP_LARGE == 1 && CalendarTime::DAYS_TILL_ORIGINAL_BASE_YEAR.base() == 701265);
static_assert(VSG_SCOPE_SELF == 0 && VSG_SCOPE_PARENT == 1);
static_assert(to_underlying(GoodsEntry::State::Acceptance) == 0 && to_underlying(GoodsEntry::State::Rating) == 1);
static_assert(to_underlying(GoodsEntry::State::EverAccepted) == 2 && to_underlying(GoodsEntry::State::LastMonth) == 3);
static_assert(to_underlying(GoodsEntry::State::CurrentMonth) == 4 && to_underlying(GoodsEntry::State::AcceptedBigtick) == 5);

const OpenTTDCargoStorageServices *CargoStorageServices();
static uint16_t RustEvaluateStationCallback(uint16_t cb, uint32_t p1, uint32_t p2, const StationSpec *s, BaseStation *st, uint32_t t, uint8_t axis, int32_t *regs, size_t count) noexcept
{
	StationResolverObject object(s, st, TileIndex{t}, static_cast<CallbackID>(cb), p1, p2);
	object.station_scope.axis = static_cast<Axis>(axis);
	return object.ResolveCallback({regs, count});
}

const OpenTTDStationResolverServices &GetStationResolverServices() noexcept
{
	static const OpenTTDStationResolverServices services{
		.shared = &GetRustSharedServices(),
		.cargo = CargoStorageServices(),
		.cargo_owner = RustStationExistingCargoOwner,
		.group_count = [](const StationSpec *s) noexcept { return s->grf_prop.spritegroups.size(); },
		.group_cargo = [](const StationSpec *s, size_t i) noexcept { return s->grf_prop.spritegroups[i].first; },
		.group = [](const StationSpec *s, uint8_t c) noexcept { return s->grf_prop.GetSpriteGroup(c); },
		.spec = [](const StationSpec *s) noexcept { return OpenTTDStationResolverSpec{s->cargo_threshold, s->flags.Test(StationSpecFlag::DivByStationSize)}; },
		.loading_count = RustRealLoadingCount,
		.loaded_count = RustRealLoadedCount,
		.loading = RustRealLoading,
		.loaded = RustRealLoaded,
		.axis = RustRailStationAxis,
		.tile_add = RustTileAdd,
		.tile_random = RustStationTileRandomBits,
		.station_random = RustStationRandomBits<BaseStation>,
		.waiting = RustStationWaitingTriggers<BaseStation>,
		.set_waiting = RustStationSetWaitingTriggers,
		.set_random = RustStationSetRandomBits,
		.cached_cargo = RustStationCachedCargoTriggers,
		.spec_cargo = RustStationCargoTriggers,
		.resolve_random = [](const StationSpec *s, BaseStation *st, uint32_t t, uint8_t waiting) noexcept {
			StationResolverObject object(s, st, TileIndex{t}, CBID_RANDOM_TRIGGER, 0);
			object.SetWaitingRandomTriggers(StationRandomTriggers(waiting));
			object.ResolveRerandomisation();
			return OpenTTDStationReseed{object.GetReseedSum(), object.GetUsedRandomTriggers().base()};
		},
		.resolve_callback = RustEvaluateStationCallback,
		.company = RustCompanyInfo,
		.nearby_station = RustNearbyStationTile,

		.build_date = RustStationBuildDate<BaseStation>,
		.airport_ttd_type = RustAirportTTDType,
		.identity = [](BaseStation *st, uint8_t i) noexcept { const auto key = RustStationSlotKey(st, i); return OpenTTDStationSpecIdentity{key.grfid, key.local_id}; },
		/* Only shared native ScopeResolver frames are materialized here; Rust
		 * selects and caches the owner before either synchronous shell call. */
		.town_scope = RustResolverTownScope<StationResolverObject>,
		.default_scope = RustResolverBaseScopeFor<StationResolverObject>,
		.unhandled = [](uint8_t variable) noexcept { Debug(grf, 1, "Unhandled station variable 0x{:X}", variable); },
		.slope_error = RustLocationCallbackError<StationSpec, StationSpec, OpenTTDStationSlopeResult, OpenTTDCommandCost, STR_ERROR_LAND_SLOPED_IN_WRONG_DIRECTION>,
	};
	return services;
}

const OpenTTDStationAssociationServices &GetStationAssociationServices() noexcept
{
	static const OpenTTDStationAssociationServices services{
		.triggers = RustStationSpecTriggers,
		.shared = &GetRustSharedServices(),
		.roadstop = &GetRoadStopResolverServices(),
		.count = RustStationSpecCount,
		.vacancy = RustStationSlotVacancy,
		.spec = RustStationSlotSpec,
		.key = RustStationSlotKey,
		.resize = RustStationSpecResize,
		.clear = RustStationSpecClear,
		.set_slot = RustStationSetSpecSlot,
		.rebind = RustStationRebindSpec,
		.set_cached = RustStationSetCachedTriggers,
	};
	return services;
}

template <>
std::span<const OpenTTDGRFClassDefault> StationClass::Defaults()
{
	static const OpenTTDGRFClassDefault defaults[]{
		{STATION_CLASS_LABEL_DEFAULT, STR_STATION_CLASS_DFLT},
		{STATION_CLASS_LABEL_WAYPOINT, STR_STATION_CLASS_WAYP},
	};
	return defaults;
}


/* Instantiate StationClass. */
template class NewGRFClass<StationSpec, StationClassID, STAT_CLASS_MAX>;


/**
 * Convert GoodsEntry status to the form required for NewGRF variables.
 * @return NewGRF representation of GoodsEntry state.
 */
uint8_t GoodsEntry::ConvertState() const
{
	static_assert(to_underlying(GoodsEntry::State::EverAccepted) == 2);
	static_assert(to_underlying(GoodsEntry::State::LastMonth) == 3);
	static_assert(to_underlying(GoodsEntry::State::CurrentMonth) == 4);
	static_assert(to_underlying(GoodsEntry::State::AcceptedBigtick) == 5);
	return openttd_rust_goods_convert_state(this->status.base());
}



/**
 * Evaluate a tile's position within a station, and return the result in a bit-stuffed format.
 * if not centered: .TNLcCpP, if centered: .TNL..CP
 * - T = Tile layout number (#GetStationGfx)
 * - N = Number of platforms
 * - L = Length of platforms
 * - C = Current platform number from start, c = from end
 * - P = Position along platform from start, p = from end
 * .
 * if centered, C/P start from the centre and c/p are not available.
 * @return Platform information in bit-stuffed format.
 */
uint32_t GetPlatformInfo(Axis axis, uint8_t tile, int platforms, int length, int x, int y, bool centred)
{
	return openttd_rust_platform_info(axis, tile, platforms, length, x, y, centred);
}


/**
 * Find the end of a railway station, from the \a tile, in the direction of \a delta.
 * @param tile Start tile.
 * @param delta Movement direction.
 * @param check_type Stop when the custom station type changes.
 * @param check_axis Stop when the station direction changes.
 * @return Found end of the railway station.
 */

/* Station Resolver Functions */
/* virtual */ uint32_t StationScopeResolver::GetRandomBits() const
{
	return openttd_rust_station_random_bits(&this->state);
}


/* virtual */ uint32_t StationScopeResolver::GetRandomTriggers() const
{
	return openttd_rust_station_random_triggers(&this->state);
}


/**
 * Get the town scope associated with a station, if it exists.
 * On the first call, the town scope is created (if possible).
 * @return Town scope, if available.
 */
TownScopeResolver *StationResolverObject::GetTown()
{
	return openttd_rust_station_town_scope(&this->station_scope.state, this);
}

/* virtual */ uint32_t StationScopeResolver::GetVariable(uint8_t variable, [[maybe_unused]] uint32_t parameter, bool &available) const
{
	return openttd_rust_station_variable(&this->state, &this->ro, this->ro.grffile, variable, parameter, &available);
}

uint32_t Station::GetNewGRFVariable(const ResolverObject &object, uint8_t variable, uint8_t parameter, bool &available) const
{
	return openttd_rust_station_base_variable(const_cast<Station *>(this), object.grffile, variable, parameter, &available);
}

uint32_t Waypoint::GetNewGRFVariable(const ResolverObject &, uint8_t variable, [[maybe_unused]] uint8_t parameter, bool &available) const
{
	return openttd_rust_station_base_variable(const_cast<Waypoint *>(this), nullptr, variable, parameter, &available);
}

/* virtual */ const SpriteGroup *StationResolverObject::ResolveReal(const RealSpriteGroup &group) const
{
	return openttd_rust_station_resolve_real(&this->station_scope.state, &group);
}

GrfSpecFeature StationResolverObject::GetFeature() const
{
	return GSF_STATIONS;
}

uint32_t StationResolverObject::GetDebugID() const
{
	return this->station_scope.statspec->grf_prop.local_id;
}

/**
 * Resolver for stations.
 * @param statspec Station (type) specification.
 * @param base_station Instance of the station.
 * @param tile %Tile of the station.
 * @param callback Callback ID.
 * @param callback_param1 First parameter (var 10) of the callback.
 * @param callback_param2 Second parameter (var 18) of the callback.
 */
StationResolverObject::StationResolverObject(const StationSpec *statspec, BaseStation *base_station, TileIndex tile,
		CallbackID callback, uint32_t callback_param1, uint32_t callback_param2)
	: SpecializedResolverObject<StationRandomTriggers>(statspec->grf_prop.grffile, callback, callback_param1, callback_param2),
	station_scope(*this, statspec, base_station, tile)
{
	/* StationScopeResolver initialized the canonical Rust state and published its root. */
}

/**
 * Resolve sprites for drawing a station tile.
 * @param statspec Station spec
 * @param st Station (nullptr in GUI)
 * @param tile Station tile being drawn (INVALID_TILE in GUI)
 * @param var10 Value to put in variable 10; normally 0; 1 when resolving the groundsprite and StationSpecFlag::SeparateGround is set.
 * @return First sprite of the Action 1 spriteset to use, minus an offset of SPR_RAIL_PLATFORM_Y_FRONT (0x42D) to accommodate for weird NewGRF specs.
 */
SpriteID GetCustomStationRelocation(const StationSpec *statspec, BaseStation *st, TileIndex tile, uint32_t var10)
{
	StationResolverObject object(statspec, st, tile, CBID_NO_CALLBACK, var10);
	const auto *group = object.Resolve<ResultSpriteGroup>();

	/* A zero-length ResultSpriteGroup is valid because the output value is an offset, not a sprite ID within the ResultSpriteGroup. */
	if (group == nullptr) return 0;
	return group->sprite - SPR_RAIL_PLATFORM_Y_FRONT;
}

void GetCustomStationRelocation(SpriteLayoutProcessor &processor, const StationSpec *statspec, BaseStation *st, TileIndex tile)
{
	StationResolverObject object(statspec, st, tile, CBID_NO_CALLBACK);
	for (uint8_t var10 : processor.Var10Values()) {
		object.callback_param1 = var10;
		const auto *group = object.Resolve<ResultSpriteGroup>();

		/* ProcessRegisters must be called no matter the type of the sprite resolve result or whether it is valid.
		 * The sprite offset is only used for layouts with the SPRITE_MODIFIER_CUSTOM_SPRITE flag, however other aspects of layouts
		 * such as register operations must still be processed even if this flag is not set and the sprite offset is never used.
		 * A zero-length ResultSpriteGroup is valid because the output value is an offset, not a sprite ID within the ResultSpriteGroup. */
		processor.ProcessRegisters(object, var10, group != nullptr ? group->sprite - SPR_RAIL_PLATFORM_Y_FRONT : 0);
	}
}

/**
 * Resolve the sprites for custom station foundations.
 * @param statspec Station spec
 * @param st Station
 * @param tile Station tile being drawn
 * @param layout Spritelayout as returned by previous callback
 * @param edge_info Information about northern tile edges; whether they need foundations or merge into adjacent tile's foundations.
 * @return First sprite of a set of foundation sprites for various slopes, or 0 if default foundations shall be drawn.
 */
SpriteID GetCustomStationFoundationRelocation(const StationSpec *statspec, BaseStation *st, TileIndex tile, uint layout, uint edge_info)
{
	/* callback_param1 == 2 means  we are resolving the foundation sprites. */
	StationResolverObject object(statspec, st, tile, CBID_NO_CALLBACK, 2, layout | (edge_info << 16));

	const auto *group = object.Resolve<ResultSpriteGroup>();
	/* Note: SpriteGroup::Resolve zeroes all registers, so register 0x100 is initialised to 0. (compatibility) */
	uint32_t offset = static_cast<uint32_t>(object.GetRegister(0x100));
	if (group == nullptr || group->num_sprites <= offset) return 0;

	return group->sprite + offset;
}


uint16_t GetStationCallback(CallbackID callback, uint32_t param1, uint32_t param2, const StationSpec *statspec, BaseStation *st, TileIndex tile, std::span<int32_t> regs100)
{
	return openttd_rust_station_callback(callback, param1, param2, statspec, st, tile.base(), regs100.data(), regs100.size());
}

/**
 * Check the slope of a tile of a new station.
 * @param north_tile Norther tile of the station rect.
 * @param cur_tile Tile to check.
 * @param statspec Station spec.
 * @param axis Axis of the new station.
 * @param plat_len Platform length.
 * @param numtracks Number of platforms.
 * @return Succeeded or failed command.
 */
CommandCost PerformStationTileSlopeCheck(TileIndex north_tile, TileIndex cur_tile, const StationSpec *statspec, Axis axis, uint8_t plat_len, uint8_t numtracks)
{
	CommandCost result;
	openttd_rust_station_slope_check(reinterpret_cast<OpenTTDCommandCost *>(&result), north_tile.base(), cur_tile.base(), statspec, axis, plat_len, numtracks);
	return result;
}


/**
 * Allocate a StationSpec to a Station. This is called once per build operation.
 * @param spec StationSpec to allocate.
 * @param st Station to allocate it to.
 * @return Index within the Station's station spec list, or std::nullopt if the allocation failed.
 */
std::optional<uint8_t> AllocateSpecToStation(const StationSpec *spec, BaseStation *st)
{
	auto index = openttd_rust_station_spec_allocate(&GetStationAssociationServices(), spec, st);
	return index.valid ? std::optional<uint8_t>{index.value} : std::nullopt;
}

/**
 * Assign a previously allocated StationSpec specindex to a Station.
 * @param spec StationSpec to assign..
 * @param st Station to allocate it to.
 * @param specindex Spec index of allocation.
 */
void AssignSpecToStation(const StationSpec *spec, BaseStation *st, uint8_t specindex)
{
	openttd_rust_station_spec_assign(&GetStationAssociationServices(), spec, st, specindex);
}

/**
 * Deallocate a StationSpec from a Station. Called when removing a single station tile.
 * @param st Station to work with.
 * @param specindex Index of the custom station within the Station's station spec list.
 */

/**
 * Draw representation of a station tile for GUI purposes.
 * @param x Position x of image.
 * @param y Position y of image.
 * @param axis Axis.
 * @param railtype Rail type.
 * @param sclass, station Type of station.
 * @param station station ID
 * @return True if the tile was drawn (allows for fallback to default graphic)
 */
bool DrawStationTile(int x, int y, RailType railtype, Axis axis, StationClassID sclass, uint station)
{
	const DrawTileSprites *sprites = nullptr;
	const RailTypeInfo *rti = GetRailTypeInfo(railtype);
	PaletteID palette = GetCompanyPalette(_local_company);
	uint tile = 2;

	const StationSpec *statspec = StationClass::Get(sclass)->GetSpec(station);
	if (statspec == nullptr) return false;

	if (statspec->callback_mask.Test(StationCallbackMask::DrawTileLayout)) {
		uint16_t callback = GetStationCallback(CBID_STATION_DRAW_TILE_LAYOUT, 0, 0, statspec, nullptr, INVALID_TILE);
		if (callback != CALLBACK_FAILED) tile = callback & ~1;
	}

	uint32_t total_offset = rti->GetRailtypeSpriteOffset();
	uint32_t relocation = 0;
	uint32_t ground_relocation = 0;
	const NewGRFSpriteLayout *layout = nullptr;
	SpriteLayoutProcessor processor; // owns heap, borrowed by tmp_rail_layout and sprites
	DrawTileSpriteSpan tmp_rail_layout;

	if (statspec->renderdata.empty()) {
		sprites = GetStationTileLayout(StationType::Rail, tile + axis);
	} else {
		layout = &statspec->renderdata[(tile < statspec->renderdata.size()) ? tile + axis : (uint)axis];
		if (!layout->NeedsPreprocessing()) {
			sprites = layout;
			layout = nullptr;
		}
	}

	if (layout != nullptr) {
		/* Sprite layout which needs preprocessing */
		bool separate_ground = statspec->flags.Test(StationSpecFlag::SeparateGround);
		processor = SpriteLayoutProcessor(*layout, total_offset, rti->fallback_railtype, 0, 0, separate_ground);
		GetCustomStationRelocation(processor, statspec, nullptr, INVALID_TILE);
		tmp_rail_layout = processor.GetLayout();
		sprites = &tmp_rail_layout;
		total_offset = 0;
	} else {
		/* Simple sprite layout */
		ground_relocation = relocation = GetCustomStationRelocation(statspec, nullptr, INVALID_TILE, 0);
		if (HasBit(sprites->ground.sprite, SPRITE_MODIFIER_CUSTOM_SPRITE)) {
			ground_relocation = GetCustomStationRelocation(statspec, nullptr, INVALID_TILE, 1);
		}
		ground_relocation += rti->fallback_railtype;
	}

	SpriteID image = sprites->ground.sprite;
	PaletteID pal = sprites->ground.pal;
	RailTrackOffset overlay_offset;
	if (rti->UsesOverlay() && SplitGroundSpriteForOverlay(nullptr, &image, &overlay_offset)) {
		SpriteID ground = GetCustomRailSprite(rti, INVALID_TILE, RTSG_GROUND);
		DrawSprite(image, PAL_NONE, x, y);
		DrawSprite(ground + overlay_offset, PAL_NONE, x, y);
	} else {
		image += HasBit(image, SPRITE_MODIFIER_CUSTOM_SPRITE) ? ground_relocation : total_offset;
		if (HasBit(pal, SPRITE_MODIFIER_CUSTOM_SPRITE)) pal += ground_relocation;
		DrawSprite(image, GroundSpritePaletteTransform(image, pal, palette), x, y);
	}

	DrawRailTileSeqInGUI(x, y, sprites, total_offset, relocation, palette);

	return true;
}


const StationSpec *GetStationSpec(TileIndex t)
{
	if (!IsCustomStationSpecIndex(t)) return nullptr;

	const BaseStation *st = BaseStation::GetByTile(t);
	uint specindex = GetCustomStationSpecIndex(t);
	return specindex < st->speclist.size() ? st->speclist[specindex].spec : nullptr;
}

/** Wrapper for animation control, see GetStationCallback. */
uint16_t GetAnimStationCallback(CallbackID callback, uint32_t param1, uint32_t param2, const StationSpec *statspec, BaseStation *st, TileIndex tile, int)
{
	return RustEvaluateStationCallback(callback, param1, param2, statspec, st, tile.base(), INVALID_AXIS, nullptr, 0);
}

/** Helper class for animation control. */
const OpenTTDStationAnimation &GetStationAnimationBinding() noexcept
{
	static const auto binding = MakeRustAnimationBinding<StationSpec, BaseStation, int, GetAnimStationCallback>(
		[](uint32_t t) noexcept -> OpenTTDStationAnimationEntry { const auto *s = RustAnimationRailSpec(t); if (s == nullptr) return {nullptr, nullptr}; return {s, RustBaseStationByTile(t)}; },
		[](const StationSpec *s) noexcept -> OpenTTDAnimationInfo { return {s->animation.speed, s->animation.frames, s->animation.status == AnimationStatus::Looping, s->callback_mask.Test(StationCallbackMask::AnimationSpeed), s->callback_mask.Test(StationCallbackMask::AnimationNextFrame), s->flags.Test(StationSpecFlag::Cb141RandomBits), true}; });
	return binding;
}

void AnimateStationTile(TileIndex tile)
{
	openttd_rust_animate_station(tile.base());
}

void TriggerStationAnimation(BaseStation *st, TileIndex trigger_tile, StationAnimationTrigger trigger, CargoType cargo_type)
{
	openttd_rust_trigger_station(st, trigger_tile.base(), to_underlying(trigger), cargo_type);
}

/**
 * Trigger station randomisation
 * @param st station being triggered
 * @param trigger_tile specific tile of platform to trigger
 * @param trigger trigger type
 * @param cargo_type cargo type causing trigger
 */
void TriggerStationRandomisation(BaseStation *st, TileIndex trigger_tile, StationRandomTrigger trigger, CargoType cargo_type)
{
	assert(st != nullptr);
	openttd_rust_station_randomisation(st, trigger_tile.base(), static_cast<uint8_t>(trigger), cargo_type);
}

/**
 * Update the cached animation trigger bitmask for a station.
 * @param st Station to update.
 */
void StationUpdateCachedTriggers(BaseStation *st)
{
	openttd_rust_station_spec_cache(&GetStationAssociationServices(), st);
}
