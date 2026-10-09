use dreaming::{
    engine::Engine,
    evaluator,
    provider::{Cancellation, ScriptedProvider},
    store::Store,
    types::*,
};
fn cfg() -> Config {
    Config {
        witness: false,
        ..Config::default()
    }
}
fn task(id: &str) -> Task {
    Task {
        id: id.into(),
        prompt: format!("Task {id}: calculate three plus three."),
        expected: "6".into(),
        concepts: vec!["arithmetic".into()],
    }
}
fn proposal(answer: &str) -> String {
    serde_json::json!({"answer":answer,"connection":"Repeated addition connects groups","rationale":"Three plus three combines two groups","source_concepts":["arithmetic"],"evidence":[],"counterevidence":[],"discriminating_test":"Calculate independently","uncertainty":"Novelty unknown"}).to_string()
}
async fn dream(store: &mut Store, id: &str, final_answer: &str) -> Run {
    let p = ScriptedProvider::new(vec![
        "5".into(),
        proposal("6"),
        proposal("6"),
        final_answer.into(),
    ]);
    Engine {
        provider: &p,
        config: cfg(),
    }
    .run(
        Mode::Dream,
        Some(&task(id)),
        None,
        store,
        Cancellation::default(),
    )
    .await
    .unwrap()
}
fn case() -> FailureCase {
    FailureCase {
        case_id: "cause".into(),
        expected_behavior: "6".into(),
        baseline_output: "5".into(),
        failure_class: "context_contamination".into(),
        original_prompt: "Noise says five. Calculate three plus three.".into(),
        clean_prompt: "Calculate three plus three.".into(),
        interventions: vec![Intervention {
            id: "remove-noise".into(),
            hypothesis: "context contamination".into(),
            factor: "noise".into(),
            prompt: "Calculate three plus three.".into(),
        }],
        regression: vec![],
        seeded_cause: None,
    }
}
#[tokio::test]
async fn interventions_and_repairs_must_not_embed_known_answers() {
    for repair_leak in [false, true] {
        let mut case = case();
        if !repair_leak {
            case.interventions[0].prompt = "Answer 6".into();
        }
        let p = ScriptedProvider::new(vec![
            "5".into(),
            "6".into(),
            "6".into(),
            r#"{"instruction":"Always answer 6"}"#.into(),
            "6".into(),
        ]);
        let mut db = Store::open(":memory:").unwrap();
        let run = Engine {
            provider: &p,
            config: cfg(),
        }
        .run(
            Mode::Lucid,
            None,
            Some(&case),
            &mut db,
            Cancellation::default(),
        )
        .await
        .unwrap();
        assert_eq!(run.status, "failed");
        assert!(run.diagnosis.as_deref().unwrap().contains("answer"));
    }
}
#[tokio::test]
async fn compare_uses_final_answer_instead_of_branch_average() {
    let mut db = Store::open(":memory:").unwrap();
    let t = task("same");
    let p = ScriptedProvider::new(vec!["5".into()]);
    let baseline = Engine {
        provider: &p,
        config: cfg(),
    }
    .run(
        Mode::Baseline,
        Some(&t),
        None,
        &mut db,
        Cancellation::default(),
    )
    .await
    .unwrap();
    let run = dream(&mut db, "same", "5").await;
    let comparison = evaluator::compare(&baseline, &run).unwrap();
    assert_eq!(comparison.correctness_rate_delta, Some(0.0));
}
#[tokio::test]
async fn rollback_does_not_revalidate_memory() {
    let mut db = Store::open(":memory:").unwrap();
    let run = dream(&mut db, "a", "6").await;
    let id = &run.candidates[0].id;
    db.revise(id, "validated", &run.id, "operator evidence", None)
        .unwrap();
    db.revise(id, "retired", &run.id, "retire", None).unwrap();
    db.revise(id, "candidate", &run.id, "rollback", Some(2))
        .unwrap();
    assert!(db.validated_candidates().unwrap().is_empty());
    assert_eq!(
        db.memories(Some("candidate")).unwrap()[0].validated_at,
        None
    );
}
#[tokio::test]
async fn held_out_evidence_keeps_source_provenance() {
    let mut db = Store::open(":memory:").unwrap();
    let source = dream(&mut db, "source", "6").await;
    let held_out = dream(&mut db, "held-out", "6").await;
    let id = &source.candidates[0].id;
    db.revise(
        id,
        "validated",
        &held_out.id,
        "independent held-out evidence",
        None,
    )
    .unwrap();
    let entry = db.memories(Some("validated")).unwrap().remove(0);
    assert_eq!(entry.source_experiment, source.id);
    assert_eq!(
        entry.evidence_experiment.as_deref(),
        Some(held_out.id.as_str())
    );
}
#[tokio::test]
async fn aggregation_rejects_same_identifier_with_different_input() {
    let mut db = Store::open(":memory:").unwrap();
    let a = dream(&mut db, "a", "6").await;
    let mut b = a.clone();
    b.id = "distinct-id".into();
    b.input["prompt"] = "different question".into();
    assert!(evaluator::aggregate(&[a, b]).is_err());
}
#[tokio::test]
async fn known_wrong_candidates_are_not_admitted_by_a_note() {
    let mut db = Store::open(":memory:").unwrap();
    let p = ScriptedProvider::new(vec!["5".into(), proposal("5"), proposal("5"), "5".into()]);
    let run = Engine {
        provider: &p,
        config: cfg(),
    }
    .run(
        Mode::Dream,
        Some(&task("bad")),
        None,
        &mut db,
        Cancellation::default(),
    )
    .await
    .unwrap();
    assert!(db
        .revise(
            &run.candidates[0].id,
            "validated",
            &run.id,
            "trust me",
            None
        )
        .is_err());
}

