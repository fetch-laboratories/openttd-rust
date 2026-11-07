/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Direct history operations on canonical town/industry records and immutable native ranges.

/// By-value descriptor fields; tokens identify C++ objects but are never dereferenced here.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Descriptor {
    /// Child descriptor identity, zero when absent.
    pub child: usize,
    /// Number of reportable periods.
    pub periods: u8,
    /// Number of stored records.
    pub records: u8,
    /// First stored index.
    pub first: u8,
    /// Exclusive last stored index.
    pub last: u8,
    /// Number of child records in one period.
    pub division: u8,
    /// Number of initial periods in one period.
    pub total_division: u8,
    /// Child period count, zero when absent.
    pub child_periods: u8,
    /// Child division, zero when absent.
    pub child_division: u8,
}

/// One structural instruction; no typed data or callback crosses the ABI.
#[repr(C)]
#[derive(Default)]
pub struct Step {
    /// 0 done, 1 describe, 2 shift, 3 copy, 4 reset, 5 rotate reduction,
    /// 6 construct scratch, 7 child query, 8 query reduction, 9 leaf, 10 fatal.
    pub kind: u8,
    /// Number of typed inputs for a reduction or scratch phase.
    pub count: u8,
    /// Source index or shift beginning.
    pub first: u32,
    /// Exclusive shift end.
    pub last: u32,
    /// Destination index in history or the current scratch array.
    pub target: u32,
    /// Descriptor identity for a describe instruction only.
    pub token: usize,
    /// Final mask for update or boolean validity otherwise.
    pub value: u64,
}

#[derive(Clone, Copy)]
enum Stage {
    Enter,
    AfterChild,
    Leading,
    Reset,
    Pop,
    AwaitPhase,
    Children,
    Complete,
}

struct Frame {
    token: usize,
    descriptor: Option<Descriptor>,
    age: u32,
    stage: Stage,
    start: u32,
    child_index: u32,
    valid: bool,
}

impl Frame {
    fn new(token: usize, age: u32) -> Self {
        Self {
            token,
            descriptor: None,
            age,
            stage: Stage::Enter,
            start: 0,
            child_index: 0,
            valid: false,
        }
    }
}

fn has_bit(mask: u64, slot: u32) -> bool {
    // HasBit's C++ uint8 parameter narrows the unsigned index before shifting.
    let slot = slot.to_le_bytes()[0];
    mask & (1_u64 << slot) != 0
}

fn aggregate(d: Descriptor, age: u32) -> bool {
    // C++ first subtracts promoted signed ints, then converts to native uint.
    let threshold = (i32::from(d.child_periods) - i32::from(d.division)).to_ne_bytes();
    d.child != 0 && age.wrapping_mul(u32::from(d.division)) < u32::from_ne_bytes(threshold)
}

fn start(d: Descriptor, age: u32, month: u32) -> u32 {
    age.wrapping_mul(u32::from(d.division))
        .wrapping_add((month / u32::from(d.child_division)) % u32::from(d.division))
}

fn slot(d: Descriptor, age: u32) -> u32 {
    let offset = if d.child == 0 {
        0
    } else {
        (u32::from(d.child_periods) / u32::from(d.division)).wrapping_sub(1)
    };
    u32::from(d.first).wrapping_add(age).wrapping_sub(offset)
}

pub(crate) fn update(mask: u64, d: Descriptor, month: u32) -> u64 {
    update_record(mask, d, month).unwrap_or(mask)
}

fn update_record(mask: u64, d: Descriptor, month: u32) -> Option<u64> {
    if has_bit(mask, u32::from(d.last).wrapping_sub(1))
        || !month.is_multiple_of(u32::from(d.total_division))
        || (d.division != 1
            && !has_bit(mask, u32::from(d.first).wrapping_sub(u32::from(d.division))))
    {
        return None;
    }
    let window = (1_u64 << d.records) - 1;
    // GB returns uint (32 bits) even when its input is uint64; retain truncation.
    let extracted = (mask >> d.first) & window;
    let low = extracted.to_le_bytes();
    let extracted = u32::from_le_bytes([low[0], low[1], low[2], low[3]]);
    let shifted = (u64::from(extracted) << 1) | 1;
    // SB clears the window, then ORs the supplied bits without a second mask.
    Some((mask & !(window << d.first)) | (shifted << d.first))
}

