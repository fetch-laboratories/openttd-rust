/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargopacket_sl.cpp Code handling saving and loading of cargo packets */

#include "../stdafx.h"

#include "saveload.h"
#include "compat/cargopacket_sl_compat.h"

#include "../vehicle_base.h"
#include "../station_base.h"

#include "../safeguards.h"

/**
 * Savegame conversion for cargopackets.
 */
/* static */ void CargoPacket::AfterLoad()
{
	if (IsSavegameVersionBefore(SLV_44)) {
		for (const Vehicle *v : Vehicle::Iterate()) {
			auto packets = v->cargo.Packets();
			for (CargoPacket *cp : *packets) {
				auto f = cp->Export();
				StationID first(f.first_station);
				f.source_xy = (Station::IsValidID(first) ? Station::Get(first)->xy : v->tile).base();
				cp->Import(f);
			}
		}
		for (Station *st : Station::Iterate()) {
			for (GoodsEntry &ge : st->goods) {
				if (!ge.HasData()) continue;
				auto packets = ge.GetData().cargo.Packets();
				for (auto it = StationCargoList::ConstIterator(packets->begin()); it != packets->end(); ++it) {
					CargoPacket *cp = *it;
					auto f = cp->Export();
					StationID first(f.first_station);
					f.source_xy = (Station::IsValidID(first) ? Station::Get(first)->xy : st->xy).base();
					cp->Import(f);
				}
			}
		}
	}
	if (IsSavegameVersionBefore(SLV_120)) {
		for (CargoPacket *cp : CargoPacket::Iterate()) {
			if (!Station::IsValidID(cp->GetFirstStation())) openttd_rust_cargo_packet_set(cp->RustOwner(), 1, StationID::Invalid().base());
		}
	}
	if (!IsSavegameVersionBefore(SLV_68)) {
		for (Vehicle *v : Vehicle::Iterate()) v->cargo.InvalidateCache();
		for (Station *st : Station::Iterate()) {
			for (GoodsEntry &ge : st->goods) if (ge.HasData()) ge.GetData().cargo.InvalidateCache();
		}
	}
	if (IsSavegameVersionBefore(SLV_181)) for (Vehicle *v : Vehicle::Iterate()) v->cargo.KeepAll();
	if (IsSavegameVersionBefore(SLV_CARGO_TRAVELLED)) {
		for (Station *st : Station::Iterate()) {
			for (GoodsEntry &ge : st->goods) {
				if (!ge.HasData()) continue;
				auto packets = ge.GetData().cargo.Packets();
				for (auto &pair : *packets) for (CargoPacket *cp : pair.second) {
					auto f = cp->Export();
					if (f.source_xy != INVALID_TILE.base() && f.source_xy != st->xy.base()) {
						f.travelled_x = TileX(TileIndex(f.source_xy)) - TileX(st->xy);
						f.travelled_y = TileY(TileIndex(f.source_xy)) - TileY(st->xy);
						cp->Import(f);
					}
				}
			}
		}
		for (Vehicle *v : Vehicle::Iterate()) {
			auto packets = v->cargo.Packets();
			for (CargoPacket *cp : *packets) if (cp->Export().source_xy != INVALID_TILE.base()) cp->UpdateLoadingTile(TileIndex(cp->Export().source_xy));
		}
	}
#ifdef WITH_ASSERT
	for (Vehicle *v : Vehicle::Iterate()) {
		auto packets = v->cargo.Packets();
		for (CargoPacket *cp : *packets) { auto f = cp->Export(); f.in_vehicle = true; cp->Import(f); }
	}
#endif
}

/**
 * Wrapper function to get the CargoPacket's internal structure while
 * some of the variables itself are private.
 * @return the saveload description for CargoPackets.
 */
SaveLoadTable GetCargoPacketDesc()
{
	static const SaveLoad desc[] = {
		SLE_VARNAME(OpenTTDCargoPacketFields, first_station, "source", SLE_UINT16),
		SLE_VAR(OpenTTDCargoPacketFields, source_xy, SLE_UINT32),
		SLE_CONDVARNAME(OpenTTDCargoPacketFields, next_hop, "loaded_at_xy", SLE_FILE_U32 | SLE_VAR_U16, SL_MIN_VERSION, SLV_REMOVE_LOADED_AT_XY),
		SLE_CONDVARNAME(OpenTTDCargoPacketFields, next_hop, "loaded_at_xy", SLE_UINT16, SLV_REMOVE_LOADED_AT_XY, SL_MAX_VERSION),
		SLE_VAR(OpenTTDCargoPacketFields, count, SLE_UINT16),
		SLE_CONDVARNAME(OpenTTDCargoPacketFields, periods_in_transit, "days_in_transit", SLE_FILE_U8 | SLE_VAR_U16, SL_MIN_VERSION, SLV_MORE_CARGO_AGE),
		SLE_CONDVARNAME(OpenTTDCargoPacketFields, periods_in_transit, "days_in_transit", SLE_UINT16, SLV_MORE_CARGO_AGE, SLV_PERIODS_IN_TRANSIT_RENAME),
		SLE_CONDVAR(OpenTTDCargoPacketFields, periods_in_transit, SLE_UINT16, SLV_PERIODS_IN_TRANSIT_RENAME, SL_MAX_VERSION),
		SLE_VAR(OpenTTDCargoPacketFields, feeder_share, SLE_INT64),
		SLE_CONDVARNAME(OpenTTDCargoPacketFields, source_type, "source_type", SLE_UINT8, SLV_125, SL_MAX_VERSION),
		SLE_CONDVARNAME(OpenTTDCargoPacketFields, source_id, "source_id", SLE_UINT16, SLV_125, SL_MAX_VERSION),
		SLE_CONDVARNAME(OpenTTDCargoPacketFields, travelled_x, "travelled.x", SLE_INT16, SLV_CARGO_TRAVELLED, SL_MAX_VERSION),
		SLE_CONDVARNAME(OpenTTDCargoPacketFields, travelled_y, "travelled.y", SLE_INT16, SLV_CARGO_TRAVELLED, SL_MAX_VERSION),
	};
	return desc;
}

struct CAPAChunkHandler : ChunkHandler {
	CAPAChunkHandler() : ChunkHandler('CAPA', CH_TABLE) {}

	void Save() const override
	{
		SlTableHeader(GetCargoPacketDesc());

		for (CargoPacket *cp : CargoPacket::Iterate()) {
			SlSetArrayIndex(cp->index);
			SlObject(static_cast<OpenTTDCargoPacketFields *>(cp->RustOwner()), GetCargoPacketDesc());
		}
	}

	void Load() const override
	{
		const std::vector<SaveLoad> slt = SlCompatTableHeader(GetCargoPacketDesc(), _cargopacket_sl_compat);

		int index;

		while ((index = SlIterateArray()) != -1) {
			CargoPacket *cp = new (CargoPacketID(index)) CargoPacket();
			SlObject(static_cast<OpenTTDCargoPacketFields *>(cp->RustOwner()), slt);
		}
	}
};

static const CAPAChunkHandler CAPA;
static const ChunkHandlerRef cargopacket_chunk_handlers[] = {
	CAPA,
};

extern const ChunkHandlerTable _cargopacket_chunk_handlers(cargopacket_chunk_handlers);
