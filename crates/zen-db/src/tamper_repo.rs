use sqlx::SqlitePool;
use zen_domain::{errors::DomainError, events::TamperKind};

use crate::error::map_sqlx;

pub struct SqliteTamperRepository {
    pool: SqlitePool,
}

impl SqliteTamperRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn record(
        &self,
        kind: &TamperKind,
        detail: Option<&str>,
        at_rfc3339: &str,
    ) -> Result<(), DomainError> {
        insert_tamper_event(&self.pool, kind, detail, at_rfc3339).await
    }

    pub async fn list_recent(&self, limit: i64) -> Result<Vec<TamperRecord>, DomainError> {
        use sqlx::Row;
        let rows = sqlx::query(
            "SELECT event_type, detail, detected_at FROM tamper_events
             ORDER BY detected_at DESC LIMIT ?1",
        )
        .bind(limit.max(1))
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;

        rows.iter()
            .map(|row| {
                Ok(TamperRecord {
                    event_type: row.try_get("event_type").map_err(map_sqlx)?,
                    detail: row.try_get("detail").map_err(map_sqlx)?,
                    detected_at: row.try_get("detected_at").map_err(map_sqlx)?,
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct TamperRecord {
    pub event_type: String,
    pub detail: Option<String>,
    pub detected_at: String,
}

pub(crate) async fn insert_tamper_event(
    pool: &SqlitePool,
    kind: &TamperKind,
    detail: Option<&str>,
    at_rfc3339: &str,
) -> Result<(), DomainError> {
    let kind_str = kind.discriminant();
    let combined_detail = match (kind, detail) {
        (TamperKind::PermissionRevoked { permission }, Some(extra)) => {
            Some(format!("{permission}: {extra}"))
        }
        (TamperKind::PermissionRevoked { permission }, None) => Some(permission.clone()),
        (_, Some(extra)) => Some(extra.to_string()),
        (_, None) => None,
    };

    sqlx::query("INSERT INTO tamper_events (event_type, detail, detected_at) VALUES (?1, ?2, ?3)")
        .bind(kind_str)
        .bind(&combined_detail)
        .bind(at_rfc3339)
        .execute(pool)
        .await
        .map_err(map_sqlx)?;

    Ok(())
}
