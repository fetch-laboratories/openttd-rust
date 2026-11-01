/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_airport.cpp NewGRF handling of airports. */

#include "stdafx.h"
#include "debug.h"
#include "timer/timer_game_calendar.h"
#include "newgrf_badge.h"
#include "newgrf_spritegroup.h"
#include "newgrf_text.h"
#include "station_base.h"
#include "town.h"

#include "table/strings.h"

#include "newgrf_class_func.h"

#include "rust/services_station.h"
#include "rust/tile_animation_services.h"
#include "rust/settings_services.h"
#include "rust/engine_services.h"
#include "rust/services_assert.h"
#include "rust/airport_resolver_ffi.h"

#include "safeguards.h"

static uint16_t RustEvaluateAirportTile(uint16_t callback, uint32_t p1, uint32_t p2, const AirportTileSpec *spec, Station *station, uint32_t tile, int32_t *registers, size_t count) noexcept
{
	/* Native shells and optional scopes live for this synchronous invocation.
	 * Common evaluation and feature operations call their canonical Rust owners. */
	AirportTileResolverObject object(spec, TileIndex{tile}, station, static_cast<CallbackID>(callback), p1, p2);
	return openttd_rust_resolver_callback(&GetRustSpriteGroupServices(), object.root_spritegroup, &object, object.CommonState(), object.grffile, registers, count);
}
const OpenTTDAirportResolverServices &GetAirportResolverServices() noexcept
{
	static const OpenTTDAirportResolverServices services{
		.shared = &GetRustSharedServices(),
		.storage = &GetRustStorageServices(),
		.vehicles = &GetRustVehicleServices(),
		.by_tile = RustStationByTile,
		.station_id = RustBaseStationID<const BaseStation>,
		.facilities = RustStationFacilities,
		.get_station = RustStationGet<Station *>,
		.check = RustAssertCondition,
		.engine_if_valid = RustEngineByIDIfValid,
		.engine_id = RustEngineID,
		.engine_kind = RustEngineType,
		.engine_subtype = GetRustEngineCapabilitiesServices().aircraft_subtype,
		.fta_flags = RustAirportFlags,
		.has_hangar = RustSelectedAirportHasHangar,
		.hangar_tile = RustSelectedHangarTile,
		.depot_count = [](const AirportSpec *spec) noexcept { return spec->depots.size(); },
		.depot_offset = [](const AirportSpec *spec, size_t index) noexcept { return spec->depots[index].ti; },
		.rotated_offset = [](const BaseStation *st, TileIndexDiffC offset, const AirportSpec *spec) noexcept { return Station::From(st)->airport.GetRotatedTileFromOffset(offset, spec).base(); },
		.fta_heading = RustAirportLayoutHeading,
		.fta_position = [](const AirportFTAClass *fta, uint32_t index) noexcept { return fta->layout[index].position; },
		.dummy_spec = &AirportSpec::dummy,
		.dummy_fta = RustDummyAirportFTA(),
		.fta = RustAirportSpecFTA,
		.catchment = [](const AirportSpec *spec) noexcept { return spec->catchment; },
		.base_spec = RustAirportSpec<const AirportSpec *>,
		.enabled = RustSpecEnabled<AirportSpec>,
		.min_year = [](const AirportSpec *spec) noexcept { return spec->min_year.base(); },
		.max_year = [](const AirportSpec *spec) noexcept { return spec->max_year.base(); },
		.override_id = RustSpecOverride<AirportSpec>,
		.never_expire = RustNeverExpireAirports,
		.station_layout = RustStationAirportLayout,
		.random = RustStationRandomBits<BaseStation>,
		.build_date = RustStationBuildDate<BaseStation>,
		.root = RustSpecSpriteGroup<AirportSpec>,
		.tile_root = RustSpecSpriteGroup<AirportTileSpec>,
		.badges = RustSpecBadges<AirportSpec>,
		.tile_badges = RustSpecBadges<AirportTileSpec>,
		.tile_spec = RustAirportTileSpec<const AirportTileSpec *>,
		.tile_override = RustSpecOverride<AirportTileSpec>,
		.tile_local = RustSpecLocalID<AirportTileSpec>,
		.tile_grfid = RustSpecGRFID<AirportTileSpec>,
		.tile_has_groups = RustSpecHasSpriteGroups<AirportTileSpec>,
		.tile_substitute = RustSpecSubstitute<AirportTileSpec>,
		.belongs = RustStationAirportMember,
		.tile_random = RustStationTileRandomBits,
		.frame = OpenTTDAnimationFrame,
		.version = RustResolverGRFVersion,
		.grfid = RustResolverGRFFileID,
		.base_scope = RustResolverBaseScope,
		.town_scope = RustResolverTownScope<AirportResolverObject>,
		.evaluate_tile = RustEvaluateAirportTile,
		.unhandled = [](uint8_t var) noexcept { Debug(grf, 1, "Unhandled airport tile variable 0x{:X}", var); },
	};
	return services;
}

/**
 * Reset airport classes to their default state.
 * This includes initialising the defaults classes with an empty
 * entry, for standard airports.
 */
template <>
std::span<const OpenTTDGRFClassDefault> AirportClass::Defaults()
{
	static const OpenTTDGRFClassDefault defaults[]{
		{'SMAL', STR_AIRPORT_CLASS_SMALL},
		{'LARG', STR_AIRPORT_CLASS_LARGE},
		{'HUB_', STR_AIRPORT_CLASS_HUB},
		{'HELI', STR_AIRPORT_CLASS_HELIPORTS},
	};
	return defaults;
}


/* Instantiate AirportClass. */
template class NewGRFClass<AirportSpec, AirportClassID, APC_MAX>;


