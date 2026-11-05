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
        pub map_x: *const u8,
        pub map_y: *const u8,
        pub max_bridge_length: *const u16,
        pub max_companies: *const u8,
        pub max_competitors: *const u8,
        pub never_expire_airports: *const bool,
        pub never_expire_vehicles: *const bool,
        pub recalc_interval: *const u16,
        pub se_flat_world_height: *const u8,
        pub secondary_colour: *const u8,
        pub station_noise: *const bool,
        pub tree_placer: *const u8,
        pub used_liveries: *const u64,
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
        map_x: std::ptr::null(),
        map_y: std::ptr::null(),
        max_bridge_length: std::ptr::null(),
        max_companies: std::ptr::null(),
        max_competitors: std::ptr::null(),
        never_expire_airports: std::ptr::null(),
        never_expire_vehicles: std::ptr::null(),
        recalc_interval: std::ptr::null(),
        se_flat_world_height: std::ptr::null(),
        secondary_colour: std::ptr::null(),
        station_noise: std::ptr::null(),
        tree_placer: std::ptr::null(),
        used_liveries: std::ptr::null(),
    };
}
