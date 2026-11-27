/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! The rail and road type registry: the globals and bodies over `_railtypes` and `_roadtypes`
//! of `rail_cmd.cpp` and `road_cmd.cpp`, the lookups by label of `rail.cpp` and `road.cpp`, the
//! translations and saved label lists of `newgrf_railtype.cpp` and `newgrf_roadtype.cpp`, and
//! the list growth of the `RAIL` and `ROTT` chunks' `Load` (`saveload/labelmaps_sl.cpp`).
//!
//! `RailTypeInfo` and `RoadTypeInfo` stay C++ classes in `_railtypes` and `_roadtypes`. A body
//! resolves an element once with its table's `spec` (`&_railtypes[rt]`), reads it through
//! [`RailTypeInfoRef`] or [`RoadTypeInfoRef`] and writes its members through its table's
//! stores. The files stay in C++'s `_grf_files` and are read through [`GRFFileRef`].
//!
//! The registry's state is process-wide. The types are reset and rebuilt only by `NewGRF`
//! loading, and the label lists written only by world generation, saving and loading, all of
//! which run outside the game loop and command execution. C++ reads the four masks in place
//! through the references it binds, and the sorted lists through the spans this module
//! publishes after each change. No borrow of the state lasts across a service call.
use crate::container_func::find_index;
use crate::engine_type::VehicleAccelerationModel;
use crate::game_state::GameState;
use crate::map_func::{Map, World};
use crate::newgrf::{GRFFile, GRFFileRef};
use crate::newgrf_bytereader::BadgeList;
use crate::newgrf_commons::LabelObject;
use crate::rail::{RailTypeFlag, RailTypeFlags, RailTypeInfo, RailTypeInfoRef, get_rail_type_info};
use crate::rail_type::{RailType, RailTypeLabel, RailTypes};
use crate::road::{
    RoadTypeFlags, RoadTypeInfo, RoadTypeInfoRef, get_road_tram_type, get_road_type_info,
    road_type_is_road,
};
use crate::road_type::{RoadTramType, RoadType, RoadTypeLabel, RoadTypes};
use crate::services::{RailMapServices, Services as Shared};
use crate::tile_type::{TileIndex, TileType};
use crate::timer_game_calendar::CalendarDate;
use crate::timer_game_common::INVALID_DATE;
use crate::transport_type::TransportType;
use crate::type_availability;
use std::ffi::c_void;
use std::ptr;

type Handle = *const c_void;

/// `std::span<const RailTypeLabel>`: a `GRFFile::railtype_list`, live until its owner changes.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RailTypeLabels {
    data: *const RailTypeLabel,
    size: usize,
}

impl RailTypeLabels {
    /// `size()`.
    pub(crate) const fn len(self) -> usize {
        self.size
    }

    /// The labels, borrowed for a search with no call in between.
    #[allow(
        unsafe_code,
        reason = "the span's live labels; an empty span may carry a null pointer"
    )]
    pub(crate) fn as_slice(&self) -> &[RailTypeLabel] {
        if self.size == 0 {
            &[]
        } else {
            // SAFETY: A span holds `size` live labels until its owner changes, and the
            // returned slice borrows the span for a search that makes no call.
            unsafe { std::slice::from_raw_parts(self.data, self.size) }
        }
    }
}

/// `std::span<const RoadTypeLabel>`: a `GRFFile::roadtype_list` or `tramtype_list`, live until
/// its owner changes.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoadTypeLabels {
    data: *const RoadTypeLabel,
    size: usize,
}

impl RoadTypeLabels {
    /// `size()`.
    pub(crate) const fn len(self) -> usize {
        self.size
    }

    /// The labels, borrowed for a search with no call in between.
    #[allow(
        unsafe_code,
        reason = "the span's live labels; an empty span may carry a null pointer"
    )]
    pub(crate) fn as_slice(&self) -> &[RoadTypeLabel] {
        if self.size == 0 {
            &[]
        } else {
            // SAFETY: As for `RailTypeLabels::as_slice`.
            unsafe { std::slice::from_raw_parts(self.data, self.size) }
        }
    }
}

/// A `GRFFile::railtype_map` (`std::array<RailType, RAILTYPE_END>`): its elements, read and
/// written in place at the original's point, and its fixed size.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RailTypeMap {
    pub(crate) data: *mut RailType,
    pub(crate) size: usize,
}

impl RailTypeMap {
    /// `std::size(type_map)`.
    pub(crate) const fn size(self) -> usize {
        self.size
    }

    /// `type_map[id]`.
    ///
    /// # Safety
    /// `id` is below [`Self::size`], as the original's bound on `last` makes it.
    #[allow(unsafe_code, reason = "an element load of the file's live array")]
    pub(crate) unsafe fn at(self, id: usize) -> RailType {
        // SAFETY: The array is live while its file is (`GRFFileRef`'s contract), and the
        // caller keeps `id` in bounds.
        unsafe { self.data.add(id).read() }
    }

    /// `type_map[id] = rt`.
    ///
    /// # Safety
    /// As for [`Self::at`].
    #[allow(unsafe_code, reason = "an element store to the file's live array")]
    pub(crate) unsafe fn set(self, id: usize, rt: RailType) {
        // SAFETY: As for `at`.
        unsafe { self.data.add(id).write(rt) }
    }
}

/// A `GRFFile::roadtype_map` or `tramtype_map` (`std::array<RoadType, ROADTYPE_END>`): its
/// elements, read and written in place at the original's point, and its fixed size.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoadTypeMap {
    pub(crate) data: *mut RoadType,
    pub(crate) size: usize,
}

impl RoadTypeMap {
    /// `std::size(type_map)`.
    pub(crate) const fn size(self) -> usize {
        self.size
    }

    /// `type_map[id]`.
    ///
    /// # Safety
    /// `id` is below [`Self::size`], as the original's bound on `last` makes it.
    #[allow(unsafe_code, reason = "an element load of the file's live array")]
    pub(crate) unsafe fn at(self, id: usize) -> RoadType {
        // SAFETY: As for `RailTypeMap::at`.
        unsafe { self.data.add(id).read() }
    }

    /// `type_map[id] = rt`.
    ///
    /// # Safety
    /// As for [`Self::at`].
    #[allow(unsafe_code, reason = "an element store to the file's live array")]
    pub(crate) unsafe fn set(self, id: usize, rt: RoadType) {
        // SAFETY: As for `RailTypeMap::at`.
        unsafe { self.data.add(id).write(rt) }
    }
}

/// The rail half of the registry table: the C++ bodies over `_railtypes` and
/// `_original_railtypes`, the sorted-list span GUI code reads, and the `RailTypeInfo` members
/// Rust writes. Each element pointer is one `spec` returned.
#[repr(C)]
pub struct RailServices {
    /// `*rti = RailTypeInfo{}`.
    pub(crate) clear: extern "C" fn(*mut RailTypeInfo),
    /// `*rti = _original_railtypes[source]`.
    pub(crate) copy_default: extern "C" fn(*mut RailTypeInfo, RailType),
    /// `std::size(_original_railtypes)`.
    pub(crate) default_count: u8,
    /// `ResolveRailTypeGUISprites(rti)`, which calls the unported `GetCustomRailSprite` and
    /// `GetCustomSignalSprite`.
    pub(crate) resolve_rail_type_gui_sprites: extern "C" fn(*mut RailTypeInfo),
    /// Store the span `_sorted_railtypes`.
    pub(crate) set_sorted_railtypes: extern "C" fn(*mut RailType, usize),
    /// `std::sort(first, first + count, CompareRailTypes)`. The order of equal
    /// `sorting_order` values is the standard library's.
    pub(crate) sort: extern "C" fn(*mut RailType, usize),
    /// `&_railtypes[rt]`.
    pub(crate) spec: extern "C" fn(RailType) -> *mut RailTypeInfo,

