/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `NetworkJoinStatus`, `ShowNetworkError`, `GetNetworkErrorMsg` and `NetworkTextMessage` from
//! `network_internal.h`.
//!
//! `NetworkTextMessage` runs natively (`network.cpp`) until the chat GUI is ported, through the
//! news table of `news_func.rs`, which is bound once for the process.

use crate::company_type::CompanyID;
use crate::error::{WarningLevel, show_error_message};
use crate::gfx_type::TextColour;
use crate::network_type::{NetworkAction, NetworkErrorCode};
use crate::openttd::SwitchMode;
use crate::services::ByteSpan;
use crate::strings_func::{descriptor, get_encoded_string};
use crate::strings_type::{EncodedString, StringID, StringParameterData};
use crate::table::strings::{
    STR_NETWORK_ERROR_CLIENT_CHEATER, STR_NETWORK_ERROR_CLIENT_COMPANY_MISMATCH,
    STR_NETWORK_ERROR_CLIENT_CONNECTION_LOST, STR_NETWORK_ERROR_CLIENT_DESYNC,
    STR_NETWORK_ERROR_CLIENT_GENERAL, STR_NETWORK_ERROR_CLIENT_INVALID_CLIENT_NAME,
    STR_NETWORK_ERROR_CLIENT_KICKED, STR_NETWORK_ERROR_CLIENT_NAME_IN_USE,
    STR_NETWORK_ERROR_CLIENT_NEWGRF_MISMATCH,
    STR_NETWORK_ERROR_CLIENT_NO_AUTHENTICATION_METHOD_AVAILABLE,
    STR_NETWORK_ERROR_CLIENT_NOT_AUTHORIZED, STR_NETWORK_ERROR_CLIENT_NOT_EXPECTED,
    STR_NETWORK_ERROR_CLIENT_NOT_ON_ALLOW_LIST, STR_NETWORK_ERROR_CLIENT_PROTOCOL_ERROR,
    STR_NETWORK_ERROR_CLIENT_SAVEGAME, STR_NETWORK_ERROR_CLIENT_SERVER_FULL,
    STR_NETWORK_ERROR_CLIENT_TIMEOUT_COMPUTER, STR_NETWORK_ERROR_CLIENT_TIMEOUT_JOIN,
    STR_NETWORK_ERROR_CLIENT_TIMEOUT_MAP, STR_NETWORK_ERROR_CLIENT_TIMEOUT_PASSWORD,
    STR_NETWORK_ERROR_CLIENT_TOO_MANY_COMMANDS, STR_NETWORK_ERROR_CLIENT_WRONG_PASSWORD,
    STR_NETWORK_ERROR_CLIENT_WRONG_REVISION,
};

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

/// List of possible network errors, used by `PACKET_SERVER_ERROR` and `PACKET_CLIENT_ERROR`.
const NETWORK_ERROR_STRINGS: [StringID; 23] = [
    STR_NETWORK_ERROR_CLIENT_GENERAL,
    STR_NETWORK_ERROR_CLIENT_DESYNC,
    STR_NETWORK_ERROR_CLIENT_SAVEGAME,
    STR_NETWORK_ERROR_CLIENT_CONNECTION_LOST,
    STR_NETWORK_ERROR_CLIENT_PROTOCOL_ERROR,
    STR_NETWORK_ERROR_CLIENT_NEWGRF_MISMATCH,
    STR_NETWORK_ERROR_CLIENT_NOT_AUTHORIZED,
    STR_NETWORK_ERROR_CLIENT_NOT_EXPECTED,
    STR_NETWORK_ERROR_CLIENT_WRONG_REVISION,
    STR_NETWORK_ERROR_CLIENT_NAME_IN_USE,
    STR_NETWORK_ERROR_CLIENT_WRONG_PASSWORD,
    STR_NETWORK_ERROR_CLIENT_COMPANY_MISMATCH,
    STR_NETWORK_ERROR_CLIENT_KICKED,
    STR_NETWORK_ERROR_CLIENT_CHEATER,
    STR_NETWORK_ERROR_CLIENT_SERVER_FULL,
    STR_NETWORK_ERROR_CLIENT_TOO_MANY_COMMANDS,
    STR_NETWORK_ERROR_CLIENT_TIMEOUT_PASSWORD,
    STR_NETWORK_ERROR_CLIENT_TIMEOUT_COMPUTER,
    STR_NETWORK_ERROR_CLIENT_TIMEOUT_MAP,
    STR_NETWORK_ERROR_CLIENT_TIMEOUT_JOIN,
    STR_NETWORK_ERROR_CLIENT_INVALID_CLIENT_NAME,
    STR_NETWORK_ERROR_CLIENT_NOT_ON_ALLOW_LIST,
    STR_NETWORK_ERROR_CLIENT_NO_AUTHENTICATION_METHOD_AVAILABLE,
];
const _: () =
    assert!(NETWORK_ERROR_STRINGS.len() == NetworkErrorCode::NETWORK_ERROR_END.0 as usize);

/// `GetNetworkErrorMsg`: retrieve the string id of an internal error number.
pub(crate) fn get_network_error_msg(mut err: NetworkErrorCode) -> StringID {
    if usize::from(err.0) >= NETWORK_ERROR_STRINGS.len() {
        err = NetworkErrorCode::NETWORK_ERROR_GENERAL;
    }

    NETWORK_ERROR_STRINGS[usize::from(err.0)]
}

fn span(bytes: &[u8]) -> ByteSpan {
    ByteSpan {
        data: bytes.as_ptr(),
        size: bytes.len(),
    }
}

