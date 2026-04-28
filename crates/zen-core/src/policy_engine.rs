use std::sync::Arc;

use zen_domain::{
    entities::{EnforcementPlan, ProfileId, SessionId, Strictness},
    errors::DomainError,
    policy::PolicyCompiler,
};

use crate::ports::ProfileRepository;

pub struct PolicyEngine {
    profiles: Arc<dyn ProfileRepository>,
}

impl PolicyEngine {
    pub fn new(profiles: Arc<dyn ProfileRepository>) -> Self {
        Self { profiles }
    }

    pub async fn compile_for_session(
        &self,
        profile_id: &ProfileId,
        session_id: SessionId,
        revision: i64,
        strictness: Strictness,
    ) -> Result<EnforcementPlan, DomainError> {
        let profile =
            self.profiles
                .find_by_id(profile_id)
                .await?
                .ok_or_else(|| DomainError::NotFound {
                    entity: "Profile".to_string(),
                    id: format!("{:?}", profile_id),
                })?;

        PolicyCompiler::compile(session_id, &profile, revision, strictness)
    }
}
