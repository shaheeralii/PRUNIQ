use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::core::errors::AdtError;
use crate::core::package_models::{
    ConfiguredPackageState, EffectivePackageState, InstalledPackage,
};
use crate::operations::models::{
    CapturedPackageState, OperationStatus, PackageAction, PackageOperationResult,
    PackageSnapshotStatus, PackageStateSnapshot, RestorationFidelity, SnapshotType,
};
use crate::operations::snapshots::{next_id, now_epoch_ms};
use crate::storage::models::{AppSettings, DeviceRecord};

const MIGRATION_0001: &str = include_str!("../../migrations/0001_initial.sql");
const LATEST_MIGRATION_VERSION: i64 = 1;

/// Local SQLite storage manager for ADT devices (random local UUIDs, zero raw serials),
/// package cache, immutable baseline & pre-operation snapshots, operation history, and settings.
pub struct Storage {
    conn: Mutex<Connection>,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self, AdtError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|err| AdtError::DatabaseError {
                    message: format!("failed to create database directory: {err}"),
                })?;
            }
        }
        let conn = Connection::open(path).map_err(map_db_err)?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Self, AdtError> {
        let conn = Connection::open_in_memory().map_err(map_db_err)?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Self, AdtError> {
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;\n\
             PRAGMA journal_mode = WAL;\n\
             PRAGMA synchronous = NORMAL;",
        )
        .map_err(map_db_err)?;

        run_migrations(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn current_schema_version(&self) -> Result<i64, AdtError> {
        let conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        let version: Option<i64> = conn
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .optional()
            .map_err(map_db_err)?
            .flatten();
        Ok(version.unwrap_or(0))
    }

    /// Upserts a local device profile identified only by a random local UUID.
    /// Never persists raw hardware ADB serial numbers.
    pub fn upsert_device_profile(
        &self,
        existing_device_id: Option<&str>,
        manufacturer: Option<&str>,
        model: Option<&str>,
        codename: Option<&str>,
        android_version: Option<&str>,
        api_level: Option<u32>,
    ) -> Result<DeviceRecord, AdtError> {
        let conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        let now_ms = now_epoch_ms();
        let now_i64 = to_i64(now_ms);
        let api_i64 = api_level.map(i64::from);

        let resolved_id = if let Some(id) = existing_device_id {
            id.to_string()
        } else if let (Some(mfg), Some(mdl), Some(code)) = (manufacturer, model, codename) {
            let found: Option<String> = conn
                .query_row(
                    "SELECT device_id FROM devices
                     WHERE manufacturer = ?1 AND model = ?2 AND codename = ?3
                     ORDER BY last_connection_ms DESC LIMIT 1",
                    params![mfg, mdl, code],
                    |row| row.get(0),
                )
                .optional()
                .map_err(map_db_err)?;
            found.unwrap_or_else(|| Uuid::new_v4().to_string())
        } else {
            Uuid::new_v4().to_string()
        };

        conn.execute(
            "INSERT INTO devices (device_id, manufacturer, model, codename, android_version, api_level, last_connection_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(device_id) DO UPDATE SET
               manufacturer = COALESCE(excluded.manufacturer, devices.manufacturer),
               model = COALESCE(excluded.model, devices.model),
               codename = COALESCE(excluded.codename, devices.codename),
               android_version = COALESCE(excluded.android_version, devices.android_version),
               api_level = COALESCE(excluded.api_level, devices.api_level),
               last_connection_ms = excluded.last_connection_ms",
            params![
                resolved_id,
                manufacturer,
                model,
                codename,
                android_version,
                api_i64,
                now_i64,
            ],
        )
        .map_err(map_db_err)?;

        let record = conn
            .query_row(
                "SELECT device_id, manufacturer, model, codename, android_version, api_level, last_connection_ms
                 FROM devices WHERE device_id = ?1",
                params![resolved_id],
                |row| {
                    let api: Option<i64> = row.get(5)?;
                    let last_ms: i64 = row.get(6)?;
                    Ok(DeviceRecord {
                        device_id: row.get(0)?,
                        manufacturer: row.get(1)?,
                        model: row.get(2)?,
                        codename: row.get(3)?,
                        android_version: row.get(4)?,
                        api_level: api.and_then(|v| u32::try_from(v).ok()),
                        last_connection_ms: to_u64(last_ms),
                    })
                },
            )
            .map_err(map_db_err)?;

        Ok(record)
    }

    pub fn list_devices(&self) -> Result<Vec<DeviceRecord>, AdtError> {
        let conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        let mut stmt = conn
            .prepare(
                "SELECT device_id, manufacturer, model, codename, android_version, api_level, last_connection_ms
                 FROM devices ORDER BY last_connection_ms DESC",
            )
            .map_err(map_db_err)?;

        let rows = stmt
            .query_map([], |row| {
                let api: Option<i64> = row.get(5)?;
                let last_ms: i64 = row.get(6)?;
                Ok(DeviceRecord {
                    device_id: row.get(0)?,
                    manufacturer: row.get(1)?,
                    model: row.get(2)?,
                    codename: row.get(3)?,
                    android_version: row.get(4)?,
                    api_level: api.and_then(|v| u32::try_from(v).ok()),
                    last_connection_ms: to_u64(last_ms),
                })
            })
            .map_err(map_db_err)?;

        rows.collect::<Result<Vec<_>, _>>().map_err(map_db_err)
    }

    pub fn cache_scanned_packages(
        &self,
        device_id: &str,
        user_id: u32,
        packages: &[InstalledPackage],
    ) -> Result<(), AdtError> {
        let mut conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        let now_i64 = to_i64(now_epoch_ms());
        let uid_i64 = i64::from(user_id);

        let tx = conn.transaction().map_err(map_db_err)?;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO package_cache (
                        device_id, user_id, package_name, app_label, label_source,
                        version_name, apk_path, category_json, effective_state,
                        state_confidence, last_scan_ms
                     ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?8, ?9, ?10)
                     ON CONFLICT(device_id, user_id, package_name) DO UPDATE SET
                        app_label = excluded.app_label,
                        label_source = excluded.label_source,
                        apk_path = excluded.apk_path,
                        category_json = excluded.category_json,
                        effective_state = excluded.effective_state,
                        state_confidence = excluded.state_confidence,
                        last_scan_ms = excluded.last_scan_ms",
                )
                .map_err(map_db_err)?;

            for pkg in packages {
                let category_json = serde_json::json!({
                    "origin": pkg.origin,
                    "partition": pkg.partition,
                    "isSystem": pkg.is_system,
                    "isThirdParty": pkg.is_third_party,
                    "isUpdatedSystem": pkg.is_updated_system,
                    "isGoogle": pkg.is_google,
                    "isOem": pkg.is_oem,
                })
                .to_string();
                let label_source = enum_to_json_str(&pkg.label_source);
                let effective_state = enum_to_json_str(&pkg.effective_state);
                let confidence = if pkg.effective_state == EffectivePackageState::Unknown {
                    "low"
                } else {
                    "bulk_verified"
                };

                stmt.execute(params![
                    device_id,
                    uid_i64,
                    pkg.package_name,
                    pkg.label,
                    label_source,
                    pkg.apk_path,
                    category_json,
                    effective_state,
                    confidence,
                    now_i64,
                ])
                .map_err(map_db_err)?;
            }
        }
        tx.commit().map_err(map_db_err)?;
        Ok(())
    }

    /// Captures the Initial Toolkit Baseline ONLY if none exists for `(device_id, user_id, package_name)`.
    /// Protected both in logic and by a SQLite partial unique index so it is never overwritten.
    pub fn capture_baseline_if_absent(
        &self,
        device_id: &str,
        user_id: u32,
        package_name: &str,
        state: &CapturedPackageState,
        android_version: Option<&str>,
    ) -> Result<PackageStateSnapshot, AdtError> {
        let conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(existing) = query_latest_snapshot(
            &conn,
            device_id,
            user_id,
            package_name,
            SnapshotType::InitialBaseline,
        )? {
            return Ok(existing);
        }

        let snapshot = PackageStateSnapshot {
            snapshot_id: next_id("base"),
            device_id: device_id.to_string(),
            user_id,
            package_name: package_name.to_string(),
            snapshot_type: SnapshotType::InitialBaseline,
            effective_state: state.effective_state,
            configured_state: state.configured_state,
            configured_state_code: state.configured_state_code,
            android_version: android_version.map(str::to_string),
            captured_at_ms: now_epoch_ms(),
        };

        insert_snapshot(&conn, &snapshot)?;
        Ok(snapshot)
    }

    pub fn record_pre_operation(
        &self,
        device_id: &str,
        user_id: u32,
        package_name: &str,
        state: &CapturedPackageState,
        android_version: Option<&str>,
    ) -> Result<PackageStateSnapshot, AdtError> {
        let conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        let snapshot = PackageStateSnapshot {
            snapshot_id: next_id("pre"),
            device_id: device_id.to_string(),
            user_id,
            package_name: package_name.to_string(),
            snapshot_type: SnapshotType::PreOperation,
            effective_state: state.effective_state,
            configured_state: state.configured_state,
            configured_state_code: state.configured_state_code,
            android_version: android_version.map(str::to_string),
            captured_at_ms: now_epoch_ms(),
        };

        insert_snapshot(&conn, &snapshot)?;
        Ok(snapshot)
    }

    pub fn get_baseline(
        &self,
        device_id: &str,
        user_id: u32,
        package_name: &str,
    ) -> Result<Option<PackageStateSnapshot>, AdtError> {
        let conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        query_latest_snapshot(
            &conn,
            device_id,
            user_id,
            package_name,
            SnapshotType::InitialBaseline,
        )
    }

    pub fn get_latest_pre_operation(
        &self,
        device_id: &str,
        user_id: u32,
        package_name: &str,
    ) -> Result<Option<PackageStateSnapshot>, AdtError> {
        let conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        query_latest_snapshot(
            &conn,
            device_id,
            user_id,
            package_name,
            SnapshotType::PreOperation,
        )
    }

    pub fn list_snapshot_statuses_for_device(
        &self,
        device_id: &str,
        user_id: u32,
    ) -> Result<HashMap<String, PackageSnapshotStatus>, AdtError> {
        let conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        let mut stmt = conn
            .prepare(
                "SELECT snapshot_id, device_id, user_id, package_name, snapshot_type,
                        effective_state, configured_state, configured_state_code,
                        android_version, captured_at_ms
                 FROM state_snapshots
                 WHERE device_id = ?1 AND user_id = ?2
                 ORDER BY captured_at_ms ASC, rowid ASC",
            )
            .map_err(map_db_err)?;

        let rows = stmt
            .query_map(params![device_id, i64::from(user_id)], row_to_snapshot)
            .map_err(map_db_err)?;

        let mut map: HashMap<String, PackageSnapshotStatus> = HashMap::new();
        for row in rows {
            let snap = row.map_err(map_db_err)?;
            let entry = map
                .entry(snap.package_name.clone())
                .or_insert_with(|| PackageSnapshotStatus {
                    package_name: snap.package_name.clone(),
                    baseline: None,
                    latest_pre_operation: None,
                    pre_operation_count: 0,
                });

            match snap.snapshot_type {
                SnapshotType::InitialBaseline => entry.baseline = Some(snap),
                SnapshotType::PreOperation => {
                    entry.latest_pre_operation = Some(snap);
                    entry.pre_operation_count += 1;
                }
            }
        }

        Ok(map)
    }

    pub fn record_operation_result(
        &self,
        result: &PackageOperationResult,
        target_state: Option<&str>,
    ) -> Result<(), AdtError> {
        let conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        let prev_json = result
            .previous_state
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(map_serde_err)?;
        let verified_json = result
            .verified_state
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(map_serde_err)?;

        conn.execute(
            "INSERT INTO operation_history (
                operation_id, device_id, user_id, package_name, action_type,
                previous_state_json, target_state, verified_state_json,
                status, restoration_fidelity, executed_at_ms,
                sanitized_error, limitation_note
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                result.operation_id,
                result.device_id,
                i64::from(result.user_id),
                result.package_name,
                enum_to_json_str(&result.action),
                prev_json,
                target_state,
                verified_json,
                enum_to_json_str(&result.status),
                enum_to_json_str(&result.restoration_fidelity),
                to_i64(result.executed_at_ms),
                result.error_message,
                result.limitation_note,
            ],
        )
        .map_err(map_db_err)?;

        Ok(())
    }

    pub fn get_operation_by_id(
        &self,
        operation_id: &str,
    ) -> Result<Option<(PackageOperationResult, Option<String>)>, AdtError> {
        let conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        conn.query_row(
            "SELECT operation_id, device_id, user_id, package_name, action_type,
                    previous_state_json, target_state, verified_state_json,
                    status, restoration_fidelity, executed_at_ms,
                    sanitized_error, limitation_note
             FROM operation_history
             WHERE operation_id = ?1",
            params![operation_id],
            row_to_operation_with_target,
        )
        .optional()
        .map_err(map_db_err)
    }

    pub fn update_reconciled_operation(
        &self,
        operation_id: &str,
        status: OperationStatus,
        verified_state: &CapturedPackageState,
        restoration_fidelity: RestorationFidelity,
        limitation_note: Option<&str>,
        sanitized_error: Option<&str>,
    ) -> Result<PackageOperationResult, AdtError> {
        let conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        let verified_json = serde_json::to_string(verified_state).map_err(map_serde_err)?;

        let updated = conn
            .execute(
                "UPDATE operation_history
                 SET status = ?1,
                     verified_state_json = ?2,
                     restoration_fidelity = ?3,
                     limitation_note = ?4,
                     sanitized_error = ?5
                 WHERE operation_id = ?6",
                params![
                    enum_to_json_str(&status),
                    verified_json,
                    enum_to_json_str(&restoration_fidelity),
                    limitation_note,
                    sanitized_error,
                    operation_id,
                ],
            )
            .map_err(map_db_err)?;

        if updated == 0 {
            return Err(AdtError::OperationNotFound {
                operation_id: operation_id.to_string(),
            });
        }

        let (record, _) = conn
            .query_row(
                "SELECT operation_id, device_id, user_id, package_name, action_type,
                        previous_state_json, target_state, verified_state_json,
                        status, restoration_fidelity, executed_at_ms,
                        sanitized_error, limitation_note
                 FROM operation_history
                 WHERE operation_id = ?1",
                params![operation_id],
                row_to_operation_with_target,
            )
            .map_err(map_db_err)?;

        Ok(record)
    }

    pub fn list_operation_history(
        &self,
        device_id: Option<&str>,
        status_filter: Option<&str>,
        search: Option<&str>,
        limit: usize,
    ) -> Result<Vec<PackageOperationResult>, AdtError> {
        let conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        let search_like = search
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{}%", s.to_ascii_lowercase()));

        let status_norm = status_filter
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != "all");

        let mut stmt = conn
            .prepare(
                "SELECT operation_id, device_id, user_id, package_name, action_type,
                        previous_state_json, target_state, verified_state_json,
                        status, restoration_fidelity, executed_at_ms,
                        sanitized_error, limitation_note
                 FROM operation_history
                 WHERE (?1 IS NULL OR device_id = ?1)
                   AND (?2 IS NULL OR status = ?2)
                   AND (?3 IS NULL OR LOWER(package_name) LIKE ?3 OR LOWER(COALESCE(limitation_note, '')) LIKE ?3)
                 ORDER BY executed_at_ms DESC, rowid DESC
                 LIMIT ?4",
            )
            .map_err(map_db_err)?;

        let limit_i64 = i64::try_from(limit).unwrap_or(500);
        let rows = stmt
            .query_map(
                params![device_id, status_norm, search_like, limit_i64],
                row_to_operation_with_target,
            )
            .map_err(map_db_err)?;

        let mut out = Vec::new();
        for row in rows {
            let (op, _) = row.map_err(map_db_err)?;
            out.push(op);
        }
        Ok(out)
    }

    /// Clears operation history entries while preserving immutable `state_snapshots`
    /// so Restore Original State capability is never accidentally destroyed.
    pub fn clear_operation_history(&self, device_id: Option<&str>) -> Result<usize, AdtError> {
        let conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        let deleted = match device_id {
            Some(id) => conn
                .execute(
                    "DELETE FROM operation_history WHERE device_id = ?1",
                    params![id],
                )
                .map_err(map_db_err)?,
            None => conn
                .execute("DELETE FROM operation_history", [])
                .map_err(map_db_err)?,
        };
        Ok(deleted)
    }

    pub fn get_settings(&self) -> Result<AppSettings, AdtError> {
        let conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        let mut stmt = conn
            .prepare("SELECT key, value FROM settings")
            .map_err(map_db_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(map_db_err)?;

        let mut settings = AppSettings::default();
        for row in rows {
            let (key, val) = row.map_err(map_db_err)?;
            match key.as_str() {
                "custom_adb_path" => {
                    let trimmed = val.trim().to_string();
                    settings.custom_adb_path = if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed)
                    };
                }
                "auto_scan_on_select" => {
                    settings.auto_scan_on_select = val.eq_ignore_ascii_case("true");
                }
                "log_level" => {
                    if matches!(val.as_str(), "minimal" | "standard") {
                        settings.log_level = val;
                    }
                }
                _ => {}
            }
        }
        Ok(settings)
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<AppSettings, AdtError> {
        let mut conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        let now_i64 = to_i64(now_epoch_ms());

        let normalized = AppSettings {
            custom_adb_path: settings
                .custom_adb_path
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            auto_scan_on_select: settings.auto_scan_on_select,
            log_level: if settings.log_level == "minimal" {
                "minimal".to_string()
            } else {
                "standard".to_string()
            },
        };

        let tx = conn.transaction().map_err(map_db_err)?;
        upsert_setting(
            &tx,
            "custom_adb_path",
            normalized.custom_adb_path.as_deref().unwrap_or(""),
            now_i64,
        )?;
        upsert_setting(
            &tx,
            "auto_scan_on_select",
            if normalized.auto_scan_on_select {
                "true"
            } else {
                "false"
            },
            now_i64,
        )?;
        upsert_setting(&tx, "log_level", &normalized.log_level, now_i64)?;
        tx.commit().map_err(map_db_err)?;

        Ok(normalized)
    }
}

