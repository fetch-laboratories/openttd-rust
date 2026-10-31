/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file subsidy.cpp Handling of subsidies. */

#include "stdafx.h"
#include "company_func.h"
#include "industry.h"
#include "town.h"
#include "news_func.h"
#include "ai/ai.hpp"
#include "station_base.h"
#include "strings_func.h"
#include "window_func.h"
#include "subsidy_base.h"
#include "subsidy_func.h"
#include "core/pool_func.hpp"
#include "core/random_func.hpp"
#include "core/container_func.hpp"
#include "game/game.hpp"
#include "command_func.h"
#include "string_func.h"
#include "tile_cmd.h"
#include "subsidy_cmd.h"
#include "timer/timer.h"
#include "timer/timer_game_economy.h"

#include "table/strings.h"

#include "rust/subsidy_ffi.h"
#include "rust/industry_ffi.h"
#include "rust/industry_shell_services.h"
#include "rust/company_services.h"
#include "rust/house_ffi.h"
#include "rust/settings_services.h"
#include "rust/services_cargo.h"
#include "rust/services_station.h"
#include "rust/clock_ffi.h"

#include "safeguards.h"

SubsidyPool _subsidy_pool("Subsidy"); ///< Pool for the subsidies.
INSTANTIATE_POOL_METHODS(Subsidy)

/**
 * Get the NewsReference for a subsidy Source.
 * @returns NewsReference.
 */
NewsReference Source::GetNewsReference() const
{
	switch (this->type) {
		case SourceType::Industry: return static_cast<IndustryID>(this->id);
		case SourceType::Town: return static_cast<TownID>(this->id);
		default: NOT_REACHED();
	}
}

/**
 * Get the format string for a subsidy Source.
 * @returns The format string.
 */
StringID Source::GetFormat() const
{
	switch (this->type) {
		case SourceType::Industry: return STR_INDUSTRY_NAME;
		case SourceType::Town: return STR_TOWN_NAME;
		default: NOT_REACHED();
	}
}

/**
 * Marks subsidy as awarded, creates news and AI event
 * @param company awarded company
 */

/**
 * Sets a flag indicating that given town/industry is part of subsidised route.
 * @param source actual source
 * @param flag flag to set
 */

/** Perform a full rebuild of the subsidies cache. */
void RebuildSubsidisedSourceAndDestinationCache()
{
	openttd_rust_subsidy_rebuild(&GetRustSubsidyServices());
}

/**
 * Delete the subsidies associated with a given cargo source type and id.
 * @param source The source to look for.
 */

/**
 * Check whether a specific subsidy already exists.
 * @param cargo Cargo type.
 * @param src The source.
 * @param dst The destination.
 * @return \c true if the subsidy already exists, \c false if not.
 */

/**
 * Creates a subsidy with the given parameters.
 * @param cargo_type Subsidised cargo.
 * @param src Source of cargo.
 * @param dst Destination of cargo.
 */

/**
 * Create a new subsidy.
 * @param flags type of operation
 * @param cargo_type CargoType of subsidy.
 * @param src Source.
 * @param dst Destination.
 * @return the cost of this operation or an error
 */
CommandCost CmdCreateSubsidy(DoCommandFlags flags, CargoType cargo_type, Source src, Source dst)
{
	return openttd_rust_subsidy_command(&GetRustSubsidyServices(), flags.base(), cargo_type, src, dst) ? CommandCost() : CMD_ERROR;
}

/**
 * Tries to create a passenger subsidy between two towns.
 * @return True iff the subsidy was created.
 */



/**
 * Tries to create a cargo subsidy with a town as source.
 * @return True iff the subsidy was created.
 */

/**
 * Tries to create a cargo subsidy with an industry as source.
 * @return True iff the subsidy was created.
 */

/**
 * Tries to find a suitable destination for the given source and cargo.
 * @param cargo_type Subsidized cargo.
 * @param src Source of cargo.
 * @return True iff the subsidy was created.
 */

/** Perform the economy monthly update of open subsidies, and try to create a new one. */

