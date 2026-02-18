//! Embedded catalog for prompt-assemble assets (templates, contracts, tasks).
//!
//! These assets are resolved at runtime from the binary. They are never
//! deployed to `.jules/`.

use include_dir::{Dir, include_dir};

static PROMPT_ASSEMBLE_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/assets/prompt-assemble");

/// Read a prompt-assemble asset by its relative path.
///
/// `path` is relative to `src/assets/prompt-assemble/`
/// (e.g. `"decider/contracts.yml"`, `"principle.yml"`).
pub fn read_prompt_assemble_asset(path: &str) -> Option<String> {
    PROMPT_ASSEMBLE_DIR.get_file(path).and_then(|file| file.contents_utf8()).map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use include_dir::DirEntry;

    #[test]
    fn read_known_asset_returns_content() {
        let content = read_prompt_assemble_asset("principle.yml");
        assert!(content.is_some(), "principle.yml should exist");
        let text = content.unwrap();
        assert!(text.contains("coding:"), "principle.yml should contain coding section");
    }

    #[test]
    fn read_unknown_asset_returns_none() {
        assert!(read_prompt_assemble_asset("nonexistent.yml").is_none());
    }

    #[test]
    fn all_prompt_assemble_files_are_non_empty() {
        fn check_entry(entry: &DirEntry) {
            match entry {
                DirEntry::File(file) => {
                    let path = file.path().to_string_lossy();
                    assert!(!file.contents().is_empty(), "Prompt-assemble file {} is empty", path);
                }
                DirEntry::Dir(dir) => {
                    for entry in dir.entries() {
                        check_entry(entry);
                    }
                }
            }
        }

        assert!(
            !PROMPT_ASSEMBLE_DIR.entries().is_empty(),
            "Prompt-assemble directory should not be empty"
        );
        for entry in PROMPT_ASSEMBLE_DIR.entries() {
            check_entry(entry);
        }
    }

    #[test]
    fn each_layer_has_contracts_and_template() {
        for layer in crate::domain::Layer::ALL {
            let name = layer.dir_name();
            assert!(
                read_prompt_assemble_asset(&format!("{}/contracts.yml", name)).is_some(),
                "Missing contracts.yml for {}",
                name
            );
            let has_template = PROMPT_ASSEMBLE_DIR
                .get_dir(name)
                .map(|dir| dir.entries().iter().any(|e| {
                    matches!(e, DirEntry::File(f) if f.path().to_string_lossy().ends_with("_prompt.j2"))
                }))
                .unwrap_or(false);
            assert!(has_template, "Missing prompt template for {}", name);
        }
    }
}
