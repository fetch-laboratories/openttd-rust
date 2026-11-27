/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file company_identity_adapter.cpp The company identity and face tables over native records. */
#include "../stdafx.h"
#include "../company_base.h"
#include "../company_func.h"
#include "../company_manager_face.h"
#include "../smallmap_gui.h"
#include "company_services.h"
#include "engine_services.h"
#include "object_resolver_ffi.h"
#include "services_cargo.h"
#include "town_ffi.h"
#include "command_services_ffi.h"
#include "company_identity_ffi.h"
static_assert(MAX_COMPANIES == 15 && MAX_LENGTH_COMPANY_NAME_CHARS == 32 && MAX_LENGTH_PRESIDENT_NAME_CHARS == 32);
static_assert(to_underlying(Livery::Flag::Primary) == 0 && to_underlying(Livery::Flag::Secondary) == 1);
static_assert(COLOUR_END == 16 && INVALID_COLOUR == 255);
static_assert(COLOUR_DARK_BLUE == 0 && COLOUR_PALE_GREEN == 1 && COLOUR_YELLOW == 3 && COLOUR_RED == 4 && COLOUR_LIGHT_BLUE == 5 && COLOUR_GREEN == 6 && COLOUR_DARK_GREEN == 7 && COLOUR_BLUE == 8);
static_assert(COLOUR_CREAM == 9 && COLOUR_MAUVE == 10 && COLOUR_PURPLE == 11 && COLOUR_ORANGE == 12 && COLOUR_BROWN == 13 && COLOUR_GREY == 14 && COLOUR_WHITE == 15);
static_assert(LS_DEFAULT == 0 && LS_STEAM == 1 && LS_DIESEL == 2 && LS_ELECTRIC == 3 && LS_MONORAIL == 4 && LS_MAGLEV == 5 && LS_DMU == 6 && LS_EMU == 7);
static_assert(LS_PASSENGER_WAGON_STEAM == 8 && LS_PASSENGER_WAGON_DIESEL == 9 && LS_PASSENGER_WAGON_ELECTRIC == 10 && LS_PASSENGER_WAGON_MONORAIL == 11 && LS_PASSENGER_WAGON_MAGLEV == 12 && LS_FREIGHT_WAGON == 13);
static_assert(LS_BUS == 14 && LS_TRUCK == 15 && LS_PASSENGER_SHIP == 16 && LS_FREIGHT_SHIP == 17 && LS_HELICOPTER == 18 && LS_SMALL_PLANE == 19 && LS_LARGE_PLANE == 20 && LS_PASSENGER_TRAM == 21 && LS_FREIGHT_TRAM == 22 && LS_END == 23);
static_assert(to_underlying(EngineMiscFlag::RailIsMU) == 2 && to_underlying(EngineMiscFlag::RoadIsTram) == 0 && AIR_FAST == 2);
static_assert(to_underlying(FaceVarType::Sprite) == 0 && to_underlying(FaceVarType::Palette) == 1 && to_underlying(FaceVarType::Toggle) == 2);
#include "company_identity_services.h"
static void OPENTTD_RUST_CALL Identity_build_owner_legend() noexcept { BuildOwnerLegend(); }
const OpenTTDFaceServices &GetRustFaceServices() noexcept
{
	static const OpenTTDFaceServices services{
	.at = Face_at,
	.bits = Face_bits,
	.count = Face_count,
	.data = Face_data,
	.length = Face_length,
	.offset = Face_offset,
	.set_style = Face_set_style,
	.set_style_label = Face_set_style_label,
	.spec = Face_spec,
	.style = Face_style,
	.type = Face_type,
	.valid_values = Face_valid_values,
	.vars = Face_vars,
	};
	return services;
}
const OpenTTDCompanyIdentityServices &GetRustCompanyIdentityServices() noexcept
{
	static const OpenTTDCompanyIdentityServices services{
	.authority = &GetRustTownAuthorityServices(),
	.capabilities = &GetRustEngineCapabilitiesServices(),
	.commands = &GetRustCommandServices(),
	.faces = &GetRustFaceServices(),
	.groups = &FleetGroupServices(),
	.randomizer = &_random,
	.shared = &GetRustSharedServices(),
	.admin_update = Identity_admin_update,
	.build_owner_legend = Identity_build_owner_legend,
	.colour = Identity_colour,
	.company = Identity_company,
	.company_if_valid = [](uint8_t id) noexcept -> Company * { return Company::GetIfValid(CompanyID(id)); },
	.engine_renew = RustCompanyEngineRenew,
	.engine_renew_money = RustCompanyEngineRenewMoney,
	.engine_renew_months = RustCompanyEngineRenewMonths,
	.face = Identity_face,
	.finances = [](Company *c) noexcept -> CompanyFinances * { return &c->Finances(); },
	.index = Identity_index,
	.is_ai = Identity_is_ai,
	.is_freight = Identity_is_freight,
	.last_build_coordinate = Identity_last_build_coordinate,
	.livery = Identity_livery,
	.name = Identity_name,
	.name_1 = Identity_name_1,
	.name_2 = Identity_name_2,
	.next_company = RustCompanyNext,
	.objects = []() noexcept { return &GetRustObjectServices(); },
	.president_name = Identity_president_name,
	.president_name_1 = Identity_president_name_1,
	.president_name_2 = Identity_president_name_2,
	.renew_keep_length = RustCompanyRenewKeepLength,
	.renewal_head = RustCompanyRenewalHead,
	.reset_colourmap = Identity_reset_colourmap,
	.set_colour = Identity_set_colour,
	.set_name = Identity_set_name,
	.set_name_1 = Identity_set_name_1,
	.set_name_2 = Identity_set_name_2,
	.set_president_name = Identity_set_president_name,
	.set_president_name_1 = Identity_set_president_name_1,
	.set_president_name_2 = Identity_set_president_name_2,
	.vehicle_defaults = RustCompanyVehicleDefaultsOf,
	};
	return services;
}
