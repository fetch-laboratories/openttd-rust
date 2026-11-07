/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Industry trigger and availability vocabulary from `newgrf_industries.h`.

/// Native `newgrf_industries.h::IndustryTrigger`, retaining the full underlying integer domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryTrigger(pub u8);
impl IndustryTrigger {
    /// Triggered each tile loop.
    pub const INDUSTRY_TRIGGER_TILELOOP_PROCESS: Self = Self(1);
    /// Triggered for the whole industry each 256 ticks.
    pub const INDUSTRY_TRIGGER_256_TICKS: Self = Self(2);
    /// Triggered on cargo delivery.
    pub const INDUSTRY_TRIGGER_CARGO_DELIVERY: Self = Self(4);
}

/// Native `newgrf_industries.h::IndustryAvailabilityCallType`, retaining the full underlying integer domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryAvailabilityCallType(pub u8);
impl IndustryAvailabilityCallType {
    /// during random map generation
    pub const IACT_MAPGENERATION: Self = Self(0);
    /// during creation of random ingame industry
    pub const IACT_RANDOMCREATION: Self = Self(1);
    /// from the Fund/build window
    pub const IACT_USERCREATION: Self = Self(2);
    /// from the Fund/build using prospecting
    pub const IACT_PROSPECTCREATION: Self = Self(3);
}
