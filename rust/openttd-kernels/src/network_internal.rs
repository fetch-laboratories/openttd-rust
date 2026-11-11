/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `NetworkJoinStatus` from `network_internal.h`.

/// Status of the clients during joining, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetworkJoinStatus(pub u8);

#[allow(dead_code)] // Complete original enumerator vocabulary.
impl NetworkJoinStatus {
    /// Native `NETWORK_JOIN_STATUS_CONNECTING`.
    pub const NETWORK_JOIN_STATUS_CONNECTING: Self = Self(0);
    /// Native `NETWORK_JOIN_STATUS_AUTHORIZING`.
    pub const NETWORK_JOIN_STATUS_AUTHORIZING: Self = Self(1);
    /// Native `NETWORK_JOIN_STATUS_WAITING`.
    pub const NETWORK_JOIN_STATUS_WAITING: Self = Self(2);
    /// Native `NETWORK_JOIN_STATUS_DOWNLOADING`.
    pub const NETWORK_JOIN_STATUS_DOWNLOADING: Self = Self(3);
    /// Native `NETWORK_JOIN_STATUS_PROCESSING`.
    pub const NETWORK_JOIN_STATUS_PROCESSING: Self = Self(4);
    /// Native `NETWORK_JOIN_STATUS_REGISTERING`.
    pub const NETWORK_JOIN_STATUS_REGISTERING: Self = Self(5);

    /// Native `NETWORK_JOIN_STATUS_GETTING_COMPANY_INFO`.
    pub const NETWORK_JOIN_STATUS_GETTING_COMPANY_INFO: Self = Self(6);
    /// Native `NETWORK_JOIN_STATUS_END`.
    pub const NETWORK_JOIN_STATUS_END: Self = Self(7);
}
