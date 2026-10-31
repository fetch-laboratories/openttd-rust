/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file industry.cpp Canonical industry slot/history and native ABI checks. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#ifdef WITH_RUST
#include "../industry.h"
#include "abi_layout.hpp"
#include "../rust/industry_generation_adapter.hpp"
#include "../rust/industry_shell_services.h"
#include "../rust/industry_resolver_ffi.h"
#include "../rust/override_manager_ffi.h"
#include "../rust/industry_definition_ffi.h"
#include "../rust/industry_property_ffi.h"
#include "../rust/newgrf_bytereader_services.h"
#include "../rust/newgrf_loading_ffi.h"
#include "../newgrf.h"
#include "../newgrf_spritegroup.h"
#include "../rust/newgrf_global_ffi.h"
#include "../object_base.h"
#include "../industrytype.h"
#include "../water_map.h"
#include "../core/backup_type.hpp"
#include "../economy_func.h"
#include <thread>
static_assert(sizeof(CargoArray::value_type) == sizeof(uint32_t) && sizeof(CargoTypes) == sizeof(uint64_t));
static_assert(std::is_trivially_destructible_v<TileArea>);
static_assert(sizeof(TileArea) == sizeof(OpenTTDIndustryLocation));
static_assert(alignof(TileArea) == alignof(OpenTTDIndustryMetadata));
static_assert(offsetof(TileArea, tile) == offsetof(OpenTTDIndustryMetadata, tile));
static_assert(offsetof(TileArea, w) == offsetof(OpenTTDIndustryMetadata, width));
static_assert(offsetof(TileArea, h) == offsetof(OpenTTDIndustryMetadata, height));
static_assert(sizeof(IndustryID) == sizeof(uint16_t) && alignof(IndustryID) == alignof(uint16_t));
static_assert(sizeof(Owner) == sizeof(uint8_t) && alignof(Owner) == alignof(uint8_t));
static_assert(sizeof(Colours) == sizeof(uint8_t));
static_assert(sizeof(TimerGameCalendar::Date) == sizeof(int32_t));

