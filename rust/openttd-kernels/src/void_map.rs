/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `void_map.h` writes through the audited map view.

/// `MakeVoid`: make a nice void tile ;)
use crate::tile_type::TileIndex;

pub(crate) fn make_void(
    w: &crate::map_func::World<'_>,
    t: crate::tile_type::TileIndex,
    freeform_edges: impl FnOnce() -> bool,
) {
    let t = t.0;
    crate::tile_map::set_tile_type(w, t, crate::tile_type::MP_VOID, freeform_edges);
    crate::tile_map::set_tile_height(w, t, 0);
    let m = &w.types;
    let _ = (
        m.set_m1(TileIndex(t), 0),
        m.set_m2(TileIndex(t), 0),
        m.set_m3(TileIndex(t), 0),
        m.set_m4(TileIndex(t), 0),
    );
    let _ = (
        m.set_m5(TileIndex(t), 0),
        m.set_m6(TileIndex(t), 0),
        m.set_m7(TileIndex(t), 0),
        m.set_m8(TileIndex(t), 0),
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
