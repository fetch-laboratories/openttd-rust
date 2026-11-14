/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file industry_cmd.cpp Handling of industry tiles. */

#include "stdafx.h"
#include "misc/history_type.hpp"
#include "misc/history_func.hpp"
#include "clear_map.h"
#include "industry.h"
#include "station_base.h"
#include "landscape.h"
#include "viewport_func.h"
#include "command_func.h"
#include "town.h"
#include "news_func.h"
#include "cheat_type.h"
#include "company_base.h"
#include "genworld.h"
#include "tree_map.h"
#include "newgrf_cargo.h"
#include "newgrf_debug.h"
#include "newgrf_industrytiles.h"
#include "autoslope.h"
#include "water.h"
#include "strings_func.h"
#include "window_func.h"
#include "vehicle_func.h"
#include "sound_func.h"
#include "animated_tile_func.h"
#include "effectvehicle_func.h"
#include "effectvehicle_base.h"
#include "ai/ai.hpp"
#include "core/pool_func.hpp"
#include "subsidy_func.h"
#include "core/backup_type.hpp"
#include "object_base.h"
#include "game/game.hpp"
#include "error.h"
#include "string_func.h"
#include "industry_cmd.h"
#include "landscape_cmd.h"
#include "terraform_cmd.h"
#include "map_func.h"
#include "timer/timer.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_game_economy.h"
#include "timer/timer_game_tick.h"

#include "table/strings.h"
#include "table/industry_land.h"

#include "rust/services_industry.h"
#include "rust/industry_generation_adapter.hpp"
#include "rust/subsidy_ffi.h"
#include "rust/landscape_loop_ffi.h"
#include "rust/ground_services.h"
#include "rust/company_services.h"
#include "rust/environment_services.h"
#include "rust/services_cargo.h"
#include "rust/services_newgrf.h"
#include "rust/industry_resolver_ffi.h"
#include "rust/command_cost_native_services.h"
#include "rust/services_map.h"
#include "rust/tile_animation_services.h"
#include "rust/orders_ffi.h"
#include "rust/newgrf_text_ffi.h"
const OpenTTDIndustryConstructionServices &GetIndustryConstructionServices();
const OpenTTDIndustryGenerationServices &GetIndustryGenerationServices();
static const OpenTTDIndustryCommandServices &GetIndustryCommandServices();
const OpenTTDIndustryCostServices &GetIndustryCostServices() noexcept;
const OpenTTDIndustryTilePolicyServices &GetIndustryTilePolicyServices() noexcept;
static const OpenTTDIndustryTransactionServices &GetIndustryTransactionServices();
static OpenTTDIndustryTransactionContext GetIndustryTransactionContext();
static Industry *IndustryIfValid(uint16_t id) noexcept { return Industry::GetIfValid(IndustryID(id)); }

#include "rust/industry_shell_services.h"
#include "rust/services_water.h"
#include "rust/settings_services.h"
#include "rust/services_landscape.h"
#include "rust/command_services.h"

#include "rust/services_newgrf.h"
#include "rust/industry_resolver_ffi.h"

#include "rust/industry_definition_ffi.h"
#include "newgrf/newgrf_stringmapping.h"
#include "table/build_industry.h"

#include "rust/clock_ffi.h"

#include "safeguards.h"

IndustryPool _industry_pool("Industry");
INSTANTIATE_POOL_METHODS(Industry)

void ShowIndustryViewWindow(IndustryID industry);
void BuildOilRig(TileIndex tile);
extern bool _ignore_industry_restrictions;

#	define _industry_sound_ctr _industry_builder.sound_ctr
#	define _industry_sound_tile reinterpret_cast<TileIndex &>(_industry_builder.sound_tile)

std::array<RustIndustryRegistry, NUM_INDUSTRYTYPES> Industry::industries = [] {
	std::array<RustIndustryRegistry, NUM_INDUSTRYTYPES> result{};
	for (uint8_t type = 0; type < NUM_INDUSTRYTYPES; type++) result[type].type = type;
	return result;
}();

IndustryBuildData _industry_builder; ///< In-game manager of industries.


/**
 * Retrieve the type for this industry.  Although it is accessed by a tile,
 * it will return the general type of industry, and not the sprite index
 * as would do GetIndustryGfx.
 * @param tile that is queried
 * @pre IsTileType(tile, MP_INDUSTRY)
 * @return general type for this industry, as defined in industry.h
 */
IndustryType GetIndustryType(Tile tile)
{
	assert(IsTileType(tile, MP_INDUSTRY));

	const Industry *ind = Industry::GetByTile(tile);
	assert(ind != nullptr);
	return ind->type;
}

/**
 * Accessor for array _industry_specs.
 * This will ensure at once : proper access and
 * not allowing modifications of it.
 * @param thistype of industry (which is the index in _industry_specs)
 * @pre thistype < NUM_INDUSTRYTYPES
 * @return a pointer to the corresponding industry spec
 */
const IndustrySpec *GetIndustrySpec(IndustryType thistype)
{
	assert(thistype < NUM_INDUSTRYTYPES);
	return &openttd_rust_industry_specs()[thistype];
}

/**
 * Accessor for array _industry_tile_specs.
 * This will ensure at once : proper access and
 * not allowing modifications of it.
 * @param gfx of industrytile (which is the index in _industry_tile_specs)
 * @pre gfx < INVALID_INDUSTRYTILE
 * @return a pointer to the corresponding industrytile spec
 */
const IndustryTileSpec *GetIndustryTileSpec(IndustryGfx gfx)
{
	assert(gfx < NUM_INDUSTRYTILES);
	return &openttd_rust_industry_tile_specs()[gfx];
}

Industry::~Industry()
{
	openttd_rust_industry_cleanup(this, this->production_owner.get(), RustIndustryBuilderState(), this->index.base(), CleaningPool(), {Map::SizeX(), Map::SizeY(), 0}, &GetIndustryRustServices(), &GetIndustryLifecycleRustServices());
}

/**
 * Invalidating some stuff after removing item from the pool.
 * @param index index of deleted item
 */
void Industry::PostDestructor(size_t)
{
	InvalidateWindowData(WC_INDUSTRY_DIRECTORY, 0, IDIWD_FORCE_REBUILD);
	SetWindowDirty(WC_BUILD_INDUSTRY, 0);
}


