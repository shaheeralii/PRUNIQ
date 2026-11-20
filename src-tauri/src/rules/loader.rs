use std::collections::HashSet;

use crate::adb::validation::validate_package_name;
use crate::core::errors::AdtError;
use crate::rules::models::{PackageRule, RuleDatabaseFile, RuleEvidence, VerificationStatus};

pub const SUPPORTED_SCHEMA_VERSION: u32 = 1;

const EMBEDDED_RULES_JSON: &str = include_str!("../../../data/package-rules/rules.json");

/// Loads and validates the version-controlled offline package-rule JSON bundled with ADT.
pub fn load_embedded_rule_database() -> Result<RuleDatabaseFile, AdtError> {
    parse_and_validate_rule_database(EMBEDDED_RULES_JSON)
}

/// Parses and validates a package-rule database JSON document.
/// Strictly enforces schema version, package-name validity, API range sanity, and
/// mandatory evidence requirements for `verified` rules.
pub fn parse_and_validate_rule_database(raw_json: &str) -> Result<RuleDatabaseFile, AdtError> {
    let db: RuleDatabaseFile =
        serde_json::from_str(raw_json).map_err(|err| AdtError::RuleValidation {
            message: format!("invalid rule database JSON: {err}"),
        })?;

    validate_rule_database(&db)?;
    Ok(db)
}

pub fn validate_rule_database(db: &RuleDatabaseFile) -> Result<(), AdtError> {
    if db.schema_version != SUPPORTED_SCHEMA_VERSION {
        return Err(AdtError::RuleValidation {
            message: format!(
                "unsupported rule schema version {} (expected {})",
                db.schema_version, SUPPORTED_SCHEMA_VERSION
            ),
        });
    }

    if db.database_version.trim().is_empty() {
        return Err(AdtError::RuleValidation {
            message: "databaseVersion must not be empty".to_string(),
        });
    }

    let mut seen_ids = HashSet::with_capacity(db.rules.len());
    for rule in &db.rules {
        validate_rule(rule, &mut seen_ids)?;
    }

    Ok(())
}

fn validate_rule(rule: &PackageRule, seen_ids: &mut HashSet<String>) -> Result<(), AdtError> {
    let id = rule.id.trim();
    if id.is_empty() {
        return Err(AdtError::RuleValidation {
            message: "rule id must not be empty".to_string(),
        });
    }
    if !seen_ids.insert(id.to_string()) {
        return Err(AdtError::RuleValidation {
            message: format!("duplicate rule id '{id}'"),
        });
    }

    validate_package_name(&rule.package_name).map_err(|err| AdtError::RuleValidation {
        message: format!("rule '{id}' has invalid packageName: {err}"),
    })?;

    if rule.app_label.trim().is_empty() {
        return Err(AdtError::RuleValidation {
            message: format!("rule '{id}' must provide a non-empty appLabel"),
        });
    }
    if rule.purpose.trim().is_empty() {
        return Err(AdtError::RuleValidation {
            message: format!("rule '{id}' must provide a non-empty purpose"),
        });
    }
    if rule.known_consequences.trim().is_empty() {
        return Err(AdtError::RuleValidation {
            message: format!("rule '{id}' must provide non-empty knownConsequences"),
        });
    }

    if let (Some(min_api), Some(max_api)) = (rule.min_api_level, rule.max_api_level) {
        if min_api > max_api {
            return Err(AdtError::RuleValidation {
                message: format!(
                    "rule '{id}' has invalid API range: minApiLevel ({min_api}) > maxApiLevel ({max_api})"
                ),
            });
        }
    }

    for (idx, evidence) in rule.evidence.iter().enumerate() {
        validate_evidence(id, idx, evidence)?;
    }

    // Section 6F integrity gate: verified rules MUST have real supporting evidence, date, and reviewer.
    if rule.verification_status == VerificationStatus::Verified {
        if rule.evidence.is_empty() {
            return Err(AdtError::RuleValidation {
                message: format!(
                    "rule '{id}' is marked verified but has no supporting evidence entries"
                ),
            });
        }
        if rule
            .last_verified_date
            .as_deref()
            .map_or(true, |d| d.trim().is_empty())
        {
            return Err(AdtError::RuleValidation {
                message: format!("rule '{id}' is marked verified without lastVerifiedDate"),
            });
        }
        if rule
            .reviewer
            .as_deref()
            .map_or(true, |r| r.trim().is_empty())
        {
            return Err(AdtError::RuleValidation {
                message: format!("rule '{id}' is marked verified without reviewer"),
            });
        }
    }

    Ok(())
}