fn run_migrations(conn: &Connection) -> Result<(), AdtError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at_ms INTEGER NOT NULL
        );",
    )
    .map_err(map_db_err)?;

    let current: i64 = conn
        .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .map_err(map_db_err)?;

    if current < LATEST_MIGRATION_VERSION {
        conn.execute_batch(MIGRATION_0001).map_err(map_db_err)?;
        conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, applied_at_ms) VALUES (?1, ?2)",
            params![LATEST_MIGRATION_VERSION, to_i64(now_epoch_ms())],
        )
        .map_err(map_db_err)?;
    }

    Ok(())
}

fn insert_snapshot(conn: &Connection, snap: &PackageStateSnapshot) -> Result<(), AdtError> {
    conn.execute(
        "INSERT INTO state_snapshots (
            snapshot_id, device_id, user_id, package_name, snapshot_type,
            effective_state, configured_state, configured_state_code,
            captured_at_ms, android_version, restoration_fidelity
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'not_applicable')",
        params![
            snap.snapshot_id,
            snap.device_id,
            i64::from(snap.user_id),
            snap.package_name,
            enum_to_json_str(&snap.snapshot_type),
            enum_to_json_str(&snap.effective_state),
            enum_to_json_str(&snap.configured_state),
            snap.configured_state_code,
            to_i64(snap.captured_at_ms),
            snap.android_version,
        ],
    )
    .map_err(map_db_err)?;
    Ok(())
}

