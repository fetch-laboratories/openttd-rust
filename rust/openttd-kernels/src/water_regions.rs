/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Map-lifetime water-region cache and ordered graph traversal.
#![allow(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Patch {
    pub x: i32,
    pub y: i32,
    pub label: u8,
}

#[repr(C)]
pub struct Snapshot {
    pub edges: [u16; 4],
    pub labels: [u8; 256],
    pub patches: u8,
    pub aqueducts: u8,
}

#[repr(C)]
pub struct Leaves {
    pub shared: *const crate::services::Services,
    pub follow: unsafe extern "C" fn(u32, u8, *mut u8) -> u32,
    pub aqueduct: unsafe extern "C" fn(u32) -> u32,
    pub debug: unsafe extern "C" fn(u8, i32, i32),
}

/// Current native-owned region allocation and its immutable leaves.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Context {
    pub owner: *mut Regions,
    pub leaves: *const Leaves,
}

#[derive(Default)]
struct Region {
    edges: [u16; 4],
    labels: Option<Box<[u8; 256]>>,
    aqueducts: bool,
    patches: u8,
    valid: bool,
}

pub struct Regions {
    width: u32,
    height: u32,
    data: Vec<Region>,
    scratch: Vec<u32>,
}

const OFFSETS: [(i32, i32); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)];
const INVALID: u32 = u32::MAX;

impl Regions {
    fn index(&self, x: i32, y: i32) -> usize {
        ((self.width / 16)
            .wrapping_mul(y as u32)
            .wrapping_add(x as u32)) as usize
    }

    fn coords(&self, tile: u32) -> (i32, i32) {
        (
            (tile % self.width / 16) as i32,
            (tile / self.width / 16) as i32,
        )
    }

    fn tile(&self, x: i32, y: i32, local: usize) -> u32 {
        ((y as u32)
            .wrapping_mul(16)
            .wrapping_add((local / 16) as u32))
        .wrapping_mul(self.width)
        .wrapping_add((x as u32).wrapping_mul(16))
        .wrapping_add((local % 16) as u32)
    }

    fn local(&self, tile: u32) -> usize {
        (tile % self.width % 16 + 16 * (tile / self.width % 16)) as usize
    }

    fn label(&self, index: usize, local: usize) -> u8 {
        let r = &self.data[index];
        r.labels
            .as_ref()
            .map_or(u8::from(r.patches != 0), |a| a[local])
    }

    fn update(&mut self, x: i32, y: i32, leaves: &Leaves) -> usize {
        // The original GetUpdatedWaterRegion narrows its coordinate arguments.
        let x = i32::from(x as u16);
        let y = i32::from(y as u16);
        let index = self.index(x, y);
        if self.data[index].valid {
            return index;
        }
        // SAFETY: Immutable registry outlives this synchronous rebuild.
        let movement = crate::movement::CachedTable::new(unsafe { &*leaves.shared });
        // SAFETY: All leaves are synchronous, noexcept and cannot reenter cache ownership.
        crate::witness::hit(crate::witness::WATER_REBUILDS);
        unsafe {
            (leaves.debug)(0, x, y);
        }
        self.data[index].aqueducts = false;
        let labels = self.data[index]
            .labels
            .get_or_insert_with(|| Box::new([0; 256]));
        labels.fill(0);
        self.data[index].edges.fill(0);
        let mut current_label = 1_u8;
        let mut highest_label = 0;
        for start in 0..256 {
            self.scratch.clear();
            self.scratch.push(self.tile(x, y, start));
            let mut increase_label = false;
            while let Some(tile) = self.scratch.pop() {
                // SAFETY: See leaf contract above; no live world reference is retained.
                crate::witness::hit(crate::witness::WATER_TRACKS);
                let mut dirs = crate::track_func::track_bits_to_trackdir_bits(
                    movement
                        .get()
                        .water_tracks(tile, crate::direction_type::INVALID_DIAGDIR)
                        as u8,
                );
                if dirs == 0 {
                    continue;
                }
                let local = self.local(tile);
                let r = &mut self.data[index];
                if r.labels.as_ref().unwrap()[local] != 0 {
                    continue;
                }
                r.labels.as_mut().unwrap()[local] = current_label;
                highest_label = current_label;
                increase_label = true;
                while dirs != 0 {
                    let dir = dirs.trailing_zeros() as u8;
                    dirs &= dirs - 1;
                    let mut bridge = 0;
                    // SAFETY: Initialized output is call-local; follower neither borrows nor mutates cache.
                    crate::witness::hit(crate::witness::WATER_FOLLOWS);
                    let next = unsafe { (leaves.follow)(tile, dir, &raw mut bridge) };
                    if next == INVALID {
                        continue;
                    }
                    if self.coords(next) == (x, y) {
                        self.scratch.push(next);
                    } else if bridge == 0 {
                        let side = if next % self.width < tile % self.width {
                            0
                        } else if next / self.width > tile / self.width {
                            1
                        } else if next % self.width > tile % self.width {
                            2
                        } else {
                            3
                        };
                        let bit = if side == 0 || side == 2 {
                            local / 16
                        } else {
                            local % 16
                        };
                        self.data[index].edges[side] |= 1_u16 << bit;
                    } else {
                        self.data[index].aqueducts = true;
                    }
                }
            }
            if increase_label {
                current_label = current_label.wrapping_add(1);
            }
        }
        let r = &mut self.data[index];
        r.patches = highest_label;
        if r.patches == 0 || (r.patches == 1 && r.labels.as_ref().unwrap().iter().all(|&v| v == 1))
        {
            r.labels = None;
        }
        r.valid = true;
        index
    }

