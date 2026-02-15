//! Setup tools configuration model and parser (`.jlo/setup/tools.yml`).

use serde::Deserialize;

use crate::domain::AppError;

/// Configuration for setup artifact generation.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SetupConfig {
    /// List of tool names to install.
    #[serde(default)]
    pub tools: Vec<String>,
}

/// Parse and validate setup tools configuration content.
pub fn parse_tools_config_content(content: &str) -> Result<SetupConfig, AppError> {
    let config: SetupConfig = serde_yaml::from_str(content)
        .map_err(|e| AppError::ParseError { what: "tools.yml".into(), details: e.to_string() })?;

    if config.tools.is_empty() {
        return Err(AppError::Validation(
            "No tools specified in tools.yml. Add tools to the 'tools' list.".into(),
        ));
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rejects_empty_tools() {
        let err = parse_tools_config_content("tools: []").unwrap_err();
        assert!(matches!(err, AppError::Validation(msg) if msg.contains("No tools specified")));
    }

    #[test]
    fn parse_accepts_tools() {
        let cfg = parse_tools_config_content("tools:\n  - just\n").unwrap();
        assert_eq!(cfg.tools, vec!["just".to_string()]);
    }
}
