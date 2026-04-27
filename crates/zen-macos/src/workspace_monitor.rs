use zen_os::process::{ProcessEvent, ProcessMonitor};

pub struct NsWorkspaceMonitor;

impl NsWorkspaceMonitor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NsWorkspaceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessMonitor for NsWorkspaceMonitor {
    fn subscribe(&self, _callback: Box<dyn Fn(ProcessEvent) + Send + Sync>) {
        // TODO: Register NSWorkspace notification observers
        tracing::info!("macOS workspace monitor: subscribed (stub)");
    }

    fn stop(&self) {
        // TODO: Unregister notification observers
    }
}
