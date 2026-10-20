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

//! Monocypher 4.0.2 normal X25519 and coarse Montgomery ladder.
//! Private ten-limb arithmetic remains internal; original C++ field helpers
//! still serve untouched Edwards/Elligator code. No heap or vendor import.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_lines
)]
// Original unrolled field notation/order and proven-range/modular narrowing.
use std::ffi::c_void;
use std::mem::{MaybeUninit, size_of};
type Fe = [i32; 10];
#[repr(C)]
pub(crate) struct Leaves {
    wipe: unsafe extern "C" fn(*mut c_void, usize),
    verify32: unsafe extern "C" fn(*const u8, *const u8) -> i32,
}
const SQRTM1: Fe = [
    -32_595_792,
    -7_943_725,
    9_377_950,
    3_500_415,
    12_389_472,
    -272_473,
    -25_146_209,
    -2_005_654,
    326_686,
    11_406_482,
];
const BASE: [u8; 32] = [
    9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
// CLOSED arithmetic bounds: carry outputs even |limb|<1.1*2^25 and odd
// |limb|<1.1*2^24. Ladder add/sub remain <1.65*2^26/25; doubled odd
// terms and *19/*38 factors fit i32. Original unrolled mul/sq sums are
// <0.67*2^61 (others smaller), mul_small products <2^58; carry additions
// therefore stay well inside i64, including rounding. Serialization has
// |19*t9+2^24|<2^29; its signed i32 operations fit. Frombytes intermediates
// are <=2^32. These helpers use unchecked SIGNED arithmetic under those
// established internal bounds, not signed wrapping or silently hidden errors.
// It preserves source arithmetic and avoids overflow branches on secret limbs.
#[inline]
fn add32(a: i32, b: i32) -> i32 {
    unsafe { a.unchecked_add(b) }
}
#[inline]
fn sub32(a: i32, b: i32) -> i32 {
    unsafe { a.unchecked_sub(b) }
}
#[inline]
fn mul32(a: i32, b: i32) -> i32 {
    unsafe { a.unchecked_mul(b) }
}
#[inline]
fn add64(a: i64, b: i64) -> i64 {
    unsafe { a.unchecked_add(b) }
}
#[inline]
fn sub64(a: i64, b: i64) -> i64 {
    unsafe { a.unchecked_sub(b) }
}
#[inline]
fn mul64(a: i64, b: i64) -> i64 {
    unsafe { a.unchecked_mul(b) }
}
#[inline]
fn get(f: *const i32, i: usize) -> i32 {
    unsafe { f.add(i).read() }
}
#[inline]
fn put(h: *mut i32, i: usize, value: i32) {
    unsafe {
        h.add(i).write(value);
    }
}
fn wipe<T>(leaves: &Leaves, p: *mut T, size: usize) {
    unsafe {
        (leaves.wipe)(p.cast(), size);
    }
}
fn load(s: *const u8, count: usize) -> u32 {
    let mut value = 0;
    for i in 0..count {
        value |= u32::from(unsafe { s.add(i).read() }) << (i * 8);
    }
    value
}
fn store(s: *mut u8, value: u32) {
    for i in 0..4 {
        unsafe {
            s.add(i).write((value >> (i * 8)) as u8);
        }
    }
}
fn fe_0(h: *mut i32) {
    for i in 0..10 {
        put(h, i, 0);
    }
}
fn fe_1(h: *mut i32) {
    put(h, 0, 1);
    for i in 1..10 {
        put(h, i, 0);
    }
}
fn fe_copy(h: *mut i32, f: *const i32) {
    for i in 0..10 {
        put(h, i, get(f, i));
    }
}
fn fe_neg(h: *mut i32, f: *const i32) {
    for i in 0..10 {
        put(h, i, sub32(0, get(f, i)));
    }
}
fn fe_add(h: *mut i32, f: *const i32, g: *const i32) {
    for i in 0..10 {
        put(h, i, add32(get(f, i), get(g, i)));
    }
}
fn fe_sub(h: *mut i32, f: *const i32, g: *const i32) {
    for i in 0..10 {
        put(h, i, sub32(get(f, i), get(g, i)));
    }
}
fn fe_cswap(f: *mut i32, g: *mut i32, b: i32) {
    let mask = sub32(0, b);
    for i in 0..10 {
        let x = (get(f, i) ^ get(g, i)) & mask;
        put(f, i, get(f, i) ^ x);
        put(g, i, get(g, i) ^ x);
    }
}
fn fe_ccopy(f: *mut i32, g: *const i32, b: i32) {
    let mask = sub32(0, b);
    for i in 0..10 {
        let x = (get(f, i) ^ get(g, i)) & mask;
        put(f, i, get(f, i) ^ x);
    }
}
// Exact original carry order, with Rust's defined arithmetic right shift.
macro_rules! carry {
    ($h:expr,$t0:ident,$t1:ident,$t2:ident,$t3:ident,$t4:ident,$t5:ident,$t6:ident,$t7:ident,$t8:ident,$t9:ident) => {{
        let c = add64($t0, 1_i64 << 25) >> 26;
        $t0 = sub64($t0, mul64(c, 1_i64 << 26));
        $t1 = add64($t1, c);
        let c = add64($t4, 1_i64 << 25) >> 26;
        $t4 = sub64($t4, mul64(c, 1_i64 << 26));
        $t5 = add64($t5, c);
        let c = add64($t1, 1_i64 << 24) >> 25;
        $t1 = sub64($t1, mul64(c, 1_i64 << 25));
        $t2 = add64($t2, c);
        let c = add64($t5, 1_i64 << 24) >> 25;
        $t5 = sub64($t5, mul64(c, 1_i64 << 25));
        $t6 = add64($t6, c);
        let c = add64($t2, 1_i64 << 25) >> 26;
        $t2 = sub64($t2, mul64(c, 1_i64 << 26));
        $t3 = add64($t3, c);
        let c = add64($t6, 1_i64 << 25) >> 26;
        $t6 = sub64($t6, mul64(c, 1_i64 << 26));
        $t7 = add64($t7, c);
        let c = add64($t3, 1_i64 << 24) >> 25;
        $t3 = sub64($t3, mul64(c, 1_i64 << 25));
        $t4 = add64($t4, c);
        let c = add64($t7, 1_i64 << 24) >> 25;
        $t7 = sub64($t7, mul64(c, 1_i64 << 25));
        $t8 = add64($t8, c);
        let c = add64($t4, 1_i64 << 25) >> 26;
        $t4 = sub64($t4, mul64(c, 1_i64 << 26));
        $t5 = add64($t5, c);
        let c = add64($t8, 1_i64 << 25) >> 26;
        $t8 = sub64($t8, mul64(c, 1_i64 << 26));
        $t9 = add64($t9, c);
        let c = add64($t9, 1_i64 << 24) >> 25;
        $t9 = sub64($t9, mul64(c, 1_i64 << 25));
        $t0 = add64($t0, mul64(c, 19));
        let c = add64($t0, 1_i64 << 25) >> 26;
        $t0 = sub64($t0, mul64(c, 1_i64 << 26));
        $t1 = add64($t1, c);
        put($h, 0, $t0 as i32);
        put($h, 1, $t1 as i32);
        put($h, 2, $t2 as i32);
        put($h, 3, $t3 as i32);
        put($h, 4, $t4 as i32);
        put($h, 5, $t5 as i32);
        put($h, 6, $t6 as i32);
        put($h, 7, $t7 as i32);
        put($h, 8, $t8 as i32);
        put($h, 9, $t9 as i32);
    }};
}
fn fe_frombytes(h: *mut i32, s: *const u8) {
    let mut t0 = i64::from(load(s, 4));
    let mut t1 = i64::from(load(unsafe { s.add(4) }, 3) << 6);
    let mut t2 = i64::from(load(unsafe { s.add(7) }, 3) << 5);
    let mut t3 = i64::from(load(unsafe { s.add(10) }, 3) << 3);
    let mut t4 = i64::from(load(unsafe { s.add(13) }, 3) << 2);
    let mut t5 = i64::from(load(unsafe { s.add(16) }, 4));
    let mut t6 = i64::from(load(unsafe { s.add(20) }, 3) << 7);
    let mut t7 = i64::from(load(unsafe { s.add(23) }, 3) << 5);
    let mut t8 = i64::from(load(unsafe { s.add(26) }, 3) << 4);
    let mut t9 = i64::from((load(unsafe { s.add(29) }, 3) & 0x007f_ffff) << 2);
    carry!(h, t0, t1, t2, t3, t4, t5, t6, t7, t8, t9);
}
fn fe_tobytes(leaves: &Leaves, s: *mut u8, h: *const i32) {
    let mut storage = MaybeUninit::<Fe>::uninit();
    let t = storage.as_mut_ptr().cast::<i32>();
    fe_copy(t, h);
    let mut q = add32(mul32(19, get(t, 9)), 1_i32 << 24) >> 25;
    for i in 0..5 {
        q = add32(q, get(t, i * 2)) >> 26;
        q = add32(q, get(t, i * 2 + 1)) >> 25;
    }
    q = mul32(q, 19);
    for i in 0..5 {
        put(t, i * 2, add32(get(t, i * 2), q));
        q = get(t, i * 2) >> 26;
        put(t, i * 2, sub32(get(t, i * 2), mul32(q, 1_i32 << 26)));
        put(t, i * 2 + 1, add32(get(t, i * 2 + 1), q));
        q = get(t, i * 2 + 1) >> 25;
        put(
            t,
            i * 2 + 1,
            sub32(get(t, i * 2 + 1), mul32(q, 1_i32 << 25)),
        );
    }
    store(
        unsafe { s.add(0) },
        (get(t, 0) as u32) | ((get(t, 1) as u32) << 26),
    );
    store(
        unsafe { s.add(4) },
        ((get(t, 1) as u32) >> 6) | ((get(t, 2) as u32) << 19),
    );
    store(
        unsafe { s.add(8) },
        ((get(t, 2) as u32) >> 13) | ((get(t, 3) as u32) << 13),
    );
    store(
        unsafe { s.add(12) },
        ((get(t, 3) as u32) >> 19) | ((get(t, 4) as u32) << 6),
    );
    store(
        unsafe { s.add(16) },
        (get(t, 5) as u32) | ((get(t, 6) as u32) << 25),
    );
    store(
        unsafe { s.add(20) },
        ((get(t, 6) as u32) >> 7) | ((get(t, 7) as u32) << 19),
    );
    store(
        unsafe { s.add(24) },
        ((get(t, 7) as u32) >> 13) | ((get(t, 8) as u32) << 12),
    );
    store(
        unsafe { s.add(28) },
        ((get(t, 8) as u32) >> 20) | ((get(t, 9) as u32) << 6),
    );
    wipe(leaves, t, size_of::<Fe>());
}
fn fe_mul_small(h: *mut i32, f: *const i32, g: i32) {
    let mut t0 = mul64(i64::from(get(f, 0)), i64::from(g));
    let mut t1 = mul64(i64::from(get(f, 1)), i64::from(g));
    let mut t2 = mul64(i64::from(get(f, 2)), i64::from(g));
    let mut t3 = mul64(i64::from(get(f, 3)), i64::from(g));
    let mut t4 = mul64(i64::from(get(f, 4)), i64::from(g));
    let mut t5 = mul64(i64::from(get(f, 5)), i64::from(g));
    let mut t6 = mul64(i64::from(get(f, 6)), i64::from(g));
    let mut t7 = mul64(i64::from(get(f, 7)), i64::from(g));
    let mut t8 = mul64(i64::from(get(f, 8)), i64::from(g));
    let mut t9 = mul64(i64::from(get(f, 9)), i64::from(g));
    carry!(h, t0, t1, t2, t3, t4, t5, t6, t7, t8, t9);
}
fn fe_mul(h: *mut i32, f: *const i32, g: *const i32) {
    let f0 = get(f, 0);
    let f1 = get(f, 1);
    let f2 = get(f, 2);
    let f3 = get(f, 3);
    let f4 = get(f, 4);
    let f5 = get(f, 5);
    let f6 = get(f, 6);
    let f7 = get(f, 7);
    let f8 = get(f, 8);
    let f9 = get(f, 9);
    let g0 = get(g, 0);
    let g1 = get(g, 1);
    let g2 = get(g, 2);
    let g3 = get(g, 3);
    let g4 = get(g, 4);
    let g5 = get(g, 5);
    let g6 = get(g, 6);
    let g7 = get(g, 7);
    let g8 = get(g, 8);
    let g9 = get(g, 9);
    let f1_2 = mul32(f1, 2);
    let f3_2 = mul32(f3, 2);
    let f5_2 = mul32(f5, 2);
    let f7_2 = mul32(f7, 2);
    let f9_2 = mul32(f9, 2);
    let g1_19 = mul32(g1, 19);
    let g2_19 = mul32(g2, 19);
    let g3_19 = mul32(g3, 19);
    let g4_19 = mul32(g4, 19);
    let g5_19 = mul32(g5, 19);
    let g6_19 = mul32(g6, 19);
    let g7_19 = mul32(g7, 19);
    let g8_19 = mul32(g8, 19);
    let g9_19 = mul32(g9, 19);
    let mut t0 = add64(
        add64(
            add64(
                add64(
                    add64(
                        add64(
                            add64(
                                add64(
                                    add64(
                                        mul64(i64::from(f0), i64::from(g0)),
                                        mul64(i64::from(f1_2), i64::from(g9_19)),
                                    ),
                                    mul64(i64::from(f2), i64::from(g8_19)),
                                ),
                                mul64(i64::from(f3_2), i64::from(g7_19)),
                            ),
                            mul64(i64::from(f4), i64::from(g6_19)),
                        ),
                        mul64(i64::from(f5_2), i64::from(g5_19)),
                    ),
                    mul64(i64::from(f6), i64::from(g4_19)),
                ),
                mul64(i64::from(f7_2), i64::from(g3_19)),
            ),
            mul64(i64::from(f8), i64::from(g2_19)),
        ),
        mul64(i64::from(f9_2), i64::from(g1_19)),
    );
    let mut t1 = add64(
        add64(
            add64(
                add64(
                    add64(
                        add64(
                            add64(
                                add64(
                                    add64(
                                        mul64(i64::from(f0), i64::from(g1)),
                                        mul64(i64::from(f1), i64::from(g0)),
                                    ),
                                    mul64(i64::from(f2), i64::from(g9_19)),
                                ),
                                mul64(i64::from(f3), i64::from(g8_19)),
                            ),
                            mul64(i64::from(f4), i64::from(g7_19)),
                        ),
                        mul64(i64::from(f5), i64::from(g6_19)),
                    ),
                    mul64(i64::from(f6), i64::from(g5_19)),
                ),
                mul64(i64::from(f7), i64::from(g4_19)),
            ),
            mul64(i64::from(f8), i64::from(g3_19)),
        ),
        mul64(i64::from(f9), i64::from(g2_19)),
    );
    let mut t2 = add64(
        add64(
            add64(
                add64(
                    add64(
                        add64(
                            add64(
                                add64(
                                    add64(
                                        mul64(i64::from(f0), i64::from(g2)),
                                        mul64(i64::from(f1_2), i64::from(g1)),
                                    ),
                                    mul64(i64::from(f2), i64::from(g0)),
                                ),
                                mul64(i64::from(f3_2), i64::from(g9_19)),
                            ),
                            mul64(i64::from(f4), i64::from(g8_19)),
                        ),
                        mul64(i64::from(f5_2), i64::from(g7_19)),
                    ),
                    mul64(i64::from(f6), i64::from(g6_19)),
                ),
                mul64(i64::from(f7_2), i64::from(g5_19)),
            ),
            mul64(i64::from(f8), i64::from(g4_19)),
        ),
        mul64(i64::from(f9_2), i64::from(g3_19)),
    );
    let mut t3 = add64(
        add64(
            add64(
                add64(
                    add64(
                        add64(
                            add64(
                                add64(
                                    add64(
                                        mul64(i64::from(f0), i64::from(g3)),
                                        mul64(i64::from(f1), i64::from(g2)),
                                    ),
                                    mul64(i64::from(f2), i64::from(g1)),
                                ),
                                mul64(i64::from(f3), i64::from(g0)),
                            ),
                            mul64(i64::from(f4), i64::from(g9_19)),
                        ),
                        mul64(i64::from(f5), i64::from(g8_19)),
                    ),
                    mul64(i64::from(f6), i64::from(g7_19)),
                ),
                mul64(i64::from(f7), i64::from(g6_19)),
            ),
            mul64(i64::from(f8), i64::from(g5_19)),
        ),
        mul64(i64::from(f9), i64::from(g4_19)),
    );
    let mut t4 = add64(
        add64(
            add64(
                add64(
                    add64(
                        add64(
                            add64(
                                add64(
                                    add64(
                                        mul64(i64::from(f0), i64::from(g4)),
                                        mul64(i64::from(f1_2), i64::from(g3)),
                                    ),
                                    mul64(i64::from(f2), i64::from(g2)),
                                ),
                                mul64(i64::from(f3_2), i64::from(g1)),
                            ),
                            mul64(i64::from(f4), i64::from(g0)),
                        ),
                        mul64(i64::from(f5_2), i64::from(g9_19)),
                    ),
                    mul64(i64::from(f6), i64::from(g8_19)),
                ),
                mul64(i64::from(f7_2), i64::from(g7_19)),
            ),
            mul64(i64::from(f8), i64::from(g6_19)),
        ),
        mul64(i64::from(f9_2), i64::from(g5_19)),
    );
    let mut t5 = add64(
        add64(
            add64(
                add64(
                    add64(
                        add64(
                            add64(
                                add64(
                                    add64(
                                        mul64(i64::from(f0), i64::from(g5)),
                                        mul64(i64::from(f1), i64::from(g4)),
                                    ),
                                    mul64(i64::from(f2), i64::from(g3)),
                                ),
                                mul64(i64::from(f3), i64::from(g2)),
                            ),
                            mul64(i64::from(f4), i64::from(g1)),
                        ),
                        mul64(i64::from(f5), i64::from(g0)),
                    ),
                    mul64(i64::from(f6), i64::from(g9_19)),
                ),
                mul64(i64::from(f7), i64::from(g8_19)),
            ),
            mul64(i64::from(f8), i64::from(g7_19)),
        ),
        mul64(i64::from(f9), i64::from(g6_19)),
    );
    let mut t6 = add64(
        add64(
            add64(
                add64(
                    add64(
                        add64(
                            add64(
                                add64(
                                    add64(
                                        mul64(i64::from(f0), i64::from(g6)),
                                        mul64(i64::from(f1_2), i64::from(g5)),
                                    ),
                                    mul64(i64::from(f2), i64::from(g4)),
                                ),
                                mul64(i64::from(f3_2), i64::from(g3)),
                            ),
                            mul64(i64::from(f4), i64::from(g2)),
                        ),
                        mul64(i64::from(f5_2), i64::from(g1)),
                    ),
                    mul64(i64::from(f6), i64::from(g0)),
                ),
                mul64(i64::from(f7_2), i64::from(g9_19)),
            ),
            mul64(i64::from(f8), i64::from(g8_19)),
        ),
        mul64(i64::from(f9_2), i64::from(g7_19)),
    );
    let mut t7 = add64(
        add64(
            add64(
                add64(
                    add64(
                        add64(
                            add64(
                                add64(
                                    add64(
                                        mul64(i64::from(f0), i64::from(g7)),
                                        mul64(i64::from(f1), i64::from(g6)),
                                    ),
                                    mul64(i64::from(f2), i64::from(g5)),
                                ),
                                mul64(i64::from(f3), i64::from(g4)),
                            ),
                            mul64(i64::from(f4), i64::from(g3)),
                        ),
                        mul64(i64::from(f5), i64::from(g2)),
                    ),
                    mul64(i64::from(f6), i64::from(g1)),
                ),
                mul64(i64::from(f7), i64::from(g0)),
            ),
            mul64(i64::from(f8), i64::from(g9_19)),
        ),
        mul64(i64::from(f9), i64::from(g8_19)),
    );
    let mut t8 = add64(
        add64(
            add64(
                add64(
                    add64(
                        add64(
                            add64(
                                add64(
                                    add64(
                                        mul64(i64::from(f0), i64::from(g8)),
                                        mul64(i64::from(f1_2), i64::from(g7)),
                                    ),
                                    mul64(i64::from(f2), i64::from(g6)),
                                ),
                                mul64(i64::from(f3_2), i64::from(g5)),
                            ),
                            mul64(i64::from(f4), i64::from(g4)),
                        ),
                        mul64(i64::from(f5_2), i64::from(g3)),
                    ),
                    mul64(i64::from(f6), i64::from(g2)),
                ),
                mul64(i64::from(f7_2), i64::from(g1)),
            ),
            mul64(i64::from(f8), i64::from(g0)),
        ),
        mul64(i64::from(f9_2), i64::from(g9_19)),
    );
    let mut t9 = add64(
        add64(
            add64(
                add64(
                    add64(
                        add64(
                            add64(
                                add64(
                                    add64(
                                        mul64(i64::from(f0), i64::from(g9)),
                                        mul64(i64::from(f1), i64::from(g8)),
                                    ),
                                    mul64(i64::from(f2), i64::from(g7)),
                                ),
                                mul64(i64::from(f3), i64::from(g6)),
                            ),
                            mul64(i64::from(f4), i64::from(g5)),
                        ),
                        mul64(i64::from(f5), i64::from(g4)),
                    ),
                    mul64(i64::from(f6), i64::from(g3)),
                ),
                mul64(i64::from(f7), i64::from(g2)),
            ),
            mul64(i64::from(f8), i64::from(g1)),
        ),
        mul64(i64::from(f9), i64::from(g0)),
    );
    carry!(h, t0, t1, t2, t3, t4, t5, t6, t7, t8, t9);
}
fn fe_sq(h: *mut i32, f: *const i32) {
    let f0 = get(f, 0);
    let f1 = get(f, 1);
    let f2 = get(f, 2);
    let f3 = get(f, 3);
    let f4 = get(f, 4);
    let f5 = get(f, 5);
    let f6 = get(f, 6);
    let f7 = get(f, 7);
    let f8 = get(f, 8);
    let f9 = get(f, 9);
    let f0_2 = mul32(f0, 2);
    let f1_2 = mul32(f1, 2);
    let f2_2 = mul32(f2, 2);
    let f3_2 = mul32(f3, 2);
    let f4_2 = mul32(f4, 2);
    let f5_2 = mul32(f5, 2);
    let f6_2 = mul32(f6, 2);
    let f7_2 = mul32(f7, 2);
    let f5_38 = mul32(f5, 38);
    let f6_19 = mul32(f6, 19);
    let f7_38 = mul32(f7, 38);
    let f8_19 = mul32(f8, 19);
    let f9_38 = mul32(f9, 38);
    let mut t0 = add64(
        add64(
            add64(
                add64(
                    add64(
                        mul64(i64::from(f0), i64::from(f0)),
                        mul64(i64::from(f1_2), i64::from(f9_38)),
                    ),
                    mul64(i64::from(f2_2), i64::from(f8_19)),
                ),
                mul64(i64::from(f3_2), i64::from(f7_38)),
            ),
            mul64(i64::from(f4_2), i64::from(f6_19)),
        ),
        mul64(i64::from(f5), i64::from(f5_38)),
    );
    let mut t1 = add64(
        add64(
            add64(
                add64(
                    mul64(i64::from(f0_2), i64::from(f1)),
                    mul64(i64::from(f2), i64::from(f9_38)),
                ),
                mul64(i64::from(f3_2), i64::from(f8_19)),
            ),
            mul64(i64::from(f4), i64::from(f7_38)),
        ),
        mul64(i64::from(f5_2), i64::from(f6_19)),
    );
    let mut t2 = add64(
        add64(
            add64(
                add64(
                    add64(
                        mul64(i64::from(f0_2), i64::from(f2)),
                        mul64(i64::from(f1_2), i64::from(f1)),
                    ),
                    mul64(i64::from(f3_2), i64::from(f9_38)),
                ),
                mul64(i64::from(f4_2), i64::from(f8_19)),
            ),
            mul64(i64::from(f5_2), i64::from(f7_38)),
        ),
        mul64(i64::from(f6), i64::from(f6_19)),
    );
    let mut t3 = add64(
        add64(
            add64(
                add64(
                    mul64(i64::from(f0_2), i64::from(f3)),
                    mul64(i64::from(f1_2), i64::from(f2)),
                ),
                mul64(i64::from(f4), i64::from(f9_38)),
            ),
            mul64(i64::from(f5_2), i64::from(f8_19)),
        ),
        mul64(i64::from(f6), i64::from(f7_38)),
    );
    let mut t4 = add64(
        add64(
            add64(
                add64(
                    add64(
                        mul64(i64::from(f0_2), i64::from(f4)),
                        mul64(i64::from(f1_2), i64::from(f3_2)),
                    ),
                    mul64(i64::from(f2), i64::from(f2)),
                ),
                mul64(i64::from(f5_2), i64::from(f9_38)),
            ),
            mul64(i64::from(f6_2), i64::from(f8_19)),
        ),
        mul64(i64::from(f7), i64::from(f7_38)),
    );
    let mut t5 = add64(
        add64(
            add64(
                add64(
                    mul64(i64::from(f0_2), i64::from(f5)),
                    mul64(i64::from(f1_2), i64::from(f4)),
                ),
                mul64(i64::from(f2_2), i64::from(f3)),
            ),
            mul64(i64::from(f6), i64::from(f9_38)),
        ),
        mul64(i64::from(f7_2), i64::from(f8_19)),
    );
    let mut t6 = add64(
        add64(
            add64(
                add64(
                    add64(
                        mul64(i64::from(f0_2), i64::from(f6)),
                        mul64(i64::from(f1_2), i64::from(f5_2)),
                    ),
                    mul64(i64::from(f2_2), i64::from(f4)),
                ),
                mul64(i64::from(f3_2), i64::from(f3)),
            ),
            mul64(i64::from(f7_2), i64::from(f9_38)),
        ),
        mul64(i64::from(f8), i64::from(f8_19)),
    );
    let mut t7 = add64(
        add64(
            add64(
                add64(
                    mul64(i64::from(f0_2), i64::from(f7)),
                    mul64(i64::from(f1_2), i64::from(f6)),
                ),
                mul64(i64::from(f2_2), i64::from(f5)),
            ),
            mul64(i64::from(f3_2), i64::from(f4)),
        ),
        mul64(i64::from(f8), i64::from(f9_38)),
    );
    let mut t8 = add64(
        add64(
            add64(
                add64(
                    add64(
                        mul64(i64::from(f0_2), i64::from(f8)),
                        mul64(i64::from(f1_2), i64::from(f7_2)),
                    ),
                    mul64(i64::from(f2_2), i64::from(f6)),
                ),
                mul64(i64::from(f3_2), i64::from(f5_2)),
            ),
            mul64(i64::from(f4), i64::from(f4)),
        ),
        mul64(i64::from(f9), i64::from(f9_38)),
    );
    let mut t9 = add64(
        add64(
            add64(
                add64(
                    mul64(i64::from(f0_2), i64::from(f9)),
                    mul64(i64::from(f1_2), i64::from(f8)),
                ),
                mul64(i64::from(f2_2), i64::from(f7)),
            ),
            mul64(i64::from(f3_2), i64::from(f6)),
        ),
        mul64(i64::from(f4), i64::from(f5_2)),
    );
    carry!(h, t0, t1, t2, t3, t4, t5, t6, t7, t8, t9);
}
fn fe_isequal(leaves: &Leaves, f: *const i32, g: *const i32) -> i32 {
    let mut fs_storage = MaybeUninit::<[u8; 32]>::uninit();
    let fs = fs_storage.as_mut_ptr().cast::<u8>();
    let mut gs_storage = MaybeUninit::<[u8; 32]>::uninit();
    let gs = gs_storage.as_mut_ptr().cast::<u8>();
    fe_tobytes(leaves, fs, f);
    fe_tobytes(leaves, gs, g);
    let different = unsafe { (leaves.verify32)(fs, gs) };
    wipe(leaves, fs, 32);
    wipe(leaves, gs, 32);
    add32(1, different)
}
fn invsqrt(leaves: &Leaves, isr: *mut i32, x: *const i32) -> i32 {
    let mut t0_storage = MaybeUninit::<Fe>::uninit();
    let t0 = t0_storage.as_mut_ptr().cast::<i32>();
    let mut t1_storage = MaybeUninit::<Fe>::uninit();
    let t1 = t1_storage.as_mut_ptr().cast::<i32>();
    let mut t2_storage = MaybeUninit::<Fe>::uninit();
    let t2 = t2_storage.as_mut_ptr().cast::<i32>();
    fe_sq(t0, x);
    fe_sq(t1, t0);
    fe_sq(t1, t1);
    fe_mul(t1, x, t1);
    fe_mul(t0, t0, t1);
    fe_sq(t0, t0);
    fe_mul(t0, t1, t0);
    fe_sq(t1, t0);
    for _ in 1..5 {
        fe_sq(t1, t1);
    }
    fe_mul(t0, t1, t0);
    fe_sq(t1, t0);
    for _ in 1..10 {
        fe_sq(t1, t1);
    }
    fe_mul(t1, t1, t0);
    fe_sq(t2, t1);
    for _ in 1..20 {
        fe_sq(t2, t2);
    }
    fe_mul(t1, t2, t1);
    fe_sq(t1, t1);
    for _ in 1..10 {
        fe_sq(t1, t1);
    }
    fe_mul(t0, t1, t0);
    fe_sq(t1, t0);
    for _ in 1..50 {
        fe_sq(t1, t1);
    }
    fe_mul(t1, t1, t0);
    fe_sq(t2, t1);
    for _ in 1..100 {
        fe_sq(t2, t2);
    }
    fe_mul(t1, t2, t1);
    fe_sq(t1, t1);
    for _ in 1..50 {
        fe_sq(t1, t1);
    }
    fe_mul(t0, t1, t0);
    fe_sq(t0, t0);
    for _ in 1..2 {
        fe_sq(t0, t0);
    }
    fe_mul(t0, t0, x);
    let quartic = t1;
    fe_sq(quartic, t0);
    fe_mul(quartic, quartic, x);
    let check = t2;
    fe_0(check);
    let z0 = fe_isequal(leaves, x, check);
    fe_1(check);
    let p1 = fe_isequal(leaves, quartic, check);
    fe_neg(check, check);
    let m1 = fe_isequal(leaves, quartic, check);
    fe_neg(check, SQRTM1.as_ptr());
    let ms = fe_isequal(leaves, quartic, check);
    fe_mul(isr, t0, SQRTM1.as_ptr());
    fe_ccopy(isr, t0, sub32(1, m1 | ms));
    wipe(leaves, t0, size_of::<Fe>());
    wipe(leaves, t1, size_of::<Fe>());
    wipe(leaves, t2, size_of::<Fe>());
    p1 | m1 | z0
}
fn fe_invert(leaves: &Leaves, out: *mut i32, x: *const i32) {
    let mut storage = MaybeUninit::<Fe>::uninit();
    let tmp = storage.as_mut_ptr().cast::<i32>();
    fe_sq(tmp, x);
    invsqrt(leaves, tmp, tmp);
    fe_sq(tmp, tmp);
    fe_mul(out, tmp, x);
    wipe(leaves, tmp, size_of::<Fe>());
}
fn trim(out: *mut u8, input: *const u8) {
    // Preserve original FOR copy, including forward-overlap propagation.
    for i in 0..32 {
        unsafe {
            out.add(i).write(input.add(i).read());
        }
    }
    unsafe {
        out.write(out.read() & 0xf8);
        out.add(31).write((out.add(31).read() & 0x7f) | 0x40);
    }
}
fn scalarmult(leaves: &Leaves, q: *mut u8, scalar: *const u8, p: *const u8, bits: i32) {
    let mut x1_storage = MaybeUninit::<Fe>::uninit();
    let x1 = x1_storage.as_mut_ptr().cast::<i32>();
    let mut x2_storage = MaybeUninit::<Fe>::uninit();
    let x2 = x2_storage.as_mut_ptr().cast::<i32>();
    let mut z2_storage = MaybeUninit::<Fe>::uninit();
    let z2 = z2_storage.as_mut_ptr().cast::<i32>();
    let mut x3_storage = MaybeUninit::<Fe>::uninit();
    let x3 = x3_storage.as_mut_ptr().cast::<i32>();
    let mut z3_storage = MaybeUninit::<Fe>::uninit();
    let z3 = z3_storage.as_mut_ptr().cast::<i32>();
    let mut t0_storage = MaybeUninit::<Fe>::uninit();
    let t0 = t0_storage.as_mut_ptr().cast::<i32>();
    let mut t1_storage = MaybeUninit::<Fe>::uninit();
    let t1 = t1_storage.as_mut_ptr().cast::<i32>();
    fe_frombytes(x1, p);
    fe_1(x2);
    fe_0(z2);
    fe_copy(x3, x1);
    fe_1(z3);
    let mut swap = 0;
    for position in (0..bits).rev() {
        let position = position as usize; // closed caller range: 255/256 iterations
        let b = i32::from((unsafe { scalar.add(position >> 3).read() } >> (position & 7)) & 1);
        swap ^= b;
        fe_cswap(x2, x3, swap);
        fe_cswap(z2, z3, swap);
        swap = b;
        fe_sub(t0, x3, z3);
        fe_sub(t1, x2, z2);
        fe_add(x2, x2, z2);
        fe_add(z2, x3, z3);
        fe_mul(z3, t0, x2);
        fe_mul(z2, z2, t1);
        fe_sq(t0, t1);
        fe_sq(t1, x2);
        fe_add(x3, z3, z2);
        fe_sub(z2, z3, z2);
        fe_mul(x2, t1, t0);
        fe_sub(t1, t1, t0);
        fe_sq(z2, z2);
        fe_mul_small(z3, t1, 121_666);
        fe_sq(x3, x3);
        fe_add(t0, t0, z3);
        fe_mul(z3, x1, z2);
        fe_mul(z2, t1, t0);
    }
    fe_cswap(x2, x3, swap);
    fe_cswap(z2, z3, swap);
    fe_invert(leaves, z2, z2);
    fe_mul(x2, x2, z2);
    fe_tobytes(leaves, q, x2);
    // Exact original order and physical final stack storage; no Fe aggregate move.
    wipe(leaves, x1, size_of::<Fe>());
    wipe(leaves, x2, size_of::<Fe>());
    wipe(leaves, z2, size_of::<Fe>());
    wipe(leaves, t0, size_of::<Fe>());
    wipe(leaves, x3, size_of::<Fe>());
    wipe(leaves, z3, size_of::<Fe>());
    wipe(leaves, t1, size_of::<Fe>());
}
fn x25519(leaves: &Leaves, out: *mut u8, secret: *const u8, point: *const u8) {
    let mut storage = MaybeUninit::<[u8; 32]>::uninit();
    let e = storage.as_mut_ptr().cast::<u8>();
    trim(e, secret);
    scalarmult(leaves, out, e, point, 255);
    wipe(leaves, e, 32);
}
// SAFETY for exports: original fixed 32-byte readable/writable buffer extents;
// immutable initialized leaf table is borrowed synchronously, only original
// nonthrowing wipe/verify32 callbacks. Output may overlap inputs, including
// partial overlap: scalar/point reads finish before ladder serialization.
// Trim separately preserves original forward-copy overlap behavior. Coarse
// ladder callers supply 255 or 256 bits. No field/layout crosses FFI, no pointers
// survive calls, no external buffer becomes overlapping Rust references.
// No heap, global registration or C++ unwinding; Rust panics abort.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_x25519_trim(out: *mut u8, input: *const u8) {
    trim(out, input);
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_x25519_ladder(
    leaves: *const Leaves,
    out: *mut u8,
    scalar: *const u8,
    point: *const u8,
    bits: i32,
) {
    scalarmult(unsafe { &*leaves }, out, scalar, point, bits);
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_x25519(
    leaves: *const Leaves,
    out: *mut u8,
    secret: *const u8,
    point: *const u8,
) {
    x25519(unsafe { &*leaves }, out, secret, point);
}
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn openttd_rust_x25519_public_key(
    leaves: *const Leaves,
    out: *mut u8,
    secret: *const u8,
) {
    x25519(unsafe { &*leaves }, out, secret, BASE.as_ptr());
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDX25519Leaves" ["probe"] => Leaves {
        wipe,
        verify32,
    };
}
