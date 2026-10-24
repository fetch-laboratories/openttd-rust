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

#ifdef WITH_RUST
#include "rust/services_station.h"
#include "rust/tile_animation_services.h"
#include "rust/settings_services.h"
#include "rust/engine_services.h"
#include "rust/services_assert.h"
#include "rust/airport_resolver_ffi.h"
#endif
#include "safeguards.h"

#ifdef WITH_RUST
static uint16_t RustEvaluateAirportTile(uint16_t callback, uint32_t p1, uint32_t p2, const AirportTileSpec *spec, Station *station, uint32_t tile, int32_t *registers, size_t count) noexcept
{
	/* The common evaluator owns the native virtual frames until its own port
	 * integrates. Feature policy and context initialization stay in Rust. */
	AirportTileResolverObject object(spec, TileIndex{tile}, station, static_cast<CallbackID>(callback), p1, p2);
	return object.ResolveCallback({registers, count});
}
const OpenTTDAirportResolverServices &GetAirportResolverServices() noexcept
{
	static const OpenTTDAirportResolverServices services{
		.shared = &GetRustSharedServices(),
		.storage = &GetRustStorageServices(),
		.vehicles = &GetRustVehicleServices(),
		.get_station = RustStationGet<Station *>,
		.check = RustAssertCondition,
		.engine_if_valid = RustEngineByIDIfValid,
		.engine_id = RustEngineID,
		.engine_kind = RustEngineType,
		.engine_subtype = RustEngineAircraftSubtype,
		.primary_road_stop = RustStationPrimaryRoadStop,
		.fta_flags = RustAirportFlags,
		.has_hangar = RustSelectedAirportHasHangar,
		.hangar_tile = RustSelectedHangarTile,
		.depot_count = [](const AirportSpec *spec) noexcept { return spec->depots.size(); },
		.depot_offset = [](const AirportSpec *spec, size_t index) noexcept { return spec->depots[index].ti; },
		.rotated_offset = [](const BaseStation *st, TileIndexDiffC offset, const AirportSpec *spec) noexcept { return Station::From(st)->airport.GetRotatedTileFromOffset(offset, spec).base(); },
		.fta_heading = [](const AirportFTAClass *fta, uint32_t index) noexcept { return fta->layout[index].heading; },
		.fta_position = [](const AirportFTAClass *fta, uint32_t index) noexcept { return fta->layout[index].position; },
		.dummy_spec = &AirportSpec::dummy,
		.dummy_fta = RustDummyAirportFTA(),
		.fta = RustAirportSpecFTA,
		.base_spec = [](uint8_t type) noexcept -> const AirportSpec * { return AirportSpec::GetWithoutOverride(type); },
		.enabled = [](const AirportSpec *spec) noexcept { return spec->enabled; },
		.min_year = [](const AirportSpec *spec) noexcept { return spec->min_year.base(); },
		.max_year = [](const AirportSpec *spec) noexcept { return spec->max_year.base(); },
		.override_id = RustSpecOverride<AirportSpec>,
		.manager_grfid = [](uint8_t type) noexcept { return _airport_mngr.GetGRFID(type); },
		.substitute = [](uint8_t type) noexcept { return _airport_mngr.GetSubstituteID(type); },
		.never_expire = RustNeverExpireAirports,
		.station_layout = RustStationAirportLayout,
		.random = RustStationRandomBits,
		.build_date = RustStationBuildDate,
		.root = RustSpecSpriteGroup<AirportSpec>,
		.tile_root = RustSpecSpriteGroup<AirportTileSpec>,
		.badges = RustSpecBadges<AirportSpec>,
		.tile_badges = RustSpecBadges<AirportTileSpec>,
		.tile_spec = [](uint8_t gfx) noexcept { return AirportTileSpec::Get(gfx); },
		.tile_override = RustSpecOverride<AirportTileSpec>,
		.tile_local = RustSpecLocalID<AirportTileSpec>,
		.tile_grfid = RustSpecGRFID<AirportTileSpec>,
		.tile_has_groups = RustSpecHasSpriteGroups<AirportTileSpec>,
		.tile_substitute = RustSpecSubstitute<AirportTileSpec>,
		.belongs = RustStationAirportMember,
		.tile_random = RustStationTileRandomBits,
		.nearby = RustSignedNearbyTile,
		.nearby_info = RustNearbyTileInformation,
		.terrain = RustTerrainType,
		.frame = OpenTTDAnimationFrame,
		.version = RustResolverGRFVersion,
		.grfid = RustResolverGRFFileID,
		.base_scope = RustResolverBaseScope,
		.town_scope = [](AirportResolverObject *object, Town *town, bool readonly) noexcept -> TownScopeResolver * { if (!object->town_scope.has_value()) object->town_scope.emplace(*object, town, readonly); return &*object->town_scope; },
		.evaluate_tile = RustEvaluateAirportTile,
		.unhandled = [](uint8_t var) noexcept { Debug(grf, 1, "Unhandled airport tile variable 0x{:X}", var); },
	};
	return services;
}
#endif

/**
 * Reset airport classes to their default state.
 * This includes initialising the defaults classes with an empty
 * entry, for standard airports.
 */
