/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file town_cmd.cpp Handling of town tiles. */

#include "stdafx.h"
#include "misc/history_type.hpp"
#include "misc/history_func.hpp"
#include "road.h"
#include "road_internal.h" /* Cleaning up road bits */
#include "road_cmd.h"
#include "landscape.h"
#include "viewport_func.h"
#include "viewport_kdtree.h"
#include "command_func.h"
#include "company_func.h"
#include "industry.h"
#include "station_base.h"
#include "waypoint_base.h"
#include "station_kdtree.h"
#include "company_base.h"
#include "news_func.h"
#include "error.h"
#include "object.h"
#include "genworld.h"
#include "newgrf_debug.h"
#include "newgrf_house.h"
#include "newgrf_text.h"
#include "autoslope.h"
#include "tunnelbridge_map.h"
#include "strings_func.h"
#include "window_func.h"
#include "string_func.h"
#include "newgrf_cargo.h"
#include "cheat_type.h"
#include "animated_tile_func.h"
#include "subsidy_func.h"
#include "core/pool_func.hpp"
#include "town.h"
#include "rust/command_services.h"
#include "rust/road_command_services.h"
#include "rust/services_road.h"
#include "rust/company_services.h"
#include "rust/environment_services.h"
#include "rust/services_cargo.h"
#include "rust/services_newgrf.h"
#include "rust/tile_animation_services.h"
#include "rust/services_station.h"
#include "rust/services_landscape.h"
#include "rust/settings_services.h"
#include "rust/town_founding_ffi.h"
#include "rust/services_map.h"
#include "rust/generation_services.hpp"
#include "rust/house_ffi.h"
#include "rust/town_command_ffi.h"
#include "rust/town_services.h"
#include "rust/services_object.h"
#include "rust/services_industry.h"
#include "rust/presentation_services.h"
#include "rust/services_rail.h"
#include "rust/type_availability_services.h"
#include "town_kdtree.h"
#include "townname_func.h"
#include "core/random_func.hpp"
#include "core/backup_type.hpp"
#include "depot_base.h"
#include "object_map.h"
#include "object_base.h"
#include "ai/ai.hpp"
#include "game/game.hpp"
#include "town_cmd.h"
#include "landscape_cmd.h"
#include "terraform_cmd.h"
#include "tunnelbridge_cmd.h"
#include "clear_map.h"
#include "tree_map.h"
#include "map_func.h"
#include "timer/timer.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_game_economy.h"
#include "timer/timer_game_tick.h"

#include "table/strings.h"
#include "table/town_land.h"

#include "rust/clock_ffi.h"

#include "safeguards.h"

/* Initialize the town-pool */
TownPool _town_pool("Town");
INSTANTIATE_POOL_METHODS(Town)

void RebuildTownKdtree()
{
	openttd_rust_town_rebuild_index(&GetRustSharedServices());
}

/** Set if a town is being generated. */

/**
 * Check if a town 'owns' a bridge.
 * Bridges do not directly have an owner, so we check the tiles adjacent to the bridge ends.
 * If either adjacent tile belongs to the town then it will be assumed that the town built
 * the bridge.
 * @param tile The bridge tile to test
 * @param t The town we are interested in
 * @return true If town 'owns' a bridge.
 */
Town::~Town()
{
	openttd_rust_town_destruct(&GetRustTownCommandServices(), {.shell = this, .state = this->growth_owner.get(), .id = this->index.base()});
}
void Town::PostDestructor(size_t) { openttd_rust_town_post_destruct(&GetRustTownCommandServices()); }

/**
 * Assign the town layout.
 * @param layout The desired layout. If TL_RANDOM, we pick one based on TileHash.
 */

void Town::FillCachedName() const
{
	this->cached_name = GetTownName(this);
}


static void TownDrawHouseLift(const TileInfo *ti)
{
	AddChildSpriteScreen(SPR_LIFT, PAL_NONE, 14, 60 - GetLiftPosition(ti->tile));
}

typedef void TownDrawTileProc(const TileInfo *ti);
static TownDrawTileProc * const _town_draw_tile_procs[1] = {
	TownDrawHouseLift
};

/**
 * Return a random direction
 *
 * @return a random direction
 */

/**
 * Draw a house and its tile. This is a tile callback routine.
 * @param ti TileInfo of the tile to draw
 */
static void DrawTile_Town(TileInfo *ti)
{
	HouseID house_id = GetHouseType(ti->tile);

	if (house_id >= NEW_HOUSE_OFFSET) {
		/* Houses don't necessarily need new graphics. If they don't have a
		 * spritegroup associated with them, then the sprite for the substitute
		 * house id is drawn instead. */
		if (HouseSpec::Get(house_id)->grf_prop.HasSpriteGroups()) {
			DrawNewHouseTile(ti, house_id);
			return;
		} else {
			house_id = HouseSpec::Get(house_id)->grf_prop.subst_id;
		}
	}

	/* Retrieve pointer to the draw town tile struct */
	const DrawBuildingsTileStruct *dcts = &_town_draw_tile_data[house_id << 4 | TileHash2Bit(ti->x, ti->y) << 2 | GetHouseBuildingStage(ti->tile)];

	if (ti->tileh != SLOPE_FLAT) DrawFoundation(ti, FOUNDATION_LEVELED);

	DrawGroundSprite(dcts->ground.sprite, dcts->ground.pal);

	/* If houses are invisible, do not draw the upper part */
	if (IsInvisibilitySet(TO_HOUSES)) return;

	/* Add a house on top of the ground? */
	SpriteID image = dcts->building.sprite;
	if (image != 0) {
		AddSortableSpriteToDraw(image, dcts->building.pal, *ti, *dcts, IsTransparencySet(TO_HOUSES));

		if (IsTransparencySet(TO_HOUSES)) return;
	}

	{
		int proc = dcts->draw_proc - 1;

		if (proc >= 0) _town_draw_tile_procs[proc](ti);
	}
}


/**
 * Get the foundation for a house. This is a tile callback routine.
 * @param tile The tile to find a foundation for.
 * @param tileh The slope of the tile.
 */

/**
 * Animate a tile for a town.
 * Only certain houses can be animated.
 * The newhouses animation supersedes regular ones.
 * @param tile TileIndex of the house to animate.
 */

/** Resize the sign (label) of the town after it changes population. */
void Town::UpdateVirtCoord()
{
	Point pt = RemapCoords2(TileX(this->xy) * TILE_SIZE, TileY(this->xy) * TILE_SIZE);

	if (this->cache.sign.kdtree_valid) _viewport_sign_kdtree.Remove(ViewportSignKdtreeItem::MakeTown(this->index));

	std::string town_string;
	if (this->larger_town) {
		town_string = GetString(_settings_client.gui.population_in_label ? STR_VIEWPORT_TOWN_CITY_POP : STR_VIEWPORT_TOWN_CITY, this->index, this->cache.population);
	} else {
		town_string = GetString(_settings_client.gui.population_in_label ? STR_VIEWPORT_TOWN_POP : STR_TOWN_NAME, this->index, this->cache.population);
	}

	this->cache.sign.UpdatePosition(pt.x, pt.y - 24 * ZOOM_BASE,
		town_string,
		GetString(STR_TOWN_NAME, this->index, this->cache.population)
);

	_viewport_sign_kdtree.Insert(ViewportSignKdtreeItem::MakeTown(this->index));

	SetWindowDirty(WC_TOWN_VIEW, this->index);
}

