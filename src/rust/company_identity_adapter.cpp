/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file company_identity_adapter.cpp Borrowed native fields and presentation operations. */
#include "../stdafx.h"
#ifdef WITH_RUST
#include "../ai/ai.hpp"
#include "../game/game.hpp"
#include "../company_base.h"
#include "../company_func.h"
#include "../company_cmd.h"
#include "../command_func.h"
#include "../engine_base.h"
#include "../group.h"
#include "../town.h"
#include "../object.h"
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
void Identity_admin_update(uint8_t id) noexcept { CompanyAdminUpdate(Company::Get(CompanyID{id})); }
struct CompanyNames { std::string candidate, other; };
// Rust supplies call-local placement storage. No container or field snapshot crosses.
static_assert(sizeof(CompanyNames) <= sizeof(OpenTTDCompanyNameStorage));
static_assert(alignof(CompanyNames) <= alignof(OpenTTDCompanyNameStorage));
static CompanyNames &Names(OpenTTDCompanyNameStorage *p) noexcept { return *std::launder(reinterpret_cast<CompanyNames *>(p)); }
#include "company_identity_services.h"
static void Identity_name_frame_init(OpenTTDCompanyNameStorage *slot) noexcept { new (slot) CompanyNames{}; }
static void Identity_name_frame_drop(OpenTTDCompanyNameStorage *slot) noexcept { Names(slot).~CompanyNames(); }
static OpenTTDByteSpan Identity_render(OpenTTDCompanyNameStorage *slot, uint32_t str, uint32_t param) noexcept { auto &s = Names(slot).candidate; s = GetString(str, param); return {reinterpret_cast<const uint8_t *>(s.data()), s.size()}; }
static OpenTTDByteSpan Identity_render_other(OpenTTDCompanyNameStorage *slot, uint32_t str, uint32_t param) noexcept { auto &s = Names(slot).other; s = GetString(str, param); return {reinterpret_cast<const uint8_t *>(s.data()), s.size()}; }
static void Identity_company_renamed(Company *c, OpenTTDCompanyNameStorage *slot) noexcept { AI::BroadcastNewEvent(new ScriptEventCompanyRenamed(c->index, Names(slot).candidate)); Game::NewEvent(new ScriptEventCompanyRenamed(c->index, Names(slot).candidate)); }
static void Identity_president_renamed(Company *c, OpenTTDCompanyNameStorage *slot) noexcept { AI::BroadcastNewEvent(new ScriptEventPresidentRenamed(c->index, Names(slot).candidate)); Game::NewEvent(new ScriptEventPresidentRenamed(c->index, Names(slot).candidate)); }
static void Identity_launch_news(Company *c, const Town *t) noexcept { auto cni = std::make_unique<CompanyNewsInformation>(STR_NEWS_COMPANY_LAUNCH_TITLE, c); EncodedString headline = GetEncodedString(STR_NEWS_COMPANY_LAUNCH_DESCRIPTION, cni->company_name, t->index); AddNewsItem(std::move(headline), NewsType::CompanyInfo, NewsStyle::Company, {}, c->last_build_coordinate, {}, std::move(cni)); }
static void Identity_rename_transport(OpenTTDByteSpan text) noexcept { Command<CMD_RENAME_COMPANY>::Do(DoCommandFlag::Execute, std::string(reinterpret_cast<const char *>(text.data), text.size) + " Transport"); }
static void Identity_president_windows() noexcept { InvalidateWindowClassesData(WC_COMPANY, 1); }
static void Identity_colour_windows() noexcept { InvalidateWindowData(WC_INCOME_GRAPH, 0); InvalidateWindowData(WC_OPERATING_PROFIT, 0); InvalidateWindowData(WC_DELIVERED_CARGO, 0); InvalidateWindowData(WC_PERFORMANCE_HISTORY, 0); InvalidateWindowData(WC_COMPANY_VALUE, 0); InvalidateWindowData(WC_LINKGRAPH_LEGEND, 0); BuildOwnerLegend(); InvalidateWindowData(WC_SMALLMAP, 0, 1); }
const OpenTTDFaceServices &GetRustFaceServices() noexcept
{
	static const OpenTTDFaceServices services{
	.count = Face_count,
	.vars = Face_vars,
	.variable = Face_variable,
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
extern const OpenTTDFleetGroupServices &FleetGroupServices();
const OpenTTDCompanyIdentityServices &GetRustCompanyIdentityServices() noexcept
{
	static const OpenTTDCompanyIdentityServices services{
	.shared = &GetRustSharedServices(), .randomizer = &_random, .faces = &GetRustFaceServices(), .groups = &FleetGroupServices(),
	.strings = {STR_SV_UNNAMED, STR_PRESIDENT_NAME, STR_COMPANY_NAME, SPECSTR_PRESIDENT_NAME, SPECSTR_ANDCO_NAME, SPECSTR_TOWNNAME_START, SPECSTR_TOWNNAME_END, SPECSTR_COMPANY_NAME_START},
	.company = Identity_company,
	.next_company = Identity_next_company,
	.company_id = Identity_company_id,
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
	.current = RustCurrentCompanyID,
	.dirty = RustScreenDirty,
	.utf8_length = RustUtf8Length,
	.reset_colourmap = ResetVehicleColourMap,
	.engine = RustEngineByID,
	.engine_type = RustEngineType,
	.engine_cargo = RustEngineDefaultCargo,
	.engine_kind = Engine_rail_kind,
	.engine_class = RustEngineTrainClass,
	.engine_misc = RustEngineMiscFlags,
	.aircraft_subtype = RustEngineAircraftSubtype,
	.vehicle_cargo = RustVehicleCargoType,
	.vehicle_first = RustGroundFirst,
	.articulated = RustVehicleRead<bool, RustVehicleArticulated>,
	.wagon_override = RustVehicleRead<bool, RustVehicleUsesWagonOverride>,
	.cargo_freight = Identity_cargo_freight,
	.cargo_passengers = Identity_cargo_passengers,
	.goods = Identity_goods,
	.vehicle_id = RustVehicleIndexOpaque,
	.invalidate_vehicle = RustVehicleInvoke<RustVehicleInvalidateNewGRFCache>,
	.vehicle_owner = RustVehicleOwnerOf,
	};
	return services;
}
#endif
