import { useEffect, useState } from "react";
import { AlertBanner } from "../../components/feedback/AlertBanner";
import { IconDownload } from "../../components/shared/Icons";
import type { AppSettings, DiagnosticBundle } from "../../types/history";
import type { RuleDatabaseSummary } from "../../types/rules";
import styles from "./SettingsPanel.module.css";

interface SettingsPanelProps {
  settings: AppSettings;
  ruleDbSummary: RuleDatabaseSummary | null;
  diagnosticBundle: DiagnosticBundle | null;
  busy: boolean;
  onSaveSettings: (next: AppSettings) => void;
  onGenerateDiagnostics: () => void;
}

export function SettingsPanel({
  settings,
  ruleDbSummary,
  diagnosticBundle,
  busy,
  onSaveSettings,
  onGenerateDiagnostics,
}: SettingsPanelProps) {
  const [adbPath, setAdbPath] = useState(settings.customAdbPath ?? "");
  const [autoScan, setAutoScan] = useState(settings.autoScanOnSelect);
  const [logLevel, setLogLevel] = useState<"minimal" | "standard">(
    settings.logLevel,
  );
  const [savedNotice, setSavedNotice] = useState(false);
  const [showJsonPreview, setShowJsonPreview] = useState(true);

  useEffect(() => {
    setAdbPath(settings.customAdbPath ?? "");
    setAutoScan(settings.autoScanOnSelect);
    setLogLevel(settings.logLevel);
  }, [settings]);

  const handleSave = () => {
    setSavedNotice(false);
    onSaveSettings({
      customAdbPath: adbPath.trim() ? adbPath.trim() : null,
      autoScanOnSelect: autoScan,
      logLevel,
    });
    setSavedNotice(true);
  };

  const handleDownloadDiagnostics = () => {
    if (!diagnosticBundle) {
      return;
    }
    const json = JSON.stringify(diagnosticBundle, null, 2);
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `adt-diagnostics-redacted-${diagnosticBundle.exportedAtMs}.json`;
    document.body.appendChild(link);
    link.click();
    link.remove();
    URL.revokeObjectURL(url);
  };

  return (
    <div className={styles.stack}>
      <section className={styles.card}>
        <h2 className={styles.heading}>Local Application Settings</h2>
        <p className={styles.muted}>
          Settings are saved locally in SQLite. ADT operates offline-first and never uploads device inventories or telemetry.
        </p>

        {savedNotice && (
          <AlertBanner
            tone="info"
            title="Settings saved locally."
            onDismiss={() => setSavedNotice(false)}
          />
        )}

        <div className={styles.field}>
          <label htmlFor="settings-adb-path" className={styles.label}>
            Custom ADB executable path (optional)
          </label>
          <input
            id="settings-adb-path"
            type="text"
            className={styles.input}
            placeholder="Leave empty to detect adb via system PATH"
            value={adbPath}
            onChange={(e) => setAdbPath(e.target.value)}
          />
        </div>

        <label className={styles.checkbox}>
          <input
            type="checkbox"
            checked={autoScan}
            onChange={(e) => setAutoScan(e.target.checked)}
          />
          Automatically scan installed packages after selecting an authorized device
        </label>

        <div className={styles.field}>
          <label htmlFor="settings-log-level" className={styles.label}>
            Diagnostic verbosity
          </label>
          <select
            id="settings-log-level"
            className={styles.select}
            value={logLevel}
            onChange={(e) =>
              setLogLevel(e.target.value === "minimal" ? "minimal" : "standard")
            }
          >
            <option value="standard">Standard (recommended)</option>
            <option value="minimal">Minimal</option>
          </select>
        </div>

        <p className={styles.muted}>
          Confirmation policy: Explicit preview and confirmation before package mutations is always enforced.
        </p>

        <div className={styles.row}>
          <button
            type="button"
            className={`${styles.button} ${styles.buttonPrimary}`}
            disabled={busy}
            onClick={handleSave}
          >
            Save Settings
          </button>
        </div>
      </section>

      {ruleDbSummary && (
        <section className={styles.card}>
          <h2 className={styles.heading}>Offline Recommendation Rule Database</h2>
          <p className={styles.muted}>
            Version <strong>{ruleDbSummary.databaseVersion}</strong> (schema v
            {ruleDbSummary.schemaVersion}):{" "}
            <strong>{ruleDbSummary.activeRules}</strong> active rules (
            <strong>{ruleDbSummary.verifiedRules}</strong> verified,{" "}
            <strong>{ruleDbSummary.communityReportedRules}</strong>{" "}
            community-reported, <strong>{ruleDbSummary.draftRules}</strong>{" "}
            draft, <strong>{ruleDbSummary.deprecatedRules}</strong> deprecated).
          </p>
        </section>
      )}

      <section className={styles.card}>
        <h2 className={styles.heading}>Privacy-Aware Diagnostic Export</h2>
        <ul className={styles.list}>
          <li>Raw hardware ADB serials are never stored and are replaced with [REDACTED_SERIAL].</li>
          <li>OS user profile folder names (for example, C:\Users\name) are replaced with [REDACTED].</li>
          <li>Raw shell command strings are omitted; only structured action records are exported.</li>
          <li>Export runs strictly on demand and remains local to your machine.</li>
        </ul>

        <div className={styles.row}>
          <button
            type="button"
            className={styles.button}
            disabled={busy}
            onClick={onGenerateDiagnostics}
          >
            Generate Redacted Diagnostic Bundle
          </button>
          {diagnosticBundle && (
            <>
              <button
                type="button"
                className={`${styles.button} ${styles.buttonPrimary}`}
                onClick={handleDownloadDiagnostics}
              >
                <IconDownload size={13} />
                <span>Download JSON</span>
              </button>
              <button
                type="button"
                className={styles.button}
                onClick={() => setShowJsonPreview((prev) => !prev)}
              >
                {showJsonPreview ? "Hide Preview" : "Show Preview"}
              </button>
            </>
          )}
        </div>

        {diagnosticBundle && showJsonPreview && (
          <pre className={styles.pre}>
            {JSON.stringify(diagnosticBundle, null, 2)}
          </pre>
        )}
      </section>
    </div>
  );
}
