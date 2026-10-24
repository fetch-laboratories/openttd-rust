/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Clone-name suffix policy with platform-long parsing and retained byte scratch.
use super::CommonServices;
use crate::game_state::GameState;
use crate::services::ByteSpan;
use std::ffi::c_long;
use std::io::Write;
#[derive(Default)]
struct Frame {
    prefix: Vec<u8>,
    candidate: Vec<u8>,
}
static POOL: GameState<Vec<Frame>> = GameState::new(Vec::new());
struct Transaction(Frame);
impl Transaction {
    fn new() -> Self {
        // SAFETY: Short serialized access; nested commands obtain distinct frames.
        Self(unsafe { (&mut *POOL.get()).pop() }.unwrap_or_default())
    }
}
impl Drop for Transaction {
    fn drop(&mut self) {
        self.0.prefix.clear();
        self.0.candidate.clear();
        // SAFETY: Returning cleared buffers invokes no external callbacks.
        unsafe {
            (&mut *POOL.get()).push(std::mem::take(&mut self.0));
        }
    }
}
/// # Safety
/// Live source/destination shells and immutable services. Native parsing runs
/// the original `from_chars<long>`/assert protocol; no owner borrow spans it.
pub(super) unsafe fn clone_name(
    s: &CommonServices,
    parse: extern "C" fn(ByteSpan) -> c_long,
    source: *mut (),
    destination: *mut (),
) {
    let name = (s.base.name)(source);
    let mut transaction = Transaction::new();
    let mut position = name.size;
    {
        let bytes = if name.size == 0 {
            &[]
        } else {
            // SAFETY: Source name remains live and unmodified until it is copied.
            unsafe { std::slice::from_raw_parts(name.data, name.size) }
        };
        while position > 0 && bytes[position - 1].is_ascii_digit() {
            position -= 1;
        }
        transaction.0.prefix.extend_from_slice(&bytes[..position]);
    }
    let mut padding = 0_u8;
    let mut number: c_long;
    if position == name.size {
        transaction.0.prefix.push(b' ');
        number = 2;
    } else {
        padding = name.size.wrapping_sub(position) as u8;
        let suffix = ByteSpan {
            // SAFETY: Position lies within the live source-name byte span.
            data: unsafe { name.data.add(position) },
            size: name.size - position,
        };
        number = parse(suffix).wrapping_add(1);
    }
    for _ in 0..1000 {
        transaction.0.candidate.clear();
        // No scratch reference survives name observation or assignment callbacks.
        transaction
            .0
            .candidate
            .extend_from_slice(&transaction.0.prefix);
        write!(
            &mut transaction.0.candidate,
            "{number:0width$}",
            width = usize::from(padding)
        )
        .expect("writing formatted bytes to Vec cannot fail");
        let proposed = ByteSpan {
            data: transaction.0.candidate.as_ptr(),
            size: transaction.0.candidate.len(),
        };
        if unsafe { super::openttd_rust_vehicle_unique_name(s.base, proposed) } {
            (s.base.set_name)(destination, proposed);
            break;
        }
        number = number.wrapping_add(1);
    }
}
