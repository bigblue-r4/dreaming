use std::{path::Path, process::Command};
fn run(db: &Path, args: &[&str]) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_dreaming"))
        .arg("--db")
        .arg(db)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
fn cli_runs_both_modes_and_memory_lifecycle() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.sqlite");
    let baseline = run(
        &db,
        &[
            "--script",
            "fixtures/baseline-script.json",
            "baseline",
            "--input",
            "fixtures/known-solutions/bridge-001.json",
        ],
    );
    let dream = run(
        &db,
        &[
            "--script",
            "fixtures/dream-script.json",
            "run",
            "--mode",
            "dream",
            "--input",
            "fixtures/known-solutions/bridge-001.json",
        ],
    );
    let lucid = run(
        &db,
        &[
            "--script",
            "fixtures/lucid-script.json",
            "run",
            "--mode",
            "lucid",
            "--case",
            "fixtures/seeded-errors/contamination-001.json",
        ],
    );
    assert_eq!(dream["status"], "completed");
    assert_eq!(lucid["metrics"]["seeded_cause_correct"], true);
    assert_eq!(lucid["metrics"]["repair_success"], true);
    let id = dream["id"].as_str().unwrap();
    assert_eq!(run(&db, &["results", "--run", id])["id"], id);
    let candidates = run(&db, &["memory", "candidates"]);
    let memory = candidates[0]["id"].as_str().unwrap();
    let updated = run(
        &db,
        &[
            "memory",
            "validate",
            memory,
            "--experiment",
            id,
            "--evidence",
            "Operator independently calculated arithmetic",
        ],
    );
    assert!(updated
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["status"] == "validated"));
    run(
        &db,
        &[
            "memory",
            "retire",
            memory,
            "--experiment",
            id,
            "--evidence",
            "Retirement test",
        ],
    );
    let rolled = run(
        &db,
        &[
            "memory",
            "rollback",
            memory,
            "--experiment",
            id,
            "--version",
            "1",
            "--evidence",
            "Restore candidate state",
        ],
    );
    assert!(rolled
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["id"] == memory && m["status"] == "candidate" && m["version"] == 4));
    let compared = run(
        &db,
        &[
            "benchmark",
            "compare",
            "--baseline",
            baseline["id"].as_str().unwrap(),
            "--run",
            id,
        ],
    );
    assert_eq!(compared["comparable_call_budget"], false);
    let second = run(
        &db,
        &[
            "--script",
            "fixtures/dream-script.json",
            "run",
            "--mode",
            "dream",
            "--input",
            "fixtures/known-solutions/bridge-001.json",
        ],
    );
    let trials = format!("{},{}", id, second["id"].as_str().unwrap());
    let aggregate = run(&db, &["benchmark", "aggregate", "--runs", &trials]);
    assert_eq!(aggregate["trials"], 2);
    assert_eq!(aggregate["correct"], 2);
    assert!(aggregate["wilson_95"].is_array());
    assert!(aggregate["monetary_cost"].is_null());
}
#[test]
fn cli_does_not_claim_success_on_exhausted_script() {
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_dreaming"))
        .arg("--db")
        .arg(dir.path().join("test.sqlite"))
        .args([
            "--script",
            "fixtures/baseline-script.json",
            "run",
            "--mode",
            "dream",
            "--input",
            "fixtures/known-solutions/bridge-001.json",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let record: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(record["status"], "failed");
}

#[test]
fn six_seeded_failure_categories_are_diagnosed_by_the_fixture_pipeline() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("seeded.sqlite");
    for (case, script) in [
        ("contamination-001", "lucid"),
        ("claim-001", "claim-001"),
        ("contradiction-001", "contradiction-001"),
        ("constraint-001", "constraint-001"),
        ("conflict-001", "conflict-001"),
        ("tool-001", "tool-001"),
    ] {
        let script = format!("fixtures/{script}-script.json");
        let fixture = format!("fixtures/seeded-errors/{case}.json");
        let record = run(
            &db,
            &[
                "--script", &script, "run", "--mode", "lucid", "--case", &fixture,
            ],
        );
        assert_eq!(record["metrics"]["seeded_cause_correct"], true, "{case}");
        assert_eq!(record["metrics"]["repair_success"], true, "{case}");
        assert_eq!(record["metrics"]["regression_passed"], 1, "{case}");
    }
}
