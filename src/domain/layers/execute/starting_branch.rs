use crate::domain::{ControlPlaneConfig, Layer};

pub fn resolve_starting_branch(
    layer: Layer,
    config: &ControlPlaneConfig,
    branch_override: Option<&str>,
) -> String {
    branch_override.map(String::from).unwrap_or_else(|| {
        if layer.uses_worker_branch() {
            config.run.jules_worker_branch.clone()
        } else {
            config.run.jlo_target_branch.clone()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::resolve_starting_branch;
    use crate::domain::{ControlPlaneConfig, Layer};

    fn test_config() -> ControlPlaneConfig {
        let mut config = ControlPlaneConfig::default();
        config.run.jules_worker_branch = "worker-branch".to_string();
        config.run.jlo_target_branch = "main".to_string();
        config
    }

    #[test]
    fn defaults_to_worker_branch_for_worker_layer() {
        let config = test_config();
        let branch = resolve_starting_branch(Layer::Narrator, &config, None);
        assert_eq!(branch, "worker-branch");
    }

    #[test]
    fn defaults_to_target_branch_for_target_layer() {
        let config = test_config();
        let branch = resolve_starting_branch(Layer::Implementer, &config, None);
        assert_eq!(branch, "main");
    }

    #[test]
    fn override_takes_precedence() {
        let config = test_config();
        let branch = resolve_starting_branch(Layer::Integrator, &config, Some("feature/custom"));
        assert_eq!(branch, "feature/custom");
    }
}
