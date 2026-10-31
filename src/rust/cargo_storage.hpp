/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargo_storage.hpp Pool services and native cargo ownership facade. */
static uint8_t CargoCanAllocate() noexcept { return CargoPacket::CanAllocateItem(); }
static void *CargoCreate(const OpenTTDCargoPacketFields *fields) noexcept { return new CargoPacket(*fields); }
static OpenTTDCargoPacket *CargoOwner(void *shell) noexcept { return static_cast<CargoPacket *>(shell)->RustOwner(); }
static void CargoDestroy(void *shell) noexcept { delete static_cast<CargoPacket *>(shell); }
static uint32_t CargoRandom(uint32_t limit) noexcept { return RandomRange(limit); }
static uint32_t CargoCoordinate(uint32_t tile, uint8_t axis) noexcept { return axis == 0 ? TileX(TileIndex(tile)) : TileY(TileIndex(tile)); }
static uint16_t CargoFlow(const void *context, uint8_t mode, uint16_t origin, [[maybe_unused]] uint16_t avoid, [[maybe_unused]] uint16_t avoid2, const uint16_t *, size_t, uint8_t *restricted) noexcept
{
	const GoodsEntry *ge = static_cast<const GoodsEntry *>(context);
	if (mode == 4) return ge->HasData() && !ge->GetData().flows.empty();
	if (!ge->HasData()) return StationID::Invalid().base();
	const auto &flows = ge->GetData().flows;
	if (mode == 3) return flows.begin()->first.base();
	auto it = flows.find(StationID(origin));
	if (it == flows.end()) return StationID::Invalid().base();
	if (mode == 0) {
		bool value = false;
		StationID via = it->second.GetViaWithRestricted(value);
		*restricted = value;
		return via.base();
	}
	if (mode == 5) return it->second.GetVia().base();
	NOT_REACHED();
}
static int64_t CargoPay(void *context, uint8_t final, uint8_t cargo, const void *shell, uint32_t amount, uint32_t tile) noexcept
{
	CargoPayment *payment = static_cast<CargoPayment *>(context);
	const CargoPacket *cp = static_cast<const CargoPacket *>(shell);
	if (final != 0) {
		payment->PayFinalDelivery(cargo, cp, amount, TileIndex(tile));
		return 0;
	}
	return payment->PayTransfer(cargo, cp, amount, TileIndex(tile)).base();
}
static void CargoOrigin(void *context, uint16_t origin, uint32_t amount, uint8_t remove) noexcept
{
	auto &map = *static_cast<StationCargoAmountMap *>(context);
	if (remove != 0) map[StationID(origin)] -= amount;
	else map[StationID(origin)] += amount;
}
static OpenTTDCargoFlowMap *CargoFlowMap(const void *context) noexcept
{
	const GoodsEntry *ge = static_cast<const GoodsEntry *>(context);
	return ge->HasData() ? ge->GetData().flows.RustState() : nullptr;
}
static uint32_t CargoRandomDraw() noexcept { return Random(); }
static void *CargoNextPacket(void *shell) noexcept
{
	auto range = CargoPacket::Iterate(shell == nullptr ? 0 : static_cast<CargoPacket *>(shell)->index.base() + 1);
	auto it = range.begin();
	return it == range.end() ? nullptr : *it;
}
static const OpenTTDCargoStorageServices _cargo_storage_services = {
	CargoCanAllocate, CargoCreate, CargoOwner, CargoDestroy, CargoRandom, CargoCoordinate, CargoFlow, CargoPay, CargoOrigin, CargoFlowMap, CargoRandomDraw, CargoNextPacket,
};

