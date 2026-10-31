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
#include "rust/services_sound.h"
#include "rust/tile_animation_services.h"
#include "rust/orders_ffi.h"
const OpenTTDIndustryConstructionServices &GetIndustryConstructionServices();
const OpenTTDIndustryGenerationServices &GetIndustryGenerationServices();
static const OpenTTDIndustryCommandServices &GetIndustryCommandServices();
const OpenTTDIndustryCostServices &GetIndustryCostServices() noexcept;
const OpenTTDIndustryTilePolicyServices &GetIndustryTilePolicyServices() noexcept;
static OpenTTDIndustryTransactionContext GetIndustryTransactionContext();
static Industry *IndustryIfValid(uint16_t id) noexcept { return Industry::GetIfValid(IndustryID(id)); }

static bool IndustryCommandDeity() noexcept { return OpenTTDCurrentCompany() == OWNER_DEITY; }

#include "rust/industry_shell_services.h"
#include "rust/services_water.h"
#include "rust/settings_services.h"
#include "rust/services_landscape.h"
#include "rust/command_services.h"

#include "rust/services_newgrf.h"
#include "rust/industry_resolver_ffi.h"

#include "rust/industry_definition_ffi.h"
#include "rust/industry_property_ffi.h"

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

IndustrySpec _industry_specs[NUM_INDUSTRYTYPES];
IndustryTileSpec _industry_tile_specs[NUM_INDUSTRYTILES];
IndustryBuildData _industry_builder; ///< In-game manager of industries.

static int WhoCanServiceIndustry(Industry *ind);

/**
 * This function initialize the spec arrays of both
 * industry and industry tiles.
 * It adjusts the enabling of the industry too, based on climate availability.
 * This will allow for clearer testings
 */

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
	return &_industry_specs[thistype];
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
	return &_industry_tile_specs[gfx];
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
/* static */ Industry *Industry::GetRandom()
{
	if (Industry::GetNumItems() == 0) return nullptr;
	int num = RandomRange((uint16_t)Industry::GetNumItems());
	size_t index = std::numeric_limits<size_t>::max();

	while (num >= 0) {
		num--;
		index++;

		/* Make sure we have a valid industry */
		while (!Industry::IsValidID(index)) {
			index++;
			assert(index < Industry::GetPoolSize());
		}
	}

	return Industry::Get(index);
}


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




OpenTTDIndustryLoopServices GetRustIndustryLoopServices() noexcept
{
	return {
		.production = &GetIndustryRustServices(),
		.construction = &GetIndustryConstructionServices(),
		.lifecycle = &GetIndustryLifecycleRustServices(),
		.tiles = &GetIndustryTileServices(),
		.generation = &GetIndustryGenerationServices(),
	};
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
	/* Check for industry tile */
	if (!IsTileType(tile, MP_INDUSTRY)) return false;

	const Industry *ind = Industry::GetByTile(tile);

	/* Check for organic industry (i.e. not processing or extractive) */
	if (!GetIndustrySpec(ind->type)->life_type.Test(IndustryLifeType::Organic)) return false;

	/* Check for wood production */
	return std::any_of(std::begin(ind->produced), std::end(ind->produced), [](const auto &p) { return IsValidCargoType(p.cargo) && CargoSpec::Get(p.cargo)->label == CT_WOOD; });
}


void PlantRandomFarmField(const Industry *i)
{
	openttd_rust_industry_farm(const_cast<Industry *>(i), i->index.base(), {Map::SizeX(), Map::SizeY(), to_underlying(_settings_game.game_creation.landscape)}, &GetIndustryRustServices());
}

/**
 * Perform a circular search around the Lumber Mill in order to find trees to cut
 * @param i industry
 */

OpenTTDIndustryTickRecord RustIndustryTickRecord() noexcept
{
	/* Values the original reads inline; none changes during the tick. */
	return OpenTTDIndustryTickRecord{TimerGameTick::counter, {Map::SizeX(), Map::SizeY(), to_underlying(_settings_game.game_creation.landscape)},
		ScaleByInverseCargoScale(Ticks::INDUSTRY_PRODUCE_TICKS, false), _settings_client.sound.ambient, _game_mode == GM_EDITOR};
}


/**
 * Check the conditions of #CHECK_NOTHING (Always succeeds).
 * @return Succeeded or failed command.
 */

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

static void AdvertiseIndustryOpening(const Industry *ind)
{
	const IndustrySpec *ind_spc = GetIndustrySpec(ind->type);
	EncodedString headline;
	if (ind_spc->new_industry_text > STR_LAST_STRINGID) {
		headline = GetEncodedString(ind_spc->new_industry_text, ind_spc->name, STR_TOWN_NAME, ind->town->index);
	} else {
		headline = GetEncodedString(ind_spc->new_industry_text, ind_spc->name, ind->town->index);
	}
	AddIndustryNewsItem(std::move(headline), NewsType::IndustryOpen, ind->index);
	AI::BroadcastNewEvent(new ScriptEventIndustryOpen(ind->index));
	Game::NewEvent(new ScriptEventIndustryOpen(ind->index));
}

