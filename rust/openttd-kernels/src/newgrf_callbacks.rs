/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared constants and the native callback ID domain from `newgrf_callbacks.h`.

/// Native `CallbackID : uint16_t`, preserving unnamed values too.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CallbackID(pub u16);

pub const CBID_VEHICLE_LENGTH: CallbackID = CallbackID(0x11);
pub const CBID_VEHICLE_32DAY_CALLBACK: CallbackID = CallbackID(0x32);

/// `CALLBACK_FAILED`.
pub const CALLBACK_FAILED: u16 = 0xFFFF;
