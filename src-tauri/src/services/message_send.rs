use crate::domain::room::{Message, normalize_body};
use crate::repositories::{messages, rooms};
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn run(pool: &SqlitePool, room_id: &str, body: &str) -> Result<Message, SessionError> {
    let body = normalize_body(body).ok_or(SessionError::InvalidMessage)?;
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    if rooms::get(&mut *transaction, &workspace_id, room_id)
        .await?
        .is_none()
    {
        return Err(SessionError::RoomNotFound);
    }
    let message = messages::insert_from_user(
        &mut *transaction,
        &Uuid::now_v7().to_string(),
        room_id,
        &body,
    )
    .await?;
    rooms::touch(&mut *transaction, room_id, &message.created_at).await?;
    transaction.commit().await?;
    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;
    use crate::services::agent_run_engine::testing::{fast_limits, harness};
    use crate::services::agent_run_registry::Registry;
    use crate::services::{first_launch, room_create, room_finish, room_get, workspace_create};
    use futures_util::future::join_all;

    async fn count(pool: &SqlitePool, sql: &'static str) -> i64 {
        sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
    }

    #[tokio::test]
    async fn saves_a_user_message_and_marks_the_room_activity() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let room = room_create::run(&pool, "Daily", vec![]).await.unwrap();

        let message = run(&pool, &room.id, "  Hola equipo  ").await.unwrap();

        assert_eq!(message.room_id, room.id);
        assert_eq!(message.agent_id, None);
        assert_eq!(message.body, "Hola equipo");
        let detail = room_get::run(&pool, &Registry::default(), &room.id)
            .await
            .unwrap();
        assert_eq!(detail.messages, std::slice::from_ref(&message));
        assert_eq!(detail.room.updated_at, message.created_at);
    }

    #[tokio::test]
    async fn rejects_blank_and_too_long_bodies_without_touching_the_room() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let room = room_create::run(&pool, "Daily", vec![]).await.unwrap();

        for body in ["  ".to_string(), "a".repeat(4001)] {
            assert!(matches!(
                run(&pool, &room.id, &body).await,
                Err(SessionError::InvalidMessage)
            ));
        }
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM messages").await, 0);
        let detail = room_get::run(&pool, &Registry::default(), &room.id)
            .await
            .unwrap();
        assert_eq!(detail.room.updated_at, room.updated_at);
    }

    #[tokio::test]
    async fn counts_each_emoji_as_one_character_up_to_the_maximum() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let room = room_create::run(&pool, "Daily", vec![]).await.unwrap();

        let message = run(&pool, &room.id, &"😀".repeat(4000)).await.unwrap();

        assert_eq!(message.body.chars().count(), 4000);
        assert!(matches!(
            run(&pool, &room.id, &"😀".repeat(4001)).await,
            Err(SessionError::InvalidMessage)
        ));
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM messages").await, 1);
    }

    #[tokio::test]
    async fn rejects_unknown_rooms_and_rooms_of_another_workspace() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let room = room_create::run(&pool, "Daily", vec![]).await.unwrap();

        assert!(matches!(
            run(&pool, "missing", "hola").await,
            Err(SessionError::RoomNotFound)
        ));
        workspace_create::run(&pool, "Globex").await.unwrap();
        assert!(matches!(
            run(&pool, &room.id, "hola").await,
            Err(SessionError::RoomNotFound)
        ));
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM messages").await, 0);
    }

    #[tokio::test]
    async fn keeps_every_concurrent_message_and_the_latest_activity() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let room = room_create::run(&pool, "Daily", vec![]).await.unwrap();

        let sent = join_all((0..20).map(|index| {
            let pool = pool.clone();
            let room_id = room.id.clone();
            async move { run(&pool, &room_id, &format!("m{index}")).await }
        }))
        .await;

        assert!(sent.iter().all(Result::is_ok));
        let detail = room_get::run(&pool, &Registry::default(), &room.id)
            .await
            .unwrap();
        assert_eq!(detail.messages.len(), 20);
        let latest = detail
            .messages
            .iter()
            .map(|message| message.created_at.as_str())
            .max()
            .unwrap();
        assert_eq!(detail.room.updated_at, latest);
    }

    #[tokio::test]
    async fn sending_while_the_room_is_finished_never_stores_a_message_after_it() {
        let h = harness("http://127.0.0.1:1", fast_limits(), &[]).await;
        let pool = &h.engine.pool;
        let room_id = &h.room_id;

        let bodies: Vec<String> = (0..10).map(|index| format!("m{index}")).collect();

        let (sent, finished) = tokio::join!(
            join_all(bodies.iter().map(|body| run(pool, room_id, body))),
            room_finish::run(&h.engine, room_id)
        );

        assert!(finished.is_ok());
        assert!(
            sent.iter()
                .all(|result| matches!(result, Ok(_) | Err(SessionError::RoomNotFound)))
        );
        let stored = sent.iter().filter(|result| result.is_ok()).count() as i64;
        assert_eq!(count(pool, "SELECT COUNT(*) FROM messages").await, stored);
        assert!(matches!(
            run(pool, room_id, "late").await,
            Err(SessionError::RoomNotFound)
        ));
    }
}