/// Rust-owned scalar state; C++ owns every history/scratch element and operation.
pub struct Engine {
    mode: u8,
    mask: u64,
    month: u32,
    frames: Vec<Frame>,
    final_valid: bool,
}

impl Engine {
    pub fn new(mode: u8, token: usize, mask: u64, age: u32, month: u32) -> Self {
        assert!(mode <= 3);
        Self {
            mode,
            mask,
            month,
            frames: vec![Frame::new(token, age)],
            final_valid: false,
        }
    }

    pub fn describe(&mut self, descriptor: Descriptor) {
        let frame = self.frames.last_mut().unwrap();
        assert!(frame.descriptor.is_none());
        frame.descriptor = Some(descriptor);
    }

    pub fn phase(&mut self, month: u32) {
        assert_eq!(self.mode, 3);
        let frame = self.frames.last_mut().unwrap();
        assert!(matches!(frame.stage, Stage::AwaitPhase));
        frame.start = start(frame.descriptor.unwrap(), frame.age, month);
        frame.stage = Stage::Children;
    }

    pub fn complete_query(&mut self) -> bool {
        assert_eq!(self.mode, 3);
        let frame = self.frames.pop().unwrap();
        assert!(matches!(frame.stage, Stage::Complete));
        if let Some(parent) = self.frames.last_mut() {
            parent.valid |= frame.valid;
        } else {
            self.final_valid = frame.valid;
        }
        frame.valid
    }

    fn rotation_step(&mut self, index: usize, d: Descriptor) -> Option<Step> {
        match self.frames[index].stage {
            Stage::Enter => {
                self.frames[index].stage = Stage::AfterChild;
                if d.child != 0 {
                    self.frames.push(Frame::new(d.child, 0));
                }
            }
            Stage::AfterChild if self.mode == 0 => {
                self.mask = update(self.mask, d, self.month);
                self.frames.pop();
            }
            Stage::AfterChild => {
                if !self.month.is_multiple_of(u32::from(d.total_division)) {
                    self.frames.pop();
                    return None;
                }
                self.frames[index].stage = Stage::Leading;
                return Some(Step {
                    kind: 2,
                    first: u32::from(d.first),
                    last: u32::from(d.last),
                    ..Step::default()
                });
            }
            Stage::Leading => {
                if d.total_division == 1 {
                    self.frames[index].stage = Stage::Reset;
                    return Some(Step {
                        kind: 3,
                        first: u32::from(d.first).wrapping_sub(1),
                        target: u32::from(d.first),
                        ..Step::default()
                    });
                }
                self.frames[index].stage = Stage::Pop;
                let source = u32::from(d.first).wrapping_sub(u32::from(d.division));
                if has_bit(self.mask, source) {
                    return Some(Step {
                        kind: 5,
                        count: d.division,
                        first: source,
                        target: u32::from(d.first),
                        ..Step::default()
                    });
                }
            }
            Stage::Reset => {
                self.frames[index].stage = Stage::Pop;
                return Some(Step {
                    kind: 4,
                    ..Step::default()
                });
            }
            Stage::Pop => {
                self.frames.pop();
            }
            _ => unreachable!("invalid rotation/update stage"),
        }
        None
    }

    fn query_step(&mut self, index: usize, d: Descriptor, age: u32) -> Step {
        match self.frames[index].stage {
            Stage::Enter => {
                if aggregate(d, age) {
                    self.frames[index].stage = Stage::AwaitPhase;
                    return Step {
                        kind: 6,
                        count: d.division,
                        ..Step::default()
                    };
                }
                if age < u32::from(d.periods) {
                    let source = slot(d, age);
                    self.frames[index].valid = has_bit(self.mask, source);
                    self.frames[index].stage = Stage::Complete;
                    return Step {
                        kind: 9,
                        first: source,
                        ..Step::default()
                    };
                }
                Step {
                    kind: 10,
                    ..Step::default()
                }
            }
            Stage::Children => {
                let child_index = self.frames[index].child_index;
                if child_index < u32::from(d.division) {
                    self.frames[index].child_index += 1;
                    let age = self.frames[index].start.wrapping_add(child_index);
                    self.frames.push(Frame::new(d.child, age));
                    return Step {
                        kind: 7,
                        target: child_index,
                        ..Step::default()
                    };
                }
                self.frames[index].stage = Stage::Complete;
                Step {
                    kind: 8,
                    count: d.division,
                    ..Step::default()
                }
            }
            _ => unreachable!("query operation was not completed"),
        }
    }

