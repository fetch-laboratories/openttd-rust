/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `FlowStat` and `FlowStatMap` (`station_base.h`, `station_cmd.cpp`), and the application of
//! a finished link graph job's flows to the stations (`LinkGraphJob::~LinkGraphJob` and
//! `LinkGraphJob::EraseFlows`, `linkgraph/linkgraphjob.cpp`). The original's unsigned
//! arithmetic wraps explicitly.
//!
//! A [`FlowStatMap`] owns each of its flows in its own allocation, which no later insertion or
//! erasure of another origin moves, as `std::map` never moves its nodes: C++ holds a node's
//! address across later calls on the map. A reference to a node lasts only for the method or
//! the borrow that made it. Callers serialise access: the game-state mutex for a station's
//! flows, and the worker that owns a job's maps until the join.
use crate::cargo_type::CargoType;
use crate::station_type::StationID;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::ops::Bound::{Excluded, Unbounded};

/// `FlowStat::SharesMap`.
pub(crate) type SharesMap = BTreeMap<u32, StationID>;

/// `FlowStat`: flow statistics telling how much flow should be sent along a link. This is done
/// by creating "flow shares" and using `upper_bound` to look them up with a random number. A
/// flow share is the difference between a key in a map and the previous key. So one key in the
/// map doesn't actually mean anything by itself.
#[derive(Clone)]
pub struct FlowStat {
    /// Shares of flow to be sent via specified station (or consumed locally).
    shares: SharesMap,
    /// Limit for unrestricted shares.
    unrestricted: u32,
}

impl FlowStat {
    /// Create a `FlowStat` with an initial entry.
    pub(crate) fn new(st: StationID, flow: u32, restricted: bool) -> Self {
        crate::services_assert::assert_with(|| flow > 0);
        let mut shares = SharesMap::new();
        shares.insert(flow, st);
        Self {
            shares,
            unrestricted: if restricted { 0 } else { flow },
        }
    }

    /// `FlowStat::AppendShare`: add some flow to the end of the shares map. Only do that if you
    /// know that the station isn't in the map yet. Anything else may lead to inconsistencies.
    pub(crate) fn append_share(&mut self, st: StationID, flow: u32, restricted: bool) {
        crate::services_assert::assert_with(|| flow > 0);
        let last = *self.shares.last_key_value().unwrap().0;
        self.shares.insert(last.wrapping_add(flow), st);
        if !restricted {
            self.unrestricted = self.unrestricted.wrapping_add(flow);
        }
    }

    /// `FlowStat::GetShare`: get flow for a station.
    pub(crate) fn get_share(&self, st: StationID) -> u32 {
        let mut prev = 0_u32;
        for (&key, &station) in &self.shares {
            if station == st {
                return key.wrapping_sub(prev);
            }
            prev = key;
        }
        0
    }

    /// `FlowStat::GetShares`: the actual shares, to be iterated over.
    pub(crate) fn get_shares(&self) -> &SharesMap {
        &self.shares
    }

    /// `FlowStat::GetUnrestricted`: the total amount of unrestricted shares.
    pub(crate) fn get_unrestricted(&self) -> u32 {
        self.unrestricted
    }

    /// `FlowStat::SwapShares`: swap the shares maps, and thus the content of this `FlowStat`
    /// with the other one.
    pub(crate) fn swap_shares(&mut self, other: &mut Self) {
        std::mem::swap(&mut self.shares, &mut other.shares);
        std::mem::swap(&mut self.unrestricted, &mut other.unrestricted);
    }

