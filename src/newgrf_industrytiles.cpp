/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_industrytiles.cpp NewGRF handling of industry tiles. */

#include "stdafx.h"
#include "debug.h"
#include "landscape.h"
#include "newgrf_badge.h"
#include "newgrf_industrytiles.h"
#include "newgrf_sound.h"
#include "industry.h"
#include "town.h"
#include "command_func.h"
#include "water.h"
#include "newgrf_animation_base.h"
#ifdef WITH_RUST
#include "rust/animation_binding.hpp"
#include "rust/services_industry.h"
#endif

#include "table/strings.h"

#ifdef WITH_RUST
#include "rust/industry_resolver_ffi.h"
#endif
#include "safeguards.h"

/**
 * Based on newhouses equivalent, but adapted for newindustries
 * @param parameter from callback.  It's in fact a pair of coordinates
 * @param tile TileIndex from which the callback was initiated
 * @param index of the industry been queried for
 * @param signed_offsets Are the x and y offset encoded in parameter signed?
 * @param grf_version8 True, if we are dealing with a new NewGRF which uses GRF version >= 8.
 * @return a construction of bits obeying the newgrf format
 */
#ifndef WITH_RUST
uint32_t GetNearbyIndustryTileInformation(uint8_t parameter, TileIndex tile, IndustryID index, bool signed_offsets, bool grf_version8)
{
	if (parameter != 0) tile = GetNearbyTile(parameter, tile, signed_offsets); // only perform if it is required
	bool is_same_industry = (IsTileType(tile, MP_INDUSTRY) && GetIndustryIndex(tile) == index);

	return GetNearbyTileInformation(tile, grf_version8) | (is_same_industry ? 1 : 0) << 8;
}
#endif

/**
 * This is the position of the tile relative to the northernmost tile of the industry.
 * Format: 00yxYYXX
 * Variable  Content
 * x         the x offset from the northernmost tile
 * XX        same, but stored in a byte instead of a nibble
 * y         the y offset from the northernmost tile
 * YY        same, but stored in a byte instead of a nibble
 * @param tile TileIndex of the tile to evaluate
 * @param ind_tile northernmost tile of the industry
 */
uint32_t GetRelativePosition(TileIndex tile, TileIndex ind_tile)
{
#ifdef WITH_RUST
	return openttd_rust_industry_relative_position(tile.base(), ind_tile.base(), Map::LogX());
#else
	uint8_t x = TileX(tile) - TileX(ind_tile);
	uint8_t y = TileY(tile) - TileY(ind_tile);

	return ((y & 0xF) << 20) | ((x & 0xF) << 16) | (y << 8) | x;
#endif
}

/* virtual */ uint32_t IndustryTileScopeResolver::GetVariable(uint8_t variable, [[maybe_unused]] uint32_t parameter, bool &available) const
{
#ifdef WITH_RUST
	return openttd_rust_industry_tile_variable(&this->context, &this->ro, variable, parameter, &available, &GetIndustryResolverServices());
#else
	switch (variable) {
		/* Construction state of the tile: a value between 0 and 3 */
		case 0x40: return (IsTileType(this->tile, MP_INDUSTRY)) ? GetIndustryConstructionStage(this->tile) : 0;

		/* Terrain type */
		case 0x41: return GetTerrainType(this->tile);

		/* Current town zone of the tile in the nearest town */
		case 0x42: return to_underlying(GetTownRadiusGroup(ClosestTownFromTile(this->tile, UINT_MAX), this->tile));

		/* Relative position */
		case 0x43: return GetRelativePosition(this->tile, this->industry->location.tile);

		/* Animation frame. Like house variable 46 but can contain anything 0..FF. */
		case 0x44: return IsTileType(this->tile, MP_INDUSTRY) ? GetAnimationFrame(this->tile) : 0;

		/* Land info of nearby tiles */
		case 0x60: return GetNearbyIndustryTileInformation(parameter, this->tile,
				this->industry == nullptr ? (IndustryID)IndustryID::Invalid() : this->industry->index, true, this->ro.grffile->grf_version >= 8);

		/* Animation stage of nearby tiles */
		case 0x61: {
			TileIndex tile = GetNearbyTile(parameter, this->tile);
			if (IsTileType(tile, MP_INDUSTRY) && Industry::GetByTile(tile) == this->industry) {
				return GetAnimationFrame(tile);
			}
			return UINT_MAX;
		}

		/* Get industry tile ID at offset */
		case 0x62: return GetIndustryIDAtOffset(GetNearbyTile(parameter, this->tile), this->industry, this->ro.grffile->grfid);

		case 0x7A: return GetBadgeVariableResult(*this->ro.grffile, GetIndustryTileSpec(GetIndustryGfx(this->tile))->badges, parameter);
	}

	Debug(grf, 1, "Unhandled industry tile variable 0x{:X}", variable);

	available = false;
	return UINT_MAX;
#endif
}