TEST_CASE("Industry - canonical native layouts")
{
	ABI_LAYOUT_CHECK;
#include "abi-layout-industry.inc"

	REQUIRE(std::size(OpenTTDIndustryBuilderFields{}.builddata) == NUM_INDUSTRYTYPES);
	REQUIRE(HISTORY_RECORDS == 61);
	REQUIRE(sizeof(Industry::ProducedHistory) == 4);
	REQUIRE(sizeof(Industry::AcceptedHistory) == 4);
	REQUIRE(sizeof(TimerGameEconomy::Year) == sizeof(int32_t));
	REQUIRE(sizeof(TimerGameEconomy::Date) == sizeof(int32_t));
	REQUIRE(sizeof(IndustryControlFlags) == sizeof(uint8_t));
	REQUIRE(sizeof(IndustryTypeBuildData) == sizeof(OpenTTDIndustryBuildFields));
}
class TestOverrideManager : public OverrideManagerBase {
public:
	using OverrideManagerBase::OverrideManagerBase;
	bool CheckValidNewID(uint16_t id) override { return id != 3; }
	uint16_t GetID(uint16_t local, uint32_t grfid) const override { return local == 42 ? 1 : OverrideManagerBase::GetID(local, grfid); }
	uint16_t Requested(uint16_t id) const { return this->entity_overrides[id]; }
};
TEST_CASE("NewGRF - canonical manager virtual dispatch and reset lifetimes")
{
	TestOverrideManager manager(2, 6, 99);
	CHECK(manager.AddEntityID(5, 17, 1) == 2);
	CHECK(manager.AddEntityID(6, 17, 0) == 4);
	CHECK(manager.AddEntityID(5, 17, 0) == 2);
	CHECK(manager.AddEntityID(42, 17, 0) == 1);
	CHECK(manager.GetGRFID(2) == 17);
	CHECK(manager.GetSubstituteID(2) == 1);
	TestOverrideManager copied = manager;
	copied.ResetMapping();
	CHECK(copied.GetID(5, 17) == 99);
	CHECK(manager.GetID(5, 17) == 2);
	copied = manager;
	CHECK(copied.GetID(5, 17) == 2);
	manager.Add(5, 17, 0);
	manager.Add(6, 18, 0);
	CHECK(manager.Requested(0) == 5);
	manager.ResetOverride();
	CHECK(manager.Requested(0) == 99);
	CHECK(manager.GetID(5, 17) == 2);
	manager.Add(5, 17, 0);
	manager.ResetMapping();
	CHECK(manager.GetID(5, 17) == 99);
	CHECK(manager.Requested(0) == 5);
}
class TestIndustryTileOverrideManager : public IndustryTileOverrideManager {
public:
	using IndustryTileOverrideManager::IndustryTileOverrideManager;
	bool CheckValidNewID(uint16_t id) override { return id != 3 && IndustryTileOverrideManager::CheckValidNewID(id); }
	uint16_t GetID(uint16_t local, uint32_t grfid) const override { return local == 42 ? 1 : IndustryTileOverrideManager::GetID(local, grfid); }
};
TEST_CASE("NewGRF - industry tile manager retains virtual allocation")
{
	TestIndustryTileOverrideManager manager(2, 6, 99);
	CHECK(manager.AddEntityID(5, 17, 1) == 2);
	CHECK(manager.AddEntityID(6, 17, 0) == 4);
	CHECK(manager.AddEntityID(42, 17, 0) == 1);
	IndustryTileOverrideManager water(254, 257, 512);
	CHECK(water.AddEntityID(1, 17, 0) == 254);
	CHECK(water.AddEntityID(2, 17, 0) == 256);
}
static uint32_t IndustryMappingCount(uint8_t local, bool &available)
{
	RustIndustryOwner owner{openttd_rust_industry_new(), openttd_rust_industry_destroy};
	RustIndustryBuilderOwner builder{openttd_rust_industry_builder_new(), openttd_rust_industry_builder_destroy};
	openttd_rust_industry_registry_insert(builder.get(), NUM_INDUSTRYTYPES - 1, 0);
	const OpenTTDIndustryResolverContext context{.owner = owner.get(), .industry = nullptr, .town = nullptr, .tile = INVALID_TILE.base(), .random_bits = 0, .id = 0, .type = 0};
	ResolverObject resolver(nullptr);
	const auto saved = openttd_rust_storage_temp_get(0x100);
	openttd_rust_storage_temp_store(0x100, 0x475475);
	/* Var 67 counts the current industry but excludes it from the closest-distance search. */
	const auto result = openttd_rust_industry_variable(&context, &resolver, 0x67, local, &available, builder.get(), &GetIndustryResolverServices());
	openttd_rust_storage_temp_store(0x100, saved);
	return result;
}
TEST_CASE("NewGRF - temporary managers preserve canonical industry owner")
{
	const auto saved = _industry_mngr.mappings[NUM_INDUSTRYTYPES - 1];
	_industry_mngr.mappings[NUM_INDUSTRYTYPES - 1] = {0x475475, 126, 0};
	auto check_owner = [] {
		bool available = true;
		CHECK(IndustryMappingCount(126, available) == 0x1FFFF);
		CHECK(available);
		CHECK(_industry_mngr.GetID(126, 0x475475) == NUM_INDUSTRYTYPES - 1);
	};
	{
		IndustryOverrideManager temporary(2, 6, 99);
		temporary.mappings[3] = {0x475475, 126, 0};
		check_owner();
		IndustryOverrideManager copy = temporary;
		check_owner();
		copy = _industry_mngr;
		CHECK(copy.GetID(126, 0x475475) == NUM_INDUSTRYTYPES - 1);
		copy.mappings[NUM_INDUSTRYTYPES - 1] = {};
		copy.mappings[3] = {0x475475, 126, 0};
		check_owner();
	}
	check_owner();
	_industry_mngr.mappings[NUM_INDUSTRYTYPES - 1] = saved;
}
TEST_CASE("NewGRF - temporary managers preserve canonical tile owner")
{
	std::vector<EntityIDMapping> saved(_industile_mngr.mappings.begin(), _industile_mngr.mappings.end());
	_industile_mngr.ResetMapping();
	GRFFile file;
	file.grfid = 0x475475;
	const OpenTTDIndustryPropertyContext context{&file, file.grfid, 8, 0};
	const uint8_t substitute = 0;
	auto define = [&](uint32_t id) {
		ByteReader reader(&substitute, 1);
		int32_t result = CIR_UNKNOWN;
		CHECK(openttd_rust_industry_tile_properties(id, id + 1, 8, &reader, &context, &GetRustByteReaderServices(), &GetIndustryDefinitionServices(), &GetIndustryPropertyServices(), &GetRustGrfLoadingServices(), &result));
		CHECK(result == CIR_SUCCESS);
		CHECK(_industile_mngr.GetID(id, file.grfid) == NEW_INDUSTRYTILEOFFSET + id);
	};
	{
		IndustryTileOverrideManager temporary(2, 6, 99);
		define(0);
	}
	define(1);
	std::copy(saved.begin(), saved.end(), _industile_mngr.mappings.begin());
}
TEST_CASE("NewGRF - industry mapping is shared across serialized threads")
{
	const auto saved = _industry_mngr.mappings[NUM_INDUSTRYTYPES - 1];
	_industry_mngr.mappings[NUM_INDUSTRYTYPES - 1] = {0x475475, 126, 0};
	uint32_t result = 0;
	uint16_t native_result = IT_INVALID;
	bool available = true;
	std::thread worker([&] {
		result = IndustryMappingCount(126, available);
		native_result = _industry_mngr.GetID(126, 0x475475);
	});
	worker.join();
	_industry_mngr.mappings[NUM_INDUSTRYTYPES - 1] = saved;
	CHECK(result == 0x1FFFF);
	CHECK(available);
	CHECK(native_result == NUM_INDUSTRYTYPES - 1);
}
TEST_CASE("Industry - canonical allocation and history lifetime")
{
	RustIndustryOwner owner(openttd_rust_industry_new(), openttd_rust_industry_destroy);
	RustIndustryVector<Industry::AcceptedCargo, false> accepted(owner.get());
	RustIndustryVector<Industry::ProducedCargo, true> produced(owner.get());
	accepted.emplace_back().waiting = 65535;
	auto &history = accepted[0].GetOrCreateHistory();
	history[THIS_MONTH].accepted = 65535;
	auto *allocation = accepted[0].history.get();
	accepted.reserve(256);
	CHECK(accepted[0].history.get() == allocation);
	CHECK(accepted[0].waiting == 65535);
	CHECK(accepted[0].GetOrCreateHistory()[THIS_MONTH].accepted == 65535);
	accepted.emplace_back().cargo = INVALID_CARGO;
	produced.emplace_back().cargo = 3;
	produced.emplace_back().cargo = INVALID_CARGO;
	openttd_rust_industry_trim(owner.get());
	CHECK(accepted.size() == 1);
	CHECK(produced.size() == 1);
	CHECK(accepted[0].history.get() == allocation);
	accepted.clear();
	CHECK(accepted.empty());
	CHECK(accepted.emplace_back().history == nullptr);
}
TEST_CASE("Industry - construction owner and ascending type registry")
{
	RustIndustryOwner owner{openttd_rust_industry_new(), openttd_rust_industry_destroy};
	auto *metadata = openttd_rust_industry_metadata(owner.get());
	CHECK(metadata->tile == INVALID_TILE.base());
	CHECK(metadata->width == 0);
	CHECK(metadata->owner == INVALID_OWNER);
	CHECK(metadata->founder == INVALID_OWNER);
	CHECK(metadata->exclusive_supplier == INVALID_OWNER);
	CHECK(metadata->exclusive_consumer == INVALID_OWNER);
	std::construct_at(reinterpret_cast<TileArea *>(&metadata->tile), TileIndex(456), 7, 9);
	CHECK(metadata->tile == 456);
	CHECK(metadata->width == 7);
	CHECK(metadata->height == 9);
	const uint8_t text[] = {0, 0xFF, 0x1E, 'a'};
	openttd_rust_industry_set_text(owner.get(), text, sizeof(text));
	auto text_view = openttd_rust_industry_text(owner.get());
	CHECK(text_view.size == sizeof(text));
	CHECK(std::equal(std::begin(text), std::end(text), text_view.data));
	openttd_rust_industry_set_text(owner.get(), nullptr, 0);
	CHECK(openttd_rust_industry_text(owner.get()).size == 0);
	RustIndustryBuilderOwner builder{openttd_rust_industry_builder_new(), openttd_rust_industry_builder_destroy};
	for (uint16_t id : {9, 1, 9, 5}) openttd_rust_industry_registry_insert(builder.get(), 3, id);
	const uint16_t expected[] = {1, 5, 9};
	auto view = openttd_rust_industry_registry_view(builder.get(), 3);
	CHECK(view.size == 3);
	CHECK(std::equal(std::begin(expected), std::end(expected), static_cast<uint16_t *>(view.data)));
	openttd_rust_industry_registry_erase(builder.get(), 3, 5);
	openttd_rust_industry_registry_erase(builder.get(), 3, 99);
	CHECK(openttd_rust_industry_registry_view(builder.get(), 3).size == 2);
	CHECK(openttd_rust_industry_registry_view(builder.get(), 4).size == 0);
	openttd_rust_industry_registry_insert(builder.get(), 3, 5);
	CHECK(openttd_rust_industry_registry_view(builder.get(), 3).size == 3);
	openttd_rust_industry_registry_clear(builder.get(), 3);
	CHECK(openttd_rust_industry_registry_view(builder.get(), 3).size == 0);
}

