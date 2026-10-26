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
use crate::{
    game_state::GameState,
    rail_type::{
        INVALID_RAILTYPE, RAILTYPE_ELECTRIC, RAILTYPE_END, RAILTYPE_MAGLEV, RAILTYPE_MONO,
        RAILTYPE_RAIL,
    },
    road_type::{INVALID_ROADTYPE, ROADTYPE_END, ROADTYPE_ROAD, ROADTYPE_TRAM},
};
use std::{ffi::c_void, ptr};
type Handle = *const c_void;
#[repr(C)]
pub struct Publication {
    label: u32,
    hidden: bool,
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
    fn slice(self) -> &'static [u32] {
        if self.size == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.data, self.size) }
        }
    }
}
#[repr(C)]
pub struct TypeMap {
    pub(crate) data: *mut u8,
    pub(crate) size: usize,
}
#[repr(C)]
pub struct RailServices {
    spec: extern "C" fn(u8) -> Handle,
    label: extern "C" fn(Handle) -> u32,
    alternate: extern "C" fn(Handle, u32) -> bool,
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
    spec: extern "C" fn(u8) -> Handle,
    label: extern "C" fn(Handle) -> u32,
    alternate: extern "C" fn(Handle, u32) -> bool,
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
pub fn rail_hidden() -> u64 {
    unsafe { (*REGISTRY.get()).rail_hidden }
}
pub fn road_hidden() -> u64 {
    unsafe { (*REGISTRY.get()).road_hidden }
}
pub fn road_tram_mask(rtt: u8) -> u64 {
    unsafe {
        if rtt == 0 {
            (*REGISTRY.get()).road
        } else {
            (*REGISTRY.get()).tram
        }
    }
}
#[must_use]
pub fn road_tram_type(rt: u8) -> u8 {
    u8::from(road_tram_mask(1) & (1u64 << rt) != 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_hidden_mask() -> *mut u64 {
    unsafe { &raw mut (*REGISTRY.get()).rail_hidden }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_reset(s: *const Services, defaults: u8) {
    unsafe {
        (*REGISTRY.get()).services = s;
    }
    let s = services();
    for rt in 0..RAILTYPE_END {
        let info = (s.rail().spec)(rt);
        if rt < defaults {
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
    for rt in 0..RAILTYPE_END {
        if (s.rail().label)((s.rail().spec)(rt)) == label {
            return rt;
        }
    }
    if alternate {
        for rt in 0..RAILTYPE_END {
            if (s.rail().alternate)((s.rail().spec)(rt), label) {
                return rt;
            }
        }
    }
    INVALID_RAILTYPE
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_allocate(label: u32) -> u8 {
    let s = services();
    for rt in 0..RAILTYPE_END {
        let info = (s.rail().spec)(rt);
        if (s.rail().label)(info) != 0 {
            continue;
        }
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
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_init() {
    let s = services();
    unsafe {
        (*REGISTRY.get()).rail_sorted.clear();
    }
    for rt in 0..RAILTYPE_END {
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
    for rt in 0..RAILTYPE_END {
        let label = (s.rail().label)((s.rail().spec)(rt));
        openttd_rust_rail_saved_append(label, 0);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_hidden_mask() -> *mut u64 {
    unsafe { &raw mut (*REGISTRY.get()).road_hidden }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_reset(s: *const Services, defaults: u8) {
    unsafe {
        (*REGISTRY.get()).services = s;
    }
    let s = services();
    for rt in 0..ROADTYPE_END {
        let info = (s.road().spec)(rt);
        if rt < defaults {
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
    for rt in 0..ROADTYPE_END {
        if (s.road().label)((s.road().spec)(rt)) == label {
            return rt;
        }
    }
    if alternate {
        for rt in 0..ROADTYPE_END {
            if (s.road().alternate)((s.road().spec)(rt), label) {
                return rt;
            }
        }
    }
    INVALID_ROADTYPE
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_allocate(label: u32, rtt: u8) -> u8 {
    let s = services();
    for rt in 0..ROADTYPE_END {
        let info = (s.road().spec)(rt);
        if (s.road().label)(info) != 0 {
            continue;
        }
        (s.road().copy_default)(
            info,
            if rtt == 1 {
                ROADTYPE_TRAM
            } else {
                ROADTYPE_ROAD
            },
        );
        let value = Allocation {
            label,
            self_: 1u64 << rt,
            order: rt.wrapping_shl(2) | 7,
        };
        (s.road().allocate)(info, &raw const value);
        unsafe {
            let bit = 1u64 << rt;
            (*REGISTRY.get()).road =
                ((*REGISTRY.get()).road & !bit) | if rtt == 0 { bit } else { 0 };
            (*REGISTRY.get()).tram =
                ((*REGISTRY.get()).tram & !bit) | if rtt == 1 { bit } else { 0 };
        }
        return rt;
    }
    INVALID_ROADTYPE
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_init() {
    let s = services();
    unsafe {
        (*REGISTRY.get()).road_sorted.clear();
    }
    for rt in 0..ROADTYPE_END {
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
    for rt in 0..ROADTYPE_END {
        let label = (s.road().label)((s.road().spec)(rt));
        openttd_rust_road_saved_append(label, road_tram_type(rt));
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
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_translate(rt: u8, file: Handle) -> u8 {
    let s = services();
    let labels = if file.is_null() {
        &[][..]
    } else {
        (s.rail_labels)(file).slice()
    };
    if labels.is_empty() {
        if rt >= RAILTYPE_END || (s.rail().label)((s.rail().spec)(rt)) == 0 {
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
pub extern "C" fn openttd_rust_road_translate(rtt: u8, rt: u8, file: Handle) -> u8 {
    if file.is_null() {
        return INVALID_ROADTYPE;
    }
    let s = services();
    let labels = if rtt == 1 {
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
    if result != INVALID_ROADTYPE && road_tram_type(result) != rtt {
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
    let label = (s.rail().label)((s.rail().spec)(rt));
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
    let labels = if road_tram_mask(0) & (1u64 << rt) != 0 {
        (s.road_labels)(file)
    } else {
        (s.tram_labels)(file)
    }
    .slice();
    if labels.is_empty() {
        return rt;
    }
    let label = (s.road().label)((s.road().spec)(rt));
    labels
        .iter()
        .position(|v| *v == label)
        .map_or(0xff, |index| index as u8)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_type_maps_init(s: *const Services, file: Handle) {
    let s = unsafe { &*s };
    for (map, invalid, defaults) in [
        (
            (s.rail_map)(file),
            INVALID_RAILTYPE,
            &[
                RAILTYPE_RAIL,
                RAILTYPE_ELECTRIC,
                RAILTYPE_MONO,
                RAILTYPE_MAGLEV,
            ][..],
        ),
        ((s.road_map)(file), INVALID_ROADTYPE, &[ROADTYPE_ROAD][..]),
        ((s.tram_map)(file), INVALID_ROADTYPE, &[ROADTYPE_TRAM][..]),
    ] {
        let entries = unsafe { std::slice::from_raw_parts_mut(map.data, map.size) };
        entries.fill(invalid);
        entries[..defaults.len()].copy_from_slice(defaults);
    }
}
crate::abi::layouts! {
    fn abi_layout;
 "OpenTTDTypePublication" ["engine"] => Publication { label, hidden };
 "OpenTTDTypeAllocation" ["engine"] => Allocation { label, self_ => self, order };
 "OpenTTDTypeLabels" ["engine"] => Labels { data, size };
 "OpenTTDTypeMap" ["engine"] => TypeMap { data, size };
 "OpenTTDRailRegistryServices" ["engine"] => RailServices { spec, label, alternate, clear, copy_default, resolve_gui, publication, sort, publish_sorted, allocate };
 "OpenTTDRoadRegistryServices" ["engine"] => RoadServices { spec, label, alternate, clear, copy_default, resolve_gui, publication, sort, publish_sorted, allocate };
 "OpenTTDTypeRegistryServices" ["engine"] => Services { rail, road, rail_labels, road_labels, tram_labels, rail_map, road_map, tram_map };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_rail_saved_convert(
    shared: *const crate::services::Services,
    map: *const crate::services::RailMapServices,
) {
    use crate::{
        tile_type::{MP_RAILWAY, MP_ROAD, MP_STATION, MP_TUNNELBRIDGE},
        transport_type::TRANSPORT_RAIL,
    };
    let s = unsafe { &*shared };
    let map = unsafe { &*map };
    let mut conversion = Vec::new();
    let mut needed = false;
    let count = unsafe { (*REGISTRY.get()).rail_saved.len() };
    for index in 0..count {
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
    let count = ((s.map_max_x)() + 1) * ((s.map_max_y)() + 1);
    for tile in 0..count {
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
pub unsafe extern "C" fn openttd_rust_road_saved_convert(shared: *const crate::services::Services) {
    use crate::{
        tile_type::{MP_ROAD, MP_STATION, MP_TUNNELBRIDGE},
        transport_type::TRANSPORT_ROAD,
    };
    let s = unsafe { &*shared };
    let mut conversion = Vec::new();
    let mut needed = false;
    let count = unsafe { (*REGISTRY.get()).road_saved.len() };
    for index in 0..count {
        let (label, subtype) = unsafe { (&(*REGISTRY.get()).road_saved)[index] };
        let mut rt = openttd_rust_road_by_label(label, true);
        if rt == INVALID_ROADTYPE || road_tram_type(rt) != subtype {
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
    let count = ((s.map_max_x)() + 1) * ((s.map_max_y)() + 1);
    for tile in 0..count {
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