/** Update the virtual coords needed to draw the town sign for all towns. */
void UpdateAllTownVirtCoords()
{
	for (Town *t : Town::Iterate()) {
		t->UpdateVirtCoord();
	}
}

/** Clear the cached_name of all towns. */
void ClearAllTownCachedNames()
{
	for (Town *t : Town::Iterate()) {
		t->cached_name.clear();
	}
}

/**
 * Change the town's population as recorded in the town cache, town label, and town directory.
 * @param t The town which has changed.
 * @param mod The population change (can be positive or negative).
 */

/**
 * Get the total population, the sum of all towns in the world.
 * @return The calculated population of the world
 */
uint32_t GetWorldPopulation() { return openttd_rust_town_world_population(&GetRustTownServices()); }

static void AddProducedCargo_Town(TileIndex tile, CargoArray &produced)
{
	openttd_rust_house_produced(&GetRustTownServices(), &GetRustSharedServices(), tile.base(), produced.data());
}


static void AddAcceptedCargo_Town(TileIndex tile, CargoArray &acceptance, CargoTypes &always_accepted)
{
	openttd_rust_house_accepted_tile(&GetRustTownServices(), &GetRustSharedServices(), tile.base(), acceptance.data(), &always_accepted);
}

/**
 * Get accepted cargo of a house prototype.
 * @param hs Spec of the house.
 * @return CargoArray filled with cargo accepted by the house.
 */
CargoArray GetAcceptedCargoOfHouse(const HouseSpec *hs)
{
	CargoArray acceptance;
	openttd_rust_house_accepted_prototype(&GetRustTownServices(), &GetRustSharedServices(), hs->Index(), acceptance.data());
	return acceptance;
}

static void GetTileDesc_Town(TileIndex tile, TileDesc &td)
{
	const HouseID house = GetHouseType(tile);
	const HouseSpec *hs = HouseSpec::Get(house);
	bool house_completed = IsHouseCompleted(tile);

	td.str = hs->building_name;
	td.town_can_upgrade = !IsHouseProtected(tile);

	std::array<int32_t, 1> regs100;
	uint16_t callback_res = GetHouseCallback(CBID_HOUSE_CUSTOM_NAME, house_completed ? 1 : 0, 0, house, Town::GetByTile(tile), tile, regs100);
	if (callback_res != CALLBACK_FAILED && callback_res != 0x400) {
		StringID new_name = STR_NULL;
		if (callback_res == 0x40F) {
			new_name = GetGRFStringID(hs->grf_prop.grfid, static_cast<GRFStringID>(regs100[0]));
		} else if (callback_res > 0x400) {
			ErrorUnknownCallbackResult(hs->grf_prop.grfid, CBID_HOUSE_CUSTOM_NAME, callback_res);
		} else {
			new_name = GetGRFStringID(hs->grf_prop.grfid, GRFSTR_MISC_GRF_TEXT + callback_res);
		}
		if (new_name != STR_NULL && new_name != STR_UNDEFINED) {
			td.str = new_name;
		}
	}

	if (!house_completed) {
		td.dparam = td.str;
		td.str = STR_LAI_TOWN_INDUSTRY_DESCRIPTION_UNDER_CONSTRUCTION;
	}

	if (hs->grf_prop.HasGrfFile()) {
		const GRFConfig *gc = GetGRFConfig(hs->grf_prop.grfid);
		td.grf = gc->GetName();
	}

	td.owner[0] = OWNER_TOWN;
}


static void ChangeTileOwner_Town(TileIndex, Owner, Owner)
{
	/* not used */
}

/* Constants consumed by the Rust policy retain their original values. */
static_assert(Ticks::TOWN_GROWTH_TICKS == 70 && MAX_BRIDGES == 13);
static_assert(OWNER_TOWN.base() == 15 && OWNER_NONE.base() == 16);
static_assert(CalendarTime::MAX_DATE.base() == 1826212865);
static uint8_t RustTown_growth_rate() noexcept { return _settings_game.economy.town_growth_rate; }
static bool RustTown_allow_crossings() noexcept { return _settings_game.economy.allow_town_level_crossings; }
static uint8_t RustTownRoadFlags(uint8_t id) noexcept { return GetRoadTypeInfo(static_cast<RoadType>(id))->flags.base(); }
static OpenTTDTownHouseAvailability RustTownHouseAvailability(uint16_t id) noexcept
{
	const auto *hs = HouseSpec::Get(id);
	return {.zones = RustHouseAvailability(hs), .override_id = RustSpecOverride(hs), .class_id = RustTownHouseClass(hs), .probability = hs->probability, .enabled = RustSpecEnabled<HouseSpec>(hs)};
}
static OpenTTDTownHousePlacement RustTownHousePlacement(uint16_t id) noexcept
{
	const auto *hs = HouseSpec::Get(id);
	return {.min_year = RustHouseMinYear(hs), .max_year = hs->max_year.base()};
}
static uint8_t RustTownHouseExtraFlags(const HouseSpec *spec) noexcept { return spec->extra_flags.base(); }
static OpenTTDTownRoadChoice RustTownRoadInfo(uint8_t id) noexcept
{
	const auto *rti = GetRoadTypeInfo(static_cast<RoadType>(id));
	return {.introduction_date = RustRoadTypeIntroductionDate(rti), .max_speed = rti->max_speed, .town_build = RustRoadTypeTownBuild(rti)};
}
static void RustTown_dirty_town(uint32_t town) noexcept
{
	SetWindowDirty(WC_TOWN_VIEW, TownID(town));
}
static int64_t RustTown_terraform_price() noexcept
{
	return _price[PR_TERRAFORM];
}
static uint32_t RustTown_clean_road_bits(uint32_t tile, uint32_t bits) noexcept
{
	return RustCleanRoadBits(TileIndex{tile}, static_cast<RoadBits>(bits));
}
static void RustTown_set_road_town(uint32_t tile, uint32_t town) noexcept
{
	RustSetRoadOwner(tile, RTT_ROAD, OWNER_TOWN.base()); RustSetTownIndex(tile, static_cast<uint16_t>(town));
}
static uint32_t RustTown_exchange_company(uint32_t company) noexcept
{
	auto old = OpenTTDCurrentCompany();
	OpenTTDSetCurrentCompany(CompanyID(company));
	return old.base();
}
static void RustTown_make_house(uint32_t tile, uint32_t town, uint32_t house, uint32_t construction) noexcept
{
	MakeHouseTile(TileIndex{tile}, TownID(town), construction & 0xFF, (construction >> 8) & 0xFF, house, (construction >> 16) & 0xFF, construction >> 24);
}
static void RustTown_population_changed(uint32_t town) noexcept
{
	Town *t = Town::Get(TownID(town));
	OpenTTDInvalidateTownView(t->index.base());
	if (_settings_client.gui.population_in_label) t->UpdateVirtCoord();
	InvalidateWindowData(WC_TOWN_DIRECTORY, 0, TDIWD_POPULATION_CHANGE);
}