TEST_CASE("Industry - native generation handoff retains named cancellation")
{
	OpenTTDIndustryGenerationContext context;
	CHECK(RunGenerationProgress(&context, [] { }));
	CHECK(context.cancellation == nullptr);
	CHECK_FALSE(RunGenerationProgress(&context, [] { throw AbortGenerateWorldSignal(); }));
	REQUIRE(context.cancellation != nullptr);
	bool aborted = false;
	try { std::rethrow_exception(context.cancellation); }
	catch (AbortGenerateWorldSignal &) { aborted = true; }
	CHECK(aborted);
}

namespace {
struct IndustryGenerationFixture {
	static inline uint8_t company;
	static inline uint32_t draws, created, increases, sets;
	static inline int cancel_set, cancel_increase;
	static inline int shell;

	static OpenTTDIndustryServices World()
	{
		auto world = GetIndustryRustServices();
		world.create = [](uint8_t, uint8_t creation) noexcept -> void * { CHECK(creation == 0); ++created; return &shell; };
		world.type_count = [](uint8_t) noexcept -> uint16_t { return 0; };
		static auto resolver = GetIndustryResolverServices();
		static auto construction = GetIndustryConstructionServices();
		construction.callback_mask = [](const OpenTTDIndustrySpecHandle *) noexcept -> uint16_t { return 0; };
		resolver.construction = &construction;
		world.resolver = &resolver;
		world.scale_by_map_size = [](uint32_t n) noexcept -> uint32_t { return n; };
		static auto shared = GetRustSharedServices();
		shared.random = [](void *) noexcept -> uint32_t { ++draws; return 0; };
		world.shared = &shared;
		world.company_none = []() noexcept -> uint8_t { uint8_t previous = company; company = 255; return previous; };
		world.restore_company = [](uint8_t previous) noexcept { CHECK(company == 255); company = previous; };
		return world;
	}
	static OpenTTDIndustryGenerationServices Generation()
	{
		return {
			.spec = [](uint8_t type) noexcept -> const OpenTTDIndustrySpecHandle * { return reinterpret_cast<const OpenTTDIndustrySpecHandle *>(uintptr_t(type) + 1); },
			.spec_behaviour = [](const OpenTTDIndustrySpecHandle *spec) noexcept -> uint32_t { return uintptr_t(spec) == 2 ? 1U << 2 : 0; },
			.info = [](const OpenTTDIndustrySpecHandle *spec) noexcept -> OpenTTDIndustryGenerationInfo { return {1, uintptr_t(spec) <= 2, true}; },
			.coastal = [](const OpenTTDIndustrySpecHandle *) noexcept -> bool { return false; },
			.environment = {.editor = []() noexcept -> bool { return false; }, .freeform_edges = []() noexcept -> bool { return false; }},
			.density = []() noexcept -> uint8_t { return 4; },
			.custom_number = []() noexcept -> uint32_t { return 0; },
			.scale_by_size_1d = [](uint32_t n) noexcept -> uint32_t { return n; },
			.scale_by_land_proportion = [](uint32_t n) noexcept -> uint32_t { return n; },
			.set_progress = [](OpenTTDIndustryGenerationContext *, bool water, uint32_t total) noexcept -> bool { CHECK(company == 9); CHECK(total == 27); ++sets; return cancel_set != int(water); },
			.increase_progress = [](OpenTTDIndustryGenerationContext *, bool) noexcept -> bool { CHECK(company == 255); return int(++increases) != cancel_increase; },
			.missing_warning = [](const OpenTTDIndustrySpecHandle *) noexcept { },
		};
	}
};
}

