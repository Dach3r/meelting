use crate::domain::room::{Roster, unique_ids};
use crate::repositories::{agents, room_agents, rooms};
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(
    pool: &SqlitePool,
    room_id: &str,
    agent_ids: Vec<String>,
) -> Result<Roster, SessionError> {
    let agent_ids = unique_ids(agent_ids);
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    if rooms::get(&mut *transaction, &workspace_id, room_id)
        .await?
        .is_none()
    {
        return Err(SessionError::RoomNotFound);
    }
    for agent_id in &agent_ids {
        if !agents::exists(&mut *transaction, &workspace_id, agent_id).await? {
            return Err(SessionError::AgentNotFound);
        }
        room_agents::insert(&mut *transaction, room_id, agent_id, &workspace_id).await?;
    }
    transaction.commit().await?;
    roster(pool, &workspace_id, room_id).await
}

pub async fn roster(
    pool: &SqlitePool,
    workspace_id: &str,
    room_id: &str,
) -> Result<Roster, SessionError> {
    let mut transaction = pool.begin().await?;
    let agents = room_agents::list(&mut *transaction, workspace_id)
        .await?
        .into_iter()
        .filter(|(room, _)| room == room_id)
        .map(|(_, participant)| participant)
        .collect();
    let former_agents = room_agents::former(&mut *transaction, workspace_id, room_id).await?;
    transaction.commit().await?;
    Ok(Roster {
        agents,
        former_agents,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::domain::team::AgentDraft;
    use crate::repositories::database::open_for_test;
    use crate::services::{
        agent_create, first_launch, room_create, team_create, workspace_create, workspace_switch,
    };

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
        agent_create::run(pool, draft)
            .await
            .unwrap()
            .into_iter()
            .flat_map(|team| team.agents)
            .find(|agent| agent.name == name)
            .unwrap()
            .id
    }

    fn names(participants: &[crate::domain::room::Participant]) -> Vec<&str> {
        participants
            .iter()
            .map(|agent| agent.name.as_str())
            .collect()
    }

    #[tokio::test]
    async fn adds_agents_to_an_existing_room() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let ana = hire(&pool, "Ana").await;
        let bob = hire(&pool, "Bob").await;
        let room = room_create::run(&pool, "Daily", vec![ana.clone()])
            .await
            .unwrap();

        let roster = run(&pool, &room.id, vec![bob.clone(), ana, bob])
            .await
            .unwrap();

        assert_eq!(names(&roster.agents), ["Ana", "Bob"]);
        assert!(roster.former_agents.is_empty());
    }

    #[tokio::test]
    async fn rejects_agents_of_another_workspace_without_adding_anyone() {
        let (pool, _dir) = open_for_test().await;
        let session = first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let ana = hire(&pool, "Ana").await;
        let room = room_create::run(&pool, "Daily", vec![]).await.unwrap();
        workspace_create::run(&pool, "Globex").await.unwrap();
        let eva = hire(&pool, "Eva").await;
        workspace_switch::run(&pool, &session.current_workspace_id)
            .await
            .unwrap();

        assert!(matches!(
            run(&pool, &room.id, vec![ana, eva]).await,
            Err(SessionError::AgentNotFound)
        ));
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM room_agents")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn rejects_rooms_of_another_workspace() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let room = room_create::run(&pool, "Daily", vec![]).await.unwrap();
        workspace_create::run(&pool, "Globex").await.unwrap();
        let eva = hire(&pool, "Eva").await;

        assert!(matches!(
            run(&pool, &room.id, vec![eva]).await,
            Err(SessionError::RoomNotFound)
        ));
    }
}
