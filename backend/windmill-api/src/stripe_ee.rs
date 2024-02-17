#[cfg(feature = "stripe")]
use axum::response::Redirect;
#[cfg(feature = "stripe")]
use axum::{routing::get, Router};
#[cfg(feature = "stripe")]
use chrono::{Datelike, TimeZone, Timelike};
#[cfg(feature = "stripe")]
use std::str::FromStr;
#[cfg(feature = "stripe")]
use stripe::CustomerId;

#[cfg(feature = "stripe")]
use crate::db::{ApiAuthed, DB};

#[cfg(feature = "stripe")]
use crate::BASE_URL;
#[cfg(feature = "stripe")]
use axum::{
    extract::{Extension, Path, Query},
    Json,
};
#[cfg(feature = "stripe")]
use chrono::Utc;

#[cfg(feature = "stripe")]
use windmill_common::{
    error::{to_anyhow, Error, JsonResult, Result},
    utils::require_admin,
};

#[cfg(feature = "stripe")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "stripe")]
#[cfg(feature = "enterprise")]
lazy_static::lazy_static! {
    pub static ref STRIPE_KEY: Option<String> = std::env::var("STRIPE_KEY").ok();
}

#[cfg(feature = "stripe")]
pub fn add_stripe_routes(router: Router) -> Router {
    if STRIPE_KEY.is_none() {
        return router;
    } else {
        tracing::info!("stripe enabled");

        return router
            .route("/premium_info", get(premium_info))
            .route("/checkout", get(stripe_checkout))
            .route("/billing_portal", get(stripe_portal));
    }
}