TEST_CASE("Industry - generation cancellation restores company before return")
{
	using F = IndustryGenerationFixture;
	const auto world = F::World();
	const auto generation = F::Generation();
	for (const auto [cancel_set, cancel_increase, draws, created, increases, sets] : {
		std::tuple{0, -1, 0U, 0U, 0U, 1U},
		std::tuple{-1, 1, 0U, 0U, 1U, 1U},
		std::tuple{-1, 2, 1U, 1U, 2U, 1U},
		std::tuple{1, -1, 26U, 27U, 27U, 2U},
		std::tuple{-1, 28, 26U, 27U, 28U, 2U},
		std::tuple{-1, -1, 52U, 54U, 54U, 2U},
	}) {
		F::company = 9;
		F::draws = F::created = F::increases = F::sets = 0;
		F::cancel_set = cancel_set;
		F::cancel_increase = cancel_increase;
		auto *builder = openttd_rust_industry_builder_new();
		auto *fields = openttd_rust_industry_builder_fields(builder);
		fields->wanted_inds = 123;
		const bool complete = openttd_rust_industry_generate(builder, nullptr, &world, &generation);
		CHECK(complete == (cancel_set == -1 && cancel_increase == -1));
		CHECK(F::company == 9);
		CHECK(F::draws == draws);
		CHECK(F::created == created);
		CHECK(F::increases == increases);
		CHECK(F::sets == sets);
		CHECK(fields->wanted_inds == (complete ? 0 : 123));
		openttd_rust_industry_builder_destroy(builder);
	}
}