/**
 * Return a random valid industry.
 * @return random industry, nullptr if there are no industries
 */


static void IndustryDrawSugarMine(const TileInfo *ti)
{
	if (!IsIndustryCompleted(ti->tile)) return;

	const DrawIndustryAnimationStruct *d = &_draw_industry_spec1[GetAnimationFrame(ti->tile)];

	AddChildSpriteScreen(SPR_IT_SUGAR_MINE_SIEVE + d->image_1, PAL_NONE, d->x, 0);

	if (d->image_2 != 0) {
		AddChildSpriteScreen(SPR_IT_SUGAR_MINE_CLOUDS + d->image_2 - 1, PAL_NONE, 8, 41);
	}

	if (d->image_3 != 0) {
		AddChildSpriteScreen(SPR_IT_SUGAR_MINE_PILE + d->image_3 - 1, PAL_NONE,
			_drawtile_proc1[d->image_3 - 1].x, _drawtile_proc1[d->image_3 - 1].y);
	}
}

static void IndustryDrawToffeeQuarry(const TileInfo *ti)
{
	uint8_t x = 0;

	if (IsIndustryCompleted(ti->tile)) {
		x = _industry_anim_offs_toffee[GetAnimationFrame(ti->tile)];
		if (x == 0xFF) {
			x = 0;
		}
	}

	AddChildSpriteScreen(SPR_IT_TOFFEE_QUARRY_SHOVEL, PAL_NONE, 22 - x, 24 + x);
	AddChildSpriteScreen(SPR_IT_TOFFEE_QUARRY_TOFFEE, PAL_NONE, 6, 14);
}

static void IndustryDrawBubbleGenerator( const TileInfo *ti)
{
	if (IsIndustryCompleted(ti->tile)) {
		AddChildSpriteScreen(SPR_IT_BUBBLE_GENERATOR_BUBBLE, PAL_NONE, 5, _industry_anim_offs_bubbles[GetAnimationFrame(ti->tile)]);
	}
	AddChildSpriteScreen(SPR_IT_BUBBLE_GENERATOR_SPRING, PAL_NONE, 3, 67);
}

static void IndustryDrawToyFactory(const TileInfo *ti)
{
	const DrawIndustryAnimationStruct *d = &_industry_anim_offs_toys[GetAnimationFrame(ti->tile)];

	if (d->image_1 != 0xFF) {
		AddChildSpriteScreen(SPR_IT_TOY_FACTORY_CLAY, PAL_NONE, d->x, 96 + d->image_1);
	}

	if (d->image_2 != 0xFF) {
		AddChildSpriteScreen(SPR_IT_TOY_FACTORY_ROBOT, PAL_NONE, 16 - d->image_2 * 2, 100 + d->image_2);
	}

	AddChildSpriteScreen(SPR_IT_TOY_FACTORY_STAMP, PAL_NONE, 7, d->image_3);
	AddChildSpriteScreen(SPR_IT_TOY_FACTORY_STAMP_HOLDER, PAL_NONE, 0, 42);
}

static void IndustryDrawCoalPlantSparks(const TileInfo *ti)
{
	if (IsIndustryCompleted(ti->tile)) {
		uint8_t image = GetAnimationFrame(ti->tile);

		if (image != 0 && image < 7) {
			AddChildSpriteScreen(image + SPR_IT_POWER_PLANT_TRANSFORMERS,
				PAL_NONE,
				_coal_plant_sparks[image - 1].x,
				_coal_plant_sparks[image - 1].y
			);
		}
	}
}

typedef void IndustryDrawTileProc(const TileInfo *ti);
static IndustryDrawTileProc * const _industry_draw_tile_procs[5] = {
	IndustryDrawSugarMine,
	IndustryDrawToffeeQuarry,
	IndustryDrawBubbleGenerator,
	IndustryDrawToyFactory,
	IndustryDrawCoalPlantSparks,
};

static void DrawTile_Industry(TileInfo *ti)
{
	IndustryGfx gfx = GetIndustryGfx(ti->tile);
	Industry *ind = Industry::GetByTile(ti->tile);
	const IndustryTileSpec *indts = GetIndustryTileSpec(gfx);

	/* Retrieve pointer to the draw industry tile struct */
	if (gfx >= NEW_INDUSTRYTILEOFFSET) {
		/* Draw the tile using the specialized method of newgrf industrytile.
		 * DrawNewIndustry will return false if ever the resolver could not
		 * find any sprite to display.  So in this case, we will jump on the
		 * substitute gfx instead. */
		if (indts->grf_prop.HasSpriteGroups() && DrawNewIndustryTile(ti, ind, gfx, indts)) {
			return;
		} else {
			/* No sprite group (or no valid one) found, meaning no graphics associated.
			 * Use the substitute one instead */
			if (indts->grf_prop.subst_id != INVALID_INDUSTRYTILE) {
				gfx = indts->grf_prop.subst_id;
				/* And point the industrytile spec accordingly */
				indts = GetIndustryTileSpec(gfx);
			}
		}
	}

	const DrawBuildingsTileStruct *dits = &_industry_draw_tile_data[gfx << 2 | (indts->anim_state ?
			GetAnimationFrame(ti->tile) & INDUSTRY_COMPLETED :
			GetIndustryConstructionStage(ti->tile))];

	SpriteID image = dits->ground.sprite;

	/* DrawFoundation() modifies ti->z and ti->tileh */
	if (ti->tileh != SLOPE_FLAT) DrawFoundation(ti, FOUNDATION_LEVELED);

	/* If the ground sprite is the default flat water sprite, draw also canal/river borders.
	 * Do not do this if the tile's WaterClass is 'land'. */
	if (image == SPR_FLAT_WATER_TILE && IsTileOnWater(ti->tile)) {
		DrawWaterClassGround(ti);
	} else {
		DrawGroundSprite(image, GroundSpritePaletteTransform(image, dits->ground.pal, GetColourPalette(ind->random_colour)));
	}

	/* If industries are transparent and invisible, do not draw the upper part */
	if (IsInvisibilitySet(TO_INDUSTRIES)) return;

	/* Add industry on top of the ground? */
	image = dits->building.sprite;
	if (image != 0) {
		AddSortableSpriteToDraw(image, SpriteLayoutPaletteTransform(image, dits->building.pal, GetColourPalette(ind->random_colour)),
			*ti, *dits, IsTransparencySet(TO_INDUSTRIES));

		if (IsTransparencySet(TO_INDUSTRIES)) return;
	}

	{
		int proc = dits->draw_proc - 1;
		if (proc >= 0) _industry_draw_tile_procs[proc](ti);
	}
}


