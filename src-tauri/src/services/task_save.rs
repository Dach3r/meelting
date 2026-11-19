use crate::domain::task::{Task, TaskDraft, TaskOrigin};
use crate::repositories::{agents, git_repositories, messages, rooms, task_agents, tasks};
use crate::services::session_load::{self, SessionError};
use crate::services::tasks_list;
use crate::system::git::{self, GitError};
use sqlx::SqlitePool;
use std::path::Path;
use uuid::Uuid;

pub async fn create(
    pool: &SqlitePool,
    draft: TaskDraft,
    origin: Option<TaskOrigin>,
) -> Result<Task, SessionError> {
    let draft = draft.normalize().ok_or(SessionError::InvalidTask)?;
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    check_base_ref(pool, &workspace_id, &draft).await?;
    let id = Uuid::now_v7().to_string();
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    if let Some(origin) = &origin {
        if rooms::get(&mut *transaction, &workspace_id, &origin.room_id)
            .await?
            .is_none()
        {
            return Err(SessionError::RoomNotFound);
        }
        if let Some(message_id) = &origin.message_id
            && messages::get(&mut *transaction, &origin.room_id, message_id)
                .await?
                .is_none()
        {
            return Err(SessionError::MessageNotFound);
        }
    }
    check_references(&mut transaction, &workspace_id, &draft).await?;
    tasks::insert(
        &mut *transaction,
        &id,
        &workspace_id,
        &draft,
        origin.as_ref().map(|origin| origin.room_id.as_str()),
        origin
            .as_ref()
            .and_then(|origin| origin.message_id.as_deref()),
    )
    .await?;
    assign(&mut transaction, &id, &workspace_id, &draft.agent_ids).await?;
    transaction.commit().await?;
    tasks_list::one(pool, &workspace_id, &id).await
}

pub async fn update(
    pool: &SqlitePool,
    task_id: &str,
    draft: TaskDraft,
) -> Result<Task, SessionError> {
    let draft = draft.normalize().ok_or(SessionError::InvalidTask)?;
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let current = tasks_list::one(pool, &workspace_id, task_id).await?;
    if (&current.repository_id, &current.base_ref) != (&draft.repository_id, &draft.base_ref) {
        check_base_ref(pool, &workspace_id, &draft).await?;
    }
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    check_references(&mut transaction, &workspace_id, &draft).await?;
    if !tasks::update(&mut *transaction, &workspace_id, task_id, &draft).await? {
        return Err(SessionError::TaskNotFound);
    }
    task_agents::delete_others(&mut *transaction, task_id, &draft.agent_ids).await?;
    assign(&mut transaction, task_id, &workspace_id, &draft.agent_ids).await?;
    transaction.commit().await?;
    tasks_list::one(pool, &workspace_id, task_id).await
}

async fn check_base_ref(
    pool: &SqlitePool,
    workspace_id: &str,
    draft: &TaskDraft,
) -> Result<(), SessionError> {
    let (Some(repository_id), Some(base_ref)) = (&draft.repository_id, &draft.base_ref) else {
        return Ok(());
    };
    let repository = git_repositories::get(pool, workspace_id, repository_id)
        .await?
        .ok_or(SessionError::RepositoryNotFound)?;
    let branches = git::local_branches(Path::new(&repository.path))
        .await
        .map_err(|error| match error {
            GitError::Unavailable(_) => SessionError::GitUnavailable,
            GitError::Failed(_) => SessionError::RepositoryUnavailable,
        })?;
    if branches.contains(base_ref) {
        Ok(())
    } else {
        Err(SessionError::InvalidBaseRef)
    }
}

async fn check_references(
    transaction: &mut sqlx::SqliteConnection,
    workspace_id: &str,
    draft: &TaskDraft,
) -> Result<(), SessionError> {
    if let Some(repository_id) = &draft.repository_id
        && git_repositories::get(&mut *transaction, workspace_id, repository_id)
            .await?
            .is_none()
    {
        return Err(SessionError::RepositoryNotFound);
    }
    for agent_id in &draft.agent_ids {
        if !agents::exists(&mut *transaction, workspace_id, agent_id).await? {
            return Err(SessionError::AgentNotFound);
        }
    }
    Ok(())
}

