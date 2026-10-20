// Monocypher version 4.0.2
//
// This file is dual-licensed.  Choose whichever licence you want from
// the two licences listed below.
//
// The first licence is a regular 2-clause BSD licence.  The second licence
// is the CC-0 from Creative Commons. It is intended to release Monocypher
// to the public domain.  The BSD licence serves as a fallback option.
//
// SPDX-License-Identifier: BSD-2-Clause OR CC0-1.0
//
// ------------------------------------------------------------------------
//
// Copyright (c) 2017-2020, Loup Vaillant
// All rights reserved.
//
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are
// met:
//
// 1. Redistributions of source code must retain the above copyright
//    notice, this list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright
//    notice, this list of conditions and the following disclaimer in the
//    documentation and/or other materials provided with the
//    distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
// HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
// LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
// DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
// THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
// (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
//
// ------------------------------------------------------------------------
//
// Written in 2017-2020 by Loup Vaillant
//
// To the extent possible under law, the author(s) have dedicated all copyright
// and related neighboring rights to this software to the public domain
// worldwide.  This software is distributed without any warranty.
//
// You should have received a copy of the CC0 Public Domain Dedication along
// with this software.  If not, see
// <https://creativecommons.org/publicdomain/zero/1.0/>

//! Monocypher 4.0.2 `BLAKE2b`. Borrowed caller-owned contexts use actual C++
//! field offsets; raw operations preserve initialization and overlapping hashes.
#![allow(clippy::cast_possible_truncation, clippy::too_many_arguments)]
// Narrowing matches little-endian serialization; arity preserves the vendor ABI.
use crate::crypto_primitives::Leaves;
use std::ffi::c_void;
use std::ptr;

