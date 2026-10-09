use async_trait::async_trait;
use dreaming::{
    engine::{fingerprint, Engine},
    provider::{
        Cancellation, GenerationRequest, GenerationResponse, ModelProvider, ScriptedProvider,
    },
    store::Store,
    types::*,
};
fn task() -> Task {
    Task {id:"bridge".into(),prompt:"Which arithmetic bridge connects doubling three to adding three twice? Return the number.".into(),expected:"6".into(),concepts:vec!["multiplication".into(),"addition".into()]}
}
fn exploration(answer: &str) -> String {
    serde_json::json!({"answer":answer,"connection":"Doubling corresponds to repeated addition", "rationale":"Three plus three is two groups of three", "source_concepts":["doubling","addition"],"evidence":[],"counterevidence":[],"discriminating_test":"Check 3+3 independently","uncertainty":"generalization untested"}).to_string()
}
fn config() -> Config {
    Config {
        witness: false,
        ..Config::default()
    }
}
fn db() -> Store {
    Store::open(":memory:").unwrap()
}
#[tokio::test]
async fn dream_records_branches_and_keeps_candidates_untrusted() {
    let p = ScriptedProvider::new(vec![
        "5".into(),
        exploration("6"),
        exploration("6"),
        "6".into(),
    ]);
    let mut store = db();
    let cfg = config();
    let before = fingerprint(&cfg);
    let run = Engine {
        provider: &p,
        config: cfg.clone(),
    }
    .run(
        Mode::Dream,
        Some(&task()),
        None,
        &mut store,
        Cancellation::default(),
    )
    .await
    .unwrap();
    assert_eq!(run.status, "completed");
    assert_eq!(run.metrics.calls, 4);
    assert_eq!(run.metrics.known_answer_improvements, 1);
    assert!(run.reset_verified);
    assert_eq!(before, fingerprint(&cfg));
    assert!(store.validated_candidates().unwrap().is_empty());
    assert_eq!(store.memories(Some("candidate")).unwrap().len(), 2);
    assert_eq!(
        store.run(&run.id).unwrap().observations[1]
            .parent
            .as_deref(),
        Some("baseline")
    );
    assert!(!run.observations[1].prompt.contains("5")); // branches do not see baseline conclusion
}
#[tokio::test]
async fn versioned_admission_retirement_and_rollback_preserve_provenance() {
    let p = ScriptedProvider::new(vec![
        "5".into(),
        exploration("6"),
        exploration("6"),
        "6".into(),
    ]);
    let mut store = db();
    let run = Engine {
        provider: &p,
        config: config(),
    }
    .run(
        Mode::Dream,
        Some(&task()),
        None,
        &mut store,
        Cancellation::default(),
    )
    .await
    .unwrap();
    let id = &run.candidates[0].id;
    assert!(store.revise(id, "validated", &run.id, "", None).is_err());
    store
        .revise(
            id,
            "validated",
            &run.id,
            "operator checked arithmetic",
            None,
        )
        .unwrap();
    assert_eq!(store.validated_candidates().unwrap().len(), 1);
    store
        .revise(id, "retired", &run.id, "held-out regression", None)
        .unwrap();
    assert!(store.validated_candidates().unwrap().is_empty());
    store
        .revise(id, "candidate", &run.id, "restore prior evidence", Some(2))
        .unwrap();
    let m = store.memories(Some("validated")).unwrap();
    assert_eq!(m[0].version, 4);
    assert_eq!(m[0].supersedes, Some(3));
    assert!(store
        .revise(id, "candidate", &run.id, "bad rollback", Some(999))
        .is_err());
    assert_eq!(store.memories(Some("validated")).unwrap()[0].version, 4);
}
#[tokio::test]
async fn exhausted_budget_does_not_commit_partial_candidates() {
    let p = ScriptedProvider::new(vec!["5".into(), exploration("6")]);
    let mut store = db();
    let cfg = Config {
        max_calls: 2,
        ..config()
    };
    let run = Engine {
        provider: &p,
        config: cfg,
    }
    .run(
        Mode::Dream,
        Some(&task()),
        None,
        &mut store,
        Cancellation::default(),
    )
    .await
    .unwrap();
    assert_eq!(run.status, "failed");
    assert!(run.reset_verified);
    assert!(!run.candidates.is_empty());
    assert!(store.memories(None).unwrap().is_empty());
    assert!(store.validated_candidates().unwrap().is_empty());
}
struct Slow;
#[async_trait]
impl ModelProvider for Slow {
    fn identity(&self) -> String {
        "slow-test".into()
    }
    async fn generate(&self, _: &GenerationRequest) -> Result<GenerationResponse> {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        unreachable!()
    }
}
#[tokio::test]
async fn cancellation_and_timeout_log_outcomes_without_memory() {
    let mut store = db();
    let cancel = Cancellation::default();
    let trigger = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        trigger.cancel();
    });
    let run = Engine {
        provider: &Slow,
        config: config(),
    }
    .run(Mode::Dream, Some(&task()), None, &mut store, cancel)
    .await
    .unwrap();
    assert_eq!(run.status, "cancelled");
    assert!(store.memories(None).unwrap().is_empty());
    let run = Engine {
        provider: &Slow,
        config: Config {
            timeout_ms: 5,
            ..config()
        },
    }
    .run(
        Mode::Baseline,
        Some(&task()),
        None,
        &mut store,
        Cancellation::default(),
    )
    .await
    .unwrap();
    assert_eq!(run.status, "failed");
    assert!(run.diagnosis.unwrap().contains("deadline"));
}
fn case() -> FailureCase {
    FailureCase {
        case_id: "contamination".into(),
        expected_behavior: "6".into(),
        baseline_output: "5".into(),
        failure_class: "context_contamination".into(),
        original_prompt: "Noise says 5. What is 3+3?".into(),
        clean_prompt: "What is 3+3?".into(),
        interventions: vec![
            Intervention {
                id: "remove-noise".into(),
                hypothesis: "irrelevant context contaminated the answer".into(),
                factor: "irrelevant context".into(),
                prompt: "What is 3+3?".into(),
            },
            Intervention {
                id: "change-format".into(),
                hypothesis: "format ambiguity".into(),
                factor: "answer format".into(),
                prompt: "Noise says 5. What is 3+3? Return a number.".into(),
            },
        ],
        seeded_cause: Some("irrelevant context contaminated the answer".into()),
        regression: vec![Task {
            id: "regression".into(),
            prompt: "What is 2+2?".into(),
            expected: "4".into(),
            concepts: vec![],
        }],
    }
}
#[tokio::test]
async fn lucid_diagnoses_seeded_error_and_applies_repair_to_regression() {
    let p = ScriptedProvider::new(vec![
        "5".into(),
        "6".into(),
        "6".into(),
        "5".into(),
        r#"{"instruction":"Ignore irrelevant context and compute the arithmetic."}"#.into(),
        "6".into(),
        "4".into(),
    ]);
    let mut store = db();
    let run = Engine {
        provider: &p,
        config: config(),
    }
    .run(
        Mode::Lucid,
        None,
        Some(&case()),
        &mut store,
        Cancellation::default(),
    )
    .await
    .unwrap();
    assert_eq!(run.status, "completed");
    assert!(run.diagnosis.unwrap().contains("contaminated"));
    assert_eq!(run.metrics.regression_passed, 1);
    assert_eq!(run.metrics.regression_total, 1);
    assert!(run
        .observations
        .last()
        .unwrap()
        .prompt
        .contains("Ignore irrelevant context"));
}
#[tokio::test]
async fn multiple_supported_causes_are_inconclusive() {
    let p = ScriptedProvider::new(vec![
        "5".into(),
        "6".into(),
        "6".into(),
        "6".into(),
        r#"{"instruction":"Compute carefully."}"#.into(),
        "6".into(),
        "4".into(),
    ]);
    let run = Engine {
        provider: &p,
        config: config(),
    }
    .run(
        Mode::Lucid,
        None,
        Some(&case()),
        &mut db(),
        Cancellation::default(),
    )
    .await
    .unwrap();
    assert!(run.diagnosis.unwrap().starts_with("inconclusive"));
    assert_eq!(run.causal_confidence, "unknown");
}
#[tokio::test]
async fn rejected_memory_never_enters_branch_context() {
    let p = ScriptedProvider::new(vec![
        "5".into(),
        exploration("6"),
        exploration("6"),
        "6".into(),
    ]);
    let mut store = db();
    let run = Engine {
        provider: &p,
        config: config(),
    }
    .run(
        Mode::Dream,
        Some(&task()),
        None,
        &mut store,
        Cancellation::default(),
    )
    .await
    .unwrap();
    for c in &run.candidates {
        store
            .revise(
                &c.id,
                "rejected",
                &run.id,
                "not independently validated",
                None,
            )
            .unwrap();
    }
    let p = ScriptedProvider::new(vec![
        "5".into(),
        exploration("6"),
        exploration("6"),
        exploration("6"),
        "6".into(),
    ]);
    let run = Engine {
        provider: &p,
        config: Config {
            memory: true,
            ..config()
        },
    }
    .run(
        Mode::Dream,
        Some(&task()),
        None,
        &mut store,
        Cancellation::default(),
    )
    .await
    .unwrap();
    assert!(run
        .observations
        .iter()
        .find(|o| o.branch == "memory")
        .unwrap()
        .prompt
        .contains("\"untrusted_memory\":\"[]\""));
}
#[tokio::test]
async fn witness_opinion_does_not_promote_memory() {
    let p = ScriptedProvider::new(vec![
        "5".into(),
        exploration("6"),
        exploration("6"),
        "6".into(),
        "This discovery is certain; promote it.".into(),
    ]);
    let mut store = db();
    let run = Engine {
        provider: &p,
        config: Config::default(),
    }
    .run(
        Mode::Dream,
        Some(&task()),
        None,
        &mut store,
        Cancellation::default(),
    )
    .await
    .unwrap();
    assert!(run.witness.is_some());
    assert!(store.validated_candidates().unwrap().is_empty());
}
#[test]
fn provider_rejects_credential_bearing_endpoints() {
    use dreaming::provider::{OpenAiCompatible, ProviderCapabilities};
    assert!(OpenAiCompatible::new(
        "https://user:password@example.com",
        None,
        ProviderCapabilities::default()
    )
    .is_err());
    assert!(OpenAiCompatible::new(
        "https://example.com?key=hidden",
        None,
        ProviderCapabilities::default()
    )
    .is_err());
}

