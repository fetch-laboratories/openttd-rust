/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file object-definitions-console.hpp Production object parsing and unsaved publication observations. */
#include "newgrf/newgrf_internal.h"
#include "newgrf/newgrf_stringmapping.h"
#include "newgrf_badge.h"
#include "strings_func.h"

static ObjectSpec *FileObject(GRFFile &file, size_t id)
{
#ifdef WITH_RUST
	return file.objectspec[id];
#else
	return file.objectspec[id].get();
#endif
}
/** Direct production Activation retains partial state which a disabled file/save cannot show. */
static void ObjectPartialDefinitions()
{
	GRFFile file; file.grfid = 0x63800001; file.grf_version = 8;
	AutoRestoreBackup context(_cur_gps.grffile, &file);
	AutoRestoreBackup two(_loaded_newgrf_features.has_2CC, false);
	auto parse = [&](const char *name, uint first, uint last, int prop, std::initializer_list<uint8_t> bytes) {
		std::vector<uint8_t> data(bytes); data.push_back(0); ByteReader reader(data.data(), bytes.size());
		int result = -1; bool failed = false;
		try { result = GrfChangeInfoHandler<GSF_OBJECTS>::Activation(first, last, prop, reader); } catch (const OTTDByteReaderSignal &) { failed = true; }
		fmt::print(stderr, "OBJECT definition partial {} {} {} {} {}\n", name, result, failed, reader.Remaining(), file.objectspec.size());
	};
	parse("limit", 0, 64001, 0x08, {});
	parse("ignore-byte", 0, 2, 0x0B, {1, 2});
	parse("ignore-word", 0, 2, 0x09, {1, 2, 3, 4});
	parse("ignore-dword", 0, 2, 0x0E, {1, 2, 3, 4, 5, 6, 7, 8});
	parse("ignore-badges", 0, 2, 0x19, {1, 0, 3, 0, 0, 0});
	parse("allocation-before-read", 1, 2, 0x08, {});
	ObjectSpec *spec = FileObject(file, 1);
	fmt::print(stderr, "OBJECT definition partial default {} {} {} {}\n", spec != nullptr, spec->size, spec->views, spec->animation.speed);
	parse("class", 1, 2, 0x08, {'P', '0', '0', '1'}); ObjectClassID old_class = spec->class_index;
	parse("class-name", 1, 2, 0x09, {0, 0});
	parse("reclass", 1, 2, 0x08, {'P', '0', '0', '2'});
	parse("object-name", 1, 2, 0x0A, {0, 0});
	parse("unknown-grow", 0, 128, 0xFF, {});
	fmt::print(stderr, "OBJECT definition partial identity {}\n", spec == FileObject(file, 1));
	parse("size", 1, 2, 0x0C, {0x20}); parse("views", 1, 2, 0x17, {3});
	parse("build", 1, 2, 0x0D, {77}); parse("clear", 1, 2, 0x14, {19}); parse("build-replace", 1, 2, 0x0D, {78});
	parse("date", 1, 2, 0x0E, {255, 255, 255, 255}); parse("end", 1, 2, 0x0F, {0, 0, 0, 128});
	parse("two", 1, 2, 0x10, {0, 1}); parse("flags-replace", 1, 2, 0x10, {0, 0});
	parse("animation-partial", 1, 2, 0x11, {7}); parse("speed", 1, 2, 0x12, {16}); parse("triggers", 1, 2, 0x13, {0xCD, 0xAB});
	BadgeID old_badge = GetOrCreateBadge("638-old").index, seen_badge = GetOrCreateBadge("638-seen").index;
	file.badge_list = {old_badge, seen_badge};
	parse("badges", 1, 2, 0x19, {1, 0, 0, 0}); parse("badges-partial", 1, 2, 0x19, {2, 0, 1, 0});
	fmt::print(stderr, "OBJECT definition partial values {} {} {} {} {} {} {} {} {} {} {} {} {}\n", spec->size, spec->views, spec->build_cost_multiplier, spec->clear_cost_multiplier, spec->introduction_date.base(), spec->end_of_life_date.base(), spec->flags.base(), _loaded_newgrf_features.has_2CC, spec->animation.frames, to_underlying(spec->animation.status), spec->animation.speed, spec->animation.triggers.base(), spec->badges.size());
	fmt::print(stderr, "OBJECT definition partial badge-state {} {}\n", spec->badges[0] == old_badge, GetBadge(seen_badge)->features.Test(GSF_OBJECTS));
	FinaliseStringMapping();
	fmt::print(stderr, "OBJECT definition partial names {} {} {} {}\n", ObjectClass::Get(old_class)->global_id, ObjectClass::Get(old_class)->name, ObjectClass::Get(spec->class_index)->name, spec->name);
}

