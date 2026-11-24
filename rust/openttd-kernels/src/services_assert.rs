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

/// `assert(condition)`: evaluate `condition` only where the native build asserts,
/// and report a failure through `check`. Before the shared table is bound (tests
/// and tools that bind only the callback) the condition is evaluated.
pub(crate) fn assert_with(condition: impl FnOnce() -> bool) {
    let shared = SHARED.load(Ordering::Acquire);
    // SAFETY: Installed only by the startup binding's process-lifetime contract;
    // the map statics it names are immutable.
    if (shared.is_null() || unsafe { (*(*shared).map).assertions }) && !condition() {
        check(false);
    }
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

/// The environment variable an isolating spawn sets on its child, beside the
/// `--exact` argument and the one test name it passes. A test binds the
/// process-wide policy only where it is set: no other test thread exists there,
/// so no other test reads the tables the binding publishes.
#[cfg(test)]
pub(crate) const ISOLATED_CHILD: &str = "OPENTTD_ASSERT_ISOLATED_CHILD";

/// Panic, before any binding, unless this process is an isolating spawn's child.
#[cfg(test)]
fn require_isolated_child() {
    assert!(
        std::env::var_os(ISOLATED_CHILD).is_some(),
        "a test binds the native assertion policy outside an isolated child process"
    );
}

/// Replace only the assertion callback for a Rust test; retain fatal dispatch.
#[cfg(test)]
pub(crate) fn bind_test_check(check: extern "C" fn(bool)) {
    require_isolated_child();
    // SAFETY: A null table changes no pointer lifetime contract.
    unsafe {
        openttd_rust_assert_bind(check, std::ptr::null());
    }
}

/// Bind the assertion callback and the shared table for a Rust test. The binding
/// retains `shared` for the rest of the process, so it takes a process-lifetime table,
/// and every later `assert_with` reads `shared.map`, so the binder refuses a null map.
///
/// # Safety
/// `shared.map` points to map statics that live through the child process's last
/// assertion.
#[cfg(test)]
pub(crate) unsafe fn bind_test_policy(
    check: extern "C" fn(bool),
    shared: &'static crate::services::Services,
) {
    require_isolated_child();
    assert!(
        !shared.map.is_null(),
        "a test binds a shared table with no map statics"
    );
    // SAFETY: `shared` lives for the process, as the export requires, and the caller
    // keeps its map statics live through every assertion.
    unsafe {
        openttd_rust_assert_bind(check, shared);
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
        .env(ISOLATED_CHILD, "1")
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
const ASSERT_EXIT_CODE: i32 = 86;
/// What the exiting assertion policy writes before it ends the child.
#[cfg(test)]
const ASSERT_EXIT_MARKER: &str = "openttd-rust test: native assertion failed";

#[cfg(test)]
extern "C" fn exit_on_failure(condition: bool) {
    if !condition {
        eprintln!("{ASSERT_EXIT_MARKER}");
        std::process::exit(ASSERT_EXIT_CODE);
    }
}

/// Run a test in a separate process whose native assertion policy ends the
/// process at the first failed assertion, and require the child to start the
/// named test and end through that policy: with its exit code, after writing its
/// marker. A body that cannot continue past a failed assertion is observed this
/// way without returning into it. Return true in the parent; the child binds the
/// policy and returns false to execute the test body.
#[cfg(test)]
pub(crate) fn isolate_test_exit(name: &str) -> bool {
    const CHILD: &str = "OPENTTD_ASSERT_EXIT_TEST_CHILD";
    if std::env::var(CHILD).ok().as_deref() == Some(name) {
        bind_test_check(exit_on_failure);
        return false;
    }
    let (status, stdout, stderr) = run_isolated_child(CHILD, name);
    assert!(
        status.code() == Some(ASSERT_EXIT_CODE)
            && stdout.contains("running 1 test")
            && stderr.contains(ASSERT_EXIT_MARKER),
        "isolated native assertion test {name} did not run and end at a failed native \
         assertion: {status:?}\n{stdout}{stderr}",
    );
    true
}

/// Run a test in a separate process that the test body itself ends, with
/// `std::process::exit(code)` after writing `marker` to standard error, at an observation
/// point inside the code under test. Require the child to start the named test and end
/// that way. Return true in the parent; the child, which binds nothing, returns false to
/// execute the test body.
#[cfg(test)]
pub(crate) fn isolate_test_ends(name: &str, code: i32, marker: &str) -> bool {
    const CHILD: &str = "OPENTTD_ENDING_TEST_CHILD";
    if std::env::var(CHILD).ok().as_deref() == Some(name) {
        return false;
    }
    let (status, stdout, stderr) = run_isolated_child(CHILD, name);
    assert!(
        status.code() == Some(code) && stdout.contains("running 1 test") && stderr.contains(marker),
        "isolated test {name} did not run and end with code {code} after {marker:?}: \
         {status:?}\n{stdout}{stderr}",
    );
    true
}

/// Run the one test `name` in a child of this test binary, with `variable` naming it, and
/// return its status and output. A child that does not end within a minute is killed.
#[cfg(test)]
fn run_isolated_child(variable: &str, name: &str) -> (std::process::ExitStatus, String, String) {
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--test-threads", "1", "--nocapture"])
        .env(variable, name)
        .env(ISOLATED_CHILD, "1")
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
    (
        output.status,
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[cfg(test)]
mod tests {
    extern "C" fn unused_check(_: bool) {}

    /// The parent test process has no marker, so the binder panics before binding.
    #[test]
    #[should_panic(expected = "outside an isolated child process")]
    fn binding_the_check_in_the_parent_process_panics() {
        super::bind_test_check(unused_check);
    }

    #[test]
    #[should_panic(expected = "outside an isolated child process")]
    fn binding_the_policy_in_the_parent_process_panics() {
        // SAFETY: The guard panics before anything is bound.
        unsafe {
            super::bind_test_policy(
                unused_check,
                Box::leak(Box::new(crate::test_tables::SHARED)),
            );
        }
    }

    #[test]
    #[should_panic(expected = "did not run and pass exactly one test")]
    fn isolating_a_name_that_matches_no_test_fails() {
        super::isolate_test_check("services_assert::tests::no_such_test", unused_check);
    }

    #[test]
    #[should_panic(expected = "did not run and end at a failed native assertion")]
    fn isolating_an_exit_for_a_name_that_matches_no_test_fails() {
        super::isolate_test_exit("services_assert::tests::no_such_test");
    }

    /// A child that ends without a failed assertion fails the parent, whatever
    /// its exit code.
    #[test]
    #[should_panic(expected = "did not run and end at a failed native assertion")]
    fn isolating_an_exit_for_a_test_that_makes_no_assertion_fails() {
        if super::isolate_test_exit(
            "services_assert::tests::isolating_an_exit_for_a_test_that_makes_no_assertion_fails",
        ) {
            return;
        }
        std::process::exit(super::ASSERT_EXIT_CODE);
    }
}