static void AddAcceptedCargo_Industry(TileIndex tile, CargoArray &acceptance, CargoTypes &always_accepted)
{
	openttd_rust_industry_tile_acceptance(tile.base(), acceptance.data(), &always_accepted, &GetIndustryTileServices());
}

static void GetTileDesc_Industry(TileIndex tile, TileDesc &td)
{
	const Industry *i = Industry::GetByTile(tile);
	const IndustrySpec *is = GetIndustrySpec(i->type);

	td.owner[0] = i->owner;
	td.str = is->name;
	if (!IsIndustryCompleted(tile)) {
		td.dparam = td.str;
		td.str = STR_LAI_TOWN_INDUSTRY_DESCRIPTION_UNDER_CONSTRUCTION;
	}

	if (is->grf_prop.HasGrfFile()) {
		td.grf = GetGRFConfig(is->grf_prop.grfid)->GetName();
	}
}


/**
 * Move produced cargo from industry to nearby stations.
 * @param tile Industry tile
 * @return true if any cargo was moved.
 */
static bool TransportIndustryGoods(TileIndex tile)
{
	Industry *i = Industry::GetByTile(tile);
	return openttd_rust_industry_transport(i, i->production_owner.get(), GetIndustrySpec(i->type)->minimal_cargo, EconomyIsInRecession(), &GetIndustryRustServices());
}


const OpenTTDIndustryLoopServices &GetRustIndustryLoopServices() noexcept
{
	static const OpenTTDIndustryLoopServices services{
		.production = &GetIndustryRustServices(),
		.construction = &GetIndustryConstructionServices(),
		.lifecycle = &GetIndustryLifecycleRustServices(),
		.tiles = &GetIndustryTileServices(),
		.generation = &GetIndustryGenerationServices(),
	};
	return services;
}


static bool ClickTile_Industry(TileIndex tile)
{
	ShowIndustryViewWindow(GetIndustryIndex(tile));
	return true;
}


/**
 * Check whether the tile is a forest.
 * @param tile the tile to investigate.
 * @return true if and only if the tile is a forest
 */
bool IsTileForestIndustry(TileIndex tile)
{
	return openttd_rust_industry_forest(tile.base(), &GetIndustryRustServices());
}


void PlantRandomFarmField(const Industry *i)
{
	openttd_rust_industry_farm(i->production_owner.get(), i->index.base(), &GetIndustryRustServices());
}


/**
 * Check the conditions of #CHECK_NOTHING (Always succeeds).
 * @return Succeeded or failed command.
 */

static_assert(Ticks::INDUSTRY_PRODUCE_TICKS == 256);

bool IsSlopeRefused(Slope current, Slope refused)
{
	return openttd_rust_industry_slope_refused(current, refused);
}

/**
 * Are the tiles of the industry free?
 * @param tile                    Position to check.
 * @param layout                  Industry tiles table.
 * @param type                    Type of the industry.
 * @return Failed or succeeded command.
 */



/**
 * Build/Fund an industry
 * @param flags of operations to conduct
 * @param tile tile where industry is built
 * @param it industry type see build_industry.h and see industry.h
 * @param first_layout first layout to try
 * @param fund false = prospect, true = fund (only valid if current company is DEITY)
 * @param seed seed to use for desyncfree randomisations
 * @return the cost of this operation or an error
 */
CommandCost CmdBuildIndustry(DoCommandFlags flags, TileIndex tile, IndustryType it, uint32_t first_layout, bool fund, uint32_t seed)
{
	CommandCost result;
	auto context = GetIndustryTransactionContext();
	openttd_rust_industry_build(&result, tile.base(), it, flags.base(), first_layout, fund, seed, &context);
	return result;
}

/**
 * Set industry control flags.
 * @param flags Type of operation.
 * @param ind_id IndustryID
 * @param ctlflags IndustryControlFlags
 * @return Empty cost or an error.
 */
CommandCost CmdIndustrySetFlags(DoCommandFlags flags, IndustryID ind_id, IndustryControlFlags ctlflags)
{
	return openttd_rust_industry_set_flags_command(ind_id.base(), flags.Test(DoCommandFlag::Execute), ctlflags.base(), &GetIndustryCommandServices()) ? CommandCost() : CMD_ERROR;
}

/**
 * Set industry production.
 * @param flags Type of operation.
 * @param ind_id IndustryID
 * @param prod_level Production level.
 * @param show_news Show a news message on production change.
 * @param custom_news Custom news message text.
 * @return Empty cost or an error.
 */
CommandCost CmdIndustrySetProduction(DoCommandFlags flags, IndustryID ind_id, uint8_t prod_level, bool show_news, const EncodedString &custom_news)
{
	(void)show_news;
	(void)custom_news; // The original post-assignment news test is always false.
	return openttd_rust_industry_set_production_command(ind_id.base(), flags.Test(DoCommandFlag::Execute), prod_level, &GetIndustryCommandServices(), &GetIndustryRustServices()) ? CommandCost() : CMD_ERROR;
}

/**
 * Change exclusive consumer or supplier for the industry.
 * @param flags Type of operation.
 * @param ind_id IndustryID
 * @param company_id CompanyID to set or INVALID_OWNER (available to everyone) or
 *                   OWNER_NONE (neutral stations only) or OWNER_DEITY (no one)
 * @param consumer Set exclusive consumer if true, supplier if false.
 * @return Empty cost or an error.
 */
CommandCost CmdIndustrySetExclusivity(DoCommandFlags flags, IndustryID ind_id, Owner company_id, bool consumer)
{
	return openttd_rust_industry_set_exclusivity_command(ind_id.base(), flags.Test(DoCommandFlag::Execute), company_id.base(), consumer, &GetIndustryCommandServices()) ? CommandCost() : CMD_ERROR;
}

/**
 * Change additional industry text.
 * @param flags Type of operation.
 * @param ind_id IndustryID
 * @param text - Additional industry text.
 * @return Empty cost or an error.
 */
