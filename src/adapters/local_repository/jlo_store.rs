//! `JloStore` implementation for `LocalRepositoryAdapter`.

use std::fs;
use std::path::PathBuf;

use crate::domain::repository::paths::jlo;
use crate::domain::{AppError, JLO_DIR, Layer, RoleId};
use crate::ports::{DiscoveredRole, JloStore, RepositoryFilesystem};

use super::LocalRepositoryAdapter;

impl JloStore for LocalRepositoryAdapter {
    fn jlo_exists(&self) -> bool {
        self.jlo_path().exists()
    }

    fn jlo_path(&self) -> PathBuf {
        self.root.join(JLO_DIR)
    }

    fn jlo_write_version(&self, version: &str) -> Result<(), AppError> {
        let path = format!("{}/{}", JLO_DIR, crate::domain::VERSION_FILE);
        self.write_file(&path, &format!("{}\n", version))
    }

    fn jlo_read_version(&self) -> Result<Option<String>, AppError> {
        let path = format!("{}/{}", JLO_DIR, crate::domain::VERSION_FILE);
        if !self.file_exists(&path) {
            return Ok(None);
        }
        let content = self.read_file(&path)?;
        Ok(Some(content.trim().to_string()))
    }

    fn discover_roles(&self) -> Result<Vec<DiscoveredRole>, AppError> {
        let mut roles = Vec::new();

        for layer in Layer::ALL {
            if layer.is_single_role() {
                continue;
            }
            let layer_dir = jlo::layer_dir(&self.root, layer);
            if !layer_dir.exists() {
                continue;
            }

            for entry in fs::read_dir(&layer_dir)? {
                let entry = entry?;
                if !entry.path().is_dir() {
                    continue;
                }
                let role_id_str = entry.file_name().to_string_lossy().to_string();
                if let Ok(role_id) = RoleId::new(&role_id_str)
                    && entry.path().join("role.yml").exists()
                {
                    roles.push(DiscoveredRole { layer, id: role_id });
                }
            }
        }

        roles.sort_by(|a, b| {
            let layer_cmp = a.layer.dir_name().cmp(b.layer.dir_name());
            if layer_cmp == std::cmp::Ordering::Equal { a.id.cmp(&b.id) } else { layer_cmp }
        });

        Ok(roles)
    }

    fn find_role_fuzzy(&self, query: &str) -> Result<Option<DiscoveredRole>, AppError> {
        let roles = self.discover_roles()?;

        // Exact match first
        if let Some(role) = roles.iter().find(|r| r.id.as_str() == query) {
            return Ok(Some(role.clone()));
        }

        // layer/role format
        if let Some((layer_part, role_part)) = query.split_once('/')
            && let Some(layer) = Layer::from_dir_name(layer_part)
            && let Some(role) =
                roles.iter().find(|r| r.layer == layer && r.id.as_str() == role_part)
        {
            return Ok(Some(role.clone()));
        }

        // Prefix match
        let matches: Vec<_> = roles.iter().filter(|r| r.id.as_str().starts_with(query)).collect();

        match matches.len() {
            1 => Ok(Some(matches[0].clone())),
            _ => Ok(None),
        }
    }

    fn role_path(&self, role: &DiscoveredRole) -> Option<PathBuf> {
        let path = jlo::role_dir(&self.root, role.layer, role.id.as_str());
        if path.exists() { Some(path) } else { None }
    }

    fn write_role(&self, layer: Layer, role_id: &str, content: &str) -> Result<(), AppError> {
        let path = jlo::role_yml(&self.root, layer, role_id);
        let rel = path.strip_prefix(&self.root).unwrap_or(&path);
        self.write_file(&rel.to_string_lossy(), content)
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::test_store;
    use crate::domain::Layer;
    use crate::ports::{JloStore, RepositoryFilesystem};

    #[test]
    fn discover_roles_finds_and_sorts() {
        let (_dir, store) = test_store();

        // Create minimal .jlo/roles structure
        let obs_path = ".jlo/roles/observers/taxonomy/role.yml";
        store.write_file(obs_path, "role: taxonomy\nlayer: observers").unwrap();

        let inn_path = ".jlo/roles/innovators/screener/role.yml";
        store.write_file(inn_path, "role: screener\nlayer: innovators").unwrap();

        let roles = store.discover_roles().unwrap();
        assert_eq!(roles.len(), 2);
        assert_eq!(roles[0].layer, Layer::Innovators);
        assert_eq!(roles[0].id.as_str(), "screener");
        assert_eq!(roles[1].layer, Layer::Observers);
        assert_eq!(roles[1].id.as_str(), "taxonomy");
    }

    #[test]
    fn find_role_fuzzy_matches() {
        let (_dir, store) = test_store();

        store.write_file(".jlo/roles/observers/taxonomy/role.yml", "role: taxonomy").unwrap();
        store.write_file(".jlo/roles/innovators/taxman/role.yml", "role: taxman").unwrap();

        // Exact
        let found = store.find_role_fuzzy("taxonomy").unwrap().unwrap();
        assert_eq!(found.layer, Layer::Observers);

        // layer/role
        let found = store.find_role_fuzzy("innovators/taxman").unwrap().unwrap();
        assert_eq!(found.layer, Layer::Innovators);

        // Prefix (unique)
        let found = store.find_role_fuzzy("taxo").unwrap().unwrap();
        assert_eq!(found.id.as_str(), "taxonomy");

        // Prefix (ambiguous)
        assert!(store.find_role_fuzzy("tax").unwrap().is_none());

        // No match
        assert!(store.find_role_fuzzy("nonexistent").unwrap().is_none());
    }

    #[test]
    fn version_roundtrip() {
        let (_dir, store) = test_store();

        // Need .jlo/ to exist for version ops
        store.create_dir_all(".jlo").unwrap();

        store.jlo_write_version("1.2.3").unwrap();
        assert_eq!(store.jlo_read_version().unwrap(), Some("1.2.3".to_string()));
    }
}