uint32_t OpenTTDNextTownID(uint32_t from) noexcept
{
	{ auto range = Town::Iterate(from); auto it = range.begin(); return it == range.end() ? UINT32_MAX : (*it)->index.base(); }
}
static bool RustTown_build_bridge(uint32_t tile, uint32_t end, uint32_t bridge, uint32_t road_type, uint32_t execute) noexcept
{
	auto flags = CommandFlagsToDCFlags(GetCommandFlags<CMD_BUILD_BRIDGE>());
	if (execute != 0) flags.Set(DoCommandFlag::Execute);
	return Command<CMD_BUILD_BRIDGE>::Do(flags, TileIndex{tile}, TileIndex{end}, TRANSPORT_ROAD, bridge, static_cast<RoadType>(road_type)).Succeeded();
}
static bool RustTown_build_tunnel(uint32_t tile, uint32_t road_type, uint32_t execute) noexcept
{
	auto flags = CommandFlagsToDCFlags(GetCommandFlags<CMD_BUILD_TUNNEL>());
	if (execute != 0) flags.Set(DoCommandFlag::Execute);
	return Command<CMD_BUILD_TUNNEL>::Do(flags, TileIndex{tile}, TRANSPORT_ROAD, static_cast<RoadType>(road_type)).Succeeded();
}
static void RustTown_assert_clear_house([[maybe_unused]] bool success) noexcept { assert(success); }
static void RustTownStations(uint32_t tile, uint32_t radius, void *context, void (*visit)(void *, const OpenTTDStation *, uint32_t)) noexcept
{
	struct Context { void *context; void (*visit)(void *, const OpenTTDStation *, uint32_t); } state{context, visit};
	RustStationRadius(tile, radius, &state, [](void *opaque, BaseStation *base) {
		const auto &state = *static_cast<Context *>(opaque);
		state.visit(state.context, RustStation(Station::From(base)), RustStationLocation(base));
	});
}
/* Thin FFI conversion facades over the shared station observations. */
static void RustTownDirtyAuthority(uint32_t town) noexcept { SetWindowDirty(WC_TOWN_AUTHORITY, TownID(town)); }

static_assert(NEW_HOUSE_OFFSET == 110 && HOUSE_NUM_ACCEPTS == 16 && MP_HOUSE == 3 && TOWN_PRODUCTION_DIVISOR == 256);
static_assert(OWNER_WATER.base() == 0x11 && CALLBACK_HOUSEPRODCARGO_END == 0x20FF && TOWN_COUNCIL_PERMISSIVE == 3);
static_assert(RATING_MAXIMUM == 1000 && RATING_HOUSE_MINIMUM == -1000 && PH_ALLOWED_CONSTRUCTED == 2 && EXPENSES_CONSTRUCTION == 0);
/* Canonical packed-map readers shared by the narrow phase observations below. */
static uint16_t RustHouseType(uint32_t tile) noexcept { return GetHouseType(TileIndex{tile}); }
static bool RustHouseCompleted(uint32_t tile) noexcept { return IsHouseCompleted(TileIndex{tile}); }
static bool RustHouseLiftHasDestination(uint32_t tile) noexcept { return LiftHasDestination(TileIndex{tile}); }
static OpenTTDTownRef RustHouseTown(uint32_t tile) noexcept
{
	return OpenTTDTownResolveByID(GetRustSharedServices().town_index(tile));
}
static OpenTTDHouseEntry RustHouseEntry(uint32_t tile) noexcept
{
	return {.town = RustHouseTown(tile), .house = RustHouseType(tile)};
}
static OpenTTDHouseLoopTile RustHouseLoopTile(uint32_t tile) noexcept
{
	return {.age = static_cast<uint8_t>(RustHouseAge(tile)), .completed = RustHouseCompleted(tile), .lift_has_destination = RustHouseLiftHasDestination(tile)};
}
static OpenTTDHouseLift RustHouseLift(uint32_t tile) noexcept
{
	return {.position = GetLiftPosition(TileIndex{tile}), .destination = GetLiftDestination(TileIndex{tile}), .has_destination = RustHouseLiftHasDestination(tile)};
}
static OpenTTDHouseRandom RustHouseRandom(uint32_t tile) noexcept
{
	return {.bits = RustHouseRandomBits(tile), .triggers = RustHouseRandomTriggers(tile)};
}
static OpenTTDHouseConstruction RustHouseConstruction(uint32_t tile) noexcept
{
	return {.house = RustHouseType(tile), .tick = GetHouseConstructionTick(TileIndex{tile}), .completed = RustHouseCompleted(tile)};
}
static OpenTTDHouseCache RustHouseCache(uint32_t tile) noexcept
{
	return {.house = RustHouseType(tile), .town = GetRustSharedServices().town_index(tile), .completed = RustHouseCompleted(tile)};
}
static OpenTTDHouseAge RustHouseAgeObservation(uint32_t tile) noexcept
{
	return {.age = static_cast<uint8_t>(RustHouseAge(tile)), .completed = RustHouseCompleted(tile)};
}
/* These phase projections call each field's single typed reader. Keep field
 * definitions here rather than repeating HouseSpec reads in a projection. */
