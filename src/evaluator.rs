use crate::types::{Result, Run};
use serde::Serialize;

pub fn exact(output: &str, expected: &str) -> bool {
    output.trim() == expected.trim()
}
#[derive(Debug, Serialize)]
pub struct Comparison {
    pub baseline_correct: usize,
    pub baseline_total: usize,
    pub experimental_correct: usize,
    pub experimental_total: usize,
    pub discovery_count: usize,
    pub baseline_branch_correct: usize,
    pub baseline_branch_total: usize,
    pub experimental_branch_correct: usize,
    pub experimental_branch_total: usize,
    pub baseline_witness_calls: usize,
    pub experimental_witness_calls: usize,
    pub baseline_calls: usize,
    pub experimental_calls: usize,
    pub comparable_call_budget: bool,
    pub correctness_rate_delta: Option<f64>,
}
pub fn compare(baseline: &Run, experiment: &Run) -> Result<Comparison> {
    if baseline.subject_id != experiment.subject_id
        || baseline.config.model != experiment.config.model
        || baseline.provider != experiment.provider
        || baseline.input != experiment.input
    {
        return Err("comparison requires matching task, subject, provider and model".into());
    }
    if baseline.status != "completed" || experiment.status != "completed" {
        return Err("comparison requires completed runs".into());
    }
    Ok(Comparison {
        baseline_correct: usize::from(final_answer(baseline).is_some_and(|o| o.correct)),
        baseline_total: usize::from(final_answer(baseline).is_some()),
        experimental_correct: usize::from(final_answer(experiment).is_some_and(|o| o.correct)),
        experimental_total: usize::from(final_answer(experiment).is_some()),
        discovery_count: experiment.metrics.validated_discoveries,
        baseline_branch_correct: baseline.metrics.correct,
        baseline_branch_total: baseline.metrics.evaluated,
        experimental_branch_correct: experiment.metrics.correct,
        experimental_branch_total: experiment.metrics.evaluated,
        baseline_witness_calls: witness_calls(baseline),
        experimental_witness_calls: witness_calls(experiment),
        baseline_calls: baseline.metrics.calls,
        experimental_calls: experiment.metrics.calls,
        comparable_call_budget: baseline
            .metrics
            .calls
            .saturating_sub(witness_calls(baseline))
            == experiment
                .metrics
                .calls
                .saturating_sub(witness_calls(experiment)),
        correctness_rate_delta: final_answer(experiment)
            .zip(final_answer(baseline))
            .map(|(a, b)| f64::from(a.correct) - f64::from(b.correct)),
    })
}

#[derive(Debug, Serialize)]
pub struct Aggregate {
    pub trials: usize,
    pub correct: usize,
    pub accuracy: Option<f64>,
    pub wilson_95: Option<(f64, f64)>,
    pub total_calls: usize,
    pub total_latency_ms: u64,
    pub token_usage_complete: bool,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub monetary_cost: Option<f64>,
}
/// Trial-level interval; branches within a run are not independent trials.
pub fn aggregate(runs: &[Run]) -> Result<Aggregate> {
    if runs.is_empty() {
        return Err("at least one trial required".into());
    }
    let mut ids = std::collections::HashSet::new();
    for run in runs {
        if run.status != "completed"
            || run.subject_id != runs[0].subject_id
            || run.input != runs[0].input
            || run.config_fingerprint != runs[0].config_fingerprint
            || run.mode != runs[0].mode
            || run.provider != runs[0].provider
            || !ids.insert(&run.id)
        {
            return Err("aggregate needs distinct completed trials with matching subject, provider, mode and configuration".into());
        }
    }
    let correct = runs
        .iter()
        .filter(|r| final_answer(r).is_some_and(|o| o.correct))
        .count();
    let n = runs.len() as f64;
    let p = correct as f64 / n;
    let z = 1.96_f64;
    let denom = 1.0 + z * z / n;
    let center = (p + z * z / (2.0 * n)) / denom;
    let half = z * ((p * (1.0 - p) + z * z / (4.0 * n)) / n).sqrt() / denom;
    Ok(Aggregate {
        trials: runs.len(),
        correct,
        accuracy: Some(p),
        wilson_95: Some((center - half, center + half)),
        total_calls: runs.iter().map(|r| r.metrics.calls).sum(),
        total_latency_ms: runs.iter().map(|r| r.metrics.elapsed_ms).sum(),
        token_usage_complete: runs.iter().all(|r| r.metrics.usage_complete),
        prompt_tokens: runs.iter().map(|r| r.metrics.prompt_tokens).sum(),
        completion_tokens: runs.iter().map(|r| r.metrics.completion_tokens).sum(),
        monetary_cost: None,
    })
}

pub fn final_answer(run: &Run) -> Option<&crate::types::Observation> {
    run.observations.iter().rev().find(|o| {
        matches!(o.branch.as_str(), "synthesis" | "correction" | "baseline")
            || o.branch.starts_with("diversity-")
    })
}
fn witness_calls(run: &Run) -> usize {
    run.observations
        .iter()
        .filter(|o| o.branch == "witness")
        .count()
}
