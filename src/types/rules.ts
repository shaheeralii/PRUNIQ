export type RiskClassification =
  | "recommended_to_disable"
  | "caution"
  | "do_not_disable"
  | "unknown_unverified";

export type VerificationStatus =
  | "draft"
  | "community_reported"
  | "verified"
  | "deprecated";

export type ConfidenceLevel = "high" | "medium" | "low" | "unverified";

export type MatchScope =
  | "exact_device"
  | "manufacturer"
  | "android_version"
  | "general"
  | "unknown";

export interface RuleEvidence {
  sourceReferences: string[];
  deviceModelOrCodename: string;
  androidVersion: string;
  testDate: string;
  observedBehavior: string;
  limitations: string;
}

export interface DeviceMatchContext {
  manufacturer: string | null;
  model: string | null;
  codename: string | null;
  apiLevel: number | null;
}

export interface RuleDatabaseSummary {
  schemaVersion: number;
  databaseVersion: string;
  description: string | null;
  totalRules: number;
  activeRules: number;
  verifiedRules: number;
  communityReportedRules: number;
  draftRules: number;
  deprecatedRules: number;
}

export interface PackageRecommendation {
  packageName: string;
  risk: RiskClassification;
  matchScope: MatchScope;
  matchedRuleId: string | null;
  appLabel: string | null;
  purpose: string | null;
  knownConsequences: string | null;
  compatibilityNotes: string | null;
  confidence: ConfidenceLevel;
  verificationStatus: VerificationStatus | null;
  lastVerifiedDate: string | null;
  reviewer: string | null;
  evidence: RuleEvidence[];
  conflictDetected: boolean;
  conflictNote: string | null;
  explanation: string;
}

export interface RecommendationBatchResult {
  database: RuleDatabaseSummary;
  recommendations: Record<string, PackageRecommendation>;
}
