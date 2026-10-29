/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Native save/load completion results.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct SaveOrLoadResult(pub u8);
pub(crate) const SL_OK: SaveOrLoadResult = SaveOrLoadResult(0);
pub(crate) const SL_REINIT: SaveOrLoadResult = SaveOrLoadResult(2);
