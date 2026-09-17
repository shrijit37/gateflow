//! Control-plane: workflow CRUD + deploy. JSON over HTTP, no hot path here.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use tracing::{debug, warn};

use crate::models::{
    CreateWorkflowRequest, DeployResponse, ErrorResponse, UpdateWorkflowRequest, WorkflowDetail,
    WorkflowSummary,
};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn list_workflows(State(state): State<AppState>) -> Response {
    match state.db.list_workflows().await {
        Ok(rows) => Json(
            rows.iter()
                .map(WorkflowSummary::from)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(e) => server_error(e.to_string()),
    }
}

pub async fn get_workflow(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.db.get_workflow(&id).await {
        Ok(Some(row)) => {
            match serde_json::from_str(&row.definition) {
                Ok(def) => Json(WorkflowDetail {
                    summary: WorkflowSummary::from(&row),
                    definition: def,
                })
                .into_response(),
                Err(e) => server_error(format!("corrupt definition: {e}")),
            }
        }
        Ok(None) => not_found(&id),
        Err(e) => server_error(e.to_string()),
    }
}

pub async fn create_workflow(
    State(state): State<AppState>,
    Json(req): Json<CreateWorkflowRequest>,
) -> Response {
    let name = req.name.trim().to_string();
    let slug = req.slug.trim().to_string();
    if name.is_empty() {
        return bad_request("name is required");
    }
    if slug.is_empty() || !is_valid_slug(&slug) {
        return bad_request("slug must be lowercase alphanumeric, '-' or '_' only");
    }

    // Lightweight structural validation up front (no node factory needed).
    if let Err(e) = gateflow_engine::dag::topological_order(&req.definition) {
        return bad_request(&format!("invalid DAG: {e}"));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let definition_json = match serde_json::to_string(&req.definition) {
        Ok(s) => s,
        Err(e) => return server_error(format!("serialize: {e}")),
    };

    match state.db.create_workflow(&id, &name, &slug, &definition_json).await {
        Ok(()) => {
            debug!(target: "gateflow::control", id, slug, "workflow created");
            (StatusCode::CREATED, Json(created_detail(&id, &name, &slug, &req.definition, 1, false)))
                .into_response()
        }
        Err(e) => {
            if e.to_string().contains("UNIQUE") {
                bad_request("slug already exists")
            } else {
                server_error(e.to_string())
            }
        }
    }
}

pub async fn update_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateWorkflowRequest>,
) -> Response {
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return bad_request("name is required");
    }

    // Updating invalidates the deployed cache; it must be redeployed.
    let row = match state.db.get_workflow(&id).await {
        Ok(Some(r)) => r,
        Ok(None) => return not_found(&id),
        Err(e) => return server_error(e.to_string()),
    };

    let slug = row.slug.clone();
    let definition_json = match serde_json::to_string(&req.definition) {
        Ok(s) => s,
        Err(e) => return server_error(format!("serialize: {e}")),
    };

    if let Err(e) = state.db.update_workflow(&id, &name, &definition_json).await {
        return server_error(e.to_string());
    }
    state.invalidate(&slug).await;

    let version = row.version + 1;
    debug!(target: "gateflow::control", id, slug, version, "workflow updated");
    Json(created_detail(&id, &name, &slug, &req.definition, version, false)).into_response()
}

pub async fn deploy_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let row = match state.db.get_workflow(&id).await {
        Ok(Some(r)) => r,
        Ok(None) => return not_found(&id),
        Err(e) => return server_error(e.to_string()),
    };

    match state.deploy(&row).await {
        Ok(deployed) => {
            let _ = deployed;
            if let Err(e) = state.db.mark_deployed(&id, true).await {
                warn!(target: "gateflow::control", error = %e, "failed to mark deployed");
            }
            debug!(target: "gateflow::control", id, slug = %row.slug, "workflow deployed");
            Json(DeployResponse {
                id: row.id.clone(),
                slug: row.slug.clone(),
                deployed: true,
                version: row.version,
                errors: vec![],
            })
            .into_response()
        }
        Err(errors) => {
            warn!(target: "gateflow::control", id, %errors, "deploy failed");
            Json(DeployResponse {
                id: row.id.clone(),
                slug: row.slug.clone(),
                deployed: false,
                version: row.version,
                errors: vec![errors],
            })
            .into_response()
        }
    }
}

pub async fn healthz(State(state): State<AppState>) -> Response {
    let db_ok = sqlx::query("SELECT 1").execute(&state.db.pool).await.is_ok();
    Json(serde_json::json!({
        "status": if db_ok { "ok" } else { "degraded" },
        "deployed_workflows": state.cache.read().await.len(),
        "version": env!("CARGO_PKG_VERSION"),
    }))
    .into_response()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 64
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

fn created_detail(
    id: &str,
    name: &str,
    slug: &str,
    definition: &gateflow_engine::WorkflowDag,
    version: i64,
    deployed: bool,
) -> WorkflowDetail {
    WorkflowDetail {
        summary: WorkflowSummary {
            id: id.to_string(),
            name: name.to_string(),
            slug: slug.to_string(),
            version,
            deployed,
            updated_at: String::new(),
        },
        definition: definition.clone(),
    }
}

fn not_found(id: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: format!("workflow {id} not found"),
        }),
    )
        .into_response()
}

fn bad_request(msg: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: msg.to_string(),
        }),
    )
        .into_response()
}

fn server_error(msg: String) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse { error: msg }),
    )
        .into_response()
}