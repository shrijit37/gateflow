//! Shared application state.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use gateflow_engine::Executable;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::db::Db;
use crate::node_registry::{IngressConfig, Registry};

/// A compiled, deployable workflow as held in the hot-path cache.
#[derive(Clone)]
pub struct Deployed {
    pub exe: Executable,
    pub ingress: IngressMeta,
}

#[derive(Clone)]
pub struct IngressMeta {
    pub method: http::Method,
    pub any_method: bool,
    pub protocol_hint: Option<String>,
    pub timeout: Duration,
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Db,
    pub registry: Arc<Registry>,
    pub cache: Arc<RwLock<HashMap<String, Arc<Deployed>>>>,
}

impl AppState {
    pub fn new(config: Config, db: Db, registry: Registry) -> Self {
        Self {
            config: Arc::new(config),
            db,
            registry: Arc::new(registry),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Compile a definition and insert it into the hot-path cache.
    pub async fn deploy(&self, row: &crate::models::WorkflowRow) -> Result<Arc<Deployed>, String> {
        let dag: gateflow_engine::WorkflowDag = serde_json::from_str(&row.definition)
            .map_err(|e| format!("corrupt definition JSON: {e}"))?;
        let exe = gateflow_engine::compile(&row.id, &row.slug, &dag, self.registry.as_ref())
            .map_err(|e| e.to_string())?;

        // Ingress metadata for routing + budget, read from the first node.
        let ingress_def = dag
            .nodes
            .iter()
            .find(|n| n.kind == gateflow_engine::NodeKind::Ingress)
            .ok_or_else(|| "workflow has no ingress node".to_string())?;
        let ingress_cfg: IngressConfig = serde_json::from_value(ingress_def.config.clone())
            .map_err(|e| format!("ingress config: {e}"))?;

        let method = if ingress_cfg.method.eq_ignore_ascii_case("ANY") {
            http::Method::GET
        } else {
            http::Method::from_bytes(ingress_cfg.method.to_uppercase().as_bytes())
                .map_err(|e| format!("ingress method: {e}"))?
        };

        let any_method = ingress_cfg.method.eq_ignore_ascii_case("ANY");
        let timeout = Duration::from_millis(
            ingress_cfg
                .timeout_ms
                .unwrap_or(self.config.default_timeout.as_millis() as u64),
        );
        let protocol_hint = if ingress_cfg.protocol.eq_ignore_ascii_case("auto") {
            None
        } else {
            Some(ingress_cfg.protocol.to_ascii_lowercase())
        };

        let deployed = Arc::new(Deployed {
            exe,
            ingress: IngressMeta {
                method,
                any_method,
                protocol_hint,
                timeout,
            },
        });

        self.cache.write().await.insert(row.slug.clone(), deployed.clone());
        Ok(deployed)
    }

    pub async fn invalidate(&self, slug: &str) {
        self.cache.write().await.remove(slug);
    }
}