    /// `FlowStat::ChangeShare`: change share for specified station. By specifying `INT_MIN` as
    /// parameter you can erase a share. Newly added flows will be unrestricted.
    #[allow(
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "the original's `(uint)(-flow)`, `(uint)flow` and `flow += share`"
    )]
    pub(crate) fn change_share(&mut self, st: StationID, mut flow: i32) {
        // We assert only before changing as afterwards the shares can actually be empty. In
        // that case the whole flow stat must be deleted then.
        crate::services_assert::assert_with(|| !self.shares.is_empty());

        let mut removed_shares = 0_u32;
        let mut added_shares = 0_u32;
        let mut last_share = 0_u32;
        let mut new_shares = SharesMap::new();
        for (&key, &station) in &self.shares {
            if station == st {
                if flow < 0 {
                    let share = key.wrapping_sub(last_share);
                    if flow == i32::MIN || flow.wrapping_neg() as u32 >= share {
                        removed_shares = removed_shares.wrapping_add(share);
                        if key <= self.unrestricted {
                            self.unrestricted = self.unrestricted.wrapping_sub(share);
                        }
                        if flow != i32::MIN {
                            flow = flow.wrapping_add(share as i32);
                        }
                        last_share = key;
                        continue; // remove the whole share
                    }
                    removed_shares = removed_shares.wrapping_add(flow.wrapping_neg() as u32);
                } else {
                    added_shares = added_shares.wrapping_add(flow as u32);
                }
                if key <= self.unrestricted {
                    self.unrestricted = self.unrestricted.wrapping_add(flow as u32);
                }

                // If we don't continue above the whole flow has been added or removed.
                flow = 0;
            }
            new_shares.insert(
                key.wrapping_add(added_shares).wrapping_sub(removed_shares),
                station,
            );
            last_share = key;
        }
        if flow > 0 {
            new_shares.insert(last_share.wrapping_add(flow as u32), st);
            if self.unrestricted < last_share {
                // This releases the share in the old map, which the swap below replaces; only
                // its change to `unrestricted` remains.
                self.release_share(st);
            } else {
                self.unrestricted = self.unrestricted.wrapping_add(flow as u32);
            }
        }
        self.shares = new_shares;
    }

    /// `FlowStat::RestrictShare`: restrict a flow by moving it to the end of the map and
    /// decreasing the amount of unrestricted flow.
    pub(crate) fn restrict_share(&mut self, st: StationID) {
        crate::services_assert::assert_with(|| !self.shares.is_empty());
        let mut flow = 0_u32;
        let mut last_share = 0_u32;
        let mut new_shares = SharesMap::new();
        for (&key, &station) in &self.shares {
            if flow == 0 {
                if key > self.unrestricted {
                    return; // Not present or already restricted.
                }
                if station == st {
                    flow = key.wrapping_sub(last_share);
                    self.unrestricted = self.unrestricted.wrapping_sub(flow);
                } else {
                    new_shares.insert(key, station);
                }
            } else {
                new_shares.insert(key.wrapping_sub(flow), station);
            }
            last_share = key;
        }
        if flow == 0 {
            return;
        }
        new_shares.insert(last_share.wrapping_add(flow), st);
        self.shares = new_shares;
        crate::services_assert::assert_with(|| !self.shares.is_empty());
    }

    /// `FlowStat::ReleaseShare`: release ("unrestrict") a flow by moving it to the begin of the
    /// map and increasing the amount of unrestricted flow.
    #[allow(clippy::if_not_else, reason = "the original's branch order")]
    pub(crate) fn release_share(&mut self, st: StationID) {
        crate::services_assert::assert_with(|| !self.shares.is_empty());
        let mut flow = 0_u32;
        let mut next_share = 0_u32;
        let mut found = false;
        for (&key, &station) in self.shares.iter().rev() {
            if key < self.unrestricted {
                return; // Note: not <= as the share may hit the limit.
            }
            if found {
                flow = next_share.wrapping_sub(key);
                self.unrestricted = self.unrestricted.wrapping_add(flow);
                break;
            } else {
                if key == self.unrestricted {
                    return; // !found -> Limit not hit.
                }
                if station == st {
                    found = true;
                }
            }
            next_share = key;
        }
        if flow == 0 {
            return;
        }
        let mut new_shares = SharesMap::new();
        new_shares.insert(flow, st);
        for (&key, &station) in &self.shares {
            if station != st {
                new_shares.insert(flow.wrapping_add(key), station);
            } else {
                flow = 0;
            }
        }
        self.shares = new_shares;
        crate::services_assert::assert_with(|| !self.shares.is_empty());
    }

    /// `FlowStat::ScaleToMonthly`: scale all shares from link graph's runtime to monthly
    /// values. `runtime` must be greater than 0 as we don't want infinite flow values.
    pub(crate) fn scale_to_monthly(&mut self, runtime: u32) {
        crate::services_assert::assert_with(|| runtime > 0);
        let mut new_shares = SharesMap::new();
        let mut share = 0_u32;
        for (&key, &station) in &self.shares {
            share = share.wrapping_add(1).max(key.wrapping_mul(30) / runtime);
            new_shares.insert(share, station);
            if self.unrestricted == key {
                self.unrestricted = share;
            }
        }
        self.shares = new_shares;
    }

    /// `shares.upper_bound(key)`: the first share whose key is above `key`, or `None` at
    /// `end()`.
    fn upper_bound(&self, key: u32) -> Option<(u32, StationID)> {
        self.shares
            .range((Excluded(key), Unbounded))
            .next()
            .map(|(&key, &station)| (key, station))
    }

    /// The key before the share ending at `end`, as `it == begin() ? 0 : (--it)->first`.
    fn begin_of(&self, end: u32) -> u32 {
        self.shares
            .range(..end)
            .next_back()
            .map_or(0, |(&key, _)| key)
    }

    /// `FlowStat::GetViaWithRestricted`: get a station a package can be routed to. This done by
    /// drawing a random number between 0 and `sum_shares` and then looking that up in the map
    /// with `lower_bound`. So each share gets selected with a probability dependent on its
    /// flow. Do include restricted flows here. `random_range` is `RandomRange`.
    pub(crate) fn get_via_with_restricted(
        &self,
        is_restricted: &mut bool,
        mut random_range: impl FnMut(u32) -> u32,
    ) -> StationID {
        crate::services_assert::assert_with(|| !self.shares.is_empty());
        let rand = random_range(*self.shares.last_key_value().unwrap().0);
        *is_restricted = rand >= self.unrestricted;
        self.upper_bound(rand).unwrap().1
    }

    /// `FlowStat::GetVia()`: get a station a package can be routed to, as
    /// [`get_via_with_restricted`](Self::get_via_with_restricted) does. Don't include
    /// restricted flows.
    pub(crate) fn get_via(&self, mut random_range: impl FnMut(u32) -> u32) -> StationID {
        crate::services_assert::assert_with(|| !self.shares.is_empty());
        if self.unrestricted > 0 {
            self.upper_bound(random_range(self.unrestricted)).unwrap().1
        } else {
            StationID::INVALID
        }
    }

    /// `FlowStat::GetVia(excluded, excluded2)`: get a station a package can be routed to, but
    /// exclude the given ones.
    pub(crate) fn get_via_excluded(
        &self,
        excluded: StationID,
        excluded2: StationID,
        mut random_range: impl FnMut(u32) -> u32,
    ) -> StationID {
        if self.unrestricted == 0 {
            return StationID::INVALID;
        }
        crate::services_assert::assert_with(|| !self.shares.is_empty());
        let it = self.upper_bound(random_range(self.unrestricted));
        crate::services_assert::assert_with(|| it.is_some_and(|(key, _)| key <= self.unrestricted));
        let (mut end, station) = it.unwrap();
        if station != excluded && station != excluded2 {
            return station;
        }

        // We've hit one of the excluded stations.
        // Draw another share, from outside its range.

        let mut begin = self.begin_of(end);
        let mut interval = end.wrapping_sub(begin);
        if interval >= self.unrestricted {
            return StationID::INVALID; // Only one station in the map.
        }
        let mut new_max = self.unrestricted.wrapping_sub(interval);
        let rand = random_range(new_max);
        let it2 = if rand < begin {
            self.upper_bound(rand)
        } else {
            self.upper_bound(rand.wrapping_add(interval))
        };
        crate::services_assert::assert_with(|| {
            it2.is_some_and(|(key, _)| key <= self.unrestricted)
        });
        let (mut end2, station2) = it2.unwrap();
        if station2 != excluded && station2 != excluded2 {
            return station2;
        }

        // We've hit the second excluded station.
        // Same as before, only a bit more complicated.

        let mut begin2 = self.begin_of(end2);
        let mut interval2 = end2.wrapping_sub(begin2);
        if interval2 >= new_max {
            return StationID::INVALID; // Only the two excluded stations in the map.
        }
        new_max = new_max.wrapping_sub(interval2);
        if begin > begin2 {
            std::mem::swap(&mut begin, &mut begin2);
            std::mem::swap(&mut end, &mut end2);
            std::mem::swap(&mut interval, &mut interval2);
        }
        let rand = random_range(new_max);
        // The original first sets it3 to `upper_bound(unrestricted)`, a lookup each branch
        // below replaces.
        let it3 = if rand < begin {
            self.upper_bound(rand)
        } else if rand < begin2.wrapping_sub(interval) {
            self.upper_bound(rand.wrapping_add(interval))
        } else {
            self.upper_bound(rand.wrapping_add(interval).wrapping_add(interval2))
        };
        crate::services_assert::assert_with(|| {
            it3.is_some_and(|(key, _)| key <= self.unrestricted)
        });
        it3.unwrap().1
    }

    /// `FlowStat::Invalidate`: reduce all flows to minimum capacity so that they don't get in
    /// the way of link usage statistics too much. Keep them around, though, to continue routing
    /// any remaining cargo.
    pub(crate) fn invalidate(&mut self) {
        crate::services_assert::assert_with(|| !self.shares.is_empty());
        let mut new_shares = SharesMap::new();
        let mut i = 0_u32;
        for (&key, &station) in &self.shares {
            i = i.wrapping_add(1);
            new_shares.insert(i, station);
            if key == self.unrestricted {
                self.unrestricted = i;
            }
        }
        self.shares = new_shares;
        crate::services_assert::assert_with(|| {
            self.shares
                .last_key_value()
                .is_some_and(|(&last, _)| self.unrestricted <= last)
        });
    }
}

/// `FlowStatMap`: flow descriptions by origin stations. Each value is a [`FlowStat`] the map
/// owns in its own allocation (see the module's contract).
#[derive(Default)]
pub struct FlowStatMap {
    entries: BTreeMap<StationID, *mut FlowStat>,
}

impl Drop for FlowStatMap {
    #[allow(unsafe_code, reason = "frees the nodes the map owns")]
    fn drop(&mut self) {
        for &node in self.entries.values() {
            // SAFETY: Each stored pointer is a live allocation the map owns.
            drop(unsafe { Box::from_raw(node) });
        }
    }
}

