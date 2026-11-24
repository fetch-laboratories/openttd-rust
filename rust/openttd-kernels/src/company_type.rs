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
/// `OWNER_END`.
pub const OWNER_END: u8 = 0x13;
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

/// Opaque native `company_type.h::CompanyManagerFace`.
///
/// Native storage owns the style index, all 32 style-dependent face bits and the
/// style label string. Services borrow it while its containing company or GUI
/// face remains live; Rust neither owns nor reproduces that record's layout.
#[repr(C)]
pub struct CompanyManagerFace {
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

/// `CompanyMask`: a set of companies, one bit per `CompanyID`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CompanyMask(pub u16);
impl CompanyMask {
    /// `BaseBitSet::MASK` of `CompanyMask`: every bit of its `uint16_t` storage.
    pub const MASK: u16 = u16::MAX;
    /// The bit `BaseBitSet` shifts in for a company: `1ULL << value`, truncated to the
    /// `uint16_t` storage, so an owner of 16 or more has no bit.
    const fn bit(company: CompanyID) -> u16 {
        match 1_u64.checked_shl(company.0 as u32) {
            Some(bit) => bit as u16,
            None => 0,
        }
    }
    /// `BaseBitSet::Test`.
    #[must_use]
    pub const fn test(self, company: CompanyID) -> bool {
        self.0 & Self::bit(company) != 0
    }
    /// `BaseBitSet::All`: whether every company is in the set.
    #[must_use]
    pub const fn all(self) -> bool {
        self.0 == Self::MASK
    }
    /// `BaseBitSet::Any`: whether any company is in the set.
    #[must_use]
    pub const fn any(self) -> bool {
        self.0 != 0
    }
    /// `BaseBitSet::None`: whether no company is in the set.
    #[must_use]
    pub const fn none(self) -> bool {
        self.0 == 0
    }
    /// `BaseBitSet::Set`.
    pub const fn set(&mut self, company: CompanyID) -> &mut Self {
        self.0 |= Self::bit(company);
        self
    }
    /// `BaseBitSet::Set()`: every company.
    pub const fn set_all(&mut self) -> &mut Self {
        self.0 = Self::MASK;
        self
    }
    /// `BaseBitSet::Reset()`: no company.
    pub const fn reset_all(&mut self) -> &mut Self {
        self.0 = 0;
        self
    }
}
impl CompanyID {
    /// Native `OWNER_TOWN`.
    pub const TOWN: Self = Self(0x0F);
    /// Native `OWNER_NONE`.
    pub const NONE: Self = Self(0x10);
    /// Native `OWNER_WATER`.
    pub const WATER: Self = Self(0x11);
    /// Native `OWNER_DEITY`.
    pub const DEITY: Self = Self(0x12);
    /// Native `COMPANY_NEW_COMPANY`: the client wants a new company.
    pub const NEW_COMPANY: Self = Self(254);
    /// Native `COMPANY_SPECTATOR`.
    pub const SPECTATOR: Self = Self(0xFF);
}

/// Native name limits, including the terminating character.
pub const MAX_LENGTH_COMPANY_NAME_CHARS: usize = 32;
/// Native `MAX_LENGTH_PRESIDENT_NAME_CHARS`.
pub const MAX_LENGTH_PRESIDENT_NAME_CHARS: usize = 32;

/// `COMPANY_NEW_COMPANY`.
pub const COMPANY_NEW_COMPANY: u8 = CompanyID::NEW_COMPANY.0;

/// `MAX_HISTORY_QUARTERS`: the maximum number of quarters kept as performance's history.
pub const MAX_HISTORY_QUARTERS: usize = 24;

/// `CompanyRemoveReason`: the reason why the company was removed.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CompanyRemoveReason(pub u8);
impl CompanyRemoveReason {
    /// The company is manually removed.
    pub const CRR_MANUAL: Self = Self(0);
    /// The company is removed due to autoclean.
    pub const CRR_AUTOCLEAN: Self = Self(1);
    /// The company went belly-up.
    pub const CRR_BANKRUPT: Self = Self(2);
    /// Sentinel for end.
    pub const CRR_END: Self = Self(3);
    /// Dummy reason for actions that don't need one.
    pub const CRR_NONE: Self = Self::CRR_MANUAL;
}

/// `CompanyCtrlAction`: the action to do with `CMD_COMPANY_CTRL`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompanyCtrlAction(pub u8);
impl CompanyCtrlAction {
    /// Create a new company.
    pub const CCA_NEW: Self = Self(0);
    /// Create a new AI company.
    pub const CCA_NEW_AI: Self = Self(1);
    /// Delete a company.
    pub const CCA_DELETE: Self = Self(2);
    /// Sentinel for end.
    pub const CCA_END: Self = Self(3);
}

/// `CompanyAllowListCtrlAction`: the action to do with `CMD_COMPANY_ALLOW_LIST_CTRL`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompanyAllowListCtrlAction(pub u8);
impl CompanyAllowListCtrlAction {
    /// Create a public key.
    pub const CALCA_ADD: Self = Self(0);
    /// Remove a public key.
    pub const CALCA_REMOVE: Self = Self(1);
    /// Sentinel for end.
    pub const CALCA_END: Self = Self(2);
}

/// `LoanCommand` of `misc_cmd.h`, beside the other command enums: how a loan command reads
/// its amount.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoanCommand(pub u8);
impl LoanCommand {
    /// One `LOAN_INTERVAL`.
    pub const INTERVAL: Self = Self(0);
    /// As much as possible.
    pub const MAX: Self = Self(1);
    /// The amount the command gives.
    pub const AMOUNT: Self = Self(2);
}

impl CompanyID {
    /// Native `PoolID::End()`.
    pub const END: Self = Self(15);
}
