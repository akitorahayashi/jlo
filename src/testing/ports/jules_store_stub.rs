use std::sync::{Arc, Mutex};

#[derive(Clone)]
#[allow(dead_code)]
pub struct MockJulesStore {
    pub exists: Arc<Mutex<bool>>,
    pub version: Arc<Mutex<Option<String>>>,
    pub created_structure: Arc<Mutex<bool>>,
}

impl Default for MockJulesStore {
    fn default() -> Self {
        Self {
            exists: Arc::new(Mutex::new(false)),
            version: Arc::new(Mutex::new(None)),
            created_structure: Arc::new(Mutex::new(false)),
        }
    }
}

#[allow(dead_code)]
impl MockJulesStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_exists(&self, exists: bool) {
        *self.exists.lock().expect("jules exists lock poisoned") = exists;
    }

    pub fn set_version(&self, version: &str) {
        *self.version.lock().expect("jules version lock poisoned") = Some(version.to_string());
    }

    pub fn mark_structure_created(&self) {
        *self.created_structure.lock().expect("jules created_structure lock poisoned") = true;
    }
}