static const IntervalTimer<TimerGameEconomy> _economy_subsidies_monthly({TimerGameEconomy::MONTH, TimerGameEconomy::Priority::SUBSIDY}, RustClockCallback{openttd_rust_clock_subsidy_monthly, &GetRustSubsidyServices()});


/**
 * Tests whether given delivery is subsidised and possibly awards the subsidy to delivering company
 * @param cargo_type type of cargo
 * @param company company delivering the cargo
 * @param src source of cargo
 * @param st station where the cargo is delivered to
 * @return is the delivery subsidised?
 */
bool CheckSubsidised(CargoType cargo_type, CompanyID company, Source src, const Station *st)
{
	return openttd_rust_subsidy_check(&GetRustSubsidyServices(), cargo_type, company.base(), src, RustBaseStationID(st));
}

static void SubsidyOfferedNews(uint16_t id) noexcept
{
	const Subsidy *s = Subsidy::Get(id);
	const CargoSpec *cs = CargoSpec::Get(s->cargo_type);
	EncodedString headline = GetEncodedString(STR_NEWS_SERVICE_SUBSIDY_OFFERED, cs->name, s->src.GetFormat(), s->src.id, s->dst.GetFormat(), s->dst.id, _settings_game.difficulty.subsidy_duration);
	AddNewsItem(std::move(headline), NewsType::Subsidies, NewsStyle::Normal, {}, s->src.GetNewsReference(), s->dst.GetNewsReference());
}
static void SubsidyAwardedNews(uint16_t id) noexcept
{
	const Subsidy *s = Subsidy::Get(id);
	std::string company_name = GetString(STR_COMPANY_NAME, s->awarded);

	/* Add a news item */
	const CargoSpec *cs = CargoSpec::Get(s->cargo_type);
	EncodedString headline = GetEncodedString(STR_NEWS_SERVICE_SUBSIDY_AWARDED_HALF + _settings_game.difficulty.subsidy_multiplier, std::move(company_name), cs->name, s->src.GetFormat(), s->src.id, s->dst.GetFormat(), s->dst.id, _settings_game.difficulty.subsidy_duration);
	AddNewsItem(std::move(headline), NewsType::Subsidies, NewsStyle::Normal, {}, s->src.GetNewsReference(), s->dst.GetNewsReference());
}
static void SubsidyOfferExpiredNews(uint16_t id) noexcept
{
	const Subsidy *s = Subsidy::Get(id);
	const CargoSpec *cs = CargoSpec::Get(s->cargo_type);
	EncodedString headline = GetEncodedString(STR_NEWS_OFFER_OF_SUBSIDY_EXPIRED, cs->name, s->src.GetFormat(), s->src.id, s->dst.GetFormat(), s->dst.id);
	AddNewsItem(std::move(headline), NewsType::Subsidies, NewsStyle::Normal, {}, s->src.GetNewsReference(), s->dst.GetNewsReference());
}
static void SubsidyWithdrawnNews(uint16_t id) noexcept
{
	const Subsidy *s = Subsidy::Get(id);
	const CargoSpec *cs = CargoSpec::Get(s->cargo_type);
	EncodedString headline = GetEncodedString(STR_NEWS_SUBSIDY_WITHDRAWN_SERVICE, cs->name, s->src.GetFormat(), s->src.id, s->dst.GetFormat(), s->dst.id);
	AddNewsItem(std::move(headline), NewsType::Subsidies, NewsStyle::Normal, {}, s->src.GetNewsReference(), s->dst.GetNewsReference());
}
const OpenTTDSubsidyServices &GetRustSubsidyServices() noexcept
{
	static const OpenTTDSubsidyServices services = {
		.shared = &GetRustSharedServices(), .town = &GetRustTownServices(), .industry = &GetIndustryRustServices(),
		.industry_owner = RustIndustryCanonicalOwner<void *>, .industry_id = RustIndustryID<void *>, .next_industry = RustIndustryNextID,
		.has_source_flag = [](Source source, PartOfSubsidy part) noexcept { switch (source.type) { case SourceType::Town: return Town::Get(source.ToTownID())->cache.part_of_subsidy.Test(part); case SourceType::Industry: return Industry::Get(source.ToIndustryID())->part_of_subsidy.Test(part); default: NOT_REACHED(); } },
		.set_source_flag = [](Source source, PartOfSubsidy part) noexcept { switch (source.type) { case SourceType::Town: Town::Get(source.ToTownID())->cache.part_of_subsidy.Set(part); return; case SourceType::Industry: Industry::Get(source.ToIndustryID())->part_of_subsidy.Set(part); return; default: NOT_REACHED(); } },
		.clear_source_flags = [](Source source) noexcept { switch (source.type) { case SourceType::Town: Town::Get(source.ToTownID())->cache.part_of_subsidy = {}; return; case SourceType::Industry: Industry::Get(source.ToIndustryID())->part_of_subsidy = {}; return; default: NOT_REACHED(); } },
		.valid_source = [](Source source) noexcept { return source.type == SourceType::Town ? Town::IsValidID(source.ToTownID()) : source.type == SourceType::Industry && Industry::IsValidID(source.ToIndustryID()); },
		.can_allocate = []() noexcept { return Subsidy::CanAllocateItem(); },
		.allocate = [](CargoType cargo, Source src, Source dst, uint16_t remaining) noexcept { return (new Subsidy(cargo, src, dst, remaining))->index.base(); },
		.destroy = [](uint16_t id) noexcept { delete Subsidy::Get(id); },
		.duration = OpenTTDSubsidyDuration,
		.local_company = RustLocalCompanyID,
		.cargo_valid = OpenTTDCargoSpecValid,
		.manual = OpenTTDManualCargoDistribution,
		.manual_pax = OpenTTDManualPassengerDistribution,
		.any_manual = []() noexcept { return _settings_game.linkgraph.distribution_pax == DT_MANUAL || _settings_game.linkgraph.distribution_mail == DT_MANUAL || _settings_game.linkgraph.distribution_armoured == DT_MANUAL || _settings_game.linkgraph.distribution_default == DT_MANUAL; },
		.offered_news = SubsidyOfferedNews, .awarded_news = SubsidyAwardedNews,
		.offer_expired_news = SubsidyOfferExpiredNews, .withdrawn_news = SubsidyWithdrawnNews,
		.offer_events = [](uint16_t id) noexcept { AI::BroadcastNewEvent(new ScriptEventSubsidyOffer(SubsidyID(id))); Game::NewEvent(new ScriptEventSubsidyOffer(SubsidyID(id))); },
		.award_events = [](uint16_t id) noexcept { AI::BroadcastNewEvent(new ScriptEventSubsidyAwarded(SubsidyID(id))); Game::NewEvent(new ScriptEventSubsidyAwarded(SubsidyID(id))); },
		.offer_expired_events = [](uint16_t id) noexcept { AI::BroadcastNewEvent(new ScriptEventSubsidyOfferExpired(SubsidyID(id))); Game::NewEvent(new ScriptEventSubsidyOfferExpired(SubsidyID(id))); },
		.expired_events = [](uint16_t id) noexcept { AI::BroadcastNewEvent(new ScriptEventSubsidyExpired(SubsidyID(id))); Game::NewEvent(new ScriptEventSubsidyExpired(SubsidyID(id))); },
		.dirty = []() noexcept { InvalidateWindowData(WC_SUBSIDIES_LIST, 0); },
		.station = RustStationGet<const Station *>,
		.assert_destination = [](Source source) noexcept { if (source.type == SourceType::Town) { assert(Town::Get(source.ToTownID())->cache.part_of_subsidy.Test(PartOfSubsidy::Destination)); } else { assert(Industry::Get(source.ToIndustryID())->part_of_subsidy.Test(PartOfSubsidy::Destination)); } },
	};
	return services;
}
#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDSubsidyServices.hpp"
bool CheckAbi_subsidy() { return AbiServicesComplete(GetRustSubsidyServices()); }
