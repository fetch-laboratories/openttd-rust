/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `OpenTTD`'s Monocypher constructions over maintained primitive crates.
//! The incremental AEAD keeps its nonce fixed and replaces its key after each
//! successful message with the upper half of `ChaCha20` block zero.

use blake2::{Blake2b128, Blake2b256, Blake2b512, Digest};
use chacha20::{
    ChaCha20Legacy, R20,
    cipher::{KeyIvInit, StreamCipher},
    hchacha,
};
use curve25519_dalek::{
    edwards::{CompressedEdwardsY, EdwardsPoint},
    montgomery::MontgomeryPoint,
    scalar::Scalar,
    traits::IsIdentity,
};
use poly1305::{
    Poly1305,
    universal_hash::{KeyInit, UniversalHash},
};
use sha2::Sha512;
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, Zeroizing};

/// `crypto_blake2b_ctx`, for the digest sizes `OpenTTD` uses.
pub(crate) enum Hash {
    Short(Blake2b128),
    Medium(Blake2b256),
    Long(Blake2b512),
}

impl Hash {
    /// `crypto_blake2b_init(&ctx, size)`.
    pub(crate) fn new(size: usize) -> Self {
        match size {
            16 => Self::Short(Blake2b128::new()),
            32 => Self::Medium(Blake2b256::new()),
            64 => Self::Long(Blake2b512::new()),
            // OpenTTD hashes only 16-, 32- and 64-byte digests.
            _ => {
                crate::services_assert::check(false);
                std::process::abort()
            }
        }
    }

    /// `crypto_blake2b_update`.
    pub(crate) fn update(&mut self, bytes: &[u8]) {
        match self {
            Self::Short(hash) => hash.update(bytes),
            Self::Medium(hash) => hash.update(bytes),
            Self::Long(hash) => hash.update(bytes),
        }
    }

    /// `crypto_blake2b_final`.
    pub(crate) fn finish(self, out: &mut [u8]) {
        match self {
            Self::Short(hash) => out.copy_from_slice(&hash.finalize()),
            Self::Medium(hash) => out.copy_from_slice(&hash.finalize()),
            Self::Long(hash) => out.copy_from_slice(&hash.finalize()),
        }
    }
}

/// `crypto_x25519_public_key`.
pub(crate) fn public_key(secret: &[u8; 32]) -> [u8; 32] {
    MontgomeryPoint::mul_base_clamped(*secret).to_bytes()
}

/// `crypto_x25519`.
pub(crate) fn x25519(secret: &[u8; 32], peer: &[u8; 32]) -> [u8; 32] {
    MontgomeryPoint(*peer).mul_clamped(*secret).to_bytes()
}

/// `X25519DerivedKeys::Exchange`: perform the actual key exchange. `server` is
/// whether we are the server; it orders the public keys in the hash. Returns
/// false when an illegal public key was given.
pub(crate) fn derive(
    keys: &mut [u8; 64],
    secret: &[u8; 32],
    public: &[u8; 32],
    peer: &[u8; 32],
    server: bool,
    extra: &[u8],
) -> bool {
    let shared = Zeroizing::new(x25519(secret, peer));
    if bool::from(shared.ct_eq(&[0; 32])) {
        /* A shared secret of all zeros means that the peer tried to force the shared secret to a known constant. */
        return false;
    }
    let mut hash = Hash::new(keys.len());
    hash.update(shared.as_ref());
    let (server, client) = if server {
        (public, peer)
    } else {
        (peer, public)
    };
    hash.update(server);
    hash.update(client);
    hash.update(extra);
    hash.finish(keys);
    true
}

/// `crypto_aead_ctx`.
#[derive(Clone)]
pub(crate) struct Aead {
    key: [u8; 32],
    nonce: [u8; 8],
}

impl Drop for Aead {
    fn drop(&mut self) {
        self.key.zeroize();
        self.nonce.zeroize();
    }
}

