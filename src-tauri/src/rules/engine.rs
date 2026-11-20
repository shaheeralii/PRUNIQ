use std::collections::{BTreeMap, HashMap};

use crate::adb::validation::validate_package_name;
use crate::core::errors::AdtError;
use crate::rules::loader::{load_embedded_rule_database, validate_rule_database};
use crate::rules::models::{
    ConfidenceLevel, DeviceMatchContext, MatchScope, PackageRecommendation, PackageRule,
    RecommendationBatchResult, RiskClassification, RuleDatabaseFile, RuleDatabaseSummary,
    VerificationStatus,
};

/// In-memory, deterministic, conservative recommendation engine.
pub struct RuleEngine {
    rules_by_package: HashMap<String, Vec<PackageRule>>,
    summary: RuleDatabaseSummary,
}

impl RuleEngine {
    pub fn from_embedded() -> Result<Self, AdtError> {
        let db = load_embedded_rule_database()?;
        Self::from_database(db)
    }

    pub fn from_database(db: RuleDatabaseFile) -> Result<Self, AdtError> {
        validate_rule_database(&db)?;

        let mut verified_rules = 0usize;
        let mut community_reported_rules = 0usize;
        let mut draft_rules = 0usize;
        let mut deprecated_rules = 0usize;
        let mut active_rules = 0usize;

        let mut rules_by_package: HashMap<String, Vec<PackageRule>> = HashMap::new();

        for rule in db.rules {
            match rule.verification_status {
                VerificationStatus::Verified => verified_rules += 1,
                VerificationStatus::CommunityReported => community_reported_rules += 1,
                VerificationStatus::Draft => draft_rules += 1,
                VerificationStatus::Deprecated => deprecated_rules += 1,
            }

            if rule.verification_status != VerificationStatus::Deprecated {
                active_rules += 1;
                rules_by_package
                    .entry(rule.package_name.clone())
                    .or_default()
                    .push(rule);
            }
        }

        let total_rules = active_rules + deprecated_rules;
        let summary = RuleDatabaseSummary {
            schema_version: db.schema_version,
            database_version: db.database_version,
            description: db.description,
            total_rules,
            active_rules,
            verified_rules,
            community_reported_rules,
            draft_rules,
            deprecated_rules,
        };

        Ok(Self {
            rules_by_package,
            summary,
        })
    }

    pub fn summary(&self) -> RuleDatabaseSummary {
        self.summary.clone()
    }

    /// Evaluates a single package deterministically against the target device context.
    /// Unknown packages always resolve conservatively to `UnknownUnverified`.
    pub fn evaluate_package(
        &self,
        package_name: &str,
        context: &DeviceMatchContext,
    ) -> PackageRecommendation {
        let Some(candidates) = self.rules_by_package.get(package_name) else {
            return unknown_recommendation(package_name);
        };

        let matching: Vec<&PackageRule> = candidates
            .iter()
            .filter(|rule| rule_matches_context(rule, context))
            .collect();

        if matching.is_empty() {
            return unknown_recommendation(package_name);
        }

        let max_specificity = matching
            .iter()
            .map(|r| specificity_key(r))
            .max()
            .expect("matching is non-empty");

        let mut top_matches: Vec<&PackageRule> = matching
            .into_iter()
            .filter(|r| specificity_key(r) == max_specificity)
            .collect();

        // Deterministic ordering:
        // 1. Most restrictive risk classification wins on equally specific conflicts.
        // 2. Higher verification trust wins on equal risk.
        // 3. Lexicographically smallest rule ID guarantees cross-run determinism.
        top_matches.sort_by(|a, b| {
            b.risk
                .restrictiveness_rank()
                .cmp(&a.risk.restrictiveness_rank())
                .then_with(|| {
                    b.verification_status
                        .trust_rank()
                        .cmp(&a.verification_status.trust_rank())
                })
                .then_with(|| a.id.cmp(&b.id))
        });

        let winner = top_matches[0];
        let scope = match_scope_for_specificity(max_specificity);

        let distinct_risks: Vec<RiskClassification> = top_matches
            .iter()
            .map(|r| r.risk)
            .fold(Vec::new(), |mut acc, risk| {
                if !acc.contains(&risk) {
                    acc.push(risk);
                }
                acc
            });

        let conflict_detected = distinct_risks.len() > 1;
        let conflict_note = if conflict_detected {
            let conflicting_ids: Vec<&str> = top_matches.iter().map(|r| r.id.as_str()).collect();
            Some(format!(
                "Multiple equally specific rules matched ({}); selected the most restrictive classification from '{}'.",
                conflicting_ids.join(", "),
                winner.id
            ))
        } else {
            None
        };

        let explanation = build_explanation(winner, scope, conflict_detected);

        PackageRecommendation {
            package_name: package_name.to_string(),
            risk: winner.risk,
            match_scope: scope,
            matched_rule_id: Some(winner.id.clone()),
            app_label: Some(winner.app_label.clone()),
            purpose: Some(winner.purpose.clone()),
            known_consequences: Some(winner.known_consequences.clone()),
            compatibility_notes: if winner.compatibility_notes.trim().is_empty() {
                None
            } else {
                Some(winner.compatibility_notes.clone())
            },
            confidence: winner.confidence,
            verification_status: Some(winner.verification_status),
            last_verified_date: winner.last_verified_date.clone(),
            reviewer: winner.reviewer.clone(),
            evidence: winner.evidence.clone(),
            conflict_detected,
            conflict_note,
            explanation,
        }
    }

