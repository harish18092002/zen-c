use zen_domain::errors::DomainError;

pub fn map_sqlx(err: sqlx::Error) -> DomainError {
    DomainError::Persistence {
        reason: format!("sqlite: {err}"),
    }
}

pub fn map_serde(err: serde_json::Error) -> DomainError {
    DomainError::Persistence {
        reason: format!("serde_json: {err}"),
    }
}

pub fn map_time_format(err: time::error::Format) -> DomainError {
    DomainError::Persistence {
        reason: format!("time-format: {err}"),
    }
}

pub fn map_time_parse(err: time::error::Parse) -> DomainError {
    DomainError::Persistence {
        reason: format!("time-parse: {err}"),
    }
}

pub fn map_uuid(err: uuid::Error) -> DomainError {
    DomainError::Persistence {
        reason: format!("uuid: {err}"),
    }
}
