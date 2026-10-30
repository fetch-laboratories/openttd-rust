/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `heightmap.cpp` input policy: format dispatch, limits, palettes and the greyscale buffer.
//! libpng (with its setjmp frame), the BMP codec and file objects stay native; each
//! decode is one call whose output stays valid until `finish`.
#![allow(unsafe_code)]
use crate::fileio_type::{DFT_HEIGHTMAP_BMP, DFT_HEIGHTMAP_PNG};
use crate::gfx_type::Colour;
use crate::map_type::MAX_MAP_SIZE;
use std::ffi::c_void;

/// `HM_COUNTER_CLOCKWISE`.
pub(crate) const HM_COUNTER_CLOCKWISE: u8 = 0;
/// `HM_CLOCKWISE`.
pub(crate) const HM_CLOCKWISE: u8 = 1;

/// `MAX_HEIGHTMAP_SIDE_LENGTH_IN_PIXELS`.
pub(crate) const MAX_HEIGHTMAP_SIDE_LENGTH_IN_PIXELS: u32 = 2 * MAX_MAP_SIZE;
/// `MAX_HEIGHTMAP_SIZE_PIXELS`.
const MAX_HEIGHTMAP_SIZE_PIXELS: u32 = 256 << 20;
const _: () = assert!(MAX_HEIGHTMAP_SIZE_PIXELS < u32::MAX / 8);

/// Native decode succeeded.
pub const DECODE_OK: u8 = 0;
/// `FioFOpenFile` found no file; any other status is a libpng or `BmpReadHeader` failure.
pub const DECODE_NOT_FOUND: u8 = 1;

#[repr(C)]
pub struct Strings {
    pub png: u32,
    pub bmp: u32,
    pub not_found: u32,
    pub png_misc: u32,
    pub png_image_type: u32,
    pub bmp_image_type: u32,
    pub too_large: u32,
}

/// libpng results after `png_read_png`; rows and RGB palette triples live until `finish`.
#[repr(C)]
pub struct Png {
    pub width: u32,
    pub height: u32,
    pub rows: *const *const u8,
    pub palette: *const u8,
    pub palette_size: i32,
    pub channels: u8,
    pub bit_depth: u8,
    pub has_palette: bool,
}

/// `BmpInfo`/`BmpData` fields; palette and bitmap live until `finish`.
#[repr(C)]
pub struct Bmp {
    pub width: u32,
    pub height: u32,
    pub palette_size: u32,
    pub palette: *const Colour,
    pub palette_len: usize,
    pub bitmap: *const u8,
    pub bpp: u16,
}

/// One synchronous read's native decoder. `source` fixes the file for the read.
#[repr(C)]
pub struct Input {
    pub context: *mut c_void,
    pub strings: Strings,
    pub source: extern "C" fn(*mut c_void) -> u8,
    pub png: extern "C" fn(*mut c_void, *mut Png) -> u8,
    pub bmp_header: extern "C" fn(*mut c_void, *mut Bmp) -> u8,
    pub bmp_bitmap: extern "C" fn(*mut c_void, *mut Bmp) -> bool,
    pub finish: extern "C" fn(*mut c_void),
    pub error: extern "C" fn(u32, u32),
    pub not_reached: extern "C" fn(),
}

/// A decoded image: dimensions and, when requested, one greyscale byte per pixel.
pub(crate) struct Image {
    pub width: u32,
    pub height: u32,
    pub map: Vec<u8>,
}

/// `IsValidHeightmapDimension`.
fn is_valid_heightmap_dimension(width: u32, height: u32) -> bool {
    u64::from(width) * u64::from(height) <= u64::from(MAX_HEIGHTMAP_SIZE_PIXELS)
        && width > 0
        && width <= MAX_HEIGHTMAP_SIDE_LENGTH_IN_PIXELS
        && height > 0
        && height <= MAX_HEIGHTMAP_SIDE_LENGTH_IN_PIXELS
}

/// `RGBToGreyscale`.
#[allow(clippy::cast_possible_truncation)]
fn rgb_to_greyscale(red: u8, green: u8, blue: u8) -> u8 {
    ((u32::from(red) * 19595 + u32::from(green) * 38470 + u32::from(blue) * 7471) / 65536) as u8
}

