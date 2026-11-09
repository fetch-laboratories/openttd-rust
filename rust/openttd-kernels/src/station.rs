/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Station catchment, nearby lists and station rectangles: `station.cpp`,
//! `ForAllStationsAroundTiles` (`station_base.h`) and the `station_cmd.cpp`
//! finder and around-tile cargo scans.
//!
//! Opaque native station handles: native code performs inheritance conversions,
//! preserving the C++ pointer adjustment; Rust never dereferences a handle. Handles
//! stay live until their owner is retired.
//!
//! Each station's rectangle, catchment bitmap and delivery list live in its
//! `station_service::Service`; town and industry nearby sets live in their Rust
//! owners. Native facades borrow that storage and never outlive the owner. Every
//! container borrow is immediate and ends before any service call, so callbacks
//! may reenter and mutate other owners; cursors re-read live storage at each step.
#![allow(unsafe_code)]

pub(crate) mod spatial;

use crate::base_station_base::StationRectMode;
use crate::bitmap_type::{BitmapTileArea, BitmapTileIterator};
use crate::cargo_type::{CargoArray, CargoType, CargoTypes, NUM_CARGO};
use crate::company_type::CompanyID;
use crate::geometry_type::{rect_height, rect_width};
use crate::industry::tile::ShellHandle;
use crate::industry_type::IndustryID;
use crate::industrytype::NativeVec;
use crate::map_func::Map;
use crate::services::Services;
use crate::station_service::Service;
use crate::station_type::StationID;
use crate::station_type::{
    CA_BUS, CA_DOCK, CA_NONE, CA_TRAIN, CA_TRUCK, CA_UNMODIFIED, MAX_CATCHMENT, StationType,
};
use crate::tile_type::{INVALID_TILE, TileIndex, TileType};
use crate::tilearea::TileArea;
use crate::town_type::TownID;
use StationRectMode as RectMode;

/// A native `Station`, distinct from the shared station/waypoint pool base.
pub use crate::station_type::Station;

/// A native `BaseStation`, which may also be a waypoint.
pub use crate::station_type::BaseStation;

/// `StationRect`: the station spread rectangle, aliased by the native `Rect` facade.
#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct StationRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}
impl StationRect {
    /// `MakeEmpty`.
    fn make_empty(&mut self) {
        *self = Self::default();
    }
    /// `PtInExtendedRect`.
    #[must_use]
    pub fn pt_in_extended_rect(&self, x: i32, y: i32, distance: i32) -> bool {
        self.left - distance <= x
            && x <= self.right + distance
            && self.top - distance <= y
            && y <= self.bottom + distance
    }
    /// `IsEmpty`; a zero left edge also counts as empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.left == 0 || self.left > self.right || self.top > self.bottom
    }
    /// `BeforeAddTile`; `false` is `STR_ERROR_STATION_TOO_SPREAD_OUT`.
    #[allow(clippy::cast_possible_wrap, clippy::many_single_char_names)]
    pub fn before_add_tile(&mut self, tile: TileIndex, mode: StationRectMode, c: &Ctx<'_>) -> bool {
        let x = c.map.tile_x(tile.0) as i32;
        let y = c.map.tile_y(tile.0) as i32;
        if self.is_empty() {
            // we are adding the first station tile
            if mode != RectMode::ADD_TEST {
                self.left = x;
                self.right = x;
                self.top = y;
                self.bottom = y;
            }
        } else if !self.pt_in_extended_rect(x, y, 0) {
            // current rect is not empty and new point is outside this rect
            // make new spread-out rectangle
            let new_rect = Self {
                left: x.min(self.left),
                top: y.min(self.top),
                right: x.max(self.right),
                bottom: y.max(self.bottom),
            };
            // check new rect dimensions against preset max
            let w = rect_width(new_rect.left, new_rect.right);
            let h = rect_height(new_rect.top, new_rect.bottom);
            if mode != RectMode::ADD_FORCE
                && (w > i32::from((c.c.spread)()) || h > i32::from((c.c.spread)()))
            {
                c.check(mode != RectMode::ADD_TRY);
                return false;
            }
            // spread-out ok, return true
            if mode != RectMode::ADD_TEST {
                // we should update the station rect
                *self = new_rect;
            }
        }
        // Otherwise the new point is inside the rect, we don't need to do anything.
        true
    }
    /// `BeforeAddRect`: an overlarge unforced rectangle succeeds without mutation;
    /// a failing second corner keeps the first corner's effect.
    pub fn before_add_rect(
        &mut self,
        tile: TileIndex,
        w: i32,
        h: i32,
        mode: StationRectMode,
        c: &Ctx<'_>,
    ) -> bool {
        if mode == RectMode::ADD_FORCE
            || (w <= i32::from((c.c.spread)()) && h <= i32::from((c.c.spread)()))
        {
            // Important when the old rect is completely inside the new rect, resp. the old one was empty.
            let ok = self.before_add_tile(tile, mode, c);
            if ok {
                return self.before_add_tile(
                    TileIndex(c.map.tile_add_xy(tile.0, w - 1, h - 1)),
                    mode,
                    c,
                );
            }
            return ok;
        }
        true
    }
    /// `AfterRemoveTile`: shrink edges while no station tile remains on them.
    #[allow(clippy::cast_possible_wrap)]
    pub fn after_remove_tile(&mut self, id: StationID, tile: TileIndex, c: &Ctx<'_>) -> bool {
        let mut x = c.map.tile_x(tile.0) as i32;
        let mut y = c.map.tile_y(tile.0) as i32;
        // look if removed tile was on the bounding rect edge
        // and try to reduce the rect by this edge
        // do it until we have empty rect or nothing to do
        loop {
            // check if removed tile is on rect edge
            let left_edge = x == self.left;
            let right_edge = x == self.right;
            let top_edge = y == self.top;
            let bottom_edge = y == self.bottom;
            // can we reduce the rect in either direction?
            let reduce_x = (left_edge || right_edge)
                && !scan_for_station_tiles(id, x, self.top, x, self.bottom, c);
            let reduce_y = (top_edge || bottom_edge)
                && !scan_for_station_tiles(id, self.left, y, self.right, y, c);
            if !(reduce_x || reduce_y) {
                break; // nothing to do (can't reduce)
            }
            if reduce_x {
                // reduce horizontally
                if left_edge {
                    // move left edge right
                    x += 1;
                    self.left = x;
                } else {
                    // move right edge left
                    x -= 1;
                    self.right = x;
                }
            }
            if reduce_y {
                // reduce vertically
                if top_edge {
                    // move top edge down
                    y += 1;
                    self.top = y;
                } else {
                    // move bottom edge up
                    y -= 1;
                    self.bottom = y;
                }
            }
            if self.left > self.right || self.top > self.bottom {
                // can't continue, if the remaining rectangle is empty
                self.make_empty();
                return true; // empty remaining rect
            }
        }
        false // non-empty remaining rect
    }
    /// `AfterRemoveRect`: the second corner is skipped once the first empties.
    #[allow(clippy::cast_possible_wrap)]
    pub fn after_remove_rect(&mut self, id: StationID, ta: TileArea, c: &Ctx<'_>) -> bool {
        let x = c.map.tile_x(ta.tile);
        let y = c.map.tile_y(ta.tile);
        c.check(self.pt_in_extended_rect(x as i32, y as i32, 0));
        c.check(self.pt_in_extended_rect(
            (x + u32::from(ta.w)).wrapping_sub(1) as i32,
            (y + u32::from(ta.h)).wrapping_sub(1) as i32,
            0,
        ));
        let mut empty = self.after_remove_tile(id, TileIndex(ta.tile), c);
        if ta.w != 1 || ta.h != 1 {
            empty = empty
                || self.after_remove_tile(
                    id,
                    TileIndex(
                        c.map
                            .tile_add_xy(ta.tile, i32::from(ta.w) - 1, i32::from(ta.h) - 1),
                    ),
                    c,
                );
        }
        empty
    }
}

