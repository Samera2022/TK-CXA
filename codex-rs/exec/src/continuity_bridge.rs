use std::path::Path;

use codex_app_server_protocol::{AdditionalContextEntry, AdditionalContextKind};
use codex_project_continuity_client::{
    ContinuityClient, CreateCheckpointRequest, GitState, ProjectCheckpoint,
};
use tracing::warn;

const CONTINUITY_CONTEXT_KEY: &str = "project_continuity.checkpoint";
const MAX_CONTEXT_CHARS: usize = 8_000;

pub(crate) struct ContinuityBridge {
    client: ContinuityClient,
}

impl ContinuityBridge {
    pub(crate) fn from_env() -> Option<Self> {
        let base_url = std::env::var("CCR_BASE_URL").ok()?;
        let token = std::env::var("CCR_TOKEN").ok();
        let client = match ContinuityClient::new(&base_url, token) {
            Ok(client) => client,
            Err(error) => {
                warn!(%error, "CCR continuity configuration is invalid");
                return None;
            }
        };
        Some(Self { client })
    }

    pub(crate) async fn bootstrap(
        &self,
        project_directory: &Path,
    ) -> Option<std::collections::HashMap<String, AdditionalContextEntry>> {
        let directory = project_directory.to_string_lossy();
        let checkpoint = match self.client.get_latest(&directory, None).await {
            Ok(Some(checkpoint)) => checkpoint,
            Ok(None) => return None,
            Err(error) => {
                warn!(%error, "CCR continuity bootstrap failed");
                return None;
            }
        };
        let current_git = match self.client.get_git_state(&directory).await {
            Ok(state) => state,
            Err(error) => {
                warn!(%error, "CCR continuity Git state lookup failed");
                GitState {
                    branch: None,
                    head_sha: None,
                    dirty: false,
                }
            }
        };
        if !checkpoint_matches_git(&checkpoint, &current_git) {
            return None;
        }
        let value = render_checkpoint(&checkpoint);
        Some(std::collections::HashMap::from([(
            CONTINUITY_CONTEXT_KEY.to_owned(),
            AdditionalContextEntry {
                value,
                kind: AdditionalContextKind::Application,
            },
        )]))
    }

    pub(crate) async fn save(
        &self,
        project_directory: &Path,
        objective: &str,
        final_message: Option<&str>,
        failed: bool,
    ) {
        let directory = project_directory.to_string_lossy().into_owned();
        let git_state = self.client.get_git_state(&directory).await.ok();
        let request = CreateCheckpointRequest {
            id: None,
            project_directory: directory,
            client: "codex".to_owned(),
            git_branch: git_state.as_ref().and_then(|state| state.branch.clone()),
            git_head_sha: git_state.as_ref().and_then(|state| state.head_sha.clone()),
            git_dirty: git_state.as_ref().is_some_and(|state| state.dirty),
            objective: non_empty(objective),
            completed: final_message
                .filter(|_| !failed)
                .map(|message| vec![truncate(message)])
                .unwrap_or_default(),
            blockers: if failed {
                vec!["The turn failed or was interrupted.".to_owned()]
            } else {
                vec![]
            },
            next_steps: vec![],
            modified_files: vec![],
            verification_status: Some(if failed { "failed" } else { "passed" }.to_owned()),
        };
        if let Err(error) = self.client.create_checkpoint(&request).await {
            warn!(%error, "CCR continuity checkpoint save failed");
        }
    }
}

fn checkpoint_matches_git(checkpoint: &ProjectCheckpoint, current: &GitState) -> bool {
    current.branch.as_ref().is_none_or(|branch| {
        checkpoint
            .git_branch
            .as_ref()
            .is_none_or(|saved| saved == branch)
    }) && current.head_sha.as_ref().is_none_or(|head| {
        checkpoint
            .git_head_sha
            .as_ref()
            .is_none_or(|saved| saved == head)
    })
}

fn render_checkpoint(checkpoint: &ProjectCheckpoint) -> String {
    let mut output = String::from("# Project Continuity Checkpoint\n\n");
    output.push_str(&format!("**Project**: {}\n", checkpoint.project_directory));
    if let Some(branch) = &checkpoint.git_branch {
        output.push_str(&format!("**Branch**: {branch}\n"));
    }
    if let Some(objective) = &checkpoint.objective {
        output.push_str(&format!("\n**Current Objective**:\n{objective}\n"));
    }
    append_list(&mut output, "Completed", &checkpoint.completed);
    append_list(&mut output, "Blockers", &checkpoint.blockers);
    append_list(&mut output, "Next Steps", &checkpoint.next_steps);
    truncate(&output)
}

fn append_list(output: &mut String, title: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    output.push_str(&format!("\n**{title}**:\n"));
    for value in values {
        output.push_str(&format!("- {value}\n"));
    }
}

fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn truncate(value: &str) -> String {
    value.chars().take(MAX_CONTEXT_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checkpoint() -> ProjectCheckpoint {
        ProjectCheckpoint {
            id: "chk_1".to_owned(),
            project_directory: "/tmp/project".to_owned(),
            client: "codex".to_owned(),
            git_branch: Some("main".to_owned()),
            git_head_sha: Some("abc".to_owned()),
            git_dirty: false,
            objective: Some("Continue".to_owned()),
            completed: vec!["Scaffold".to_owned()],
            blockers: vec![],
            next_steps: vec!["Integrate".to_owned()],
            modified_files: vec![],
            verification_status: Some("passed".to_owned()),
            created_at: 1,
            updated_at: 2,
        }
    }

    #[test]
    fn accepts_matching_git_state_and_rejects_branch_or_head_drift() {
        let checkpoint = checkpoint();
        assert!(checkpoint_matches_git(
            &checkpoint,
            &GitState {
                branch: Some("main".to_owned()),
                head_sha: Some("abc".to_owned()),
                dirty: true,
            }
        ));
        assert!(!checkpoint_matches_git(
            &checkpoint,
            &GitState {
                branch: Some("other".to_owned()),
                head_sha: Some("abc".to_owned()),
                dirty: false,
            }
        ));
        assert!(!checkpoint_matches_git(
            &checkpoint,
            &GitState {
                branch: Some("main".to_owned()),
                head_sha: Some("def".to_owned()),
                dirty: false,
            }
        ));
    }

    #[test]
    fn renders_bounded_application_context() {
        let rendered = render_checkpoint(&checkpoint());
        assert!(rendered.contains("# Project Continuity Checkpoint"));
        assert!(rendered.contains("Continue"));
        assert!(rendered.contains("- Scaffold"));
        assert!(rendered.chars().count() <= MAX_CONTEXT_CHARS);
    }
}