/// Grey levels for a 16-entry non-grey palette: the entry order is the height.
#[allow(clippy::cast_possible_truncation)]
fn ordered_palette(gray: &mut [u8; 256], size: usize) {
    for (i, level) in gray.iter_mut().enumerate().take(size) {
        *level = (256 * i / size) as u8;
    }
}

impl Input {
    fn fail(&self, summary: u32, detail: u32) -> Option<Image> {
        (self.error)(summary, detail);
        (self.finish)(self.context);
        None
    }

    /// `ReadHeightmapPNG` and `ReadHeightmapPNGImageData`.
    fn read_png(&self, want_map: bool) -> Option<Image> {
        let s = &self.strings;
        let mut png = Png {
            width: 0,
            height: 0,
            rows: std::ptr::null(),
            palette: std::ptr::null(),
            palette_size: 0,
            channels: 0,
            bit_depth: 0,
            has_palette: false,
        };
        match (self.png)(self.context, &raw mut png) {
            DECODE_OK => {}
            DECODE_NOT_FOUND => return self.fail(s.png, s.not_found),
            _ => return self.fail(s.png, s.png_misc),
        }
        // Unreachable after the strip transforms, kept with the original's && chain.
        if png.channels != 1 && png.channels != 3 && png.bit_depth != 8 {
            return self.fail(s.png, s.png_image_type);
        }
        if !is_valid_heightmap_dimension(png.width, png.height) {
            return self.fail(s.png, s.too_large);
        }
        let mut map = Vec::new();
        if want_map {
            map.resize(png.width as usize * png.height as usize, 0);
            // SAFETY: libpng rows and palette stay alive until finish below.
            unsafe { png_image_data(&mut map, &png) };
        }
        (self.finish)(self.context);
        Some(Image {
            width: png.width,
            height: png.height,
            map,
        })
    }

    /// `ReadHeightmapBMP` and `ReadHeightmapBMPImageData`.
    fn read_bmp(&self, want_map: bool) -> Option<Image> {
        let s = &self.strings;
        let mut bmp = Bmp {
            width: 0,
            height: 0,
            palette_size: 0,
            palette: std::ptr::null(),
            palette_len: 0,
            bitmap: std::ptr::null(),
            bpp: 0,
        };
        match (self.bmp_header)(self.context, &raw mut bmp) {
            DECODE_OK => {}
            DECODE_NOT_FOUND => {
                (self.error)(s.bmp, s.not_found);
                return None;
            }
            _ => return self.fail(s.bmp, s.bmp_image_type),
        }
        if !is_valid_heightmap_dimension(bmp.width, bmp.height) {
            return self.fail(s.bmp, s.too_large);
        }
        let mut map = Vec::new();
        if want_map {
            if !(self.bmp_bitmap)(self.context, &raw mut bmp) {
                return self.fail(s.bmp, s.bmp_image_type);
            }
            map.resize(bmp.width as usize * bmp.height as usize, 0);
            // SAFETY: BmpData stays alive until finish below.
            unsafe { bmp_image_data(&mut map, &bmp) };
        }
        (self.finish)(self.context);
        Some(Image {
            width: bmp.width,
            height: bmp.height,
            map,
        })
    }

    /// `ReadHeightMap`: the image dimensions and, if `want_map`, its greyscale pixels.
    pub(crate) fn read(&self, want_map: bool) -> Option<Image> {
        match (self.source)(self.context) {
            DFT_HEIGHTMAP_PNG => self.read_png(want_map),
            DFT_HEIGHTMAP_BMP => self.read_bmp(want_map),
            _ => {
                (self.not_reached)();
                None
            }
        }
    }
}

/// # Safety
/// `png` holds libpng's live rows and palette for its dimensions and channels.
#[allow(clippy::cast_sign_loss)]
unsafe fn png_image_data(map: &mut [u8], png: &Png) {
    let mut gray = [0_u8; 256];
    let channels = u32::from(png.channels);
    if png.has_palette {
        let size = png.palette_size;
        let mut all_gray = true;
        let mut i = 0;
        while i < size && (size != 16 || all_gray) {
            // SAFETY: libpng's palette holds `size` RGB triples of at most 256.
            let [r, g, b] = unsafe { *png.palette.add(3 * i as usize).cast::<[u8; 3]>() };
            all_gray &= r == g && r == b;
            gray[i as usize] = rgb_to_greyscale(r, g, b);
            i += 1;
        }
        if size == 16 && !all_gray {
            ordered_palette(&mut gray, 16);
        }
    }
    for x in 0..png.width {
        for y in 0..png.height {
            // SAFETY: one row per image line, each `width * channels` bytes.
            let row = unsafe { *png.rows.add(y as usize) };
            let x_offset = (x * channels) as usize;
            let at = |i: usize| unsafe { *row.add(x_offset + i) };
            map[(y * png.width + x) as usize] = if png.has_palette {
                gray[usize::from(at(0))]
            } else if channels == 3 {
                rgb_to_greyscale(at(0), at(1), at(2))
            } else {
                at(0)
            };
        }
    }
}

