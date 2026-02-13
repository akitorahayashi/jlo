use crate::domain::{AppError, Component};
use std::collections::BTreeMap;

/// Split setup environment artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupEnvArtifacts {
    pub vars_toml: String,
    pub secrets_toml: String,
}

/// Generate or merge vars.toml and secrets.toml content.
pub fn merge(
    components: &[Component],
    existing_vars_toml: Option<&str>,
    existing_secrets_toml: Option<&str>,
) -> Result<SetupEnvArtifacts, AppError> {
    // Load existing values
    let existing_vars = if let Some(content) = existing_vars_toml {
        parse_env_toml(content)?
    } else {
        BTreeMap::new()
    };

    let existing_secrets = if let Some(content) = existing_secrets_toml {
        parse_env_toml(content)?
    } else {
        BTreeMap::new()
    };

    // Collect all env specs from components.
    // First occurrence wins to keep deterministic behavior when keys are duplicated.
    let mut all_env: BTreeMap<String, (String, Option<String>, bool)> = BTreeMap::new();
    for component in components {
        for env_spec in &component.env {
            if !all_env.contains_key(&env_spec.name) {
                all_env.insert(
                    env_spec.name.clone(),
                    (env_spec.description.clone(), env_spec.default.clone(), env_spec.secret),
                );
            }
        }
    }

    let vars_toml = build_env_toml(
        "# Non-secret environment configuration for jlo setup",
        &all_env,
        false,
        &existing_vars,
        &existing_secrets,
    )?;
    let secrets_toml = build_env_toml(
        "# Secret environment configuration for jlo setup",
        &all_env,
        true,
        &existing_secrets,
        &existing_vars,
    )?;

    Ok(SetupEnvArtifacts { vars_toml, secrets_toml })
}

fn build_env_toml(
    header: &str,
    all_env: &BTreeMap<String, (String, Option<String>, bool)>,
    include_secret: bool,
    existing_primary: &BTreeMap<String, BTreeMap<String, String>>,
    existing_secondary: &BTreeMap<String, BTreeMap<String, String>>,
) -> Result<String, AppError> {
    let mut lines = vec![
        header.to_string(),
        "# Edit values as needed before running install.sh".to_string(),
        String::new(),
    ];

    for (name, (description, default, secret)) in all_env {
        if *secret != include_secret {
            continue;
        }

        lines.push(format!("[{}]", name));

        let existing_table = existing_primary.get(name).or_else(|| existing_secondary.get(name));

        let value = if let Some(table) = existing_table {
            table.get("value").cloned().unwrap_or_else(|| default.clone().unwrap_or_default())
        } else {
            default.clone().unwrap_or_default()
        };
        let value_str =
            serde_json::to_string(&value).map_err(|e| AppError::MalformedEnvToml(e.to_string()))?;
        lines.push(format!("value = {}", value_str));

        let note = if let Some(table) = existing_table {
            table.get("note").cloned().unwrap_or_else(|| description.clone())
        } else {
            description.clone()
        };
        if !note.is_empty() {
            let note_str = serde_json::to_string(&note)
                .map_err(|e| AppError::MalformedEnvToml(e.to_string()))?;
            lines.push(format!("note = {}", note_str));
        }

        lines.push(String::new());
    }

    Ok(lines.join("\n"))
}

/// Parse vars.toml/secrets.toml content into table name -> key/value pairs.
fn parse_env_toml(content: &str) -> Result<BTreeMap<String, BTreeMap<String, String>>, AppError> {
    let data: toml::Value =
        toml::from_str(content).map_err(|e| AppError::MalformedEnvToml(e.to_string()))?;

    let mut result: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();

    if let toml::Value::Table(table) = data {
        for (key, value) in table {
            if let toml::Value::Table(inner) = value {
                let mut inner_map = BTreeMap::new();
                for (k, v) in inner {
                    if let toml::Value::String(s) = v {
                        inner_map.insert(k, s);
                    }
                }
                result.insert(key, inner_map);
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ComponentId, EnvSpec};

    fn make_component(name: &str, env: Vec<EnvSpec>) -> Component {
        Component {
            name: ComponentId::new(name).unwrap(),
            summary: format!("{} component", name),
            dependencies: vec![],
            env,
            script_content: format!("echo {}", name),
        }
    }

    #[test]
    fn merge_env_artifacts_creates_new() {
        let components = vec![make_component(
            "test",
            vec![
                EnvSpec {
                    name: "TEST_VAR".to_string(),
                    description: "A test variable".to_string(),
                    default: Some("default_value".to_string()),
                    secret: false,
                },
                EnvSpec {
                    name: "TEST_SECRET".to_string(),
                    description: "A test secret".to_string(),
                    default: Some("secret_value".to_string()),
                    secret: true,
                },
            ],
        )];

        let result = merge(&components, None, None).unwrap();

        assert!(result.vars_toml.contains("[TEST_VAR]"));
        assert!(result.vars_toml.contains("value = \"default_value\""));
        assert!(result.vars_toml.contains("note = \"A test variable\""));
        assert!(!result.vars_toml.contains("TEST_SECRET"));

        assert!(result.secrets_toml.contains("[TEST_SECRET]"));
        assert!(result.secrets_toml.contains("value = \"secret_value\""));
        assert!(result.secrets_toml.contains("note = \"A test secret\""));
        assert!(!result.secrets_toml.contains("TEST_VAR"));
    }

    #[test]
    fn merge_env_artifacts_preserves_existing() {
        let existing_vars = r#"
[TEST_VAR]
value = "custom_value"
note = "Custom note"
"#;

        let existing_secrets = r#"
[TEST_SECRET]
value = "custom_secret"
note = "Custom secret note"
"#;

        let components = vec![make_component(
            "test",
            vec![
                EnvSpec {
                    name: "TEST_VAR".to_string(),
                    description: "A test variable".to_string(),
                    default: Some("default_value".to_string()),
                    secret: false,
                },
                EnvSpec {
                    name: "TEST_SECRET".to_string(),
                    description: "A test secret".to_string(),
                    default: Some("default_secret".to_string()),
                    secret: true,
                },
            ],
        )];

        let result = merge(&components, Some(existing_vars), Some(existing_secrets)).unwrap();

        assert!(
            result.vars_toml.contains("value = \"custom_value\""),
            "should preserve existing non-secret value"
        );
        assert!(
            result.vars_toml.contains("note = \"Custom note\""),
            "should preserve existing non-secret note"
        );
        assert!(
            result.secrets_toml.contains("value = \"custom_secret\""),
            "should preserve existing secret value"
        );
        assert!(
            result.secrets_toml.contains("note = \"Custom secret note\""),
            "should preserve existing secret note"
        );
    }

    #[test]
    fn merge_env_artifacts_migrates_value_when_secret_classification_changes() {
        let existing_vars = r#"
[GH_TOKEN]
value = "from-vars"
note = "legacy location"
"#;

        let components = vec![make_component(
            "gh",
            vec![EnvSpec {
                name: "GH_TOKEN".to_string(),
                description: "Token for gh CLI authentication".to_string(),
                default: None,
                secret: true,
            }],
        )];

        let result = merge(&components, Some(existing_vars), None).unwrap();

        assert!(result.secrets_toml.contains("value = \"from-vars\""));
        assert!(result.secrets_toml.contains("note = \"legacy location\""));
    }
}
