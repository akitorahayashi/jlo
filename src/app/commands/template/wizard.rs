use dialoguer::Select;

use crate::app::AppContext;
use crate::domain::{AppError, Layer};
use crate::ports::{ClipboardWriter, RoleTemplateStore, WorkspaceStore};

use super::command::create_role;
use super::outcome::TemplateOutcome;
use super::workstream::create_workstream;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TemplateChoice {
    Workstream,
    ObserverRole,
    DeciderRole,
}

impl TemplateChoice {
    const ALL: [TemplateChoice; 3] =
        [TemplateChoice::Workstream, TemplateChoice::ObserverRole, TemplateChoice::DeciderRole];

    fn label(self) -> &'static str {
        match self {
            TemplateChoice::Workstream => "Workstream",
            TemplateChoice::ObserverRole => "Observer Role",
            TemplateChoice::DeciderRole => "Decider Role",
        }
    }
}

pub(super) fn run_template_wizard<W, R, C>(
    ctx: &AppContext<W, R, C>,
) -> Result<TemplateOutcome, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
    C: ClipboardWriter,
{
    let items: Vec<&str> = TemplateChoice::ALL.iter().map(|choice| choice.label()).collect();

    let selection = Select::new()
        .with_prompt("Select template type to apply")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|e| AppError::config_error(format!("Template selection failed: {e}")))?;

    match TemplateChoice::ALL[selection] {
        TemplateChoice::Workstream => {
            let name = create_workstream(ctx)?;
            Ok(TemplateOutcome::Workstream { name })
        }
        TemplateChoice::ObserverRole => create_role(ctx, Layer::Observers, None, None),
        TemplateChoice::DeciderRole => create_role(ctx, Layer::Deciders, None, None),
    }
}
