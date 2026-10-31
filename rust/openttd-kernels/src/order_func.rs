/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Service interval defaults and limits from `order_func.h`. Native assertions pin their values.
#![allow(missing_docs)]
pub const DEF_SERVINT_DAYS_TRAINS: u16 = 150;
pub const DEF_SERVINT_DAYS_ROADVEH: u16 = 150;
pub const DEF_SERVINT_DAYS_AIRCRAFT: u16 = 100;
pub const DEF_SERVINT_DAYS_SHIPS: u16 = 360;
pub const MIN_SERVINT_DAYS: u16 = 30;
pub const MAX_SERVINT_DAYS: u16 = 800;
pub const DEF_SERVINT_MINUTES_TRAINS: u16 = 5;
pub const DEF_SERVINT_MINUTES_ROADVEH: u16 = 5;
pub const DEF_SERVINT_MINUTES_AIRCRAFT: u16 = 3;
pub const DEF_SERVINT_MINUTES_SHIPS: u16 = 12;
pub const MIN_SERVINT_MINUTES: u16 = 1;
pub const MAX_SERVINT_MINUTES: u16 = 30;
pub const DEF_SERVINT_PERCENT: u16 = 50;
pub const MIN_SERVINT_PERCENT: u16 = 5;
pub const MAX_SERVINT_PERCENT: u16 = 90;
