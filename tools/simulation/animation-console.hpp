/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file animation-console.hpp Controlled resolver programs on real tiles; both games execute their own bodies. */
#include "animated_tile_map.h"
#include "animated_tile_func.h"
#include "industrytype.h"
#include "industry_map.h"
#include "newgrf_house.h"
#include "newgrf_industrytiles.h"
#include "newgrf_station.h"
#include "newgrf_airporttiles.h"
#include "newgrf_roadstop.h"
#include "timer/timer_game_tick.h"
#ifdef WITH_RUST
#include "rust/animation_ffi.h"
#include "rust/animation_save.hpp"
#else
extern std::vector<TileIndex> _animated_tiles;
#endif
static std::vector<TileIndex> AnimationQueue()
{
#ifdef WITH_RUST
	std::vector<TileIndex> tiles;
	for (size_t i = 0; i < openttd_rust_animation_count(); ++i) tiles.emplace_back(openttd_rust_animation_at(i));
	return tiles;
#else
	return _animated_tiles;
#endif
}
static void RestoreAnimationQueue(const std::vector<TileIndex> &tiles)
{
	InitializeAnimatedTiles();
#ifdef WITH_RUST
	for (auto t : tiles) openttd_rust_animation_append(t.base());
#else
	_animated_tiles = tiles;
#endif
}
/** Leaf observations are gated to keyed cases; every wrapper calls the actual operation. */
static std::string animation_case, animation_version;
static TileIndex animation_tile;
static bool animation_road = false;
static uint animation_sequence = 0;
static void AnimationTrace(std::string_view event)
{
	if (animation_case.empty()) return;
	auto *st = BaseStation::GetByTile(animation_tile);
	fmt::print(stderr, "OBJECT animation-trace {} {} {} {} {}", animation_case, animation_sequence++, event, animation_road ? st->GetRoadStopAnimationFrame(animation_tile) : GetAnimationFrame(animation_tile), to_underlying(GetAnimatedTileState(animation_tile)));
	for (auto t : AnimationQueue()) fmt::print(stderr, " q{}", t.base());
	for (const auto &r : st->custom_roadstop_tile_data) fmt::print(stderr, " r{}:{}:{}", r.tile.base(), r.random_bits, r.animation_frame);
	fmt::print(stderr, "\n");
}
static uint generic_sound_calls = 0;
static const GRFFile *generic_sound_file = nullptr;
static SoundID generic_sound_id = 0;
static TileIndex generic_sound_tile;
extern "C" void __real__Z13PlayTileSoundPK7GRFFiletN10StrongType7TypedefIj12TileIndexTagJNS2_7CompareENS2_7IntegerENS2_10CompatibleIiEENS7_IlEEEEE(const GRFFile *, SoundID, TileIndex);
extern "C" void __wrap__Z13PlayTileSoundPK7GRFFiletN10StrongType7TypedefIj12TileIndexTagJNS2_7CompareENS2_7IntegerENS2_10CompatibleIiEENS7_IlEEEEE(const GRFFile *f, SoundID s, TileIndex t)
{
	generic_sound_calls++; generic_sound_file = f; generic_sound_id = s; generic_sound_tile = t;
	AnimationTrace("sound-enter");
	__real__Z13PlayTileSoundPK7GRFFiletN10StrongType7TypedefIj12TileIndexTagJNS2_7CompareENS2_7IntegerENS2_10CompatibleIiEENS7_IlEEEEE(f, s, t);
	AnimationTrace("sound-exit");
}
extern "C" bool __real__ZN11BaseStation19SetRoadStopTileDataEN10StrongType7TypedefIj12TileIndexTagJNS0_7CompareENS0_7IntegerENS0_10CompatibleIiEENS5_IlEEEEEhb(BaseStation *, TileIndex, uint8_t, bool);
extern "C" bool __wrap__ZN11BaseStation19SetRoadStopTileDataEN10StrongType7TypedefIj12TileIndexTagJNS0_7CompareENS0_7IntegerENS0_10CompatibleIiEENS5_IlEEEEEhb(BaseStation *s, TileIndex t, uint8_t f, bool a)
{
	AnimationTrace(a ? "setter-enter" : "random-enter");
	bool changed = __real__ZN11BaseStation19SetRoadStopTileDataEN10StrongType7TypedefIj12TileIndexTagJNS0_7CompareENS0_7IntegerENS0_10CompatibleIiEENS5_IlEEEEEhb(s, t, f, a);
	AnimationTrace(changed ? "setter-changed" : "setter-unchanged");
	return changed;
}
extern "C" void __real__Z19MarkTileDirtyByTileN10StrongType7TypedefIj12TileIndexTagJNS_7CompareENS_7IntegerENS_10CompatibleIiEENS4_IlEEEEEii(TileIndex, int, int);
extern "C" void __wrap__Z19MarkTileDirtyByTileN10StrongType7TypedefIj12TileIndexTagJNS_7CompareENS_7IntegerENS_10CompatibleIiEENS4_IlEEEEEii(TileIndex t, int b, int h)
{
	AnimationTrace("dirty");
	__real__Z19MarkTileDirtyByTileN10StrongType7TypedefIj12TileIndexTagJNS_7CompareENS_7IntegerENS_10CompatibleIiEENS4_IlEEEEEii(t, b, h);
}
extern "C" void __real__Z26ErrorUnknownCallbackResultjtt(uint32_t, uint16_t, uint16_t);
extern "C" void __wrap__Z26ErrorUnknownCallbackResultjtt(uint32_t g, uint16_t c, uint16_t r)
{
	fmt::print(stderr, "OBJECT animation-diagnostic {} {} {} {}\n", animation_version, g, c, r);
	__real__Z26ErrorUnknownCallbackResultjtt(g, c, r);
}
/** ANIT-only input stream; malformed cases terminate before the game can resume. */
struct AnimationInput : LoadFilter {
	std::vector<uint8_t> bytes;
	size_t position = 0;
	explicit AnimationInput(std::vector<uint8_t> bytes) : LoadFilter(nullptr), bytes(std::move(bytes)) {}
	size_t Read(uint8_t *out, size_t length) override {
		length = std::min(length, this->bytes.size() - this->position);
		std::copy_n(this->bytes.data() + this->position, length, out);
		this->position += length;
		return length;
	}
};
static void AnimationLoadProbe(std::string_view kind)
{
	const auto &handler = *std::ranges::find_if(ChunkHandlers(), [](const ChunkHandler &ch) { return ch.id == 'ANIT'; });
	RestoreAnimationQueue({TileIndex{333}, TileIndex{444}, TileIndex{555}});
	_sl = {}; _sl.action = SLA_SAVE; _sl_version = SAVEGAME_VERSION;
	_sl.dumper = std::make_unique<MemoryDumper>();
	SlSaveChunk(handler);
	const auto size = _sl.dumper->GetSize();
	std::vector<uint8_t> bytes(_sl.dumper->blocks[0].get() + 4, _sl.dumper->blocks[0].get() + size);
	if (kind == "header") bytes = {CH_TABLE, 2, 0xFF}; // Invalid table field type, before vector decoding.
	if (kind == "partial") bytes.resize(bytes.size() - 5); // Two values decoded, third default-constructed.
	if (kind == "excess") { bytes.back() = 2; bytes.push_back(0); bytes.push_back(0); }
	if (kind == "fixed" || kind == "riff") {
		bytes = {CH_RIFF, 0, 0, 12, 0, 0, 1, 77, 0, 0, 1, 188};
		if (kind == "fixed") { bytes.resize(1028); bytes[2] = 4; bytes[3] = 0; }
	}
	RestoreAnimationQueue({TileIndex{11}, TileIndex{22}});
	if (AnimationQueue().size() != 2) std::abort();
	_sl = {}; _sl.action = SLA_LOAD; _next_offs = 0;
	_sl_version = kind == "fixed" ? static_cast<SaveLoadVersion>(79) : kind == "riff" ? SLV_80 : SAVEGAME_VERSION;
	_sl.reader = std::make_unique<ReadBuffer>(std::make_shared<AnimationInput>(std::move(bytes)));
#ifdef WITH_RUST
	RustAnimationSave outer(false); // Load's nested staging must restore this exact active pointer on unwind.
#endif
	bool caught = false;
	try { SlLoadChunk(handler); } catch (const std::exception &) { caught = true; }
	bool restored = true;
#ifdef WITH_RUST
	restored = RustAnimationSave::active == &outer.tiles;
#endif
	if (!restored || AnimationQueue().empty() || caught != (kind != "valid" && kind != "fixed") || (caught && _sl.action != SLA_NULL)) std::abort();
	fmt::print(stderr, "OBJECT animation-load {} {} {} {} {} {}", kind, caught, _sl.action == SLA_NULL, restored, _sl.error_str, _sl.extra_msg);
	for (TileIndex tile : AnimationQueue()) fmt::print(stderr, " {}", tile.base());
	fmt::print(stderr, "\nOBJECT animation-load-complete {}\n", kind);
	std::fflush(stderr);
	std::_Exit(0); // SlNullPointers invalidates other pools; never return to play or normal shutdown.
}
struct AnimationGroup : SpriteGroup {
	AnimationGroup() {} // Preserve the pool index assigned by operator new.
	uint16_t speed = 0, next = 0;
	std::function<void(ResolverObject &)> observation;
	ResolverResult Resolve(ResolverObject &ro) const override {
		if (this->observation) this->observation(ro);
		fmt::print(stderr, "OBJECT animation-callback {} {} {}\n", to_underlying(ro.callback), ro.callback_param1, ro.callback_param2);
		bool is_speed = ro.callback == CBID_HOUSE_ANIMATION_SPEED || ro.callback == CBID_INDTILE_ANIMATION_SPEED || ro.callback == CBID_STATION_ANIMATION_SPEED || ro.callback == CBID_AIRPTILE_ANIMATION_SPEED || ro.callback == CBID_OBJECT_ANIMATION_SPEED;
		return CallbackResult(is_speed ? speed : next);
	}
};
template <typename Spec, typename Mask, typename Animate, typename Frame, typename Random>
static void AnimationBindingProbe(const char *name, TileIndex tile, Spec *spec, Mask speed, Mask next, Animate animate, Frame frame, Random random)
{
	Backup<Spec> restore(*spec);
	const auto queue = AnimationQueue();
	const auto state = GetAnimatedTileState(tile);
	const auto old_frame = GetAnimationFrame(tile);
	Backup<uint64_t> ticks(TimerGameTick::counter, 0);
	GRFFile file; file.grf_version = 7; file.grfid = 0x385;
	SpriteGroup::CanAllocateItem(); auto group = std::make_unique<AnimationGroup>();
	spec->grf_prop.grffile = &file; spec->grf_prop.grfid = file.grfid;
	if constexpr (std::is_base_of_v<CargoGRFFileProps, std::remove_cvref_t<decltype(spec->grf_prop)>>) {
		spec->grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, group.get());
	} else {
		spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, group.get());
	}
	spec->animation.frames = 7; spec->animation.speed = 0;
	for (bool callbacks : {false, true}) for (bool looping : {false, true}) for (bool bits : {false, true}) {
		spec->callback_mask.Reset(); if (callbacks) spec->callback_mask = {speed, next};
		random(bits); spec->animation.status = looping ? AnimationStatus::Looping : AnimationStatus::NonLooping;
		for (auto result : {uint16_t(0xFFFF), uint16_t(0xFD), uint16_t(0xFE), uint16_t(0xFF), uint16_t(3)}) {
			group->speed = 0; group->next = result;
			InitializeAnimatedTiles(); SetAnimatedTileState(tile, AnimatedTileState::None); AddAnimatedTile(tile, false); frame(7);
			animate(tile);
			fmt::print(stderr, "OBJECT animation-step {} {} {} {} {} {} {} {} {}\n", name, callbacks, looping, bits, result, frame(-1), to_underlying(GetAnimatedTileState(tile)), _random.state[0], _random.state[1]);
		}
	}
	spec->callback_mask = {speed, next}; random(false);
	for (uint16_t value : {0U, 1U, 16U, 17U, 255U, 0x100U, 0x10FU, 0xFFFFU}) {
		group->speed = value; group->next = 4;
		for (uint64_t tick : {0ULL, 1ULL, 65536ULL}) {
			TimerGameTick::counter = tick; frame(0); animate(tile);
			fmt::print(stderr, "OBJECT animation-speed {} {} {} {}\n", name, value, tick, frame(-1));
		}
	}
	spec->callback_mask.Reset();
	for (uint8_t raw : {0, 16, 63, 64, 255}) for (uint64_t tick : {0ULL, 1ULL, 65536ULL}) {
		spec->animation.speed = raw; TimerGameTick::counter = tick; frame(0); animate(tile);
		fmt::print(stderr, "OBJECT animation-property-speed {} {} {} {}\n", name, raw, tick, frame(-1));
	}

	// A valid config is required by the native diagnostic's GRF lookup; restore the list afterwards.
	_grfconfig.push_back(std::make_unique<GRFConfig>("animation-probe")); _grfconfig.back()->ident.grfid = file.grfid;
	spec->callback_mask = {speed}; spec->animation.speed = 0; TimerGameTick::counter = 0;
	for (uint8_t version : {7, 8}) for (uint16_t result : {0xFF, 0x100, 0xFFFF}) {
		file.grf_version = version; animation_version = fmt::format("{}-{}-{}", name, version, result); group->speed = result; frame(7); animate(tile);
		fmt::print(stderr, "OBJECT animation-version {} {} {} {}\n", name, version, result, frame(-1));
	}
	_grfconfig.pop_back(); file.grf_version = 7;

	if constexpr (std::is_same_v<Spec, StationSpec> || std::is_same_v<Spec, RoadStopSpec>) {
		auto *st = BaseStation::GetByTile(tile);
		std::vector<TileIndex> members;
		if constexpr (std::is_same_v<Spec, StationSpec>) {
			for (TileIndex t : st->train_station) if (st->TileBelongsToRailStation(t)) members.push_back(t);
		} else {
			for (TileIndex t : Map::Iterate()) if (IsAnyRoadStopTile(t) && GetStationIndex(t) == st->index) members.push_back(t);
		}
		if (members.size() < 2) std::abort();
		std::vector<std::tuple<TileIndex, uint8_t, uint8_t, AnimatedTileState>> saved;
		for (TileIndex t : members) {
			uint8_t index;
			if constexpr (std::is_same_v<Spec, StationSpec>) { index = GetCustomStationSpecIndex(t); SetCustomStationSpecIndex(t, 1); }
			else { index = GetCustomRoadStopSpecIndex(t); SetCustomRoadStopSpecIndex(t, 1); }
			saved.emplace_back(t, index, GetAnimationFrame(t), GetAnimatedTileState(t));
		}
		auto &cached = [&]() -> StationAnimationTriggers & {
			if constexpr (std::is_same_v<Spec, StationSpec>) return st->cached_anim_triggers;
			else return st->cached_roadstop_anim_triggers;
		}();
		Backup<StationAnimationTriggers> old_cached(cached);
		Backup<std::vector<RoadStopTileData>> data(st->custom_roadstop_tile_data);
		if constexpr (std::is_same_v<Spec, RoadStopSpec>) {
			st->custom_roadstop_tile_data.clear();
			for (auto it = members.rbegin(); it != members.rend(); ++it) st->custom_roadstop_tile_data.push_back({*it, 19, 0});
		}
		for (bool trigger : {false, true}) for (bool ambient : {false, true}) for (uint16_t result : {0xFFFF, 0x1FD, 0x1FE, 0x1FF, 0x103, 0x107, 0x8003}) {
			AutoRestoreBackup<bool> sound(_settings_client.sound.ambient, ambient);
			spec->callback_mask = {next}; spec->animation.speed = 0; spec->animation.status = AnimationStatus::NonLooping;
			spec->animation.triggers = {StationAnimationTrigger::Built}; cached = spec->animation.triggers;
			group->next = result; random(false); frame(7);
			RestoreAnimationQueue({tile}); SetAnimatedTileState(tile, trigger && (result & 0xFF) != 0xFF ? AnimatedTileState::Deleted : AnimatedTileState::Animated);
			animation_case = fmt::format("{}-{}-{}-{}", name, trigger, ambient, result); animation_tile = tile;
			animation_road = std::is_same_v<Spec, RoadStopSpec>; animation_sequence = 0;
			group->observation = [](ResolverObject &) { AnimationTrace("callback"); };
			if (!trigger) animate(tile);
			else if constexpr (std::is_same_v<Spec, StationSpec>) TriggerStationAnimation(st, tile, StationAnimationTrigger::Built, INVALID_CARGO);
			else TriggerRoadStopAnimation(st, tile, StationAnimationTrigger::Built, INVALID_CARGO);
			AnimationTrace("end"); fmt::print(stderr, "OBJECT animation-matrix {}\n", animation_case); animation_case.clear();
		}
		if constexpr (std::is_same_v<Spec, StationSpec>) {
			for (auto rejection : {"cached", "nonmember", "nullspec", "disabled", "accepted"}) {
				cached = std::string_view(rejection) == "cached" ? StationAnimationTriggers{} : StationAnimationTriggers{StationAnimationTrigger::Built};
				spec->animation.triggers = std::string_view(rejection) == "disabled" ? StationAnimationTriggers{} : StationAnimationTriggers{StationAnimationTrigger::Built};
				SetCustomStationSpecIndex(tile, std::string_view(rejection) == "nullspec" ? 0 : 1);
				uint calls = 0; group->observation = [&](ResolverObject &) { ++calls; }; group->next = 0xFD;
				fmt::print(stderr, "OBJECT animation-rejection {} before {} {}\n", rejection, _random.state[0], _random.state[1]); animation_case = fmt::format("reject-{}", rejection); animation_sequence = 0; AnimationTrace("rejection-before");
				TriggerStationAnimation(st, std::string_view(rejection) == "nonmember" ? TileXY(0, 0) : tile, StationAnimationTrigger::Built, INVALID_CARGO);
				fmt::print(stderr, "OBJECT animation-rejection {} after {} {} {}\n", rejection, _random.state[0], _random.state[1], calls); AnimationTrace("rejection-after"); animation_case.clear();
			}
			SetCustomStationSpecIndex(tile, 1); group->observation = {};
		}

		group->next = 0xFD; file.cargo_map[0] = 37;
		for (uint8_t phase = 0; phase < 4; phase++) for (uint8_t value = 0; value < 9; value++) {
			auto event = static_cast<StationAnimationTrigger>(value);
			cached = phase == 0 ? StationAnimationTriggers{} : StationAnimationTriggers{event};
			spec->animation.triggers = phase == 1 ? StationAnimationTriggers{} : StationAnimationTriggers{event};
			group->observation = [&, phase](ResolverObject &ro) {
				TileIndex t;
				if constexpr (std::is_same_v<Spec, StationSpec>) {
#ifdef WITH_RUST
					t = TileIndex{static_cast<StationResolverObject &>(ro).station_scope.tile};
#else
					t = static_cast<StationResolverObject &>(ro).station_scope.tile;
#endif
				}
				else {
#ifdef WITH_RUST
					t = TileIndex{static_cast<RoadStopResolverObject &>(ro).roadstop_scope.state.tile};
#else
					t = static_cast<RoadStopResolverObject &>(ro).roadstop_scope.tile;
#endif
				}
				fmt::print(stderr, "OBJECT animation-visit {} {} {} {} {} {}\n", name, t.base(), ro.callback_param1, ro.callback_param2, _random.state[0], _random.state[1]);
				if (phase == 3) spec->animation.triggers.Reset();
			};
			if constexpr (std::is_same_v<Spec, StationSpec>) TriggerStationAnimation(st, tile, event, 0);
			else TriggerRoadStopAnimation(st, tile, event, 0);
			fmt::print(stderr, "OBJECT animation-traversal {} {} {} {} {}\n", name, phase, value, _random.state[0], _random.state[1]);
		}
		for (auto [t, index, f, flags] : saved) {
			if constexpr (std::is_same_v<Spec, StationSpec>) SetCustomStationSpecIndex(t, index);
			else SetCustomRoadStopSpecIndex(t, index);
			SetAnimationFrame(t, f); SetAnimatedTileState(t, flags);
		}
		group->observation = {};
	}
	if constexpr (std::is_same_v<Spec, AirportTileSpec>) {
		auto *st = Station::GetByTile(tile);
		const auto airport_tile = st->airport.tile; Backup<TileIndex> origin(st->airport.tile);
		Backup<uint16_t> width(st->airport.w, st->airport.w + 1); // Include nonmembers; the real traversal must filter them.
		std::vector<std::pair<AirportTileSpec *, AirportTileSpec>> saved;
		for (TileIndex t : st->airport) if (st->TileBelongsToAirport(t)) {
			auto *ats = const_cast<AirportTileSpec *>(AirportTileSpec::GetByTile(t));
			if (std::ranges::none_of(saved, [ats](const auto &entry) { return entry.first == ats; })) saved.emplace_back(ats, *ats);
		}
		if (saved.size() < 3) std::abort();
		group->next = 0xFD; file.cargo_map[0] = 37;
		for (uint8_t phase = 0; phase < 5; phase++) {
			st->airport.tile = phase == 0 ? INVALID_TILE : airport_tile;
			size_t index = 0;
			for (auto &[ats, old] : saved) {
				ats->grf_prop.grffile = phase == 1 || (phase == 3 && index % 3 == 0) ? nullptr : &file;
				ats->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, group.get());
				ats->animation.triggers = phase == 3 && index % 3 == 1 ? AirportAnimationTriggers{} : AirportAnimationTriggers{AirportAnimationTrigger::NewCargo};
				index++;
			}
			group->observation = [phase](ResolverObject &ro) {
				auto &scope = static_cast<AirportTileResolverObject &>(ro).tiles_scope;
#ifdef WITH_RUST
				TileIndex tile{scope.context.tile}; const auto *spec = scope.context.tile_spec;
#else
				TileIndex tile = scope.tile; const auto *spec = scope.ats;
#endif
				fmt::print(stderr, "OBJECT animation-visit airport {} {} {} {} {}\n", tile.base(), ro.callback_param1, ro.callback_param2, _random.state[0], _random.state[1]);
				if (phase == 4) const_cast<AirportTileSpec *>(spec)->animation.triggers.Reset();
			};
			bool accepted = TriggerAirportAnimation(st, AirportAnimationTrigger::NewCargo, 0);
			fmt::print(stderr, "OBJECT animation-airport {} {} {} {}\n", phase, accepted, _random.state[0], _random.state[1]);
		}
		for (auto &[ats, old] : saved) *ats = old;
		group->observation = {};
	}
	SetAnimationFrame(tile, old_frame); SetAnimatedTileState(tile, state); RestoreAnimationQueue(queue);
	fmt::print(stderr, "OBJECT animation-binding {} true\n", name);
}
static std::array<TileIndex, 6> AnimationFixtureTiles()
{
	std::array<TileIndex, 6> tiles; tiles.fill(INVALID_TILE);
	std::map<StationID, std::vector<TileIndex>> stops;
	for (TileIndex t : Map::Iterate()) {
		if (IsTileType(t, MP_HOUSE)) tiles[0] = t;
		if (IsTileType(t, MP_INDUSTRY)) tiles[1] = t;
		if (HasStationTileRail(t) && (tiles[2] == INVALID_TILE || BaseStation::GetByTile(t)->train_station.w * BaseStation::GetByTile(t)->train_station.h > BaseStation::GetByTile(tiles[2])->train_station.w * BaseStation::GetByTile(tiles[2])->train_station.h)) tiles[2] = t;
		if (IsTileType(t, MP_STATION) && IsAirport(t)) tiles[3] = t;
		if (IsAnyRoadStopTile(t)) stops[GetStationIndex(t)].push_back(t);
		if (IsTileType(t, MP_OBJECT)) tiles[5] = t;
	}
	size_t largest = 0; for (const auto &[id, members] : stops) if (members.size() > largest) { largest = members.size(); tiles[4] = members.front(); }
	if (std::ranges::find(tiles, INVALID_TILE) != tiles.end()) std::abort();
	return tiles;
}
static void AnimationReloadObservation()
{
	const auto queue = AnimationQueue(); if (queue.empty()) std::abort();
	fmt::print(stderr, "OBJECT animation-reload {} {} {}", queue.size(), _random.state[0], _random.state[1]);
	for (auto t : queue) fmt::print(stderr, " {}:{}:{}", t.base(), GetAnimationFrame(t), to_underlying(GetAnimatedTileState(t)));
	for (const auto *st : BaseStation::Iterate()) for (const auto &data : st->custom_roadstop_tile_data) fmt::print(stderr, " r{}:{}:{}:{}", st->index.base(), data.tile.base(), data.random_bits, data.animation_frame);
	fmt::print(stderr, "\n");
}
static void AnimationBindingsProbe()
{
	const auto tiles = AnimationFixtureTiles();
	auto frame = [](TileIndex t) { return [t](int value) { if (value >= 0) SetAnimationFrame(t, value); return GetAnimationFrame(t); }; };
	auto *h = HouseSpec::Get(GetHouseType(tiles[0]));
	AnimationBindingProbe("house", tiles[0], h, HouseCallbackMask::AnimationSpeed, HouseCallbackMask::AnimationNextFrame, AnimateNewHouseTile, frame(tiles[0]), [h](bool bits) { h->extra_flags.Set(HouseExtraFlag::Callback1ARandomBits, bits); });
	auto *i = const_cast<IndustryTileSpec *>(GetIndustryTileSpec(GetIndustryGfx(tiles[1])));
	AnimationBindingProbe("industry", tiles[1], i, IndustryTileCallbackMask::AnimationSpeed, IndustryTileCallbackMask::AnimationNextFrame, AnimateNewIndustryTile, frame(tiles[1]), [i](bool bits) { i->special_flags.Set(IndustryTileSpecialFlag::NextFrameRandomBits, bits); });
	auto *st = BaseStation::GetByTile(tiles[2]); StationSpec rail{};
	Backup<std::vector<SpecMapping<StationSpec>>> rail_specs(st->speclist);
	const auto rail_index = GetCustomStationSpecIndex(tiles[2]); st->speclist.resize(2); st->speclist[1].spec = &rail; SetCustomStationSpecIndex(tiles[2], 1);
	AnimationBindingProbe("station", tiles[2], &rail, StationCallbackMask::AnimationSpeed, StationCallbackMask::AnimationNextFrame, AnimateStationTile, frame(tiles[2]), [&rail](bool bits) { rail.flags.Set(StationSpecFlag::Cb141RandomBits, bits); });
	SetCustomStationSpecIndex(tiles[2], rail_index);
	auto *a = const_cast<AirportTileSpec *>(AirportTileSpec::GetByTile(tiles[3]));
	AnimationBindingProbe("airport", tiles[3], a, AirportTileCallbackMask::AnimationSpeed, AirportTileCallbackMask::AnimationNextFrame, AnimateAirportTile, frame(tiles[3]), [a](bool bits) { AssignBit(a->animation_special_flags, 0, bits); });
	auto *rs = BaseStation::GetByTile(tiles[4]); RoadStopSpec road{};
	Backup<std::vector<SpecMapping<RoadStopSpec>>> road_specs(rs->roadstop_speclist);
	Backup<std::vector<RoadStopTileData>> road_data(rs->custom_roadstop_tile_data);
	const auto road_index = GetCustomRoadStopSpecIndex(tiles[4]); rs->roadstop_speclist.resize(2); rs->roadstop_speclist[1].spec = &road; SetCustomRoadStopSpecIndex(tiles[4], 1);
	AnimationBindingProbe("roadstop", tiles[4], &road, RoadStopCallbackMask::AnimationSpeed, RoadStopCallbackMask::AnimationNextFrame, AnimateRoadStopTile, [rs, t = tiles[4]](int value) { if (value >= 0) rs->SetRoadStopAnimationFrame(t, value); return rs->GetRoadStopAnimationFrame(t); }, [&road](bool bits) { road.flags.Set(RoadStopSpecFlag::Cb141RandomBits, bits); });
	const auto zero_queue = AnimationQueue(); const auto zero_state = GetAnimatedTileState(tiles[4]);
	rs->custom_roadstop_tile_data.clear();
	fmt::print(stderr, "OBJECT animation-road-zero {}", rs->GetRoadStopAnimationFrame(tiles[4]));
	road.animation.frames = 0; road.animation.speed = 0; road.callback_mask.Reset();
	SetAnimatedTileState(tiles[4], AnimatedTileState::None); AddAnimatedTile(tiles[4], false);
	AnimateRoadStopTile(tiles[4]);
	fmt::print(stderr, " {} {} {}\n", rs->custom_roadstop_tile_data.size(), rs->GetRoadStopAnimationFrame(tiles[4]), GetAnimatedTileState(tiles[4]) == AnimatedTileState::Deleted);
	RestoreAnimationQueue(zero_queue); SetAnimatedTileState(tiles[4], zero_state);
	SetCustomRoadStopSpecIndex(tiles[4], road_index);
	auto *o = const_cast<ObjectSpec *>(ObjectSpec::GetByTile(tiles[5]));
	Backup<ObjectFlags> flags(o->flags); o->flags.Set(ObjectFlag::Animation);
	AnimationBindingProbe("object", tiles[5], o, ObjectCallbackMask::AnimationSpeed, ObjectCallbackMask::AnimationNextFrame, AnimateNewObjectTile, frame(tiles[5]), [o](bool bits) { o->flags.Set(ObjectFlag::AnimRandomBits, bits); });
}

