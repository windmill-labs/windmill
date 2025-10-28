use sqlx::Postgres;

use crate::error;

pub struct Mailbox {
    mailbox_id: Option<String>,
    mailbox_type: MailboxType,
}

pub type MsgPayload = serde_json::Value;

#[derive(sqlx::FromRow)]
pub struct MailboxMsg {
    payload: MsgPayload,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::Type, Clone, Copy)]
#[sqlx(rename_all = "snake_case", type_name = "mailbox_type")]
pub enum MailboxType {
    Trigger,
    DebouncingStaleData,
}

impl Mailbox {
    pub fn open(mailbox_id: Option<&str>, mailbox_type: MailboxType) -> Self {
        Self { mailbox_id: mailbox_id.map(str::to_owned), mailbox_type }
    }
    pub async fn push<'c>(
        &self,
        payload: MsgPayload,
        e: impl sqlx::Executor<'c, Database = Postgres>,
    ) -> error::Result<()> {
        sqlx::query!(
            r#"INSERT INTO mailbox(mailbox_id, type, payload) VALUES ($1, $2, $3)"#,
            self.mailbox_id.as_ref(),
            self.mailbox_type as MailboxType,
            payload
        )
        .execute(e)
        .await?;

        Ok(())
    }
    pub async fn pull<'c>(
        &self,
        e: impl sqlx::Executor<'c, Database = Postgres>,
    ) -> error::Result<Option<MailboxMsg>> {
        Ok(sqlx::query_as!(
            MailboxMsg,
            r#"DELETE FROM mailbox WHERE type = $1 AND mailbox_id = $2 RETURNING (message_id, created_at)"#,
            self.mailbox_type as MailboxType,
            self.mailbox_id.as_ref(),
        )
        .fetch_optional(e)
        .await?)
    }
    pub async fn pull_all() {}
}