/* virtual */ uint32_t IndustryTileScopeResolver::GetRandomBits() const
{
#ifdef WITH_RUST
	assert(this->context.industry != nullptr && IsValidTile(TileIndex(this->context.tile)));
	assert(this->context.id == IndustryID::Invalid().base() || IsTileType(TileIndex(this->context.tile), MP_INDUSTRY));
	return openttd_rust_industry_tile_random(&this->context, &GetIndustryResolverServices());
#else
	assert(this->industry != nullptr && IsValidTile(this->tile));
	assert(this->industry->index == IndustryID::Invalid() || IsTileType(this->tile, MP_INDUSTRY));

	return (this->industry->index != IndustryID::Invalid()) ? GetIndustryRandomBits(this->tile) : 0;
#endif
}

/* virtual */ uint32_t IndustryTileScopeResolver::GetRandomTriggers() const
{
#ifdef WITH_RUST
	assert(this->context.industry != nullptr && IsValidTile(TileIndex(this->context.tile)));
	assert(this->context.id == IndustryID::Invalid().base() || IsTileType(TileIndex(this->context.tile), MP_INDUSTRY));
	return openttd_rust_industry_tile_triggers(&this->context, &GetIndustryResolverServices());
#else
	assert(this->industry != nullptr && IsValidTile(this->tile));
	assert(this->industry->index == IndustryID::Invalid() || IsTileType(this->tile, MP_INDUSTRY));
	if (this->industry->index == IndustryID::Invalid()) return 0;
	return GetIndustryRandomTriggers(this->tile).base();
#endif
}

/**
 * Get the associated NewGRF file from the industry graphics.
 * @param gfx Graphics to query.
 * @return Grf file associated with the graphics, if any.
 */
static const GRFFile *GetIndTileGrffile(IndustryGfx gfx)
{
	const IndustryTileSpec *its = GetIndustryTileSpec(gfx);
	return (its != nullptr) ? its->grf_prop.grffile : nullptr;
}

/**
 * Constructor of the industry tiles scope resolver.
 * @param gfx Graphics of the industry.
 * @param tile %Tile of the industry.
 * @param indus %Industry owning the tile.
 * @param callback Callback ID.
 * @param callback_param1 First parameter (var 10) of the callback.
 * @param callback_param2 Second parameter (var 18) of the callback.
 */
IndustryTileResolverObject::IndustryTileResolverObject(IndustryGfx gfx, TileIndex tile, Industry *indus,
			CallbackID callback, uint32_t callback_param1, uint32_t callback_param2)
	: SpecializedResolverObject<IndustryRandomTriggers>(GetIndTileGrffile(gfx), callback, callback_param1, callback_param2),
	indtile_scope(*this, indus, tile),
	ind_scope(*this, tile, indus, indus->type),
	gfx(gfx)
{
#ifdef WITH_RUST
	this->root_spritegroup = openttd_rust_industry_tile_root(&this->indtile_scope.context, gfx, &GetIndustryResolverServices());
#else
	this->root_spritegroup = GetIndustryTileSpec(gfx)->grf_prop.GetSpriteGroup(indus->index != IndustryID::Invalid());
#endif
}

GrfSpecFeature IndustryTileResolverObject::GetFeature() const
{
	return GSF_INDUSTRYTILES;
}

uint32_t IndustryTileResolverObject::GetDebugID() const
{
	return GetIndustryTileSpec(gfx)->grf_prop.local_id;
}