CommandCost CmdIndustrySetText(DoCommandFlags flags, IndustryID ind_id, const EncodedString &text)
{
	return openttd_rust_industry_set_text_command(ind_id.base(), flags.Test(DoCommandFlag::Execute), Industry::TextView(text), &GetIndustryCommandServices()) ? CommandCost() : CMD_ERROR;
}

/**
 * Create a new industry of random layout.
 * @param tile The location to build the industry.
 * @param type The industry type to build.
 * @param creation_type The circumstances the industry is created under.
 * @return the created industry or nullptr if it failed.
 */
static Industry *CreateNewIndustry(TileIndex tile, IndustryType type, IndustryAvailabilityCallType creation_type)
{
	CommandCost result;
	Industry *industry = nullptr;
	auto context = GetIndustryTransactionContext();
	openttd_rust_industry_create(&result, &industry, tile.base(), type, creation_type, &context);
	assert(industry != nullptr || result.Failed());
	return industry;
}


/** Reset the entry. */
void IndustryTypeBuildData::Reset()
{
	openttd_rust_industry_build_type_reset(reinterpret_cast<OpenTTDIndustryBuildFields *>(this));
}

/** Completely reset the industry build data. */
void IndustryBuildData::Reset()
{
	openttd_rust_industry_build_reset(this->owner.get(), &GetIndustryRustServices());
}


/**
 * This function will create random industries during game creation.
 * It will scale the amount of industries by mapsize and difficulty level.
 */
void GenerateIndustries()
{
	OpenTTDIndustryGenerationContext context;
	if (!openttd_rust_industry_generate(RustIndustryBuilderState(), &context, &GetIndustryRustServices(), &GetIndustryGenerationServices())) std::rethrow_exception(context.cancellation);
}

template <>
Industry::ProducedHistory SumHistory(std::span<const Industry::ProducedHistory> history)
{
	const auto sum = openttd_rust_industry_sum_history(reinterpret_cast<const OpenTTDIndustryPair *>(history.data()), history.size());
	return {.production = sum.first, .transported = sum.second};
}

template <>
Industry::AcceptedHistory SumHistory(std::span<const Industry::AcceptedHistory> history)
{
	const auto sum = openttd_rust_industry_sum_history(reinterpret_cast<const OpenTTDIndustryPair *>(history.data()), history.size());
	return {.accepted = sum.first, .waiting = sum.second};
}

/**
 * Monthly update of industry statistics.
 * @param i Industry to update.
 */

void Industry::RecomputeProductionMultipliers()
{
	assert(UsesOriginalEconomy(GetIndustrySpec(this->type)));
	openttd_rust_industry_recompute(this->production_owner.get());
}

void Industry::FillCachedName() const
{
	auto tmp_params = MakeParameters(this->index);
	this->cached_name = GetStringWithArgs(STR_INDUSTRY_NAME, tmp_params);
}

void ClearAllIndustryCachedNames()
{
	for (Industry *ind : Industry::Iterate()) {
		ind->cached_name.clear();
	}
}

/**
 * Protects an industry from closure if the appropriate flags and conditions are met
 * CanCloseLastInstance must be set (which, by default, it is not) and the
 * count of industries of this type must one (or lower) in order to be protected
 * against closure.
 * @param type IndustryType been queried
 * @result true if protection is on, false otherwise (except for oil wells)
 */


/**
 * Report news that industry production has changed significantly
 *
 * @param ind Industry with changed production
 * @param cargo Cargo type that has changed
 * @param percent Percentage of change (>0 means increase, <0 means decrease)
 */


/**
 * Every economy day handler for the industry changes
 * Taking the original map size of 256*256, the number of random changes was always of just one unit.
 * But it cannot be the same on smaller or bigger maps. That number has to be scaled up or down.
 * For small maps, it implies that less than one change per month is required, while on bigger maps,
 * it would be way more. The daily loop handles those changes.
 */
static const OpenTTDClockIndustry clock_industry{RustIndustryBuilderState(), &GetIndustryRustServices()};
static const IntervalTimer<TimerGameEconomy> _economy_industries_daily({TimerGameEconomy::DAY, TimerGameEconomy::Priority::INDUSTRY}, RustClockCallback{openttd_rust_clock_industry_daily, &clock_industry});


static const IntervalTimer<TimerGameEconomy> _economy_industries_monthly({TimerGameEconomy::MONTH, TimerGameEconomy::Priority::INDUSTRY}, RustClockCallback{openttd_rust_clock_industry_monthly, &clock_industry});


void InitializeIndustries()
{
	for (auto &registry : Industry::industries) registry.clear();
	_industry_sound_tile = TileIndex{};

	_industry_builder.Reset();
}

/** Verify whether the generated industries are complete, and warn the user if not. */
void CheckIndustries()
{
	openttd_rust_industry_check(RustIndustryBuilderState(), &GetIndustryRustServices(), &GetIndustryGenerationServices());
}

/**
 * Is an industry with the spec a raw industry?
 * @return true if it should be handled as a raw industry
 */
bool IsRawIndustry(const IndustrySpec *spec)
{
	return openttd_rust_industry_spec_raw(spec);
}

/**
 * Is an industry with the spec a processing industry?
 * @return true if it should be handled as a processing industry
 */
bool IsProcessingIndustry(const IndustrySpec *spec)
{
	return openttd_rust_industry_spec_processing(spec);
}

/**
 * Get the cost for constructing this industry
 * @return the cost (inflation corrected etc)
 */
Money GetConstructionCost(const IndustrySpec *spec)
{
	return openttd_rust_industry_construction_cost(spec, &GetRustSharedServices());
}

/**
 * Determines whether this industrytype uses standard/newgrf production changes.
 * @return true if original economy is used.
 */
bool UsesOriginalEconomy(const IndustrySpec *spec)
{
	return openttd_rust_industry_original_economy(spec, &GetRustSharedServices());
}

const OpenTTDIndustryDefinitionServices &GetIndustryDefinitionServices() noexcept
{
	static const OpenTTDIndustryDefinitionServices services{
		.shared = &GetRustSharedServices(),
		.origin = _origin_industry_specs,
		.origin_tiles = _origin_industry_tile_specs,
		.invalid_name = STR_NEWGRF_INVALID_INDUSTRYTYPE,
		.map_string = [](uint16_t source, uint32_t *target) noexcept { AddStringForMapping(GRFStringID{source}, target); },
	};
	return services;
}


