use async_trait::async_trait;
use sqlx::SqlitePool;
use zen_domain::{
    entities::{Profile, ProfileId},
    errors::DomainError,
};

use zen_core::ports::ProfileRepository;

pub struct SqliteProfileRepository {
    #[allow(dead_code)]
    pool: SqlitePool,
}

impl SqliteProfileRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProfileRepository for SqliteProfileRepository {
    async fn save(&self, _profile: &Profile) -> Result<(), DomainError> {
        // TODO: Upsert profile and block_rules rows
        Ok(())
    }

    async fn find_by_id(&self, _id: &ProfileId) -> Result<Option<Profile>, DomainError> {
        // TODO: Query profiles + block_rules by profile ID
        Ok(None)
    }

    async fn list_all(&self) -> Result<Vec<Profile>, DomainError> {
        // TODO: Query all profiles with their rules
        Ok(Vec::new())
    }
}