    /// `RailTypeInfo::alternate_labels.clear()`.
    pub(crate) alternate_labels_clear: extern "C" fn(*mut RailTypeInfo),
    /// `RailTypeInfo::alternate_labels.contains(label)`.
    pub(crate) alternate_labels_contains: extern "C" fn(*const RailTypeInfo, RailTypeLabel) -> bool,
    /// `RailTypeInfo::alternate_labels.insert(label)`.
    pub(crate) alternate_labels_insert: extern "C" fn(*mut RailTypeInfo, RailTypeLabel),
    /// `&RailTypeInfo::badges`, which `ReadBadgeList`'s result is assigned to.
    pub(crate) badges: extern "C" fn(*mut RailTypeInfo) -> *mut BadgeList,
    /// `RailTypeInfo::compatible_railtypes.Set(rt)`.
    pub(crate) compatible_railtypes_set: extern "C" fn(*mut RailTypeInfo, RailType),
    /// `RailTypeInfo::introduces_railtypes.Set(rt)`.
    pub(crate) introduces_railtypes_set: extern "C" fn(*mut RailTypeInfo, RailType),
    /// `RailTypeInfo::introduction_required_railtypes.Set(rt)`.
    pub(crate) introduction_required_railtypes_set: extern "C" fn(*mut RailTypeInfo, RailType),
    /// `RailTypeInfo::powered_railtypes.Set(rt)`.
    pub(crate) powered_railtypes_set: extern "C" fn(*mut RailTypeInfo, RailType),
    /// Store `RailTypeInfo::acceleration_type`.
    pub(crate) set_acceleration_type: extern "C" fn(*mut RailTypeInfo, VehicleAccelerationModel),
    /// Store `RailTypeInfo::compatible_railtypes`.
    pub(crate) set_compatible_railtypes: extern "C" fn(*mut RailTypeInfo, RailTypes),
    /// Store `RailTypeInfo::cost_multiplier`.
    pub(crate) set_cost_multiplier: extern "C" fn(*mut RailTypeInfo, u16),
    /// Store `RailTypeInfo::curve_speed`.
    pub(crate) set_curve_speed: extern "C" fn(*mut RailTypeInfo, u8),
    /// Store `RailTypeInfo::fallback_railtype`.
    pub(crate) set_fallback_railtype: extern "C" fn(*mut RailTypeInfo, u8),
    /// Store `RailTypeInfo::flags`.
    pub(crate) set_flags: extern "C" fn(*mut RailTypeInfo, RailTypeFlags),
    /// Store `RailTypeInfo::introduces_railtypes`.
    pub(crate) set_introduces_railtypes: extern "C" fn(*mut RailTypeInfo, RailTypes),
    /// Store `RailTypeInfo::introduction_date`.
    pub(crate) set_introduction_date: extern "C" fn(*mut RailTypeInfo, CalendarDate),
    /// Store `RailTypeInfo::label`.
    pub(crate) set_label: extern "C" fn(*mut RailTypeInfo, RailTypeLabel),
    /// Store `RailTypeInfo::maintenance_multiplier`.
    pub(crate) set_maintenance_multiplier: extern "C" fn(*mut RailTypeInfo, u16),
    /// Store `RailTypeInfo::map_colour`.
    pub(crate) set_map_colour: extern "C" fn(*mut RailTypeInfo, u8),
    /// Store `RailTypeInfo::max_speed`.
    pub(crate) set_max_speed: extern "C" fn(*mut RailTypeInfo, u16),
    /// Store `RailTypeInfo::powered_railtypes`.
    pub(crate) set_powered_railtypes: extern "C" fn(*mut RailTypeInfo, RailTypes),
    /// Store `RailTypeInfo::sorting_order`.
    pub(crate) set_sorting_order: extern "C" fn(*mut RailTypeInfo, u8),
    /// `&RailTypeInfo::strings.build_caption`.
    pub(crate) strings_build_caption: extern "C" fn(*mut RailTypeInfo) -> *mut u32,
    /// `&RailTypeInfo::strings.menu_text`.
    pub(crate) strings_menu_text: extern "C" fn(*mut RailTypeInfo) -> *mut u32,
    /// `&RailTypeInfo::strings.name`.
    pub(crate) strings_name: extern "C" fn(*mut RailTypeInfo) -> *mut u32,
    /// `&RailTypeInfo::strings.new_loco`.
    pub(crate) strings_new_loco: extern "C" fn(*mut RailTypeInfo) -> *mut u32,
    /// `&RailTypeInfo::strings.replace_text`.
    pub(crate) strings_replace_text: extern "C" fn(*mut RailTypeInfo) -> *mut u32,
    /// `&RailTypeInfo::strings.toolbar_caption`.
    pub(crate) strings_toolbar_caption: extern "C" fn(*mut RailTypeInfo) -> *mut u32,
}

/// The road half of the registry table, as [`RailServices`] is for rail.
#[repr(C)]
pub struct RoadServices {
    /// `*rti = RoadTypeInfo{}`.
    pub(crate) clear: extern "C" fn(*mut RoadTypeInfo),
    /// `*rti = _original_roadtypes[source]`.
    pub(crate) copy_default: extern "C" fn(*mut RoadTypeInfo, RoadType),
    /// `std::size(_original_roadtypes)`.
    pub(crate) default_count: u8,
    /// `ResolveRoadTypeGUISprites(rti)`, which calls the unported `GetCustomRoadSprite`.
    pub(crate) resolve_road_type_gui_sprites: extern "C" fn(*mut RoadTypeInfo),
    /// Store the span `_sorted_roadtypes`.
    pub(crate) set_sorted_roadtypes: extern "C" fn(*mut RoadType, usize),
    /// `std::sort(first, first + count, CompareRoadTypes)`. The order of equal
    /// `sorting_order` values is the standard library's.
    pub(crate) sort: extern "C" fn(*mut RoadType, usize),
    /// `&_roadtypes[rt]`.
    pub(crate) spec: extern "C" fn(RoadType) -> *mut RoadTypeInfo,

    /// `RoadTypeInfo::alternate_labels.clear()`.
    pub(crate) alternate_labels_clear: extern "C" fn(*mut RoadTypeInfo),
    /// `RoadTypeInfo::alternate_labels.contains(label)`.
    pub(crate) alternate_labels_contains: extern "C" fn(*const RoadTypeInfo, RoadTypeLabel) -> bool,
    /// `RoadTypeInfo::alternate_labels.insert(label)`.
    pub(crate) alternate_labels_insert: extern "C" fn(*mut RoadTypeInfo, RoadTypeLabel),
    /// `&RoadTypeInfo::badges`, which `ReadBadgeList`'s result is assigned to.
    pub(crate) badges: extern "C" fn(*mut RoadTypeInfo) -> *mut BadgeList,
    /// `RoadTypeInfo::introduces_roadtypes.Set(rt)`.
    pub(crate) introduces_roadtypes_set: extern "C" fn(*mut RoadTypeInfo, RoadType),
    /// `RoadTypeInfo::introduction_required_roadtypes.Set(rt)`.
    pub(crate) introduction_required_roadtypes_set: extern "C" fn(*mut RoadTypeInfo, RoadType),
    /// `RoadTypeInfo::powered_roadtypes.Set(rt)`.
    pub(crate) powered_roadtypes_set: extern "C" fn(*mut RoadTypeInfo, RoadType),
    /// Store `RoadTypeInfo::cost_multiplier`.
    pub(crate) set_cost_multiplier: extern "C" fn(*mut RoadTypeInfo, u16),
    /// Store `RoadTypeInfo::flags`.
    pub(crate) set_flags: extern "C" fn(*mut RoadTypeInfo, RoadTypeFlags),
    /// Store `RoadTypeInfo::introduces_roadtypes`.
    pub(crate) set_introduces_roadtypes: extern "C" fn(*mut RoadTypeInfo, RoadTypes),
    /// Store `RoadTypeInfo::introduction_date`.
    pub(crate) set_introduction_date: extern "C" fn(*mut RoadTypeInfo, CalendarDate),
    /// Store `RoadTypeInfo::label`.
    pub(crate) set_label: extern "C" fn(*mut RoadTypeInfo, RoadTypeLabel),
    /// Store `RoadTypeInfo::maintenance_multiplier`.
    pub(crate) set_maintenance_multiplier: extern "C" fn(*mut RoadTypeInfo, u16),
    /// Store `RoadTypeInfo::map_colour`.
    pub(crate) set_map_colour: extern "C" fn(*mut RoadTypeInfo, u8),
    /// Store `RoadTypeInfo::max_speed`.
    pub(crate) set_max_speed: extern "C" fn(*mut RoadTypeInfo, u16),
    /// Store `RoadTypeInfo::powered_roadtypes`.
    pub(crate) set_powered_roadtypes: extern "C" fn(*mut RoadTypeInfo, RoadTypes),
    /// Store `RoadTypeInfo::sorting_order`.
    pub(crate) set_sorting_order: extern "C" fn(*mut RoadTypeInfo, u8),
    /// `&RoadTypeInfo::strings.build_caption`.
    pub(crate) strings_build_caption: extern "C" fn(*mut RoadTypeInfo) -> *mut u32,
    /// `&RoadTypeInfo::strings.menu_text`.
    pub(crate) strings_menu_text: extern "C" fn(*mut RoadTypeInfo) -> *mut u32,
    /// `&RoadTypeInfo::strings.name`.
    pub(crate) strings_name: extern "C" fn(*mut RoadTypeInfo) -> *mut u32,
    /// `&RoadTypeInfo::strings.new_engine`.
    pub(crate) strings_new_engine: extern "C" fn(*mut RoadTypeInfo) -> *mut u32,
    /// `&RoadTypeInfo::strings.replace_text`.
    pub(crate) strings_replace_text: extern "C" fn(*mut RoadTypeInfo) -> *mut u32,
    /// `&RoadTypeInfo::strings.toolbar_caption`.
    pub(crate) strings_toolbar_caption: extern "C" fn(*mut RoadTypeInfo) -> *mut u32,
}