template <>
/* static */ void AirportClass::InsertDefaults()
{
	AirportClass::Get(AirportClass::Allocate('SMAL'))->name = STR_AIRPORT_CLASS_SMALL;
	AirportClass::Get(AirportClass::Allocate('LARG'))->name = STR_AIRPORT_CLASS_LARGE;
	AirportClass::Get(AirportClass::Allocate('HUB_'))->name = STR_AIRPORT_CLASS_HUB;
	AirportClass::Get(AirportClass::Allocate('HELI'))->name = STR_AIRPORT_CLASS_HELIPORTS;
}

template <>
bool AirportClass::IsUIAvailable(uint) const
{
	return true;
}

/* Instantiate AirportClass. */
template class NewGRFClass<AirportSpec, AirportClassID, APC_MAX>;


AirportOverrideManager _airport_mngr(NEW_AIRPORT_OFFSET, NUM_AIRPORTS, AT_INVALID);

AirportSpec AirportSpec::specs[NUM_AIRPORTS]; ///< Airport specifications.

/**
 * Retrieve airport spec for the given airport. If an override is available
 *  it is returned.
 * @param type index of airport
 * @return A pointer to the corresponding AirportSpec
 */
/* static */ const AirportSpec *AirportSpec::Get(uint8_t type)
{
#ifdef WITH_RUST
	return openttd_rust_airport_spec(type, &GetAirportResolverServices());
#else
	assert(type < lengthof(AirportSpec::specs));
	const AirportSpec *as = &AirportSpec::specs[type];
	if (type >= NEW_AIRPORT_OFFSET && !as->enabled) {
		if (_airport_mngr.GetGRFID(type) == 0) return as;
		uint8_t subst_id = _airport_mngr.GetSubstituteID(type);
		if (subst_id == AT_INVALID) return as;
		as = &AirportSpec::specs[subst_id];
	}
	if (as->grf_prop.override_id != AT_INVALID) return &AirportSpec::specs[as->grf_prop.override_id];
	return as;
#endif
}

/**
 * Retrieve airport spec for the given airport. Even if an override is
 *  available the base spec is returned.
 * @param type index of airport
 * @return A pointer to the corresponding AirportSpec
 */
/* static */ AirportSpec *AirportSpec::GetWithoutOverride(uint8_t type)
{
	assert(type < lengthof(AirportSpec::specs));
	return &AirportSpec::specs[type];
}

/** Check whether this airport is available to build. */
bool AirportSpec::IsAvailable() const
{
#ifdef WITH_RUST
	return openttd_rust_airport_available(this, &GetAirportResolverServices());
#else
	if (!this->enabled) return false;
	if (TimerGameCalendar::year < this->min_year) return false;
	if (_settings_game.station.never_expire_airports) return true;
	return TimerGameCalendar::year <= this->max_year;
#endif
}

/**
 * Check if the airport would be within the map bounds at the given tile.
 * @param table Selected layout table. This affects airport rotation, and therefore dimensions.
 * @param tile Top corner of the airport.
 * @return true iff the airport would be within the map bounds at the given tile.
 */
bool AirportSpec::IsWithinMapBounds(uint8_t table, TileIndex tile) const
{
#ifdef WITH_RUST
	return openttd_rust_airport_in_bounds(this, table, tile.base(), &GetAirportResolverServices());
#else
	if (table >= this->layouts.size()) return false;

	uint8_t w = this->size_x;
	uint8_t h = this->size_y;
	if (this->layouts[table].rotation == DIR_E || this->layouts[table].rotation == DIR_W) std::swap(w, h);

	return TileX(tile) + w < Map::SizeX() &&
		TileY(tile) + h < Map::SizeY();
#endif
}

/**
 * This function initializes the airportspec array.
 */
void AirportSpec::ResetAirports()
{
	extern const AirportSpec _origin_airport_specs[NEW_AIRPORT_OFFSET];

	auto insert = std::copy(std::begin(_origin_airport_specs), std::end(_origin_airport_specs), std::begin(AirportSpec::specs));
	std::fill(insert, std::end(AirportSpec::specs), AirportSpec{});

	_airport_mngr.ResetOverride();
}

/**
 * Tie all airportspecs to their class.
 */
void BindAirportSpecs()
{
	for (int i = 0; i < NUM_AIRPORTS; i++) {
		AirportSpec *as = AirportSpec::GetWithoutOverride(i);
		if (as->enabled) AirportClass::Assign(as);
	}
}


void AirportOverrideManager::SetEntitySpec(AirportSpec &&as)
{
	uint8_t airport_id = this->AddEntityID(as.grf_prop.local_id, as.grf_prop.grfid, as.grf_prop.subst_id);

	if (airport_id == this->invalid_id) {
		GrfMsg(1, "Airport.SetEntitySpec: Too many airports allocated. Ignoring.");
		return;
	}

	AirportSpec::specs[airport_id] = std::move(as);

	/* Now add the overrides. */
	for (int i = 0; i < this->max_offset; i++) {
		AirportSpec *overridden_as = AirportSpec::GetWithoutOverride(i);

		if (this->entity_overrides[i] != AirportSpec::specs[airport_id].grf_prop.local_id || this->grfid_overrides[i] != AirportSpec::specs[airport_id].grf_prop.grfid) continue;

		overridden_as->grf_prop.override_id = airport_id;
		overridden_as->enabled = false;
		this->entity_overrides[i] = this->invalid_id;
		this->grfid_overrides[i] = 0;
	}
}

