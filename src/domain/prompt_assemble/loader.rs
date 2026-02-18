use std::path::Path;

/// Abstraction for prompt asset loading.
pub trait PromptAssetLoader {
    fn read_asset(&self, path: &Path) -> std::io::Result<String>;
    fn asset_exists(&self, path: &Path) -> bool;
    fn ensure_asset_dir(&self, path: &Path) -> std::io::Result<()>;
    fn copy_asset(&self, from: &Path, to: &Path) -> std::io::Result<u64>;
}
