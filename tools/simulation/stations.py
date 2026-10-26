"""Station service evidence from the existing road/CAPY reference preparation."""

import hashlib
import struct
from pathlib import Path

from . import core, economy
from .console_adapter import prepared_console
from .core import ROOT, SNAPSHOT_TICKS, decode_element, field_spans, read_save
from .play_saves import DISTRIBUTIONS

PREPARE_ORDER = 5
PREFIX = "normal[0]/"
VEHICLE = "roadveh[0]/common[0]/"
CARGODIST = ("cargodist", "stale-refresh", "stale-restrict", "stale-release")
# LinkRefresher/IncreaseStats branches (vehicle/visits.rs) reached by real arrivals,
# departures, full-load waits and stale-link refreshes in these inputs.
VISIT_ROUTES = ["arrival_stats", "full_load_random", "full_load_refresh"]
VISIT_WITNESSES = {
    "manual": VISIT_ROUTES + ["refresh", "seen_hop", "stats"],
    "cargodist": VISIT_ROUTES + ["refresh", "seen_hop", "stats"],
    "full-gradual": ["full_load_random", "full_load_refresh", "refresh", "stats"],
    "stale-refresh": ["refresh", "seen_hop", "stats"],
}


def game_environment(scenario):
    witnessed = (
        scenario.get("station_service") or scenario.get("station_commands")
    ) and scenario.get("witnesses")
    return core.WITNESS_ENVIRONMENT if witnessed else {}


def scenarios(soak):
    return (
        [
            dict(
                name="stations-cargo-rating",
                kind="save",
                save=str(ROOT / "migration/saves/padhattan-ridge-2000.sav"),
                engines="cargo-rating-reload",
                short_checkpoint=True,
                ticks=1,
            )
        ]
        + [
            {
                "name": f"stations-{mode}",
                "kind": "save",
                "save": str(ROOT / "migration/saves/grok-159-001.sav"),
                "console": DISTRIBUTIONS[
                    "cargodist" if mode in CARGODIST else "manual"
                ],
                "economy": "cargodist"
                if mode in CARGODIST
                else "stockpile"
                if mode in ("acceptance-loss", "full-gradual", "loading-context")
                else "manual",
                "station_service": mode,
                "ticks": 1
                if mode == "loading-context"
                else 32
                if mode == "acceptance-loss"
                else (8 if soak else 4) * SNAPSHOT_TICKS,
                "snapshot_minimum": 0
                if mode in ("acceptance-loss", "loading-context")
                else 2,
            }
            for mode in (
                "manual",
                "cargodist",
                "acceptance-loss",
                "full-gradual",
                "loading-context",
                "rating-expire",
                "rating-cap",
                "stale-refresh",
                "stale-remove",
                "stale-restrict",
                "stale-release",
            )
        ]
        + [
            {
                "name": "stations-commands-" + mode,
                "kind": "save",
                "save": str(ROOT / "migration/saves/road-crossing.sav"),
                "station_commands": mode,
                "ticks": 1,
                "short_checkpoint": True,
            }
            for mode in (
                "commands",
                "linkgraph",
                "reload",
                "removed-destination",
                "map-edges",
                "railtype-63",
                "road-stops",
                "road-stops-reload",
                "waypoints",
                "waypoints-reload",
                "resolver",
                "resolver-reload",
                "airport",
                "airport-reload",
            )
        ]
    )


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "station_commands" in scenario:
        return prepare_commands(scenario, binaries, builds, out, timeout, env, result)
    if "station_service" not in scenario:
        return scenario
    original = read_save(Path(scenario["save"]))
    source = Path(scenario["save"]).read_bytes()
    data = bytearray(source)
    patches = []

    def patch(cid, index, field, value):
        chunk = original[cid]
        body = dict(chunk["elements"])[index]
        begin, end, kind = field_spans(body, chunk["header"])[field]
        if source.count(body, *chunk["span"]) != 1:
            raise RuntimeError("ambiguous station input row")
        encoded = struct.pack(core.FILE_TYPES[kind & 15][0], value)
        if len(encoded) != end - begin:
            raise RuntimeError("station patch width mismatch")
        start = source.index(body, *chunk["span"]) + begin
        data[start : start + len(encoded)] = encoded
        patches.append((cid, index, field, value))

    mode = scenario["station_service"]
    stations = core.rows(original, "STNN")
    if not any(row.get(PREFIX + "loading_vehicles") for row in stations.values()):
        raise RuntimeError("station input lacks a real active loading queue")
    if mode in ("acceptance-loss", "full-gradual", "loading-context"):
        front = economy.pending(original)
        if front is None:
            raise RuntimeError("station input lacks staged coal unloading")
        vehicle = core.rows(original, "VEHS")[front]
        station = vehicle[VEHICLE + "last_station_visited"]
        cargo = vehicle[VEHICLE + "cargo_type"]
        if mode == "acceptance-loss":
            field = PREFIX + f"goods[{cargo}]/status"
            patch("STNN", station, field, stations[station][field] & ~1)
            # This is a short first-unload checkpoint, before natural acceptance
            # polling can restore acceptance. Preserve the already staged packet.
            tick = decode_element(original["DATE"], original["DATE"]["elements"][0][1])[
                "tick_counter"
            ]
            if (tick + station) % 250 > 217:
                raise RuntimeError("acceptance-loss input is too near a big tick")
        else:
            flags = vehicle[VEHICLE + "current_order.flags"]
            patch(
                "VEHS", front, VEHICLE + "current_order.flags", (flags & ~0x70) | 0x20
            )
            if mode == "loading-context":
                # Modern save fixup accepts this independent last-visited field;
                # the queue stays at the real unloading station. The original
                # resolves the vehicle's station before reservation/unloading.
                alternate = 3
                row = stations[alternate]
                if (
                    alternate == station
                    or row[PREFIX + "base[0]/owner"] != vehicle[VEHICLE + "owner"]
                    or row[PREFIX + f"goods[{cargo}]/status"] & 1
                    or row[PREFIX + "loading_vehicles"]
                    or stations[station][PREFIX + "loading_vehicles"] != (front + 1,)
                    or vehicle[VEHICLE + "load_unload_ticks"] != 1
                ):
                    raise RuntimeError(
                        "loading-context input lacks distinct station witness"
                    )
                patch("VEHS", front, VEHICLE + "last_station_visited", alternate)
        result["station_front"] = front
        result["station_target"] = (station, cargo)
    if mode.startswith(("rating-", "stale-")):
        # Stop transport, retaining the real waiting packet at the first rated
        # passenger station. No expected result is changed with this input.
        for index, vehicle in core.rows(original, "VEHS").items():
            if vehicle["type"] == 1:
                patch(
                    "VEHS",
                    index,
                    VEHICLE + "vehstatus",
                    vehicle[VEHICLE + "vehstatus"] | 2,
                )
    if mode.startswith("rating-") or mode == "stale-remove":
        # These manual-distribution witnesses inspect stationary cargo and edge
        # expiry, not completed link jobs. Keep the saved jobs' relative order
        # but defer their joins beyond the whole run so asynchronous reload
        # pauses cannot consume its null-driver iteration budget. Cargodist and
        # stale-refresh retain their live-flow preparation and job coverage.
        delay = (scenario["ticks"] + core.TICKS_PER_DAY - 1) // core.TICKS_PER_DAY + 32
        for index, job in core.rows(original, "LGRJ").items():
            patch("LGRJ", index, "join_date", job["join_date"] + delay)
    if mode.startswith("stale-"):
        date = core.rows(original, "DATE")[0]["economy_date"]
        old = date - 2000
        # Remove/refresh age the whole edge. Restrict ages only the
        # unrestricted update under a fresh restricted one; release the reverse.
        stamps = {"stale-restrict": (old, date), "stale-release": (date, old)}
        unrestricted, restricted = stamps.get(mode, (old, -1))
        edges = 0
        for index, row in core.rows(original, "LGRP").items():
            for field in row:
                if field.endswith("/last_unrestricted_update"):
                    patch("LGRP", index, field, unrestricted)
                    other = field.replace(
                        "last_unrestricted_update", "last_restricted_update"
                    )
                    patch("LGRP", index, other, restricted)
                    edges += 1
        if not edges:
            raise RuntimeError("station input lacks real stale-link edges")
        result["station_edges"] = (edges, old)
    if mode.startswith("rating-"):
        station = next(
            index
            for index, row in stations.items()
            if row.get(PREFIX + "goods[0]/cargo[0]/second")
            and row[PREFIX + "goods[0]/status"] & 2
        )
        goods = PREFIX + "goods[0]/"
        patch("STNN", station, PREFIX + "base[0]/delete_ctr", 184)
        patch("STNN", station, goods + "time_since_pickup", 254)
        patch("STNN", station, goods + "rating", 1)
        patch("STNN", station, goods + "last_age", 255)
        patch("STNN", station, goods + "last_speed", 1)
        if mode == "rating-cap":
            packet = stations[station][goods + "cargo[0]/second"][0] - 1
            patch("CAPA", packet, "count", 32768)
            patch("STNN", station, goods + "max_waiting_cargo", 32768)
        result["station_target"] = (station, 0)
    prepared = Path(scenario["save"]).with_name("station-input.sav")
    prepared.write_bytes(data)
    read_save(prepared)
    result["station_input"] = {
        "reference_sha256": hashlib.sha256(source).hexdigest(),
        "input_sha256": hashlib.sha256(data).hexdigest(),
        "patches": patches,
        "loading_queues": {
            index: row.get(PREFIX + "loading_vehicles", ())
            for index, row in stations.items()
        },
    }
    scenario = dict(scenario, save=str(prepared))
    if mode not in ("manual", "cargodist"):
        del scenario["economy"]
    commands = [c for c in scenario["console"] if "order.gradual_loading" not in c]
    commands.insert(0, "setting order.gradual_loading 1")
    if mode.startswith("rating-"):
        commands.insert(0, f"setting order.selectgoods {int(mode == 'rating-expire')}")
    return dict(scenario, console=commands)