/// `StationRect::ScanForStationTiles`: check whether station tiles of the given
/// station id exist in the given rectangle (edges inclusive).
#[allow(clippy::cast_sign_loss)]
fn scan_for_station_tiles(
    id: StationID,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    c: &Ctx<'_>,
) -> bool {
    let ta = TileArea::between(
        c.map,
        c.map.tile_xy(left as u32, top as u32),
        c.map.tile_xy(right as u32, bottom as u32),
    );
    ta.tiles(c.map).map(TileIndex).any(|tile| {
        TileType((c.s.tile_type)(tile.0)) == TileType::MP_STATION && c.station_index(tile) == id
    })
}

crate::abi::shared_records! {
    fn abi_record;
    /// One `StationList` member; the set orders by station ID.
    #[derive(Clone, Copy)]
    "OpenTTDStationListEntry" ["station_service"] => pub struct StationListEntry {
        /// Live member station.
        pub station: *mut BaseStation,
        /// Its pool index, the `StationCompare` key.
        pub id: StationID,
    }
    /// One canonical ordered set, owned by its existing town, industry or finder.
    #[derive(Default)]
    "OpenTTDStationList" ["station_service"] => pub struct StationList {
        /// Native borrows expire on mutation; this is the only storage metadata.
        pub entries: NativeVec<StationListEntry> => "RustVector<OpenTTDStationListEntry>",
    }
}
impl StationList {
    /// `insert`, ignoring a station already present.
    pub fn insert(&mut self, station: *mut BaseStation, id: StationID) {
        if let Err(at) = self.entries().binary_search_by_key(&id, |e| e.id) {
            let mut entries = std::mem::take(&mut self.entries).into_vec();
            entries.insert(at, StationListEntry { station, id });
            self.entries = NativeVec::from_vec(entries);
        }
    }
    /// `erase` by key.
    pub fn erase(&mut self, id: StationID) {
        if let Ok(at) = self.entries().binary_search_by_key(&id, |e| e.id) {
            let mut entries = std::mem::take(&mut self.entries).into_vec();
            entries.remove(at);
            self.entries = NativeVec::from_vec(entries);
        }
    }
    /// `clear`.
    pub fn clear(&mut self) {
        let mut entries = std::mem::take(&mut self.entries).into_vec();
        entries.clear();
        self.entries = NativeVec::from_vec(entries);
    }
    /// `begin()` for `previous == None`, else `upper_bound(previous)`.
    #[must_use]
    pub fn next_after(&self, previous: Option<StationID>) -> Option<StationListEntry> {
        let entries = self.entries();
        let at = previous.map_or(0, |id| entries.partition_point(|e| e.id <= id));
        entries.get(at).copied()
    }
    /// Ordered members.
    #[must_use]
    pub fn entries(&self) -> &[StationListEntry] {
        self.entries.as_slice()
    }
}
/// Walk a live list in order, re-reading it after each visit. Visits may mutate
/// the list; traversal resumes after the visited key like an erase-safe iterator.
/// # Safety
/// `list` stays live across every visit.
pub(crate) unsafe fn for_each_station(
    list: *const StationList,
    mut visit: impl FnMut(StationListEntry),
) {
    let mut previous = None;
    // SAFETY: Caller contract; each lookup borrow ends before the visit.
    while let Some(entry) = unsafe { (*list).next_after(previous) } {
        previous = Some(entry.id);
        visit(entry);
    }
}

/// One `IndustryList` member.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IndustryListEntry {
    /// `IndustryListEntry::distance`.
    pub distance: u32,
    /// Live industry shell.
    pub industry: *mut ShellHandle,
    /// Its pool index, the `IndustryCompare` tie-breaker.
    pub id: IndustryID,
}
/// `IndustryList`: ordered by distance, then industry index.
#[derive(Default)]
pub struct IndustryList(Vec<IndustryListEntry>);
impl IndustryList {
    /// Set `insert`: an equal key is not inserted again.
    pub fn insert(&mut self, entry: IndustryListEntry) {
        if let Err(at) = self
            .0
            .binary_search_by_key(&(entry.distance, entry.id), |e| (e.distance, e.id))
        {
            self.0.insert(at, entry);
        }
    }
    fn position(&self, industry: *mut ShellHandle) -> Option<usize> {
        self.0.iter().position(|e| e.industry == industry)
    }
    /// `clear`.
    pub fn clear(&mut self) {
        self.0.clear();
    }
    /// The member at `index`, in order.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<IndustryListEntry> {
        self.0.get(index).copied()
    }
    /// Ordered members.
    #[must_use]
    pub fn entries(&self) -> &[IndustryListEntry] {
        &self.0
    }
}

