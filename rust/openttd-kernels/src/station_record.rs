/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared station, goods, airport and road-stop records (#634).
//!
//! One `StationRecord` lives beside every `BaseStation` shell, `Station` and
//! `Waypoint` alike, from before the shell's members until after its base
//! destructor. It is the first member of the canonical Rust station owner, so a
//! record pointer is also the owner's address. One `RoadStopRecord` lives beside
//! every `RoadStop` shell. Both languages access fields in place under the
//! game-state mutex; Rust holds no reference to a record across a callback.
#![allow(unsafe_code)]

use crate::company_type::CompanyID;
use crate::station_type::BaseStation;
use crate::tile_type::TileIndex;

/// Opaque native `Town`.
#[repr(C)]
pub struct Town {
    _private: [u8; 0],
}
/// Opaque native `PersistentStorage`.
#[repr(C)]
pub struct PersistentStorage {
    _private: [u8; 0],
}
/// Opaque native `GoodsEntryData`: the cargo list and flow facades, owned by C++.
#[repr(C)]
pub struct GoodsEntryData {
    _private: [u8; 0],
}
/// Opaque native `RoadStop` shell.
#[repr(C)]
pub struct RoadStop {
    _private: [u8; 0],
}

/// `NUM_CARGO`.
pub const NUM_CARGO: usize = 64;

crate::abi::shared_records! {
    fn abi_record;
    /// Native `OrthogonalTileArea` scalars.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    "OpenTTDTileArea" ["station"] => pub struct TileAreaRecord {
        pub tile: TileIndex,
        pub w: u16,
        pub h: u16,
    }
    /// `StationRect`, the station spread rectangle.
    #[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
    "OpenTTDStationRect" ["station"] => pub struct StationRect {
        pub left: i32,
        pub top: i32,
        pub right: i32,
        pub bottom: i32,
    }
    /// One `GoodsEntry` cargo slot.
    #[derive(Clone, Copy)]
    "OpenTTDGoodsEntry" ["station"] => pub struct GoodsRecord {
        /// `GoodsEntry::data`, owned by the native station shell.
        pub data: *mut GoodsEntryData,
        pub max_waiting_cargo: u32,
        /// `NodeID`.
        pub node: u16,
        /// `LinkGraphID`.
        pub link_graph: u16,
        /// Native `GoodsEntry::States`.
        pub status: u8,
        pub time_since_pickup: u8,
        pub rating: u8,
        pub last_speed: u8,
        pub last_age: u8,
        pub amount_fract: u8,
    }
    /// `Station::airport`.
    "OpenTTDAirport" ["station"] => pub struct AirportRecord {
        /// Native `AirportBlocks`.
        pub blocks: u64,
        pub psa: *mut PersistentStorage,
        pub area: TileAreaRecord,
        pub r#type: u8,
        pub layout: u8,
        /// Native `Direction`.
        pub rotation: u8,
    }
    /// The fields of one `BaseStation`, `Station` or `Waypoint` shell.
    "OpenTTDStationRecord" ["station"] => pub struct StationRecord {
        /// Owning native shell; stable while the record lives.
        pub shell: *mut BaseStation,
        pub town: *mut Town,
        /// NOSAVE `Station::industry`.
        pub industry: *mut crate::industry_type::Industry,
        /// `Station::bus_stops`.
        pub bus_stops: *mut RoadStopRecord,
        /// `Station::truck_stops`.
        pub truck_stops: *mut RoadStopRecord,
        /// NOSAVE `BaseStation::cached_cargo_triggers`.
        pub cached_cargo_triggers: u64,
        /// NOSAVE `BaseStation::cached_roadstop_cargo_triggers`.
        pub cached_roadstop_cargo_triggers: u64,
        pub always_accepted: u64,
        pub airport: AirportRecord,
        pub goods: [GoodsRecord; 64],
        pub rect: StationRect,
        pub train_station: TileAreaRecord,
        pub bus_station: TileAreaRecord,
        pub truck_station: TileAreaRecord,
        pub ship_station: TileAreaRecord,
        pub docking_station: TileAreaRecord,
        /// `Waypoint::road_waypoint_area`.
        pub road_waypoint_area: TileAreaRecord,
        pub xy: TileIndex,
        /// `StringID`.
        pub string_id: u32,
        /// `TimerGameCalendar::Date`.
        pub build_date: i32,
        pub random_bits: u16,
        /// NOSAVE native `StationAnimationTriggers`.
        pub cached_anim_triggers: u16,
        /// NOSAVE native `StationAnimationTriggers`.
        pub cached_roadstop_anim_triggers: u16,
        /// `Waypoint::town_cn`.
        pub town_cn: u16,
        /// `Waypoint::waypoint_flags`.
        pub waypoint_flags: u16,
        pub owner: CompanyID,
        /// Native `StationFacilities`.
        pub facilities: u8,
        /// Native `StationRandomTriggers`.
        pub waiting_random_triggers: u8,
        /// `IndustryType`.
        pub indtype: u8,
        /// Native `StationHadVehicleOfType`.
        pub had_vehicle_of_type: u8,
        pub delete_ctr: u8,
        pub time_since_load: u8,
        pub time_since_unload: u8,
        pub last_vehicle_type: u8,
    }
    /// The fields of one `RoadStop` shell.
    "OpenTTDRoadStopState" ["station"] => pub struct RoadStopRecord {
        /// Owning native shell; stable while the record lives.
        pub shell: *mut RoadStop,
        /// `RoadStop::next`.
        pub next: *mut RoadStopRecord,
        /// Shared entries, owned by the `BaseEntry` holder.
        pub entries: *mut crate::road_stop::Entries,
        pub xy: TileIndex,
        /// Native `RoadStopStatusFlags`.
        pub status: u8,
    }
}