def check(scenario, run, mode, role, result):
    if "station_commands" in scenario:
        return check_commands(scenario, run, mode, role, result)
    if "station_service" not in scenario:
        return
    kind = scenario["station_service"]
    result[f"{mode}_{role}_movement_profile"] = core.branch_witnesses(
        scenario,
        run,
        role,
        "movement",
        ["pixel_z", "road_stop_admit"] if kind == "manual" else [],
    )
    required = {
        "acceptance-loss": ["rejected_delivery"],
        "full-gradual": ["full_load_reserve", "gradual_unload"],
        "manual": ["unload_cargo"],
        "cargodist": ["unload_cargo"],
        "stale-remove": ["stale_remove"],
        "stale-refresh": ["stale_refresh"],
        "stale-restrict": ["stale_restrict"],
        "stale-release": ["stale_release"],
    }.get(kind, [])
    if counts := core.branch_witnesses(scenario, run, role, "station", required):
        result[f"{mode}_{role}_station_branches"] = counts
    graph_required = {"stale-restrict": ["restrict"], "stale-release": ["release"]}.get(
        kind, []
    )
    core.branch_witnesses(scenario, run, role, "linkgraph", graph_required)
    # Route prediction and link statistics on real station visits.
    visits = VISIT_WITNESSES.get(kind, [])
    if counts := core.branch_witnesses(scenario, run, role, "vehicle_visits", visits):
        result[f"{mode}_{role}_visit_branches"] = counts
    if not run["snapshots"]:
        return
    saved = [read_save(path) for path in run["snapshots"]]
    before = read_save(Path(scenario["save"]))
    kind = scenario["station_service"]
    if kind.startswith("stale-"):
        count, old = result["station_edges"]
        updates = [
            value
            for row in core.rows(saved[-1], "LGRP").values()
            for field, value in row.items()
            if field.endswith("/last_unrestricted_update")
        ]
        if kind == "stale-remove" and len(updates) >= count:
            result["problems"].append(f"{mode}/{role}: no stale edge was removed")
        if kind == "stale-refresh" and not any(value > old for value in updates):
            result["problems"].append(f"{mode}/{role}: no stale edge was refreshed")
        result[f"{mode}_{role}_station_edges"] = (count, len(updates))
        return
    if kind in ("manual", "cargodist"):
        queues = [
            row.get(PREFIX + "loading_vehicles", ())
            for chunks in saved
            for row in core.rows(chunks, "STNN").values()
        ]
        fractions = [
            value
            for chunks in saved
            for row in core.rows(chunks, "STNN").values()
            for field, value in row.items()
            if field.endswith("/amount_fract") and value
        ]
        if mode == "snapshots" and (not any(queues) or not fractions):
            result["problems"].append(
                f"{mode}/{role}: no active queue/fraction witness"
            )
        result[f"{mode}_{role}_station_queues"] = sum(bool(q) for q in queues)
        return
    station, cargo = result["station_target"]
    goods = PREFIX + f"goods[{cargo}]/"
    rows = [core.rows(chunks, "STNN")[station] for chunks in saved]
    if kind == "rating-expire":
        if rows[-1][goods + "status"] & 2 or rows[-1][goods + "last_speed"]:
            result["problems"].append(f"{mode}/{role}: rating did not expire")
    elif kind == "rating-cap":
        counts = [
            sum(packet["count"] for packet in core.rows(chunks, "CAPA").values())
            for chunks in (before, saved[-1])
        ]
        if counts[1] >= counts[0]:
            result["problems"].append(f"{mode}/{role}: waiting cargo did not truncate")
        result[f"{mode}_{role}_packet_counts"] = counts
    else:
        front = result["station_front"]
        vehicle = core.rows(saved[-1], "VEHS")[front]
        if kind == "full-gradual":
            if vehicle[VEHICLE + "vehicle_flags"] & 1:
                result["problems"].append(
                    f"{mode}/{role}: full load finished while cargo unavailable"
                )
        elif kind == "loading-context":
            initial = core.rows(before, "VEHS")[front]
            actions = vehicle[VEHICLE + "cargo.action_counts"]
            queued = rows[-1][PREFIX + "loading_vehicles"]
            if (
                actions != (0, 0, sum(initial[VEHICLE + "cargo.action_counts"]), 0)
                or vehicle[VEHICLE + "cargo.packets"]
                != initial[VEHICLE + "cargo.packets"]
                or vehicle[VEHICLE + "vehicle_flags"] & 1
                or queued != (front + 1,)
            ):
                result["problems"].append(
                    f"{mode}/{role}: vehicle station did not retain staged cargo/queue"
                )
            result[f"{mode}_{role}_loading_context"] = {
                "last_station": vehicle[VEHICLE + "last_station_visited"],
                "queue_station": station,
                "action_counts": actions,
                "loading_queue": queued,
            }
        elif not vehicle[VEHICLE + "cargo.action_counts"][2]:
            result["problems"].append(
                f"{mode}/{role}: acceptance loss did not retain staged delivery"
            )


