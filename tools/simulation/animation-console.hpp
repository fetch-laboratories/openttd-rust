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
struct AnimationGroup : SpriteGroup {
	AnimationGroup() {} // Preserve the pool index assigned by operator new.
	uint16_t speed = 0, next = 0;
	ResolverResult Resolve(ResolverObject &ro) const override {
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

	SetAnimationFrame(tile, old_frame); SetAnimatedTileState(tile, state); RestoreAnimationQueue(queue);
	fmt::print(stderr, "OBJECT animation-binding {} true\n", name);
}
static void AnimationBindingsProbe()
{
	std::array<TileIndex, 6> tiles; tiles.fill(INVALID_TILE);
	for (TileIndex t : Map::Iterate()) {
		if (IsTileType(t, MP_HOUSE)) tiles[0] = t;
		if (IsTileType(t, MP_INDUSTRY)) tiles[1] = t;
		if (HasStationTileRail(t)) tiles[2] = t;
		if (IsTileType(t, MP_STATION) && IsAirport(t)) tiles[3] = t;
		if (IsAnyRoadStopTile(t)) tiles[4] = t;
		if (IsTileType(t, MP_OBJECT)) tiles[5] = t;
	}
	if (std::ranges::find(tiles, INVALID_TILE) != tiles.end()) std::abort();
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
	SetCustomRoadStopSpecIndex(tiles[4], road_index);
	auto *o = const_cast<ObjectSpec *>(ObjectSpec::GetByTile(tiles[5]));
	Backup<ObjectFlags> flags(o->flags); o->flags.Set(ObjectFlag::Animation);
	AnimationBindingProbe("object", tiles[5], o, ObjectCallbackMask::AnimationSpeed, ObjectCallbackMask::AnimationNextFrame, AnimateNewObjectTile, frame(tiles[5]), [o](bool bits) { o->flags.Set(ObjectFlag::AnimRandomBits, bits); });
}
