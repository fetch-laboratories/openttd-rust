/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared industry-map graphics identities.
#![allow(unsafe_code)]

/// Raw compatibility value for the water check marker.
pub const GFX_WATERTILE_SPECIALCHECK: u16 = IndustryGraphics::GFX_WATERTILE_SPECIALCHECK.0 as u16;

/// `GetIndustryIndex`, including its tile-type assertion policy.
pub(crate) fn get_industry_index(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
) -> Option<crate::industry_type::IndustryID> {
    m.require_type(tile, crate::tile_type::MP_INDUSTRY)?;
    Some(crate::industry_type::IndustryID(m.m2(tile)?))
}
/// `Industry::GetByTile` (`industry.h`).
pub(crate) fn get_industry_by_tile(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
    get: extern "C" fn(crate::industry_type::IndustryID) -> *mut crate::industry::tile::ShellHandle,
    check: extern "C" fn(bool),
) -> *mut crate::industry::tile::ShellHandle {
    check(m.read(tile) == crate::tile_type::MP_INDUSTRY);
    get(get_industry_index(m, tile).unwrap())
}
/// `GetIndustryType`, with the source assertion before and after its lookup.
pub(crate) fn get_industry_type(
    m: &crate::map_func::TileTypeAccess,
    tile: u32,
    get: extern "C" fn(crate::industry_type::IndustryID) -> *mut crate::industry::tile::ShellHandle,
    owner: extern "C" fn(*mut crate::industry::tile::ShellHandle) -> *mut crate::industry::Industry,
    check: extern "C" fn(bool),
) -> u8 {
    check(m.read(tile) == crate::tile_type::MP_INDUSTRY);
    let shell = get_industry_by_tile(m, tile, get, check);
    check(!shell.is_null());
    unsafe { (*crate::industry::openttd_rust_industry_metadata(owner(shell))).kind }
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

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_COAL_MINE_TOWER_NOT_ANIMATED: u16 =
    IndustryGraphics::GFX_COAL_MINE_TOWER_NOT_ANIMATED.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_COAL_MINE_TOWER_ANIMATED: u16 =
    IndustryGraphics::GFX_COAL_MINE_TOWER_ANIMATED.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_POWERPLANT_CHIMNEY: u16 = IndustryGraphics::GFX_POWERPLANT_CHIMNEY.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_POWERPLANT_SPARKS: u16 = IndustryGraphics::GFX_POWERPLANT_SPARKS.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_OILRIG_1: u16 = IndustryGraphics::GFX_OILRIG_1.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_OILWELL_NOT_ANIMATED: u16 = IndustryGraphics::GFX_OILWELL_NOT_ANIMATED.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_OILWELL_ANIMATED_1: u16 = IndustryGraphics::GFX_OILWELL_ANIMATED_1.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_OILWELL_ANIMATED_3: u16 = IndustryGraphics::GFX_OILWELL_ANIMATED_3.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_COPPER_MINE_TOWER_NOT_ANIMATED: u16 =
    IndustryGraphics::GFX_COPPER_MINE_TOWER_NOT_ANIMATED.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_COPPER_MINE_TOWER_ANIMATED: u16 =
    IndustryGraphics::GFX_COPPER_MINE_TOWER_ANIMATED.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_COPPER_MINE_CHIMNEY: u16 = IndustryGraphics::GFX_COPPER_MINE_CHIMNEY.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_GOLD_MINE_TOWER_NOT_ANIMATED: u16 =
    IndustryGraphics::GFX_GOLD_MINE_TOWER_NOT_ANIMATED.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_GOLD_MINE_TOWER_ANIMATED: u16 =
    IndustryGraphics::GFX_GOLD_MINE_TOWER_ANIMATED.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_TOY_FACTORY: u16 = IndustryGraphics::GFX_TOY_FACTORY.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_PLASTIC_FOUNTAIN_ANIMATED_1: u16 =
    IndustryGraphics::GFX_PLASTIC_FOUNTAIN_ANIMATED_1.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_PLASTIC_FOUNTAIN_ANIMATED_8: u16 =
    IndustryGraphics::GFX_PLASTIC_FOUNTAIN_ANIMATED_8.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_BUBBLE_GENERATOR: u16 = IndustryGraphics::GFX_BUBBLE_GENERATOR.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_BUBBLE_CATCHER: u16 = IndustryGraphics::GFX_BUBBLE_CATCHER.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_TOFFEE_QUARRY: u16 = IndustryGraphics::GFX_TOFFEE_QUARRY.0 as u16;

/// Native graphics value promoted to `IndustryGfx` for integer matching.
pub const GFX_SUGAR_MINE_SIEVE: u16 = IndustryGraphics::GFX_SUGAR_MINE_SIEVE.0 as u16;