extern const TileTypeProcs _tile_type_industry_procs = {
	DrawTile_Industry,           // draw_tile_proc
	nullptr, // Rust movement owner dispatches directly.
	nullptr,                 // Rust landscape commands call the owner directly.
	AddAcceptedCargo_Industry,   // add_accepted_cargo_proc
	GetTileDesc_Industry,        // get_tile_desc_proc
	nullptr, // Rust movement owner dispatches directly.
	ClickTile_Industry,          // click_tile_proc
	nullptr,                 // Rust scheduler calls the owner directly.
	nullptr,                 // Rust ownership dispatcher calls the owner directly.
	nullptr,                        // add_produced_cargo_proc
	nullptr,                        // vehicle_enter_tile_proc
	nullptr, // Rust movement owner dispatches directly.
	nullptr,                 // Rust landscape commands call the owner directly.
	nullptr, // check_build_above_proc
};


/**
 * Remove unused industry accepted/produced slots -- entries after the last slot with valid cargo.
 * @param ind Industry to trim slots.
 */
void TrimIndustryAcceptedProduced(Industry *ind)
{
	openttd_rust_industry_trim(ind->production_owner.get());
}

/** Industry world services with immediate effects. Script event services only enqueue
	* events; construction, landscape clearing and industry destruction can reenter Rust.
	* Their CommandCost failures are handled by the native transactions. */

static Industry *RustIndustry(void *handle) noexcept { return static_cast<Industry *>(handle); }

/** One typed noexcept slot per original operation, in industry_ffi.h order. */
static const OpenTTDIndustryServices _industry_services = {
	.shared = &GetRustSharedServices(),
	.next_tick = [](uint32_t from) noexcept -> OpenTTDIndustryEntry {
		Industry *i = OpenTTDNextIndustry(from);
		if (i == nullptr) return {};
		return {i, RustIndustryCanonicalOwner(i), RustIndustryID(i)};
	},
	.industry_sound = [](void *handle, uint32_t index) noexcept {
		Industry *i = RustIndustry(handle);
		RustPlayTileSound(RustIndustryTile(handle), GetIndustrySpec(i->type)->random_sounds[index]);
	},
	.play_sound = RustPlayTileSoundBySound,
	.special_effect = [](void *handle, uint32_t random, uint32_t parameter) noexcept -> uint16_t {
		Industry *i = RustIndustry(handle);
		return GetIndustryCallback(CBID_INDUSTRY_SPECIAL_EFFECT, random, parameter, i, i->type, i->location.tile);
	},
	.tick_trigger = [](void *handle) noexcept {
		Industry *i = RustIndustry(handle);
		TriggerIndustryRandomisation(i, IndustryRandomTrigger::IndustryTick);
		TriggerIndustryAnimation(i, IndustryAnimationTrigger::IndustryTick);
	},
	.production_callback = [](void *i, uint8_t reason) noexcept { IndustryProductionCallback(RustIndustry(i), reason); },
	.scale_cargo = OpenTTDScaleIndustryCargo,
	.environment = &GetRustEnvironmentServices(),
	.farm_tile = [](uint32_t index) noexcept -> OpenTTDIndustryFarmTile {
		TileIndex tile(index);
		const auto type = SharedTileType(index);
		switch (type) {
			case MP_CLEAR: return {MP_CLEAR, RustSnowTile(index), static_cast<uint8_t>(RustClearGround(Tile{tile})), 0};
			case MP_TREES: return {MP_TREES, 0, static_cast<uint8_t>(RustTreeGround(Tile{tile})), GetTreeGrowth(tile) >= TreeGrowthStage::Grown};
			default: return {type, 0, 0, 0};
		}
	},
	.make_field = [](uint32_t index, uint8_t field_type, uint8_t counter, uint16_t industry) noexcept {
		TileIndex tile(index);
		MakeField(tile, field_type, IndustryID(industry));
		RustSetClearCounter(Tile{tile}, counter);
		MarkTileDirtyByTile(tile);
	},
	.fence_wanted = [](uint32_t index, uint8_t side) noexcept -> bool {
		TileIndex index_tile{index};
		Tile tile{index_tile};
		auto dir = static_cast<DiagDirection>(side);
		if (!RustClearFieldTile(tile)) return false;
		Tile neighbour{index_tile + TileOffsByDiagDir(dir)};
		return !RustClearFieldTile(neighbour) || RustFieldFence(neighbour, ReverseDiagDir(dir)) == 0;
	},
	.tile_completed = [](void *i, uint32_t tile) noexcept -> bool { return !RustIndustryTileBelongs(i, tile) || IsIndustryCompleted(TileIndex(tile)); },
	.harvest = [](uint32_t tile) noexcept {
		if (_settings_client.sound.ambient) RustPlayTileSound(tile, SND_38_LUMBER_MILL_1);
		AutoRestoreBackup<CompanyID> current(_current_company, OWNER_NONE);
		Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, TileIndex(tile));
	},
	.change_callback = [](void *handle, bool monthly, uint32_t random) noexcept -> OpenTTDIndustryChange {
		Industry *i = RustIndustry(handle);
		std::array<int32_t, 1> reg{};
		uint16_t result = GetIndustryCallback(monthly ? CBID_INDUSTRY_MONTHLYPROD_CHANGE : CBID_INDUSTRY_PRODUCTION_CHANGE, 0, random, i, i->type, i->location.tile, reg);
		return {result, reg[0]};
	},
	.text = &GetRustGrfTextServices(),
	.strings = {
		.format_industry_name = STR_FORMAT_INDUSTRY_NAME,
		.industry_production_decrease_smooth = STR_NEWS_INDUSTRY_PRODUCTION_DECREASE_SMOOTH,
		.industry_production_increase_smooth = STR_NEWS_INDUSTRY_PRODUCTION_INCREASE_SMOOTH,
		.last_stringid = STR_LAST_STRINGID,
		.town_name = STR_TOWN_NAME,
	},
	.callback_error = [](void *handle, bool invalid_cargo) noexcept {
		Industry *i = RustIndustry(handle);
		const IndustrySpec *spec = GetIndustrySpec(i->type);
		auto detail = !invalid_cargo ? GetEncodedString(STR_NEWGRF_BUGGY_ENDLESS_PRODUCTION_CALLBACK, std::monostate{}, spec->name)
			: GetEncodedString(STR_NEWGRF_BUGGY_INVALID_CARGO_PRODUCTION_CALLBACK, std::monostate{}, spec->name, i->location.tile);
		ShowErrorMessage(GetEncodedString(STR_NEWGRF_BUGGY, spec->grf_prop.grffile->filename), std::move(detail), WL_WARNING);
	},
	.set_dirty = [](void *i) noexcept { SetWindowDirty(WC_INDUSTRY_VIEW, RustIndustry(i)->index); },
	.destroy = [](void *i) noexcept { delete RustIndustry(i); },
	.create = [](uint8_t type, uint8_t creation) noexcept -> void * { return CreateNewIndustry(RandomTile(), type, static_cast<IndustryAvailabilityCallType>(creation)); },
	.num_items = []() noexcept -> uint32_t { return static_cast<uint32_t>(Industry::GetNumItems()); },
	.get = [](uint16_t id) noexcept -> void * { return IndustryIfValid(id); },
	.resolver = &GetIndustryResolverServices(),
	.company_none = []() noexcept -> uint8_t {
		CompanyID old = OpenTTDCurrentCompany();
		OpenTTDSetCurrentCompany(OWNER_NONE);
		return old.base();
	},
	.restore_company = [](uint8_t company) noexcept { OpenTTDSetCurrentCompany(CompanyID(company)); },
	.directory_dirty = [](uint8_t reason) noexcept { InvalidateWindowData(WC_INDUSTRY_DIRECTORY, 0, reason); },
};
const OpenTTDIndustryServices &GetIndustryRustServices() { return _industry_services; }

