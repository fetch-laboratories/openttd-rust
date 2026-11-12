/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Variable-length curve limits and reversal swaps the stock-engine scenarios cannot reach
//! (#736, #946).
//!
//! Moved from `tools/migration/train-edge-comparison.cpp`, which ran the pinned original's
//! `Train::GetCurveSpeedLimit` and `ReverseTrainSwapVeh` and these production entries over
//! the same random consists and compared them. The original's results are frozen in
//! `edge-oracle.txt` (`python3 tools/oracle-capture.py train_edge`), whose header gives the
//! line format. This test regenerates the probe's 20,000 trials from its generator, builds
//! the 16 vehicle records the probe linked, and calls:
//!
//! - `curve_speed_limit`, the crate function `settings_table.rs` calls, for the curve
//!   limit. The probe called the export it replaced, which had no other caller.
//! - The export `openttd_rust_train_reverse_swap`, as the probe did, for the reversal.
//!
//! The live services are the probe's and no more: the curve advantage, the viewport, the
//! virtual tile, the vehicle table's tile links and map width, the acceleration model, a 256
//! by 256 map and the movement table's vehicle resolution. Every other entry is an
//! `unreachable` stub, so a call to any other service fails the test.
//!
//! Each tile entry the original made was asserted to be on the entering vehicle's tile, so
//! the expected entry tiles are decoded from the expected line. As in the probe, the map is
//! plain rail on the next expected entry tile only and clear elsewhere: an entry on any other
//! tile skips the vehicle resolution and changes the recorded events.
use super::{Handle, Leaves, curve_speed_limit, openttd_rust_train_reverse_swap};
use crate::ground_vehicle::GroundState;
use crate::map_func::{BaseTileLayout, ExtendedTileLayout, TileTypeAllocation};
use crate::movement::{MovementServices, MovementVehicle};
use crate::oracle;
use crate::services::Services;
use crate::settings_type::SettingAddresses;
use crate::test_tables::{MOVEMENT, SHARED, TRAIN_LEAVES, VEHICLE};
use crate::tile_type::TileIndex;
use crate::train_state::{State, openttd_rust_train_state_destroy, openttd_rust_train_state_new};
use crate::vehicle::occupancy::{Links, openttd_rust_occupancy_update};
use crate::vehicle::{OpenTTDVehicle, VehicleRef, VehicleServices};
use crate::vehicle_base::{openttd_rust_vehicle_record_delete, openttd_rust_vehicle_record_new};
use crate::vehicle_type::VehicleType;
use std::cell::{Cell, RefCell};
use std::fmt::Write as _;
use std::ptr::{self, null_mut};

const ORACLE: &str = include_str!("edge-oracle.txt");
const TRIALS: usize = 20_000;
const VEHICLES: usize = 16;
/// `MP_RAILWAY` in the tile type bits.
const RAILWAY: u8 = 1 << 4;
const MAP_LOG_X: u32 = 8;
const MAP_SIZE: usize = 1 << (2 * MAP_LOG_X);

/// The compared fields of one vehicle, in the probe's order: track, direction, x, y, z,
/// tile, `gv_flags` and `vehstatus`.
type Fields = [u32; 8];

/// One trial's inputs.
struct Trial {
    count: usize,
    model: u8,
    curve_speed: u8,
    before: [Fields; VEHICLES],
    lengths: [u16; VEHICLES],
    tilts: [bool; VEHICLES],
    curve_mods: [i16; VEHICLES],
    l: usize,
    r: usize,
}

/// The probe's generator.
struct Draws(u32);
impl Draws {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    /// One trial, with the draws in the probe's order; `l` and `r` follow the curve check.
    fn trial(&mut self, trial: usize) -> Trial {
        let count = 1 + (self.next() % 16) as usize;
        let model = (trial % 2) as u8;
        let curve_speed = self.next() as u8;
        let mut t = Trial {
            count,
            model,
            curve_speed,
            before: [[0; 8]; VEHICLES],
            lengths: [0; VEHICLES],
            tilts: [false; VEHICLES],
            curve_mods: [0; VEHICLES],
            l: 0,
            r: 0,
        };
        for i in 0..VEHICLES {
            let direction = self.next() % 8;
            t.lengths[i] = (1 + self.next() % 8) as u16;
            t.tilts[i] = self.next() & 1 != 0;
            t.curve_mods[i] = self.next() as i16;
            let track = 1_u32 << (self.next() % 8);
            let flags = u32::from(self.next() as u16);
            let status = u32::from(self.next() as u8);
            let x = self.next() % 4000;
            let y = self.next() % 4000;
            let z = self.next() % 256;
            let tile = self.next() % 65536;
            t.before[i] = [track, direction, x, y, z, tile, flags, status];
        }
        t
    }
}