/// Byte offset of the record pointer inside every `BaseStation` shell, published
/// by the native record owner at the first construction. It is fixed by the
/// shell's ABI layout, so a shell reaches its record in one load.
static SHELL_RECORD_OFFSET: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(usize::MAX);

/// Publish a real shell's record-pointer offset; fixtures without a shell pass null.
pub(crate) fn publish_offset_for(shell: *mut BaseStation, offset: usize) {
    if !shell.is_null() {
        publish_offset(offset);
    }
}

/// Publish the shell's record-pointer offset; every shell reports the same one.
fn publish_offset(offset: usize) {
    use std::sync::atomic::Ordering::Relaxed;
    let previous = SHELL_RECORD_OFFSET.swap(offset, Relaxed);
    assert!(
        previous == usize::MAX || previous == offset,
        "inconsistent station record offset"
    );
}

/// The record of a live `BaseStation`, `Station` or `Waypoint` shell.
///
/// # Safety
/// `shell` is a live native station shell; at least one shell has been constructed.
#[inline]
pub(crate) unsafe fn record_of(shell: *const BaseStation) -> *mut StationRecord {
    let offset = SHELL_RECORD_OFFSET.load(std::sync::atomic::Ordering::Relaxed);
    // SAFETY: Live shells hold their owning, pointer-aligned record pointer at the
    // published offset of the native record owner member.
    #[allow(clippy::cast_ptr_alignment)]
    unsafe {
        shell
            .cast::<u8>()
            .add(offset)
            .cast::<*mut StationRecord>()
            .read()
    }
}

/// The canonical Rust owner of a live station shell: its record's enclosing owner.
/// `shell` is a live native station shell of the serialized game state.
#[inline]
pub(crate) fn service_of(shell: *const BaseStation) -> *mut crate::station_service::Service {
    // SAFETY: Callers pass live shells, as every former owner callback required.
    unsafe { record_of(shell).cast() }
}

/// `BaseStation::owner` of a live shell, as its company byte.
/// `shell` is a live native station shell of the serialized game state.
#[inline]
pub(crate) fn owner_company(shell: *const BaseStation) -> u8 {
    // SAFETY: Immediate scalar read of the live record.
    unsafe { (*record_of(shell)).owner.0 }
}

/// `BaseStation::Get(id)`: the live shell at a valid station index.
/// `id` is a valid index of the canonical pool, as `Get` asserts natively.
pub(crate) fn station_get(s: &crate::services::Services, id: u16) -> *mut BaseStation {
    let pool = (s.station_pool)();
    let index = usize::from(id);
    // SAFETY: The live extent and slot array come from the canonical pool.
    unsafe {
        assert!(index < pool.first_unused.read());
        pool.slots.add(index).read()
    }
}

/// Call-scoped view of the native station pool's slot array. Growth relocates
/// `slots`, but never lowers `first_unused`; refresh before reading past `slots_len`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct StationPool {
    pub slots: *const *mut BaseStation,
    pub slots_len: usize,
    pub first_unused: *const usize,
}

/// `StationFacility::Waypoint`.
const FACILITY_WAYPOINT: u8 = 1 << 7;