/// `RailServices` with every entry unreachable, for tests.
#[cfg(test)]
pub(crate) const TEST_RAIL: RailServices = RailServices {
    clear: crate::test_tables::unreachable1,
    copy_default: crate::test_tables::unreachable2,
    default_count: 0,
    resolve_rail_type_gui_sprites: crate::test_tables::unreachable1,
    set_sorted_railtypes: crate::test_tables::unreachable2,
    sort: crate::test_tables::unreachable2,
    spec: crate::test_tables::unreachable1,
    alternate_labels_clear: crate::test_tables::unreachable1,
    alternate_labels_contains: crate::test_tables::unreachable2,
    alternate_labels_insert: crate::test_tables::unreachable2,
    badges: crate::test_tables::unreachable1,
    compatible_railtypes_set: crate::test_tables::unreachable2,
    introduces_railtypes_set: crate::test_tables::unreachable2,
    introduction_required_railtypes_set: crate::test_tables::unreachable2,
    powered_railtypes_set: crate::test_tables::unreachable2,
    set_acceleration_type: crate::test_tables::unreachable2,
    set_compatible_railtypes: crate::test_tables::unreachable2,
    set_cost_multiplier: crate::test_tables::unreachable2,
    set_curve_speed: crate::test_tables::unreachable2,
    set_fallback_railtype: crate::test_tables::unreachable2,
    set_flags: crate::test_tables::unreachable2,
    set_introduces_railtypes: crate::test_tables::unreachable2,
    set_introduction_date: crate::test_tables::unreachable2,
    set_label: crate::test_tables::unreachable2,
    set_maintenance_multiplier: crate::test_tables::unreachable2,
    set_map_colour: crate::test_tables::unreachable2,
    set_max_speed: crate::test_tables::unreachable2,
    set_powered_railtypes: crate::test_tables::unreachable2,
    set_sorting_order: crate::test_tables::unreachable2,
    strings_build_caption: crate::test_tables::unreachable1,
    strings_menu_text: crate::test_tables::unreachable1,
    strings_name: crate::test_tables::unreachable1,
    strings_new_loco: crate::test_tables::unreachable1,
    strings_replace_text: crate::test_tables::unreachable1,
    strings_toolbar_caption: crate::test_tables::unreachable1,
};

/// `RoadServices` with every entry unreachable, for tests.
#[cfg(test)]
pub(crate) const TEST_ROAD: RoadServices = RoadServices {
    clear: crate::test_tables::unreachable1,
    copy_default: crate::test_tables::unreachable2,
    default_count: 0,
    resolve_road_type_gui_sprites: crate::test_tables::unreachable1,
    set_sorted_roadtypes: crate::test_tables::unreachable2,
    sort: crate::test_tables::unreachable2,
    spec: crate::test_tables::unreachable1,
    alternate_labels_clear: crate::test_tables::unreachable1,
    alternate_labels_contains: crate::test_tables::unreachable2,
    alternate_labels_insert: crate::test_tables::unreachable2,
    badges: crate::test_tables::unreachable1,
    introduces_roadtypes_set: crate::test_tables::unreachable2,
    introduction_required_roadtypes_set: crate::test_tables::unreachable2,
    powered_roadtypes_set: crate::test_tables::unreachable2,
    set_cost_multiplier: crate::test_tables::unreachable2,
    set_flags: crate::test_tables::unreachable2,
    set_introduces_roadtypes: crate::test_tables::unreachable2,
    set_introduction_date: crate::test_tables::unreachable2,
    set_label: crate::test_tables::unreachable2,
    set_maintenance_multiplier: crate::test_tables::unreachable2,
    set_map_colour: crate::test_tables::unreachable2,
    set_max_speed: crate::test_tables::unreachable2,
    set_powered_roadtypes: crate::test_tables::unreachable2,
    set_sorting_order: crate::test_tables::unreachable2,
    strings_build_caption: crate::test_tables::unreachable1,
    strings_menu_text: crate::test_tables::unreachable1,
    strings_name: crate::test_tables::unreachable1,
    strings_new_engine: crate::test_tables::unreachable1,
    strings_replace_text: crate::test_tables::unreachable1,
    strings_toolbar_caption: crate::test_tables::unreachable1,
};

/// The registry table: its rail and road halves and the shared table.
#[repr(C)]
pub struct Services {
    /// The rail half.
    rail: *const RailServices,
    /// The road half.
    road: *const RoadServices,
    /// The shared table.
    shared: *const Shared,
}

impl Services {
    /// The rail half.
    #[allow(unsafe_code, reason = "one borrow of the process-lifetime table")]
    pub(crate) fn rail(&self) -> &RailServices {
        // SAFETY: Every table is process-lifetime.
        unsafe { &*self.rail }
    }

    /// The road half.
    #[allow(unsafe_code, reason = "one borrow of the process-lifetime table")]
    pub(crate) fn road(&self) -> &RoadServices {
        // SAFETY: Every table is process-lifetime.
        unsafe { &*self.road }
    }

    /// The shared table.
    #[allow(unsafe_code, reason = "one borrow of the process-lifetime table")]
    pub(crate) fn shared(&self) -> &Shared {
        // SAFETY: Every table is process-lifetime.
        unsafe { &*self.shared }
    }
}

/// Bitset of hidden railtypes (`_railtypes_hidden_mask`).
static RAILTYPES_HIDDEN_MASK: GameState<RailTypes> = GameState::new(RailTypes::EMPTY);
/// Bitset of hidden roadtypes (`_roadtypes_hidden_mask`).
static ROADTYPES_HIDDEN_MASK: GameState<RoadTypes> = GameState::new(RoadTypes::EMPTY);
/// Bitset of road roadtypes (`_roadtypes_road`).
static ROADTYPES_ROAD: GameState<RoadTypes> = GameState::new(RoadTypes::EMPTY);
/// Bitset of tram roadtypes (`_roadtypes_tram`).
static ROADTYPES_TRAM: GameState<RoadTypes> = GameState::new(RoadTypes::EMPTY);

/// A field-sized load of one of the four masks, each its own static that C++ binds a
/// reference to and no borrow covers.
#[allow(unsafe_code, reason = "a field-sized load of a published global")]
fn load<T: Copy>(global: &GameState<T>) -> T {
    // SAFETY: The static is the whole place, no borrow covers it, and the game state is
    // serialized.
    unsafe { global.get().read() }
}

/// A field-sized store to one of the four masks.
#[allow(unsafe_code, reason = "a field-sized store to a published global")]
fn store<T>(global: &GameState<T>, value: T) {
    // SAFETY: As for `load`.
    unsafe { global.get().write(value) }
}

/// `_railtypes_hidden_mask`.
pub(crate) fn railtypes_hidden_mask() -> RailTypes {
    load(&RAILTYPES_HIDDEN_MASK)
}

/// `_roadtypes_hidden_mask`.
pub(crate) fn roadtypes_hidden_mask() -> RoadTypes {
    load(&ROADTYPES_HIDDEN_MASK)
}

/// `_roadtypes_road`.
pub(crate) fn roadtypes_road() -> RoadTypes {
    load(&ROADTYPES_ROAD)
}

/// `_roadtypes_tram`.
pub(crate) fn roadtypes_tram() -> RoadTypes {
    load(&ROADTYPES_TRAM)
}

/// The registry state C++ does not read in place, with the table the last reset installed.
/// The sorted lists' buffers are published to C++ as spans after each change.
struct Registry {
    services: *const Services,
    /// Sorted list of rail types.
    sorted_railtypes: Vec<RailType>,
    /// Sorted list of road types.
    sorted_roadtypes: Vec<RoadType>,
    /// `_railtype_list`: the rail type labels of a saved game, in its order.
    railtype_list: Vec<LabelObject<RailTypeLabel>>,
    /// `_roadtype_list`: the road type labels of a saved game, in its order.
    roadtype_list: Vec<LabelObject<RoadTypeLabel>>,
}

