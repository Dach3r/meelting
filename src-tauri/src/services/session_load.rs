use crate::domain::session::Session;
use crate::repositories::{profile, workspaces};
use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("database")]
    Database(#[from] sqlx::Error),
    #[error("invalid_name")]
    InvalidName,
    #[error("workspace_not_found")]
    WorkspaceNotFound,
    #[error("team_not_found")]
    TeamNotFound,
    #[error("agent_not_found")]
    AgentNotFound,
    #[error("invalid_agent")]
    InvalidAgent,
    #[error("room_not_found")]
    RoomNotFound,
    #[error("invalid_message")]
    InvalidMessage,
    #[error("agent_has_activity")]
    AgentHasActivity,
    #[error("invalid_client_request_id")]
    InvalidClientRequestId,
    #[error("agent_busy")]
    AgentBusy,
    #[error("client_request_conflict")]
    ClientRequestConflict,
    #[error("not_set_up")]
    NotSetUp,
    #[error("invalid_provider")]
    InvalidProvider,
    #[error("invalid_api_key")]
    InvalidApiKey,
    #[error("provider_not_configured")]
    ProviderNotConfigured,
    #[error("git_unavailable")]
    GitUnavailable,
    #[error("not_a_repository")]
    NotARepository,
    #[error("repository_bare")]
    RepositoryBare,
    #[error("repository_is_worktree")]
    RepositoryIsWorktree,
    #[error("repository_without_commits")]
    RepositoryWithoutCommits,
    #[error("repository_with_submodules")]
    RepositoryWithSubmodules,
    #[error("repository_already_registered")]
    RepositoryAlreadyRegistered,
    #[error("repository_not_found")]
    RepositoryNotFound,
    #[error("repository_in_use")]
    RepositoryInUse,
    #[error("repository_unavailable")]
    RepositoryUnavailable,
    #[error("invalid_base_ref")]
    InvalidBaseRef,
    #[error("invalid_task")]
    InvalidTask,
    #[error("task_not_found")]
    TaskNotFound,
    #[error("invalid_task_transition")]
    InvalidTaskTransition,
    #[error("message_not_found")]
    MessageNotFound,
    #[error("storage_network")]
    StorageNetwork,
    #[error("storage_removable")]
    StorageRemovable,
    #[error("storage_synced")]
    StorageSynced,
    #[error("storage_unknown")]
    StorageUnknown,
    #[error("invalid_worktrees_root")]
    InvalidWorktreesRoot,
    #[error("keychain_unavailable")]
    Keychain(#[from] crate::system::keychain::KeychainError),
}

impl From<crate::domain::storage::StorageIssue> for SessionError {
    fn from(issue: crate::domain::storage::StorageIssue) -> Self {
        use crate::domain::storage::StorageIssue;
        match issue {
            StorageIssue::Network => SessionError::StorageNetwork,
            StorageIssue::Removable => SessionError::StorageRemovable,
            StorageIssue::Synced => SessionError::StorageSynced,
            StorageIssue::Unknown => SessionError::StorageUnknown,
        }
    }
}

impl Serialize for SessionError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub async fn run(pool: &SqlitePool) -> Result<Option<Session>, SessionError> {
    let Some(profile) = profile::get(pool).await? else {
        return Ok(None);
    };
    let Some(current_workspace_id) = profile.current_workspace_id else {
        return Ok(None);
    };
    Ok(Some(Session {
        locale: profile.locale,
        theme: profile.theme,
        ui_scale: profile.ui_scale,
        loading_enabled: profile.loading_enabled,
        current_workspace_id,
        workspaces: workspaces::list(pool).await?,
    }))
}

pub async fn require(pool: &SqlitePool) -> Result<Session, SessionError> {
    run(pool).await?.ok_or(SessionError::NotSetUp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;

    #[tokio::test]
    async fn is_none_before_first_launch() {
        let (pool, _dir) = open_for_test().await;

        assert_eq!(run(&pool).await.unwrap(), None);
        assert!(matches!(require(&pool).await, Err(SessionError::NotSetUp)));
    }

    #[tokio::test]
    async fn loads_profile_and_workspaces() {
        let (pool, _dir) = open_for_test().await;
        workspaces::insert(&pool, "01", "Acme").await.unwrap();
        profile::create(&pool, Locale::Es, "01").await.unwrap();

        let session = run(&pool).await.unwrap().unwrap();

        assert_eq!(session.locale, Locale::Es);
        assert_eq!(session.current_workspace_id, "01");
        assert_eq!(session.workspaces.len(), 1);
    }

    #[tokio::test]
    async fn error_serializes_as_its_kind() {
        assert_eq!(
            serde_json::to_string(&SessionError::InvalidName).unwrap(),
            "\"invalid_name\""
        );
    }
}