static void IndustryDrawTileLayout(const TileInfo *ti, const DrawTileSpriteSpan &dts, Colours rnd_colour, uint8_t stage)
{
	SpriteID image = dts.ground.sprite;
	PaletteID pal = dts.ground.pal;

	if (HasBit(image, SPRITE_MODIFIER_CUSTOM_SPRITE)) image += stage;
	if (HasBit(pal, SPRITE_MODIFIER_CUSTOM_SPRITE)) pal += stage;

	if (GB(image, 0, SPRITE_WIDTH) != 0) {
		/* If the ground sprite is the default flat water sprite, draw also canal/river borders
		 * Do not do this if the tile's WaterClass is 'land'. */
		if (image == SPR_FLAT_WATER_TILE && IsTileOnWater(ti->tile)) {
			DrawWaterClassGround(ti);
		} else {
			DrawGroundSprite(image, GroundSpritePaletteTransform(image, pal, GetColourPalette(rnd_colour)));
		}
	}

	DrawNewGRFTileSeq(ti, &dts, TO_INDUSTRIES, stage, GetColourPalette(rnd_colour));
}

uint16_t GetIndustryTileCallback(CallbackID callback, uint32_t param1, uint32_t param2, IndustryGfx gfx_id, Industry *industry, TileIndex tile, std::span<int32_t> regs100)
{
	assert(industry != nullptr && IsValidTile(tile));
	assert(industry->index == IndustryID::Invalid() || IsTileType(tile, MP_INDUSTRY));

	IndustryTileResolverObject object(gfx_id, tile, industry, callback, param1, param2);
	return object.ResolveCallback(regs100);
}

bool DrawNewIndustryTile(TileInfo *ti, Industry *i, IndustryGfx gfx, const IndustryTileSpec *inds)
{
	if (ti->tileh != SLOPE_FLAT) {
		bool draw_old_one = true;
		if (inds->callback_mask.Test(IndustryTileCallbackMask::DrawFoundations)) {
			/* Called to determine the type (if any) of foundation to draw for industry tile */
			uint32_t callback_res = GetIndustryTileCallback(CBID_INDTILE_DRAW_FOUNDATIONS, 0, 0, gfx, i, ti->tile);
			if (callback_res != CALLBACK_FAILED) draw_old_one = ConvertBooleanCallback(inds->grf_prop.grffile, CBID_INDTILE_DRAW_FOUNDATIONS, callback_res);
		}

		if (draw_old_one) DrawFoundation(ti, FOUNDATION_LEVELED);
	}

	IndustryTileResolverObject object(gfx, ti->tile, i);

	const auto *group = object.Resolve<TileLayoutSpriteGroup>();
	if (group == nullptr) return false;

	/* Limit the building stage to the number of stages supplied. */
	uint8_t stage = GetIndustryConstructionStage(ti->tile);
	auto processor = group->ProcessRegisters(object, &stage);
	auto dts = processor.GetLayout();
	IndustryDrawTileLayout(ti, dts, i->random_colour, stage);
	return true;
}

extern bool IsSlopeRefused(Slope current, Slope refused);

/**
 * Check the slope of a tile of a new industry.
 * @param ind_base_tile Base tile of the industry.
 * @param ind_tile      Tile to check.
 * @param its           Tile specification.
 * @param type          Industry type.
 * @param gfx           Gfx of the tile.
 * @param layout_index  Layout.
 * @param initial_random_bits Random bits of industry after construction
 * @param founder       Industry founder
 * @param creation_type The circumstances the industry is created under.
 * @return Succeeded or failed command.
 */