/// The probe's world: the recorded events, the live inputs of its leaves and the map.
struct World {
    records: [*mut OpenTTDVehicle; VEHICLES],
    links: *mut [Links; VEHICLES],
    /// `(kind, vehicle)`: 1 `UpdatePosition`, 2 `UpdateViewport`, 3 `VehicleEnterTile`.
    events: Vec<(u8, usize)>,
    curve_speed: u8,
    base: *mut BaseTileLayout,
    /// The original's entry tiles for this trial, and the next one the candidate uses.
    entries: Vec<TileIndex>,
    next_entry: usize,
}
thread_local! {
    static WORLD: RefCell<World> = const {
        RefCell::new(World {
            records: [null_mut(); VEHICLES],
            links: null_mut(),
            events: Vec::new(),
            curve_speed: 0,
            base: null_mut(),
            entries: Vec::new(),
            next_entry: 0,
        })
    };
    static MODEL: Cell<u8> = const { Cell::new(0) };
}

fn index(v: *const OpenTTDVehicle) -> usize {
    WORLD
        .with_borrow(|w| w.records.iter().position(|&r| ptr::eq(r, v)))
        .expect("a fixture vehicle")
}
fn set_kind(tile: TileIndex, kind: u8) {
    WORLD.with_borrow(|w| {
        // SAFETY: The map allocation holds MAP_SIZE tiles; one field-sized store.
        unsafe { (&raw mut (*w.base.add(tile.0 as usize)).kind).write(kind) };
    });
}

extern "C" fn curve_advantage(_: Handle) -> u64 {
    u64::from(WORLD.with_borrow(|w| w.curve_speed))
}
extern "C" fn viewport(h: Handle, a: u8, b: u8) {
    assert!(a != 0 && b != 0, "UpdateViewport(true, true)");
    let i = index(h.record);
    WORLD.with_borrow_mut(|w| w.events.push((2, i)));
}
extern "C" fn tile_virt(x: i32, y: i32) -> u32 {
    ((y >> 4) * 256 + (x >> 4)) as u32
}
extern "C" fn tile_links(v: VehicleRef) -> *mut Links {
    let i = index(v.0.as_ptr());
    WORLD.with_borrow_mut(|w| {
        w.events.push((1, i));
        // SAFETY: The links array lives for the test; no reference to it is held.
        unsafe { (&raw mut (*w.links)[i]) }
    })
}
extern "C" fn map_log_x() -> u32 {
    MAP_LOG_X
}
extern "C" fn resolve(v: VehicleRef) -> MovementVehicle {
    let i = index(v.0.as_ptr());
    let (entered, next) = WORLD.with_borrow_mut(|w| {
        w.events.push((3, i));
        let entered = w.entries.get(w.next_entry).copied();
        w.next_entry += 1;
        (entered, w.entries.get(w.next_entry).copied())
    });
    if let Some(tile) = entered {
        set_kind(tile, 0);
    }
    if let Some(tile) = next {
        set_kind(tile, RAILWAY);
    }
    let record = v.0.as_ptr();
    // SAFETY: The fixture record is live and links its train state.
    let owner = unsafe { (*record).train };
    MovementVehicle {
        train: Handle { record, owner },
        road: null_mut(),
        ship: null_mut(),
        kind: VehicleType::VEH_TRAIN,
    }
}