impl FlowStatMap {
    /// `find(origin)`: the flow from `origin`, or `None` at `end()`.
    #[allow(unsafe_code, reason = "borrows a node the map owns")]
    pub(crate) fn find(&self, origin: StationID) -> Option<&FlowStat> {
        // SAFETY: Each stored pointer is a live allocation the map owns; the borrow is the
        // map's.
        self.entries.get(&origin).map(|&node| unsafe { &*node })
    }

    /// `find(origin)`, for a flow the caller changes.
    #[allow(unsafe_code, reason = "borrows a node the map owns")]
    pub(crate) fn find_mut(&mut self, origin: StationID) -> Option<&mut FlowStat> {
        // SAFETY: As for `find`; the borrow is the map's, and unique.
        self.entries.get(&origin).map(|&node| unsafe { &mut *node })
    }

    /// The flows in origin order, as the map's iterators walk them.
    #[allow(unsafe_code, reason = "borrows the nodes the map owns")]
    pub(crate) fn iter(&self) -> impl Iterator<Item = (StationID, &FlowStat)> {
        // SAFETY: As for `find`.
        self.entries
            .iter()
            .map(|(&origin, &node)| (origin, unsafe { &*node }))
    }

    /// The flows in origin order, for flows the caller changes.
    #[allow(unsafe_code, reason = "borrows the nodes the map owns")]
    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (StationID, &mut FlowStat)> {
        // SAFETY: As for `find_mut`; each node is a distinct allocation.
        self.entries
            .iter()
            .map(|(&origin, &node)| (origin, unsafe { &mut *node }))
    }

    /// `begin()->first`: the first origin, or `None` at `end()`.
    pub(crate) fn begin(&self) -> Option<StationID> {
        self.entries.keys().next().copied()
    }

    /// `++it` from the entry at `origin`: the next origin, or `None` at `end()`.
    pub(crate) fn next(&self, origin: StationID) -> Option<StationID> {
        self.entries
            .range((Excluded(origin), Unbounded))
            .next()
            .map(|(&origin, _)| origin)
    }

    /// `emplace(origin, flow)`: move `flow` into a new entry if `origin` has none, and report
    /// whether it did.
    pub(crate) fn emplace(&mut self, origin: StationID, flow: FlowStat) -> bool {
        if let Entry::Vacant(entry) = self.entries.entry(origin) {
            entry.insert(Box::into_raw(Box::new(flow)));
            true
        } else {
            false
        }
    }

    /// `insert(other.begin(), other.end())`: copy each of `other`'s flows whose origin has no
    /// entry here.
    #[allow(unsafe_code, reason = "reads the nodes the other map owns")]
    pub(crate) fn insert(&mut self, other: &Self) {
        for (&origin, &node) in &other.entries {
            if let Entry::Vacant(entry) = self.entries.entry(origin) {
                // SAFETY: Each stored pointer is a live allocation `other` owns.
                entry.insert(Box::into_raw(Box::new(unsafe { (*node).clone() })));
            }
        }
    }

    /// `erase(origin)`.
    #[allow(unsafe_code, reason = "frees a node the map owned")]
    pub(crate) fn erase(&mut self, origin: StationID) {
        if let Some(node) = self.entries.remove(&origin) {
            // SAFETY: The removed pointer was a live allocation the map owned.
            drop(unsafe { Box::from_raw(node) });
        }
    }

    /// `FlowStatMap::GetFlow`: get the sum of all flows from this `FlowStatMap`.
    pub(crate) fn get_flow(&self) -> u32 {
        let mut ret = 0_u32;
        for (_, flow) in self.iter() {
            ret = ret.wrapping_add(*flow.get_shares().last_key_value().unwrap().0);
        }
        ret
    }

    /// `FlowStatMap::GetFlowVia`: get the sum of flows via a specific station from this
    /// `FlowStatMap`.
    pub(crate) fn get_flow_via(&self, via: StationID) -> u32 {
        let mut ret = 0_u32;
        for (_, flow) in self.iter() {
            ret = ret.wrapping_add(flow.get_share(via));
        }
        ret
    }

    /// `FlowStatMap::GetFlowFrom`: get the sum of flows from a specific station from this
    /// `FlowStatMap`.
    pub(crate) fn get_flow_from(&self, from: StationID) -> u32 {
        let Some(flow) = self.find(from) else {
            return 0;
        };
        *flow.get_shares().last_key_value().unwrap().0
    }

    /// `FlowStatMap::GetFlowFromVia`: get the flow from a specific station via a specific other
    /// station.
    pub(crate) fn get_flow_from_via(&self, from: StationID, via: StationID) -> u32 {
        let Some(flow) = self.find(from) else {
            return 0;
        };
        flow.get_share(via)
    }

    /// `FlowStatMap::AddFlow`: add some flow from `origin`, going via `via`.
    pub(crate) fn add_flow(&mut self, origin: StationID, via: StationID, flow: u32) {
        if let Some(origin_flow) = self.find_mut(origin) {
            origin_flow.change_share(via, flow.cast_signed());
            crate::services_assert::assert_with(|| !origin_flow.get_shares().is_empty());
        } else {
            self.emplace(origin, FlowStat::new(via, flow, false));
        }
    }

    /// `FlowStatMap::PassOnFlow`: pass on some flow, remembering it as invalid, for later
    /// subtraction from locally consumed flow. This is necessary because we can't have negative
    /// flows and we don't want to sort the flows before adding them up.
    pub(crate) fn pass_on_flow(&mut self, origin: StationID, via: StationID, flow: u32) {
        if let Some(prev) = self.find_mut(origin) {
            prev.change_share(via, flow.cast_signed());
            prev.change_share(StationID::INVALID, flow.cast_signed());
            crate::services_assert::assert_with(|| !prev.get_shares().is_empty());
        } else {
            let mut fs = FlowStat::new(via, flow, false);
            fs.append_share(StationID::INVALID, flow, false);
            self.emplace(origin, fs);
        }
    }

    /// `FlowStatMap::FinalizeLocalConsumption`: subtract invalid flows from locally consumed
    /// flow.
    #[allow(
        clippy::cast_possible_wrap,
        reason = "the original's `-(int)local` of a value at most `INT_MAX`"
    )]
    pub(crate) fn finalize_local_consumption(&mut self, self_station: StationID) {
        for (_, fs) in self.iter_mut() {
            let mut local = fs.get_share(StationID::INVALID);
            if local > i32::MAX.cast_unsigned() {
                // make sure it fits in an int
                fs.change_share(self_station, -i32::MAX);
                fs.change_share(StationID::INVALID, -i32::MAX);
                local = local.wrapping_sub(i32::MAX.cast_unsigned());
            }
            fs.change_share(self_station, (local as i32).wrapping_neg());
            fs.change_share(StationID::INVALID, (local as i32).wrapping_neg());

            // If the local share is used up there must be a share for some remote station.
            crate::services_assert::assert_with(|| !fs.get_shares().is_empty());
        }
    }

    /// `FlowStatMap::DeleteFlows`: delete all flows at a station for specific cargo and
    /// destination. Returns the IDs of source stations for which the complete `FlowStat`, not
    /// only a share, has been erased.
    pub(crate) fn delete_flows(&mut self, via: StationID) -> Vec<StationID> {
        let mut ret = Vec::new();
        let mut f_it = self.begin();
        while let Some(origin) = f_it {
            let s_flows = self.find_mut(origin).unwrap();
            s_flows.change_share(via, i32::MIN);
            let empty = s_flows.get_shares().is_empty();
            f_it = self.next(origin);
            if empty {
                ret.push(origin);
                self.erase(origin);
            }
        }
        ret
    }

    /// `FlowStatMap::RestrictFlows`: restrict all flows at a station for specific cargo and
    /// destination.
    pub(crate) fn restrict_flows(&mut self, via: StationID) {
        for (_, flow) in self.iter_mut() {
            flow.restrict_share(via);
        }
    }

    /// `FlowStatMap::ReleaseFlows`: release all flows at a station for specific cargo and
    /// destination. The original has no caller either.
    #[allow(
        dead_code,
        reason = "the original's `FlowStatMap::ReleaseFlows`, which has no caller"
    )]
    pub(crate) fn release_flows(&mut self, via: StationID) {
        for (_, flow) in self.iter_mut() {
            flow.release_share(via);
        }
    }
}

