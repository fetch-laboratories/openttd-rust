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
    /// Optional station cargo and flow owners. Native views borrow these pointers
    /// only until clear/destruction; no facade owns either allocation.
    "OpenTTDStationGoodsData" ["station"] => pub struct StationGoodsData {
        pub cargo: *mut crate::cargo_storage::List => struct OpenTTDCargoList,
        pub flows: *mut crate::cargo_flow::FlowMap => struct OpenTTDCargoFlowMap,
    }
    /// One `GoodsEntry` cargo slot.
    #[derive(Clone, Copy)]
    "OpenTTDGoodsEntry" ["station"] => pub struct GoodsRecord {
        /// Optional Rust-owned cargo/flow attachment, cleared before destruction.
        pub data: *mut StationGoodsData,
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

/// Read from the record of a live station shell; the borrow ends with `f`, before
/// any callback. `shell` is a live native station shell of the serialized game state.
#[inline]
pub(crate) fn read<T>(shell: *const BaseStation, f: impl FnOnce(&StationRecord) -> T) -> T {
    // SAFETY: Live record; the shared borrow lasts only for the immediate read.
    f(unsafe { &*record_of(shell) })
}

/// `BaseStation::xy` of a live shell.
#[inline]
pub(crate) fn xy(shell: *const BaseStation) -> u32 {
    read(shell, |r| r.xy.0)
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

/// `BaseStation::GetIfValid(id)`: the live shell, or null for a hole or an index
/// at or beyond the pool's live extent.
pub(crate) fn station_get_if_valid(s: &crate::services::Services, id: u16) -> *mut BaseStation {
    let pool = (s.station_pool)();
    let index = usize::from(id);
    // SAFETY: The live extent and slot array come from the canonical pool.
    unsafe {
        if index >= pool.first_unused.read() {
            return std::ptr::null_mut();
        }
        pool.slots.add(index).read()
    }
}

/// Resolve a typed station ID to its canonical record under `BaseStation::Get`'s
/// live-index precondition. No record borrow survives a callback.
#[inline]
pub(crate) fn get_record(
    s: &crate::services::Services,
    id: crate::station_type::StationID,
) -> *mut StationRecord {
    // SAFETY: The caller supplies a live ID, as the original Get required.
    unsafe { record_of(station_get(s, id.0)) }
}

/// `BaseStation::GetIfValid`: preserve index/hole validity without adding a
/// specialized station/waypoint type filter. A reused ID may name a new owner.
#[inline]
pub(crate) fn get_record_if_valid(
    s: &crate::services::Services,
    id: crate::station_type::StationID,
) -> *mut StationRecord {
    let shell = station_get_if_valid(s, id.0);
    if shell.is_null() {
        std::ptr::null_mut()
    } else {
        // SAFETY: The slot currently holds a live shell with its canonical record.
        unsafe { record_of(shell) }
    }
}

/// Create cargo first and flows second, then publish the complete attachment.
/// The caller supplies a live goods slot under the game-state mutex.
pub(crate) unsafe fn goods_data_ensure(goods: *mut GoodsRecord) -> *mut StationGoodsData {
    let existing = unsafe { (*goods).data };
    if !existing.is_null() {
        return existing;
    }
    let cargo = crate::cargo_storage::openttd_rust_cargo_list_new(0);
    let flows = crate::cargo_flow::openttd_rust_flow_map_new();
    let data = Box::into_raw(Box::new(StationGoodsData { cargo, flows }));
    unsafe { (*goods).data = data };
    data
}

/// Existing data only; NewGRF observations must not allocate an empty list.
#[inline]
pub(crate) fn existing_cargo(
    shell: *const BaseStation,
    cargo: u8,
) -> *mut crate::cargo_storage::List {
    // SAFETY: Native callers required a live Station and a valid cargo slot.
    unsafe {
        let data = (*record_of(shell)).goods[usize::from(cargo)].data;
        if data.is_null() {
            std::ptr::null_mut()
        } else {
            (*data).cargo
        }
    }
}

/// Resolve the live goods slot and create its canonical cargo/flow attachment.
#[inline]
pub(crate) fn ensure_cargo(
    shell: *const BaseStation,
    cargo: u8,
) -> *mut crate::cargo_storage::List {
    // SAFETY: Same serialized live shell/valid cargo contract as GetOrCreateData.
    unsafe {
        let goods = &raw mut (*record_of(shell)).goods[usize::from(cargo)];
        (*goods_data_ensure(goods)).cargo
    }
}

/// Source `GoodsEntryData::IsEmpty`: available plus reserved cargo (u32), then
/// flow emptiness. No reference survives this immediate observation.
pub(crate) unsafe fn goods_data_is_empty(data: *const StationGoodsData) -> bool {
    unsafe {
        crate::cargo_storage::openttd_rust_cargo_list_station_total((*data).cargo) == 0
            && (*(*data).flows).is_empty()
    }
}

/// Detach before destruction. Cargo packet callbacks may reenter the goods slot
/// and create a new attachment; never overwrite that new link after callbacks.
pub(crate) unsafe fn goods_data_clear(
    goods: *mut GoodsRecord,
    storage: *const crate::cargo_storage::Services,
) {
    let data = unsafe { std::ptr::replace(&raw mut (*goods).data, std::ptr::null_mut()) };
    if data.is_null() {
        return;
    }
    let (flows, cargo) = unsafe { ((*data).flows, (*data).cargo) };
    unsafe {
        crate::cargo_flow::openttd_rust_flow_map_destroy(flows);
        crate::cargo_storage::openttd_rust_cargo_list_destroy(cargo, storage);
        drop(Box::from_raw(data));
    }
}

/// Native borrowed presentation; lifetime is that of the live goods slot.
/// # Safety
/// `goods` is live and accessed under the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_goods_data_ensure(
    goods: *mut GoodsRecord,
) -> *mut StationGoodsData {
    unsafe { goods_data_ensure(goods) }
}

/// # Safety
/// `data` is a live attachment, with no concurrent mutation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_goods_data_is_empty(
    data: *const StationGoodsData,
) -> u8 {
    u8::from(unsafe { goods_data_is_empty(data) })
}

