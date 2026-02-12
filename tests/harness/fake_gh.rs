use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tempfile::TempDir;

pub struct FakeGh {
    pub root: TempDir,
    pub bin_dir: PathBuf,
    pub log_file: PathBuf,
}

impl FakeGh {
    pub fn new() -> Self {
        let root = TempDir::new().expect("Failed to create temp dir for fake gh");
        let bin_dir = root.path().join("bin");
        fs::create_dir_all(&bin_dir).expect("Failed to create bin dir");
        let log_file = root.path().join("gh.log");

        let gh_script_path = bin_dir.join("gh");

        // We use single quotes for echo to avoid shell expansion issues,
        // but we need to insert the log file path which is safe.
        let script_content = format!(
            r#"#!/bin/sh
echo "$@" >> "{}"

# Basic argument parsing to simulate responses
# We flatten args to a single string for easier matching in case/esac
ARGS="$*"

case "$1" in
    pr)
        if [ "$2" = "create" ]; then
            echo "https://github.com/owner/repo/pull/123"
        elif [ "$2" = "view" ]; then
            echo '{{"number":123,"headRefName":"head","baseRefName":"base","isDraft":false,"autoMergeRequest":null}}'
        elif [ "$2" = "diff" ]; then
            echo "file1.rs"
            echo "file2.rs"
        fi
        ;;
    issue)
        if [ "$2" = "create" ]; then
            echo "https://github.com/owner/repo/issues/456"
        fi
        ;;
    label)
        if [ "$2" = "list" ]; then
            echo '[{{"name":"bug"}},{{"name":"feature"}},{{"name":"bugs"}}]'
        fi
        ;;
    api)
        # Default empty array for list endpoints
        echo '[]'
        ;;
esac

exit 0
"#,
            log_file.to_string_lossy()
        );

        fs::write(&gh_script_path, script_content).expect("Failed to write gh script");

        let mut perms =
            fs::metadata(&gh_script_path).expect("Failed to get metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&gh_script_path, perms).expect("Failed to set permissions");

        Self { root, bin_dir, log_file }
    }

    pub fn get_log(&self) -> String {
        fs::read_to_string(&self.log_file).unwrap_or_default()
    }
}