/// `std::pair<const uint32_t, StationID>`, a share the shares iterators return (`found` is 0 at
/// `end()`).
#[repr(C)]
#[derive(Default)]
pub struct Share {
    pub cumulative: u32,
    pub station: u16,
    pub found: u8,
}

impl Share {
    fn of(share: Option<(&u32, &StationID)>) -> Self {
        share.map_or_else(Self::default, |(&cumulative, &station)| Self {
            cumulative,
            station: station.0,
            found: 1,
        })
    }
}

/// `std::pair<const StationID, FlowStat>`, an entry the map iterators return (`found` is 0 at
/// `end()`).
#[repr(C)]
#[allow(
    clippy::struct_field_names,
    reason = "the facade's `OpenTTDCargoOrigin` members"
)]
pub struct Origin {
    pub flow: *mut FlowStat,
    pub origin: u16,
    pub found: u8,
}

impl Origin {
    fn of(entry: Option<(&StationID, &*mut FlowStat)>) -> Self {
        entry.map_or(
            Self {
                flow: std::ptr::null_mut(),
                origin: 0,
                found: 0,
            },
            |(&origin, &flow)| Self {
                flow,
                origin: origin.0,
                found: 1,
            },
        )
    }
}

/// `FlowStat(st, flow, restricted)`, owned by the caller until `openttd_rust_flow_destroy`.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol")]
pub extern "C" fn openttd_rust_flow_new(st: u16, flow: u32, restricted: u8) -> *mut FlowStat {
    Box::into_raw(Box::new(FlowStat::new(
        StationID(st),
        flow,
        restricted != 0,
    )))
}

/// `~FlowStat` of a flow `openttd_rust_flow_new` made.
/// # Safety
/// `flow` came from `openttd_rust_flow_new` and is destroyed once.
#[unsafe(no_mangle)]
#[allow(
    unsafe_code,
    reason = "the exported symbol and the caller's allocation"
)]
pub unsafe extern "C" fn openttd_rust_flow_destroy(flow: *mut FlowStat) {
    // SAFETY: The caller passes its own allocation once.
    drop(unsafe { Box::from_raw(flow) });
}

/// `FlowStat::AppendShare`.
/// # Safety
/// `flow` is a live flow no other borrow covers.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's flow")]
pub unsafe extern "C" fn openttd_rust_flow_append_share(
    flow: *mut FlowStat,
    st: u16,
    amount: u32,
    restricted: u8,
) {
    // SAFETY: The caller's live flow.
    unsafe { &mut *flow }.append_share(StationID(st), amount, restricted != 0);
}

/// `FlowStat::GetUnrestricted`.
/// # Safety
/// `flow` is a live flow.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's flow")]
pub unsafe extern "C" fn openttd_rust_flow_get_unrestricted(flow: *const FlowStat) -> u32 {
    // SAFETY: The caller's live flow.
    unsafe { &*flow }.get_unrestricted()
}

/// `GetShares()->begin()`.
/// # Safety
/// `flow` is a live flow.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's flow")]
pub unsafe extern "C" fn openttd_rust_flow_shares_begin(flow: *const FlowStat) -> Share {
    // SAFETY: The caller's live flow.
    Share::of(unsafe { &*flow }.get_shares().first_key_value())
}

/// `++it` from the share at `key` of `GetShares()`.
/// # Safety
/// `flow` is a live flow.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's flow")]
pub unsafe extern "C" fn openttd_rust_flow_shares_next(flow: *const FlowStat, key: u32) -> Share {
    // SAFETY: The caller's live flow.
    Share::of(
        unsafe { &*flow }
            .get_shares()
            .range((Excluded(key), Unbounded))
            .next(),
    )
}

/// `FlowStat::GetVia()`, drawing through `random_range`, which is `RandomRange`.
/// # Safety
/// `flow` is a live flow; `random_range` changes only the random state.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's flow")]
pub unsafe extern "C" fn openttd_rust_flow_get_via(
    flow: *const FlowStat,
    random_range: extern "C" fn(u32) -> u32,
) -> u16 {
    // SAFETY: The caller's live flow.
    unsafe { &*flow }.get_via(|limit| random_range(limit)).0
}

/// `FlowStatMap()`, owned by the caller until `openttd_rust_flow_map_destroy`.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol")]
pub extern "C" fn openttd_rust_flow_map_new() -> *mut FlowStatMap {
    Box::into_raw(Box::default())
}

/// `~FlowStatMap` of a map `openttd_rust_flow_map_new` made.
/// # Safety
/// `map` came from `openttd_rust_flow_map_new` and is destroyed once.
#[unsafe(no_mangle)]
#[allow(
    unsafe_code,
    reason = "the exported symbol and the caller's allocation"
)]
pub unsafe extern "C" fn openttd_rust_flow_map_destroy(map: *mut FlowStatMap) {
    // SAFETY: The caller passes its own allocation once.
    drop(unsafe { Box::from_raw(map) });
}

/// `begin()`.
/// # Safety
/// `map` is a live map.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's map")]
pub unsafe extern "C" fn openttd_rust_flow_map_begin(map: *const FlowStatMap) -> Origin {
    // SAFETY: The caller's live map.
    Origin::of(unsafe { &*map }.entries.first_key_value())
}

/// `find(origin)`.
/// # Safety
/// `map` is a live map.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's map")]
pub unsafe extern "C" fn openttd_rust_flow_map_find(
    map: *const FlowStatMap,
    origin: u16,
) -> Origin {
    // SAFETY: The caller's live map.
    Origin::of(unsafe { &*map }.entries.get_key_value(&StationID(origin)))
}

/// `++it` from the entry at `origin`.
/// # Safety
/// `map` is a live map.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's map")]
pub unsafe extern "C" fn openttd_rust_flow_map_next(
    map: *const FlowStatMap,
    origin: u16,
) -> Origin {
    // SAFETY: The caller's live map.
    Origin::of(
        unsafe { &*map }
            .entries
            .range((Excluded(StationID(origin)), Unbounded))
            .next(),
    )
}

