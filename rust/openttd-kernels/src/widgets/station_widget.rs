/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Types related to the station widgets (`widgets/station_widget.h`).

use crate::window_type::WidgetID;

/// Widgets of the `StationViewWindow` class.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StationViewWidgets(pub WidgetID);

impl StationViewWidgets {
    /// 'Close airport' button.
    pub const WID_SV_CLOSE_AIRPORT: Self = Self(11);
}
