use crate::catalogs::agent_roles;
use crate::domain::team::{AgentEdit, Team};
use crate::repositories::{agents, teams};
use crate::services::session_load::{self, SessionError};
use crate::services::teams_list;
use sqlx::SqlitePool;

pub async fn run(
    pool: &SqlitePool,
    agent_id: &str,
    edit: AgentEdit,
) -> Result<Vec<Team>, SessionError> {
    let edit = edit.normalize().ok_or(SessionError::InvalidAgent)?;
    if !agent_roles::exists(&edit.role) {
        return Err(SessionError::InvalidAgent);
    }
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    if !teams::exists(pool, &workspace_id, &edit.team_id).await? {
        return Err(SessionError::TeamNotFound);
    }
    if !agents::update(pool, &workspace_id, agent_id, &edit).await? {
        return Err(SessionError::AgentNotFound);
    }
    teams_list::run(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::domain::team::AgentDraft;
    use crate::repositories::database::open_for_test;
    use crate::services::{agent_create, first_launch, team_create, workspace_create};

    async fn setup(pool: &SqlitePool) -> (String, String, String) {
        first_launch::run(pool, Locale::En, "First").await.unwrap();
        team_create::run(pool, "General").await.unwrap();
        let teams = team_create::run(pool, "Platform").await.unwrap();
        let draft = AgentDraft {
            team_id: teams[0].id.clone(),
            template_key: Some("qa_engineer".into()),
            name: "Ana".into(),
            role: "QA Engineer".into(),
            description: String::new(),
            instructions: String::new(),
            provider: "anthropic".into(),
            model: "claude-sonnet-5-5".into(),
        };
        let agent_id = agent_create::run(pool, draft).await.unwrap()[0].agents[0]
            .id
            .clone();
        (agent_id, teams[0].id.clone(), teams[1].id.clone())
    }

    fn edit(team_id: &str) -> AgentEdit {
        AgentEdit {
            team_id: team_id.into(),
            name: "Bea".into(),
            role: "Software Developer".into(),
            description: "Writes code".into(),
            instructions: "Be careful".into(),
        }
    }

    #[tokio::test]
    async fn edits_the_agent_and_moves_it_to_another_team() {
        let (pool, _dir) = open_for_test().await;
        let (agent_id, _, other_team) = setup(&pool).await;

        let teams = run(&pool, &agent_id, edit(&other_team)).await.unwrap();

        assert!(teams[0].agents.is_empty());
        assert_eq!(teams[1].agents[0].name, "Bea");
        assert_eq!(teams[1].agents[0].role, "Software Developer");
    }

    #[tokio::test]
    async fn rejects_invalid_edits() {
        let (pool, _dir) = open_for_test().await;
        let (agent_id, team_id, _) = setup(&pool).await;
        let mut blank = edit(&team_id);
        blank.name = " ".into();
        let mut wizard = edit(&team_id);
        wizard.role = "Wizard".into();

        assert!(matches!(
            run(&pool, &agent_id, blank).await,
            Err(SessionError::InvalidAgent)
        ));
        assert!(matches!(
            run(&pool, &agent_id, wizard).await,
            Err(SessionError::InvalidAgent)
        ));
    }

    #[tokio::test]
    async fn rejects_unknown_agents_and_teams_of_another_workspace() {
        let (pool, _dir) = open_for_test().await;
        let (agent_id, team_id, _) = setup(&pool).await;

        assert!(matches!(
            run(&pool, "missing", edit(&team_id)).await,
            Err(SessionError::AgentNotFound)
        ));
        assert!(matches!(
            run(&pool, &agent_id, edit("missing")).await,
            Err(SessionError::TeamNotFound)
        ));
        workspace_create::run(&pool, "Second").await.unwrap();
        assert!(matches!(
            run(&pool, &agent_id, edit(&team_id)).await,
            Err(SessionError::TeamNotFound)
        ));
    }
}
