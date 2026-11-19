use crate::domain::agent_run::{AgentRun, ErrorKind, RunStatus};
use crate::domain::locale::Locale;
use crate::repositories::{agents, profile};
use crate::services::session_load::SessionError;
use sqlx::SqlitePool;

#[derive(Debug, PartialEq, Eq)]
pub struct Notice {
    pub title: String,
    pub body: &'static str,
    pub sound: bool,
}

pub async fn build(pool: &SqlitePool, run: &AgentRun) -> Result<Option<Notice>, SessionError> {
    let Some(profile) = profile::get(pool).await? else {
        return Ok(None);
    };
    let (enabled, body) = match (run.status, run.error_kind) {
        (RunStatus::Succeeded, _) => (
            profile.notify_run_succeeded,
            match profile.locale {
                Locale::Es => "Terminó su respuesta",
                Locale::En => "Finished its reply",
            },
        ),
        (RunStatus::Failed, Some(ErrorKind::Cancelled | ErrorKind::Expired)) => {
            return Ok(None);
        }
        (RunStatus::Failed, _) => (
            profile.notify_run_failed,
            match profile.locale {
                Locale::Es => "No pudo responder",
                Locale::En => "Could not reply",
            },
        ),
        (RunStatus::Queued | RunStatus::Running, _) => return Ok(None),
    };
    if !enabled {
        return Ok(None);
    }
    let Some(title) = agents::name(pool, &run.agent_id).await? else {
        return Ok(None);
    };
    Ok(Some(Notice {
        title,
        body,
        sound: profile.notify_sound,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::domain::notification::NotificationSetting;
    use crate::repositories::database::open_for_test;
    use crate::services::{first_launch, notification_change};

    fn run(status: RunStatus, error_kind: Option<ErrorKind>, agent_id: &str) -> AgentRun {
        AgentRun {
            id: "r1".into(),
            room_id: "room".into(),
            agent_id: agent_id.into(),
            request_message_id: "m1".into(),
            response_message_id: None,
            status,
            error_kind,
            attempts: 1,
            provider: "openai".into(),
            model: "gpt".into(),
            input_tokens: None,
            output_tokens: None,
            created_at: "x".into(),
            started_at: None,
            finished_at: None,
        }
    }

    async fn setup(locale: Locale) -> (SqlitePool, tempfile::TempDir) {
        let (pool, dir) = open_for_test().await;
        let session = first_launch::run(&pool, locale, "Acme").await.unwrap();
        let workspace = session.current_workspace_id;
        sqlx::query("INSERT INTO teams VALUES ('t1', ?, 'General', 'x', 'x')")
            .bind(&workspace)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO agents VALUES ('a1', ?, 't1', NULL, 'Ana', 'Developer', '', '', 'openai', 'gpt', 'x', 'x')")
            .bind(&workspace)
            .execute(&pool)
            .await
            .unwrap();
        (pool, dir)
    }

    #[tokio::test]
    async fn notifies_a_finished_run_in_the_profile_language() {
        let (pool, _dir) = setup(Locale::Es).await;

        let notice = build(&pool, &run(RunStatus::Succeeded, None, "a1"))
            .await
            .unwrap();

        assert_eq!(
            notice,
            Some(Notice {
                title: "Ana".into(),
                body: "Terminó su respuesta",
                sound: true
            })
        );
    }

    #[tokio::test]
    async fn notifies_a_failed_run() {
        let (pool, _dir) = setup(Locale::En).await;

        let notice = build(
            &pool,
            &run(
                RunStatus::Failed,
                Some(ErrorKind::ProviderUnavailable),
                "a1",
            ),
        )
        .await
        .unwrap();

        assert_eq!(notice.unwrap().body, "Could not reply");
    }

    #[tokio::test]
    async fn stays_silent_when_the_matching_setting_is_off() {
        let (pool, _dir) = setup(Locale::En).await;
        notification_change::run(&pool, NotificationSetting::RunSucceeded, false)
            .await
            .unwrap();

        assert_eq!(
            build(&pool, &run(RunStatus::Succeeded, None, "a1"))
                .await
                .unwrap(),
            None
        );
        assert!(
            build(
                &pool,
                &run(RunStatus::Failed, Some(ErrorKind::Timeout), "a1")
            )
            .await
            .unwrap()
            .is_some()
        );
    }

    #[tokio::test]
    async fn carries_the_sound_preference() {
        let (pool, _dir) = setup(Locale::En).await;
        notification_change::run(&pool, NotificationSetting::Sound, false)
            .await
            .unwrap();

        let notice = build(&pool, &run(RunStatus::Succeeded, None, "a1"))
            .await
            .unwrap();

        assert!(!notice.unwrap().sound);
    }

    #[tokio::test]
    async fn ignores_cancelled_expired_and_unfinished_runs() {
        let (pool, _dir) = setup(Locale::En).await;

        for (status, kind) in [
            (RunStatus::Failed, Some(ErrorKind::Cancelled)),
            (RunStatus::Failed, Some(ErrorKind::Expired)),
            (RunStatus::Running, None),
            (RunStatus::Queued, None),
        ] {
            assert_eq!(build(&pool, &run(status, kind, "a1")).await.unwrap(), None);
        }
    }

    #[tokio::test]
    async fn ignores_a_run_of_an_unknown_agent() {
        let (pool, _dir) = setup(Locale::En).await;

        assert_eq!(
            build(&pool, &run(RunStatus::Succeeded, None, "missing"))
                .await
                .unwrap(),
            None
        );
    }
}
