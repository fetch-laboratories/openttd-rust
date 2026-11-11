/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! House and shared-town query runtime. Stack contexts live with the native Action2 facades.
use super::{BUILDING_COUNTS, Leaves, State, house};
use crate::cargo_type::{CargoLabel, CargoType, CargoTypes, is_valid_cargo_type};
use crate::debug::debug;
use crate::game_state::GameState;
use crate::history_type::{LAST_MONTH, THIS_MONTH};
use crate::math::{clamp_to_u8, clamp_to_u16};
use crate::newgrf_storage;
use crate::services::{Map, Services, TownHandle};
use crate::tile_type::{INVALID_TILE, TileIndex, TileType};
use std::{cell::OnceCell, ffi::c_void};

use crate::bitmath_func::gb;
use crate::cargotype::TownAcceptanceEffect;
use crate::house::HouseZone;
use crate::house_type::{HouseClassID, HouseID, HouseRandomTriggers};
use crate::newgrf_callbacks::CallbackID;
use crate::newgrf_spritegroup::{VSG_SCOPE_PARENT, VSG_SCOPE_SELF};

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
    pub(crate) shared: *const Services,
    storage: *const newgrf_storage::Services,
    larger_towns: extern "C" fn() -> u8,
    stage: extern "C" fn(u32) -> u8,
    age: extern "C" fn(u32) -> i32,
    random_bits: extern "C" fn(u32) -> u8,
    random_triggers: extern "C" fn(u32) -> u8,
    search_spec: extern "C" fn(u16) -> SearchSpec,
    nearby_spec: extern "C" fn(u16) -> NearbySpec,
    file_identity: extern "C" fn(u16) -> FileIdentity,
    badges: extern "C" fn(u16) -> crate::newgrf_specs::BadgeSpan,
    root: extern "C" fn(u16, bool) -> *const c_void,
    spec_house: extern "C" fn(*const c_void) -> u16,
    local_id: extern "C" fn(u16) -> u16,
    base_scope: extern "C" fn(*mut c_void, u8, u8) -> *mut c_void,
    pub(crate) evaluate: extern "C" fn(*const CallbackArgs, *mut i32, usize) -> u16,
    reseed: extern "C" fn(u16, u32, *mut TownHandle, u8) -> house::HouseReseed,
}
/// Test-only table for `crate::test_tables`; its private fields keep it in this module.
#[cfg(test)]
pub(crate) const TEST_TABLE: ResolverServices = ResolverServices {
    towns: std::ptr::null(),
    shared: std::ptr::null(),
    storage: std::ptr::null(),
    larger_towns: crate::test_tables::unreachable0,
    stage: crate::test_tables::unreachable1,
    age: crate::test_tables::unreachable1,
    random_bits: crate::test_tables::unreachable1,
    random_triggers: crate::test_tables::unreachable1,
    search_spec: crate::test_tables::unreachable1,
    nearby_spec: crate::test_tables::unreachable1,
    file_identity: crate::test_tables::unreachable1,
    badges: crate::test_tables::unreachable1,
    root: crate::test_tables::unreachable2,
    spec_house: crate::test_tables::unreachable1,
    local_id: crate::test_tables::unreachable1,
    base_scope: crate::test_tables::unreachable3,
    evaluate: crate::test_tables::unreachable3,
    reseed: crate::test_tables::unreachable4,
};
impl ResolverServices {
    pub(crate) fn shared(&self) -> &Services {
        // SAFETY: `GetHouseResolverServices` points at the process-lifetime shared table.
        unsafe { &*self.shared }
    }
}
/// Makes class IDs unique to each GRF file (`HouseClassMapping`).
#[derive(Clone, Copy, Default)]
struct HouseClassMapping {
    /// The GRF ID of the file this class belongs to.
    grfid: u32,
    /// The class id within the grf file.
    class_id: u8,
}
/// Original `_class_mapping`.
static CLASSES: GameState<Vec<HouseClassMapping>> = GameState::new(Vec::new());
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_house_classes_reset() {
    reset_house_classes();
}
/// `ResetHouseClassIDs`.
pub(crate) fn reset_house_classes() {
    unsafe {
        let classes = &mut *CLASSES.get();
        classes.clear();
        // Add initial entry for HOUSE_NO_CLASS.
        classes.push(HouseClassMapping::default());
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_house_class_allocate(grf_class_id: u8, grfid: u32) -> HouseClassID {
    allocate_house_class_id(grf_class_id, grfid)
}
/// `AllocateHouseClassID`.
pub(crate) fn allocate_house_class_id(grf_class_id: u8, grfid: u32) -> HouseClassID {
    unsafe {
        let classes = &mut *CLASSES.get();
        // Start from 1 because 0 means that no class has been assigned.
        if let Some(index) = classes
            .iter()
            .skip(1)
            .position(|map| map.class_id == grf_class_id && map.grfid == grfid)
        {
            return (index + 1) as HouseClassID;
        }
        // HouseClass not found, allocate a new one.
        let index = classes.len();
        classes.push(HouseClassMapping {
            grfid,
            class_id: grf_class_id,
        });
        index as HouseClassID
    }
}
/// Initialise building counts for a town (`InitializeBuildingCounts(Town *)`).
unsafe fn initialize_counts(owner: *mut State, specs: usize, classes: usize) {
    let owner = unsafe { &mut *owner };
    owner.id_counts.clear();
    owner.class_counts.clear();
    owner.id_counts.resize(specs, 0);
    owner.class_counts.resize(classes, 0);
}
/// Initialise global building counts and all town building counts
/// (`InitializeBuildingCounts()`), or only `owner`'s counts when it is not null.
pub(crate) unsafe fn initialize_building_counts(
    l: &Leaves,
    s: &Services,
    owner: *mut State,
    specs: usize,
) {
    let classes = unsafe { (*CLASSES.get()).len() };
    if !owner.is_null() {
        unsafe { initialize_counts(owner, specs, classes) };
        return;
    }
    unsafe {
        let counts = &mut *BUILDING_COUNTS.get();
        counts.ids.clear();
        counts.classes.clear();
        counts.ids.resize(specs, 0);
        counts.classes.resize(classes, 0);
    };
    for id in super::iterate(|from| l.next_town(from)) {
        let owner = (s.town_owner_by_id)(u32::from(id.0));
        unsafe { initialize_counts(owner, specs, classes) };
    }
}
/// `GetNumHouses`: direct canonical counts; all four `ClampTo<uint8_t>` precede packing.
unsafe fn get_num_houses(owner: *const State, house_id: HouseID, class_id: HouseClassID) -> u32 {
    let (town_id_count, town_class_count) = unsafe {
        let state = &*owner;
        (
            state.id_counts[usize::from(house_id)],
            state.class_counts[usize::from(class_id)],
        )
    };
    let (map_id_count, map_class_count) = unsafe {
        let c = &*BUILDING_COUNTS.get();
        (
            c.ids[usize::from(house_id)],
            c.classes[usize::from(class_id)],
        )
    };
    u32::from(clamp_to_u8(i64::from(map_class_count))) << 24
        | u32::from(clamp_to_u8(i64::from(town_class_count))) << 16
        | u32::from(clamp_to_u8(i64::from(map_id_count))) << 8
        | u32::from(clamp_to_u8(i64::from(town_id_count)))
}
/// `TownHistoryHelper`: lookup is performed after the shared label translation.
unsafe fn town_history_helper(
    owner: *const State,
    cargo: CargoType,
    period: u8,
    transported: bool,
) -> u32 {
    unsafe { super::supplied_history(owner, cargo, period) }.map_or(0, |h| {
        u32::from(clamp_to_u16(i64::from(if transported {
            h.transported
        } else {
            h.production
        })))
    })
}
/// # Safety
/// As `town_get_variable`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_variable(
    s: &ResolverServices,
    scope: *const TownScope,
    ro: *const c_void,
    variable: u8,
    parameter: u32,
    available: *mut bool,
) -> u32 {
    unsafe { town_get_variable(s, scope, ro, variable, parameter, available) }
}
/// `TownScopeResolver::GetVariable`.
pub(crate) unsafe fn town_get_variable(
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
    // Get a variable from the persistent storage; the shared storage reads the
    // GrfID stored in register 100h.
    if variable == 0x7C {
        let file = (s.shared().resolver_grf_file)(ro);
        return unsafe {
            newgrf_storage::openttd_rust_storage_town_get(
                &*s.storage,
                scope.town.cast(),
                file,
                parameter,
            )
        };
    }
    // Larger towns.
    if variable == 0x40 {
        if (s.larger_towns)() == 0 {
            return 2;
        }
        return u32::from(unsafe { (*scope.owner).simulation.larger_town });
    }
    // Passenger and mail production and transport, this and last month.
    if (0xBA..=0xC9).contains(&variable) {
        let index = variable - 0xBA;
        let cargo = crate::cargo_registry::get_cargo_type_by_label(if index & 2 == 0 {
            CargoLabel::CT_PASSENGERS
        } else {
            CargoLabel::CT_MAIL
        });
        let value = unsafe {
            town_history_helper(
                scope.owner,
                cargo,
                if index < 8 { THIS_MONTH } else { LAST_MONTH },
                index & 4 != 0,
            )
        };
        return if index & 1 == 0 { value } else { value >> 8 };
    }
    if variable == 0xCA || variable == 0xCB {
        let cargo = crate::cargo_registry::get_cargo_type_by_label(if variable == 0xCA {
            CargoLabel::CT_PASSENGERS
        } else {
            CargoLabel::CT_MAIL
        });
        return u32::from(unsafe { super::get_percent_transported(scope.owner, cargo) });
    }
    let value = unsafe {
        let state = &*scope.owner;
        match variable {
            // Town index.
            0x41 => u32::from(scope.id),
            // Town properties.
            0x80 => state.simulation.xy,
            0x81 => gb(state.simulation.xy, 8, 8),
            0x82 | 0x83 => {
                u32::from(clamp_to_u16(i64::from(state.simulation.population)))
                    >> (8 * (variable & 1))
            }
            0x8A => u32::from(state.growth.counter / TOWN_GROWTH_TICKS),
            // In original game, 0x92 and 0x93 are one word.
            0x92 => u32::from(state.growth.flags),
            0x93 => 0,
            // Squared zone radii from `HouseZone::TownEdge` to `HouseZone::TownCentre`.
            0x94..=0x9D => {
                u32::from(clamp_to_u16(i64::from(
                    state.simulation.radius
                        [usize::from(HouseZone::TownEdge.0 + (variable - 0x94) / 2)],
                ))) >> (8 * (variable & 1))
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
            0xB6 => u32::from(clamp_to_u16(i64::from(state.simulation.num_houses))),
            0xB9 => u32::from(state.growth.rate / TOWN_GROWTH_TICKS),
            0xCC..=0xD3 => {
                let index = variable - 0xCC;
                let effect = if index & 2 == 0 {
                    TownAcceptanceEffect::TAE_FOOD
                } else {
                    TownAcceptanceEffect::TAE_WATER
                };
                let received = state.simulation.received[usize::from(effect.0)];
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
                debug!(grf, 1, "Unhandled town variable 0x{:X}", variable);
                *available = false;
                return u32::MAX;
            }
        }
    };
    // Only the high-byte aliases take GB(..., 8, 8); full words retain their widths.
    if matches!(
        variable,
        0x83 | 0x95 | 0x97 | 0x99 | 0x9B | 0x9D | 0xCD | 0xCF | 0xD1 | 0xD3
    ) {
        gb(value, 0, 8)
    } else {
        value
    }
}
/// # Safety
/// As `town_store_psa`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_scope_store(
    s: &ResolverServices,
    scope: *const TownScope,
    ro: *const c_void,
    pos: u32,
    value: i32,
) {
    unsafe { town_store_psa(s, scope, ro, pos, value) }
}
pub(crate) unsafe fn town_store_psa(
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
    let file = (s.shared().resolver_grf_file)(ro);
    unsafe {
        newgrf_storage::openttd_rust_storage_town(
            &*s.storage,
            scope.town.cast(),
            scope.readonly,
            file,
            pos,
            value,
        );
    }
}
/// # Safety
/// As `house_random_bits`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_random(
    s: &ResolverServices,
    scope: *const HouseScope,
    triggers: bool,
) -> u32 {
    unsafe { house_random_bits(s, scope, triggers) }
}
/// `HouseScopeResolver::GetRandomBits` and `GetRandomTriggers`.
pub(crate) unsafe fn house_random_bits(
    s: &ResolverServices,
    scope: *const HouseScope,
    triggers: bool,
) -> u32 {
    let scope = unsafe { *scope };
    // Note: Towns build houses over houses. So during construction checks 'tile' may be a
    // valid but unrelated house.
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
/// Activate a search around a central tile, looking for houses that fit the requested
/// characteristics (`GetDistanceFromNearbyHouse`). `parameter` bits 0..5 are the radius
/// of the search and bits 6..7 the search type: 0 = houseID, 1 = classID, 2 = grfID.
/// Return the Manhattan distance from the center tile, if any, and 0 if failure.
unsafe fn get_distance_from_nearby_house(
    s: &ResolverServices,
    parameter: u8,
    start_tile: TileIndex,
    start_house: HouseID,
    map: Map,
) -> u32 {
    let searchtype = gb(u32::from(parameter), 6, 2);
    let searchradius = gb(u32::from(parameter), 0, 6);
    // do not use a too low radius
    if searchradius < 1 {
        return 0;
    }
    let start_hs = (s.search_spec)(start_house);
    let mut start_house = start_house;
    let leaves = unsafe { &*s.towns };
    // modifies 'start_house'!
    let start_north_tile = start_tile
        .0
        .wrapping_add_signed(house::get_house_north_part(leaves, &mut start_house, map));
    let size_y = crate::map_func::World::new(unsafe { &*s.shared }).size_y();
    let mut spiral = crate::spiral::square(
        map.tile_x(start_tile),
        map.tile_y(start_tile),
        2 * searchradius + 1,
        map.size_x(),
        size_y,
    );
    while !spiral.is_end() {
        let tile = map.tile_xy(spiral.x, spiral.y);
        if TileType((unsafe { &*s.shared }.tile_type)(tile.0)) == TileType::MP_HOUSE {
            let mut house = (unsafe { &*leaves.houses }.house_type)(tile.0);
            let hs = (s.search_spec)(house);
            // must be one from a grf file
            // Always ignore origin house
            if hs.has_file
                && start_north_tile
                    != tile
                        .0
                        .wrapping_add_signed(house::get_house_north_part(leaves, &mut house, map))
            {
                let matched = match searchtype {
                    0 => {
                        hs.local == start_hs.local // same local id as the one requested
                            && hs.grfid == start_hs.grfid // from the same grf
                    }
                    1 => {
                        hs.house_class == start_hs.house_class // same classid as the one requested
                            && hs.grfid == start_hs.grfid // from the same grf
                    }
                    2 => hs.grfid == start_hs.grfid, // from the same grf
                    _ => return 0,
                };
                if matched {
                    return map.distance_manhattan(tile, start_tile);
                }
            }
        }
        spiral = spiral.advance(map.size_x(), size_y);
    }
    0
}
/// # Safety
/// As `house_get_variable`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_variable(
    s: &ResolverServices,
    scope: *const HouseScope,
    ro: *const c_void,
    variable: u8,
    parameter: u32,
    available: *mut bool,
) -> u32 {
    unsafe { house_get_variable(s, scope, ro, variable, parameter, available) }
}
/// `HouseScopeResolver::GetVariable`, used by the resolver to get values for feature 07
/// deterministic spritegroups.
pub(crate) unsafe fn house_get_variable(
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
        // House does not yet exist, nor is it being planned to exist. Provide some default values instead.
        return match variable {
            // Construction stage.
            0x40 => u32::from(TOWN_HOUSE_COMPLETED) | (scope.view as u32).wrapping_shl(2),
            0x41..=0x44 | 0x46 | 0x47 | 0x60..=0x65 | 0x67 => 0,
            0x45 => u32::from(l.generating_world()),
            // Class and ID of nearby house.
            0x66 => u32::MAX,
            0x7A => (s.badges)(scope.house).resolve(s.shared(), ro, parameter),
            _ => {
                debug!(grf, 1, "Unhandled house variable 0x{:X}", variable);
                unsafe { *available = false };
                u32::MAX
            }
        };
    }
    let dimensions = OnceCell::new();
    let map = || *dimensions.get_or_init(|| Map::from_log_x((l.map_log_x)()));
    let tile = scope.tile;
    let is_house = |tile: u32| TileType((shared.tile_type)(tile)) == TileType::MP_HOUSE;
    match variable {
        // Construction stage.
        0x40 => {
            let stage = if is_house(tile) {
                u32::from((s.stage)(tile))
            } else {
                0
            };
            stage
                | crate::tile_map::tile_hash_2bit(
                    map().tile_x(TileIndex(tile)),
                    map().tile_y(TileIndex(tile)),
                ) << 2
        }
        // Building age.
        0x41 => {
            if is_house(tile) {
                (s.age)(tile) as u32
            } else {
                0
            }
        }
        // Town zone.
        0x42 => {
            let map = map();
            u32::from(
                unsafe { super::get_town_radius_group(scope.town.owner, TileIndex(tile), map) }.0,
            )
        }
        // Terrain type.
        0x43 => (shared.terrain_type)(tile),
        // Number of this type of building on the map.
        0x44 => unsafe {
            get_num_houses(scope.town.owner, scope.house, (l.house_class)(scope.house))
        },
        // Whether the town is being created or just expanded.
        0x45 => u32::from(l.generating_world()),
        // Current animation frame.
        0x46 => {
            if is_house(tile) {
                u32::from(crate::animated_tile_map::animation_frame(
                    crate::animated_tile::services(),
                    tile,
                ))
            } else {
                0
            }
        }
        // Position of the house.
        0x47 => map().tile_y(TileIndex(tile)) << 16 | map().tile_x(TileIndex(tile)),
        // Building counts for old houses with id = parameter.
        0x60 => {
            if parameter < u32::from(NEW_HOUSE_OFFSET) {
                unsafe {
                    get_num_houses(
                        scope.town.owner,
                        parameter as HouseID,
                        (l.house_class)(parameter as HouseID),
                    )
                }
            } else {
                0
            }
        }
        // Building counts for new houses with id = parameter.
        0x61 => {
            let spec = (s.file_identity)(scope.house);
            if !spec.has_file {
                return 0;
            }
            let house = unsafe {
                crate::override_manager::get(
                    crate::override_manager::house(),
                    parameter as HouseID,
                    spec.grfid,
                )
            };
            if house == INVALID_HOUSE_ID {
                0
            } else {
                unsafe { get_num_houses(scope.town.owner, house, (l.house_class)(house)) }
            }
        }
        // Land info for nearby tiles.
        0x62 => (shared.nearby_tile_information)(
            (shared.nearby_tile)(parameter as u8, tile),
            (shared.grf_file_version)((s.shared().resolver_grf_file)(ro)) >= 8,
        ),
        // Current animation frame of nearby house tiles.
        0x63 => {
            let tile = (shared.nearby_tile)(parameter as u8, tile);
            if is_house(tile) {
                u32::from(crate::animated_tile_map::animation_frame(
                    crate::animated_tile::services(),
                    tile,
                ))
            } else {
                0
            }
        }
        // Cargo acceptance history of nearby stations
        0x64 => unsafe { acceptance_history(s, scope, ro, parameter, map()) },
        // Distance test for some house types.
        0x65 => unsafe {
            get_distance_from_nearby_house(
                s,
                parameter as u8,
                TileIndex(scope.tile),
                scope.house,
                map(),
            )
        },
        // Class and ID of nearby house tile.
        0x66 => {
            let testtile = (shared.nearby_tile)(parameter as u8, tile);
            if !is_house(testtile) {
                return u32::MAX;
            }
            let nearby_house_id = (unsafe { &*l.houses }.house_type)(testtile);
            let hs = (s.nearby_spec)(nearby_house_id);
            let file = (s.shared().resolver_grf_file)(ro);
            let same_file = if hs.file == file { 1 } else { 2 };
            // Information about the grf local classid if the house has a class.
            let houseclass = if hs.house_class == HOUSE_NO_CLASS {
                0
            } else {
                let local_class =
                    unsafe { (&*CLASSES.get())[usize::from(hs.house_class)].class_id };
                same_file << 8 | u32::from(local_class)
            };
            // Old house type or grf-local houseid.
            let local_houseid = if nearby_house_id < NEW_HOUSE_OFFSET {
                u32::from(nearby_house_id)
            } else {
                // Spec only allows 8 bits, so all local-ids above 254 are clamped.
                same_file << 8 | u32::from(clamp_to_u8(i64::from(hs.local)))
            };
            houseclass << 16 | local_houseid
        }
        // GRFID of nearby house tile.
        0x67 => {
            let testtile = (shared.nearby_tile)(parameter as u8, tile);
            if !is_house(testtile) {
                return u32::MAX;
            }
            let house = (unsafe { &*l.houses }.house_type)(testtile);
            if house < NEW_HOUSE_OFFSET {
                0
            } else {
                // Checking the grffile information via HouseSpec doesn't work in case the
                // newgrf was removed.
                unsafe {
                    crate::override_manager::openttd_rust_override_grfid(
                        crate::override_manager::house(),
                        house,
                    )
                }
            }
        }
        0x7A => (s.badges)(scope.house).resolve(s.shared(), ro, parameter),
        _ => {
            debug!(grf, 1, "Unhandled house variable 0x{:X}", variable);
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
    let cargo = crate::cargo_registry::get_cargo_translation(
        parameter as u8,
        (s.shared().resolver_grf_file)(ro),
        false,
    );
    if !is_valid_cargo_type(cargo.0) {
        return 0;
    }
    // Extract tile offset. ResolverObject::GetRegister uses this process-wide canonical
    // temporary storage. The fixed 0x100 index satisfies its original <0x110 precondition.
    let register = unsafe { newgrf_storage::openttd_rust_storage_temp_get(0x100) } as u32;
    let x_offs = gb(register, 0, 8) as i8;
    let y_offs = gb(register, 8, 8) as i8;
    let offset = map
        .diff(i32::from(x_offs), i32::from(y_offs))
        .cast_unsigned();
    let shared = unsafe { &*s.shared };
    let tile = map
        .wrap_to_map(
            TileIndex(scope.tile.wrapping_add(offset)),
            crate::map_func::World::new(shared).size_y(),
        )
        .0;
    let c = crate::station::Ctx::from_shared(shared);
    let mut stations =
        crate::station::StationFinder::new(crate::tilearea::TileArea { tile, w: 1, h: 1 });
    let list = std::ptr::from_ref(stations.get_stations(&c));
    // Collect acceptance stats.
    let mut result = 0;
    // SAFETY: The call-local finder's set; each visit reads canonical status only.
    unsafe {
        crate::station::for_each_station(list, |near| {
            let ge = crate::station_service::goods((c.c.owner)(near.station), cargo);
            // Read only canonical status, then use the shared pure conversion.
            result |= u32::from(crate::station_base::goods_entry::convert_state(
                crate::station_base::goods_entry::States((*ge).status),
            ));
        });
    }
    // Cargo triggered CB 148?
    if scope.watched & (1_u64 << cargo.0) != 0 {
        crate::bitmath_func::set_bit(&mut result, 4);
    }
    result
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
        towns, shared, storage, larger_towns, stage, age, random_bits,
        random_triggers, search_spec, nearby_spec, file_identity,
        badges, root, spec_house, local_id, base_scope, evaluate, reseed, };
}