/// Catchment state of one `BaseStation`, held by its station service owner.
#[derive(Default)]
pub struct Catchment {
    rect: StationRect,
    tiles: BitmapTileArea,
    industries_near: IndustryList,
}
/// The canonical catchment of a live station owner.
/// # Safety
/// `state` is live; the pointer is used only for immediate borrows.
pub(crate) unsafe fn catchment(state: *mut Service) -> *mut Catchment {
    unsafe { crate::station_service::catchment(state) }
}
/// Visit `industries_near` in order, re-reading the live list before each visit.
/// # Safety
/// The station owner stays live across every visit.
pub(crate) unsafe fn for_each_industry_near(
    state: *mut Service,
    mut visit: impl FnMut(IndustryListEntry) -> bool,
) {
    let mut index = 0;
    // SAFETY: Caller contract; the lookup borrow ends before the visit.
    while let Some(entry) = unsafe { (*catchment(state)).industries_near.get(index) } {
        if !visit(entry) {
            break;
        }
        index += 1;
    }
}
/// `TileIsInCatchment`.
/// # Safety
/// `state` is a live station owner.
pub(crate) unsafe fn tile_is_in_catchment(
    state: *mut Service,
    tile: TileIndex,
    c: &Ctx<'_>,
) -> bool {
    unsafe {
        (*catchment(state))
            .tiles
            .has_tile(tile.0, c.map, |ok| c.check(ok))
    }
}
/// `rect.IsEmpty()` of a live station owner.
/// # Safety
/// `state` is a live station owner.
pub(crate) unsafe fn rect_is_empty(state: *mut Service) -> bool {
    unsafe { (*catchment(state)).rect.is_empty() }
}
/// The live rectangle of a station owner.
/// # Safety
/// `state` is a live station owner; the pointer is used only for immediate borrows.
pub(crate) unsafe fn rect(state: *mut Service) -> *mut StationRect {
    unsafe { &raw mut (*catchment(state)).rect }
}

/// Facility fields `GetCatchmentRadius` observes, at its source observation point.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RadiusFacilities {
    /// `bus_stops != nullptr`.
    pub bus: bool,
    /// `truck_stops != nullptr`.
    pub truck: bool,
    /// `train_station.tile`.
    pub train: u32,
    /// `ship_station.tile`.
    pub ship: u32,
    /// `airport.tile`.
    pub airport: u32,
}

/// Typed native services for station catchment. All leaves are direct,
/// synchronous and noexcept; none unwinds during ordinary play.
#[repr(C)]
pub struct CatchmentServices {
    pub shared: *const Services,
    pub airport: *const crate::newgrf_airport::Services,
    pub industry_tile: *const crate::industry::tile::TileServices,
    pub object: *const crate::objects::ObjectServices,
    pub town: *const crate::town::Leaves,
    /// `MoveGoodsToStation`'s station services.
    pub station: *const crate::station_service::World,
    /// `MoveGoodsToStation`'s vehicle visit services.
    pub visits: *const crate::vehicle::visits::Services,
    pub station_count: extern "C" fn() -> usize,
    pub station_if_valid: extern "C" fn(StationID) -> *mut BaseStation,
    pub next_station: extern "C" fn(u32) -> *mut BaseStation,
    pub owner: extern "C" fn(*mut BaseStation) -> *mut Service,
    /// `Station::Get(id)`'s owner.
    pub station_owner: extern "C" fn(StationID) -> *mut Service,
    pub id: extern "C" fn(*mut BaseStation) -> StationID,
    pub location: extern "C" fn(*const BaseStation) -> u32,
    pub associated_industry: extern "C" fn(*mut BaseStation) -> *mut ShellHandle,
    pub radius_facilities: extern "C" fn(*mut BaseStation) -> RadiusFacilities,
    pub modified_catchment: extern "C" fn() -> bool,
    pub spread: extern "C" fn() -> u8,
    pub industry_by_id: extern "C" fn(IndustryID) -> *mut ShellHandle,
    pub industry_id: extern "C" fn(*mut ShellHandle) -> IndustryID,
    pub industry_owner: extern "C" fn(*mut ShellHandle) -> *mut crate::industry::Industry,
    pub industry_neutral: extern "C" fn(*mut ShellHandle) -> *mut BaseStation,
    pub next_industry: extern "C" fn(u32) -> *mut ShellHandle,
}

/// Immutable tables and map dimensions captured once per synchronous entry.
pub struct Ctx<'a> {
    pub c: &'a CatchmentServices,
    pub s: &'a Services,
    pub map: Map,
    size_y: u32,
}
impl<'a> Ctx<'a> {
    /// Capture the tables and dimensions of one entry.
    /// # Safety
    /// `c` and its nested tables are the live immutable native tables.
    #[must_use]
    pub unsafe fn new(c: *const CatchmentServices) -> Self {
        let c = unsafe { &*c };
        let s = unsafe { &*c.shared };
        Self {
            c,
            s,
            map: s.map(),
            size_y: (s.map_max_y)().wrapping_add(1),
        }
    }
    /// The catchment table reachable from the shared services.
    #[must_use]
    pub fn from_shared(s: &'a Services) -> Self {
        // SAFETY: The shared table returns the live immutable catchment table.
        unsafe { Self::new((s.station_catchment)()) }
    }
    /// A native assertion.
    pub fn check(&self, ok: bool) {
        crate::movement::native_assert(self.s, ok);
    }
    /// `Town::GetByTile` (`ClosestTownFromTile`'s town index form).
    fn town_by_tile(&self, tile: TileIndex) -> *mut crate::town::State {
        (self.s.town_owner_by_id)(u32::from(self.town_index(tile).0))
    }
    /// `GetTownIndex`.
    fn town_index(&self, tile: TileIndex) -> TownID {
        TownID((self.s.town_index)(tile.0))
    }
    /// `GetIndustryIndex`.
    fn industry_index(&self, tile: TileIndex) -> IndustryID {
        // SAFETY: Live immutable table.
        IndustryID(unsafe { ((*self.c.industry_tile).industry_index)(tile.0) })
    }
    /// `GetStationIndex`.
    fn station_index(&self, tile: TileIndex) -> StationID {
        StationID((self.s.station_index)(tile.0))
    }
    fn industry_stations(&self, industry: *mut ShellHandle) -> *mut StationList {
        crate::industry::stations_near((self.c.industry_owner)(industry))
    }
}