#include "town_cmd.h"
#include "station_cmd.h"
#include "terraform_cmd.h"
void BuildOilRig(TileIndex tile);
static void AnimationLifecycleProbe(TileIndex scratch)
{
	const auto tiles = AnimationFixtureTiles();
	const auto queue = AnimationQueue();
	const std::array survivors{scratch + TileDiffXY(5, 9), scratch + TileDiffXY(6, 9)};
	for (auto t : survivors) { Object::CanAllocateItem(); BuildObject(OBJECT_OWNED_LAND, t, _current_company); }
	AutoRestoreBackup<GameMode> editor(_game_mode, GM_EDITOR);
	auto exercise = [&](const char *key, TileIndex tile, auto operation) {
		for (auto t : AnimationQueue()) SetAnimatedTileState(t, AnimatedTileState::None);
		RestoreAnimationQueue({tile, survivors[0], survivors[1]});
		for (auto t : AnimationQueue()) SetAnimatedTileState(t, AnimatedTileState::Animated);
		auto observe = [&](const char *phase) {
			fmt::print(stderr, "OBJECT animation-lifecycle {} {} {}:{}", key, phase, tile.base(), to_underlying(GetAnimatedTileState(tile)));
			for (auto t : AnimationQueue()) fmt::print(stderr, " {}:{}", t.base(), to_underlying(GetAnimatedTileState(t)));
			fmt::print(stderr, "\n");
		};
		observe("before"); if (!operation()) std::abort(); observe("after");
		for (auto t : survivors) if (!IsTileType(t, MP_OBJECT) || GetAnimatedTileState(t) != AnimatedTileState::Animated || std::ranges::count(AnimationQueue(), t) != 1) std::abort();
	};
	exercise("house-replace", tiles[0], [&] { return CmdPlaceHouse(DoCommandFlag::Execute, tiles[0], 0, false, true).Succeeded() && IsTileType(tiles[0], MP_HOUSE); });
	exercise("house-clear", tiles[0], [&] { return Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, tiles[0]).Succeeded() && IsTileType(tiles[0], MP_CLEAR); });
	exercise("oilrig-convert", tiles[1], [&] { BuildOilRig(tiles[1]); return IsOilRig(tiles[1]); });
	TileIndex industry = INVALID_TILE; for (auto t : Map::Iterate()) if (IsTileType(t, MP_INDUSTRY) && GetTileZ(t) > 0 && GetTileSlope(t) == SLOPE_FLAT) { industry = t; break; }
	exercise("industry-terraform", industry, [&] { return std::get<0>(Command<CMD_TERRAFORM_LAND>::Do(DoCommandFlag::Execute, industry, SLOPE_N, false)).Succeeded(); });
	industry = INVALID_TILE; for (auto t : Map::Iterate()) if (IsTileType(t, MP_INDUSTRY)) { industry = t; break; }
	exercise("industry-clear", industry, [&] { return Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, industry).Succeeded() && !IsTileType(industry, MP_INDUSTRY); });
	for (auto [key, tile] : {std::pair{"rail-remove", tiles[2]}, {"roadstop-remove", tiles[4]}, {"airport-remove", tiles[3]}}) {
		if (std::string_view(key) == "airport-remove") { tile = scratch + TileDiffXY(0, 3); auto built = Command<CMD_BUILD_AIRPORT>::Do(DoCommandFlag::Execute, tile, AT_HELIPORT, 0, NEW_STATION, false); Result("lifecycle-airport-build", built); if (built.Failed()) std::abort(); }
		AutoRestoreBackup<CompanyID> owner(_current_company, GetTileOwner(tile));
		exercise(key, tile, [&] { return Command<CMD_LANDSCAPE_CLEAR>::Do({DoCommandFlag::Execute, DoCommandFlag::Bankrupt}, tile).Succeeded() && !IsTileType(tile, MP_STATION); });
	}
	TileIndex rail = INVALID_TILE; for (auto t : Map::Iterate()) if (HasStationTileRail(t)) { rail = t; break; }
	const auto rail_owner = GetTileOwner(rail);
	exercise("station-preserve", rail, [&] { ChangeTileOwner(rail, rail_owner, rail_owner); return HasStationTileRail(rail) && GetTileOwner(rail) == rail_owner; });
	exercise("station-remove", rail, [&] { ChangeTileOwner(rail, rail_owner, INVALID_OWNER); return !IsTileType(rail, MP_STATION); });
	TileIndex object = scratch + TileDiffXY(4, 9); Object::CanAllocateItem(); BuildObject(OBJECT_OWNED_LAND, object, _current_company);
	exercise("object-preserve", object, [&] { ChangeTileOwner(object, _current_company, _current_company); return IsTileType(object, MP_OBJECT); });
	exercise("object-remove", object, [&] { ChangeTileOwner(object, _current_company, INVALID_OWNER); return !IsTileType(object, MP_OBJECT); });
	Object::CanAllocateItem(); BuildObject(OBJECT_OWNED_LAND, object, _current_company);
	exercise("object-clear", object, [&] { return Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, object).Succeeded() && !IsTileType(object, MP_OBJECT); });
	for (auto t : AnimationQueue()) SetAnimatedTileState(t, AnimatedTileState::None);
	InitializeAnimatedTiles(); for (auto t : queue) if (MayAnimateTile(t)) AddAnimatedTile(t, false);
}