    pub fn next(&mut self) -> Step {
        loop {
            let Some(index) = self.frames.len().checked_sub(1) else {
                return Step {
                    value: if self.mode == 0 {
                        self.mask
                    } else {
                        u64::from(self.final_valid)
                    },
                    ..Step::default()
                };
            };
            let Some(d) = self.frames[index].descriptor else {
                return Step {
                    kind: 1,
                    token: self.frames[index].token,
                    ..Step::default()
                };
            };
            if matches!(self.mode, 0 | 2) {
                if let Some(step) = self.rotation_step(index, d) {
                    return step;
                }
                continue;
            }
            let age = self.frames[index].age;
            if self.mode == 1 {
                if aggregate(d, age) {
                    self.frames[index] = Frame::new(d.child, start(d, age, self.month));
                    continue;
                }
                self.final_valid = age < u32::from(d.periods) && has_bit(self.mask, slot(d, age));
                self.frames.clear();
                continue;
            }
            return self.query_step(index, d, age);
        }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "HistoryRange" ["probe"] => HistoryRange {
        hr, periods, records, first, last, division, total_division,
    };
    "OpenTTDRustHistoryQuery" ["probe"] => HistoryQuery { is_valid, in_range };

}

/// Native immutable `HistoryRange` chain from `misc/history_type.hpp`.
#[repr(C)]
pub struct HistoryRange {
    /// Previous subdivision, or null for the initial range.
    pub hr: *const Self,
    /// Number of periods for this range.
    pub periods: u8,
    /// Number of records needed for this range.
    pub records: u8,
    /// Index of first element in history data.
    pub first: u8,
    /// Index past the last element in history data.
    pub last: u8,
    /// Number of divisions of the previous history range.
    pub division: u8,
    /// Number of divisions of the initial history range.
    pub total_division: u8,
}

/// Native fatal handling and data validity are separate query outcomes.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HistoryQuery {
    /// Whether any queried child has valid data.
    pub is_valid: bool,
    /// Whether the original query returns rather than reaching `NOT_REACHED`.
    pub in_range: bool,
}

impl From<Option<bool>> for HistoryQuery {
    fn from(value: Option<bool>) -> Self {
        Self {
            is_valid: value.unwrap_or(false),
            in_range: value.is_some(),
        }
    }
}

// Maximum subdivision among the original month, quarter and year ranges.
const HISTORY_MAX_DIVISION: usize = 4;

trait HistoryElement: Copy + Default {
    fn sum(history: &[Self]) -> Self;
}

impl HistoryElement for crate::industry::Pair {
    fn sum(history: &[Self]) -> Self {
        crate::industry::policy::sum_history(history)
    }
}

impl HistoryElement for crate::town::SuppliedHistory {
    fn sum(history: &[Self]) -> Self {
        crate::town::sum_history(history)
    }
}

// All raw accesses are bounded by the original history/range preconditions. The range
// chain is immutable; history/result may alias, so no history reference spans a store.
#[allow(unsafe_code)]
impl HistoryRange {
    unsafe fn descriptor(&self) -> Descriptor {
        // SAFETY: The native caller keeps the immutable range chain live.
        let child = unsafe { self.hr.as_ref() };
        Descriptor {
            child: usize::from(child.is_some()),
            periods: self.periods,
            records: self.records,
            first: self.first,
            last: self.last,
            division: self.division,
            total_division: self.total_division,
            child_periods: child.map_or(0, |hr| hr.periods),
            child_division: child.map_or(0, |hr| hr.division),
        }
    }

