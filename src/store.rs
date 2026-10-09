use crate::types::{Candidate, Result, Run, Task};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub struct Store {
    connection: Connection,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub source_experiment: String,
    pub evidence: String,
    pub counterevidence: String,
    pub status: String,
    pub confidence: String,
    pub created_at: String,
    pub validated_at: Option<String>,
    pub version: i64,
    pub supersedes: Option<i64>,
    pub evidence_experiment: Option<String>,
    pub known_answer_override: bool,
}
impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| e.to_string())?;
        connection.execute_batch("PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS schema_version(version INTEGER PRIMARY KEY);
            INSERT OR IGNORE INTO schema_version VALUES(1);
            CREATE TABLE IF NOT EXISTS runs(id TEXT PRIMARY KEY, status TEXT NOT NULL, provider TEXT NOT NULL, payload TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS memory(id TEXT NOT NULL, version INTEGER NOT NULL, content TEXT NOT NULL,
                source TEXT NOT NULL REFERENCES runs(id), evidence TEXT NOT NULL, counterevidence TEXT NOT NULL,
                status TEXT NOT NULL CHECK(status IN ('candidate','validated','rejected','retired')),
                confidence TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                validated_at TEXT, supersedes INTEGER, PRIMARY KEY(id,version));").map_err(|e| e.to_string())?;
        let version: i64 = connection
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if ![1, 2].contains(&version) {
            return Err("unsupported SQLite schema version".into());
        }
        if version == 1 {
            let tx = connection.transaction().map_err(|e| e.to_string())?;
            tx.execute_batch(
                "ALTER TABLE memory ADD COLUMN evidence_run TEXT REFERENCES runs(id);
                ALTER TABLE memory ADD COLUMN known_answer_override INTEGER NOT NULL DEFAULT 0;
                UPDATE memory SET evidence_run=source;
                INSERT INTO schema_version VALUES(2);",
            )
            .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
        }
        Ok(Self { connection })
    }
    pub fn record(&mut self, run: &Run, provider: &str) -> Result<()> {
        let payload = serde_json::to_string(run).map_err(|e| e.to_string())?;
        if crate::privacy::screen(&payload).is_err() || crate::privacy::screen(provider).is_err() {
            let mut safe = run.clone();
            safe.status = "failed".into();
            if crate::privacy::screen(&safe.id).is_err() {
                use sha2::{Digest, Sha256};
                safe.id = format!("redacted-{:x}", Sha256::digest(safe.id.as_bytes()));
            }
            safe.causal_confidence = "unknown".into();
            safe.input = serde_json::Value::Null;
            safe.subject_id = "redacted".into();
            safe.provider = "redacted".into();
            safe.observations.clear();
            safe.candidates.clear();
            safe.witness = None;
            safe.diagnosis = Some("run record refused by privacy screen; details redacted".into());
            safe.config = crate::types::Config::default();
            safe.config_fingerprint = crate::engine::fingerprint(&safe.config);
            self.record(&safe, "redacted")?;
            return Err("unsafe payload replaced by redacted failed run record".into());
        }
        crate::privacy::screen(provider)?;
        let tx = self.connection.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO runs VALUES(?1,?2,?3,?4)",
            params![run.id, run.status, provider, payload],
        )
        .map_err(|e| e.to_string())?;
        if run.status == "completed" {
            for candidate in &run.candidates {
                tx.execute("INSERT INTO memory(id,version,content,source,evidence,counterevidence,status,confidence,evidence_run) VALUES(?1,1,?2,?3,?4,?5,?7,?6,?3)",
                    params![candidate.id, serde_json::to_string(candidate).map_err(|e| e.to_string())?, run.id,
                        serde_json::to_string(&candidate.evidence).map_err(|e| e.to_string())?,
                        serde_json::to_string(&candidate.counterevidence).map_err(|e| e.to_string())?, candidate.uncertainty,if candidate.status=="rejected" {"rejected"} else {"candidate"}]).map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn run(&self, id: &str) -> Result<Run> {
        let text: String = self
            .connection
            .query_row("SELECT payload FROM runs WHERE id=?1", [id], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }
    pub fn memories(&self, status: Option<&str>) -> Result<Vec<MemoryEntry>> {
        let mut stmt = self.connection.prepare("SELECT m.id,m.content,m.source,m.evidence,m.counterevidence,m.status,m.confidence,m.created_at,m.validated_at,m.version,m.supersedes,m.evidence_run,m.known_answer_override FROM memory m WHERE m.version=(SELECT MAX(version) FROM memory WHERE id=m.id) AND (?1 IS NULL OR m.status=?1) ORDER BY m.id").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([status], |r| {
                Ok(MemoryEntry {
                    id: r.get(0)?,
                    content: r.get(1)?,
                    source_experiment: r.get(2)?,
                    evidence: r.get(3)?,
                    counterevidence: r.get(4)?,
                    status: r.get(5)?,
                    confidence: r.get(6)?,
                    created_at: r.get(7)?,
                    validated_at: r.get(8)?,
                    version: r.get(9)?,
                    supersedes: r.get(10)?,
                    evidence_experiment: r.get(11)?,
                    known_answer_override: r.get(12)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }
    pub fn revise(
        &mut self,
        id: &str,
        status: &str,
        experiment: &str,
        evidence: &str,
        rollback: Option<i64>,
    ) -> Result<()> {
        self.revise_with_override(id, status, experiment, evidence, rollback, false)
    }
    pub fn revise_with_override(
        &mut self,
        id: &str,
        status: &str,
        experiment: &str,
        evidence: &str,
        rollback: Option<i64>,
        known_answer_override: bool,
    ) -> Result<()> {
        if !["validated", "rejected", "retired", "candidate"].contains(&status)
            || evidence.trim().is_empty()
        {
            return Err("status and explicit evidence required".into());
        }
        let run = self.run(experiment)?;
        crate::privacy::screen(evidence)?;
        if run.status != "completed" {
            return Err("admission requires completed experiment".into());
        }
        let current = self
            .memories(None)?
            .into_iter()
            .find(|m| m.id == id)
            .ok_or("memory not found")?;
        let tx = self.connection.transaction().map_err(|e| e.to_string())?;
        let old_version = rollback.unwrap_or(current.version);
        // Rollback copies a prior revision into a new version; history is never overwritten.
        let content: String = tx
            .query_row(
                "SELECT content FROM memory WHERE id=?1 AND version=?2",
                params![id, old_version],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let new_status = if rollback.is_some() {
            "candidate"
        } else {
            status
        };
        let candidate: Candidate = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        if rollback.is_none() {
            let allowed = matches!(
                (current.status.as_str(), new_status),
                ("candidate", "validated" | "rejected" | "retired")
                    | ("validated", "retired" | "rejected")
                    | ("rejected", "retired")
            ) || (current.status == "rejected"
                && new_status == "validated"
                && known_answer_override);
            if !allowed {
                return Err(
                    "invalid memory transition; rollback to candidate before revalidation".into(),
                );
            }
        }
        if new_status == "validated" && candidate.status == "rejected" && !known_answer_override {
            return Err("known-answer failure requires explicit override".into());
        }
        tx.execute("INSERT INTO memory(id,version,content,source,evidence,counterevidence,status,confidence,validated_at,supersedes,evidence_run,known_answer_override) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,CASE WHEN ?7='validated' THEN CURRENT_TIMESTAMP ELSE NULL END,?9,?10,?11)",
            params![id,current.version+1,content,current.source_experiment,evidence,current.counterevidence,new_status,current.confidence,current.version,experiment,known_answer_override && new_status=="validated"]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn validated_candidates(&self) -> Result<Vec<Candidate>> {
        self.memories(Some("validated"))?
            .into_iter()
            .map(|m| serde_json::from_str(&m.content).map_err(|e| e.to_string()))
            .collect()
    }
}

impl Store {
    /// Newest relevant eligible memories; exclude source task and identical prompt even under a new id.
    pub fn retrieval(&self, task: &Task) -> Result<Vec<Candidate>> {
        let mut eligible = vec![];
        for memory in self.memories(Some("validated"))? {
            let source = self.run(&memory.source_experiment)?;
            if source.subject_id == task.id
                || source.input.get("prompt").and_then(|v| v.as_str()) == Some(task.prompt.as_str())
            {
                continue;
            }
            let candidate: Candidate =
                serde_json::from_str(&memory.content).map_err(|e| e.to_string())?;
            let relevance = candidate
                .source_concepts
                .iter()
                .filter(|c| task.concepts.contains(c))
                .count();
            eligible.push((relevance, memory.created_at, memory.id, candidate));
        }
        eligible.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| b.1.cmp(&a.1))
                .then_with(|| b.2.cmp(&a.2))
        });
        Ok(eligible.into_iter().take(8).map(|(_, _, _, c)| c).collect())
    }
}