/// `NetworkTextMessage(action, colour, self_send, name, str, data)`; pass `b""` and
/// `StringParameterData::Empty` for the original's defaults of `str` and `data`.
pub(crate) fn network_text_message(
    action: NetworkAction,
    colour: TextColour,
    self_send: bool,
    name: &[u8],
    str: &[u8],
    data: StringParameterData<'_>,
) {
    let data = descriptor(data);
    (crate::news_func::services().network_text_message)(
        action.0,
        colour.0,
        self_send,
        span(name),
        span(str),
        &raw const data,
    );
}

#[cfg(test)]
mod tests {
    use crate::console_type::CC_DEFAULT;
    use crate::network_type::{NetworkAction, NetworkErrorCode};
    use crate::news_func::testing::{Effect, Parameter};
    use crate::news_func::testing::{encoded, error_message, on_error_message, take};
    use crate::openttd::SwitchMode;
    use crate::strings_type::{StringID, StringParameterData};
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

    /// Every byte code maps to the original's table entry below `NETWORK_ERROR_END` and to
    /// the general error from there up.
    #[test]
    fn get_network_error_msg_maps_every_code() {
        use crate::table::strings as t;
        // `network.cpp`'s `network_error_strings`, in the original's order.
        let table = [
            t::STR_NETWORK_ERROR_CLIENT_GENERAL,
            t::STR_NETWORK_ERROR_CLIENT_DESYNC,
            t::STR_NETWORK_ERROR_CLIENT_SAVEGAME,
            t::STR_NETWORK_ERROR_CLIENT_CONNECTION_LOST,
            t::STR_NETWORK_ERROR_CLIENT_PROTOCOL_ERROR,
            t::STR_NETWORK_ERROR_CLIENT_NEWGRF_MISMATCH,
            t::STR_NETWORK_ERROR_CLIENT_NOT_AUTHORIZED,
            t::STR_NETWORK_ERROR_CLIENT_NOT_EXPECTED,
            t::STR_NETWORK_ERROR_CLIENT_WRONG_REVISION,
            t::STR_NETWORK_ERROR_CLIENT_NAME_IN_USE,
            t::STR_NETWORK_ERROR_CLIENT_WRONG_PASSWORD,
            t::STR_NETWORK_ERROR_CLIENT_COMPANY_MISMATCH,
            t::STR_NETWORK_ERROR_CLIENT_KICKED,
            t::STR_NETWORK_ERROR_CLIENT_CHEATER,
            t::STR_NETWORK_ERROR_CLIENT_SERVER_FULL,
            t::STR_NETWORK_ERROR_CLIENT_TOO_MANY_COMMANDS,
            t::STR_NETWORK_ERROR_CLIENT_TIMEOUT_PASSWORD,
            t::STR_NETWORK_ERROR_CLIENT_TIMEOUT_COMPUTER,
            t::STR_NETWORK_ERROR_CLIENT_TIMEOUT_MAP,
            t::STR_NETWORK_ERROR_CLIENT_TIMEOUT_JOIN,
            t::STR_NETWORK_ERROR_CLIENT_INVALID_CLIENT_NAME,
            t::STR_NETWORK_ERROR_CLIENT_NOT_ON_ALLOW_LIST,
            t::STR_NETWORK_ERROR_CLIENT_NO_AUTHENTICATION_METHOD_AVAILABLE,
        ];
        for code in 0..=u8::MAX {
            let expected: StringID = table
                .get(usize::from(code))
                .copied()
                .unwrap_or(t::STR_NETWORK_ERROR_CLIENT_GENERAL);
            assert_eq!(
                super::get_network_error_msg(NetworkErrorCode(code)),
                expected,
                "code {code}"
            );
        }
    }

    /// Each of the six arguments reaches the native call unchanged and in its position, for
    /// each alternative of the string parameter.
    #[test]
    fn network_text_message_passes_every_argument() {
        take();
        super::network_text_message(
            NetworkAction::NETWORK_ACTION_JOIN,
            CC_DEFAULT,
            false,
            b"n\xE4me",
            b"",
            StringParameterData::Empty,
        );
        super::network_text_message(
            NetworkAction::NETWORK_ACTION_KICKED,
            crate::gfx_type::TextColour(0x0D),
            true,
            b"",
            b"reason\x00x",
            StringParameterData::Integer(u64::MAX),
        );
        super::network_text_message(
            NetworkAction::NETWORK_ACTION_LEAVE,
            CC_DEFAULT,
            false,
            b"a",
            b"b",
            StringParameterData::String(b"s\xFF"),
        );
        let text = |action: NetworkAction, colour, self_send, name: &[u8], str: &[u8], data| {
            Effect::NetworkTextMessage {
                action: action.0,
                colour,
                self_send,
                name: name.to_vec(),
                str: str.to_vec(),
                data,
            }
        };
        assert_eq!(
            take(),
            [
                text(
                    NetworkAction::NETWORK_ACTION_JOIN,
                    0x01,
                    false,
                    b"n\xE4me",
                    b"",
                    Parameter::Empty
                ),
                text(
                    NetworkAction::NETWORK_ACTION_KICKED,
                    0x0D,
                    true,
                    b"",
                    b"reason\x00x",
                    Parameter::Integer(u64::MAX)
                ),
                text(
                    NetworkAction::NETWORK_ACTION_LEAVE,
                    0x01,
                    false,
                    b"a",
                    b"b",
                    Parameter::String(b"s\xFF".to_vec())
                ),
            ]
        );
    }
}
