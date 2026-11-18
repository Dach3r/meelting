use crate::domain::room::Message;
use sqlx::SqliteExecutor;

type MessageRow = (String, String, Option<String>, String, String);

fn to_message((id, room_id, agent_id, body, created_at): MessageRow) -> Message {
    Message {
        id,
        room_id,
        agent_id,
        body,
        created_at,
    }
}

pub async fn insert_from_user<'e>(
    executor: impl SqliteExecutor<'e>,
    id: &str,
    room_id: &str,
    body: &str,
) -> Result<Message, sqlx::Error> {
    let row: MessageRow = sqlx::query_as(
        "INSERT INTO messages (id, room_id, agent_id, body, created_at) \
         VALUES (?, ?, NULL, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) \
         RETURNING id, room_id, agent_id, body, created_at",
    )
    .bind(id)
    .bind(room_id)
    .bind(body)
    .fetch_one(executor)
    .await?;
    Ok(to_message(row))
}

pub async fn recent<'e>(
    executor: impl SqliteExecutor<'e>,
    room_id: &str,
    limit: i64,
) -> Result<Vec<Message>, sqlx::Error> {
    let rows: Vec<MessageRow> = sqlx::query_as(
        "SELECT id, room_id, agent_id, body, created_at FROM ( \
             SELECT id, room_id, agent_id, body, created_at FROM messages \
             WHERE room_id = ? ORDER BY created_at DESC, id DESC LIMIT ? \
         ) ORDER BY created_at, id",
    )
    .bind(room_id)
    .bind(limit)
    .fetch_all(executor)
    .await?;
    Ok(rows.into_iter().map(to_message).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::database::open_for_test;
    use sqlx::SqlitePool;

    async fn seed(pool: &SqlitePool) {
        for sql in [
            "INSERT INTO workspaces VALUES ('w1', 'A', 'x', 'x')",
            "INSERT INTO teams VALUES ('t1', 'w1', 'General', 'x', 'x')",
            "INSERT INTO agents VALUES ('a1', 'w1', 't1', NULL, 'Ana', 'QA', '', '', 'openai', 'gpt', 'x', 'x'), ('a2', 'w1', 't1', NULL, 'Bob', 'Dev', '', '', 'openai', 'gpt', 'x', 'x')",
            "INSERT INTO rooms VALUES ('r1', 'w1', 'Sala', 'x', 'x'), ('r2', 'w1', 'Otra', 'x', 'x')",
            "INSERT INTO room_agents VALUES ('r1', 'a1', 'w1', 'x'), ('r2', 'a2', 'w1', 'x')",
        ] {
            sqlx::query(sql).execute(pool).await.unwrap();
        }
    }

    async fn insert_raw(
        pool: &SqlitePool,
        id: &str,
        room_id: &str,
        agent_id: Option<&str>,
        body: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO messages VALUES (?, ?, ?, ?, 'x')")
            .bind(id)
            .bind(room_id)
            .bind(agent_id)
            .bind(body)
            .execute(pool)
            .await
            .map(|_| ())
    }

    async fn count(pool: &SqlitePool, sql: &'static str) -> i64 {
        sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
    }

    #[tokio::test]
    async fn inserts_a_user_message_without_author() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;

        let message = insert_from_user(&pool, "m1", "r1", "Hola").await.unwrap();

        assert_eq!(message.agent_id, None);
        assert_eq!(message.body, "Hola");
        assert_eq!(recent(&pool, "r1", 100).await.unwrap(), [message]);
    }

    #[tokio::test]
    async fn returns_the_latest_messages_in_chronological_order() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;
        sqlx::query("INSERT INTO messages VALUES ('m1', 'r1', NULL, 'one', '1'), ('m3', 'r1', NULL, 'three', '2'), ('m2', 'r1', NULL, 'two', '2'), ('m4', 'r2', NULL, 'other', '9')")
            .execute(&pool)
            .await
            .unwrap();

        let ids: Vec<String> = recent(&pool, "r1", 2)
            .await
            .unwrap()
            .into_iter()
            .map(|message| message.id)
            .collect();

        assert_eq!(ids, ["m2", "m3"]);
    }

    #[tokio::test]
    async fn limits_user_messages_to_4000_characters_but_not_agent_messages() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;

        assert!(
            insert_raw(&pool, "m1", "r1", None, &"ñ".repeat(4000))
                .await
                .is_ok()
        );
        assert!(
            insert_raw(&pool, "m2", "r1", None, &"a".repeat(4001))
                .await
                .is_err()
        );
        assert!(
            insert_raw(&pool, "m3", "r1", Some("a1"), &"a".repeat(5000))
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn rejects_blank_bodies() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;

        assert!(insert_raw(&pool, "m1", "r1", None, "   ").await.is_err());
        assert!(insert_raw(&pool, "m2", "r1", Some("a1"), "").await.is_err());
    }

    #[tokio::test]
    async fn rejects_agent_messages_in_rooms_the_agent_is_not_in() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;

        assert!(
            insert_raw(&pool, "m1", "r1", Some("a2"), "hi")
                .await
                .is_err()
        );
        assert!(
            insert_raw(&pool, "m2", "r1", Some("missing"), "hi")
                .await
                .is_err()
        );
        assert!(
            insert_raw(&pool, "m3", "missing", None, "hi")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn deleting_a_room_removes_its_participants_and_messages_only() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;
        insert_raw(&pool, "m1", "r1", None, "hi").await.unwrap();
        insert_raw(&pool, "m2", "r1", Some("a1"), "hello")
            .await
            .unwrap();
        insert_raw(&pool, "m3", "r2", Some("a2"), "other")
            .await
            .unwrap();

        sqlx::query("DELETE FROM rooms WHERE id = 'r1'")
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(count(&pool, "SELECT COUNT(*) FROM messages").await, 1);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM room_agents").await, 1);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM agents").await, 2);
        let violations: Vec<(String,)> =
            sqlx::query_as("SELECT \"table\" FROM pragma_foreign_key_check")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert!(violations.is_empty());
    }

    #[tokio::test]
    async fn an_agent_with_messages_cannot_be_deleted_nor_leave_the_room() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;
        insert_raw(&pool, "m1", "r1", Some("a1"), "hello")
            .await
            .unwrap();

        assert!(
            sqlx::query("DELETE FROM agents WHERE id = 'a1'")
                .execute(&pool)
                .await
                .is_err()
        );
        assert!(
            sqlx::query("DELETE FROM room_agents WHERE agent_id = 'a1'")
                .execute(&pool)
                .await
                .is_err()
        );
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM agents").await, 2);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM room_agents").await, 2);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM messages").await, 1);
    }

    #[tokio::test]
    async fn user_messages_do_not_block_deleting_an_agent() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;
        insert_raw(&pool, "m1", "r1", None, "hi").await.unwrap();

        sqlx::query("DELETE FROM agents WHERE id = 'a1'")
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(count(&pool, "SELECT COUNT(*) FROM messages").await, 1);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM room_agents").await, 1);
    }
}