static uint8_t RustHouseMail(const HouseSpec *spec) noexcept { return spec->mail_generation; }
static uint8_t RustHouseMinimumLife(const HouseSpec *spec) noexcept { return spec->minimum_life; }
static uint8_t RustHouseProcessingTime(const HouseSpec *spec) noexcept { return spec->processing_time; }
static OpenTTDHouseLoopSpec RustHouseLoopSpec(uint16_t house) noexcept
{
	const HouseSpec *spec = HouseSpec::Get(house);
	return {.callbacks = RustTownHouseCallbacks(spec), .flags = RustTownHouseFlags(spec), .population = RustTownHousePopulation(spec), .mail = RustHouseMail(spec), .minimum_life = RustHouseMinimumLife(spec)};
}
static OpenTTDHouseAnimationSpec RustHouseAnimationSpec(uint16_t house) noexcept
{
	const HouseSpec *spec = HouseSpec::Get(house);
	return {.callbacks = RustTownHouseCallbacks(spec), .extra_flags = RustTownHouseExtraFlags(spec)};
}
static OpenTTDHouseNewSpec RustHouseNewSpec(uint16_t house) noexcept
{
	const HouseSpec *spec = HouseSpec::Get(house);
	return {.callbacks = RustTownHouseCallbacks(spec), .flags = RustTownHouseFlags(spec), .processing_time = RustHouseProcessingTime(spec)};
}
static OpenTTDHouseProduction RustHouseProduction(uint16_t house) noexcept
{
	const HouseSpec *spec = HouseSpec::Get(house);
	return {.callbacks = RustTownHouseCallbacks(spec), .population = RustTownHousePopulation(spec), .mail = RustHouseMail(spec)};
}
static OpenTTDHouseAcceptance RustHouseAcceptance(uint16_t house) noexcept
{
	const HouseSpec *hs = HouseSpec::Get(house);
	OpenTTDHouseAcceptance result{};
	std::copy(std::begin(hs->accepts_cargo), std::end(hs->accepts_cargo), result.accepts);
	std::copy(std::begin(hs->cargo_acceptance), std::end(hs->cargo_acceptance), result.amounts);
	return result;
}
static const OpenTTDHouseServices &GetRustHouseServices() noexcept
{
	static const OpenTTDHouseServices services{
		.cost = &GetRustCommandCostServices(),
		.commands = &GetRustCommandServices(),
		.authority = &GetRustTownAuthorityServices(),
		.entry = RustHouseEntry,
		.loop_tile = RustHouseLoopTile,
		.house_type = RustHouseType,
		.completed = RustHouseCompleted,
		.is_protected = [](uint32_t tile) noexcept { return IsHouseProtected(TileIndex{tile}); },
		.lift = RustHouseLift,
		.random = RustHouseRandom,
		.construction = RustHouseConstruction,
		.cache = RustHouseCache,
		.age = RustHouseAgeObservation,
		.processing = [](uint32_t tile) noexcept -> uint8_t { return GetHouseProcessingTime(TileIndex{tile}); },
		.clean_type = [](uint32_t tile) noexcept -> uint16_t { return GetCleanHouseType(TileIndex{tile}); },
		.town = RustHouseTown,
		.loop_spec = RustHouseLoopSpec,
		.animation_spec = RustHouseAnimationSpec,
		.new_spec = RustHouseNewSpec,
		.production = RustHouseProduction,
		.sprite_groups = RustHouseHasSpriteGroups,
		.removal = [](uint16_t house) noexcept -> OpenTTDHouseRemoval { const HouseSpec *hs = HouseSpec::Get(house); return {hs->removal_cost, hs->remove_rating_decrease}; },
		.acceptance = RustHouseAcceptance,
		.watched = RustHouseWatchedByID,
		.inc_construction = [](uint32_t tile) noexcept { IncHouseConstructionTick(TileIndex{tile}); },
		.reset_age = [](uint32_t tile) noexcept { ResetHouseAge(TileIndex{tile}); },
		.set_age = [](uint32_t tile, uint8_t age) noexcept { Tile(TileIndex{tile}).m5() = age; },
		.set_lift_destination = [](uint32_t tile, uint8_t destination) noexcept { SetLiftDestination(TileIndex{tile}, destination); },
		.set_lift_position = [](uint32_t tile, uint8_t position) noexcept { SetLiftPosition(TileIndex{tile}, position); },
		.halt_lift = [](uint32_t tile) noexcept { HaltLift(TileIndex{tile}); },
		.dec_processing = [](uint32_t tile) noexcept { DecHouseProcessingTime(TileIndex{tile}); },
		.set_processing = [](uint32_t tile, uint8_t time) noexcept { SetHouseProcessingTime(TileIndex{tile}, time); },
		.set_random_bits = [](uint32_t tile, uint8_t bits) noexcept { SetHouseRandomBits(TileIndex{tile}, bits); },
		.set_random_triggers = [](uint32_t tile, uint8_t triggers) noexcept { SetHouseRandomTriggers(TileIndex{tile}, HouseRandomTriggers{triggers}); },
		.set_type = [](uint32_t tile, uint16_t house) noexcept { SetHouseType(TileIndex{tile}, house); },

		.delete_inspect = OpenTTDDeleteHouseInspect,
		.cargo_file = RustHouseGrf,
		.convert_boolean = RustHouseConvertBoolean,
		.convert_8bit = RustHouseConvert8bitBoolean,

		.scale_cargo = OpenTTDScaleTownCargo,
		.cargogen_mode = OpenTTDTownCargoGenMode,
		.place_houses = OpenTTDPlaceHouses,
		.company_human = OpenTTDCompanyIsValidHuman,
		.errors = {STR_ERROR_BUILDING_MUST_BE_DEMOLISHED, STR_ERROR_BUILDING_IS_PROTECTED, STR_ERROR_LOCAL_AUTHORITY_REFUSES_TO_ALLOW_THIS,
			STR_ERROR_LAND_SLOPED_IN_WRONG_DIRECTION, STR_ERROR_MUST_DEMOLISH_BRIDGE_FIRST, STR_ERROR_FLAT_LAND_REQUIRED, STR_ERROR_MUST_FOUND_TOWN_FIRST},
	};
	return services;
}
const OpenTTDTownLeaves &GetRustTownServices() noexcept
{
	static const OpenTTDTownLeaves services{
		.map_log_x = OpenTTDMapLogX,
		.growth_rate = RustTown_growth_rate,
		.snow_line = GetRustSharedServices().snow_line,
		.environment = GetRustEnvironmentServices(),
		.generating_world = GetRustSharedServices().clear_square_services.generating_world,
		.allow_crossings = RustTown_allow_crossings,
		.climate = OpenTTDLandscapeValue,
		.highest_snow_line = OpenTTDHighestSnowLine,
		.allow_roads = RustAllowTownRoads,
		.road_flags = RustTownRoadFlags,
		.house_availability = RustTownHouseAvailability,
		.house_placement = RustTownHousePlacement,
		.house_population = [](uint16_t house) noexcept -> uint8_t { return RustTownHousePopulation(HouseSpec::Get(house)); },
		.house_flags = [](uint16_t house) noexcept -> uint8_t { return RustTownHouseFlags(HouseSpec::Get(house)); },
		.house_extra_flags = [](uint16_t house) noexcept -> uint8_t { return RustTownHouseExtraFlags(HouseSpec::Get(house)); },
		.house_callbacks = [](uint16_t house) noexcept -> uint16_t { return RustTownHouseCallbacks(HouseSpec::Get(house)); },
		.road_info = RustTownRoadInfo,
		.resolve_owner = OpenTTDTownOwnerByID,
		.stations = RustTownStations,
		.station_activity_owner = OpenTTDStationActivityOwner,
		.station_owner = RustStationOwnerByte<const Station>,
		.dirty_authority = RustTownDirtyAuthority,
		.dirty_town = RustTown_dirty_town,

		.terraform_price = RustTown_terraform_price,
		.clean_road_bits = RustTown_clean_road_bits,
		.set_road_town = RustTown_set_road_town,
		.exchange_company = RustTown_exchange_company,
		.house_class = RustTownHouseClass,
		.make_house = RustTown_make_house,

		.population_changed = RustTown_population_changed,
		.house_count = RustHouseSpecCount32,
		.road_type_error = [](int32_t date) noexcept {
			if (date < INT32_MAX) {
				ShowErrorMessage(GetEncodedString(STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_YET), GetEncodedString(STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_YET_EXPLANATION, TimerGameCalendar::Date{date}), WL_CRITICAL);
			} else {
				ShowErrorMessage(GetEncodedString(STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_AT_ALL), GetEncodedString(STR_ERROR_NO_TOWN_ROADTYPES_AVAILABLE_AT_ALL_EXPLANATION), WL_CRITICAL);
			}
		},
		.next_town = GetRustSharedServices().town_next_id,
		.build_bridge = RustTown_build_bridge,
		.build_tunnel = RustTown_build_tunnel,
		.assert_clear_house = RustTown_assert_clear_house,
		.houses = &GetRustHouseServices(),
	};
	return services;
}


/** Iterate through all towns and call their tick handler. */

/**
 * Return the RoadBits of a tile, ignoring depot and bay road stops.
 * @param tile The tile to check.
 * @return The roadbits of the given tile.
 */

/**
 * Get the road type that towns should build at this current moment.
 * They may have built a different type in the past.
 */
RoadType GetTownRoadType() { return static_cast<RoadType>(openttd_rust_town_road_type(&GetRustTownServices(), &GetRustSharedServices())); }

/**
 * Check if towns are able to build road.
 * @return true iff the towns are currently able to build road.
 */
