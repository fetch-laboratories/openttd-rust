/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Functions related to `NewGRF` debugging (`newgrf_debug.h`). The native entry point is in
//! the window table of `window_func.rs`.

use crate::newgrf::GrfSpecFeature;

/// `DeleteNewGRFInspectWindow`: delete inspect window for a given feature and index. The
/// index is normally an in-game location/identifier, such as a `TileIndex` or an
/// `IndustryID`, passed as its base.
pub(crate) fn delete_newgrf_inspect_window(feature: GrfSpecFeature, index: u32) {
    (crate::window_func::services().delete_newgrf_inspect_window)(feature.0, index);
}

/// `InvalidateNewGRFInspectWindow`: invalidate the inspect window for a given feature and
/// index. The index is normally an in-game location/identifier, such as a `TileIndex` or an
/// `IndustryID`, passed as its base.
pub(crate) fn invalidate_newgrf_inspect_window(feature: GrfSpecFeature, index: u32) {
    (crate::window_func::services().invalidate_newgrf_inspect_window)(feature.0, index);
}
