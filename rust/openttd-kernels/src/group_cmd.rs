/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared definitions from `group_cmd.h`.

/// Native `AlterGroupMode`: the action for `CmdAlterGroup`, retaining every underlying `u8`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlterGroupMode(pub u8);

impl AlterGroupMode {
    /// Change group name.
    #[allow(
        non_upper_case_globals,
        reason = "the original enumerator `AlterGroupMode::Rename`"
    )]
    pub const Rename: Self = Self(0);
    /// Change group parent.
    #[allow(
        non_upper_case_globals,
        reason = "the original enumerator `AlterGroupMode::SetParent`"
    )]
    pub const SetParent: Self = Self(1);
}