/// `GetTileCatchmentRadius`: get the catchment size of an individual station
/// tile.
fn tile_catchment_radius(tile: TileIndex, st: *mut BaseStation, c: &Ctx<'_>) -> u32 {
    c.check(TileType((c.s.tile_type)(tile.0)) == TileType::MP_STATION);
    let kind = (c.s.station_type)(tile.0);
    if (c.c.modified_catchment)() {
        match kind {
            StationType::Rail => CA_TRAIN,
            StationType::Oilrig => CA_UNMODIFIED,
            StationType::Airport => airport_catchment(st, c),
            StationType::Truck => CA_TRUCK,
            StationType::Bus => CA_BUS,
            StationType::Dock => CA_DOCK,
            StationType::Buoy | StationType::RailWaypoint | StationType::RoadWaypoint => CA_NONE,
            _ => unreachable!("NOT_REACHED"),
        }
    } else {
        match kind {
            StationType::Buoy | StationType::RailWaypoint | StationType::RoadWaypoint => CA_NONE,
            _ => CA_UNMODIFIED,
        }
    }
}
/// `airport.GetSpec()->catchment`.
fn airport_catchment(st: *mut BaseStation, c: &Ctx<'_>) -> u32 {
    // SAFETY: Live immutable airport table.
    let airport = unsafe { &*c.c.airport };
    u32::from((airport.catchment)(crate::newgrf_airport::station_spec(
        st, airport,
    )))
}
/// `Station::GetCatchmentRadius`: determines the catchment radius of the
/// station.
pub(crate) fn catchment_radius(st: *mut BaseStation, c: &Ctx<'_>) -> u32 {
    let f = (c.c.radius_facilities)(st);
    let mut ret = CA_NONE;
    if (c.c.modified_catchment)() {
        if f.bus {
            ret = ret.max(CA_BUS);
        }
        if f.truck {
            ret = ret.max(CA_TRUCK);
        }
        if f.train != INVALID_TILE {
            ret = ret.max(CA_TRAIN);
        }
        if f.ship != INVALID_TILE {
            ret = ret.max(CA_DOCK);
        }
        if f.airport != INVALID_TILE {
            ret = ret.max(airport_catchment(st, c));
        }
    } else if f.bus
        || f.truck
        || f.train != INVALID_TILE
        || f.ship != INVALID_TILE
        || f.airport != INVALID_TILE
    {
        ret = CA_UNMODIFIED;
    }
    ret
}
/// `Station::GetCatchmentRect`: determines the catchment rectangle of the
/// station, clamped to the map.
#[allow(clippy::cast_possible_wrap)]
fn catchment_rect(st: *mut BaseStation, state: *mut Service, c: &Ctx<'_>) -> StationRect {
    // SAFETY: Live owner; immediate copy.
    let r = unsafe { *rect(state) };
    c.check(!r.is_empty());
    let radius = catchment_radius(st, c) as i32;
    /* Compute acceptance rectangle */
    StationRect {
        left: (r.left - radius).max(0),
        top: (r.top - radius).max(0),
        right: (r.right + radius).min(c.map.max_x() as i32),
        bottom: (r.bottom + radius).min(Map::max_y(c.size_y) as i32),
    }
}
/// `Station::AddIndustryToDeliver`: an existing entry only moves closer; a new
/// industry must accept some cargo.
pub(crate) fn add_industry_to_deliver(
    st: *mut BaseStation,
    state: *mut Service,
    ind: *mut ShellHandle,
    tile: TileIndex,
    c: &Ctx<'_>,
) {
    /* Using DistanceMax to get about the same order as with previously used SpiralTileSequence. */
    let distance = c.map.distance_max((c.c.location)(st), tile.0);
    // SAFETY: Live owner; the list borrow ends before the industry services.
    unsafe {
        let list = &mut (*catchment(state)).industries_near;
        /* Don't check further if this industry is already in the list but update the distance if it's closer */
        if let Some(pos) = list.position(ind) {
            if list.0[pos].distance > distance {
                let mut entry = list.0.remove(pos);
                entry.distance = distance;
                list.insert(entry);
            }
            return;
        }
    }
    /* Include only industries that can accept cargo */
    if !crate::industry::is_cargo_accepted((c.c.industry_owner)(ind)) {
        return;
    }
    let id = (c.c.industry_id)(ind);
    // SAFETY: Live owner; immediate mutation.
    unsafe {
        (*catchment(state))
            .industries_near
            .insert(IndustryListEntry {
                distance,
                industry: ind,
                id,
            });
    }
}
/// `Station::RemoveIndustryToDeliver`.
/// # Safety
/// `state` is a live station owner.
pub(crate) unsafe fn remove_industry_to_deliver(state: *mut Service, ind: *mut ShellHandle) {
    unsafe {
        let list = &mut (*catchment(state)).industries_near;
        if let Some(pos) = list.position(ind) {
            list.0.remove(pos);
        }
    }
}
/// A neutral station's delivery list holds only its industry, at distance zero.
/// # Safety
/// `state` is a live station owner.
pub(crate) unsafe fn set_neutral_industry(
    state: *mut Service,
    industry: *mut ShellHandle,
    id: IndustryID,
) {
    unsafe {
        let list = &mut (*catchment(state)).industries_near;
        list.clear();
        list.insert(IndustryListEntry {
            distance: 0,
            industry,
            id,
        });
    }
}
/// `FlatSet::insert` into a local sorted vector.
fn flat_insert<T: Ord>(set: &mut Vec<T>, id: T) {
    if let Err(at) = set.binary_search(&id) {
        set.insert(at, id);
    }
}
/// `Station::RemoveFromAllNearbyLists`: inherited rectangular iteration over the
/// bitmap bounds, including tiles outside the catchment.
pub(crate) fn remove_from_all_nearby_lists(st: *mut BaseStation, state: *mut Service, c: &Ctx<'_>) {
    let mut towns = Vec::new();
    let mut industries = Vec::new();
    // SAFETY: Live owner; immediate geometry copy.
    let area = unsafe { (*catchment(state)).tiles.area };
    for tile in area.tiles(c.map).map(TileIndex) {
        let kind = TileType((c.s.tile_type)(tile.0));
        if kind == TileType::MP_HOUSE {
            flat_insert(&mut towns, c.town_index(tile));
        } else if kind == TileType::MP_INDUSTRY {
            flat_insert(&mut industries, c.industry_index(tile));
        }
    }
    let id = (c.c.id)(st);
    for town in towns {
        let owner = (c.s.town_owner_by_id)(u32::from(town.0));
        // SAFETY: Live town owner; immediate mutation.
        unsafe { (*crate::town::stations_near(owner)).erase(id) };
    }
    for industry in industries {
        let list = c.industry_stations((c.c.industry_by_id)(industry));
        // SAFETY: Live industry owner; immediate mutation.
        unsafe { (*list).erase(id) };
    }
}
/// `Station::CatchmentCoversTown`: explicit bitmap iteration.
/// # Safety
/// `state` is a live station owner.
pub(crate) unsafe fn catchment_covers_town(state: *mut Service, t: TownID, c: &Ctx<'_>) -> bool {
    let bitmap = unsafe { &raw const (*catchment(state)).tiles };
    let check = |ok| c.check(ok);
    let mut it = unsafe { BitmapTileIterator::new(bitmap, c.map, check) };
    while TileIndex(it.tile()) != TileIndex::INVALID {
        let tile = it.tile();
        if TileType((c.s.tile_type)(tile)) == TileType::MP_HOUSE
            && TownID((c.s.town_index)(tile)) == t
        {
            return true;
        }
        unsafe { it.advance(bitmap, c.map, check) };
    }
    false
}
/// `Station::RecomputeCatchment`.
pub(crate) fn recompute_catchment(st: *mut BaseStation, no_clear_nearby_lists: bool, c: &Ctx<'_>) {
    let state = (c.c.owner)(st);
    // SAFETY: Live owner; immediate mutation.
    unsafe { (*catchment(state)).industries_near.clear() };
    if !no_clear_nearby_lists {
        remove_from_all_nearby_lists(st, state, c);
    }
    // SAFETY: Live owner; immediate access.
    if unsafe { rect_is_empty(state) } {
        unsafe { (*catchment(state)).tiles.reset() };
        return;
    }
    let industry = (c.c.associated_industry)(st);
    if !(c.s.station_serve_neutral)() && !industry.is_null() {
        // The station is associated with an industry, so it only delivers there.
        let location = crate::industry::location((c.c.industry_owner)(industry));
        // SAFETY: Live owner; immediate mutation.
        unsafe { (*catchment(state)).tiles.initialize(location) };
        let industry_id = (c.c.industry_id)(industry);
        for tile in location.tiles(c.map).map(TileIndex) {
            if TileType((c.s.tile_type)(tile.0)) == TileType::MP_INDUSTRY
                && c.industry_index(tile) == industry_id
            {
                // SAFETY: Live owner; immediate mutation.
                unsafe {
                    (*catchment(state))
                        .tiles
                        .set_tile(tile.0, c.map, |ok| c.check(ok));
                };
            }
        }
        // The industry's nearby set may predate its neutral station: clear and re-add.
        let list = c.industry_stations(industry);
        // SAFETY: Live industry owner; visits mutate only other stations' lists.
        unsafe {
            for_each_station(list, |near| {
                remove_industry_to_deliver((c.c.owner)(near.station), industry);
            });
            (*list).clear();
            (*list).insert(st, (c.c.id)(st));
            (*catchment(state))
                .industries_near
                .insert(IndustryListEntry {
                    distance: 0,
                    industry,
                    id: industry_id,
                });
        }
        return;
    }
    let r = catchment_rect(st, state, c);
    // SAFETY: Live owner; immediate mutation.
    unsafe {
        (*catchment(state))
            .tiles
            .initialize_rect(r.left, r.top, r.right, r.bottom, c.map);
    }
    // Loop finding all station tiles.
    // SAFETY: Live owner; immediate copy.
    let bounds = unsafe { *rect(state) };
    #[allow(clippy::cast_sign_loss)]
    let ta = TileArea::between(
        c.map,
        c.map.tile_xy(bounds.left as u32, bounds.top as u32),
        c.map.tile_xy(bounds.right as u32, bounds.bottom as u32),
    );
    let id = (c.c.id)(st);
    for tile in ta.tiles(c.map).map(TileIndex) {
        if TileType((c.s.tile_type)(tile.0)) != TileType::MP_STATION || c.station_index(tile) != id
        {
            continue;
        }
        let radius = tile_catchment_radius(tile, st, c);
        if radius == CA_NONE {
            continue;
        }
        // No tile tests are needed in this sub-loop: every tile is simply
        // added to the catchment set.
        let mut ta2 = TileArea {
            tile: tile.0,
            w: 1,
            h: 1,
        };
        #[allow(clippy::cast_possible_wrap)]
        ta2.expand(radius as i32, c.map, c.size_y);
        for tile2 in ta2.tiles(c.map) {
            // SAFETY: Live owner; immediate mutation.
            unsafe {
                (*catchment(state))
                    .tiles
                    .set_tile(tile2, c.map, |ok| c.check(ok));
            };
        }
    }
    // Search catchment tiles for towns and industries.
    // SAFETY: Live owner; the cursor re-reads the bitmap at each step.
    let bitmap = unsafe { &raw const (*catchment(state)).tiles };
    let check = |ok| c.check(ok);
    let mut it = unsafe { BitmapTileIterator::new(bitmap, c.map, check) };
    while TileIndex(it.tile()) != TileIndex::INVALID {
        let tile = TileIndex(it.tile());
        let kind = TileType((c.s.tile_type)(tile.0));
        if kind == TileType::MP_HOUSE {
            // SAFETY: Live town owner; immediate mutation.
            unsafe { (*crate::town::stations_near(c.town_by_tile(tile))).insert(st, id) };
        }
        if kind == TileType::MP_INDUSTRY {
            let i = (c.c.industry_by_id)(c.industry_index(tile));
            // An industry with a neutral station cannot be served by this station.
            if (c.s.station_serve_neutral)() || (c.c.industry_neutral)(i).is_null() {
                // SAFETY: Live industry owner; immediate mutation.
                unsafe { (*c.industry_stations(i)).insert(st, id) };
                /* Add if we can deliver to this industry as well */
                add_industry_to_deliver(st, state, i, tile, c);
            }
        }
        unsafe { it.advance(bitmap, c.map, check) };
    }
}
/// `Station::RecomputeCatchmentForAll`: towns, then industries, then stations in pool order.
pub(crate) fn recompute_catchment_for_all(c: &Ctx<'_>) {
    // The fork's town iteration: the next live index at or after the
    // argument, or `u32::MAX` past the last town.
    let mut town = (c.s.town_next_id)(u32::from(TownID::BEGIN.0));
    while town != u32::MAX {
        // SAFETY: Live town owner; immediate mutation.
        unsafe { (*crate::town::stations_near((c.s.town_owner_by_id)(town))).clear() };
        town = (c.s.town_next_id)(town + 1);
    }
    let mut industry = (c.c.next_industry)(u32::from(IndustryID::BEGIN.0));
    while !industry.is_null() {
        // SAFETY: Live industry owner; immediate mutation.
        unsafe { (*c.industry_stations(industry)).clear() };
        industry = (c.c.next_industry)(u32::from((c.c.industry_id)(industry).0) + 1);
    }
    let mut st = (c.c.next_station)(u32::from(StationID::BEGIN.0));
    while !st.is_null() {
        recompute_catchment(st, true, c);
        st = (c.c.next_station)(u32::from((c.c.id)(st).0) + 1);
    }
}