#ifndef WITH_RUST
CommandCost PerformIndustryTileSlopeCheck(TileIndex ind_base_tile, TileIndex ind_tile, const IndustryTileSpec *its, IndustryType type, IndustryGfx gfx, size_t layout_index, uint16_t initial_random_bits, Owner founder, IndustryAvailabilityCallType creation_type)
{
	Industry ind;
	ind.index = IndustryID::Invalid();
	ind.location.tile = ind_base_tile;
	ind.location.w = 0;
	ind.type = type;
	ind.random = initial_random_bits;
	ind.founder = founder;

	std::array<int32_t, 16> regs100;
	uint16_t callback_res = GetIndustryTileCallback(CBID_INDTILE_SHAPE_CHECK, 0, creation_type << 8 | static_cast<uint32_t>(layout_index), gfx, &ind, ind_tile, regs100);
	if (callback_res == CALLBACK_FAILED) {
		if (!IsSlopeRefused(GetTileSlope(ind_tile), its->slopes_refused)) return CommandCost();
		return CommandCost(STR_ERROR_SITE_UNSUITABLE);
	}
	if (its->grf_prop.grffile->grf_version < 7) {
		if (callback_res != 0) return CommandCost();
		return CommandCost(STR_ERROR_SITE_UNSUITABLE);
	}

	return GetErrorMessageFromLocationCallbackResult(callback_res, regs100, its->grf_prop.grffile, STR_ERROR_SITE_UNSUITABLE);
}
#else
extern "C" OpenTTDIndustryLocationResult openttd_industry_resolve_shape(Industry *industry, uint32_t tile, uint16_t gfx, uint32_t parameter) noexcept
{
	OpenTTDIndustryLocationResult result{};
	result.callback = GetIndustryTileCallback(CBID_INDTILE_SHAPE_CHECK, 0, parameter, gfx, industry, TileIndex(tile), result.registers);
	return result;
}

#endif

/* Simple wrapper for GetHouseCallback to keep the animation unified. */
uint16_t GetSimpleIndustryCallback(CallbackID callback, uint32_t param1, uint32_t param2, const IndustryTileSpec *spec, Industry *ind, TileIndex tile, int)
{
	return GetIndustryTileCallback(callback, param1, param2, spec - GetIndustryTileSpec(0), ind, tile);
}

/** Helper class for animation control. */
#ifdef WITH_RUST
const OpenTTDIndustryAnimation &GetIndustryAnimationBinding() noexcept
{
	static const auto binding = MakeRustAnimationBinding<IndustryTileSpec, Industry, int, GetSimpleIndustryCallback>(
		[](uint32_t t) noexcept -> OpenTTDIndustryAnimationEntry { const auto *s = GetIndustryTileSpec(GetIndustryGfx(TileIndex{t})); if (s == nullptr) return {nullptr, nullptr}; return {s, RustIndustryByTile(t)}; },
		[](const IndustryTileSpec *s) noexcept -> OpenTTDAnimationInfo { return {s->animation.speed, s->animation.frames, s->animation.status == AnimationStatus::Looping, s->callback_mask.Test(IndustryTileCallbackMask::AnimationSpeed), s->callback_mask.Test(IndustryTileCallbackMask::AnimationNextFrame), s->special_flags.Test(IndustryTileSpecialFlag::NextFrameRandomBits), true}; });
	return binding;
}
#else
struct IndustryAnimationBase : public AnimationBase<IndustryAnimationBase, IndustryTileSpec, Industry, int, GetSimpleIndustryCallback, TileAnimationFrameAnimationHelper<Industry> > {
	static constexpr CallbackID cb_animation_speed      = CBID_INDTILE_ANIMATION_SPEED;
	static constexpr CallbackID cb_animation_next_frame = CBID_INDTILE_ANIMATION_NEXT_FRAME;

	static constexpr IndustryTileCallbackMask cbm_animation_speed      = IndustryTileCallbackMask::AnimationSpeed;
	static constexpr IndustryTileCallbackMask cbm_animation_next_frame = IndustryTileCallbackMask::AnimationNextFrame;
};
#endif

void AnimateNewIndustryTile(TileIndex tile)
{
#ifdef WITH_RUST
	openttd_rust_animate_industry(tile.base());
#else
	const IndustryTileSpec *itspec = GetIndustryTileSpec(GetIndustryGfx(tile));
	if (itspec == nullptr) return;

	IndustryAnimationBase::AnimateTile(itspec, Industry::GetByTile(tile), tile, itspec->special_flags.Test(IndustryTileSpecialFlag::NextFrameRandomBits));
#endif
}