impl Aead {
    /// `crypto_aead_init_x`.
    pub(crate) fn new(key: &[u8; 32], nonce: &[u8; 24]) -> Self {
        let mut subkey = hchacha::<R20>(key.into(), nonce[..16].try_into().unwrap());
        let result = Self {
            key: subkey.as_slice().try_into().unwrap(),
            nonce: nonce[16..].try_into().unwrap(),
        };
        subkey.zeroize();
        result
    }

    fn cipher(&self) -> (ChaCha20Legacy, Zeroizing<[u8; 64]>) {
        let mut cipher = ChaCha20Legacy::new((&self.key).into(), (&self.nonce).into());
        let mut block = Zeroizing::new([0; 64]);
        cipher.apply_keystream(block.as_mut());
        (cipher, block)
    }

    fn tag(key: &[u8], ad: &[u8], text: &[u8]) -> [u8; 16] {
        let mut poly = Poly1305::new(key.try_into().unwrap());
        poly.update_padded(ad);
        poly.update_padded(text);
        let mut sizes = [0; 16];
        sizes[..8].copy_from_slice(&(ad.len() as u64).to_le_bytes());
        sizes[8..].copy_from_slice(&(text.len() as u64).to_le_bytes());
        poly.update(&[sizes.into()]);
        poly.finalize().into()
    }

    /// `crypto_aead_write`, in place.
    pub(crate) fn write(&mut self, text: &mut [u8], ad: &[u8]) -> [u8; 16] {
        let (mut cipher, block) = self.cipher();
        cipher.apply_keystream(text);
        let tag = Self::tag(&block[..32], ad, text);
        self.key.copy_from_slice(&block[32..]);
        tag
    }

    /// `crypto_aead_read`, in place: false if the tag does not match.
    pub(crate) fn read(&mut self, text: &mut [u8], tag: &[u8; 16], ad: &[u8]) -> bool {
        let (mut cipher, block) = self.cipher();
        let mut expected = Self::tag(&block[..32], ad, text);
        let valid = bool::from(expected.ct_eq(tag));
        expected.zeroize();
        if valid {
            cipher.apply_keystream(text);
            self.key.copy_from_slice(&block[32..]);
        }
        valid
    }
}

/// `crypto_ed25519_check`: whether `signature` signs `message` for `public`.
pub(crate) fn ed25519_check(signature: &[u8; 64], public: &[u8; 32], message: &[u8]) -> bool {
    let Some(a) = CompressedEdwardsY(*public).decompress() else {
        return false;
    };
    let Some(r) = CompressedEdwardsY(signature[..32].try_into().unwrap()).decompress() else {
        return false;
    };
    let Some(s) = Option::<Scalar>::from(Scalar::from_canonical_bytes(
        signature[32..].try_into().unwrap(),
    )) else {
        return false;
    };
    let mut hash = Sha512::new();
    hash.update(&signature[..32]);
    hash.update(public);
    hash.update(message);
    let h = Scalar::from_bytes_mod_order_wide(&hash.finalize().into());
    // Monocypher allows noncanonical point encodings and cofactored equations.
    (EdwardsPoint::vartime_double_scalar_mul_basepoint(&(-h), &a, &s) - r)
        .mul_by_cofactor()
        .is_identity()
}

#[cfg(test)]
mod tests;

mod ffi;

/// `GenerateUid`'s digest: hash the random bytes, then the coding string.
pub(crate) fn generate_uid_hash(out: &mut [u8; 16], random: &[u8; 32], text: &[u8]) {
    let mut hash = Hash::new(out.len());
    hash.update(random);
    hash.update(text);
    hash.finish(out);
}

/// `CalculateHashV1`: read through `feof`/`fread`, preserving the EOF test,
/// read and hash-update order, including a final zero-byte read.
pub(crate) fn calculate_hash_v1(
    out: &mut [u8; 32],
    mut eof: impl FnMut() -> bool,
    mut read: impl FnMut(&mut [u8]) -> usize,
) {
    let mut hash = Hash::new(out.len());
    while !eof() {
        let mut buf = [0; 1024];
        let len = read(&mut buf);
        hash.update(&buf[..len]);
    }
    hash.finish(out);
}
