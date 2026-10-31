/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_house.cpp Implementation of NewGRF houses. */

#include "stdafx.h"
#include "debug.h"
#include "landscape.h"
#include "newgrf_badge.h"
#include "newgrf_house.h"
#include "newgrf_spritegroup.h"
#include "newgrf_town.h"
#include "newgrf_sound.h"
#include "company_func.h"
#include "company_base.h"
#include "town.h"
#include "genworld.h"
#include "newgrf_animation_base.h"
#include "rust/animation_binding.hpp"
#include "newgrf_cargo.h"
#include "station_base.h"
#include "rust/house_ffi.h"
#include "rust/services_newgrf.h"
#include "rust/services_cargo.h"
#include "rust/settings_services.h"
#include "rust/environment_services.h"

#include "safeguards.h"

static uint16_t EvaluateHouseCallback(const OpenTTDHouseCallbackArgs *args, int32_t *regs, size_t count) noexcept
{
	HouseResolverObject object(args->house, TileIndex{args->tile}, args->town, static_cast<CallbackID>(args->callback), args->param1, args->param2, args->prospective, args->initial_random, args->watched, args->view);
	return object.ResolveCallback({regs, count});
}
static OpenTTDHouseReseed EvaluateHouseReseed(uint16_t house, uint32_t tile, Town *town, uint8_t waiting) noexcept
{
	HouseResolverObject object(house, TileIndex{tile}, town, CBID_RANDOM_TRIGGER);
	object.SetWaitingRandomTriggers(HouseRandomTriggers{waiting});
	object.ResolveRerandomisation();
	return {object.GetReseedSum(), object.GetUsedRandomTriggers().base()};
}
static OpenTTDHouseSearchSpec HouseSearchSpec(uint16_t house) noexcept
{
	const auto *s = RustHouseSpec(house);
	return {.grfid = RustSpecGRFID(s), .local = RustSpecLocalID(s), .house_class = RustTownHouseClass(s), .has_file = RustSpecHasGRFFile(s)};
}
static OpenTTDHouseNearbySpec HouseNearbySpec(uint16_t house) noexcept
{
	const auto *s = RustHouseSpec(house);
	return {.file = RustSpecGRFFile(s), .local = RustSpecLocalID(s), .house_class = RustTownHouseClass(s)};
}
const OpenTTDHouseResolverServices &GetHouseResolverServices() noexcept
{
	static const OpenTTDHouseResolverServices services{
		.towns = &GetRustTownServices(),
		.shared = &GetRustSharedServices(),
		.storage = &GetRustStorageServices(),
		.larger_towns = OpenTTDLargerTowns,

		.file = RustResolverGRFFile,

		.stage = [](uint32_t tile) noexcept { return GetHouseBuildingStage(TileIndex{tile}); },
		.age = RustHouseAge,
		.random_bits = RustHouseRandomBits,
		.random_triggers = RustHouseRandomTriggers,
		.search_spec = HouseSearchSpec,
		.nearby_spec = HouseNearbySpec,
		.file_identity = [](uint16_t house) noexcept -> OpenTTDHouseFileIdentity { const auto *spec = RustHouseSpec(house); return {.grfid = RustSpecGRFID(spec), .has_file = RustSpecHasGRFFile(spec)}; },
		.badges = RustHouseBadges,
		.root = RustHouseSpriteGroup,
		.spec_house = [](const HouseSpec *spec) noexcept -> uint16_t { return spec - HouseSpec::Get(0); },
		.local_id = RustHouseLocalID,
		.base_scope = RustResolverBaseScope,
		.evaluate = EvaluateHouseCallback,
		.reseed = EvaluateHouseReseed,
		.unhandled_house = [](uint8_t variable) noexcept { Debug(grf, 1, "Unhandled house variable 0x{:X}", variable); },
		.unhandled_town = [](uint8_t variable) noexcept { Debug(grf, 1, "Unhandled town variable 0x{:X}", variable); },
	};
	return services;
}

HouseOverrideManager _house_mngr(NEW_HOUSE_OFFSET, NUM_HOUSES, INVALID_HOUSE_ID);

/**
 * Retrieve the grf file associated with a house.
 * @param house_id House to query.
 * @return The associated GRF file (may be \c nullptr).
 */
static const GRFFile *GetHouseSpecGrf(HouseID house_id)
{
	const HouseSpec *hs = RustHouseSpec(house_id);
	return (hs != nullptr) ? RustSpecGRFFile(hs) : nullptr;
}

extern const HouseSpec _original_house_specs[NEW_HOUSE_OFFSET];
std::vector<HouseSpec> _house_specs;

/**
 * Get a reference to all HouseSpecs.
 * @return Reference to vector of all HouseSpecs.
 */
std::vector<HouseSpec> &HouseSpec::Specs()
{
	return _house_specs;
}

/**
 * Gets the index of this spec.
 * @return The index.
 */
HouseID HouseSpec::Index() const
{
	return static_cast<HouseID>(this - _house_specs.data());
}

