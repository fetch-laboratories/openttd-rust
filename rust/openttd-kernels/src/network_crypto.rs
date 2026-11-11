/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Authentication/key-exchange vocabulary from `network/network_crypto.h`.
//! The server selects available key-exchange-only, password-authenticated (PAKE)
//! or authorized-key methods. Password requests use an asynchronous user callback;
//! the other methods can respond immediately. Failed authorized keys may fall back
//! to the password method. After authentication, the server can signal upgrading
//! the connection to encryption with the shared secret.
/// Native authentication shell, with virtual Rust-owner metadata.
pub(crate) struct NetworkAuthenticationHandler {
    _private: [u8; 0],
}

/// The authentication method from `network_crypto.h`, retaining every byte.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct NetworkAuthenticationMethod(pub u8);

#[allow(
    non_upper_case_globals,
    reason = "original NetworkAuthenticationMethod enumerator vocabulary"
)]
impl NetworkAuthenticationMethod {
    /// No authentication, only X25519 key exchange; unsupported for admin connections.
    pub(crate) const X25519_KeyExchangeOnly: Self = Self(0);
    /// Authentication using X25519 password-authenticated key agreement.
    pub(crate) const X25519_PAKE: Self = Self(1);
    /// Authentication using X25519 key exchange and authorized keys.
    pub(crate) const X25519_AuthorizedKey: Self = Self(2);
    /// Must ALWAYS be at the end of the list.
    pub(crate) const End: Self = Self(3);
}

/// Original `EnumBitSet<NetworkAuthenticationMethod, uint16_t>`.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct NetworkAuthenticationMethodMask(pub u16);

impl NetworkAuthenticationMethodMask {
    /// `EnumBitSet::Test` for an authentication method.
    pub(crate) fn test(self, method: NetworkAuthenticationMethod) -> bool {
        crate::bitmath_func::has_bit(self.0, method.0)
    }
}

/// `NetworkAuthenticationClientHandler::RequestResult`, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct RequestResult(pub u8);

#[allow(
    non_upper_case_globals,
    reason = "original NetworkAuthenticationClientHandler::RequestResult enumerator vocabulary"
)]
impl RequestResult {
    /// We requested user input and must wait for it.
    pub(crate) const AwaitUserInput: Self = Self(0);
    /// No user input is needed; respond to the server immediately.
    pub(crate) const ReadyForResponse: Self = Self(1);
    /// The received request is invalid.
    pub(crate) const Invalid: Self = Self(2);
}

/// `NetworkAuthenticationServerHandler::ResponseResult`, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResponseResult(pub u8);

#[allow(
    non_upper_case_globals,
    reason = "original NetworkAuthenticationServerHandler::ResponseResult enumerator vocabulary"
)]
impl ResponseResult {
    /// The client authenticated successfully.
    pub(crate) const Authenticated: Self = Self(0);
    /// All authentications for this handler are exhausted.
    pub(crate) const NotAuthenticated: Self = Self(1);
    /// The client failed, but another method remains to try.
    pub(crate) const RetryNextMethod: Self = Self(2);
}
