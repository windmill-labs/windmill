use sqlx::Postgres;

use crate::error;

#[derive(Clone)]
pub struct Mailbox {
    mailbox_id: Option<String>,
    mailbox_type: MailboxType,
    workspace_id: String,
}

pub type MsgPayload = serde_json::Value;

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct MailboxMsg {
    pub id: i64,
    pub payload: MsgPayload,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::Type, Clone, Copy)]
#[sqlx(rename_all = "snake_case", type_name = "mailbox_type")]
pub enum MailboxType {
    Trigger,
    DebouncingStaleData,
}

impl Mailbox {
    pub fn open(mailbox_id: Option<&str>, mailbox_type: MailboxType, workspace_id: &str) -> Self {
        Self {
            mailbox_id: mailbox_id.map(str::to_owned),
            mailbox_type,
            workspace_id: workspace_id.to_owned(),
        }
    }

    pub async fn push<'c>(
        &self,
        payload: MsgPayload,
        e: impl sqlx::Executor<'c, Database = Postgres>,
    ) -> error::Result<()> {
        sqlx::query!(
            r#"INSERT INTO mailbox(mailbox_id, type, payload, workspace_id) VALUES ($1, $2, $3, $4)"#,
            self.mailbox_id.as_ref(),
            self.mailbox_type as MailboxType,
            payload,
            self.workspace_id
        )
        .execute(e)
        .await?;

        Ok(())
    }

    pub async fn pull<'c>(
        &self,
        e: impl sqlx::Executor<'c, Database = Postgres>,
    ) -> error::Result<Option<MailboxMsg>> {
        sqlx::query_as!(
            MailboxMsg,
            r#"
                DELETE FROM mailbox 
                WHERE message_id = (                                                                                                                       SELECT message_id                                                                                                  ║
                    FROM mailbox                                                                                                       
                    WHERE type = $1 AND mailbox_id = $2 AND workspace_id = $3                                                          
                    LIMIT 1                                                                                                            
                )                                                                                                                      
                RETURNING payload, created_at, message_id as id;
            "#,
            self.mailbox_type as MailboxType,
            self.mailbox_id.as_ref(),
            &self.workspace_id,
        )
        .fetch_optional(e)
        .await
        .map_err(error::Error::from)
    }

    pub async fn pull_all<'c>(
        &self,
        e: impl sqlx::Executor<'c, Database = Postgres>,
    ) -> error::Result<Vec<MailboxMsg>> {
        sqlx::query_as!(
            MailboxMsg,
            r#"
                DELETE FROM mailbox 
                WHERE type = $1 AND mailbox_id IS NOT DISTINCT FROM $2 AND workspace_id = $3
                RETURNING payload, created_at, message_id as id;
            "#,
            self.mailbox_type as MailboxType,
            self.mailbox_id.as_ref(),
            &self.workspace_id,
        )
        .fetch_all(e)
        .await
        .map_err(error::Error::from)
    }

    pub async fn delete<'c>(
        &self,
        message_id: i64,
        e: impl sqlx::Executor<'c, Database = Postgres>,
    ) -> error::Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM mailbox 
                WHERE message_id = $1
                    AND workspace_id = $2
                    AND type = $3
                    AND mailbox_id IS NOT DISTINCT FROM $4 
            "#,
            message_id,
            &self.workspace_id,
            self.mailbox_type as MailboxType,
            self.mailbox_id.as_ref(),
        )
        .fetch_all(e)
        .await?;
        Ok(())
    }

    pub async fn delete_batch<'c>(
        &self,
        message_ids: Vec<i64>,
        e: impl sqlx::Executor<'c, Database = Postgres>,
    ) -> error::Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM mailbox 
                WHERE message_id = ANY($1)
                    AND workspace_id = $2
                    AND type = $3
                    AND mailbox_id IS NOT DISTINCT FROM $4 
            "#,
            &message_ids,
            &self.workspace_id,
            self.mailbox_type as MailboxType,
            self.mailbox_id.as_ref(),
        )
        .fetch_all(e)
        .await?;
        Ok(())
    }

    pub async fn read<'c>(
        &self,
        e: impl sqlx::Executor<'c, Database = Postgres>,
    ) -> error::Result<Option<MailboxMsg>> {
        sqlx::query_as!(
            MailboxMsg,
            r#"
                SELECT payload, created_at, message_id as id
                    FROM mailbox 
                    WHERE type = $1 AND mailbox_id IS NOT DISTINCT FROM $2 AND workspace_id = $3 
            "#,
            self.mailbox_type as MailboxType,
            self.mailbox_id.as_ref(),
            &self.workspace_id,
        )
        .fetch_optional(e)
        .await
        .map_err(error::Error::from)
    }

    pub async fn read_all<'c>(
        &self,
        e: impl sqlx::Executor<'c, Database = Postgres>,
    ) -> error::Result<Vec<MailboxMsg>> {
        sqlx::query_as!(
            MailboxMsg,
            r#"
                SELECT payload, created_at, message_id as id
                FROM mailbox 
                WHERE type = $1 AND mailbox_id IS NOT DISTINCT FROM $2 AND workspace_id = $3
            "#,
            self.mailbox_type as MailboxType,
            self.mailbox_id.as_ref(),
            &self.workspace_id
        )
        .fetch_all(e)
        .await
        .map_err(error::Error::from)
    }
}