bool CheckTownRoadTypes()
{
	return openttd_rust_town_check_road_types(&GetRustTownServices());
}

/**
 * Check for parallel road inside a given distance.
 *   Assuming a road from (tile - TileOffsByDiagDir(dir)) to tile,
 *   is there a parallel road left or right of it within distance dist_multi?
 *
 * @param tile The current tile.
 * @param dir The target direction.
 * @param dist_multi The distance multiplier.
 * @return true if there is a parallel road.
 */

/**
 * Update the cached town zone radii of a town, based on the number of houses.
 * @param t The town to update.
 */
void UpdateTownRadius(Town *t) { openttd_rust_town_update_radius(t->growth_owner.get()); }

/**
 * Update the maximum amount of monthly passengers and mail for a town, based on its population.
 * @param t The town to update.
 */
void UpdateTownMaxPass(Town *t)
{
	openttd_rust_town_update_max_pass(&GetRustTownServices(), t->growth_owner.get());
}


/**
 * Actually create a town.
 *
 * @param t The town.
 * @param tile Where to put it.
 * @param townnameparts The town name.
 * @param size The preset size of the town.
 * @param city Should we create a city?
 * @param layout The road layout of the town.
 * @param manual Was the town placed manually?
 */
static_assert(TAE_FOOD == 5 && TAE_WATER == 4 && OWNER_DEITY.base() == 0x12);
static_assert(EXPENSES_OTHER == 12 && TownPool::MAX_SIZE == 64000);
static_assert(INVALID_STRING_ID == 0xFFFF && MAX_LENGTH_TOWN_NAME_CHARS == 32);
const OpenTTDTownFoundingServices &GetRustTownFoundingServices() noexcept
{
	static const OpenTTDTownFoundingServices services{
		.shared = &GetRustSharedServices(),
		.growth = &GetRustTownServices(),
		.names = &GetRustTownNameServices(),
		.cost = &GetRustCommandCostServices(),
		.allocate = [](uint32_t tile) noexcept -> OpenTTDTownRef { Town *t = new Town(TileIndex(tile)); return {.shell = t, .state = t->growth_owner.get(), .id = t->index.base()}; },
		.resolve = [](uint16_t id) noexcept -> OpenTTDTownRef { Town *t = Town::GetIfValid(TownID(id)); return {.shell = t, .state = t == nullptr ? nullptr : t->growth_owner.get(), .id = id}; },
		.can_allocate = []() noexcept { return Town::CanAllocateItem(); },
		.flat = OpenTTDIsTileFlat,
		.water_distance = RustClosestWaterDistance,
		.scale_size = OpenTTDMapScaleSize,
		.scale_land = OpenTTDMapScaleLandProportion,
		.found_setting = OpenTTDTownFoundingSetting,
		.layout_setting = OpenTTDTownLayoutSetting,
		.initial_city_size = OpenTTDInitialCitySize,
		.town_density = OpenTTDTownDensity,
		.custom_number = OpenTTDCustomTownNumber,
		.larger_towns = OpenTTDLargerTowns,
		.name_style = OpenTTDTownNameStyle,
		.prepare_rename = [](Town *town) noexcept { town->cached_name.clear(); },
		.name_changed = [](Town *town, OpenTTDByteSpan name) noexcept { town->custom_name_cache.assign(reinterpret_cast<const char *>(name.data), name.size); },
		.update_virt = [](Town *town) noexcept { town->UpdateVirtCoord(); },
		.directory_rebuild = []() noexcept { InvalidateWindowData(WC_TOWN_DIRECTORY, 0, TDIWD_FORCE_REBUILD); },
		.directory_resort = []() noexcept { InvalidateWindowData(WC_TOWN_DIRECTORY, 0, TDIWD_FORCE_RESORT); },
		.update_noise = []() noexcept { UpdateAirportsNoise(); },
		.delete_town = [](uint16_t id) noexcept { [[maybe_unused]] CommandCost rc = Command<CMD_DELETE_TOWN>::Do(DoCommandFlag::Execute, TownID(id)); assert(rc.Succeeded()); },
		.assert_random_context = OpenTTDAssertTownGenerationContext,
		.assert_allocation = []() noexcept { assert(Town::CanAllocateItem()); },
		.initialize_cost = [](OpenTTDCommandCost *cost, int multiplier) noexcept { auto *native = reinterpret_cast<CommandCost *>(cost); *native = CommandCost(EXPENSES_OTHER, _price[PR_BUILD_TOWN]); native->MultiplyCost(multiplier); },
		.command = &GetRustCommandServices(),
		.money_deficit = [](OpenTTDTownFoundingFrame *frame, const OpenTTDCommandCost *cost) noexcept { frame->extra_money = reinterpret_cast<const CommandCost *>(cost)->GetCost(); },
		.set_result = [](OpenTTDTownFoundingFrame *frame, uint16_t id) noexcept { frame->town_id = TownID(id); },
		.exchange_generating = OpenTTDExchangeGeneratingWorld,
		.founded_news = [](uint16_t value, uint32_t position) noexcept {
			TownID id(value); TileIndex tile(position);
			if (_current_company == OWNER_DEITY) {
				AddTileNewsItem(GetEncodedString(STR_NEWS_NEW_TOWN_UNSPONSORED, id), NewsType::IndustryOpen, tile);
			} else {
				std::string company_name = GetString(STR_COMPANY_NAME, _current_company);
				AddTileNewsItem(GetEncodedString(STR_NEWS_NEW_TOWN, company_name, id), NewsType::IndustryOpen, tile);
			}
		},
		.founded_events = [](uint16_t value) noexcept { TownID id(value); AI::BroadcastNewEvent(new ScriptEventTownFounded(id)); Game::NewEvent(new ScriptEventTownFounded(id)); },
		.clear_station_names = []() noexcept { ClearAllStationCachedNames(); },
		.clear_industry_names = []() noexcept { ClearAllIndustryCachedNames(); },
		.update_station_coords = []() noexcept { UpdateAllStationVirtCoords(); },
		.cached_name = [](const Town *town) noexcept -> OpenTTDByteSpan { const auto &name = town->GetCachedName(); return {reinterpret_cast<const uint8_t *>(name.data()), name.size()}; },
		.set_progress = [](OpenTTDTownFoundingFrame *frame, uint total) noexcept { return RunGenerationProgress(frame, [total] { RustSetGeneratingWorldProgress(GWP_TOWN, total); }); },
		.increase_progress = [](OpenTTDTownFoundingFrame *frame) noexcept { return RunGenerationProgress(frame, [] { RustIncreaseGeneratingWorldProgress(GWP_TOWN); }); },
		.no_town_error = []() noexcept { ShowErrorMessage(GetEncodedString(STR_ERROR_COULD_NOT_CREATE_TOWN), {}, WL_CRITICAL); },
		.random = &_random,
		.edge_error = STR_ERROR_TOO_CLOSE_TO_EDGE_OF_MAP_SUB,
		.close_error = STR_ERROR_TOO_CLOSE_TO_ANOTHER_TOWN,
		.unsuitable_error = STR_ERROR_SITE_UNSUITABLE,
		.unique_error = STR_ERROR_NAME_MUST_BE_UNIQUE,
		.too_many_error = STR_ERROR_TOO_MANY_TOWNS,
		.no_space_error = STR_ERROR_NO_SPACE_FOR_TOWN,
	};
	return services;
}

