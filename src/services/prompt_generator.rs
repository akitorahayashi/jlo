use serde::{Deserialize, Serialize};

use crate::domain::{GeneratedPrompt, Layer};

/// Serializable prompt structure for YAML output.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PromptYaml {
    role: String,
    layer: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    assign: Vec<String>,
    prompt: String,
}

/// Service for generating and serializing prompts.
pub struct PromptGenerator;

impl PromptGenerator {
    /// Generate a prompt YAML string for a role.
    pub fn generate_yaml(role_id: &str, layer: Layer, paths: &[String]) -> Result<String, String> {
        let prompt = GeneratedPrompt::new(role_id, layer, paths.to_vec());
        let yaml_struct = PromptYaml {
            role: prompt.role,
            layer: prompt.layer,
            assign: prompt.assign,
            prompt: prompt.prompt,
        };
        serde_yaml::to_string(&yaml_struct)
            .map_err(|e| format!("Failed to serialize prompt: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_serializes_to_yaml() {
        let yaml =
            PromptGenerator::generate_yaml("triage", Layer::Deciders, &["events/".to_string()])
                .expect("should serialize");

        assert!(yaml.contains("role: triage"));
        assert!(yaml.contains("layer: deciders"));
        assert!(yaml.contains("assign:"));
    }

    #[test]
    fn prompt_without_paths_omits_assign() {
        let yaml =
            PromptGenerator::generate_yaml("taxonomy", Layer::Observers, &[]).expect("should work");

        // assign should be empty and skipped
        assert!(!yaml.contains("assign:"));
    }
}