    fn patch(&mut self, tile: u32, leaves: &Leaves) -> Patch {
        let (x, y) = self.coords(tile);
        let index = self.update(x, y, leaves);
        Patch {
            x,
            y,
            label: self.label(index, self.local(tile)),
        }
    }

    fn invalidate_one(&mut self, tile: u32, leaves: &Leaves) {
        let (x, y) = self.coords(tile);
        let index = self.index(x, y);
        if !self.data[index].valid {
            // SAFETY: Debug leaf cannot reenter or unwind.
            unsafe {
                (leaves.debug)(1, x, y);
            }
        }
        self.data[index].valid = false;
    }

    fn edge_tile(&self, x: i32, y: i32, side: usize, p: usize) -> u32 {
        let local = match side {
            0 => p * 16,
            1 => 240 + p,
            2 => p * 16 + 15,
            _ => p,
        };
        self.tile(x, y, local)
    }

    fn adjacent(&mut self, patch: Patch, side: usize, leaves: &Leaves) -> Vec<Patch> {
        if patch.label == 0 {
            return Vec::new();
        }
        let current = self.update(patch.x, patch.y, leaves);
        let (dx, dy) = OFFSETS[side];
        let nx = patch.x + dx;
        let ny = patch.y + dy;
        if nx < 0 || ny < 0 || nx >= (self.width / 16) as i32 || ny >= (self.height / 16) as i32 {
            return Vec::new();
        }
        let neighbour = self.update(nx, ny, leaves);
        let opposite = side ^ 2;
        let bits = self.data[current].edges[side] & self.data[neighbour].edges[opposite];
        if bits == 0 {
            return Vec::new();
        }
        if self.data[current].patches == 1 && self.data[neighbour].patches == 1 {
            return vec![Patch {
                x: nx,
                y: ny,
                label: 1,
            }];
        }
        let mut labels = Vec::new();
        for p in 0..16 {
            if bits & (1 << p) == 0 {
                continue;
            }
            let tile = self.edge_tile(patch.x, patch.y, side, p);
            if self.label(current, self.local(tile)) != patch.label {
                continue;
            }
            let tile = self.edge_tile(nx, ny, opposite, p);
            let label = self.label(neighbour, self.local(tile));
            if !labels.contains(&label) {
                labels.push(label);
            }
        }
        labels
            .into_iter()
            .map(|label| Patch {
                x: nx,
                y: ny,
                label,
            })
            .collect()
    }
}

pub struct Visit {
    patch: Patch,
    side: usize,
    pending: std::vec::IntoIter<Patch>,
    aqueduct_tile: usize,
    aqueducts: Option<bool>,
}

impl Visit {
    fn next(&mut self, regions: &mut Regions, leaves: &Leaves, live_patch: Patch) -> Option<Patch> {
        if self.patch.label == 0 {
            return None;
        }
        loop {
            if let Some(patch) = self.pending.next() {
                return Some(patch);
            }
            if self.side == 4 {
                break;
            }
            self.pending = regions.adjacent(live_patch, self.side, leaves).into_iter();
            self.side += 1;
        }
        // The original current_region is a live cache reference. Read its flag
        // after side visitors without forcing another update here.
        let has_aqueducts = *self.aqueducts.get_or_insert_with(|| {
            regions.data[regions.index(
                i32::from(self.patch.x as u16),
                i32::from(self.patch.y as u16),
            )]
            .aqueducts
        });
        if has_aqueducts {
            while self.aqueduct_tile < 256 {
                let tile = regions.tile(
                    i32::from(self.patch.x as u16),
                    i32::from(self.patch.y as u16),
                    self.aqueduct_tile,
                );
                self.aqueduct_tile += 1;
                if regions.patch(tile, leaves) != live_patch {
                    continue;
                }
                // SAFETY: No world or cache storage is borrowed by the query.
                crate::witness::hit(crate::witness::WATER_AQUEDUCTS);
                let other = unsafe { (leaves.aqueduct)(tile) };
                if other != INVALID && regions.coords(tile) != regions.coords(other) {
                    return Some(regions.patch(other, leaves));
                }
            }
        }
        None
    }
}

/// Allocate private cache storage for a newly allocated map. Panics/OOM abort.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_water_new(width: u32, height: u32) -> *mut Regions {
    Box::into_raw(Box::new(Regions {
        width,
        height,
        data: (0..width / 16 * (height / 16))
            .map(|_| Region::default())
            .collect(),
        scratch: Vec::new(),
    }))
}

