/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Rail/road registry policy. Native specs retain their representation; canonical
//! order, masks and saved labels live here under the native game-state mutex.
//! File vector/map views last only through the synchronous no-reset call.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation
)]
use crate::services::RoadTramType;
use crate::{
    game_state::GameState,
    rail_type::{INVALID_RAILTYPE, RAILTYPE_BEGIN, RAILTYPE_END, RAILTYPE_RAIL},
    road_type::{INVALID_ROADTYPE, ROADTYPE_BEGIN, ROADTYPE_END, ROADTYPE_ROAD, ROADTYPE_TRAM},
};
use std::{ffi::c_void, ptr};
type Handle = *const c_void;
#[repr(C)]
pub struct Publication {
    label: u32,
    hidden: bool,
}
#[repr(C)]
pub struct LabelTable {
    values: [u32; RAILTYPE_END as usize],
}
#[repr(C)]
pub struct Allocation {
    label: u32,
    self_: u64,
    order: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Labels {
    data: *const u32,
    size: usize,
}
impl Labels {
    pub(crate) const fn len(self) -> usize {
        self.size
    }
    /// # Safety
    /// The index is below len and the native list has not been mutated.
    pub(crate) unsafe fn at(self, index: usize) -> u32 {
        unsafe { *self.data.add(index) }
    }

