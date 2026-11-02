/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file object_cmd.cpp Handling of object tiles. */

#include "stdafx.h"
#include "rust/animation_ffi.h"
#include "landscape.h"
#include "command_func.h"
#include "company_func.h"
#include "viewport_func.h"
#include "company_base.h"
#include "town.h"
#include "bridge_map.h"
#include "genworld.h"
#include "autoslope.h"
#include "clear_func.h"
#include "water.h"
#include "window_func.h"
#include "company_gui.h"
#include "cheat_type.h"
#include "object.h"
#include "cargopacket.h"
#include "core/random_func.hpp"
#include "core/pool_func.hpp"
#include "object_map.h"
#include "object_base.h"
#include "newgrf_config.h"
#include "newgrf_object.h"
#include "timer/timer_game_calendar.h"
#include "newgrf_debug.h"
#include "vehicle_func.h"
#include "station_func.h"
#include "object_cmd.h"
#include "landscape_cmd.h"
#include "pathfinder/water_regions.h"

#include "table/strings.h"
#include "table/object_land.h"

#include "rust/command_services.h"
#include "rust/landscape_loop_ffi.h"
#include "rust/rail_command_ffi.h"
#include "rust/ship_control_ffi.h"
#include "rust/town_founding_ffi.h"
#include "rust/company_adapter.h"
#include "rust/company_services.h"
#include "rust/environment_services.h"
#include "rust/ground_services.h"
#include "rust/services_landscape.h"
#include "rust/services_object.h"
#include "rust/services_cargo.h"
#include "rust/services_newgrf.h"
#include "rust/settings_services.h"

#include "rust/movement_ffi.h"

#include "safeguards.h"

ObjectPool _object_pool("Object");
INSTANTIATE_POOL_METHODS(Object)

/**
 * Get the object associated with a tile.
 * @param tile The tile to fetch the object for.
 * @return The object.
 */
/* static */ Object *Object::GetByTile(TileIndex tile)
{
	return Object::Get(GetObjectIndex(tile));
}

/**
 * Gets the ObjectType of the given object tile
 * @param t the tile to get the type from.
 * @pre IsTileType(t, MP_OBJECT)
 * @return the type.
 */
ObjectType GetObjectType(Tile t)
{
	assert(IsTileType(t, MP_OBJECT));
	return Object::GetByTile(t)->type;
}

/** Initialize/reset the objects. */
void InitializeObjects()
{
	Object::ResetTypeCounts();
}

/**
 * Actually build the object.
 * @param type  The type of object to build.
 * @param tile  The tile to build the northern tile of the object on.
 * @param owner The owner of the object.
 * @param town  Town the tile is related with.
 * @param view  The view for the object.
 * @pre All preconditions for building the object at that location
 *      are met, e.g. slope and clearness of tiles are checked.
 */
void BuildObject(ObjectType type, TileIndex tile, CompanyID owner, Town *town, uint8_t view)
{
	openttd_rust_object_build(&GetRustObjectServices(), type, tile.base(), owner.base(), town, view);
}

/**
 * Increase the HQ size.
 * @param tile The (northern) tile of the company HQ.
 */

/**
 * Get the size of the HQ.
 * @param tile The (northern) tile of the company HQ.
 * @return HQ size.
 */
static uint8_t GetCompanyHQSize(TileIndex tile)
{
	/* We encode the company HQ size in the animation state. */
	return GetAnimationFrame(tile);
}

/**
 * Update the CompanyHQ to the state associated with the given score
 * @param tile  The (northern) tile of the company HQ, or INVALID_TILE.
 * @param score The current (performance) score of the company.
 */
void UpdateCompanyHQ(TileIndex tile, uint score)
{
	openttd_rust_object_update_hq(&GetRustObjectServices(), tile.base(), score);
}

/**
 * Updates the colour of the object whenever a company changes.
 * @param c The company the company colour changed of.
 */
void UpdateObjectColours(const Company *c)
{
	openttd_rust_object_colours(&GetRustObjectServices(), c->index.base());
}

