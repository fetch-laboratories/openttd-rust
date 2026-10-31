/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_house.h Functions related to NewGRF houses. */

#ifndef NEWGRF_HOUSE_H
#define NEWGRF_HOUSE_H

#include "newgrf_callbacks.h"
#include "tile_cmd.h"
#include "house_type.h"
#include "newgrf_spritegroup.h"
#include "newgrf_town.h"

/** Scope resolver for houses. */
struct HouseScopeResolver : public ScopeResolver {
	OpenTTDHouseScope context;

	/**
	 * Constructor of a house scope resolver.
	 * @param ro Surrounding resolver.
	 * @param house_id House type being queried.
	 * @param tile %Tile containing the house.
	 * @param town %Town containing the house.
	 * @param not_yet_constructed House is still under construction.
	 * @param initial_random_bits Random bits during construction checks.
	 * @param watched_cargo_triggers Cargo types that triggered the watched cargo callback.
	 */
	HouseScopeResolver(ResolverObject &ro, HouseID house_id, TileIndex tile, Town *town,
			bool not_yet_constructed, uint8_t initial_random_bits, CargoTypes watched_cargo_triggers, int view);

	uint32_t GetRandomBits() const override;
	uint32_t GetVariable(uint8_t variable, [[maybe_unused]] uint32_t parameter, bool &available) const override;
	uint32_t GetRandomTriggers() const override;
};

/** Resolver object to be used for houses (feature 07 spritegroups). */
struct HouseResolverObject : public SpecializedResolverObject<HouseRandomTriggers> {
	HouseScopeResolver house_scope;
	TownScopeResolver  town_scope;

	HouseResolverObject(HouseID house_id, TileIndex tile, Town *town,
			CallbackID callback = CBID_NO_CALLBACK, uint32_t param1 = 0, uint32_t param2 = 0,
			bool not_yet_constructed = false, uint8_t initial_random_bits = 0, CargoTypes watched_cargo_triggers = 0, int view = 0);

	ScopeResolver *GetScope(VarSpriteGroupScope scope = VSG_SCOPE_SELF, uint8_t relative = 0) override
	{
		return openttd_rust_house_scope(this, &this->house_scope, &this->town_scope, scope, relative);
	}

	GrfSpecFeature GetFeature() const override;
	uint32_t GetDebugID() const override;
};

/**
 * Makes class IDs unique to each GRF file.
 * Houses can be assigned class IDs which are only comparable within the GRF
 * file they were defined in. This mapping ensures that if two houses have the
 * same class as defined by the GRF file, the classes are different within the
 * game. An array of HouseClassMapping structs is created, and the array index
 * of the struct that matches both the GRF ID and the class ID is the class ID
 * used in the game.
 *
 * Although similar to the HouseIDMapping struct above, this serves a different
 * purpose. Since the class ID is not saved anywhere, this mapping does not
 * need to be persistent; it just needs to keep class ids unique.
 */
std::span<const uint> GetBuildingHouseIDCounts();

void DrawNewHouseTile(TileInfo *ti, HouseID house_id);
void DrawNewHouseTileInGUI(int x, int y, const HouseSpec *spec, HouseID house_id, int view);
void AnimateNewHouseTile(TileIndex tile);
/* see also: void TriggerHouseAnimation_TileLoop(TileIndex tile, uint16_t random_bits) */

uint16_t GetHouseCallback(CallbackID callback, uint32_t param1, uint32_t param2, HouseID house_id, Town *town, TileIndex tile, std::span<int32_t> regs100 = {},
		bool not_yet_constructed = false, uint8_t initial_random_bits = 0, CargoTypes watched_cargo_triggers = 0, int view = 0);


#endif /* NEWGRF_HOUSE_H */