/// The 16 vehicle records with their ground and train states, and the service tables.
struct Fixture {
    records: [*mut OpenTTDVehicle; VEHICLES],
    grounds: [*mut GroundState; VEHICLES],
    states: [*mut State; VEHICLES],
    links: *mut [Links; VEHICLES],
    base: *mut [BaseTileLayout],
    extended: *mut [ExtendedTileLayout],
    vehicle: *mut VehicleServices,
    leaves: *mut Leaves,
    shared: *mut Services,
    movement: *mut MovementServices,
    settings: *mut SettingAddresses,
}
impl Fixture {
    fn new() -> Self {
        let records = std::array::from_fn(|_| {
            openttd_rust_vehicle_record_new(null_mut(), VehicleType::VEH_TRAIN)
        });
        let grounds = std::array::from_fn(|_| Box::into_raw(Box::new(GroundState::default())));
        let states = std::array::from_fn(|_| openttd_rust_train_state_new());
        let links = Box::into_raw(Box::new(std::array::from_fn(|_| Links {
            next: null_mut(),
            prev: null_mut(),
            current: null_mut(),
        })));
        let base: Box<[BaseTileLayout]> = (0..MAP_SIZE)
            .map(|_| BaseTileLayout {
                kind: 0,
                height: 0,
                m2: 0,
                m1: 0,
                m3: 0,
                m4: 0,
                m5: 0,
            })
            .collect();
        let extended: Box<[ExtendedTileLayout]> = (0..MAP_SIZE)
            .map(|_| ExtendedTileLayout {
                m6: 0,
                m7: 0,
                m8: 0,
            })
            .collect();
        let (base, extended) = (Box::into_raw(base), Box::into_raw(extended));
        crate::map_func::fixture::set(
            TileTypeAllocation {
                base: base.cast::<BaseTileLayout>(),
                extended: extended.cast::<ExtendedTileLayout>(),
                count: MAP_SIZE as u32,
                assertions: true,
            },
            MAP_LOG_X,
        );
        let vehicle = Box::into_raw(Box::new(VehicleServices {
            tile_links,
            map_log_x,
            ..VEHICLE
        }));
        let leaves = Box::into_raw(Box::new(Leaves {
            curve_advantage,
            tile_virt,
            viewport,
            vehicle,
            ..TRAIN_LEAVES
        }));
        let settings = Box::into_raw(Box::new(SettingAddresses {
            train_acceleration_model: MODEL.with(Cell::as_ptr).cast_const(),
            ..SettingAddresses::NULL
        }));
        let movement = Box::into_raw(Box::new(MovementServices {
            resolve,
            ..MOVEMENT
        }));
        let shared = Box::into_raw(Box::new(Services {
            movement,
            map: crate::map_func::fixture::statics(),
            settings,
            ..SHARED
        }));
        // SAFETY: Both tables were just allocated and nothing else refers to them yet.
        unsafe { (*movement).shared = shared };
        WORLD.with_borrow_mut(|w| {
            w.records = records;
            w.links = links;
            w.base = base.cast::<BaseTileLayout>();
        });
        Self {
            records,
            grounds,
            states,
            links,
            base,
            extended,
            vehicle,
            leaves,
            shared,
            movement,
            settings,
        }
    }

    /// Write the trial's vehicles into the records, linked as the probe's `Sync` links them.
    fn sync(&self, t: &Trial) {
        for i in 0..VEHICLES {
            let [track, direction, x, y, z, tile, flags, status] = t.before[i];
            // SAFETY: The fixture owns these allocations; no reference to them is live.
            unsafe {
                let v = &mut *self.records[i];
                v.train = self.states[i];
                v.ground = self.grounds[i];
                v.next = if i + 1 < t.count {
                    self.records[i + 1]
                } else {
                    null_mut()
                };
                v.first = self.records[0];
                v.tile = TileIndex(tile);
                v.x_pos = x as i32;
                v.y_pos = y as i32;
                v.z_pos = z as i32;
                v.direction = direction as u8;
                v.vehstatus = status as u8;
                v.subtype = ((i % 2) << 2) as u8;
                let g = &mut *self.grounds[i];
                g.cache.cached_veh_length = t.lengths[i] as u8;
                g.flags = flags as u16;
                let s = &mut *self.states[i];
                s.track = track as u8;
                s.cached_tilt = t.tilts[i];
                s.cached_curve_speed_mod = t.curve_mods[i];
            }
        }
    }

    /// The candidate's fields of every vehicle after the call.
    fn fields(&self) -> [Fields; VEHICLES] {
        std::array::from_fn(|i| {
            // SAFETY: The fixture owns these allocations; no reference to them is live.
            unsafe {
                let v = &*self.records[i];
                [
                    u32::from((*self.states[i]).track),
                    u32::from(v.direction),
                    v.x_pos as u32,
                    v.y_pos as u32,
                    v.z_pos as u32,
                    v.tile.0,
                    u32::from((*self.grounds[i]).flags),
                    u32::from(v.vehstatus),
                ]
            }
        })
    }

