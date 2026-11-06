/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Native mutation of Rust-owned name and encoded-string bytes. Views expire on
//! assignment; accesses are serialized by the game-state mutex.
#![allow(unsafe_code)]
use crate::industrytype::NativeVec;
use crate::services::ByteSpan;

/// Replace the bytes, permitting the input to alias the existing allocation.
///
/// # Safety
/// `bytes` is live Rust-owned storage; accesses hold the game-state mutex or
/// serialized startup/load. `text` is readable for its length, may alias the
/// destination and may be null only when empty. Assignment invalidates old views.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_owned_bytes_assign(
    bytes: *mut NativeVec<u8>,
    text: ByteSpan,
) {
    let value = if text.size == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(text.data, text.size).to_vec() }
    };
    unsafe { *bytes = NativeVec::from_vec(value) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignment_preserves_bytes_and_copies_aliased_sources_before_drop() {
        let mut bytes = NativeVec::from_vec(vec![0xff, 0, 0x80, b'A']);
        for (offset, count, expected) in [
            (0, 4, &[0xff, 0, 0x80, b'A'][..]),
            (1, 2, &[0, 0x80][..]),
            (0, 0, &[][..]),
        ] {
            let source = unsafe { bytes.as_mut_ptr().add(offset) };
            unsafe {
                openttd_rust_owned_bytes_assign(
                    &raw mut bytes,
                    ByteSpan {
                        data: source,
                        size: count,
                    },
                );
            }
            assert_eq!(bytes.as_slice(), expected);
        }
        unsafe {
            openttd_rust_owned_bytes_assign(
                &raw mut bytes,
                ByteSpan {
                    data: std::ptr::null(),
                    size: 0,
                },
            );
        };
        assert!(bytes.is_empty());
    }
}
