import { describe, expect, it } from "vitest";
import {
  filterInstalledPackages,
  formatCommandSupport,
  formatConfidence,
  formatConfiguredState,
  formatEffectiveState,
  formatKb,
  formatMatchScope,
  formatOrigin,
  formatPartition,
  formatRisk,
  formatSupportTier,
  formatVerificationStatus,
  riskTone,
  type ClientPackageFilter,
} from "./format";
import type { InstalledPackage } from "../types/packages";
import type { PackageRecommendation } from "../types/rules";

const SAMPLE_PACKAGES: InstalledPackage[] = [
  {
    packageName: "com.samsung.android.bixby.agent",
    label: "Bixby Voice",
    labelSource: "curated_catalog",
    apkPath: "/system/priv-app/Bixby/Bixby.apk",
    effectiveState: "disabled",
    origin: "oem",
    partition: "system",
    isSystem: true,
    isThirdParty: false,
    isUpdatedSystem: false,
    isGoogle: false,
    isOem: true,
    userId: 0,
  },
  {
    packageName: "com.google.android.youtube",
    label: "YouTube",
    labelSource: "curated_catalog",
    apkPath: "/data/app/~~abc==/com.google.android.youtube/base.apk",
    effectiveState: "enabled",
    origin: "google",
    partition: "data",
    isSystem: true,
    isThirdParty: false,
    isUpdatedSystem: true,
    isGoogle: true,
    isOem: false,
    userId: 0,
  },
  {
    packageName: "org.mozilla.firefox",
    label: "org.mozilla.firefox",
    labelSource: "package_name_fallback",
    apkPath: "/data/app/~~def==/org.mozilla.firefox/base.apk",
    effectiveState: "enabled",
    origin: "third_party",
    partition: "data",
    isSystem: false,
    isThirdParty: true,
    isUpdatedSystem: false,
    isGoogle: false,
    isOem: false,
    userId: 0,
  },
];

const SAMPLE_RECOMMENDATIONS: Record<string, PackageRecommendation> = {
  "com.samsung.android.bixby.agent": {
    packageName: "com.samsung.android.bixby.agent",
    risk: "caution",
    matchScope: "manufacturer",
    matchedRuleId: "test-bixby",
    appLabel: "Bixby Voice",
    purpose: "Voice assistant",
    knownConsequences: "Voice wake features stop",
    compatibilityNotes: null,
    confidence: "medium",
    verificationStatus: "community_reported",
    lastVerifiedDate: null,
    reviewer: null,
    evidence: [],
    conflictDetected: false,
    conflictNote: null,
    explanation: "Test recommendation entry",
  },
};

const BASE_FILTER: ClientPackageFilter = {
  search: "",
  origin: "all",
  state: "all",
  partition: "all",
  risk: "all",
  updatedSystemOnly: false,
};

describe("format helpers", () => {
  it("formats kilobytes across KB, MB, GB, and unavailable values", () => {
    expect(formatKb(null)).toBe("Unavailable");
    expect(formatKb(undefined)).toBe("Unavailable");
    expect(formatKb(Number.NaN)).toBe("Unavailable");
    expect(formatKb(512)).toBe("512 KB");
    expect(formatKb(2048)).toBe("2.0 MB");
    expect(formatKb(2 * 1024 * 1024)).toBe("2.00 GB");
  });

  it("formats package origins, partitions, effective and configured states", () => {
    expect(formatOrigin("oem")).toBe("OEM");
    expect(formatOrigin("third_party")).toBe("Third-Party");
    expect(formatPartition("system_ext")).toBe("/system_ext");
    expect(formatPartition("data")).toBe("/data");
    expect(formatEffectiveState("enabled")).toBe("Enabled");
    expect(formatEffectiveState("disabled")).toBe("Disabled");
    expect(formatConfiguredState("disabled_user", 3)).toBe("DISABLED_USER (3)");
    expect(formatConfiguredState("unknown", null)).toBe("UNKNOWN");
  });

  it("formats capability support tiers, risks, scopes, and verification metadata", () => {
    expect(formatSupportTier("primary")).toBe("Primary range (API 21+)");
    expect(formatSupportTier("exploratory_legacy")).toBe(
      "Exploratory legacy range (API 19-20)",
    );
    expect(formatCommandSupport("supported")).toBe("Supported");
    expect(formatCommandSupport("unsupported")).toBe("Not listed");
    expect(formatRisk("do_not_disable")).toBe("Do Not Disable");
    expect(riskTone("do_not_disable")).toBe("error");
    expect(riskTone("recommended_to_disable")).toBe("ok");
    expect(riskTone("caution")).toBe("warn");
    expect(riskTone("unknown_unverified")).toBe("neutral");
    expect(formatMatchScope("exact_device")).toBe("Exact device rule");
    expect(formatVerificationStatus("verified")).toBe("Verified");
    expect(formatVerificationStatus(null)).toBe("Unverified");
    expect(formatConfidence("high")).toBe("High");
  });
});

describe("filterInstalledPackages", () => {
  it("matches case-insensitively on both label and raw package identifier", () => {
    const byLabel = filterInstalledPackages(
      SAMPLE_PACKAGES,
      { ...BASE_FILTER, search: "  bixby " },
      SAMPLE_RECOMMENDATIONS,
    );
    expect(byLabel.map((p) => p.packageName)).toEqual([
      "com.samsung.android.bixby.agent",
    ]);

    const byPackageId = filterInstalledPackages(
      SAMPLE_PACKAGES,
      { ...BASE_FILTER, search: "MOZILLA" },
      SAMPLE_RECOMMENDATIONS,
    );
    expect(byPackageId.map((p) => p.packageName)).toEqual([
      "org.mozilla.firefox",
    ]);
  });

  it("treats packages missing from recommendations as unknown_unverified", () => {
    const unknownOnly = filterInstalledPackages(
      SAMPLE_PACKAGES,
      { ...BASE_FILTER, risk: "unknown_unverified" },
      SAMPLE_RECOMMENDATIONS,
    );
    expect(unknownOnly.map((p) => p.packageName)).toEqual([
      "com.google.android.youtube",
      "org.mozilla.firefox",
    ]);

    const cautionOnly = filterInstalledPackages(
      SAMPLE_PACKAGES,
      { ...BASE_FILTER, risk: "caution" },
      SAMPLE_RECOMMENDATIONS,
    );
    expect(cautionOnly.map((p) => p.packageName)).toEqual([
      "com.samsung.android.bixby.agent",
    ]);
  });

  it("filters by origin, effective state, partition, and updatedSystemOnly", () => {
    const updatedOnly = filterInstalledPackages(
      SAMPLE_PACKAGES,
      { ...BASE_FILTER, updatedSystemOnly: true },
      SAMPLE_RECOMMENDATIONS,
    );
    expect(updatedOnly.map((p) => p.packageName)).toEqual([
      "com.google.android.youtube",
    ]);

    const disabledOemSystem = filterInstalledPackages(
      SAMPLE_PACKAGES,
      {
        ...BASE_FILTER,
        origin: "oem",
        state: "disabled",
        partition: "system",
      },
      SAMPLE_RECOMMENDATIONS,
    );
    expect(disabledOemSystem.map((p) => p.packageName)).toEqual([
      "com.samsung.android.bixby.agent",
    ]);
  });
});
