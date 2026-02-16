use axum::{routing::post, Router};
use serde::{Deserialize, Serialize};
use windmill_common::{error::JsonResult, DB};

#[allow(unused_imports)]
use windmill_trigger::handler::complete_trigger_routes;
#[allow(unused_imports)]
use windmill_trigger::TriggerCrud;

pub fn generate_trigger_routers() -> Router {
    #[allow(unused_mut)]
    let mut router = Router::new();

    #[cfg(feature = "http_trigger")]
    {
        use crate::triggers::http::HttpTrigger;

        router = router.nest(
            HttpTrigger::ROUTE_PREFIX,
            complete_trigger_routes(HttpTrigger),
        );
    }

    #[cfg(feature = "websocket")]
    {
        use crate::triggers::websocket::WebsocketTrigger;

        router = router.nest(
            WebsocketTrigger::ROUTE_PREFIX,
            complete_trigger_routes(WebsocketTrigger),
        );
    }

    #[cfg(all(feature = "enterprise", feature = "kafka", feature = "private"))]
    {
        use crate::triggers::kafka::KafkaTrigger;

        router = router.nest(
            KafkaTrigger::ROUTE_PREFIX,
            complete_trigger_routes(KafkaTrigger),
        );
    }

    #[cfg(all(feature = "enterprise", feature = "nats", feature = "private"))]
    {
        use crate::triggers::nats::NatsTrigger;

        router = router.nest(
            NatsTrigger::ROUTE_PREFIX,
            complete_trigger_routes(NatsTrigger),
        );
    }

    #[cfg(feature = "mqtt_trigger")]
    {
        use crate::triggers::mqtt::MqttTrigger;

        router = router.nest(
            MqttTrigger::ROUTE_PREFIX,
            complete_trigger_routes(MqttTrigger),
        );
    }

    #[cfg(all(feature = "enterprise", feature = "sqs_trigger", feature = "private"))]
    {
        use crate::triggers::sqs::SqsTrigger;

        router = router.nest(
            SqsTrigger::ROUTE_PREFIX,
            complete_trigger_routes(SqsTrigger),
        );
    }

    #[cfg(all(feature = "enterprise", feature = "gcp_trigger", feature = "private"))]
    {
        use crate::triggers::gcp::GcpTrigger;

        router = router.nest(
            GcpTrigger::ROUTE_PREFIX,
            complete_trigger_routes(GcpTrigger),
        );
    }

    #[cfg(feature = "postgres_trigger")]
    {
        use crate::triggers::postgres::PostgresTrigger;

        router = router.nest(
            PostgresTrigger::ROUTE_PREFIX,
            complete_trigger_routes(PostgresTrigger),
        );
    }

    #[cfg(all(feature = "smtp", feature = "private"))]
    {
        use crate::triggers::email::EmailTrigger;

        router = router.nest(
            EmailTrigger::ROUTE_PREFIX,
            complete_trigger_routes(EmailTrigger),
        );
    }

    {
        use windmill_trigger::global_handler::{
            cancel_suspended_trigger_jobs, resume_suspended_trigger_jobs,
        };

        router = router
            .route(
                "/trigger/:trigger_kind/resume_suspended_trigger_jobs/*trigger_path",
                post(resume_suspended_trigger_jobs),
            )
            .route(
                "/trigger/:trigger_kind/cancel_suspended_trigger_jobs/*trigger_path",
                post(cancel_suspended_trigger_jobs),
            );
    }

    router
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TriggersCount {
    primary_schedule: Option<windmill_trigger::handler::TriggerPrimarySchedule>,
    schedule_count: i64,
    http_routes_count: i64,
    webhook_count: i64,
    email_count: i64,
    default_email_count: i64,
    websocket_count: i64,
    kafka_count: i64,
    nats_count: i64,
    postgres_count: i64,
    mqtt_count: i64,
    sqs_count: i64,
    gcp_count: i64,
    nextcloud_count: i64,
    google_count: i64,
}

pub async fn get_triggers_count_internal(
    db: &DB,
    w_id: &str,
    path: &str,
    is_flow: bool,
) -> JsonResult<TriggersCount> {
    let primary_schedule = sqlx::query_scalar!(
        "SELECT schedule FROM schedule WHERE path = $1 AND script_path = $1 AND is_flow = $2 AND workspace_id = $3",
        path,
        is_flow,
        w_id
    )
    .fetch_optional(db)
    .await?;

    let schedule_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM schedule WHERE script_path = $1 AND is_flow = $2 AND workspace_id = $3",
        path,
        is_flow,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    #[allow(unused)]
    let mut tx = db.begin().await?;

    #[cfg(feature = "http_trigger")]
    let http_routes_count = {
        use crate::triggers::http::HttpTrigger;
        let count = HttpTrigger
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await;
        count
    };
    #[cfg(not(feature = "http_trigger"))]
    let http_routes_count = 0;

    #[cfg(feature = "websocket")]
    let websocket_count = {
        use crate::triggers::websocket::WebsocketTrigger;
        let count = WebsocketTrigger
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await;
        count
    };
    #[cfg(not(feature = "websocket"))]
    let websocket_count = 0;

    #[cfg(all(feature = "kafka", feature = "enterprise", feature = "private"))]
    let kafka_count = {
        use crate::triggers::kafka::KafkaTrigger;
        let count = KafkaTrigger
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await;
        count
    };
    #[cfg(not(all(feature = "kafka", feature = "enterprise", feature = "private")))]
    let kafka_count = 0;

    #[cfg(all(feature = "nats", feature = "enterprise", feature = "private"))]
    let nats_count = {
        use crate::triggers::nats::NatsTrigger;
        let count = NatsTrigger
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await;
        count
    };
    #[cfg(not(all(feature = "nats", feature = "enterprise", feature = "private")))]
    let nats_count = 0;

    #[cfg(feature = "postgres_trigger")]
    let postgres_count = {
        use crate::triggers::postgres::PostgresTrigger;
        let count = PostgresTrigger
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await;
        count
    };
    #[cfg(not(feature = "postgres_trigger"))]
    let postgres_count = 0;

    #[cfg(feature = "mqtt_trigger")]
    let mqtt_count = {
        use crate::triggers::mqtt::MqttTrigger;
        let count = MqttTrigger
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await;
        count
    };
    #[cfg(not(feature = "mqtt_trigger"))]
    let mqtt_count = 0;

    #[cfg(all(feature = "sqs_trigger", feature = "enterprise", feature = "private"))]
    let sqs_count = {
        use crate::triggers::sqs::SqsTrigger;
        let count = SqsTrigger.trigger_count(&mut tx, w_id, is_flow, path).await;
        count
    };
    #[cfg(not(all(feature = "sqs_trigger", feature = "enterprise", feature = "private")))]
    let sqs_count = 0;

    #[cfg(all(feature = "gcp_trigger", feature = "enterprise", feature = "private"))]
    let gcp_count = {
        use crate::triggers::gcp::GcpTrigger;
        let count = GcpTrigger.trigger_count(&mut tx, w_id, is_flow, path).await;
        count
    };
    #[cfg(not(all(feature = "gcp_trigger", feature = "enterprise", feature = "private")))]
    let gcp_count = 0;

    #[cfg(all(feature = "smtp", feature = "enterprise", feature = "private"))]
    let email_count = {
        use crate::triggers::email::EmailTrigger;
        let count = EmailTrigger
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await;
        count
    };
    #[cfg(not(all(feature = "smtp", feature = "enterprise", feature = "private")))]
    let email_count = 0;

    tx.commit().await?;

    let webhook_count = (if is_flow {
        sqlx::query_scalar!(
            "SELECT COUNT(*) FROM token WHERE label LIKE 'webhook-%' AND workspace_id = $1 AND scopes @> ARRAY['run:flow/' || $2]::text[]",
            w_id,
            path,
        )
    } else {
        sqlx::query_scalar!(
            "SELECT COUNT(*) FROM token WHERE label LIKE 'webhook-%' AND workspace_id = $1 AND scopes @> ARRAY['run:' || $2]::text[]",
            w_id,
            path,
        )
    }).fetch_one(db)
    .await?
    .unwrap_or(0);

    let default_email_count = (if is_flow {
        sqlx::query_scalar!(
            "SELECT COUNT(*) FROM token WHERE label LIKE 'email-%' AND workspace_id = $1 AND scopes @> ARRAY['run:flow/' || $2]::text[]",
            w_id,
            path,
        )
    } else {
        sqlx::query_scalar!(
            "SELECT COUNT(*) FROM token WHERE label LIKE 'email-%' AND workspace_id = $1 AND scopes @> ARRAY['run:script/' || $2]::text[]",
            w_id,
            path,
        )
    }).fetch_one(db)
    .await?
    .unwrap_or(0);

    let nextcloud_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM native_trigger WHERE workspace_id = $1 AND script_path = $2 AND is_flow = $3 AND service_name = 'nextcloud'",
        w_id,
        path,
        is_flow,
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    let google_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM native_trigger WHERE workspace_id = $1 AND script_path = $2 AND is_flow = $3 AND service_name = 'google'",
        w_id,
        path,
        is_flow,
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    Ok(axum::Json(TriggersCount {
        primary_schedule: primary_schedule
            .map(|s| windmill_trigger::handler::TriggerPrimarySchedule { schedule: s }),
        schedule_count,
        http_routes_count,
        webhook_count,
        default_email_count,
        email_count,
        websocket_count,
        kafka_count,
        nats_count,
        postgres_count,
        mqtt_count,
        gcp_count,
        sqs_count,
        nextcloud_count,
        google_count,
    }))
}
