//! Independent assessment boundary. Known-answer evidence never uses model agreement.
use crate::{evaluator::exact, types::Observation};
use serde::Serialize;
#[derive(Debug, Serialize)]
pub struct Assessment {
    pub evaluated: usize,
    pub correct: usize,
    pub evidence: Vec<String>,
    pub counterevidence: Vec<String>,
    pub semantic_novelty: String,
}
pub trait Witness {
    fn assess(&self, _expected: &str, observations: &[Observation]) -> Assessment;
}
pub struct KnownAnswerWitness;
impl Witness for KnownAnswerWitness {
    fn assess(&self, _expected: &str, observations: &[Observation]) -> Assessment {
        let correct = observations
            .iter()
            .filter(|o| !o.expected.is_empty() && exact(&o.content, &o.expected))
            .count();
        Assessment {
            evaluated: observations
                .iter()
                .filter(|o| !o.expected.is_empty())
                .count(),
            correct,
            evidence: observations
                .iter()
                .filter(|o| !o.expected.is_empty() && exact(&o.content, &o.expected))
                .map(|o| format!("{} matches independent expected answer", o.branch))
                .collect(),
            counterevidence: observations
                .iter()
                .filter(|o| !o.expected.is_empty() && !exact(&o.content, &o.expected))
                .map(|o| format!("{} does not match independent expected answer", o.branch))
                .collect(),
            semantic_novelty: "unknown: exact correctness is not novelty".into(),
        }
    }
}