/// Immutable process-lifetime native leaves, installed on house-class reset
/// before any callback.
static REGISTRY: GameState<*const ResolverServices> = GameState::new(std::ptr::null());
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_resolver_exchange(
    s: *const ResolverServices,
) -> *const ResolverServices {
    REGISTRY.exchange(s)
}
fn services<'a>() -> &'a ResolverServices {
    // SAFETY: ResetHouses installs before use. Table is immutable and process-live.
    unsafe { REGISTRY.table() }
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
/// `GetHouseCallback`: direct owner entry for lifecycle, production, acceptance and animation.
pub(super) unsafe fn get_house_callback(
    callback: CallbackID,
    param1: u32,
    param2: u32,
    house_id: HouseID,
    town: *mut TownHandle,
    tile: TileIndex,
    not_yet_constructed: bool,
) -> u16 {
    let args = CallbackArgs {
        town,
        watched: 0,
        param1,
        param2,
        tile: tile.0,
        view: 0,
        house: house_id,
        callback: callback.0,
        initial_random: 0,
        prospective: not_yet_constructed,
    };
    unsafe { openttd_rust_house_callback(&raw const args, std::ptr::null_mut(), 0) }
}
/// Native random reseeding of a house tile and its waiting triggers.
pub(super) unsafe fn reseed_house_tile(
    house: HouseID,
    tile: TileIndex,
    town: *mut TownHandle,
    waiting: HouseRandomTriggers,
) -> house::HouseReseed {
    (services().reseed)(house, tile.0, town, waiting.0)
}
/// `GetHouseCallback(CBID_HOUSE_ALLOW_CONSTRUCTION, ...)` with `Convert8bitBooleanCallback`
/// (`TryBuildTownHouse`): whether the house may be built here.
pub(super) unsafe fn allow_construction(
    l: &Leaves,
    town: crate::town_type::TownID,
    tile: TileIndex,
    house: HouseID,
    random_bits: u8,
) -> bool {
    let town = (unsafe { &*services().shared }.town_by_id)(u32::from(town.0));
    let args = CallbackArgs {
        town,
        watched: 0,
        param1: 0,
        param2: 0,
        tile: tile.0,
        view: 0,
        house,
        callback: CBID_HOUSE_ALLOW_CONSTRUCTION.0,
        initial_random: random_bits,
        prospective: true,
    };
    let result = unsafe { openttd_rust_house_callback(&raw const args, std::ptr::null_mut(), 0) };
    result == crate::newgrf_callbacks::CALLBACK_FAILED
        || (unsafe { &*l.houses }.convert_8bit)(house, CBID_HOUSE_ALLOW_CONSTRUCTION.0, result)
}
/// `CBID_HOUSE_DRAW_FOUNDATIONS` for the house on `tile`, in the tile's town.
pub(crate) unsafe fn foundation(tile: TileIndex, house: HouseID) -> u16 {
    let s = services();
    let town =
        (unsafe { &*s.shared }.town_by_id)(u32::from((unsafe { &*s.shared }.town_index)(tile.0)));
    unsafe { get_house_callback(CBID_HOUSE_DRAW_FOUNDATIONS, 0, 0, house, town, tile, false) }
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
    house_get_scope(ro, house, town, scope, relative)
}
pub(crate) fn house_get_scope(
    ro: *mut c_void,
    house: *mut c_void,
    town: *mut c_void,
    scope: u8,
    relative: u8,
) -> *mut c_void {
    match scope {
        VSG_SCOPE_SELF => house,
        VSG_SCOPE_PARENT => town,
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
    town_get_scope(ro, town, scope, relative)
}
pub(crate) fn town_get_scope(
    ro: *mut c_void,
    town: *mut c_void,
    scope: u8,
    relative: u8,
) -> *mut c_void {
    if scope == VSG_SCOPE_SELF {
        town
    } else {
        (services().base_scope)(ro, scope, relative)
    }
}

/// # Safety
/// As `get_house_animation_callback`.
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
    unsafe {
        get_house_animation_callback(
            CallbackID(callback),
            param1,
            param2,
            spec,
            town,
            TileIndex(tile),
            CargoTypes(watched),
        )
    }
}
/// Original `GetSimpleHouseCallback`, as `HouseAnimationBase` calls it.
pub(crate) unsafe fn get_house_animation_callback(
    callback: CallbackID,
    param1: u32,
    param2: u32,
    spec: *const c_void,
    town: *mut TownHandle,
    tile: TileIndex,
    watched: CargoTypes,
) -> u16 {
    let house = (services().spec_house)(spec);
    let args = CallbackArgs {
        town,
        watched: watched.0,
        param1,
        param2,
        tile: tile.0,
        view: 0,
        house,
        callback: callback.0,
        initial_random: 0,
        prospective: false,
    };
    unsafe { openttd_rust_house_callback(&raw const args, std::ptr::null_mut(), 0) }
}
