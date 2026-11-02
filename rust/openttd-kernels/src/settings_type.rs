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
        pub ambient: *const bool,
        pub climate: *const u8,
        pub competitor_interval: *const u16,
        pub construction_limits: *const ConstructionLimits,
        pub extra_tree_placement: *const u8,
        pub industry_cargo_scale: *const u16,
        pub map_x: *const u8,
        pub map_y: *const u8,
        pub max_companies: *const u8,
        pub max_competitors: *const u8,
        pub recalc_interval: *const u16,
        pub secondary_colour: *const u8,
        pub used_liveries: *const u64,
    }

}