std::tuple<CommandCost, Money, TownID> CmdFoundTown(DoCommandFlags flags, TileIndex tile, TownSize size, bool city, TownLayout layout, bool random_location, uint32_t townnameparts, const std::string &text)
{
	OpenTTDTownFoundingFrame frame;
	CommandCost cost;
	RustRoadCommandContext road;
	openttd_rust_town_found(&GetRustTownFoundingServices(), &road.view, &frame, &frame.names, RustCommandCost(&cost), tile.base(), size, city, layout, random_location, townnameparts, {reinterpret_cast<const uint8_t *>(text.data()), text.size()}, flags.Test(DoCommandFlag::Execute));
	return {std::move(cost), frame.extra_money, frame.town_id};
}
uint GetDefaultTownsForMapSize()
{
	return openttd_rust_town_default_number(&GetRustTownFoundingServices());
}
bool GenerateTowns(TownLayout layout, std::optional<uint> number)
{
	OpenTTDTownFoundingFrame frame;
	bool result;
	RustRoadCommandContext road;
	if (!openttd_rust_town_generate(&GetRustTownFoundingServices(), &road.view, &frame, &frame.names, layout, number.has_value(), number.value_or(0), &result)) std::rethrow_exception(frame.cancellation);
	return result;
}


HouseZones GetClimateMaskForLandscape()
{
	return HouseZones{openttd_rust_town_climate_mask(OpenTTDLandscapeValue())};
}

/**
 * Create a random town somewhere in the world.
 * @param attempts How many times should we try?
 * @param townnameparts The name of the town.
 * @param size The size preset of the town.
 * @param city Should we build a city?
 * @param layout The road layout to build.
 * @return The town object, or nullptr if we failed to create a town anywhere.
 */

/**
 * Returns the bit corresponding to the town zone of the specified tile.
 * @param t Town on which town zone is to be found.
 * @param tile TileIndex where town zone needs to be found.
 * @return the bit position of the given zone, as defined in HouseZones.
 */
HouseZone GetTownRadiusGroup(const Town *t, TileIndex tile)
{
	return static_cast<HouseZone>(openttd_rust_town_radius_group(t->growth_owner.get(), &GetRustSharedServices(), tile.base()));
}

/**
 * Clears tile and builds a house or house part.
 * @param tile The tile to build upon.
 * @param t The town which will own the house.
 * @param counter The construction stage counter for the house.
 * @param stage The current construction stage of the house.
 * @param type The type of house.
 * @param random_bits Random bits for newgrf houses to use.
 * @param is_protected Whether the house is protected from the town upgrading it.
 * @pre The house can be built here.
 */

/**
 * Determines if a given HouseID is part of a multitile house.
 * The given ID is set to the ID of the north tile and the TileDiff to the north tile is returned.
 *
 * @param house Is changed to the HouseID of the north tile of the same house
 * @return TileDiff from the tile of the given HouseID to the north tile
 */

/**
 * Place an individual house.
 * @param flags Type of operation.
 * @param tile Tile on which to place the house.
 * @param HouseID The HouseID of the house spec.
 * @param is_protected Whether the house is protected from the town upgrading it.
 * @param replace Whether to automatically demolish an existing house on this tile, if present.
 * @return Empty cost or an error.
 */
CommandCost CmdPlaceHouse(DoCommandFlags flags, TileIndex tile, HouseID house, bool is_protected, bool replace)
{
	CommandCost result;
	openttd_rust_house_place(&GetRustTownServices(), &GetRustSharedServices(), RustCommandCost(&result), flags.base(), tile.base(), house, is_protected, replace);
	return result;
}

/**
 * Rename a town (server-only).
 * @param flags type of operation
 * @param town_id town ID to rename
 * @param text the new name or an empty string when resetting to the default
 * @return the cost of this operation or an error
 */
CommandCost CmdRenameTown(DoCommandFlags flags, TownID town_id, const std::string &text)
{
	CommandCost result;
	openttd_rust_town_rename(&GetRustTownFoundingServices(), RustCommandCost(&result), town_id.base(), {reinterpret_cast<const uint8_t *>(text.data()), text.size()}, flags.Test(DoCommandFlag::Execute));
	return result;
}

/**
 * Determines the first cargo with a certain town effect
 * @param effect Town effect of interest
 * @return first active cargo slot with that effect
 */
const CargoSpec *FindFirstCargoWithTownAcceptanceEffect(TownAcceptanceEffect effect)
{
	const CargoType cargo = openttd_rust_town_first_cargo(effect);
	return cargo == INVALID_CARGO ? nullptr : CargoSpec::Get(cargo);
}

/**
 * Change the cargo goal of a town.
 * @param flags Type of operation.
 * @param town_id Town ID to cargo game of.
 * @param tae TownEffect to change the game of.
 * @param goal The new goal value.
 * @return Empty cost or an error.
 */