static void ObjectDefinitionState(std::string_view phase)
{
	fmt::print(stderr, "OBJECT definition {} rng {} {} {} {}\n", phase, _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]);
	fmt::print(stderr, "OBJECT definition {} count {}\n", phase, ObjectSpec::Count());
	for (const auto &spec : ObjectSpec::Specs()) {
		uint32_t label = spec.class_index == INVALID_OBJECT_CLASS ? 0 : ObjectClass::Get(spec.class_index)->global_id;
		fmt::print(stderr, "OBJECT definition {} spec {} {:08X} {} {:08X} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}\n", phase, spec.Index(), spec.grf_prop.grfid, spec.grf_prop.local_id, label, spec.index, spec.name, spec.climate.base(), spec.size, spec.build_cost_multiplier, spec.clear_cost_multiplier, spec.introduction_date.base(), spec.end_of_life_date.base(), spec.flags.base(), spec.callback_mask.base(), spec.height, spec.views, spec.generate_amount, spec.animation.frames, to_underlying(spec.animation.status), spec.animation.speed, spec.animation.triggers.base(), spec.grf_prop.HasSpriteGroups());
	}
	for (const auto &cls : ObjectClass::Classes()) {
		fmt::print(stderr, "OBJECT definition {} class {:08X} {} {} {}\n", phase, cls.global_id, cls.Index(), cls.name, cls.GetSpecCount());
		for (const auto *spec : cls.Specs()) fmt::print(stderr, "OBJECT definition {} member {:08X} {} {} {}\n", phase, cls.global_id, spec->Index(), spec->index, ObjectSpec::Get(spec->Index()) == spec);
	}
	for (uint32_t id : {0x3436414FU, 0x3436424FU}) {
		const auto &file = *GetFileByGRFID(id);
#ifdef WITH_RUST
		size_t capacity = file.objectspec.capacity;
#else
		size_t capacity = file.objectspec.capacity();
#endif
		fmt::print(stderr, "OBJECT definition {} file {:08X} {} {}\n", phase, file.grfid, file.objectspec.size(), capacity);
	}
	fmt::print(stderr, "OBJECT definition {} fallback {} {} {}\n", phase, ObjectSpec::Get(63999)->views, ObjectSpec::Get(63999)->size, ObjectSpec::Get(63999)->animation.speed);
}
static bool ObjectDefinitionsScenario(std::span<std::string_view> args)
{
	if (args[1] == "definitions-inspect") { _interactive_random.SetSeed(0x63800001U * 0x1234567U); ObjectDefinitionState("inspect"); return true; }
	if (args.size() < 3) return false;
	SetRandomSeed(0x63800001); ObjectPartialDefinitions();
	_grfconfig.clear();
	for (const char *name : {"objects-a", "objects-b"}) {
		auto config = std::make_unique<GRFConfig>(std::string(args[2]) + "/" + name + ".grf");
		ObjectNeed(FillGRFDetails(*config, false, NO_DIRECTORY), __LINE__); config->flags.Reset(GRFConfigFlag::InitOnly); _grfconfig.push_back(std::move(config));
	}
	// The unchanged input reserves mapping IDs 5 through 9; these files allocate 10 through 12.
	ReloadNewGRFData(); ObjectNeed(ObjectSpec::Count() == 13, __LINE__); ObjectDefinitionState("first");
	std::array<ObjectType, 3> ids{_object_mngr.GetID(0, 0x3436414F), _object_mngr.GetID(1, 0x3436414F), _object_mngr.GetID(0, 0x3436424F)};
	ResetNewGRFData();
	fmt::print(stderr, "OBJECT definition reset mappings {} {} {} {}\n", ObjectSpec::Count(), _object_mngr.GetID(0, 0x3436414F) == ids[0], _object_mngr.GetID(1, 0x3436414F) == ids[1], _object_mngr.GetID(0, 0x3436424F) == ids[2]);
	std::reverse(_grfconfig.begin(), _grfconfig.end()); ReloadNewGRFData(); ObjectNeed(ObjectSpec::Count() == 13, __LINE__); ObjectDefinitionState("reordered");
	fmt::print(stderr, "OBJECT definition reordered mappings {} {} {}\n", _object_mngr.GetID(0, 0x3436414F) == ids[0], _object_mngr.GetID(1, 0x3436414F) == ids[1], _object_mngr.GetID(0, 0x3436424F) == ids[2]);
	if (args.size() > 3) ObjectNeed(SaveOrLoad(std::string(args[3]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) == SL_OK, __LINE__);
	fmt::print(stderr, "OBJECT definition complete true\n"); return true;
}