/**
 * Get the spec for a house ID.
 * @param house_id The ID of the house.
 * @return The HouseSpec associated with the ID.
 */
HouseSpec *HouseSpec::Get(size_t house_id)
{
	/* Empty house if index is out of range -- this might happen if NewGRFs are changed. */
	static HouseSpec empty = {};

	assert(house_id < NUM_HOUSES);
	if (house_id >= _house_specs.size()) return &empty;
	return &_house_specs[house_id];
}

/* Reset and initialise house specs. */

/**
 * Construct a resolver for a house.
 * @param house_id House to query.
 * @param tile %Tile containing the house.
 * @param town %Town containing the house.
 * @param callback Callback ID.
 * @param param1 First parameter (var 10) of the callback.
 * @param param2 Second parameter (var 18) of the callback.
 * @param not_yet_constructed House is still under construction.
 * @param initial_random_bits Random bits during construction checks.
 * @param watched_cargo_triggers Cargo types that triggered the watched cargo callback.
 */
HouseScopeResolver::HouseScopeResolver(ResolverObject &ro, HouseID house_id, TileIndex tile, Town *town,
		bool not_yet_constructed, uint8_t initial_random_bits, CargoTypes watched_cargo_triggers, int view)
	: ScopeResolver(ro), context{{town, town == nullptr ? nullptr : town->growth_owner.get(), town == nullptr ? uint16_t{} : town->index.base(), not_yet_constructed}, watched_cargo_triggers, tile.base(), view, house_id, initial_random_bits, not_yet_constructed}
{
}

HouseResolverObject::HouseResolverObject(HouseID house_id, TileIndex tile, Town *town,
		CallbackID callback, uint32_t param1, uint32_t param2,
		bool not_yet_constructed, uint8_t initial_random_bits, CargoTypes watched_cargo_triggers, int view)
	: SpecializedResolverObject<HouseRandomTriggers>(GetHouseSpecGrf(house_id), callback, param1, param2),
	house_scope(*this, house_id, tile, town, not_yet_constructed, initial_random_bits, watched_cargo_triggers, view),
	town_scope(*this, town, not_yet_constructed) // Don't access StorePSA if house is not yet constructed.
{
	/* Tile must be valid and a house tile, unless not yet constructed in which case it may also be INVALID_TILE. */
	assert((IsValidTile(tile) && (not_yet_constructed || IsTileType(tile, MP_HOUSE))) || (not_yet_constructed && tile == INVALID_TILE));

	this->root_spritegroup = openttd_rust_house_root(&this->house_scope.context);
}

GrfSpecFeature HouseResolverObject::GetFeature() const
{
	return GSF_HOUSES;
}

uint32_t HouseResolverObject::GetDebugID() const
{
	return openttd_rust_house_debug_id(&this->house_scope.context);
}


/**
 * Get read-only span of total HouseID building counts.
 * @return span of HouseID building counts.
 */
std::span<const uint> GetBuildingHouseIDCounts()
{
	const auto counts = openttd_rust_town_global_id_counts();
	return {static_cast<const uint *>(counts.data), counts.size};
}


uint32_t HouseScopeResolver::GetRandomBits() const { return openttd_rust_house_random(&GetHouseResolverServices(), &this->context, false); }
uint32_t HouseScopeResolver::GetRandomTriggers() const { return openttd_rust_house_random(&GetHouseResolverServices(), &this->context, true); }
uint32_t HouseScopeResolver::GetVariable(uint8_t variable, uint32_t parameter, bool &available) const
{
	return openttd_rust_house_variable(&GetHouseResolverServices(), &this->context, &this->ro, variable, parameter, &available);
}

uint16_t GetHouseCallback(CallbackID callback, uint32_t param1, uint32_t param2, HouseID house_id, Town *town, TileIndex tile, std::span<int32_t> regs100,
		bool not_yet_constructed, uint8_t initial_random_bits, CargoTypes watched_cargo_triggers, int view)
{
	const OpenTTDHouseCallbackArgs args{town, watched_cargo_triggers, param1, param2, tile.base(), view, house_id, callback, initial_random_bits, not_yet_constructed};
	return openttd_rust_house_callback(&args, regs100.data(), regs100.size());
}

static void DrawTileLayout(const TileInfo *ti, const DrawTileSpriteSpan &dts, uint8_t stage, HouseID house_id)
{
	const HouseSpec *hs = HouseSpec::Get(house_id);
	PaletteID palette = GetColourPalette(hs->random_colour[TileHash2Bit(ti->x, ti->y)]);
	if (hs->callback_mask.Test(HouseCallbackMask::Colour)) {
		uint16_t callback = GetHouseCallback(CBID_HOUSE_COLOUR, 0, 0, house_id, Town::GetByTile(ti->tile), ti->tile);
		if (callback != CALLBACK_FAILED) {
			/* If bit 14 is set, we should use a 2cc colour map, else use the callback value. */
			palette = HasBit(callback, 14) ? GB(callback, 0, 8) + SPR_2CCMAP_BASE : callback;
		}
	}

	SpriteID image = dts.ground.sprite;
	PaletteID pal = dts.ground.pal;

	if (HasBit(image, SPRITE_MODIFIER_CUSTOM_SPRITE)) image += stage;
	if (HasBit(pal, SPRITE_MODIFIER_CUSTOM_SPRITE)) pal += stage;

	if (GB(image, 0, SPRITE_WIDTH) != 0) {
		DrawGroundSprite(image, GroundSpritePaletteTransform(image, pal, palette));
	}

	DrawNewGRFTileSeq(ti, &dts, TO_HOUSES, stage, palette);
}