CargoPacket::CargoPacket() : CargoPacket(OpenTTDCargoPacketFields{}) {}
CargoPacket::CargoPacket(const OpenTTDCargoPacketFields &fields) : state(openttd_rust_cargo_packet_new(&fields)) {}
CargoPacket::CargoPacket(StationID first, uint16_t count, Source source) : CargoPacket()
{
	auto f = this->Export();
	f.first_station = first.base(); f.count = count; f.source_id = source.id; f.source_type = to_underlying(source.type);
	this->Import(f);
}
CargoPacket::CargoPacket(uint16_t count, uint16_t periods, StationID first, TileIndex tile, Money feeder) : CargoPacket()
{
	auto f = this->Export();
	f.count = count; f.periods_in_transit = periods; f.first_station = first.base(); f.source_xy = tile.base(); f.feeder_share = feeder.base();
	this->Import(f);
}
CargoPacket::CargoPacket(uint16_t count, Money feeder, CargoPacket &original) : CargoPacket(original.Export())
{
	auto f = this->Export(); f.count = count; f.feeder_share = feeder.base(); this->Import(f);
}
CargoPacket::~CargoPacket() { openttd_rust_cargo_packet_destroy(this->state); }
OpenTTDCargoPacketFields CargoPacket::Export() const { OpenTTDCargoPacketFields f; openttd_rust_cargo_packet_export(this->state, &f); return f; }
void CargoPacket::Import(const OpenTTDCargoPacketFields &fields) { openttd_rust_cargo_packet_import(this->state, &fields); }
CargoPacket *CargoPacket::Split(uint amount) { return static_cast<CargoPacket *>(openttd_rust_cargo_packet_split(this->state, &_cargo_storage_services, amount)); }
void CargoPacket::Merge(CargoPacket *other) { openttd_rust_cargo_packet_merge(this->state, &_cargo_storage_services, other); }
void CargoPacket::Reduce(uint amount) { openttd_rust_cargo_packet_reduce(this->state, amount); }
void CargoPacket::UpdateLoadingTile(TileIndex tile) { openttd_rust_cargo_packet_tile(this->state, &_cargo_storage_services, tile.base(), 1); }
void CargoPacket::UpdateUnloadingTile(TileIndex tile) { openttd_rust_cargo_packet_tile(this->state, &_cargo_storage_services, tile.base(), 0); }
uint CargoPacket::GetDistance(TileIndex tile) const { return openttd_rust_cargo_packet_distance(this->state, &_cargo_storage_services, tile.base()); }
void CargoPacket::InvalidateAllFrom(Source source) { openttd_rust_cargo_invalidate(&_cargo_storage_services, source.id, to_underlying(source.type)); }
void CargoPacket::InvalidateAllFrom(StationID station) { openttd_rust_cargo_invalidate(&_cargo_storage_services, station.base(), UINT8_MAX); }

template <class Tinst, class Tcont>
CargoList<Tinst, Tcont>::CargoList() : state(openttd_rust_cargo_list_new(std::is_same_v<Tinst, VehicleCargoList>)) {}
template <class Tinst, class Tcont>
CargoList<Tinst, Tcont>::~CargoList() { openttd_rust_cargo_list_destroy(this->state, &_cargo_storage_services); }
template <class Tinst, class Tcont>
void CargoList<Tinst, Tcont>::OnCleanPool() { openttd_rust_cargo_list_clear(this->state); }
template <class Tinst, class Tcont>
OpenTTDCargoListFields CargoList<Tinst, Tcont>::Export() const { OpenTTDCargoListFields f; openttd_rust_cargo_list_export(this->state, &f); return f; }
template <class Tinst, class Tcont>
void CargoList<Tinst, Tcont>::ImportMeta(const OpenTTDCargoListFields &fields) { openttd_rust_cargo_list_import(this->state, &fields); }
template <class Tinst, class Tcont>
std::unique_ptr<const Tcont> CargoList<Tinst, Tcont>::Packets() const
{
	auto result = std::make_unique<Tcont>();
	openttd_rust_cargo_list_snapshot(this->state, result.get(), [](void *context, uint16_t key, void *shell) noexcept {
		if constexpr (std::is_same_v<Tinst, VehicleCargoList>) static_cast<Tcont *>(context)->push_back(static_cast<CargoPacket *>(shell));
		else {
			auto &packets = (*static_cast<Tcont *>(context))[StationID(key)];
			if (shell != nullptr) packets.push_back(static_cast<CargoPacket *>(shell));
		}
	});
	return result;
}
template <class Tinst, class Tcont>
void CargoList<Tinst, Tcont>::ImportPackets(const Tcont &packets)
{
	openttd_rust_cargo_list_clear(this->state);
	if constexpr (std::is_same_v<Tinst, VehicleCargoList>) {
		for (CargoPacket *cp : packets) openttd_rust_cargo_list_insert(this->state, 0, cp);
	} else {
		for (auto &pair : packets) {
			openttd_rust_cargo_list_insert(this->state, pair.first.base(), nullptr);
			for (CargoPacket *cp : pair.second) openttd_rust_cargo_list_insert(this->state, pair.first.base(), cp);
		}
	}
}
template <class Tinst, class Tcont>
void CargoList<Tinst, Tcont>::InvalidateCache() { openttd_rust_cargo_list_rebuild(this->state, &_cargo_storage_services); }

/** StationID has one uint16_t member at offset zero, with no padding or stride
 * change. The native span stays live/stable until Rust returns; Rust reads raw
 * scalars and retains no reference across callbacks that may reenter cargo. */
