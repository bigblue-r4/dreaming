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
    let rate = |r: &Run| {
        if r.metrics.evaluated == 0 {
            None
        } else {
            Some(r.metrics.correct as f64 / r.metrics.evaluated as f64)
        }
    };
    Ok(Comparison {
        baseline_correct: baseline.metrics.correct,
        baseline_total: baseline.metrics.evaluated,
        experimental_correct: experiment.metrics.correct,
        experimental_total: experiment.metrics.evaluated,
        discovery_count: experiment.metrics.validated_discoveries,
        baseline_calls: baseline.metrics.calls,
        experimental_calls: experiment.metrics.calls,
        comparable_call_budget: baseline.metrics.calls == experiment.metrics.calls,
        correctness_rate_delta: rate(experiment).zip(rate(baseline)).map(|(a, b)| a - b),
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
        .filter(|r| {
            r.observations
                .iter()
                .rev()
                .find(|o| {
                    matches!(o.branch.as_str(), "synthesis" | "correction" | "baseline")
                        || o.branch.starts_with("diversity-")
                })
                .is_some_and(|o| o.correct)
        })
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