    pub fn evaluate_batch(
        &self,
        package_names: &[String],
        context: &DeviceMatchContext,
    ) -> Result<RecommendationBatchResult, AdtError> {
        let mut recommendations = BTreeMap::new();
        for pkg in package_names {
            validate_package_name(pkg)?;
            if !recommendations.contains_key(pkg) {
                let rec = self.evaluate_package(pkg, context);
                recommendations.insert(pkg.clone(), rec);
            }
        }

        Ok(RecommendationBatchResult {
            database: self.summary(),
            recommendations,
        })
    }
}

fn unknown_recommendation(package_name: &str) -> PackageRecommendation {
    PackageRecommendation {
        package_name: package_name.to_string(),
        risk: RiskClassification::UnknownUnverified,
        match_scope: MatchScope::Unknown,
        matched_rule_id: None,
        app_label: None,
        purpose: None,
        known_consequences: None,
        compatibility_notes: None,
        confidence: ConfidenceLevel::Unverified,
        verification_status: None,
        last_verified_date: None,
        reviewer: None,
        evidence: Vec::new(),
        conflict_detected: false,
        conflict_note: None,
        explanation: "No active rule matches this package for the current device. Unknown packages are never considered safe to disable by default.".to_string(),
    }
}

/// Precedence specificity tuple ordered strictly per Section 6F:
/// 1. Exact device (model or codename)
/// 2. Manufacturer
/// 3. Android API version range
/// 4. General Android rule (all false)
fn specificity_key(rule: &PackageRule) -> (bool, bool, bool) {
    let has_device = !rule.device_models.is_empty() || !rule.device_codenames.is_empty();
    let has_manufacturer = !rule.manufacturers.is_empty();
    let has_api = rule.min_api_level.is_some() || rule.max_api_level.is_some();
    (has_device, has_manufacturer, has_api)
}

const fn match_scope_for_specificity(key: (bool, bool, bool)) -> MatchScope {
    match key {
        (true, _, _) => MatchScope::ExactDevice,
        (false, true, _) => MatchScope::Manufacturer,
        (false, false, true) => MatchScope::AndroidVersion,
        (false, false, false) => MatchScope::General,
    }
}

fn rule_matches_context(rule: &PackageRule, ctx: &DeviceMatchContext) -> bool {
    if !rule.manufacturers.is_empty() {
        let Some(ref dev_mfg) = ctx.manufacturer else {
            return false;
        };
        if !contains_ignore_case(&rule.manufacturers, dev_mfg) {
            return false;
        }
    }

    if !rule.device_models.is_empty() || !rule.device_codenames.is_empty() {
        let model_matches = !rule.device_models.is_empty()
            && ctx
                .model
                .as_deref()
                .is_some_and(|m| contains_ignore_case(&rule.device_models, m));
        let codename_matches = !rule.device_codenames.is_empty()
            && ctx
                .codename
                .as_deref()
                .is_some_and(|c| contains_ignore_case(&rule.device_codenames, c));

        if !model_matches && !codename_matches {
            return false;
        }
    }

    if rule.min_api_level.is_some() || rule.max_api_level.is_some() {
        let Some(api) = ctx.api_level else {
            return false;
        };
        if let Some(min_api) = rule.min_api_level {
            if api < min_api {
                return false;
            }
        }
        if let Some(max_api) = rule.max_api_level {
            if api > max_api {
                return false;
            }
        }
    }

    true
}

fn contains_ignore_case(haystack: &[String], needle: &str) -> bool {
    let trimmed_needle = needle.trim();
    if trimmed_needle.is_empty() {
        return false;
    }
    haystack
        .iter()
        .any(|candidate| candidate.trim().eq_ignore_ascii_case(trimmed_needle))
}