/// # Safety
/// `goods` is live and `storage` is the canonical cargo operation table. Neither
/// this function nor its caller retains attachment aliases through destruction.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_goods_data_clear(
    goods: *mut GoodsRecord,
    storage: *const crate::cargo_storage::Services,
) {
    unsafe { goods_data_clear(goods, storage) };
}

// The two area types have the generated OpenTTDTileArea layout; TileIndex is
// transparent over u32. Returned pointers borrow fields of the live record.
#[inline]
pub(crate) fn train_area(shell: *const BaseStation) -> *mut crate::tilearea::TileArea {
    // SAFETY: The live shell owns this record throughout the immediate access.
    unsafe { (&raw mut (*record_of(shell)).train_station).cast() }
}
#[inline]
pub(crate) fn airport_area(shell: *const BaseStation) -> *mut crate::tilearea::TileArea {
    // SAFETY: The live shell owns this record throughout the immediate access.
    unsafe { (&raw mut (*record_of(shell)).airport.area).cast() }
}
#[inline]
pub(crate) fn ship_area(shell: *const BaseStation) -> *mut crate::tilearea::TileArea {
    // SAFETY: The live shell owns this record throughout the immediate access.
    unsafe { (&raw mut (*record_of(shell)).ship_station).cast() }
}
#[inline]
pub(crate) fn docking_area(shell: *const BaseStation) -> *mut crate::tilearea::TileArea {
    // SAFETY: The live shell owns this record throughout the immediate access.
    unsafe { (&raw mut (*record_of(shell)).docking_station).cast() }
}
#[inline]
pub(crate) fn road_waypoint_area(shell: *const BaseStation) -> *mut crate::tilearea::TileArea {
    // SAFETY: The live shell owns this record throughout the immediate access.
    unsafe { (&raw mut (*record_of(shell)).road_waypoint_area).cast() }
}
#[inline]
pub(crate) fn stop_area(shell: *const BaseStation, truck: bool) -> *mut crate::tilearea::TileArea {
    // SAFETY: The selected area is a field of the live record.
    unsafe {
        let record = record_of(shell);
        if truck {
            (&raw mut (*record).truck_station).cast()
        } else {
            (&raw mut (*record).bus_station).cast()
        }
    }
}
/// Store an area at the original SetTrain/Ship/RoadWaypointStation observation.
#[inline]
pub(crate) fn set_area(area: *mut crate::tilearea::TileArea, value: crate::tilearea::TileArea) {
    // SAFETY: A record-area accessor supplies the live, writable destination.
    unsafe { area.write(value) };
}
/// `OrthogonalTileArea::Clear` on the selected live record field.
#[inline]
pub(crate) fn clear_area(area: *mut crate::tilearea::TileArea) {
    // SAFETY: A record-area accessor supplies the live, writable destination.
    unsafe { (*area).clear() };
}

