use zen_domain::entities::PermissionStatus;
use zen_os::permissions::PermissionProbe;

pub struct ServicePermission;

impl PermissionProbe for ServicePermission {
    fn name(&self) -> &str {
        "WindowsService"
    }

    fn check(&self) -> PermissionStatus {
        // TODO: Check Windows service status via SCM
        PermissionStatus::Unavailable
    }
}
