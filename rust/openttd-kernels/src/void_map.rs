/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `void_map.h` writes through the audited map view.

use crate::tile_type::TileIndex;

/// `MakeVoid`: make a nice void tile ;)
pub(crate) fn make_void(
    w: &crate::map_func::World<'_>,
    t: TileIndex,
    freeform_edges: impl FnOnce() -> bool,
) {
    crate::tile_map::set_tile_type(
        w,
        TileIndex(t.0),
        crate::tile_type::TileType::MP_VOID,
        freeform_edges,
    );
    crate::tile_map::set_tile_height(w, t, 0);
    let m = &w.types;
    let _ = (
        m.set_m1(t, 0),
        m.set_m2(t, 0),
        m.set_m3(t, 0),
        m.set_m4(t, 0),
    );
    let _ = (
        m.set_m5(t, 0),
        m.set_m6(t, 0),
        m.set_m7(t, 0),
        m.set_m8(t, 0),
    );
}

/// Raw adapter: `MakeVoid` over a base tile, for files no pass has finished yet.
pub(crate) fn make_void_raw(
    w: &crate::map_func::World<'_>,
    t: u32,
    freeform_edges: impl FnOnce() -> bool,
) {
    make_void(w, crate::tile_type::TileIndex(t), freeform_edges);
}