void DrawNewHouseTile(TileInfo *ti, HouseID house_id)
{
	const HouseSpec *hs = HouseSpec::Get(house_id);

	if (ti->tileh != SLOPE_FLAT) {
		bool draw_old_one = true;
		if (hs->callback_mask.Test(HouseCallbackMask::DrawFoundations)) {
			/* Called to determine the type (if any) of foundation to draw for the house tile */
			uint32_t callback_res = GetHouseCallback(CBID_HOUSE_DRAW_FOUNDATIONS, 0, 0, house_id, Town::GetByTile(ti->tile), ti->tile);
			if (callback_res != CALLBACK_FAILED) draw_old_one = ConvertBooleanCallback(hs->grf_prop.grffile, CBID_HOUSE_DRAW_FOUNDATIONS, callback_res);
		}

		if (draw_old_one) DrawFoundation(ti, FOUNDATION_LEVELED);
	}

	HouseResolverObject object(house_id, ti->tile, Town::GetByTile(ti->tile));

	const auto *group = object.Resolve<TileLayoutSpriteGroup>();
	if (group != nullptr) {
		/* Limit the building stage to the number of stages supplied. */
		uint8_t stage = GetHouseBuildingStage(ti->tile);
		auto processor = group->ProcessRegisters(object, &stage);
		auto dts = processor.GetLayout();
		DrawTileLayout(ti, dts, stage, house_id);
	}
}

/**
 * Draw representation of a house tile for GUI purposes.
 * @param x Position x of image.
 * @param y Position y of image.
 * @param spec House spec to draw.
 * @param house_id House ID to draw.
 * @param view The house's 'view'.
 */
void DrawNewHouseTileInGUI(int x, int y, const HouseSpec *spec, HouseID house_id, int view)
{
	HouseResolverObject object(house_id, INVALID_TILE, nullptr, CBID_NO_CALLBACK, 0, 0, true, view);
	const auto *group = object.Resolve<TileLayoutSpriteGroup>();
	if (group == nullptr) return;

	uint8_t stage = TOWN_HOUSE_COMPLETED;
	auto processor = group->ProcessRegisters(object, &stage);
	auto dts = processor.GetLayout();

	PaletteID palette = GetColourPalette(spec->random_colour[0]);
	if (spec->callback_mask.Test(HouseCallbackMask::Colour)) {
		uint16_t callback = GetHouseCallback(CBID_HOUSE_COLOUR, 0, 0, house_id, nullptr, INVALID_TILE, {}, true, view);
		if (callback != CALLBACK_FAILED) {
			/* If bit 14 is set, we should use a 2cc colour map, else use the callback value. */
			palette = HasBit(callback, 14) ? GB(callback, 0, 8) + SPR_2CCMAP_BASE : callback;
		}
	}

	SpriteID image = dts.ground.sprite;
	PaletteID pal = dts.ground.pal;

	if (HasBit(image, SPRITE_MODIFIER_CUSTOM_SPRITE)) image += stage;
	if (HasBit(pal, SPRITE_MODIFIER_CUSTOM_SPRITE)) pal += stage;

	if (GB(image, 0, SPRITE_WIDTH) != 0) {
		DrawSprite(image, GroundSpritePaletteTransform(image, pal, palette), x, y);
	}

	DrawNewGRFTileSeqInGUI(x, y, &dts, stage, palette);
}


/** Helper class for animation control. */
const OpenTTDHouseAnimation &GetHouseAnimationBinding() noexcept
{
	static const auto binding = MakeRustAnimationBindingDirect<HouseSpec, Town, CargoTypes>(
		[](uint32_t t) noexcept -> OpenTTDHouseAnimationEntry { const auto *s = RustHouseSpec(GetHouseType(TileIndex{t})); if (s == nullptr) return {nullptr, nullptr}; return {s, Town::GetByTile(TileIndex{t})}; },
		[](const HouseSpec *s) noexcept -> OpenTTDAnimationInfo { return {s->animation.speed, s->animation.frames, s->animation.status == AnimationStatus::Looping, s->callback_mask.Test(HouseCallbackMask::AnimationSpeed), s->callback_mask.Test(HouseCallbackMask::AnimationNextFrame), s->extra_flags.Test(HouseExtraFlag::Callback1ARandomBits), true}; }, openttd_rust_house_animation_callback);
	return binding;
}

void AnimateNewHouseTile(TileIndex tile)
{
	openttd_rust_animate_house(tile.base());
}