# Station construction (station_cmd.rs): inputs for the unchanged reference
# bodies and Rust command owner. The private tile callbacks need a fixture
# translation unit; the pinned source is included verbatim and its ordinary
# object is replaced only in this executable.
COMMAND_SOURCE = r"""
#include "station_cmd.cpp"
#include "console_internal.h"
#include "clear_map.h"
#include "order_cmd.h"
#include "waypoint_cmd.h"
#include "road_cmd.h"
#include "vehicle_cmd.h"
#include "engine_func.h"
#include "newgrf_cargo.h"
#include "saveload/saveload.h"
static void StationNeed(bool value, int line) { if (!value) { fmt::print(stderr, "STCMD setup-failure {}\n", line); std::abort(); } }
static void StationCost(const char *label, const CommandCost &cost)
{
	fmt::print(stderr, "STCMD cost {} {} {} {} rng {} {}\n", label, cost.Succeeded(), cost.GetCost(), cost.GetErrorMessage(), _random.state[0], _random.state[1]);
}
static size_t StationPathSize(const RoadVehicle *v)
{
#ifdef WITH_RUST
	return v->PathSize();
#else
	return v->path.size();
#endif
}
static void StationPathPush(RoadVehicle *v, Trackdir trackdir, TileIndex tile)
{
#ifdef WITH_RUST
	v->PathPush(trackdir, tile);
#else
	v->path.emplace_back(trackdir, tile);
#endif
}
/* #354: road stop status and drive-through entries, read through the native API. */
static void StationRoadStops(const char *label)
{
	for (const RoadStop *rs : RoadStop::Iterate()) {
		fmt::print(stderr, "STCMD road-stop {} {} {} {}", label, rs->index.base(), rs->xy.base(), rs->status.base());
		if (IsDriveThroughStopTile(rs->xy)) {
			const auto &east = rs->GetEntry(DIAGDIR_NE), &west = rs->GetEntry(DIAGDIR_NW);
			fmt::print(stderr, " east {} {} west {} {}", east.GetLength(), east.GetOccupied(), west.GetLength(), west.GetOccupied());
		}
		fmt::print(stderr, "\n");
	}
}
/* Place a stopped fixture vehicle at the tile centre and enter the tile natively. */
static void StationRoadEnter(RoadVehicle *v, TileIndex t, Direction dir, Trackdir td)
{
	v->tile = t; v->x_pos = TileX(t) * TILE_SIZE + 8; v->y_pos = TileY(t) * TILE_SIZE + 8; v->direction = dir;
	v->cur_speed = 0; v->vehstatus.Set(VehState::Stopped);
#ifdef WITH_RUST
	v->SetState(td); v->SetFrame(0);
	auto state = [v] { return v->GetState(); };
#else
	v->state = td; v->frame = 0;
	auto state = [v] { return v->state; };
#endif
	v->UpdatePosition();
	auto entered = VehicleEnterTile(v, t, v->x_pos, v->y_pos);
	fmt::print(stderr, "STCMD road-enter {} {} {} {}\n", v->index.base(), t.base(), entered.base(), state());
}
static TileIndex StationPatch()
{
	for (uint y = 8; y + 32 < Map::MaxY(); y += 8) for (uint x = 8; x + 32 < Map::MaxX(); x += 8) {
		TileArea area(TileXY(x, y), 32, 32);
		bool usable = true;
		for (TileIndex t : area) if ((GetTileType(t) != MP_CLEAR && GetTileType(t) != MP_TREES && GetTileType(t) != MP_WATER) || IsBridgeAbove(t) || VehiclesOnTile(t).begin() != VehiclesOnTile(t).end()) usable = false;
		auto height = [x, y](uint xx, uint yy) {
			int d = std::max({int(x) - int(xx), int(xx) - int(x + 32), int(y) - int(yy), int(yy) - int(y + 32), 0});
			return static_cast<uint>(std::clamp<int>(TileHeight(TileXY(xx, yy)), std::max(0, 1 - d), 1 + d));
		};
		/* Preserve live vehicle corner heights so their saved z remains valid. */
		for (const Vehicle *v : Vehicle::Iterate()) {
			if (!v->IsGroundVehicle()) continue;
			uint xx = TileX(v->tile), yy = TileY(v->tile);
			for (uint dy = 0; dy < 2; dy++) for (uint dx = 0; dx < 2; dx++) if (height(xx + dx, yy + dy) != TileHeight(TileXY(xx + dx, yy + dy))) usable = false;
		}
		if (!usable) continue;
		/* Clamp a one-level plateau with a valid one-level transition for reload. */
		for (uint yy = 0; yy < Map::SizeY(); yy++) for (uint xx = 0; xx < Map::SizeX(); xx++) SetTileHeight(TileXY(xx, yy), height(xx, yy));
		for (TileIndex t : area) MakeClear(t, CLEAR_GRASS, 3);
		return area.tile;
	}
	StationNeed(false, __LINE__); return INVALID_TILE;
}

/* #410: actual Action2 and association mutations in the default command fixtures.
 * Synthetic GRF groups are process-local; reload first checks missing specs,
 * then explicitly reattaches them. This does not test external GRF loading. */
struct RoadResolverFixture {
	GRFFile file;
	std::array<RoadStopSpec, 64> specs;
	std::array<uint16_t, 64> indices;
	RoadStopClassID cls;
	CallbackResultSpriteGroup *one, *empty, *cargo0, *cargo1;
	DeterministicSpriteGroup *purchase;
	RoadResolverFixture(bool waypoint)
	{
		file.grfid = waypoint ? 0x41000002 : 0x41000001; file.grf_version = 8;
		for (CargoType cargo = 0; cargo < NUM_CARGO; cargo++) { file.cargo_map[cargo] = cargo; file.cargo_list.push_back(CargoSpec::Get(cargo)->label); }
		cls = waypoint ? ROADSTOP_CLASS_WAYP : ROADSTOP_CLASS_DFLT;
		SpriteGroup::CanAllocateItem(5);
		one = new CallbackResultSpriteGroup(1); empty = new CallbackResultSpriteGroup(13); cargo0 = new CallbackResultSpriteGroup(17); cargo1 = new CallbackResultSpriteGroup(23);
		purchase = new DeterministicSpriteGroup(); purchase->size = DSG_SIZE_DWORD; purchase->var_scope = VSG_SCOPE_SELF; purchase->default_result.group = one;
		for (uint8_t var : {0x40, 0x41, 0x43, 0x44, 0x50}) {
			purchase->adjusts.push_back({.operation = DSGA_OP_RST, .type = DSGA_TYPE_NONE, .variable = var, .and_mask = UINT32_MAX});
			purchase->adjusts.push_back({.operation = DSGA_OP_STO, .type = DSGA_TYPE_NONE, .variable = 0x1A, .and_mask = uint32_t(0x100 + purchase->adjusts.size() / 2)});
		}
		for (uint i = 0; i < specs.size(); i++) {
			auto &spec = specs[i]; spec.grf_prop.grffile = &file; spec.grf_prop.grfid = file.grfid; spec.grf_prop.local_id = i + 256;
			spec.callback_mask.Set(RoadStopCallbackMask::Avail); spec.cargo_triggers = 3; spec.animation.triggers.Set(StationAnimationTrigger::Built);
			spec.grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_PURCHASE, purchase); spec.grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT, one);
			spec.grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT_NA, empty); spec.grf_prop.SetSpriteGroup(0, cargo0); spec.grf_prop.SetSpriteGroup(1, cargo1);
			indices[i] = RoadStopClass::Get(cls)->GetSpecCount(); RoadStopClass::Get(cls)->Insert(&spec);
		}
	}
};
static RoadResolverFixture &RoadResolverSpecs(bool waypoint)
{
	static RoadResolverFixture stops(false), waypoints(true);
	return waypoint ? waypoints : stops;
}
static void RoadResolverState(const char *label, BaseStation *st)
{
	fmt::print(stderr, "STCMD resolver-state {} {} {} {} {} {} rng {} {}\n", label, st->index.base(), st->roadstop_speclist.size(), st->cached_roadstop_anim_triggers.base(), st->cached_roadstop_cargo_triggers, st->waiting_random_triggers.base(), _random.state[0], _random.state[1]);
	for (size_t i = 0; i < st->roadstop_speclist.size(); i++) { const auto &slot = st->roadstop_speclist[i]; fmt::print(stderr, "STCMD resolver-slot {} {} {} {} {}\n", label, i, slot.spec != nullptr, slot.grfid, slot.localidx); }
	for (const auto &data : st->custom_roadstop_tile_data) fmt::print(stderr, "STCMD resolver-tile {} {} {} {} {}\n", label, data.tile.base(), GetCustomRoadStopSpecIndex(data.tile), st->GetRoadStopRandomBits(data.tile), st->GetRoadStopAnimationFrame(data.tile));
}
static void RoadResolverQueries(BaseStation *st, TileIndex tile, bool waypoint)
{
	auto &f = RoadResolverSpecs(waypoint); auto *spec = &f.specs[0];
	for (BaseStation *owner : {st, static_cast<BaseStation *>(nullptr)}) {
		RoadStopResolverObject ro(spec, owner, tile, ROADTYPE_ROAD, GetStationType(tile), GetStationGfx(tile));
		for (bool incoming : {false, true}) for (uint8_t var : {0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x49, 0x50, 0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x6B, 0x7A, 0xF0, 0xFA, 0xFF}) for (uint32_t parameter : {0U, 1U, 0x101U, 0xFFU}) {
			if (owner == nullptr && (var == 0x49 || var == 0x66 || var == 0x68 || var == 0x6B)) continue;
			bool available = incoming; auto value = ro.roadstop_scope.GetVariable(var, parameter, available);
			fmt::print(stderr, "STCMD resolver-variable {} {} {} {} {} {} {}\n", waypoint, owner != nullptr, var, parameter, incoming, available, value);
		}
		StationNeed(ro.GetTown() == ro.GetTown(), __LINE__);
		StationNeed(ro.GetScope(VSG_SCOPE_SELF) == &ro.roadstop_scope && ro.GetScope(VSG_SCOPE_PARENT) == ro.GetTown(), __LINE__);
	}
	std::array<int32_t, 5> regs{};
	StationNeed(GetRoadStopCallback(CBID_STATION_AVAILABILITY, 0, 0, spec, nullptr, INVALID_TILE, ROADTYPE_ROAD, GetStationType(tile), 3, regs) == 1, __LINE__);
	StationNeed(regs[0] == 3 && regs[4] == 16, __LINE__);
	fmt::print(stderr, "STCMD resolver-registers {} {} {} {} {} {}\n", waypoint, regs[0], regs[1], regs[2], regs[3], regs[4]);
	auto root = [&] { return GetRoadStopCallback(CBID_STATION_AVAILABILITY, 0, 0, spec, st, tile, INVALID_ROADTYPE, GetStationType(tile), GetStationGfx(tile)); };
	StationNeed(root() == 13, __LINE__);
	if (!waypoint) {
		Station *station = Station::From(st);
		for (CargoType c : {1, 0}) { StationNeed(CargoPacket::CanAllocateItem(), __LINE__); station->goods[c].GetOrCreateData().cargo.Append(new CargoPacket(st->index, 7, Source{st->town->index, SourceType::Town}), StationID::Invalid()); }
		StationNeed(root() == 17, __LINE__);
		station->goods[0].GetData().cargo.Truncate(); StationNeed(root() == 23, __LINE__);
		station->goods[1].GetData().cargo.Truncate(); StationNeed(root() == 13, __LINE__);
	}
	spec->grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT_NA, nullptr); StationNeed(root() == 1, __LINE__);
	spec->grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT_NA, f.empty);
	fmt::print(stderr, "STCMD resolver-roots {} ordered purchase default\n", waypoint);
}
static void RoadResolverRandom(BaseStation *st, TileIndex tile, bool waypoint)
{
	auto &f = RoadResolverSpecs(waypoint); auto *spec = &f.specs[0];
	SpriteGroup::CanAllocateItem(3); auto *lower = new RandomizedSpriteGroup(), *upper = new RandomizedSpriteGroup(); auto *publication = new DeterministicSpriteGroup();
	publication->var_scope = VSG_SCOPE_SELF; publication->size = DSG_SIZE_DWORD;
	for (auto [var, reg] : {std::pair<uint8_t, uint32_t>{0x5F, 0x105}, {0x0C, 0x106}}) {
		publication->adjusts.push_back({.operation = DSGA_OP_RST, .type = DSGA_TYPE_NONE, .variable = var, .and_mask = 0xFF});
		publication->adjusts.push_back({.operation = DSGA_OP_STO, .type = DSGA_TYPE_NONE, .variable = 0x1A, .and_mask = reg});
	}
	lower->var_scope = upper->var_scope = VSG_SCOPE_SELF; lower->lowest_randbit = 3; upper->lowest_randbit = 18;
	lower->groups = {f.one, f.one}; upper->groups = {lower, lower};
	for (uint8_t mask : {1, 2, 3}) for (StationRandomTrigger trigger : {StationRandomTrigger::NewCargo, StationRandomTrigger::CargoTaken, StationRandomTrigger::VehicleArrives, StationRandomTrigger::VehicleDeparts, StationRandomTrigger::VehicleLoads}) {
		if (waypoint && (trigger == StationRandomTrigger::NewCargo || trigger == StationRandomTrigger::CargoTaken)) continue;
		lower->triggers = mask & 1 ? 1 << to_underlying(trigger) : 0; upper->triggers = mask & 2 ? 1 << to_underlying(trigger) : 0;
		publication->default_result.group = mask == 1 ? lower : upper;
		spec->grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT_NA, publication);
		st->waiting_random_triggers = {}; st->random_bits = 0xA55A; st->SetRoadStopRandomBits(tile, 0x69);
		RoadStopUpdateCachedTriggers(st);
		RoadStopResolverObject object(spec, st, tile, INVALID_ROADTYPE, GetStationType(tile), GetStationGfx(tile), CBID_RANDOM_TRIGGER); object.SetWaitingRandomTriggers(StationRandomTriggers(1 << to_underlying(trigger))); object.ResolveRerandomisation();
		uint32_t expected_mask = (mask & 1 ? 8 : 0) | (mask & 2 ? 1 << 18 : 0);
		StationNeed(object.GetReseedSum() == expected_mask && object.GetUsedRandomTriggers().Test(trigger), __LINE__);
		Randomizer interactive = _interactive_random; Randomizer unarmed = _random; TriggerRoadStopRandomisation(st, tile, trigger, INVALID_CARGO);
		StationNeed(std::ranges::equal(_random.state, unarmed.state) && std::ranges::equal(_interactive_random.state, interactive.state) && st->waiting_random_triggers.Test(trigger) && st->random_bits == 0xA55A && st->GetRoadStopRandomBits(tile) == 0x69, __LINE__);
		ResolverObject registers(&f.file); StationNeed(registers.GetRegister(0x105) == (1 << to_underlying(trigger)) && registers.GetRegister(0x106) == CBID_NO_CALLBACK, __LINE__);
		fmt::print(stderr, "STCMD resolver-unarmed {} {} {} no-reseed waiting-retained published\n", waypoint, mask, to_underlying(trigger));
		fmt::print(stderr, "STCMD resolver-mask {} {} {} {} {} {}\n", waypoint, mask, to_underlying(trigger), object.GetReseedSum(), object.GetUsedRandomTriggers().base(), object.roadstop_scope.GetRandomBits());
	}
	Randomizer before = _random;
	st->waiting_random_triggers = {}; spec->cargo_triggers = 0; RoadStopUpdateCachedTriggers(st); TriggerRoadStopRandomisation(st, tile, StationRandomTrigger::VehicleArrives);
	StationNeed(st->waiting_random_triggers.None() && std::ranges::equal(_random.state, before.state), __LINE__);
	spec->cargo_triggers = 3; RoadStopUpdateCachedTriggers(st); TriggerRoadStopRandomisation(st, tile, StationRandomTrigger::NewCargo, 2);
	StationNeed(st->waiting_random_triggers.None() && std::ranges::equal(_random.state, before.state), __LINE__);
	if (!waypoint) {
		auto &cargo = Station::From(st)->goods[1].GetOrCreateData().cargo; StationNeed(CargoPacket::CanAllocateItem(), __LINE__); cargo.Append(new CargoPacket(st->index, 1, Source{st->town->index, SourceType::Town}), StationID::Invalid());
		TriggerRoadStopRandomisation(st, tile, StationRandomTrigger::CargoTaken, 0);
		StationNeed(st->waiting_random_triggers.Test(StationRandomTrigger::CargoTaken) && std::ranges::equal(_random.state, before.state), __LINE__); cargo.Truncate();
	}
	st->waiting_random_triggers = {}; spec->grf_prop.SetSpriteGroup(CargoGRFFileProps::SG_DEFAULT_NA, f.empty);
	delete publication; delete upper; delete lower;
	fmt::print(stderr, "STCMD resolver-gating {} cached cargo all-empty\n", waypoint);
}
static void StationRoadResolver(TileIndex tile, bool waypoint, DoCommandFlags test, DoCommandFlags exec)
{
	auto &f = RoadResolverSpecs(waypoint);
	TileIndexDiff step = waypoint ? TileDiffXY(0, 1) : TileDiffXY(1, 0);
	if (waypoint) for (uint i = 0; i < 2; i++) StationNeed(Command<CMD_BUILD_ROAD>::Do(exec, tile + i * step, ROAD_X, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid()).Succeeded(), __LINE__);
	auto build = [&](DoCommandFlags flags, uint16_t spec, uint8_t width, StationID join) { return waypoint ? Command<CMD_BUILD_ROAD_WAYPOINT>::Do(flags, tile, AXIS_X, 1, width, f.cls, spec, join, true) : Command<CMD_BUILD_ROAD_STOP>::Do(flags, tile, width, 1, RoadStopType::Bus, true, DIAGDIR_NE, ROADTYPE_ROAD, f.cls, spec, join, true); };
	auto remove = [&](TileIndex t) { return waypoint ? Command<CMD_REMOVE_FROM_ROAD_WAYPOINT>::Do(exec, t, t) : Command<CMD_REMOVE_ROAD_STOP>::Do(exec, t, 1, 1, RoadStopType::Bus, false); };
	StationNeed(build(test, f.indices[0], 2, NEW_STATION).Succeeded() && !IsAnyRoadStopTile(tile), __LINE__);
	StationNeed(build(exec, f.indices[0], 2, NEW_STATION).Succeeded(), __LINE__);
	BaseStation *st = BaseStation::GetByTile(tile); StationID id = st->index; st->name = waypoint ? "Owner410 waypoint" : "Owner410 stop";
	StationNeed(st->roadstop_speclist.size() == 2 && GetCustomRoadStopSpecIndex(tile) == 1 && GetCustomRoadStopSpecIndex(tile + step) == 1, __LINE__);
	AssignSpecToRoadStop(nullptr, st, 0); DeallocateSpecFromRoadStop(st, 0);
	SetCustomRoadStopSpecIndex(tile, 63); StationNeed(GetRoadStopSpec(tile) == nullptr, __LINE__); SetCustomRoadStopSpecIndex(tile, 1);
	RoadResolverState("assigned", st); RoadResolverQueries(st, tile, waypoint); RoadResolverRandom(st, tile, waypoint);
	StationNeed(remove(tile).Succeeded() && st->roadstop_speclist[1].spec == &f.specs[0], __LINE__);
	StationNeed(build(test, f.indices[1], 1, id).Succeeded() && st->roadstop_speclist.size() == 2, __LINE__);
	StationNeed(build(exec, f.indices[1], 1, id).Succeeded() && GetCustomRoadStopSpecIndex(tile) == 2, __LINE__);
	StationNeed(remove(tile).Succeeded() && st->roadstop_speclist.size() == 2, __LINE__);
	/* Missing-GRF slots remain occupied, while a genuine hole can be reused. */
	st->roadstop_speclist.resize(4); st->roadstop_speclist[3].grfid = 0xDEAD410;
	StationNeed(AllocateSpecToRoadStop(&f.specs[2], st) == 2, __LINE__); AssignSpecToRoadStop(&f.specs[2], st, 2);
	StationNeed(AllocateSpecToRoadStop(&f.specs[2], st) == 2 && AllocateSpecToRoadStop(&f.specs[3], st) == 4, __LINE__);
	DeallocateSpecFromRoadStop(st, 2); StationNeed(st->roadstop_speclist.size() == 4 && AllocateSpecToRoadStop(&f.specs[3], st) == 2, __LINE__);
	AssignSpecToRoadStop(&f.specs[2], st, 4); StationNeed(AllocateSpecToRoadStop(&f.specs[2], st) == 4, __LINE__);
	AssignSpecToRoadStop(&f.specs[2], st, 2); StationNeed(AllocateSpecToRoadStop(&f.specs[2], st) == 2, __LINE__);
	DeallocateSpecFromRoadStop(st, 2); DeallocateSpecFromRoadStop(st, 4); StationNeed(st->roadstop_speclist.size() == 4, __LINE__);
	for (uint8_t index = 2; index < 63; index++) { st->roadstop_speclist.resize(index + 1); st->roadstop_speclist[index].grfid = 0xDEAD410; }
	StationNeed(!AllocateSpecToRoadStop(&f.specs[3], st).has_value() && build(test, f.indices[3], 1, id).Failed() && !IsAnyRoadStopTile(tile), __LINE__);
	for (uint8_t index = 62; index > 1; index--) DeallocateSpecFromRoadStop(st, index);
	StationNeed(st->roadstop_speclist.size() == 2 && AllocateSpecToRoadStop(nullptr, st) == 0 && AllocateSpecToRoadStop(&f.specs[0], nullptr) == 1, __LINE__);
	StationNeed(remove(tile + step).Succeeded() && st->roadstop_speclist.empty() && st->cached_roadstop_anim_triggers.None() && st->cached_roadstop_cargo_triggers == 0, __LINE__);
	StationNeed(build(exec, f.indices[0], 2, id).Succeeded(), __LINE__);
	RoadResolverState("lifecycle", st);
	fmt::print(stderr, "STCMD resolver-lifecycle {} test execute reuse full partial clear shrink\n", waypoint);
}
static void StationRoadResolverReload(bool waypoint)
{
	BaseStation *st = nullptr;
	for (BaseStation *candidate : BaseStation::Iterate()) if (candidate->name == (waypoint ? "Owner410 waypoint" : "Owner410 stop")) st = candidate;
	StationNeed(st != nullptr && st->roadstop_speclist.size() == 2, __LINE__);
	StationNeed(st->roadstop_speclist[1].spec == nullptr && st->roadstop_speclist[1].grfid == (waypoint ? 0x41000002U : 0x41000001U), __LINE__);
	StationNeed(st->cached_roadstop_anim_triggers.None() && st->cached_roadstop_cargo_triggers == 0, __LINE__);
	RoadResolverState("missing-reload", st);
	auto &f = RoadResolverSpecs(waypoint); AssignSpecToRoadStop(&f.specs[0], st, 1); RoadStopUpdateCachedTriggers(st);
	StationNeed(st->cached_roadstop_anim_triggers.Test(StationAnimationTrigger::Built) && st->cached_roadstop_cargo_triggers == 3, __LINE__);
	RoadResolverState("reattached-cache", st); RoadResolverQueries(st, st->custom_roadstop_tile_data.front().tile, waypoint);
	fmt::print(stderr, "STCMD resolver-reload {} missing reattached cache\n", waypoint);
}

/* #366: complete transactions through the unchanged reference command bodies. */
static void StationWaypoints(TileIndex origin, Company *company, DoCommandFlags test, DoCommandFlags exec)
{
	auto at = [origin](int x, int y) { return origin + TileDiffXY(x, y); };
	auto check = [](const char *label, CommandCost cost, bool success = true) { StationCost(label, cost); StationNeed(cost.Succeeded() == success, __LINE__); };
	for (Axis axis : {AXIS_X, AXIS_Y}) {
		TileIndex rail = at(axis == AXIS_X ? 2 : 10, 2), road = at(axis == AXIS_X ? 2 : 10, 10);
		TileIndexDiff step = TileOffsByAxis(OtherAxis(axis));
		uint8_t width = axis == AXIS_X ? 1 : 2, height = axis == AXIS_X ? 2 : 1;
		auto build = [&](DoCommandFlags f, TileIndex t, uint8_t w, uint8_t h, StationID id, bool adjacent) { return Command<CMD_BUILD_RAIL_WAYPOINT>::Do(f, t, axis, w, h, STAT_CLASS_WAYP, 0, id, adjacent); };
		for (uint i = 0; i < 3; ++i) check("wp-track", Command<CMD_BUILD_SINGLE_RAIL>::Do(exec, rail + i * step, RAILTYPE_RAIL, AxisToTrack(axis), false));
		check("wp-invalid-axis", Command<CMD_BUILD_RAIL_WAYPOINT>::Do(test, rail, INVALID_AXIS, width, height, STAT_CLASS_WAYP, 0, NEW_STATION, true), false);
		check("wp-invalid-size", build(test, rail, 0, 0, NEW_STATION, true), false);
		check("wp-test", build(test, rail, width, height, NEW_STATION, true));
		check("wp-build", build(exec, rail, width, height, NEW_STATION, true));
		Waypoint *wp = Waypoint::GetByTile(rail); StationID id = wp->index;
		StationNeed(GetAxisForNewRailWaypoint(rail) == axis, __LINE__);
		check("wp-replace", build(exec, rail, width, height, id, true));
		check("wp-adjacent", build(exec, rail + 2 * step, 1, 1, StationID::Invalid(), false));
		StationNeed(GetStationIndex(rail + 2 * step) == id, __LINE__);
		TileIndex distant = rail + 4 * TileOffsByAxis(axis);
		check("wp-track", Command<CMD_BUILD_SINGLE_RAIL>::Do(exec, distant, RAILTYPE_RAIL, AxisToTrack(axis), false));
		check("wp-distant", build(exec, distant, 1, 1, id, true));
		std::string name = fmt::format("Migration waypoint {}", axis);
		check("wp-rename", Command<CMD_RENAME_WAYPOINT>::Do(exec, id, name));
		check("wp-duplicate", Command<CMD_RENAME_WAYPOINT>::Do(test, id, name), false);
		check("wp-long-name", Command<CMD_RENAME_WAYPOINT>::Do(test, id, std::string(32, 'x')), false);
		check("wp-move", std::get<0>(Command<CMD_MOVE_WAYPOINT_NAME>::Do(exec, id, rail + step)));
		check("wp-move-outside", std::get<0>(Command<CMD_MOVE_WAYPOINT_NAME>::Do(test, id, at(30, 30))), false);
		Owner owner = wp->owner; wp->owner = OWNER_NONE;
		check("wp-neutral-rename", Command<CMD_RENAME_WAYPOINT>::Do(test, id, "Neutral")); wp->owner = owner;
		check("wp-reset", Command<CMD_RENAME_WAYPOINT>::Do(exec, id, ""));
		check("wp-rename", Command<CMD_RENAME_WAYPOINT>::Do(exec, id, name));
		for (uint i = 0; i < 3; ++i) check("wp-remove", Command<CMD_REMOVE_FROM_RAIL_WAYPOINT>::Do(exec, rail + i * step, rail + i * step, true));
		check("wp-remove-distant", Command<CMD_REMOVE_FROM_RAIL_WAYPOINT>::Do(exec, distant, distant, true));
		check("wp-reuse", build(exec, rail, width, height, StationID::Invalid(), false));
		StationNeed(GetStationIndex(rail) == id && Waypoint::Get(id)->name == name, __LINE__);
		for (uint i = 0; i < 3; ++i) check("wp-road", Command<CMD_BUILD_ROAD>::Do(exec, road + i * step, axis == AXIS_X ? ROAD_X : ROAD_Y, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid()));
		auto build_road = [&](DoCommandFlags f, TileIndex t, uint8_t w, uint8_t h, StationID join, bool adjacent) { return Command<CMD_BUILD_ROAD_WAYPOINT>::Do(f, t, axis, w, h, ROADSTOP_CLASS_WAYP, 0, join, adjacent); };
		check("wp-road-test", build_road(test, road, width, height, NEW_STATION, true));
		check("wp-road-build", build_road(exec, road, width, height, NEW_STATION, true));
		StationID rid = GetStationIndex(road); StationNeed(GetAxisForNewRoadWaypoint(road) == axis, __LINE__);
		TileIndex road_distant = road + 4 * TileOffsByAxis(axis);
		check("wp-road", Command<CMD_BUILD_ROAD>::Do(exec, road_distant, axis == AXIS_X ? ROAD_X : ROAD_Y, ROADTYPE_ROAD, DRD_NONE, TownID::Invalid()));
		check("wp-road-distant", build_road(exec, road_distant, 1, 1, rid, true));
		check("wp-road-replace", build_road(exec, road, width, height, rid, true));
		check("wp-road-join", build_road(exec, road + 2 * step, 1, 1, StationID::Invalid(), false));
		StationNeed(GetStationIndex(road + 2 * step) == rid, __LINE__);
		/* A real vehicle occupies the replacement tile. Restore its spatial hash before saving. */
		RoadVehicle *v = nullptr; for (RoadVehicle *candidate : RoadVehicle::Iterate()) if (!candidate->vehstatus.Test(VehState::Hidden)) { v = candidate; break; }
		StationNeed(v != nullptr, __LINE__); TileIndex old_tile = v->tile; int old_x = v->x_pos, old_y = v->y_pos, old_z = v->z_pos;
		v->tile = road; v->x_pos = TileX(road) * TILE_SIZE + 8; v->y_pos = TileY(road) * TILE_SIZE + 8; v->z_pos = GetTileZ(road); v->UpdatePosition();
		check("wp-occupied", build_road(test, road, width, height, rid, true), false);
		v->tile = old_tile; v->x_pos = old_x; v->y_pos = old_y; v->z_pos = old_z; v->UpdatePosition();
		check("wp-road-remove", Command<CMD_REMOVE_FROM_ROAD_WAYPOINT>::Do(exec, road, road + 2 * step));
		check("wp-road-remove-distant", Command<CMD_REMOVE_FROM_ROAD_WAYPOINT>::Do(exec, road_distant, road_distant));
		check("wp-road-reuse", build_road(exec, road, width, height, StationID::Invalid(), false));
		StationNeed(GetStationIndex(road) == rid, __LINE__);
		check("wp-road-rename", Command<CMD_RENAME_WAYPOINT>::Do(exec, rid, fmt::format("Migration road waypoint {}", axis)));
	}
	/* Deleted candidates at the same distance: retain pool order, not spatial order. */
	StationID ids[2];
	for (uint i = 0; i < 2; ++i) {
		TileIndex tile = at(2 + 4 * i, 22);
		check("wp-tie-track", Command<CMD_BUILD_SINGLE_RAIL>::Do(exec, tile, RAILTYPE_RAIL, TRACK_X, false));
		check("wp-tie-build", Command<CMD_BUILD_RAIL_WAYPOINT>::Do(exec, tile, AXIS_X, 1, 1, STAT_CLASS_WAYP, 0, NEW_STATION, true)); ids[i] = GetStationIndex(tile);
		check("wp-tie-remove", Command<CMD_REMOVE_FROM_RAIL_WAYPOINT>::Do(exec, tile, tile, true));
	}
	check("wp-tie-track", Command<CMD_BUILD_SINGLE_RAIL>::Do(exec, at(4, 22), RAILTYPE_RAIL, TRACK_X, false));
	check("wp-tie-reuse", Command<CMD_BUILD_RAIL_WAYPOINT>::Do(exec, at(4, 22), AXIS_X, 1, 1, STAT_CLASS_WAYP, 0, StationID::Invalid(), false));
	StationNeed(GetStationIndex(at(4, 22)) == std::min(ids[0], ids[1]), __LINE__);
	fmt::print(stderr, "STCMD waypoints axes reuse tie occupied {} {} {}\n", company->infrastructure.station, company->infrastructure.rail[RAILTYPE_RAIL], company->infrastructure.road[ROADTYPE_ROAD]);
}

/* #378: real graph mutations plus arithmetic outside ordinary play's range. */
static void StationGraphs()
{
	constexpr CargoType cargo = 0;
	std::array<Station *, 4> stations; std::array<LinkGraphID, 4> graphs; std::array<NodeID, 4> nodes;
	auto it = Station::Iterate().begin();
	for (size_t i = 0; i < stations.size(); i++, ++it) { StationNeed(it != Station::Iterate().end(), __LINE__); stations[i] = *it; graphs[i] = stations[i]->goods[cargo].link_graph; nodes[i] = stations[i]->goods[cargo].node; }
	auto join = [cargo](LinkGraph *g, Station *st) { st->goods[cargo].link_graph = g->index; st->goods[cargo].node = g->AddNode(st); };
	StationNeed(LinkGraph::CanAllocateItem(2), __LINE__);
	LinkGraph *first = new LinkGraph(cargo), *second = new LinkGraph(cargo);
	Backup<GoodsEntry::States> acceptance(stations[2]->goods[cargo].status);
	stations[2]->goods[cargo].status.Set(GoodsEntry::State::Acceptance);
	join(first, stations[0]); join(first, stations[1]); join(second, stations[2]); join(second, stations[3]);
	(*first)[0].AddEdge(1, 65535, 5, 120, {EdgeUpdateMode::Increase, EdgeUpdateMode::Restricted});
	(*second)[0].UpdateEdge(1, 65536, 4, 17, {EdgeUpdateMode::Unrestricted});
	(*second)[0].UpdateSupply(11);
	LinkGraph copy(*second);
	stations[2]->goods[cargo].status.Reset(GoodsEntry::State::Acceptance);
	first->Merge(second);
#ifdef WITH_RUST
	StationNeed(*openttd_rust_live_node_fields(first->RustState(), 2).demand == 0, __LINE__);
#else
	StationNeed((*first)[2].demand == 0, __LINE__);
#endif
	fmt::print(stderr, "STCMD graph merge {} {} {} {} {}\n", first->Size(), copy.Size(), stations[2]->goods[cargo].node, (*first)[2][3].LastUpdate().base(), (*first)[2][3].travel_time_sum);
	first->RemoveNode(1);
	fmt::print(stderr, "STCMD graph nonlast {} {} {}\n", first->Size(), stations[3]->goods[cargo].node, (*first)[2][1].dest_node);
	first->RemoveNode(first->Size() - 1);
	fmt::print(stderr, "STCMD graph last {}\n", first->Size());
	acceptance.Restore();
	for (size_t i = 0; i < stations.size(); i++) { stations[i]->goods[cargo].link_graph = graphs[i]; stations[i]->goods[cargo].node = nodes[i]; }
	delete first;
	for (uint cap : {1U, 65535U, 65536U, UINT32_MAX}) {
		LinkGraph graph(cargo); graph.Init(1);
		graph[0].UpdateSupply(UINT32_MAX); graph[0].UpdateSupply(2);
		{
#ifdef WITH_RUST
			auto &edge = graph[0].AppendEdge();
#else
			auto &edge = graph[0].edges.emplace_back();
#endif
			edge.dest_node = 0; edge.capacity = cap; edge.usage = cap - 1; edge.travel_time_sum = UINT64_MAX - 1;
			edge.last_restricted_update = TimerGameEconomy::date;
		}
		graph.Compress(); graph.ShiftDates(TimerGameEconomy::Date{9});
		const auto &edge = graph[0][0];
		fmt::print(stderr, "STCMD graph compress {} {} {} {} {} {} {}\n", cap, edge.capacity, edge.usage, edge.travel_time_sum, edge.last_unrestricted_update.base(), edge.LastUpdate().base(), edge.TravelTime());
		fmt::print(stderr, "STCMD graph scale {} {} {}\n", graph.Monthly(UINT32_MAX), LinkGraph::Scale(edge.travel_time_sum, TimerGameEconomy::Date{257}, TimerGameEconomy::Date{3}), LinkGraph::Scale(0, TimerGameEconomy::Date{1}, TimerGameEconomy::Date{-1}));
	}
	for (uint64_t sum : {uint64_t{0}, UINT64_MAX - 1}) {
		LinkGraph::BaseEdge edge(0); edge.capacity = UINT32_MAX - 2; edge.usage = UINT32_MAX - 3; edge.travel_time_sum = sum;
		edge.Update(7, 6, 100, {EdgeUpdateMode::Increase, EdgeUpdateMode::Refresh, EdgeUpdateMode::Restricted});
		edge.Update(11, 3, 0, {EdgeUpdateMode::Refresh, EdgeUpdateMode::Unrestricted});
		edge.Update(5, 1, 0, {EdgeUpdateMode::Increase});
		edge.Restrict(); edge.Release();
		fmt::print(stderr, "STCMD graph update {} {} {} {} {}\n", edge.capacity, edge.usage, edge.travel_time_sum, edge.TravelTime(), edge.LastUpdate().base());
	}
	fmt::print(stderr, "STCMD complete\n");
}
// OWNER399
static bool StationCommands(std::span<std::string_view> args)
{
	if (args.size() < 2) return false;
	if (args[1] == "linkgraph") { StationGraphs(); return true; }
	Company *company = *Company::Iterate().begin();
	RoadVehicle *bus = nullptr;
	if (args[1] == "removed-destination") {
		for (RoadVehicle *v : RoadVehicle::Iterate()) if (v->IsPrimaryVehicle() && v->cargo_type == 0 && !v->IsInDepot()) { bus = v; break; }
		StationNeed(bus != nullptr, __LINE__);
		company = Company::Get(bus->owner);
	}
	Backup<CompanyID> current(_current_company, company->index);
	/* Declare the builtin tram type available without an external vehicle GRF. */
	company->avail_roadtypes.Set(ROADTYPE_TRAM);
	_settings_game.station.distant_join_stations = true;
	_settings_game.station.never_expire_airports = true;
	_settings_game.construction.road_stop_on_town_road = true;
	_settings_game.construction.road_stop_on_competitor_road = true;
	const DoCommandFlags test{DoCommandFlag::NoTestTownRating};
	const DoCommandFlags exec{DoCommandFlag::Execute, DoCommandFlag::NoTestTownRating};
	if (args[1] == "airport-inspect") { StationAirportFixture(INVALID_TILE, test, exec, true); fmt::print(stderr, "STCMD complete\n"); current.Restore(); return true; }
	if (args[1] == "resolver-inspect") { StationResolverFixture(INVALID_TILE, exec, true); fmt::print(stderr, "STCMD complete\n"); current.Restore(); return true; }
	if (args[1] == "waypoints-inspect") {
		StationRoadResolverReload(true);
		uint found = 0;
		for (Waypoint *wp : Waypoint::Iterate()) if (wp->name.starts_with("Migration ")) { ++found; StationCost("wp-reload-name", Command<CMD_RENAME_WAYPOINT>::Do(test, wp->index, wp->name)); }
		StationNeed(found == 4, __LINE__); fmt::print(stderr, "STCMD complete\n"); current.Restore(); return true;
	}
	if (args[1] == "road-stops-inspect") {
		StationRoadResolverReload(false); StationRoadStops("reloaded"); fmt::print(stderr, "STCMD complete\n");
		current.Restore(); return true;
	}
	if (args[1] == "inspect") {
		Station *found = nullptr;
		for (Station *st : Station::Iterate()) if (st->name == "Migration station") found = st;
		StationNeed(found != nullptr, __LINE__);
		StationCost("reload-name", Command<CMD_RENAME_STATION>::Do(exec, found->index, "Reloaded station"));
		UpdateStationDockingTiles(found);
		fmt::print(stderr, "STCMD reloaded {} {}\n", found->index.base(), found->facilities.base());
	} else {
		TileIndex origin = StationPatch();
		auto tile = [origin](int x, int y) { return origin + TileDiffXY(x, y); };
		if (args[1] == "airport") { StationAirportFixture(origin, test, exec); fmt::print(stderr, "STCMD complete\n"); if (args.size() == 3) SaveOrLoad(std::string(args[2]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false); current.Restore(); return true; }
		if (args[1] == "resolver") { StationResolverFixture(origin, exec); fmt::print(stderr, "STCMD complete\n"); if (args.size() == 3) SaveOrLoad(std::string(args[2]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false); current.Restore(); return true; }
		if (args[1] == "waypoints") {
			StationRoadResolver(tile(25, 25), true, test, exec); StationWaypoints(origin, company, test, exec); fmt::print(stderr, "STCMD complete\n");
			if (args.size() == 3) SaveOrLoad(std::string(args[2]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false);
			current.Restore(); return true;
		}
		if (args[1] == "removed-destination") {
			TileIndex stop = tile(12, 1);
			StationNeed(Command<CMD_BUILD_ROAD_STOP>::Do(exec, stop, 1, 1, RoadStopType::Bus, false, DIAGDIR_NE, ROADTYPE_ROAD, ROADSTOP_CLASS_DFLT, 0, NEW_STATION, true).Succeeded(), __LINE__);
			StationID id = GetStationIndex(stop);
			StationNeed(Command<CMD_DELETE_ORDER>::Do(exec, bus->index, VehicleOrderID(255)).Succeeded(), __LINE__);
			Order order; order.MakeGoToStation(id);
			StationNeed(OrderList::CanAllocateItem(), __LINE__);
			InsertOrder(bus, Order(order), 0);
			bus->current_order = order;
			bus->cur_implicit_order_index = bus->cur_real_order_index = 0;
			bus->SetDestTile(stop);
			/* A cached route while en route, through the vehicle owner's public API. */
			StationPathPush(bus, TRACKDIR_X_NE, bus->tile);
			StationNeed(bus->tile != stop && StationPathSize(bus) != 0 && bus->GetNumOrders() == 1, __LINE__);
			StationCost("removed-destination", Command<CMD_REMOVE_ROAD_STOP>::Do(exec, stop, 1, 1, RoadStopType::Bus, false));
			fmt::print(stderr, "STCMD removed-destination {} {} {}\n", bus->dest_tile.base(), StationPathSize(bus), IsAnyRoadStopTile(stop));
			StationNeed(bus->dest_tile == TileIndex{} && StationPathSize(bus) == 0 && !IsAnyRoadStopTile(stop), __LINE__);
			fmt::print(stderr, "STCMD complete\n");
			current.Restore(); return true;
		}
		if (args[1] == "map-edges") {
			bool all_edges = true;
			DoCommandFlags edge_test = test; edge_test.Set(DoCommandFlag::Auto);
			for (Axis axis : {AXIS_X, AXIS_Y}) {
				TileIndex start = axis == AXIS_X ? TileXY(Map::MaxX() - 1, 8) : TileXY(8, Map::MaxY() - 1);
				uint8_t w = axis == AXIS_X ? 2 : 1, h = axis == AXIS_Y ? 2 : 1;
				for (uint dy = 0; dy < 2; dy++) for (uint dx = 0; dx < 2; dx++) SetTileHeight(start + TileDiffXY(dx, dy), 1);
				/* An obstacle on the first tile exposes early-error precedence. */
				MakeRoadNormal(start, ROAD_ALL, ROADTYPE_ROAD, INVALID_ROADTYPE, TownID(0), OWNER_TOWN, OWNER_NONE);
				auto rejected = [&](const char *label, const CommandCost &cost) {
					StationCost(label, cost);
					all_edges &= cost.Failed() && cost.GetErrorMessage() == INVALID_STRING_ID;
				};
				rejected("edge-rail", Command<CMD_BUILD_RAIL_STATION>::Do(edge_test, start, RAILTYPE_RAIL, AXIS_X, h, w, STAT_CLASS_DFLT, 0, NEW_STATION, true));
				rejected("edge-road", Command<CMD_BUILD_ROAD_STOP>::Do(edge_test, start, w, h, RoadStopType::Bus, false, DIAGDIR_NE, ROADTYPE_ROAD, ROADSTOP_CLASS_DFLT, 0, NEW_STATION, true));
				MakeClear(start, CLEAR_GRASS, 3);
				StationNeed(Command<CMD_BUILD_ROAD_STOP>::Do(exec, start, 1, 1, RoadStopType::Bus, false, DIAGDIR_NE, ROADTYPE_ROAD, ROADSTOP_CLASS_DFLT, 0, NEW_STATION, true).Succeeded(), __LINE__);
				rejected("edge-remove-test", Command<CMD_REMOVE_ROAD_STOP>::Do(test, start, w, h, RoadStopType::Bus, false));
				rejected("edge-remove-exec", Command<CMD_REMOVE_ROAD_STOP>::Do(exec, start, w, h, RoadStopType::Bus, false));
				all_edges &= IsBayRoadStopTile(start);
				if (IsBayRoadStopTile(start)) StationNeed(Command<CMD_REMOVE_ROAD_STOP>::Do(exec, start, 1, 1, RoadStopType::Bus, false).Succeeded(), __LINE__);
				fmt::print(stderr, "STCMD map-edge {} preserved {}\n", axis, all_edges);
			}
			StationNeed(all_edges, __LINE__);
			fmt::print(stderr, "STCMD complete\n");
			current.Restore(); return true;
		}
		if (args[1] == "road-stops") {
			StationRoadResolver(tile(25, 25), false, test, exec);
			/* Seven new single-part buses, built in a depot and placed by the fixture. */
			StationNeed(Command<CMD_BUILD_ROAD_DEPOT>::Do(exec, tile(4, 4), ROADTYPE_ROAD, DIAGDIR_NE).Succeeded(), __LINE__);
			EngineID engine = EngineID::Invalid();
			for (const Engine *e : Engine::IterateType(VEH_ROAD)) {
				if (engine == EngineID::Invalid() && IsEngineBuildable(e->index, VEH_ROAD, company->index) && e->VehInfo<RoadVehicleInfo>().roadtype == ROADTYPE_ROAD && !e->info.callback_mask.Test(VehicleCallbackMask::ArticEngine)) engine = e->index;
			}
			StationNeed(engine != EngineID::Invalid(), __LINE__);
			std::vector<RoadVehicle *> buses;
			for (int i = 0; i < 7; i++) {
				auto [cost, id, refit, capacity, cargo] = Command<CMD_BUILD_VEHICLE>::Do(exec, tile(4, 4), engine, false, INVALID_CARGO, INVALID_CLIENT_ID);
				StationNeed(cost.Succeeded() && RoadVehicle::Get(id)->Next() == nullptr, __LINE__);
				buses.push_back(RoadVehicle::Get(id));
				buses.back()->vehstatus.Reset(VehState::Hidden);
			}
			for (int x = 8; x < 17; x++) MakeRoadNormal(tile(x, 10), ROAD_X, ROADTYPE_ROAD, INVALID_ROADTYPE, TownID(0), OWNER_TOWN, OWNER_NONE);
			for (int y = 8; y < 15; y++) MakeRoadNormal(tile(20, y), ROAD_Y, ROADTYPE_ROAD, INVALID_ROADTYPE, TownID(0), OWNER_TOWN, OWNER_NONE);
			auto stop = [&](TileIndex t, bool drive, Axis axis, StationID join) {
				DiagDirection ddir = drive ? (axis == AXIS_X ? DIAGDIR_NE : DIAGDIR_SE) : DIAGDIR_NE;
				StationCost("road-stop", Command<CMD_BUILD_ROAD_STOP>::Do(exec, t, 1, 1, RoadStopType::Bus, drive, ddir, ROADTYPE_ROAD, ROADSTOP_CLASS_DFLT, 0, join, true));
				StationNeed(IsStationRoadStop(t), __LINE__);
			};
			auto remove = [&](TileIndex t) {
				StationCost("road-stop-remove", Command<CMD_REMOVE_ROAD_STOP>::Do(exec, t, 1, 1, RoadStopType::Bus, false));
				StationNeed(!IsAnyRoadStopTile(t), __LINE__);
			};
			/* X axis: alone, then a southern join that makes the new stop the parent. */
			stop(tile(11, 10), true, AXIS_X, NEW_STATION);
			StationID id = GetStationIndex(tile(11, 10));
			StationRoadEnter(buses[0], tile(11, 10), DIR_NE, TRACKDIR_X_NE);
			stop(tile(10, 10), true, AXIS_X, id);
			stop(tile(13, 10), true, AXIS_X, id);
			StationRoadEnter(buses[1], tile(13, 10), DIR_SW, TRACKDIR_X_SW);
			StationRoadStops("x-parts");
			/* Merge two occupied lines, then join one more tile from the north. */
			stop(tile(12, 10), true, AXIS_X, id);
			stop(tile(14, 10), true, AXIS_X, id);
			StationRoadStops("x-joined");
			/* Y axis, both directions. */
			stop(tile(20, 11), true, AXIS_Y, id);
			StationRoadEnter(buses[2], tile(20, 11), DIR_SE, TRACKDIR_Y_SE);
			stop(tile(20, 12), true, AXIS_Y, id);
			StationRoadEnter(buses[3], tile(20, 12), DIR_NW, TRACKDIR_Y_NW);
			stop(tile(20, 10), true, AXIS_Y, id);
			StationRoadStops("y-joined");
			/* Interior removal splits and rebuilds both lines; then head and tail. */
			remove(tile(12, 10));
			StationRoadStops("x-split");
			remove(tile(10, 10));
			remove(tile(14, 10));
			StationRoadStops("x-ends");
			/* Leaving through deletion, then the last tile of a line. */
			delete buses[3];
			remove(tile(20, 12));
			remove(tile(20, 10));
			StationRoadStops("y-ends");
			delete buses[2];
			remove(tile(20, 11));
			StationRoadStops("y-last");
			/* Bays: first bay, busy refusal, second bay, full refusal. */
			stop(tile(24, 4), false, AXIS_X, NEW_STATION);
			RoadStop *bay = nullptr;
			for (RoadStop *rs : RoadStop::Iterate()) if (rs->xy == tile(24, 4)) bay = rs;
			StationNeed(bay != nullptr, __LINE__);
			StationRoadEnter(buses[4], tile(24, 4), DIR_SW, TRACKDIR_X_SW);
			StationRoadEnter(buses[5], tile(24, 4), DIR_SW, TRACKDIR_X_SW);
			bay->status.Reset(RoadStop::RoadStopStatusFlag::EntryBusy);
			StationRoadEnter(buses[5], tile(24, 4), DIR_SW, TRACKDIR_X_SW);
			StationRoadStops("bays");
			bay->status.Reset(RoadStop::RoadStopStatusFlag::EntryBusy);
			StationRoadEnter(buses[6], tile(24, 4), DIR_SW, TRACKDIR_X_SW);
			StationRoadStops("bays-full");
			/* Free bay 1, then admit the previously refused vehicle into it. */
#ifdef WITH_RUST
			openttd_rust_road_stop_leave(&GetRustSharedServices(), RustVehicle(buses[5]), buses[5]->GetRustState());
#else
			bay->Leave(buses[5]);
#endif
			delete buses[5];
			StationRoadStops("bays-freed");
			StationRoadEnter(buses[6], tile(24, 4), DIR_SW, TRACKDIR_X_SW);
			StationRoadStops("bays-reused");
			fmt::print(stderr, "STCMD complete\n");
			if (args.size() == 3) SaveOrLoad(std::string(args[2]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false);
			current.Restore(); return true;
		}
		if (args[1] == "railtype-63") {
			RailType last = RAILTYPE_RAIL;
			for (uint32_t label = 1; ; label++) {
				RailType rt = AllocateRailType(label);
				if (rt == INVALID_RAILTYPE) break;
				company->avail_railtypes.Set(rt); last = rt;
			}
			StationNeed(last == 63, __LINE__);
			TileIndex plain = tile(1, 1);
			StationNeed(Command<CMD_BUILD_SINGLE_RAIL>::Do(exec, plain, last, TRACK_X, false).Succeeded(), __LINE__);
			for (DoCommandFlags flags : {test, exec}) {
				flags.Set(DoCommandFlag::Auto);
				auto cost = Command<CMD_BUILD_RAIL_STATION>::Do(flags, plain, last, AXIS_X, 1, 1, STAT_CLASS_DFLT, 0, NEW_STATION, true);
				StationCost("railtype-63", cost); StationNeed(cost.Succeeded(), __LINE__);
			}
			StationNeed(IsRailStationTile(plain) && GetRailType(plain) == last, __LINE__);
			/* Remove the synthetic type before saving; no external GRF is required. */
			StationNeed(Command<CMD_LANDSCAPE_CLEAR>::Do(exec, plain).Succeeded(), __LINE__);
			fmt::print(stderr, "STCMD railtype-63 overbuilt\nSTCMD complete\n");
			current.Restore(); return true;
		}
		auto rail = [&](DoCommandFlags f, TileIndex t, StationID join, uint8_t n = 2, uint8_t len = 3) { return Command<CMD_BUILD_RAIL_STATION>::Do(f, t, RAILTYPE_RAIL, AXIS_X, n, len, STAT_CLASS_DFLT, 0, join, true); };
		StationCost("rail-test", rail(test, tile(1, 1), NEW_STATION));
		StationCost("rail-build", rail(exec, tile(1, 1), NEW_STATION));
		StationNeed(IsRailStationTile(tile(1, 1)), __LINE__);
		StationID station = GetStationIndex(tile(1, 1));
		StationCost("rail-bridge-low", CheckBuildAbove_Station(tile(1, 1), test, AXIS_Y, 4));
		StationCost("rail-bridge-high", CheckBuildAbove_Station(tile(1, 1), test, AXIS_Y, 12));
		StationCost("rail-extension", rail(exec, tile(1, 3), station));
		StationCost("rail-invalid", rail(test, tile(1, 8), NEW_STATION, 0));
		StationCost("rail-overbuild", rail(test, tile(1, 1), NEW_STATION));
		StationCost("rename-test", Command<CMD_RENAME_STATION>::Do(test, station, "Migration station"));
		StationCost("rename", Command<CMD_RENAME_STATION>::Do(exec, station, "Migration station"));
		StationCost("move-sign", std::get<0>(Command<CMD_MOVE_STATION_NAME>::Do(exec, station, tile(1, 2))));
		StationCost("rename-missing", Command<CMD_RENAME_STATION>::Do(test, StationID::Invalid(), "Missing"));
		StationCost("terraform-rail", TerraformTile_Station(tile(1, 1), test, GetTileZ(tile(1, 1)), SLOPE_FLAT));
		StationCost("clear-auto", ClearTile_Station(tile(1, 1), DoCommandFlag::Auto));
		StationCost("rail-other", rail(exec, tile(7, 1), NEW_STATION, 1, 1));
		StationID other = GetStationIndex(tile(7, 1));
		StationCost("rename-duplicate", Command<CMD_RENAME_STATION>::Do(test, other, "Migration station"));
		/* Mixed ownership reaches the original accumulated-error/partial effects. */
		Station::Get(other)->owner = OWNER_NONE;
		StationCost("rail-partial-test", CmdRemoveFromRailStation(test, tile(1, 1), tile(7, 1), true));
		StationCost("rail-partial", CmdRemoveFromRailStation(exec, tile(1, 1), tile(7, 1), true));
		Station::Get(other)->owner = company->index;
		StationCost("clear-rail", Command<CMD_LANDSCAPE_CLEAR>::Do(exec, tile(7, 1)));
		StationCost("rail-delete", Command<CMD_REMOVE_FROM_RAIL_STATION>::Do(exec, tile(1, 2), tile(3, 4), true));
		StationCost("rail-reuse", rail(exec, tile(1, 1), StationID::Invalid(), 1, 2));
		fmt::print(stderr, "STCMD reuse {}\n", GetStationIndex(tile(1, 1)) == station);
		StationCost("waypoint-build", Command<CMD_BUILD_RAIL_WAYPOINT>::Do(exec, tile(1, 2), AXIS_X, 1, 2, STAT_CLASS_WAYP, 0, NEW_STATION, true));
		StationNeed(IsRailWaypointTile(tile(1, 2)), __LINE__);
		StationCost("waypoint-remove", Command<CMD_REMOVE_FROM_RAIL_WAYPOINT>::Do(exec, tile(1, 2), tile(1, 3), true));
		auto road = [&](DoCommandFlags f, TileIndex t, bool drive, StationID join) { return Command<CMD_BUILD_ROAD_STOP>::Do(f, t, 2, 1, RoadStopType::Bus, drive, DIAGDIR_NE, ROADTYPE_ROAD, ROADSTOP_CLASS_DFLT, 0, join, true); };
		StationCost("bay-test", road(test, tile(12, 1), false, NEW_STATION));
		StationCost("bay-build", road(exec, tile(12, 1), false, NEW_STATION));
		StationNeed(IsBayRoadStopTile(tile(12, 1)), __LINE__);
		StationCost("bay-bridge-low", CheckBuildAbove_Station(tile(12, 1), test, AXIS_Y, 4));
		StationCost("bay-bridge-high", CheckBuildAbove_Station(tile(12, 1), test, AXIS_Y, 12));
		StationCost("bay-remove", Command<CMD_REMOVE_ROAD_STOP>::Do(exec, tile(12, 1), 2, 1, RoadStopType::Bus, false));
		for (int x = 12; x < 14; x++) MakeRoadNormal(tile(x, 4), ROAD_X, ROADTYPE_ROAD, INVALID_ROADTYPE, TownID(0), OWNER_TOWN, OWNER_NONE);
		StationCost("drive-test", road(test, tile(12, 4), true, NEW_STATION));
		StationCost("drive-build", road(exec, tile(12, 4), true, NEW_STATION));
		StationNeed(IsDriveThroughStopTile(tile(12, 4)), __LINE__);
		StationCost("drive-remove", Command<CMD_REMOVE_ROAD_STOP>::Do(exec, tile(12, 4), 2, 1, RoadStopType::Bus, false));
		fmt::print(stderr, "STCMD road-owner {}\n", GetRoadOwner(tile(12, 4), RTT_ROAD) == OWNER_TOWN);
		StationCost("tram-test", Command<CMD_BUILD_ROAD_STOP>::Do(test, tile(12, 7), 1, 1, RoadStopType::Bus, true, DIAGDIR_NE, ROADTYPE_TRAM, ROADSTOP_CLASS_DFLT, 0, NEW_STATION, true));
		StationCost("tram-build", Command<CMD_BUILD_ROAD_STOP>::Do(exec, tile(12, 7), 1, 1, RoadStopType::Bus, true, DIAGDIR_NE, ROADTYPE_TRAM, ROADSTOP_CLASS_DFLT, 0, NEW_STATION, true));
		StationNeed(IsDriveThroughStopTile(tile(12, 7)), __LINE__);
		StationCost("tram-remove", Command<CMD_REMOVE_ROAD_STOP>::Do(exec, tile(12, 7), 1, 1, RoadStopType::Bus, false));
		StationCost("road-waypoint-test", Command<CMD_BUILD_ROAD_WAYPOINT>::Do(test, tile(12, 4), AXIS_X, 1, 1, ROADSTOP_CLASS_WAYP, 0, NEW_STATION, true));
		StationCost("road-waypoint-build", Command<CMD_BUILD_ROAD_WAYPOINT>::Do(exec, tile(12, 4), AXIS_X, 1, 1, ROADSTOP_CLASS_WAYP, 0, NEW_STATION, true));
		StationNeed(IsRoadWaypointTile(tile(12, 4)), __LINE__);
		StationCost("road-waypoint-remove", Command<CMD_REMOVE_FROM_ROAD_WAYPOINT>::Do(exec, tile(12, 4), tile(12, 4)));
		StationCost("airport-test", Command<CMD_BUILD_AIRPORT>::Do(test, tile(20, 1), AT_SMALL, 0, NEW_STATION, true));
		StationCost("airport-build", Command<CMD_BUILD_AIRPORT>::Do(exec, tile(20, 1), AT_SMALL, 0, NEW_STATION, true));
		StationNeed(IsAirportTile(tile(20, 1)), __LINE__);
		StationID airport = GetStationIndex(tile(20, 1));
		Backup<bool> noise_setting(_settings_game.economy.station_noise_level, true);
		uint distance; Town *nearest = AirportGetNearestTown(AirportSpec::Get(AT_SMALL), Station::Get(airport)->airport.rotation, tile(20, 1), AirportTileIterator(Station::Get(airport)), distance);
		Backup<uint16_t> noise(nearest->noise_reached, UINT16_MAX);
		auto noisy = Command<CMD_BUILD_AIRPORT>::Do(test, tile(20, 1), AT_SMALL, 0, NEW_STATION, true);
		StationCost("airport-noise-limit", noisy); StationNeed(noisy.GetErrorMessage() == STR_ERROR_LOCAL_AUTHORITY_REFUSES_NOISE, __LINE__);
		noise.Restore(); noise_setting.Restore();
		StationCost("airport-bridge-low", CheckBuildAbove_Station(tile(20, 1), test, AXIS_Y, 4));
		StationCost("airport-bridge-high", CheckBuildAbove_Station(tile(20, 1), test, AXIS_Y, 12));
		StationCost("airport-close-test", Command<CMD_OPEN_CLOSE_AIRPORT>::Do(test, airport));
		StationCost("airport-close", Command<CMD_OPEN_CLOSE_AIRPORT>::Do(exec, airport));
		StationCost("airport-open", Command<CMD_OPEN_CLOSE_AIRPORT>::Do(exec, airport));
		UpdateAirportsNoise();
		StationCost("airport-remove-test", Command<CMD_LANDSCAPE_CLEAR>::Do(test, tile(20, 1)));
		StationCost("airport-remove", Command<CMD_LANDSCAPE_CLEAR>::Do(exec, tile(20, 1)));
		/* A deterministic synthetic shoreline input, followed by real dock commands. */
		for (int y = 16; y < 23; y++) for (int x = 0; x < 8; x++) { SetTileHeight(tile(x, y), 0); MakeSea(tile(x, y)); }
		SetTileHeight(tile(3, 16), 1); SetTileHeight(tile(4, 16), 1); MakeClear(tile(3, 16), CLEAR_GRASS, 3);
		StationCost("dock-invalid", Command<CMD_BUILD_DOCK>::Do(test, tile(9, 16), NEW_STATION, true));
		StationCost("dock-test", Command<CMD_BUILD_DOCK>::Do(test, tile(3, 16), NEW_STATION, true));
		StationCost("dock-build", Command<CMD_BUILD_DOCK>::Do(exec, tile(3, 16), NEW_STATION, true));
		StationNeed(IsDockTile(tile(3, 16)), __LINE__);
		Station *dock = Station::GetByTile(tile(3, 16));
		StationCost("dock-bridge-low", CheckBuildAbove_Station(tile(3, 16), test, AXIS_Y, 4));
		StationCost("dock-bridge-high", CheckBuildAbove_Station(tile(3, 16), test, AXIS_Y, 12));
		UpdateStationDockingTiles(dock); CheckForDockingTile(tile(3, 18));
		fmt::print(stderr, "STCMD docking {}\n", IsPossibleDockingTile(tile(3, 18)));
		StationCost("dock-remove-test", Command<CMD_LANDSCAPE_CLEAR>::Do(test, tile(3, 16)));
		StationCost("dock-remove", Command<CMD_LANDSCAPE_CLEAR>::Do(exec, tile(3, 16)));
		StationNeed(Industry::CanAllocateItem(), __LINE__);
		Industry *industry = new Industry(); industry->location = TileArea(tile(20, 20), 1, 1); industry->town = *Town::Iterate().begin(); industry->type = 5;
		MakeIndustry(tile(20, 20), industry->index, 0, 0, WaterClass::Sea);
		BuildOilRig(tile(20, 20)); StationNeed(IsOilRig(tile(20, 20)), __LINE__);
		fmt::print(stderr, "STCMD oilrig {}\n", industry->neutral_station != nullptr);
		DeleteOilRig(tile(20, 20)); StationNeed(industry->neutral_station == nullptr, __LINE__); delete industry;
		ChangeTileOwner(tile(1, 1), company->index, company->index);
		fmt::print(stderr, "STCMD complete\n");
		if (args.size() == 3) SaveOrLoad(std::string(args[2]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false);
	}
	current.Restore(); return true;
}
extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv() { __real__Z22IConsoleStdLibRegisterv(); IConsole::CmdRegister("stations_scenario", StationCommands); }
"""


