/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `TerraGenesis` Perlin generator. Owns the temporary height map and every stage.
//!
//! Progress can abort in C++. The shared cancellation handoff catches that signal,
//! runs the registered cleanup and returns before the native caller rethrows.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::manual_midpoint // Preserve original operation order and truncation.
)]

use crate::genworld::GWP_LANDSCAPE;
use crate::landscape_generation::{Frame, World};

#[repr(C)]
#[derive(Clone, Copy)]
#[allow(missing_docs)]
pub struct Input {
    pub log_x: u32,
    pub log_y: u32,
    pub terrain: u32,
    pub custom_height: u32,
    pub height_limit: u32,
    pub smoothness: u32,
    pub landscape: u32,
    pub variety: u32,
    pub sea: u32,
    pub custom_sea: u32,
    pub freeform: u32,
    pub borders: u32,
    pub seed: u32,
    pub creation_log_x: u32,
    pub creation_log_y: u32,
}

const ONE: i16 = 16;
const PI_2: f64 = std::f64::consts::FRAC_PI_2;

struct Settings {
    log_x: u32,
    log_y: u32,
    terrain: usize,
    custom_height: u32,
    height_limit: u32,
    smoothness: usize,
    landscape: u32,
    variety: u32,
    sea: usize,
    custom_sea: u32,
    freeform: bool,
    borders: u32,
    seed: u32,
    creation_log_x: u32,
    creation_log_y: u32,
}
impl Settings {
    fn from_values(v: Input) -> Self {
        Self {
            log_x: v.log_x,
            log_y: v.log_y,
            terrain: v.terrain as usize,
            custom_height: v.custom_height,
            height_limit: v.height_limit,
            smoothness: v.smoothness as usize,
            landscape: v.landscape,
            variety: v.variety,
            sea: v.sea as usize,
            custom_sea: v.custom_sea,
            freeform: v.freeform != 0,
            borders: v.borders,
            seed: v.seed,
            creation_log_x: v.creation_log_x,
            creation_log_y: v.creation_log_y,
        }
    }
    fn max_height(&self) -> i16 {
        const HEIGHTS: [[u32; 7]; 5] = [
            [3, 3, 3, 3, 4, 5, 7],
            [5, 7, 8, 9, 14, 19, 31],
            [8, 9, 10, 15, 23, 37, 61],
            [10, 11, 17, 19, 49, 63, 73],
            [12, 19, 25, 31, 67, 75, 87],
        ];
        if self.terrain == 5 {
            return (((self.custom_height + 1) << 4) - 1) as i16;
        }
        let mut height = HEIGHTS[self.terrain][(self.log_x.min(self.log_y) - 6) as usize];
        if self.height_limit != 0 {
            height = height.min(self.height_limit);
        }
        (height << 4) as i16
    }
    fn amplitude(&self, frequency: i32) -> i32 {
        const AMPLITUDES: [[i32; 7]; 4] = [
            [16000, 5600, 1968, 688, 240, 16, 16],
            [24000, 12800, 6400, 2700, 1024, 128, 16],
            [32000, 19200, 12800, 8000, 3200, 256, 64],
            [48000, 24000, 19200, 16000, 8000, 512, 320],
        ];
        let mut index = frequency - 3;
        let mut amplitude = AMPLITUDES[self.smoothness][index.max(0) as usize];
        if index >= 0 {
            return amplitude;
        }
        let factor = [3.3, 2.8, 2.3, 1.8][self.smoothness];
        let mut height_range = 256;
        while index < 0 {
            amplitude = (factor * f64::from(amplitude)) as i32;
            height_range <<= 1;
            index += 1;
        }
        ((i32::from(self.max_height()) - height_range) / height_range).clamp(0, 1) * amplitude
    }
}

/// Private owner; no C++ view into its vectors is retained.
pub struct Generator {
    settings: Settings,
    heights: Vec<i16>,
    sx: i32,
    sy: i32,
    random: extern "C" fn(*mut std::ffi::c_void) -> u32,
    random_context: *mut std::ffi::c_void,
}
impl Generator {
    fn random(&self) -> u32 {
        (self.random)(self.random_context)
    }
    fn range(&self, limit: u32) -> u32 {
        crate::random_func::scale_to_limit(self.random(), limit)
    }