/* virtual */ uint32_t AirportScopeResolver::GetVariable(uint8_t variable, [[maybe_unused]] uint32_t parameter, bool &available) const
{
#ifdef WITH_RUST
	return openttd_rust_airport_variable(&this->context, &this->ro, this->ro.grffile, variable, parameter, &available, &GetAirportResolverServices());
#else
	switch (variable) {
		case 0x40: return this->layout;

		case 0x7A: return GetBadgeVariableResult(*this->ro.grffile, this->spec->badges, parameter);
	}

	if (this->st == nullptr) {
		available = false;
		return UINT_MAX;
	}

	switch (variable) {
		/* Get a variable from the persistent storage */
		case 0x7C: return (this->st->airport.psa != nullptr) ? this->st->airport.psa->GetValue(parameter) : 0;

		case 0xF0: return this->st->facilities.base();
		case 0xFA: return ClampTo<uint16_t>(this->st->build_date - CalendarTime::DAYS_TILL_ORIGINAL_BASE_YEAR);
	}

	return this->st->GetNewGRFVariable(this->ro, variable, parameter, available);
#endif
}

GrfSpecFeature AirportResolverObject::GetFeature() const
{
	return GSF_AIRPORTS;
}

uint32_t AirportResolverObject::GetDebugID() const
{
#ifdef WITH_RUST
	return RustSpecLocalID(this->airport_scope.context.spec);
#else
	return this->airport_scope.spec->grf_prop.local_id;
#endif
}

/* virtual */ uint32_t AirportScopeResolver::GetRandomBits() const
{
#ifdef WITH_RUST
	return openttd_rust_airport_random(&this->context, false, &GetAirportResolverServices());
#else
	return this->st == nullptr ? 0 : this->st->random_bits;
#endif
}

/**
 * Store a value into the object's persistent storage.
 * @param pos Position in the persistent storage to use.
 * @param value Value to store.
 */
/* virtual */ void AirportScopeResolver::StorePSA(uint pos, int32_t value)
{
#ifdef WITH_RUST
	openttd_rust_storage_airport(&GetRustStorageServices(), this->context.station, this->ro.grffile, pos, value);
#else
	if (this->st == nullptr) return;

	if (this->st->airport.psa == nullptr) {
		/* There is no need to create a storage if the value is zero. */
		if (value == 0) return;

		/* Create storage on first modification. */
		uint32_t grfid = (this->ro.grffile != nullptr) ? this->ro.grffile->grfid : 0;
		assert(PersistentStorage::CanAllocateItem());
		this->st->airport.psa = new PersistentStorage(grfid, GSF_AIRPORTS, this->st->airport.tile);
	}
	this->st->airport.psa->StoreValue(pos, value);
#endif
}

/**
 * Get the town scope associated with a station, if it exists.
 * On the first call, the town scope is created (if possible).
 * @return Town scope, if available.
 */
TownScopeResolver *AirportResolverObject::GetTown()
{
#ifdef WITH_RUST
	return openttd_rust_airport_town(&this->airport_scope.context, this, &GetAirportResolverServices());
#else
	if (!this->town_scope.has_value()) {
		Town *t = nullptr;
		if (this->airport_scope.st != nullptr) {
			t = this->airport_scope.st->town;
		} else if (this->airport_scope.tile != INVALID_TILE) {
			t = ClosestTownFromTile(this->airport_scope.tile, UINT_MAX);
		}
		if (t == nullptr) return nullptr;
		this->town_scope.emplace(*this, t, this->airport_scope.st == nullptr);
	}
	return &*this->town_scope;
#endif
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
	: ResolverObject(spec->grf_prop.grffile, callback, param1, param2), airport_scope(*this, tile, st, spec, layout)
{
#ifdef WITH_RUST
	this->root_spritegroup = openttd_rust_airport_root(&this->airport_scope.context, &GetAirportResolverServices());
#else
	this->root_spritegroup = spec->grf_prop.GetSpriteGroup(st != nullptr);
#endif
}

SpriteID GetCustomAirportSprite(const AirportSpec *as, uint8_t layout)
{
	AirportResolverObject object(INVALID_TILE, nullptr, as, layout);
	const auto *group = object.Resolve<ResultSpriteGroup>();
	if (group == nullptr || group->num_sprites == 0) return as->preview_sprite;

	return group->sprite;
}

#ifndef WITH_RUST
uint16_t GetAirportCallback(CallbackID callback, uint32_t param1, uint32_t param2, Station *st, TileIndex tile, std::span<int32_t> regs100)
{
	AirportResolverObject object(tile, st, AirportSpec::Get(st->airport.type), st->airport.layout, callback, param1, param2);
	return object.ResolveCallback(regs100);
}

#endif

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