/// Raw adapter for `industry/construction.rs` and `town.rs`, which open PRs
/// hold; delegates to `for_all_stations_around_tiles_typed`.
pub(crate) fn for_all_stations_around_tiles(
    ta: TileArea,
    c: &Ctx<'_>,
    mut func: impl FnMut(StationListEntry, u32) -> bool,
) {
    for_all_stations_around_tiles_typed(ta, c, |st, tile| func(st, tile.0));
}
/// `ForAllStationsAroundTiles`: visit stations, in ID order, whose catchment
/// covers part of `ta`; `func` (a `Station*, TileIndex` callback) returning
/// `false` keeps testing the area's remaining tiles for that station. Takes the
/// plain name once its held callers adopt it.
pub(crate) fn for_all_stations_around_tiles_typed(
    ta: TileArea,
    c: &Ctx<'_>,
    mut func: impl FnMut(StationListEntry, TileIndex) -> bool,
) {
    /* There are no stations, so we will never find anything. */
    if (c.c.station_count)() == 0 {
        return;
    }
    /* Not using, or don't have a nearby stations list, so we need to scan. */
    let mut seen = Vec::new();
    /* Scan an area around the building covering the maximum possible station
     * to find the possible nearby stations. */
    let max_c = if (c.c.modified_catchment)() {
        u32::from(MAX_CATCHMENT)
    } else {
        CA_UNMODIFIED
    };
    let mut ext = ta;
    #[allow(clippy::cast_possible_wrap)]
    ext.expand(max_c as i32, c.map, c.size_y);
    for tile in ext.tiles(c.map).map(TileIndex) {
        if TileType((c.s.tile_type)(tile.0)) != TileType::MP_STATION {
            continue;
        }
        flat_insert(&mut seen, c.station_index(tile));
    }
    for id in seen {
        let st = (c.c.station_if_valid)(id);
        if st.is_null() {
            continue; // Waypoint.
        }
        /* Check if station is attached to an industry */
        if !(c.s.station_serve_neutral)() && !(c.c.associated_industry)(st).is_null() {
            continue;
        }
        let state = (c.c.owner)(st);
        for tile in ta.tiles(c.map).map(TileIndex) {
            /* Test if the tile is within the station's catchment */
            // SAFETY: Live owner; the membership borrow ends before `func`.
            if unsafe { tile_is_in_catchment(state, tile, c) }
                && func(StationListEntry { station: st, id }, tile)
            {
                break;
            }
        }
    }
}

