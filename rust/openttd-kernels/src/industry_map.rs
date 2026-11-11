/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Accessors for industry tiles and the industry graphics identities (`industry_map.h`).
#![allow(unsafe_code)]
use crate::industry::tile::ShellHandle;
use crate::industry_type::{IndustryGfx, IndustryID, IndustryType};
use crate::tile_type::{TileIndex, TileType};

/// Raw compatibility value for the water check marker.
pub const GFX_WATERTILE_SPECIALCHECK: u16 = IndustryGraphics::GFX_WATERTILE_SPECIALCHECK.0 as u16;

/// `GetIndustryIndex`: get the industry ID of the given tile, including its tile-type
/// assertion policy.
pub(crate) fn get_industry_index(
    m: &crate::map_func::TileTypeAccess,
    t: TileIndex,
) -> Option<IndustryID> {
    m.require_type(t, TileType::MP_INDUSTRY)?;
    Some(IndustryID(m.m2(t)?))
}
/// `Industry::GetByTile` (`industry.h`): get the industry of the given tile.
pub(crate) fn get_industry_by_tile(
    m: &crate::map_func::TileTypeAccess,
    tile: TileIndex,
    get: extern "C" fn(IndustryID) -> *mut ShellHandle,
    check: extern "C" fn(bool),
) -> *mut ShellHandle {
    check(m.read(tile) == TileType::MP_INDUSTRY);
    get(get_industry_index(m, tile).unwrap())
}
/// `GetIndustryType`: retrieve the type for this industry. Although it is called
/// `IndustryType`, it is in fact an index in the `_industry_specs` array. The source asserts
/// the tile type before and after its lookup.
pub(crate) fn get_industry_type(
    m: &crate::map_func::TileTypeAccess,
    tile: TileIndex,
    get: extern "C" fn(IndustryID) -> *mut ShellHandle,
    owner: extern "C" fn(*mut ShellHandle) -> *mut crate::industry::Industry,
    check: extern "C" fn(bool),
) -> IndustryType {
    check(m.read(tile) == TileType::MP_INDUSTRY);
    let ind = get_industry_by_tile(m, tile, get, check);
    check(!ind.is_null());
    IndustryType(unsafe { (*crate::industry::openttd_rust_industry_metadata(owner(ind))).kind })
}