#ifndef WITH_RUST
static bool DoTriggerIndustryTileAnimation(TileIndex tile, IndustryAnimationTrigger iat, uint32_t random, uint32_t var18_extra = 0)
{
	const IndustryTileSpec *itspec = GetIndustryTileSpec(GetIndustryGfx(tile));
	if (!itspec->animation.triggers.Test(iat)) return false;

	IndustryAnimationBase::ChangeAnimationFrame(CBID_INDTILE_ANIMATION_TRIGGER, itspec, Industry::GetByTile(tile), tile, random, to_underlying(iat) | var18_extra);
	return true;
}
#endif

bool TriggerIndustryTileAnimation_ConstructionStageChanged(TileIndex tile, bool first_call)
{
#ifdef WITH_RUST
	return openttd_rust_industry_animate_tile(tile.base(), to_underlying(IndustryAnimationTrigger::ConstructionStageChanged), first_call ? 0x100 : 0, &GetIndustryRustServices(), &GetIndustryLifecycleRustServices());
#else
	auto iat = IndustryAnimationTrigger::ConstructionStageChanged;
	return DoTriggerIndustryTileAnimation(tile, iat, Random(), first_call ? 0x100 : 0);
#endif
}

bool TriggerIndustryTileAnimation(TileIndex tile, IndustryAnimationTrigger iat)
{
#ifdef WITH_RUST
	assert(iat != IndustryAnimationTrigger::ConstructionStageChanged);
	return openttd_rust_industry_animate_tile(tile.base(), to_underlying(iat), 0, &GetIndustryRustServices(), &GetIndustryLifecycleRustServices());
#else
	assert(iat != IndustryAnimationTrigger::ConstructionStageChanged);
	return DoTriggerIndustryTileAnimation(tile, iat, Random());
#endif
}

bool TriggerIndustryAnimation(const Industry *ind, IndustryAnimationTrigger iat)
{
#ifdef WITH_RUST
	return openttd_rust_industry_animate(ind->production_owner.get(), ind->index.base(), to_underlying(iat), Map::SizeX(), &GetIndustryRustServices(), &GetIndustryLifecycleRustServices());
#else
	bool ret = true;
	uint32_t random = Random();
	for (TileIndex tile : ind->location) {
		if (ind->TileBelongsToIndustry(tile)) {
			if (DoTriggerIndustryTileAnimation(tile, iat, random)) {
				SB(random, 0, 16, Random());
			} else {
				ret = false;
			}
		}
	}

	return ret;
#endif
}

/**
 * Trigger random triggers for an industry tile and reseed its random bits.
 * @param tile Industry tile to trigger.
 * @param trigger Trigger to trigger.
 * @param ind Industry of the tile.
 * @param[in,out] reseed_industry Collects bits to reseed for the industry.
 */
#ifndef WITH_RUST
static void DoTriggerIndustryTileRandomisation(TileIndex tile, IndustryRandomTrigger trigger, Industry *ind, uint32_t &reseed_industry)
{
	assert(IsValidTile(tile) && IsTileType(tile, MP_INDUSTRY));

	IndustryGfx gfx = GetIndustryGfx(tile);
	const IndustryTileSpec *itspec = GetIndustryTileSpec(gfx);

	if (!itspec->grf_prop.HasSpriteGroups()) return;

	IndustryTileResolverObject object(gfx, tile, ind, CBID_RANDOM_TRIGGER);
	auto waiting_random_triggers = GetIndustryRandomTriggers(tile);
	waiting_random_triggers.Set(trigger);
	SetIndustryRandomTriggers(tile, waiting_random_triggers); // store now for var 5F
	object.SetWaitingRandomTriggers(waiting_random_triggers);

	object.ResolveRerandomisation();

	/* Store remaining triggers. */
	waiting_random_triggers.Reset(object.GetUsedRandomTriggers());
	SetIndustryRandomTriggers(tile, waiting_random_triggers);

	/* Rerandomise tile bits */
	uint8_t new_random_bits = Random();
	uint8_t random_bits = GetIndustryRandomBits(tile);
	random_bits &= ~object.reseed[VSG_SCOPE_SELF];
	random_bits |= new_random_bits & object.reseed[VSG_SCOPE_SELF];
	SetIndustryRandomBits(tile, random_bits);
	MarkTileDirtyByTile(tile);

	reseed_industry |= object.reseed[VSG_SCOPE_PARENT];
}
#endif

