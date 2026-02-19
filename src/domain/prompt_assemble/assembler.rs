use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use minijinja::{Environment, UndefinedBehavior};

use crate::domain::Layer;

use super::error::PromptAssemblyError;
use super::loader::PromptAssetLoader;
use super::types::{AssembledPrompt, PromptContext, SeedOp};

/// The `prompt-assemble://` scheme prefix for embedded catalog resolution.
const PROMPT_ASSEMBLE_SCHEME: &str = "prompt-assemble://";

/// Assemble a prompt for the given layer using the prompt assembly spec.
///
/// `prompt_assemble_reader` resolves paths prefixed with `prompt-assemble://`
/// from the embedded catalog. Unprefixed paths resolve from the repository
/// filesystem via `loader`.
///
/// For multi-role layers (observers, innovators), the context must include
/// the `role` variable. For single-role layers, the context
/// may be empty.
pub fn assemble_prompt<L, R>(
    jules_path: &Path,
    layer: Layer,
    context: &PromptContext,
    loader: &L,
    prompt_assemble_reader: R,
) -> Result<(AssembledPrompt, Vec<SeedOp>), PromptAssemblyError>
where
    L: PromptAssetLoader + Clone + Send + Sync + 'static,
    R: Fn(&str) -> Option<String> + Send + Sync + 'static,
{
    let schemas_dir = crate::domain::layers::paths::schemas_dir(jules_path, layer);
    let root = jules_path.parent().unwrap_or(Path::new("."));

    // Load prompt template from embedded catalog
    let template_path = format!("{}/{}", layer.dir_name(), layer.prompt_template_name());
    let template = prompt_assemble_reader(&template_path)
        .ok_or_else(|| PromptAssemblyError::AssemblyTemplateNotFound(template_path.clone()))?;

    let included_files = Arc::new(Mutex::new(Vec::new()));
    let skipped_files = Arc::new(Mutex::new(Vec::new()));
    let seed_ops = Arc::new(Mutex::new(Vec::new()));
    let failure = Arc::new(Mutex::new(None));

    let include_ctx = Arc::new(IncludeContext {
        root: root.to_path_buf(),
        schemas_dir: schemas_dir.clone(),
        loader: Box::new(loader.clone()),
        prompt_assemble_reader: Box::new(prompt_assemble_reader),
        included_files: included_files.clone(),
        skipped_files: skipped_files.clone(),
        seed_ops: seed_ops.clone(),
        failure: failure.clone(),
    });

    let mut env = Environment::new();
    env.set_keep_trailing_newline(true);
    env.set_undefined_behavior(UndefinedBehavior::Strict);

    {
        let include_ctx = include_ctx.clone();
        env.add_function("include_required", move |path: String| -> String {
            include_ctx.include_file(&path, true).unwrap_or_default()
        });
    }

    {
        let include_ctx = include_ctx.clone();
        env.add_function("include_optional", move |path: String| -> String {
            include_ctx.include_file(&path, false).unwrap_or_default()
        });
    }

    {
        let include_ctx = include_ctx.clone();
        env.add_function("file_exists", move |path: String| -> bool {
            if validate_safe_path(&path).is_err() {
                return false;
            }
            let full_path = include_ctx.root.join(&path);
            include_ctx.loader.asset_exists(&full_path)
        });
    }

    env.add_function("section", |title: String, content: String| -> String {
        if content.trim().is_empty() {
            return String::new();
        }
        format!("---\n# {}\n{}", title, content.trim_end())
    });

    let rendered = env.render_str(&template, &context.variables).map_err(|err| {
        PromptAssemblyError::TemplateRenderError {
            template: template_path.clone(),
            reason: err.to_string(),
        }
    })?;

    if let Some(err) = failure.lock().unwrap().take() {
        return Err(err);
    }

    let prompt = AssembledPrompt {
        content: rendered,
        included_files: included_files.lock().unwrap().clone(),
        skipped_files: skipped_files.lock().unwrap().clone(),
    };
    let ops = seed_ops.lock().unwrap().clone();
    Ok((prompt, ops))
}

/// Validate that a path is safe and does not traverse outside the root.
fn validate_safe_path(path: &str) -> Result<(), PromptAssemblyError> {
    let p = Path::new(path);
    if p.is_absolute() {
        return Err(PromptAssemblyError::PathTraversalDetected { path: path.to_string() });
    }

    for component in p.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir => {}
            _ => {
                return Err(PromptAssemblyError::PathTraversalDetected { path: path.to_string() });
            }
        }
    }
    Ok(())
}

