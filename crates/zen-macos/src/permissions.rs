use zen_domain::entities::PermissionStatus;
use zen_os::permissions::PermissionProbe;

pub struct AccessibilityPermission;

impl PermissionProbe for AccessibilityPermission {
    fn name(&self) -> &str {
        "Accessibility"
    }

    fn check(&self) -> PermissionStatus {
        // TODO: Call AXIsProcessTrusted() via objc / core-foundation bindings
        PermissionStatus::Unavailable
    }
}
