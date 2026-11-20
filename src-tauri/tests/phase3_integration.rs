use std::fs;
use std::path::Path;

use adt_lib::core::errors::AdtError;
use adt_lib::rules::engine::RuleEngine;
use adt_lib::rules::loader::parse_and_validate_rule_database;
use adt_lib::rules::models::{
    DeviceMatchContext, MatchScope, RiskClassification, VerificationStatus,
};

fn read_fixture(filename: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures")
        .join(filename);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture '{}': {e}", path.display()))
}

#[test]
fn fixture_database_validates_and_resolves_all_precedence_tiers() {
    let raw = read_fixture("package_rules_sample.json");
    let db = parse_and_validate_rule_database(&raw).expect("fixture db must validate");
    let engine = RuleEngine::from_database(db).expect("engine should load fixture db");

    assert_eq!(engine.summary().active_rules, 7);
    assert_eq!(engine.summary().verified_rules, 3);
    assert_eq!(engine.summary().community_reported_rules, 3);
    assert_eq!(engine.summary().draft_rules, 1);

    // Exact device match
    let exact = engine.evaluate_package(
        "com.example.promo",
        &DeviceMatchContext {
            manufacturer: Some("samsung".to_string()),
            model: Some("SM-A528B".to_string()),
            codename: Some("a52sxq".to_string()),
            api_level: Some(34),
        },
    );
    assert_eq!(exact.match_scope, MatchScope::ExactDevice);
    assert_eq!(exact.matched_rule_id.as_deref(), Some("fixture-promo-exact-a52sxq"));
    assert_eq!(exact.risk, RiskClassification::RecommendedToDisable);
    assert_eq!(exact.verification_status, Some(VerificationStatus::Verified));
    assert_eq!(exact.evidence.len(), 1);

    // Manufacturer match (different Samsung device)
    let mfg = engine.evaluate_package(
        "com.example.promo",
        &DeviceMatchContext {
            manufacturer: Some("Samsung".to_string()),
            model: Some("SM-G973F".to_string()),
            codename: Some("beyond1".to_string()),
            api_level: Some(31),
        },
    );
    assert_eq!(mfg.match_scope, MatchScope::Manufacturer);
    assert_eq!(mfg.matched_rule_id.as_deref(), Some("fixture-promo-samsung"));
    assert_eq!(mfg.risk, RiskClassification::DoNotDisable);

    // API match (non-Samsung on API 33)
    let api = engine.evaluate_package(
        "com.example.promo",
        &DeviceMatchContext {
            manufacturer: Some("Google".to_string()),
            model: Some("Pixel 7".to_string()),
            codename: Some("panther".to_string()),
            api_level: Some(33),
        },
    );
    assert_eq!(api.match_scope, MatchScope::AndroidVersion);
    assert_eq!(api.matched_rule_id.as_deref(), Some("fixture-promo-api31"));
    assert_eq!(api.risk, RiskClassification::Caution);

    // General match (non-Samsung on API 29)
    let general = engine.evaluate_package(
        "com.example.promo",
        &DeviceMatchContext {
            manufacturer: Some("OnePlus".to_string()),
            model: Some("HD1900".to_string()),
            codename: Some("hotdogb".to_string()),
            api_level: Some(29),
        },
    );
    assert_eq!(general.match_scope, MatchScope::General);
    assert_eq!(general.matched_rule_id.as_deref(), Some("fixture-promo-general"));
    assert_eq!(general.risk, RiskClassification::RecommendedToDisable);
}

#[test]
fn fixture_conflict_resolution_prefers_more_restrictive_risk_and_records_conflict() {
    let raw = read_fixture("package_rules_sample.json");
    let db = parse_and_validate_rule_database(&raw).expect("fixture db must validate");
    let engine = RuleEngine::from_database(db).expect("engine");

    let rec = engine.evaluate_package(
        "com.example.conflicted",
        &DeviceMatchContext {
            manufacturer: Some("Xiaomi".to_string()),
            ..DeviceMatchContext::default()
        },
    );
    assert_eq!(rec.risk, RiskClassification::Caution);
    assert_eq!(rec.matched_rule_id.as_deref(), Some("fixture-conflict-caution"));
    assert!(rec.conflict_detected);
    assert!(rec.conflict_note.is_some());
}

#[test]
fn fixture_batch_evaluation_handles_special_android_package_and_unknowns() {
    let raw = read_fixture("package_rules_sample.json");
    let db = parse_and_validate_rule_database(&raw).expect("fixture db must validate");
    let engine = RuleEngine::from_database(db).expect("engine");

    let batch = engine
        .evaluate_batch(
            &[
                "android".to_string(),
                "com.unknown.thirdparty".to_string(),
            ],
            &DeviceMatchContext::default(),
        )
        .expect("batch evaluation should succeed");

    assert_eq!(batch.recommendations.len(), 2);
    assert_eq!(
        batch.recommendations["android"].risk,
        RiskClassification::DoNotDisable
    );
    assert_eq!(
        batch.recommendations["com.unknown.thirdparty"].risk,
        RiskClassification::UnknownUnverified
    );

    let err = engine
        .evaluate_batch(
            &["invalid package;id".to_string()],
            &DeviceMatchContext::default(),
        )
        .expect_err("invalid package identifier must be rejected");
    assert!(matches!(err, AdtError::InvalidPackageName { .. }));
}

#[test]
fn embedded_production_database_contains_no_fabricated_verified_rules() {
    let engine = RuleEngine::from_embedded().expect("embedded database must load cleanly");
    let summary = engine.summary();
    assert_eq!(summary.schema_version, 1);
    assert_eq!(summary.verified_rules, 0);
    assert_eq!(summary.active_rules, 0);
}
