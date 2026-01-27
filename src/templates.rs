//! Template loading for dynamic prompt generation.

#[allow(dead_code)]
use crate::layers::Layer;

/// Layer-specific templates.
#[allow(dead_code)]
pub mod layer_templates {
    pub static OBSERVER: &str = include_str!("templates/layers/observer.yml");
    pub static DECIDER: &str = include_str!("templates/layers/decider.yml");
    pub static PLANNER: &str = include_str!("templates/layers/planner.yml");
    pub static IMPLEMENTER: &str = include_str!("templates/layers/implementer.yml");
}

/// Get the template for a specific layer.
#[allow(dead_code)]
pub fn get_layer_template(layer: Layer) -> &'static str {
    match layer {
        Layer::Observers => layer_templates::OBSERVER,
        Layer::Deciders => layer_templates::DECIDER,
        Layer::Planners => layer_templates::PLANNER,
        Layer::Implementers => layer_templates::IMPLEMENTER,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_layer_templates_exist() {
        for layer in Layer::ALL {
            let template = get_layer_template(layer);
            assert!(!template.is_empty(), "Template for {:?} should not be empty", layer);
        }
    }

    #[test]
    fn layer_templates_contain_layer_key() {
        for layer in Layer::ALL {
            let template = get_layer_template(layer);
            assert!(
                template.contains("layer:"),
                "Template for {:?} should contain 'layer:'",
                layer
            );
        }
    }
}