#[repr(C)]
pub(crate) struct Layout {
    size: usize,
    alignment: usize,
    hash: usize,
    input_offset: usize,
    input: usize,
    input_idx: usize,
    hash_size: usize,
}
const IV: [u64; 8] = [
    0x6a09_e667_f3bc_c908,
    0xbb67_ae85_84ca_a73b,
    0x3c6e_f372_fe94_f82b,
    0xa54f_f53a_5f1d_36f1,
    0x510e_527f_ade6_82d1,
    0x9b05_688c_2b3e_6c1f,
    0x1f83_d9ab_fb41_bd6b,
    0x5be0_cd19_137e_2179,
];
const SIGMA: [[usize; 16]; 12] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
];
fn read64(ctx: *mut u8, offset: usize, index: usize) -> u64 {
    unsafe { ptr::read_unaligned(ctx.add(offset + index * 8).cast()) }
}
fn write64(ctx: *mut u8, offset: usize, index: usize, value: u64) {
    unsafe {
        ptr::write_unaligned(ctx.add(offset + index * 8).cast(), value);
    }
}
fn read_size(ctx: *mut u8, offset: usize) -> usize {
    unsafe { ptr::read_unaligned(ctx.add(offset).cast()) }
}
fn write_size(ctx: *mut u8, offset: usize, value: usize) {
    unsafe {
        ptr::write_unaligned(ctx.add(offset).cast(), value);
    }
}
fn load64(bytes: *const u8) -> u64 {
    let mut value = 0;
    for i in 0..8 {
        value |= u64::from(unsafe { bytes.add(i).read() }) << (8 * i);
    }
    value
}
fn words(layout: &Layout, ctx: *mut u8, start: usize, message: *const u8, count: usize) {
    for i in 0..count {
        write64(
            ctx,
            layout.input,
            start + i,
            load64(unsafe { message.add(i * 8) }),
        );
    }
}
// Indices use the original G-function notation.
#[allow(clippy::many_single_char_names)]
fn mix(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, x: u64, y: u64) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
    v[d] = (v[d] ^ v[a]).rotate_right(32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(24);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(63);
}
fn compress(layout: &Layout, ctx: *mut u8, last: bool) {
    let count = read_size(ctx, layout.input_idx) as u64;
    let low = read64(ctx, layout.input_offset, 0).wrapping_add(count);
    write64(ctx, layout.input_offset, 0, low);
    if low < count {
        write64(
            ctx,
            layout.input_offset,
            1,
            read64(ctx, layout.input_offset, 1).wrapping_add(1),
        );
    }
    // Work vector matches the original local scalar copies. No extra input copy.
    let mut v = [0; 16];
    for i in 0..8 {
        v[i] = read64(ctx, layout.hash, i);
        v[i + 8] = IV[i];
    }
    v[12] ^= low;
    v[13] ^= read64(ctx, layout.input_offset, 1);
    v[14] ^= if last { u64::MAX } else { 0 };
    for sigma in SIGMA {
        for (a, b, c, d, x, y) in [
            (0, 4, 8, 12, 0, 1),
            (1, 5, 9, 13, 2, 3),
            (2, 6, 10, 14, 4, 5),
            (3, 7, 11, 15, 6, 7),
            (0, 5, 10, 15, 8, 9),
            (1, 6, 11, 12, 10, 11),
            (2, 7, 8, 13, 12, 13),
            (3, 4, 9, 14, 14, 15),
        ] {
            mix(
                &mut v,
                a,
                b,
                c,
                d,
                read64(ctx, layout.input, sigma[x]),
                read64(ctx, layout.input, sigma[y]),
            );
        }
    }
    for i in 0..8 {
        write64(
            ctx,
            layout.hash,
            i,
            read64(ctx, layout.hash, i) ^ v[i] ^ v[i + 8],
        );
    }
    // Original compression does not explicitly wipe its local work vector.
}
fn init(layout: &Layout, ctx: *mut u8, hash_size: usize, key: *const u8, key_size: usize) {
    for (i, value) in IV.into_iter().enumerate() {
        write64(ctx, layout.hash, i, value);
    }
    // size_t shift happens before conversion in the original (also on i686).
    let parameter = 0x0101_0000_usize ^ key_size.wrapping_shl(8) ^ hash_size;
    write64(
        ctx,
        layout.hash,
        0,
        read64(ctx, layout.hash, 0) ^ parameter as u64,
    );
    write64(ctx, layout.input_offset, 0, 0);
    write64(ctx, layout.input_offset, 1, 0);
    write_size(ctx, layout.hash_size, hash_size);
    write_size(ctx, layout.input_idx, 0);
    for i in 0..16 {
        write64(ctx, layout.input, i, 0);
    }
    if key_size != 0 {
        let mut key_block = [0_u8; 128];
        // Defined source extent is <=128, though the documented bound is <=64.
        unsafe {
            ptr::copy_nonoverlapping(key, key_block.as_mut_ptr(), key_size);
        }
        words(layout, ctx, 0, key_block.as_ptr(), 16);
        write_size(ctx, layout.input_idx, 128);
        // Original keyed-init does not explicitly wipe this temporary.
    }
}
fn gap(index: usize, alignment: usize) -> usize {
    index.wrapping_neg() & (alignment - 1)
}
fn update(layout: &Layout, ctx: *mut u8, mut message: *const u8, mut size: usize) {
    if size == 0 {
        return;
    }
    let mut index = read_size(ctx, layout.input_idx);
    if index & 7 != 0 {
        let count = gap(index, 8).min(size);
        let word = index >> 3;
        let byte = index & 7;
        for i in 0..count {
            let value = u64::from(unsafe { message.add(i).read() }) << ((byte + i) * 8);
            write64(
                ctx,
                layout.input,
                word,
                read64(ctx, layout.input, word) | value,
            );
        }
        index += count;
        write_size(ctx, layout.input_idx, index);
        message = unsafe { message.add(count) };
        size -= count;
    }
    if index & 127 != 0 {
        let count = gap(index, 128).min(size) >> 3;
        words(layout, ctx, index >> 3, message, count);
        index += count * 8;
        write_size(ctx, layout.input_idx, index);
        message = unsafe { message.add(count * 8) };
        size -= count * 8;
    }
    for _ in 0..(size >> 7) {
        if index == 128 {
            compress(layout, ctx, false);
        }
        words(layout, ctx, 0, message, 16);
        message = unsafe { message.add(128) };
        index = 128;
        write_size(ctx, layout.input_idx, index);
    }
    size &= 127;
    if size != 0 {
        if index == 128 {
            compress(layout, ctx, false);
            index = 0;
            write_size(ctx, layout.input_idx, index);
        }
        if index == 0 {
            for i in 0..16 {
                write64(ctx, layout.input, i, 0);
            }
        }
        let count = size >> 3;
        words(layout, ctx, 0, message, count);
        index += count * 8;
        write_size(ctx, layout.input_idx, index);
        message = unsafe { message.add(count * 8) };
        size -= count * 8;
        for i in 0..size {
            let word = index >> 3;
            let byte = index & 7;
            let value = u64::from(unsafe { message.add(i).read() }) << (byte * 8);
            write64(
                ctx,
                layout.input,
                word,
                read64(ctx, layout.input, word) | value,
            );
            index += 1;
            write_size(ctx, layout.input_idx, index);
        }
    }
}
fn finish(leaves: &Leaves, layout: &Layout, ctx: *mut u8, hash: *mut u8) {
    compress(layout, ctx, true);
    let size = read_size(ctx, layout.hash_size).min(64);
    for i in 0..size {
        unsafe {
            hash.add(i)
                .write((read64(ctx, layout.hash, i >> 3) >> ((i & 7) * 8)) as u8);
        }
    }
    unsafe {
        (leaves.wipe)(ctx.cast(), layout.size);
    }
}
// SAFETY for exports: actual caller context lifetime is live, layout/leaf tables
// are initialized immutable synchronous borrows; buffers meet original extents
// (<=isize::MAX), key <=128, context distinct from inputs/output. One-shot output
// may overlap message/key because all input is consumed before final writes.
// No raw pointer survives a call; no external buffer becomes a Rust reference.
// Empty updates return before any context/layout/pointer access. Panics abort.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_blake2b_keyed_init(
    layout: *const Layout,
    ctx: *mut c_void,
    size: usize,
    key: *const u8,
    key_size: usize,
) {
    init(unsafe { &*layout }, ctx.cast(), size, key, key_size);
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_blake2b_init(
    layout: *const Layout,
    ctx: *mut c_void,
    size: usize,
) {
    init(unsafe { &*layout }, ctx.cast(), size, ptr::null(), 0);
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_blake2b_update(
    layout: *const Layout,
    ctx: *mut c_void,
    message: *const u8,
    size: usize,
) {
    if size == 0 {
        return;
    }
    update(unsafe { &*layout }, ctx.cast(), message, size);
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_blake2b_final(
    leaves: *const Leaves,
    layout: *const Layout,
    ctx: *mut c_void,
    hash: *mut u8,
) {
    finish(unsafe { &*leaves }, unsafe { &*layout }, ctx.cast(), hash);
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_blake2b_keyed(
    leaves: *const Leaves,
    layout: *const Layout,
    ctx: *mut c_void,
    hash: *mut u8,
    size: usize,
    key: *const u8,
    key_size: usize,
    message: *const u8,
    message_size: usize,
) {
    let layout = unsafe { &*layout };
    init(layout, ctx.cast(), size, key, key_size);
    update(layout, ctx.cast(), message, message_size);
    finish(unsafe { &*leaves }, layout, ctx.cast(), hash);
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_blake2b(
    leaves: *const Leaves,
    layout: *const Layout,
    ctx: *mut c_void,
    hash: *mut u8,
    size: usize,
    message: *const u8,
    message_size: usize,
) {
    unsafe {
        openttd_rust_blake2b_keyed(
            leaves,
            layout,
            ctx,
            hash,
            size,
            ptr::null(),
            0,
            message,
            message_size,
        );
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDBlake2bLayout" ["probe"] => Layout {
        size,
        alignment,
        hash,
        input_offset,
        input,
        input_idx,
        hash_size,
    };
}
