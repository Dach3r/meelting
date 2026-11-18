use crate::domain::room::{RECENT_MESSAGES, RoomDetail};
use crate::repositories::{agent_runs, messages, room_agents, rooms};
use crate::services::agent_run_registry::Registry;
use crate::services::rooms_list::with_agents;
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(
    pool: &SqlitePool,
    registry: &Registry,
    room_id: &str,
) -> Result<RoomDetail, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let partials = registry.partials(room_id);
    let mut transaction = pool.begin().await?;
    let row = rooms::get(&mut *transaction, &workspace_id, room_id)
        .await?
        .ok_or(SessionError::RoomNotFound)?;
    let participants = room_agents::list(&mut *transaction, &workspace_id).await?;
    let messages = messages::recent(&mut *transaction, room_id, RECENT_MESSAGES).await?;
    let runs = agent_runs::for_room(&mut *transaction, room_id, RECENT_MESSAGES).await?;
    transaction.commit().await?;
    let room = with_agents(vec![row], participants).remove(0);
    Ok(RoomDetail {
        room,
        messages,
        runs,
        partials,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;
    use crate::services::{first_launch, message_send, room_create, workspace_create};

    #[tokio::test]
    async fn returns_the_room_with_its_last_100_messages_in_order() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let room = room_create::run(&pool, "Sala", vec![]).await.unwrap();
        for index in 0..105 {
            message_send::run(&pool, &room.id, &format!("m{index}"))
                .await
                .unwrap();
        }

        let detail = run(&pool, &Registry::default(), &room.id).await.unwrap();

        assert_eq!(detail.room.name, "Sala");
        assert_eq!(detail.messages.len(), 100);
        assert_eq!(detail.messages[0].body, "m5");
        assert_eq!(detail.messages[99].body, "m104");
    }

    #[tokio::test]
    async fn does_not_find_unknown_rooms_or_rooms_of_another_workspace() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let room = room_create::run(&pool, "Sala", vec![]).await.unwrap();

        assert!(matches!(
            run(&pool, &Registry::default(), "missing").await,
            Err(SessionError::RoomNotFound)
        ));
        workspace_create::run(&pool, "Globex").await.unwrap();
        assert!(matches!(
            run(&pool, &Registry::default(), &room.id).await,
            Err(SessionError::RoomNotFound)
        ));
    }
}