static REGISTRY: GameState<Registry> = GameState::new(Registry {
    services: ptr::null(),
    sorted_railtypes: Vec::new(),
    sorted_roadtypes: Vec::new(),
    railtype_list: Vec::new(),
    roadtype_list: Vec::new(),
});

/// Run `f` on the registry state. `f` makes no service call, so no other borrow of the state
/// can begin while it runs.
#[allow(
    unsafe_code,
    reason = "the process-wide registry, borrowed for one closure that makes no call"
)]
fn with_registry<R>(f: impl FnOnce(&mut Registry) -> R) -> R {
    // SAFETY: No pointer into the state is published (the sorted lists publish their heap
    // buffers), and the borrow ends before any service call.
    f(unsafe { &mut *REGISTRY.get() })
}

/// The table [`reset_rail_types`] and [`reset_road_types`] install. `NewGRF` loading resets
/// the types, and so installs the process-lifetime table, before any other body runs.
#[allow(unsafe_code, reason = "the installed process-lifetime table")]
pub(crate) fn services() -> &'static Services {
    let s = with_registry(|r| r.services);
    // SAFETY: The reset installs the process-lifetime table before any reader runs.
    unsafe { &*s }
}

/// A registry table over `rail`, `road` and `shared`, for tests.
#[cfg(test)]
pub(crate) fn test_services(
    rail: &'static RailServices,
    road: &'static RoadServices,
    shared: &'static Shared,
) -> Services {
    Services { rail, road, shared }
}

/// Install `s` as the table the bodies read, returning the one installed before, for a test
/// of a body that reads the installed table without a reset.
#[cfg(test)]
pub(crate) fn exchange_services(s: *const Services) -> *const Services {
    with_registry(|r| std::mem::replace(&mut r.services, s))
}

/// The handle of a file an FFI entry or `_raw` adapter receives, or `None` for the null file
/// the original tests for.
#[allow(unsafe_code, reason = "the caller's live file")]
fn grf_file(file: Handle) -> Option<GRFFileRef<'static>> {
    // SAFETY: Each FFI entry and `_raw` adapter receives a null or live `GRFFile` (its callers'
    // contract).
    unsafe { GRFFileRef::new(services().shared(), file.cast::<GRFFile>()) }
}

/// `&_railtypes_hidden_mask`, which C++ binds its reference to.
#[allow(unsafe_code, reason = "export; C++ binds a reference to the global")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_hidden_mask() -> *mut u64 {
    RAILTYPES_HIDDEN_MASK.get().cast()
}

/// `&_roadtypes_hidden_mask`, which C++ binds its reference to.
#[allow(unsafe_code, reason = "export; C++ binds a reference to the global")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_hidden_mask() -> *mut u64 {
    ROADTYPES_HIDDEN_MASK.get().cast()
}

/// `&_roadtypes_road`, which C++ binds its reference to.
#[allow(unsafe_code, reason = "export; C++ binds a reference to the global")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_mask() -> *mut u64 {
    ROADTYPES_ROAD.get().cast()
}

/// `&_roadtypes_tram`, which C++ binds its reference to.
#[allow(unsafe_code, reason = "export; C++ binds a reference to the global")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_tram_mask() -> *mut u64 {
    ROADTYPES_TRAM.get().cast()
}

/// `ResetRailTypes`: reset all rail type information to its default values.
pub(crate) fn reset_rail_types(s: &'static Services) {
    with_registry(|r| r.services = s);
    let rail = s.rail();

    // `std::copy` of `_original_railtypes` to the front of `_railtypes`, then `std::fill` of
    // the rest with `RailTypeInfo{}`.
    for rt in RailType::RAILTYPE_BEGIN.0..RailType::RAILTYPE_END.0 {
        let rti = (rail.spec)(RailType(rt));
        if rt < rail.default_count {
            (rail.copy_default)(rti, RailType(rt));
        } else {
            (rail.clear)(rti);
        }
    }

    store(&RAILTYPES_HIDDEN_MASK, RailTypes::EMPTY);
}

/// Publish `_sorted_railtypes` as the span over the sorted list's buffer.
fn set_sorted_railtypes(rail: &RailServices) {
    let (data, count) =
        with_registry(|r| (r.sorted_railtypes.as_mut_ptr(), r.sorted_railtypes.len()));
    (rail.set_sorted_railtypes)(data, count);
}

/// `InitRailTypes`: resolve sprites of custom rail types.
pub(crate) fn init_rail_types() {
    let s = services();
    let rail = s.rail();
    let shared = s.shared();

    with_registry(|r| r.sorted_railtypes.clear());
    set_sorted_railtypes(rail);
    for rt in RailType::RAILTYPE_BEGIN.0..RailType::RAILTYPE_END.0 {
        let rt = RailType(rt);
        let rti = (rail.spec)(rt);

        (rail.resolve_rail_type_gui_sprites)(rti);
        let info = RailTypeInfoRef::of(shared, rti);
        let hidden = info.flags().test(RailTypeFlag::Hidden);
        store(
            &RAILTYPES_HIDDEN_MASK,
            railtypes_hidden_mask().set(rt, hidden),
        );

        if info.label().0 == 0 {
            continue;
        }
        with_registry(|r| r.sorted_railtypes.push(rt));
        // A push can move the buffer, and the next `ResolveRailTypeGUISprites` runs while the
        // original's vector is visible half-built.
        set_sorted_railtypes(rail);
    }
    let (data, count) =
        with_registry(|r| (r.sorted_railtypes.as_mut_ptr(), r.sorted_railtypes.len()));
    (rail.sort)(data, count);
}

/// `AllocateRailType`: allocate a new rail type label.
pub(crate) fn allocate_rail_type(label: RailTypeLabel) -> RailType {
    let s = services();
    let rail = s.rail();
    let shared = s.shared();

    // `std::ranges::find(_railtypes, 0, &RailTypeInfo::label)`.
    for rt in RailType::RAILTYPE_BEGIN.0..RailType::RAILTYPE_END.0 {
        let rt = RailType(rt);
        let rti = (rail.spec)(rt);
        if RailTypeInfoRef::of(shared, rti).label().0 != 0 {
            continue;
        }

        /* Set up new rail type based on default rail. */
        (rail.copy_default)(rti, RailType::RAILTYPE_RAIL);
        (rail.set_label)(rti, label);
        (rail.alternate_labels_clear)(rti);

        /* Make us compatible with ourself. */
        (rail.set_powered_railtypes)(rti, RailTypes::from_element(rt));
        (rail.set_compatible_railtypes)(rti, RailTypes::from_element(rt));

        /* We also introduce ourself. */
        (rail.set_introduces_railtypes)(rti, RailTypes::from_element(rt));

        /* Default sort order; order of allocation, but with some
         * offsets so it's easier for NewGRF to pick a spot without
         * changing the order of other (original) rail types.
         * The << is so you can place other railtypes in between the
         * other railtypes, the 7 is to be able to place something
         * before the first (default) rail type. */
        // The original computes `rt << 4 | 7` in `int` and stores it to a `uint8_t`; the
        // wrapping shift keeps the same low byte.
        (rail.set_sorting_order)(rti, rt.0.wrapping_shl(4) | 7);

        return rt;
    }
    RailType::INVALID_RAILTYPE
}

/// `GetRailTypeByLabel`: get the rail type for a given label.
pub(crate) fn get_rail_type_by_label(
    label: RailTypeLabel,
    allow_alternate_labels: bool,
) -> RailType {
    if label.0 == 0 {
        return RailType::INVALID_RAILTYPE;
    }
    let s = services();
    let rail = s.rail();
    let shared = s.shared();

    let types = || (RailType::RAILTYPE_BEGIN.0..RailType::RAILTYPE_END.0).map(RailType);
    let mut it = types().find(|&rt| RailTypeInfoRef::of(shared, (rail.spec)(rt)).label() == label);
    if it.is_none() && allow_alternate_labels {
        /* Test if any rail type defines the label as an alternate. */
        it = types().find(|&rt| (rail.alternate_labels_contains)((rail.spec)(rt), label));
    }

    if let Some(rt) = it {
        return rt;
    }

    /* No matching label was found, so it is invalid */
    RailType::INVALID_RAILTYPE
}

