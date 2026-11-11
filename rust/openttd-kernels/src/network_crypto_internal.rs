/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Key-exchange vocabulary from `network/network_crypto_internal.h`.

/// Number of bytes in X25519 public and secret keys.
pub(crate) const X25519_KEY_SIZE: usize = 32;
/// Number of bytes in X25519 nonces.
pub(crate) const X25519_NONCE_SIZE: usize = 24;
/// Number of bytes in X25519 message authentication codes.
pub(crate) const X25519_MAC_SIZE: usize = 16;
/// Number of random payload bytes in the authentication message.
pub(crate) const X25519_KEY_EXCHANGE_MESSAGE_SIZE: usize = 8;

/// Nonce storage projected into the existing session owner. The owner preserves
/// `X25519Nonce`'s destruction-time crypto wipe, just as it does for the key projections.
pub(crate) type X25519Nonce = [u8; X25519_NONCE_SIZE];
/// Container for an X25519 message authentication code.
pub(crate) type X25519Mac = [u8; X25519_MAC_SIZE];
/// Container for an X25519 key-exchange message.
pub(crate) type X25519KeyExchangeMessage = [u8; X25519_KEY_EXCHANGE_MESSAGE_SIZE];

/// `X25519Nonce::CreateRandom`: fill a nonce with random bytes. Initialize the
/// existing owner field in place, retaining its generation and wipe lifetime.
pub(crate) fn create_random_nonce(
    nonce: &mut X25519Nonce,
    random_bytes: impl FnOnce(&mut X25519Nonce),
) {
    random_bytes(nonce);
}

/// `X25519SecretKey::CreateRandom`: fill the existing secret-key owner with random
/// bytes without changing its generation or destruction-time wipe lifetime.
pub(crate) fn create_random_secret_key(
    key: &mut X25519SecretKey,
    random_bytes: impl FnOnce(&mut X25519SecretKey),
) {
    random_bytes(key);
}

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

/// `X25519DerivedKeys::ClientToServer`: raw key for client-to-server traffic.
pub(crate) fn client_to_server(keys: &X25519DerivedKeys) -> &X25519Key {
    keys[..X25519_KEY_SIZE].try_into().unwrap()
}

/// `X25519DerivedKeys::ServerToClient`: raw key for server-to-client traffic.
pub(crate) fn server_to_client(keys: &X25519DerivedKeys) -> &X25519Key {
    keys[X25519_KEY_SIZE..].try_into().unwrap()
}
