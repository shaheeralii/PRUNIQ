use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::operations::models::{
    CapturedPackageState, PackageOperationResult, PackageSnapshotStatus, PackageStateSnapshot,
    SnapshotType,
};

type SnapshotKey = (String, u32, String);

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

pub fn next_id(prefix: &str) -> String {
    let seq = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{}-{seq}", now_epoch_ms())
}

/// In-memory Phase 4 snapshot and session-operation store.
/// Enforces strict separation between Initial Toolkit Baseline (immutable once captured)
/// and Pre-Operation snapshots (used for Undo). Phase 5 will back this with SQLite.
#[derive(Default)]
pub struct SnapshotStore {
    baselines: HashMap<SnapshotKey, PackageStateSnapshot>,
    pre_operations: HashMap<SnapshotKey, Vec<PackageStateSnapshot>>,
    session_operations: Vec<PackageOperationResult>,
}

impl SnapshotStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Captures the Initial Toolkit Baseline ONLY if one has never been recorded
    /// for `(serial, user_id, package_name)`. Never overwrites an existing baseline.
    pub fn capture_baseline_if_absent(
        &mut self,
        serial: &str,
        user_id: u32,
        package_name: &str,
        state: &CapturedPackageState,
        android_version: Option<&str>,
    ) -> PackageStateSnapshot {
        let key = (serial.to_string(), user_id, package_name.to_string());
        self.baselines
            .entry(key)
            .or_insert_with(|| PackageStateSnapshot {
                snapshot_id: next_id("base"),
                serial: serial.to_string(),
                user_id,
                package_name: package_name.to_string(),
                snapshot_type: SnapshotType::InitialBaseline,
                effective_state: state.effective_state,
                configured_state: state.configured_state,
                configured_state_code: state.configured_state_code,
                android_version: android_version.map(str::to_string),
                captured_at_ms: now_epoch_ms(),
            })
            .clone()
    }

    /// Records a Pre-Operation snapshot immediately before an individual mutation.
    pub fn record_pre_operation(
        &mut self,
        serial: &str,
        user_id: u32,
        package_name: &str,
        state: &CapturedPackageState,
        android_version: Option<&str>,
    ) -> PackageStateSnapshot {
        let key = (serial.to_string(), user_id, package_name.to_string());
        let snapshot = PackageStateSnapshot {
            snapshot_id: next_id("pre"),
            serial: serial.to_string(),
            user_id,
            package_name: package_name.to_string(),
            snapshot_type: SnapshotType::PreOperation,
            effective_state: state.effective_state,
            configured_state: state.configured_state,
            configured_state_code: state.configured_state_code,
            android_version: android_version.map(str::to_string),
            captured_at_ms: now_epoch_ms(),
        };
        self.pre_operations
            .entry(key)
            .or_default()
            .push(snapshot.clone());
        snapshot
    }

    pub fn get_baseline(
        &self,
        serial: &str,
        user_id: u32,
        package_name: &str,
    ) -> Option<PackageStateSnapshot> {
        self.baselines
            .get(&(serial.to_string(), user_id, package_name.to_string()))
            .cloned()
    }

    pub fn get_latest_pre_operation(
        &self,
        serial: &str,
        user_id: u32,
        package_name: &str,
    ) -> Option<PackageStateSnapshot> {
        self.pre_operations
            .get(&(serial.to_string(), user_id, package_name.to_string()))
            .and_then(|stack| stack.last().cloned())
    }

    pub fn list_statuses_for_device(
        &self,
        serial: &str,
        user_id: u32,
    ) -> HashMap<String, PackageSnapshotStatus> {
        let mut map: HashMap<String, PackageSnapshotStatus> = HashMap::new();

        for ((s, uid, pkg), baseline) in &self.baselines {
            if s == serial && *uid == user_id {
                map.entry(pkg.clone())
                    .or_insert_with(|| PackageSnapshotStatus {
                        package_name: pkg.clone(),
                        baseline: None,
                        latest_pre_operation: None,
                        pre_operation_count: 0,
                    })
                    .baseline = Some(baseline.clone());
            }
        }

        for ((s, uid, pkg), stack) in &self.pre_operations {
            if s == serial && *uid == user_id {
                let entry = map
                    .entry(pkg.clone())
                    .or_insert_with(|| PackageSnapshotStatus {
                        package_name: pkg.clone(),
                        baseline: None,
                        latest_pre_operation: None,
                        pre_operation_count: 0,
                    });
                entry.latest_pre_operation = stack.last().cloned();
                entry.pre_operation_count = stack.len();
            }
        }

        map
    }

    pub fn record_operation_result(&mut self, result: PackageOperationResult) {
        self.session_operations.push(result);
    }

    pub fn list_session_operations(&self, serial: &str) -> Vec<PackageOperationResult> {
        self.session_operations
            .iter()
            .filter(|op| op.serial == serial)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::package_models::{ConfiguredPackageState, EffectivePackageState};

    #[test]
    fn baseline_is_never_overwritten_while_pre_operation_updates_each_time() {
        let mut store = SnapshotStore::new();

        let initial = CapturedPackageState {
            effective_state: EffectivePackageState::Enabled,
            configured_state: ConfiguredPackageState::Default,
            configured_state_code: Some(0),
            installed_for_user: Some(true),
        };
        let after_disable = CapturedPackageState {
            effective_state: EffectivePackageState::Disabled,
            configured_state: ConfiguredPackageState::DisabledUser,
            configured_state_code: Some(3),
            installed_for_user: Some(true),
        };

        let first_base =
            store.capture_baseline_if_absent("DEV1", 0, "com.example.app", &initial, Some("14"));
        let second_base = store.capture_baseline_if_absent(
            "DEV1",
            0,
            "com.example.app",
            &after_disable,
            Some("14"),
        );

        assert_eq!(first_base.snapshot_id, second_base.snapshot_id);
        assert_eq!(second_base.configured_state, ConfiguredPackageState::Default);
        assert_eq!(second_base.configured_state_code, Some(0));

        store.record_pre_operation("DEV1", 0, "com.example.app", &initial, Some("14"));
        store.record_pre_operation("DEV1", 0, "com.example.app", &after_disable, Some("14"));

        let latest_pre = store
            .get_latest_pre_operation("DEV1", 0, "com.example.app")
            .expect("latest pre-op snapshot should exist");
        assert_eq!(latest_pre.configured_state, ConfiguredPackageState::DisabledUser);
        assert_eq!(latest_pre.configured_state_code, Some(3));
    }
}
