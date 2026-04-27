#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub executable_path: Option<String>,
    pub bundle_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ProcessEvent {
    Launched(ProcessInfo),
    Terminated { pid: u32 },
    Focused(ProcessInfo),
}

pub trait ProcessMonitor: Send + Sync {
    fn subscribe(&self, callback: Box<dyn Fn(ProcessEvent) + Send + Sync>);
    fn stop(&self);
}
