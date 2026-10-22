/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Named cache flags from `subsidy_type.h`.
/// Native `PartOfSubsidy : uint8_t`.
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum PartOfSubsidy {
    /// Source of a subsidised route.
    Source = 0,
    /// Destination of a subsidised route.
    Destination = 1,
}