#[cfg(feature = "stripe")]
async fn premium_info(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<PremiumWorkspaceInfo> {
    require_admin(authed.is_admin, &authed.username)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query!(
        r#"SELECT premium, usage.usage as "usage?", workspace_settings.customer_id, workspace_settings.plan FROM workspace LEFT JOIN workspace_settings ON workspace_settings.workspace_id = $1 LEFT JOIN usage ON usage.id = $1 AND month_ = EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date) AND usage.is_workspace IS true WHERE workspace.id = $1"#,
       &w_id
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    let result = PremiumWorkspaceInfo { premium: row.premium, usage: row.usage, seats: None };
    #[cfg(feature = "enterprise")]
    let mut result = result;
    #[cfg(feature = "enterprise")]
    if row.premium && row.plan == Some("team".to_string()) {
        let customer_id = row.customer_id.ok_or(Error::InternalErr(format!(
            "no customer id for workspace {}",
            w_id
        )))?;
        let client = stripe::Client::new(
            STRIPE_KEY
                .clone()
                .ok_or(Error::InternalErr(format!("stripe key not set")))?,
        );
        let customer_id = CustomerId::from_str(&customer_id).map_err(to_anyhow)?;
        let subscriptions = stripe::Subscription::list(
            &client,
            &stripe::ListSubscriptions {
                customer: Some(customer_id.clone()),
                limit: Some(1),
                ..Default::default()
            },
        )
        .await
        .map_err(to_anyhow)?;
        if subscriptions.data.len() > 1 {
            return Err(Error::InternalErr(format!("multiple subscriptions for customer {}, please contact us at ccontact@windmill.dev", customer_id)));
        }
        let subscription = subscriptions.data.get(0).ok_or_else(|| {
            Error::InternalErr(format!("no subscription for customer {}", customer_id))
        })?;
        result.seats = subscription
            .items
            .data
            .iter()
            .filter_map(|item| {
                item.price
                    .clone()
                    .map(|p| {
                        p.metadata.map(|m| {
                            m.get("plan")
                                .filter(|plan| plan == &"team")
                                .map(|_| item.quantity.map(|x| x as i32))
                        })
                    })
                    .flatten()
                    .flatten()
                    .flatten()
            })
            .collect::<Vec<_>>()
            .get(0)
            .copied();
    }

    Ok(Json(result))
}

#[cfg(feature = "stripe")]
#[derive(Deserialize)]
struct PlanQuery {
    plan: String,
    seats: Option<i32>,
}

#[cfg(feature = "stripe")]
async fn stripe_checkout(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Query(plan): Query<PlanQuery>,
    Extension(db): Extension<DB>,
) -> Result<Redirect> {
    // #[cfg(feature = "enterprise")]
    {
        require_admin(authed.is_admin, &authed.username)?;
        let client = stripe::Client::new(
            STRIPE_KEY
                .clone()
                .ok_or(Error::InternalErr(format!("stripe key not set")))?,
        );
        let base_url = BASE_URL.read().await.clone();
        let success_rd = format!("{}/workspace_settings/checkout?success=true", base_url);
        let failure_rd = format!("{}/workspace_settings/checkout?success=false", base_url);
        let checkout_session = {
            let mut params = stripe::CreateCheckoutSession::new(&success_rd);
            params.mode = Some(stripe::CheckoutSessionMode::Subscription);
            params.cancel_url = Some(&failure_rd);
            params.line_items = match plan.plan.as_str() {
                "team" => Some(vec![stripe::CreateCheckoutSessionLineItems {
                    quantity: Some(plan.seats.unwrap_or(1) as u64),
                    price: Some("price_1NCNOgGU3NdFi9eLuG4fZuEP".to_string()),
                    adjustable_quantity: Some(
                        stripe::CreateCheckoutSessionLineItemsAdjustableQuantity {
                            enabled: true,
                            minimum: Some(1),
                            ..Default::default()
                        },
                    ),
                    ..Default::default()
                }]),
                _ => Err(Error::BadRequest("invalid plan".to_string()))?,
            };
            params.client_reference_id = Some(&w_id);
            let customer_id = sqlx::query_scalar!(
                "SELECT customer_id FROM workspace_settings WHERE workspace_id = $1",
                &w_id
            )
            .fetch_one(&db)
            .await?;

            match customer_id {
                Some(customer_id) => {
                    params.customer = Some(CustomerId::from_str(&customer_id).map_err(to_anyhow)?)
                }
                _ => params.customer_email = Some(&authed.email),
            }

            let now = Utc::now();
            params.subscription_data = Some(stripe::CreateCheckoutSessionSubscriptionData {
                metadata: {
                    let mut map = std::collections::HashMap::new();
                    map.insert("workspace_id".to_string(), w_id.clone());
                    Some(map)
                },
                billing_cycle_anchor: if now.day() == 1 && now.hour() < 12 {
                    // no need to prorate so close to the billing cycle renew date
                    None
                } else {
                    // first of the next month (and possibly next year) at noon UTC
                    let date = if now.month() == 12 {
                        Utc.with_ymd_and_hms(now.year() + 1, 1, 1, 12, 0, 0)
                            .single()
                            .unwrap()
                    } else {
                        Utc.with_ymd_and_hms(now.year(), now.month() + 1, 1, 12, 0, 0)
                            .single()
                            .unwrap()
                    };
                    Some(date.timestamp())
                },
                ..Default::default()
            });

            stripe::CheckoutSession::create(&client, params)
                .await
                .map_err(to_anyhow)?
        };
        let uri = checkout_session
            .url
            .ok_or_else(|| Error::InternalErr(format!("stripe checkout redirect issue")))?;
        Ok(Redirect::to(&uri))
    }
}

#[cfg(feature = "stripe")]
async fn stripe_portal(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(db): Extension<DB>,
) -> Result<Redirect> {
    require_admin(authed.is_admin, &authed.username)?;
    let customer_id = sqlx::query_scalar!(
        "SELECT customer_id FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .fetch_one(&db)
    .await?
    .ok_or_else(|| Error::InternalErr(format!("no customer id for workspace {}", w_id)))?;
    let client = stripe::Client::new(
        STRIPE_KEY
            .clone()
            .ok_or(Error::InternalErr(format!("stripe key not set")))?,
    );
    let success_rd = format!(
        "{}/workspace_settings?tab=premium",
        BASE_URL.read().await.clone()
    );
    let portal_session = {
        let customer_id = CustomerId::from_str(&customer_id).unwrap();
        let mut params = stripe::CreateBillingPortalSession::new(customer_id);
        params.return_url = Some(&success_rd);
        stripe::BillingPortalSession::create(&client, params)
            .await
            .map_err(to_anyhow)?
    };
    Ok(Redirect::to(&portal_session.url))
}

#[cfg(feature = "stripe")]
#[derive(Serialize)]
pub struct PremiumWorkspaceInfo {
    pub premium: bool,
    pub usage: Option<i32>,
    pub seats: Option<i32>,
}

// async fn stripe_usage(
//     authed: ApiAuthed,
//     Path(w_id): Path<String>,
//     Extension(db): Extension<DB>,
//     Extension(base_url): Extension<Arc<BaseUrl>>,
// ) -> Result<Redirect> {
//     require_admin(authed.is_admin, &authed.username)?;
//     let customer_id = sqlx::query_scalar!(
//         "SELECT customer_id FROM workspace_settings WHERE workspace_id = $1",
//         w_id
//     )
//     .fetch_one(&db)
//     .await?
//     .ok_or_else(|| Error::InternalErr(format!("no customer id for workspace {}", w_id)))?;
//     let client = stripe::Client::new(std::env::var("STRIPE_KEY").expect("STRIPE_KEY"));
//     let success_rd = format!("{}/workspace_settings?tab=premium", base_url.0);
//     let portal_session = {
//         let customer_id = CustomerId::from_str(&customer_id).unwrap();
//         let subscriptions = stripe::Subscription::list(
//             &client,
//             stripe::ListSubscriptions { customer: Some(customer_id), ..Default::default() },
//         )
//         .await
//         .map_err(to_anyhow)?
//         .data[0];
//         let getUsage =
//             stripe::SubscriptionItem::list(
//                 &client,
//                 stripe::ListSubscriptionItems {
//                     subscription: subscription.id,
//                     ..Default::default()
//                 },
//             )
//             .await
//             .map_err(to_anyhow)
//         };
//         let mut params = stripe::ListSubscriptionItems::new(customer_id);
//         params.return_url = Some(&success_rd);
//         stripe::BillingPortalSession::create(&client, params)
//             .await
//             .map_err(to_anyhow)?
//     };
// }
