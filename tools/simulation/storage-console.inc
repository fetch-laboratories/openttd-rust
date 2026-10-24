/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/* Real resolver facades and native associations, without an external GRF program. */
#include "newgrf_town.h"
#include "newgrf_industries.h"
#include "newgrf_airport.h"
#include "industry.h"
#include "industrytype.h"
#include "newgrf_industrytiles.h"
#include "newgrf_config.h"
/* Executable native parsed groups, rebuilt for both initial and post-load runs.
 * The normal resolver methods provide results and storage effects; neither
 * implementation is replaced by a tracing evaluator. */
static void CommonAction2(Industry *industry, GRFFile &grf)
{
	auto Require = [](bool good) { if (!good) std::abort(); };
	std::array<int32_t, 256> saved;
	for (uint i = 0; i < saved.size(); ++i) saved[i] = industry->psa->GetValue(i);
	std::vector<std::unique_ptr<SpriteGroup>> nodes;
	auto Keep = [&]<typename T>(T *g) { nodes.emplace_back(g); return g; };
	auto Adjust = [](uint8_t variable, DeterministicSpriteGroupAdjustOperation operation = DSGA_OP_RST, uint32_t mask = UINT32_MAX, uint8_t parameter = 0) {
		DeterministicSpriteGroupAdjust a; a.variable = variable; a.operation = operation; a.and_mask = mask; a.parameter = parameter; return a;
	};
	auto Det = [&](DeterministicSpriteGroupSize size = DSG_SIZE_DWORD, VarSpriteGroupScope scope = VSG_SCOPE_SELF) {
		auto *g = Keep(new DeterministicSpriteGroup()); g->size = size; g->var_scope = scope; g->default_result.calculated_result = true; return g;
	};
	IndustriesResolverObject ro(industry->location.tile, industry, industry->type);
	ro.grffile = &grf;
	ro.callback_param1 = 0x8005; ro.callback_param2 = 9;
	auto *procedure = Det();
	procedure->adjusts = {Adjust(0x10), Adjust(0x1A, DSGA_OP_STO, 14), Adjust(0x1A, DSGA_OP_STOP, 15)};
	auto *outer = Det();
	auto call = Adjust(0x7E, DSGA_OP_ADD); call.subroutine = procedure;
	outer->adjusts = {Adjust(0x18), call, Adjust(0x7B, DSGA_OP_RST, UINT32_MAX, 0x7D), Adjust(0x1C, DSGA_OP_ADD), Adjust(0x1A, DSGA_OP_STO, 0x100)};
	ro.root_spritegroup = outer;
	std::array<int32_t, 1> regs{-7};
	Require(ro.ResolveCallback(regs) == 10 && regs[0] == 0x1000A && ro.last_value == 0x1000A && ro.GetRegister(14) == 0x8005 && industry->psa->GetValue(15) == 0x8005);
	/* Nested static Resolve shares registers/last; DoResolve alone clears them. */
	auto *read = Det(); read->adjusts = {Adjust(0x7D, DSGA_OP_RST, UINT32_MAX, 14)};
	Require(std::get<CallbackResult>(SpriteGroup::Resolve(read, ro, false)) == 5 && ro.last_value == 0x8005);
	ro.root_spritegroup = read; Require(ro.ResolveCallback({}) == 0 && ro.GetRegister(14) == 0);
	/* Unavailable variable uses error_group, including the null alternative. */
	auto *error = Keep(new CallbackResultSpriteGroup(417));
	auto *missing = Det(); missing->adjusts = {Adjust(0xFF)}; missing->error_group = error;
	ro.root_spritegroup = missing; Require(ro.ResolveCallback({}) == 417);
	missing->error_group = nullptr; Require(ro.ResolveCallback({}) == CALLBACK_FAILED);
	/* Linear and >4 lower_bound paths: inclusive ends, gaps and default. */
	auto *ranges = Det(); ranges->adjusts = {Adjust(0x10)}; ranges->default_result = {false, error};
	uint64_t digest = 0;
	for (uint count : {4U, 5U}) {
		ranges->ranges.clear();
		for (uint i = 0; i < count; ++i) ranges->ranges.push_back({{false, Keep(new CallbackResultSpriteGroup(100 + i))}, i * 3, i * 3 + 1});
		for (uint value = 0; value <= count * 3; ++value) {
			ro.callback_param1 = value; ro.root_spritegroup = ranges;
			uint16_t result = ro.ResolveCallback({});
			Require(result == (value / 3 < count && value % 3 <= 1 ? 100 + value / 3 : 417));
			digest = digest * 31337 + result;
		}
	}
	/* All arithmetic operations/types at each width, including zero division,
	 * signed promotions, full-width wrapping and masked shift/rotate counts. */
	auto *arithmetic = Det();
	for (auto size : {DSG_SIZE_BYTE, DSG_SIZE_WORD, DSG_SIZE_DWORD}) {
		arithmetic->size = size;
		uint32_t mask = size == DSG_SIZE_BYTE ? UINT8_MAX : size == DSG_SIZE_WORD ? UINT16_MAX : UINT32_MAX;
		for (uint type = DSGA_TYPE_NONE; type <= DSGA_TYPE_MOD; ++type) {
			for (uint op = DSGA_OP_ADD; op <= DSGA_OP_SAR; ++op) {
				for (auto [last, value] : std::array<std::pair<uint32_t, uint32_t>, 5>{{{0x7F, 0x80}, {UINT32_MAX, 0}, {0x80000000, 0xFFFFFFFD}, {123, 31}, {0xFFFF, 0x1001}}}) {
					ro.callback_param1 = value; ro.callback_param2 = last;
					auto a = Adjust(0x10, static_cast<DeterministicSpriteGroupAdjustOperation>(op), mask);
					a.type = static_cast<DeterministicSpriteGroupAdjustType>(type); a.add_val = 127; a.divmod_val = 3;
					arithmetic->adjusts = {Adjust(0x18), a}; ro.root_spritegroup = arithmetic;
					digest = digest * 31337 + ro.ResolveCallback({}) + ro.last_value;
				}
			}
		}
	}
	/* Callback-only register copying and all terminal alternatives, including
	 * real loaded/loading fallback and null. */
	auto *sprite = Keep(new ResultSpriteGroup(17, 1));
	auto *layout = Keep(new TileLayoutSpriteGroup());
	auto *production = Keep(new IndustryProductionSpriteGroup());
	auto *real = Keep(new RealSpriteGroup());
	ResolverObject base(&grf); base.root_spritegroup = real;
	Require(base.Resolve<ResultSpriteGroup>() == nullptr);
	real->loading = {sprite}; Require(base.Resolve<ResultSpriteGroup>() == sprite);
	real->loaded = {layout}; Require(base.Resolve<TileLayoutSpriteGroup>() == layout);
	real->loaded = {production}; Require(base.Resolve<IndustryProductionSpriteGroup>() == production);
	base.root_spritegroup = sprite; regs[0] = -77;
	Require(base.ResolveCallback(regs) == CALLBACK_FAILED && regs[0] == -77);
	/* ANY/ALL, zero-trigger ALL, scope OR masks, uint8 result indexing and no
	 * RNG draws. Town parent scopes have no random bits; industry SELF does. */
	auto *random = Keep(new RandomizedSpriteGroup()); random->var_scope = VSG_SCOPE_SELF; random->lowest_randbit = 2; random->triggers = 3;
	random->groups = {error, error, error, error};
	/* Use an actual industry resolver for defined self/parent scopes even if
	 * the industry's rectangular origin is not one of its industry tiles. */
	struct TriggerResolver : IndustriesResolverObject {
		using IndustriesResolverObject::IndustriesResolverObject;
		void Waiting(uint32_t v) { this->waiting_random_triggers = v; }
		uint32_t Used() const { return this->used_random_triggers; }
	} trigger(industry->location.tile, industry, industry->type, 0, CBID_RANDOM_TRIGGER);
	trigger.grffile = &grf; trigger.root_spritegroup = random;
	std::array<uint32_t, 4> rng{_random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]};
	for (auto mode : {RSG_CMP_ANY, RSG_CMP_ALL}) {
		random->cmp_mode = mode;
		for (uint waiting : {0U, 1U, 3U}) {
			trigger.Waiting(waiting); trigger.ResolveRerandomisation();
			bool matched = mode == RSG_CMP_ANY ? waiting != 0 : waiting == 3;
			Require(trigger.Used() == (matched ? waiting : 0) && trigger.GetReseedSum() == (matched ? 12U : 0U));
		}
	}
	random->cmp_mode = RSG_CMP_ALL; random->triggers = 0; trigger.Waiting(0);
	trigger.ResolveRerandomisation(); Require(trigger.Used() == 0 && trigger.reseed[VSG_SCOPE_SELF] == 12);
	random->var_scope = VSG_SCOPE_PARENT; random->lowest_randbit = 6;
	SpriteGroup::Resolve(random, trigger, false); Require(trigger.reseed[VSG_SCOPE_SELF] == 12 && trigger.reseed[VSG_SCOPE_PARENT] == 192 && trigger.GetReseedSum() == 204);
	Require(_random.state[0] == rng[0] && _random.state[1] == rng[1] && _interactive_random.state[0] == rng[2] && _interactive_random.state[1] == rng[3]);
	/* Complete globals, including Action7/9's public shared entry, all dates,
	 * old/new snow scaling, rail-cost selection and native palette preconditions. */
	_grfconfig.push_back(std::make_unique<GRFConfig>("common-action2")); _grfconfig.back()->ident.grfid = grf.grfid;
	for (uint8_t version : {7, 8}) {
		grf.grf_version = version;
		for (uint8_t climate : {0, 1, 2, 3}) {
			Backup<LandscapeType> landscape(_settings_game.game_creation.landscape, static_cast<LandscapeType>(climate));
			for (int32_t year : {0, 1900, 1920, 2000, 2090, 2400}) {
				Backup<TimerGameCalendar::Year> current_year(TimerGameCalendar::year, TimerGameCalendar::Year{year});
				Backup<TimerGameCalendar::Date> date(TimerGameCalendar::date, TimerGameCalendar::ConvertYMDToDate(TimerGameCalendar::Year{year}, 1, 28));
				for (uint8_t variable : {0x00, 0x01, 0x02, 0x03, 0x06, 0x09, 0x0A, 0x0B, 0x0D, 0x0E, 0x0F, 0x11, 0x12, 0x1A, 0x1B, 0x1D, 0x1E, 0x20, 0x22, 0x23, 0x24}) {
					uint32_t value = 0; Require(GetGlobalVariable(variable, &value, &grf));
					arithmetic->size = DSG_SIZE_DWORD; arithmetic->adjusts = {Adjust(variable)}; base.root_spritegroup = arithmetic;
					Require(base.ResolveCallback({}) == (value & 0x7FFF) && base.last_value == value);
					digest = digest * 31337 + value;
				}
			}
		}
	}
	_grfconfig.pop_back();
	uint32_t version; Require(GetGlobalVariable(0x21, &version, &grf) && version == _openttd_newgrf_version);
	uint32_t unknown = 417; Require(!GetGlobalVariable(0xFF, &unknown, &grf) && unknown == 417);
	for (uint i = 0; i < saved.size(); ++i) industry->psa->StoreValue(i, saved[i]);
	fmt::print(stderr, "STORAGE action2 nested ranges arithmetic terminals reseed globals {}\n", digest);
}
static bool StorageScenario(std::span<std::string_view> args)
{
	auto Require = [](bool good) { if (!good) std::abort(); };
	if (args[1] == "storage-load") {
		Require(PersistentStorage::GetNumItems() == 0);
		auto file = FileHandle::Open(args[2], "rb"); Require(file.has_value());
		_sl.reader = std::make_unique<ReadBuffer>(std::make_shared<FileReader>(std::move(*file)));
		_sl.action = SLA_LOAD; _sl_version = args[3] == "legacy16" ? SLV_EXTEND_RAILTYPES : SAVEGAME_VERSION; _next_offs = 0;
		bool failed = false;
		try { SlLoadChunk(*SlFindChunkHandler('PSAC')); } catch (const std::exception &) { failed = true; }
		if (args[3] == "header") {
			Require(failed && PersistentStorage::GetNumItems() == 0);
		} else if (args[3] == "partial") {
			Require(failed && PersistentStorage::GetNumItems() == 2);
			auto *first = PersistentStorage::Get(PersistentStorageID{0}), *partial = PersistentStorage::Get(PersistentStorageID{1});
			Require(first->grfid == 0x39312345 && first->GetValue(0) == INT32_MIN && first->GetValue(255) == INT32_MAX);
			Require(partial->grfid == 0xAABBCCDD && partial->GetValue(0) == -7 && partial->GetValue(1) == 0);
		} else {
			Require(!failed && PersistentStorage::GetNumItems() == 1);
			auto *legacy = PersistentStorage::Get(PersistentStorageID{0});
			Require(legacy->grfid == 0x39312345 && legacy->GetValue(0) == INT32_MIN && legacy->GetValue(15) == -393 && legacy->GetValue(16) == 0 && legacy->GetValue(255) == 0);
		}
		fmt::print(stderr, "STORAGE load {} true\n", args[3]); std::fflush(stderr); std::_Exit(0);
	}
	Town *town = *Town::Iterate().begin();
	Industry *industry = *Industry::Iterate().begin();
	Station *station = nullptr;
	for (Station *st : Station::Iterate()) if (st->facilities.Test(StationFacility::Airport)) { station = st; break; }
	Require(station != nullptr);
	GRFFile grf; grf.grfid = 0x39312345;
	ResolverObject ro(&grf), anonymous(nullptr);
	TownScopeResolver writable(ro, town, false), readonly(ro, nullptr, true), no_grf(anonymous, town, false);
	IndustriesScopeResolver ind(ro, industry->location.tile, industry, industry->type);
	AirportScopeResolver airport(ro, station->airport.tile, station, nullptr, 0), preview(ro, TileIndex{}, nullptr, nullptr, 0);
	bool available = true;
	if (args[1] == "storage-inspect") {
		ro.SetRegister(0x100, -1);
		Require(town->psa_list.size() == 2 && writable.GetVariable(0x7C, 0, available) == 0x80000000 && writable.GetVariable(0x7C, 255, available) == INT32_MAX);
		Require(industry->psa != nullptr && ind.GetVariable(0x7C, 15, available) == static_cast<uint32_t>(-393) && station->airport.psa != nullptr && station->airport.psa->GetValue(255) == 393);
		Require(industry->psa->feature == GSF_INDUSTRIES && industry->psa->tile == industry->location.tile && station->airport.psa->feature == GSF_AIRPORTS && station->airport.psa->tile == station->airport.tile);
		for (auto *psa : town->psa_list) Require(psa->feature == GSF_FAKE_TOWNS && psa->tile == town->xy);
		CommonAction2(industry, grf);
		fmt::print(stderr, "STORAGE reloaded true\n");
		return true;
	}
	Require(town->psa_list.empty() && industry->psa == nullptr && station->airport.psa == nullptr);
	Require(ind.GetVariable(0x7C, 15, available) == 0);
	BasePersistentStorageArray::SwitchMode(PSM_LEAVE_GAMELOOP, true);
	BasePersistentStorageArray::SwitchMode(PSM_ENTER_COMMAND);
	preview.StorePSA(0, 7); readonly.StorePSA(0, 7); no_grf.StorePSA(0, 7);
	ro.SetRegister(0x100, 123); writable.StorePSA(0, 7);
	ind.StorePSA(0, 0); airport.StorePSA(0, 0);
	{ Backup<IndustryID> invalid(industry->index, IndustryID::Invalid()); ind.StorePSA(0, 7); }
	Require(town->psa_list.empty() && industry->psa == nullptr && station->airport.psa == nullptr);
	/* Airports without a GRF create the default-GRF array. */
	AirportScopeResolver default_airport(anonymous, station->airport.tile, station, nullptr, 0);
	default_airport.StorePSA(256, 7);
	Require(station->airport.psa != nullptr && station->airport.psa->grfid == 0);
	delete station->airport.psa; station->airport.psa = nullptr;
	/* Nonzero out-of-range writes allocate industry/airport, town even for zero. */
	ro.SetRegister(0x100, -1); writable.StorePSA(256, 0); ind.StorePSA(256, 7); airport.StorePSA(256, 7);
	Require(town->psa_list.size() == 1 && industry->psa != nullptr && station->airport.psa != nullptr);
	Require(industry->psa->grfid == GetIndustrySpec(industry->type)->grf_prop.grfid && station->airport.psa->grfid == grf.grfid);
	writable.StorePSA(0, INT32_MIN); writable.StorePSA(255, INT32_MAX); ind.StorePSA(15, -393); airport.StorePSA(255, 393);
	Require(ind.GetVariable(0x7C, 0, available) == 0 && ind.GetVariable(0x7C, 15, available) == static_cast<uint32_t>(-393));
	Require(ind.GetVariable(0x7C, 256, available) == 0 && ind.GetVariable(0x7C, UINT32_MAX, available) == 0);
	/* Invalid-ID construction scopes can read existing PSA, but cannot write it. */
	{
		Backup<IndustryID> invalid(industry->index, IndustryID::Invalid());
		IndustriesScopeResolver temporary(ro, industry->location.tile, industry, industry->type);
		temporary.StorePSA(15, 7);
		Require(temporary.GetVariable(0x7C, 15, available) == static_cast<uint32_t>(-393));
	}
	/* Another GRF gets a distinct array; explicit GRFID reads may cross GRFs. */
	GRFFile other; other.grfid = 0x39354321;
	TownResolverObject second(&other, town, false); second.SetRegister(0x100, -1); second.town_scope.StorePSA(0, 99);
	ro.SetRegister(0x100, other.grfid); writable.StorePSA(0, 77);
	Require(writable.GetVariable(0x7C, 0, available) == 99);
	anonymous.SetRegister(0x100, grf.grfid); Require(no_grf.GetVariable(0x7C, 0, available) == 0x80000000);
	anonymous.SetRegister(0x100, -1); Require(no_grf.GetVariable(0x7C, 0, available) == 0);
	ro.SetRegister(0x100, -1); Require(writable.GetVariable(0x7C, 0, available) == 0x80000000);
	BasePersistentStorageArray::SwitchMode(PSM_LEAVE_COMMAND);
	BasePersistentStorageArray::SwitchMode(PSM_ENTER_TESTMODE);
	writable.StorePSA(0, 1); ind.StorePSA(15, 2); airport.StorePSA(255, 3);
	Require(ind.GetVariable(0x7C, 15, available) == 2);
	BasePersistentStorageArray::SwitchMode(PSM_LEAVE_TESTMODE);
	Require(writable.GetVariable(0x7C, 0, available) == 0x80000000 && ind.GetVariable(0x7C, 15, available) == static_cast<uint32_t>(-393) && station->airport.psa->GetValue(255) == 393);
	/* Destroy a registered array and require native pool ID reuse. */
	Require(PersistentStorage::CanAllocateItem());
	auto *pending = new PersistentStorage(0, GSF_INVALID, TileIndex{}); auto id = pending->index; pending->StoreValue(0, 1); delete pending;
	Require(PersistentStorage::CanAllocateItem());
	auto *reused = new PersistentStorage(0, GSF_INVALID, TileIndex{}); Require(reused->index == id); delete reused;
	CommonAction2(industry, grf);
	/* Real PSAC saving restores temporary payloads; the next switch still visits them. */
	writable.StorePSA(0, 4); ind.StorePSA(15, 5); airport.StorePSA(255, 6);
	Require(args.size() == 3 && SaveOrLoad(args[2], SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) == SL_OK);
	Require(writable.GetVariable(0x7C, 0, available) == 0x80000000 && ind.GetVariable(0x7C, 15, available) == static_cast<uint32_t>(-393) && station->airport.psa->GetValue(255) == 393);
	BasePersistentStorageArray::SwitchMode(PSM_LEAVE_GAMELOOP, true);
	fmt::print(stderr, "STORAGE policies rollback reuse saved true\n");
	return true;
}