extern CommandCost CheckBuildableTile(TileIndex tile, DiagDirections invalid_dirs, int &allowed_z, bool allow_steep, bool check_bridge);

/**
 * Build an object object
 * @param flags type of operation
 * @param tile tile where the object will be located
 * @param type the object type to build
 * @param view the view for the object
 * @return the cost of this operation or an error
 */
CommandCost CmdBuildObject(DoCommandFlags flags, TileIndex tile, ObjectType type, uint8_t view)
{
	CommandCost out, tmp, aux;
	openttd_rust_object_build_command(&GetRustObjectServices(), RustCommandCost(&out), RustCommandCost(&tmp), RustCommandCost(&aux), flags.base(), tile.base(), type, view);
	return out;
}

/**
 * Construct multiple objects in an area
 * @param flags of operation to conduct
 * @param tile end tile of area dragging
 * @param start_tile start tile of area dragging
 * @param type the object type to build
 * @param view the view for the object
 * @param diagonal Whether to use the Orthogonal (0) or Diagonal (1) iterator.
 * @return the cost of this operation or an error
 */
CommandCost CmdBuildObjectArea(DoCommandFlags flags, TileIndex tile, TileIndex start_tile, ObjectType type, uint8_t view, bool diagonal)
{
	CommandCost out, ret, tmp, aux, last;
	openttd_rust_object_build_area(&GetRustObjectServices(), RustCommandCost(&out), RustCommandCost(&ret), RustCommandCost(&tmp), RustCommandCost(&aux), RustCommandCost(&last), flags.base(), tile.base(), start_tile.base(), type, view, diagonal);
	return out;
}

static Foundation GetFoundation_Object(TileIndex tile, Slope tileh)
{
	return openttd_rust_movement_foundation(&GetRustMovementServices(), tile.base(), tileh);
}

static void DrawTile_Object(TileInfo *ti)
{
	ObjectType type = GetObjectType(ti->tile);
	const ObjectSpec *spec = ObjectSpec::Get(type);

	/* Fall back for when the object doesn't exist anymore. */
	if (!spec->IsEnabled()) type = OBJECT_TRANSMITTER;

	DrawFoundation(ti, GetFoundation_Object(ti->tile, ti->tileh));

	if (type < NEW_OBJECT_OFFSET) {
		const DrawTileSprites *dts = nullptr;
		Owner to = GetTileOwner(ti->tile);
		PaletteID palette = to == OWNER_NONE ? PAL_NONE : GetCompanyPalette(to);

		if (type == OBJECT_HQ) {
			TileIndex diff = ti->tile - Object::GetByTile(ti->tile)->location.tile;
			dts = &_object_hq[GetCompanyHQSize(ti->tile) << 2 | TileY(diff) << 1 | TileX(diff)];
		} else {
			dts = &_objects[type];
		}

		if (spec->flags.Test(ObjectFlag::HasNoFoundation)) {
			/* If an object has no foundation, but tries to draw a (flat) ground
			 * type... we have to be nice and convert that for them. */
			switch (dts->ground.sprite) {
				case SPR_FLAT_BARE_LAND:          DrawClearLandTile(ti, 0); break;
				case SPR_FLAT_1_THIRD_GRASS_TILE: DrawClearLandTile(ti, 1); break;
				case SPR_FLAT_2_THIRD_GRASS_TILE: DrawClearLandTile(ti, 2); break;
				case SPR_FLAT_GRASS_TILE:         DrawClearLandTile(ti, 3); break;
				default: DrawGroundSprite(dts->ground.sprite, palette);     break;
			}
		} else {
			DrawGroundSprite(dts->ground.sprite, palette);
		}

		if (!IsInvisibilitySet(TO_STRUCTURES)) {
			for (const DrawTileSeqStruct &dtss : dts->GetSequence()) {
				AddSortableSpriteToDraw(dtss.image.sprite, palette, *ti, dtss, IsTransparencySet(TO_STRUCTURES));
			}
		}
	} else {
		DrawNewObjectTile(ti, spec);
	}

	DrawBridgeMiddle(ti, {});
}



