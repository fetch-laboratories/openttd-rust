/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Common Action2 execution. Native parsed nodes and payloads are immutable handles;
//! common invocation fields have one stack allocation for the resolver lifetime.
//! Entries hold the game-state mutex. Raw field accesses end before every reentrant
//! call, including procedures and scope/storage operations. Panics abort; native
//! leaves are noexcept and environmental failures terminate there.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::too_many_arguments,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]
use crate::newgrf_callbacks::{CALLBACK_FAILED, CBID_RANDOM_TRIGGER};
use crate::newgrf_storage::{
    openttd_rust_storage_temp_clear, openttd_rust_storage_temp_get, openttd_rust_storage_temp_store,
};
use std::ffi::c_void;
use std::ptr;
pub const VSG_SCOPE_SELF: u8 = 0;
pub const VSG_SCOPE_PARENT: u8 = 1;
pub const VSG_SCOPE_RELATIVE: u8 = 2;
type Handle = *const c_void;
type Resolver = *mut c_void;
#[repr(C)]
pub struct State {
    callback: u16,
    callback_param1: u32,
    callback_param2: u32,
    last_value: u32,
    waiting_random_triggers: u32,
    used_random_triggers: u32,
    reseed: [u32; 3],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ResultValue {
    group: Handle,
    callback: u16,
    kind: u8,
}
impl ResultValue {
    fn failed() -> Self {
        Self {
            group: ptr::null(),
            callback: 0,
            kind: 0,
        }
    }
    fn callback(value: u16) -> Self {
        Self {
            group: ptr::null(),
            callback: value,
            kind: 1,
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GroupResult {
    group: Handle,
    calculated_result: bool,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DeterministicNode {
    adjusts: usize,
    ranges: usize,
    default_result: GroupResult,
    error_group: Handle,
    scope: u8,
    size: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RandomizedNode {
    groups: usize,
    scope: u8,
    compare: u8,
    triggers: u8,
    count: u8,
    lowest_bit: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub union NodeData {
    deterministic: DeterministicNode,
    randomized: RandomizedNode,
    callback: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Node {
    data: NodeData,
    kind: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Adjust {
    subroutine: Handle,
    and_mask: u32,
    add_val: u32,
    divmod_val: u32,
    operation: u8,
    kind: u8,
    variable: u8,
    parameter: u8,
    shift_num: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Range {
    result: GroupResult,
    low: u32,
    high: u32,
}
#[repr(C)]
pub struct Services {
    node: unsafe extern "C" fn(Handle, *mut Node),
    adjust: unsafe extern "C" fn(Handle, usize) -> Adjust,
    range: unsafe extern "C" fn(Handle, usize) -> Range,
    random_group: unsafe extern "C" fn(Handle, usize) -> Handle,
    scope: unsafe extern "C" fn(Resolver, u8, u8) -> Resolver,
    variable: unsafe extern "C" fn(Handle, u8, u32, *mut bool) -> u32,
    random_bits: unsafe extern "C" fn(Handle) -> u32,
    random_triggers: unsafe extern "C" fn(Handle) -> u32,
    store_psa: unsafe extern "C" fn(Resolver, u32, i32),
    real: unsafe extern "C" fn(Resolver, Handle) -> Handle,
    native: unsafe extern "C" fn(Handle, Resolver) -> ResultValue,
    parameter: unsafe extern "C" fn(Handle, u32) -> u32,
    global: unsafe extern "C" fn(u8, *mut u32, Handle) -> bool,
    profile_enter: unsafe extern "C" fn(Resolver, bool) -> Resolver,
    profile_end: unsafe extern "C" fn(Resolver, *const ResultValue),
}
#[derive(Clone, Copy)]
struct Invocation<'a> {
    s: &'a Services,
    ro: Resolver,
    state: *mut State,
    file: Handle,
}
impl Invocation<'_> {
    unsafe fn variable(
        self,
        scope: Resolver,
        variable: u8,
        parameter: u32,
        available: *mut bool,
    ) -> u32 {
        unsafe {
            match variable {
                0x0C => u32::from((*self.state).callback),
                0x10 => (*self.state).callback_param1,
                0x18 => (*self.state).callback_param2,
                0x1C => (*self.state).last_value,
                0x5F => {
                    (self.s.random_bits)(scope).wrapping_shl(8) | (self.s.random_triggers)(scope)
                }
                0x7D => openttd_rust_storage_temp_get(parameter) as u32,
                0x7F => {
                    if self.file.is_null() {
                        0
                    } else {
                        (self.s.parameter)(self.file, parameter)
                    }
                }
                _ => {
                    let mut value = 0;
                    if variable < 0x40 && (self.s.global)(variable, &raw mut value, self.file) {
                        value
                    } else {
                        (self.s.variable)(scope, variable, parameter, available)
                    }
                }
            }
        }
    }
    unsafe fn resolve(self, group: Handle, top: bool) -> ResultValue {
        if group.is_null() {
            return ResultValue::failed();
        }
        unsafe {
            let profiler = (self.s.profile_enter)(self.ro, top);
            let mut node = std::mem::MaybeUninit::<Node>::uninit();
            (self.s.node)(group, node.as_mut_ptr());
            let node = node.assume_init();
            let result = match node.kind {
                0 => self.resolve((self.s.real)(self.ro, group), false),
                1 => self.deterministic(group, node.data.deterministic),
                2 => self.randomized(group, node.data.randomized),
                3 => ResultValue::callback(node.data.callback),
                // Terminal payloads stay in the native graph; no payload read/copy.
                4..=6 => ResultValue {
                    group,
                    callback: 0,
                    kind: node.kind - 2,
                },
                7 => (self.s.native)(group, self.ro),
                _ => std::hint::unreachable_unchecked(), // Native closed group-kind enum.
            };
            if top && !profiler.is_null() {
                (self.s.profile_end)(profiler, &raw const result);
            }
            result
        }
    }
    unsafe fn deterministic(self, group: Handle, node: DeterministicNode) -> ResultValue {
        unsafe {
            let scope = (self.s.scope)(self.ro, node.scope, 0);
            let mut last = 0;
            let mut value = 0;
            for i in 0..node.adjusts {
                let adjust = (self.s.adjust)(group, i);
                let mut available = true;
                value = match adjust.variable {
                    0x7E => {
                        let result = self.resolve(adjust.subroutine, false);
                        if result.kind == 1 {
                            u32::from(result.callback)
                        } else {
                            u32::from(u16::MAX)
                        }
                    }
                    0x7B => self.variable(scope, adjust.parameter, last, &raw mut available),
                    _ => self.variable(
                        scope,
                        adjust.variable,
                        u32::from(adjust.parameter),
                        &raw mut available,
                    ),
                };
                if !available {
                    return self.resolve(node.error_group, false);
                }
                value = self.adjust(&adjust, scope, node.size, last, value);
                last = value;
            }
            (*self.state).last_value = last;
            let mut result = node.default_result;
            if node.ranges > 4 {
                let mut first = 0;
                let mut count = node.ranges;
                // std::lower_bound: compare high first, including inclusive endpoints.
                while count != 0 {
                    let step = count / 2;
                    let mid = first + step;
                    let range = (self.s.range)(group, mid);
                    if range.high < value {
                        first = mid + 1;
                        count -= step + 1;
                    } else {
                        count = step;
                    }
                }
                if first != node.ranges {
                    let range = (self.s.range)(group, first);
                    if range.low <= value {
                        result = range.result;
                    }
                }
            } else {
                for i in 0..node.ranges {
                    let range = (self.s.range)(group, i);
                    if range.low <= value && value <= range.high {
                        result = range.result;
                        break;
                    }
                }
            }
            if result.calculated_result {
                ResultValue::callback((value & 0x7FFF) as u16)
            } else {
                self.resolve(result.group, false)
            }
        }
    }
    unsafe fn randomized(self, group: Handle, node: RandomizedNode) -> ResultValue {
        unsafe {
            let scope = (self.s.scope)(self.ro, node.scope, node.count);
            if (*self.state).callback == CBID_RANDOM_TRIGGER.0 {
                let matched = node.triggers & (*self.state).waiting_random_triggers as u8;
                let matches = if node.compare == 0 {
                    matched != 0
                } else {
                    matched == node.triggers
                };
                if matches {
                    (*self.state).used_random_triggers |= u32::from(matched);
                    let reseed = (node.groups.wrapping_sub(1) << node.lowest_bit) as u32;
                    let slot = (&raw mut (*self.state).reseed)
                        .cast::<u32>()
                        .add(usize::from(node.scope));
                    *slot |= reseed;
                }
            }
            let mask = (node.groups as u32).wrapping_sub(1) << node.lowest_bit;
            let index = (((self.s.random_bits)(scope) & mask) >> node.lowest_bit) as u8;
            self.resolve((self.s.random_group)(group, usize::from(index)), false)
        }
    }
    unsafe fn adjust(
        self,
        a: &Adjust,
        scope: Resolver,
        size: u8,
        last: u32,
        mut value: u32,
    ) -> u32 {
        // Byte/word signed operands promote to int before adjustment arithmetic;
        // only dword addition wraps at 32 bits. The returned U truncates last.
        let unsigned = |v: u32| match size {
            0 => u32::from(v as u8),
            1 => u32::from(v as u16),
            _ => v,
        };
        let signed = |v: u32| match size {
            0 => i32::from(v as i8),
            1 => i32::from(v as i16),
            _ => v as i32,
        };
        let last = unsigned(last);
        value = (value >> a.shift_num) & a.and_mask;
        match a.kind {
            1 => {
                value = signed(value)
                    .wrapping_add(signed(a.add_val))
                    .wrapping_div(signed(a.divmod_val)) as u32
            }
            2 => {
                value = signed(value)
                    .wrapping_add(signed(a.add_val))
                    .wrapping_rem(signed(a.divmod_val)) as u32
            }
            _ => {}
        }
        let result = match a.operation {
            0 => last.wrapping_add(value),
            1 => last.wrapping_sub(value),
            2 => signed(last).min(signed(value)) as u32,
            3 => signed(last).max(signed(value)) as u32,
            4 => last.min(unsigned(value)),
            5 => last.max(unsigned(value)),
            6 => {
                (if value == 0 {
                    signed(last)
                } else {
                    signed(last).wrapping_div(signed(value))
                }) as u32
            }
            7 => {
                (if value == 0 {
                    signed(last)
                } else {
                    signed(last).wrapping_rem(signed(value))
                }) as u32
            }
            8 => {
                if value == 0 {
                    last
                } else {
                    last / unsigned(value)
                }
            }
            9 => {
                if value == 0 {
                    last
                } else {
                    last % unsigned(value)
                }
            }
            10 => last.wrapping_mul(value),
            11 => last & value,
            12 => last | value,
            13 => last ^ value,
            14 => {
                unsafe {
                    openttd_rust_storage_temp_store(unsigned(value), signed(last));
                }
                last
            }
            16 => {
                unsafe {
                    (self.s.store_psa)(scope, unsigned(value), signed(last));
                }
                last
            }
            17 => last.rotate_right(unsigned(value) & 0x1F),
            18 => {
                if signed(last) == signed(value) {
                    1
                } else if signed(last) < signed(value) {
                    0
                } else {
                    2
                }
            }
            19 => {
                if last == unsigned(value) {
                    1
                } else if last < unsigned(value) {
                    0
                } else {
                    2
                }
            }
            20 => last << (unsigned(value) & 0x1F),
            21 => last >> (unsigned(value) & 0x1F),
            22 => (signed(last) >> (unsigned(value) & 0x1F)) as u32,
            _ => value,
        };
        unsigned(result)
    }
    unsafe fn do_resolve(self, group: Handle) -> ResultValue {
        unsafe {
            openttd_rust_storage_temp_clear();
            (*self.state).last_value = 0;
            (*self.state).used_random_triggers = 0;
            (*self.state).reseed = [0; 3];
            self.resolve(group, true)
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_resolver_init(
    state: *mut State,
    callback: u16,
    callback_param1: u32,
    callback_param2: u32,
) {
    unsafe {
        state.write(State {
            callback,
            callback_param1,
            callback_param2,
            last_value: 0,
            waiting_random_triggers: 0,
            used_random_triggers: 0,
            reseed: [0; 3],
        });
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_spritegroup_resolve(
    s: &Services,
    group: Handle,
    ro: Resolver,
    state: *mut State,
    file: Handle,
    top: bool,
) -> ResultValue {
    unsafe { Invocation { s, ro, state, file }.resolve(group, top) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_resolver_resolve(
    s: &Services,
    group: Handle,
    ro: Resolver,
    state: *mut State,
    file: Handle,
) -> ResultValue {
    unsafe { Invocation { s, ro, state, file }.do_resolve(group) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_resolver_callback(
    s: &Services,
    group: Handle,
    ro: Resolver,
    state: *mut State,
    file: Handle,
    registers: *mut i32,
    count: usize,
) -> u16 {
    unsafe {
        let result = Invocation { s, ro, state, file }.do_resolve(group);
        if result.kind != 1 {
            return CALLBACK_FAILED;
        }
        for i in 0..count {
            registers
                .add(i)
                .write(openttd_rust_storage_temp_get(0x100 + i as u32));
        }
        result.callback
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDResolverState" ["storage"] => State { callback, callback_param1, callback_param2, last_value, waiting_random_triggers, used_random_triggers, reseed };
    "OpenTTDResolverResult" ["storage"] => ResultValue { group, callback, kind };
    "OpenTTDGroupResult" ["storage"] => GroupResult { group, calculated_result };
    "OpenTTDDeterministicNode" ["storage"] => DeterministicNode { adjusts, ranges, default_result, error_group, scope, size };
    "OpenTTDRandomizedNode" ["storage"] => RandomizedNode { groups, scope, compare, triggers, count, lowest_bit };
    "OpenTTDGroupNode" ["storage"] => Node { data, kind };
    "OpenTTDGroupAdjust" ["storage"] => Adjust { subroutine, and_mask, add_val, divmod_val, operation, kind => type, variable, parameter, shift_num };
    "OpenTTDGroupRange" ["storage"] => Range { result, low, high };
    "OpenTTDSpriteGroupServices" ["storage"] => Services { node, adjust, range, random_group, scope, variable, random_bits, random_triggers, store_psa, real, native, parameter, global, profile_enter, profile_end };
}