    unsafe fn update_valid(&self, mask: *mut u64, month: u32) {
        /* Update for subdivisions first. */
        // SAFETY: The native caller keeps the immutable range chain live.
        if let Some(child) = unsafe { self.hr.as_ref() } {
            // SAFETY: Child and mask share the enclosing call's validity contract.
            unsafe { child.update_valid(mask, month) };
        }
        // SAFETY: The mask is live and exclusively accessible at this observation.
        let current = unsafe { mask.read() };
        // SAFETY: The immutable native range chain is live.
        let descriptor = unsafe { self.descriptor() };
        // Preserve the original guards and GB's uint truncation in the shared node operation.
        if let Some(updated) = update_record(current, descriptor, month) {
            // SAFETY: No borrow or callback spans this native mask write.
            unsafe { mask.write(updated) };
        }
    }

    unsafe fn is_valid(&self, mask: u64, age: u32) -> bool {
        // SAFETY: The immutable native range chain is live.
        let descriptor = unsafe { self.descriptor() };
        if aggregate(descriptor, age) {
            // SAFETY: Aggregation requires the live previous subdivision.
            let child = unsafe { &*self.hr };
            let month = u32::from(crate::timer_game_common::economy_month());
            // SAFETY: The child retains the enclosing chain's lifetime/preconditions.
            return unsafe { child.is_valid(mask, start(descriptor, age, month)) };
        }
        age < u32::from(self.periods) && has_bit(mask, slot(descriptor, age))
    }

    unsafe fn rotate<T: HistoryElement>(&self, history: *mut T, mask: u64, month: u32) {
        // SAFETY: The immutable native range chain is live.
        if let Some(child) = unsafe { self.hr.as_ref() } {
            // SAFETY: The same history allocation and original child range are valid.
            unsafe { child.rotate(history, mask, month) };
        }
        if !month.is_multiple_of(u32::from(self.total_division)) {
            return;
        }
        let first = usize::from(self.first);
        let last = usize::from(self.last);
        for target in (first + 1..last).rev() {
            // SAFETY: Original move_backward visits these initialized slots in reverse order.
            let source = unsafe { history.add(target - 1).read() };
            // SAFETY: No history borrow spans this write; the range bounds are native preconditions.
            unsafe { history.add(target).write(source) };
        }
        if self.total_division == 1 {
            // SAFETY: Original first - 1 is a live history slot.
            let source = unsafe { history.add(first - 1).read() };
            // SAFETY: Original first is a live history slot; no borrow spans the write.
            unsafe { history.add(first).write(source) };
            // SAFETY: The current period is the initialized front element.
            unsafe { history.write(T::default()) };
        } else if has_bit(
            mask,
            u32::from(self.first).wrapping_sub(u32::from(self.division)),
        ) {
            let division = usize::from(self.division);
            // SAFETY: Original reduction reads this initialized span; reducers have no callbacks.
            let inputs =
                unsafe { std::slice::from_raw_parts(history.add(first - division), division) };
            let result = T::sum(inputs);
            // SAFETY: The input borrow ended before assigning the original target slot.
            unsafe { history.add(first).write(result) };
        }
    }

    unsafe fn get<T: HistoryElement>(
        &self,
        history: *const T,
        mask: u64,
        age: u32,
        result: *mut T,
    ) -> Option<bool> {
        // SAFETY: The immutable native range chain is live.
        let descriptor = unsafe { self.descriptor() };
        if aggregate(descriptor, age) {
            let mut is_valid = false;
            // No need to clear as we fill every element we use. These concrete types
            // default-construct all four elements without callbacks or side effects.
            let mut tmp_result = [T::default(); HISTORY_MAX_DIVISION];
            let month = u32::from(crate::timer_game_common::economy_month());
            let start = start(descriptor, age, month);
            // SAFETY: Aggregation requires the live previous subdivision.
            let child = unsafe { &*self.hr };
            for i in 0..self.division {
                // SAFETY: Each scratch target is distinct; original child ages and slots apply.
                is_valid |= unsafe {
                    child.get(
                        history,
                        mask,
                        start.wrapping_add(u32::from(i)),
                        &raw mut tmp_result[usize::from(i)],
                    )
                }?;
            }
            let sum = T::sum(&tmp_result[..usize::from(self.division)]);
            // SAFETY: Scratch reads have ended; result can alias any live history element.
            unsafe { result.write(sum) };
            return Some(is_valid);
        }
        if age < u32::from(self.periods) {
            let index = slot(descriptor, age) as usize;
            // SAFETY: The original range/age select a live initialized history slot.
            let source = unsafe { history.add(index).read() };
            // SAFETY: Read completes before the potentially aliasing result write.
            unsafe { result.write(source) };
            return Some(has_bit(mask, slot(descriptor, age)));
        }
        None
    }
}