/// `GetRailTypeTranslation`: translate a railtype from GRF-local to OpenTTD-global.
pub(crate) fn get_rail_type_translation(railtype: u8, grffile: Option<GRFFileRef<'_>>) -> RailType {
    match grffile.map(GRFFileRef::railtype_list) {
        Some(list) if list.len() != 0 => {
            /* Railtype table present, but invalid index, return invalid type. */
            if usize::from(railtype) >= list.len() {
                return RailType::INVALID_RAILTYPE;
            }

            /* Look up railtype including alternate labels. */
            let label = list.as_slice()[usize::from(railtype)];
            get_rail_type_by_label(label, true)
        }
        _ => {
            /* No railtype table present. Return railtype as-is (if valid), so it works for original railtypes. */
            if railtype >= RailType::RAILTYPE_END.0
                || get_rail_type_info(services().shared(), RailType(railtype))
                    .label()
                    .0
                    == 0
            {
                return RailType::INVALID_RAILTYPE;
            }

            RailType(railtype)
        }
    }
}

/// `GetReverseRailTypeTranslation`: perform a reverse railtype lookup to get the GRF internal
/// ID.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the original's `return idx` narrows the `int` to `uint8_t`"
)]
pub(crate) fn get_reverse_rail_type_translation(
    railtype: RailType,
    grffile: Option<GRFFileRef<'_>>,
) -> u8 {
    /* No rail type table present, return rail type as-is */
    let Some(list) = grffile.map(GRFFileRef::railtype_list) else {
        return railtype.0;
    };
    if list.len() == 0 {
        return railtype.0;
    }

    /* Look for a matching rail type label in the table */
    let label = get_rail_type_info(services().shared(), railtype).label();

    let idx = find_index(list.as_slice(), &label);
    if idx >= 0 {
        return idx as u8;
    }

    /* If not found, return as invalid */
    0xFF
}

/// `ConvertRailTypes`: test if any saved rail type labels are different to the currently
/// loaded rail types. Rail types stored in the map will be converted if necessary.
pub(crate) fn convert_rail_types(shared: &Shared, map: &RailMapServices, count: u32) {
    let mut railtype_conversion_map = Vec::new();
    let mut needs_conversion = false;

    let len = with_registry(|r| r.railtype_list.len());
    for index in 0..len {
        let it = with_registry(|r| r.railtype_list[index]);
        let mut rt = get_rail_type_by_label(it.label, true);
        if rt == RailType::INVALID_RAILTYPE {
            rt = RailType::RAILTYPE_RAIL;
        }

        railtype_conversion_map.push(rt);

        /* Conversion is needed if the rail type is in a different position than the list. */
        if it.label.0 != 0 && usize::from(rt.0) != index {
            needs_conversion = true;
        }
    }

    if !needs_conversion {
        return;
    }

    // `railtype_conversion_map[...]` indexes a `std::vector` unchecked in the original; here a
    // tile type past the end of a short saved list ends the game instead.
    let convert = |t: TileIndex, rt: RailType| {
        (map.set_rail_type)(t.0, railtype_conversion_map[usize::from(rt.0)].0);
    };
    let w = World::new(shared);
    for t in Map::iterate(count) {
        match w.kind(t) {
            TileType::MP_RAILWAY => convert(t, w.get_rail_type(t)),

            // Each guard's tile type has no other arm, so a failed guard reaches `_`, as the
            // original's `break` does.
            TileType::MP_ROAD if w.level_crossing(t.0) => convert(t, w.get_rail_type(t)),

            TileType::MP_STATION if w.has_station_rail(t) => convert(t, w.get_rail_type(t)),

            TileType::MP_TUNNELBRIDGE
                if w.asserted(|m| {
                    crate::tunnelbridge_map::get_tunnel_bridge_transport_type(m, t)
                }) == TransportType::TRANSPORT_RAIL =>
            {
                convert(t, w.get_rail_type(t));
            }

            _ => {}
        }
    }
}

/// `SetCurrentRailTypeLabelList`: populate railtype label list with current values.
pub(crate) fn set_current_rail_type_label_list() {
    let shared = services().shared();
    with_registry(|r| r.railtype_list.clear());

    for rt in RailType::RAILTYPE_BEGIN.0..RailType::RAILTYPE_END.0 {
        let label = get_rail_type_info(shared, RailType(rt)).label();
        with_registry(|r| r.railtype_list.push(LabelObject { label, subtype: 0 }));
    }
}

/// `ClearRailTypeLabelList`.
pub(crate) fn clear_rail_type_label_list() {
    with_registry(|r| r.railtype_list.clear());
}

/// `ResetRoadTypes`: reset all road type information to its default values.
pub(crate) fn reset_road_types(s: &'static Services) {
    with_registry(|r| r.services = s);
    let road = s.road();

    // `std::copy` of `_original_roadtypes` to the front of `_roadtypes`, then `std::fill` of
    // the rest with `RoadTypeInfo{}`.
    for rt in RoadType::ROADTYPE_BEGIN.0..RoadType::ROADTYPE_END.0 {
        let rti = (road.spec)(RoadType(rt));
        if rt < road.default_count {
            (road.copy_default)(rti, RoadType(rt));
        } else {
            (road.clear)(rti);
        }
    }

    store(&ROADTYPES_HIDDEN_MASK, RoadTypes::EMPTY);
    store(
        &ROADTYPES_ROAD,
        RoadTypes::from_element(RoadType::ROADTYPE_ROAD),
    );
    store(
        &ROADTYPES_TRAM,
        RoadTypes::from_element(RoadType::ROADTYPE_TRAM),
    );
}

/// Publish `_sorted_roadtypes` as the span over the sorted list's buffer.
fn set_sorted_roadtypes(road: &RoadServices) {
    let (data, count) =
        with_registry(|r| (r.sorted_roadtypes.as_mut_ptr(), r.sorted_roadtypes.len()));
    (road.set_sorted_roadtypes)(data, count);
}

/// `InitRoadTypes`: resolve sprites of custom road types.
pub(crate) fn init_road_types() {
    let s = services();
    let road = s.road();
    let types = type_availability::services(s.shared());

    with_registry(|r| r.sorted_roadtypes.clear());
    set_sorted_roadtypes(road);
    for rt in RoadType::ROADTYPE_BEGIN.0..RoadType::ROADTYPE_END.0 {
        let rt = RoadType(rt);
        let rti = (road.spec)(rt);

        (road.resolve_road_type_gui_sprites)(rti);
        let info = RoadTypeInfoRef::of(types, rti);
        let hidden = info.flags().test(crate::road::RoadTypeFlag::Hidden);
        store(
            &ROADTYPES_HIDDEN_MASK,
            roadtypes_hidden_mask().set(rt, hidden),
        );

        if info.label().0 == 0 {
            continue;
        }
        with_registry(|r| r.sorted_roadtypes.push(rt));
        // A push can move the buffer, and the next `ResolveRoadTypeGUISprites` runs while the
        // original's vector is visible half-built.
        set_sorted_roadtypes(road);
    }
    let (data, count) =
        with_registry(|r| (r.sorted_roadtypes.as_mut_ptr(), r.sorted_roadtypes.len()));
    (road.sort)(data, count);
}

/// `AllocateRoadType`: allocate a new road type label.
pub(crate) fn allocate_road_type(label: RoadTypeLabel, rtt: RoadTramType) -> RoadType {
    let s = services();
    let road = s.road();
    let types = type_availability::services(s.shared());

    // `std::ranges::find(_roadtypes, 0, &RoadTypeInfo::label)`.
    for rt in RoadType::ROADTYPE_BEGIN.0..RoadType::ROADTYPE_END.0 {
        let rt = RoadType(rt);
        let rti = (road.spec)(rt);
        if RoadTypeInfoRef::of(types, rti).label().0 != 0 {
            continue;
        }

        /* Set up new road type based on default tram or road. */
        (road.copy_default)(
            rti,
            if rtt == RoadTramType::RTT_TRAM {
                RoadType::ROADTYPE_TRAM
            } else {
                RoadType::ROADTYPE_ROAD
            },
        );
        (road.set_label)(rti, label);
        (road.alternate_labels_clear)(rti);
        (road.set_flags)(rti, RoadTypeFlags(0));
        (road.set_introduction_date)(rti, CalendarDate(INVALID_DATE));

        /* Make us compatible with ourself. */
        (road.set_powered_roadtypes)(rti, RoadTypes::from_element(rt));

        /* We also introduce ourself. */
        (road.set_introduces_roadtypes)(rti, RoadTypes::from_element(rt));

        /* Default sort order; order of allocation, but with some
         * offsets so it's easier for NewGRF to pick a spot without
         * changing the order of other (original) road types.
         * The << is so you can place other roadtypes in between the
         * other roadtypes, the 7 is to be able to place something
         * before the first (default) road type. */
        // `rt << 2 | 7` narrows to the `uint8_t` as `AllocateRailType`'s does.
        (road.set_sorting_order)(rti, rt.0.wrapping_shl(2) | 7);

        /* Set bitmap of road/tram types */
        store(
            &ROADTYPES_ROAD,
            roadtypes_road().set(rt, rtt == RoadTramType::RTT_ROAD),
        );
        store(
            &ROADTYPES_TRAM,
            roadtypes_tram().set(rt, rtt == RoadTramType::RTT_TRAM),
        );

        return rt;
    }
    RoadType::INVALID_ROADTYPE
}