static const uint16_t *CargoNext(std::span<const StationID> next)
{
	static_assert(std::is_standard_layout_v<StationID>);
	static_assert(std::is_same_v<StationID::BaseType, uint16_t>);
	static_assert(sizeof(StationID) == sizeof(uint16_t));
	static_assert(alignof(StationID) == alignof(uint16_t));
	return reinterpret_cast<const uint16_t *>(next.data());
}
StationID VehicleCargoList::GetFirstStation() const { return StationID(openttd_rust_cargo_list_first(this->state, &_cargo_storage_services)); }
StationID StationCargoList::GetFirstStation() const { return StationID(openttd_rust_cargo_list_first(this->state, &_cargo_storage_services)); }
void VehicleCargoList::Append(CargoPacket *cp, MoveToAction action) { openttd_rust_cargo_list_append(this->state, &_cargo_storage_services, cp, action); }
void StationCargoList::Append(CargoPacket *cp, StationID next) { openttd_rust_cargo_list_append(this->state, &_cargo_storage_services, cp, next.base()); }
void VehicleCargoList::AgeCargo() { openttd_rust_cargo_list_age(this->state, &_cargo_storage_services); }
void VehicleCargoList::KeepAll() { openttd_rust_cargo_list_keep(this->state); }
bool StationCargoList::HasCargoFor(std::span<const StationID> next) const
{
	return openttd_rust_cargo_list_has(this->state, CargoNext(next), next.size()) != 0;
}
bool VehicleCargoList::Stage(bool accepted, StationID station, std::span<const StationID> next, OrderUnloadType unload, const GoodsEntry *ge, CargoType cargo, CargoPayment *payment, TileIndex tile)
{
	return openttd_rust_cargo_list_stage(this->state, &_cargo_storage_services, accepted, station.base(), CargoNext(next), next.size(), to_underlying(unload), ge, cargo, payment, tile.base()) != 0;
}
template <VehicleCargoList::MoveToAction Tfrom, VehicleCargoList::MoveToAction Tto>
uint VehicleCargoList::Reassign(uint amount) { return openttd_rust_cargo_list_reassign(this->state, &_cargo_storage_services, Tfrom, Tto, amount); }
static uint CargoMove(OpenTTDCargoList *src, OpenTTDCargoList *dest, uint8_t mode, uint amount, StationID avoid = StationID::Invalid(), StationID avoid2 = StationID::Invalid(), std::span<const StationID> next = {}, const GoodsEntry *ge = nullptr, CargoType cargo = 0, void *payment = nullptr, TileIndex tile = INVALID_TILE)
{
	return openttd_rust_cargo_list_move(src, dest, &_cargo_storage_services, mode, amount, avoid.base(), avoid2.base(), CargoNext(next), next.size(), ge, cargo, payment, tile.base());
}
uint VehicleCargoList::Return(uint amount, StationCargoList *dest, StationID next, TileIndex tile) { return CargoMove(this->state, dest->RustOwner(), 0, amount, next, StationID::Invalid(), {}, nullptr, 0, nullptr, tile); }
uint VehicleCargoList::Unload(uint amount, StationCargoList *dest, CargoType cargo, CargoPayment *payment, TileIndex tile) { return CargoMove(this->state, dest->RustOwner(), 10, amount, StationID::Invalid(), StationID::Invalid(), {}, nullptr, cargo, payment, tile); }
uint VehicleCargoList::Shift(uint amount, VehicleCargoList *dest) { return CargoMove(this->state, dest->RustOwner(), 3, amount); }
uint VehicleCargoList::Truncate(uint amount) { return CargoMove(this->state, nullptr, 4, amount); }
uint VehicleCargoList::Reroute(uint amount, VehicleCargoList *dest, StationID avoid, StationID avoid2, const GoodsEntry *ge) { return CargoMove(this->state, dest->RustOwner(), 5, amount, avoid, avoid2, {}, ge); }
uint StationCargoList::Reserve(uint amount, VehicleCargoList *dest, std::span<const StationID> next, TileIndex tile) { return CargoMove(this->state, dest->RustOwner(), 6, amount, StationID::Invalid(), StationID::Invalid(), next, nullptr, 0, nullptr, tile); }
uint StationCargoList::Load(uint amount, VehicleCargoList *dest, std::span<const StationID> next, TileIndex tile) { return CargoMove(this->state, dest->RustOwner(), 7, amount, StationID::Invalid(), StationID::Invalid(), next, nullptr, 0, nullptr, tile); }
uint StationCargoList::Reroute(uint amount, StationCargoList *dest, StationID avoid, StationID avoid2, const GoodsEntry *ge) { return CargoMove(this->state, dest->RustOwner(), 8, amount, avoid, avoid2, {}, ge); }
uint StationCargoList::Truncate(uint amount, StationCargoAmountMap *origins) { return CargoMove(this->state, nullptr, 9, amount, StationID::Invalid(), StationID::Invalid(), {}, nullptr, 0, origins); }

template class CargoList<VehicleCargoList, CargoPacketList>;
template class CargoList<StationCargoList, StationCargoPacketMap>;
template uint VehicleCargoList::Reassign<VehicleCargoList::MTA_DELIVER, VehicleCargoList::MTA_KEEP>(uint);
template uint VehicleCargoList::Reassign<VehicleCargoList::MTA_DELIVER, VehicleCargoList::MTA_TRANSFER>(uint);
template uint VehicleCargoList::Reassign<VehicleCargoList::MTA_LOAD, VehicleCargoList::MTA_KEEP>(uint);

const OpenTTDCargoStorageServices *CargoStorageServices() { return &_cargo_storage_services; }