/// `emplace(origin, flow)`: copy `flow`, which the caller keeps, into a new entry if `origin`
/// has none, and report whether it did.
/// # Safety
/// `map` is a live map no other borrow covers, and `flow` a live flow outside it.
#[unsafe(no_mangle)]
#[allow(
    unsafe_code,
    reason = "the exported symbol and the caller's map and flow"
)]
pub unsafe extern "C" fn openttd_rust_flow_map_insert(
    map: *mut FlowStatMap,
    origin: u16,
    flow: *const FlowStat,
) -> u8 {
    // SAFETY: The caller's live map and flow.
    let (map, flow) = unsafe { (&mut *map, &*flow) };
    u8::from(map.emplace(StationID(origin), flow.clone()))
}

/// `FlowStatMap::GetFlow`.
/// # Safety
/// `map` is a live map.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's map")]
pub unsafe extern "C" fn openttd_rust_flow_map_get_flow(map: *const FlowStatMap) -> u32 {
    // SAFETY: The caller's live map.
    unsafe { &*map }.get_flow()
}

/// `FlowStatMap::GetFlowVia`.
/// # Safety
/// `map` is a live map.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's map")]
pub unsafe extern "C" fn openttd_rust_flow_map_get_flow_via(
    map: *const FlowStatMap,
    via: u16,
) -> u32 {
    // SAFETY: The caller's live map.
    unsafe { &*map }.get_flow_via(StationID(via))
}

/// `FlowStatMap::GetFlowFrom`.
/// # Safety
/// `map` is a live map.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's map")]
pub unsafe extern "C" fn openttd_rust_flow_map_get_flow_from(
    map: *const FlowStatMap,
    from: u16,
) -> u32 {
    // SAFETY: The caller's live map.
    unsafe { &*map }.get_flow_from(StationID(from))
}

/// `FlowStatMap::GetFlowFromVia`.
/// # Safety
/// `map` is a live map.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's map")]
pub unsafe extern "C" fn openttd_rust_flow_map_get_flow_from_via(
    map: *const FlowStatMap,
    from: u16,
    via: u16,
) -> u32 {
    // SAFETY: The caller's live map.
    unsafe { &*map }.get_flow_from_via(StationID(from), StationID(via))
}

/// The finished job, the station tables and the three `GoodsEntry` members the station pool
/// stores in C++ (#634), each taking the goods entry.
#[repr(C)]
pub struct Services {
    /// `GoodsEntry::ClearData`.
    pub clear_data: extern "C" fn(*mut crate::station_base::GoodsEntry),
    /// `GoodsEntry::GetData().IsEmpty()`.
    pub data_is_empty: extern "C" fn(*const crate::station_base::GoodsEntry) -> bool,
    /// `&GoodsEntry::GetOrCreateData().flows`.
    pub get_or_create_data_flows:
        extern "C" fn(*mut crate::station_base::GoodsEntry) -> *mut FlowStatMap,
    pub job: *mut crate::linkgraph::job::JobRecord,
    pub links: *const crate::station_service::Links,
    pub stations: *const crate::station_service::World,
}

/// `LinkGraphJob::EraseFlows`: erase all flows originating at a specific station.
#[allow(
    unsafe_code,
    reason = "the job's node maps, private to the game thread after the join"
)]
fn erase_flows(job: &crate::linkgraph::job::JobRecord, size: u16, from: StationID) {
    for node_id in 0..size {
        let flows = job.nodes[usize::from(node_id)].flows;
        // SAFETY: Each job node owns its map, which no other borrow covers after the join.
        unsafe { &mut *flows }.erase(from);
    }
}

/// `LinkGraphJob::~LinkGraphJob` after the join: apply the job's flows to the stations. The
/// join, the `CleaningPool()` return and the assertion that the job was not aborted stay in the
/// native destructor.
/// # Safety
/// `s` holds the joined job, private to the game thread, and the live station tables; the call
/// runs under the game-state mutex.
#[allow(
    unsafe_code,
    reason = "the joined job, the live graph and each station's flows, borrowed between calls"
)]
unsafe fn link_graph_job_destruct(s: &Services) {
    // SAFETY: The caller's live tables and joined job.
    let (job, stations, links) = unsafe { (&*s.job, &*s.stations, &*s.links) };

    // Link graph has been merged into another one.
    let lg = links.graphs.get(job.graph_index);
    if lg.is_null() {
        return;
    }

    // SAFETY: The job's immutable snapshot, which the job owns.
    let snapshot = unsafe { &*job.graph };
    let cargo = CargoType(snapshot.cargo);
    let size = snapshot.size();
    let mut edge_begin = 0;
    for node_id in 0..size {
        let from = &snapshot.nodes[usize::from(node_id)];
        let station = StationID(from.station);
        let first_edge = edge_begin;
        edge_begin += from.edges.len();

        // The station can have been deleted. Remove all flows originating from it then.
        let st = (stations.get)(station.0);
        if st.is_null() || !(stations.is_station)(st) {
            erase_flows(job, size, station);
            continue;
        }

        // Link graph merging and station deletion may change around IDs. Make sure that
        // everything is still consistent or ignore it otherwise.
        if (links.graphs.graph_id)(st, cargo.0) != job.graph_index.0
            || (links.graphs.node)(st, cargo.0) != node_id
        {
            erase_flows(job, size, station);
            continue;
        }
        let ge = (links.goods_entry)(st, cargo.0).cast_mut();

        let flows = job.nodes[usize::from(node_id)].flows;
        // SAFETY: The job node's map, which no other borrow covers after the join.
        let flows = unsafe { &mut *flows };
        let geflows = (s.get_or_create_data_flows)(ge);

        for (index, edge) in from.edges.iter().enumerate() {
            if job.edge_flows[first_edge + index] == 0 {
                continue;
            }
            let dest_id = edge.dest_node;
            let to = StationID(snapshot.nodes[usize::from(dest_id)].station);
            let st2 = (stations.get)(to.0);
            // SAFETY: The live graph, which no station leaf changes.
            let live_edge = || unsafe { &*lg }.nodes[usize::from(node_id)].edge(dest_id);
            if st2.is_null()
                || !(stations.is_station)(st2)
                || (links.graphs.graph_id)(st2, cargo.0) != job.graph_index.0
                || (links.graphs.node)(st2, cargo.0) != dest_id
                || live_edge()
                    .is_none_or(|edge| edge.last_update() == crate::timer_game_common::INVALID_DATE)
            {
                // Edge has been removed. Delete flows.
                let erased = flows.delete_flows(to);
                // Delete old flows for source stations which have been deleted from the new
                // flows. This avoids flow cycles between old and new flows.
                for station in erased {
                    // SAFETY: The station's flows, borrowed for this call.
                    unsafe { &mut *geflows }.erase(station);
                }
            } else if live_edge().unwrap().last_unrestricted_update
                == crate::timer_game_common::INVALID_DATE
            {
                // Edge is fully restricted.
                flows.restrict_flows(to);
            }
        }

        // Swap shares and invalidate ones that are completely deleted. Don't really delete them
        // as we could then end up with unroutable cargo somewhere. Do delete them and also
        // reroute relevant cargo if automatic distribution has been turned off for that cargo.
        // SAFETY: The station's flows, borrowed for this call.
        let mut it = unsafe { &*geflows }.begin();
        while let Some(origin) = it {
            if let Some(new) = flows.find_mut(origin) {
                // SAFETY: The station's flows, borrowed until the erase.
                let live = unsafe { &mut *geflows };
                live.find_mut(origin).unwrap().swap_shares(new);
                flows.erase(origin);
                it = live.next(origin);
            } else if (links.automatic_distribution)(cargo.0) {
                // SAFETY: The station's flows, borrowed for this call.
                let live = unsafe { &mut *geflows };
                live.find_mut(origin).unwrap().invalidate();
                it = live.next(origin);
            } else {
                let mut shares = FlowStat::new(StationID::INVALID, 1, false);
                {
                    // SAFETY: The station's flows, borrowed until the erase, which ends before
                    // rerouting reads them again.
                    let live = unsafe { &mut *geflows };
                    live.find_mut(origin).unwrap().swap_shares(&mut shares);
                    it = live.next(origin);
                    live.erase(origin);
                }
                for &via in shares.get_shares().values() {
                    // SAFETY: The live station, its tables and no borrow of its flows.
                    unsafe {
                        crate::station_service::reroute_cargo(
                            st, cargo, via, station, stations, links,
                        );
                    }
                }
            }
        }
        // SAFETY: The station's flows, borrowed for this call.
        unsafe { &mut *geflows }.insert(flows);
        if (s.data_is_empty)(ge) {
            (s.clear_data)(ge);
        }
        crate::window_func::invalidate_window_data(
            crate::window_type::WindowClass::WC_STATION_VIEW,
            station.into(),
            i32::from(cargo.0),
            false,
        );
    }
}