/// `GetRoadTypeByLabel`: get the road type for a given label.
pub(crate) fn get_road_type_by_label(
    label: RoadTypeLabel,
    allow_alternate_labels: bool,
) -> RoadType {
    if label.0 == 0 {
        return RoadType::INVALID_ROADTYPE;
    }
    let s = services();
    let road = s.road();
    let types = type_availability::services(s.shared());

    let roadtypes = || (RoadType::ROADTYPE_BEGIN.0..RoadType::ROADTYPE_END.0).map(RoadType);
    let mut it =
        roadtypes().find(|&rt| RoadTypeInfoRef::of(types, (road.spec)(rt)).label() == label);
    if it.is_none() && allow_alternate_labels {
        /* Test if any road type defines the label as an alternate. */
        it = roadtypes().find(|&rt| (road.alternate_labels_contains)((road.spec)(rt), label));
    }

    if let Some(rt) = it {
        return rt;
    }

    /* No matching label was found, so it is invalid */
    RoadType::INVALID_ROADTYPE
}

/// `GetRoadTypeTranslation`: translate a roadtype from GRF-local to OpenTTD-global.
pub(crate) fn get_road_type_translation(
    rtt: RoadTramType,
    tracktype: u8,
    grffile: Option<GRFFileRef<'_>>,
) -> RoadType {
    /* Because OpenTTD mixes RoadTypes and TramTypes into the same type,
     * the mapping of the original road- and tramtypes does not match the default GRF-local mapping.
     * So, this function cannot provide any similar behavior to GetCargoTranslation() and GetRailTypeTranslation()
     * when the GRF defines no translation table.
     * But since there is only one default road/tram-type, this makes little sense anyway.
     * So for GRF without translation table, we always return INVALID_ROADTYPE.
     */

    let Some(grffile) = grffile else {
        return RoadType::INVALID_ROADTYPE;
    };

    let list = if rtt == RoadTramType::RTT_TRAM {
        grffile.tramtype_list()
    } else {
        grffile.roadtype_list()
    };
    if usize::from(tracktype) >= list.len() {
        return RoadType::INVALID_ROADTYPE;
    }

    /* Look up roadtype including alternate labels. */
    let result = get_road_type_by_label(list.as_slice()[usize::from(tracktype)], true);

    /* Check whether the result is actually the wanted road/tram-type */
    if result != RoadType::INVALID_ROADTYPE && get_road_tram_type(result) != rtt {
        return RoadType::INVALID_ROADTYPE;
    }

    result
}

/// `GetReverseRoadTypeTranslation`: perform a reverse roadtype lookup to get the GRF internal
/// ID.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the original's `return index` narrows the `int` to `uint8_t`"
)]
pub(crate) fn get_reverse_road_type_translation(
    roadtype: RoadType,
    grffile: Option<GRFFileRef<'_>>,
) -> u8 {
    /* No road type table present, return road type as-is */
    let Some(grffile) = grffile else {
        return roadtype.0;
    };

    let list = if road_type_is_road(roadtype) {
        grffile.roadtype_list()
    } else {
        grffile.tramtype_list()
    };
    if list.len() == 0 {
        return roadtype.0;
    }

    /* Look for a matching road type label in the table */
    let types = type_availability::services(services().shared());
    let label = get_road_type_info(types, roadtype).label();

    let index = find_index(list.as_slice(), &label);
    if index >= 0 {
        return index as u8;
    }

    /* If not found, return as invalid */
    0xFF
}

/// `ConvertRoadTypes`: test if any saved road type labels are different to the currently
/// loaded road types. Road types stored in the map will be converted if necessary.
pub(crate) fn convert_road_types(shared: &Shared, count: u32) {
    let mut roadtype_conversion_map = Vec::new();
    let mut needs_conversion = false;
    let len = with_registry(|r| r.roadtype_list.len());
    for index in 0..len {
        let it = with_registry(|r| r.roadtype_list[index]);
        let mut rt = get_road_type_by_label(it.label, true);
        if rt == RoadType::INVALID_ROADTYPE || u8::from(get_road_tram_type(rt).0) != it.subtype {
            rt = if it.subtype != 0 {
                RoadType::ROADTYPE_TRAM
            } else {
                RoadType::ROADTYPE_ROAD
            };
        }

        roadtype_conversion_map.push(rt);

        /* Conversion is needed if the road type is in a different position than the list. */
        if it.label.0 != 0 && usize::from(rt.0) != index {
            needs_conversion = true;
        }
    }
    if !needs_conversion {
        return;
    }

    // `roadtype_conversion_map[rt]` indexes a `std::vector` unchecked in the original; here a
    // tile type past the end of a short saved list ends the game instead.
    let w = World::new(shared);
    let convert = |t: TileIndex| {
        let rt = w.asserted(|m| crate::road_map::get_road_type_road(m, t.0));
        if rt != RoadType::INVALID_ROADTYPE {
            crate::road_map::set_road_type_road(
                shared,
                t.0,
                roadtype_conversion_map[usize::from(rt.0)].0,
            );
        }
        let rt = w.asserted(|m| crate::road_map::get_road_type_tram(m, t.0));
        if rt != RoadType::INVALID_ROADTYPE {
            crate::road_map::set_road_type_tram(
                shared,
                t.0,
                roadtype_conversion_map[usize::from(rt.0)].0,
            );
        }
    };
    for t in Map::iterate(count) {
        match w.kind(t) {
            TileType::MP_ROAD => convert(t),

            // Each guard's tile type has no other arm, so a failed guard reaches `_`, as the
            // original's `break` does.
            TileType::MP_STATION if w.asserted(|m| crate::station_map::is_any_road_stop(m, t)) => {
                convert(t);
            }

            TileType::MP_TUNNELBRIDGE
                if w.asserted(|m| {
                    crate::tunnelbridge_map::get_tunnel_bridge_transport_type(m, t)
                }) == TransportType::TRANSPORT_ROAD =>
            {
                convert(t);
            }

            _ => {}
        }
    }
}

/// `SetCurrentRoadTypeLabelList`: populate road type label list with current values.
pub(crate) fn set_current_road_type_label_list() {
    let types = type_availability::services(services().shared());
    with_registry(|r| r.roadtype_list.clear());
    for rt in RoadType::ROADTYPE_BEGIN.0..RoadType::ROADTYPE_END.0 {
        let rt = RoadType(rt);
        let label = get_road_type_info(types, rt).label();
        let subtype = u8::from(get_road_tram_type(rt).0);
        with_registry(|r| r.roadtype_list.push(LabelObject { label, subtype }));
    }
}

/// `ClearRoadTypeLabelList`.
pub(crate) fn clear_road_type_label_list() {
    with_registry(|r| r.roadtype_list.clear());
}

/* Shims for the C++ facades and the files no pass has finished. */

/// Shim for `GetRailTypeByLabel`.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_by_label(label: u32, alternate: bool) -> u8 {
    get_rail_type_by_label(RailTypeLabel(label), alternate).0
}

/// Shim for `GetRoadTypeByLabel`.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_by_label(label: u32, alternate: bool) -> u8 {
    get_road_type_by_label(RoadTypeLabel(label), alternate).0
}

/// Shim for `GetRailTypeTranslation`.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_translate(rt: u8, file: Handle) -> u8 {
    get_rail_type_translation(rt, grf_file(file)).0
}

/// Shim for `GetRoadTypeTranslation`.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_translate(rtt: RoadTramType, rt: u8, file: Handle) -> u8 {
    get_road_type_translation(rtt, rt, grf_file(file)).0
}

/// Shim for `GetReverseRailTypeTranslation`.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_reverse(rt: u8, file: Handle) -> u8 {
    get_reverse_rail_type_translation(RailType(rt), grf_file(file))
}

/// Shim for `GetReverseRoadTypeTranslation`.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_reverse(rt: u8, file: Handle) -> u8 {
    get_reverse_road_type_translation(RoadType(rt), grf_file(file))
}

/// Shim for `ConvertRailTypes`.
#[allow(
    unsafe_code,
    reason = "export and the caller's process-lifetime tables"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_saved_convert(
    shared: *const Shared,
    map: *const RailMapServices,
    count: u32,
) {
    // SAFETY: The caller passes the process-lifetime tables.
    convert_rail_types(unsafe { &*shared }, unsafe { &*map }, count);
}

