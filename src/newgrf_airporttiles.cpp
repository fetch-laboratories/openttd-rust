/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_airporttiles.cpp NewGRF handling of airport tiles. */

#include "stdafx.h"
#include "debug.h"
#include "newgrf_airporttiles.h"
#include "newgrf_badge.h"
#include "newgrf_spritegroup.h"
#include "newgrf_sound.h"
#include "station_base.h"
#include "water.h"
#include "landscape.h"
#include "company_base.h"
#include "town.h"
#include "newgrf_animation_base.h"
#include "rust/animation_binding.hpp"
#include "rust/services_station.h"

#include "table/strings.h"
#include "table/airporttiles.h"

#include "rust/airport_definitions_ffi.h"

#include "safeguards.h"


AirportTileSpec AirportTileSpec::tiles[NUM_AIRPORTTILES];
void RustAirportTileDefault(uint16_t id) noexcept { *const_cast<AirportTileSpec *>(AirportTileSpec::Get(id)) = _origin_airporttile_specs[id]; }

AirportTileOverrideManager _airporttile_mngr(NEW_AIRPORTTILE_OFFSET, NUM_AIRPORTTILES, INVALID_AIRPORTTILE);

/**
 * Retrieve airport tile spec for the given airport tile
 * @param gfx index of airport tile
 * @return A pointer to the corresponding AirportTileSpec
 */
/* static */ const AirportTileSpec *AirportTileSpec::Get(StationGfx gfx)
{
	/* should be assert(gfx < lengthof(tiles)), but that gives compiler warnings
	 * since it's always true if the following holds: */
	static_assert(std::numeric_limits<StationGfx>::max() + 1 == lengthof(tiles));
	return &AirportTileSpec::tiles[gfx];
}

/**
 * Retrieve airport tile spec for the given airport tile.
 * @param tile The airport tile.
 * @return A pointer to the corresponding AirportTileSpec.
 */
/* static */ const AirportTileSpec *AirportTileSpec::GetByTile(TileIndex tile)
{
	return AirportTileSpec::Get(GetAirportGfx(tile));
}

/**
 * This function initializes the tile array of AirportTileSpec
 */


/**
 * Do airporttile gfx ID translation for NewGRFs.
 * @param gfx the type to get the override for.
 * @return the gfx to actually work with.
 */
StationGfx GetTranslatedAirportTileID(StationGfx gfx)
{
	return openttd_rust_airport_translate_tile(gfx, &GetAirportResolverServices());
}

/**
 * Based on newhouses/newindustries equivalent, but adapted for airports.
 * @param parameter from callback. It's in fact a pair of coordinates
 * @param tile TileIndex from which the callback was initiated
 * @param index of the industry been queried for
 * @param grf_version8 True, if we are dealing with a new NewGRF which uses GRF version >= 8.
 * @return a construction of bits obeying the newgrf format
 */

/* virtual */ uint32_t AirportTileScopeResolver::GetVariable(uint8_t variable, [[maybe_unused]] uint32_t parameter, bool &available) const
{
	return openttd_rust_airport_tile_variable(&this->context, &this->ro, variable, parameter, &available, &GetAirportResolverServices());
}

/* virtual */ uint32_t AirportTileScopeResolver::GetRandomBits() const
{
	return openttd_rust_airport_random(&this->context, true, &GetAirportResolverServices());
}

/**
 * Constructor of the resolver for airport tiles.
 * @param ats Specification of the airport tiles.
 * @param tile %Tile for the callback, only valid for airporttile callbacks.
 * @param st Station of the airport for which the callback is run, or \c nullptr for build gui.
 * @param callback Callback ID.
 * @param callback_param1 First parameter (var 10) of the callback.
 * @param callback_param2 Second parameter (var 18) of the callback.
 */
AirportTileResolverObject::AirportTileResolverObject(const AirportTileSpec *ats, TileIndex tile, Station *st,
		CallbackID callback, uint32_t callback_param1, uint32_t callback_param2)
	: ResolverObject(
		RustSpecGRFFile(ats),
		callback, callback_param1, callback_param2),
		tiles_scope(*this, ats, tile, st),
		airport_scope(*this, tile, st, nullptr, 0)
{
	/* The parent scope initializes last; publish the tile invocation root. */
	this->root_spritegroup = openttd_rust_airport_tile_init(&this->tiles_scope.context, &this->airport_scope.context, &GetAirportResolverServices());
}