const OpenTTDTownCommandServices &GetRustTownCommandServices() noexcept
{
	static const OpenTTDTownCommandServices services{
		.founding = &GetRustTownFoundingServices(),
		.rail = &GetRustRailMapServices(),
		.cargo = CargoStorageServices(),
		.objects = &GetRustObjectServices(),
		.subsidies = &GetRustSubsidyServices(),
		.fund_roads = OpenTTDFundRoads,
		.fund_buildings = OpenTTDFundBuildings,
		.exclusive_rights = OpenTTDExclusiveRights,
		.bribe = OpenTTDBribe,
		.invalidate_view = OpenTTDInvalidateTownView,
		.invalidate_authority = [](uint16_t town) noexcept { InvalidateWindowData(WC_TOWN_AUTHORITY, TownID(town)); },
		.modify_station_rating = OpenTTDModifyStationRatingAround,
		.road_news = [](uint16_t id) noexcept {
			std::string company_name = GetString(STR_COMPANY_NAME, _current_company);
			AddNewsItem(GetEncodedString(TimerGameEconomy::UsingWallclockUnits() ? STR_NEWS_ROAD_REBUILDING_MINUTES : STR_NEWS_ROAD_REBUILDING_MONTHS, TownID(id), company_name), NewsType::General, NewsStyle::Normal, {}, TownID(id));
		},
		.road_events = [](uint16_t id) noexcept { AI::BroadcastNewEvent(new ScriptEventRoadReconstruction(_current_company, TownID(id))); Game::NewEvent(new ScriptEventRoadReconstruction(_current_company, TownID(id))); },
		.rights_news = [](uint16_t id) noexcept {
			auto cni = std::make_unique<CompanyNewsInformation>(STR_NEWS_EXCLUSIVE_RIGHTS_TITLE, Company::Get(_current_company));
			EncodedString message = GetEncodedString(TimerGameEconomy::UsingWallclockUnits() ? STR_NEWS_EXCLUSIVE_RIGHTS_DESCRIPTION_MINUTES : STR_NEWS_EXCLUSIVE_RIGHTS_DESCRIPTION_MONTHS, TownID(id), cni->company_name);
			AddNewsItem(std::move(message), NewsType::General, NewsStyle::Company, {}, TownID(id), {}, std::move(cni));
		},
		.rights_events = [](uint16_t id) noexcept { AI::BroadcastNewEvent(new ScriptEventExclusiveTransportRights(_current_company, TownID(id))); Game::NewEvent(new ScriptEventExclusiveTransportRights(_current_company, TownID(id))); },
		.station_views_dirty = []() noexcept { SetWindowClassesDirty(WC_STATION_VIEW); },
		.next_station = [](uint32_t from) noexcept -> OpenTTDTownStationCursor { if (Station *st = OpenTTDNextStation(from)) return {.shell = st, .next = RustBaseStationID(st) + 1U}; return {.shell = nullptr, .next = UINT32_MAX}; },
		.station_town = RustStationStationTown,
		.station_owner = RustStationOwnerByte<const BaseStation>,
		.station_airport = RustStationHasAirport,
		.airport_type = RustStationAirportType,
		.airport_tile = RustStationAirportTile,
		.station_activity_owner = OpenTTDStationActivityOwner,
		.local_company = RustIsLocalCompany,
		.bribe_error = []() noexcept { ShowErrorMessage(GetEncodedString(STR_ERROR_BRIBE_FAILED), {}, WL_INFO); },
		.next_waypoint = [](uint32_t from) noexcept -> OpenTTDTownPoolRelation { for (const Waypoint *wp : Waypoint::Iterate(from)) return {.next = RustBaseStationID(wp) + 1U, .town = OpenTTDBaseStationTown(wp)}; return {.next = UINT32_MAX, .town = nullptr}; },
		.next_depot = [](uint32_t from) noexcept -> OpenTTDTownPoolRelation { for (const Depot *d : Depot::Iterate(from)) return {.next = d->index.base() + 1U, .town = d->town}; return {.next = UINT32_MAX, .town = nullptr}; },
		.next_industry = [](uint32_t from) noexcept -> OpenTTDTownPoolRelation { if (const Industry *i = OpenTTDNextIndustry(from)) return {.next = i->index.base() + 1U, .town = OpenTTDIndustryTown(i)}; return {.next = UINT32_MAX, .town = nullptr}; },
		.set_object_town = OpenTTDSetObjectTown,
		.industry_town_by_tile = OpenTTDIndustryTownByTile,
		.remove_town_sign = [](Town *t) noexcept { if (t->cache.sign.kdtree_valid) _viewport_sign_kdtree.Remove(ViewportSignKdtreeItem::MakeTown(t->index)); },
		.delete_town = [](Town *town) noexcept { delete town; },
		.cleaning_pool = []() noexcept { return Town::CleaningPool(); },
		.close_view = [](uint16_t town) noexcept { CloseWindowById(WC_TOWN_VIEW, TownID(town)); },
		.close_graph = [](uint16_t town) noexcept { CloseWindowById(WC_TOWN_CARGO_GRAPH, TownID(town)); },
		.storage_count = RustTownStorageCount<Town>,
		.delete_storage = [](Town *town, size_t index) noexcept { delete town->psa_list[index]; },
		.clear_storage = [](Town *town) noexcept { town->psa_list.clear(); },
		.delete_inspect = OpenTTDDeleteTownInspect,
#ifdef WITH_ASSERT
		.assertions = true,
#else
		.assertions = false,
#endif
		.too_many_objects = STR_ERROR_TOO_MANY_OBJECTS,
		.no_statue_place = STR_ERROR_STATUE_NO_SUITABLE_PLACE,
	};
	return services;
}

CommandCost CmdTownCargoGoal(DoCommandFlags flags, TownID town_id, TownAcceptanceEffect tae, uint32_t goal)
{
	CommandCost result;
	openttd_rust_town_cargo_goal(&GetRustTownCommandServices(), RustCommandCost(&result), flags.base(), town_id.base(), tae, goal);
	return result;
}

/**
 * Set a custom text in the Town window.
 * @param flags Type of operation.
 * @param town_id Town ID to change the text of.
 * @param text The new text (empty to remove the text).
 * @return Empty cost or an error.
 */
CommandCost CmdTownSetText(DoCommandFlags flags, TownID town_id, const EncodedString &text)
{
	CommandCost result;
	openttd_rust_town_text_command(&GetRustTownCommandServices(), RustCommandCost(&result), flags.base(), town_id.base(), Town::TextView(text));
	return result;
}

/**
 * Change the growth rate of the town.
 * @param flags Type of operation.
 * @param town_id Town ID to cargo game of.
 * @param growth_rate Amount of days between growth, or TOWN_GROWTH_RATE_NONE, or 0 to reset custom growth rate.
 * @return Empty cost or an error.
 */
CommandCost CmdTownGrowthRate(DoCommandFlags flags, TownID town_id, uint16_t growth_rate)
{
	CommandCost result;
	openttd_rust_town_growth_command(&GetRustTownCommandServices(), RustCommandCost(&result), flags.base(), town_id.base(), growth_rate);
	return result;
}

/**
 * Change the rating of a company in a town
 * @param flags Type of operation.
 * @param town_id Town ID to change, bit 16..23 =
 * @param company_id Company ID to change.
 * @param rating New rating of company (signed int16_t).
 * @return Empty cost or an error.
 */
CommandCost CmdTownRating(DoCommandFlags flags, TownID town_id, CompanyID company_id, int16_t rating)
{
	CommandCost result;
	openttd_rust_town_rating_command(&GetRustTownCommandServices(), RustCommandCost(&result), flags.base(), town_id.base(), company_id.base(), rating);
	return result;
}

/**
 * Expand a town (scenario editor only).
 * @param flags Type of operation.
 * @param TownID Town ID to expand.
 * @param grow_amount Amount to grow, or 0 to grow a random size up to the current amount of houses.
 * @return Empty cost or an error.
 */
CommandCost CmdExpandTown(DoCommandFlags flags, TownID town_id, uint32_t grow_amount, TownExpandModes modes)
{
	CommandCost result;
	RustRoadCommandContext road;
	openttd_rust_town_expand_command(&GetRustTownCommandServices(), &road.view, RustCommandCost(&result), flags.base(), town_id.base(), grow_amount, modes.base());
	return result;
}

/**
 * Delete a town (scenario editor or worldgen only).
 * @param flags Type of operation.
 * @param town_id Town ID to delete.
 * @return Empty cost or an error.
 */
CommandCost CmdDeleteTown(DoCommandFlags flags, TownID town_id)
{
	CommandCost result;
	openttd_rust_town_delete_command(&GetRustTownCommandServices(), RustCommandCost(&result), flags.base(), town_id.base());
	return result;
}

/**
 * Get cost factors for a TownAction
 * @param action TownAction to get cost factor for.
 * @returns Cost factor.
 */
uint8_t GetTownActionCost(TownAction action) { return openttd_rust_town_action_cost(to_underlying(action)); }
TownActions GetMaskOfTownActions(CompanyID cid, const Town *t)
{
	return TownActions(openttd_rust_town_action_mask(&GetRustTownCommandServices(), cid.base(), {.shell = const_cast<Town *>(t), .state = t->growth_owner.get(), .id = t->index.base()}));
}

/**
 * Do a town action.
 * This performs an action such as advertising, building a statue, funding buildings,
 * but also bribing the town-council
 * @param flags type of operation
 * @param town_id town to do the action at
 * @param action action to perform, @see _town_action_proc for the list of available actions
 * @return the cost of this operation or an error
 */
