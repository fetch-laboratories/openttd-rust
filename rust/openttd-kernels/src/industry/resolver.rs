/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Industry and industry-tile resolvers (`newgrf_industries.cpp`, `newgrf_industrytiles.cpp`).
//! Contexts are stack values; industry state and the ordered registries are read directly,
//! and Action2, town and storage calls can reenter.
use super::{Builder, Industry, LifecycleServices, SpecHandle, TileServices, TownHandle};
use crate::bitmath_func::{gb, has_bit, set_bit};
use crate::cargo_registry::get_cargo_translation;
use crate::cargo_type::CargoType;
use crate::company_type::{CompanyID, Owner};
use crate::debug::debug;
use crate::history_type::{LAST_MONTH, THIS_MONTH};
use crate::industry_type::{IndustryGfx, IndustryID, IndustryType};
use crate::industrytype::{
    IndustryBehaviour, get_industry_spec, get_industry_tile_spec, spec_behaviour,
    spec_callback_mask,
};
use crate::newgrf_callbacks::{
    CALLBACK_FAILED, CallbackID, IndustryCallbackMask, IndustryCallbackMasks,
};
use crate::newgrf_industries::IndustryAvailabilityCallType;
use crate::newgrf_spritegroup::{VSG_SCOPE_PARENT, VSG_SCOPE_SELF};
use crate::tile_type::{TILE_HEIGHT, TileIndex};
use crate::timer_game_common::{DAYS_TILL_ORIGINAL_BASE_YEAR, ORIGINAL_BASE_YEAR};
use crate::town_type::TownID;
use crate::witness;
use std::{ffi::c_void, ptr};

/// The scope state shared by `IndustriesScopeResolver` and `IndustryTileScopeResolver`.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ResolverContext {
    pub owner: *mut Industry,
    pub industry: *mut c_void,
    pub town: *mut TownHandle,
    pub tile: u32,
    pub random_bits: u32,
    pub id: u16,
    pub kind: u8,
}
impl ResolverContext {
    /// `tile`: the tile related to the object.
    fn tile(&self) -> TileIndex {
        TileIndex(self.tile)
    }
    /// `industry->index`, or `IndustryID::Invalid()` for a construction check.
    fn index(&self) -> IndustryID {
        IndustryID(self.id)
    }
    /// `IndustriesScopeResolver::type`: the type of the industry.
    fn industry_type(&self) -> IndustryType {
        IndustryType(self.kind)
    }
}
#[repr(C)]
pub struct ResolverRef {
    pub owner: *mut Industry,
    pub town: *mut TownHandle,
}
#[repr(C)]
pub struct FounderInfo {
    pub colours: u8,
    pub is_ai: bool,
}
#[repr(C)]
pub struct ResolverServices {
    pub shared: *const crate::services::Services,
    pub lifecycle: *const LifecycleServices,
    pub tile: *const TileServices,
    pub get: unsafe extern "C" fn(u16) -> ResolverRef,
    pub clean_gfx: unsafe extern "C" fn(u32) -> u16,
    pub nearby: unsafe extern "C" fn(u8, u32, bool) -> u32,
    pub water_distance: unsafe extern "C" fn(u32, bool) -> u32,
    pub town_tile: unsafe extern "C" fn(*const crate::services::TownHandle) -> u32,
    pub town_id: unsafe extern "C" fn(*const crate::services::TownHandle) -> u16,
    pub founder_info: unsafe extern "C" fn(u8) -> FounderInfo,
    pub callback_id: unsafe extern "C" fn(*const c_void) -> u16,
    pub storage_owner:
        unsafe extern "C" fn(*mut c_void) -> *mut crate::newgrf_storage::Payload<256>,
    pub unhandled: unsafe extern "C" fn(u8, bool, bool, u16),
    pub base_scope: unsafe extern "C" fn(*mut c_void, u8, u8) -> *mut c_void,
    pub existing_town: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    pub make_town: unsafe extern "C" fn(*mut c_void, *mut TownHandle, bool) -> *mut c_void,
    pub callback: unsafe extern "C" fn(u16, u32, u32, *mut c_void, u8, u32) -> u16,
    pub callback_error: unsafe extern "C" fn(*const SpecHandle, u16, u16),
}
impl ResolverServices {
    fn shared(&self) -> &crate::services::Services {
        unsafe { &*self.shared }
    }
    pub(crate) fn lifecycle(&self) -> &LifecycleServices {
        unsafe { &*self.lifecycle }
    }
    fn tile(&self) -> &TileServices {
        unsafe { &*self.tile }
    }
    fn map(&self) -> crate::map_func::Map {
        self.shared().map()
    }
    /// `IsTileType(tile, MP_INDUSTRY)`.
    fn is_industry_tile(&self, tile: TileIndex) -> bool {
        crate::tile_map::is_tile_type(
            self.shared(),
            TileIndex(tile.0),
            crate::tile_type::TileType::MP_INDUSTRY,
        )
    }
    /// `Industry::TileBelongsToIndustry`, for the industry with this index.
    fn tile_belongs_to_industry(&self, tile: TileIndex, index: IndustryID) -> bool {
        unsafe { (self.lifecycle().belongs)(tile.0, index.0) }
    }
    /// `GetNearbyTile`.
    fn get_nearby_tile(&self, parameter: u8, tile: TileIndex, signed_offsets: bool) -> TileIndex {
        TileIndex(unsafe { (self.nearby)(parameter, tile.0, signed_offsets) })
    }
    /// `GetCleanIndustryGfx`.
    fn get_clean_industry_gfx(&self, tile: TileIndex) -> IndustryGfx {
        IndustryGfx(unsafe { (self.clean_gfx)(tile.0) })
    }
    /// `GetClosestWaterDistance`.
    fn get_closest_water_distance(&self, tile: TileIndex, water: bool) -> u32 {
        unsafe { (self.water_distance)(tile.0, water) }
    }
    /// `GetAnimationFrame` of an industry tile.
    fn get_animation_frame(&self, tile: TileIndex) -> u8 {
        unsafe { (self.tile().frame)(tile.0) }
    }
    /// `GetIndustryRandomBits`.
    fn get_industry_random_bits(&self, tile: TileIndex) -> u8 {
        unsafe { (self.lifecycle().random_bits)(tile.0) }
    }
    /// `ResolverObject::callback` of the native resolver.
    fn resolver_callback(&self, ro: *const c_void) -> CallbackID {
        CallbackID(unsafe { (self.callback_id)(ro) })
    }
}
/// `Industry::location.tile`.
fn location_tile(i: *const Industry) -> TileIndex {
    TileIndex(unsafe { (*i).metadata.tile })
}
/// `Town::xy`.
fn town_xy(town: *const TownHandle, s: &ResolverServices) -> TileIndex {
    TileIndex(unsafe { (s.town_tile)(town.cast()) })
}
/// `Town::index`.
fn town_index(town: *const TownHandle, s: &ResolverServices) -> TownID {
    TownID(unsafe { (s.town_id)(town.cast()) })
}