fn query_latest_snapshot(
    conn: &Connection,
    device_id: &str,
    user_id: u32,
    package_name: &str,
    snapshot_type: SnapshotType,
) -> Result<Option<PackageStateSnapshot>, AdtError> {
    conn.query_row(
        "SELECT snapshot_id, device_id, user_id, package_name, snapshot_type,
                effective_state, configured_state, configured_state_code,
                android_version, captured_at_ms
         FROM state_snapshots
         WHERE device_id = ?1 AND user_id = ?2 AND package_name = ?3 AND snapshot_type = ?4
         ORDER BY captured_at_ms DESC, rowid DESC
         LIMIT 1",
        params![
            device_id,
            i64::from(user_id),
            package_name,
            enum_to_json_str(&snapshot_type),
        ],
        row_to_snapshot,
    )
    .optional()
    .map_err(map_db_err)
}

fn row_to_snapshot(row: &rusqlite::Row<'_>) -> rusqlite::Result<PackageStateSnapshot> {
    let uid_i64: i64 = row.get(2)?;
    let snap_type_str: String = row.get(4)?;
    let eff_str: String = row.get(5)?;
    let cfg_str: String = row.get(6)?;
    let captured_i64: i64 = row.get(9)?;

    Ok(PackageStateSnapshot {
        snapshot_id: row.get(0)?,
        device_id: row.get(1)?,
        user_id: u32::try_from(uid_i64).unwrap_or(0),
        package_name: row.get(3)?,
        snapshot_type: enum_from_json_str(&snap_type_str).unwrap_or(SnapshotType::PreOperation),
        effective_state: enum_from_json_str(&eff_str).unwrap_or(EffectivePackageState::Unknown),
        configured_state: enum_from_json_str(&cfg_str).unwrap_or(ConfiguredPackageState::Unknown),
        configured_state_code: row.get(7)?,
        android_version: row.get(8)?,
        captured_at_ms: to_u64(captured_i64),
    })
}

