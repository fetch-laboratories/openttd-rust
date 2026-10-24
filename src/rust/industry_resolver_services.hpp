/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file industry_resolver_services.hpp Native leaves; no migrated owner policy. */
#ifndef RUST_INDUSTRY_RESOLVER_SERVICES_HPP
#define RUST_INDUSTRY_RESOLVER_SERVICES_HPP
#include "industry_resolver_ffi.h"
#include "services_newgrf.h"
#include "services_cargo.h"
#include "services_map.h"
#include "industry_shell_services.h"
#include "industry_generation_adapter.hpp"
#include "town_services.h"
#include "environment_services.h"
#include "tile_animation_services.h"
#include "../newgrf_industrytiles.h"
#include "../company_base.h"

/* Spec, Action2 and native associations remain live for synchronous resolution.
 * No ordinary-play leaf throws; debug/allocation failures terminate in noexcept.
 * Action2 and town/storage operations may reenter Rust; no owner borrow spans them. */
inline const OpenTTDIndustryResolverServices &GetIndustryResolverServices() noexcept
{
	static const OpenTTDIndustryResolverServices services{
		.shared = &GetRustSharedServices(),
		.generation = &GetIndustryGenerationServices(),
		.construction = &GetIndustryConstructionServices(),
		.lifecycle = &GetIndustryLifecycleRustServices(),
		.tile = &GetIndustryTileServices(),
		.get = [](uint16_t id) noexcept -> OpenTTDIndustryResolverRef { Industry *i = Industry::Get(IndustryID(id)); return {i->production_owner.get(), i->town}; },
		.map_type = [](uint8_t local, uint32_t grfid) noexcept { return _industry_mngr.GetID(local, grfid); },
		.spec_grfid = RustSpecGRFID<IndustrySpec>,
		.spec_local_id = RustSpecLocalID<IndustrySpec>,
		.root = RustSpecSpriteGroup<IndustrySpec>,
		.tile_root = RustSpecSpriteGroup<IndustryTileSpec>,
		.tile_local_id = RustSpecLocalID<IndustryTileSpec>,
		.tile_override = [](const IndustryTileSpec *spec) noexcept { return spec->grf_prop.override_id; },
		.tile_grfid = RustSpecGRFID<IndustryTileSpec>,
		.tile_has_groups = [](const IndustryTileSpec *spec) noexcept { return spec->grf_prop.HasSpriteGroups(); },
		.tile_substitute = [](const IndustryTileSpec *spec) noexcept { return spec->grf_prop.subst_id; },
		.clean_gfx = [](uint32_t tile) noexcept { return GetCleanIndustryGfx(TileIndex(tile)); },
		.nearby = RustNearbyTile,
		.nearby_info = RustNearbyTileInformation,
		.terrain = RustTerrainType,
		.town_zone = RustTownRadiusGroup,
		.water_distance = RustClosestWaterDistance,
		.town_tile = OpenTTDTownTile,
		.town_id = RustTownAuthorityIndex,
		.founder_info = [](uint8_t founder) noexcept -> uint32_t {
			const Company *company = Company::GetIfValid(CompanyID(founder));
			return (company != nullptr && company->is_ai ? 0x10000 : 0) | (company != nullptr ? company->GetCompanyRecolourOffset(LS_DEFAULT) << 24 : 0);
		},
		.grfid = RustResolverGRFFileID,
		.version = RustResolverGRFVersion,
		.callback_id = RustResolverCallback,
		.register_value = RustResolverRegister,
		.cargo_translation = [](const ResolverObject *r, uint8_t cargo) noexcept { return OpenTTDCargoTranslation(cargo, r->grffile); },
		.cargo_map = [](const IndustrySpec *spec, uint8_t cargo) noexcept { return spec->grf_prop.grffile->cargo_map[cargo]; },
		.badges = RustSpecBadges<IndustrySpec>,
		.tile_badges = RustSpecBadges<IndustryTileSpec>,
		.storage_value = [](Industry *i, uint32_t pos) noexcept { return i->psa != nullptr ? i->psa->GetValue(pos) : 0; },
		.unhandled = [](uint8_t variable, bool tile, bool missing, uint16_t callback) noexcept {
			if (missing) Debug(grf, 1, "Unhandled variable 0x{:X} (no available industry) in callback 0x{:x}", variable, callback);
			else if (tile) Debug(grf, 1, "Unhandled industry tile variable 0x{:X}", variable);
			else Debug(grf, 1, "Unhandled industry variable 0x{:X}", variable);
		},
		.pointer_magic = []() noexcept { Debug(grf, 0, "NewGRFs shouldn't be doing pointer magic"); },
		.base_scope = [](ResolverObject *r, uint8_t scope, uint8_t relative) noexcept { return r->ResolverObject::GetScope(static_cast<VarSpriteGroupScope>(scope), relative); },
		.existing_town = [](ResolverObject *r) noexcept -> TownScopeResolver * { auto &town = static_cast<IndustriesResolverObject *>(r)->town_scope; return town.has_value() ? &*town : nullptr; },
		.make_town = [](ResolverObject *r, Town *t, bool readonly) noexcept -> TownScopeResolver * { return &static_cast<IndustriesResolverObject *>(r)->town_scope.emplace(*r, t, readonly); },
		.callback = [](uint16_t cb, uint32_t p1, uint32_t p2, Industry *i, uint8_t type, uint32_t tile) noexcept { return GetIndustryCallback(static_cast<CallbackID>(cb), p1, p2, i, type, TileIndex(tile)); },
		.boolean = RustSpecBooleanCallback<IndustrySpec>,
		.callback_error = [](const IndustrySpec *spec, uint16_t cb, uint16_t result) noexcept { RustSpecCallbackError<IndustrySpec, IndustrySpec>(spec, cb, result); },
	};
	return services;
}
#endif
