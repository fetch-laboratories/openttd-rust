/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Pause policy transferred from src/tests/session.cpp. Native aliases, serialized-thread
//! access, native callback reentry and load partial effects remain in that C++ fixture.

use super::{PauseMode, Services};
use std::fmt::Write as _;
use std::sync::Mutex;

struct Observations {
    networking: bool,
    events: String,
}
static OBSERVATIONS: Mutex<Observations> = Mutex::new(Observations {
    networking: false,
    events: String::new(),
});

extern "C" fn networking() -> bool {
    OBSERVATIONS.lock().unwrap().networking
}
extern "C" fn query() {
    OBSERVATIONS.lock().unwrap().events.push_str("query,");
}
extern "C" fn changed(previous: u8, mode: PauseMode) {
    write!(
        OBSERVATIONS.lock().unwrap().events,
        "change:{previous}:{},",
        mode.0
    )
    .unwrap();
}
extern "C" fn dirty() {
    // The whole native pause_dirty leaf calls these windows in this order.
    OBSERVATIONS
        .lock()
        .unwrap()
        .events
        .push_str("status,toolbar,");
}

fn observe(services: &Services, mode: PauseMode, paused: bool, execute: bool) -> String {
    OBSERVATIONS.lock().unwrap().events.clear();
    let success = super::openttd_rust_session_pause_abi(services, mode, paused, execute);
    let observations = OBSERVATIONS.lock().unwrap();
    let events = if observations.events.is_empty() {
        "-"
    } else {
        &observations.events
    };
    format!("{success} {} {events}", super::pause())
}

#[test]
fn pause_input_matrix_matches_original_and_callback_order() {
    const FINANCE: crate::company::FinanceServices = crate::company::FinanceServices {
        networking,
        ..crate::test_tables::FINANCE
    };
    const COMPANY: crate::company::Services = crate::company::Services {
        finance: &FINANCE,
        ..crate::test_tables::COMPANY
    };
    let _lock = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
    let saved_pause = super::pause();
    let services = Services {
        company: &COMPANY,
        pause_query: query,
        pause_change: changed,
        pause_dirty: dirty,
        ..crate::test_tables::SESSION
    };
    let oracle = crate::oracle::parse(include_str!("pause-oracle.txt"));
    let mut actual = Vec::new();
    // Retiring fixture order: network, all underlying PauseMode values, execute.
    for network in [false, true] {
        for value in 0..=u8::MAX {
            for execute in [false, true] {
                OBSERVATIONS.lock().unwrap().networking = network;
                super::set_pause(PauseMode::CommandDuringPause.bit());
                let selected = PauseMode(value);
                let mut output = observe(&services, selected, true, execute);
                let valid = value < 7 && (network || (value != 2 && value != 4));
                if valid && execute {
                    output.push_str(" | ");
                    output.push_str(&observe(&services, selected, false, true));
                }
                actual.push((
                    format!("network={network},value={value},execute={execute}"),
                    output,
                ));
            }
        }
    }
    let compared = crate::oracle::check(&oracle, "pause", actual);
    super::set_pause(PauseMode::Error.bit());
    let query_result = observe(&services, PauseMode::Normal, false, true);
    super::set_pause(saved_pause);
    assert_eq!(compared, 1024);
    assert_eq!(
        crate::oracle::check(&oracle, "query", [("unsafe unpause".into(), query_result)]),
        1
    );
}
