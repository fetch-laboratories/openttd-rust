/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared header helpers over the canonical map descriptor.

impl crate::map_func::World<'_> {
    pub(crate) fn tunnel(&self, t: u32) -> bool {
        self.check(self.kind(t) == crate::tile_type::MP_TUNNELBRIDGE);
        !crate::bitmath_func::has_bit(Self::byte(self.types.m5(t)), 7)
    }
}