RELOADS = {
    "reload": ("commands", "inspect"),
    "road-stops-reload": ("road-stops", "road-stops-inspect"),
    "resolver-reload": ("resolver", "resolver-inspect"),
    "airport-reload": ("airport", "airport-inspect"),
    "waypoints-reload": ("waypoints", "waypoints-inspect"),
}
# Road stop owner (road_stop.rs) branches; cache checks run only with desync snapshots.
ROAD_STOPS = (
    "clear_head clear_last clear_split clear_tail enter_bay0 enter_bay1 enter_drive"
    " enter_refused leave_bay leave_drive make_alone make_join make_merge make_parent rebuild"
    " rebuild_vehicle"
).split()


def adapter(build, folder, env):
    from .console_adapter import console_adapter

    folder.mkdir(parents=True, exist_ok=True)
    source = folder / "station-commands.cpp"
    source.write_text(
        COMMAND_SOURCE.replace(
            "// OWNER399",
            (ROOT / "tools/simulation/station-resolver-console.hpp").read_text()
            + (ROOT / "tools/simulation/airport-resolver-console.hpp").read_text(),
        )
    )
    return console_adapter(
        build,
        folder,
        env,
        source,
        "stations",
        replace_objects=("/src/station_cmd.cpp.o",),
    )


def prepare_commands(scenario, binaries, builds, out, timeout, env, result):
    folder = out / scenario["name"] / "input"
    source = Path(scenario["save"])
    read_save(source)
    executables = prepared_console("stations", adapter, binaries, builds, folder, env)
    if binaries["candidate"] == binaries["reference"]:
        executables["candidate"] = executables["reference"]
    scenario = dict(scenario, save=str(source), executables=executables)
    # Reload modes capture the command fixture's save, then inspect it after load.
    capture, inspect = RELOADS.get(scenario["station_commands"], (None, None))
    if capture:
        saves = {role: folder / role / "stations.sav" for role in executables}
        runs = core.capture_pair(
            lambda role: dict(
                scenario,
                console=[f'stations_scenario {capture} "{saves[role]}"', "unpause"],
            ),
            executables,
            builds,
            folder,
            timeout,
            env,
        )
        logs = {}
        for role, run in runs.items():
            if (
                run["exit"]
                or not saves[role].is_file()
                or "STCMD complete" not in run["log"]
            ):
                raise RuntimeError("station reload capture failed")
            logs[role] = [line for line in run["log"] if line.startswith("STCMD ")]
        differences = core.compare_saves(
            saves["reference"],
            saves["candidate"],
            scenario["name"],
            10,
            result["stats"],
        )
        if differences or logs["reference"] != logs["candidate"]:
            raise RuntimeError("station reload input differs")
        return dict(
            scenario,
            role_inputs={role: {"save": str(save)} for role, save in saves.items()},
            console=[f"stations_scenario {inspect}", "unpause"],
        )
    return dict(
        scenario,
        console=["stations_scenario " + scenario["station_commands"], "unpause"],
    )