GrfSpecFeature AirportTileResolverObject::GetFeature() const
{
	return GSF_AIRPORTTILES;
}

uint32_t AirportTileResolverObject::GetDebugID() const
{
	return RustSpecLocalID(this->tiles_scope.context.tile_spec);
}

static uint16_t GetAirportTileCallback(CallbackID callback, uint32_t param1, uint32_t param2, const AirportTileSpec *ats, Station *st, TileIndex tile, std::span<int32_t> regs100 = {})
{
	AirportTileResolverObject object(ats, tile, st, callback, param1, param2);
	return object.ResolveCallback(regs100);
}

static void AirportDrawTileLayout(const TileInfo *ti, const DrawTileSpriteSpan &dts, Colours colour)
{
	SpriteID image = dts.ground.sprite;
	SpriteID pal = dts.ground.pal;

	if (GB(image, 0, SPRITE_WIDTH) != 0) {
		if (image == SPR_FLAT_WATER_TILE && IsTileOnWater(ti->tile)) {
			DrawWaterClassGround(ti);
		} else {
			DrawGroundSprite(image, GroundSpritePaletteTransform(image, pal, GetColourPalette(colour)));
		}
	}

	DrawNewGRFTileSeq(ti, &dts, TO_BUILDINGS, 0, GetColourPalette(colour));
}

bool DrawNewAirportTile(TileInfo *ti, Station *st, const AirportTileSpec *airts)
{
	if (ti->tileh != SLOPE_FLAT) {
		bool draw_old_one = true;
		if (airts->callback_mask.Test(AirportTileCallbackMask::DrawFoundations)) {
			/* Called to determine the type (if any) of foundation to draw */
			uint32_t callback_res = GetAirportTileCallback(CBID_AIRPTILE_DRAW_FOUNDATIONS, 0, 0, airts, st, ti->tile);
			if (callback_res != CALLBACK_FAILED) draw_old_one = ConvertBooleanCallback(airts->grf_prop.grffile, CBID_AIRPTILE_DRAW_FOUNDATIONS, callback_res);
		}

		if (draw_old_one) DrawFoundation(ti, FOUNDATION_LEVELED);
	}

	AirportTileResolverObject object(airts, ti->tile, st);
	const auto *group = object.Resolve<TileLayoutSpriteGroup>();
	if (group == nullptr) {
		return false;
	}

	auto processor = group->ProcessRegisters(object, nullptr);
	auto dts = processor.GetLayout();
	AirportDrawTileLayout(ti, dts, Company::Get(st->owner)->colour);
	return true;
}


/** Helper class for animation control. */
const OpenTTDAirportAnimation &GetAirportAnimationBinding() noexcept
{
	static const OpenTTDAirportAnimation binding{
		.info = [](const AirportTileSpec *s) noexcept -> OpenTTDAnimationInfo { return {s->animation.speed, s->animation.frames, s->animation.status == AnimationStatus::Looping, s->callback_mask.Test(AirportTileCallbackMask::AnimationSpeed), s->callback_mask.Test(AirportTileCallbackMask::AnimationNextFrame), HasBit(s->animation_special_flags, 0), true}; },
		.sound = [](const AirportTileSpec *s, uint8_t sound, uint32_t t) noexcept { PlayTileSound(RustSpecGRFFile(s), sound, TileIndex{t}); },
		.version8 = RustSpecGRFVersion8<AirportTileSpec>,
		.speed_error = RustSpecCallbackError<AirportTileSpec, AirportTileSpec>,
	};
	return binding;
}

void AnimateAirportTile(TileIndex tile)
{
	openttd_rust_animate_airport(tile.base());
}


bool TriggerAirportTileAnimation(Station *st, TileIndex tile, AirportAnimationTrigger trigger)
{
	return openttd_rust_trigger_airport_tile(st, tile.base(), to_underlying(trigger));
}

bool TriggerAirportAnimation(Station *st, AirportAnimationTrigger trigger, CargoType cargo_type)
{
	return openttd_rust_trigger_airport(st, to_underlying(trigger), cargo_type);
}
