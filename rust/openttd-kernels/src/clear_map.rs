/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Read helpers of `clear_map.h` for the audited map reader.
//! `None` routes to the original typed native accessor (including failed assertions).
use crate::direction_type::DiagDirection;
use crate::industry_type::IndustryID;
use crate::map_func::{TileFields, TileTypeAccess, World};
use crate::tile_type::TileIndex;
use crate::tile_type::TileType;
/// `ClearGround::CLEAR_GRASS`.
pub(crate) const CLEAR_GRASS: u8 = ClearGround::CLEAR_GRASS.0;
/// `ClearGround::CLEAR_ROUGH`.
pub(crate) const CLEAR_ROUGH: u8 = ClearGround::CLEAR_ROUGH.0;
/// `ClearGround::CLEAR_ROCKS`.
pub(crate) const CLEAR_ROCKS: u8 = ClearGround::CLEAR_ROCKS.0;
/// `ClearGround::CLEAR_DESERT`.
pub(crate) const CLEAR_DESERT: u8 = ClearGround::CLEAR_DESERT.0;
/// `ClearGround::CLEAR_FIELDS`.
pub(crate) const CLEAR_FIELDS: u8 = ClearGround::CLEAR_FIELDS.0;
/// `IsSnowTile`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_snow_tile(m: &TileTypeAccess, t: TileIndex) -> Option<bool> {
    Some(is_snow_tile_from_fields(
        &m.require_type(t, TileType::MP_CLEAR)?,
    ))
}
/// `IsSnowTile` over fields already checked as `MP_CLEAR`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_snow_tile_from_fields(fields: &TileFields<'_>) -> bool {
    fields.m3() & 16 != 0
}
/// `GetClearGround`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_clear_ground(m: &TileTypeAccess, t: TileIndex) -> Option<ClearGround> {
    Some(get_clear_ground_from_fields(
        &m.require_type(t, TileType::MP_CLEAR)?,
    ))
}
/// `IsClearGround`: whether the clear tile has the given ground type.
#[inline]
pub(crate) fn is_clear_ground(m: &TileTypeAccess, t: TileIndex, ct: ClearGround) -> Option<bool> {
    Some(get_clear_ground(m, t)? == ct)
}
/// `GetClearGround` over fields already checked as `MP_CLEAR`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_clear_ground_from_fields(fields: &TileFields<'_>) -> ClearGround {
    ClearGround((fields.m5() >> 2) & 7)
}
/// `GetClearDensity`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_clear_density(m: &TileTypeAccess, t: TileIndex) -> Option<u8> {
    Some(get_clear_density_from_fields(
        &m.require_type(t, TileType::MP_CLEAR)?,
    ))
}
/// `GetClearDensity` over fields already checked as `MP_CLEAR`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_clear_density_from_fields(fields: &TileFields<'_>) -> u8 {
    fields.m5() & 3
}
/// `GetClearCounter`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_clear_counter(m: &TileTypeAccess, t: TileIndex) -> Option<u8> {
    Some(get_clear_counter_from_fields(
        &m.require_type(t, TileType::MP_CLEAR)?,
    ))
}
/// `GetClearCounter` over fields already checked as `MP_CLEAR`.
#[inline]
pub(crate) fn get_clear_counter_from_fields(fields: &TileFields<'_>) -> u8 {
    fields.m5() >> 5
}
/// `AddClearDensity`: increment the density of a non-field clear tile.
pub(crate) fn add_clear_density(m: &TileTypeAccess, t: TileIndex, d: i32) -> Option<()> {
    let fields = m.require_type(t, TileType::MP_CLEAR)?; // XXX incomplete
    add_clear_density_from_fields(m, t, &fields, d)
}
/// `AddClearDensity`'s store over fields already checked as `MP_CLEAR`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "`t.m5() += d` stores the int sum back into the original uint8_t m5"
)]
#[inline]
pub(crate) fn add_clear_density_from_fields(
    m: &TileTypeAccess,
    t: TileIndex,
    fields: &TileFields<'_>,
    d: i32,
) -> Option<()> {
    m.set_m5(t, i32::from(fields.m5()).wrapping_add(d) as u8)
}
/// `AddClearCounter`: increments the counter used to advance to the next clear density/field
/// type.
pub(crate) fn add_clear_counter(m: &TileTypeAccess, t: TileIndex, c: i32) -> Option<()> {
    let fields = m.require_type(t, TileType::MP_CLEAR)?; // XXX incomplete
    add_clear_counter_from_fields(m, t, &fields, c)
}
/// `AddClearCounter`'s store over fields already checked as `MP_CLEAR`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "`t.m5() += c << 5` stores the int sum back into the original uint8_t m5"
)]
#[inline]
pub(crate) fn add_clear_counter_from_fields(
    m: &TileTypeAccess,
    t: TileIndex,
    fields: &TileFields<'_>,
    c: i32,
) -> Option<()> {
    m.set_m5(t, i32::from(fields.m5()).wrapping_add(c << 5) as u8)
}
/// `SetClearGroundDensity`: sets ground type and density in one go, also sets the counter to 0.
pub(crate) fn set_clear_ground_density(
    m: &TileTypeAccess,
    t: TileIndex,
    r#type: ClearGround,
    density: u32,
) -> Option<()> {
    let fields = m.require_type(t, TileType::MP_CLEAR)?; // XXX incomplete
    set_clear_ground_density_from_fields(m, t, &fields, r#type, density)
}
/// `SetClearGroundDensity`'s store over fields already checked as `MP_CLEAR`; the fields are
/// the proof and are not read.
#[allow(
    clippy::cast_possible_truncation,
    reason = "`t.m5() = 0 << 5 | type << 2 | density` stores into the original uint8_t m5"
)]
#[inline]
pub(crate) fn set_clear_ground_density_from_fields(
    m: &TileTypeAccess,
    t: TileIndex,
    _fields: &TileFields<'_>,
    r#type: ClearGround,
    density: u32,
) -> Option<()> {
    m.set_m5(t, (u32::from(r#type.0) << 2 | density) as u8)
}
/// The fields of a tile, checked as `IsClearGround(t, CLEAR_FIELDS)` when asserting.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn require_field(m: &TileTypeAccess, t: TileIndex) -> Option<TileFields<'_>> {
    let fields = if m.assertions() {
        let fields = m.require_type(t, TileType::MP_CLEAR)?;
        m.check(get_clear_ground_from_fields(&fields) == ClearGround::CLEAR_FIELDS)?;
        fields
    } else {
        m.fields(t)?
    };
    Some(fields)
}
/// `GetFieldType`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_field_type(fields: &TileFields<'_>) -> u32 {
    u32::from(fields.m3() & 15)
}
/// `GetIndustryIndexOfField`.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_industry_index_of_field(fields: &TileFields<'_>) -> IndustryID {
    IndustryID(fields.m2())
}
/// `GetFence`; the caller supplies its non-returning failure operation.
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_fence(
    fields: &TileFields<'_>,
    side: DiagDirection,
    not_reached: impl FnOnce(),
) -> u32 {
    u32::from(match side {
        DiagDirection::DIAGDIR_SE => (fields.m4() >> 2) & 7,
        DiagDirection::DIAGDIR_SW => fields.m4() >> 5,
        DiagDirection::DIAGDIR_NE => fields.m3() >> 5,
        DiagDirection::DIAGDIR_NW => (fields.m6() >> 2) & 7,
        _ => {
            not_reached();
            std::process::abort()
        }
    })
}

