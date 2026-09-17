//! SQLite persistence: pool, migrations, workflow + execution CRUD.

use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};

use crate::models::{ExecutionRecord, WorkflowRow};

#[derive(Debug, Clone)]
pub struct Db {
    pub pool: SqlitePool,
}

impl Db {
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let options = SqliteConnectOptions::from_str(url)
            .map_err(|e| anyhow::anyhow!("invalid DATABASE_URL: {e}"))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .busy_timeout(std::time::Duration::from_secs(5));

        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(options)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    // ---- workflows ----

    pub async fn list_workflows(&self) -> anyhow::Result<Vec<WorkflowRow>> {
        Ok(sqlx::query_as::<_, WorkflowRow>(
            "SELECT id, name, slug, definition, version, deployed, updated_at
             FROM workflows ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get_workflow(&self, id: &str) -> anyhow::Result<Option<WorkflowRow>> {
        Ok(sqlx::query_as::<_, WorkflowRow>(
            "SELECT id, name, slug, definition, version, deployed, updated_at
             FROM workflows WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn get_workflow_by_slug(&self, slug: &str) -> anyhow::Result<Option<WorkflowRow>> {
        Ok(sqlx::query_as::<_, WorkflowRow>(
            "SELECT id, name, slug, definition, version, deployed, updated_at
             FROM workflows WHERE slug = ?",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn create_workflow(
        &self,
        id: &str,
        name: &str,
        slug: &str,
        definition_json: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO workflows (id, name, slug, definition)
             VALUES (?, ?, ?, ?)",
        )
        .bind(id)
        .bind(name)
        .bind(slug)
        .bind(definition_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_workflow(
        &self,
        id: &str,
        name: &str,
        definition_json: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE workflows SET name = ?, definition = ?, version = version + 1, deployed = 0, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?",
        )
        .bind(name)
        .bind(definition_json)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_deployed(&self, id: &str, deployed: bool) -> anyhow::Result<()> {
        sqlx::query("UPDATE workflows SET deployed = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?")
            .bind(deployed as i64)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Load all deployed workflows (for cache warm-up at startup).
    pub async fn deployed_rows(&self) -> anyhow::Result<Vec<WorkflowRow>> {
        Ok(sqlx::query_as::<_, WorkflowRow>(
            "SELECT id, name, slug, definition, version, deployed, updated_at
             FROM workflows WHERE deployed = 1",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    // ---- executions ----

    pub async fn record_execution(&self, rec: &ExecutionRecord) {
        let _ = sqlx::query(
            "INSERT INTO executions
                (request_id, workflow_id, slug, protocol, status, status_code, latency_ms, bytes_in, bytes_out, started_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&rec.request_id)
        .bind(&rec.workflow_id)
        .bind(&rec.slug)
        .bind(rec.protocol.as_deref())
        .bind(rec.status)
        .bind(rec.status_code)
        .bind(rec.latency_ms)
        .bind(rec.bytes_in as i64)
        .bind(rec.bytes_out as i64)
        .bind(&rec.started_at)
        .execute(&self.pool)
        .await
        .inspect_err(|e| tracing::warn!("failed to record execution: {e}"));
    }
}