export interface ProcessMemoryEntry {
  processName: string;
  pid: number | null;
  pssKb: number;
}

export interface RunningProcessEntry {
  user: string;
  pid: number;
  rssKb: number | null;
  name: string;
}

export interface MemorySnapshot {
  snapshotId: string;
  serial: string;
  label: string;
  capturedAtMs: number;
  captureDurationMs: number;
  totalKb: number | null;
  availableKb: number | null;
  freeKb: number | null;
  cachedKb: number | null;
  buffersKb: number | null;
  swapTotalKb: number | null;
  swapFreeKb: number | null;
  usedEstimatedKb: number | null;
  dumpsysTotalRamKb: number | null;
  dumpsysFreeRamKb: number | null;
  dumpsysUsedRamKb: number | null;
  dumpsysLostRamKb: number | null;
  topProcesses: ProcessMemoryEntry[];
  runningProcessCount: number | null;
  runningProcesses: RunningProcessEntry[];
  warnings: string[];
}

export interface ProcessMemoryDelta {
  processName: string;
  beforePssKb: number | null;
  afterPssKb: number | null;
  deltaKb: number;
}

export interface MemoryComparison {
  beforeSnapshotId: string;
  beforeLabel: string;
  beforeCapturedAtMs: number;
  afterSnapshotId: string;
  afterLabel: string;
  afterCapturedAtMs: number;
  elapsedMs: number;
  availableDeltaKb: number | null;
  freeDeltaKb: number | null;
  cachedDeltaKb: number | null;
  usedEstimatedDeltaKb: number | null;
  processCountDelta: number | null;
  topProcessDeltas: ProcessMemoryDelta[];
  interpretationNotes: string[];
}

export interface StoragePartitionEntry {
  filesystem: string;
  mountPoint: string;
  totalKb: number | null;
  usedKb: number | null;
  availableKb: number | null;
  usePercent: number | null;
  isReadOnlySystemPartition: boolean;
}

export interface StorageDashboard {
  serial: string;
  capturedAtMs: number;
  captureDurationMs: number;
  dataPartition: StoragePartitionEntry;
  partitions: StoragePartitionEntry[];
  disablingStorageNote: string;
  advancedRemovalNote: string;
  warnings: string[];
}

export interface PerformanceMetrics {
  serial: string;
  lastPackageScanMs: number | null;
  lastPackageCount: number | null;
  lastMemoryProbeMs: number | null;
  lastStorageProbeMs: number | null;
}