/// Indices into `table/industry_land.h::_industry_draw_tile_data`: gfx << 2 | stage (0 to 3).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryGraphics(pub u8);
impl IndustryGraphics {
    /// Native `GFX_COAL_MINE_TOWER_NOT_ANIMATED`.
    pub const GFX_COAL_MINE_TOWER_NOT_ANIMATED: Self = Self(0);
    /// Native `GFX_COAL_MINE_TOWER_ANIMATED`.
    pub const GFX_COAL_MINE_TOWER_ANIMATED: Self = Self(1);
    /// Native `GFX_POWERPLANT_CHIMNEY`.
    pub const GFX_POWERPLANT_CHIMNEY: Self = Self(8);
    /// Native `GFX_POWERPLANT_SPARKS`.
    pub const GFX_POWERPLANT_SPARKS: Self = Self(10);
    /// Native `GFX_OILRIG_1`.
    pub const GFX_OILRIG_1: Self = Self(24);
    /// Native `GFX_OILRIG_2`.
    pub const GFX_OILRIG_2: Self = Self(25);
    /// Native `GFX_OILRIG_3`.
    pub const GFX_OILRIG_3: Self = Self(26);
    /// Native `GFX_OILRIG_4`.
    pub const GFX_OILRIG_4: Self = Self(27);
    /// Native `GFX_OILRIG_5`.
    pub const GFX_OILRIG_5: Self = Self(28);
    /// Native `GFX_OILWELL_NOT_ANIMATED`.
    pub const GFX_OILWELL_NOT_ANIMATED: Self = Self(29);
    /// Native `GFX_OILWELL_ANIMATED_1`.
    pub const GFX_OILWELL_ANIMATED_1: Self = Self(30);
    /// Native `GFX_OILWELL_ANIMATED_2`.
    pub const GFX_OILWELL_ANIMATED_2: Self = Self(31);
    /// Native `GFX_OILWELL_ANIMATED_3`.
    pub const GFX_OILWELL_ANIMATED_3: Self = Self(32);
    /// Native `GFX_COPPER_MINE_TOWER_NOT_ANIMATED`.
    pub const GFX_COPPER_MINE_TOWER_NOT_ANIMATED: Self = Self(47);
    /// Native `GFX_COPPER_MINE_TOWER_ANIMATED`.
    pub const GFX_COPPER_MINE_TOWER_ANIMATED: Self = Self(48);
    /// Native `GFX_COPPER_MINE_CHIMNEY`.
    pub const GFX_COPPER_MINE_CHIMNEY: Self = Self(49);
    /// Native `GFX_GOLD_MINE_TOWER_NOT_ANIMATED`.
    pub const GFX_GOLD_MINE_TOWER_NOT_ANIMATED: Self = Self(79);
    /// Native `GFX_GOLD_MINE_TOWER_ANIMATED`.
    pub const GFX_GOLD_MINE_TOWER_ANIMATED: Self = Self(88);
    /// Native `GFX_TOY_FACTORY`.
    pub const GFX_TOY_FACTORY: Self = Self(143);
    /// Native `GFX_PLASTIC_FOUNTAIN_ANIMATED_1`.
    pub const GFX_PLASTIC_FOUNTAIN_ANIMATED_1: Self = Self(148);
    /// Native `GFX_PLASTIC_FOUNTAIN_ANIMATED_2`.
    pub const GFX_PLASTIC_FOUNTAIN_ANIMATED_2: Self = Self(149);
    /// Native `GFX_PLASTIC_FOUNTAIN_ANIMATED_3`.
    pub const GFX_PLASTIC_FOUNTAIN_ANIMATED_3: Self = Self(150);
    /// Native `GFX_PLASTIC_FOUNTAIN_ANIMATED_4`.
    pub const GFX_PLASTIC_FOUNTAIN_ANIMATED_4: Self = Self(151);
    /// Native `GFX_PLASTIC_FOUNTAIN_ANIMATED_5`.
    pub const GFX_PLASTIC_FOUNTAIN_ANIMATED_5: Self = Self(152);
    /// Native `GFX_PLASTIC_FOUNTAIN_ANIMATED_6`.
    pub const GFX_PLASTIC_FOUNTAIN_ANIMATED_6: Self = Self(153);
    /// Native `GFX_PLASTIC_FOUNTAIN_ANIMATED_7`.
    pub const GFX_PLASTIC_FOUNTAIN_ANIMATED_7: Self = Self(154);
    /// Native `GFX_PLASTIC_FOUNTAIN_ANIMATED_8`.
    pub const GFX_PLASTIC_FOUNTAIN_ANIMATED_8: Self = Self(155);
    /// Native `GFX_BUBBLE_GENERATOR`.
    pub const GFX_BUBBLE_GENERATOR: Self = Self(161);
    /// Native `GFX_BUBBLE_CATCHER`.
    pub const GFX_BUBBLE_CATCHER: Self = Self(162);
    /// Native `GFX_TOFFEE_QUARRY`.
    pub const GFX_TOFFEE_QUARRY: Self = Self(165);
    /// Native `GFX_SUGAR_MINE_SIEVE`.
    pub const GFX_SUGAR_MINE_SIEVE: Self = Self(174);
    /// not really a tile, but rather a very special check
    pub const GFX_WATERTILE_SPECIALCHECK: Self = Self(255);
}