/**
 * Reseeds the random bits of an industry.
 * @param ind Industry.
 * @param reseed Bits to reseed.
 */
#ifndef WITH_RUST
static void DoReseedIndustry(Industry *ind, uint32_t reseed)
{
	if (reseed == 0 || ind == nullptr) return;

	uint16_t random_bits = Random();
	ind->random &= reseed;
	ind->random |= random_bits & reseed;
}
#endif

/**
 * Trigger a random trigger for a single industry tile.
 * @param tile Industry tile to trigger.
 * @param trigger Trigger to trigger.
 */
void TriggerIndustryTileRandomisation(TileIndex tile, IndustryRandomTrigger trigger)
{
#ifdef WITH_RUST
	Industry *ind = Industry::GetByTile(tile);
	openttd_rust_industry_randomise_tile(tile.base(), to_underlying(trigger), ind, ind->production_owner.get(), &GetIndustryRustServices(), &GetIndustryLifecycleRustServices());
#else
	uint32_t reseed_industry = 0;
	Industry *ind = Industry::GetByTile(tile);
	DoTriggerIndustryTileRandomisation(tile, trigger, ind, reseed_industry);
	DoReseedIndustry(ind, reseed_industry);
#endif
}

/**
 * Trigger a random trigger for all industry tiles.
 * @param ind Industry to trigger.
 * @param trigger Trigger to trigger.
 */
void TriggerIndustryRandomisation(Industry *ind, IndustryRandomTrigger trigger)
{
#ifdef WITH_RUST
	openttd_rust_industry_randomise(ind, ind->production_owner.get(), ind->index.base(), to_underlying(trigger), Map::SizeX(), &GetIndustryRustServices(), &GetIndustryLifecycleRustServices());
#else
	uint32_t reseed_industry = 0;
	for (TileIndex tile : ind->location) {
		if (ind->TileBelongsToIndustry(tile)) {
			DoTriggerIndustryTileRandomisation(tile, trigger, ind, reseed_industry);
		}
	}
	DoReseedIndustry(ind, reseed_industry);
#endif
}


#ifdef WITH_RUST
OpenTTDIndustryRandomisationSpec IndustryRandomisationSpec(uint32_t index) noexcept
{
	TileIndex tile(index);
	assert(IsValidTile(tile) && IsTileType(tile, MP_INDUSTRY));
	IndustryGfx gfx = GetIndustryGfx(tile);
	return {gfx, GetIndustryTileSpec(gfx)->grf_prop.HasSpriteGroups()};
}
OpenTTDIndustryReseed IndustryResolveReseed(uint32_t tile, uint16_t gfx, void *industry, uint8_t waiting) noexcept
{
	IndustryTileResolverObject object(gfx, TileIndex(tile), static_cast<Industry *>(industry), CBID_RANDOM_TRIGGER);
	object.SetWaitingRandomTriggers(IndustryRandomTriggers(waiting));
	object.ResolveRerandomisation();
	return {object.reseed[VSG_SCOPE_SELF], object.reseed[VSG_SCOPE_PARENT], object.GetUsedRandomTriggers().base()};
}
OpenTTDIndustryAnimationSpec IndustryAnimationSpec(uint32_t tile) noexcept
{
	const IndustryTileSpec *spec = GetIndustryTileSpec(GetIndustryGfx(TileIndex(tile)));
	return {spec, spec->animation.triggers.base()};
}
#endif

#ifdef WITH_RUST
IndustryTileScopeResolver::IndustryTileScopeResolver(ResolverObject &ro, Industry *industry, TileIndex tile)
	: ScopeResolver(ro), context{industry == nullptr ? nullptr : industry->production_owner.get(), industry, industry == nullptr ? nullptr : industry->town, tile.base(), 0, industry == nullptr ? uint16_t{0xFFFF} : industry->index.base(), 0}
{
}
ScopeResolver *IndustryTileResolverObject::GetScope(VarSpriteGroupScope scope, uint8_t relative)
{
	return openttd_rust_industry_tile_scope(this, &this->indtile_scope, &this->ind_scope, scope, relative, &GetIndustryResolverServices());
}
#endif
