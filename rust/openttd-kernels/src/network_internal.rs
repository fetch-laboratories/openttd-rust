/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `NetworkJoinStatus` and `ShowNetworkError` from `network_internal.h`.

use crate::company_type::CompanyID;
use crate::error::{WarningLevel, show_error_message};
use crate::openttd::SwitchMode;
use crate::strings_func::get_encoded_string;
use crate::strings_type::{EncodedString, StringID};

/// Status of the clients during joining, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetworkJoinStatus(pub u8);

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

/// `ShowNetworkError` (`network.cpp`): there was a non-recoverable error, drop back to the main
/// menu with a nice error.
pub(crate) fn show_network_error(error_string: StringID) {
    crate::session::set_pending(SwitchMode::MENU);
    show_error_message(
        &get_encoded_string(error_string, &[]),
        &EncodedString::default(),
        WarningLevel::WL_CRITICAL,
        0,
        0,
        &EncodedString::default(),
        CompanyID::INVALID,
    );
}

#[cfg(test)]
mod tests {
    use crate::news_func::testing::{encoded, error_message, on_error_message, take};
    use crate::openttd::SwitchMode;
    use crate::table::strings::STR_NETWORK_ERROR_DESYNC;
    use std::cell::Cell;

    thread_local! {
        static PENDING_AT_MESSAGE: Cell<Option<SwitchMode>> = const { Cell::new(None) };
    }
    fn observe_pending() {
        PENDING_AT_MESSAGE.set(Some(crate::session::pending()));
    }

    /// `ShowNetworkError` sets the pending mode to the menu before it shows the message as
    /// critical.
    #[test]
    fn show_network_error_switches_to_the_menu_and_shows_a_critical_message() {
        take();
        let saved = crate::session::pending();
        crate::session::set_pending(SwitchMode::NONE);
        on_error_message(Some(observe_pending));
        super::show_network_error(STR_NETWORK_ERROR_DESYNC);
        on_error_message(None);
        crate::session::set_pending(saved);
        assert_eq!(PENDING_AT_MESSAGE.get(), Some(SwitchMode::MENU));
        assert_eq!(
            take(),
            [error_message(
                encoded(STR_NETWORK_ERROR_DESYNC.0, &[]),
                Vec::new(),
                3
            )]
        );
    }
}
