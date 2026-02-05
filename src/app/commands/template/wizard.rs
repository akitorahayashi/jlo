use dialoguer::Select;

use crate::app::AppContext;
use crate::domain::{AppError, Layer};
use crate::ports::{RoleTemplateStore, WorkspaceStore};

use super::command::create_role_from_template;
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

pub(super) fn run_template_wizard<W, R>(ctx: &AppContext<W, R>) -> Result<TemplateOutcome, AppError>
where
    W: WorkspaceStore,
    R: RoleTemplateStore,
{
    let items: Vec<&str> = TemplateChoice::ALL.iter().map(|choice| choice.label()).collect();

    let selection = Select::new()
        .with_prompt("Select template type to apply")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|e| AppError::Internal { message: format!("Template selection failed: {e}") })?;

    match TemplateChoice::ALL[selection] {
        TemplateChoice::Workstream => {
            let name = create_workstream(ctx)?;
            Ok(TemplateOutcome::Workstream { name })
        }
        TemplateChoice::ObserverRole => {
            create_role_from_template(ctx, Layer::Observers, None, None)
        }
        TemplateChoice::DeciderRole => create_role_from_template(ctx, Layer::Deciders, None, None),
    }
}