/// `IndustryGraphics` values promoted to `IndustryGfx`, as the original compares and
/// switches on them; the promotion widens every value unchanged.
impl IndustryGfx {
    /// `GFX_COAL_MINE_TOWER_NOT_ANIMATED` promoted to `IndustryGfx`.
    pub const GFX_COAL_MINE_TOWER_NOT_ANIMATED: Self =
        Self(IndustryGraphics::GFX_COAL_MINE_TOWER_NOT_ANIMATED.0 as u16);
    /// `GFX_COAL_MINE_TOWER_ANIMATED` promoted to `IndustryGfx`.
    pub const GFX_COAL_MINE_TOWER_ANIMATED: Self =
        Self(IndustryGraphics::GFX_COAL_MINE_TOWER_ANIMATED.0 as u16);
    /// `GFX_POWERPLANT_CHIMNEY` promoted to `IndustryGfx`.
    pub const GFX_POWERPLANT_CHIMNEY: Self =
        Self(IndustryGraphics::GFX_POWERPLANT_CHIMNEY.0 as u16);
    /// `GFX_POWERPLANT_SPARKS` promoted to `IndustryGfx`.
    pub const GFX_POWERPLANT_SPARKS: Self = Self(IndustryGraphics::GFX_POWERPLANT_SPARKS.0 as u16);
    /// `GFX_OILRIG_1` promoted to `IndustryGfx`.
    pub const GFX_OILRIG_1: Self = Self(IndustryGraphics::GFX_OILRIG_1.0 as u16);
    /// `GFX_OILRIG_2` promoted to `IndustryGfx`.
    pub const GFX_OILRIG_2: Self = Self(IndustryGraphics::GFX_OILRIG_2.0 as u16);
    /// `GFX_OILRIG_3` promoted to `IndustryGfx`.
    pub const GFX_OILRIG_3: Self = Self(IndustryGraphics::GFX_OILRIG_3.0 as u16);
    /// `GFX_OILRIG_4` promoted to `IndustryGfx`.
    pub const GFX_OILRIG_4: Self = Self(IndustryGraphics::GFX_OILRIG_4.0 as u16);
    /// `GFX_OILRIG_5` promoted to `IndustryGfx`.
    pub const GFX_OILRIG_5: Self = Self(IndustryGraphics::GFX_OILRIG_5.0 as u16);
    /// `GFX_OILWELL_NOT_ANIMATED` promoted to `IndustryGfx`.
    pub const GFX_OILWELL_NOT_ANIMATED: Self =
        Self(IndustryGraphics::GFX_OILWELL_NOT_ANIMATED.0 as u16);
    /// `GFX_OILWELL_ANIMATED_1` promoted to `IndustryGfx`.
    pub const GFX_OILWELL_ANIMATED_1: Self =
        Self(IndustryGraphics::GFX_OILWELL_ANIMATED_1.0 as u16);
    /// `GFX_OILWELL_ANIMATED_2` promoted to `IndustryGfx`.
    pub const GFX_OILWELL_ANIMATED_2: Self =
        Self(IndustryGraphics::GFX_OILWELL_ANIMATED_2.0 as u16);
    /// `GFX_OILWELL_ANIMATED_3` promoted to `IndustryGfx`.
    pub const GFX_OILWELL_ANIMATED_3: Self =
        Self(IndustryGraphics::GFX_OILWELL_ANIMATED_3.0 as u16);
    /// `GFX_COPPER_MINE_TOWER_NOT_ANIMATED` promoted to `IndustryGfx`.
    pub const GFX_COPPER_MINE_TOWER_NOT_ANIMATED: Self =
        Self(IndustryGraphics::GFX_COPPER_MINE_TOWER_NOT_ANIMATED.0 as u16);
    /// `GFX_COPPER_MINE_TOWER_ANIMATED` promoted to `IndustryGfx`.
    pub const GFX_COPPER_MINE_TOWER_ANIMATED: Self =
        Self(IndustryGraphics::GFX_COPPER_MINE_TOWER_ANIMATED.0 as u16);
    /// `GFX_COPPER_MINE_CHIMNEY` promoted to `IndustryGfx`.
    pub const GFX_COPPER_MINE_CHIMNEY: Self =
        Self(IndustryGraphics::GFX_COPPER_MINE_CHIMNEY.0 as u16);
    /// `GFX_GOLD_MINE_TOWER_NOT_ANIMATED` promoted to `IndustryGfx`.
    pub const GFX_GOLD_MINE_TOWER_NOT_ANIMATED: Self =
        Self(IndustryGraphics::GFX_GOLD_MINE_TOWER_NOT_ANIMATED.0 as u16);
    /// `GFX_GOLD_MINE_TOWER_ANIMATED` promoted to `IndustryGfx`.
    pub const GFX_GOLD_MINE_TOWER_ANIMATED: Self =
        Self(IndustryGraphics::GFX_GOLD_MINE_TOWER_ANIMATED.0 as u16);
    /// `GFX_TOY_FACTORY` promoted to `IndustryGfx`.
    pub const GFX_TOY_FACTORY: Self = Self(IndustryGraphics::GFX_TOY_FACTORY.0 as u16);
    /// `GFX_PLASTIC_FOUNTAIN_ANIMATED_1` promoted to `IndustryGfx`.
    pub const GFX_PLASTIC_FOUNTAIN_ANIMATED_1: Self =
        Self(IndustryGraphics::GFX_PLASTIC_FOUNTAIN_ANIMATED_1.0 as u16);
    /// `GFX_PLASTIC_FOUNTAIN_ANIMATED_2` promoted to `IndustryGfx`.
    pub const GFX_PLASTIC_FOUNTAIN_ANIMATED_2: Self =
        Self(IndustryGraphics::GFX_PLASTIC_FOUNTAIN_ANIMATED_2.0 as u16);
    /// `GFX_PLASTIC_FOUNTAIN_ANIMATED_3` promoted to `IndustryGfx`.
    pub const GFX_PLASTIC_FOUNTAIN_ANIMATED_3: Self =
        Self(IndustryGraphics::GFX_PLASTIC_FOUNTAIN_ANIMATED_3.0 as u16);
    /// `GFX_PLASTIC_FOUNTAIN_ANIMATED_4` promoted to `IndustryGfx`.
    pub const GFX_PLASTIC_FOUNTAIN_ANIMATED_4: Self =
        Self(IndustryGraphics::GFX_PLASTIC_FOUNTAIN_ANIMATED_4.0 as u16);
    /// `GFX_PLASTIC_FOUNTAIN_ANIMATED_5` promoted to `IndustryGfx`.
    pub const GFX_PLASTIC_FOUNTAIN_ANIMATED_5: Self =
        Self(IndustryGraphics::GFX_PLASTIC_FOUNTAIN_ANIMATED_5.0 as u16);
    /// `GFX_PLASTIC_FOUNTAIN_ANIMATED_6` promoted to `IndustryGfx`.
    pub const GFX_PLASTIC_FOUNTAIN_ANIMATED_6: Self =
        Self(IndustryGraphics::GFX_PLASTIC_FOUNTAIN_ANIMATED_6.0 as u16);
    /// `GFX_PLASTIC_FOUNTAIN_ANIMATED_7` promoted to `IndustryGfx`.
    pub const GFX_PLASTIC_FOUNTAIN_ANIMATED_7: Self =
        Self(IndustryGraphics::GFX_PLASTIC_FOUNTAIN_ANIMATED_7.0 as u16);
    /// `GFX_PLASTIC_FOUNTAIN_ANIMATED_8` promoted to `IndustryGfx`.
    pub const GFX_PLASTIC_FOUNTAIN_ANIMATED_8: Self =
        Self(IndustryGraphics::GFX_PLASTIC_FOUNTAIN_ANIMATED_8.0 as u16);
    /// `GFX_BUBBLE_GENERATOR` promoted to `IndustryGfx`.
    pub const GFX_BUBBLE_GENERATOR: Self = Self(IndustryGraphics::GFX_BUBBLE_GENERATOR.0 as u16);
    /// `GFX_BUBBLE_CATCHER` promoted to `IndustryGfx`.
    pub const GFX_BUBBLE_CATCHER: Self = Self(IndustryGraphics::GFX_BUBBLE_CATCHER.0 as u16);
    /// `GFX_TOFFEE_QUARRY` promoted to `IndustryGfx`.
    pub const GFX_TOFFEE_QUARRY: Self = Self(IndustryGraphics::GFX_TOFFEE_QUARRY.0 as u16);
    /// `GFX_SUGAR_MINE_SIEVE` promoted to `IndustryGfx`.
    pub const GFX_SUGAR_MINE_SIEVE: Self = Self(IndustryGraphics::GFX_SUGAR_MINE_SIEVE.0 as u16);
    /// `GFX_WATERTILE_SPECIALCHECK` promoted to `IndustryGfx`.
    pub const GFX_WATERTILE_SPECIALCHECK: Self =
        Self(IndustryGraphics::GFX_WATERTILE_SPECIALCHECK.0 as u16);
}