/// `LinkGraphJob::~LinkGraphJob` after the join, through [`link_graph_job_destruct`].
/// # Safety
/// As for [`link_graph_job_destruct`].
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's tables")]
pub unsafe extern "C" fn openttd_rust_flow_apply(services: *const Services) {
    // SAFETY: The caller's tables.
    unsafe { link_graph_job_destruct(&*services) }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDCargoShare" ["probe"] => Share {
        cumulative,
        station,
        found,
    };
    "OpenTTDCargoOrigin" ["probe"] => Origin {
        flow,
        origin,
        found,
    };
    "OpenTTDCargoFlowServices" ["probe"] => Services {
        clear_data,
        data_is_empty,
        get_or_create_data_flows,
        job,
        links,
        stations,
    };
}

#[cfg(test)]
#[allow(
    unsafe_code,
    reason = "the fixtures' raw job, graph, map and cargo list records"
)]
mod tests {
    use super::*;
    use crate::linkgraph::job::{JobNode, JobRecord, SnapshotSettings};
    use crate::linkgraph::live::{self, Graph};
    use crate::linkgraph_type::{EdgeUpdateModes, LinkGraphID};
    use crate::pool::{PoolDescriptor, PoolSlot};
    use crate::station::BaseStation;
    use crate::station_base::GoodsEntry;
    use crate::window_func::testing::Effect;
    use crate::window_type::{WindowClass, WindowNumber};
    use std::cell::{Cell, RefCell};

    fn shares<const N: usize>(entries: [(u32, u16); N]) -> SharesMap {
        entries
            .into_iter()
            .map(|(key, station)| (key, StationID(station)))
            .collect()
    }

    #[test]
    fn source_restriction_tail_and_old_map_release_are_preserved() {
        let mut flow = FlowStat::new(StationID(1), 10, false);
        flow.append_share(StationID(2), 20, false);
        flow.restrict_share(StationID(1));
        assert_eq!(flow.shares, shares([(20, 2), (40, 1)]));
        assert_eq!(flow.unrestricted, 20);
        flow.change_share(StationID(3), 5);
        assert_eq!(flow.shares, shares([(20, 2), (40, 1), (45, 3)]));
        assert_eq!(flow.unrestricted, 20);
        flow.release_share(StationID(1));
        assert_eq!(flow.shares, shares([(20, 1), (40, 2), (45, 3)]));
        assert_eq!(flow.unrestricted, 40);
        flow.invalidate();
        assert_eq!(flow.shares, shares([(1, 1), (2, 2), (3, 3)]));
        assert_eq!(flow.unrestricted, 2);
    }

    #[test]
    fn shares_keep_unsigned_key_wrap_and_empty_origin_state() {
        let mut flow = FlowStat::new(StationID(1), u32::MAX, false);
        flow.change_share(StationID(2), 2);
        assert_eq!(flow.shares, shares([(1, 2), (u32::MAX, 1)]));
        assert_eq!(flow.unrestricted, 1);
        let mut flow = FlowStat::new(StationID(1), 1, false);
        flow.change_share(StationID(1), -1);
        assert!(flow.shares.is_empty());
        assert_eq!(flow.unrestricted, 0);
    }

    #[test]
    fn excluded_selection_draws_once_for_each_hit() {
        let mut draws = 0;
        let mut zero = |_: u32| {
            draws += 1;
            0
        };
        let mut flow = FlowStat::new(StationID(1), 10, false);
        flow.append_share(StationID(2), 20, false);
        flow.append_share(StationID(3), 30, false);
        assert_eq!(
            flow.get_via_excluded(StationID(1), StationID(2), &mut zero),
            StationID(3)
        );
        assert_eq!(draws, 3);
        flow.change_share(StationID(3), i32::MIN);
        draws = 0;
        let mut zero = |_: u32| {
            draws += 1;
            0
        };
        assert_eq!(
            flow.get_via_excluded(StationID(1), StationID(2), &mut zero),
            StationID::INVALID
        );
        assert_eq!(draws, 2);
        let restricted = FlowStat::new(StationID(1), 5, true);
        draws = 0;
        let mut zero = |_: u32| {
            draws += 1;
            0
        };
        assert_eq!(restricted.get_via(&mut zero), StationID::INVALID);
        assert_eq!(
            restricted.get_via_excluded(StationID(0), StationID(0), &mut zero),
            StationID::INVALID
        );
        let mut is_restricted = false;
        assert_eq!(
            restricted.get_via_with_restricted(&mut is_restricted, &mut zero),
            StationID(1)
        );
        assert!(is_restricted);
        assert_eq!(draws, 1);
    }

    #[test]
    fn scaling_wraps_before_division_and_nodes_keep_identity() {
        let mut flow = FlowStat::new(StationID(1), u32::MAX, false);
        flow.scale_to_monthly(30);
        assert_eq!(flow.unrestricted, u32::MAX.wrapping_mul(30) / 30);
        let mut map = FlowStatMap::default();
        map.add_flow(StationID(10), StationID(1), 5);
        let first = map.entries[&StationID(10)];
        map.add_flow(StationID(5), StationID(2), 8);
        map.add_flow(StationID(20), StationID(3), 9);
        map.erase(StationID(5));
        assert_eq!(map.entries[&StationID(10)], first);
        let mut copy = FlowStatMap::default();
        copy.insert(&map);
        assert_ne!(copy.entries[&StationID(10)], first);
        assert_eq!(copy.get_flow_from_via(StationID(10), StationID(1)), 5);
    }

