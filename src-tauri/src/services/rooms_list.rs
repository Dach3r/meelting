use crate::domain::room::{PREVIEW_MESSAGES, Participant, Room, RoomList};
use crate::repositories::{messages, room_agents, rooms};
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool) -> Result<RoomList, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let mut transaction = pool.begin().await?;
    let rows = rooms::list(&mut *transaction, &workspace_id).await?;
    let participants = room_agents::list(&mut *transaction, &workspace_id).await?;
    let rooms = with_agents(rows, participants);
    let featured_messages = match rooms.first() {
        Some(room) => messages::recent(&mut *transaction, &room.id, PREVIEW_MESSAGES).await?,
        None => Vec::new(),
    };
    transaction.commit().await?;
    Ok(RoomList {
        rooms,
        featured_messages,
    })
}

pub fn with_agents(
    rows: Vec<(String, String, String)>,
    participants: Vec<(String, Participant)>,
) -> Vec<Room> {
    rows.into_iter()
        .map(|(id, name, updated_at)| Room {
            agents: participants
                .iter()
                .filter(|(room_id, _)| *room_id == id)
                .map(|(_, agent)| agent.clone())
                .collect(),
            id,
            name,
            updated_at,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;
    use crate::services::{first_launch, message_send, room_create, workspace_create};

    #[tokio::test]
    async fn is_empty_for_a_new_workspace() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();

        let list = run(&pool).await.unwrap();

        assert!(list.rooms.is_empty());
        assert!(list.featured_messages.is_empty());
    }

    #[tokio::test]
    async fn orders_by_last_activity_and_previews_the_last_three_messages_of_the_first() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let older = room_create::run(&pool, "Older", vec![]).await.unwrap();
        let newer = room_create::run(&pool, "Newer", vec![]).await.unwrap();
        for body in ["one", "two", "three", "four"] {
            message_send::run(&pool, &older.id, body).await.unwrap();
        }

        let list = run(&pool).await.unwrap();

        let names: Vec<&str> = list.rooms.iter().map(|room| room.name.as_str()).collect();
        assert_eq!(names, ["Older", "Newer"]);
        assert!(list.rooms[0].updated_at > newer.updated_at);
        let bodies: Vec<&str> = list
            .featured_messages
            .iter()
            .map(|message| message.body.as_str())
            .collect();
        assert_eq!(bodies, ["two", "three", "four"]);
    }

    #[tokio::test]
    async fn lists_only_the_rooms_of_the_current_workspace() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        room_create::run(&pool, "Acme room", vec![]).await.unwrap();

        workspace_create::run(&pool, "Globex").await.unwrap();

        assert!(run(&pool).await.unwrap().rooms.is_empty());
    }
}