def check_commands(scenario, run, mode, role, result):
    if scenario["station_commands"] == "linkgraph":
        core.branch_witnesses(
            scenario,
            run,
            role,
            "linkgraph",
            [
                "merge",
                "remove_last",
                "remove_nonlast",
                "restrict",
                "release",
                "compress",
            ],
        )
    required = (
        ["rename"]
        if scenario["station_commands"] == "reload"
        else [
            "build_rail",
            "build_road",
            "build_airport",
            "remove_airport",
            "build_dock",
            "remove_dock",
            "build_oilrig",
            "delete_oilrig",
            "check_docking",
            "rename",
            "move_name",
            "change_owner",
            "clear",
            "terraform",
            "remove_road_tile",
            "remove_rail_area",
            "join",
            "reuse",
            "partial_rail",
            "build_above",
        ]
    )
    fidelity = {
        "removed-destination": (
            ["remove_road_tile"],
            ["STCMD removed-destination 0 0 false"],
        ),
        "map-edges": (
            ["build_rail", "build_road", "remove_road_tile"],
            ["STCMD map-edge 0 preserved true", "STCMD map-edge 1 preserved true"],
        ),
        "railtype-63": (["build_rail"], ["STCMD railtype-63 overbuilt"]),
    }
    if scenario["station_commands"] in fidelity:
        required, evidence = fidelity[scenario["station_commands"]]
        for line in evidence:
            if line not in run["log"]:
                raise RuntimeError("station fidelity witness missing: " + line)
    if scenario["station_commands"].startswith("road-stops"):
        reload = scenario["station_commands"] == "road-stops-reload"
        required = [] if reload else ["build_road", "remove_road_tile"]
        branches = ["after_load", "rebuild_vehicle"] if reload else ROAD_STOPS
        cached = ["integrity"] if mode == "snapshots" else []
        core.branch_witnesses(scenario, run, role, "road_stop", branches + cached)
        core.branch_witnesses(
            scenario,
            run,
            role,
            "movement",
            [] if reload else ["road_stop_admit", "road_stop_refuse"],
        )
        core.branch_witnesses(
            scenario,
            run,
            role,
            "occupancy",
            ["insert", "reset"] + ([] if reload else ["same_bucket"]),
        )
    if scenario["station_commands"].startswith("waypoints"):
        required = []
        branches = (
            ["rename"]
            if scenario["station_commands"].endswith("reload")
            else [
                "build_rail",
                "build_road",
                "axis_rail",
                "axis_road",
                "reuse",
                "tie",
                "rename",
                "move_name",
                "validate",
            ]
        )
        core.branch_witnesses(scenario, run, role, "waypoint_command", branches)
    if scenario["station_commands"].startswith("airport"):
        required = (
            [] if scenario["station_commands"].endswith("reload") else ["build_airport"]
        )
        markers = [
            "airport-built",
            "airport-animation",
            "airport-variable airport",
            "airport-variable tile",
            "airport-variable preview",
            "airport-variable nowhere",
            "airport-date",
            "airport-storage-zero",
            "airport-roots live-default purchase",
            "airport-parameter",
            "airport-relative-wrap",
            "airport-town",
            "airport-random",
            "airport-id",
            "airport-offset",
            "airport-selection",
            "airport-state",
        ]
        if not scenario["station_commands"].endswith("reload"):
            markers += [
                "cost airport-admission",
                "airport-bounds",
                "cost airport-build-test",
                "cost airport-build",
            ]
        for marker in markers:
            if not any(line.startswith("STCMD " + marker) for line in run["log"]):
                raise RuntimeError("airport runtime witness missing: " + marker)
    if scenario["station_commands"].startswith("resolver"):
        required = (
            [] if scenario["station_commands"].endswith("reload") else ["build_rail"]
        )
        for marker in [
            "resolver-cache " + v for v in ("40", "41", "45", "46", "47", "49")
        ] + [
            "resolver-rng lower",
            "resolver-rng cases",
            "resolver-rng empty",
            "resolver-rng combined",
            "resolver-rng all",
            "resolver-rng waypoint",
            "cost resolver-slope",
            "resolver-reconstruction",
            "resolver-variable station",
            "resolver-variable waypoint",
            "resolver-variable purchase",
            "resolver-variable slope-x",
            "resolver-variable slope-y",
            "resolver-parent",
            "resolver-scope-fallback",
            "resolver-root",
            "resolver-root-null",
            "resolver-state",
            "resolver-real-empty",
            "cost resolver-slope-policy",
            "resolver-variable terrain-live",
            "resolver-variable terrain-null",
        ]:
            if not any(line.startswith("STCMD " + marker) for line in run["log"]):
                raise RuntimeError("station resolver witness missing: " + marker)
    if scenario["station_commands"].startswith(("road-stops", "waypoints")):
        waypoint = scenario["station_commands"].startswith("waypoints")
        kind = str(waypoint).lower()
        reload = scenario["station_commands"].endswith("reload")
        markers = (
            [f"STCMD resolver-reload {kind} missing reattached cache"]
            if reload
            else [
                f"STCMD resolver-roots {kind} ordered purchase default",
                f"STCMD resolver-gating {kind} cached cargo all-empty",
                f"STCMD resolver-lifecycle {kind} test execute reuse full partial clear shrink",
            ]
        )
        for marker in markers:
            if marker not in run["log"]:
                raise RuntimeError("road-stop owner evidence missing: " + marker)
        if not reload and sum(
            line.startswith("STCMD resolver-mask ") for line in run["log"]
        ) != (9 if waypoint else 15):
            raise RuntimeError("road-stop owner reseed matrix incomplete")
    if scenario["station_commands"] != "linkgraph":
        core.branch_witnesses(scenario, run, role, "station_command", required)
    lines = [line for line in run["log"] if line.startswith("STCMD ")]
    expected = (
        "STCMD reloaded"
        if scenario["station_commands"] == "reload"
        else "STCMD complete"
    )
    if not any(line.startswith(expected) for line in lines):
        raise RuntimeError("station command scenario did not complete")