/// Shim for `ConvertRoadTypes`.
#[allow(unsafe_code, reason = "export and the caller's process-lifetime table")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_saved_convert(shared: *const Shared, count: u32) {
    // SAFETY: The caller passes the process-lifetime table.
    convert_road_types(unsafe { &*shared }, count);
}

/// `_railtype_list.reserve(RAILTYPE_END)`, of the `RAIL` chunk's `Load`.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_saved_reserve() {
    with_registry(|r| {
        r.railtype_list
            .reserve(usize::from(RailType::RAILTYPE_END.0));
    });
}

/// `_railtype_list.push_back(lo)`, of the `RAIL` chunk's `Load`.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_saved_append(label: u32, subtype: u8) {
    with_registry(|r| {
        r.railtype_list.push(LabelObject {
            label: RailTypeLabel(label),
            subtype,
        });
    });
}

/// Shim for `ClearRailTypeLabelList`.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_saved_clear() {
    clear_rail_type_label_list();
}

/// Shim for `SetCurrentRailTypeLabelList`.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_saved_current() {
    set_current_rail_type_label_list();
}

/// `_roadtype_list.reserve(ROADTYPE_END)`, of the `ROTT` chunk's `Load`.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_saved_reserve() {
    with_registry(|r| {
        r.roadtype_list
            .reserve(usize::from(RoadType::ROADTYPE_END.0));
    });
}

/// `_roadtype_list.push_back(lo)`, of the `ROTT` chunk's `Load`.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_saved_append(label: u32, subtype: u8) {
    with_registry(|r| {
        r.roadtype_list.push(LabelObject {
            label: RoadTypeLabel(label),
            subtype,
        });
    });
}

/// Shim for `ClearRoadTypeLabelList`.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_saved_clear() {
    clear_road_type_label_list();
}

/// Shim for `SetCurrentRoadTypeLabelList`.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_saved_current() {
    set_current_road_type_label_list();
}

/// Raw adapter: `GRFFile::railtype_list` of a live file.
#[allow(unsafe_code, reason = "the caller's live file")]
pub(crate) fn railtype_list_raw(file: Handle) -> RailTypeLabels {
    // SAFETY: The caller passes the loader's live current file.
    unsafe { GRFFileRef::of(services().shared(), file.cast::<GRFFile>()) }.railtype_list()
}