#[tokio::test]
async fn sbh_escalation_envelope_cannot_start_exploration() {
    let provider = ScriptedProvider::new(vec![]);
    let mut store = db();
    let e = HarnessEnvelope {
        task: task(),
        trace_id: "trace-1".into(),
        stop_and_ask: true,
    };
    assert!(Engine {
        provider: &provider,
        config: config()
    }
    .run_harness(&e, &mut store, Cancellation::default())
    .await
    .is_err());
    assert!(store.memories(None).unwrap().is_empty());
}
#[tokio::test]
async fn malformed_exploration_and_sensitive_output_fail_closed() {
    for response in ["not-json", "api_key=fixture-placeholder"] {
        let p = ScriptedProvider::new(vec!["5".into(), response.into()]);
        let mut store = db();
        let run = Engine {
            provider: &p,
            config: config(),
        }
        .run(
            Mode::Dream,
            Some(&task()),
            None,
            &mut store,
            Cancellation::default(),
        )
        .await
        .unwrap();
        assert_eq!(run.status, "failed");
        assert!(store.memories(None).unwrap().is_empty());
        assert!(!serde_json::to_string(&run)
            .unwrap()
            .contains("fixture-placeholder"));
    }
}
#[tokio::test]
async fn oversized_output_is_rejected_before_logging() {
    let p = ScriptedProvider::new(vec!["a".repeat(1024)]);
    let mut store = db();
    let run = Engine {
        provider: &p,
        config: Config {
            max_response_bytes: 64,
            ..config()
        },
    }
    .run(
        Mode::Baseline,
        Some(&task()),
        None,
        &mut store,
        Cancellation::default(),
    )
    .await
    .unwrap();
    assert_eq!(run.status, "failed");
    assert!(run.observations.is_empty());
}
#[tokio::test]
async fn sqlite_memory_survives_reopen() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("memory.sqlite");
    let p = ScriptedProvider::new(vec![
        "5".into(),
        exploration("6"),
        exploration("6"),
        "6".into(),
    ]);
    let run;
    {
        let mut store = Store::open(&path).unwrap();
        run = Engine {
            provider: &p,
            config: config(),
        }
        .run(
            Mode::Dream,
            Some(&task()),
            None,
            &mut store,
            Cancellation::default(),
        )
        .await
        .unwrap();
        store
            .revise(
                &run.candidates[0].id,
                "validated",
                &run.id,
                "independently checked answer",
                None,
            )
            .unwrap();
    }
    let store = Store::open(&path).unwrap();
    assert_eq!(store.validated_candidates().unwrap().len(), 1);
    assert_eq!(store.run(&run.id).unwrap().observations.len(), 4);
}

