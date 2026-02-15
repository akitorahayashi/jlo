use std::collections::BTreeSet;
use std::path::{Component, Path};

use crate::ports::ScaffoldFile;

pub fn collect_action_dirs(files: &[ScaffoldFile]) -> Vec<String> {
    let mut action_dirs = BTreeSet::new();

    for file in files {
        let path = Path::new(&file.path);
        if let Ok(rest) = path.strip_prefix(".github/actions")
            && let Some(Component::Normal(name)) = rest.components().next()
        {
            action_dirs.insert(format!(".github/actions/{}", name.to_string_lossy()));
        }
    }

    action_dirs.into_iter().collect()
}