/// Raw adapter: `GetRailTypeInfo(rt)->label`.
pub(crate) fn rail_label_raw(rt: u8) -> u32 {
    get_rail_type_info(services().shared(), RailType(rt))
        .label()
        .0
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRailTypeLabels" ["engine"] => RailTypeLabels { data, size };
    "OpenTTDRoadTypeLabels" ["engine"] => RoadTypeLabels { data, size };
    "OpenTTDRailTypeMap" ["engine"] => RailTypeMap { data, size };
    "OpenTTDRoadTypeMap" ["engine"] => RoadTypeMap { data, size };
    "OpenTTDRailRegistryServices" ["engine"] => RailServices {
        clear,
        copy_default,
        default_count,
        resolve_rail_type_gui_sprites,
        set_sorted_railtypes,
        sort,
        spec,
        alternate_labels_clear,
        alternate_labels_contains,
        alternate_labels_insert,
        badges,
        compatible_railtypes_set,
        introduces_railtypes_set,
        introduction_required_railtypes_set,
        powered_railtypes_set,
        set_acceleration_type,
        set_compatible_railtypes,
        set_cost_multiplier,
        set_curve_speed,
        set_fallback_railtype,
        set_flags,
        set_introduces_railtypes,
        set_introduction_date,
        set_label,
        set_maintenance_multiplier,
        set_map_colour,
        set_max_speed,
        set_powered_railtypes,
        set_sorting_order,
        strings_build_caption,
        strings_menu_text,
        strings_name,
        strings_new_loco,
        strings_replace_text,
        strings_toolbar_caption,
    };
    "OpenTTDRoadRegistryServices" ["engine"] => RoadServices {
        clear,
        copy_default,
        default_count,
        resolve_road_type_gui_sprites,
        set_sorted_roadtypes,
        sort,
        spec,
        alternate_labels_clear,
        alternate_labels_contains,
        alternate_labels_insert,
        badges,
        introduces_roadtypes_set,
        introduction_required_roadtypes_set,
        powered_roadtypes_set,
        set_cost_multiplier,
        set_flags,
        set_introduces_roadtypes,
        set_introduction_date,
        set_label,
        set_maintenance_multiplier,
        set_map_colour,
        set_max_speed,
        set_powered_roadtypes,
        set_sorting_order,
        strings_build_caption,
        strings_menu_text,
        strings_name,
        strings_new_engine,
        strings_replace_text,
        strings_toolbar_caption,
    };
    "OpenTTDTypeRegistryServices" ["engine"] => Services { rail, road, shared };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    thread_local! {
        /// Each element's label, by the element index its fake pointer carries.
        static LABELS: RefCell<[u32; 64]> = const { RefCell::new([0; 64]) };
        /// The stores, in order: (member, element, value).
        static STORES: RefCell<Vec<(&'static str, usize, u64)>> = const { RefCell::new(Vec::new()) };
        /// The availability table the shared table names.
        static TYPES: Cell<*const type_availability::Services> = const { Cell::new(ptr::null()) };
    }

    /// The fake element of type `rt`: never dereferenced, only carried back to the stubs.
    fn element<T>(rt: u8) -> *mut T {
        ptr::without_provenance_mut(usize::from(rt) + 1)
    }
    fn index<T>(rti: *const T) -> usize {
        rti.addr() - 1
    }
    fn record(member: &'static str, element: usize, value: u64) {
        STORES.with(|s| s.borrow_mut().push((member, element, value)));
    }

    extern "C" fn rail_spec(rt: RailType) -> *mut RailTypeInfo {
        element(rt.0)
    }
    extern "C" fn rail_label(rti: *const RailTypeInfo) -> u32 {
        LABELS.with(|l| l.borrow()[index(rti)])
    }
    extern "C" fn rail_copy_default(rti: *mut RailTypeInfo, source: RailType) {
        record("copy_default", index(rti), u64::from(source.0));
    }
    extern "C" fn rail_set_label(rti: *mut RailTypeInfo, label: RailTypeLabel) {
        record("label", index(rti), u64::from(label.0));
    }
    extern "C" fn rail_alternate_labels_clear(rti: *mut RailTypeInfo) {
        record("alternate_labels.clear", index(rti), 0);
    }
    extern "C" fn rail_set_powered(rti: *mut RailTypeInfo, value: RailTypes) {
        record("powered_railtypes", index(rti), value.0);
    }
    extern "C" fn rail_set_compatible(rti: *mut RailTypeInfo, value: RailTypes) {
        record("compatible_railtypes", index(rti), value.0);
    }
    extern "C" fn rail_set_introduces(rti: *mut RailTypeInfo, value: RailTypes) {
        record("introduces_railtypes", index(rti), value.0);
    }
    extern "C" fn rail_set_sorting_order(rti: *mut RailTypeInfo, value: u8) {
        record("sorting_order", index(rti), u64::from(value));
    }

    extern "C" fn road_spec(rt: RoadType) -> *mut RoadTypeInfo {
        element(rt.0)
    }
    extern "C" fn road_label(rti: *const RoadTypeInfo) -> RoadTypeLabel {
        RoadTypeLabel(LABELS.with(|l| l.borrow()[index(rti)]))
    }
    extern "C" fn road_copy_default(rti: *mut RoadTypeInfo, source: RoadType) {
        record("copy_default", index(rti), u64::from(source.0));
    }
    extern "C" fn road_set_label(rti: *mut RoadTypeInfo, label: RoadTypeLabel) {
        record("label", index(rti), u64::from(label.0));
    }
    extern "C" fn road_alternate_labels_clear(rti: *mut RoadTypeInfo) {
        record("alternate_labels.clear", index(rti), 0);
    }
    extern "C" fn road_set_flags(rti: *mut RoadTypeInfo, value: RoadTypeFlags) {
        record("flags", index(rti), u64::from(value.0));
    }
    #[allow(clippy::cast_sign_loss, reason = "the recorded date's bits")]
    extern "C" fn road_set_introduction_date(rti: *mut RoadTypeInfo, value: CalendarDate) {
        record("introduction_date", index(rti), u64::from(value.0 as u32));
    }
    extern "C" fn road_set_powered(rti: *mut RoadTypeInfo, value: RoadTypes) {
        record("powered_roadtypes", index(rti), value.0);
    }
    extern "C" fn road_set_introduces(rti: *mut RoadTypeInfo, value: RoadTypes) {
        record("introduces_roadtypes", index(rti), value.0);
    }
    extern "C" fn road_set_sorting_order(rti: *mut RoadTypeInfo, value: u8) {
        record("sorting_order", index(rti), u64::from(value));
    }
    thread_local! {
        /// The one element and label `alternate_labels_contains` answers true for.
        static ALTERNATE: Cell<(usize, u32)> = const { Cell::new((usize::MAX, 0)) };
    }
    extern "C" fn rail_alternate_labels_contains(
        rti: *const RailTypeInfo,
        label: RailTypeLabel,
    ) -> bool {
        ALTERNATE.with(Cell::get) == (index(rti), label.0)
    }
    extern "C" fn road_alternate_labels_contains(
        rti: *const RoadTypeInfo,
        label: RoadTypeLabel,
    ) -> bool {
        ALTERNATE.with(Cell::get) == (index(rti), label.0)
    }
    extern "C" fn type_availability() -> *const type_availability::Services {
        TYPES.with(Cell::get)
    }

    /// Run `f` with the recording registry installed and the labels `labels` in place, under
    /// the game-state test lock, restoring the installed table and the road masks after.
    fn with_recording_registry<R>(
        labels: [u32; 64],
        f: impl FnOnce() -> R,
    ) -> (R, Vec<(&'static str, usize, u64)>) {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let shared: &'static Shared = Box::leak(Box::new(Shared {
            rail_type_label: rail_label,
            type_availability,
            ..crate::test_tables::SHARED
        }));
        TYPES.with(|t| {
            t.set(Box::leak(Box::new(type_availability::Services {
                road_label,
                shared,
                ..crate::test_tables::TYPE_AVAILABILITY
            })));
        });
        let rail: &'static RailServices = Box::leak(Box::new(RailServices {
            spec: rail_spec,
            copy_default: rail_copy_default,
            set_label: rail_set_label,
            alternate_labels_clear: rail_alternate_labels_clear,
            alternate_labels_contains: rail_alternate_labels_contains,
            set_powered_railtypes: rail_set_powered,
            set_compatible_railtypes: rail_set_compatible,
            set_introduces_railtypes: rail_set_introduces,
            set_sorting_order: rail_set_sorting_order,
            ..TEST_RAIL
        }));
        let road: &'static RoadServices = Box::leak(Box::new(RoadServices {
            spec: road_spec,
            copy_default: road_copy_default,
            set_label: road_set_label,
            alternate_labels_clear: road_alternate_labels_clear,
            alternate_labels_contains: road_alternate_labels_contains,
            set_flags: road_set_flags,
            set_introduction_date: road_set_introduction_date,
            set_powered_roadtypes: road_set_powered,
            set_introduces_roadtypes: road_set_introduces,
            set_sorting_order: road_set_sorting_order,
            ..TEST_ROAD
        }));
        let s: &'static Services = Box::leak(Box::new(Services { rail, road, shared }));
        LABELS.with(|l| *l.borrow_mut() = labels);
        STORES.with(|s| s.borrow_mut().clear());
        let saved = (exchange_services(s), roadtypes_road(), roadtypes_tram());
        let result = f();
        exchange_services(saved.0);
        store(&ROADTYPES_ROAD, saved.1);
        store(&ROADTYPES_TRAM, saved.2);
        (result, STORES.with(RefCell::take))
    }

    /// `AllocateRailType` takes the first type with no label, copies the default rail type
    /// into it and then stores its label, clears its alternates, makes it powered on,
    /// compatible with and introduce itself, and gives it `rt << 4 | 7` narrowed to a byte;
    /// with every type labelled it stores nothing. The scenario compares the type after
    /// allocation, which cannot show the stores' order or that the copy comes first.
    #[test]
    fn allocate_rail_type_stores_in_the_original_order_after_the_default_copy() {
        let mut labels = [1; 64];
        labels[63] = 0;
        let label = RailTypeLabel(u32::from_be_bytes(*b"NEW1"));
        let (rt, stores) = with_recording_registry(labels, || allocate_rail_type(label));
        let bit = 1 << 63;
        assert_eq!(rt, RailType(63));
        assert_eq!(
            stores,
            [
                ("copy_default", 63, u64::from(RailType::RAILTYPE_RAIL.0)),
                ("label", 63, u64::from(label.0)),
                ("alternate_labels.clear", 63, 0),
                ("powered_railtypes", 63, bit),
                ("compatible_railtypes", 63, bit),
                ("introduces_railtypes", 63, bit),
                ("sorting_order", 63, 0xF7),
            ]
        );

        let (rt, stores) = with_recording_registry([1; 64], || allocate_rail_type(label));
        assert_eq!(rt, RailType::INVALID_RAILTYPE);
        assert_eq!(stores, []);
    }

    /// `AllocateRoadType` copies the default tram type for a tram, then stores its label,
    /// clears its alternates and flags, invalidates its introduction date, makes it powered
    /// on and introduce itself, gives it `rt << 2 | 7` narrowed to a byte, and moves it from
    /// the road mask to the tram mask; with every type labelled it stores nothing.
    #[test]
    fn allocate_road_type_stores_in_the_original_order_after_the_default_copy() {
        let mut labels = [1; 64];
        labels[62] = 0;
        let label = RoadTypeLabel(u32::from_be_bytes(*b"NEW2"));
        let ((rt, road, tram), stores) = with_recording_registry(labels, || {
            store(&ROADTYPES_ROAD, RoadTypes(1 << 62));
            store(&ROADTYPES_TRAM, RoadTypes::EMPTY);
            let rt = allocate_road_type(label, RoadTramType::RTT_TRAM);
            (rt, roadtypes_road(), roadtypes_tram())
        });
        let bit = 1 << 62;
        assert_eq!(rt, RoadType(62));
        assert_eq!((road, tram), (RoadTypes::EMPTY, RoadTypes(bit)));
        assert_eq!(
            stores,
            [
                ("copy_default", 62, u64::from(RoadType::ROADTYPE_TRAM.0)),
                ("label", 62, u64::from(label.0)),
                ("alternate_labels.clear", 62, 0),
                ("flags", 62, 0),
                ("introduction_date", 62, u64::from(u32::MAX)),
                ("powered_roadtypes", 62, bit),
                ("introduces_roadtypes", 62, bit),
                ("sorting_order", 62, 0xFF),
            ]
        );

        let (rt, stores) = with_recording_registry([1; 64], || {
            allocate_road_type(label, RoadTramType::RTT_ROAD)
        });
        assert_eq!(rt, RoadType::INVALID_ROADTYPE);
        assert_eq!(stores, []);
    }

    /// `GetRailTypeByLabel` and `GetRoadTypeByLabel` read each type's label where C++ keeps it,
    /// at every lookup: a label written between lookups is seen at the first and last type,
    /// an exact label wins over another type's alternate, an alternate is found only when
    /// allowed, and a missing label or label 0 is invalid. These are the observations of the
    /// retired native case `Engine type label services expose live canonical scalars`.
    #[test]
    fn get_type_by_label_reads_each_label_in_place_and_prefers_an_exact_label() {
        let (first, last, missing) = (0x5150_0001, 0x5150_0002, 0x5150_0003);
        let mut labels = [0; 64];
        labels[0] = first;
        labels[63] = last;
        let (found, _) = with_recording_registry(labels, || {
            let rail = |label, alternate| get_rail_type_by_label(RailTypeLabel(label), alternate).0;
            let road = |label, alternate| get_road_type_by_label(RoadTypeLabel(label), alternate).0;
            let mut found = vec![
                rail(first, false),
                rail(last, false),
                rail(missing, false),
                rail(0, true),
            ];
            found.extend([road(first, false), road(missing, true)]);
            LABELS.with(|l| l.borrow_mut()[0] = missing);
            found.extend([
                rail(first, false),
                rail(missing, false),
                road(missing, false),
            ]);
            ALTERNATE.with(|a| a.set((0, last)));
            found.extend([rail(last, true), rail(first, true), road(last, true)]);
            LABELS.with(|l| l.borrow_mut()[63] = 0);
            found.extend([rail(last, false), rail(last, true)]);
            ALTERNATE.with(|a| a.set((usize::MAX, 0)));
            found
        });
        assert_eq!(
            found,
            [0, 63, 255, 255, 0, 63, 255, 0, 0, 63, 255, 0, 255, 0]
        );
    }
}