/// One clear tile's reads and writes in `TileLoop_Clear` (`clear_cmd.cpp`), with its
/// `assert(IsTileType(t, MP_CLEAR))` checked once.
///
/// The proof is `require_type(tile, MP_CLEAR)` on the world's own reader, which must carry no
/// native handler: the probe then fires no assertion and loads only the type byte, so taking it
/// early is unobservable. `Some` means the build does not assert, or the tile is in range and
/// is `MP_CLEAR`; either way none of the accessors' type assertions could fire, and each method
/// is one field-sized load or store at its observation point. `None` proves nothing: each
/// method then calls the asserting accessor, so the native handler sees every assertion the
/// original makes, in its order.
///
/// Nothing in `TileLoop_Clear` changes the tile's type, so the proof holds for the whole body:
/// `TileLoopClearDesert`, `TileLoopClearAlps`, `UpdateFences`, `MakeSnow`, `ClearSnow`,
/// `AddClearDensity`, `AddClearCounter`, `SetClearCounter`, `SetClearGroundDensity`,
/// `SetFence`, `SetFieldType`, `Random`, `MarkTileDirtyByTile` and the neighbour reads write
/// no type byte, and `MakeClear` stores `MP_CLEAR` again with no read of the tile after it.
#[derive(Clone, Copy)]
pub(crate) struct ClearTile<'w> {
    world: &'w World<'w>,
    tile: TileIndex,
    fields: Option<TileFields<'w>>,
}
impl<'w> ClearTile<'w> {
    /// Probe `tile` once. `world.types` carries no native handler.
    #[inline]
    pub(crate) fn new(world: &'w World<'w>, tile: TileIndex) -> Self {
        Self {
            world,
            tile,
            fields: world.types.require_type(tile, TileType::MP_CLEAR),
        }
    }
    /// The tile.
    #[inline]
    pub(crate) fn tile(self) -> TileIndex {
        self.tile
    }
    /// `IsSnowTile`.
    #[inline]
    pub(crate) fn is_snow_tile(self) -> bool {
        let Self {
            world,
            tile,
            fields,
        } = self;
        match fields {
            Some(fields) => is_snow_tile_from_fields(&fields),
            None => asserted(world, move |m| is_snow_tile(m, tile)),
        }
    }
    /// `GetClearGround`.
    #[inline]
    pub(crate) fn get_clear_ground(self) -> ClearGround {
        let Self {
            world,
            tile,
            fields,
        } = self;
        match fields {
            Some(fields) => get_clear_ground_from_fields(&fields),
            None => asserted(world, move |m| get_clear_ground(m, tile)),
        }
    }
    /// `IsClearGround`.
    #[inline]
    pub(crate) fn is_clear_ground(self, ct: ClearGround) -> bool {
        self.get_clear_ground() == ct
    }
    /// `GetClearDensity`.
    #[inline]
    pub(crate) fn get_clear_density(self) -> u8 {
        let Self {
            world,
            tile,
            fields,
        } = self;
        match fields {
            Some(fields) => get_clear_density_from_fields(&fields),
            None => asserted(world, move |m| get_clear_density(m, tile)),
        }
    }
    /// `GetClearCounter`.
    #[inline]
    pub(crate) fn get_clear_counter(self) -> u8 {
        let Self {
            world,
            tile,
            fields,
        } = self;
        match fields {
            Some(fields) => get_clear_counter_from_fields(&fields),
            None => asserted(world, move |m| get_clear_counter(m, tile)),
        }
    }
    /// `AddClearDensity`.
    #[inline]
    pub(crate) fn add_clear_density(self, d: i32) {
        let Self {
            world,
            tile,
            fields,
        } = self;
        match fields {
            Some(fields) => add_clear_density_from_fields(&world.types, tile, &fields, d)
                .expect("live map scalar"),
            None => asserted(world, move |m| add_clear_density(m, tile, d)),
        }
    }
    /// `AddClearCounter`.
    #[inline]
    pub(crate) fn add_clear_counter(self, c: i32) {
        let Self {
            world,
            tile,
            fields,
        } = self;
        match fields {
            Some(fields) => add_clear_counter_from_fields(&world.types, tile, &fields, c)
                .expect("live map scalar"),
            None => asserted(world, move |m| add_clear_counter(m, tile, c)),
        }
    }
    /// `SetClearGroundDensity`.
    #[inline]
    pub(crate) fn set_clear_ground_density(self, r#type: ClearGround, density: u32) {
        let Self {
            world,
            tile,
            fields,
        } = self;
        match fields {
            Some(fields) => {
                set_clear_ground_density_from_fields(&world.types, tile, &fields, r#type, density)
                    .expect("live map scalar");
            }
            None => asserted(world, move |m| {
                set_clear_ground_density(m, tile, r#type, density)
            }),
        }
    }
    /// The field proof, taken where `UpdateFences` asserts
    /// `IsTileType(tile, MP_CLEAR) && IsClearGround(tile, CLEAR_FIELDS)` and again after each
    /// write to `m5`: present when the clear proof holds and either the build does not assert
    /// or the ground read from it is `CLEAR_FIELDS`, so none of the field accessors'
    /// `GetClearGround(t) == CLEAR_FIELDS` assertions could fire.
    #[inline]
    pub(crate) fn field(self) -> FieldTile<'w> {
        let Self {
            world,
            tile,
            fields,
        } = self;
        FieldTile {
            world,
            tile,
            fields: fields.filter(|fields| {
                !world.types.assertions()
                    || get_clear_ground_from_fields(fields) == ClearGround::CLEAR_FIELDS
            }),
        }
    }
}

/// A field tile's reads under [`ClearTile::field`]'s proof. The proof ends at a write to the
/// tile's type or `m5`; `SetFence` stores only `m3`, `m4` or `m6` fence bits.
#[derive(Clone, Copy)]
pub(crate) struct FieldTile<'w> {
    world: &'w World<'w>,
    tile: TileIndex,
    fields: Option<TileFields<'w>>,
}
impl FieldTile<'_> {
    /// The tile.
    #[inline]
    pub(crate) fn tile(self) -> TileIndex {
        self.tile
    }
    /// Whether the proof is present.
    #[inline]
    pub(crate) fn is_proven(self) -> bool {
        self.fields.is_some()
    }
    /// `GetFence`.
    #[inline]
    pub(crate) fn get_fence(self, side: DiagDirection) -> u32 {
        let Self {
            world,
            tile,
            fields,
        } = self;
        let not_reached = || crate::services_assert::not_reached();
        match fields {
            Some(fields) => get_fence(&fields, side, not_reached),
            None => asserted(world, move |m| {
                Some(get_fence(&require_field(m, tile)?, side, not_reached))
            }),
        }
    }
    /// `GetFieldType`.
    #[inline]
    pub(crate) fn get_field_type(self) -> u32 {
        let Self {
            world,
            tile,
            fields,
        } = self;
        match fields {
            Some(fields) => get_field_type(&fields),
            None => asserted(world, move |m| {
                Some(get_field_type(&require_field(m, tile)?))
            }),
        }
    }
    /// `GetIndustryIndexOfField`.
    #[inline]
    pub(crate) fn get_industry_index_of_field(self) -> IndustryID {
        let Self {
            world,
            tile,
            fields,
        } = self;
        match fields {
            Some(fields) => get_industry_index_of_field(&fields),
            None => asserted(world, move |m| {
                Some(get_industry_index_of_field(&require_field(m, tile)?))
            }),
        }
    }
}