/** Native shells, packed map, resolver and shared station/cargo services only.
 * Destruction may reenter; Rust holds no owner or registry borrow across a slot. */
static const OpenTTDIndustryLifecycleServices _industry_lifecycle_services = {
	.neutral_station = [](void *shell) noexcept -> OpenTTDIndustryStationHandle * { return reinterpret_cast<OpenTTDIndustryStationHandle *>(RustIndustryNeutralStation(RustIndustry(shell))); },
	.cleanup_tile = [](uint32_t index) noexcept -> OpenTTDIndustryCleanupTile {
		TileIndex tile(index);
		auto type = SharedTileType(index);
		return {type == MP_INDUSTRY ? RustIndustryTileIndex(index) : uint16_t{0xFFFF}, static_cast<uint8_t>(type), type == MP_STATION && IsOilRig(tile)};
	},
	.delete_tile_inspect = OpenTTDDeleteIndustryTileInspect,
	.restoration = &GetRustWaterRestorationServices(),
	.delete_oilrig = [](uint32_t tile) noexcept { DeleteOilRig(TileIndex(tile)); },
	.clear_docking = [](uint32_t tile) noexcept { ClearDockingTilesCheckingNeighbours(TileIndex(tile)); },
	.industry_field = RustFieldOfIndustryValue,
	.disassociate_field = [](uint32_t tile) noexcept { SetIndustryIndexOfField(TileIndex(tile), IndustryID::Invalid()); },
	.release_disasters = [](uint16_t industry) noexcept { ReleaseDisastersTargetingIndustry(IndustryID(industry)); },
	.delete_storage = [](void *shell) noexcept { delete static_cast<Industry *>(shell)->psa; },
	.close_view = [](uint16_t industry) noexcept { CloseWindowById(WC_INDUSTRY_VIEW, industry); },
	.close_production = [](uint16_t industry) noexcept { CloseWindowById(WC_INDUSTRY_PRODUCTION, industry); },
	.delete_inspect = OpenTTDDeleteIndustryInspect,
	.subsidies = RustSubsidyServices,
	.invalidate_cargo = [](uint16_t industry) noexcept { CargoPacket::InvalidateAllFrom({IndustryID(industry), SourceType::Industry}); },
	.belongs = [](uint32_t tile, uint16_t industry) noexcept -> bool { return IsTileType(TileIndex(tile), MP_INDUSTRY) && RustIndustryTileIndex(tile) == industry; },
	.randomisation_spec = IndustryRandomisationSpec,
	.random_triggers = [](uint32_t tile) noexcept -> uint8_t { return GetIndustryRandomTriggers(TileIndex(tile)).base(); },
	.set_random_triggers = [](uint32_t tile, uint8_t bits) noexcept { SetIndustryRandomTriggers(TileIndex(tile), IndustryRandomTriggers(bits)); },
	.resolve_reseed = IndustryResolveReseed,
	.random_bits = [](uint32_t tile) noexcept -> uint8_t { return GetIndustryRandomBits(TileIndex(tile)); },
	.set_random_bits = [](uint32_t tile, uint8_t bits) noexcept { SetIndustryRandomBits(TileIndex(tile), bits); },
	.animation_spec = IndustryAnimationSpec,

};
const OpenTTDIndustryLifecycleServices &GetIndustryLifecycleRustServices() { return _industry_lifecycle_services; }

const OpenTTDIndustryGenerationServices &GetIndustryGenerationServices()
{
	static const OpenTTDIndustryGenerationServices services = {
		.environment = GetRustEnvironmentServices(),
		.scale_by_size_1d = OpenTTDMapScaleSize1D,
		.scale_by_land_proportion = OpenTTDMapScaleLandProportion,
		/* Only AbortGenerateWorldSignal is ordinary here. Retain the native
		 * exception on this stack frame and rethrow after the Rust entry returns. */
		.set_progress = [](OpenTTDIndustryGenerationContext *context, bool water, uint32_t total) noexcept -> bool {
			return RunGenerationProgress(context, [&] { RustSetGeneratingWorldProgress(water ? GWP_WATER_INDUSTRY : GWP_LAND_INDUSTRY, total); });
		},
		.increase_progress = [](OpenTTDIndustryGenerationContext *context, bool water) noexcept -> bool {
			return RunGenerationProgress(context, [&] { RustIncreaseGeneratingWorldProgress(water ? GWP_WATER_INDUSTRY : GWP_LAND_INDUSTRY); });
		},
		.missing_warning = [](const OpenTTDIndustrySpecHandle *handle) noexcept {
			const auto *spec = reinterpret_cast<const IndustrySpec *>(handle);
			ShowErrorMessage(GetEncodedString(STR_ERROR_NO_SUITABLE_PLACES_FOR_INDUSTRIES, spec->name),
				GetEncodedString(STR_ERROR_NO_SUITABLE_PLACES_FOR_INDUSTRIES_EXPLANATION), WL_WARNING);
		},
	};
	return services;
}