/// Store the canonical `town` field at the original native setter point.
#[inline]
pub(crate) fn set_town(shell: *const BaseStation, value: *const crate::services::TownHandle) {
    // SAFETY: The live shell owns a writable record during this serialized access.
    let record = unsafe { record_of(shell) };
    crate::raw_field::write_field!(record, town, value.cast_mut().cast());
}
/// Store the canonical `owner` field at the original native setter point.
#[inline]
pub(crate) fn set_station_owner(shell: *const BaseStation, value: u8) {
    // SAFETY: The live shell owns a writable record during this serialized access.
    let record = unsafe { record_of(shell) };
    crate::raw_field::write_field!(record, owner, crate::company_type::CompanyID(value));
}
/// Store the canonical `facilities` field at the original native setter point.
#[inline]
pub(crate) fn set_facilities(shell: *const BaseStation, value: u8) {
    // SAFETY: The live shell owns a writable record during this serialized access.
    let record = unsafe { record_of(shell) };
    crate::raw_field::write_field!(record, facilities, value);
}
/// Store the canonical `build_date` field at the original native setter point.
#[inline]
pub(crate) fn set_build_date(shell: *const BaseStation, value: i32) {
    // SAFETY: The live shell owns a writable record during this serialized access.
    let record = unsafe { record_of(shell) };
    crate::raw_field::write_field!(record, build_date, value);
}
/// Store the canonical `xy` field at the original native setter point.
#[inline]
pub(crate) fn set_location(shell: *const BaseStation, value: u32) {
    // SAFETY: The live shell owns a writable record during this serialized access.
    let record = unsafe { record_of(shell) };
    crate::raw_field::write_field!(record, xy, crate::tile_type::TileIndex(value));
}
/// Store the canonical `string_id` field at the original native setter point.
#[inline]
pub(crate) fn set_string_id(shell: *const BaseStation, value: u32) {
    // SAFETY: The live shell owns a writable record during this serialized access.
    let record = unsafe { record_of(shell) };
    crate::raw_field::write_field!(record, string_id, value);
}
/// Store the canonical `waiting_random_triggers` field at the original native setter point.
#[inline]
pub(crate) fn set_waiting_triggers(shell: *const BaseStation, value: u8) {
    // SAFETY: The live shell owns a writable record during this serialized access.
    let record = unsafe { record_of(shell) };
    crate::raw_field::write_field!(record, waiting_random_triggers, value);
}
/// Store the canonical `random_bits` field at the original native setter point.
#[inline]
pub(crate) fn set_random_bits(shell: *const BaseStation, value: u16) {
    // SAFETY: The live shell owns a writable record during this serialized access.
    let record = unsafe { record_of(shell) };
    crate::raw_field::write_field!(record, random_bits, value);
}
/// The station's canonical town link, as the shared native town handle.
#[inline]
pub(crate) fn station_town(shell: *const BaseStation) -> *const crate::services::TownHandle {
    read(shell, |r| r.town.cast_const().cast())
}
/// `StationFacilities::Reset` at the original station observation point.
pub(crate) fn reset_facilities(shell: *const BaseStation, bits: u8) {
    // SAFETY: This live record is serialized; no borrow spans any callback.
    let record = unsafe { record_of(shell) };
    crate::raw_field::write_field!(
        record,
        facilities,
        crate::raw_field::read_field!(record, facilities) & !bits
    );
}
/// `StationFacilities::Set` at the original station observation point.
pub(crate) fn set_facility_bits(shell: *const BaseStation, bits: u8) {
    // SAFETY: This live record is serialized; no borrow spans any callback.
    let record = unsafe { record_of(shell) };
    crate::raw_field::write_field!(
        record,
        facilities,
        crate::raw_field::read_field!(record, facilities) | bits
    );
}
/// The station command's airport type/layout, cleared blocks and rotation stores.
pub(crate) fn set_airport_details(shell: *const BaseStation, kind: u8, layout: u8, rotation: u8) {
    // SAFETY: These field-sized stores address the live record and call nothing.
    let record = unsafe { record_of(shell) };
    crate::raw_field::write_field!(record, airport.r#type, kind);
    crate::raw_field::write_field!(record, airport.layout, layout);
    crate::raw_field::write_field!(record, airport.blocks, 0);
    crate::raw_field::write_field!(record, airport.rotation, rotation);
}
/// The oil-rig command's airport type and rotation stores.
pub(crate) fn set_airport_kind_rotation(shell: *const BaseStation, kind: u8, rotation: u8) {
    // SAFETY: These field-sized stores address the live record and call nothing.
    let record = unsafe { record_of(shell) };
    crate::raw_field::write_field!(record, airport.r#type, kind);
    crate::raw_field::write_field!(record, airport.rotation, rotation);
}
/// `AirportBlocks::Flip(AirportClosed)`.
pub(crate) fn flip_airport_closed(shell: *const BaseStation) {
    // SAFETY: The live record's mask is serialized at this immediate write.
    let record = unsafe { record_of(shell) };
    crate::raw_field::write_field!(
        record,
        airport.blocks,
        crate::raw_field::read_field!(record, airport.blocks) ^ crate::airport::AIRPORT_CLOSED
    );
}
/// The station command's cached animation/cargo trigger publication.
pub(crate) fn set_cached_triggers(shell: *const BaseStation, animation: u16, cargo: u64) {
    // SAFETY: Stores end before returning to any resolver callback.
    let record = unsafe { record_of(shell) };
    crate::raw_field::write_field!(record, cached_anim_triggers, animation);
    crate::raw_field::write_field!(record, cached_cargo_triggers, cargo);
}
/// The road-stop command's cached animation/cargo trigger publication.
pub(crate) fn set_cached_roadstop_triggers(shell: *const BaseStation, animation: u16, cargo: u64) {
    // SAFETY: Stores end before returning to any resolver callback.
    let record = unsafe { record_of(shell) };
    crate::raw_field::write_field!(record, cached_roadstop_anim_triggers, animation);
    crate::raw_field::write_field!(record, cached_roadstop_cargo_triggers, cargo);
}

/// `BaseStation::GetPoolSize()`: the pool's live extent.
pub(crate) fn pool_size(s: &crate::services::Services) -> usize {
    // SAFETY: The live extent is a scalar of the canonical pool.
    unsafe { (s.station_pool)().first_unused.read() }
}

/// `StationFacilities` of a live shell.
/// `shell` is a live native station shell of the serialized game state.
#[inline]
pub(crate) fn facilities(shell: *const BaseStation) -> u8 {
    // SAFETY: Immediate scalar read of the live record.
    unsafe { (*record_of(shell)).facilities }
}

/// `BaseStation::IsInUse`: any of the train, truck, bus, airport or dock facilities.
#[inline]
pub(crate) fn in_use(shell: *const BaseStation) -> bool {
    facilities(shell) & 0x1F != 0
}

/// `Station::IsExpected`: the shell is a station, not a waypoint.
#[inline]
pub(crate) fn is_station(shell: *const BaseStation) -> bool {
    facilities(shell) & FACILITY_WAYPOINT == 0
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
pub(crate) const FACILITY_WAYPOINT: u8 = 1 << 7;

/// The next live station or waypoint slot at or after `*from`: reread the live
/// extent and slot, skip holes, then advance. Reach the shell's record in one load.
///
/// # Safety
/// Serialized game state; `pool` came from the shared services in this entry; no
/// pool reset or load runs while the cursor lives.
unsafe fn next_base_station(
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
        return Some((shell, record));
    }
}

/// `Station::Iterate` filters waypoints from the live base-station slots.
/// # Safety
/// The cursor obeys `next_base_station`'s serialized, no-reset lifetime contract.
pub(crate) unsafe fn next_station(
    s: &crate::services::Services,
    pool: &mut StationPool,
    from: &mut usize,
) -> Option<(*mut BaseStation, *mut StationRecord)> {
    // SAFETY: The caller supplies the original traversal lifetime contract.
    while let Some((shell, record)) = unsafe { next_base_station(s, pool, from) } {
        if crate::raw_field::read_field!(record, facilities) & FACILITY_WAYPOINT == 0 {
            return Some((shell, record));
        }
    }
    None
}

/// Resolve the first Station at or after `from`, as the native filtered iterator.
pub(crate) fn next_station_shell(s: &crate::services::Services, from: u32) -> *mut BaseStation {
    let mut pool = (s.station_pool)();
    let mut cursor = from as usize;
    // SAFETY: This cursor is confined to the immediate serialized lookup.
    unsafe { next_station(s, &mut pool, &mut cursor) }
        .map_or(std::ptr::null_mut(), |(shell, _)| shell)
}

/// Visit canonical records in `BaseStation::Iterate` order. A visit may mutate
/// the pool; reacquire slots and the live extent before observing the next slot.
pub(crate) fn for_each_base_station(
    s: &crate::services::Services,
    mut visit: impl FnMut(*mut StationRecord, crate::station_type::StationID),
) {
    let mut pool = (s.station_pool)();
    let mut from = 0;
    // SAFETY: Serialized entry; no record borrow spans a visit or pool mutation.
    while let Some((_, record)) = unsafe { next_base_station(s, &mut pool, &mut from) } {
        #[allow(clippy::cast_possible_truncation)]
        visit(record, crate::station_type::StationID((from - 1) as u16));
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

/// `SetBit(Waypoint::waypoint_flags, WPF_ROAD)`.
pub(crate) fn set_road_waypoint(shell: *const BaseStation) {
    // SAFETY: A live waypoint owns this serialized field.
    let record = unsafe { record_of(shell) };
    crate::raw_field::write_field!(
        record,
        waypoint_flags,
        crate::raw_field::read_field!(record, waypoint_flags)
            | (1 << crate::waypoint_base::WPF_ROAD)
    );
}
