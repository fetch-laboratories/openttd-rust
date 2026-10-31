/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file industry-policy-console.hpp Industry timer consequences on real pool shells. */
#include "company_type.h"
#include "economy_type.h"
#include "news_gui.h"
#include "news_func.h"
#include "station_base.h"
#include "vehicle_base.h"
#include "window_func.h"
#include "ai/ai.hpp"
#include "game/game.hpp"
#include "saveload/saveload.h"
#include "misc/history_func.hpp"
#include "table/strings.h"
#ifdef WITH_RUST
#include "rust/clock_ffi.h"
#else
#include "industry-policy-reference.inc"
#endif

static void IndustryPolicyRequire(bool ok) { if (!ok) std::abort(); }
static void IndustryPolicyObserve()
{
	uint forests = 0, others = 0;
	for (const Industry *ind : Industry::Iterate()) {
		for (TileIndex tile : TileArea{ind->location}) if (IsTileType(tile, MP_INDUSTRY) && Industry::GetByTile(tile) == ind) {
			if (IsTileForestIndustry(tile)) ++forests; else ++others;
		}
		fmt::print(stderr, "INDUSTRY-POLICY industry {} {} {}", ind->index.base(), ind->prod_level, ind->valid_history);
		for (const auto &p : ind->produced) fmt::print(stderr, " {}:{}:{}:{}", p.cargo, p.rate, p.history[LAST_MONTH].production, p.history[LAST_MONTH].transported);
		fmt::print(stderr, "\n");
	}
	IndustryPolicyRequire(!IsTileForestIndustry(TileIndex{0}) && forests != 0 && others != 0);
	fmt::print(stderr, "INDUSTRY-POLICY forests {} {}\n", forests, others);
	for (const auto &news : GetNews()) fmt::print(stderr, "INDUSTRY-POLICY news {} {} {}\n", to_underlying(news.type), SerialiseNewsReference(news.ref1), news.headline.GetDecodedString());
	fmt::print(stderr, "INDUSTRY-POLICY rng {} {} {} {}\n", _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]);
}
static bool IndustryPolicy(std::span<std::string_view> args)
{
	if (args.size() < 2) return false;
	if (args[1] == "inspect") { _interactive_random.SetSeed(0x60100002); IndustryPolicyObserve(); fmt::print(stderr, "INDUSTRY-POLICY complete true\n"); return true; }
	Vehicle *vehicle = nullptr;
	for (Vehicle *v : Vehicle::Iterate()) if (v->type == VEH_ROAD && v->GetNumOrders() != 0) { vehicle = v; break; }
	IndustryPolicyRequire(vehicle != nullptr);
	Backup<CompanyID> local(_local_company);
	Backup<CargoType> cargo(vehicle->cargo_type);
	Backup<Order> order(*vehicle->GetOrder(0));
	_settings_game.economy.type = ET_SMOOTH;
	_random.SetSeed(0x601); _interactive_random.SetSeed(0x60100001);
	for (uint outcome = 0; outcome < 3; ++outcome) {
		local.Change(outcome == 2 ? vehicle->owner : CompanyID{OWNER_NONE});
		Industry *ind = nullptr;
		for (Industry *candidate : Industry::Iterate()) {
			const auto *spec = GetIndustrySpec(candidate->type);
			if (candidate->produced.empty() || !IsValidCargoType(candidate->produced[0].cargo) || candidate->produced[0].cargo == GetCargoTypeByLabel(CT_PASSENGERS) || !spec->IsRawIndustry() || spec->UsesOriginalEconomy()) continue;
			if (candidate->stations_near.empty() == (outcome == 0)) { ind = candidate; break; }
		}
		IndustryPolicyRequire(ind != nullptr);
		ind->ctlflags.Set(IndustryControlFlag::NoClosure);
		vehicle->cargo_type = ind->produced[0].cargo;
		if (outcome != 0) vehicle->GetOrder(0)->MakeGoToStation((*ind->stations_near.begin())->index);
		DeleteIndustryNews(ind->index);
		bool found = false;
		for (uint attempt = 0; attempt < 512 && !found; ++attempt) {
			ind->produced[0].rate = 8;
#ifdef WITH_RUST
			const OpenTTDClockIndustry clock{_industry_builder.owner.get(), &GetIndustryRustServices()};
			openttd_rust_clock_industry_monthly(&clock, nullptr);
#else
			IndustryPolicyMonthly();
#endif
			for (const auto &news : GetNews()) if (news.ref1 == NewsReference{ind->index}) {
				IndustryPolicyRequire(news.type == (outcome == 0 ? NewsType::IndustryNobody : outcome == 1 ? NewsType::IndustryOther : NewsType::IndustryCompany));
				found = true;
			}
		}
		IndustryPolicyRequire(found);
		fmt::print(stderr, "INDUSTRY-POLICY servicing {} true\n", outcome);
	}
	_settings_game.economy.type = ET_ORIGINAL;
	for (uint n = 0; n < 16; ++n) {
#ifdef WITH_RUST
		_industry_builder.daily_counter = 2 << 16;
		const OpenTTDClockIndustry clock{_industry_builder.owner.get(), &GetIndustryRustServices()};
		openttd_rust_clock_industry_daily(&clock, nullptr);
#else
		_economy.industry_daily_change_counter = 2 << 16;
		IndustryPolicyDaily();
#endif
	}
	order.Restore(); cargo.Restore(); local.Restore();
	IndustryPolicyObserve();
	if (args.size() == 3) IndustryPolicyRequire(SaveOrLoad(std::string(args[2]), SLO_SAVE, DFT_GAME_FILE, SAVE_DIR, false) == SL_OK);
	fmt::print(stderr, "INDUSTRY-POLICY complete true\n"); return true;
}