fn validate_evidence(rule_id: &str, idx: usize, ev: &RuleEvidence) -> Result<(), AdtError> {
    if ev.source_references.is_empty()
        || ev.source_references.iter().any(|s| s.trim().is_empty())
    {
        return Err(AdtError::RuleValidation {
            message: format!(
                "rule '{rule_id}' evidence[{idx}] must include at least one non-empty sourceReference"
            ),
        });
    }
    if ev.device_model_or_codename.trim().is_empty() {
        return Err(AdtError::RuleValidation {
            message: format!(
                "rule '{rule_id}' evidence[{idx}] must include deviceModelOrCodename"
            ),
        });
    }
    if ev.android_version.trim().is_empty() {
        return Err(AdtError::RuleValidation {
            message: format!("rule '{rule_id}' evidence[{idx}] must include androidVersion"),
        });
    }
    if ev.test_date.trim().is_empty() {
        return Err(AdtError::RuleValidation {
            message: format!("rule '{rule_id}' evidence[{idx}] must include testDate"),
        });
    }
    if ev.observed_behavior.trim().is_empty() {
        return Err(AdtError::RuleValidation {
            message: format!("rule '{rule_id}' evidence[{idx}] must include observedBehavior"),
        });
    }
    if ev.limitations.trim().is_empty() {
        return Err(AdtError::RuleValidation {
            message: format!("rule '{rule_id}' evidence[{idx}] must include limitations"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_embedded_empty_database_validly() {
        let db = load_embedded_rule_database().expect("embedded rules.json must validate");
        assert_eq!(db.schema_version, 1);
        assert!(db.rules.is_empty());
    }

    #[test]
    fn rejects_verified_rule_without_evidence() {
        let json = r#"{
          "schemaVersion": 1,
          "databaseVersion": "test-1",
          "rules": [
            {
              "id": "rule-no-evidence",
              "packageName": "com.example.bloat",
              "appLabel": "Example Bloat",
              "purpose": "Promotional companion",
              "risk": "recommended_to_disable",
              "knownConsequences": "Promotional widgets stop updating",
              "manufacturers": [],
              "deviceModels": [],
              "deviceCodenames": [],
              "compatibilityNotes": "",
              "confidence": "high",
              "evidence": [],
              "verificationStatus": "verified",
              "lastVerifiedDate": "2026-11-01",
              "reviewer": "maintainer"
            }
          ]
        }"#;

        let err = parse_and_validate_rule_database(json).expect_err("must reject missing evidence");
        assert!(matches!(err, AdtError::RuleValidation { .. }));
    }

    #[test]
    fn rejects_verified_rule_with_missing_reviewer_or_blank_evidence_field() {
        let json = r#"{
          "schemaVersion": 1,
          "databaseVersion": "test-2",
          "rules": [
            {
              "id": "rule-incomplete-evidence",
              "packageName": "com.example.bloat",
              "appLabel": "Example Bloat",
              "purpose": "Promotional companion",
              "risk": "caution",
              "knownConsequences": "Minor feature loss",
              "manufacturers": [],
              "deviceModels": [],
              "deviceCodenames": [],
              "compatibilityNotes": "",
              "confidence": "medium",
              "evidence": [
                {
                  "sourceReferences": ["internal-lab"],
                  "deviceModelOrCodename": "SM-A528B",
                  "androidVersion": "Android 14",
                  "testDate": "2026-11-01",
                  "observedBehavior": "Disabled cleanly",
                  "limitations": ""
                }
              ],
              "verificationStatus": "verified",
              "lastVerifiedDate": "2026-11-01",
              "reviewer": "maintainer"
            }
          ]
        }"#;

        let err =
            parse_and_validate_rule_database(json).expect_err("must reject incomplete evidence");
        assert!(matches!(err, AdtError::RuleValidation { .. }));
    }

    #[test]
    fn rejects_duplicate_rule_ids_and_invalid_api_range() {
        let invalid_range = r#"{
          "schemaVersion": 1,
          "databaseVersion": "test-3",
          "rules": [
            {
              "id": "rule-bad-api",
              "packageName": "com.example.app",
              "appLabel": "Example",
              "purpose": "Test",
              "risk": "caution",
              "knownConsequences": "Test",
              "minApiLevel": 34,
              "maxApiLevel": 30,
              "compatibilityNotes": "",
              "confidence": "low",
              "evidence": [],
              "verificationStatus": "draft"
            }
          ]
        }"#;
        assert!(matches!(
            parse_and_validate_rule_database(invalid_range),
            Err(AdtError::RuleValidation { .. })
        ));

        let dup_id = r#"{
          "schemaVersion": 1,
          "databaseVersion": "test-4",
          "rules": [
            {
              "id": "dup-1",
              "packageName": "com.example.one",
              "appLabel": "One",
              "purpose": "Test",
              "risk": "caution",
              "knownConsequences": "Test",
              "compatibilityNotes": "",
              "confidence": "low",
              "evidence": [],
              "verificationStatus": "draft"
            },
            {
              "id": "dup-1",
              "packageName": "com.example.two",
              "appLabel": "Two",
              "purpose": "Test",
              "risk": "caution",
              "knownConsequences": "Test",
              "compatibilityNotes": "",
              "confidence": "low",
              "evidence": [],
              "verificationStatus": "draft"
            }
          ]
        }"#;
        assert!(matches!(
            parse_and_validate_rule_database(dup_id),
            Err(AdtError::RuleValidation { .. })
        ));
    }

    #[test]
    fn rejects_invalid_package_name_in_rule() {
        let bad_pkg = r#"{
          "schemaVersion": 1,
          "databaseVersion": "test-5",
          "rules": [
            {
              "id": "bad-pkg",
              "packageName": "not a valid pkg",
              "appLabel": "Bad",
              "purpose": "Test",
              "risk": "caution",
              "knownConsequences": "Test",
              "compatibilityNotes": "",
              "confidence": "low",
              "evidence": [],
              "verificationStatus": "draft"
            }
          ]
        }"#;
        assert!(matches!(
            parse_and_validate_rule_database(bad_pkg),
            Err(AdtError::RuleValidation { .. })
        ));
    }
}
