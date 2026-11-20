use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskClassification {
    RecommendedToDisable,
    Caution,
    DoNotDisable,
    UnknownUnverified,
}

impl RiskClassification {
    /// Higher rank means more restrictive/conservative.
    /// Used to resolve equally specific conflicting rules deterministically.
    pub const fn restrictiveness_rank(self) -> u8 {
        match self {
            Self::DoNotDisable => 4,
            Self::Caution => 3,
            Self::UnknownUnverified => 2,
            Self::RecommendedToDisable => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Draft,
    CommunityReported,
    Verified,
    Deprecated,
}

impl VerificationStatus {
    pub const fn trust_rank(self) -> u8 {
        match self {
            Self::Verified => 3,
            Self::CommunityReported => 2,
            Self::Draft => 1,
            Self::Deprecated => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
    Unverified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchScope {
    ExactDevice,
    Manufacturer,
    AndroidVersion,
    General,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleEvidence {
    pub source_references: Vec<String>,
    pub device_model_or_codename: String,
    pub android_version: String,
    pub test_date: String,
    pub observed_behavior: String,
    pub limitations: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageRule {
    pub id: String,
    pub package_name: String,
    pub app_label: String,
    pub purpose: String,
    pub risk: RiskClassification,
    pub known_consequences: String,
    #[serde(default)]
    pub manufacturers: Vec<String>,
    #[serde(default)]
    pub device_models: Vec<String>,
    #[serde(default)]
    pub device_codenames: Vec<String>,
    #[serde(default)]
    pub min_api_level: Option<u32>,
    #[serde(default)]
    pub max_api_level: Option<u32>,
    #[serde(default)]
    pub compatibility_notes: String,
    pub confidence: ConfidenceLevel,
    #[serde(default)]
    pub evidence: Vec<RuleEvidence>,
    pub verification_status: VerificationStatus,
    #[serde(default)]
    pub last_verified_date: Option<String>,
    #[serde(default)]
    pub reviewer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleDatabaseFile {
    pub schema_version: u32,
    pub database_version: String,
    #[serde(default)]
    pub description: Option<String>,
    pub rules: Vec<PackageRule>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceMatchContext {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub codename: Option<String>,
    pub api_level: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleDatabaseSummary {
    pub schema_version: u32,
    pub database_version: String,
    pub description: Option<String>,
    pub total_rules: usize,
    pub active_rules: usize,
    pub verified_rules: usize,
    pub community_reported_rules: usize,
    pub draft_rules: usize,
    pub deprecated_rules: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageRecommendation {
    pub package_name: String,
    pub risk: RiskClassification,
    pub match_scope: MatchScope,
    pub matched_rule_id: Option<String>,
    pub app_label: Option<String>,
    pub purpose: Option<String>,
    pub known_consequences: Option<String>,
    pub compatibility_notes: Option<String>,
    pub confidence: ConfidenceLevel,
    pub verification_status: Option<VerificationStatus>,
    pub last_verified_date: Option<String>,
    pub reviewer: Option<String>,
    pub evidence: Vec<RuleEvidence>,
    pub conflict_detected: bool,
    pub conflict_note: Option<String>,
    pub explanation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationBatchResult {
    pub database: RuleDatabaseSummary,
    pub recommendations: BTreeMap<String, PackageRecommendation>,
}