    fn slice(self) -> &'static [u32] {
        if self.size == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.data, self.size) }
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TypeMap {
    pub(crate) data: *mut u8,
    pub(crate) size: usize,
}
#[repr(C)]
pub struct RailServices {
    default_count: u8,
    spec: extern "C" fn(u8) -> Handle,
    type_label: extern "C" fn(u8) -> u32,
    labels: extern "C" fn() -> LabelTable,
    alternate: extern "C" fn(u8, u32) -> bool,
    clear: extern "C" fn(Handle),
    copy_default: extern "C" fn(Handle, u8),
    resolve_gui: extern "C" fn(Handle),
    publication: extern "C" fn(Handle) -> Publication,
    sort: extern "C" fn(*mut u8, usize),
    publish_sorted: extern "C" fn(*mut u8, usize),
    allocate: extern "C" fn(Handle, *const Allocation),
}
#[repr(C)]
pub struct RoadServices {
    default_count: u8,
    spec: extern "C" fn(u8) -> Handle,
    type_label: extern "C" fn(u8) -> u32,
    labels: extern "C" fn() -> LabelTable,
    alternate: extern "C" fn(u8, u32) -> bool,
    clear: extern "C" fn(Handle),
    copy_default: extern "C" fn(Handle, u8),
    resolve_gui: extern "C" fn(Handle),
    publication: extern "C" fn(Handle) -> Publication,
    sort: extern "C" fn(*mut u8, usize),
    publish_sorted: extern "C" fn(*mut u8, usize),
    allocate: extern "C" fn(Handle, *const Allocation),
}
#[repr(C)]
pub struct Services {
    rail: *const RailServices,
    road: *const RoadServices,
    rail_labels: extern "C" fn(Handle) -> Labels,
    road_labels: extern "C" fn(Handle) -> Labels,
    tram_labels: extern "C" fn(Handle) -> Labels,
    rail_map: extern "C" fn(Handle) -> TypeMap,
    road_map: extern "C" fn(Handle) -> TypeMap,
    tram_map: extern "C" fn(Handle) -> TypeMap,
}
struct Registry {
    services: *const Services,
    rail_sorted: Vec<u8>,
    road_sorted: Vec<u8>,
    rail_hidden: u64,
    road_hidden: u64,
    road: u64,
    tram: u64,
    rail_saved: Vec<(u32, u8)>,
    road_saved: Vec<(u32, u8)>,
}
static REGISTRY: GameState<Registry> = GameState::new(Registry {
    services: ptr::null(),
    rail_sorted: Vec::new(),
    road_sorted: Vec::new(),
    rail_hidden: 0,
    road_hidden: 0,
    road: 0,
    tram: 0,
    rail_saved: Vec::new(),
    road_saved: Vec::new(),
});
fn services() -> &'static Services {
    unsafe { &*(*REGISTRY.get()).services }
}
pub(crate) fn rail_label(rt: u8) -> u32 {
    (services().rail().type_label)(rt)
}
pub fn rail_hidden() -> u64 {
    unsafe { (*REGISTRY.get()).rail_hidden }
}
pub fn road_hidden() -> u64 {
    unsafe { (*REGISTRY.get()).road_hidden }
}
pub fn road_tram_mask(rtt: RoadTramType) -> u64 {
    unsafe {
        if rtt.0 {
            (*REGISTRY.get()).tram
        } else {
            (*REGISTRY.get()).road
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_hidden_mask() -> *mut u64 {
    unsafe { &raw mut (*REGISTRY.get()).rail_hidden }
}
pub(crate) unsafe fn openttd_rust_rail_reset(s: *const Services) {
    unsafe {
        (*REGISTRY.get()).services = s;
    }
    let s = services();
    for rt in RAILTYPE_BEGIN..RAILTYPE_END {
        let info = (s.rail().spec)(rt);
        if rt < s.rail().default_count {
            (s.rail().copy_default)(info, rt);
        } else {
            (s.rail().clear)(info);
        }
    }
    unsafe {
        (*REGISTRY.get()).rail_hidden = 0;
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_by_label(label: u32, alternate: bool) -> u8 {
    if label == 0 {
        return INVALID_RAILTYPE;
    }
    let s = services();
    let labels = (s.rail().labels)();
    for rt in RAILTYPE_BEGIN..RAILTYPE_END {
        if labels.values[usize::from(rt)] == label {
            return rt;
        }
    }
    if alternate {
        for rt in RAILTYPE_BEGIN..RAILTYPE_END {
            if (s.rail().alternate)(rt, label) {
                return rt;
            }
        }
    }
    INVALID_RAILTYPE
}
pub(crate) fn rail_allocate(label: u32) -> u8 {
    let s = services();
    for rt in RAILTYPE_BEGIN..RAILTYPE_END {
        if (s.rail().type_label)(rt) != 0 {
            continue;
        }
        let info = (s.rail().spec)(rt);
        (s.rail().copy_default)(info, RAILTYPE_RAIL);
        let value = Allocation {
            label,
            self_: 1u64 << rt,
            order: rt.wrapping_shl(4) | 7,
        };
        (s.rail().allocate)(info, &raw const value);
        return rt;
    }
    INVALID_RAILTYPE
}
pub(crate) fn rail_init() {
    let s = services();
    unsafe {
        (*REGISTRY.get()).rail_sorted.clear();
    }
    publish_rail_sorted();
    for rt in RAILTYPE_BEGIN..RAILTYPE_END {
        let spec = (s.rail().spec)(rt);
        (s.rail().resolve_gui)(spec);
        let value = (s.rail().publication)(spec);
        unsafe {
            let bit = 1u64 << rt;
            (*REGISTRY.get()).rail_hidden =
                ((*REGISTRY.get()).rail_hidden & !bit) | if value.hidden { bit } else { 0 };
            if value.label != 0 {
                (*REGISTRY.get()).rail_sorted.push(rt);
            }
        }
        publish_rail_sorted();
    }
    let (data, count) = unsafe {
        let sorted = &mut (*REGISTRY.get()).rail_sorted;
        (sorted.as_mut_ptr(), sorted.len())
    };
    (s.rail().sort)(data, count);
    (s.rail().publish_sorted)(data, count);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_saved_reserve() {
    unsafe {
        (*REGISTRY.get())
            .rail_saved
            .reserve(usize::from(RAILTYPE_END));
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_saved_append(label: u32, subtype: u8) {
    unsafe {
        (*REGISTRY.get()).rail_saved.push((label, subtype));
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_saved_clear() {
    unsafe {
        (*REGISTRY.get()).rail_saved.clear();
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_saved_current() {
    openttd_rust_rail_saved_clear();
    let s = services();
    for rt in RAILTYPE_BEGIN..RAILTYPE_END {
        let label = (s.rail().type_label)(rt);
        openttd_rust_rail_saved_append(label, 0);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_hidden_mask() -> *mut u64 {
    unsafe { &raw mut (*REGISTRY.get()).road_hidden }
}
pub(crate) unsafe fn openttd_rust_road_reset(s: *const Services) {
    unsafe {
        (*REGISTRY.get()).services = s;
    }
    let s = services();
    for rt in ROADTYPE_BEGIN..ROADTYPE_END {
        let info = (s.road().spec)(rt);
        if rt < s.road().default_count {
            (s.road().copy_default)(info, rt);
        } else {
            (s.road().clear)(info);
        }
    }
    unsafe {
        (*REGISTRY.get()).road_hidden = 0;
    }
    unsafe {
        (*REGISTRY.get()).road = 1u64 << ROADTYPE_ROAD;
        (*REGISTRY.get()).tram = 1u64 << ROADTYPE_TRAM;
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_by_label(label: u32, alternate: bool) -> u8 {
    if label == 0 {
        return INVALID_ROADTYPE;
    }
    let s = services();
    let labels = (s.road().labels)();
    for rt in ROADTYPE_BEGIN..ROADTYPE_END {
        if labels.values[usize::from(rt)] == label {
            return rt;
        }
    }
    if alternate {
        for rt in ROADTYPE_BEGIN..ROADTYPE_END {
            if (s.road().alternate)(rt, label) {
                return rt;
            }
        }
    }
    INVALID_ROADTYPE
}
pub(crate) fn road_allocate(label: u32, rtt: RoadTramType) -> u8 {
    let s = services();
    for rt in ROADTYPE_BEGIN..ROADTYPE_END {
        if (s.road().type_label)(rt) != 0 {
            continue;
        }
        let info = (s.road().spec)(rt);
        (s.road().copy_default)(info, if rtt.0 { ROADTYPE_TRAM } else { ROADTYPE_ROAD });
        let value = Allocation {
            label,
            self_: 1u64 << rt,
            order: rt.wrapping_shl(2) | 7,
        };
        (s.road().allocate)(info, &raw const value);
        unsafe {
            let bit = 1u64 << rt;
            (*REGISTRY.get()).road = ((*REGISTRY.get()).road & !bit) | if rtt.0 { 0 } else { bit };
            (*REGISTRY.get()).tram = ((*REGISTRY.get()).tram & !bit) | if rtt.0 { bit } else { 0 };
        }
        return rt;
    }
    INVALID_ROADTYPE
}
pub(crate) fn road_init() {
    let s = services();
    unsafe {
        (*REGISTRY.get()).road_sorted.clear();
    }
    publish_road_sorted();
    for rt in ROADTYPE_BEGIN..ROADTYPE_END {
        let spec = (s.road().spec)(rt);
        (s.road().resolve_gui)(spec);
        let value = (s.road().publication)(spec);
        unsafe {
            let bit = 1u64 << rt;
            (*REGISTRY.get()).road_hidden =
                ((*REGISTRY.get()).road_hidden & !bit) | if value.hidden { bit } else { 0 };
            if value.label != 0 {
                (*REGISTRY.get()).road_sorted.push(rt);
            }
        }
        publish_road_sorted();
    }
    let (data, count) = unsafe {
        let sorted = &mut (*REGISTRY.get()).road_sorted;
        (sorted.as_mut_ptr(), sorted.len())
    };
    (s.road().sort)(data, count);
    (s.road().publish_sorted)(data, count);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_saved_reserve() {
    unsafe {
        (*REGISTRY.get())
            .road_saved
            .reserve(usize::from(ROADTYPE_END));
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_saved_append(label: u32, subtype: u8) {
    unsafe {
        (*REGISTRY.get()).road_saved.push((label, subtype));
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_saved_clear() {
    unsafe {
        (*REGISTRY.get()).road_saved.clear();
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_saved_current() {
    openttd_rust_road_saved_clear();
    let s = services();
    for rt in ROADTYPE_BEGIN..ROADTYPE_END {
        let label = (s.road().type_label)(rt);
        openttd_rust_road_saved_append(label, u8::from(crate::road::get_road_tram_type(rt).0));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_mask() -> *mut u64 {
    unsafe { &raw mut (*REGISTRY.get()).road }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_tram_mask() -> *mut u64 {
    unsafe { &raw mut (*REGISTRY.get()).tram }
}
/// Call-scoped alias into the file's canonical translation list. Loading cannot
/// reset or mutate the list until the caller has finished reading the alias.
pub(crate) fn rail_labels(file: Handle) -> Labels {
    (services().rail_labels)(file)
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_translate(rt: u8, file: Handle) -> u8 {
    let s = services();
    let labels = if file.is_null() {
        &[][..]
    } else {
        (s.rail_labels)(file).slice()
    };
    if labels.is_empty() {
        if rt >= RAILTYPE_END || (s.rail().type_label)(rt) == 0 {
            INVALID_RAILTYPE
        } else {
            rt
        }
    } else {
        labels
            .get(usize::from(rt))
            .map_or(INVALID_RAILTYPE, |label| {
                openttd_rust_rail_by_label(*label, true)
            })
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_translate(rtt: RoadTramType, rt: u8, file: Handle) -> u8 {
    if file.is_null() {
        return INVALID_ROADTYPE;
    }
    let s = services();
    let labels = if rtt.0 {
        (s.tram_labels)(file)
    } else {
        (s.road_labels)(file)
    }
    .slice();
    let result = labels
        .get(usize::from(rt))
        .map_or(INVALID_ROADTYPE, |label| {
            openttd_rust_road_by_label(*label, true)
        });
    if result != INVALID_ROADTYPE && crate::road::get_road_tram_type(result).0 != rtt.0 {
        INVALID_ROADTYPE
    } else {
        result
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_reverse(rt: u8, file: Handle) -> u8 {
    if file.is_null() {
        return rt;
    }
    let s = services();
    let labels = (s.rail_labels)(file).slice();
    if labels.is_empty() {
        return rt;
    }
    let label = (s.rail().type_label)(rt);
    labels
        .iter()
        .position(|v| *v == label)
        .map_or(0xff, |index| index as u8)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_reverse(rt: u8, file: Handle) -> u8 {
    if file.is_null() {
        return rt;
    }
    let s = services();
    let labels = if crate::road::road_type_is_road(rt) {
        (s.road_labels)(file)
    } else {
        (s.tram_labels)(file)
    }
    .slice();
    if labels.is_empty() {
        return rt;
    }
    let label = (s.road().type_label)(rt);
    labels
        .iter()
        .position(|v| *v == label)
        .map_or(0xff, |index| index as u8)
}
crate::abi::layouts! {
    fn abi_layout;
 "OpenTTDTypePublication" ["engine"] => Publication { label, hidden };
 "OpenTTDTypeLabelTable" ["engine"] => LabelTable { values };
 "OpenTTDTypeAllocation" ["engine"] => Allocation { label, self_ => self, order };
 "OpenTTDTypeLabels" ["engine"] => Labels { data, size };
 "OpenTTDTypeMap" ["engine"] => TypeMap { data, size };
 "OpenTTDRailRegistryServices" ["engine"] => RailServices { default_count, spec, type_label, labels, alternate, clear, copy_default, resolve_gui, publication, sort, publish_sorted, allocate };
 "OpenTTDRoadRegistryServices" ["engine"] => RoadServices { default_count, spec, type_label, labels, alternate, clear, copy_default, resolve_gui, publication, sort, publish_sorted, allocate };
 "OpenTTDTypeRegistryServices" ["engine"] => Services { rail, road, rail_labels, road_labels, tram_labels, rail_map, road_map, tram_map };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_saved_convert(
    shared: *const crate::services::Services,
    map: *const crate::services::RailMapServices,
    count: u32,
) {
    use crate::{
        tile_type::{MP_RAILWAY, MP_ROAD, MP_STATION, MP_TUNNELBRIDGE},
        transport_type::TRANSPORT_RAIL,
    };
    let s = unsafe { &*shared };
    let map = unsafe { &*map };
    let mut conversion = Vec::new();
    let mut needed = false;
    let saved_count = unsafe { (*REGISTRY.get()).rail_saved.len() };
    for index in 0..saved_count {
        let (label, _) = unsafe { (&(*REGISTRY.get()).rail_saved)[index] };
        let mut rt = openttd_rust_rail_by_label(label, true);
        if rt == INVALID_RAILTYPE {
            rt = RAILTYPE_RAIL;
        }
        conversion.push(rt);
        if label != 0 && usize::from(rt) != index {
            needed = true;
        }
    }
    if !needed {
        return;
    }
    for tile in crate::map_func::Map::iterate(count) {
        let convert = match (s.tile_type)(tile) {
            MP_RAILWAY => true,
            MP_ROAD => (s.is_level_crossing)(tile),
            MP_STATION => (map.has_station_rail)(tile),
            MP_TUNNELBRIDGE => (s.tunnel_transport)(tile) == TRANSPORT_RAIL,
            _ => false,
        };
        if convert {
            (map.set_rail_type)(tile, conversion[usize::from((map.rail_type)(tile).0)]);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_saved_convert(
    shared: *const crate::services::Services,
    count: u32,
) {
    use crate::{
        tile_type::{MP_ROAD, MP_STATION, MP_TUNNELBRIDGE},
        transport_type::TRANSPORT_ROAD,
    };
    let s = unsafe { &*shared };
    let mut conversion = Vec::new();
    let mut needed = false;
    let saved_count = unsafe { (*REGISTRY.get()).road_saved.len() };
    for index in 0..saved_count {
        let (label, subtype) = unsafe { (&(*REGISTRY.get()).road_saved)[index] };
        let mut rt = openttd_rust_road_by_label(label, true);
        if rt == INVALID_ROADTYPE || u8::from(crate::road::get_road_tram_type(rt).0) != subtype {
            rt = if subtype != 0 {
                ROADTYPE_TRAM
            } else {
                ROADTYPE_ROAD
            };
        }
        conversion.push(rt);
        if label != 0 && usize::from(rt) != index {
            needed = true;
        }
    }
    if !needed {
        return;
    }
    for tile in crate::map_func::Map::iterate(count) {
        let convert = match (s.tile_type)(tile) {
            MP_ROAD => true,
            MP_STATION => (s.is_any_road_stop)(tile),
            MP_TUNNELBRIDGE => (s.tunnel_transport)(tile) == TRANSPORT_ROAD,
            _ => false,
        };
        if !convert {
            continue;
        }
        let rt = (s.road_type)(tile);
        if rt != INVALID_ROADTYPE {
            crate::road_map::set_road_type_road(s, tile, conversion[usize::from(rt)]);
        }
        let rt = (s.tram_type)(tile).0;
        if rt != INVALID_ROADTYPE {
            crate::road_map::set_road_type_tram(s, tile, conversion[usize::from(rt)]);
        }
    }
}

impl Services {
    fn rail(&self) -> &RailServices {
        unsafe { &*self.rail }
    }
    fn road(&self) -> &RoadServices {
        unsafe { &*self.road }
    }
}

pub(crate) fn rail_spec(rt: u8) -> Handle {
    (services().rail().spec)(rt)
}
pub(crate) fn road_spec(rt: u8) -> Handle {
    (services().road().spec)(rt)
}

fn publish_rail_sorted() {
    let (data, count) = unsafe {
        let sorted = &mut (*REGISTRY.get()).rail_sorted;
        (sorted.as_mut_ptr(), sorted.len())
    };
    (services().rail().publish_sorted)(data, count);
}

fn publish_road_sorted() {
    let (data, count) = unsafe {
        let sorted = &mut (*REGISTRY.get()).road_sorted;
        (sorted.as_mut_ptr(), sorted.len())
    };
    (services().road().publish_sorted)(data, count);
}