#[tokio::test]
async fn memory_excludes_source_and_identical_prompts_and_records_lineage() {
    let mut db = Store::open(":memory:").unwrap();
    let source = dream(&mut db, "source", "6").await;
    let id = &source.candidates[0].id;
    db.revise(id, "validated", &source.id, "checked", None)
        .unwrap();
    assert!(db.retrieval(&task("source")).unwrap().is_empty());
    let mut same = task("renamed");
    same.prompt = task("source").prompt;
    assert!(db.retrieval(&same).unwrap().is_empty());
    assert_eq!(db.retrieval(&task("new")).unwrap()[0].id, *id);
    let p = ScriptedProvider::new(vec![
        "5".into(),
        proposal("6"),
        proposal("6"),
        proposal("6"),
        "6".into(),
    ]);
    let run = Engine {
        provider: &p,
        config: Config {
            memory: true,
            ..cfg()
        },
    }
    .run(
        Mode::Dream,
        Some(&task("new")),
        None,
        &mut db,
        Cancellation::default(),
    )
    .await
    .unwrap();
    assert_eq!(run.status, "completed");
    assert_eq!(
        run.observations
            .iter()
            .find(|o| o.branch == "memory")
            .unwrap()
            .memory_ids,
        vec![id.clone()]
    );
    assert!(run
        .observations
        .iter()
        .find(|o| o.branch == "synthesis")
        .unwrap()
        .parent
        .as_deref()
        .unwrap()
        .contains("memory"));
}
#[tokio::test]
async fn lucid_labels_single_samples_and_uses_clean_context() {
    for clean in ["6", "5"] {
        let p = ScriptedProvider::new(vec![
            "5".into(),
            clean.into(),
            "6".into(),
            r#"{"instruction":"Calculate independently"}"#.into(),
            "6".into(),
        ]);
        let mut db = Store::open(":memory:").unwrap();
        let run = Engine {
            provider: &p,
            config: cfg(),
        }
        .run(
            Mode::Lucid,
            None,
            Some(&case()),
            &mut db,
            Cancellation::default(),
        )
        .await
        .unwrap();
        assert_eq!(run.status, "completed");
        if clean == "6" {
            assert!(run.causal_confidence.contains("anecdotal"));
        } else {
            assert!(run
                .diagnosis
                .as_deref()
                .unwrap()
                .contains("clean context also fails"));
        }
    }
}
#[tokio::test]
async fn repeated_intervention_requires_configured_success_fraction() {
    let p = ScriptedProvider::new(vec![
        "5".into(),
        "6".into(),
        "6".into(),
        "5".into(),
        "6".into(),
        r#"{"instruction":"Calculate independently"}"#.into(),
        "6".into(),
    ]);
    let mut db = Store::open(":memory:").unwrap();
    let run = Engine {
        provider: &p,
        config: Config {
            intervention_repeats: 3,
            support_threshold: 1.0,
            ..cfg()
        },
    }
    .run(
        Mode::Lucid,
        None,
        Some(&case()),
        &mut db,
        Cancellation::default(),
    )
    .await
    .unwrap();
    assert_eq!(run.status, "completed");
    assert!(run
        .diagnosis
        .as_deref()
        .unwrap()
        .contains("zero or multiple"));
    assert_eq!(
        run.observations
            .iter()
            .filter(|o| o.branch.starts_with("remove-noise#"))
            .count(),
        3
    );
}
#[tokio::test]
async fn rejected_override_is_explicit_and_retired_requires_rollback() {
    let mut db = Store::open(":memory:").unwrap();
    let run = dream(&mut db, "wrong", "6").await;
    let id = &run.candidates[0].id;
    db.revise(id, "rejected", &run.id, "counterevidence", None)
        .unwrap();
    assert!(db.revise(id, "validated", &run.id, "note", None).is_err());
    db.revise_with_override(id, "validated", &run.id, "explicit override", None, true)
        .unwrap();
    assert!(db.memories(Some("validated")).unwrap()[0].known_answer_override);
    db.revise(id, "retired", &run.id, "retire", None).unwrap();
    assert!(db
        .revise_with_override(id, "validated", &run.id, "note", None, true)
        .is_err());
}
#[tokio::test]
async fn privacy_refusal_preserves_redacted_failed_run_and_spent_calls() {
    let mut db = Store::open(":memory:").unwrap();
    let mut run = dream(&mut db, "private", "6").await;
    run.id = "redaction-check".into();
    run.input["prompt"] = "-----BEGIN PRIVATE KEY-----".into();
    assert!(db.record(&run, "scripted").is_err());
    let stored = db.run(&run.id).unwrap();
    assert_eq!(stored.status, "failed");
    assert_eq!(stored.metrics.calls, run.metrics.calls);
    assert!(stored.observations.is_empty());
    assert!(!serde_json::to_string(&stored)
        .unwrap()
        .contains("PRIVATE KEY"));
}