CommandCost CmdDoTownAction(DoCommandFlags flags, TownID town_id, TownAction action)
{
	CommandCost result;
	openttd_rust_town_action(&GetRustTownCommandServices(), RustCommandCost(&result), flags.base(), town_id.base(), to_underlying(action));
	return result;
}


/**
 * Updates town grow counter after growth rate change.
 * Preserves relative house builting progress whenever it can.
 * @param t The town to calculate grow counter for
 * @param prev_growth_rate Town growth rate before it changed (one that was used with grow counter to be updated)
 */

/**
 * Checks whether the local authority allows construction of a new station (rail, road, airport, dock) on the given tile
 * @param tile The tile where the station shall be constructed.
 * @param flags Command flags. DoCommandFlag::NoTestTownRating is tested.
 * @return Succeeded or failed command.
 */
static uint8_t RustTownCouncilTolerance() noexcept { return _settings_game.difficulty.town_council_tolerance; }
static uint8_t RustTownAuthorityDistance() noexcept { return _settings_game.economy.dist_local_authority; }
const OpenTTDTownAuthorityServices &GetRustTownAuthorityServices() noexcept
{
	static const OpenTTDTownAuthorityServices services{
		.council_tolerance = RustTownCouncilTolerance, .bulldozer = []() noexcept { return _cheats.magic_bulldozer.value; },
		.authority_distance = RustTownAuthorityDistance, .town_index = RustTownAuthorityIndex,
		.refusal = STR_ERROR_LOCAL_AUTHORITY_REFUSES_TO_ALLOW_THIS,
	};
	return services;
}

CommandCost CheckIfAuthorityAllowsNewStation(TileIndex tile, DoCommandFlags flags)
{
	CommandCost result;
	openttd_rust_town_authority_station(&GetRustTownAuthorityServices(), &GetRustSharedServices(), &GetRustCommandCostServices(), RustCommandCost(&result), tile.base(), flags.Test(DoCommandFlag::NoTestTownRating));
	return result;
}

/**
 * Return the town closest to the given tile within \a threshold.
 * @param tile      Starting point of the search.
 * @param threshold Biggest allowed distance to the town.
 * @return Closest town to \a tile within \a threshold, or \c nullptr if there is no such town.
 *
 * @note This function only uses distance, the #ClosestTownFromTile function also takes town ownership into account.
 */
Town *CalcClosestTownFromTile(TileIndex tile, uint threshold)
{
	return const_cast<Town *>(reinterpret_cast<const Town *>(openttd_rust_town_calc_closest(&GetRustSharedServices(), tile.base(), threshold)));
}

/**
 * Return the town closest (in distance or ownership) to a given tile, within a given threshold.
 * @param tile      Starting point of the search.
 * @param threshold Biggest allowed distance to the town.
 * @return Closest town to \a tile within \a threshold, or \c nullptr if there is no such town.
 *
 * @note If you only care about distance, you can use the #CalcClosestTownFromTile function.
 */
Town *ClosestTownFromTile(TileIndex tile, uint threshold)
{
	return const_cast<Town *>(reinterpret_cast<const Town *>(openttd_rust_town_closest(&GetRustSharedServices(), tile.base(), threshold)));
}

void SetTownRatingTestMode(bool mode) { openttd_rust_town_rating_test(mode); }
void ChangeTownRating(Town *t, int add, int max, DoCommandFlags flags)
{
	openttd_rust_town_change_rating(&GetRustTownServices(), &GetRustSharedServices(), t == nullptr ? nullptr : t->growth_owner.get(),
		t == nullptr ? TownID::Invalid().base() : t->index.base(), reinterpret_cast<uintptr_t>(t), _current_company.base(), add, max,
		flags.Test(DoCommandFlag::NoModifyTownRating), _cheats.magic_bulldozer.value);
}

/**
 * Does the town authority allow the (destructive) action of the current company?
 * @param flags Checking flags of the command.
 * @param t     Town that must allow the company action.
 * @param type  Type of action that is wanted.
 * @return A succeeded command if the action is allowed, a failed command if it is not allowed.
 */
CommandCost CheckforTownRating(DoCommandFlags flags, Town *t, TownRatingCheckType type)
{
	CommandCost result;
	openttd_rust_town_authority_remove(&GetRustTownAuthorityServices(), &GetRustSharedServices(), &GetRustCommandCostServices(), RustCommandCost(&result), reinterpret_cast<const OpenTTDTownHandle *>(t), flags.Test(DoCommandFlag::NoTestTownRating), to_underlying(type));
	return result;
}

template <>
Town::SuppliedHistory SumHistory(std::span<const Town::SuppliedHistory> history)
{
	const auto sum = openttd_rust_town_sum_history(reinterpret_cast<const OpenTTDTownSuppliedHistory *>(history.data()), history.size());
	return {.production = sum.production, .transported = sum.transported};
}

static const OpenTTDClockTown clock_town{&GetRustTownServices(), &GetRustSharedServices()};
static const IntervalTimer<TimerGameEconomy> _economy_towns_monthly({TimerGameEconomy::MONTH, TimerGameEconomy::Priority::TOWN}, RustClockCallback{openttd_rust_clock_town_monthly, &clock_town});

static const IntervalTimer<TimerGameEconomy> _economy_towns_yearly({TimerGameEconomy::YEAR, TimerGameEconomy::Priority::TOWN}, RustClockCallback{openttd_rust_clock_town_yearly, &clock_town});


/** Tile callback functions for a town */
extern const TileTypeProcs _tile_type_town_procs = {
	DrawTile_Town,           // draw_tile_proc
	nullptr, // Rust movement owner dispatches directly.
	nullptr,                 // Rust landscape commands call the owner directly.
	AddAcceptedCargo_Town,   // add_accepted_cargo_proc
	GetTileDesc_Town,        // get_tile_desc_proc
	nullptr, // Rust movement owner dispatches directly.
	nullptr,                    // click_tile_proc
	nullptr,                 // Rust scheduler calls the owner directly.
	ChangeTileOwner_Town,    // change_tile_owner_proc
	AddProducedCargo_Town,   // add_produced_cargo_proc
	nullptr,                    // vehicle_enter_tile_proc
	nullptr, // Rust movement owner dispatches directly.
	nullptr,                 // Rust landscape commands call the owner directly.
	nullptr, // check_build_above_proc
};

std::span<const DrawBuildingsTileStruct> GetTownDrawTileData()
{
	return _town_draw_tile_data;
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDTownLeaves.hpp"
#include "abi-visit-OpenTTDTownCommandServices.hpp"
#include "abi-visit-OpenTTDTownFoundingServices.hpp"
#include "abi-visit-OpenTTDTownAuthorityServices.hpp"
#include "abi-visit-OpenTTDHouseServices.hpp"
#include "abi-visit-OpenTTDHouseResolverServices.hpp"
#include "abi-visit-OpenTTDEnvironmentServices.hpp"
bool CheckAbi_town_cmd()
{
	return AbiServicesComplete(GetRustTownCommandServices()) && AbiServicesComplete(GetRustTownServices().environment) &&
		AbiServicesComplete(GetRustTownServices()) &&
		AbiServicesComplete(GetRustTownFoundingServices()) &&
		AbiServicesComplete(GetRustTownAuthorityServices()) &&
		AbiServicesComplete(*GetRustTownServices().houses) && AbiServicesComplete(GetHouseResolverServices());
}
