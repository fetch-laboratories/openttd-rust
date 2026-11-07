/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared definitions from `newgrf_text_type.h`.

/// GRF-internal string ID, distinct from an OpenTTD string ID.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GRFStringID(pub u32);

impl GRFStringID {
    /// Miscellaneous GRF text range.
    pub const GRFSTR_MISC_GRF_TEXT: Self = Self(0xD000);
}
/// Thorn indicates a Unicode string to NFO.
pub const NFO_UTF8_IDENTIFIER: u32 = 0x00DE;
