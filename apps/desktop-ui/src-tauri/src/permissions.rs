use zen_domain::entities::PermissionStatus;
use zen_os::permissions::PermissionProbe;

use crate::dto::{permission_to_dto, PermissionDto};

/// Aggregate every platform permission probe into the wire DTO list. Calls
/// each probe synchronously — they are expected to be cheap OS lookups.
pub fn probe_all() -> Vec<PermissionDto> {
    let mut out = Vec::new();

    #[cfg(target_os = "macos")]
    {
        let probe = zen_macos::permissions::AccessibilityPermission;
        let status = probe.check();
        out.push(permission_to_dto(
            probe.name(),
            status.clone(),
            describe(&status),
        ));
    }

    #[cfg(target_os = "windows")]
    {
        let probe = zen_windows::permissions::ServicePermission;
        let status = probe.check();
        out.push(permission_to_dto(
            probe.name(),
            status.clone(),
            describe(&status),
        ));
    }

    if out.is_empty() {
        out.push(permission_to_dto(
            "Platform",
            PermissionStatus::Unavailable,
            Some("no platform permission probes registered".into()),
        ));
    }

    out
}

fn describe(status: &PermissionStatus) -> Option<String> {
    match status {
        PermissionStatus::Healthy => None,
        PermissionStatus::Degraded => Some("limited functionality".into()),
        PermissionStatus::Unavailable => Some("not yet implemented for this platform".into()),
        PermissionStatus::PermissionDenied => {
            Some("the user denied this permission — request via system settings".into())
        }
    }
}
