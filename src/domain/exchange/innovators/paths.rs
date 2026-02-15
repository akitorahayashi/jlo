use std::path::{Path, PathBuf};

use crate::domain::exchange;

/// `.jules/exchange/innovators/`
pub fn innovators_dir(jules_path: &Path) -> PathBuf {
    exchange::paths::exchange_dir(jules_path).join("innovators")
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
