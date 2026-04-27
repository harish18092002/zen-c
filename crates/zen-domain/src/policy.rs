use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::entities::{
    AppRule, BlockAction, BlockRuleKind, DomainRule, EnforcementPlan, Profile, SessionId,
    Strictness,
};
use crate::errors::DomainError;

pub struct PolicyCompiler;

impl PolicyCompiler {
    pub fn compile(
        session_id: SessionId,
        profile: &Profile,
        revision: i64,
        strictness: Strictness,
    ) -> Result<EnforcementPlan, DomainError> {
        let mut app_rules = Vec::new();
        let mut domain_rules = Vec::new();

        for rule in &profile.block_rules {
            if !rule.enabled {
                continue;
            }
            match rule.kind {
                BlockRuleKind::App => {
                    app_rules.push(AppRule {
                        bundle_id: Some(rule.target.clone()),
                        executable_path: None,
                        action: BlockAction::Block,
                    });
                }
                BlockRuleKind::Domain => {
                    let normalized = normalize_domain(&rule.target)?;
                    domain_rules.push(DomainRule {
                        pattern: rule.target.clone(),
                        normalized,
                        action: BlockAction::Block,
                    });
                }
                // Category expansion and network rules resolved by platform adapters
                BlockRuleKind::Category | BlockRuleKind::Network => {}
            }
        }

        let plan_hash = compute_plan_hash(&app_rules, &domain_rules, &strictness);

        Ok(EnforcementPlan {
            revision,
            session_id,
            app_rules,
            domain_rules,
            network_rules: vec![],
            strictness,
            plan_hash,
        })
    }
}

fn normalize_domain(domain: &str) -> Result<String, DomainError> {
    let lowercased = domain.to_lowercase();
    let trimmed = lowercased.trim_end_matches('.');
    if trimmed.is_empty() {
        return Err(DomainError::PlanCompilationFailed {
            reason: "Domain cannot be empty".to_string(),
        });
    }
    Ok(trimmed.to_string())
}

fn compute_plan_hash(
    app_rules: &[AppRule],
    domain_rules: &[DomainRule],
    strictness: &Strictness,
) -> String {
    let mut hasher = DefaultHasher::new();
    app_rules.len().hash(&mut hasher);
    domain_rules.len().hash(&mut hasher);
    format!("{:?}", strictness).hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{BlockRule, BlockRuleKind, ProfileId};
    use time::OffsetDateTime;
    use uuid::Uuid;

    fn make_profile(rules: Vec<BlockRule>) -> Profile {
        Profile {
            id: ProfileId::new(),
            name: "Test".to_string(),
            revision: 1,
            block_rules: rules,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        }
    }

    #[test]
    fn compiles_domain_rules() {
        let profile_id = ProfileId::new();
        let profile = make_profile(vec![BlockRule {
            id: Uuid::new_v4(),
            profile_id,
            kind: BlockRuleKind::Domain,
            target: "Twitter.com".to_string(),
            enabled: true,
        }]);

        let plan =
            PolicyCompiler::compile(SessionId::new(), &profile, 1, Strictness::Normal).unwrap();

        assert_eq!(plan.domain_rules.len(), 1);
        assert_eq!(plan.domain_rules[0].normalized, "twitter.com");
    }

    #[test]
    fn skips_disabled_rules() {
        let profile_id = ProfileId::new();
        let profile = make_profile(vec![BlockRule {
            id: Uuid::new_v4(),
            profile_id,
            kind: BlockRuleKind::Domain,
            target: "example.com".to_string(),
            enabled: false,
        }]);

        let plan =
            PolicyCompiler::compile(SessionId::new(), &profile, 1, Strictness::Normal).unwrap();

        assert!(plan.domain_rules.is_empty());
    }
}
