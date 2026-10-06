use zen_domain::entities::PermissionStatus;
use zen_os::permissions::PermissionProbe;

pub struct AccessibilityPermission;

impl PermissionProbe for AccessibilityPermission {
    fn name(&self) -> &str {
        "Accessibility"
    }

    fn check(&self) -> PermissionStatus {
        if accessibility_is_trusted() {
            PermissionStatus::Healthy
        } else {
            PermissionStatus::PermissionDenied
        }
    }
}

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
}

#[cfg(target_os = "macos")]
fn accessibility_is_trusted() -> bool {
    // Safety: parameterless FFI to a documented thread-safe predicate.
    unsafe { AXIsProcessTrusted() != 0 }
}

#[cfg(not(target_os = "macos"))]
fn accessibility_is_trusted() -> bool {
    false
}
