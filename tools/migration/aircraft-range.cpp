/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
/** @file aircraft-range.cpp Finite NewGRF range gap against unchanged event bodies. */
#include "rust/aircraft_ffi.h"
#include "rust/movement_ffi.h"
#include "rust/vehicle_maintenance_ffi.h"
#include <algorithm>
#include <array>
#include <cstdio>
#include <cstdlib>
#include <memory>

static constexpr uint32_t INVALID_TILE = UINT32_MAX;
static constexpr uint8_t ENDTAKEOFF = 12, HELIENDLANDING = 18;
static constexpr uint8_t VAF_DEST_TOO_FAR = 0;
static constexpr int OT_GOTO_STATION = 1, OT_GOTO_DEPOT = 2, OT_LOADING = 3, OT_LEAVESTATION = 4;
static constexpr int WC_VEHICLE_VIEW = 0, WID_VV_START_STOP = 0, STR_NEWS_AIRCRAFT_DEST_TOO_FAR = 0;
static constexpr uint8_t _local_company = 0;
enum class VehState { Stopped = 1, Crashed = 7 };
enum class AdviceType { AircraftDestinationTooFar };
struct Status { uint8_t value = 0; bool Test(VehState bit) const { return (this->value & (1 << static_cast<int>(bit))) != 0; } };
struct Destination { uint16_t value; uint16_t ToStationID() const { return this->value; } };
struct Order { int type = OT_GOTO_STATION; uint16_t destination = 1; bool IsType(int type) const { return this->type == type; } Destination GetDestination() const { return {this->destination}; } };
struct Station { struct { uint32_t tile; } airport; static Station *GetIfValid(uint16_t id); };
struct Aircraft {
	Status vehstatus; Order current_order; struct { uint32_t cached_max_range_sqr; } acache;
	uint16_t targetairport = 0; uint8_t state = 1, flags = 0, owner = 0; uint32_t index = 17;
	void HandleBreakdown() {} void HandleLoading(bool) {}
};
static std::array<Station, 2> stations;
static std::array<bool, 2> valid;
static std::array<unsigned, 4> messages{};
Station *Station::GetIfValid(uint16_t id) { return id < 2 && valid[id] ? &stations[id] : nullptr; }
static uint32_t DistanceSquare(uint32_t a, uint32_t b) { int dx = (a & 63) - (b & 63), dy = (a >> 6) - (b >> 6); return dx * dx + dy * dy; }
static bool HasBit(uint8_t value, uint8_t bit) { return (value & (1 << bit)) != 0; }
static void SetBit(uint8_t &value, uint8_t bit) { value |= 1 << bit; }
static void ClrBit(uint8_t &value, uint8_t bit) { value &= ~(1 << bit); }
static void SetWindowWidgetDirty(int, uint32_t, int) { messages[0]++; }
struct ScriptEventAircraftDestTooFar { explicit ScriptEventAircraftDestTooFar(uint32_t) {} };
struct AI { static void NewEvent(uint8_t, ScriptEventAircraftDestTooFar *event) { delete event; messages[1]++; } };
static int GetEncodedString(int, uint32_t) { return 0; }
static void AddVehicleAdviceNewsItem(AdviceType, int, uint32_t) { messages[2]++; }
static void DeleteVehicleNews(uint32_t, AdviceType) { messages[3]++; }
static bool HandleCrashedAircraft(Aircraft *) { return true; }
static void HandleAircraftSmoke(Aircraft *, bool) {}
static void ProcessOrders(Aircraft *) {}
static void AirportGoToNextPosition(Aircraft *) {}
/* Generated solely from the pinned, unchanged src/aircraft_cmd.cpp. */
#include "aircraft-range-reference.inc"