    fn index(&self, x: i32, y: i32) -> usize {
        (x + y * (self.sx + 1)) as usize
    }
    fn get(&self, x: i32, y: i32) -> i16 {
        self.heights[self.index(x, y)]
    }
    fn set(&mut self, x: i32, y: i32, h: i16) {
        let i = self.index(x, y);
        self.heights[i] = h;
    }
    fn valid(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.sx && y < self.sy
    }
    fn random_height(&self, amplitude: i32) -> i16 {
        // RandomRange's unsigned subtraction converts back to int before A2H.
        ((self
            .range((2 * amplitude + 1) as u32)
            .wrapping_sub(amplitude as u32) as i32)
            >> 6) as i16
    }
    fn generate(&mut self) {
        let start = (10 - self.settings.log_x.min(self.settings.log_y) as i32).max(0);
        let mut first = true;
        for frequency in start..10 {
            let amplitude = self.settings.amplitude(frequency);
            if amplitude == 0 {
                continue;
            }
            let step = 1 << (9 - frequency);
            if first {
                for y in (0..=self.sy).step_by(step as usize) {
                    for x in (0..=self.sx).step_by(step as usize) {
                        let h = if amplitude > 0 {
                            self.random_height(amplitude)
                        } else {
                            0
                        };
                        self.set(x, y, h);
                    }
                }
                first = false;
                continue;
            }
            for y in (0..=self.sy).step_by((2 * step) as usize) {
                for x in (0..=self.sx - 2 * step).step_by((2 * step) as usize) {
                    self.set(
                        x + step,
                        y,
                        ((i32::from(self.get(x, y)) + i32::from(self.get(x + 2 * step, y))) / 2)
                            as i16,
                    );
                }
            }
            for y in (0..=self.sy - 2 * step).step_by((2 * step) as usize) {
                for x in (0..=self.sx).step_by(step as usize) {
                    self.set(
                        x,
                        y + step,
                        ((i32::from(self.get(x, y)) + i32::from(self.get(x, y + 2 * step))) / 2)
                            as i16,
                    );
                }
            }
            for y in (0..=self.sy).step_by(step as usize) {
                for x in (0..=self.sx).step_by(step as usize) {
                    self.set(
                        x,
                        y,
                        self.get(x, y).wrapping_add(self.random_height(amplitude)),
                    );
                }
            }
        }
    }
    fn adjust_water(&mut self, water_percent: i64, max_new: i16) {
        let min = *self.heights.iter().min().unwrap();
        let max = *self.heights.iter().max().unwrap();
        let mut hist = vec![0i32; (i32::from(max) - i32::from(min) + 1) as usize];
        for &h in &self.heights {
            hist[(i32::from(h) - i32::from(min)) as usize] += 1;
        }
        let desired = water_percent * i64::from(self.sx) * i64::from(self.sy) / 1024;
        let mut water = min;
        let mut count = 0i64;
        while water < max {
            count += i64::from(hist[(i32::from(water) - i32::from(min)) as usize]);
            if count >= desired {
                break;
            }
            water += 1;
        }
        for h in &mut self.heights {
            *h = ((i32::from(max_new) * (i32::from(*h) - i32::from(water))
                / (i32::from(max) - i32::from(water))) as i16)
                .wrapping_add(ONE);
            if *h < 0 {
                *h = 0;
            }
            if *h >= max_new {
                *h = max_new - 1;
            }
        }
    }
    fn sine_transform(&mut self, min: i16, max: i16) {
        for h in &mut self.heights {
            if *h < min {
                continue;
            }
            let mut f = f64::from(*h - min) / f64::from(max - min);
            match self.settings.landscape {
                0 | 3 => {
                    f = 2.0 * f - 1.0;
                    f = (f * PI_2).sin();
                    f = 0.5 * (f + 1.0);
                }
                1 => {
                    if f >= 0.75 {
                        f = 1.0 - (1.0 - f) / 2.0;
                    } else {
                        let m = 1.0 - (1.0 - 0.75) / 2.0;
                        f = 2.0 * f / 0.75 - 1.0;
                        f = (f * PI_2).sin();
                        f = 0.5 * (f + 1.0) * m;
                    }
                }
                2 => {
                    if f <= 0.5 {
                        f /= 2.0;
                    } else {
                        let m = 0.5 / 2.0;
                        f = 2.0 * ((f - 0.5) / (1.0 - 0.5)) - 1.0;
                        f = (f * PI_2).sin();
                        f = 0.5 * ((1.0 - m) * f + (1.0 + m));
                    }
                }
                _ => unreachable!("landscape enumeration"),
            }
            *h = (f * f64::from(max - min) + f64::from(min)) as i16;
            if *h < 0 {
                *h = 0;
            }
            if *h >= max {
                *h = max - 1;
            }
        }
    }
    fn curves(&mut self, level: u32) {
        let mh = self.settings.max_height() - ONE;
        let f = |fraction: f64| (fraction * f64::from(mh)) as i16;
        let curves: [&[(i16, i16)]; 4] = [
            &[(f(0.0), f(0.0)), (f(0.8), f(0.13)), (f(1.0), f(0.4))],
            &[
                (f(0.0), f(0.0)),
                (f(0.53), f(0.13)),
                (f(0.8), f(0.27)),
                (f(1.0), f(0.6)),
            ],
            &[
                (f(0.0), f(0.0)),
                (f(0.53), f(0.27)),
                (f(0.8), f(0.57)),
                (f(1.0), f(0.8)),
            ],
            &[
                (f(0.0), f(0.0)),
                (f(0.4), f(0.3)),
                (f(0.7), f(0.8)),
                (f(0.92), f(0.99)),
                (f(1.0), f(0.99)),
            ],
        ];
        let mut ht = [0i16; 4];
        // C++ sqrt receives a float expression, promotes it to double and narrows.
        let factor = f64::from(self.sx as f32 / self.sy as f32).sqrt() as f32;
        let sx = (f64::from((1u32 << level) as f32 * factor) + 0.5) as i32;
        let sy = (f64::from((1u32 << level) as f32 / factor) + 0.5) as i32;
        let sx = sx.clamp(1, 128) as u32;
        let sy = sy.clamp(1, 128) as u32;
        let mut c = vec![0u8; (sx * sy) as usize];
        for value in &mut c {
            *value = self.range(4) as u8;
        }
        for x in 0..self.sx {
            let fx = (sx * x as u32) as f32 / self.sx as f32 + 1.0;
            let mut x1 = fx as u32;
            let mut x2 = x1;
            let mut xr = 2.0 * (fx - x1 as f32) - 1.0;
            xr = (f64::from(xr) * PI_2).sin() as f32;
            xr = (f64::from(xr) * PI_2).sin() as f32;
            xr = 0.5 * (xr + 1.0);
            let xri = 1.0 - xr;
            if x1 > 0 {
                x1 -= 1;
                if x2 >= sx {
                    x2 -= 1;
                }
            }
            for y in 0..self.sy {
                let fy = (sy * y as u32) as f32 / self.sy as f32 + 1.0;
                let mut y1 = fy as u32;
                let mut y2 = y1;
                let mut yr = 2.0 * (fy - y1 as f32) - 1.0;
                yr = (f64::from(yr) * PI_2).sin() as f32;
                yr = (f64::from(yr) * PI_2).sin() as f32;
                yr = 0.5 * (yr + 1.0);
                let yri = 1.0 - yr;
                if y1 > 0 {
                    y1 -= 1;
                    if y2 >= sy {
                        y2 -= 1;
                    }
                }
                let a = usize::from(c[(x1 + sx * y1) as usize]);
                let b = usize::from(c[(x1 + sx * y2) as usize]);
                let cc = usize::from(c[(x2 + sx * y1) as usize]);
                let d = usize::from(c[(x2 + sx * y2) as usize]);
                let bits = (1 << a) | (1 << b) | (1 << cc) | (1 << d);
                let mut h = self.get(x, y);
                if h < ONE {
                    continue;
                }
                h -= ONE;
                for t in 0..4 {
                    if bits & (1 << t) == 0 {
                        continue;
                    }
                    for pair in curves[t].windows(2) {
                        let (x1, y1) = pair[0];
                        let (x2, y2) = pair[1];
                        if h >= x1 && h < x2 {
                            ht[t] = (i32::from(y1)
                                + (i32::from(h) - i32::from(x1)) * (i32::from(y2) - i32::from(y1))
                                    / (i32::from(x2) - i32::from(x1)))
                                as i16;
                            break;
                        }
                    }
                }
                h = ((f32::from(ht[a]) * yri + f32::from(ht[b]) * yr) * xri
                    + (f32::from(ht[cc]) * yri + f32::from(ht[d]) * yr) * xr)
                    as i16;
                self.set(x, y, h.wrapping_add(ONE));
            }
        }
    }
    fn coast_noise(&self, x: f64, y: f64, p: f64, prime: i32) -> f64 {
        let mut total = 0.0;
        for i in 0..6 {
            let frequency = f64::from(1 << i);
            // powf uses the platform pow, as does C++ pow(p, double(i)).
            let amplitude = p.powf(f64::from(i));
            total += interpolated_noise(
                x * frequency / 64.0,
                y * frequency / 64.0,
                prime,
                self.settings.seed,
            ) * amplitude;
        }
        total
    }
    fn coast_lines(&mut self, borders: u32) {
        let smallest = self
            .settings
            .creation_log_x
            .min(self.settings.creation_log_y) as i32;
        let base = f64::from(smallest * smallest / 64);
        for y in 0..=self.sy {
            if borders & 1 != 0 {
                let mut max = ((self.coast_noise(f64::from(self.sy - y), f64::from(y), 0.9, 53)
                    + 0.25)
                    * 5.0
                    + (self.coast_noise(f64::from(y), f64::from(y), 0.35, 179) + 1.0) * 12.0)
                    .abs();
                max = (base + max).max(base + 4.0 - max);
                if smallest < 8 && max > 5.0 {
                    max /= 1.5;
                }
                let mut x = 0;
                while f64::from(x) < max {
                    self.set(x, y, 0);
                    x += 1;
                }
            }
            if borders & 4 != 0 {
                let mut max = ((self.coast_noise(f64::from(self.sy - y), f64::from(y), 0.85, 101)
                    + 0.3)
                    * 6.0
                    + (self.coast_noise(f64::from(y), f64::from(y), 0.45, 67) + 0.75) * 8.0)
                    .abs();
                max = (base + max).max(base + 4.0 - max);
                if smallest < 8 && max > 5.0 {
                    max /= 1.5;
                }
                let mut x = self.sx;
                while f64::from(x) > f64::from(self.sx - 1) - max {
                    self.set(x, y, 0);
                    x -= 1;
                }
            }
        }
        for x in 0..=self.sx {
            if borders & 8 != 0 {
                let mut max = ((self.coast_noise(f64::from(x), f64::from(self.sy / 2), 0.9, 167)
                    + 0.4)
                    * 5.0
                    + (self.coast_noise(f64::from(x), f64::from(self.sy / 3), 0.4, 211) + 0.7)
                        * 9.0)
                    .abs();
                max = (base + max).max(base + 4.0 - max);
                if smallest < 8 && max > 5.0 {
                    max /= 1.5;
                }
                let mut y = 0;
                while f64::from(y) < max {
                    self.set(x, y, 0);
                    y += 1;
                }
            }
            if borders & 2 != 0 {
                let mut max = ((self.coast_noise(f64::from(x), f64::from(self.sy / 3), 0.85, 71)
                    + 0.25)
                    * 6.0
                    + (self.coast_noise(f64::from(x), f64::from(self.sy / 3), 0.35, 193) + 0.75)
                        * 12.0)
                    .abs();
                max = (base + max).max(base + 4.0 - max);
                if smallest < 8 && max > 5.0 {
                    max /= 1.5;
                }
                let mut y = self.sy;
                while f64::from(y) > f64::from(self.sy - 1) - max {
                    self.set(x, y, 0);
                    y -= 1;
                }
            }
        }
    }
    fn smooth_direction(&mut self, mut x: i32, mut y: i32, dx: i32, dy: i32) {
        for _ in 0..35 {
            if !self.valid(x, y) {
                break;
            }
            if self.get(x, y) >= ONE {
                break;
            }
            if self.valid(x + dy, y + dx) && self.get(x + dy, y + dx) > 0 {
                break;
            }
            if self.valid(x - dy, y - dx) && self.get(x - dy, y - dx) > 0 {
                break;
            }
            x += dx;
            y += dy;
        }
        let mut prev = ONE;
        for depth in 0..=35 {
            if !self.valid(x, y) {
                break;
            }
            let h = ((i32::from(self.get(x, y)) as u32).min((i32::from(prev) + 4 + depth) as u32))
                as i16;
            self.set(x, y, h);
            prev = h;
            x += dx;
            y += dy;
        }
    }
    fn smooth_coasts(&mut self, borders: u32) {
        for x in 0..self.sx {
            if borders & 8 != 0 {
                self.smooth_direction(x, 0, 0, 1);
            }
            if borders & 2 != 0 {
                self.smooth_direction(x, self.sy - 1, 0, -1);
            }
        }
        for y in 0..self.sy {
            if borders & 1 != 0 {
                self.smooth_direction(0, y, 1, 0);
            }
            if borders & 4 != 0 {
                self.smooth_direction(self.sx - 1, y, -1, 0);
            }
        }
    }
    fn smooth_slopes(&mut self, delta: i16) {
        for y in 0..=self.sy {
            for x in 0..=self.sx {
                let max = self
                    .get((x - 1).max(0), y)
                    .min(self.get(x, (y - 1).max(0)))
                    .wrapping_add(delta);
                if self.get(x, y) > max {
                    self.set(x, y, max);
                }
            }
        }
        for y in (0..=self.sy).rev() {
            for x in (0..=self.sx).rev() {
                let max = self
                    .get((x + 1).min(self.sx), y)
                    .min(self.get(x, (y + 1).min(self.sy)))
                    .wrapping_add(delta);
                if self.get(x, y) > max {
                    self.set(x, y, max);
                }
            }
        }
    }
    fn normalize(&mut self) {
        let percent = if self.settings.sea == 4 {
            i64::from(self.settings.custom_sea) * 1024 / 100
        } else {
            [70, 170, 270, 420][self.settings.sea]
        };
        let max = self.settings.max_height();
        let roughness = (7 + 3 * self.settings.smoothness) as i16;
        self.adjust_water(percent, max);
        let mut borders = if self.settings.freeform {
            self.settings.borders
        } else {
            15
        };
        if borders == 16 {
            borders = self.random() & 15;
        }
        self.coast_lines(borders);
        self.smooth_slopes(roughness);
        self.smooth_coasts(borders);
        self.smooth_slopes(roughness);
        self.sine_transform(ONE, max);
        if self.settings.variety > 0 {
            self.curves(self.settings.variety);
        }
    }
}