const OpenTTDIndustryConstructionServices &GetIndustryConstructionServices()
{
	static const OpenTTDIndustryConstructionServices services = {
		.set_town = [](void *shell, const OpenTTDTownHandle *town) noexcept { RustIndustry(shell)->town = const_cast<Town *>(reinterpret_cast<const Town *>(town)); },
		.callback = [](void *shell, uint16_t callback, uint32_t parameter, uint32_t random) noexcept -> uint16_t {
			Industry *i = RustIndustry(shell);
			return GetIndustryCallback(static_cast<CallbackID>(callback), parameter, random, i, i->type, INVALID_TILE);
		},
		.callback_error = RustSpecCallbackError<IndustrySpec, OpenTTDIndustrySpecHandle>,
		.water_class = [](uint32_t tile) noexcept -> uint8_t { return IsWaterTile(TileIndex(tile)) ? to_underlying(RustWaterClass(tile)) : to_underlying(WaterClass::Invalid); },
		.clear_tile = [](uint32_t tile, uint16_t flags) noexcept { Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlags{flags}, TileIndex(tile)); },
		.make_industry = [](uint32_t tile, uint16_t industry, uint16_t gfx, uint32_t random, uint8_t water) noexcept { MakeIndustry(TileIndex(tile), IndustryID(industry), gfx, random, static_cast<WaterClass>(water)); },
		.set_construction_counter = [](uint32_t tile, uint8_t counter) noexcept { SetIndustryConstructionCounter(TileIndex(tile), counter); },
		.set_construction_stage = [](uint32_t tile, uint8_t stage) noexcept { SetIndustryConstructionStage(TileIndex(tile), stage); },
		.translated_gfx = [](uint16_t gfx) noexcept -> uint16_t { return GetTranslatedIndustryTileID(gfx); },
		.build_window_dirty = []() noexcept { SetWindowDirty(WC_BUILD_INDUSTRY, 0); },
	};
	return services;
}

const OpenTTDIndustryTileServices &GetIndustryTileServices() noexcept
{
	static const OpenTTDIndustryTileServices services = {
		.shared = &GetRustSharedServices(),
		.gfx = [](uint32_t tile) noexcept -> uint16_t { return GetIndustryGfx(TileIndex(tile)); },
		.set_gfx = [](uint32_t tile, uint16_t gfx) noexcept { SetIndustryGfx(TileIndex(tile), gfx); },
		.frame = OpenTTDAnimationFrame,
		.animation_loop = [](uint32_t tile) noexcept -> uint8_t { return GetIndustryAnimationLoop(TileIndex(tile)); },
		.set_animation_loop = [](uint32_t tile, uint8_t value) noexcept { SetIndustryAnimationLoop(TileIndex(tile), value); },

		.construction_counter = [](uint32_t tile) noexcept -> uint8_t { return GetIndustryConstructionCounter(TileIndex(tile)); },
		.construction_stage = [](uint32_t tile) noexcept -> uint8_t { return GetIndustryConstructionStage(TileIndex(tile)); },
		.completed = [](uint32_t tile) noexcept -> bool { return IsIndustryCompleted(TileIndex(tile)); },
		.set_completed = [](uint32_t tile) noexcept { SetIndustryCompleted(TileIndex(tile)); },
		.reset_construction = RustResetIndustryConstruction,
		.industry = [](uint32_t tile) noexcept -> OpenTTDIndustryTileOwner { Industry *i = RustIndustryByTile(tile); return {i, RustIndustryCanonicalOwner(i)}; },
		.industry_index = RustIndustryTileIndex,
		.toffee_offset = [](uint8_t frame) noexcept -> uint8_t { return _industry_anim_offs_toffee[frame]; },
		.on_water = RustOnWater,
		.transport = [](uint32_t tile) noexcept -> bool { return TransportIndustryGoods(TileIndex(tile)); },
		.build_oilrig = [](uint32_t tile) noexcept { BuildOilRig(TileIndex(tile)); },
		.pixel_max_z = OpenTTDTileMaxPixelZ,
		.callback = OpenTTDIndustryTileCallback,
	};
	return services;
}

static const OpenTTDIndustryCommandServices &GetIndustryCommandServices()
{
	static const OpenTTDIndustryCommandServices services = {
		.shared = &GetRustSharedServices(),
		.get = IndustryIfValid,
		.owner = RustIndustryCanonicalOwner<Industry *>,
		.valid_company = [](uint8_t company) noexcept -> bool { return Company::IsValidID(CompanyID(company)); },
		.invalidate_view = [](uint16_t id) noexcept { InvalidateWindowData(WC_INDUSTRY_VIEW, IndustryID(id)); },
	};
	return services;
}

const OpenTTDIndustryCostServices &GetIndustryCostServices() noexcept
{
	static const OpenTTDIndustryCostServices services = {
		.zero = OpenTTDCommandCostZero,
		.error = OpenTTDCommandCostError,
		.expense_money = [](OpenTTDIndustryCost *out, uint8_t expense, int64_t money) noexcept { OpenTTDCommandCostExpenseMoney(out, static_cast<ExpensesType>(expense), Money(money)); },
		.success = OpenTTDCommandCostSuccess,
	};
	return services;
}
const OpenTTDIndustryTilePolicyServices &GetIndustryTilePolicyServices() noexcept
{
	static const OpenTTDIndustryTilePolicyServices services = {
		.object_in_way = [](OpenTTDIndustryCost *out, const OpenTTDIndustrySpecHandle *spec) noexcept { *out = CommandCostWithParam(STR_ERROR_GENERIC_OBJECT_IN_THE_WAY, reinterpret_cast<const IndustrySpec *>(spec)->name); },
		.destroy = [](Industry *industry) noexcept { delete industry; },
		.tile_max_z = [](uint32_t tile) noexcept -> int32_t { return OpenTTDTileMaxZ(TileIndex(tile)); },
		.command_checks = &GetRustCommandServices(),
	};
	return services;
}

