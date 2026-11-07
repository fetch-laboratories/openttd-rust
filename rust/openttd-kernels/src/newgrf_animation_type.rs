/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared animation statuses from `newgrf_animation_type.h`.
/// Native `ANIMATION_NON_LOOPING`.
pub const ANIMATION_NON_LOOPING: u8 = AnimationStatus::NonLooping.0;
/// Native `ANIMATION_LOOPING`.
pub const ANIMATION_LOOPING: u8 = AnimationStatus::Looping.0;

/// Native `AnimationStatus`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AnimationStatus(pub u8);

#[allow(non_upper_case_globals)]
impl AnimationStatus {
    /// Animation is not looping.
    pub const NonLooping: Self = Self(0);
    /// Animation is looping.
    pub const Looping: Self = Self(1);
    /// There is no animation.
    pub const NoAnimation: Self = Self(255);
}
