use crate::types::{Candidate, Result, Run};
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
}
impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
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
        if version != 1 {
            return Err("unsupported SQLite schema version".into());
        }
        Ok(Self { connection })
    }
    pub fn record(&mut self, run: &Run, provider: &str) -> Result<()> {
        let payload = serde_json::to_string(run).map_err(|e| e.to_string())?;
        crate::privacy::screen(&payload)?;
        let tx = self.connection.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO runs VALUES(?1,?2,?3,?4)",
            params![run.id, run.status, provider, payload],
        )
        .map_err(|e| e.to_string())?;
        if run.status == "completed" {
            for candidate in &run.candidates {
                tx.execute("INSERT INTO memory(id,version,content,source,evidence,counterevidence,status,confidence) VALUES(?1,1,?2,?3,?4,?5,'candidate',?6)",
                    params![candidate.id, serde_json::to_string(candidate).map_err(|e| e.to_string())?, run.id,
                        serde_json::to_string(&candidate.evidence).map_err(|e| e.to_string())?,
                        serde_json::to_string(&candidate.counterevidence).map_err(|e| e.to_string())?, candidate.uncertainty]).map_err(|e| e.to_string())?;
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
        let mut stmt = self.connection.prepare("SELECT m.id,m.content,m.source,m.evidence,m.counterevidence,m.status,m.confidence,m.created_at,m.validated_at,m.version,m.supersedes FROM memory m WHERE m.version=(SELECT MAX(version) FROM memory WHERE id=m.id) AND (?1 IS NULL OR m.status=?1) ORDER BY m.id").map_err(|e| e.to_string())?;
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
        if !["validated", "rejected", "retired", "candidate"].contains(&status)
            || evidence.trim().is_empty()
        {
            return Err("status and explicit evidence required".into());
        }
        let run = self.run(experiment)?;
        crate::privacy::screen(evidence)?;
        if run.status != "completed" || !run.reset_verified {
            return Err("admission requires completed experiment".into());
        }
        let current = self
            .memories(None)?
            .into_iter()
            .find(|m| m.id == id)
            .ok_or("memory not found")?;
        if current.source_experiment != experiment {
            return Err("experiment does not match memory provenance".into());
        }
        let tx = self.connection.transaction().map_err(|e| e.to_string())?;
        let old_version = rollback.unwrap_or(current.version);
        // Rollback copies a prior revision into a new version; history is never overwritten.
        let (content, old_status): (String, String) = tx
            .query_row(
                "SELECT content,status FROM memory WHERE id=?1 AND version=?2",
                params![id, old_version],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
        let new_status = if rollback.is_some() {
            old_status.as_str()
        } else {
            status
        };
        tx.execute("INSERT INTO memory(id,version,content,source,evidence,counterevidence,status,confidence,validated_at,supersedes) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,CASE WHEN ?7='validated' THEN CURRENT_TIMESTAMP ELSE NULL END,?9)",
            params![id,current.version+1,content,experiment,evidence,current.counterevidence,new_status,current.confidence,current.version]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn validated_candidates(&self) -> Result<Vec<Candidate>> {
        self.memories(Some("validated"))?
            .into_iter()
            .take(8)
            .map(|m| serde_json::from_str(&m.content).map_err(|e| e.to_string()))
            .collect()
    }
}