TEST_CASE("Industry - shared plain water predicate excludes depot and lock parts")
{
	Map::Allocate(64, 64);
	const auto &shared = GetRustSharedServices();
	TileIndex tile = TileXY(20, 20);
	SetTileType(tile, MP_CLEAR);
	CHECK_FALSE(shared.is_water_tile(tile.base()));
	MakeSea(tile);
	CHECK(shared.is_water_tile(tile.base()));
	MakeCanal(tile, Owner(0), 0);
	CHECK(shared.is_water_tile(tile.base()));
	MakeRiver(tile, 0);
	CHECK(shared.is_water_tile(tile.base()));
	MakeShore(tile);
	CHECK_FALSE(shared.is_water_tile(tile.base()));
	for (DepotPart part : {DepotPart::North, DepotPart::South}) {
		MakeShipDepot(tile, Owner(0), DepotID(0), part, AXIS_X, WaterClass::Sea);
		CHECK_FALSE(shared.is_water_tile(tile.base()));
	}
	for (LockPart part : {LockPart::Lower, LockPart::Middle, LockPart::Upper}) {
		MakeLockTile(tile, Owner(0), part, DIAGDIR_NE, WaterClass::Sea);
		CHECK_FALSE(shared.is_water_tile(tile.base()));
	}
}

