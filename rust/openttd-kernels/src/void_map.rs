/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `void_map.h` writes through the audited map view.

/// `MakeVoid`.
pub(crate) fn make_void(
    w: &crate::map_func::World<'_>,
    t: u32,
    freeform_edges: impl FnOnce() -> bool,
) {
    crate::tile_map::set_tile_type(w, t, crate::tile_type::MP_VOID, freeform_edges);
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
