use std::collections::HashMap;
use std::str::FromStr;

use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;
use zen_core::ports::ProfileRepository;
use zen_domain::{
    entities::{BlockRule, BlockRuleKind, Profile, ProfileId},
    errors::DomainError,
};

use crate::error::{map_sqlx, map_time_format, map_time_parse, map_uuid};

pub struct SqliteProfileRepository {
    pool: SqlitePool,
}

impl SqliteProfileRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn delete(&self, id: &ProfileId) -> Result<bool, DomainError> {
        let id_str = id.0.to_string();
        let result = sqlx::query("DELETE FROM profiles WHERE id = ?1")
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?;
        Ok(result.rows_affected() > 0)
    }
}

#[async_trait]
impl ProfileRepository for SqliteProfileRepository {
    async fn save(&self, profile: &Profile) -> Result<(), DomainError> {
        let id_str = profile.id.0.to_string();
        let created_at = profile
            .created_at
            .format(&Rfc3339)
            .map_err(map_time_format)?;
        let updated_at = profile
            .updated_at
            .format(&Rfc3339)
            .map_err(map_time_format)?;

        let mut tx = self.pool.begin().await.map_err(map_sqlx)?;

        sqlx::query(
            "INSERT INTO profiles (id, name, revision, data, created_at, updated_at)
             VALUES (?1, ?2, ?3, '{}', ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                name       = excluded.name,
                revision   = excluded.revision,
                updated_at = excluded.updated_at",
        )
        .bind(&id_str)
        .bind(&profile.name)
        .bind(profile.revision)
        .bind(&created_at)
        .bind(&updated_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;

        sqlx::query("DELETE FROM block_rules WHERE profile_id = ?1")
            .bind(&id_str)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;

        for rule in &profile.block_rules {
            let rule_id = rule.id.to_string();
            sqlx::query(
                "INSERT INTO block_rules (id, profile_id, kind, target, enabled)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .bind(&rule_id)
            .bind(&id_str)
            .bind(rule.kind.as_str())
            .bind(&rule.target)
            .bind(rule.enabled as i64)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
        }

        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn find_by_id(&self, id: &ProfileId) -> Result<Option<Profile>, DomainError> {
        let id_str = id.0.to_string();
        let row_opt = sqlx::query(
            "SELECT id, name, revision, created_at, updated_at
             FROM profiles WHERE id = ?1",
        )
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;

        let Some(row) = row_opt else {
            return Ok(None);
        };

        let rules = load_rules_for_profile(&self.pool, &id_str).await?;
        let profile = row_to_profile(&row, rules)?;
        Ok(Some(profile))
    }

    async fn list_all(&self) -> Result<Vec<Profile>, DomainError> {
        let profile_rows = sqlx::query(
            "SELECT id, name, revision, created_at, updated_at
             FROM profiles ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;

        if profile_rows.is_empty() {
            return Ok(Vec::new());
        }

        let rule_rows = sqlx::query(
            "SELECT id, profile_id, kind, target, enabled FROM block_rules ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;

        let mut by_profile: HashMap<String, Vec<BlockRule>> = HashMap::new();
        for row in &rule_rows {
            let profile_id_str: String = row.try_get("profile_id").map_err(map_sqlx)?;
            let parsed = row_to_block_rule(row, &profile_id_str)?;
            by_profile.entry(profile_id_str).or_default().push(parsed);
        }

        let mut out = Vec::with_capacity(profile_rows.len());
        for row in profile_rows.iter() {
            let id_str: String = row.try_get("id").map_err(map_sqlx)?;
            let rules = by_profile.remove(&id_str).unwrap_or_default();
            out.push(row_to_profile(row, rules)?);
        }

        Ok(out)
    }
}

async fn load_rules_for_profile(
    pool: &SqlitePool,
    profile_id_str: &str,
) -> Result<Vec<BlockRule>, DomainError> {
    let rows = sqlx::query(
        "SELECT id, profile_id, kind, target, enabled FROM block_rules
         WHERE profile_id = ?1 ORDER BY created_at",
    )
    .bind(profile_id_str)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx)?;

    rows.iter()
        .map(|row| row_to_block_rule(row, profile_id_str))
        .collect()
}

fn row_to_profile(
    row: &sqlx::sqlite::SqliteRow,
    rules: Vec<BlockRule>,
) -> Result<Profile, DomainError> {
    let id_str: String = row.try_get("id").map_err(map_sqlx)?;
    let id = Uuid::from_str(&id_str).map_err(map_uuid)?;
    let name: String = row.try_get("name").map_err(map_sqlx)?;
    let revision: i64 = row.try_get("revision").map_err(map_sqlx)?;
    let created_at_str: String = row.try_get("created_at").map_err(map_sqlx)?;
    let updated_at_str: String = row.try_get("updated_at").map_err(map_sqlx)?;

    Ok(Profile {
        id: ProfileId(id),
        name,
        revision,
        block_rules: rules,
        created_at: parse_datetime(&created_at_str)?,
        updated_at: parse_datetime(&updated_at_str)?,
    })
}

fn row_to_block_rule(
    row: &sqlx::sqlite::SqliteRow,
    profile_id_str: &str,
) -> Result<BlockRule, DomainError> {
    let id_str: String = row.try_get("id").map_err(map_sqlx)?;
    let kind_str: String = row.try_get("kind").map_err(map_sqlx)?;
    let target: String = row.try_get("target").map_err(map_sqlx)?;
    let enabled: i64 = row.try_get("enabled").map_err(map_sqlx)?;

    let kind = BlockRuleKind::parse(&kind_str).ok_or_else(|| DomainError::Persistence {
        reason: format!("unknown block_rule kind: {kind_str}"),
    })?;
    let profile_uuid = Uuid::from_str(profile_id_str).map_err(map_uuid)?;
    let id = Uuid::from_str(&id_str).map_err(map_uuid)?;

    Ok(BlockRule {
        id,
        profile_id: ProfileId(profile_uuid),
        kind,
        target,
        enabled: enabled != 0,
    })
}

/// Accept either RFC3339 timestamps (what we write) or SQLite's
/// `datetime('now')` default format (e.g. `2026-05-12 10:30:00`) used by the
/// schema's DEFAULT clauses.
fn parse_datetime(s: &str) -> Result<OffsetDateTime, DomainError> {
    if let Ok(t) = OffsetDateTime::parse(s, &Rfc3339) {
        return Ok(t);
    }
    let fmt = time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    let pdt = time::PrimitiveDateTime::parse(s, &fmt).map_err(map_time_parse)?;
    Ok(pdt.assume_utc())
}