    fn head(&self) -> Handle {
        Handle {
            record: self.records[0],
            owner: self.states[0],
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        WORLD.with_borrow_mut(|w| {
            w.records = [null_mut(); VEHICLES];
            w.links = null_mut();
            w.base = null_mut();
        });
        // SAFETY: Each allocation was made by `Fixture::new` and is released once; the
        // vehicles left the tile index after the last trial.
        unsafe {
            for i in 0..VEHICLES {
                openttd_rust_vehicle_record_delete(self.records[i]);
                drop(Box::from_raw(self.grounds[i]));
                openttd_rust_train_state_destroy(self.states[i]);
            }
            drop(Box::from_raw(self.links));
            drop(Box::from_raw(self.base));
            drop(Box::from_raw(self.extended));
            drop(Box::from_raw(self.vehicle));
            drop(Box::from_raw(self.leaves));
            drop(Box::from_raw(self.shared));
            drop(Box::from_raw(self.movement));
            drop(Box::from_raw(self.settings));
        }
    }
}

/// The original's entry tiles in order, decoded from the expected swap line: the tile of
/// the vehicle at `l` (side 0) or `r` (side 1) after the swap, its XOR difference applied
/// to the other index's tile before it.
fn expected_entries(line: &str, t: &Trial) -> Vec<TileIndex> {
    let tokens: Vec<&str> = line.split(' ').collect();
    let tile = |side: usize| {
        let stored: u32 = tokens[2 + side * 8 + 5].parse().expect("oracle tile field");
        let other = if side == 0 { t.r } else { t.l };
        TileIndex(stored ^ t.before[other][5])
    };
    tokens[0]
        .as_bytes()
        .chunks(2)
        .filter(|pair| pair[0] == b'3')
        .map(|pair| tile(usize::from(pair[1] - b'0')))
        .collect()
}

/// The swap line in the oracle's format.
fn swap_line(events: &[(u8, usize)], after: &[Fields; VEHICLES], t: &Trial) -> String {
    let mut line = String::new();
    for &(kind, i) in events {
        let _ = write!(line, "{kind}{}", u8::from(i != t.l));
    }
    let others = (0..VEHICLES)
        .filter(|&i| i != t.l && i != t.r && after[i] != t.before[i])
        .count();
    let _ = write!(line, " {others}");
    for (side, other) in [(t.l, t.r), (t.r, t.l)] {
        for (f, value) in after[side].iter().enumerate() {
            let reference = if f < 6 {
                t.before[other][f]
            } else {
                t.before[side][f]
            };
            let _ = write!(line, " {}", value ^ reference);
        }
        if t.l == t.r {
            break;
        }
    }
    line
}

/// `(inputs, output)` of each trial, in trial order.
type Cases = Vec<(String, String)>;

/// Run every trial in order: the curve limits and the swap lines.
fn run() -> (Cases, Cases) {
    let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
    let sections = oracle::parse(ORACLE);
    let swaps = &sections["swap"];
    assert_eq!(swaps.len(), TRIALS, "the oracle holds every swap trial");
    let fixture = Fixture::new();
    let mut draws = Draws(130);
    let (mut curves, mut lines) = (Vec::new(), Vec::new());
    for trial in 0..TRIALS {
        let mut t = draws.trial(trial);
        MODEL.set(t.model);
        WORLD.with_borrow_mut(|w| w.curve_speed = t.curve_speed);
        fixture.sync(&t);
        let inputs = format!("trial {trial} count {} model {}", t.count, t.model);
        // SAFETY: The fixture's tables and records are live for the call.
        let limit =
            unsafe { curve_speed_limit(fixture.head(), &*fixture.leaves, &*fixture.shared) };
        curves.push((inputs.clone(), limit.to_string()));

        t.l = (draws.next() as usize) % t.count;
        t.r = (draws.next() as usize) % t.count;
        let entries = expected_entries(swaps.case(trial).expect("swap case"), &t);
        if let Some(&first) = entries.first() {
            set_kind(first, RAILWAY);
        }
        WORLD.with_borrow_mut(|w| {
            w.events.clear();
            w.entries.clone_from(&entries);
            w.next_entry = 0;
        });
        // SAFETY: The fixture's tables and records are live for the call.
        unsafe {
            openttd_rust_train_reverse_swap(
                fixture.head(),
                t.l as i32,
                t.r as i32,
                fixture.leaves,
                fixture.shared,
            );
        }
        for &tile in &entries {
            set_kind(tile, 0);
        }
        let events = WORLD.with_borrow_mut(|w| std::mem::take(&mut w.events));
        let line = swap_line(&events, &fixture.fields(), &t);
        lines.push((format!("{inputs} l {} r {}", t.l, t.r), line));
        for &record in &fixture.records {
            // SAFETY: The vehicle table and the record are live for the call.
            unsafe {
                openttd_rust_occupancy_update(&*fixture.vehicle, VehicleRef::of(record), true);
            }
        }
    }
    WORLD.with_borrow_mut(|w| w.events.clear());
    (curves, lines)
}

/// 20,000 curve limits equal the original's, under both acceleration models.
#[test]
fn curve_limits_match_original() {
    let sections = oracle::parse(ORACLE);
    let (curves, _) = run();
    // The oracle stores the even trials first, then the odd ones.
    let ordered = curves
        .iter()
        .step_by(2)
        .chain(curves.iter().skip(1).step_by(2))
        .cloned();
    assert_eq!(oracle::check(&sections, "curve", ordered), TRIALS);
}

/// For 20,000 reversals, the event order, the state of all 16 vehicles and the entry tiles
/// equal the original's, and no other service is called.
#[test]
fn reversals_match_original() {
    let sections = oracle::parse(ORACLE);
    let (_, lines) = run();
    assert_eq!(oracle::check(&sections, "swap", lines), TRIALS);
}
