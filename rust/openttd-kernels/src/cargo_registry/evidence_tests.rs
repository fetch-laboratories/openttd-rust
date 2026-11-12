/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `SetupCargoForClimate` resets and lookups, published process-wide (#736).
//!
//! Moved from `src/tests/cargo_registry.cpp`, whose expectations were fork-written
//! literals for one Arctic and one Temperate setup. The pinned original ran
//! `SetupCargoForClimate` for each climate after each climate, each on its own thread, and
//! its observations are frozen in `climate-oracle.txt` (`python3 tools/oracle-capture.py
//! cargo_registry`). These tests run the production [`openttd_rust_cargo_setup`] the same
//! way, behind a services table whose specification stubs stand in for the native
//! `CargoSpec::array`: `copy_default` copies the climate's slot from the original's
//! captured specifications, as the native copy from `_default_climate_cargo` does, and
//! `clear_specs` resets the remaining slots to `CargoSpec{}`. The lookups, translation
//! tables, cargo mask and default-cargo tests are read through the exports the C++ glue
//! calls, on the test thread.
use super::*;
use crate::oracle;
use crate::test_tables::SHARED;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

const ORACLE: &str = include_str!("climate-oracle.txt");
const CLIMATES: u8 = crate::landscape_type::NUM_LANDSCAPE;

/// `CargoSpec{}`: the empty label with `INVALID_CARGO_BITNUM`.
const EMPTY_SPEC: u64 = 0xFF << 32;

/// The native `CargoSpec::array` as `bitnum << 32 | label` per slot.
static SPECS: [AtomicU64; NUM_CARGO as usize] =
    [const { AtomicU64::new(EMPTY_SPEC) }; NUM_CARGO as usize];
/// The original's specifications after each climate's setup, from the oracle.
static DEFAULTS: OnceLock<Vec<Vec<u64>>> = OnceLock::new();

fn slot(handle: Handle) -> usize {
    (handle.addr() - SPECS.as_ptr().addr()) / std::mem::size_of::<AtomicU64>()
}

extern "C" fn spec(cargo: u8) -> Handle {
    (&raw const SPECS[usize::from(cargo)]).cast()
}

extern "C" fn copy_default(climate: u8, cargo: u8) {
    let spec = DEFAULTS.get().unwrap()[usize::from(climate)][usize::from(cargo)];
    SPECS[usize::from(cargo)].store(spec, Ordering::Relaxed);
}

extern "C" fn clear_specs(first: u8) {
    for spec in &SPECS[usize::from(first)..] {
        spec.store(EMPTY_SPEC, Ordering::Relaxed);
    }
}

extern "C" fn label_spec(handle: Handle) -> LabelSpec {
    let spec = SPECS[slot(handle)].load(Ordering::Relaxed);
    LabelSpec {
        label: spec as u32,
        bitnum: (spec >> 32) as u8,
    }
}

extern "C" fn index(handle: Handle) -> u8 {
    slot(handle) as u8
}

static TABLE: GameState<Services> = GameState::new(Services {
    shared: &SHARED,
    spec,
    copy_default,
    clear_specs,
    label_spec,
    index,
    ..TEST_TABLE
});

/// A `label:bitnum` specification as the capture prints it.
fn parse_spec(text: &str) -> u64 {
    let (label, bitnum) = text.split_once(':').unwrap();
    u64::from(u32::from_str_radix(label, 16).unwrap())
        | u64::from(bitnum.parse::<u8>().unwrap()) << 32
}

fn labels(labels: Labels) -> String {
    // SAFETY: the registry's array, read before any further registry call.
    let labels = unsafe { std::slice::from_raw_parts(labels.data, labels.size) };
    labels
        .iter()
        .map(|label| format!("{label:08X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn climate_setup_is_process_wide_and_matches_original() {
    let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
    let sections = oracle::parse(ORACLE);
    // Case `climate` is climate after climate 0, so its specifications are the climate's.
    let specs = &sections["spec"];
    let defaults = DEFAULTS.get_or_init(|| {
        (0..usize::from(CLIMATES))
            .map(|climate| {
                specs
                    .case(climate)
                    .unwrap()
                    .split(' ')
                    .map(parse_spec)
                    .collect()
            })
            .collect()
    });
    // The lookup domain: every label the four climates hold, in first-appearance order.
    let mut domain = Vec::new();
    for label in defaults.iter().flatten().map(|&spec| spec as u32) {
        if !domain.contains(&label) {
            domain.push(label);
        }
    }
    let mut rows: [Vec<(String, String)>; 4] = Default::default();
    for previous in 0..CLIMATES {
        for climate in 0..CLIMATES {
            for setup in [previous, climate] {
                // SAFETY: the table is static and never written; the lock serializes the registry.
                std::thread::spawn(move || unsafe { openttd_rust_cargo_setup(TABLE.get(), setup) })
                    .join()
                    .unwrap();
            }
            let inputs = format!("climate {climate} after {previous}");
            let spec_row: Vec<String> = SPECS
                .iter()
                .map(|spec| {
                    let spec = spec.load(Ordering::Relaxed);
                    format!("{:08X}:{}", spec as u32, spec >> 32)
                })
                .collect();
            let lookups: Vec<String> = domain
                .iter()
                .map(|&label| openttd_rust_cargo_by_label(label).to_string())
                .collect();
            let tables = format!(
                "{} | {}",
                labels(openttd_rust_cargo_climate_labels()),
                labels(openttd_rust_cargo_bitnum_labels())
            );
            // SAFETY: the registry's mask, read before any further registry call.
            let mask = unsafe { *openttd_rust_cargo_mask() };
            let defaults: String = (0..NUM_CARGO)
                .map(|cargo| char::from(b'0' + openttd_rust_cargo_is_default_abi(cargo)))
                .collect();
            for (rows, output) in rows.iter_mut().zip([
                spec_row.join(" "),
                lookups.join(" "),
                tables,
                format!("{mask:016X} {defaults}"),
            ]) {
                rows.push((inputs.clone(), output));
            }
        }
    }
    let [spec, lookup, table, mask] = rows;
    for (name, rows) in [
        ("spec", spec),
        ("lookup", lookup),
        ("table", table),
        ("mask", mask),
    ] {
        assert_eq!(oracle::check(&sections, name, rows), 16, "{name}");
    }
    clear_test_labels();
}