/**
 * Populate an industry's list of nearby stations, and if it accepts any cargo, also
 * add the industry to each station's nearby industry list.
 * @param ind Industry
 */

/**
 * Put an industry on the map.
 * @param i                   Just allocated poolitem, mostly empty.
 * @param tile                North tile of the industry.
 * @param type                Type of the industry.
 * @param layout              Industrylayout to build.
 * @param layout_index        Number of the industry layout.
 * @param t                   Nearest town.
 * @param founder             Founder of the industry; OWNER_NONE in case of random construction.
 * @param initial_random_bits Random bits for the industry.
 */

/**
 * Helper function for Build/Fund an industry
 * @param tile tile where industry is built
 * @param type of industry to build
 * @param flags of operations to conduct
 * @param indspec pointer to industry specifications
 * @param layout_index the index of the itsepc to build/fund
 * @param random_var8f random seed (possibly) used by industries
 * @param random_initial_bits The random bits the industry is going to have after construction.
 * @param founder Founder of the industry
 * @param creation_type The circumstances the industry is created under.
 * @param[out] ip Pointer to store newly created industry.
 * @return Succeeded or failed command.
 *
 * @post \c *ip contains the newly created industry if all checks are successful and the \a flags request actual creation, else it contains \c nullptr afterwards.
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

/**
 * Compute the appearance probability for an industry during map creation.
 * @param it Industry type to compute.
 * @param water Whether to get probability of land-based, water-based, (or both, if std::nullopt), industry types.
 * @param[out] force_at_least_one Returns whether at least one instance should be forced on map creation.
 * @return Relative probability for the industry to appear.
 */

/**
 * Compute the probability for constructing a new industry during game play.
 * @param it Industry type to compute.
 * @param[out] min_number Minimal number of industries that should exist at the map.
 * @return Relative probability for the industry to appear.
 */


/**
 * Try to place the industry in the game.
 * Since there is no feedback why placement fails, there is no other option
 * than to try a few times before concluding it does not work.
 * @param type     Industry type of the desired industry.
 * @param try_hard Try very hard to find a place. (Used to place at least one industry per type.)
 * @return Pointer to created industry, or \c nullptr if creation failed.
 */

/**
 * Try to build a industry on the map.
 * @param type IndustryType of the desired industry
 * @param try_hard Try very hard to find a place. (Used to place at least one industry per type)
 */

/**
 * Get total number of industries existing in the game.
 * @return Number of industries currently in the game.
 */