/**
 * Perform the actual removal of the object from the map.
 * @param o The object to really clear.
 */



static void AddAcceptedCargo_Object(TileIndex tile, CargoArray &acceptance, CargoTypes &always_accepted)
{
	openttd_rust_object_cargo(&GetRustObjectServices(), tile.base(), acceptance.data(), &always_accepted);
}

static void AddProducedCargo_Object(TileIndex tile, CargoArray &produced)
{
	openttd_rust_object_cargo(&GetRustObjectServices(), tile.base(), produced.data(), nullptr);
}


static void GetTileDesc_Object(TileIndex tile, TileDesc &td)
{
	const ObjectSpec *spec = ObjectSpec::GetByTile(tile);
	td.str = spec->name;
	td.owner[0] = GetTileOwner(tile);
	td.build_date = Object::GetByTile(tile)->build_date;

	if (spec->grf_prop.HasGrfFile()) {
		td.grf = GetGRFConfig(spec->grf_prop.grfid)->GetName();
	}
}

static void TileLoop_Object(uint32_t tile_index) noexcept
{
	OpenTTDGroundCall call;
	openttd_rust_object_tile_loop(&GetRustObjectServices(), &call, tile_index);
}



static bool ClickTile_Object(TileIndex tile)
{
	if (!IsObjectType(tile, OBJECT_HQ)) return false;

	ShowCompany(GetTileOwner(tile));
	return true;
}


/**
 * Try to build a lighthouse.
 * @return True iff building a lighthouse succeeded.
 */

/**
 * Try to build a transmitter.
 * @return True iff a transmitter was built.
 */


static void ChangeTileOwner_Object(TileIndex tile, Owner old_owner, Owner new_owner)
{
	/* The shared Rust dispatcher owns object ownership changes. */
	ChangeTileOwner(tile, old_owner, new_owner);
}

static CommandCost TerraformTile_Object(TileIndex tile, DoCommandFlags flags, int z_new, Slope tileh_new)
{
	CommandCost out;
	openttd_rust_object_terraform(&GetRustObjectServices(), RustCommandCost(&out), tile.base(), flags.base(), z_new, tileh_new);
	return out;
}

static CommandCost CheckBuildAbove_Object(TileIndex tile, DoCommandFlags flags, Axis, int height)
{
	CommandCost out;
	openttd_rust_object_build_above(&GetRustObjectServices(), RustCommandCost(&out), tile.base(), flags.base(), height);
	return out;
}

extern const TileTypeProcs _tile_type_object_procs = {
	DrawTile_Object,             // draw_tile_proc
	nullptr, // Rust movement owner dispatches directly.
	nullptr,                 // Rust landscape commands call the owner directly.
	AddAcceptedCargo_Object,     // add_accepted_cargo_proc
	GetTileDesc_Object,          // get_tile_desc_proc
	nullptr, // Rust movement owner dispatches directly.
	ClickTile_Object,            // click_tile_proc
	TileLoop_Object,             // tile_loop_proc
	ChangeTileOwner_Object,      // change_tile_owner_proc
	AddProducedCargo_Object,     // add_produced_cargo_proc
	nullptr,                        // vehicle_enter_tile_proc
	nullptr, // Rust movement owner dispatches directly.
	TerraformTile_Object,        // terraform_tile_proc
	CheckBuildAbove_Object, // check_build_above_proc
};