/// `MapNewGRFIndustryType`: map the GRF local type `grf_type` of the GRF `grf_id` to an
/// industry type in the global scope.
#[allow(
    clippy::cast_possible_truncation,
    reason = "MapNewGRFIndustryType keeps the original's implicit C++ integer conversions"
)]
pub(crate) fn map_new_grf_industry_type(grf_type: IndustryType, grf_id: u32) -> IndustryType {
    if grf_type == IndustryType::IT_INVALID {
        return IndustryType::IT_INVALID;
    }
    if !has_bit(grf_type.0, 7) {
        return IndustryType(gb(u32::from(grf_type.0), 0, 7) as u8);
    }
    IndustryType(
        (unsafe {
            crate::override_manager::industry_id(gb(u32::from(grf_type.0), 0, 7) as u16, grf_id)
        }) as u8,
    )
}
/// `GetIndustryIDAtOffset`: make an analysis of a tile and check for its belonging to the
/// same industry, and/or the same grf file. `tile` is the tile to query, `i` the industry to
/// compare the tile to and `cur_grfid` the GRFID of the current callback chain. Returns the
/// value encoded as per NFO specs.
fn get_industry_id_at_offset(
    tile: TileIndex,
    i: IndustryID,
    cur_grfid: u32,
    s: &ResolverServices,
) -> u32 {
    if !s.tile_belongs_to_industry(tile, i) {
        // No industry and/or the tile does not have the same industry as the one we match it with
        return 0xFFFF;
    }
    let gfx = s.get_clean_industry_gfx(tile);
    let indtsp = unsafe { (*get_industry_tile_spec(gfx)).grf_prop };
    // Does it belongs to an old type?
    if gfx < IndustryGfx::NEW_INDUSTRYTILEOFFSET {
        // It is an old tile.  We have to see if it's been overridden
        // has it been overridden?
        if IndustryGfx(indtsp.override_id) == IndustryGfx::INVALID_INDUSTRYTILE {
            // no. Tag FF + the gfx id of that tile
            return 0xFF << 8 | u32::from(gfx.0);
        }
        // Overridden
        let tile_ovr =
            unsafe { (*get_industry_tile_spec(IndustryGfx(indtsp.override_id))).grf_prop };
        return if tile_ovr.grfid == cur_grfid {
            // same grf file
            u32::from(tile_ovr.local_id)
        } else {
            // not the same grf file
            0xFFFE
        };
    }
    // Not an 'old type' tile
    if indtsp.has_sprite_groups() {
        // same industry, same grf ?
        if indtsp.grfid == cur_grfid {
            u32::from(indtsp.local_id)
        } else {
            // Defined in another grf file
            0xFFFE
        }
    } else {
        // The tile has no spritegroup
        // so just give it the substitute
        0xFF << 8 | u32::from(indtsp.subst_id)
    }
}
/// `GetNearbyIndustryTileInformation`: based on newhouses equivalent, but adapted for
/// newindustries. `parameter` from the callback is in fact a pair of coordinates; `tile` is the
/// tile from which the callback was initiated and `index` the industry being queried for.
/// `signed_offsets` tells whether the x and y offsets encoded in `parameter` are signed, and
/// `grf_version8` whether we are dealing with a new `NewGRF` which uses GRF version >= 8.
/// Returns a construction of bits obeying the newgrf format.
fn get_nearby_industry_tile_information(
    parameter: u8,
    mut tile: TileIndex,
    index: IndustryID,
    signed_offsets: bool,
    grf_version8: bool,
    s: &ResolverServices,
) -> u32 {
    // only perform if it is required
    if parameter != 0 {
        tile = s.get_nearby_tile(parameter, tile, signed_offsets);
    }
    let is_same_industry = s.tile_belongs_to_industry(tile, index);
    (s.shared().nearby_tile_information)(tile.0, grf_version8) | (u32::from(is_same_industry) << 8)
}
/// `GetRelativePosition`: the position of `tile` relative to `ind_tile`, the northernmost tile
/// of the industry.
///
/// ```text
/// Format: 00yxYYXX
/// Variable  Content
/// x         the x offset from the northernmost tile
/// XX        same, but stored in a byte instead of a nibble
/// y         the y offset from the northernmost tile
/// YY        same, but stored in a byte instead of a nibble
/// ```
#[allow(
    clippy::cast_possible_truncation,
    reason = "GetRelativePosition keeps the original's implicit C++ integer conversions"
)]
pub(crate) fn get_relative_position(
    tile: TileIndex,
    ind_tile: TileIndex,
    map: crate::map_func::Map,
) -> u32 {
    let x = map.tile_x(tile).wrapping_sub(map.tile_x(ind_tile)) as u8;
    let y = map.tile_y(tile).wrapping_sub(map.tile_y(ind_tile)) as u8;
    (u32::from(y & 0xF) << 20) | (u32::from(x & 0xF) << 16) | (u32::from(y) << 8) | u32::from(x)
}
/// `Industry::industries[type].size()`.
unsafe fn industries_of_type_count(builder: *const Builder, kind: IndustryType) -> usize {
    unsafe { (&(*builder).registry)[usize::from(kind.0)].len() }
}
/// `Industry::industries[type][n]`.
unsafe fn industries_of_type_at(
    builder: *const Builder,
    kind: IndustryType,
    n: usize,
) -> IndustryID {
    unsafe { (&(*builder).registry)[usize::from(kind.0)][n] }
}
/// `GetClosestIndustry`.
fn get_closest_industry(
    tile: TileIndex,
    kind: IndustryType,
    current: &ResolverContext,
    builder: *const Builder,
    s: &ResolverServices,
    map: crate::map_func::Map,
) -> u32 {
    let mut best_dist = u32::MAX;
    for n in 0..unsafe { industries_of_type_count(builder, kind) } {
        let industry = unsafe { industries_of_type_at(builder, kind, n) };
        if industry == current.index() {
            continue;
        }
        let other = unsafe { (s.get)(industry.0) };
        best_dist = best_dist.min(map.distance_manhattan(tile, location_tile(other.owner)));
    }
    best_dist
}
/// `GetCountAndDistanceOfClosestInstance`: implementation of both var 67 and 68, since the
/// mechanism is almost the same, it is easier to regroup them on the same function.
/// `param_set_id` is the parameter given to the callback, which is the set id, or the local
/// id, in our terminology. `layout_filter` is the layout to filter on, `town_filter` whether
/// to filter on the same town as the current industry, and `current` the industry for which
/// the inquiry is made. Returns the formatted answer to the callback: rr(reserved) cc(count)
/// dddd(manhattan distance of closest sister).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "GetCountAndDistanceOfClosestInstance keeps the original's implicit C++ integer conversions"
)]
fn get_count_and_distance_of_closest_instance(
    param_set_id: u8,
    layout_filter: u8,
    town_filter: bool,
    current: &ResolverContext,
    builder: *const Builder,
    s: &ResolverServices,
) -> u32 {
    // Get the GRFID of the definition to look for in register 100h
    let mut grf_id = unsafe { crate::newgrf_storage::openttd_rust_storage_temp_get(0x100) } as u32;
    // Determine what will be the industry type to look for
    let industry_type = if grf_id == 0 {
        // this is a default industry type
        IndustryType(param_set_id)
    } else {
        // current grf
        if grf_id == 0xFFFF_FFFF {
            grf_id = unsafe {
                (*get_industry_spec(IndustryType((*current.owner).metadata.kind)))
                    .grf_prop
                    .grfid
            };
        }
        // use the grfid specified in register 100h
        let mut param_set_id = u32::from(param_set_id);
        // bit 7 means it is not an old type
        set_bit(&mut param_set_id, 7);
        map_new_grf_industry_type(IndustryType(param_set_id as u8), grf_id)
    };
    // If the industry type is invalid, there is none and the closest is far away.
    if industry_type >= IndustryType::NUM_INDUSTRYTYPES {
        return 0xFFFF;
    }
    let map = s.map();
    let tile = location_tile(current.owner);
    let mut count: u8 = 0;
    let mut closest_dist = u32::MAX;
    if layout_filter == 0 && !town_filter {
        // If the filter is 0, it could be because none was specified as well as being really a 0.
        // In either case, just do the regular var67
        witness::hit(witness::INDUSTRY_RESOLVER_COUNT);
        closest_dist = get_closest_industry(tile, industry_type, current, builder, s, map);
        count = unsafe { industries_of_type_count(builder, industry_type) }
            .min(usize::from(u8::MAX)) as u8;
    } else {
        // Count only those who match the same industry type and layout filter
        // Unfortunately, we have to do it manually
        witness::hit(witness::INDUSTRY_RESOLVER_FILTERED_COUNT);
        for n in 0..unsafe { industries_of_type_count(builder, industry_type) } {
            let industry = unsafe { industries_of_type_at(builder, industry_type, n) };
            if industry == current.index() {
                continue;
            }
            let i = unsafe { (s.get)(industry.0) };
            if (layout_filter == 0
                || unsafe { (*i.owner).metadata.selected_layout } == layout_filter)
                && (!town_filter || i.town == current.town)
            {
                closest_dist =
                    closest_dist.min(map.distance_manhattan(tile, location_tile(i.owner)));
                count = count.wrapping_add(1);
            }
        }
    }
    (u32::from(count) << 16) | gb(closest_dist, 0, 16)
}
/// The `Debug(grf, 1, "Unhandled ...")` tail of the scope resolvers: report the
/// unhandled variable, mark it unavailable and return `UINT_MAX`.
fn unhandled_variable(
    variable: u8,
    available: *mut bool,
    tile: bool,
    missing: bool,
    callback: CallbackID,
    s: &ResolverServices,
) -> u32 {
    unsafe {
        (s.unhandled)(variable, tile, missing, callback.0);
        *available = false;
    }
    u32::MAX
}
/// `IndustriesScopeResolver::GetVariable`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "IndustriesScopeResolver::GetVariable keeps the original's implicit C++ integer conversions"
)]
#[allow(
    clippy::too_many_lines,
    reason = "IndustriesScopeResolver::GetVariable is ported as the original's single body; its length is tracked in the Clippy debt ledger"
)]
fn industries_get_variable(
    c: &ResolverContext,
    ro: *const c_void,
    variable: u8,
    parameter: u32,
    available: *mut bool,
    builder: *const Builder,
    s: &ResolverServices,
) -> u32 {
    witness::hit(witness::INDUSTRY_RESOLVER_VARIABLE);
    let callback = s.resolver_callback(ro);
    let i = c.owner;
    if callback == CallbackID::CBID_INDUSTRY_LOCATION {
        // Variables available during construction check.
        witness::hit(witness::INDUSTRY_RESOLVER_LOCATION);
        match variable {
            0x7A => {
                let badges = unsafe { (*get_industry_spec(c.industry_type())).badges.badge_view() };
                return badges.resolve(s.shared(), ro, parameter);
            }
            0x80 => return c.tile().0,
            0x81 => return gb(c.tile().0, 8, 8),
            // Pointer to the town the industry is associated with
            0x82 => return u32::from(town_index(c.town, s).0),
            // not supported
            0x83..=0x85 => debug!(grf, 0, "NewGRFs shouldn't be doing pointer magic"),
            // Number of the layout
            0x86 => return u32::from(unsafe { (*i).metadata.selected_layout }),
            // Ground type
            0x87 => return (s.shared().terrain_type)(c.tile().0),
            // Town zone
            0x88 => {
                return u32::from(
                    unsafe {
                        crate::town::get_town_radius_group(
                            (s.shared().town_owner)(c.town),
                            c.tile(),
                            s.shared().map(),
                        )
                    }
                    .0,
                );
            }
            // Manhattan distance of the closest town
            0x89 => {
                return s
                    .map()
                    .distance_manhattan(town_xy(c.town, s), c.tile())
                    .min(u32::from(u8::MAX));
            }
            // Lowest height of the tile
            0x8A => {
                return (s.shared().tile_z)(c.tile().0)
                    .wrapping_mul(if (s.shared().resolver_grf_version)(ro) >= 8 {
                        1
                    } else {
                        TILE_HEIGHT as i32
                    })
                    .clamp(0, i32::from(u8::MAX)) as u32;
            }
            // Distance to the nearest water/land tile
            0x8B => {
                return s.get_closest_water_distance(
                    c.tile(),
                    !spec_behaviour(get_industry_spec(IndustryType(unsafe {
                        (*i).metadata.kind
                    })))
                    .test(IndustryBehaviour::BuiltOnWater),
                );
            }
            // Square of Euclidean distance from town
            0x8D => {
                return s
                    .map()
                    .distance_square(town_xy(c.town, s), c.tile())
                    .min(u32::from(u16::MAX));
            }
            // 32 random bits
            0x8F => return c.random_bits,
            _ => {}
        }
    }
    let indspec = get_industry_spec(c.industry_type());
    if i.is_null() {
        return unhandled_variable(variable, available, false, true, callback, s);
    }
    match variable {
        // waiting cargo, but only if those two callback flags are set
        0x40..=0x42 => {
            let callback = spec_callback_mask(indspec);
            if !callback.any(
                IndustryCallbackMasks::from_element(IndustryCallbackMask::ProductionCargoArrival)
                    .with(IndustryCallbackMasks::from_element(
                        IndustryCallbackMask::Production256Ticks,
                    )),
            ) {
                return 0;
            }
            let multiplier = if spec_behaviour(indspec).test(IndustryBehaviour::ProdMultiHandling) {
                let level = unsafe { (*i).fields.prod_level };
                if level == 0 {
                    return 0;
                }
                level
            } else {
                1
            };
            u32::from(
                unsafe { (*super::get_accepted(i, usize::from(variable - 0x40))).waiting }
                    / u16::from(multiplier),
            )
        }
        // Manhattan distance of closes dry/water tile
        0x43 if c.tile() != TileIndex::INVALID => s.get_closest_water_distance(
            c.tile(),
            !spec_behaviour(indspec).test(IndustryBehaviour::BuiltOnWater),
        ),
        // Layout number
        0x44 => u32::from(unsafe { (*i).metadata.selected_layout }),
        // Company info
        0x45 => {
            let founder: Owner = CompanyID(unsafe { (*i).metadata.founder });
            let c = unsafe { (s.founder_info)(founder.0) };
            u32::from(founder.0) | (u32::from(c.is_ai) << 16) | (u32::from(c.colours) << 24)
        }
        // Date when built - long format - (in days)
        0x46 => (unsafe { (*i).metadata.construction_date }) as u32,
        // Override flags from GS
        0x47 => u32::from(unsafe { (*i).fields.ctlflags }),
        // Get industry ID at offset param
        0x60 => get_industry_id_at_offset(
            s.get_nearby_tile(parameter as u8, location_tile(i), false),
            c.index(),
            (s.shared().resolver_grf_file_id)(ro),
            s,
        ),
        // Get random tile bits at offset param
        0x61 if c.tile() != TileIndex::INVALID => {
            let tile = s.get_nearby_tile(parameter as u8, c.tile(), false);
            if s.tile_belongs_to_industry(tile, c.index()) {
                u32::from(s.get_industry_random_bits(tile))
            } else {
                0
            }
        }
        // Land info of nearby tiles
        0x62 if c.tile() != TileIndex::INVALID => get_nearby_industry_tile_information(
            parameter as u8,
            c.tile(),
            IndustryID::INVALID,
            false,
            (s.shared().resolver_grf_version)(ro) >= 8,
            s,
        ),
        // Animation stage of nearby tiles
        0x63 if c.tile() != TileIndex::INVALID => {
            let tile = s.get_nearby_tile(parameter as u8, c.tile(), false);
            if s.tile_belongs_to_industry(tile, c.index()) {
                u32::from(s.get_animation_frame(tile))
            } else {
                0xFFFF_FFFF
            }
        }
        // Distance of nearest industry of given type
        0x64 if c.tile() != TileIndex::INVALID => {
            let kind = map_new_grf_industry_type(IndustryType(parameter as u8), unsafe {
                (*indspec).grf_prop.grfid
            });
            if kind >= IndustryType::NUM_INDUSTRYTYPES {
                u32::MAX
            } else {
                get_closest_industry(c.tile(), kind, c, builder, s, s.map())
            }
        }
        // Get town zone and Manhattan distance of closest town
        0x65 if c.tile() != TileIndex::INVALID => {
            let tile = s.get_nearby_tile(parameter as u8, c.tile(), true);
            (u32::from(
                unsafe {
                    crate::town::get_town_radius_group(
                        (s.shared().town_owner)(c.town),
                        tile,
                        s.shared().map(),
                    )
                }
                .0,
            ) << 16)
                | s.map()
                    .distance_manhattan(tile, town_xy(c.town, s))
                    .min(u32::from(u16::MAX))
        }
        // Get square of Euclidean distance of closest town
        0x66 if c.tile() != TileIndex::INVALID => {
            let tile = s.get_nearby_tile(parameter as u8, c.tile(), true);
            s.map().distance_square(tile, town_xy(c.town, s))
        }
        // Count of industry, distance of closest instance
        // 68 is the same as 67, but with a filtering on selected layout
        0x67 | 0x68 => {
            let mut layout_filter = 0;
            let mut town_filter = false;
            if variable == 0x68 {
                let reg =
                    (unsafe { crate::newgrf_storage::openttd_rust_storage_temp_get(0x101) }) as u32;
                layout_filter = gb(reg, 0, 8) as u8;
                town_filter = has_bit(reg, 8);
            }
            get_count_and_distance_of_closest_instance(
                parameter as u8,
                layout_filter,
                town_filter,
                c,
                builder,
                s,
            )
        }
        0x69..=0x6D | 0x70 | 0x71 => {
            let cargo: CargoType =
                get_cargo_translation(parameter as u8, (s.shared().resolver_grf_file)(ro), false);
            let Some(it) = (unsafe { super::get_cargo_produced(i, cargo) }) else {
                return 0; // invalid cargo
            };
            let it = unsafe { &*super::produced(i, it) };
            match variable {
                0x69 => u32::from(it.waiting),
                0x6A => u32::from(it.history[usize::from(THIS_MONTH)].first),
                0x6B => u32::from(it.history[usize::from(THIS_MONTH)].second),
                0x6C => u32::from(it.history[usize::from(LAST_MONTH)].first),
                0x6D => u32::from(it.history[usize::from(LAST_MONTH)].second),
                0x70 => u32::from(it.rate),
                _ => u32::from(it.history[usize::from(LAST_MONTH)].pct_transported()),
            }
        }
        0x6E | 0x6F => {
            let cargo: CargoType =
                get_cargo_translation(parameter as u8, (s.shared().resolver_grf_file)(ro), false);
            let Some(it) = (unsafe { super::get_cargo_accepted(i, cargo) }) else {
                return 0; // invalid cargo
            };
            let it = unsafe { &*super::accepted(i, it) };
            if variable == 0x6E {
                it.last_accepted as u32
            } else {
                u32::from(it.waiting)
            }
        }
        0x7A => {
            let badges = unsafe { (*get_industry_spec(c.industry_type())).badges.badge_view() };
            badges.resolve(s.shared(), ro, parameter)
        }
        // Get a variable from the persistent storage
        0x7C => {
            // Resolve the native shell association once, then read the canonical
            // Rust payload directly. Temporary invalid-ID scopes can read PSA;
            // the invalid-ID gate belongs only to the storage write operation.
            let psa = unsafe { (s.storage_owner)(c.industry) };
            if psa.is_null() {
                0
            } else {
                (unsafe { crate::newgrf_storage::openttd_rust_storage256_get(psa, parameter) })
                    as u32
            }
        }
        // Industry structure access
        0x80 => location_tile(i).0,
        0x81 => gb(location_tile(i).0, 8, 8),
        // Pointer to the town the industry is associated with
        0x82 => u32::from(town_index(c.town, s).0),
        // not supported
        0x83..=0x85 => {
            debug!(grf, 0, "NewGRFs shouldn't be doing pointer magic");
            unhandled_variable(variable, available, false, false, callback, s)
        }
        // xy dimensions
        0x86 => u32::from(unsafe { (*i).metadata.width }),
        0x87 => u32::from(unsafe { (*i).metadata.height }),
        0x88 | 0x89 => {
            u32::from(unsafe { (*super::get_produced(i, usize::from(variable - 0x88))).cargo })
        }
        // 0x8A and 0x8C read the waiting cargo of slots 0 and 1; 0x8B and 0x8D its high byte.
        0x8A..=0x8D => {
            let p = unsafe { super::get_produced(i, usize::from((variable - 0x8A) / 2)) };
            let waiting = u32::from(unsafe { (*p).waiting });
            if has_bit(variable, 0) {
                gb(waiting, 8, 8)
            } else {
                waiting
            }
        }
        0x8E | 0x8F => {
            u32::from(unsafe { (*super::get_produced(i, usize::from(variable - 0x8E))).rate })
        }
        0x90..=0x92 => {
            u32::from(unsafe { (*super::get_accepted(i, usize::from(variable - 0x90))).cargo })
        }
        0x93 => u32::from(unsafe { (*i).fields.prod_level }),
        // 0x94 to 0x97: amount of cargo produced so far THIS month.
        // 0x98 to 0x9B: amount of cargo transported so far THIS month.
        // 0x9E to 0xA1: amount of cargo produced LAST month.
        // 0xA2 to 0xA5: amount of cargo transported last month.
        // Within each group of four, slots 0 and 1 alternate a value and its high byte.
        0x94..=0x9B | 0x9E..=0xA5 => {
            let (base, month) = if variable < 0x9C {
                (0x94, THIS_MONTH)
            } else {
                (0x9E, LAST_MONTH)
            };
            let offset = variable - base;
            let p = unsafe { super::get_produced(i, usize::from((offset % 4) / 2)) };
            let history = unsafe { (*p).history[usize::from(month)] };
            let value = u32::from(if offset < 4 {
                history.first
            } else {
                history.second
            });
            if has_bit(offset, 0) {
                gb(value, 8, 8)
            } else {
                value
            }
        }
        // fraction of cargo transported LAST month.
        0x9C | 0x9D => u32::from(unsafe {
            (*super::get_produced(i, usize::from(variable - 0x9C))).history[usize::from(LAST_MONTH)]
                .pct_transported()
        }),
        0xA6 => u32::from(unsafe { (*indspec).grf_prop.local_id }),
        0xA7 => u32::from(unsafe { (*i).metadata.founder }),
        0xA8 => u32::from(unsafe { (*i).metadata.random_colour }),
        0xA9 => unsafe { (*i).fields.last_prod_year }
            .wrapping_sub(ORIGINAL_BASE_YEAR)
            .clamp(0, i32::from(u8::MAX)) as u32,
        0xAA => u32::from(unsafe { (*i).fields.counter }),
        0xAB => gb(u32::from(unsafe { (*i).fields.counter }), 8, 8),
        0xAC => u32::from(unsafe { (*i).fields.was_cargo_delivered }),
        // Date when built since 1920 (in days)
        0xB0 => unsafe { (*i).metadata.construction_date }
            .wrapping_sub(DAYS_TILL_ORIGINAL_BASE_YEAR)
            .clamp(0, i32::from(u16::MAX)) as u32,
        // Construction type
        0xB3 => u32::from(unsafe { (*i).metadata.construction_type }),
        // Date last cargo accepted since 1920 (in days)
        0xB4 => {
            let accepted = unsafe { &(*i).accepted };
            accepted.iter().map(|a| a.last_accepted).max()
        }
        .map_or(0, |date| {
            date.wrapping_sub(DAYS_TILL_ORIGINAL_BASE_YEAR)
                .clamp(0, i32::from(u16::MAX)) as u32
        }),
        _ => unhandled_variable(variable, available, false, false, callback, s),
    }
}
/// `IndustryTileScopeResolver::GetVariable`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "IndustryTileScopeResolver::GetVariable keeps the original's implicit C++ integer conversions"
)]
fn industry_tile_get_variable(
    c: &ResolverContext,
    ro: *const c_void,
    variable: u8,
    parameter: u32,
    available: *mut bool,
    s: &ResolverServices,
) -> u32 {
    witness::hit(witness::INDUSTRY_RESOLVER_TILE_VARIABLE);
    match variable {
        // Construction state of the tile: a value between 0 and 3
        0x40 => {
            if s.is_industry_tile(c.tile()) {
                u32::from(unsafe { (s.tile().construction_stage)(c.tile().0) })
            } else {
                0
            }
        }
        // Terrain type
        0x41 => (s.shared().terrain_type)(c.tile().0),
        // Current town zone of the tile in the nearest town
        0x42 => {
            let town = crate::town::spatial::closest_town_from_tile(s.shared(), c.tile(), u32::MAX);
            u32::from(
                unsafe {
                    crate::town::get_town_radius_group(
                        (s.shared().town_owner)(town),
                        c.tile(),
                        s.shared().map(),
                    )
                }
                .0,
            )
        }
        // Relative position
        0x43 => get_relative_position(c.tile(), location_tile(c.owner), s.map()),
        // Animation frame. Like house variable 46 but can contain anything 0..FF.
        0x44 => {
            if s.is_industry_tile(c.tile()) {
                u32::from(s.get_animation_frame(c.tile()))
            } else {
                0
            }
        }
        // Land info of nearby tiles
        0x60 => get_nearby_industry_tile_information(
            parameter as u8,
            c.tile(),
            c.index(),
            true,
            (s.shared().resolver_grf_version)(ro) >= 8,
            s,
        ),
        // Animation stage of nearby tiles
        0x61 => {
            let tile = s.get_nearby_tile(parameter as u8, c.tile(), true);
            if s.is_industry_tile(tile)
                && unsafe { (s.tile().industry)(tile.0).shell.cast::<c_void>() } == c.industry
            {
                u32::from(s.get_animation_frame(tile))
            } else {
                u32::MAX
            }
        }
        // Get industry tile ID at offset
        0x62 => get_industry_id_at_offset(
            s.get_nearby_tile(parameter as u8, c.tile(), true),
            c.index(),
            (s.shared().resolver_grf_file_id)(ro),
            s,
        ),
        0x7A => {
            let gfx = IndustryGfx(unsafe { (s.tile().gfx)(c.tile().0) });
            let badges = unsafe { (*get_industry_tile_spec(gfx)).badges.badge_view() };
            badges.resolve(s.shared(), ro, parameter)
        }
        _ => unhandled_variable(
            variable,
            available,
            true,
            false,
            CallbackID::CBID_NO_CALLBACK,
            s,
        ),
    }
}

