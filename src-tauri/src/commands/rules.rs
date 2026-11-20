use tauri::State;

use crate::core::errors::AdtError;
use crate::rules::models::{
    DeviceMatchContext, RecommendationBatchResult, RuleDatabaseSummary,
};
use crate::AppState;

#[tauri::command]
pub fn get_rule_database_summary(
    state: State<'_, AppState>,
) -> Result<RuleDatabaseSummary, AdtError> {
    Ok(state.rule_engine.summary())
}

#[tauri::command]
pub fn evaluate_recommendations(
    package_names: Vec<String>,
    context: DeviceMatchContext,
    state: State<'_, AppState>,
) -> Result<RecommendationBatchResult, AdtError> {
    state.rule_engine.evaluate_batch(&package_names, &context)
}