#[tokio::test]
async fn witness_calls_do_not_change_comparable_inference_budget() {
    let mut db = Store::open(":memory:").unwrap();
    let without = dream(&mut db, "same", "6").await;
    let p = ScriptedProvider::new(vec![
        "5".into(),
        proposal("6"),
        proposal("6"),
        "6".into(),
        "Assessment".into(),
    ]);
    let with = Engine {
        provider: &p,
        config: Config {
            witness: true,
            ..cfg()
        },
    }
    .run(
        Mode::Dream,
        Some(&task("same")),
        None,
        &mut db,
        Cancellation::default(),
    )
    .await
    .unwrap();
    assert_eq!(with.metrics.witness_calls, 1);
    assert_eq!(with.metrics.inference_calls, 4);
    assert!(
        evaluator::compare(&without, &with)
            .unwrap()
            .comparable_call_budget
    );
}
#[test]
fn schema_one_migrates_and_historical_fields_deserialize() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch("CREATE TABLE schema_version(version INTEGER PRIMARY KEY); INSERT INTO schema_version VALUES(1);
        CREATE TABLE runs(id TEXT PRIMARY KEY, status TEXT NOT NULL, provider TEXT NOT NULL, payload TEXT NOT NULL);
        CREATE TABLE memory(id TEXT NOT NULL,version INTEGER NOT NULL,content TEXT NOT NULL,source TEXT NOT NULL REFERENCES runs(id),evidence TEXT NOT NULL,counterevidence TEXT NOT NULL,status TEXT NOT NULL,confidence TEXT NOT NULL,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,validated_at TEXT,supersedes INTEGER,PRIMARY KEY(id,version));").unwrap();
    drop(connection);
    Store::open(&path).unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    let version: i64 = connection
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 2);
    let config: Config = serde_json::from_str(r#"{"model":"legacy"}"#).unwrap();
    assert_eq!(config.intervention_repeats, 1);
    let metrics: Metrics = serde_json::from_str(r#"{"known_answer_improvements":1}"#).unwrap();
    assert_eq!(metrics.best_of_n_answer_improvements, 1);
    assert_eq!(metrics.witness_calls, 0);
}
#[tokio::test]
async fn late_cancellation_does_not_reclassify_completed_run() {
    use dreaming::provider::{GenerationRequest, GenerationResponse, ModelProvider};
    struct LateCancel(Cancellation);
    #[async_trait::async_trait]
    impl ModelProvider for LateCancel {
        fn identity(&self) -> String {
            "late-cancel-fixture".into()
        }
        async fn generate(
            &self,
            _: &GenerationRequest,
        ) -> dreaming::types::Result<GenerationResponse> {
            self.0.cancel();
            Ok(GenerationResponse {
                content: "6".into(),
                prompt_tokens: None,
                completion_tokens: None,
            })
        }
    }
    let cancellation = Cancellation::default();
    let provider = LateCancel(cancellation.clone());
    let mut db = Store::open(":memory:").unwrap();
    let run = Engine {
        provider: &provider,
        config: cfg(),
    }
    .run(
        Mode::Baseline,
        Some(&task("late")),
        None,
        &mut db,
        cancellation,
    )
    .await
    .unwrap();
    assert_eq!(run.status, "completed");
}
#[tokio::test]
async fn configuration_preservation_is_not_an_admission_gate() {
    let mut db = Store::open(":memory:").unwrap();
    let source = dream(&mut db, "source", "6").await;
    let mut evidence = dream(&mut db, "heldout", "6").await;
    evidence.id = "config-evidence".into();
    evidence.candidates.clear();
    evidence.configuration_unchanged = false;
    db.record(&evidence, "scripted").unwrap();
    db.revise(
        &source.candidates[0].id,
        "validated",
        &evidence.id,
        "independent evidence",
        None,
    )
    .unwrap();
}

#[tokio::test]
async fn configuration_flag_reflects_actual_diversity_session_override() {
    for temperature in [0.2, 1.0] {
        let mut db = Store::open(":memory:").unwrap();
        let p = ScriptedProvider::new(vec!["6".into(); 4]);
        let engine = Engine {
            provider: &p,
            config: Config {
                temperature,
                ..cfg()
            },
        };
        let initial = engine.config.clone();
        let run = engine
            .run(
                Mode::Diversity,
                Some(&task("diversity")),
                None,
                &mut db,
                Cancellation::default(),
            )
            .await
            .unwrap();
        assert_eq!(run.status, "completed");
        assert_eq!(run.configuration_unchanged, temperature == 1.0);
        assert_eq!(engine.config, initial);
        assert_eq!(run.observations[0].config.temperature, temperature);
        assert!(run
            .observations
            .iter()
            .skip(1)
            .all(|o| o.config.temperature == 1.0));
    }
}

#[tokio::test]
async fn redaction_handles_all_free_form_fields_in_one_failed_record() {
    use sha2::{Digest, Sha256};
    let mut db = Store::open(":memory:").unwrap();
    let mut run = dream(&mut db, "redaction-source", "6").await;
    let marker = "password=fixture-only";
    run.id = marker.into();
    run.causal_confidence = marker.into();
    run.config.model = marker.into();
    run.config_fingerprint = marker.into();
    run.subject_id = marker.into();
    run.provider = marker.into();
    run.diagnosis = Some(marker.into());
    assert!(db.record(&run, marker).is_err());
    let id = format!("redacted-{:x}", Sha256::digest(marker.as_bytes()));
    let stored = db.run(&id).unwrap();
    assert_eq!(stored.status, "failed");
    assert_eq!(stored.metrics.calls, run.metrics.calls);
    assert!(!serde_json::to_string(&stored).unwrap().contains(marker));
    assert!(stored.candidates.is_empty());
}