fn row_to_operation_with_target(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<(PackageOperationResult, Option<String>)> {
    let uid_i64: i64 = row.get(2)?;
    let action_str: String = row.get(4)?;
    let prev_json: Option<String> = row.get(5)?;
    let target_state: Option<String> = row.get(6)?;
    let verified_json: Option<String> = row.get(7)?;
    let status_str: String = row.get(8)?;
    let fidelity_str: String = row.get(9)?;
    let executed_i64: i64 = row.get(10)?;

    let previous_state = prev_json
        .as_deref()
        .and_then(|s| serde_json::from_str::<CapturedPackageState>(s).ok());
    let verified_state = verified_json
        .as_deref()
        .and_then(|s| serde_json::from_str::<CapturedPackageState>(s).ok());

    let record = PackageOperationResult {
        operation_id: row.get(0)?,
        device_id: row.get(1)?,
        user_id: u32::try_from(uid_i64).unwrap_or(0),
        package_name: row.get(3)?,
        action: enum_from_json_str(&action_str).unwrap_or(PackageAction::KeepCurrent),
        status: enum_from_json_str(&status_str).unwrap_or(OperationStatus::Failed),
        previous_state,
        verified_state,
        restoration_fidelity: enum_from_json_str(&fidelity_str)
            .unwrap_or(RestorationFidelity::NotApplicable),
        blocked_reasons: Vec::new(),
        limitation_note: row.get(12)?,
        error_message: row.get(11)?,
        executed_at_ms: to_u64(executed_i64),
    };

    Ok((record, target_state))
}

fn upsert_setting(
    conn: &Connection,
    key: &str,
    value: &str,
    updated_at_ms: i64,
) -> Result<(), AdtError> {
    conn.execute(
        "INSERT INTO settings (key, value, updated_at_ms) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at_ms = excluded.updated_at_ms",
        params![key, value, updated_at_ms],
    )
    .map_err(map_db_err)?;
    Ok(())
}

fn enum_to_json_str<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn enum_from_json_str<T: serde::de::DeserializeOwned>(raw: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(raw.to_string())).ok()
}

