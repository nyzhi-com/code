#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPermission {
    ReadOnly,
    NeedsApproval,
}