const OpenTTDObjectServices &GetRustObjectServices() noexcept
{
	static_assert(to_underlying(ObjectAnimationTrigger::Built) == 0 && to_underlying(ObjectAnimationTrigger::TileLoop) == 1 && to_underlying(ObjectAnimationTrigger::TileLoopNorth) == 2);
	static_assert(NUM_OBJECTS == 64000 && OBJECT_HQ == 4 && OBJECT_OWNED_LAND == 3 && OBJECT_STATUE == 2);
	static_assert(EXPENSES_CONSTRUCTION == 0 && CBID_OBJECT_AUTOSLOPE == 0x15D && PR_BUILD_FOUNDATION == 57 && PR_BUILD_OBJECT == 50 && PR_CLEAR_OBJECT == 51);
	static_assert(OWNER_NONE.base() == 16 && OWNER_WATER.base() == 17 && OWNER_DEITY.base() == 18 && MAX_COMPANIES == 15);
	static_assert(NEW_OBJECT_OFFSET == 5 && OBJECT_SIZE_1X1 == 0x11 && INVALID_STRING_ID == 0xFFFF && WaterClass::Invalid == static_cast<WaterClass>(3) && SourceType::Headquarters == static_cast<SourceType>(2));
	static const OpenTTDObjectServices services{
		.shared = &GetRustSharedServices(),
		.cost = &GetRustCommandCostServices(),
		.command = &GetRustCommandServices(),
		.company = &GetRustCompanyServices(),
		.cargo = CargoStorageServices(),
		.town = &GetRustTownServices(),
		.founding = &GetRustTownFoundingServices(),
		.industry_tiles = GetRustIndustryLoopServices().tiles,
		.strings = {STR_ERROR_TOO_MANY_OBJECTS,STR_ERROR_MUST_FOUND_TOWN_FIRST,STR_ERROR_TOO_CLOSE_TO_EDGE_OF_MAP_SUB,STR_ERROR_CAN_T_BUILD_ON_WATER,STR_ERROR_TOO_CLOSE_TO_EDGE_OF_MAP,STR_ERROR_MUST_BE_BUILT_ON_WATER,STR_ERROR_MUST_DEMOLISH_BRIDGE_FIRST,STR_ERROR_FLAT_LAND_REQUIRED,STR_ERROR_YOU_ALREADY_OWN_IT,STR_ERROR_ALREADY_BUILT,STR_ERROR_BUILD_OBJECT_LIMIT_REACHED,STR_ERROR_COMPANY_HEADQUARTERS_IN,STR_ERROR_OBJECT_IN_THE_WAY},
		.spec_count = []() noexcept -> size_t { return ObjectSpec::Count(); },
		.spec = [](uint16_t type) noexcept -> const ObjectSpec * { return ObjectSpec::Get(type); },
		.callbacks = RustObjectSpecCallbacks,
		.colour_policy = [](Object *o) noexcept -> OpenTTDObjectPolicy { const ObjectSpec *spec = RustObjectSpecByTile(o->location.tile.base()); return {RustObjectSpecFlags(spec), RustObjectSpecCallbacks(spec)}; },
		.flags = RustObjectSpecFlags,
		.footprint = [](const ObjectSpec *spec) noexcept -> OpenTTDObjectFootprint { return {RustObjectSpecFlags(spec), RustObjectSpecCallbacks(spec), spec->size}; },
		.clearance = [](uint32_t tile) noexcept -> OpenTTDObjectClearance { const ObjectSpec *spec = RustObjectSpecByTile(tile); return {RustObjectSpecFlags(spec), spec->height}; },
		.area_shape = [](const ObjectSpec *spec) noexcept -> OpenTTDObjectAreaShape { return {RustObjectViews(spec), spec->size}; },
		.admission = [](const ObjectSpec *spec) noexcept -> OpenTTDObjectAdmission { return {RustObjectViews(spec), spec->height}; },
		.generation = [](const ObjectSpec *spec) noexcept -> OpenTTDObjectGeneration { return {RustObjectSpecFlags(spec), spec->generate_amount, RustObjectViews(spec)}; },
		.can_allocate = []() noexcept -> bool { return Object::CanAllocateItem(); },
		.allocate = [](uint16_t type, const Town *town, OpenTTDObjectArea area, uint8_t view) noexcept -> OpenTTDObjectAllocation { Object *o = new Object(type, const_cast<Town *>(town), TileArea(TileIndex(area.tile), area.width, area.height), TimerGameCalendar::date, view); return {o, o->index.base()}; },
		.object_type = [](uint32_t tile) noexcept { return GetObjectType(TileIndex(tile)); },
		.loop_policy = [](uint32_t tile) noexcept -> OpenTTDObjectLoopPolicy { Object *o = RustObjectByTile(tile); const auto kind = o->type; const ObjectSpec *spec = RustObjectSpecByTile(tile); const auto flags = RustObjectSpecFlags(spec); return {spec, o, ObjectFlags{flags}.Test(ObjectFlag::Animation) ? o->location.tile.base() : 0, flags, kind}; },
		.identity = [](uint32_t tile) noexcept -> OpenTTDObjectIdentity { Object *o = RustObjectByTile(tile); return {o, {o->location.tile.base(), o->location.w, o->location.h}, o->type}; },
		.location = [](Object *o) noexcept -> OpenTTDObjectArea { const auto &a = o->location; return {a.tile.base(), a.w, a.h}; },
		.next = [](uint32_t from) noexcept -> OpenTTDObjectNext { for (Object *o : Object::Iterate(from)) return {o, o->index.base(), o->location.tile.base()}; return {nullptr, UINT32_MAX, 0}; },
		.set_colour = [](Object *o, uint8_t colour) noexcept -> void { o->colour = colour; },
		.recolour = RustObjectRecolour,
		.colour_error = RustFixedSpecCallbackError<CBID_OBJECT_COLOUR, ObjectSpec>,
		.make = [](uint32_t tile, uint8_t owner, uint32_t id, WaterClass water, uint32_t random) noexcept -> void { MakeObject(TileIndex(tile), Owner(owner), ObjectID(id), water, random); },
		.delete_object = [](Object *o) noexcept -> void { delete o; },
		.query = &GetRustObjectQueryServices(),
		.resolver = &GetObjectResolverServices(),
		.slope_error = RustLocationCallbackError<ObjectSpec, ObjectSpec, OpenTTDObjectSlopeResult, OpenTTDCommandCost, STR_ERROR_LAND_SLOPED_IN_WRONG_DIRECTION>,
		.build_price = [](const ObjectSpec *spec) noexcept -> OpenTTDObjectPrice { return {spec->build_cost_multiplier, RustGRFPriceShift(RustSpecGRFFile(spec), PR_BUILD_OBJECT)}; },
		.clear_price = [](const ObjectSpec *spec) noexcept -> OpenTTDObjectPrice { return {spec->clear_cost_multiplier, RustGRFPriceShift(RustSpecGRFFile(spec), PR_CLEAR_OBJECT)}; },
		.set_hq = [](uint8_t owner, uint32_t tile) noexcept -> void { Company::Get(CompanyID(owner))->location_of_HQ = TileIndex(tile); SetWindowDirty(WC_COMPANY, owner); },
		.statue_town = [](Object *o) noexcept -> OpenTTDTownRef { Town *t = o->town; return {t, t->growth_owner.get(), t->index.base()}; },
		.close_inspect = RustCloseInspect<GSF_OBJECTS, uint32_t>,
		.progress_set = [](OpenTTDObjectGenerationFrame *frame, uint32_t total) noexcept -> bool { return RunGenerationProgress(frame, [total] { RustSetGeneratingWorldProgress(GWP_OBJECT, total); }); },
		.progress_increase = [](OpenTTDObjectGenerationFrame *frame) noexcept -> bool { return RunGenerationProgress(frame, [] { RustIncreaseGeneratingWorldProgress(GWP_OBJECT); }); },
		.normal = RustNormalGame,
		.bulldozer = GetRustTownAuthorityServices().bulldozer,
		.water_class = GetRustClearServices().water_class,
		.restoration = &GetRustWaterRestorationServices(),
		.set_owner = GetRustSharedServices().set_tile_owner,
		.scale_size_1d = GetRustIndustryLoopServices().generation->scale_by_size_1d,

		.scale_cargo = OpenTTDScaleTownCargo,
	};
	return services;
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDObjectServices.hpp"
#include "abi-visit-OpenTTDObjectQueryServices.hpp"
#include "abi-visit-OpenTTDObjectResolverServices.hpp"
bool CheckAbi_object_cmd() { return AbiServicesComplete(GetRustObjectServices()) && AbiServicesComplete(*GetRustObjectServices().query) && AbiServicesComplete(*GetRustObjectServices().resolver); }
