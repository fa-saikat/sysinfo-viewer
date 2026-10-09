//! Fixture seams for integration testing.
//!
//! Collectors that read well-known absolute paths (`/proc/...`,
//! `/sys/...`) route those reads through [`sys_path`], which re-roots
//! them under `$SYSINFO_SYS_ROOT` when that variable is set. Tests point
//! it at a synthetic tree; production never sets it, so behavior there
//! is byte-for-byte what it always was.
//!
//! External tools (`lspci`, `nvidia-smi`) are resolved through `PATH` as
//! usual — tests prepend a stub `bin/` directory (or an empty one for the
//! no-tools fixture) instead of needing a seam in code.
//!
//! Environment mutation is process-global, so tests that set these
//! variables must hold [`env_lock`] across the whole arrange–act–assert
//! span. Each test binary is its own process, so one lock per binary is
//! enough; unit tests in this crate and integration tests each hold
//! their own.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Absolute path (e.g. `/proc/cpuinfo`) re-rooted under the fixture root
/// when `SYSINFO_SYS_ROOT` is set, untouched otherwise.
pub fn sys_path(absolute: &str) -> PathBuf {
    let relative = absolute.trim_start_matches('/');
    sys_root().join(relative)
}

fn sys_root() -> PathBuf {
    std::env::var("SYSINFO_SYS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}

/// Process-wide serialisation for tests that mutate the environment.
pub fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sys_path_passes_through_without_a_fixture_root() {
        let _guard = env_lock();
        std::env::remove_var("SYSINFO_SYS_ROOT");
        assert_eq!(sys_path("/proc/cpuinfo"), PathBuf::from("/proc/cpuinfo"));
        assert_eq!(
            sys_path("/sys/class/net/lo/operstate"),
            PathBuf::from("/sys/class/net/lo/operstate")
        );
    }

    #[test]
    fn sys_path_reroots_under_the_fixture_root() {
        let _guard = env_lock();
        std::env::set_var("SYSINFO_SYS_ROOT", "/tmp/fake-root");
        assert_eq!(
            sys_path("/proc/cpuinfo"),
            PathBuf::from("/tmp/fake-root/proc/cpuinfo")
        );
        std::env::remove_var("SYSINFO_SYS_ROOT");
    }
}