static uint GetCurrentTotalNumberOfIndustries()
{
	uint total = 0;
	for (const auto &industries : Industry::industries) {
		total += static_cast<uint16_t>(std::size(industries));
	}
	return total;
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

/** Monthly update of industry build data. */
void IndustryBuildData::EconomyMonthlyLoop()
{
	openttd_rust_industry_build_monthly(this->owner.get(), &GetIndustryRustServices());
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
	uint32_t production = std::accumulate(std::begin(history), std::end(history), 0, [](uint32_t r, const auto &p) { return r + p.production; });
	uint32_t transported = std::accumulate(std::begin(history), std::end(history), 0, [](uint32_t r, const auto &p) { return r + p.transported; });
	auto count = std::size(history);
	return {.production = ClampTo<uint16_t>(production / count), .transported = ClampTo<uint16_t>(transported / count)};
}

template <>
Industry::AcceptedHistory SumHistory(std::span<const Industry::AcceptedHistory> history)
{
	uint32_t accepted = std::accumulate(std::begin(history), std::end(history), 0, [](uint32_t r, const auto &a) { return r + a.accepted; });
	uint32_t waiting = std::accumulate(std::begin(history), std::end(history), 0, [](uint32_t r, const auto &a) { return r + a.waiting; });;
	auto count = std::size(history);
	return {.accepted = ClampTo<uint16_t>(accepted / count), .waiting = ClampTo<uint16_t>(waiting / count)};
}

/**
 * Monthly update of industry statistics.
 * @param i Industry to update.
 */

void Industry::RecomputeProductionMultipliers()
{
	assert(GetIndustrySpec(this->type)->UsesOriginalEconomy());
	openttd_rust_industry_recompute(this, this->production_owner.get(), &GetIndustryRustServices());
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
 * Set the #probability and #min_number fields for the industry type \a it for a running game.
 * @param it Industry type.
 * @return At least one of the fields has changed value.
 */
bool IndustryTypeBuildData::GetIndustryTypeData(IndustryType it)
{
	return openttd_rust_industry_build_type(reinterpret_cast<OpenTTDIndustryBuildFields *>(this), it, &GetIndustryRustServices()) != 0;
}

/** Decide how many industries of each type are needed. */
void IndustryBuildData::SetupTargetCount()
{
	openttd_rust_industry_build_targets(this->owner.get(), &GetIndustryRustServices());
}

/**
 * Try to create a random industry, during gameplay
 */
void IndustryBuildData::TryBuildNewIndustry()
{
	openttd_rust_industry_build_try(this->owner.get(), &GetIndustryRustServices());
}

/**
 * Protects an industry from closure if the appropriate flags and conditions are met
 * CanCloseLastInstance must be set (which, by default, it is not) and the
 * count of industries of this type must one (or lower) in order to be protected
 * against closure.
 * @param type IndustryType been queried
 * @result true if protection is on, false otherwise (except for oil wells)
 */

static void CanCargoServiceIndustry(CargoType cargo, Industry *ind, bool *c_accepts, bool *c_produces)
{
	if (!IsValidCargoType(cargo)) return;

	/* Check for acceptance of cargo */
	if (ind->IsCargoAccepted(cargo) && !IndustryTemporarilyRefusesCargo(ind, cargo)) *c_accepts = true;

	/* Check for produced cargo */
	if (ind->IsCargoProduced(cargo)) *c_produces = true;
}

/**
 * Compute who can service the industry.
 *
 * Here, 'can service' means that they have trains and stations close enough
 * to the industry with the right cargo type and the right orders (ie has the
 * technical means).
 *
 * @param ind: Industry being investigated.
 *
 * @return: 0 if nobody can service the industry, 2 if the local company can
 * service the industry, and 1 otherwise (only competitors can service the
 * industry)
 */
int WhoCanServiceIndustry(Industry *ind)
{
	if (ind->stations_near.empty()) return 0; // No stations found at all => nobody services

	int result = 0;
	for (const Vehicle *v : Vehicle::Iterate()) {
		/* Is it worthwhile to try this vehicle? */
		if (v->owner != _local_company && result != 0) continue;

		/* Check whether it accepts the right kind of cargo */
		bool c_accepts = false;
		bool c_produces = false;
		if (v->type == VEH_TRAIN && v->IsFrontEngine()) {
			for (const Vehicle *u = v; u != nullptr; u = u->Next()) {
				CanCargoServiceIndustry(u->cargo_type, ind, &c_accepts, &c_produces);
			}
		} else if (v->type == VEH_ROAD || v->type == VEH_SHIP || v->type == VEH_AIRCRAFT) {
			CanCargoServiceIndustry(v->cargo_type, ind, &c_accepts, &c_produces);
		} else {
			continue;
		}
		if (!c_accepts && !c_produces) continue; // Wrong cargo

		/* Check orders of the vehicle.
		 * We cannot check the first of shared orders only, since the first vehicle in such a chain
		 * may have a different cargo type.
		 */
		for (const Order &o : v->Orders()) {
			if (o.IsType(OT_GOTO_STATION) && o.GetUnloadType() != OrderUnloadType::Transfer) {
				/* Vehicle visits a station to load or unload */
				Station *st = Station::Get(o.GetDestination().ToStationID());
				assert(st != nullptr);

				/* Same cargo produced by industry is dropped here => not serviced by vehicle v */
				if (o.GetUnloadType() == OrderUnloadType::Unload && !c_accepts) break;

				if (ind->stations_near.find(st) != ind->stations_near.end()) {
					if (v->owner == _local_company) return 2; // Company services industry
					result = 1; // Competitor services industry
				}
			}
		}
	}
	return result;
}

/**
 * Report news that industry production has changed significantly
 *
 * @param ind Industry with changed production
 * @param cargo Cargo type that has changed
 * @param percent Percentage of change (>0 means increase, <0 means decrease)
 */
static void ReportNewsProductionChangeIndustry(Industry *ind, CargoType cargo, int percent)
{
	NewsType nt;

	switch (WhoCanServiceIndustry(ind)) {
		case 0: nt = NewsType::IndustryNobody;  break;
		case 1: nt = NewsType::IndustryOther;   break;
		case 2: nt = NewsType::IndustryCompany; break;
		default: NOT_REACHED();
	}
	AddIndustryNewsItem(
		GetEncodedString(percent >= 0 ? STR_NEWS_INDUSTRY_PRODUCTION_INCREASE_SMOOTH : STR_NEWS_INDUSTRY_PRODUCTION_DECREASE_SMOOTH,
			CargoSpec::Get(cargo)->name, ind->index, abs(percent)
		),
		nt,
		ind->index
	);
}


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
	openttd_rust_industry_check(&GetIndustryRustServices(), &GetIndustryGenerationServices());
}

/**
 * Is an industry with the spec a raw industry?
 * @return true if it should be handled as a raw industry
 */
bool IndustrySpec::IsRawIndustry() const
{
	return this->life_type.Any({IndustryLifeType::Extractive, IndustryLifeType::Organic});
}

/**
 * Is an industry with the spec a processing industry?
 * @return true if it should be handled as a processing industry
 */
bool IndustrySpec::IsProcessingIndustry() const
{
	/* Lumber mills are neither raw nor processing */
	return this->life_type.Test(IndustryLifeType::Processing) &&
			!this->behaviour.Test(IndustryBehaviour::CutTrees);
}

/**
 * Get the cost for constructing this industry
 * @return the cost (inflation corrected etc)
 */
Money IndustrySpec::GetConstructionCost() const
{
	/* Building raw industries like secondary uses different price base */
	return (_price[(_settings_game.construction.raw_industry_construction == 1 && this->IsRawIndustry()) ?
			PR_BUILD_INDUSTRY_RAW : PR_BUILD_INDUSTRY] * this->cost_multiplier) >> 8;
}

/**
 * Get the cost for removing this industry
 * Take note that the cost will always be zero for non-grf industries.
 * Only if the grf author did specified a cost will it be applicable.
 * @return the cost (inflation corrected etc)
 */
Money IndustrySpec::GetRemovalCost() const
{
	return (_price[PR_CLEAR_INDUSTRY] * this->removal_cost_multiplier) >> 8;
}

/**
 * Determines whether this industrytype uses standard/newgrf production changes.
 * @return true if original economy is used.
 */
bool IndustrySpec::UsesOriginalEconomy() const
{
	return _settings_game.economy.type == ET_ORIGINAL ||
		this->callback_mask.Any({
			IndustryCallbackMask::Production256Ticks,
			IndustryCallbackMask::ProductionCargoArrival, // production callbacks
			IndustryCallbackMask::MonthlyProdChange,
			IndustryCallbackMask::ProductionChange,
			IndustryCallbackMask::ProdChangeBuild}); // production change callbacks
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
static void RustIndustryNews(Industry *i, StringID str, bool close) noexcept
{
	NewsType nt;
	/* Compute news category */
	if (close) {
		nt = NewsType::IndustryClose;
		AI::BroadcastNewEvent(new ScriptEventIndustryClose(i->index));
		Game::NewEvent(new ScriptEventIndustryClose(i->index));
	} else {
		switch (WhoCanServiceIndustry(i)) {
			case 0: nt = NewsType::IndustryNobody;  break;
			case 1: nt = NewsType::IndustryOther;   break;
			case 2: nt = NewsType::IndustryCompany; break;
			default: NOT_REACHED();
		}
	}
	/* Set parameters of news string */
	EncodedString headline;
	if (str > STR_LAST_STRINGID) {
		headline = GetEncodedString(str, STR_TOWN_NAME, OpenTTDIndustryTown(i)->index, GetIndustrySpec(i->type)->name);
	} else if (close) {
		headline = GetEncodedString(str, STR_FORMAT_INDUSTRY_NAME, OpenTTDIndustryTown(i)->index, GetIndustrySpec(i->type)->name);
	} else {
		headline = GetEncodedString(str, i->index);
	}
	/* and report the news to the user */
	if (close) {
		AddTileNewsItem(std::move(headline), nt, i->location.tile + TileDiffXY(1, 1));
	} else {
		AddIndustryNewsItem(std::move(headline), nt, i->index);
	}
}

static Industry *RustIndustry(void *handle) noexcept { return static_cast<Industry *>(handle); }
static const IndustrySpec *RustIndustrySpec(void *handle) noexcept { return RustIndustryTypeSpec(RustIndustry(handle)->type); }

/** One typed noexcept slot per original operation, in industry_ffi.h order. */
static const OpenTTDIndustryServices _industry_services = {
	.shared = &GetRustSharedServices(),
	.next_tick = [](uint32_t from) noexcept -> OpenTTDIndustryEntry {
		Industry *i = OpenTTDNextIndustry(from);
		if (i == nullptr) return {};
		return {i, RustIndustryCanonicalOwner(i), RustIndustryID(i), RustIndustrySpecCallbackMask(GetIndustrySpec(i->type))};
	},
	.sound_count = [](void *i) noexcept -> uint16_t { return static_cast<uint16_t>(RustIndustrySpec(i)->random_sounds.size()); },
	.behaviour = RustIndustryBehaviour,
	.location = [](void *handle) noexcept -> OpenTTDIndustryLocation {
		const Industry *i = RustIndustry(handle);
		return {RustIndustryTile(handle), i->location.w, i->location.h};
	},
	.industry_sound = [](void *handle, uint32_t index) noexcept {
		Industry *i = RustIndustry(handle);
		RustPlayTileSound(RustIndustryTile(handle), GetIndustrySpec(i->type)->random_sounds[index]);
	},
	.play_sound = RustPlayTileSoundBySound,
	.special_effect = [](void *handle, uint32_t random, uint32_t parameter) noexcept -> uint16_t {
		Industry *i = RustIndustry(handle);
		uint16_t result = GetIndustryCallback(CBID_INDUSTRY_SPECIAL_EFFECT, random, parameter, i, i->type, i->location.tile);
		return result == CALLBACK_FAILED ? CALLBACK_FAILED : OpenTTDConvertBooleanCallback(GetIndustrySpec(i->type)->grf_prop.grffile, CBID_INDUSTRY_SPECIAL_EFFECT, result);
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
	.snow_line = GetRustSharedServices().snow_line,
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
	.observe = [](void *handle, OpenTTDIndustryObservation *out) noexcept {
		Industry *i = RustIndustry(handle);
		const IndustrySpec *spec = GetIndustrySpec(i->type);
		*out = {RustIndustryCanonicalOwner(i), RustIndustrySpecBehaviour(spec), RustIndustrySpecCallbackMask(spec), RustIndustrySpecLife(spec), uint8_t(spec->UsesOriginalEconomy()), i->type,
			spec->production_up_text, spec->production_down_text, spec->closure_text};
	},
	.production_rate = [](void *i, uint32_t slot) noexcept -> uint8_t { return RustIndustrySpec(i)->production_rate[slot]; },
	.change_callback = [](void *handle, bool monthly, uint32_t random) noexcept -> OpenTTDIndustryChange {
		Industry *i = RustIndustry(handle);
		std::array<int32_t, 1> reg{};
		uint16_t result = GetIndustryCallback(monthly ? CBID_INDUSTRY_MONTHLYPROD_CHANGE : CBID_INDUSTRY_PRODUCTION_CHANGE, 0, random, i, i->type, i->location.tile, reg);
		return {result, reg[0]};
	},
	.custom_text = [](void *i, uint16_t text) noexcept -> uint32_t { return MapGRFStringID(RustIndustrySpec(i)->grf_prop.grfid, GRFStringID(text)); },
	.news = [](void *i, uint32_t text, bool close) noexcept { RustIndustryNews(RustIndustry(i), text, close); },
	.rate_news = [](void *i, uint8_t cargo, int32_t percent) noexcept { ReportNewsProductionChangeIndustry(RustIndustry(i), cargo, percent); },
	.callback_error = [](void *handle, bool invalid_cargo) noexcept {
		Industry *i = RustIndustry(handle);
		const IndustrySpec *spec = GetIndustrySpec(i->type);
		auto detail = !invalid_cargo ? GetEncodedString(STR_NEWGRF_BUGGY_ENDLESS_PRODUCTION_CALLBACK, std::monostate{}, spec->name)
			: GetEncodedString(STR_NEWGRF_BUGGY_INVALID_CARGO_PRODUCTION_CALLBACK, std::monostate{}, spec->name, i->location.tile);
		ShowErrorMessage(GetEncodedString(STR_NEWGRF_BUGGY, spec->grf_prop.grffile->filename), std::move(detail), WL_WARNING);
	},
	.set_dirty = [](void *i) noexcept { SetWindowDirty(WC_INDUSTRY_VIEW, RustIndustry(i)->index); },
	.destroy = [](void *i) noexcept { delete RustIndustry(i); },
	.advertise = [](void *i) noexcept { AdvertiseIndustryOpening(RustIndustry(i)); },
	.create = [](uint8_t type, uint8_t creation) noexcept -> void * { return CreateNewIndustry(RandomTile(), type, static_cast<IndustryAvailabilityCallType>(creation)); },
	.random_industry = []() noexcept -> void * { return Industry::GetRandom(); },
	.get = [](uint16_t id) noexcept -> void * { return IndustryIfValid(id); },
	.type_count = [](uint8_t type) noexcept -> uint16_t { return Industry::GetIndustryTypeCount(type); },
	.total = []() noexcept -> uint32_t { return GetCurrentTotalNumberOfIndustries(); },
	.type_info = [](uint8_t type) noexcept -> OpenTTDIndustryTypeInfo {
		const IndustrySpec *spec = GetIndustrySpec(type);
		return {RustIndustrySpecBehaviour(spec), spec->enabled, !spec->layouts.empty(), spec->appear_ingame[to_underlying(_settings_game.game_creation.landscape)]};
	},
	.resolver = &GetIndustryResolverServices(),
	.scale_by_map_size = OpenTTDMapScaleSize,
	.company_none = []() noexcept -> uint8_t {
		CompanyID old = OpenTTDCurrentCompany();
		OpenTTDSetCurrentCompany(OWNER_NONE);
		return old.base();
	},
	.restore_company = [](uint8_t company) noexcept { OpenTTDSetCurrentCompany(CompanyID(company)); },
	.directory_dirty = [](uint8_t reason) noexcept { InvalidateWindowData(WC_INDUSTRY_DIRECTORY, 0, reason); },
	.economy_month = []() noexcept -> uint8_t { return TimerGameEconomy::month; },
	.economy_year = []() noexcept -> int32_t { return TimerGameEconomy::year.base(); },
	.days_since_last_month = []() noexcept -> uint32_t { return TimerGameEconomy::days_since_last_month; },
	.landscape = OpenTTDLandscapeValue,
	.economy_type = []() noexcept -> uint8_t { return _settings_game.economy.type; },

	.fund_only = []() noexcept -> bool { return _settings_game.difficulty.industry_density == ID_FUND_ONLY; },
	.deity = IndustryCommandDeity,
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
	.delete_news = [](uint16_t industry) noexcept { DeleteIndustryNews(IndustryID(industry)); },
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
		.spec = RustIndustryTypeSpec<OpenTTDIndustrySpecHandle>,
		.spec_behaviour = RustIndustrySpecHandleBehaviour,
		.info = [](const OpenTTDIndustrySpecHandle *handle) noexcept -> OpenTTDIndustryGenerationInfo {
			const auto *spec = reinterpret_cast<const IndustrySpec *>(handle);
			return {spec->appear_creation[to_underlying(_settings_game.game_creation.landscape)], spec->enabled, !spec->layouts.empty()};
		},
		.coastal = [](const OpenTTDIndustrySpecHandle *handle) noexcept -> bool {
			auto proc = reinterpret_cast<const IndustrySpec *>(handle)->check_proc;
			return proc == CHECK_REFINERY || proc == CHECK_OIL_RIG;
		},
		.environment = GetRustEnvironmentServices(),
		.density = []() noexcept -> uint8_t { return _settings_game.difficulty.industry_density; },
		.custom_number = []() noexcept -> uint32_t { return _settings_game.game_creation.custom_industry_number; },
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
		.cargo_lists = [](const OpenTTDIndustrySpecHandle *handle) noexcept -> OpenTTDIndustryCargoLists {
			const auto *spec = reinterpret_cast<const IndustrySpec *>(handle);
			OpenTTDIndustryCargoLists result;
			std::ranges::copy(spec->produced_cargo, result.produced);
			std::ranges::copy(spec->accepts_cargo, result.accepted);
			return result;
		},
		.callback_mask = RustIndustrySpecCallbackMask<OpenTTDIndustrySpecHandle>,
		.set_town = [](void *shell, const OpenTTDTownHandle *town) noexcept { RustIndustry(shell)->town = const_cast<Town *>(reinterpret_cast<const Town *>(town)); },
		.generating = GetRustSharedServices().clear_square_services.generating_world,
		.callback = [](void *shell, uint16_t callback, uint32_t parameter, uint32_t random) noexcept -> uint16_t {
			Industry *i = RustIndustry(shell);
			return GetIndustryCallback(static_cast<CallbackID>(callback), parameter, random, i, i->type, INVALID_TILE);
		},
		.callback_error = RustSpecCallbackError<IndustrySpec, OpenTTDIndustrySpecHandle>,
		.grf_version = RustSpecGRFVersion<IndustrySpec, OpenTTDIndustrySpecHandle>,
		.cargo_file = RustSpecHandleGRFFile<IndustrySpec, OpenTTDIndustrySpecHandle>,
		.layout_size = [](const OpenTTDIndustryLayoutHandle *layout) noexcept -> size_t { return reinterpret_cast<const IndustryTileLayout *>(layout)->size(); },
		.layout_tile = [](const OpenTTDIndustryLayoutHandle *layout, size_t index) noexcept -> OpenTTDIndustryLayoutTile {
			const auto &tile = (*reinterpret_cast<const IndustryTileLayout *>(layout))[index];
			return {tile.ti.x, tile.ti.y, tile.gfx};
		},
		.water_class = [](uint32_t tile) noexcept -> uint8_t { return IsWaterTile(TileIndex(tile)) ? to_underlying(RustWaterClass(tile)) : to_underlying(WaterClass::Invalid); },
		.clear_tile = [](uint32_t tile, uint16_t flags) noexcept { Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlags{flags}, TileIndex(tile)); },
		.make_industry = [](uint32_t tile, uint16_t industry, uint16_t gfx, uint32_t random, uint8_t water) noexcept { MakeIndustry(TileIndex(tile), IndustryID(industry), gfx, random, static_cast<WaterClass>(water)); },
		.set_construction_counter = [](uint32_t tile, uint8_t counter) noexcept { SetIndustryConstructionCounter(TileIndex(tile), counter); },
		.set_construction_stage = [](uint32_t tile, uint8_t stage) noexcept { SetIndustryConstructionStage(TileIndex(tile), stage); },
		.translated_gfx = [](uint16_t gfx) noexcept -> uint16_t { return GetTranslatedIndustryTileID(gfx); },
		.animation_enabled = [](uint16_t gfx) noexcept -> bool { return GetIndustryTileSpec(gfx)->animation.status != AnimationStatus::NoAnimation; },

		.build_window_dirty = []() noexcept { SetWindowDirty(WC_BUILD_INDUSTRY, 0); },
		.serve_neutral = GetRustSharedServices().station_serve_neutral,
	};
	return services;
}

const OpenTTDIndustryTileServices &GetIndustryTileServices() noexcept
{
	static const OpenTTDIndustryTileServices services = {
		.gfx = [](uint32_t tile) noexcept -> uint16_t { return GetIndustryGfx(TileIndex(tile)); },
		.set_gfx = [](uint32_t tile, uint16_t gfx) noexcept { SetIndustryGfx(TileIndex(tile), gfx); },
		.frame = OpenTTDAnimationFrame,
		.set_frame = GetRustSharedServices().tile_animation_set_frame,
		.animation_loop = [](uint32_t tile) noexcept -> uint8_t { return GetIndustryAnimationLoop(TileIndex(tile)); },
		.set_animation_loop = [](uint32_t tile, uint8_t value) noexcept { SetIndustryAnimationLoop(TileIndex(tile), value); },
		.ambient = OpenTTDAmbientSound,

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
		.anim_production = [](uint16_t gfx) noexcept -> uint16_t { return GetIndustryTileSpec(gfx)->anim_production; },
		.anim_next = [](uint16_t gfx) noexcept -> uint16_t { return GetIndustryTileSpec(gfx)->anim_next; },
		.build_oilrig = [](uint32_t tile) noexcept { BuildOilRig(TileIndex(tile)); },
		.pixel_max_z = OpenTTDTileMaxPixelZ,
		.spec = [](uint16_t gfx) noexcept -> const OpenTTDIndustryTileSpec * { return GetIndustryTileSpec(gfx); },
		.acceptance = [](const OpenTTDIndustryTileSpec *spec) noexcept -> OpenTTDIndustryTileAcceptance {
			OpenTTDIndustryTileAcceptance result;
			std::ranges::copy(spec->accepts_cargo, result.cargo);
			std::ranges::copy(spec->acceptance, result.amount);
			result.accepts_all = spec->special_flags.Test(IndustryTileSpecialFlag::AcceptsAllCargo);
			result.accepts_callback = spec->callback_mask.Test(IndustryTileCallbackMask::AcceptCargo);
			result.amount_callback = spec->callback_mask.Test(IndustryTileCallbackMask::CargoAcceptance);
			return result;
		},
		.callback = OpenTTDIndustryTileCallback,
		.cargo_file = RustSpecGRFFile<IndustryTileSpec>,
	};
	return services;
}

static const OpenTTDIndustryCommandServices &GetIndustryCommandServices()
{
	static const OpenTTDIndustryCommandServices services = {
		.deity = IndustryCommandDeity,
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
		.current_company = []() noexcept -> uint8_t { return OpenTTDCurrentCompany().base(); },
		.magic_bulldozer = GetRustTownAuthorityServices().bulldozer,
		.object_in_way = [](OpenTTDIndustryCost *out, const OpenTTDIndustrySpecHandle *spec) noexcept { *out = CommandCostWithParam(STR_ERROR_GENERIC_OBJECT_IN_THE_WAY, reinterpret_cast<const IndustrySpec *>(spec)->name); },
		.slopes_refused = [](const OpenTTDIndustryTileSpec *spec) noexcept -> uint8_t { return spec->slopes_refused; },
		.close_ai = [](Industry *industry) noexcept { AI::BroadcastNewEvent(new ScriptEventIndustryClose(industry->index)); },
		.close_game = [](Industry *industry) noexcept { Game::NewEvent(new ScriptEventIndustryClose(industry->index)); },
		.destroy = [](Industry *industry) noexcept { delete industry; },
		.removal_cost = [](const OpenTTDIndustrySpecHandle *spec) noexcept -> int64_t { return reinterpret_cast<const IndustrySpec *>(spec)->GetRemovalCost(); },
		.tile_max_z = [](uint32_t tile) noexcept -> int32_t { return OpenTTDTileMaxZ(TileIndex(tile)); },
		.autoslope_callback = [](const OpenTTDIndustryTileSpec *spec) noexcept -> bool { return spec->callback_mask.Test(IndustryTileCallbackMask::Autoslope); },
		.callback_boolean = RustSpecBooleanCallback<IndustryTileSpec>,
		.command_checks = &GetRustCommandServices(),
	};
	return services;
}

static const OpenTTDIndustryTransactionServices &GetIndustryTransactionServices()
{
	static const OpenTTDIndustryTransactionServices services = {
		.enabled = RustSpecEnabled<IndustrySpec, OpenTTDIndustrySpecHandle>,
		.layout_count = [](const OpenTTDIndustrySpecHandle *spec) noexcept -> size_t { return reinterpret_cast<const IndustrySpec *>(spec)->layouts.size(); },
		.layout = [](const OpenTTDIndustrySpecHandle *spec, size_t index) noexcept -> const OpenTTDIndustryLayoutHandle * { return reinterpret_cast<const OpenTTDIndustryLayoutHandle *>(&reinterpret_cast<const IndustrySpec *>(spec)->layouts[index]); },
		.raw = [](const OpenTTDIndustrySpecHandle *spec) noexcept -> bool { return reinterpret_cast<const IndustrySpec *>(spec)->IsRawIndustry(); },
		.raw_setting = []() noexcept -> uint8_t { return _settings_game.construction.raw_industry_construction; },
		.check_proc = [](const OpenTTDIndustrySpecHandle *spec) noexcept -> uint8_t { return reinterpret_cast<const IndustrySpec *>(spec)->check_proc; },
		.conflicting = [](const OpenTTDIndustrySpecHandle *spec) noexcept -> OpenTTDIndustryConflictKinds { OpenTTDIndustryConflictKinds result; std::ranges::copy(reinterpret_cast<const IndustrySpec *>(spec)->conflicting, result.kinds); return result; },
		.multiple_per_town = []() noexcept -> bool { return _settings_game.economy.multiple_industry_per_town; },
		.town = OpenTTDIndustryTownHandle,
		.population = [](const OpenTTDTownHandle *town) noexcept -> uint32_t { return reinterpret_cast<const Town *>(town)->cache.population; },
		.town_position = OpenTTDTownTile,
		.industry = RustIndustryByID<uint16_t>,
		.highest_snow_line = OpenTTDHighestSnowLine,
		.edge_distance = [](uint32_t tile, uint8_t direction) noexcept -> uint32_t { return DistanceFromEdgeDir(TileIndex(tile), static_cast<DiagDirection>(direction)); },
		.refinery_limit = []() noexcept -> uint8_t { return _settings_game.game_creation.oil_refinery_limit; },
		.tile_height = [](uint32_t tile) noexcept -> uint32_t { return TileHeight(TileIndex(tile)); },
		.set_company = [](uint8_t company) noexcept { OpenTTDSetCurrentCompany(CompanyID(company)); },
		.temporary_shell = [](OpenTTDIndustry *owner, void *context, OpenTTDIndustryTemporaryVisit visit) noexcept { Industry industry(owner); industry.index = IndustryID::Invalid(); visit(context, &industry); },
		.resolve_location = openttd_industry_resolve_location,
		.resolve_shape = openttd_industry_resolve_shape,
		.location_error = RustLocationCallbackError<IndustrySpec, OpenTTDIndustrySpecHandle, OpenTTDIndustryLocationResult, OpenTTDIndustryCost, STR_ERROR_SITE_UNSUITABLE>,
		.shape_error = RustLocationCallbackError<IndustryTileSpec, IndustryTileSpec, OpenTTDIndustryLocationResult, OpenTTDIndustryCost, STR_ERROR_SITE_UNSUITABLE>,
		.tile_grf_version = RustSpecGRFVersion<IndustryTileSpec>,
		.shape_callback = [](const OpenTTDIndustryTileSpec *spec) noexcept -> bool { return spec->callback_mask.Test(IndustryTileCallbackMask::ShapeCheck); },
		.terragenesis = []() noexcept -> bool { return _settings_game.game_creation.land_generator == LG_TERRAGENESIS; },
		.ignore_restrictions = []() noexcept -> bool { return _ignore_industry_restrictions; },
		.platform = []() noexcept -> uint8_t { return _settings_game.construction.industry_platform; },
		.can_allocate = []() noexcept -> bool { return Industry::CanAllocateItem(); },
		.allocate = [](uint32_t tile) noexcept -> OpenTTDIndustryAllocation { Industry *i = new Industry(TileIndex(tile)); return {i, RustIndustryCanonicalOwner(i), RustIndustryID(i)}; },
		.prospecting_chance = [](const OpenTTDIndustrySpecHandle *spec) noexcept -> uint32_t { return reinterpret_cast<const IndustrySpec *>(spec)->prospecting_chance; },
		.local_company = []() noexcept -> bool { return IsLocalCompany(); },
		.prospect_no_place = []() noexcept { ShowErrorMessage(GetEncodedString(STR_ERROR_CAN_T_PROSPECT_INDUSTRY), GetEncodedString(STR_ERROR_NO_SUITABLE_PLACES_FOR_PROSPECTING), WL_INFO); },
		.prospect_unlucky = []() noexcept { ShowErrorMessage(GetEncodedString(STR_ERROR_CAN_T_PROSPECT_INDUSTRY), GetEncodedString(STR_ERROR_PROSPECTING_WAS_UNLUCKY), WL_INFO); },
		.construction_cost = [](const OpenTTDIndustrySpecHandle *spec) noexcept -> int64_t { return reinterpret_cast<const IndustrySpec *>(spec)->GetConstructionCost(); },
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
		.tile_policy = &GetIndustryTilePolicyServices(),
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
#include "abi-visit-OpenTTDIndustryPropertyServices.hpp"
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
		AbiServicesComplete(GetIndustryPropertyServices()) &&
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