type CatalogReader = Box<dyn Fn(&str) -> Option<String> + Send + Sync>;

struct IncludeContext {
    root: PathBuf,
    schemas_dir: PathBuf,
    loader: Box<dyn PromptAssetLoader + Send + Sync>,
    prompt_assemble_reader: CatalogReader,
    included_files: Arc<Mutex<Vec<String>>>,
    skipped_files: Arc<Mutex<Vec<String>>>,
    seed_ops: Arc<Mutex<Vec<SeedOp>>>,
    failure: Arc<Mutex<Option<PromptAssemblyError>>>,
}

impl IncludeContext {
    fn include_file(&self, path: &str, required: bool) -> Option<String> {
        if self.failure.lock().unwrap().is_some() {
            return None;
        }

        // Resolve prompt-assemble:// scheme from embedded catalog
        if let Some(embedded_path) = path.strip_prefix(PROMPT_ASSEMBLE_SCHEME) {
            return self.include_from_catalog(embedded_path, path, required);
        }

        if let Err(err) = validate_safe_path(path) {
            self.failure.lock().unwrap().replace(err);
            return None;
        }

        let full_path = self.root.join(path);
        let seed_source = self.seed_from_schema(path, &full_path, required);

        if self.failure.lock().unwrap().is_some() {
            return None;
        }

        // If a seed source was found, read content from the schema source so the prompt
        // assembly can proceed; the app layer will execute the SeedOp to persist the copy.
        let read_path = seed_source.as_deref().unwrap_or(&full_path);

        if self.loader.asset_exists(read_path) {
            match self.loader.read_asset(read_path) {
                Ok(content) => {
                    self.included_files.lock().unwrap().push(path.to_string());
                    Some(content)
                }
                Err(err) => {
                    if required {
                        self.failure.lock().unwrap().replace(
                            PromptAssemblyError::IncludeReadError {
                                path: path.to_string(),
                                reason: err.to_string(),
                            },
                        );
                    } else {
                        self.skipped_files
                            .lock()
                            .unwrap()
                            .push(format!("{} (read error: {})", path, err));
                    }
                    None
                }
            }
        } else if required {
            self.failure.lock().unwrap().replace(PromptAssemblyError::RequiredIncludeNotFound {
                path: path.to_string(),
                title: path.to_string(),
            });
            None
        } else {
            self.skipped_files.lock().unwrap().push(format!("{} (not found)", path));
            None
        }
    }

    fn include_from_catalog(
        &self,
        embedded_path: &str,
        original_path: &str,
        required: bool,
    ) -> Option<String> {
        match (self.prompt_assemble_reader)(embedded_path) {
            Some(content) => {
                self.included_files.lock().unwrap().push(original_path.to_string());
                Some(content)
            }
            None => {
                if required {
                    self.failure.lock().unwrap().replace(
                        PromptAssemblyError::RequiredIncludeNotFound {
                            path: original_path.to_string(),
                            title: original_path.to_string(),
                        },
                    );
                } else {
                    self.skipped_files
                        .lock()
                        .unwrap()
                        .push(format!("{} (not found in catalog)", original_path));
                }
                None
            }
        }
    }

