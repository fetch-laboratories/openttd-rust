/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/* Native loaders remain compiled unchanged so fixtures exercise their real publication phases. */
#include "newgrf.cpp"
#include "newgrf/newgrf_act0_cargo.cpp"
#include "economy_func.h"
#include "newgrf_profiling.h"

static void CargoRequire(bool good, int line) { if (!good) { fmt::print(stderr, "CARGO failure {}\n", line); std::abort(); } }
#define CARGO_REQUIRE(good) CargoRequire(good, __LINE__)
static void CargoSnapshot(std::string_view phase)
{
	fmt::print(stderr, "CARGO registry {} {} {}", phase, _cargo_mask, _standard_cargo_mask);
	for (uint8_t order : _sorted_cargo_types) fmt::print(stderr, " {}", order);
	for (const CargoSpec *cs : _sorted_cargo_specs) fmt::print(stderr, " s{}", cs->Index());
	for (const CargoSpec *cs : _sorted_standard_cargo_specs) fmt::print(stderr, " n{}", cs->Index());
	for (uint8_t effect = 0; effect < NUM_TPE; ++effect) for (const CargoSpec *cs : CargoSpec::town_production_cargoes[effect]) fmt::print(stderr, " t{}:{}", effect, cs->Index());
	fmt::print(stderr, "\n");
}
static bool CargoRegistryScenario(std::span<std::string_view> args)
{
	if (args[1] != "cargo-registry") return false;
	auto *saved_file = _cur_gps.grffile;
	for (LandscapeType climate : {LandscapeType::Temperate, LandscapeType::Arctic, LandscapeType::Tropic, LandscapeType::Toyland}) {
		SetupCargoForClimate(climate);
		FinaliseCargoArray(); InitializeSortedCargoSpecs();
		CargoSnapshot("climate");
		GRFFile file;
		_cur_gps.grffile = &file;
		for (uint8_t version : {6, 7, 8}) {
			file.grf_version = version;
			for (uint8_t cargo = 0; cargo < 64; ++cargo) for (bool usebit : {false, true}) fmt::print(stderr, "CARGO forward {} {} {} {}\n", version, cargo, usebit, GetCargoTranslation(cargo, &file, usebit));
			BuildCargoTranslationMap();
			for (uint8_t cargo = 0; cargo < 64; ++cargo) fmt::print(stderr, "CARGO inverse {} {} {}\n", version, cargo, file.cargo_map[cargo]);
		}
		const CargoSpec original = *CargoSpec::Get(0);
		*CargoSpec::Get(20) = original;
		BuildCargoLabelMap(); CARGO_REQUIRE(GetCargoTypeByLabel(original.label) == 0);
		CargoSpec::Get(0)->bitnum = INVALID_CARGO_BITNUM;
		BuildCargoLabelMap(); CARGO_REQUIRE(GetCargoTypeByLabel(original.label) == 20 && IsDefaultCargo(20));
		CargoSpec::Get(21)->bitnum = 20; CargoSpec::Get(21)->label = CargoLabel{};
		CargoSpec::Get(22)->bitnum = 21; CargoSpec::Get(22)->label = CT_INVALID;
		BuildCargoLabelMap(); CARGO_REQUIRE(GetCargoTypeByLabel(CargoLabel{}) == INVALID_CARGO && GetCargoTypeByLabel(CT_INVALID) == INVALID_CARGO);
		/* Use the real Action0 handler, including rebuilding before fallback installation. */
		std::array<uint8_t, 4> label{'T', 'E', 'S', 'T'};
		ByteReader label_reader(label.data(), label.size());
		CARGO_REQUIRE(CargoReserveInfo(23, 24, 0x17, label_reader) == CIR_SUCCESS);
		CARGO_REQUIRE(file.cargo_list_is_fallback && file.cargo_list.size() >= 24 && file.cargo_list[23] == CargoLabel{0x54455354});
		std::array<uint8_t, 1> bitnum{22}; ByteReader bit_reader(bitnum.data(), bitnum.size());
		CARGO_REQUIRE(CargoReserveInfo(23, 24, 0x08, bit_reader) == CIR_SUCCESS);
		CARGO_REQUIRE(GetCargoTypeByLabel(CargoLabel{0x54455354}) == 23 && HasBit(_cargo_mask, 23));
		file.cargo_list.assign(300, CT_INVALID); file.cargo_list[257] = original.label; file.cargo_list[258] = original.label;
		file.cargo_list_is_fallback = false; file.grf_version = 8;
		BuildCargoTranslationMap(); CARGO_REQUIRE(file.cargo_map[20] == 1);
		CARGO_REQUIRE(GetCargoTranslation(1, &file) == INVALID_CARGO);
		file.cargo_list = {original.label, original.label};
		BuildCargoTranslationMap(); CARGO_REQUIRE(file.cargo_map[20] == 0 && GetCargoTranslation(1, &file) == 20);
		/* Same names and bitnums retain the native unstable sorter and its ties. */
		*CargoSpec::Get(24) = original; CargoSpec::Get(24)->classes.Set(CargoClass::Special);
		CargoSpec::Get(24)->town_production_effect = TPE_MAIL;
		FinaliseCargoArray(); InitializeSortedCargoSpecs();
		CargoSnapshot("changed");
		auto old_order = _sorted_cargo_types[20];
		CargoSpec::Get(20)->bitnum = INVALID_CARGO_BITNUM;
		BuildCargoLabelMap(); InitializeSortedCargoSpecs();
		CARGO_REQUIRE(_sorted_cargo_types[20] == old_order);
		CARGO_REQUIRE(CargoSpec::town_production_cargoes[TPE_MAIL].back()->Index() == 24);
		CargoSnapshot("disabled");
	}
	_cur_gps.grffile = saved_file;
	ReloadNewGRFData(); CargoSnapshot("reloaded");
	fmt::print(stderr, "CARGO registry-controls true\n");
	return true;
}
static void CargoProfitFixture()
{
	CargoType cargo = GetCargoTypeByLabel(CT_COAL);
	CARGO_REQUIRE(cargo != INVALID_CARGO);
	CargoSpec *cs = CargoSpec::Get(cargo);
	CargoSpec saved = *cs;
	GRFFile file; file.grf_version = 8;
	CallbackResultSpriteGroup result(0x4001);
	DeterministicSpriteGroup program; program.size = DSG_SIZE_DWORD; program.var_scope = VSG_SCOPE_SELF;
	auto read = [](uint8_t variable, uint32_t mask = UINT32_MAX, DeterministicSpriteGroupAdjustOperation op = DSGA_OP_RST) { return DeterministicSpriteGroupAdjust{.operation = op, .type = DSGA_TYPE_NONE, .variable = variable, .parameter = 0, .shift_num = 0, .and_mask = mask}; };
	program.adjusts = {read(0x18), read(0x1A, 0x100, DSGA_OP_STO), read(0x10), read(0x1A, 0x101, DSGA_OP_STO), read(0x0C), read(0x1A, 0x102, DSGA_OP_STO)};
	program.default_result = {false, &result};
	cs->grffile = &file; cs->group = &program; cs->callback_mask.Set(CargoCallbackMask::ProfitCalc);
	Randomizer before = _random;
	Money income = GetTransportedGoodsIncome(31, 17, 5, cargo);
	ResolverObject observer(nullptr);
	fmt::print(stderr, "CARGO profit {} {} {} {} {} {}\n", income.base(), observer.GetRegister(0x100), observer.GetRegister(0x101), observer.GetRegister(0x102), _random.state[0], _random.state[1]);
	CARGO_REQUIRE(observer.GetRegister(0x100) == (17 | (31 << 16) | (5 << 24)) && observer.GetRegister(0x101) == 0 && observer.GetRegister(0x102) == CBID_CARGO_PROFIT_CALC);
	CARGO_REQUIRE(_random.state[0] == before.state[0] && _random.state[1] == before.state[1]);
	std::array<int32_t, 3> regs{-1, -2, -3};
	CARGO_REQUIRE(GetCargoCallback(CBID_CARGO_PROFIT_CALC, 7, 19, cs, regs) == 0x4001 && regs[0] == 19 && regs[1] == 7 && regs[2] == CBID_CARGO_PROFIT_CALC);
	cs->group = nullptr; regs.fill(-7);
	CARGO_REQUIRE(GetCargoCallback(CBID_CARGO_PROFIT_CALC, 7, 19, cs, regs) == CALLBACK_FAILED && regs[0] == -7 && GetCustomCargoSprite(cs) == 0);
	ResultSpriteGroup sprite(17, 0); cs->group = &sprite;
	CARGO_REQUIRE(GetCustomCargoSprite(cs) == 0); sprite.num_sprites = 1;
	CARGO_REQUIRE(GetCustomCargoSprite(cs) == 17);
	*cs = saved;
	fmt::print(stderr, "CARGO profit-controls true\n");
}
#undef CARGO_REQUIRE
