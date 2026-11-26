/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargo_storage.hpp Pool services and native cargo ownership facade. */
#include "services_ffi.h"
static uint8_t CargoCanAllocate() noexcept { return CargoPacket::CanAllocateItem(); }
static CargoPacket *CargoCreate(const OpenTTDCargoPacket *fields) noexcept { return new CargoPacket(*fields); }
static OpenTTDCargoPacket *CargoOwner(CargoPacket *shell) noexcept { return shell->RustOwner(); }
static void CargoDestroy(CargoPacket *shell) noexcept { delete shell; }
static OpenTTDCargoFlowMap *CargoFlowMap(const GoodsEntry *ge) noexcept
{
	return ge->HasData() ? ge->GetData().flows.RustState() : nullptr;
}
static CargoPacket *CargoNextPacket(CargoPacket *shell) noexcept
{
	auto range = CargoPacket::Iterate(shell == nullptr ? 0 : shell->index.base() + 1);
	auto it = range.begin();
	return it == range.end() ? nullptr : *it;
}
static const OpenTTDCargoStorageServices _cargo_storage_services = {
	.can_allocate = CargoCanAllocate,
	.create = CargoCreate,
	.destroy = CargoDestroy,
	.flow_map = CargoFlowMap,
	.packet = CargoOwner,
	.packet_next = CargoNextPacket,
	.payment = &GetRustCargoServices(),
	.payment_state = [](CargoPayment *payment) noexcept { return payment->rust_state; },
	.shared = &_rust_shared_services,
};

CargoPacket::CargoPacket() : state(openttd_rust_cargo_packet_new(nullptr)) {}
CargoPacket::CargoPacket(const OpenTTDCargoPacket &fields) : state(openttd_rust_cargo_packet_new(&fields)) {}
CargoPacket::CargoPacket(StationID first, uint16_t count, Source source) : CargoPacket()
{
	auto &f = *this->state;
	f.first_station = first.base(); f.count = count; f.source_id = source.id; f.source_type = to_underlying(source.type);
	assert(count != 0);
}
CargoPacket::CargoPacket(uint16_t count, uint16_t periods, StationID first, TileIndex tile, Money feeder) : CargoPacket()
{
	auto &f = *this->state;
	f.count = count; f.periods_in_transit = periods; f.first_station = first.base(); f.source_xy = tile.base(); f.feeder_share = feeder.base();
	assert(count != 0);
}
CargoPacket::~CargoPacket() { openttd_rust_cargo_packet_destroy(this->state); }
void CargoPacket::UpdateLoadingTile(TileIndex tile) { openttd_rust_cargo_packet_update_loading_tile(this->state, &_cargo_storage_services, tile.base()); }
void CargoPacket::InvalidateAllFrom(Source source) { openttd_rust_cargo_invalidate(&_cargo_storage_services, source.id, to_underlying(source.type)); }

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
	openttd_rust_cargo_list_snapshot(this->state, result.get(), [](void *context, uint16_t key, CargoPacket *shell) noexcept {
		if constexpr (std::is_same_v<Tinst, VehicleCargoList>) static_cast<Tcont *>(context)->push_back(shell);
		else {
			auto &packets = (*static_cast<Tcont *>(context))[StationID(key)];
			if (shell != nullptr) packets.push_back(shell);
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

StationID VehicleCargoList::GetFirstStation() const { return StationID(openttd_rust_cargo_list_first(this->state, &_cargo_storage_services)); }
StationID StationCargoList::GetFirstStation() const { return StationID(openttd_rust_cargo_list_first(this->state, &_cargo_storage_services)); }
void VehicleCargoList::Append(CargoPacket *cp, MoveToAction action) { openttd_rust_vehicle_cargo_list_append(this->state, &_cargo_storage_services, cp, action); }
void StationCargoList::Append(CargoPacket *cp, StationID next) { openttd_rust_station_cargo_list_append(this->state, &_cargo_storage_services, cp, next.base()); }
void VehicleCargoList::KeepAll() { openttd_rust_cargo_list_keep(this->state); }
uint VehicleCargoList::Truncate(uint amount) { return openttd_rust_vehicle_cargo_list_truncate(this->state, &_cargo_storage_services, amount); }
uint StationCargoList::Truncate(uint amount) { return openttd_rust_station_cargo_list_truncate(this->state, &_cargo_storage_services, amount); }

template class CargoList<VehicleCargoList, CargoPacketList>;
template class CargoList<StationCargoList, StationCargoPacketMap>;

const OpenTTDCargoStorageServices *CargoStorageServices() { return &_cargo_storage_services; }