async fn assign(
    transaction: &mut sqlx::SqliteConnection,
    task_id: &str,
    workspace_id: &str,
    agent_ids: &[String],
) -> Result<(), SessionError> {
    for agent_id in agent_ids {
        task_agents::insert(&mut *transaction, task_id, agent_id, workspace_id).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::domain::task::{TaskPriority, TaskStatus};
    use crate::domain::team::AgentDraft;
    use crate::repositories::database::open_for_test;
    use crate::services::repository_register::tests::{git, local, repo_with_commit};
    use crate::services::{
        agent_create, agent_delete, first_launch, message_send, repository_register,
        repository_unregister, room_create, task_delete, task_status_change, team_create,
        workspace_create, workspace_switch,
    };

    fn draft(title: &str) -> TaskDraft {
        TaskDraft {
            title: title.into(),
            description: String::new(),
            priority: TaskPriority::Medium,
            repository_id: None,
            base_ref: None,
            agent_ids: Vec::new(),
        }
    }

    async fn hire(pool: &SqlitePool, name: &str) -> String {
        let teams = team_create::run(pool, "General").await.unwrap();
        let draft = AgentDraft {
            team_id: teams.last().unwrap().id.clone(),
            template_key: None,
            name: name.into(),
            role: "QA Engineer".into(),
            description: String::new(),
            instructions: String::new(),
            provider: "anthropic".into(),
            model: "claude-sonnet-5-5".into(),
        };
        agent_create::run(pool, &crate::catalogs::llm::providers(), draft)
            .await
            .unwrap()
            .into_iter()
            .flat_map(|team| team.agents)
            .find(|agent| agent.name == name)
            .unwrap()
            .id
    }

    async fn count(pool: &SqlitePool, sql: &'static str) -> i64 {
        sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
    }

    async fn setup() -> (SqlitePool, tempfile::TempDir, String) {
        let (pool, dir) = open_for_test().await;
        let session = first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        (pool, dir, session.current_workspace_id)
    }

    #[tokio::test]
    async fn creates_a_pending_task_without_repository_or_agents() {
        let (pool, _dir, _) = setup().await;

        let task = create(&pool, draft("  Write the plan "), None)
            .await
            .unwrap();

        assert_eq!(task.title, "Write the plan");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.priority, TaskPriority::Medium);
        assert_eq!(task.repository_id, None);
        assert!(task.agent_ids.is_empty());
        assert_eq!(tasks_list::run(&pool).await.unwrap(), [task]);
    }

    #[tokio::test]
    async fn assigns_several_agents_once_without_starting_runs() {
        let (pool, _dir, _) = setup().await;
        let ana = hire(&pool, "Ana").await;
        let bob = hire(&pool, "Bob").await;
        let eva = hire(&pool, "Eva").await;

        let task = create(
            &pool,
            TaskDraft {
                agent_ids: vec![eva.clone(), ana.clone(), bob.clone(), ana.clone()],
                ..draft("Release")
            },
            None,
        )
        .await
        .unwrap();

        assert_eq!(task.agent_ids, [ana, bob, eva]);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM task_agents").await, 3);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM agent_runs").await, 0);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM messages").await, 0);
    }

    #[tokio::test]
    async fn rejects_agents_and_repositories_of_another_workspace_without_writing() {
        let (pool, _dir, first) = setup().await;
        let ana = hire(&pool, "Ana").await;
        workspace_create::run(&pool, "Globex").await.unwrap();
        let eva = hire(&pool, "Eva").await;
        let parent = repo_with_commit("app");
        let foreign = repository_register::run(&pool, &parent.path().join("app"), local)
            .await
            .unwrap()[0]
            .id
            .clone();
        workspace_switch::run(&pool, &first).await.unwrap();

        assert!(matches!(
            create(
                &pool,
                TaskDraft {
                    agent_ids: vec![ana, eva],
                    ..draft("Release")
                },
                None
            )
            .await,
            Err(SessionError::AgentNotFound)
        ));
        assert!(matches!(
            create(
                &pool,
                TaskDraft {
                    repository_id: Some(foreign),
                    ..draft("Release")
                },
                None
            )
            .await,
            Err(SessionError::RepositoryNotFound)
        ));
        assert!(matches!(
            create(&pool, draft("   "), None).await,
            Err(SessionError::InvalidTask)
        ));
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM tasks").await, 0);
    }

    #[tokio::test]
    async fn keeps_the_room_and_message_it_came_from() {
        let (pool, _dir, _) = setup().await;
        let room = room_create::run(&pool, "Daily", vec![]).await.unwrap();
        let message = message_send::run(&pool, &room.id, "Ship the login fix")
            .await
            .unwrap();

        let task = create(
            &pool,
            draft("Login fix"),
            Some(TaskOrigin {
                room_id: room.id.clone(),
                message_id: Some(message.id.clone()),
            }),
        )
        .await
        .unwrap();

        assert_eq!(task.origin_room_id.as_deref(), Some(room.id.as_str()));
        assert_eq!(task.origin_room_name.as_deref(), Some("Daily"));
        assert_eq!(task.origin_message_id.as_deref(), Some(message.id.as_str()));
    }

    #[tokio::test]
    async fn rejects_unknown_or_finished_rooms_and_messages_of_another_room() {
        let (pool, _dir, workspace_id) = setup().await;
        let daily = room_create::run(&pool, "Daily", vec![]).await.unwrap();
        let other = room_create::run(&pool, "Other", vec![]).await.unwrap();
        let message = message_send::run(&pool, &other.id, "Hi").await.unwrap();
        let origin = |room_id: &str, message_id: Option<&str>| {
            Some(TaskOrigin {
                room_id: room_id.into(),
                message_id: message_id.map(String::from),
            })
        };

        assert!(matches!(
            create(&pool, draft("A"), origin(&daily.id, Some(&message.id))).await,
            Err(SessionError::MessageNotFound)
        ));
        assert!(matches!(
            create(&pool, draft("A"), origin("missing", None)).await,
            Err(SessionError::RoomNotFound)
        ));
        rooms::finish(&pool, &workspace_id, &daily.id)
            .await
            .unwrap();
        assert!(matches!(
            create(&pool, draft("A"), origin(&daily.id, None)).await,
            Err(SessionError::RoomNotFound)
        ));
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM tasks").await, 0);
    }

    #[tokio::test]
    async fn links_a_repository_and_base_branch_without_touching_disk() {
        let (pool, _dir, _) = setup().await;
        let parent = repo_with_commit("app");
        let repo = parent.path().join("app");
        git(&repo, &["branch", "develop"]);
        let repository_id = repository_register::run(&pool, &repo, local).await.unwrap()[0]
            .id
            .clone();
        let entries = std::fs::read_dir(parent.path()).unwrap().count();

        let task = create(
            &pool,
            TaskDraft {
                repository_id: Some(repository_id.clone()),
                base_ref: Some("develop".into()),
                ..draft("Login")
            },
            None,
        )
        .await
        .unwrap();

        assert_eq!(task.base_ref.as_deref(), Some("develop"));
        assert_eq!(std::fs::read_dir(parent.path()).unwrap().count(), entries);
        let worktrees = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["worktree", "list", "--porcelain"])
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&worktrees.stdout)
                .matches("worktree ")
                .count(),
            1
        );
        assert!(matches!(
            create(
                &pool,
                TaskDraft {
                    repository_id: Some(repository_id),
                    base_ref: Some("missing".into()),
                    ..draft("Login")
                },
                None
            )
            .await,
            Err(SessionError::InvalidBaseRef)
        ));
    }

    #[tokio::test]
    async fn updates_fields_and_replaces_the_assigned_agents() {
        let (pool, _dir, _) = setup().await;
        let ana = hire(&pool, "Ana").await;
        let bob = hire(&pool, "Bob").await;
        let task = create(
            &pool,
            TaskDraft {
                agent_ids: vec![ana.clone()],
                ..draft("Old")
            },
            None,
        )
        .await
        .unwrap();

        let updated = update(
            &pool,
            &task.id,
            TaskDraft {
                description: "Details".into(),
                priority: TaskPriority::High,
                agent_ids: vec![bob.clone()],
                ..draft("New")
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.title, "New");
        assert_eq!(updated.description, "Details");
        assert_eq!(updated.priority, TaskPriority::High);
        assert_eq!(updated.status, TaskStatus::Pending);
        assert_eq!(updated.agent_ids, [bob]);
        assert!(matches!(
            update(&pool, "missing", draft("New")).await,
            Err(SessionError::TaskNotFound)
        ));
    }

    #[tokio::test]
    async fn editing_without_changing_the_base_branch_does_not_need_the_repository_on_disk() {
        let (pool, _dir, _) = setup().await;
        let parent = repo_with_commit("app");
        let repo = parent.path().join("app");
        let repository_id = repository_register::run(&pool, &repo, local).await.unwrap()[0]
            .id
            .clone();
        let linked = TaskDraft {
            repository_id: Some(repository_id),
            base_ref: Some("main".into()),
            ..draft("Login")
        };
        let task = create(&pool, linked.clone(), None).await.unwrap();
        std::fs::remove_dir_all(&repo).unwrap();

        let renamed = update(
            &pool,
            &task.id,
            TaskDraft {
                title: "Login v2".into(),
                ..linked.clone()
            },
        )
        .await
        .unwrap();

        assert_eq!(renamed.title, "Login v2");
        assert!(matches!(
            update(
                &pool,
                &task.id,
                TaskDraft {
                    base_ref: Some("develop".into()),
                    ..linked
                }
            )
            .await,
            Err(SessionError::RepositoryUnavailable)
        ));
    }

    #[tokio::test]
    async fn completes_and_reopens_following_the_state_machine() {
        let (pool, _dir, _) = setup().await;
        let task = create(&pool, draft("Plan"), None).await.unwrap();

        let completed = task_status_change::run(&pool, &task.id, TaskStatus::Completed)
            .await
            .unwrap();
        assert!(completed.completed_at.is_some());
        assert!(matches!(
            task_status_change::run(&pool, &task.id, TaskStatus::Blocked).await,
            Err(SessionError::InvalidTaskTransition)
        ));

        let reopened = task_status_change::run(&pool, &task.id, TaskStatus::InProgress)
            .await
            .unwrap();
        assert_eq!(reopened.status, TaskStatus::InProgress);
        assert_eq!(reopened.completed_at, None);
        assert!(matches!(
            task_status_change::run(&pool, "missing", TaskStatus::Pending).await,
            Err(SessionError::TaskNotFound)
        ));
    }

    #[tokio::test]
    async fn deletes_tasks_and_then_allows_unregistering_their_repository() {
        let (pool, _dir, _) = setup().await;
        let parent = repo_with_commit("app");
        let repo = parent.path().join("app");
        let repository_id = repository_register::run(&pool, &repo, local).await.unwrap()[0]
            .id
            .clone();
        let task = create(
            &pool,
            TaskDraft {
                repository_id: Some(repository_id.clone()),
                ..draft("Login")
            },
            None,
        )
        .await
        .unwrap();

        assert!(matches!(
            repository_unregister::run(&pool, &repository_id).await,
            Err(SessionError::RepositoryInUse)
        ));
        assert!(task_delete::run(&pool, &task.id).await.unwrap().is_empty());
        assert!(matches!(
            task_delete::run(&pool, &task.id).await,
            Err(SessionError::TaskNotFound)
        ));
        assert!(
            repository_unregister::run(&pool, &repository_id)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(repo.join(".git").exists());
        assert!(matches!(
            repository_unregister::run(&pool, &repository_id).await,
            Err(SessionError::RepositoryNotFound)
        ));
    }

    #[tokio::test]
    async fn deleting_an_agent_without_activity_unassigns_it() {
        let (pool, _dir, workspace_id) = setup().await;
        let ana = hire(&pool, "Ana").await;
        let task = create(
            &pool,
            TaskDraft {
                agent_ids: vec![ana.clone()],
                ..draft("Plan")
            },
            None,
        )
        .await
        .unwrap();

        agent_delete::run(&pool, &ana).await.unwrap();

        assert!(
            tasks_list::one(&pool, &workspace_id, &task.id)
                .await
                .unwrap()
                .agent_ids
                .is_empty()
        );
    }
}
