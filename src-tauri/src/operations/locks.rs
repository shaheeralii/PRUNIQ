use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex, PoisonError};

use tokio::sync::{Mutex as TokioMutex, OwnedMutexGuard};

use crate::core::errors::AdtError;

/// Per-device asynchronous operation lock preventing concurrent conflicting
/// package-management operations on the same Android device.
#[derive(Default)]
pub struct DeviceLockManager {
    locks: StdMutex<HashMap<String, Arc<TokioMutex<()>>>>,
}

impl DeviceLockManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attempts to immediately acquire the exclusive mutation lock for `serial`.
    /// Fails fast with `AdtError::OperationInProgress` instead of silently queuing
    /// conflicting mutations behind an in-flight operation.
    pub fn try_lock_device(&self, serial: &str) -> Result<OwnedMutexGuard<()>, AdtError> {
        let device_mutex = {
            let mut map = self.locks.lock().unwrap_or_else(PoisonError::into_inner);
            Arc::clone(
                map.entry(serial.to_string())
                    .or_insert_with(|| Arc::new(TokioMutex::new(()))),
            )
        };

        device_mutex
            .try_lock_owned()
            .map_err(|_| AdtError::OperationInProgress {
                serial: serial.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prevents_concurrent_locks_on_same_device_and_allows_distinct_devices() {
        let manager = DeviceLockManager::new();

        let guard_a = manager
            .try_lock_device("DEV_A")
            .expect("first lock on DEV_A should succeed");

        let second_a = manager.try_lock_device("DEV_A");
        assert!(matches!(
            second_a,
            Err(AdtError::OperationInProgress { ref serial }) if serial == "DEV_A"
        ));

        let guard_b = manager
            .try_lock_device("DEV_B")
            .expect("lock on distinct device DEV_B should succeed");

        drop(guard_a);
        drop(guard_b);

        assert!(
            manager.try_lock_device("DEV_A").is_ok(),
            "DEV_A lock should be re-acquirable after drop"
        );
    }
}
