/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Read helpers of `water_map.h` for the audited map reader.
//! `None` routes to the original typed native accessor (including failed assertions).
use crate::map_func::{TileFields, TileTypeAccess};

use crate::tile_type::{MP_INDUSTRY, MP_OBJECT, MP_STATION, MP_TREES, MP_WATER};
/// `WaterTileType::Coast`.
const COAST: u8 = 1;
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_water_tile_type(m: &TileTypeAccess, t: u32) -> Option<u8> {
    let fields = m.require_type(t, MP_WATER)?;
    Some(fields.m5() >> 4)
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn has_tile_water_class(m: &TileTypeAccess, t: u32) -> Option<bool> {
    let probe = m.without_native_assert();
    if let Some(fields) = probe.require_range(t) {
        return Some(has_tile_water_class_from_fields(&fields));
    }
    has_tile_water_class_slow(m, t)
}
// Each IsTileType in the original || expression has its own range assertion.
// A returning handler can change the type before the following observation.
#[cold]
#[inline(never)]
fn has_tile_water_class_slow(m: &TileTypeAccess, t: u32) -> Option<bool> {
    Some(
        crate::tile_map::get_tile_type(m, t)? == MP_WATER
            || crate::tile_map::get_tile_type(m, t)? == MP_STATION
            || crate::tile_map::get_tile_type(m, t)? == MP_INDUSTRY
            || crate::tile_map::get_tile_type(m, t)? == MP_OBJECT
            || crate::tile_map::get_tile_type(m, t)? == MP_TREES,
    )
}
#[allow(clippy::inline_always)]
#[inline(always)]
fn has_tile_water_class_from_fields(fields: &TileFields<'_>) -> bool {
    matches!(
        crate::tile_map::get_tile_type_from_fields(fields),
        MP_WATER | MP_STATION | MP_INDUSTRY | MP_OBJECT | MP_TREES
    )
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn get_water_class(m: &TileTypeAccess, t: u32) -> Option<u8> {
    if m.assertions() {
        let probe = m.without_native_assert();
        if let Some(fields) = probe.require_range(t) {
            m.check(has_tile_water_class_from_fields(&fields))?;
            return Some((fields.m1() >> 5) & 3);
        }
        m.check(has_tile_water_class_slow(m, t)?)?;
    }
    Some((m.fields(t)?.m1() >> 5) & 3)
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_coast(m: &TileTypeAccess, t: u32) -> Option<bool> {
    Some(get_water_tile_type(m, t)? == COAST)
}
#[allow(clippy::inline_always)]
#[inline(always)]
pub(crate) fn is_non_flooding_water_tile(m: &TileTypeAccess, t: u32) -> Option<bool> {
    let fields = m.require_type(t, MP_WATER)?;
    Some(fields.m3() & 1 != 0)
}

/// `IsWater`.
pub(crate) fn is_water(m: &TileTypeAccess, tile: u32) -> Option<bool> {
    Some(get_water_tile_type(m, tile)? == 0)
}

#[cfg(test)]
mod tests {
    use super::{get_water_class, has_tile_water_class};
    use crate::map_func::{BaseTileLayout, ExtendedTileLayout, TileTypeAccess, TileTypeAllocation};
    use crate::tile_type::{MP_CLEAR, MP_INDUSTRY, MP_OBJECT, MP_STATION, MP_TREES, MP_WATER};
    use std::cell::Cell;

    thread_local! {
        static ASSERTS: Cell<u32> = const { Cell::new(0) };
        static MUTATE: Cell<*mut BaseTileLayout> = const { Cell::new(std::ptr::null_mut()) };
    }
    extern "C" fn no_callback(_: u32) -> u8 {
        std::process::abort()
    }
    extern "C" fn returning_assert(ok: bool) {
        assert!(!ok);
        let calls = ASSERTS.get() + 1;
        ASSERTS.set(calls);
        let tile = MUTATE.get();
        if !tile.is_null() {
            // SAFETY: The test supplies live backing, without a tile reference.
            #[allow(unsafe_code)]
            unsafe {
                (*tile).kind = if calls < 3 { MP_TREES } else { MP_INDUSTRY } << 4;
                (*tile).m1 = 2 << 5;
            }
        }
    }

    // Tile 1 is readable but outside the logical one-tile map. This distinguishes
    // the original repeated GetTileType assertions without out-of-allocation reads.
    #[test]
    #[allow(unsafe_code)]
    fn water_class_returning_assertions_keep_short_circuit_observations() {
        let mut base = std::array::from_fn::<_, 2, _>(|_| BaseTileLayout {
            kind: 0,
            height: 0,
            m2: 0,
            m1: 1 << 5,
            m3: 0,
            m4: 0,
            m5: 0,
        });
        let extended = std::array::from_fn::<_, 2, _>(|_| ExtendedTileLayout {
            m6: 0,
            m7: 0,
            m8: 0,
        });
        let base_pointer = base.as_mut_ptr();
        for assertions in [true, false] {
            let map = unsafe {
                TileTypeAccess::raw(
                    TileTypeAllocation {
                        base: base_pointer,
                        extended: extended.as_ptr(),
                        count: 1,
                        assertions,
                    },
                    no_callback,
                )
            };
            for (kind, checks) in [
                (MP_WATER, 1),
                (MP_STATION, 2),
                (MP_INDUSTRY, 3),
                (MP_OBJECT, 4),
                (MP_TREES, 5),
                (MP_CLEAR, 5),
            ] {
                unsafe {
                    (*base_pointer.add(1)).kind = kind << 4;
                    (*base_pointer.add(1)).m1 = 1 << 5;
                }
                ASSERTS.set(0);
                assert_eq!(
                    map.with_native_assert(returning_assert, |m| has_tile_water_class(m, 1)),
                    Some(kind != MP_CLEAR)
                );
                assert_eq!(ASSERTS.get(), if assertions { checks } else { 0 });
                ASSERTS.set(0);
                assert_eq!(
                    map.with_native_assert(returning_assert, |m| get_water_class(m, 1)),
                    Some(1)
                );
                assert_eq!(
                    ASSERTS.get(),
                    if assertions {
                        checks + u32::from(kind == MP_CLEAR)
                    } else {
                        0
                    }
                );
            }
            if assertions {
                unsafe { (*base_pointer.add(1)).kind = MP_STATION << 4 };
                MUTATE.set(base_pointer.wrapping_add(1));
                ASSERTS.set(0);
                assert_eq!(
                    map.with_native_assert(returning_assert, |m| get_water_class(m, 1)),
                    Some(2)
                );
                MUTATE.set(std::ptr::null_mut());
                assert_eq!(ASSERTS.get(), 3);
            }
        }
    }
}
