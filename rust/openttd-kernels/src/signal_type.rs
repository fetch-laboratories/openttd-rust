/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Signal type values from `signal_type.h`.
/// `SIGTYPE_BLOCK`.
pub const SIGTYPE_BLOCK: u8 = SignalType::SIGTYPE_BLOCK.0;
/// `SIGTYPE_PBS_ONEWAY`.
pub const SIGTYPE_PBS_ONEWAY: u8 = SignalType::SIGTYPE_PBS_ONEWAY.0;
/// `SIGTYPE_PBS`.
pub const SIGTYPE_PBS: u8 = SignalType::SIGTYPE_PBS.0;
/// `SIGNAL_STATE_RED`.
pub const SIGNAL_STATE_RED: u8 = SignalState::SIGNAL_STATE_RED.0;
/// `SIGNAL_STATE_GREEN`.
pub const SIGNAL_STATE_GREEN: u8 = SignalState::SIGNAL_STATE_GREEN.0;

/// Native `SignalVariant` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct SignalVariant(pub u8);

/// Native `SignalType` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct SignalType(pub u8);

/// Native `SignalState` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct SignalState(pub u8);

#[allow(non_upper_case_globals)]
impl SignalVariant {
    /// Light signal
    pub const SIG_ELECTRIC: Self = Self(0);
    /// Old-fashioned semaphore signal
    pub const SIG_SEMAPHORE: Self = Self(1);
}

#[allow(non_upper_case_globals)]
impl SignalType {
    /// block signal
    pub const SIGTYPE_BLOCK: Self = Self(0);
    /// presignal block entry
    pub const SIGTYPE_ENTRY: Self = Self(1);
    /// presignal block exit
    pub const SIGTYPE_EXIT: Self = Self(2);
    /// presignal inter-block
    pub const SIGTYPE_COMBO: Self = Self(3);
    /// normal pbs signal
    pub const SIGTYPE_PBS: Self = Self(4);
    /// no-entry signal
    pub const SIGTYPE_PBS_ONEWAY: Self = Self(5);
    /// Native `SIGTYPE_END`.
    pub const SIGTYPE_END: Self = Self(6);
    /// Native `SIGTYPE_LAST`.
    pub const SIGTYPE_LAST: Self = Self(5);
    /// Native `SIGTYPE_LAST_NOPBS`.
    pub const SIGTYPE_LAST_NOPBS: Self = Self(3);
}

#[allow(non_upper_case_globals)]
impl SignalState {
    /// The signal is red
    pub const SIGNAL_STATE_RED: Self = Self(0);
    /// The signal is green
    pub const SIGNAL_STATE_GREEN: Self = Self(1);
}