    #[test]
    fn passed_flow_is_subtracted_from_local_consumption_in_two_steps() {
        // Values worked from the original's PassOnFlow, ChangeShare and
        // FinalizeLocalConsumption; station 2 consumes locally.
        let mut map = FlowStatMap::default();
        map.pass_on_flow(StationID(1), StationID(2), 5);
        let flow = map.find(StationID(1)).unwrap();
        assert_eq!(
            (&flow.shares, flow.unrestricted),
            (&shares([(5, 2), (10, u16::MAX)]), 10)
        );
        map.pass_on_flow(StationID(1), StationID(3), 4);
        let flow = map.find(StationID(1)).unwrap();
        assert_eq!(
            (&flow.shares, flow.unrestricted),
            (&shares([(5, 2), (14, u16::MAX), (18, 3)]), 18)
        );
        // A local share above INT_MAX is subtracted in two steps.
        let mut large = FlowStat::new(StationID(2), 0x7000_0000, false);
        large.append_share(StationID::INVALID, 0x8000_0001, false);
        large.append_share(StationID(6), 3, false);
        assert!(map.emplace(StationID(9), large));
        map.finalize_local_consumption(StationID(2));
        let flow = map.find(StationID(1)).unwrap();
        assert_eq!((&flow.shares, flow.unrestricted), (&shares([(4, 3)]), 4));
        let flow = map.find(StationID(9)).unwrap();
        assert_eq!((&flow.shares, flow.unrestricted), (&shares([(3, 6)]), 3));
    }

    /// What the application calls on the native side, in its order.
    #[derive(Debug, PartialEq)]
    enum Event {
        GetOrCreateDataFlows(StationID),
        FlowMap(StationID),
        DataIsEmpty(StationID),
        ClearData(StationID),
        Window(WindowClass, WindowNumber, i32, bool),
    }
    const STATIONS: usize = 4;
    thread_local! {
        static LIVE: Cell<[*mut FlowStatMap; STATIONS]> = const { Cell::new([std::ptr::null_mut(); STATIONS]) };
        static EVENTS: RefCell<Vec<Event>> = const { RefCell::new(Vec::new()) };
        static MANUAL: Cell<bool> = const { Cell::new(false) };
        static CARGO_LIST: Cell<*mut crate::cargo_storage::List> = const { Cell::new(std::ptr::null_mut()) };
        static OWNER: Cell<*mut crate::station_service::Service> = const { Cell::new(std::ptr::null_mut()) };
    }
    fn record(event: Event) {
        EVENTS.with(|events| events.borrow_mut().push(event));
    }
    fn live(station: StationID) -> *mut FlowStatMap {
        LIVE.get()[usize::from(station.0 - 100)]
    }
    /// A goods entry is its station's live map; this returns the station.
    fn station_of(ge: *const GoodsEntry) -> StationID {
        let index = LIVE
            .get()
            .iter()
            .position(|&map| map.cast_const().cast() == ge)
            .unwrap();
        StationID(100 + index as u16)
    }
    extern "C" fn clear_data(ge: *mut GoodsEntry) {
        record(Event::ClearData(station_of(ge)));
    }
    extern "C" fn data_is_empty(ge: *const GoodsEntry) -> bool {
        record(Event::DataIsEmpty(station_of(ge)));
        unsafe { &*ge.cast::<FlowStatMap>() }.begin().is_none()
    }
    extern "C" fn get_or_create_data_flows(ge: *mut GoodsEntry) -> *mut FlowStatMap {
        record(Event::GetOrCreateDataFlows(station_of(ge)));
        ge.cast()
    }
    extern "C" fn flow_map(ge: *const std::ffi::c_void) -> *mut FlowStatMap {
        record(Event::FlowMap(station_of(ge.cast())));
        ge.cast_mut().cast()
    }
    fn window(effect: &Effect) {
        if let &Effect::InvalidateWindowData(cls, number, data, gui_scope) = effect {
            record(Event::Window(cls, number, data, gui_scope));
        }
    }
    /// Station `id` is the address `id + 1`; station 102 was deleted.
    extern "C" fn station_get(id: u16) -> *mut BaseStation {
        if id == 102 {
            std::ptr::null_mut()
        } else {
            std::ptr::without_provenance_mut(usize::from(id) + 1)
        }
    }
    fn id(st: *mut BaseStation) -> StationID {
        StationID((st.addr() - 1) as u16)
    }
    extern "C" fn is_station(_: *mut BaseStation) -> bool {
        true
    }
    extern "C" fn graph_id(_: *mut BaseStation, _: u8) -> u16 {
        42
    }
    extern "C" fn node_id(st: *mut BaseStation, _: u8) -> u16 {
        id(st).0 - 100
    }
    extern "C" fn goods_entry(st: *mut BaseStation, _: u8) -> *const GoodsEntry {
        live(id(st)).cast_const().cast()
    }
    extern "C" fn automatic_distribution(_: u8) -> bool {
        !MANUAL.get()
    }
    extern "C" fn cargo_owner(st: *mut BaseStation, _: u8) -> *const crate::cargo_storage::List {
        if id(st) == StationID(100) {
            CARGO_LIST.get()
        } else {
            std::ptr::null()
        }
    }
    extern "C" fn station_owner(_: *mut BaseStation) -> *mut crate::station_service::Service {
        OWNER.get()
    }

