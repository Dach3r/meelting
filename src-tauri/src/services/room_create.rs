use crate::domain::room::{Room, unique_ids};
use crate::domain::workspace::normalize_name;
use crate::repositories::{agents, room_agents, rooms};
use crate::services::rooms_list::with_agents;
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn run(
    pool: &SqlitePool,
    room_name: &str,
    agent_ids: Vec<String>,
) -> Result<Room, SessionError> {
    let name = normalize_name(room_name).ok_or(SessionError::InvalidName)?;
    let agent_ids = unique_ids(agent_ids);
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let id = Uuid::now_v7().to_string();
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    for agent_id in &agent_ids {
        if !agents::exists(&mut *transaction, &workspace_id, agent_id).await? {
            return Err(SessionError::AgentNotFound);
        }
    }
    rooms::insert(&mut *transaction, &id, &workspace_id, &name).await?;
    for agent_id in &agent_ids {
        room_agents::insert(&mut *transaction, &id, agent_id, &workspace_id).await?;
    }
    transaction.commit().await?;
    let row = rooms::get(pool, &workspace_id, &id)
        .await?
        .ok_or(SessionError::RoomNotFound)?;
    let participants = room_agents::list(pool, &workspace_id).await?;
    Ok(with_agents(vec![row], participants).remove(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::domain::team::AgentDraft;
    use crate::repositories::database::open_for_test;
    use crate::services::{
        agent_create, first_launch, rooms_list, team_create, workspace_create, workspace_switch,
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

    #[tokio::test]
    async fn creates_a_room_with_its_agents() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let ana = hire(&pool, "Ana").await;
        let bob = hire(&pool, "Bob").await;

        let room = run(&pool, "  Daily  ", vec![bob.clone(), ana.clone()])
            .await
            .unwrap();

        assert_eq!(room.name, "Daily");
        let ids: Vec<&str> = room.agents.iter().map(|agent| agent.id.as_str()).collect();
        assert_eq!(ids, [ana.as_str(), bob.as_str()]);
        assert_eq!(rooms_list::run(&pool).await.unwrap().rooms, [room]);
    }

    #[tokio::test]
    async fn allows_a_room_without_agents() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();

        let room = run(&pool, "Solo", vec![]).await.unwrap();

        assert!(room.agents.is_empty());
    }

    #[tokio::test]
    async fn ignores_repeated_agents() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let ana = hire(&pool, "Ana").await;

        let room = run(&pool, "Daily", vec![ana.clone(), ana.clone()])
            .await
            .unwrap();

        assert_eq!(room.agents.len(), 1);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM room_agents").await, 1);
    }

    #[tokio::test]
    async fn rejects_invalid_names_without_writing() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();

        assert!(matches!(
            run(&pool, "   ", vec![]).await,
            Err(SessionError::InvalidName)
        ));
        assert!(matches!(
            run(&pool, &"a".repeat(81), vec![]).await,
            Err(SessionError::InvalidName)
        ));
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM rooms").await, 0);
    }

    #[tokio::test]
    async fn rejects_agents_of_another_workspace_without_writing_anything() {
        let (pool, _dir) = open_for_test().await;
        let session = first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let ana = hire(&pool, "Ana").await;
        workspace_create::run(&pool, "Globex").await.unwrap();
        let eva = hire(&pool, "Eva").await;
        workspace_switch::run(&pool, &session.current_workspace_id)
            .await
            .unwrap();

        for agent_ids in [vec![ana.clone(), eva.clone()], vec![ana, "missing".into()]] {
            assert!(matches!(
                run(&pool, "Daily", agent_ids).await,
                Err(SessionError::AgentNotFound)
            ));
        }
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM rooms").await, 0);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM room_agents").await, 0);
    }
}