fn build_explanation(rule: &PackageRule, scope: MatchScope, conflict_detected: bool) -> String {
    let scope_label = match scope {
        MatchScope::ExactDevice => "exact device rule",
        MatchScope::Manufacturer => "manufacturer rule",
        MatchScope::AndroidVersion => "Android API-range rule",
        MatchScope::General => "general Android rule",
        MatchScope::Unknown => "no matching rule",
    };

    let status_note = match rule.verification_status {
        VerificationStatus::Verified => "Evidence-verified",
        VerificationStatus::CommunityReported => {
            "Community-reported (not yet independently verified)"
        }
        VerificationStatus::Draft => "Draft entry (unverified)",
        VerificationStatus::Deprecated => "Deprecated",
    };

    if conflict_detected {
        format!(
            "Matched {scope_label} '{}' ({status_note}). Conservative conflict resolution selected the most restrictive classification.",
            rule.id
        )
    } else {
        format!("Matched {scope_label} '{}' ({status_note}).", rule.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::models::RuleEvidence;

    fn sample_evidence() -> RuleEvidence {
        RuleEvidence {
            source_references: vec!["local-test-fixture".to_string()],
            device_model_or_codename: "SM-A528B / a52sxq".to_string(),
            android_version: "Android 14 (API 34)".to_string(),
            test_date: "2026-11-10".to_string(),
            observed_behavior: "Test fixture behavior".to_string(),
            limitations: "Test fixture scope only".to_string(),
        }
    }

    fn make_rule(id: &str, pkg: &str, risk: RiskClassification) -> PackageRule {
        PackageRule {
            id: id.to_string(),
            package_name: pkg.to_string(),
            app_label: "Test App".to_string(),
            purpose: "Fixture rule".to_string(),
            risk,
            known_consequences: "Fixture consequence".to_string(),
            manufacturers: Vec::new(),
            device_models: Vec::new(),
            device_codenames: Vec::new(),
            min_api_level: None,
            max_api_level: None,
            compatibility_notes: String::new(),
            confidence: ConfidenceLevel::High,
            evidence: vec![sample_evidence()],
            verification_status: VerificationStatus::Verified,
            last_verified_date: Some("2026-11-10".to_string()),
            reviewer: Some("fixture-reviewer".to_string()),
        }
    }

    fn engine_with(rules: Vec<PackageRule>) -> RuleEngine {
        RuleEngine::from_database(RuleDatabaseFile {
            schema_version: 1,
            database_version: "unit-test".to_string(),
            description: None,
            rules,
        })
        .expect("unit test rule db should validate")
    }

    #[test]
    fn unknown_package_defaults_to_unknown_unverified_conservatively() {
        let engine = RuleEngine::from_embedded().expect("embedded engine");
        let rec = engine.evaluate_package("com.vendor.unknownapp", &DeviceMatchContext::default());
        assert_eq!(rec.risk, RiskClassification::UnknownUnverified);
        assert_eq!(rec.match_scope, MatchScope::Unknown);
        assert_eq!(rec.confidence, ConfidenceLevel::Unverified);
        assert!(rec.matched_rule_id.is_none());
    }

    #[test]
    fn enforces_exact_device_over_manufacturer_over_api_over_general_precedence() {
        let pkg = "com.example.layered";

        let general = make_rule("r-general", pkg, RiskClassification::RecommendedToDisable);

        let mut api_rule = make_rule("r-api", pkg, RiskClassification::Caution);
        api_rule.min_api_level = Some(30);
        api_rule.max_api_level = Some(35);

        let mut mfg_rule = make_rule("r-mfg", pkg, RiskClassification::DoNotDisable);
        mfg_rule.manufacturers = vec!["samsung".to_string()];

        let mut device_rule = make_rule("r-device", pkg, RiskClassification::Caution);
        device_rule.device_codenames = vec!["a52sxq".to_string()];

        let engine = engine_with(vec![general, api_rule, mfg_rule, device_rule]);

        // 1. Exact device matches -> r-device wins
        let exact_ctx = DeviceMatchContext {
            manufacturer: Some("Samsung".to_string()),
            model: Some("SM-A528B".to_string()),
            codename: Some("a52sxq".to_string()),
            api_level: Some(34),
        };
        let exact = engine.evaluate_package(pkg, &exact_ctx);
        assert_eq!(exact.matched_rule_id.as_deref(), Some("r-device"));
        assert_eq!(exact.match_scope, MatchScope::ExactDevice);

        // 2. Different codename, same manufacturer -> r-mfg wins over r-api and r-general
        let mfg_ctx = DeviceMatchContext {
            manufacturer: Some("SAMSUNG".to_string()),
            model: Some("SM-S911B".to_string()),
            codename: Some("dm1q".to_string()),
            api_level: Some(34),
        };
        let mfg = engine.evaluate_package(pkg, &mfg_ctx);
        assert_eq!(mfg.matched_rule_id.as_deref(), Some("r-mfg"));
        assert_eq!(mfg.match_scope, MatchScope::Manufacturer);
        assert_eq!(mfg.risk, RiskClassification::DoNotDisable);

        // 3. Other manufacturer within API 30-35 -> r-api wins over r-general
        let api_ctx = DeviceMatchContext {
            manufacturer: Some("Google".to_string()),
            model: Some("Pixel 7".to_string()),
            codename: Some("panther".to_string()),
            api_level: Some(33),
        };
        let api = engine.evaluate_package(pkg, &api_ctx);
        assert_eq!(api.matched_rule_id.as_deref(), Some("r-api"));
        assert_eq!(api.match_scope, MatchScope::AndroidVersion);

        // 4. API outside 30-35 and other manufacturer -> r-general wins
        let old_ctx = DeviceMatchContext {
            manufacturer: Some("Motorola".to_string()),
            model: Some("Moto G".to_string()),
            codename: Some("falcon".to_string()),
            api_level: Some(23),
        };
        let gen = engine.evaluate_package(pkg, &old_ctx);
        assert_eq!(gen.matched_rule_id.as_deref(), Some("r-general"));
        assert_eq!(gen.match_scope, MatchScope::General);
    }

    #[test]
    fn most_restrictive_risk_wins_on_equally_specific_conflict() {
        let pkg = "com.example.conflict";

        let mut r_safe = make_rule("r-a-safe", pkg, RiskClassification::RecommendedToDisable);
        r_safe.manufacturers = vec!["xiaomi".to_string()];

        let mut r_caution = make_rule("r-b-caution", pkg, RiskClassification::Caution);
        r_caution.manufacturers = vec!["xiaomi".to_string()];

        let mut r_block = make_rule("r-c-block", pkg, RiskClassification::DoNotDisable);
        r_block.manufacturers = vec!["xiaomi".to_string()];

        let engine = engine_with(vec![r_safe, r_caution, r_block]);
        let ctx = DeviceMatchContext {
            manufacturer: Some("Xiaomi".to_string()),
            ..DeviceMatchContext::default()
        };

        let rec = engine.evaluate_package(pkg, &ctx);
        assert_eq!(rec.risk, RiskClassification::DoNotDisable);
        assert_eq!(rec.matched_rule_id.as_deref(), Some("r-c-block"));
        assert!(rec.conflict_detected);
        assert!(rec.conflict_note.is_some());
    }

    #[test]
    fn ignores_deprecated_rules_during_evaluation() {
        let mut deprecated = make_rule(
            "r-old",
            "com.example.deprecated",
            RiskClassification::RecommendedToDisable,
        );
        deprecated.verification_status = VerificationStatus::Deprecated;

        let engine = engine_with(vec![deprecated]);
        assert_eq!(engine.summary().total_rules, 1);
        assert_eq!(engine.summary().active_rules, 0);
        assert_eq!(engine.summary().deprecated_rules, 1);

        let rec = engine.evaluate_package("com.example.deprecated", &DeviceMatchContext::default());
        assert_eq!(rec.risk, RiskClassification::UnknownUnverified);
        assert!(rec.matched_rule_id.is_none());
    }

    #[test]
    fn does_not_match_constrained_rules_when_device_context_fields_are_missing() {
        let mut mfg_only = make_rule("r-mfg", "com.example.scoped", RiskClassification::RecommendedToDisable);
        mfg_only.manufacturers = vec!["samsung".to_string()];

        let mut api_only = make_rule("r-api", "com.example.scoped", RiskClassification::RecommendedToDisable);
        api_only.min_api_level = Some(21);

        let engine = engine_with(vec![mfg_only, api_only]);
        let rec = engine.evaluate_package("com.example.scoped", &DeviceMatchContext::default());
        assert_eq!(rec.risk, RiskClassification::UnknownUnverified);
    }

    #[test]
    fn matches_manufacturer_model_and_codename_case_insensitively() {
        let mut rule = make_rule("r-case", "com.example.case", RiskClassification::Caution);
        rule.manufacturers = vec!["Samsung".to_string()];
        rule.device_models = vec!["sm-a528b".to_string()];

        let engine = engine_with(vec![rule]);
        let ctx = DeviceMatchContext {
            manufacturer: Some("  samsung  ".to_string()),
            model: Some("SM-A528B".to_string()),
            codename: None,
            api_level: None,
        };

        let rec = engine.evaluate_package("com.example.case", &ctx);
        assert_eq!(rec.risk, RiskClassification::Caution);
        assert_eq!(rec.match_scope, MatchScope::ExactDevice);
    }
}