/// Destroy the cache owner.
/// # Safety
/// Pointer is uniquely owned, live, allocated by `water_new`; destroy once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_water_destroy(owner: *mut Regions) {
    // SAFETY: Unique ownership is transferred back by the facade.
    drop(unsafe { Box::from_raw(owner) });
}

/// Observe a tile's label, rebuilding invalid cache at its original use point.
/// # Safety
/// Owner is exclusively accessible for this call. Leaves are live synchronous
/// noexcept/nonreentrant queries. Tile obeys the original valid-map precondition.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_water_label(
    owner: *mut Regions,
    leaves: *const Leaves,
    tile: u32,
) -> u8 {
    // SAFETY: Facade supplies exclusive owner and immutable call-local leaves.
    unsafe { (&mut *owner).patch(tile, &*leaves).label }
}

/// Invalidate a tile's region and adjacent regions touched at its edges.
/// # Safety
/// Same owner/leaves requirements as `water_label`; the facade checks `IsValidTile` first.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_water_invalidate(
    owner: *mut Regions,
    leaves: *const Leaves,
    tile: u32,
) {
    // SAFETY: Call-local exclusive access supplied by facade.
    let (regions, leaves) = unsafe { (&mut *owner, &*leaves) };
    // The facade applies original IsValidTile before entering this operation.
    let x = tile % regions.width;
    let y = tile / regions.width;
    regions.invalidate_one(tile, leaves);
    for (dx, dy) in OFFSETS {
        let nx = (x as i32).wrapping_add(dx);
        let ny = (y as i32).wrapping_add(dy);
        if nx < 0 || ny < 0 || nx >= regions.width as i32 || ny >= regions.height as i32 {
            continue;
        }
        let adjacent = (ny as u32) * regions.width + nx as u32;
        if regions.coords(adjacent) != regions.coords(tile) {
            regions.invalidate_one(adjacent, leaves);
        }
    }
}

/// Copy the diagnostic display state; no cache borrow survives formatting.
/// # Safety
/// Owner/leaves follow `water_label`. Output addresses one exclusive initialized snapshot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_water_snapshot(
    owner: *mut Regions,
    leaves: *const Leaves,
    tile: u32,
    out: *mut Snapshot,
) {
    // SAFETY: Facade supplies exclusive owner and output, immutable leaves.
    let (regions, leaves, out) = unsafe { (&mut *owner, &*leaves, &mut *out) };
    let (x, y) = regions.coords(tile);
    let index = regions.update(x, y, leaves);
    *out = Snapshot {
        edges: regions.data[index].edges,
        labels: std::array::from_fn(|local| regions.label(index, local)),
        patches: regions.data[index].patches,
        aqueducts: u8::from(regions.data[index].aqueducts),
    };
}

/// Begin visiting a patch. Only copied progress is kept between calls.
/// # Safety
/// Owner/leaves follow `water_label`; coordinates follow the original region precondition.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_water_visit_new(
    owner: *mut Regions,
    leaves: *const Leaves,
    patch: Patch,
) -> *mut Visit {
    if patch.label != 0 {
        // SAFETY: Cache access ends before returning to arbitrary visitors.
        unsafe {
            (&mut *owner).update(patch.x, patch.y, &*leaves);
        }
    }
    Box::into_raw(Box::new(Visit {
        patch,
        side: 0,
        pending: Vec::new().into_iter(),
        aqueduct_tile: 0,
        aqueducts: None,
    }))
}

/// Destroy an owned visitor cursor, including on a C++ visitor exception.
/// # Safety
/// Cursor is uniquely owned, live and returned by `water_visit_new`; destroy once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_water_visit_destroy(visit: *mut Visit) {
    // SAFETY: Unique ownership is transferred back by the facade.
    drop(unsafe { Box::from_raw(visit) });
}

/// Return the next visitor argument, observing the live caller-owned descriptor.
/// No owner borrow survives the return.
/// # Safety
/// Owner/leaves follow `water_label`; cursor/output are exclusive, live disjoint allocations.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_water_visit_next(
    owner: *mut Regions,
    leaves: *const Leaves,
    visit: *mut Visit,
    live_patch: Patch,
    out: *mut Patch,
) -> u8 {
    // SAFETY: All borrows end on return, before C++ invokes arbitrary visitors.
    if let Some(patch) = unsafe { (&mut *visit).next(&mut *owner, &*leaves, live_patch) } {
        // SAFETY: Output addresses an initialized writable Patch.
        unsafe {
            *out = patch;
        }
        1
    } else {
        0
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDWaterRegionContext" ["common"] => Context {
        owner,
        leaves,
    };
    "OpenTTDWaterPatch" ["probe"] => Patch {
        x,
        y,
        label,
    };
    "OpenTTDWaterSnapshot" ["probe"] => Snapshot {
        edges,
        labels,
        patches,
        aqueducts,
    };
    "OpenTTDWaterLeaves" ["probe"] => Leaves {
        shared,
        follow,
        aqueduct,
        debug,
    };
}
