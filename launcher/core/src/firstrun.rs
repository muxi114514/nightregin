//! 首次运行要走哪些步：前端按这份计划依次弹横带

use std::path::Path;

use serde::Serialize;

use crate::preflight::{Issue, IssueKind, PackState, health_issues};
use crate::saves::{account_dirs, resolve, vanilla_save};

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstRunPlan {
    pub done: bool,
    pub game_found: bool,
    pub accounts: Vec<String>,
    pub active_account: Option<String>,
    pub need_account_choice: bool,
    pub can_copy_vanilla: bool,
    /// 只含可一键改正的不一致
    pub issues: Vec<Issue>,
}

pub fn first_run_plan(state: &PackState, root: Option<&Path>, active: Option<u64>) -> FirstRunPlan {
    let accounts = root.map_or_else(Vec::new, account_dirs);
    let remembered = state
        .settings
        .save_account
        .filter(|id| accounts.contains(id));
    let savefile = state.savefile();
    let can_copy_vanilla = root
        .and_then(|root| resolve(root, savefile, active, state.settings.save_account).ok())
        .is_some_and(|save| !save.file.exists() && vanilla_save(&save).is_file());
    FirstRunPlan {
        done: state.settings.first_run_done,
        game_found: state.game.is_some(),
        need_account_choice: active.is_none() && remembered.is_none() && accounts.len() > 1,
        accounts: accounts.iter().map(u64::to_string).collect(),
        active_account: active.map(|id| id.to_string()),
        can_copy_vanilla,
        issues: health_issues(state)
            .into_iter()
            .filter(|issue| {
                matches!(
                    issue.kind,
                    IssueKind::SuffixMismatch
                        | IssueKind::ChanceSetterSuffix
                        | IssueKind::EverdarkMismatch
                )
            })
            .collect(),
    }
}
