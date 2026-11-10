/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `road_func.h` helpers.
/// `DiagDirToRoadBits`, with the native valid direction precondition.
pub fn diag_dir_to_road_bits(dir: u8, check: impl Fn(bool) + Copy) -> u8 {
    if dir >= 4 {
        check(false);
    }
    (u32::from(crate::road_type::ROAD_NW).wrapping_shl(u32::from(3 ^ dir)) & 0xff) as u8
}

/// `MirrorRoadBits`, retaining `IsValidRoadBits`'s precondition.
pub fn mirror_road_bits(bits: u8, check: impl Fn(bool) + Copy) -> u8 {
    if bits >= crate::road_type::ROAD_END {
        check(false);
    }
    (bits & 3) << 2 | (bits >> 2) & 3
}

/// `AxisToRoadBits`: create the road-part which belongs to the given `Axis`.
pub(crate) fn axis_to_road_bits(
    a: crate::direction_type::Axis,
    check: impl Fn(bool) + Copy,
) -> crate::road_type::RoadBits {
    check(crate::direction_func::is_valid_axis_raw(a.0));
    if a == crate::direction_type::Axis::AXIS_X {
        crate::road_type::RoadBits::ROAD_X
    } else {
        crate::road_type::RoadBits::ROAD_Y
    }
}
