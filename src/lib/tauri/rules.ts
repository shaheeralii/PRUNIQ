import { invoke } from "@tauri-apps/api/core";
import type {
  DeviceMatchContext,
  RecommendationBatchResult,
  RuleDatabaseSummary,
} from "../../types/rules";

export async function getRuleDatabaseSummary(): Promise<RuleDatabaseSummary> {
  return invoke<RuleDatabaseSummary>("get_rule_database_summary");
}

export async function evaluateRecommendations(
  packageNames: string[],
  context: DeviceMatchContext,
): Promise<RecommendationBatchResult> {
  return invoke<RecommendationBatchResult>("evaluate_recommendations", {
    packageNames,
    context,
  });
}
