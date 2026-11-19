use crate::domain::locale::Locale;
use crate::domain::profile::Profile;
use crate::domain::theme::Theme;
use crate::domain::ui_scale::UiScale;
use sqlx::SqliteExecutor;

pub async fn get<'e>(executor: impl SqliteExecutor<'e>) -> Result<Option<Profile>, sqlx::Error> {
    let row: Option<(String, String, u16, Option<String>)> =
        sqlx::query_as("SELECT locale, theme, ui_scale, current_workspace_id FROM profile")
            .fetch_optional(executor)
            .await?;
    Ok(
        row.and_then(|(locale, theme, ui_scale, current_workspace_id)| {
            Some(Profile {
                locale: Locale::parse(&locale)?,
                theme: Theme::parse(&theme)?,
                ui_scale: UiScale::from_percent(ui_scale)?,
                current_workspace_id,
            })
        }),
    )
}

pub async fn create<'e>(
    executor: impl SqliteExecutor<'e>,
    locale: Locale,
    current_workspace_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO profile (id, locale, current_workspace_id, created_at, updated_at) \
         VALUES ('singleton', ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
    )
    .bind(locale.as_str())
    .bind(current_workspace_id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn set_current_workspace<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE profile SET current_workspace_id = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
    )
    .bind(workspace_id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn set_locale<'e>(
    executor: impl SqliteExecutor<'e>,
    locale: Locale,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE profile SET locale = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
    )
    .bind(locale.as_str())
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn set_theme<'e>(
    executor: impl SqliteExecutor<'e>,
    theme: Theme,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE profile SET theme = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')")
        .bind(theme.as_str())
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn set_ui_scale<'e>(
    executor: impl SqliteExecutor<'e>,
    ui_scale: UiScale,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE profile SET ui_scale = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
    )
    .bind(ui_scale.percent())
    .execute(executor)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::{database::open_for_test, workspaces};

    #[tokio::test]
    async fn is_empty_before_first_launch() {
        let (pool, _dir) = open_for_test().await;

        assert_eq!(get(&pool).await.unwrap(), None);
    }

    #[tokio::test]
    async fn stores_locale_theme_and_current_workspace() {
        let (pool, _dir) = open_for_test().await;
        workspaces::insert(&pool, "01", "Acme").await.unwrap();

        create(&pool, Locale::Es, "01").await.unwrap();

        assert_eq!(
            get(&pool).await.unwrap(),
            Some(Profile {
                locale: Locale::Es,
                theme: Theme::System,
                ui_scale: UiScale::default(),
                current_workspace_id: Some("01".into())
            })
        );
    }

    #[tokio::test]
    async fn switches_current_workspace() {
        let (pool, _dir) = open_for_test().await;
        workspaces::insert(&pool, "01", "A").await.unwrap();
        workspaces::insert(&pool, "02", "B").await.unwrap();
        create(&pool, Locale::En, "01").await.unwrap();

        set_current_workspace(&pool, "02").await.unwrap();

        let profile = get(&pool).await.unwrap().unwrap();
        assert_eq!(profile.current_workspace_id, Some("02".into()));
    }

    #[tokio::test]
    async fn changes_locale_and_theme() {
        let (pool, _dir) = open_for_test().await;
        workspaces::insert(&pool, "01", "Acme").await.unwrap();
        create(&pool, Locale::En, "01").await.unwrap();

        set_locale(&pool, Locale::Es).await.unwrap();
        set_theme(&pool, Theme::Dark).await.unwrap();

        let profile = get(&pool).await.unwrap().unwrap();
        assert_eq!(profile.locale, Locale::Es);
        assert_eq!(profile.theme, Theme::Dark);
    }

    #[tokio::test]
    async fn changes_ui_scale() {
        let (pool, _dir) = open_for_test().await;
        workspaces::insert(&pool, "01", "Acme").await.unwrap();
        create(&pool, Locale::En, "01").await.unwrap();

        set_ui_scale(&pool, UiScale::from_percent(150).unwrap())
            .await
            .unwrap();

        let profile = get(&pool).await.unwrap().unwrap();
        assert_eq!(profile.ui_scale.percent(), 150);
    }

    #[tokio::test]
    async fn allows_only_one_profile_row() {
        let (pool, _dir) = open_for_test().await;
        workspaces::insert(&pool, "01", "Acme").await.unwrap();
        create(&pool, Locale::En, "01").await.unwrap();

        assert!(create(&pool, Locale::Es, "01").await.is_err());
    }

    #[tokio::test]
    async fn rejects_unknown_workspace() {
        let (pool, _dir) = open_for_test().await;

        assert!(create(&pool, Locale::En, "missing").await.is_err());
    }
}
