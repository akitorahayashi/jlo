use crate::domain::{Layer, RoleId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinRoleEntry {
    pub layer: Layer,
    pub name: RoleId,
    pub category: String,
    pub summary: String,
    pub path: String,
}

impl BuiltinRoleEntry {
    pub fn matches(&self, layer: Layer, role: &RoleId) -> bool {
        self.layer == layer && &self.name == role
    }
}