/// The next live `Station` (not waypoint) slot at or after `*from`, as the native
/// filtered pool iterator finds it: reread the live extent and slot, skip holes and
/// waypoints, then advance past it. The shell's record is reached in one load.
///
/// # Safety
/// Serialized game state; `pool` came from the shared services in this entry; no
/// pool reset or load runs while the cursor lives.
pub(crate) unsafe fn next_station(
    s: &crate::services::Services,
    pool: &mut StationPool,
    from: &mut usize,
) -> Option<(*mut BaseStation, *mut StationRecord)> {
    loop {
        // SAFETY: The live extent is a scalar of the canonical pool.
        let live = unsafe { pool.first_unused.read() };
        if *from >= live {
            return None;
        }
        if live > pool.slots_len {
            *pool = (s.station_pool)();
        }
        // SAFETY: `*from` is below the live extent, inside the refreshed slot array.
        let shell = unsafe { pool.slots.add(*from).read() };
        *from += 1;
        if shell.is_null() {
            continue;
        }
        // SAFETY: A non-null slot holds a live shell.
        let record = unsafe { record_of(shell) };
        // SAFETY: Immediate scalar read of the live record.
        if unsafe { (*record).facilities } & FACILITY_WAYPOINT == 0 {
            return Some((shell, record));
        }
    }
}

/// `for (Station *st : Station::Iterate())`, visiting each station shell with its
/// record owner and index. A visit may reenter, delete or allocate stations: the
/// cursor holds only the next index and rereads the pool after each visit.
pub(crate) fn for_each_station(
    s: &crate::services::Services,
    mut visit: impl FnMut(*const crate::station::Station, *mut crate::station_service::Service, u16),
) {
    let mut pool = (s.station_pool)();
    let mut from = 0;
    // SAFETY: Serialized entry; no record reference spans a visit.
    while let Some((shell, record)) = unsafe { next_station(s, &mut pool, &mut from) } {
        // Station IDs are 16-bit; the pool never exceeds their range.
        #[allow(clippy::cast_possible_truncation)]
        visit(shell.cast_const().cast(), record.cast(), (from - 1) as u16);
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDStationPool" ["station"] => crate::station_record::StationPool { slots, slots_len, first_unused, };
}

/// `INVALID_TILE`.
const INVALID_TILE: TileIndex = TileIndex(u32::MAX);
/// An empty `TileArea` at `INVALID_TILE`.
pub const EMPTY_AREA: TileAreaRecord = TileAreaRecord {
    tile: INVALID_TILE,
    w: 0,
    h: 0,
};

impl Default for GoodsRecord {
    /// The original `GoodsEntry` member defaults.
    fn default() -> Self {
        Self {
            data: std::ptr::null_mut(),
            max_waiting_cargo: 0,
            node: u16::MAX,
            link_graph: u16::MAX,
            status: 0,
            time_since_pickup: 255,
            rating: crate::station_base::INITIAL_STATION_RATING,
            last_speed: 0,
            last_age: 255,
            amount_fract: 0,
        }
    }
}

impl StationRecord {
    /// The original `BaseStation`, `Station` and `Waypoint` member defaults.
    pub(crate) fn new(shell: *mut BaseStation, xy: TileIndex) -> Self {
        Self {
            shell,
            town: std::ptr::null_mut(),
            industry: std::ptr::null_mut(),
            bus_stops: std::ptr::null_mut(),
            truck_stops: std::ptr::null_mut(),
            cached_cargo_triggers: 0,
            cached_roadstop_cargo_triggers: 0,
            always_accepted: 0,
            airport: AirportRecord {
                blocks: 0,
                psa: std::ptr::null_mut(),
                area: EMPTY_AREA,
                r#type: 0,
                layout: 0,
                rotation: u8::MAX, // INVALID_DIR
            },
            goods: [GoodsRecord::default(); NUM_CARGO],
            rect: StationRect::default(),
            train_station: EMPTY_AREA,
            bus_station: EMPTY_AREA,
            truck_station: EMPTY_AREA,
            ship_station: EMPTY_AREA,
            docking_station: EMPTY_AREA,
            road_waypoint_area: EMPTY_AREA,
            xy,
            string_id: u32::from(u16::MAX),
            build_date: 0,
            random_bits: 0,
            cached_anim_triggers: 0,
            cached_roadstop_anim_triggers: 0,
            town_cn: 0,
            waypoint_flags: 0,
            owner: CompanyID(u8::MAX),
            facilities: 0,
            waiting_random_triggers: 0,
            indtype: u8::MAX,
            had_vehicle_of_type: 0,
            delete_ctr: 0,
            time_since_load: 255,
            time_since_unload: 255,
            last_vehicle_type: 255,
        }
    }
}
