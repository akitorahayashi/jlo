use super::Layer;

/// A structured prompt ready for clipboard transport (domain type without serde).
#[derive(Debug, Clone)]
pub struct GeneratedPrompt {
    /// Role identifier.
    pub role: String,
    /// Layer this role belongs to.
    pub layer: String,
    /// Assigned context paths (user-provided).
    pub assign: Vec<String>,
    /// The prompt instruction text.
    pub prompt: String,
}

impl GeneratedPrompt {
    /// Create a new generated prompt.
    pub fn new(role_id: impl Into<String>, layer: Layer, paths: Vec<String>) -> Self {
        let role_id = role_id.into();
        let layer_name = layer.dir_name().to_string();
        let prompt = Self::generate_prompt_text(&role_id, layer);

        Self { role: role_id, layer: layer_name, assign: paths, prompt }
    }

    /// Generate the prompt instruction text.
    fn generate_prompt_text(role_id: &str, layer: Layer) -> String {
        let layer_name = layer.dir_name();

        format!(
            r#"あなたは {role_id} です（{layer} レイヤー）。
必ず以下を読み、最新の制約に従って行動してください。
- JULES.md
- .jules/JULES.md
- .jules/roles/{layer}/{role_id}/role.yml

{layer_behavior}"#,
            role_id = role_id,
            layer = layer_name,
            layer_behavior = Self::layer_specific_behavior(layer)
        )
    }

    /// Get layer-specific behavior instructions.
    fn layer_specific_behavior(layer: Layer) -> &'static str {
        match layer {
            Layer::Observers => {
                "役割に沿って観測し、events/ にYAMLで記録し、notes/ を宣言的に更新する。\n\
                 issues/ には書き込まない。プロダクトコードは編集しない。"
            }
            Layer::Deciders => {
                "events/ を批判的に評価し、採用/却下を判断する。\n\
                 採用した観測を issues/ にMarkdownで作成する。\n\
                 処理済みの events/ は削除する。プロダクトコードは編集しない。"
            }
            Layer::Planners => {
                "issues/ を読み込み、具体的なタスクに分解する。\n\
                 tasks/ に検証計画を含むタスクファイルを作成する。\n\
                 処理済みの issues/ は削除する。プロダクトコードは編集しない。"
            }
            Layer::Implementers => {
                "tasks/ を読み込み、コードを実装する。\n\
                 検証を実行し、成功したら該当 tasks/ を削除する。\n\
                 レポートファイルは生成しない。成果はコード変更のみ。"
            }
        }
    }
}
