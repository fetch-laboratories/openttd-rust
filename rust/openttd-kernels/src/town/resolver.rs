/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! House and shared-town query runtime. Stack contexts live with native Action2
//! facades, which retain no parallel state. Native game-state serialization covers
//! every entry. No owner/container borrow survives a leaf, evaluator or PSA call.
use super::{BUILDING_COUNTS, GameState, Leaves, State, house};
use crate::cargo_type::{CT_MAIL, CT_PASSENGERS, is_valid_cargo_type};
use crate::history_type::{LAST_MONTH, THIS_MONTH};
use crate::newgrf_storage;
use crate::services::{Map, Services, TownHandle};
use crate::tile_type::{INVALID_TILE, MP_HOUSE};
use std::{
    cell::{OnceCell, UnsafeCell},
    ffi::c_void,
};

use crate::house::{HOUSE_NO_CLASS, INVALID_HOUSE_ID, NEW_HOUSE_OFFSET, TOWN_HOUSE_COMPLETED};
use crate::newgrf_callbacks::{CBID_HOUSE_ALLOW_CONSTRUCTION, CBID_HOUSE_DRAW_FOUNDATIONS};
use crate::timer_game_tick::TOWN_GROWTH_TICKS;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TownScope {
    town: *mut TownHandle,
    owner: *mut State,
    id: u16,
    readonly: bool,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseScope {
    town: TownScope,
    watched: u64,
    tile: u32,
    view: i32,
    house: u16,
    initial_random: u16,
    prospective: bool,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SearchSpec {
    grfid: u32,
    local: u16,
    house_class: u16,
    has_file: bool,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FileIdentity {
    grfid: u32,
    has_file: bool,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NearbySpec {
    file: *const c_void,
    local: u16,
    house_class: u16,
}
#[repr(C)]
pub struct ResolverServices {
    towns: *const Leaves,
    shared: *const Services,
    storage: *const newgrf_storage::Services,
    larger_towns: extern "C" fn() -> u8,
    cargo_by_label: extern "C" fn(u32) -> u8,
    file: extern "C" fn(*const c_void) -> *const c_void,
    version: extern "C" fn(*const c_void) -> u8,
    cargo_translation: extern "C" fn(u8, *const c_void) -> u8,
    nearby: extern "C" fn(u8, u32) -> u32,
    nearby_info: extern "C" fn(u32, bool) -> u32,
    terrain: extern "C" fn(u32) -> u32,
    stage: extern "C" fn(u32) -> u8,
    age: extern "C" fn(u32) -> i32,
    random_bits: extern "C" fn(u32) -> u8,
    random_triggers: extern "C" fn(u32) -> u8,
    search_spec: extern "C" fn(u16) -> SearchSpec,
    nearby_spec: extern "C" fn(u16) -> NearbySpec,
    file_identity: extern "C" fn(u16) -> FileIdentity,
    house_id: extern "C" fn(u16, u32) -> u16,
    house_grfid: extern "C" fn(u16) -> u32,
    badges: extern "C" fn(*const c_void, u16, u32) -> u32,
    stations:
        extern "C" fn(u32, *mut c_void, extern "C" fn(*mut c_void, *const crate::station::Station)),
    town_by_id: extern "C" fn(u32) -> *mut TownHandle,
    root: extern "C" fn(u16, bool) -> *const c_void,
    spec_house: extern "C" fn(*const c_void) -> u16,
    local_id: extern "C" fn(u16) -> u16,
    base_scope: extern "C" fn(*mut c_void, u8, u8) -> *mut c_void,
    evaluate: extern "C" fn(*const CallbackArgs, *mut i32, usize) -> u16,
    reseed: extern "C" fn(u16, u32, *mut TownHandle, u8) -> house::HouseReseed,
    unhandled_house: extern "C" fn(u8),
    unhandled_town: extern "C" fn(u8),
}
#[derive(Clone, Copy, Default)]
struct ClassMapping {
    grfid: u32,
    class: u8,
}
static CLASSES: GameState<Vec<ClassMapping>> = GameState::new(Vec::new());
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_house_classes_reset() {
    CLASSES.with(|cell| unsafe {
        let classes = &mut *cell.get();
        classes.clear();
        classes.push(ClassMapping::default());
    });
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_house_class_allocate(class: u8, grfid: u32) -> u16 {
    CLASSES.with(|cell| unsafe {
        let classes = &mut *cell.get();
        if let Some(index) = classes
            .iter()
            .skip(1)
            .position(|c| c.class == class && c.grfid == grfid)
        {
            return (index + 1) as u16;
        }
        let index = classes.len();
        classes.push(ClassMapping { grfid, class });
        index as u16
    })
}
unsafe fn initialize_counts(owner: *mut State, specs: usize, classes: usize) {
    let owner = unsafe { &mut *owner };
    owner.id_counts.clear();
    owner.class_counts.clear();
    owner.id_counts.resize(specs, 0);
    owner.class_counts.resize(classes, 0);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_counts_init(
    l: &Leaves,
    s: &Services,
    owner: *mut State,
    specs: usize,
) {
    let classes = CLASSES.with(|cell| unsafe { (*cell.get()).len() });
    if !owner.is_null() {
        unsafe { initialize_counts(owner, specs, classes) };
        return;
    }
    BUILDING_COUNTS.with(|cell| unsafe {
        let counts = &mut *cell.get();
        counts.ids.clear();
        counts.classes.clear();
        counts.ids.resize(specs, 0);
        counts.classes.resize(classes, 0);
    });
    let mut id = (l.next_town)(0);
    while id != u32::MAX {
        let owner = (s.town_owner_by_id)(id);
        unsafe { initialize_counts(owner, specs, classes) };
        id = (l.next_town)(id + 1);
    }
}
/// `GetNumHouses`: direct canonical counts; all four clamps precede packing.
unsafe fn num_houses(owner: *const State, house: u16, class: u16) -> u32 {
    let (town_id, town_class) = unsafe {
        let state = &*owner;
        (
            state.id_counts[usize::from(house)],
            state.class_counts[usize::from(class)],
        )
    };
    let (map_id, map_class) = BUILDING_COUNTS.with(|cell| unsafe {
        let c = &*cell.get();
        (c.ids[usize::from(house)], c.classes[usize::from(class)])
    });
    map_class.min(255) << 24
        | u32::from(town_class.min(255)) << 16
        | map_id.min(255) << 8
        | u32::from(town_id.min(255))
}
/// `TownHistoryHelper`: lookup is performed after the shared label translation.
unsafe fn history(owner: *const State, cargo: u8, period: u8, transported: bool) -> u32 {
    unsafe { super::supplied_history(owner, cargo, period) }.map_or(0, |h| {
        (if transported {
            h.transported
        } else {
            h.production
        })
        .min(u32::from(u16::MAX))
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_variable(
    s: &ResolverServices,
    scope: *const TownScope,
    ro: *const c_void,
    variable: u8,
    parameter: u32,
    available: *mut bool,
) -> u32 {
    let scope = unsafe { *scope };
    if scope.town.is_null() {
        unsafe { *available = false };
        return u32::MAX;
    }
    if variable == 0x7C {
        let file = (s.file)(ro);
        return unsafe {
            newgrf_storage::openttd_rust_storage_town_get(
                &*s.storage,
                scope.town.cast(),
                file,
                parameter,
            )
        };
    }
    if variable == 0x40 {
        if (s.larger_towns)() == 0 {
            return 2;
        }
        return u32::from(unsafe { (*scope.owner).simulation.larger_town });
    }
    if (0xBA..=0xC9).contains(&variable) {
        let index = variable - 0xBA;
        let cargo = (s.cargo_by_label)(if index & 2 == 0 {
            CT_PASSENGERS
        } else {
            CT_MAIL
        });
        let value = unsafe {
            history(
                scope.owner,
                cargo,
                if index < 8 { THIS_MONTH } else { LAST_MONTH },
                index & 4 != 0,
            )
        };
        return if index & 1 == 0 { value } else { value >> 8 };
    }
    if variable == 0xCA || variable == 0xCB {
        let cargo = (s.cargo_by_label)(if variable == 0xCA {
            CT_PASSENGERS
        } else {
            CT_MAIL
        });
        return u32::from(unsafe { super::subsidy_percent_transported(scope.owner, cargo) });
    }
    let value = unsafe {
        let state = &*scope.owner;
        match variable {
            0x41 => u32::from(scope.id),
            0x80 => state.simulation.xy,
            0x81 => (state.simulation.xy >> 8) & 255,
            0x82 | 0x83 => state.simulation.population.min(65535) >> (8 * (variable & 1)),
            0x8A => u32::from(state.growth.counter / TOWN_GROWTH_TICKS),
            0x92 => u32::from(state.growth.flags),
            0x93 => 0,
            0x94..=0x9D => {
                state.simulation.radius[usize::from((variable - 0x94) / 2)].min(65535)
                    >> (8 * (variable & 1))
            }
            0x9E..=0xAD => {
                let rating = state.simulation.ratings[usize::from((variable - 0x9E) / 2)];
                if variable & 1 == 0 {
                    rating as u32
                } else {
                    u32::from((rating as u16) >> 8)
                }
            }
            0xAE => u32::from(state.simulation.have_ratings),
            0xB2 => u32::from(state.simulation.statues),
            0xB6 => state.simulation.num_houses.min(65535),
            0xB9 => u32::from(state.growth.rate / TOWN_GROWTH_TICKS),
            0xCC..=0xD3 => {
                let index = variable - 0xCC;
                let effect = if index & 2 == 0 {
                    crate::cargotype::TAE_FOOD
                } else {
                    crate::cargotype::TAE_WATER
                };
                let received = state.simulation.received[usize::from(effect)];
                let value = if index < 4 {
                    received.new_act
                } else {
                    received.old_act
                };
                u32::from(value) >> (8 * (index & 1))
            }
            0xD4 => u32::from(state.growth.road),
            0xD5 => u32::from(state.growth.funding),
            _ => {
                // End the canonical borrow before diagnostics below.
                (s.unhandled_town)(variable);
                *available = false;
                return u32::MAX;
            }
        }
    };
    // Only the high-byte aliases mask to eight bits; full words retain their widths.
    if matches!(
        variable,
        0x83 | 0x95 | 0x97 | 0x99 | 0x9B | 0x9D | 0xCD | 0xCF | 0xD1 | 0xD3
    ) {
        value & 255
    } else {
        value
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_scope_store(
    s: &ResolverServices,
    scope: *const TownScope,
    ro: *const c_void,
    pos: u32,
    value: i32,
) {
    let scope = unsafe { *scope };
    if scope.readonly {
        return;
    }
    let file = (s.file)(ro);
    unsafe {
        newgrf_storage::openttd_rust_storage_town(
            &*s.storage,
            scope.town.cast(),
            scope.readonly,
            file,
            pos,
            value,
        );
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_random(
    s: &ResolverServices,
    scope: *const HouseScope,
    triggers: bool,
) -> u32 {
    let scope = unsafe { *scope };
    if scope.prospective {
        return if triggers {
            0
        } else {
            u32::from(scope.initial_random)
        };
    }
    u32::from(if triggers {
        (s.random_triggers)(scope.tile)
    } else {
        (s.random_bits)(scope.tile)
    })
}
unsafe fn distance(s: &ResolverServices, scope: HouseScope, parameter: u8, map: Map) -> u32 {
    let kind = parameter >> 6;
    let radius = parameter & 63;
    if radius < 1 {
        return 0;
    }
    let start_spec = (s.search_spec)(scope.house);
    let mut start_house = scope.house;
    let leaves = unsafe { &*s.towns };
    let start_north = scope
        .tile
        .wrapping_add(house::north_part(leaves, &mut start_house, map));
    let size_y = (unsafe { &*s.shared }.map_max_y)() + 1;
    let mut spiral = crate::spiral::square(
        map.tile_x(scope.tile),
        map.tile_y(scope.tile),
        2 * u32::from(radius) + 1,
        map.size_x(),
        size_y,
    );
    while !spiral.is_end() {
        let tile = map.tile_xy(spiral.x, spiral.y);
        if (unsafe { &*s.shared }.tile_type)(tile) == MP_HOUSE {
            let mut house = (unsafe { &*leaves.houses }.house_type)(tile);
            let spec = (s.search_spec)(house);
            if spec.has_file
                && start_north != tile.wrapping_add(house::north_part(leaves, &mut house, map))
            {
                let matched = match kind {
                    0 => spec.local == start_spec.local && spec.grfid == start_spec.grfid,
                    1 => {
                        spec.house_class == start_spec.house_class && spec.grfid == start_spec.grfid
                    }
                    2 => spec.grfid == start_spec.grfid,
                    _ => return 0,
                };
                if matched {
                    return map.distance_manhattan(tile, scope.tile);
                }
            }
        }
        spiral = spiral.advance(map.size_x(), size_y);
    }
    0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_variable(
    s: &ResolverServices,
    scope: *const HouseScope,
    ro: *const c_void,
    variable: u8,
    parameter: u32,
    available: *mut bool,
) -> u32 {
    let scope = unsafe { *scope };
    let l = unsafe { &*s.towns };
    let shared = unsafe { &*s.shared };
    if scope.tile == INVALID_TILE {
        return match variable {
            0x40 => u32::from(TOWN_HOUSE_COMPLETED) | (scope.view as u32).wrapping_shl(2),
            0x41..=0x44 | 0x46 | 0x47 | 0x60..=0x65 | 0x67 => 0,
            0x45 => u32::from((l.generating_world)()),
            0x66 => u32::MAX,
            0x7A => (s.badges)(ro, scope.house, parameter),
            _ => {
                (s.unhandled_house)(variable);
                unsafe { *available = false };
                u32::MAX
            }
        };
    }
    let dimensions = OnceCell::new();
    let map = || *dimensions.get_or_init(|| Map::from_log_x((l.map_log_x)()));
    let tile = scope.tile;
    match variable {
        0x40 => {
            let stage = if (shared.tile_type)(tile) == MP_HOUSE {
                u32::from((s.stage)(tile))
            } else {
                0
            };
            stage | crate::tile_map::tile_hash_2bit(map().tile_x(tile), map().tile_y(tile)) << 2
        }
        0x41 => {
            if (shared.tile_type)(tile) == MP_HOUSE {
                (s.age)(tile) as u32
            } else {
                0
            }
        }
        0x42 => {
            let map = map();
            unsafe {
                let state = &*scope.town.owner;
                u32::from(super::radius_group(
                    state.growth.funding != 0,
                    &state.simulation.radius,
                    map.distance_square(tile, state.simulation.xy),
                ))
            }
        }
        0x43 => (s.terrain)(tile),
        0x44 => unsafe { num_houses(scope.town.owner, scope.house, (l.house_class)(scope.house)) },
        0x45 => u32::from((l.generating_world)()),
        0x46 => {
            if (shared.tile_type)(tile) == MP_HOUSE {
                u32::from(crate::animated_tile_map::animation_frame(
                    crate::animated_tile::services(),
                    tile,
                ))
            } else {
                0
            }
        }
        0x47 => map().tile_y(tile) << 16 | map().tile_x(tile),
        0x60 => {
            if parameter < u32::from(NEW_HOUSE_OFFSET) {
                unsafe {
                    num_houses(
                        scope.town.owner,
                        parameter as u16,
                        (l.house_class)(parameter as u16),
                    )
                }
            } else {
                0
            }
        }
        0x61 => {
            let spec = (s.file_identity)(scope.house);
            if !spec.has_file {
                return 0;
            }
            let house = (s.house_id)(parameter as u16, spec.grfid);
            if house == INVALID_HOUSE_ID {
                0
            } else {
                unsafe { num_houses(scope.town.owner, house, (l.house_class)(house)) }
            }
        }
        0x62 => (s.nearby_info)(
            (s.nearby)(parameter as u8, tile),
            (s.version)((s.file)(ro)) >= 8,
        ),
        0x63 => {
            let tile = (s.nearby)(parameter as u8, tile);
            if (shared.tile_type)(tile) == MP_HOUSE {
                u32::from(crate::animated_tile_map::animation_frame(
                    crate::animated_tile::services(),
                    tile,
                ))
            } else {
                0
            }
        }
        // Use the canonical GoodsEntry::ConvertState helper.
        0x64 => unsafe { acceptance_history(s, scope, ro, parameter, map()) },
        0x65 => unsafe { distance(s, scope, parameter as u8, map()) },
        0x66 => {
            let tile = (s.nearby)(parameter as u8, tile);
            if (shared.tile_type)(tile) != MP_HOUSE {
                return u32::MAX;
            }
            let house = (unsafe { &*l.houses }.house_type)(tile);
            let spec = (s.nearby_spec)(house);
            let file = (s.file)(ro);
            let same_file = if spec.file == file { 1 } else { 2 };
            let class = if spec.house_class == HOUSE_NO_CLASS {
                0
            } else {
                let local_class = CLASSES
                    .with(|cell| unsafe { (&*cell.get())[usize::from(spec.house_class)].class });
                same_file << 8 | u32::from(local_class)
            };
            let local = if house < NEW_HOUSE_OFFSET {
                u32::from(house)
            } else {
                same_file << 8 | u32::from(spec.local.min(255))
            };
            class << 16 | local
        }
        0x67 => {
            let tile = (s.nearby)(parameter as u8, tile);
            if (shared.tile_type)(tile) != MP_HOUSE {
                return u32::MAX;
            }
            let house = (unsafe { &*l.houses }.house_type)(tile);
            if house < NEW_HOUSE_OFFSET {
                0
            } else {
                (s.house_grfid)(house)
            }
        }
        0x7A => (s.badges)(ro, scope.house, parameter),
        _ => {
            (s.unhandled_house)(variable);
            unsafe { *available = false };
            u32::MAX
        }
    }
}
unsafe fn acceptance_history(
    s: &ResolverServices,
    scope: HouseScope,
    ro: *const c_void,
    parameter: u32,
    map: Map,
) -> u32 {
    struct Context {
        leaves: *const Leaves,
        cargo: u8,
        result: u32,
    }
    extern "C" fn visit(context: *mut c_void, station: *const crate::station::Station) {
        // SAFETY: Synchronous StationFinder traversal supplies live canonical shells.
        let (leaves, cargo) = unsafe {
            let c = &*context.cast::<Context>();
            (c.leaves, c.cargo)
        };
        let owner = (unsafe { &*leaves }.station_activity_owner)(station);
        let fields = unsafe {
            crate::station_service::openttd_rust_station_service_cargo(owner.cast_mut(), cargo)
        };
        let status = unsafe { (*fields).status };
        // Read only canonical status, then use the shared pure conversion.
        let state = crate::station_service::openttd_rust_goods_convert_state(status);
        unsafe { (*context.cast::<Context>()).result |= u32::from(state) };
    }
    let cargo = (s.cargo_translation)(parameter as u8, (s.file)(ro));
    if !is_valid_cargo_type(cargo) {
        return 0;
    }
    // ResolverObject::GetRegister uses this process-wide canonical temporary
    // storage. The fixed 0x100 index satisfies its original <0x110 precondition.
    let register = unsafe { newgrf_storage::openttd_rust_storage_temp_get(0x100) } as u32;
    let offset = map.diff(i32::from(register as i8), i32::from((register >> 8) as i8));
    let shared = unsafe { &*s.shared };
    let tile = map.wrap_to_map(scope.tile.wrapping_add(offset), (shared.map_max_y)() + 1);
    let mut context = Context {
        leaves: s.towns,
        cargo,
        result: 0,
    };
    (s.stations)(tile, (&raw mut context).cast(), visit);
    if scope.watched & (1_u64 << cargo) != 0 {
        context.result |= 1 << 4;
    }
    context.result
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDHouseCallbackArgs" ["probe"] => CallbackArgs { town, watched, param1, param2, tile, view, house, callback, initial_random, prospective };
    "OpenTTDTownScope" ["probe"] => TownScope { town, owner, id, readonly };
    "OpenTTDHouseScope" ["probe"] => HouseScope { town, watched, tile, view, house, initial_random, prospective };
    "OpenTTDHouseSearchSpec" ["probe"] => SearchSpec { grfid, local, house_class, has_file };
    "OpenTTDHouseFileIdentity" ["probe"] => FileIdentity { grfid, has_file };
    "OpenTTDHouseNearbySpec" ["probe"] => NearbySpec { file, local, house_class };
    "OpenTTDHouseResolverServices" ["probe"] => ResolverServices {
        towns, shared, storage, larger_towns, cargo_by_label, file, version,
        cargo_translation, nearby, nearby_info, terrain, stage, age, random_bits,
        random_triggers, search_spec, nearby_spec, file_identity, house_id, house_grfid,
        badges, stations, town_by_id, root, spec_house, local_id, base_scope, evaluate, reseed, unhandled_house, unhandled_town,
    };
}

/// Immutable process-lifetime native leaves, installed on house-class reset
/// before any callback. The native table contains no mutable world-state mirror.
struct Registry(UnsafeCell<*const ResolverServices>);
// SAFETY: Installation and all reads hold the native game-state mutex.
unsafe impl Sync for Registry {}
static REGISTRY: Registry = Registry(UnsafeCell::new(std::ptr::null()));
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_resolver_exchange(
    s: *const ResolverServices,
) -> *const ResolverServices {
    unsafe {
        let previous = *REGISTRY.0.get();
        *REGISTRY.0.get() = s;
        previous
    }
}
fn services<'a>() -> &'a ResolverServices {
    // SAFETY: ResetHouses installs before use. Table is immutable and process-live.
    unsafe { &**REGISTRY.0.get() }
}
#[repr(C)]
pub struct CallbackArgs {
    town: *mut TownHandle,
    watched: u64,
    param1: u32,
    param2: u32,
    tile: u32,
    view: i32,
    house: u16,
    callback: u16,
    initial_random: u8,
    prospective: bool,
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_callback(
    args: *const CallbackArgs,
    registers: *mut i32,
    count: usize,
) -> u16 {
    // Native leaf constructs only the stack virtual facade and enters common
    // Action2. No Rust owner/context reference spans its evaluation or reentry.
    (services().evaluate)(args, registers, count)
}
/// Direct owner entry for lifecycle, production, acceptance and animation.
pub(super) unsafe fn callback(
    callback: u16,
    param1: u32,
    param2: u32,
    house: u16,
    town: *mut TownHandle,
    tile: u32,
    prospective: bool,
) -> u16 {
    let args = CallbackArgs {
        town,
        watched: 0,
        param1,
        param2,
        tile,
        view: 0,
        house,
        callback,
        initial_random: 0,
        prospective,
    };
    unsafe { openttd_rust_house_callback(&raw const args, std::ptr::null_mut(), 0) }
}
pub(super) unsafe fn reseed(
    house: u16,
    tile: u32,
    town: *mut TownHandle,
    waiting: u8,
) -> house::HouseReseed {
    (services().reseed)(house, tile, town, waiting)
}
pub(super) unsafe fn allow_construction(
    l: &Leaves,
    town: u16,
    tile: u32,
    house: u16,
    initial_random: u8,
) -> bool {
    let town = (services().town_by_id)(u32::from(town));
    let args = CallbackArgs {
        town,
        watched: 0,
        param1: 0,
        param2: 0,
        tile,
        view: 0,
        house,
        callback: CBID_HOUSE_ALLOW_CONSTRUCTION.0,
        initial_random,
        prospective: true,
    };
    let result = unsafe { openttd_rust_house_callback(&raw const args, std::ptr::null_mut(), 0) };
    result == crate::newgrf_callbacks::CALLBACK_FAILED
        || (unsafe { &*l.houses }.convert_8bit)(house, CBID_HOUSE_ALLOW_CONSTRUCTION.0, result)
}
pub(crate) unsafe fn foundation(tile: u32, house: u16) -> u16 {
    let s = services();
    let town = (s.town_by_id)(u32::from((unsafe { &*s.shared }.town_index)(tile)));
    unsafe {
        callback(
            CBID_HOUSE_DRAW_FOUNDATIONS.0,
            0,
            0,
            house,
            town,
            tile,
            false,
        )
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_root(scope: *const HouseScope) -> *const c_void {
    let scope = unsafe { *scope };
    (services().root)(scope.house, !scope.prospective)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_debug_id(scope: *const HouseScope) -> u32 {
    let house = unsafe { (*scope).house };
    u32::from((services().local_id)(house))
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_house_scope(
    ro: *mut c_void,
    house: *mut c_void,
    town: *mut c_void,
    scope: u8,
    relative: u8,
) -> *mut c_void {
    match scope {
        0 => house,
        1 => town,
        _ => (services().base_scope)(ro, scope, relative),
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_town_scope(
    ro: *mut c_void,
    town: *mut c_void,
    scope: u8,
    relative: u8,
) -> *mut c_void {
    if scope == 0 {
        town
    } else {
        (services().base_scope)(ro, scope, relative)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_animation_callback(
    callback: u16,
    param1: u32,
    param2: u32,
    spec: *const c_void,
    town: *mut TownHandle,
    tile: u32,
    watched: u64,
) -> u16 {
    let house = (services().spec_house)(spec);
    let args = CallbackArgs {
        town,
        watched,
        param1,
        param2,
        tile,
        view: 0,
        house,
        callback,
        initial_random: 0,
        prospective: false,
    };
    unsafe { openttd_rust_house_callback(&raw const args, std::ptr::null_mut(), 0) }
}