namespace IndustryTemporaryFixture {
static std::vector<uint8_t> trace;
static uint16_t result;
static uint8_t slope, refused, version;
static IndustrySpec industry_spec;
static IndustryTileSpec tile_spec;
static OpenTTDIndustry *active_owner;
static OpenTTDIndustryLocationResult Observe(Industry *shell)
{
	CHECK(shell->production_owner.get() == active_owner);
	CHECK(shell->index == IndustryID::Invalid());
	CHECK(shell->location.tile == TileIndex(123));
	CHECK(shell->location.w == 0);
	CHECK(shell->type == 7);
	CHECK(shell->random == 0xABCD);
	CHECK(shell->founder == Owner(2));
	CHECK(shell->accepted.empty());
	CHECK(shell->produced.empty());
	trace.push_back(3);
	OpenTTDIndustryLocationResult output{};
	output.callback = result;
	output.registers[3] = 456;
	return output;
}
}
TEST_CASE("Industry - borrowed stack resolver shell and callback policy")
{
	using namespace IndustryTemporaryFixture;
	auto shared = GetRustSharedServices();
	OpenTTDIndustryServices production{};
	production.shared = &shared;
	OpenTTDIndustryGenerationServices generation{};
	OpenTTDIndustryConstructionServices construction{};
	OpenTTDIndustryLifecycleServices lifecycle{};
	OpenTTDIndustryTileServices tiles{};
	OpenTTDIndustryTilePolicyServices policy{};
	OpenTTDIndustryCommandServices commands{};
	OpenTTDIndustryCostServices costs{};
	OpenTTDIndustryTransactionServices transaction{};
	OpenTTDIndustryConstructionErrors errors{};
	errors.site = 1234;
	generation.spec = [](uint8_t kind) noexcept -> const OpenTTDIndustrySpecHandle * { CHECK(kind == 7); trace.push_back(0); return reinterpret_cast<const OpenTTDIndustrySpecHandle *>(&industry_spec); };
	commands.owner = RustIndustryCanonicalOwner<Industry *>;
	construction.set_town = [](void *shell, const OpenTTDTownHandle *town) noexcept { CHECK(town == nullptr); static_cast<Industry *>(shell)->town = nullptr; };
	transaction.closest_town = [](uint32_t tile, uint32_t threshold) noexcept -> const OpenTTDTownHandle * { CHECK(tile == 123); CHECK(threshold == UINT32_MAX); CHECK(openttd_rust_industry_metadata(active_owner)->random == 0); trace.push_back(2); return nullptr; };
	transaction.temporary_shell = [](OpenTTDIndustry *owner, void *context, OpenTTDIndustryTemporaryVisit visit) noexcept {
		active_owner = owner;
		trace.push_back(1);
		{ Industry shell(owner); shell.index = IndustryID::Invalid(); visit(context, &shell); }
		trace.push_back(5);
		/* The owner still lives after the actual native destructor returns. */
		CHECK(openttd_rust_industry_metadata(owner)->width == 0);
		active_owner = nullptr;
	};
	transaction.resolve_location = [](Industry *shell, uint32_t tile, uint8_t kind, uint32_t seed, uint8_t creation) noexcept -> OpenTTDIndustryLocationResult { CHECK(tile == 123); CHECK(kind == 7); CHECK(seed == 0x12345678); CHECK(creation == 2); CHECK(shell->selected_layout == 2); return Observe(shell); };
	transaction.resolve_shape = [](Industry *shell, uint32_t tile, uint16_t gfx, uint32_t parameter) noexcept -> OpenTTDIndustryLocationResult { CHECK(tile == 456); CHECK(gfx == 8); CHECK(parameter == 0x302); CHECK(shell->selected_layout == 0); return Observe(shell); };
	transaction.location_error = [](CommandCost *out, const OpenTTDIndustryLocationResult *callback, const OpenTTDIndustrySpecHandle *spec) noexcept { CHECK(spec == reinterpret_cast<const OpenTTDIndustrySpecHandle *>(&industry_spec)); CHECK(callback->registers[3] == 456); trace.push_back(4); *out = CommandCost(EXPENSES_OTHER, Money(789)); };
	transaction.shape_error = [](CommandCost *out, const OpenTTDIndustryLocationResult *callback, const IndustryTileSpec *spec) noexcept { CHECK(spec == &tile_spec); CHECK(callback->registers[3] == 456); trace.push_back(4); *out = CommandCost(EXPENSES_OTHER, Money(789)); };
	transaction.tile_grf_version = [](const IndustryTileSpec *spec) noexcept -> uint8_t { CHECK(spec == &tile_spec); trace.push_back(6); return version; };
	shared.tile_slope = [](uint32_t tile) noexcept -> uint8_t { CHECK(tile == 456); trace.push_back(7); return slope; };
	policy.slopes_refused = [](const IndustryTileSpec *spec) noexcept -> uint8_t { CHECK(spec == &tile_spec); trace.push_back(8); return refused; };
	costs.zero = [](CommandCost *out) noexcept { *out = CommandCost(); };
	costs.error = [](CommandCost *out, uint32_t message) noexcept { *out = CommandCost(message); };
	OpenTTDIndustryTransactionContext context{
		.map = {},
		.builder = nullptr, .production = &production, .generation = &generation,
		.construction = &construction, .lifecycle = &lifecycle, .tiles = &tiles,
		.tile_policy = &policy, .commands = &commands, .command_checks = nullptr, .costs = &costs,
		.transaction = &transaction, .errors = &errors,
	};
	const auto pool_count = Industry::GetNumItems();
	for (uint16_t callback : {uint16_t{0}, uint16_t{1}, uint16_t{0xFFFF}}) {
		result = callback; trace.clear(); CommandCost cost;
		openttd_rust_industry_location_check(&cost, 123, 7, 258, 0x12345678, 0xABCD, 2, 2, &context);
		CHECK(cost.Succeeded());
		CHECK(cost.GetCost() == (callback == 0xFFFF ? 0 : 789));
		CHECK(trace == (callback == 0xFFFF ? std::vector<uint8_t>{0, 1, 2, 3, 5} : std::vector<uint8_t>{0, 1, 2, 3, 4, 5}));
		for (uint8_t grf_version : {6, 7, 8}) for (uint8_t current_slope : {0, 16}) {
			version = grf_version; slope = current_slope; refused = 0; trace.clear();
			openttd_rust_industry_shape_check(&cost, 123, 456, &tile_spec, 7, 8, 258, 0xABCD, 2, 3, &context);
			CHECK(cost.Succeeded() == (callback == 0xFFFF ? slope == 0 : version >= 7 || callback != 0));
			CHECK(cost.GetCost() == (callback != 0xFFFF && version >= 7 ? 789 : 0));
			if (callback == 0xFFFF) { CHECK(cost.GetErrorMessage() == (slope == 0 ? INVALID_STRING_ID : errors.site)); CHECK(trace == (std::vector<uint8_t>{1, 3, 7, 8, 5})); }
			else if (version < 7) { CHECK(trace == (std::vector<uint8_t>{1, 3, 6, 5})); }
			else { CHECK(trace == (std::vector<uint8_t>{1, 3, 6, 4, 5})); }
		}
	}
	CHECK(Industry::GetNumItems() == pool_count);
}
#endif