struct Fixture {
	Aircraft aircraft;
	std::unique_ptr<OpenTTDAircraftState, decltype(&openttd_rust_aircraft_state_destroy)> state{openttd_rust_aircraft_state_new(), openttd_rust_aircraft_state_destroy};
	int32_t x = 0, y = 0, z = 1;
	uint16_t speed = 0;
	uint8_t subtype = 2;
	Direction direction{};
	uint64_t blocks = 0;
	std::array<unsigned, 4> messages{};
	OpenTTDVehicleTileLinks links[2];
};
#include "aircraft-boundary.hpp"
#include "controller-orders.hpp"
static Fixture *fixture;
static OpenTTDVehicleServices vehicle;
static OpenTTDVehicleMaintenanceServices maintenance;
static OpenTTDSharedServices shared;
static OpenTTDMovementServices movement;
static std::unique_ptr<ControllerOrders> range_orders;
static OpenTTDAircraftLeaves RangeLeaves()
{
	auto leaves = AircraftTestLeaves(vehicle, shared);
	shared.map_log_x = []() noexcept { return uint32_t{6}; };
	leaves.station = [](uint16_t id) noexcept -> const OpenTTDStation * { return reinterpret_cast<const OpenTTDStation *>(Station::GetIfValid(id)); };
	leaves.airport_tile = [](const OpenTTDStation *s) noexcept { return reinterpret_cast<const Station *>(s)->airport.tile; };
	vehicle.status = [](OpenTTDVehicle *) noexcept { return static_cast<uint8_t>(fixture->aircraft.vehstatus.value); };
	vehicle.order_type = [](OpenTTDVehicle *) noexcept { return static_cast<uint8_t>(fixture->aircraft.current_order.type); };
	vehicle.order_destination = [](OpenTTDVehicle *) noexcept { return fixture->aircraft.current_order.destination; };
	vehicle.maintenance = &maintenance;
	maintenance.breakdown_probe = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleMaintenanceBreakdownProbe{.counter = 0, .status = 0}; };
	range_orders = std::make_unique<ControllerOrders>(&vehicle);
	/* Matching current/list station orders take the ordinary unchanged-order
	 * path. This preserves the range reference's deliberate no-op orders. */
	range_orders->vehicle->current = openttd_rust_order_vector_data(range_orders->vector.get())[0];
	range_orders->services.vehicle_type = [](void *) noexcept { return uint64_t{3}; };
	range_orders->services.vehicle = [](void *v) noexcept { if (v != fixture) std::abort(); return range_orders->vehicle.get(); };
	range_orders->services.consist = [](void *) noexcept { return &range_orders->consist; };
	vehicle.orders = []() noexcept -> const OpenTTDOrdersLeaves * { return &range_orders->services; };
	vehicle.loading = [](OpenTTDVehicle *, bool, const OpenTTDVehicleVisits *) noexcept {};
	vehicle.visits = []() noexcept -> const OpenTTDVehicleVisits * { return nullptr; };
	vehicle.start_stop_dirty = [](OpenTTDVehicle *) noexcept { fixture->messages[0]++; };
	leaves.destination_too_far = [](OpenTTDAircraftVehicle) noexcept { fixture->messages[1]++; fixture->messages[2]++; };
	leaves.delete_range_news = [](OpenTTDAircraftVehicle) noexcept { fixture->messages[3]++; };
	leaves.airport_fta = [](const OpenTTDStation *) noexcept -> const void * { return &fixture->blocks; };
	leaves.station_tile = [](const OpenTTDStation *s) noexcept { return reinterpret_cast<const Station *>(s)->airport.tile; };
	leaves.rotation = [](const OpenTTDStation *) noexcept { return uint8_t{0}; };
	leaves.geometry = [](const OpenTTDStation *s) noexcept { auto tile = reinterpret_cast<const Station *>(s)->airport.tile; return OpenTTDAircraftGeometry{tile, 0, 7, 7, true}; };
	vehicle.position_observations = [](OpenTTDVehicle *) noexcept { return OpenTTDVehiclePositionObservations{.subtype = &fixture->subtype, .z = &fixture->z}; };
	vehicle.movement_observations = [](OpenTTDVehicle *) noexcept {
		return OpenTTDVehicleMovementObservations{.position = {.subtype = &fixture->subtype, .z = &fixture->z},
			.x = &fixture->x, .y = &fixture->y, .speed = &fixture->speed, .direction = &fixture->direction};
	};
	vehicle.max_speed = [](OpenTTDVehicle *) noexcept { return uint16_t{100}; };
	vehicle.x = vehicle.y = [](OpenTTDVehicle *) noexcept { return int32_t{0}; };
	vehicle.direction = [](OpenTTDVehicle *) noexcept { return uint8_t{0}; };
	leaves.dummy_airport = []() noexcept -> const void * { return &fixture->blocks; };
	leaves.airport_elements = [](const void *) noexcept { return uint8_t{1}; };
	leaves.moving = [](const void *, uint8_t) noexcept { return OpenTTDAircraftMoving{0, 0, 16, 0}; };
	vehicle.write_speed = [](OpenTTDVehicle *, uint16_t) noexcept {};
	leaves.node = [](const void *ap, uint8_t) noexcept -> const void * { return ap; };
	leaves.fta = [](const void *) noexcept { return OpenTTDAircraftNode{nullptr, uint64_t{1} << 30, 0, 0, 0}; };
	leaves.airport_blocks = [](const OpenTTDStation *) noexcept { return &fixture->blocks; };
	vehicle.write_subspeed = [](OpenTTDVehicle *, uint8_t) noexcept {};
	vehicle.newgrf = []() noexcept -> const OpenTTDVehicleResolverServices * { return nullptr; };
	vehicle.property = [](const OpenTTDVehicleResolverServices *, OpenTTDVehicle *, uint8_t property, int32_t fallback) noexcept { if ((property != 0x0C && property != 0x1C) || fallback != 0) std::abort(); return int32_t{0}; };
	leaves.engine_speed = [](OpenTTDAircraftVehicle) noexcept { return uint16_t{100}; };
	vehicle.write_max_speed = [](OpenTTDVehicle *, uint16_t) noexcept {};
	vehicle.write_cargo_age = [](OpenTTDVehicle *, uint16_t) noexcept {};
	leaves.cargo_age_default = [](OpenTTDAircraftVehicle) noexcept { return uint16_t{0}; };
	leaves.next = [](OpenTTDAircraftVehicle v) noexcept { return v; };
	shared.map_max_x = shared.map_max_y = []() noexcept { return uint32_t{63}; };
	shared.plane_speed = []() noexcept { return uint8_t{1}; };
	vehicle.z = [](OpenTTDVehicle *) noexcept { return int32_t{1}; };
	vehicle.subtype = [](OpenTTDVehicle * v) noexcept { return uint8_t(v == reinterpret_cast<OpenTTDVehicle *>(fixture) ? 2 : 4); };
	vehicle.vehicle_type = [](OpenTTDVehicle *) noexcept { return uint8_t{3}; };
	vehicle.altitude_inputs = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleAltitudeInputs{100, 0}; };
	movement.shared = &shared;
	shared.movement_services = []() noexcept -> const OpenTTDMovementServices * { return &movement; };
	shared.tile_type = [](uint32_t) noexcept { return uint8_t{0}; }; // Flat clear ground.
	movement.pixel_location = [](int32_t x, int32_t y) noexcept -> OpenTTDPixelLocation { return {.tile = ((uint32_t(y) >> 4) << 6) + (uint32_t(x) >> 4), .kind = 0}; };
	movement.pixel_slope = [](uint32_t) noexcept { return OpenTTDFoundationSlope{0, Slope{0}}; };
	shared.tile_height = [](uint32_t) noexcept { return uint32_t{0}; };
	leaves.airport_entry = [](const void *, uint8_t) noexcept { return uint8_t{0}; };
	vehicle.write_x = vehicle.write_y = vehicle.write_z = [](OpenTTDVehicle *, int32_t) noexcept {};
	leaves.viewport = [](OpenTTDAircraftVehicle) noexcept {};
	vehicle.base_viewport = [](OpenTTDVehicle *) noexcept {};
	vehicle.tile_links = [](OpenTTDVehicle *v) noexcept { return &fixture->links[v == reinterpret_cast<OpenTTDVehicle *>(fixture) ? 0 : 1]; };
	vehicle.tile = [](OpenTTDVehicle *) noexcept { return uint32_t{0}; };
	vehicle.occupancy_update = [](OpenTTDVehicle *shell) noexcept -> OpenTTDOccupancyUpdateObservation { return {.links = vehicle.tile_links(shell), .tile = vehicle.tile(shell)}; };
	vehicle.map_log_x = []() noexcept { return uint32_t{6}; };
	leaves.copy_sprite = [](OpenTTDVehicle *, OpenTTDVehicle *) noexcept {};
	vehicle.next = [](OpenTTDVehicle *v) noexcept -> OpenTTDVehicle * { return v == reinterpret_cast<OpenTTDVehicle *>(fixture) ? reinterpret_cast<OpenTTDVehicle *>(uintptr_t{18}) : nullptr; };
	leaves.next = [](OpenTTDAircraftVehicle v) noexcept { if (v.id == 17) { v.id = 18; v.handle = reinterpret_cast<OpenTTDVehicle *>(uintptr_t{18}); return v; } return OpenTTDAircraftVehicle{nullptr, nullptr, 1048575}; };
	vehicle.speed = [](OpenTTDVehicle *) noexcept { return uint16_t{0}; };
	leaves.can_use_station = [](OpenTTDAircraftVehicle, const OpenTTDStation *) noexcept { return true; };
	leaves.station_owner = [](const OpenTTDStation *) noexcept { return uint8_t{0}; };
	vehicle.owner = [](OpenTTDVehicle *) noexcept { return uint8_t{0}; };
	leaves.helipads = [](const void *) noexcept { return uint8_t{0}; };
	leaves.airport_flags = [](const void *) noexcept { return uint8_t{1}; };
	vehicle.waiting_for_unbunching = [](OpenTTDVehicle *) noexcept { return false; };
	leaves.block_node = [](const void *) noexcept { return OpenTTDAircraftBlockNode{uint64_t{1} << 30, 0, 0}; };
	leaves.route_node = [](const void *) noexcept { return OpenTTDAircraftRouteNode{nullptr, 0, 0}; };
	leaves.block_choice = [](const void *) noexcept { return OpenTTDAircraftBlockChoice{nullptr, uint64_t{1} << 30, 0}; };
	vehicle.xy = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleXY{0, 0}; };
	vehicle.coordinates = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleCoordinates{0, 0, 1}; };
	vehicle.speed_inputs = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleSpeedInputs{0, 100, 8, 0, 0}; };
	vehicle.direction_progress = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleDirectionProgress{0, 0}; };
	vehicle.write_coordinates = [](OpenTTDVehicle *, int32_t, int32_t, int32_t) noexcept {};
	leaves.layout_blocks = [](const void *, uint8_t) noexcept { return uint64_t{1} << 30; };
	leaves.layout_heading = leaves.layout_next_position = [](const void *, uint8_t) noexcept { return uint8_t{0}; };
	leaves.layout_next = [](const void *, uint8_t) noexcept -> const void * { return nullptr; };
	leaves.layout_route = [](const void *ap, uint8_t) noexcept { return OpenTTDAircraftLayoutRoute{ap, nullptr, 0, 0}; };
	return leaves;
}
int main()
{
	unsigned cases = 0;
	for (uint32_t range : {0U, 1U, 16U, 64U, UINT32_MAX}) for (uint32_t destination : {0U, 1U, 4U, 8U})
	for (uint8_t state : {uint8_t{1}, uint8_t{10}, uint8_t{12}, uint8_t{14}, uint8_t{18}})
	for (uint8_t flag : {uint8_t{0}, uint8_t{1}}) for (unsigned validity = 0; validity < 4; validity++) {
		Fixture f; stations = {{{0}, {destination}}}; valid = {{(validity & 1) != 0, (validity & 2) != 0}};
		f.aircraft.acache.cached_max_range_sqr = range; f.aircraft.state = state; f.aircraft.flags = flag;
		f.state->cached_max_range_sqr = range; f.state->state = state; f.state->flags = flag; f.state->targetairport = 0;
		messages = {}; AircraftEventHandler(&f.aircraft, 0); auto expected = messages;
		fixture = &f;
		const auto leaves = RangeLeaves();
		openttd_rust_aircraft_event(&leaves, {reinterpret_cast<OpenTTDVehicle *>(&f), f.state.get(), 17}, false);
		if ((f.state->flags & 1) != (f.aircraft.flags & 1) || f.messages != expected) { std::fprintf(stderr, "range mismatch range=%u dest=%u state=%u flags=%u validity=%u\n", range, destination, state, flag, validity); return 1; }
		openttd_rust_occupancy_update(&vehicle, reinterpret_cast<OpenTTDVehicle *>(&f), true);
		openttd_rust_occupancy_update(&vehicle, reinterpret_cast<OpenTTDVehicle *>(uintptr_t{18}), true);
		cases++;
	}
	std::printf("finite range transition: %u unchanged-reference cases passed\n", cases);
}
