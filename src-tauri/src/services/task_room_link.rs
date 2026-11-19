use crate::domain::task::{Task, TaskOrigin};
use crate::repositories::{messages, rooms, task_room_links, tasks};
use crate::services::session_load::{self, SessionError};
use crate::services::tasks_list;
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn run(
    pool: &SqlitePool,
    task_id: &str,
    origin: TaskOrigin,
) -> Result<Task, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let task = tasks::get(&mut *transaction, &workspace_id, task_id)
        .await?
        .ok_or(SessionError::TaskNotFound)?;
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
    let is_origin = task.origin_room_id.as_deref() == Some(origin.room_id.as_str())
        && task.origin_message_id == origin.message_id;
    if !is_origin {
        task_room_links::insert(
            &mut *transaction,
            &Uuid::now_v7().to_string(),
            task_id,
            &workspace_id,
            &origin.room_id,
            origin.message_id.as_deref(),
        )
        .await?;
    }
    transaction.commit().await?;
    tasks_list::one(pool, &workspace_id, task_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::domain::task::{TaskDraft, TaskPriority};
    use crate::services::{first_launch, message_send, room_create, task_save};

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

    fn origin(room_id: &str, message_id: Option<&str>) -> TaskOrigin {
        TaskOrigin {
            room_id: room_id.into(),
            message_id: message_id.map(String::from),
        }
    }

    async fn setup() -> (SqlitePool, tempfile::TempDir) {
        let (pool, dir) = crate::repositories::database::open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        (pool, dir)
    }

    #[tokio::test]
    async fn links_an_existing_task_to_a_room_and_a_message_once() {
        let (pool, _dir) = setup().await;
        let room = room_create::run(&pool, "Daily", vec![]).await.unwrap();
        let message = message_send::run(&pool, &room.id, "Status?").await.unwrap();
        let task = task_save::create(&pool, draft("Status"), None)
            .await
            .unwrap();

        run(&pool, &task.id, origin(&room.id, None)).await.unwrap();
        run(&pool, &task.id, origin(&room.id, None)).await.unwrap();
        let linked = run(&pool, &task.id, origin(&room.id, Some(&message.id)))
            .await
            .unwrap();

        assert_eq!(linked.linked_rooms.len(), 2);
        assert_eq!(linked.linked_rooms[0].room_name, "Daily");
        assert_eq!(linked.linked_rooms[0].message_id, None);
        assert_eq!(
            linked.linked_rooms[1].message_id.as_deref(),
            Some(message.id.as_str())
        );
        assert_eq!(linked.origin_room_id, None);
    }

    #[tokio::test]
    async fn does_not_duplicate_the_origin_as_a_link() {
        let (pool, _dir) = setup().await;
        let room = room_create::run(&pool, "Daily", vec![]).await.unwrap();
        let task = task_save::create(&pool, draft("Status"), Some(origin(&room.id, None)))
            .await
            .unwrap();

        let linked = run(&pool, &task.id, origin(&room.id, None)).await.unwrap();

        assert!(linked.linked_rooms.is_empty());
    }

    #[tokio::test]
    async fn rejects_unknown_tasks_rooms_and_messages_without_writing() {
        let (pool, _dir) = setup().await;
        let room = room_create::run(&pool, "Daily", vec![]).await.unwrap();
        let other = room_create::run(&pool, "Other", vec![]).await.unwrap();
        let message = message_send::run(&pool, &other.id, "Hi").await.unwrap();
        let task = task_save::create(&pool, draft("Status"), None)
            .await
            .unwrap();

        assert!(matches!(
            run(&pool, "missing", origin(&room.id, None)).await,
            Err(SessionError::TaskNotFound)
        ));
        assert!(matches!(
            run(&pool, &task.id, origin("missing", None)).await,
            Err(SessionError::RoomNotFound)
        ));
        assert!(matches!(
            run(&pool, &task.id, origin(&room.id, Some(&message.id))).await,
            Err(SessionError::MessageNotFound)
        ));
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM task_room_links")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
}
