/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Native abstract file kinds.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct AbstractFileType(pub u8);
pub(crate) const FT_SAVEGAME: AbstractFileType = AbstractFileType(1);
pub(crate) const FT_SCENARIO: AbstractFileType = AbstractFileType(2);
pub(crate) const FT_HEIGHTMAP: AbstractFileType = AbstractFileType(3);