/// # Safety
/// Live serialized invocation, resolver, builder and immutable table. No borrow
/// of canonical owner/container state spans a native callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_variable(
    c: *const ResolverContext,
    ro: *const c_void,
    var: u8,
    parameter: u32,
    available: *mut bool,
    builder: *mut Builder,
    s: *const ResolverServices,
) -> u32 {
    // End the native context borrow before a synchronous service can reenter.
    let c = unsafe { *c };
    industries_get_variable(&c, ro, var, parameter, available, builder, unsafe { &*s })
}
/// # Safety
/// Live scope/resolver/table; original variable-specific map/industry preconditions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_variable(
    c: *const ResolverContext,
    ro: *const c_void,
    var: u8,
    parameter: u32,
    available: *mut bool,
    s: *const ResolverServices,
) -> u32 {
    let c = unsafe { *c };
    industry_tile_get_variable(&c, ro, var, parameter, available, unsafe { &*s })
}
impl ResolverContext {
    /// `IndustriesScopeResolver::GetRandomBits`.
    fn random_bits(&self) -> u32 {
        if self.owner.is_null() {
            0
        } else {
            // SAFETY: A non-null owner remains live through the immediate read.
            u32::from(unsafe { (*self.owner).metadata.random })
        }
    }
    /// `IndustryTileScopeResolver::GetRandomBits`.
    fn tile_random_bits(&self, s: &ResolverServices) -> u32 {
        if self.index() == IndustryID::INVALID {
            0
        } else {
            u32::from(s.get_industry_random_bits(self.tile()))
        }
    }
    /// `IndustryTileScopeResolver::GetRandomTriggers`.
    fn tile_random_triggers(&self, s: &ResolverServices) -> u32 {
        if self.index() == IndustryID::INVALID {
            0
        } else {
            u32::from(unsafe { ((*s.lifecycle).random_triggers)(self.tile().0) })
        }
    }
    /// The root sprite group of `IndustriesResolverObject`.
    fn root(&self) -> *const c_void {
        witness::hit(witness::INDUSTRY_RESOLVER_ROOT);
        // SAFETY: Native specs live for the resolver invocation.
        let props = unsafe { (*get_industry_spec(self.industry_type())).grf_prop };
        props.sprite_group(!self.owner.is_null() && self.index() != IndustryID::INVALID)
    }
    /// The root sprite group of `IndustryTileResolverObject`.
    fn tile_root(&self, gfx: IndustryGfx) -> *const c_void {
        witness::hit(witness::INDUSTRY_RESOLVER_TILE_ROOT);
        // SAFETY: Valid gfx precondition; stable native group.
        let props = unsafe { (*get_industry_tile_spec(gfx)).grf_prop };
        props.sprite_group(self.index() != IndustryID::INVALID)
    }
}
/// # Safety
/// Live context; a non-null owner remains live through the immediate read.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_resolver_random(c: *const ResolverContext) -> u32 {
    unsafe { (*c).random_bits() }
}
/// # Safety
/// Original non-null industry/valid tile assertions are checked by the facade.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_random(
    c: *const ResolverContext,
    s: *const ResolverServices,
) -> u32 {
    unsafe { (*c).tile_random_bits(&*s) }
}
/// # Safety
/// Original non-null industry/valid tile assertions are checked by the facade.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_triggers(
    c: *const ResolverContext,
    s: *const ResolverServices,
) -> u32 {
    unsafe { (*c).tile_random_triggers(&*s) }
}
/// # Safety
/// Live context and native specs; root group lives for the resolver invocation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_resolver_root(
    c: *const ResolverContext,
) -> *const c_void {
    unsafe { (*c).root() }
}
/// # Safety
/// Original non-null industry and valid gfx preconditions; stable native group.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_root(
    c: *const ResolverContext,
    gfx: u16,
) -> *const c_void {
    unsafe { (*c).tile_root(IndustryGfx(gfx)) }
}
/// Get or create the town scope object associated with the industry.
/// @return The associated town scope, if it exists.
fn get_town(c: &ResolverContext, ro: *mut c_void, s: &ResolverServices) -> *mut c_void {
    witness::hit(witness::INDUSTRY_RESOLVER_TOWN);
    let existing = unsafe { (s.existing_town)(ro) };
    if !existing.is_null() {
        return existing;
    }
    let (t, readonly) = if !c.owner.is_null() {
        (c.town, c.index() == IndustryID::INVALID)
    } else if c.tile() != TileIndex::INVALID {
        (
            crate::town::spatial::closest_town_from_tile(s.shared(), c.tile(), u32::MAX)
                .cast_mut()
                .cast(),
            true,
        )
    } else {
        (ptr::null_mut(), true)
    };
    if t.is_null() {
        ptr::null_mut()
    } else {
        unsafe { (s.make_town)(ro, t, readonly) }
    }
}
/// `IndustriesResolverObject::GetTown`.
///
/// # Safety
/// Live native industries resolver; optional town scope belongs to that stack frame.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_resolver_town(
    c: *const ResolverContext,
    ro: *mut c_void,
    s: *const ResolverServices,
) -> *mut c_void {
    let c = unsafe { *c };
    get_town(&c, ro, unsafe { &*s })
}
/// # Safety
/// All native scopes and resolver live for this synchronous resolution.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_resolver_scope(
    c: *const ResolverContext,
    ro: *mut c_void,
    own: *mut c_void,
    scope: u8,
    relative: u8,
    s: *const ResolverServices,
) -> *mut c_void {
    unsafe { resolver_scope(&*c, ro, own, scope, relative, &*s) }
}
/// `IndustriesResolverObject::GetScope`.
/// # Safety
/// All native scopes and resolver live for this synchronous resolution.
unsafe fn resolver_scope(
    c: &ResolverContext,
    ro: *mut c_void,
    own: *mut c_void,
    scope: u8,
    relative: u8,
    s: &ResolverServices,
) -> *mut c_void {
    match scope {
        VSG_SCOPE_SELF => own,
        VSG_SCOPE_PARENT => {
            let tsr = get_town(c, ro, s);
            if tsr.is_null() {
                unsafe { (s.base_scope)(ro, scope, relative) }
            } else {
                tsr
            }
        }
        _ => unsafe { (s.base_scope)(ro, scope, relative) },
    }
}
/// # Safety
/// Live tile resolver and its two native virtual facades, valid table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_scope(
    ro: *mut c_void,
    own: *mut c_void,
    parent: *mut c_void,
    scope: u8,
    relative: u8,
    s: *const ResolverServices,
) -> *mut c_void {
    tile_scope(ro, own, parent, scope, relative, unsafe { &*s })
}
/// `IndustryTileResolverObject::GetScope`.
fn tile_scope(
    ro: *mut c_void,
    own: *mut c_void,
    parent: *mut c_void,
    scope: u8,
    relative: u8,
    s: &ResolverServices,
) -> *mut c_void {
    match scope {
        VSG_SCOPE_SELF => own,
        VSG_SCOPE_PARENT => parent,
        _ => unsafe { (s.base_scope)(ro, scope, relative) },
    }
}
/// `GetIndustryProbabilityCallback`: check with callback `CBID_INDUSTRY_PROBABILITY` whether
/// the industry `kind` can be built, for the reason `creation_type`. Returns `default_prob`
/// unless the callback replaces or disallows it.
pub(crate) fn get_industry_probability_callback(
    kind: IndustryType,
    creation_type: IndustryAvailabilityCallType,
    mut default_prob: u32,
    s: &ResolverServices,
) -> u32 {
    witness::hit(witness::INDUSTRY_RESOLVER_PROBABILITY);
    let indspec = get_industry_spec(kind);
    if spec_callback_mask(indspec).test(IndustryCallbackMask::Probability) {
        let res = unsafe {
            (s.callback)(
                CallbackID::CBID_INDUSTRY_PROBABILITY.0,
                0,
                u32::from(creation_type.0),
                ptr::null_mut(),
                kind.0,
                TileIndex::INVALID.0,
            )
        };
        if res != CALLBACK_FAILED {
            if (s.shared().grf_file_version)(unsafe { (*indspec).grf_prop.grffile }) < 8 {
                // Disallow if result != 0
                if res != 0 {
                    default_prob = 0;
                }
            // Use returned probability. 0x100 to use default
            } else if res < 0x100 {
                default_prob = u32::from(res);
            } else if res > 0x100 {
                unsafe {
                    (s.callback_error)(indspec, CallbackID::CBID_INDUSTRY_PROBABILITY.0, res);
                }
            }
        }
    }
    default_prob
}
/// # Safety
/// Live specs/table, callback may synchronously reenter with its own stack context.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_probability(
    kind: u8,
    creation: u8,
    default: u32,
    s: *const ResolverServices,
) -> u32 {
    get_industry_probability_callback(
        IndustryType(kind),
        IndustryAvailabilityCallType(creation),
        default,
        unsafe { &*s },
    )
}
/// # Safety
/// Live owner and original accepted-cargo assertion, checked by the native facade.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_refuses_abi(
    industry: *mut c_void,
    owner: *mut Industry,
    cargo: u8,
    s: *const ResolverServices,
) -> u8 {
    u8::from(unsafe { industry_temporarily_refuses_cargo(industry, owner, CargoType(cargo), &*s) })
}
/// `IndustryTemporarilyRefusesCargo`: check whether an industry temporarily refuses to accept
/// a certain cargo. Precondition: `cargo_type` is in the industry's accepted cargo.
///
/// SAFETY: live owner/shell, caller has established an accepted cargo slot.
/// Action2 may reenter; owner observations end before every native call.
pub(crate) unsafe fn industry_temporarily_refuses_cargo(
    industry: *mut c_void,
    owner: *mut Industry,
    cargo_type: CargoType,
    s: &ResolverServices,
) -> bool {
    witness::hit(witness::INDUSTRY_RESOLVER_REFUSAL);
    let kind = IndustryType(unsafe { (*owner).metadata.kind });
    let indspec = get_industry_spec(kind);
    if spec_callback_mask(indspec).test(IndustryCallbackMask::RefuseCargo) {
        let file = unsafe { (*indspec).grf_prop.grffile };
        let parameter = (s.shared().grf_cargo_map)(file, cargo_type.0);
        let tile = location_tile(owner);
        let res = unsafe {
            (s.callback)(
                CallbackID::CBID_INDUSTRY_REFUSE_CARGO.0,
                0,
                u32::from(parameter),
                industry,
                kind.0,
                tile.0,
            )
        };
        if res != CALLBACK_FAILED {
            return !(s.shared().grf_boolean_callback)(
                file,
                CallbackID::CBID_INDUSTRY_REFUSE_CARGO.0,
                res,
            );
        }
    }
    false
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDIndustryFounderInfo" ["industry"] => FounderInfo { colours, is_ai, };
    "OpenTTDIndustryResolverContext" ["industry"] => ResolverContext { owner, industry, town, tile, random_bits, id, kind => type, };
    "OpenTTDIndustryResolverRef" ["industry"] => ResolverRef { owner, town, };
    "OpenTTDIndustryResolverServices" ["industry"] => ResolverServices {
        shared,
        lifecycle,
        tile,
        get,
        clean_gfx,
        nearby,
        water_distance,
        town_tile,
        town_id,
        founder_info,
        callback_id,
        storage_owner,
        unhandled,
        base_scope,
        existing_town,
        make_town,
        callback,
        callback_error,    };
}
