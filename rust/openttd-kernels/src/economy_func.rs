/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared economy helpers from `economy_func.h`.

/// Original `GetPrice`. `grf_shift` is the `NewGRF`'s `price_base_multipliers[index]`
/// (zero without a GRF file), read natively at the original point; `shift` is the
/// caller's extra shift. The `PR_END` check is `openttd_rust_economy_price`'s bounds test.
pub(crate) fn get_price(index: u32, cost_factor: u32, grf_shift: i32, shift: i32) -> i64 {
    crate::company::openttd_rust_economy_price(index, cost_factor, shift.wrapping_add(grf_shift))
}