#[cfg(test)]
mod mailbox_tests {
    use serde_json::json;

    use crate::mailbox::Mailbox;

    #[sqlx::test(fixtures("../../migrations/20251028105101_mailbox.up.sql"))]
    async fn test_mailbox(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
        let db = &db;
        let push = async |mbox: Mailbox| {
            mbox.push(json!(1), db).await.unwrap();
            mbox.push(json!(2), db).await.unwrap();
            mbox.push(json!(3), db).await.unwrap();
        };

        let assert_read = async |mbox: Mailbox| {
            assert_eq!(mbox.read(db).await.unwrap().unwrap().payload, json!(1));
        };

        let assert_read_all = async |mbox: Mailbox| {
            let all = mbox.read_all(db).await.unwrap();
            assert_eq!(all.len(), 3);
            assert_eq!(all[0].payload, json!(1));
            assert_eq!(all[1].payload, json!(2));
            assert_eq!(all[2].payload, json!(3));
        };

        let assert_pull = async |mbox: Mailbox| {
            assert_eq!(mbox.pull(db).await.unwrap().unwrap().payload, json!(1));
            assert_eq!(mbox.pull(db).await.unwrap().unwrap().payload, json!(2));
            assert_eq!(mbox.pull(db).await.unwrap().unwrap().payload, json!(3));
            assert!(mbox.pull(db).await.unwrap().is_none());
        };

        let assert_pull_all = async |mbox: Mailbox| {
            let all = mbox.pull_all(db).await.unwrap();
            assert_eq!(all.len(), 3);
            assert_eq!(all[0].payload, json!(1));
            assert_eq!(all[1].payload, json!(2));
            assert_eq!(all[2].payload, json!(3));
        };

        // Run those in parallel to make sure they are not conflicting
        tokio::join!(
            // Main body
            // All others will be small deviations from this one
            async {
                let mbox = Mailbox::open(
                    Some("mymailbox"),
                    crate::mailbox::MailboxType::Trigger,
                    "test-workspace_id",
                );
                push(mbox.clone()).await;
                assert_read(mbox.clone()).await;
                assert_read_all(mbox.clone()).await;
                assert_pull(mbox.clone()).await;
            },
            // Same as above, but different workspace_id
            async {
                let mbox = Mailbox::open(
                    Some("mymailbox"),
                    crate::mailbox::MailboxType::Trigger,
                    "another-workspace_id",
                );
                push(mbox.clone()).await;
                assert_read(mbox.clone()).await;
                assert_read_all(mbox.clone()).await;
                assert_pull(mbox.clone()).await;
            },
            // Different id
            async {
                let mbox = Mailbox::open(
                    Some("another id"),
                    crate::mailbox::MailboxType::Trigger,
                    "test-workspace_id",
                );
                push(mbox.clone()).await;
                assert_read(mbox.clone()).await;
                assert_read_all(mbox.clone()).await;
                assert_pull(mbox.clone()).await;
            },
            // Different kind
            async {
                let mbox = Mailbox::open(
                    Some("mymailbox"),
                    crate::mailbox::MailboxType::DebouncingStaleData,
                    "test-workspace_id",
                );
                push(mbox.clone()).await;
                assert_read(mbox.clone()).await;
                assert_read_all(mbox.clone()).await;
                assert_pull(mbox.clone()).await;
            },
            // Global mailboix
            async {
                let mbox = Mailbox::open(
                    None,
                    crate::mailbox::MailboxType::Trigger,
                    "test-workspace_id",
                );
                push(mbox.clone()).await;
                dbg!(
                    sqlx::query!("SELECT mailbox_id, payload, workspace_id FROM mailbox")
                        .fetch_all(db)
                        .await
                        .unwrap()
                );
                assert_read(mbox.clone()).await;
                assert_read_all(mbox.clone()).await;
                // Also test pull_all
                assert_pull_all(mbox.clone()).await;
            },
        );

        Ok(())
    }
}
