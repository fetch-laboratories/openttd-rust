/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Functions related to depots (`depot_func.h`).
use crate::direction_type::DiagDirection;
use crate::slope_type::Slope;

/// `CanBuildDepotByTileh`: find out if the slope of the tile is suitable to
/// build a depot with given direction.
pub(crate) fn can_build_depot_by_tileh(direction: DiagDirection, tileh: Slope) -> bool {
    crate::services_assert::assert_with(|| tileh != Slope::SLOPE_FLAT);
    let entrance_corners =
        crate::slope_func::inclined_slope(direction, || crate::services_assert::not_reached());
    /* For steep slopes both entrance corners must be raised (i.e. neither of them is the lowest corner),
     * For non-steep slopes at least one corner must be raised. */
    if crate::slope_func::is_steep_slope_raw(tileh.0) {
        tileh & entrance_corners == entrance_corners
    } else {
        tileh & entrance_corners != Slope::SLOPE_FLAT
    }
}
