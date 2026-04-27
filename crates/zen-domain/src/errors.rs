use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Invalid state transition from {from} to {to}")]
    InvalidTransition { from: String, to: String },

    #[error("Entity not found: {entity} with id {id}")]
    NotFound { entity: String, id: String },

    #[error("Profile is immutable: cannot edit a profile in use by an active session")]
    ProfileImmutable,

    #[error("Enforcement plan compilation failed: {reason}")]
    PlanCompilationFailed { reason: String },
}
