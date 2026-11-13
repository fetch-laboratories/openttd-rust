/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `error_func.h`: fatal errors.

use crate::services::Services;

/// `FatalError(message)`: show the fatal error `message` and end the program. It does not
/// return. The message is the original's formatted text, passed as bytes.
pub(crate) fn fatal_error(shared: &Services, message: &str) -> ! {
    (shared.fatal_error)(message.as_ptr(), message.len());
    /* The native FatalErrorI is [[noreturn]]. */
    std::process::abort()
}