AirportOverrideManager _airport_mngr(NEW_AIRPORT_OFFSET, NUM_AIRPORTS, AT_INVALID);

AirportSpec AirportSpec::specs[NUM_AIRPORTS]; ///< Airport specifications.
AirportSpec *AirportSpec::RustRawSpec(uint16_t type) noexcept { return &AirportSpec::specs[type]; }
static_assert(NUM_AIRPORTS == 128 && NEW_AIRPORT_OFFSET == 10 && AT_INVALID == 254 && INVALID_AIRPORTTILE == 256 && NEW_AIRPORTTILE_OFFSET == 74);

/**
 * Retrieve airport spec for the given airport. If an override is available
 *  it is returned.
 * @param type index of airport
 * @return A pointer to the corresponding AirportSpec
 */
/* static */ const AirportSpec *AirportSpec::Get(uint8_t type)
{
	return openttd_rust_airport_spec(type, &GetAirportResolverServices());
}

/**
 * Retrieve airport spec for the given airport. Even if an override is
 *  available the base spec is returned.
 * @param type index of airport
 * @return A pointer to the corresponding AirportSpec
 */

/** Check whether this airport is available to build. */
bool AirportSpec::IsAvailable() const
{
	return openttd_rust_airport_available(this, &GetAirportResolverServices());
}

/**
 * Check if the airport would be within the map bounds at the given tile.
 * @param table Selected layout table. This affects airport rotation, and therefore dimensions.
 * @param tile Top corner of the airport.
 * @return true iff the airport would be within the map bounds at the given tile.
 */
bool AirportSpec::IsWithinMapBounds(uint8_t table, TileIndex tile) const
{
	return openttd_rust_airport_in_bounds(this, table, tile.base(), &GetAirportResolverServices());
}

/**
 * This function initializes the airportspec array.
 */

/**
 * Tie all airportspecs to their class.
 */



/* virtual */ uint32_t AirportScopeResolver::GetVariable(uint8_t variable, [[maybe_unused]] uint32_t parameter, bool &available) const
{
	return openttd_rust_airport_variable(&this->context, &this->ro, this->ro.grffile, variable, parameter, &available, &GetAirportResolverServices());
}

GrfSpecFeature AirportResolverObject::GetFeature() const
{
	return GSF_AIRPORTS;
}

uint32_t AirportResolverObject::GetDebugID() const
{
	return RustSpecLocalID(this->airport_scope.context.spec);
}

/* virtual */ uint32_t AirportScopeResolver::GetRandomBits() const
{
	return openttd_rust_airport_random(&this->context, false, &GetAirportResolverServices());
}

/**
 * Store a value into the object's persistent storage.
 * @param pos Position in the persistent storage to use.
 * @param value Value to store.
 */
/* virtual */ void AirportScopeResolver::StorePSA(uint pos, int32_t value)
{
	openttd_rust_storage_airport(&GetRustStorageServices(), this->context.station, this->ro.grffile, pos, value);
}

/**
 * Get the town scope associated with a station, if it exists.
 * On the first call, the town scope is created (if possible).
 * @return Town scope, if available.
 */
TownScopeResolver *AirportResolverObject::GetTown()
{
	return openttd_rust_airport_town(&this->airport_scope.context, this, &GetAirportResolverServices());
}

/**
 * Constructor of the airport resolver.
 * @param tile %Tile for the callback, only valid for airporttile callbacks.
 * @param st %Station of the airport for which the callback is run, or \c nullptr for build gui.
 * @param spec AirportSpec for which the callback is run.
 * @param layout Layout of the airport to build.
 * @param callback Callback ID.
 * @param param1 First parameter (var 10) of the callback.
 * @param param2 Second parameter (var 18) of the callback.
 */
AirportResolverObject::AirportResolverObject(TileIndex tile, Station *st, const AirportSpec *spec, uint8_t layout,
		CallbackID callback, uint32_t param1, uint32_t param2)
	: ResolverObject(
		RustSpecGRFFile(spec),
		callback, param1, param2), airport_scope(*this, tile, st, spec, layout)
{
	this->root_spritegroup = openttd_rust_airport_root(&this->airport_scope.context, &GetAirportResolverServices());
}

SpriteID GetCustomAirportSprite(const AirportSpec *as, uint8_t layout)
{
	AirportResolverObject object(INVALID_TILE, nullptr, as, layout);
	const auto *group = object.Resolve<ResultSpriteGroup>();
	if (group == nullptr || group->num_sprites == 0) return as->preview_sprite;

	return group->sprite;
}


/**
 * Get a custom text for the airport.
 * @param as The airport type's specification.
 * @param layout The layout index.
 * @param callback The callback to call.
 * @return The custom text.
 */
StringID GetAirportTextCallback(const AirportSpec *as, uint8_t layout, uint16_t callback)
{
	AirportResolverObject object(INVALID_TILE, nullptr, as, layout, (CallbackID)callback);
	std::array<int32_t, 1> regs100;
	uint16_t cb_res = object.ResolveCallback(regs100);
	if (cb_res == CALLBACK_FAILED || cb_res == 0x400) return STR_UNDEFINED;
	if (cb_res == 0x40F) {
		return GetGRFStringID(as->grf_prop.grfid, static_cast<GRFStringID>(regs100[0]));
	}
	if (cb_res > 0x400) {
		ErrorUnknownCallbackResult(as->grf_prop.grfid, callback, cb_res);
		return STR_UNDEFINED;
	}

	return GetGRFStringID(as->grf_prop.grfid, GRFSTR_MISC_GRF_TEXT + cb_res);
}