/// Update the mask, observing subdivision writes before each parent.
///
/// # Safety
/// `mask` is live/exclusive; `hr` is a live immutable acyclic native range chain.
/// Original valid-index, divisor and shift preconditions apply to every node.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_history_update_valid(
    mask: *mut u64,
    hr: *const HistoryRange,
    month: u32,
) {
    // SAFETY: Caller grants the native range chain's lifetime.
    let hr = unsafe { &*hr };
    // SAFETY: Caller grants the original mask and range preconditions.
    unsafe { hr.update_valid(mask, month) };
}

/// Test first-child validity without extracting data.
///
/// # Safety
/// `hr` meets the immutable range-chain and original index/divisor/shift contract.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_history_is_valid(
    mask: u64,
    hr: *const HistoryRange,
    age: u32,
) -> u8 {
    // SAFETY: Caller grants the native range chain's lifetime.
    let hr = unsafe { &*hr };
    // SAFETY: Caller grants the original range and bit-index preconditions.
    u8::from(unsafe { hr.is_valid(mask, age) })
}

/// Rotate industry produced/accepted history using its canonical pair layout.
///
/// # Safety
/// `history` contains the original initialized record array, exclusively accessible;
/// `hr` meets the immutable range-chain and original index/divisor/shift contract.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_history_rotate_industry(
    history: *mut crate::industry::Pair,
    mask: u64,
    hr: *const HistoryRange,
    month: u32,
) {
    // SAFETY: Caller grants the native range chain's lifetime.
    let hr = unsafe { &*hr };
    // SAFETY: Caller grants the canonical array and original range preconditions.
    unsafe { hr.rotate(history, mask, month) };
}

/// Query industry produced/accepted history, preserving nested reduction rounding.
///
/// # Safety
/// `history` is the original initialized array, `result` a writable canonical pair
/// (possibly an array element); `hr` meets the immutable range-chain contract.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_history_get_industry(
    history: *const crate::industry::Pair,
    mask: u64,
    hr: *const HistoryRange,
    age: u32,
    result: *mut crate::industry::Pair,
) -> HistoryQuery {
    // SAFETY: Caller grants the native range chain's lifetime.
    let hr = unsafe { &*hr };
    // SAFETY: Caller grants the initialized array/result and original range preconditions.
    unsafe { hr.get(history, mask, age, result) }.into()
}

/// Rotate town supplied history using its canonical element layout.
///
/// # Safety
/// `history` contains the original initialized record array, exclusively accessible;
/// `hr` meets the immutable range-chain and original index/divisor/shift contract.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_history_rotate_town(
    history: *mut crate::town::SuppliedHistory,
    mask: u64,
    hr: *const HistoryRange,
    month: u32,
) {
    // SAFETY: Caller grants the native range chain's lifetime.
    let hr = unsafe { &*hr };
    // SAFETY: Caller grants the canonical array and original range preconditions.
    unsafe { hr.rotate(history, mask, month) };
}

/// Query town supplied history, preserving nested reduction and aliasing order.
///
/// # Safety
/// `history` is the original initialized array, `result` a writable canonical element
/// (possibly an array element); `hr` meets the immutable range-chain contract.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_history_get_town(
    history: *const crate::town::SuppliedHistory,
    mask: u64,
    hr: *const HistoryRange,
    age: u32,
    result: *mut crate::town::SuppliedHistory,
) -> HistoryQuery {
    // SAFETY: Caller grants the native range chain's lifetime.
    let hr = unsafe { &*hr };
    // SAFETY: Caller grants the initialized array/result and original range preconditions.
    unsafe { hr.get(history, mask, age, result) }.into()
}