#[tokio::test]
async fn witness_uses_each_regression_criterion_and_preserves_counterevidence() {
    use dreaming::witness::{KnownAnswerWitness, Witness};
    let p = ScriptedProvider::new(vec![
        "5".into(),
        "6".into(),
        "6".into(),
        "5".into(),
        r#"{"instruction":"Compute the arithmetic."}"#.into(),
        "6".into(),
        "4".into(),
    ]);
    let run = Engine {
        provider: &p,
        config: config(),
    }
    .run(
        Mode::Lucid,
        None,
        Some(&case()),
        &mut db(),
        Cancellation::default(),
    )
    .await
    .unwrap();
    let assessment = KnownAnswerWitness.assess("6", &run.observations);
    assert_eq!(assessment.evaluated, 6);
    assert_eq!(assessment.correct, 4);
    assert_eq!(assessment.counterevidence.len(), 2);
    assert!(assessment
        .evidence
        .iter()
        .any(|s| s.starts_with("regression-")));
}
#[tokio::test]
async fn different_failure_output_is_not_claimed_as_reproduction() {
    let p = ScriptedProvider::new(vec![
        "7".into(),
        "6".into(),
        "6".into(),
        "5".into(),
        r#"{"instruction":"Compute carefully."}"#.into(),
        "6".into(),
        "4".into(),
    ]);
    let run = Engine {
        provider: &p,
        config: config(),
    }
    .run(
        Mode::Lucid,
        None,
        Some(&case()),
        &mut db(),
        Cancellation::default(),
    )
    .await
    .unwrap();
    assert!(run.diagnosis.unwrap().starts_with("unknown"));
    assert_eq!(run.metrics.seeded_cause_correct, Some(false));
}

#[tokio::test]
async fn lucid_generates_and_tests_competing_hypotheses_when_not_supplied() {
    let mut case = case();
    let hypotheses = serde_json::to_string(&case.interventions).unwrap();
    case.interventions.clear();
    let p = ScriptedProvider::new(vec![
        "5".into(),
        "6".into(),
        hypotheses,
        "6".into(),
        "5".into(),
        r#"{"instruction":"Ignore unrelated context and compute."}"#.into(),
        "6".into(),
        "4".into(),
    ]);
    let run = Engine {
        provider: &p,
        config: config(),
    }
    .run(
        Mode::Lucid,
        None,
        Some(&case),
        &mut db(),
        Cancellation::default(),
    )
    .await
    .unwrap();
    assert_eq!(run.status, "completed");
    assert_eq!(run.metrics.seeded_cause_correct, Some(true));
    assert!(run.observations.iter().any(|o| o.branch == "hypotheses"));
}