    /// A pool descriptor with 43 slots, holding `graph` at index 42.
    fn graph_pool(graph: *mut Graph) -> *mut PoolDescriptor {
        let mut slots = vec![
            PoolSlot {
                shell: std::ptr::null_mut(),
                record: std::ptr::null_mut(),
            };
            43
        ];
        slots[42].record = graph.cast();
        Box::into_raw(Box::new(PoolDescriptor {
            slots: crate::industrytype::NativeVec::from_vec(slots),
            used_bitmap: crate::industrytype::NativeVec::new(),
            first_free: 0,
            first_unused: 43,
            items: 0,
            checked: 0,
            cleaning: false,
            alloc_cache: std::ptr::null_mut(),
        }))
    }
    fn graph_services(pool: *mut PoolDescriptor) -> &'static live::Services {
        Box::leak(Box::new(live::Services {
            pool: Box::leak(Box::new(pool.cast_const())),
            graph_id,
            node: node_id,
            ..crate::test_tables::LINK_GRAPHS
        }))
    }

    /// A job of graph 42 over stations 100 to 103, at nodes 0 to 3; node 0 has one
    /// annotated edge, to node 2, carrying 10. The snapshot is also the live graph.
    fn job_fixture(flows: [FlowStatMap; STATIONS]) -> (*mut Graph, *mut JobRecord) {
        let graph = live::openttd_rust_live_graph_new(0);
        let settings = SnapshotSettings {
            recalc_time: 60,
            recalc_interval: 4,
            distribution_pax: 1,
            distribution_mail: 1,
            distribution_armoured: 1,
            distribution_default: 1,
            accuracy: 2,
            demand_size: 100,
            demand_distance: 100,
            short_path_saturation: 80,
        };
        unsafe {
            (*graph).last_compression = 0;
            live::openttd_rust_live_graph_init(graph, STATIONS as u32);
            for node in 0..STATIONS as u16 {
                (&mut (*graph).nodes)[usize::from(node)].station = 100 + node;
            }
            live::openttd_rust_live_node_add_edge(
                graph,
                0,
                2,
                20,
                0,
                0,
                EdgeUpdateModes::UNRESTRICTED,
            );
        }
        let job = unsafe {
            crate::linkgraph::job::openttd_rust_linkgraph_job_new(
                graph,
                LinkGraphID(42),
                &raw const settings,
            )
        };
        unsafe {
            (*job).nodes = crate::industrytype::NativeVec::from_vec(
                flows
                    .map(|flows| JobNode {
                        flows: Box::into_raw(Box::new(flows)),
                    })
                    .into(),
            );
            (*job).edge_flows = crate::industrytype::NativeVec::from_vec(vec![10]);
        }
        (graph, job)
    }
    fn map<const N: usize>(flows: [(u16, u16, u32); N]) -> FlowStatMap {
        let mut map = FlowStatMap::default();
        for (origin, via, flow) in flows {
            map.add_flow(StationID(origin), StationID(via), flow);
        }
        map
    }

    #[test]
    fn live_application_erases_before_rerouting_and_preserves_node_order() {
        for manual in [false, true] {
            MANUAL.set(manual);
            EVENTS.with(|events| events.borrow_mut().clear());
            crate::window_func::testing::take();
            crate::window_func::testing::observe(Some(window));
            LIVE.set([
                Box::into_raw(Box::new(map([(1, 105, 9), (7, 106, 4), (7, 107, 6)]))),
                Box::into_raw(Box::default()),
                Box::into_raw(Box::default()),
                Box::into_raw(Box::default()),
            ]);
            // Station 100 holds one packet from origin 7 for each of the next hops 106, 107
            // and 108, of 1, 2 and 3 units.
            let list = crate::cargo_storage::tests::list(false);
            for (count, via) in [(1, 106), (2, 107), (3, 108)] {
                unsafe {
                    crate::cargo_storage::tests::append_to(
                        list,
                        crate::cargo_storage::tests::station_cargo(count, 7, 0),
                        via,
                    );
                }
            }
            CARGO_LIST.set(list);
            OWNER.set(crate::station_service::openttd_rust_station_service_new());
            let storage = crate::cargo_storage::Services {
                flow_map,
                ..crate::cargo_storage::tests::services()
            };
            let resolver =
                crate::newgrf_station::Services::cargo_fixture(&raw const storage, cargo_owner);
            let (graph, job) = job_fixture([
                map([(1, 102, 10), (3, 100, 4)]),
                map([(102, 101, 8)]),
                FlowStatMap::default(),
                FlowStatMap::default(),
            ]);
            let pool = graph_pool(graph);
            let graphs = graph_services(pool);
            let stations = crate::station_service::World {
                graphs,
                get: station_get,
                is_station,
                owner: station_owner,
                ..crate::test_tables::STATION_WORLD
            };
            let links = crate::station_service::Links {
                graphs,
                automatic_distribution,
                goods_entry,
                station_resolver: &raw const resolver,
                ..crate::test_tables::STATION_LINKS
            };
            let services = Services {
                clear_data,
                data_is_empty,
                get_or_create_data_flows,
                job,
                links: &raw const links,
                stations: &raw const stations,
            };
            unsafe {
                openttd_rust_flow_apply(&raw const services);
            }
            crate::window_func::testing::observe(None);

            let live0 = unsafe { &*live(StationID(100)) };
            // Origin 1's only share went to deleted station 102.
            assert!(live0.find(StationID(1)).is_none());
            assert!(live0.find(StationID(3)).is_some());
            // Node 1 already applied its flow when deleted node 2 removes origin 102.
            assert!(
                unsafe { &*(&(*job).nodes)[1].flows }
                    .find(StationID(102))
                    .is_none()
            );
            assert!(
                unsafe { &*live(StationID(101)) }
                    .find(StationID(102))
                    .is_some()
            );
            let window = |station: StationID| {
                Event::Window(
                    WindowClass::WC_STATION_VIEW,
                    WindowNumber::from(station),
                    0,
                    false,
                )
            };
            let mut expected = vec![Event::GetOrCreateDataFlows(StationID(100))];
            if manual {
                // Both reroutes find no flow from origin 7, so both packets lose their hop.
                expected.extend([
                    Event::FlowMap(StationID(100)),
                    Event::FlowMap(StationID(100)),
                ]);
                assert!(live0.find(StationID(7)).is_none());
                let mut moved = unsafe { crate::cargo_storage::tests::counts_and_keys(list) };
                moved.sort_unstable();
                assert_eq!(moved, [(1, u16::MAX), (2, u16::MAX), (3, 108)]);
            } else {
                let flow = live0.find(StationID(7)).unwrap();
                assert_eq!(flow.shares, shares([(1, 106), (2, 107)]));
                let mut kept = unsafe { crate::cargo_storage::tests::counts_and_keys(list) };
                kept.sort_unstable();
                assert_eq!(kept, [(1, 106), (2, 107), (3, 108)]);
            }
            expected.extend([
                Event::DataIsEmpty(StationID(100)),
                window(StationID(100)),
                Event::GetOrCreateDataFlows(StationID(101)),
                Event::DataIsEmpty(StationID(101)),
                window(StationID(101)),
                Event::GetOrCreateDataFlows(StationID(103)),
                Event::DataIsEmpty(StationID(103)),
                Event::ClearData(StationID(103)),
                window(StationID(103)),
            ]);
            EVENTS.with(|events| assert_eq!(*events.borrow(), expected));

            unsafe {
                crate::linkgraph::job::openttd_rust_linkgraph_job_delete(job);
                live::openttd_rust_live_graph_delete(graph);
                drop(Box::from_raw(pool));
                crate::cargo_storage::tests::clean(list);
                crate::station_service::openttd_rust_station_service_destroy(OWNER.get());
                for map in LIVE.get() {
                    drop(Box::from_raw(map));
                }
            }
        }
    }

    #[test]
    fn merged_graph_returns_before_reading_any_station() {
        let (graph, job) = job_fixture([
            map([(1, 102, 10)]),
            FlowStatMap::default(),
            FlowStatMap::default(),
            FlowStatMap::default(),
        ]);
        // The pool holds no graph at the job's index: it was merged into another one.
        let pool = graph_pool(std::ptr::null_mut());
        let graphs = graph_services(pool);
        let stations = crate::station_service::World {
            graphs,
            ..crate::test_tables::STATION_WORLD
        };
        let links = crate::station_service::Links {
            graphs,
            ..crate::test_tables::STATION_LINKS
        };
        extern "C" fn trap_goods(_: *mut GoodsEntry) {
            unreachable!()
        }
        extern "C" fn trap_empty(_: *const GoodsEntry) -> bool {
            unreachable!()
        }
        extern "C" fn trap_flows(_: *mut GoodsEntry) -> *mut FlowStatMap {
            unreachable!()
        }
        let services = Services {
            clear_data: trap_goods,
            data_is_empty: trap_empty,
            get_or_create_data_flows: trap_flows,
            job,
            links: &raw const links,
            stations: &raw const stations,
        };
        unsafe {
            openttd_rust_flow_apply(&raw const services);
            // The job's flows are untouched.
            assert!((*(&(*job).nodes)[0].flows).find(StationID(1)).is_some());
            crate::linkgraph::job::openttd_rust_linkgraph_job_delete(job);
            live::openttd_rust_live_graph_delete(graph);
            drop(Box::from_raw(pool));
        }
    }
}
