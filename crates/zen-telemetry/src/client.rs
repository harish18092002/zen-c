use crate::events::TelemetryEvent;

pub struct TelemetryClient {
    enabled: bool,
}

impl TelemetryClient {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn disabled() -> Self {
        Self { enabled: false }
    }

    pub fn track(&self, event: TelemetryEvent) {
        if !self.enabled {
            return;
        }
        tracing::info!(telemetry = true, event = ?event, "telemetry");
        // TODO: Batch and submit to backend with consent gating
    }
}
