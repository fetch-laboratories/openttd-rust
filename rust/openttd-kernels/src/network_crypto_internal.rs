/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Key-exchange vocabulary from `network/network_crypto_internal.h`.

/// Number of bytes in X25519 public and secret keys.
pub(crate) const X25519_KEY_SIZE: usize = 32;
/// The borrowed key buffers keep their owner's existing wipe-on-destruction policy.
pub(crate) type X25519Key = [u8; X25519_KEY_SIZE];
/// Container for an X25519 public key.
pub(crate) type X25519PublicKey = X25519Key;
/// Container for an X25519 secret key.
pub(crate) type X25519SecretKey = X25519Key;
/// Contiguous buffer holding both communication directions' derived keys.
pub(crate) type X25519DerivedKeys = [u8; X25519_KEY_SIZE + X25519_KEY_SIZE];

/// The side of the key exchange, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct X25519KeyExchangeSide(pub u8);
impl X25519KeyExchangeSide {
    /// We are the client.
    pub(crate) const CLIENT: Self = Self(0);
    /// We are the server.
    pub(crate) const SERVER: Self = Self(1);
}
