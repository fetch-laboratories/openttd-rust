/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Flags from `waypoint_base.h`.
/// The road-waypoint bit in `Waypoint::waypoint_flags`.
pub(crate) const WPF_ROAD: u8 = 0;

/// Whether `Waypoint::waypoint_flags` marks a road waypoint.
pub(crate) fn is_road(flags: u16) -> bool {
    flags & (1 << WPF_ROAD) != 0
}