fn to_i64(v: u64) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

fn to_u64(v: i64) -> u64 {
    u64::try_from(v).unwrap_or(0)
}

fn map_db_err(err: rusqlite::Error) -> AdtError {
    AdtError::DatabaseError {
        message: err.to_string(),
    }
}

fn map_serde_err(err: serde_json::Error) -> AdtError {
    AdtError::DatabaseError {
        message: format!("serialization error: {err}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_baseline_is_immutable_while_pre_operation_stack_grows() {
        let storage = Storage::open_in_memory().expect("in-memory db");
        assert_eq!(storage.current_schema_version().expect("version"), 1);

        let device = storage
            .upsert_device_profile(None, Some("samsung"), Some("SM-A528B"), Some("a52sxq"), Some("14"), Some(34))
            .expect("device profile");

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

        let first_base = storage
            .capture_baseline_if_absent(&device.device_id, 0, "com.example.app", &initial, Some("14"))
            .expect("first baseline");
        let second_base = storage
            .capture_baseline_if_absent(&device.device_id, 0, "com.example.app", &after_disable, Some("14"))
            .expect("second baseline");

        assert_eq!(first_base.snapshot_id, second_base.snapshot_id);
        assert_eq!(second_base.configured_state, ConfiguredPackageState::Default);

        storage
            .record_pre_operation(&device.device_id, 0, "com.example.app", &initial, Some("14"))
            .expect("pre 1");
        storage
            .record_pre_operation(&device.device_id, 0, "com.example.app", &after_disable, Some("14"))
            .expect("pre 2");

        let statuses = storage
            .list_snapshot_statuses_for_device(&device.device_id, 0)
            .expect("statuses");
        let st = statuses.get("com.example.app").expect("status entry");
        assert_eq!(st.pre_operation_count, 2);
        assert_eq!(
            st.latest_pre_operation.as_ref().expect("latest pre").configured_state,
            ConfiguredPackageState::DisabledUser
        );
    }
}
