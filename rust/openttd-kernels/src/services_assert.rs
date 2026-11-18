/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical native invariant check, following the native build assertion policy.
#![allow(unsafe_code)]

use std::sync::atomic::{AtomicPtr, Ordering};

extern "C" fn unbound(_: bool) {
    std::process::abort();
}
static SHARED: AtomicPtr<crate::services::Services> = AtomicPtr::new(std::ptr::null_mut());
static CHECK: AtomicPtr<()> = AtomicPtr::new(unbound as *mut ());

/// Bind the native invariant policy before game/tool entries. A null shared
/// pointer replaces only the assertion callback, preserving the fatal binding.
///
/// # Safety
/// A non-null `shared` points to the canonical process-lifetime shared table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_assert_bind(
    check: extern "C" fn(bool),
    shared: *const crate::services::Services,
) {
    if !shared.is_null() {
        SHARED.store(shared.cast_mut(), Ordering::Release);
    }
    CHECK.store(check as *mut (), Ordering::Release);
}

/// `RustAssertCondition`; this leaf does not mutate Rust state.
pub extern "C" fn check(condition: bool) {
    // SAFETY: Both the initial function and every bound function have this signature.
    let check: extern "C" fn(bool) = unsafe { std::mem::transmute(CHECK.load(Ordering::Acquire)) };
    check(condition);
}

/// The canonical native `NOT_REACHED` operation, independent of assertion policy.
pub(crate) fn not_reached() -> ! {
    let shared = SHARED.load(Ordering::Acquire);
    if !shared.is_null() {
        // SAFETY: Installed only by the startup binding's process-lifetime contract.
        (unsafe { &*shared }.not_reached)();
    }
    std::process::abort()
}

/// Replace only the assertion callback for a Rust test; retain fatal dispatch.
#[cfg(test)]
pub(crate) fn bind_test_check(check: extern "C" fn(bool)) {
    // SAFETY: A null table changes no pointer lifetime contract.
    unsafe {
        openttd_rust_assert_bind(check, std::ptr::null());
    }
}

/// Run a test with its native assertion policy in a separate process, so binding
/// cannot race with parallel tests that deliberately replace the process-wide policy.
/// Return true in the parent after the child has run and passed exactly one test;
/// the child binds `check` and returns false to execute the existing test body.
/// libtest exits successfully when `name` matches no test, so the parent also
/// reads the child's summary: a renamed test must fail, not pass having run nothing.
#[cfg(test)]
pub(crate) fn isolate_test_check(name: &str, check: extern "C" fn(bool)) -> bool {
    const CHILD: &str = "OPENTTD_ASSERT_TEST_CHILD";
    if std::env::var(CHILD).ok().as_deref() == Some(name) {
        bind_test_check(check);
        return false;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name])
        .env(CHILD, name)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("test result: ok. 1 passed;"),
        "isolated native assertion test {name} did not run and pass exactly one test\n\
         {stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

/// Exit code of `isolate_test_exit`'s child after a failed native assertion.
#[cfg(test)]
static EXIT_CODE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

#[cfg(test)]
extern "C" fn exit_on_failure(condition: bool) {
    if !condition {
        std::process::exit(EXIT_CODE.load(Ordering::Acquire));
    }
}

/// Run a test in a separate process whose native assertion policy ends the
/// process with `code` at the first failed assertion, and require the child to
/// start the named test and end with that code. A body that cannot continue
/// past a failed assertion is observed this way without returning into it.
/// Return true in the parent; the child binds the policy and returns false to
/// execute the test body.
#[cfg(test)]
pub(crate) fn isolate_test_exit(name: &str, code: i32) -> bool {
    const CHILD: &str = "OPENTTD_ASSERT_EXIT_TEST_CHILD";
    if std::env::var(CHILD).ok().as_deref() == Some(name) {
        EXIT_CODE.store(code, Ordering::Release);
        bind_test_check(exit_on_failure);
        return false;
    }
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--test-threads", "1", "--nocapture"])
        .env(CHILD, name)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    // A body that returns from the failed assertion may never end: kill it.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while child.try_wait().unwrap().is_none() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.code() == Some(code) && stdout.contains("running 1 test"),
        "isolated native assertion test {name} did not run and exit with code {code}: {:?}\n\
         {stdout}{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

#[cfg(test)]
mod tests {
    extern "C" fn unused_check(_: bool) {}

    #[test]
    #[should_panic(expected = "did not run and pass exactly one test")]
    fn isolating_a_name_that_matches_no_test_fails() {
        super::isolate_test_check("services_assert::tests::no_such_test", unused_check);
    }

    #[test]
    #[should_panic(expected = "did not run and exit with code")]
    fn isolating_an_exit_for_a_name_that_matches_no_test_fails() {
        super::isolate_test_exit("services_assert::tests::no_such_test", 86);
    }
}
