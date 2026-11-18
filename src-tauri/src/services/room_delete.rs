use crate::domain::room::RoomList;
use crate::repositories::rooms;
use crate::services::rooms_list;
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool, room_id: &str) -> Result<RoomList, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    if !rooms::delete(pool, &workspace_id, room_id).await? {
        return Err(SessionError::RoomNotFound);
    }
    rooms_list::run(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::domain::team::AgentDraft;
    use crate::repositories::database::open_for_test;
    use crate::services::{
        agent_create, first_launch, message_send, room_create, team_create, workspace_create,
    };

    async fn count(pool: &SqlitePool, sql: &'static str) -> i64 {
        sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
    }

    #[tokio::test]
    async fn deletes_the_room_with_its_participants_and_messages() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let teams = team_create::run(&pool, "General").await.unwrap();
        let draft = AgentDraft {
            team_id: teams[0].id.clone(),
            template_key: None,
            name: "Ana".into(),
            role: "QA Engineer".into(),
            description: String::new(),
            instructions: String::new(),
            provider: "anthropic".into(),
            model: "claude-sonnet-5-5".into(),
        };
        let agent_id = agent_create::run(&pool, draft).await.unwrap()[0].agents[0]
            .id
            .clone();
        let room = room_create::run(&pool, "Daily", vec![agent_id.clone()])
            .await
            .unwrap();
        let kept = room_create::run(&pool, "Kept", vec![agent_id.clone()])
            .await
            .unwrap();
        message_send::run(&pool, &room.id, "hola").await.unwrap();
        sqlx::query("INSERT INTO messages VALUES ('m-agent', ?, ?, 'hi', 'x')")
            .bind(&room.id)
            .bind(&agent_id)
            .execute(&pool)
            .await
            .unwrap();

        let list = run(&pool, &room.id).await.unwrap();

        assert_eq!(list.rooms.len(), 1);
        assert_eq!(list.rooms[0].id, kept.id);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM messages").await, 0);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM room_agents").await, 1);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM agents").await, 1);
    }

    #[tokio::test]
    async fn rejects_unknown_rooms_and_rooms_of_another_workspace() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let room = room_create::run(&pool, "Daily", vec![]).await.unwrap();

        assert!(matches!(
            run(&pool, "missing").await,
            Err(SessionError::RoomNotFound)
        ));
        workspace_create::run(&pool, "Globex").await.unwrap();
        assert!(matches!(
            run(&pool, &room.id).await,
            Err(SessionError::RoomNotFound)
        ));
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM rooms").await, 1);
    }
}
