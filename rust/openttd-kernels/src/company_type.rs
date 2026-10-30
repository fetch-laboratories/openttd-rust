/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `company_type.h` owner values. `services.cpp` asserts every value.

/// `OWNER_TOWN`.
pub const OWNER_TOWN: u8 = Owner::TOWN.0;
/// `OWNER_NONE`.
pub const OWNER_NONE: u8 = Owner::NONE.0;
/// `OWNER_WATER`.
pub const OWNER_WATER: u8 = Owner::WATER.0;
/// `OWNER_DEITY`.
pub const OWNER_DEITY: u8 = Owner::DEITY.0;
/// `MAX_COMPANIES`.
pub const MAX_COMPANIES: u8 = 0x0F;
/// `INVALID_OWNER`.
pub const INVALID_OWNER: u8 = Owner::INVALID.0;

/// `COMPANY_SPECTATOR`.
pub const COMPANY_SPECTATOR: u8 = CompanyID::SPECTATOR.0;

/// Opaque native `Company`; valid only while its native pool shell lives.
#[repr(C)]
pub struct OpenTTDCompany {
    _private: [u8; 0],
}

/// Native `CompanyID` with its full original integer domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompanyID(pub u8);
impl CompanyID {
    /// Native first company ID.
    pub const BEGIN: Self = Self(0);
    /// Native invalid sentinel.
    pub const INVALID: Self = Self(u8::MAX);
}

/// Native `Owner` is the same type as `CompanyID`.
pub type Owner = CompanyID;
impl CompanyID {
    /// Native `OWNER_TOWN`.
    pub const TOWN: Self = Self(0x0F);
    /// Native `OWNER_NONE`.
    pub const NONE: Self = Self(0x10);
    /// Native `OWNER_WATER`.
    pub const WATER: Self = Self(0x11);
    /// Native `OWNER_DEITY`.
    pub const DEITY: Self = Self(0x12);
    /// Native `COMPANY_SPECTATOR`.
    pub const SPECTATOR: Self = Self(0xFF);
}

/// Native name limits, including the terminating character.
pub const MAX_LENGTH_COMPANY_NAME_CHARS: usize = 32;
pub const MAX_LENGTH_PRESIDENT_NAME_CHARS: usize = 32;
