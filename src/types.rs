use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub model: String,
    pub temperature: f32,
    pub seed: Option<u64>,
    pub max_calls: usize,
    pub max_tokens: u32,
    pub max_response_bytes: usize,
    pub timeout_ms: u64,
    pub run_timeout_ms: u64,
    pub witness: bool,
    pub memory: bool,
    pub intervention_repeats: usize,
    pub support_threshold: f64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            model: "local-model".into(),
            temperature: 0.2,
            seed: None,
            max_calls: 12,
            max_tokens: 512,
            max_response_bytes: 65536,
            timeout_ms: 30000,
            run_timeout_ms: 120000,
            witness: true,
            memory: false,
            intervention_repeats: 1,
            support_threshold: 1.0,
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        if self.model.is_empty()
            || !self.temperature.is_finite()
            || !(0.0..=2.0).contains(&self.temperature)
            || !(1..=10).contains(&self.intervention_repeats)
            || !self.support_threshold.is_finite()
            || !(0.5..=1.0).contains(&self.support_threshold)
            || self.max_calls == 0
            || self.max_calls > 100
            || self.max_tokens == 0
            || self.max_tokens > 32768
            || self.max_response_bytes == 0
            || self.max_response_bytes > 1048576
            || self.timeout_ms == 0
            || self.run_timeout_ms == 0
        {
            return Err("invalid model configuration or resource bounds".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: String,
    pub prompt: String,
    pub expected: String,
    #[serde(default)]
    pub concepts: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Intervention {
    pub id: String,
    pub hypothesis: String,
    pub factor: String,
    pub prompt: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailureCase {
    pub case_id: String,
    pub expected_behavior: String,
    pub baseline_output: String,
    pub failure_class: String,
    pub original_prompt: String,
    pub clean_prompt: String,
    pub interventions: Vec<Intervention>,
    #[serde(default)]
    pub regression: Vec<Task>,
    #[serde(default)]
    pub seeded_cause: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Baseline,
    Diversity,
    Dream,
    Lucid,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exploration {
    pub answer: String,
    pub connection: String,
    pub rationale: String,
    pub source_concepts: Vec<String>,
    pub evidence: Vec<String>,
    pub counterevidence: Vec<String>,
    pub discriminating_test: String,
    pub uncertainty: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Observation {
    pub branch: String,
    pub parent: Option<String>,
    #[serde(default)]
    pub memory_ids: Vec<String>,
    pub content: String,
    pub prompt: String,
    pub expected: String,
    pub config: Config,
    pub exploration: Option<Exploration>,
    pub correct: bool,
    pub latency_ms: u64,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub run_id: String,
    pub branch: String,
    pub hypothesis: String,
    pub answer: String,
    pub source_concepts: Vec<String>,
    pub rationale: String,
    pub evidence: Vec<String>,
    pub counterevidence: Vec<String>,
    pub discriminating_test: String,
    pub uncertainty: String,
    pub baseline_contained: Option<bool>,
    pub status: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Metrics {
    pub calls: usize,
    pub witness_calls: usize,
    pub inference_calls: usize,
    pub correct: usize,
    pub evaluated: usize,
    pub validated_discoveries: usize,
    #[serde(alias = "known_answer_improvements")]
    pub best_of_n_answer_improvements: usize,
    pub final_answer_improvements: usize,
    pub regression_passed: usize,
    pub regression_total: usize,
    pub repair_success: Option<bool>,
    pub seeded_cause_correct: Option<bool>,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub usage_complete: bool,
    pub elapsed_ms: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Run {
    pub id: String,
    pub mode: Mode,
    pub subject_id: String,
    pub input: serde_json::Value,
    pub provider: String,
    pub config: Config,
    pub config_fingerprint: String,
    pub status: String,
    pub observations: Vec<Observation>,
    pub candidates: Vec<Candidate>,
    pub diagnosis: Option<String>,
    pub causal_confidence: String,
    pub witness: Option<String>,
    pub metrics: Metrics,
    #[serde(alias = "reset_verified")]
    pub configuration_unchanged: bool,
}
/// Optional data-only SBH integration; does not authorize tool execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessEnvelope {
    pub task: Task,
    pub trace_id: String,
    pub stop_and_ask: bool,
}