/// `StationFinder`: a lazily computed station list for an area.
pub struct StationFinder {
    area: TileArea,
    stations: StationList,
}
impl StationFinder {
    /// `StationFinder(const TileArea &)`.
    #[must_use]
    pub fn new(area: TileArea) -> Self {
        Self {
            area,
            stations: StationList::default(),
        }
    }
    /// `GetStations`: computed on first use, then the area tile is invalidated.
    pub fn get_stations(&mut self, c: &Ctx<'_>) -> &StationList {
        if self.area.tile != INVALID_TILE {
            let tile = TileIndex(self.area.tile);
            if TileType((c.s.tile_type)(tile.0)) == TileType::MP_HOUSE {
                // Only one tile is supported: town nearby stations are
                // filtered per tile.
                c.check(self.area.w == 1 && self.area.h == 1);
                let nearby = crate::town::stations_near(c.town_by_tile(tile));
                add_nearby_stations_by_catchment(tile, &mut self.stations, nearby, c);
            } else {
                let stations = &mut self.stations;
                for_all_stations_around_tiles_typed(self.area, c, |st, _| {
                    stations.insert(st.station, st.id);
                    true
                });
            }
            self.area.tile = INVALID_TILE;
        }
        &self.stations
    }
}
/// `AddNearbyStationsByCatchment`: add the stations in `nearby` whose
/// catchment covers `tile`.
fn add_nearby_stations_by_catchment(
    tile: TileIndex,
    stations: &mut StationList,
    nearby: *const StationList,
    c: &Ctx<'_>,
) {
    // SAFETY: Live town owner; the membership reads do not mutate the set.
    unsafe {
        for_each_station(nearby, |st| {
            if tile_is_in_catchment((c.c.owner)(st.station), tile, c) {
                stations.insert(st.station, st.id);
            }
        });
    }
}

