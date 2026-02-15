use std::collections::HashSet;

use include_dir::{Dir, DirEntry, include_dir};
use serde::Deserialize;

use crate::domain::{AppError, BuiltinRoleEntry, Layer, RoleId};

static BUILTIN_ROLES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/assets/roles");

#[derive(Debug, Deserialize)]
struct RoleYaml {
    role: String,
    layer: String,
    description: String,
}

pub fn load_builtin_role_catalog() -> Result<Vec<BuiltinRoleEntry>, AppError> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    let mut role_files = Vec::new();
    collect_role_files(&BUILTIN_ROLES_DIR, &mut role_files);

    for path in role_files {
        let role_yaml = read_role_yaml(&path)?;
        let (layer, category, role_dir) = parse_role_path(&path)?;

        let layer_enum = Layer::from_dir_name(&layer).ok_or_else(|| {
            AppError::AssetError(format!("Invalid builtin role layer '{}'", layer))
        })?;
        let role_id = RoleId::new(&role_yaml.role).map_err(|_| {
            AppError::AssetError(format!("Invalid builtin role name '{}'", role_yaml.role))
        })?;

        if role_yaml.layer != layer_enum.dir_name() {
            return Err(AppError::AssetError(format!(
                "Builtin role '{}' has mismatched layer '{}' in role.yml",
                role_yaml.role, role_yaml.layer
            )));
        }

        if role_yaml.role != role_dir {
            return Err(AppError::AssetError(format!(
                "Builtin role '{}' path does not match role.yml",
                role_yaml.role
            )));
        }

        if role_yaml.description.trim().is_empty() {
            return Err(AppError::AssetError(format!(
                "Builtin role '{}' has empty description",
                role_yaml.role
            )));
        }

        let key = format!("{}:{}", layer_enum.dir_name(), role_id.as_str());
        if !seen.insert(key) {
            return Err(AppError::AssetError(format!(
                "Duplicate builtin role entry '{}'",
                role_yaml.role
            )));
        }

        out.push(BuiltinRoleEntry {
            layer: layer_enum,
            name: role_id,
            category,
            summary: role_yaml.description,
            path,
        });
    }

    Ok(out)
}

pub fn read_builtin_role_file(path: &str) -> Result<String, AppError> {
    let file = BUILTIN_ROLES_DIR
        .get_file(path)
        .ok_or_else(|| AppError::AssetError(format!("Missing builtin role asset: {path}")))?;

    file.contents_utf8().map(str::to_string).ok_or_else(|| {
        AppError::AssetError(format!("Builtin role asset is not valid UTF-8: {path}"))
    })
}

fn read_role_yaml(path: &str) -> Result<RoleYaml, AppError> {
    let content = read_builtin_role_file(path)?;
    serde_yaml::from_str(&content).map_err(|err| {
        AppError::AssetError(format!("Failed to parse builtin role {}: {}", path, err))
    })
}

fn collect_role_files(dir: &'static Dir<'static>, paths: &mut Vec<String>) {
    for entry in dir.entries() {
        match entry {
            DirEntry::File(file) => {
                let path = file.path().to_string_lossy().to_string();
                if path.ends_with("/role.yml") {
                    paths.push(path);
                }
            }
            DirEntry::Dir(subdir) => collect_role_files(subdir, paths),
        }
    }
}

fn parse_role_path(path: &str) -> Result<(String, String, String), AppError> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() != 4 || parts[3] != "role.yml" {
        return Err(AppError::AssetError(format!(
            "Builtin role path must be <layer>/<category>/<role>/role.yml: {}",
            path
        )));
    }

    Ok((parts[0].to_string(), parts[1].to_string(), parts[2].to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_builtin_role_catalog() {
        let catalog = load_builtin_role_catalog().expect("catalog should load");
        assert!(catalog.iter().any(|entry| entry.name.as_str() == "taxonomy"));
        assert!(catalog.iter().any(|entry| entry.name.as_str() == "recruiter"));
    }

    #[test]
    fn reads_builtin_role_file() {
        let content = read_builtin_role_file("observers/language/taxonomy/role.yml").unwrap();
        assert!(content.contains("role: taxonomy"));
    }
}