#include "../newgrf_storage.h"
TEST_CASE("NewGRF storage canonical aliases and complete transaction lifetime", "[industry][storage]")
{
	using Modern = PersistentStorageArray<int32_t, 256>;
	OldPersistentStorage legacy;
	Modern modern;
#ifdef WITH_RUST
	CHECK(modern.storage.data() == RustStorageData(modern.rust_storage));
	CHECK(legacy.storage.data() == RustStorageData(legacy.rust_storage));
	ABI_LAYOUT_CHECK;
#include "abi-layout-storage.inc"
#endif
	legacy.storage[15] = -7;
	std::copy(legacy.storage.begin(), legacy.storage.end(), modern.storage.begin());
	CHECK(modern.GetValue(15) == -7);
	CHECK(modern.GetValue(16) == 0);
	modern.StoreValue(15, -8);
	modern.StoreValue(15, -9);
	modern.StoreValue(256, 99);
	legacy.StoreValue(15, 4);
	BasePersistentStorageArray::SwitchMode(PSM_ENTER_GAMELOOP);
	CHECK(modern.GetValue(15) == -7);
	CHECK(legacy.GetValue(15) == -7);
	modern.StoreValue(255, INT32_MIN);
	BasePersistentStorageArray::SwitchMode(PSM_ENTER_COMMAND);
	modern.StoreValue(1, INT32_MAX);
	BasePersistentStorageArray::SwitchMode(PSM_LEAVE_COMMAND);
	BasePersistentStorageArray::SwitchMode(PSM_ENTER_TESTMODE);
	modern.StoreValue(255, 7);
	BasePersistentStorageArray::SwitchMode(PSM_LEAVE_TESTMODE);
	CHECK(modern.GetValue(255) == INT32_MIN);
	BasePersistentStorageArray::SwitchMode(PSM_LEAVE_GAMELOOP);
	modern.StoreValue(1, -3);
	modern.ClearChanges(); // Same operation the PSAC saver uses.
	CHECK(modern.GetValue(1) == INT32_MAX);
#ifdef WITH_RUST
	static uint discarded;
	discarded = 0;
	openttd_rust_storage_switch_mode(PSM_LEAVE_GAMELOOP, true, RustAssertCondition, [](void *) noexcept { ++discarded; });
	CHECK(discarded == 1); // Saving rolls back but retains registry membership.
#endif
	{ auto pending = std::make_unique<Modern>(); pending->StoreValue(0, 1); }
	BasePersistentStorageArray::SwitchMode(PSM_LEAVE_GAMELOOP, true);
	TemporaryStorageArray<int32_t, 0x110> temporary;
	temporary.StoreValue(0x10F, -123);
	CHECK(temporary.GetValue(0x10F) == -123);
	for (uint i = 0; i < 65535; ++i) temporary.ClearChanges();
	CHECK(temporary.GetValue(0x10F) == 0);
	temporary.StoreValue(0x10F, 456);
	CHECK(temporary.GetValue(0x10F) == 456);
	temporary.ClearChanges();
	CHECK(temporary.GetValue(0x110) == 0);
}

TEST_CASE("Industry - specification price widths and saturation")
{
	IndustrySpec spec{};
	spec.life_type = {IndustryLifeType::Organic};
	Backup<uint8_t> setting(_settings_game.construction.raw_industry_construction, 1);
	for (Money price : {Money(INT64_MIN), Money(-12345), Money(0), Money(12345), Money(INT64_MAX)}) {
		Backup<Money> raw(_price[PR_BUILD_INDUSTRY_RAW], price), clear(_price[PR_CLEAR_INDUSTRY], price);
		for (uint8_t multiplier : {uint8_t(0), uint8_t(1), uint8_t(255)}) {
			spec.cost_multiplier = multiplier;
			CHECK(spec.GetConstructionCost() == ((price * multiplier) >> 8));
		}
		for (uint32_t multiplier : {0U, 1U, 0x7FFFFFFFU, 0x80000000U, UINT32_MAX}) {
			spec.removal_cost_multiplier = multiplier;
			CHECK(spec.GetRemovalCost() == ((price * multiplier) >> 8));
		}
	}
}
