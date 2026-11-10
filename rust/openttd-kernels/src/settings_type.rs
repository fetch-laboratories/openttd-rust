/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared settings records from `settings_type.h`.
//! Canonical native allocations survive settings assignment. Raw field accesses are serialized by
//! the game-state mutex and end before callbacks; values are reloaded at each source observation.
#![allow(unsafe_code)]
/// `SettingsProfile::SP_CUSTOM`.
pub const SP_CUSTOM: u32 = 3;

crate::abi::shared_records! {
    fn abi_record;
    /// Canonical construction-rate settings, embedded in `ConstructionSettings`.
    "OpenTTDConstructionLimits" ["settings"] => pub struct ConstructionLimits {
        pub terraform_per_64k_frames: u32,
        pub terraform_frame_burst: u16,
        pub clear_per_64k_frames: u32,
        pub clear_frame_burst: u16,
        pub tree_per_64k_frames: u32,
        pub tree_frame_burst: u16,
        pub build_object_per_64k_frames: u32,
        pub build_object_frame_burst: u16,
    }
    /// Process-lifetime addresses of canonical settings, shared by simulation owners.
    "OpenTTDSettingAddresses" ["settings"] => pub struct SettingAddresses {
        pub allow_town_roads: *const bool,
        pub ambient: *const bool,
        pub climate: *const u8,
        pub competitor_interval: *const u16,
        pub construction_limits: *const ConstructionLimits,
        pub disable_elrails: *const bool,
        pub extra_tree_placement: *const u8,
        pub freeform_edges: *const bool,
        pub generation_seed: *const u32,
        pub height_limit: *const u8,
        pub heightmap_height: *const u8,
        pub industry_cargo_scale: *const u16,
        pub land_generator: *const u8,
        pub lost_vehicle_warn: *const bool,
        pub map_x: *const u8,
        pub map_y: *const u8,
        pub max_bridge_length: *const u16,
        pub max_companies: *const u8,
        pub max_competitors: *const u8,
        pub max_trains: *const u16,
        pub maximum_go_to_depot_penalty: *const u32,
        pub never_expire_airports: *const bool,
        pub never_expire_vehicles: *const bool,
        pub recalc_interval: *const u16,
        pub reverse_at_signals: *const bool,
        pub roadveh_acceleration_model: *const u8,
        pub se_flat_world_height: *const u8,
        pub secondary_colour: *const u8,
        pub station_noise: *const bool,
        pub train_acceleration_model: *const u8,
        pub tree_placer: *const u8,
        pub used_liveries: *const u64,
        pub wait_for_pbs_path: *const u8,
        pub wait_oneway_signal: *const u8,
        pub wait_twoway_signal: *const u8,
    }

}

#[cfg(test)]
impl SettingAddresses {
    /// A catalog with every address null; tests fill the fields their entry reads.
    pub(crate) const NULL: Self = Self {
        allow_town_roads: std::ptr::null(),
        ambient: std::ptr::null(),
        climate: std::ptr::null(),
        competitor_interval: std::ptr::null(),
        construction_limits: std::ptr::null(),
        disable_elrails: std::ptr::null(),
        extra_tree_placement: std::ptr::null(),
        freeform_edges: std::ptr::null(),
        generation_seed: std::ptr::null(),
        height_limit: std::ptr::null(),
        heightmap_height: std::ptr::null(),
        industry_cargo_scale: std::ptr::null(),
        land_generator: std::ptr::null(),
        lost_vehicle_warn: std::ptr::null(),
        map_x: std::ptr::null(),
        map_y: std::ptr::null(),
        max_bridge_length: std::ptr::null(),
        max_companies: std::ptr::null(),
        max_competitors: std::ptr::null(),
        max_trains: std::ptr::null(),
        maximum_go_to_depot_penalty: std::ptr::null(),
        never_expire_airports: std::ptr::null(),
        never_expire_vehicles: std::ptr::null(),
        recalc_interval: std::ptr::null(),
        reverse_at_signals: std::ptr::null(),
        roadveh_acceleration_model: std::ptr::null(),
        se_flat_world_height: std::ptr::null(),
        secondary_colour: std::ptr::null(),
        station_noise: std::ptr::null(),
        train_acceleration_model: std::ptr::null(),
        tree_placer: std::ptr::null(),
        used_liveries: std::ptr::null(),
        wait_for_pbs_path: std::ptr::null(),
        wait_oneway_signal: std::ptr::null(),
        wait_twoway_signal: std::ptr::null(),
    };
}

/// Possible values for the `place_houses` setting (`settings_type.h`), retaining every underlying byte.
/// `PH_ALLOWED` (1) has no Rust reader; the settings table stays native.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaceHouses(pub u8);
impl PlaceHouses {
    /// Placing houses is forbidden.
    pub const PH_FORBIDDEN: Self = Self(0);
    /// Placing houses is allowed; they are placed fully constructed.
    pub const PH_ALLOWED_CONSTRUCTED: Self = Self(2);
}

/// Native `settings_type.h::TreePlacer`, retaining the full underlying integer domain.
///
/// List of tree placer algorithm.
///
/// This enumeration defines all possible tree placer algorithm in the game.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TreePlacer(pub u8);
impl TreePlacer {
    /// No tree placer algorithm
    pub const TP_NONE: Self = Self(0);
    /// The original algorithm
    pub const TP_ORIGINAL: Self = Self(1);
    /// A 'improved' algorithm
    pub const TP_IMPROVED: Self = Self(2);
}
/// Raw adapter: `TP_NONE` as a base value, for files no pass has finished yet.
pub const TP_NONE: u8 = TreePlacer::TP_NONE.0;
/// Raw adapter: `TP_ORIGINAL` as a base value.
pub const TP_ORIGINAL: u8 = TreePlacer::TP_ORIGINAL.0;
/// Raw adapter: `TP_IMPROVED` as a base value.
pub const TP_IMPROVED: u8 = TreePlacer::TP_IMPROVED.0;

/// Native `settings_type.h::IndustryDensity`, retaining the full underlying integer domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryDensity(pub u8);
impl IndustryDensity {
    /// The game does not build industries.
    pub const ID_FUND_ONLY: Self = Self(0);
    /// Start with just the industries that must be present.
    pub const ID_MINIMAL: Self = Self(1);
    /// Very few industries at game start.
    pub const ID_VERY_LOW: Self = Self(2);
    /// Few industries at game start.
    pub const ID_LOW: Self = Self(3);
    /// Normal amount of industries at game start.
    pub const ID_NORMAL: Self = Self(4);
    /// Many industries at game start.
    pub const ID_HIGH: Self = Self(5);
    /// Custom number of industries.
    pub const ID_CUSTOM: Self = Self(6);
    /// Number of industry density settings.
    pub const ID_END: Self = Self(7);
}