fn int_noise(x: i32, y: i32, prime: i32, seed: u32) -> f64 {
    // C++ long differs between LP64 and Windows. Only the low 31 result bits
    // are observed; modular polynomial arithmetic gives identical bits in both.
    let mut n = (x as u32)
        .wrapping_add((y as u32).wrapping_mul(prime as u32))
        .wrapping_add(seed);
    n = n.wrapping_shl(13) ^ n;
    let bits = n
        .wrapping_mul(n.wrapping_mul(n).wrapping_mul(15731).wrapping_add(789_221))
        .wrapping_add(1_376_312_589)
        & 0x7fff_ffff;
    1.0 - f64::from(bits) / 1_073_741_824.0
}
fn interpolate(a: f64, b: f64, x: f64) -> f64 {
    a + x * (b - a)
}
fn interpolated_noise(x: f64, y: f64, prime: i32, seed: u32) -> f64 {
    let ix = x as i32;
    let iy = y as i32;
    let fx = x - f64::from(ix);
    let fy = y - f64::from(iy);
    let v1 = int_noise(ix, iy, prime, seed);
    let v2 = int_noise(ix + 1, iy, prime, seed);
    let v3 = int_noise(ix, iy + 1, prime, seed);
    let v4 = int_noise(ix + 1, iy + 1, prime, seed);
    interpolate(interpolate(v1, v2, fx), interpolate(v3, v4, fx), fy)
}

