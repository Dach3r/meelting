use crate::domain::locale::Locale;
use crate::domain::profile::Profile;
use sqlx::SqliteExecutor;

pub async fn get<'e>(executor: impl SqliteExecutor<'e>) -> Result<Option<Profile>, sqlx::Error> {
    let row: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT locale, current_workspace_id FROM profile")
            .fetch_optional(executor)
            .await?;
    Ok(row.and_then(|(locale, current_workspace_id)| {
        Locale::parse(&locale).map(|locale| Profile {
            locale,
            current_workspace_id,
        })
    }))
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
    async fn stores_locale_and_current_workspace() {
        let (pool, _dir) = open_for_test().await;
        workspaces::insert(&pool, "01", "Acme").await.unwrap();

        create(&pool, Locale::Es, "01").await.unwrap();

        assert_eq!(
            get(&pool).await.unwrap(),
            Some(Profile {
                locale: Locale::Es,
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
