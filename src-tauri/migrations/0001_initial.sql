CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS devices (
    device_id TEXT PRIMARY KEY,
    manufacturer TEXT,
    model TEXT,
    codename TEXT,
    android_version TEXT,
    api_level INTEGER,
    last_connection_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS package_cache (
    device_id TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    package_name TEXT NOT NULL,
    app_label TEXT NOT NULL,
    label_source TEXT NOT NULL,
    version_name TEXT,
    apk_path TEXT,
    category_json TEXT NOT NULL,
    effective_state TEXT NOT NULL,
    state_confidence TEXT NOT NULL,
    last_scan_ms INTEGER NOT NULL,
    PRIMARY KEY (device_id, user_id, package_name),
    FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS state_snapshots (
    snapshot_id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    package_name TEXT NOT NULL,
    snapshot_type TEXT NOT NULL CHECK (snapshot_type IN ('initial_baseline', 'pre_operation')),
    effective_state TEXT NOT NULL,
    configured_state TEXT NOT NULL,
    configured_state_code INTEGER,
    captured_at_ms INTEGER NOT NULL,
    android_version TEXT,
    restoration_fidelity TEXT NOT NULL DEFAULT 'not_applicable',
    FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_unique_initial_baseline
    ON state_snapshots (device_id, user_id, package_name)
    WHERE snapshot_type = 'initial_baseline';

CREATE INDEX IF NOT EXISTS idx_snapshots_lookup
    ON state_snapshots (device_id, user_id, package_name, snapshot_type, captured_at_ms);

CREATE TABLE IF NOT EXISTS operation_history (
    operation_id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    package_name TEXT NOT NULL,
    action_type TEXT NOT NULL,
    previous_state_json TEXT,
    target_state TEXT,
    verified_state_json TEXT,
    status TEXT NOT NULL,
    restoration_fidelity TEXT NOT NULL,
    executed_at_ms INTEGER NOT NULL,
    sanitized_error TEXT,
    limitation_note TEXT,
    FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_history_device_time
    ON operation_history (device_id, executed_at_ms DESC);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL
);
