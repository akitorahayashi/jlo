//! `.jules/` runtime artifact paths.

use std::path::{Path, PathBuf};

use crate::domain::workspace::layer::Layer;

// ── Top-level files ────────────────────────────────────────────────────

/// `.jules/JULES.md`
pub fn readme(root: &Path) -> PathBuf {
    root.join(super::JULES_DIR).join("JULES.md")
}

/// `.jules/README.md`
pub fn project_readme(root: &Path) -> PathBuf {
    root.join(super::JULES_DIR).join("README.md")
}

/// `.jules/.jlo-version`
pub fn version_file(root: &Path) -> PathBuf {
    root.join(super::JULES_DIR).join(super::VERSION_FILE)
}

/// `.jules/github-labels.json`
pub fn github_labels(jules_path: &Path) -> PathBuf {
    jules_path.join("github-labels.json")
}

// ── Roles layer structure ──────────────────────────────────────────────

/// `.jules/roles/`
pub fn roles_dir(jules_path: &Path) -> PathBuf {
    jules_path.join("roles")
}

/// `.jules/roles/<layer>/`
pub fn layer_dir(jules_path: &Path, layer: Layer) -> PathBuf {
    jules_path.join("roles").join(layer.dir_name())
}

/// `.jules/roles/<layer>/<layer>_prompt.j2`
pub fn prompt_template(jules_path: &Path, layer: Layer) -> PathBuf {
    layer_dir(jules_path, layer).join(layer.prompt_template_name())
}

/// `.jules/roles/<layer>/contracts.yml`
pub fn contracts(jules_path: &Path, layer: Layer) -> PathBuf {
    layer_dir(jules_path, layer).join("contracts.yml")
}

/// `.jules/roles/<layer>/contracts_<phase>.yml`
pub fn phase_contracts(jules_path: &Path, layer: Layer, phase: &str) -> PathBuf {
    layer_dir(jules_path, layer).join(format!("contracts_{}.yml", phase))
}

/// `.jules/roles/<layer>/schemas/`
pub fn schemas_dir(jules_path: &Path, layer: Layer) -> PathBuf {
    layer_dir(jules_path, layer).join("schemas")
}

/// `.jules/roles/<layer>/tasks/`
pub fn tasks_dir(jules_path: &Path, layer: Layer) -> PathBuf {
    layer_dir(jules_path, layer).join("tasks")
}

/// `.jules/roles/<layer>/schemas/<filename>`
pub fn schema_file(jules_path: &Path, layer: Layer, filename: &str) -> PathBuf {
    schemas_dir(jules_path, layer).join(filename)
}

/// `.jules/roles/<layer>/roles/` (multi-role container)
pub fn layer_roles_container(jules_path: &Path, layer: Layer) -> PathBuf {
    layer_dir(jules_path, layer).join("roles")
}

/// `.jules/roles/narrator/schemas/changes.yml`
pub fn narrator_change_schema(jules_path: &Path) -> PathBuf {
    schema_file(jules_path, Layer::Narrators, "changes.yml")
}

// ── Narrator output ────────────────────────────────────────────────────

/// `.jules/exchange/changes.yml`
pub fn exchange_changes(jules_path: &Path) -> PathBuf {
    exchange_dir(jules_path).join("changes.yml")
}

// ── Exchange ───────────────────────────────────────────────────────────

/// `.jules/exchange/`
pub fn exchange_dir(jules_path: &Path) -> PathBuf {
    jules_path.join("exchange")
}

/// `.jules/exchange/events/`
pub fn events_dir(jules_path: &Path) -> PathBuf {
    exchange_dir(jules_path).join("events")
}

/// `.jules/exchange/events/<state>/`
pub fn events_state_dir(jules_path: &Path, state: &str) -> PathBuf {
    events_dir(jules_path).join(state)
}

/// `.jules/exchange/events/pending/`
pub fn events_pending_dir(jules_path: &Path) -> PathBuf {
    events_state_dir(jules_path, "pending")
}

/// `.jules/exchange/events/decided/`
pub fn events_decided_dir(jules_path: &Path) -> PathBuf {
    events_state_dir(jules_path, "decided")
}

/// `.jules/exchange/issues/`
pub fn issues_dir(jules_path: &Path) -> PathBuf {
    exchange_dir(jules_path).join("issues")
}