/// `GetProductionAroundTiles`: get the cargo types being produced around the
/// tile (in a rectangle).
pub(crate) fn production_around_tiles(
    north_tile: TileIndex,
    w: i32,
    h: i32,
    rad: i32,
    c: &Ctx<'_>,
) -> CargoArray {
    let mut produced = [0_u32; NUM_CARGO as usize];
    let mut industries = Vec::new();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let mut ta = TileArea {
        tile: north_tile.0,
        w: w as u16,
        h: h as u16,
    };
    ta.expand(rad, c.map, c.size_y);
    // Loop over all tiles to get the produced cargo of
    // everything except industries.
    for tile in ta.tiles(c.map).map(TileIndex) {
        if TileType((c.s.tile_type)(tile.0)) == TileType::MP_INDUSTRY {
            flat_insert(&mut industries, c.industry_index(tile));
        }
        crate::tile_cmd::add_produced_cargo(tile.0, &mut produced, c);
    }
    // Loop over the seen industries. They produce cargo for
    // anything that is within 'rad' of any one of their tiles.
    for industry in industries {
        let i = (c.c.industry_by_id)(industry);
        // Skip industry with neutral station.
        if !(c.c.industry_neutral)(i).is_null() && !(c.s.station_serve_neutral)() {
            continue;
        }
        crate::industry::for_each_produced_cargo((c.c.industry_owner)(i), |cargo| {
            if crate::cargo_type::is_valid_cargo_type(cargo) {
                let slot = &mut produced[usize::from(cargo)];
                *slot = slot.wrapping_add(1);
            }
        });
    }
    produced
}
/// `GetAcceptanceAroundTiles`: get the acceptance of cargoes around the tile
/// in 1/8.
pub(crate) fn acceptance_around_tiles(
    center_tile: TileIndex,
    w: i32,
    h: i32,
    rad: i32,
    always_accepted: *mut CargoTypes,
    c: &Ctx<'_>,
) -> CargoArray {
    let mut acceptance = [0_u32; NUM_CARGO as usize];
    // SAFETY: Null or the caller's writable mask.
    if let Some(always) = unsafe { always_accepted.as_mut() } {
        *always = CargoTypes(0);
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let mut ta = TileArea {
        tile: center_tile.0,
        w: w as u16,
        h: h as u16,
    };
    ta.expand(rad, c.map, c.size_y);
    for tile in ta.tiles(c.map).map(TileIndex) {
        // Ignore industry if it has a neutral station.
        if !(c.s.station_serve_neutral)()
            && TileType((c.s.tile_type)(tile.0)) == TileType::MP_INDUSTRY
            && !(c.c.industry_neutral)((c.c.industry_by_id)(c.industry_index(tile))).is_null()
        {
            continue;
        }
        crate::tile_cmd::add_accepted_cargo(tile.0, &mut acceptance, always_accepted.cast(), c);
    }
    acceptance
}
/// Raw adapter for `station_service.rs`, reserved by the #737 V remainder;
/// delegates to `acceptance_around_station_typed`.
/// # Safety
/// As `acceptance_around_station_typed`.
pub(crate) unsafe fn acceptance_around_station(
    state: *mut Service,
    always_accepted: *mut u64,
    c: &Ctx<'_>,
) -> [u32; NUM_CARGO as usize] {
    unsafe { acceptance_around_station_typed(state, always_accepted.cast(), c) }
}
/// `GetAcceptanceAroundStation`: explicit bitmap iteration of the catchment. Takes
/// the plain name once `station_service.rs` adopts it.
/// # Safety
/// `state` is a live station owner whose catchment is not reset by the tile procs.
pub(crate) unsafe fn acceptance_around_station_typed(
    state: *mut Service,
    always_accepted: *mut CargoTypes,
    c: &Ctx<'_>,
) -> [u32; NUM_CARGO as usize] {
    let mut acceptance = [0_u32; NUM_CARGO as usize];
    // SAFETY: Null or the caller's writable mask.
    if let Some(always) = unsafe { always_accepted.as_mut() } {
        *always = CargoTypes(0);
    }
    let bitmap = unsafe { &raw const (*catchment(state)).tiles };
    let check = |ok| c.check(ok);
    let mut it = unsafe { BitmapTileIterator::new(bitmap, c.map, check) };
    while TileIndex(it.tile()) != TileIndex::INVALID {
        crate::tile_cmd::add_accepted_cargo(it.tile(), &mut acceptance, always_accepted.cast(), c);
        unsafe { it.advance(bitmap, c.map, check) };
    }
    acceptance
}
/// Visit each tile of a live catchment by explicit bitmap iteration.
/// # Safety
/// `state` is a live station owner whose catchment the visits do not reset.
pub(crate) unsafe fn iterate_catchment_tiles(
    state: *mut Service,
    c: &Ctx<'_>,
    mut visit: impl FnMut(TileIndex),
) {
    let bitmap = unsafe { &raw const (*catchment(state)).tiles };
    let check = |ok| c.check(ok);
    let mut it = unsafe { BitmapTileIterator::new(bitmap, c.map, check) };
    while TileIndex(it.tile()) != TileIndex::INVALID {
        visit(TileIndex(it.tile()));
        unsafe { it.advance(bitmap, c.map, check) };
    }
}
/// Raw adapter for `subsidy.rs` and `station_service.rs`, outside this pass;
/// delegates to `iterate_catchment_tiles`.
/// # Safety
/// As `iterate_catchment_tiles`.
pub(crate) unsafe fn for_each_catchment_tile(
    state: *mut Service,
    c: &Ctx<'_>,
    mut visit: impl FnMut(u32),
) {
    unsafe { iterate_catchment_tiles(state, c, |tile| visit(tile.0)) }
}

/// `MoveGoodsToStation` with this entry's station and visit services.
/// # Safety
/// `all_stations` is live for the call.
pub(crate) unsafe fn move_goods_to_station(
    cargo: CargoType,
    amount: u32,
    source: crate::cargo_storage::Source,
    all_stations: *const StationList,
    exclusivity: CompanyID,
    c: &Ctx<'_>,
) -> u32 {
    // SAFETY: Live immutable tables; caller contract for the set.
    unsafe {
        crate::station_service::distribute(
            cargo.0,
            amount,
            source,
            all_stations,
            exclusivity.0,
            c.c.station,
            &*c.c.visits,
        )
    }
}
/// Raw adapter for `objects.rs`, outside this pass; delegates to `move_goods_to_station`.
/// # Safety
/// As `move_goods_to_station`.
pub(crate) unsafe fn move_goods_to_station_raw(
    cargo: u8,
    amount: u32,
    source: crate::cargo_storage::Source,
    all_stations: *const StationList,
    exclusivity: u8,
    c: &Ctx<'_>,
) -> u32 {
    unsafe {
        move_goods_to_station(
            CargoType(cargo),
            amount,
            source,
            all_stations,
            CompanyID(exclusivity),
            c,
        )
    }
}

/// Native list view: the ordered members at the time of the call.
#[repr(C)]
pub struct ListView<T> {
    pub data: *const T,
    pub size: usize,
}
/// Borrowed view of a live `industries_near`; valid until the list next changes.
/// # Safety
/// `state` is a live station owner.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_industries_near(
    state: *mut Service,
) -> ListView<IndustryListEntry> {
    let entries = unsafe { (*catchment(state)).industries_near.entries() };
    ListView {
        data: entries.as_ptr(),
        size: entries.len(),
    }
}
/// The canonical rectangle aliased by `BaseStation::rect`.
/// # Safety
/// `state` is live; the alias ends with the owner.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_rect(state: *mut Service) -> *mut StationRect {
    unsafe { rect(state) }
}
/// The canonical bitmap aliased by `Station::catchment_tiles`.
/// # Safety
/// `state` is live; the alias ends with the owner.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_catchment_tiles(
    state: *mut Service,
) -> *mut BitmapTileArea {
    unsafe { &raw mut (*catchment(state)).tiles }
}
/// Native `StationRect::IsEmpty`.
/// # Safety
/// `r` is a live rectangle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_rect_empty_abi(r: *const StationRect) -> u8 {
    u8::from(unsafe { (*r).is_empty() })
}
/// Native `StationRect::PtInExtendedRect`.
/// # Safety
/// `r` is a live rectangle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_rect_pt_in_extended_rect_abi(
    r: *const StationRect,
    x: i32,
    y: i32,
    distance: i32,
) -> u8 {
    u8::from(unsafe { (*r).pt_in_extended_rect(x, y, distance) })
}
/// Native `StationRect::BeforeAddTile`; `false` is the too-spread-out error.
/// # Safety
/// `r` is a live rectangle; `c` is the live catchment table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_rect_before_add_tile_abi(
    r: *mut StationRect,
    tile: u32,
    mode: u8,
    c: *const CatchmentServices,
) -> u8 {
    let c = unsafe { Ctx::new(c) };
    let mut copy = unsafe { *r };
    let ok = copy.before_add_tile(TileIndex(tile), StationRectMode(mode), &c);
    unsafe { *r = copy };
    u8::from(ok)
}
/// Native `StationRect::BeforeAddRect`; `false` is the too-spread-out error.
/// # Safety
/// `r` is a live rectangle; `c` is the live catchment table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_rect_before_add_rect_abi(
    r: *mut StationRect,
    tile: u32,
    w: i32,
    h: i32,
    mode: u8,
    c: *const CatchmentServices,
) -> u8 {
    let c = unsafe { Ctx::new(c) };
    let mut copy = unsafe { *r };
    let ok = copy.before_add_rect(TileIndex(tile), w, h, StationRectMode(mode), &c);
    unsafe { *r = copy };
    u8::from(ok)
}
/// Native `Station::RecomputeCatchmentForAll`.
/// # Safety
/// `c` is the live catchment table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_recompute_catchment_for_all(
    c: *const CatchmentServices,
) {
    recompute_catchment_for_all(&unsafe { Ctx::new(c) });
}
/// Native `Station::RemoveFromAllNearbyLists`, from the station destructor.
/// # Safety
/// `st` is a live station; `c` is the live catchment table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_remove_from_nearby_lists(
    st: *mut BaseStation,
    c: *const CatchmentServices,
) {
    let c = unsafe { Ctx::new(c) };
    remove_from_all_nearby_lists(st, (c.c.owner)(st), &c);
}
/// Native `Station::GetCatchmentRadius`.
/// # Safety
/// `st` is a live station; `c` is the live catchment table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_catchment_radius(
    st: *mut BaseStation,
    c: *const CatchmentServices,
) -> u32 {
    catchment_radius(st, &unsafe { Ctx::new(c) })
}
/// Native `GetProductionAroundTiles`, writing the caller's `CargoArray`.
/// # Safety
/// `out` names `NUM_CARGO` writable values; `c` is the live catchment table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_production_around(
    tile: u32,
    w: i32,
    h: i32,
    rad: i32,
    out: *mut [u32; NUM_CARGO as usize],
    c: *const CatchmentServices,
) {
    let produced = production_around_tiles(TileIndex(tile), w, h, rad, &unsafe { Ctx::new(c) });
    unsafe { *out = produced };
}
/// Native `GetAcceptanceAroundTiles`, writing the caller's `CargoArray`.
/// # Safety
/// `out` names `NUM_CARGO` writable values, `always` is null or writable; `c` is the live table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_acceptance_around(
    tile: u32,
    w: i32,
    h: i32,
    rad: i32,
    out: *mut [u32; NUM_CARGO as usize],
    always: *mut u64,
    c: *const CatchmentServices,
) {
    let acceptance = acceptance_around_tiles(TileIndex(tile), w, h, rad, always.cast(), &unsafe {
        Ctx::new(c)
    });
    unsafe { *out = acceptance };
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDStationRect" ["station_service"] => StationRect { left, top, right, bottom };
    "OpenTTDIndustryListEntry" ["station_service"] => IndustryListEntry { distance, industry, id };
    "OpenTTDStationRadiusFacilities" ["station_service"] => RadiusFacilities { bus, truck, train, ship, airport };
    "OpenTTDStationCatchmentServices" ["station_service"] => CatchmentServices {
        shared, airport, industry_tile, object, town, station, visits, station_count, station_if_valid,
        next_station, owner, station_owner, id, location, associated_industry, radius_facilities,
        modified_catchment, spread, industry_by_id, industry_id, industry_owner,
        industry_neutral, next_industry,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn station_list_keeps_id_order_and_erase_safe_traversal() {
        let mut list = StationList::default();
        let p = |n: usize| std::ptr::without_provenance_mut::<BaseStation>(n);
        for id in [7, 3, 9, 3] {
            list.insert(p(usize::from(id)), StationID(id));
        }
        assert_eq!(
            list.entries().iter().map(|e| e.id.0).collect::<Vec<_>>(),
            [3, 7, 9]
        );
        let mut seen = Vec::new();
        let raw = &raw mut list;
        unsafe {
            for_each_station(raw, |e| {
                seen.push(e.id.0);
                (*raw).erase(e.id);
            });
        }
        assert_eq!(seen, [3, 7, 9]);
        assert!(list.entries().is_empty());
    }

    #[test]
    fn station_list_reacquires_after_membership_changes() {
        let mut list = StationList::default();
        let p = |n: usize| std::ptr::without_provenance_mut::<BaseStation>(n);
        for id in [3, 7, 9] {
            list.insert(p(usize::from(id)), StationID(id));
        }
        let mut seen = Vec::new();
        let raw = &raw mut list;
        // SAFETY: The list stays live; each copied row's borrow ends before mutation.
        unsafe {
            for_each_station(raw, |entry| {
                seen.push(entry.id.0);
                if entry.id == StationID(3) {
                    (*raw).erase(entry.id);
                    (*raw).insert(p(33), entry.id);
                    (*raw).insert(p(1), StationID(1));
                    (*raw).insert(p(5), StationID(5));
                    (*raw).erase(StationID(9));
                } else if entry.id == StationID(7) {
                    (*raw).insert(p(11), StationID(11));
                }
            });
        }
        // Reuse and insertion behind the ID cursor are skipped; later keys are live.
        assert_eq!(seen, [3, 5, 7, 11]);
        assert_eq!(
            list.entries()
                .iter()
                .map(|entry| entry.id.0)
                .collect::<Vec<_>>(),
            [1, 3, 5, 7, 11]
        );
    }

    #[test]
    fn industry_list_orders_by_distance_then_id_and_deduplicates() {
        let mut list = IndustryList::default();
        let p = |n: usize| std::ptr::without_provenance_mut::<ShellHandle>(n);
        for (distance, id) in [(4, 2), (1, 9), (4, 1), (4, 1)] {
            list.insert(IndustryListEntry {
                distance,
                industry: p(usize::from(id)),
                id: IndustryID(id),
            });
        }
        let order: Vec<_> = list
            .entries()
            .iter()
            .map(|e| (e.distance, e.id.0))
            .collect();
        assert_eq!(order, [(1, 9), (4, 1), (4, 2)]);
        assert_eq!(list.position(p(2)), Some(2));
    }

    #[test]
    fn empty_rectangle_rules_follow_the_original() {
        let r = StationRect::default();
        assert!(r.is_empty());
        // A zero left edge is empty even with a valid extent.
        assert!(
            StationRect {
                left: 0,
                top: 1,
                right: 3,
                bottom: 4
            }
            .is_empty()
        );
        assert!(
            !StationRect {
                left: 1,
                top: 1,
                right: 1,
                bottom: 1
            }
            .is_empty()
        );
        assert!(
            StationRect {
                left: 2,
                top: 1,
                right: 1,
                bottom: 1
            }
            .is_empty()
        );
    }
}