/// Return the generator's height estimate without allocating.
/// # Safety
/// The named settings are initialized with original game settings and dimensions.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_tgp_estimate(input: *const Input) -> u32 {
    let settings = Settings::from_values(unsafe { input.read_unaligned() });
    (settings.max_height() >> 4) as u32
}

// The original height map and abort hook have process scope. No reference to this
// allocation lives across progress, which may invoke abort and destroy it.
#[allow(unsafe_code)]
static mut OWNER: *mut Generator = std::ptr::null_mut();
#[allow(unsafe_code)]
extern "C" fn abort() {
    let owner = unsafe { std::ptr::replace(&raw mut OWNER, std::ptr::null_mut()) };
    if !owner.is_null() {
        drop(unsafe { Box::from_raw(owner) });
    }
}
/// Complete native facade loops beside the generator, without an output copy.
/// Each generator borrow ends before the progress/cancellation callback.
#[allow(unsafe_code)]
pub(crate) fn run(world: &World<'_>, frame: *mut Frame, input: Input) -> bool {
    let settings = Settings::from_values(input);
    let sx = 1_i32 << settings.log_x;
    let sy = 1_i32 << settings.log_y;
    let generator = Box::new(Generator {
        settings,
        heights: vec![0; ((sx + 1) * (sy + 1)) as usize],
        sx,
        sy,
        random: world.s.random,
        random_context: world.s.context,
    });
    let owner = Box::into_raw(generator);
    unsafe {
        OWNER = owner;
    }
    (world.a.set_abort_callback)(Some(abort));
    unsafe {
        (*owner).generate();
    }
    if !(world.a.increase_progress)(frame, GWP_LANDSCAPE) {
        crate::witness::hit(crate::witness::GENERATION_TGP_ABORT);
        return false;
    }
    unsafe {
        (*owner).normalize();
    }
    if !(world.a.increase_progress)(frame, GWP_LANDSCAPE) {
        crate::witness::hit(crate::witness::GENERATION_TGP_ABORT);
        return false;
    }
    if input.freeform != 0 {
        world.void_top_edges();
    }
    for y in 0..sy {
        for x in 0..sx {
            let height = {
                let g = unsafe { &*owner };
                let max = i32::from(g.settings.max_height() >> 4);
                i32::from(g.get(x, y) >> 4).clamp(0, max) as u32
            };
            let tile = world.map.tile_xy(x as u32, y as u32);
            tgen_set_tile_height(&world, tile, height);
        }
    }
    abort();
    (world.a.set_abort_callback)(None);
    true
}

/// `tgp.cpp::TgenSetTileHeight`.
fn tgen_set_tile_height(world: &crate::landscape_generation::World<'_>, tile: u32, height: u32) {
    (world.a.set_height)(tile, height);
    if (world.a.inner_tile)(tile) {
        (world.clear.make_clear)(
            tile,
            crate::services::ClearGroundType(crate::clear_map::CLEAR_GRASS),
            3,
        );
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTGPSettings" ["landscape"] => crate::tgp::Input { log_x,log_y,terrain,custom_height,height_limit,smoothness,landscape,variety,sea,custom_sea,freeform,borders,seed,creation_log_x,creation_log_y, };
}
