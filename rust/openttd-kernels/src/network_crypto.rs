/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Authentication method and result values of `network/network_crypto.h`.
pub(crate) const METHOD_PAKE: u8 = NetworkAuthenticationMethod::X25519_PAKE.0;
pub(crate) const METHOD_AUTHORIZED_KEY: u8 = NetworkAuthenticationMethod::X25519_AuthorizedKey.0;
pub(crate) const REQUEST_AWAIT_INPUT: u8 = 0;
pub(crate) const REQUEST_READY: u8 = 1;
pub(crate) const RESPONSE_AUTHENTICATED: u8 = 0;
pub(crate) const RESPONSE_RETRY: u8 = 2;

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
