use super::asset_collect::AssetSourceFile;

pub fn should_render_asset(source: &AssetSourceFile) -> bool {
    // Exclude internal documentation from deployed scaffold
    let path = source.relative_path();
    if path == "AGENTS.md" || path.ends_with("/AGENTS.md") || path.ends_with("\\AGENTS.md") {
        return false;
    }

    if !source.is_template() {
        return true;
    }

    !is_partial_template(path)
}

fn is_partial_template(path: &str) -> bool {
    if !path.starts_with("workflows/") {
        return false;
    }

    path.contains("/components/") || path.contains("/macros/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excludes_workflow_components_and_macros() {
        assert!(is_partial_template("workflows/jules-workflows/components/run-narrator.yml.j2"));
        assert!(is_partial_template("workflows/jules-workflows/macros/job_blocks.j2"));
    }

    #[test]
    fn keeps_primary_templates_and_static_files() {
        assert!(!is_partial_template("workflows/jules-workflows.yml.j2"));
        assert!(!is_partial_template("actions/install-jlo/action.yml"));
    }
}
