/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared `effectvehicle_func.h` effect identities. `services.cpp` pins native values.

/// Native `EffectVehicleType`: Effect vehicle types
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EffectVehicleType(pub u8);
impl EffectVehicleType {
    /// Smoke of power plant (industry).
    pub const EV_CHIMNEY_SMOKE: Self = Self(0);
    /// Smoke of steam engines.
    pub const EV_STEAM_SMOKE: Self = Self(1);
    /// Smoke of diesel engines.
    pub const EV_DIESEL_SMOKE: Self = Self(2);
    /// Sparks of electric engines.
    pub const EV_ELECTRIC_SPARK: Self = Self(3);
    /// Smoke of disasters.
    pub const EV_CRASH_SMOKE: Self = Self(4);
    /// Various explosions.
    pub const EV_EXPLOSION_LARGE: Self = Self(5);
    /// Smoke of broken vehicles except aircraft.
    pub const EV_BREAKDOWN_SMOKE: Self = Self(6);
    /// Various explosions.
    pub const EV_EXPLOSION_SMALL: Self = Self(7);
    /// Bulldozer at roadworks.
    pub const EV_BULLDOZER: Self = Self(8);
    /// Bubble of bubble generator (industry).
    pub const EV_BUBBLE: Self = Self(9);
    /// Smoke of broken aircraft.
    pub const EV_BREAKDOWN_SMOKE_AIRCRAFT: Self = Self(10);
    /// Smoke at copper mine.
    pub const EV_COPPER_MINE_SMOKE: Self = Self(11);
    /// `EV_END`.
    pub const EV_END: Self = Self(12);
}
