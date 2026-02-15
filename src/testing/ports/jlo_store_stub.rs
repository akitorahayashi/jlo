use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::domain::{Layer, RoleId};

#[derive(Clone)]
#[allow(dead_code)]
pub struct MockJloStore {
    pub exists: Arc<Mutex<bool>>,
    pub roles: Arc<Mutex<HashMap<(Layer, RoleId), bool>>>,
    pub version: Arc<Mutex<Option<String>>>,
}

impl Default for MockJloStore {
    fn default() -> Self {
        Self {
            exists: Arc::new(Mutex::new(false)),
            roles: Arc::new(Mutex::new(HashMap::new())),
            version: Arc::new(Mutex::new(None)),
        }
    }
}

#[allow(dead_code)]
impl MockJloStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_exists(&self, exists: bool) {
        *self.exists.lock().expect("jlo exists lock poisoned") = exists;
    }

    pub fn set_version(&self, version: &str) {
        *self.version.lock().expect("jlo version lock poisoned") = Some(version.to_string());
    }

    pub fn add_role(&self, layer: Layer, role_id: &str) {
        let id = RoleId::new(role_id).expect("invalid role id in test setup");
        self.roles.lock().expect("jlo roles lock poisoned").insert((layer, id), true);
    }
}
