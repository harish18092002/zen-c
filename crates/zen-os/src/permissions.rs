use zen_domain::entities::PermissionStatus;

pub trait PermissionProbe: Send + Sync {
    fn name(&self) -> &str;
    fn check(&self) -> PermissionStatus;
}
