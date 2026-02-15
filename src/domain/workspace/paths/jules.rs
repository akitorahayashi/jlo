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

/// `.jules/layers/`
pub fn layers_dir(jules_path: &Path) -> PathBuf {
    jules_path.join(super::LAYERS_DIR)
}

/// `.jules/layers/<layer>/`
pub fn layer_dir(jules_path: &Path, layer: Layer) -> PathBuf {
    jules_path.join(super::LAYERS_DIR).join(layer.dir_name())
}

/// `.jules/layers/<layer>/<layer>_prompt.j2`
pub fn prompt_template(jules_path: &Path, layer: Layer) -> PathBuf {
    layer_dir(jules_path, layer).join(layer.prompt_template_name())
}

/// `.jules/layers/<layer>/contracts.yml`
pub fn contracts(jules_path: &Path, layer: Layer) -> PathBuf {
    layer_dir(jules_path, layer).join("contracts.yml")
}

/// `.jules/layers/<layer>/schemas/`
pub fn schemas_dir(jules_path: &Path, layer: Layer) -> PathBuf {
    layer_dir(jules_path, layer).join("schemas")
}

/// `.jules/layers/<layer>/tasks/`
pub fn tasks_dir(jules_path: &Path, layer: Layer) -> PathBuf {
    layer_dir(jules_path, layer).join("tasks")
}

/// `.jules/layers/<layer>/schemas/<filename>`
pub fn schema_file(jules_path: &Path, layer: Layer, filename: &str) -> PathBuf {
    schemas_dir(jules_path, layer).join(filename)
}

/// `.jules/layers/<layer>/roles/` (multi-role container)
pub fn layer_roles_container(jules_path: &Path, layer: Layer) -> PathBuf {
    layer_dir(jules_path, layer).join("roles")
}

/// `.jules/layers/narrator/schemas/changes.yml`
pub fn narrator_change_schema(jules_path: &Path) -> PathBuf {
    schema_file(jules_path, Layer::Narrator, "changes.yml")
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

/// `.jules/exchange/requirements/`
pub fn requirements_dir(jules_path: &Path) -> PathBuf {
    exchange_dir(jules_path).join("requirements")
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
        assert_eq!(layer_dir(jp, Layer::Narrator), PathBuf::from("/ws/.jules/layers/narrator"));
        assert_eq!(
            prompt_template(jp, Layer::Observers),
            PathBuf::from("/ws/.jules/layers/observers/observers_prompt.j2")
        );
        assert_eq!(
            contracts(jp, Layer::Innovators),
            PathBuf::from("/ws/.jules/layers/innovators/contracts.yml")
        );
        assert_eq!(
            tasks_dir(jp, Layer::Observers),
            PathBuf::from("/ws/.jules/layers/observers/tasks")
        );
    }

    #[test]
    fn exchange_tree() {
        let jp = Path::new("/ws/.jules");
        assert_eq!(exchange_dir(jp), PathBuf::from("/ws/.jules/exchange"));
        assert_eq!(events_pending_dir(jp), PathBuf::from("/ws/.jules/exchange/events/pending"));
        assert_eq!(events_decided_dir(jp), PathBuf::from("/ws/.jules/exchange/events/decided"));
        assert_eq!(requirements_dir(jp), PathBuf::from("/ws/.jules/exchange/requirements"));
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
            PathBuf::from("/ws/.jules/layers/narrator/schemas/changes.yml")
        );
    }

    #[test]
    fn relative_constructors() {
        assert_eq!(github_labels(Path::new(".jules")), PathBuf::from(".jules/github-labels.json"));
    }
}
