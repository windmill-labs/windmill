use std::fmt::Debug;

use futures_core::{future::BoxFuture, stream::BoxStream};
use rsmq_async::{RedisBytes, RsmqConnection};
use sqlx::{Postgres, Transaction};

pub enum RedisOp {
    SendMessage(RedisBytes, Option<chrono::DateTime<chrono::Utc>>),
    DeleteMessage(String),
}

impl RedisOp {
    pub async fn apply<R: RsmqConnection>(self, rsmq: &mut R) -> Result<(), rsmq_async::RsmqError> {
        match self {
            RedisOp::SendMessage(bytes, time) => {
                rsmq.send_message(
                    "main_queue",
                    bytes,
                    time.map(|t| (t - chrono::Utc::now()).num_seconds())
                        .and_then(|e| e.try_into().ok()),
                )
                .await?;
            }
            RedisOp::DeleteMessage(id) => {
                rsmq.delete_message("main_queue", &id).await?;
            }
        };

        Ok(())
    }
}

pub struct RedisTransaction<R: RsmqConnection> {
    rsmq: R,
    queued_ops: Vec<RedisOp>,
}

impl<R: RsmqConnection> From<R> for RedisTransaction<R> {
    fn from(value: R) -> Self {
        Self { rsmq: value, queued_ops: Vec::new() }
    }
}

impl<R: RsmqConnection> RedisTransaction<R> {
    pub async fn commit(self) -> Result<(), rsmq_async::RsmqError> {
        let mut rsmq = self.rsmq;
        for op in self.queued_ops {
            op.apply(&mut rsmq).await?;
        }
        Ok(())
    }

    pub fn send_message<E: Into<RedisBytes>>(
        &mut self,
        bytes: E,
        delay_until: Option<chrono::DateTime<chrono::Utc>>,
    ) {
        self.queued_ops
            .push(RedisOp::SendMessage(bytes.into(), delay_until))
    }

    pub fn delete_message(&mut self, id: String) {
        self.queued_ops.push(RedisOp::DeleteMessage(id))
    }
}

pub struct QueueTransaction<'c, R: RsmqConnection> {
    pub rsmq: Option<RedisTransaction<R>>,
    transaction: Transaction<'c, Postgres>,
}

impl<'c, R: RsmqConnection> From<(Option<R>, Transaction<'c, Postgres>)>
    for QueueTransaction<'c, R>
{
    fn from(value: (Option<R>, Transaction<'c, Postgres>)) -> Self {
        Self { rsmq: value.0.map(|e| e.into()), transaction: value.1 }
    }
}

impl<'c, R: RsmqConnection> Debug for QueueTransaction<'c, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueueTransaction")
            .field("rsmq", &self.rsmq.as_ref().map(|_| ())) // do not require R: Debug
            .field("transaction", &self.transaction)
            .finish()
    }
}

impl<'c, R: RsmqConnection> QueueTransaction<'c, R> {
    pub async fn commit(self) -> Result<(), windmill_common::error::Error> {
        self.transaction.commit().await?;
        if let Some(rsmq) = self.rsmq {
            rsmq.commit().await.map_err(|e| anyhow::anyhow!(e))?;
        }

        Ok(())
    }

    pub fn transaction_mut<'a>(&'a mut self) -> &'a mut Transaction<'c, Postgres> {
        &mut self.transaction
    }
}

impl<'c, 'b, R: RsmqConnection + Send> sqlx::Executor<'b> for &'b mut QueueTransaction<'c, R> {
    type Database = Postgres;

    fn fetch_many<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> BoxStream<
        'e,
        Result<
            sqlx::Either<
                <Self::Database as sqlx::Database>::QueryResult,
                <Self::Database as sqlx::Database>::Row,
            >,
            sqlx::Error,
        >,
    >
    where
        'b: 'e,
        E: sqlx::Execute<'q, Self::Database>,
    {
        self.transaction.fetch_many(query)
    }

    fn fetch_optional<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> BoxFuture<'e, Result<Option<<Self::Database as sqlx::Database>::Row>, sqlx::Error>>
    where
        'b: 'e,
        E: sqlx::Execute<'q, Self::Database>,
    {
        self.transaction.fetch_optional(query)
    }

    fn prepare_with<'e, 'q: 'e>(
        self,
        sql: &'q str,
        parameters: &'e [<Self::Database as sqlx::Database>::TypeInfo],
    ) -> BoxFuture<
        'e,
        Result<<Self::Database as sqlx::database::HasStatement<'q>>::Statement, sqlx::Error>,
    >
    where
        'b: 'e,
    {
        self.transaction.prepare_with(sql, parameters)
    }

    fn describe<'e, 'q: 'e>(
        self,
        sql: &'q str,
    ) -> BoxFuture<'e, Result<sqlx::Describe<Self::Database>, sqlx::Error>>
    where
        'b: 'e,
    {
        self.transaction.describe(sql)
    }
}