/// `World::asserted` for a failed proof, kept out of line so each proof's methods stay a
/// two-arm match.
#[cold]
#[inline(never)]
fn asserted<T>(world: &World<'_>, read: impl Fn(&TileTypeAccess) -> Option<T>) -> T {
    world.asserted(read)
}

/// Raw adapter: `IsSnowTile` over a base tile, for files no pass has finished yet.
#[inline]
pub(crate) fn is_snow_tile_raw(m: &TileTypeAccess, t: u32) -> Option<bool> {
    is_snow_tile(m, TileIndex(t))
}
/// Raw adapter: `GetClearGround` over base values.
#[inline]
pub(crate) fn get_clear_ground_raw(m: &TileTypeAccess, t: u32) -> Option<u8> {
    get_clear_ground(m, TileIndex(t)).map(|g| g.0)
}

/// Native `ClearGround` representation; every byte pattern is retained.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClearGround(pub u8);

impl ClearGround {
    /// 0-3
    pub const CLEAR_GRASS: Self = Self(0);
    /// 3
    pub const CLEAR_ROUGH: Self = Self(1);
    /// 3
    pub const CLEAR_ROCKS: Self = Self(2);
    /// 3
    pub const CLEAR_FIELDS: Self = Self(3);
    /// 0-3 (Not stored in map.)
    pub const CLEAR_SNOW: Self = Self(4);
    /// 1,3
    pub const CLEAR_DESERT: Self = Self(5);
}
