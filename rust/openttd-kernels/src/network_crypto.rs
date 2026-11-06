/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Authentication method and result values of `network/network_crypto.h`.
pub(crate) const METHOD_PAKE: u8 = 1;
pub(crate) const METHOD_AUTHORIZED_KEY: u8 = 2;
pub(crate) const REQUEST_AWAIT_INPUT: u8 = 0;
pub(crate) const REQUEST_READY: u8 = 1;
pub(crate) const RESPONSE_AUTHENTICATED: u8 = 0;
pub(crate) const RESPONSE_RETRY: u8 = 2;

/// Native authentication shell, with virtual Rust-owner metadata.
pub(crate) struct NetworkAuthenticationHandler {
    _private: [u8; 0],
}