/// `.jules/exchange/issues/<label>/`
pub fn issues_label_dir(jules_path: &Path, label: &str) -> PathBuf {
    issues_dir(jules_path).join(label)
}

/// `.jules/exchange/innovators/`
pub fn innovators_dir(jules_path: &Path) -> PathBuf {
    exchange_dir(jules_path).join("innovators")
}

/// `.jules/exchange/innovators/<persona>/`
pub fn innovator_persona_dir(jules_path: &Path, persona: &str) -> PathBuf {
    innovators_dir(jules_path).join(persona)
}

/// `.jules/exchange/innovators/<persona>/perspective.yml`
pub fn innovator_perspective(jules_path: &Path, persona: &str) -> PathBuf {
    innovator_persona_dir(jules_path, persona).join("perspective.yml")
}

/// `.jules/exchange/innovators/<persona>/idea.yml`
pub fn innovator_idea(jules_path: &Path, persona: &str) -> PathBuf {
    innovator_persona_dir(jules_path, persona).join("idea.yml")
}

/// `.jules/exchange/innovators/<persona>/proposal.yml`
pub fn innovator_proposal(jules_path: &Path, persona: &str) -> PathBuf {
    innovator_persona_dir(jules_path, persona).join("proposal.yml")
}

/// `.jules/exchange/innovators/<persona>/comments/`
pub fn innovator_comments_dir(jules_path: &Path, persona: &str) -> PathBuf {
    innovator_persona_dir(jules_path, persona).join("comments")
}

// ── Workstations ───────────────────────────────────────────────────────

/// `.jules/workstations/`
pub fn workstations_dir(jules_path: &Path) -> PathBuf {
    jules_path.join("workstations")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_level_files() {
        let root = Path::new("/ws");
        assert_eq!(readme(root), PathBuf::from("/ws/.jules/JULES.md"));
        assert_eq!(project_readme(root), PathBuf::from("/ws/.jules/README.md"));
        assert_eq!(version_file(root), PathBuf::from("/ws/.jules/.jlo-version"));
    }

    #[test]
    fn layer_structure() {
        let jp = Path::new("/ws/.jules");
        assert_eq!(layer_dir(jp, Layer::Narrators), PathBuf::from("/ws/.jules/roles/narrator"));
        assert_eq!(
            prompt_template(jp, Layer::Observers),
            PathBuf::from("/ws/.jules/roles/observers/observers_prompt.j2")
        );
        assert_eq!(
            phase_contracts(jp, Layer::Innovators, "creation"),
            PathBuf::from("/ws/.jules/roles/innovators/contracts_creation.yml")
        );
        assert_eq!(
            tasks_dir(jp, Layer::Observers),
            PathBuf::from("/ws/.jules/roles/observers/tasks")
        );
    }

    #[test]
    fn exchange_tree() {
        let jp = Path::new("/ws/.jules");
        assert_eq!(exchange_dir(jp), PathBuf::from("/ws/.jules/exchange"));
        assert_eq!(events_pending_dir(jp), PathBuf::from("/ws/.jules/exchange/events/pending"));
        assert_eq!(events_decided_dir(jp), PathBuf::from("/ws/.jules/exchange/events/decided"));
        assert_eq!(issues_label_dir(jp, "bugs"), PathBuf::from("/ws/.jules/exchange/issues/bugs"));
    }

    #[test]
    fn innovator_tree() {
        let jp = Path::new("/ws/.jules");
        assert_eq!(
            innovator_persona_dir(jp, "arch"),
            PathBuf::from("/ws/.jules/exchange/innovators/arch")
        );
        assert_eq!(
            innovator_idea(jp, "arch"),
            PathBuf::from("/ws/.jules/exchange/innovators/arch/idea.yml")
        );
        assert_eq!(
            innovator_comments_dir(jp, "arch"),
            PathBuf::from("/ws/.jules/exchange/innovators/arch/comments")
        );
    }

    #[test]
    fn narrator_paths() {
        let jp = Path::new("/ws/.jules");
        assert_eq!(exchange_changes(jp), PathBuf::from("/ws/.jules/exchange/changes.yml"));
        assert_eq!(
            narrator_change_schema(jp),
            PathBuf::from("/ws/.jules/roles/narrator/schemas/changes.yml")
        );
    }

    #[test]
    fn relative_constructors() {
        assert_eq!(github_labels(Path::new(".jules")), PathBuf::from(".jules/github-labels.json"));
    }
}
