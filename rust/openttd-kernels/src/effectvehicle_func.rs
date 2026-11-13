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

/// `EV_CHIMNEY_SMOKE`.
pub const EV_CHIMNEY_SMOKE: u8 = EffectVehicleType::EV_CHIMNEY_SMOKE.0;
/// `EV_STEAM_SMOKE`.
pub const EV_STEAM_SMOKE: u8 = EffectVehicleType::EV_STEAM_SMOKE.0;
/// `EV_DIESEL_SMOKE`.
pub const EV_DIESEL_SMOKE: u8 = EffectVehicleType::EV_DIESEL_SMOKE.0;
/// `EV_ELECTRIC_SPARK`.
pub const EV_ELECTRIC_SPARK: u8 = EffectVehicleType::EV_ELECTRIC_SPARK.0;
/// `EV_CRASH_SMOKE`.
pub const EV_CRASH_SMOKE: u8 = EffectVehicleType::EV_CRASH_SMOKE.0;
/// `EV_EXPLOSION_LARGE`.
pub const EV_EXPLOSION_LARGE: u8 = EffectVehicleType::EV_EXPLOSION_LARGE.0;
/// `EV_BREAKDOWN_SMOKE`.
pub const EV_BREAKDOWN_SMOKE: u8 = EffectVehicleType::EV_BREAKDOWN_SMOKE.0;
/// `EV_EXPLOSION_SMALL`.
pub const EV_EXPLOSION_SMALL: u8 = EffectVehicleType::EV_EXPLOSION_SMALL.0;
/// `EV_BULLDOZER`.
pub const EV_BULLDOZER: u8 = EffectVehicleType::EV_BULLDOZER.0;
/// `EV_BUBBLE`.
pub const EV_BUBBLE: u8 = EffectVehicleType::EV_BUBBLE.0;
/// `EV_BREAKDOWN_SMOKE_AIRCRAFT`.
pub const EV_BREAKDOWN_SMOKE_AIRCRAFT: u8 = EffectVehicleType::EV_BREAKDOWN_SMOKE_AIRCRAFT.0;
/// `EV_COPPER_MINE_SMOKE`.
pub const EV_COPPER_MINE_SMOKE: u8 = EffectVehicleType::EV_COPPER_MINE_SMOKE.0;
/// `EV_END`.
pub const EV_END: u8 = EffectVehicleType::EV_END.0;