static const OpenTTDIndustryTransactionServices &GetIndustryTransactionServices()
{
	static const OpenTTDIndustryTransactionServices services = {
		.town = OpenTTDIndustryTownHandle,
		.population = [](const OpenTTDTownHandle *town) noexcept -> uint32_t { return reinterpret_cast<const Town *>(town)->cache.population; },
		.town_position = OpenTTDTownTile,
		.industry = RustIndustryByID<uint16_t>,
		.edge_distance = [](uint32_t tile, uint8_t direction) noexcept -> uint32_t { return DistanceFromEdgeDir(TileIndex(tile), static_cast<DiagDirection>(direction)); },
		.tile_height = [](uint32_t tile) noexcept -> uint32_t { return TileHeight(TileIndex(tile)); },
		.set_company = [](uint8_t company) noexcept { OpenTTDSetCurrentCompany(CompanyID(company)); },
		.temporary_shell = [](OpenTTDIndustry *owner, void *context, OpenTTDIndustryTemporaryVisit visit) noexcept { Industry industry(owner); industry.index = IndustryID::Invalid(); visit(context, &industry); },
		.resolve_location = openttd_industry_resolve_location,
		.resolve_shape = openttd_industry_resolve_shape,
		.location_error = RustLocationCallbackError<IndustrySpec, OpenTTDIndustrySpecHandle, OpenTTDIndustryLocationResult, OpenTTDIndustryCost, STR_ERROR_SITE_UNSUITABLE>,
		.shape_error = RustLocationCallbackError<IndustryTileSpec, IndustryTileSpec, OpenTTDIndustryLocationResult, OpenTTDIndustryCost, STR_ERROR_SITE_UNSUITABLE>,
		.ignore_restrictions = []() noexcept -> bool { return _ignore_industry_restrictions; },
		.can_allocate = []() noexcept -> bool { return Industry::CanAllocateItem(); },
		.allocate = [](uint32_t tile) noexcept -> OpenTTDIndustryAllocation { Industry *i = new Industry(TileIndex(tile)); return {i, RustIndustryCanonicalOwner(i), RustIndustryID(i)}; },
		.local_company = []() noexcept -> bool { return IsLocalCompany(); },
		.prospect_no_place = []() noexcept { ShowErrorMessage(GetEncodedString(STR_ERROR_CAN_T_PROSPECT_INDUSTRY), GetEncodedString(STR_ERROR_NO_SUITABLE_PLACES_FOR_PROSPECTING), WL_INFO); },
		.prospect_unlucky = []() noexcept { ShowErrorMessage(GetEncodedString(STR_ERROR_CAN_T_PROSPECT_INDUSTRY), GetEncodedString(STR_ERROR_PROSPECTING_WAS_UNLUCKY), WL_INFO); },
	};
	return services;
}
static OpenTTDIndustryTransactionContext GetIndustryTransactionContext()
{
	static const OpenTTDIndustryConstructionErrors errors = {
		.site = STR_ERROR_SITE_UNSUITABLE,
		.forest = STR_ERROR_FOREST_CAN_ONLY_BE_PLANTED,
		.positioned = STR_ERROR_CAN_ONLY_BE_POSITIONED,
		.desert = STR_ERROR_CAN_ONLY_BE_BUILT_IN_DESERT,
		.rainforest = STR_ERROR_CAN_ONLY_BE_BUILT_IN_RAINFOREST,
		.lowland = STR_ERROR_CAN_ONLY_BE_BUILT_IN_LOW_AREAS,
		.one_per_town = STR_ERROR_ONLY_ONE_ALLOWED_PER_TOWN,
		.town = STR_ERROR_CAN_ONLY_BE_BUILT_IN_TOWNS,
		.population = STR_ERROR_CAN_ONLY_BE_BUILT_IN_TOWNS_WITH_POPULATION_OF_1200,
		.near_town = STR_ERROR_CAN_ONLY_BE_BUILT_NEAR_TOWN_CENTER,
		.conflict = STR_ERROR_INDUSTRY_TOO_CLOSE,
		.too_many = STR_ERROR_TOO_MANY_INDUSTRIES,
	};
	return {
		.map = {Map::SizeX(), Map::SizeY(), to_underlying(_settings_game.game_creation.landscape)},
		.builder = RustIndustryBuilderState(),
		.production = &GetIndustryRustServices(),
		.generation = &GetIndustryGenerationServices(),
		.construction = &GetIndustryConstructionServices(),
		.lifecycle = &GetIndustryLifecycleRustServices(),
		.tiles = &GetIndustryTileServices(),
		.commands = &GetIndustryCommandServices(),
		.command_checks = &GetRustCommandServices(),
		.costs = &GetIndustryCostServices(),
		.transaction = &GetIndustryTransactionServices(),
		.errors = &errors,
	};
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDIndustryResolverServices.hpp"
#include "abi-visit-OpenTTDIndustryDefinitionServices.hpp"
#include "abi-visit-OpenTTDIndustryServices.hpp"
#include "abi-visit-OpenTTDIndustryDeliveryServices.hpp"
#include "abi-visit-OpenTTDIndustryLifecycleServices.hpp"
#include "abi-visit-OpenTTDIndustryGenerationServices.hpp"
#include "abi-visit-OpenTTDIndustryConstructionServices.hpp"
#include "abi-visit-OpenTTDIndustryTileServices.hpp"
#include "abi-visit-OpenTTDIndustryCommandServices.hpp"
#include "abi-visit-OpenTTDIndustryCostServices.hpp"
#include "abi-visit-OpenTTDIndustryTilePolicyServices.hpp"
#include "abi-visit-OpenTTDIndustryTransactionServices.hpp"
#include "abi-visit-OpenTTDEnvironmentServices.hpp"
bool CheckAbi_industry_cmd()
{
	return AbiServicesComplete(GetIndustryDeliveryServices()) &&
		AbiServicesComplete(GetIndustryDefinitionServices()) &&
		AbiServicesComplete(GetIndustryGenerationServices().environment) &&
		AbiServicesComplete(GetIndustryResolverServices()) &&
		AbiServicesComplete(GetIndustryRustServices()) &&
		AbiServicesComplete(GetIndustryLifecycleRustServices()) &&
		AbiServicesComplete(GetIndustryGenerationServices()) &&
		AbiServicesComplete(GetIndustryConstructionServices()) &&
		AbiServicesComplete(GetIndustryTileServices()) &&
		AbiServicesComplete(GetIndustryCommandServices()) &&
		AbiServicesComplete(GetIndustryCostServices()) &&
		AbiServicesComplete(GetIndustryTilePolicyServices()) &&
		AbiServicesComplete(GetIndustryTransactionServices());
}