/// # Safety
/// `bmp` holds the decoded palette and bitmap for its dimensions and depth.
unsafe fn bmp_image_data(map: &mut [u8], bmp: &Bmp) {
    let mut gray = [0_u8; 256];
    if bmp.palette_len != 0 {
        if bmp.palette_size == 2 {
            // Two entries: sea (level 0), then land (level 1).
            gray[0] = 0;
            gray[1] = 16;
        } else {
            let size = bmp.palette_size as usize;
            let mut all_gray = true;
            let mut i = 0;
            while i < size && (size != 16 || all_gray) {
                // SAFETY: BmpReadHeader sized the palette to palette_size <= 256.
                let c = unsafe { *bmp.palette.add(i) };
                all_gray &= c.r == c.g && c.r == c.b;
                gray[i] = rgb_to_greyscale(c.r, c.g, c.b);
                i += 1;
            }
            if size == 16 && !all_gray {
                ordered_palette(&mut gray, 16);
            }
        }
    }
    let width = bmp.width as usize;
    let step = if bmp.bpp == 24 { 3 } else { 1 };
    for y in 0..bmp.height as usize {
        let pixels = &mut map[y * width..(y + 1) * width];
        // SAFETY: BmpReadBitmap stores `width * step` bytes per line.
        let bitmap =
            unsafe { std::slice::from_raw_parts(bmp.bitmap.add(y * width * step), width * step) };
        for (pixel, source) in pixels.iter_mut().zip(bitmap.chunks_exact(step)) {
            *pixel = if bmp.bpp == 24 {
                rgb_to_greyscale(source[0], source[1], source[2])
            } else {
                gray[usize::from(source[0])]
            };
        }
    }
}

/// `GetHeightmapDimensions`: writes the dimensions only on success.
/// # Safety
/// A live native input for one synchronous read and writable outputs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_heightmap_dimensions(
    input: *const Input,
    x: *mut u32,
    y: *mut u32,
) -> bool {
    let Some(image) = (unsafe { &*input }).read(false) else {
        return false;
    };
    unsafe {
        *x = image.width;
        *y = image.height;
    }
    true
}

crate::abi::layouts! {
    fn abi_layout;
    "Colour" ["landscape"] => crate::gfx_type::Colour { r, g, b, a, };
    "OpenTTDHeightmapStrings" ["landscape"] => Strings { png, bmp, not_found, png_misc, png_image_type, bmp_image_type, too_large, };
    "OpenTTDHeightmapPng" ["landscape"] => Png { width, height, rows, palette, palette_size, channels, bit_depth, has_palette, };
    "OpenTTDHeightmapBmp" ["landscape"] => Bmp { width, height, palette_size, palette, palette_len, bitmap, bpp, };
    "OpenTTDHeightmapInput" ["landscape"] => Input { context, strings, source, png, bmp_header, bmp_bitmap, finish, error, not_reached, };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_limits_and_greyscale_follow_the_original() {
        assert!(is_valid_heightmap_dimension(1, 1));
        assert!(is_valid_heightmap_dimension(8192, 8192));
        assert!(!is_valid_heightmap_dimension(8193, 1));
        assert!(!is_valid_heightmap_dimension(0, 5));
        assert!(!is_valid_heightmap_dimension(5, 0));
        assert!(!is_valid_heightmap_dimension(1, 8193));
        for (rgb, grey) in [
            ([0, 0, 0], 0),
            ([255, 255, 255], 255),
            ([255, 0, 0], 76),
            ([0, 255, 0], 149),
            ([0, 0, 255], 29),
        ] {
            assert_eq!(rgb_to_greyscale(rgb[0], rgb[1], rgb[2]), grey);
        }
    }
}
