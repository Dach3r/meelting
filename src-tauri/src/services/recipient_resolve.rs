use crate::domain::recipient::{self, Resolution, Selection};
use crate::repositories::{room_agents, rooms};
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(
    pool: &SqlitePool,
    room_id: &str,
    body: &str,
    selection: &Selection,
    mention_ids: &[String],
) -> Result<Resolution, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let mut transaction = pool.begin().await?;
    if rooms::get(&mut *transaction, &workspace_id, room_id)
        .await?
        .is_none()
    {
        return Err(SessionError::RoomNotFound);
    }
    let participants: Vec<_> = room_agents::list(&mut *transaction, &workspace_id)
        .await?
        .into_iter()
        .filter(|(room, _)| room == room_id)
        .map(|(_, participant)| participant)
        .collect();
    transaction.commit().await?;
    recipient::resolve(selection, body, mention_ids, &participants)
        .map_err(|_| SessionError::AgentNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::domain::team::AgentDraft;
    use crate::repositories::database::open_for_test;
    use crate::services::{agent_create, first_launch, room_create, team_create, workspace_create};

    async fn room_with(pool: &SqlitePool, names: &[&str]) -> (String, Vec<String>) {
        let team_id = team_create::run(pool, "General").await.unwrap()[0]
            .id
            .clone();
        let mut ids = Vec::new();
        for name in names {
            let teams = agent_create::run(
                pool,
                AgentDraft {
                    team_id: team_id.clone(),
                    template_key: None,
                    name: (*name).into(),
                    role: "Backend Developer".into(),
                    description: String::new(),
                    instructions: String::new(),
                    provider: "ollama".into(),
                    model: "qwen-test".into(),
                },
            )
            .await
            .unwrap();
            ids.push(
                teams[0]
                    .agents
                    .iter()
                    .find(|agent| agent.name == *name)
                    .unwrap()
                    .id
                    .clone(),
            );
        }
        let room = room_create::run(pool, "Daily", ids.clone()).await.unwrap();
        (room.id, ids)
    }

    #[tokio::test]
    async fn previews_the_recipient_without_saving_anything() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let (room_id, ids) = room_with(&pool, &["Bruno", "Bea"]).await;

        let preview = |body: &'static str| {
            let pool = pool.clone();
            let room_id = room_id.clone();
            async move {
                run(&pool, &room_id, body, &Selection::Auto, &[])
                    .await
                    .unwrap()
            }
        };

        assert_eq!(
            preview("@Bruno hi").await,
            Resolution::Agent {
                agent_id: ids[0].clone()
            }
        );
        assert_eq!(preview("Just a note").await, Resolution::Nobody);
        assert!(matches!(
            preview("@backend hi").await,
            Resolution::Choose { .. }
        ));
        let messages: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(messages, 0);
    }

    #[tokio::test]
    async fn only_rooms_and_agents_of_the_current_workspace_count() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let (room_id, ids) = room_with(&pool, &["Bruno"]).await;
        workspace_create::run(&pool, "Globex").await.unwrap();
        let (other_room, outsiders) = room_with(&pool, &["Olga"]).await;

        assert!(matches!(
            run(&pool, &room_id, "@Bruno", &Selection::Auto, &[]).await,
            Err(SessionError::RoomNotFound)
        ));
        assert!(matches!(
            run(
                &pool,
                &other_room,
                "hi",
                &Selection::Agent(ids[0].clone()),
                &[]
            )
            .await,
            Err(SessionError::AgentNotFound)
        ));
        assert_eq!(
            run(
                &pool,
                &other_room,
                "@Bruno",
                &Selection::Auto,
                &[ids[0].clone()]
            )
            .await
            .unwrap(),
            Resolution::Nobody
        );
        assert_eq!(
            run(&pool, &other_room, "@olga", &Selection::Auto, &[])
                .await
                .unwrap(),
            Resolution::Agent {
                agent_id: outsiders[0].clone()
            }
        );
    }
}
