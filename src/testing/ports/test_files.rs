//! Shared in-memory file backing store for port-scoped test doubles.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// In-memory file storage shared across port-scoped test doubles.
///
/// Provides the underlying file map that `MockRepositoryFs`, `MockJloStore`,
/// and `MockJulesStore` operate on. Tests seed files via this handle before
/// passing the port doubles to production code.
#[derive(Clone, Debug)]
pub struct TestFiles {
    pub(crate) files: Arc<Mutex<HashMap<String, String>>>,
}

impl Default for TestFiles {
    fn default() -> Self {
        Self { files: Arc::new(Mutex::new(HashMap::new())) }
    }
}

impl TestFiles {
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed a file into the in-memory store.
    #[allow(dead_code)]
    pub fn add(&self, path: &str, content: &str) {
        self.files.lock().unwrap().insert(path.to_string(), content.to_string());
    }
}
