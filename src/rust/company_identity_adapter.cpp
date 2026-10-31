/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file company_identity_adapter.cpp Borrowed native fields and presentation operations. */
#include "../stdafx.h"
#ifdef WITH_RUST
#include "../ai/ai.hpp"
#include "../game/game.hpp"
#include "../company_base.h"
#include "../company_func.h"
#include "../company_cmd.h"
#include "../company_manager_face.h"
#include "../command_func.h"
#include "../engine_base.h"
#include "../group.h"
#include "../town.h"
#include "../object.h"
#include "../smallmap_gui.h"
#include "../vehicle_func.h"
#include "../vehicle_base.h"
#include "../newgrf_engine.h"
#include "../news_func.h"
#include "../script/api/script_event_types.hpp"
#include "../strings_func.h"
#include "../table/strings.h"
#include "company_services.h"
#include "engine_services.h"
#include "vehicle_services.h"
#include "services_cargo.h"
#include "presentation_services.h"
#include "company_identity_ffi.h"
#include "town_name_ffi.h"
static_assert(MAX_COMPANIES == 15 && MAX_LENGTH_COMPANY_NAME_CHARS == 32 && MAX_LENGTH_PRESIDENT_NAME_CHARS == 32);
static_assert(to_underlying(Livery::Flag::Primary) == 0 && to_underlying(Livery::Flag::Secondary) == 1);
static_assert(COLOUR_END == 16 && INVALID_COLOUR == 255);
static_assert(COLOUR_DARK_BLUE == 0 && COLOUR_PALE_GREEN == 1 && COLOUR_YELLOW == 3 && COLOUR_RED == 4 && COLOUR_LIGHT_BLUE == 5 && COLOUR_GREEN == 6 && COLOUR_DARK_GREEN == 7 && COLOUR_BLUE == 8);
static_assert(COLOUR_CREAM == 9 && COLOUR_MAUVE == 10 && COLOUR_PURPLE == 11 && COLOUR_ORANGE == 12 && COLOUR_BROWN == 13 && COLOUR_GREY == 14 && COLOUR_WHITE == 15);
static_assert(LS_DEFAULT == 0 && LS_STEAM == 1 && LS_DIESEL == 2 && LS_ELECTRIC == 3 && LS_MONORAIL == 4 && LS_MAGLEV == 5 && LS_DMU == 6 && LS_EMU == 7);
static_assert(LS_PASSENGER_WAGON_STEAM == 8 && LS_PASSENGER_WAGON_DIESEL == 9 && LS_PASSENGER_WAGON_ELECTRIC == 10 && LS_PASSENGER_WAGON_MONORAIL == 11 && LS_PASSENGER_WAGON_MAGLEV == 12 && LS_FREIGHT_WAGON == 13);
static_assert(LS_BUS == 14 && LS_TRUCK == 15 && LS_PASSENGER_SHIP == 16 && LS_FREIGHT_SHIP == 17 && LS_HELICOPTER == 18 && LS_SMALL_PLANE == 19 && LS_LARGE_PLANE == 20 && LS_PASSENGER_TRAM == 21 && LS_FREIGHT_TRAM == 22 && LS_END == 23);
static_assert(to_underlying(EngineMiscFlag::RailIsMU) == 2 && to_underlying(EngineMiscFlag::RoadIsTram) == 0 && AIR_FAST == 2);
struct CompanyNames { std::string candidate, other; };
// Rust supplies call-local placement storage. No container or field snapshot crosses.
static_assert(sizeof(CompanyNames) <= sizeof(OpenTTDCompanyNameStorage));
static_assert(alignof(CompanyNames) <= alignof(OpenTTDCompanyNameStorage));
static CompanyNames &Names(OpenTTDCompanyNameStorage *p) noexcept { return *std::launder(reinterpret_cast<CompanyNames *>(p)); }
#include "company_identity_services.h"
void OPENTTD_RUST_CALL Identity_admin_update(uint8_t id) noexcept { CompanyAdminUpdate(Company::Get(CompanyID{id})); }
static void OPENTTD_RUST_CALL Identity_name_frame_init(OpenTTDCompanyNameStorage *slot) noexcept { new (slot) CompanyNames{}; }
static void OPENTTD_RUST_CALL Identity_name_frame_drop(OpenTTDCompanyNameStorage *slot) noexcept { Names(slot).~CompanyNames(); }
static OpenTTDByteSpan OPENTTD_RUST_CALL Identity_render(OpenTTDCompanyNameStorage *slot, uint32_t str, uint32_t param) noexcept { auto &s = Names(slot).candidate; s = GetString(str, param); return {reinterpret_cast<const uint8_t *>(s.data()), s.size()}; }
static OpenTTDByteSpan OPENTTD_RUST_CALL Identity_render_other(OpenTTDCompanyNameStorage *slot, uint32_t str, uint32_t param) noexcept { auto &s = Names(slot).other; s = GetString(str, param); return {reinterpret_cast<const uint8_t *>(s.data()), s.size()}; }
static void OPENTTD_RUST_CALL Identity_company_renamed(Company *c, OpenTTDCompanyNameStorage *slot) noexcept { AI::BroadcastNewEvent(new ScriptEventCompanyRenamed(c->index, Names(slot).candidate)); Game::NewEvent(new ScriptEventCompanyRenamed(c->index, Names(slot).candidate)); }
static void OPENTTD_RUST_CALL Identity_president_renamed(Company *c, OpenTTDCompanyNameStorage *slot) noexcept { AI::BroadcastNewEvent(new ScriptEventPresidentRenamed(c->index, Names(slot).candidate)); Game::NewEvent(new ScriptEventPresidentRenamed(c->index, Names(slot).candidate)); }
static void OPENTTD_RUST_CALL Identity_launch_news(Company *c, const Town *t) noexcept { auto cni = std::make_unique<CompanyNewsInformation>(STR_NEWS_COMPANY_LAUNCH_TITLE, c); EncodedString headline = GetEncodedString(STR_NEWS_COMPANY_LAUNCH_DESCRIPTION, cni->company_name, t->index); AddNewsItem(std::move(headline), NewsType::CompanyInfo, NewsStyle::Company, {}, c->last_build_coordinate, {}, std::move(cni)); }
static void OPENTTD_RUST_CALL Identity_rename_transport(OpenTTDByteSpan text) noexcept { Command<CMD_RENAME_COMPANY>::Do(DoCommandFlag::Execute, std::string(reinterpret_cast<const char *>(text.data), text.size) + " Transport"); }
static void OPENTTD_RUST_CALL Identity_president_windows() noexcept { InvalidateWindowClassesData(WC_COMPANY, 1); }
static void OPENTTD_RUST_CALL Identity_colour_windows() noexcept { InvalidateWindowData(WC_INCOME_GRAPH, 0); InvalidateWindowData(WC_OPERATING_PROFIT, 0); InvalidateWindowData(WC_DELIVERED_CARGO, 0); InvalidateWindowData(WC_PERFORMANCE_HISTORY, 0); InvalidateWindowData(WC_COMPANY_VALUE, 0); InvalidateWindowData(WC_LINKGRAPH_LEGEND, 0); BuildOwnerLegend(); InvalidateWindowData(WC_SMALLMAP, 0, 1); }
const OpenTTDFaceServices &GetRustFaceServices() noexcept
{
	static const OpenTTDFaceServices services{
	.offset = Face_offset, .length = Face_length, .valid_values = Face_valid_values,
	.toggle = Face_toggle, .off = Face_off, .on = Face_on,
	.count = Face_count,
	.vars = Face_vars,
	.at = Face_at,
	.spec = Face_spec,
	.set_style = Face_set_style,
	.set_label = Face_set_label,
	.style = Face_style,
	.bits = Face_bits,
	.random = GetRustTownNameServices().random_next,
	};
	return services;
}
const OpenTTDCompanyIdentityServices &GetRustCompanyIdentityServices() noexcept
{
	static const OpenTTDCompanyIdentityServices services{
	.shared = &GetRustSharedServices(), .randomizer = &_random, .faces = &GetRustFaceServices(), .groups = &FleetGroupServices(),
	.strings = {.unnamed = STR_SV_UNNAMED, .president = STR_PRESIDENT_NAME, .company = STR_COMPANY_NAME, .president_generated = SPECSTR_PRESIDENT_NAME, .andco = SPECSTR_ANDCO_NAME, .town_begin = SPECSTR_TOWNNAME_START, .town_end = SPECSTR_TOWNNAME_END, .company_begin = SPECSTR_COMPANY_NAME_START},
	.company = Identity_company,
	.next_company = RustCompanyNext,
	.company_id = Identity_company_id,
	.renew_reserve = RustCompanyRenewReserve,
	.name_id = Identity_name_id,
	.set_name_id = Identity_set_name_id,
	.name_parts = Identity_name_parts,
	.set_name_parts = Identity_set_name_parts,
	.president_id = Identity_president_id,
	.set_president_id = Identity_set_president_id,
	.president_parts = Identity_president_parts,
	.set_president_parts = Identity_set_president_parts,
	.colour = Identity_colour,
	.set_colour = Identity_set_colour,
	.last_build = Identity_last_build,
	.is_ai = Identity_is_ai,
	.name = Identity_name,
	.set_name = Identity_set_name,
	.president = Identity_president,
	.set_president = Identity_set_president,
	.livery = Identity_livery,
	.face = Identity_face,
	.name_frame_init = Identity_name_frame_init,
	.name_frame_drop = Identity_name_frame_drop,
	.render = Identity_render,
	.render_other = Identity_render_other,
	.company_renamed = Identity_company_renamed,
	.president_renamed = Identity_president_renamed,
	.launch_news = Identity_launch_news,
	.admin_update = Identity_admin_update,
	.rename_transport = Identity_rename_transport,
	.president_windows = Identity_president_windows,
	.colour_windows = Identity_colour_windows,
	.object_colours = Identity_object_colours,
	.dirty = FleetGroupServices().screen_dirty,
	.reset_colourmap = Identity_reset_colourmap,
	.engine = GetRustEngineCapabilitiesServices().engine,
	.engine_type = GetRustEngineCapabilitiesServices().kind,
	.engine_cargo = GetRustEngineCapabilitiesServices().default_cargo,
	.engine_kind = GetRustEngineCapabilitiesServices().rail_kind,
	.engine_class = GetRustEngineCapabilitiesServices().train_class,
	.engine_misc = RustEngineMiscFlags,
	.aircraft_subtype = GetRustEngineCapabilitiesServices().aircraft_subtype,
	.vehicle_cargo = GetRustEngineCapabilitiesServices().vehicle_cargo,
	.vehicle_first = GetRustVehicleResolverServices().first,
	.articulated = GetRustVehicleResolverServices().articulated,
	.wagon_override = RustVehicleRead<bool, RustVehicleUsesWagonOverride>,
	.cargo_freight = Identity_cargo_freight,
	.cargo_classes = GetRustEngineCapabilitiesServices().cargo_classes,
	.vehicle_id = RustVehicleRead<uint32_t, RustVehicleIndexOpaque>,
	.invalidate_vehicle = RustVehicleInvoke<RustVehicleInvalidateNewGRFCache>,
	.vehicle_owner = GetRustVehicleServices().owner,
	};
	return services;
}
#endif