    fn seed_from_schema(&self, path: &str, full_path: &Path, required: bool) -> Option<PathBuf> {
        if self.loader.asset_exists(full_path) {
            return None;
        }

        let file_name = Path::new(path).file_name()?;

        let schema_path = self.schemas_dir.join(file_name);
        if !self.loader.asset_exists(&schema_path) {
            return None;
        }

        self.seed_ops.lock().unwrap().push(SeedOp {
            from: schema_path.clone(),
            to: full_path.to_path_buf(),
            required,
        });
        Some(schema_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct MockPromptLoader {
        files: Arc<Mutex<HashMap<String, String>>>,
    }

    impl MockPromptLoader {
        fn new() -> Self {
            Self { files: Arc::new(Mutex::new(HashMap::new())) }
        }

        fn add_file(&self, path: &str, content: &str) {
            self.files.lock().unwrap().insert(path.to_string(), content.to_string());
        }
    }

    impl PromptAssetLoader for MockPromptLoader {
        fn read_asset(&self, path: &Path) -> std::io::Result<String> {
            let path_str = path.to_string_lossy().to_string();
            self.files
                .lock()
                .unwrap()
                .get(&path_str)
                .cloned()
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"))
        }

        fn asset_exists(&self, path: &Path) -> bool {
            let path_str = path.to_string_lossy().to_string();
            self.files.lock().unwrap().contains_key(&path_str)
        }

        fn ensure_asset_dir(&self, _path: &Path) -> std::io::Result<()> {
            Ok(())
        }

        fn copy_asset(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
            let from_str = from.to_string_lossy().to_string();
            let to_str = to.to_string_lossy().to_string();
            let mut files = self.files.lock().unwrap();
            let content = files.get(&from_str).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "Source file not found")
            })?;
            let content_clone = content.clone();
            let len = content_clone.len() as u64;
            files.insert(to_str, content_clone);
            Ok(len)
        }
    }

    /// Build a mock `prompt_assemble_reader` from a `HashMap`.
    fn mock_catalog(
        entries: HashMap<String, String>,
    ) -> impl Fn(&str) -> Option<String> + Send + Sync + 'static {
        move |path: &str| entries.get(path).cloned()
    }

    #[test]
    fn prompt_context_with_var() {
        let ctx = PromptContext::new().with_var("role", "taxonomy");

        assert_eq!(ctx.get("role"), Some("taxonomy"));
        assert_eq!(ctx.get("missing"), None);
    }

    #[test]
    fn test_assemble_prompt_mock_loader() {
        let mock_loader = MockPromptLoader::new();
        let jules_path = Path::new(".jules");

        // Template and contracts live in the embedded catalog
        let mut catalog = HashMap::new();
        catalog.insert(
            "planner/planner_prompt.j2".to_string(),
            r#"{{ section("Contracts", include_required("prompt-assemble://planner/contracts.yml")) }}"#.to_string(),
        );
        catalog.insert(
            "planner/contracts.yml".to_string(),
            "layer: planner\nconstraints: []".to_string(),
        );

        let result = assemble_prompt(
            jules_path,
            Layer::Planner,
            &PromptContext::new(),
            &mock_loader,
            mock_catalog(catalog),
        );

        assert!(result.is_ok());
        let (assembled, _seed_ops) = result.unwrap();
        assert!(assembled.content.contains("# Contracts"));
        assert!(assembled.content.contains("layer: planner"));
    }

    #[test]
    fn test_assemble_prompt_optional_include_skipped() {
        let mock_loader = MockPromptLoader::new();
        let jules_path = Path::new(".jules");

        // Template from catalog; the optional include targets a filesystem path
        let mut catalog = HashMap::new();
        catalog.insert(
            "observers/observers_prompt.j2".to_string(),
            r#"{{ section("Optional", include_optional(".jules/exchange/changes.yml")) }}"#
                .to_string(),
        );

        let ctx = PromptContext::new().with_var("role", "qa");
        let (result, _seed_ops) = assemble_prompt(
            jules_path,
            Layer::Observers,
            &ctx,
            &mock_loader,
            mock_catalog(catalog),
        )
        .unwrap();

        assert!(!result.content.contains("# Optional"));
        assert!(result.skipped_files.iter().any(|entry| entry.contains("changes.yml")));
    }

    #[test]
    fn test_assemble_prompt_missing_required_catalog_include_fails() {
        let mock_loader = MockPromptLoader::new();
        let jules_path = Path::new(".jules");

        // Template references a catalog file that does not exist
        let mut catalog = HashMap::new();
        catalog.insert(
            "planner/planner_prompt.j2".to_string(),
            r#"{{ section("Missing", include_required("prompt-assemble://planner/contracts.yml")) }}"#.to_string(),
        );

        let result = assemble_prompt(
            jules_path,
            Layer::Planner,
            &PromptContext::new(),
            &mock_loader,
            mock_catalog(catalog),
        );

        assert!(matches!(result, Err(PromptAssemblyError::RequiredIncludeNotFound { .. })));
    }

    #[test]
    fn test_assemble_prompt_schema_seed() {
        let mock_loader = MockPromptLoader::new();
        let jules_path = Path::new(".jules");

        // Template from catalog; schema seed from filesystem (.jules/schemas/observers/)
        let mut catalog = HashMap::new();
        catalog.insert(
            "observers/observers_prompt.j2".to_string(),
            r#"{{ section("Event Schema", include_required(".jules/schemas/observers/event.yml")) }}"#
                .to_string(),
        );
        mock_loader.add_file(".jules/schemas/observers/event.yml", "schema: event");

        let ctx = PromptContext::new().with_var("role", "taxonomy");
        let (result, _seed_ops) = assemble_prompt(
            jules_path,
            Layer::Observers,
            &ctx,
            &mock_loader,
            mock_catalog(catalog),
        )
        .unwrap();

        assert!(result.content.contains("schema: event"));
        assert!(result.included_files.iter().any(|path| path.ends_with("event.yml")));
    }
}
