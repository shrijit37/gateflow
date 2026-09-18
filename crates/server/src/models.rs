//! Control-plane DTOs and DB row models.

use gateflow_engine::WorkflowDag;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ---- DB rows ----

#[derive(Debug, Clone, FromRow)]
pub struct WorkflowRow {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub definition: String,
    pub version: i64,
    pub deployed: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    pub request_id: String,
    pub workflow_id: String,
    pub slug: String,
    pub protocol: Option<String>,
    pub status: &'static str,
    pub status_code: Option<u16>,
    pub latency_ms: Option<i64>,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub started_at: String,
}

// ---- Control-plane DTOs ----

#[derive(Debug, Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub slug: String,
    pub definition: WorkflowDag,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkflowRequest {
    pub name: String,
    pub definition: WorkflowDag,
}

#[derive(Debug, Serialize)]
pub struct WorkflowSummary {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub version: i64,
    pub deployed: bool,
    pub updated_at: String,
}

impl From<&WorkflowRow> for WorkflowSummary {
    fn from(row: &WorkflowRow) -> Self {
        WorkflowSummary {
            id: row.id.clone(),
            name: row.name.clone(),
            slug: row.slug.clone(),
            version: row.version,
            deployed: row.deployed == 1,
            updated_at: row.updated_at.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct WorkflowDetail {
    #[serde(flatten)]
    pub summary: WorkflowSummary,
    pub definition: WorkflowDag,
}

#[derive(Debug, Serialize)]
pub struct DeployResponse {
    pub id: String,
    pub slug: String,
    pub deployed: bool,
    pub version: i64,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
