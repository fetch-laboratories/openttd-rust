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

//! Monocypher 4.0.2 cipher/MAC composition. Original C++ owns context lifetimes.
//! All external memory uses raw access: no initialized whole-context copy or
//! overlapping input/output references. Fixed arithmetic has no secret branches.
#![allow(
    clippy::cast_possible_truncation,
    clippy::too_many_arguments,
    clippy::similar_names
)]
// Narrowing is the original modulo-2^32/8 conversion. ABI arity follows public C.
use std::ffi::c_void;
use std::mem::{MaybeUninit, align_of, offset_of, size_of};
use std::num::Wrapping as W;
use std::ptr;

#[repr(C)]
pub(crate) struct Leaves {
    pub(crate) wipe: unsafe extern "C" fn(*mut c_void, usize),
    verify16: unsafe extern "C" fn(*const u8, *const u8) -> i32,
}
#[repr(C)]
pub(crate) struct PolyLayout {
    size: usize,
    alignment: usize,
    c: usize,
    c_idx: usize,
    r: usize,
    pad: usize,
    h: usize,
}
#[repr(C)]
pub(crate) struct AeadLayout {
    size: usize,
    alignment: usize,
    counter: usize,
    key: usize,
    nonce: usize,
}
// Only internal scratch. It is never read as an initialized aggregate or copied.
#[repr(C)]
struct PolyScratch {
    c: [u8; 16],
    c_idx: usize,
    r: [u32; 4],
    pad: [u32; 4],
    h: [u32; 5],
}
const SCRATCH_LAYOUT: PolyLayout = PolyLayout {
    size: size_of::<PolyScratch>(),
    alignment: align_of::<PolyScratch>(),
    c: offset_of!(PolyScratch, c),
    c_idx: offset_of!(PolyScratch, c_idx),
    r: offset_of!(PolyScratch, r),
    pad: offset_of!(PolyScratch, pad),
    h: offset_of!(PolyScratch, h),
};
static ZERO: [u8; 128] = [0; 128];
fn load32(p: *const u8) -> u32 {
    unsafe {
        u32::from(*p)
            | (u32::from(*p.add(1)) << 8)
            | (u32::from(*p.add(2)) << 16)
            | (u32::from(*p.add(3)) << 24)
    }
}
fn store32(p: *mut u8, x: u32) {
    unsafe {
        for i in 0..4 {
            p.add(i).write((x >> (i * 8)) as u8);
        }
    }
}
fn store64(p: *mut u8, x: u64) {
    store32(p, x as u32);
    store32(unsafe { p.add(4) }, (x >> 32) as u32);
}
fn wipe<T>(ops: &Leaves, p: *mut T, size: usize) {
    unsafe {
        (ops.wipe)(p.cast(), size);
    }
}
fn quarter(b: &mut [u32; 16], a: usize, c1: usize, c: usize, d: usize) {
    b[a] = b[a].wrapping_add(b[c1]);
    b[d] = (b[d] ^ b[a]).rotate_left(16);
    b[c] = b[c].wrapping_add(b[d]);
    b[c1] = (b[c1] ^ b[c]).rotate_left(12);
    b[a] = b[a].wrapping_add(b[c1]);
    b[d] = (b[d] ^ b[a]).rotate_left(8);
    b[c] = b[c].wrapping_add(b[d]);
    b[c1] = (b[c1] ^ b[c]).rotate_left(7);
}
fn rounds(b: &mut [u32; 16]) {
    for _ in 0..10 {
        quarter(b, 0, 4, 8, 12);
        quarter(b, 1, 5, 9, 13);
        quarter(b, 2, 6, 10, 14);
        quarter(b, 3, 7, 11, 15);
        quarter(b, 0, 5, 10, 15);
        quarter(b, 1, 6, 11, 12);
        quarter(b, 2, 7, 8, 13);
        quarter(b, 3, 4, 9, 14);
    }
}
fn initial(b: &mut [u32; 16], key: *const u8) {
    for (i, word) in b.iter_mut().enumerate().take(4) {
        *word = load32(unsafe { b"expand 32-byte k".as_ptr().add(i * 4) });
    }
    for i in 0..8 {
        b[i + 4] = load32(unsafe { key.add(i * 4) });
    }
}
fn h(ops: &Leaves, out: *mut u8, key: *const u8, nonce: *const u8) {
    let mut b = [0_u32; 16];
    initial(&mut b, key);
    for i in 0..4 {
        b[i + 12] = load32(unsafe { nonce.add(i * 4) });
    }
    rounds(&mut b);
    for i in 0..4 {
        store32(unsafe { out.add(i * 4) }, b[i]);
        store32(unsafe { out.add(16 + i * 4) }, b[i + 12]);
    }
    wipe(ops, b.as_mut_ptr(), size_of_val(&b));
}
fn djb(
    ops: &Leaves,
    mut out: *mut u8,
    mut input: *const u8,
    size: usize,
    key: *const u8,
    nonce: *const u8,
    counter: u64,
) -> u64 {
    // All key/nonce bytes are loaded before any output, including Elligator alias.
    let mut state = [0_u32; 16];
    initial(&mut state, key);
    state[14] = load32(nonce);
    state[15] = load32(unsafe { nonce.add(4) });
    state[12] = counter as u32;
    state[13] = (counter >> 32) as u32;
    let mut pool = [0_u32; 16];
    for _ in 0..(size >> 6) {
        pool.copy_from_slice(&state);
        rounds(&mut pool);
        for j in 0..16 {
            let p = pool[j].wrapping_add(state[j]);
            let plain = if input.is_null() { 0 } else { load32(input) };
            store32(out, p ^ plain);
            unsafe {
                out = out.add(4);
                if !input.is_null() {
                    input = input.add(4);
                }
            }
        }
        state[12] = state[12].wrapping_add(1);
        if state[12] == 0 {
            state[13] = state[13].wrapping_add(1);
        }
    }
    let tail = size & 63;
    if tail > 0 {
        if input.is_null() {
            input = ZERO.as_ptr();
        }
        pool.copy_from_slice(&state);
        rounds(&mut pool);
        let mut tmp = [0_u8; 64];
        for i in 0..16 {
            store32(
                unsafe { tmp.as_mut_ptr().add(i * 4) },
                pool[i].wrapping_add(state[i]),
            );
        }
        for (i, byte) in tmp.iter().enumerate().take(tail) {
            unsafe {
                out.add(i).write(*byte ^ *input.add(i));
            }
        }
        wipe(ops, tmp.as_mut_ptr(), tmp.len());
    }
    let next =
        (u64::from(state[12]) | (u64::from(state[13]) << 32)).wrapping_add(u64::from(tail > 0));
    wipe(ops, pool.as_mut_ptr(), size_of_val(&pool));
    wipe(ops, state.as_mut_ptr(), size_of_val(&state));
    next
}
fn ietf(
    ops: &Leaves,
    out: *mut u8,
    input: *const u8,
    size: usize,
    key: *const u8,
    nonce: *const u8,
    counter: u32,
) -> u32 {
    let big = u64::from(counter) + (u64::from(load32(nonce)) << 32);
    djb(ops, out, input, size, key, unsafe { nonce.add(4) }, big) as u32
}
fn x(
    ops: &Leaves,
    out: *mut u8,
    input: *const u8,
    size: usize,
    key: *const u8,
    nonce: *const u8,
    counter: u64,
) -> u64 {
    let mut sub_key = MaybeUninit::<[u8; 32]>::uninit();
    h(ops, sub_key.as_mut_ptr().cast(), key, nonce);
    let next = djb(
        ops,
        out,
        input,
        size,
        sub_key.as_ptr().cast(),
        unsafe { nonce.add(16) },
        counter,
    );
    wipe(ops, sub_key.as_mut_ptr(), 32);
    next
}
#[derive(Clone, Copy)]
struct Poly<'a> {
    ctx: *mut u8,
    layout: &'a PolyLayout,
}
impl Poly<'_> {
    fn at<T>(self, offset: usize) -> *mut T {
        unsafe { self.ctx.add(offset).cast() }
    }
    fn word(self, offset: usize, i: usize) -> u32 {
        unsafe { self.at::<u32>(offset).add(i).read_unaligned() }
    }
    fn put(self, offset: usize, i: usize, value: u32) {
        unsafe {
            self.at::<u32>(offset).add(i).write_unaligned(value);
        }
    }
    fn idx(self) -> usize {
        unsafe { self.at::<usize>(self.layout.c_idx).read_unaligned() }
    }
    fn set_idx(self, value: usize) {
        unsafe {
            self.at::<usize>(self.layout.c_idx).write_unaligned(value);
        }
    }
    fn init(self, key: *const u8) {
        for i in 0..5 {
            self.put(self.layout.h, i, 0);
        }
        self.set_idx(0);
        // As in C, leave all chunk and padding bytes untouched.
        for i in 0..4 {
            self.put(self.layout.r, i, load32(unsafe { key.add(i * 4) }));
        }
        for i in 0..4 {
            self.put(self.layout.pad, i, load32(unsafe { key.add(16 + i * 4) }));
        }
        for i in 0..4 {
            self.put(
                self.layout.r,
                i,
                self.word(self.layout.r, i) & if i == 0 { 0x0fff_ffff } else { 0x0fff_fffc },
            );
        }
    }
    fn blocks(self, mut input: *const u8, count: usize, end: u32) {
        let r0 = self.word(self.layout.r, 0);
        let r1 = self.word(self.layout.r, 1);
        let r2 = self.word(self.layout.r, 2);
        let r3 = self.word(self.layout.r, 3);
        let rr0 = (r0 >> 2).wrapping_mul(5);
        let rr1 = (r1 >> 2).wrapping_add(r1);
        let rr2 = (r2 >> 2).wrapping_add(r2);
        let rr3 = (r3 >> 2).wrapping_add(r3);
        let rr4 = r0 & 3;
        let mut h0 = self.word(self.layout.h, 0);
        let mut h1 = self.word(self.layout.h, 1);
        let mut h2 = self.word(self.layout.h, 2);
        let mut h3 = self.word(self.layout.h, 3);
        let mut h4 = self.word(self.layout.h, 4);
        for _ in 0..count {
            let s0 = W(u64::from(h0)) + W(u64::from(load32(input)));
            input = unsafe { input.add(4) };
            let s1 = W(u64::from(h1)) + W(u64::from(load32(input)));
            input = unsafe { input.add(4) };
            let s2 = W(u64::from(h2)) + W(u64::from(load32(input)));
            input = unsafe { input.add(4) };
            let s3 = W(u64::from(h3)) + W(u64::from(load32(input)));
            input = unsafe { input.add(4) };
            let s4 = W(u64::from(h4.wrapping_add(end)));
            let x0 = s0 * W(u64::from(r0))
                + s1 * W(u64::from(rr3))
                + s2 * W(u64::from(rr2))
                + s3 * W(u64::from(rr1))
                + s4 * W(u64::from(rr0));
            let x1 = s0 * W(u64::from(r1))
                + s1 * W(u64::from(r0))
                + s2 * W(u64::from(rr3))
                + s3 * W(u64::from(rr2))
                + s4 * W(u64::from(rr1));
            let x2 = s0 * W(u64::from(r2))
                + s1 * W(u64::from(r1))
                + s2 * W(u64::from(r0))
                + s3 * W(u64::from(rr3))
                + s4 * W(u64::from(rr2));
            let x3 = s0 * W(u64::from(r3))
                + s1 * W(u64::from(r2))
                + s2 * W(u64::from(r1))
                + s3 * W(u64::from(r0))
                + s4 * W(u64::from(rr3));
            let x4 = (s4.0 as u32).wrapping_mul(rr4);
            let u5 = x4.wrapping_add((x3.0 >> 32) as u32);
            let u0 = W(u64::from((u5 >> 2).wrapping_mul(5))) + W(x0.0 & 0xffff_ffff);
            let u1 = W(u0.0 >> 32) + W(x1.0 & 0xffff_ffff) + W(x0.0 >> 32);
            let u2 = W(u1.0 >> 32) + W(x2.0 & 0xffff_ffff) + W(x1.0 >> 32);
            let u3 = W(u2.0 >> 32) + W(x3.0 & 0xffff_ffff) + W(x2.0 >> 32);
            h0 = u0.0 as u32;
            h1 = u1.0 as u32;
            h2 = u2.0 as u32;
            h3 = u3.0 as u32;
            h4 = ((u3.0 >> 32) as u32).wrapping_add(u5 & 3);
        }
        for (i, value) in [h0, h1, h2, h3, h4].into_iter().enumerate() {
            self.put(self.layout.h, i, value);
        }
    }
    fn update(self, mut input: *const u8, mut size: usize) {
        if size == 0 {
            return;
        }
        let chunk = self.at::<u8>(self.layout.c);
        let mut idx = self.idx();
        let aligned = idx.wrapping_neg() & 15;
        for _ in 0..aligned.min(size) {
            unsafe {
                chunk.add(idx).write(*input);
                input = input.add(1);
            }
            idx += 1;
            size -= 1;
        }
        self.set_idx(idx);
        if idx == 16 {
            self.blocks(chunk, 1, 1);
            self.set_idx(0);
        }
        let count = size >> 4;
        self.blocks(input, count, 1);
        input = unsafe { input.add(count << 4) };
        size &= 15;
        idx = self.idx();
        for i in 0..size {
            unsafe {
                chunk.add(idx).write(*input.add(i));
            }
            idx += 1;
        }
        self.set_idx(idx);
    }
    fn final_mac(self, ops: &Leaves, out: *mut u8) {
        let idx = self.idx();
        if idx != 0 {
            let chunk = self.at::<u8>(self.layout.c);
            for i in idx..16 {
                unsafe {
                    chunk.add(i).write(0);
                }
            }
            unsafe {
                chunk.add(idx).write(1);
            }
            self.blocks(chunk, 1, 0);
        }
        let mut carry = 5_u64;
        for i in 0..4 {
            carry = carry.wrapping_add(u64::from(self.word(self.layout.h, i))) >> 32;
        }
        carry = carry.wrapping_add(u64::from(self.word(self.layout.h, 4)));
        carry = (carry >> 2).wrapping_mul(5);
        for i in 0..4 {
            carry = carry
                .wrapping_add(u64::from(self.word(self.layout.h, i)))
                .wrapping_add(u64::from(self.word(self.layout.pad, i)));
            store32(unsafe { out.add(i * 4) }, carry as u32);
            carry >>= 32;
        }
        wipe(ops, self.ctx, self.layout.size);
    }
}
fn auth(
    ops: &Leaves,
    out: *mut u8,
    key: *const u8,
    ad: *const u8,
    ad_size: usize,
    cipher: *const u8,
    size: usize,
) {
    let mut sizes = [0_u8; 16];
    store64(sizes.as_mut_ptr(), ad_size as u64);
    store64(unsafe { sizes.as_mut_ptr().add(8) }, size as u64);
    let mut scratch = MaybeUninit::<PolyScratch>::uninit();
    let poly = Poly {
        ctx: scratch.as_mut_ptr().cast(),
        layout: &SCRATCH_LAYOUT,
    };
    poly.init(key);
    poly.update(ad, ad_size);
    poly.update(ZERO.as_ptr(), ad_size.wrapping_neg() & 15);
    poly.update(cipher, size);
    poly.update(ZERO.as_ptr(), size.wrapping_neg() & 15);
    poly.update(sizes.as_ptr(), 16);
    poly.final_mac(ops, out);
}
#[derive(Clone, Copy)]
struct Aead<'a> {
    ctx: *mut u8,
    layout: &'a AeadLayout,
}
impl Aead<'_> {
    fn key(self) -> *mut u8 {
        unsafe { self.ctx.add(self.layout.key) }
    }
    fn nonce(self) -> *mut u8 {
        unsafe { self.ctx.add(self.layout.nonce) }
    }
    fn counter(self) -> u64 {
        unsafe {
            self.ctx
                .add(self.layout.counter)
                .cast::<u64>()
                .read_unaligned()
        }
    }
    fn set_counter(self, x: u64) {
        unsafe {
            self.ctx
                .add(self.layout.counter)
                .cast::<u64>()
                .write_unaligned(x);
        }
    }
    fn init(self, ops: Option<&Leaves>, variant: u8, key: *const u8, nonce: *const u8) {
        if variant == 0 {
            h(ops.unwrap(), self.key(), key, nonce);
        } else {
            for i in 0..32 {
                unsafe {
                    self.key().add(i).write(*key.add(i));
                }
            }
        }
        let skip = match variant {
            0 => 16,
            1 => 0,
            2 => 4,
            _ => unreachable!(),
        };
        for i in 0..8 {
            unsafe {
                self.nonce().add(i).write(*nonce.add(skip + i));
            }
        }
        self.set_counter(if variant == 2 {
            u64::from(load32(nonce)) << 32
        } else {
            0
        });
    }
    fn write(
        self,
        ops: &Leaves,
        cipher: *mut u8,
        mac: *mut u8,
        ad: *const u8,
        ad_size: usize,
        input: *const u8,
        size: usize,
    ) {
        let mut auth_key = MaybeUninit::<[u8; 64]>::uninit();
        djb(
            ops,
            auth_key.as_mut_ptr().cast(),
            ptr::null(),
            64,
            self.key(),
            self.nonce(),
            self.counter(),
        );
        djb(
            ops,
            cipher,
            input,
            size,
            self.key(),
            self.nonce(),
            self.counter().wrapping_add(1),
        );
        auth(
            ops,
            mac,
            auth_key.as_ptr().cast(),
            ad,
            ad_size,
            cipher,
            size,
        );
        for i in 0..32 {
            unsafe {
                self.key()
                    .add(i)
                    .write(*auth_key.as_ptr().cast::<u8>().add(32 + i));
            }
        }
        wipe(ops, auth_key.as_mut_ptr(), 64);
    }
    fn read(
        self,
        ops: &Leaves,
        out: *mut u8,
        mac: *const u8,
        ad: *const u8,
        ad_size: usize,
        cipher: *const u8,
        size: usize,
    ) -> i32 {
        let mut auth_key = MaybeUninit::<[u8; 64]>::uninit();
        let mut real_mac = MaybeUninit::<[u8; 16]>::uninit();
        djb(
            ops,
            auth_key.as_mut_ptr().cast(),
            ptr::null(),
            64,
            self.key(),
            self.nonce(),
            self.counter(),
        );
        auth(
            ops,
            real_mac.as_mut_ptr().cast(),
            auth_key.as_ptr().cast(),
            ad,
            ad_size,
            cipher,
            size,
        );
        let mismatch = unsafe { (ops.verify16)(mac, real_mac.as_ptr().cast()) };
        if mismatch == 0 {
            djb(
                ops,
                out,
                cipher,
                size,
                self.key(),
                self.nonce(),
                self.counter().wrapping_add(1),
            );
            for i in 0..32 {
                unsafe {
                    self.key()
                        .add(i)
                        .write(*auth_key.as_ptr().cast::<u8>().add(32 + i));
                }
            }
        }
        wipe(ops, auth_key.as_mut_ptr(), 64);
        wipe(ops, real_mac.as_mut_ptr(), 16);
        mismatch
    }
}
// FFI inputs satisfy the common header contract; raw contexts remain unborrowed.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_chacha_h(
    ops: *const Leaves,
    out: *mut u8,
    key: *const u8,
    nonce: *const u8,
) {
    h(unsafe { &*ops }, out, key, nonce);
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_chacha_djb(
    ops: *const Leaves,
    out: *mut u8,
    input: *const u8,
    size: usize,
    key: *const u8,
    nonce: *const u8,
    counter: u64,
) -> u64 {
    djb(unsafe { &*ops }, out, input, size, key, nonce, counter)
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_chacha_ietf(
    ops: *const Leaves,
    out: *mut u8,
    input: *const u8,
    size: usize,
    key: *const u8,
    nonce: *const u8,
    counter: u32,
) -> u32 {
    ietf(unsafe { &*ops }, out, input, size, key, nonce, counter)
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_chacha_x(
    ops: *const Leaves,
    out: *mut u8,
    input: *const u8,
    size: usize,
    key: *const u8,
    nonce: *const u8,
    counter: u64,
) -> u64 {
    x(unsafe { &*ops }, out, input, size, key, nonce, counter)
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_poly_init(
    layout: *const PolyLayout,
    ctx: *mut c_void,
    key: *const u8,
) {
    Poly {
        ctx: ctx.cast(),
        layout: unsafe { &*layout },
    }
    .init(key);
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_poly_update(
    layout: *const PolyLayout,
    ctx: *mut c_void,
    input: *const u8,
    size: usize,
) {
    if size != 0 {
        Poly {
            ctx: ctx.cast(),
            layout: unsafe { &*layout },
        }
        .update(input, size);
    }
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_poly_final(
    ops: *const Leaves,
    layout: *const PolyLayout,
    ctx: *mut c_void,
    out: *mut u8,
) {
    Poly {
        ctx: ctx.cast(),
        layout: unsafe { &*layout },
    }
    .final_mac(unsafe { &*ops }, out);
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_poly(
    ops: *const Leaves,
    layout: *const PolyLayout,
    ctx: *mut c_void,
    out: *mut u8,
    input: *const u8,
    size: usize,
    key: *const u8,
) {
    let poly = Poly {
        ctx: ctx.cast(),
        layout: unsafe { &*layout },
    };
    poly.init(key);
    poly.update(input, size);
    poly.final_mac(unsafe { &*ops }, out);
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_aead_init_x(
    ops: *const Leaves,
    layout: *const AeadLayout,
    ctx: *mut c_void,
    key: *const u8,
    nonce: *const u8,
) {
    Aead {
        ctx: ctx.cast(),
        layout: unsafe { &*layout },
    }
    .init(Some(unsafe { &*ops }), 0, key, nonce);
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_aead_init_djb(
    layout: *const AeadLayout,
    ctx: *mut c_void,
    key: *const u8,
    nonce: *const u8,
) {
    Aead {
        ctx: ctx.cast(),
        layout: unsafe { &*layout },
    }
    .init(None, 1, key, nonce);
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_aead_init_ietf(
    layout: *const AeadLayout,
    ctx: *mut c_void,
    key: *const u8,
    nonce: *const u8,
) {
    Aead {
        ctx: ctx.cast(),
        layout: unsafe { &*layout },
    }
    .init(None, 2, key, nonce);
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_aead_write(
    ops: *const Leaves,
    layout: *const AeadLayout,
    ctx: *mut c_void,
    cipher: *mut u8,
    mac: *mut u8,
    ad: *const u8,
    ad_size: usize,
    input: *const u8,
    size: usize,
) {
    Aead {
        ctx: ctx.cast(),
        layout: unsafe { &*layout },
    }
    .write(unsafe { &*ops }, cipher, mac, ad, ad_size, input, size);
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_aead_read(
    ops: *const Leaves,
    layout: *const AeadLayout,
    ctx: *mut c_void,
    out: *mut u8,
    mac: *const u8,
    ad: *const u8,
    ad_size: usize,
    cipher: *const u8,
    size: usize,
) -> i32 {
    Aead {
        ctx: ctx.cast(),
        layout: unsafe { &*layout },
    }
    .read(unsafe { &*ops }, out, mac, ad, ad_size, cipher, size)
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_aead_lock(
    ops: *const Leaves,
    layout: *const AeadLayout,
    ctx: *mut c_void,
    cipher: *mut u8,
    mac: *mut u8,
    key: *const u8,
    nonce: *const u8,
    ad: *const u8,
    ad_size: usize,
    input: *const u8,
    size: usize,
) {
    let ops = unsafe { &*ops };
    let aead = Aead {
        ctx: ctx.cast(),
        layout: unsafe { &*layout },
    };
    aead.init(Some(ops), 0, key, nonce);
    aead.write(ops, cipher, mac, ad, ad_size, input, size);
    wipe(ops, aead.ctx, aead.layout.size);
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_aead_unlock(
    ops: *const Leaves,
    layout: *const AeadLayout,
    ctx: *mut c_void,
    out: *mut u8,
    mac: *const u8,
    key: *const u8,
    nonce: *const u8,
    ad: *const u8,
    ad_size: usize,
    cipher: *const u8,
    size: usize,
) -> i32 {
    let ops = unsafe { &*ops };
    let aead = Aead {
        ctx: ctx.cast(),
        layout: unsafe { &*layout },
    };
    aead.init(Some(ops), 0, key, nonce);
    let result = aead.read(ops, out, mac, ad, ad_size, cipher, size);
    wipe(ops, aead.ctx, aead.layout.size);
    result
}

// Layout metadata only; secret context contents never cross.
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDCryptoLeaves" ["probe"] => Leaves {
        wipe,
        verify16,
    };
    "OpenTTDPolyLayout" ["probe"] => PolyLayout {
        size,
        alignment,
        c,
        c_idx,
        r,
        pad,
        h,
    };
    "OpenTTDAeadLayout" ["probe"] => AeadLayout {
        size,
        alignment,
        counter,
        key,
        nonce,
    };
}
