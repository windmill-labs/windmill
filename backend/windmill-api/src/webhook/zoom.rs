use crate::webhook::{calculate_hmac_signature, encode_hmac_signature};

use super::{
    Encoding, HmacAlgorithm, HmacAuthenticationData, HmacAuthenticationDetails,
    TryGetWebhookHeader, WebhookAuthenticationMethod, WebhookError, WebhookHandler,
};
use axum::{body::Body, response::Response};
use http::{header, HeaderMap};
use serde::Deserialize;
use serde_json::json;
use std::borrow::Cow;
pub struct Zoom;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct ZoomPayload {
    plain_token: String,
}

#[derive(Debug, Deserialize)]
struct ZoomChallengeResponse {
    payload: ZoomPayload,
    event_ts: u64,
    event: String,
}

impl WebhookHandler for Zoom {
    fn handle_challenge_request<'header>(
        &self,
        _: &'header HeaderMap,
        authentication_method: &WebhookAuthenticationMethod,
        raw_payload: &str,
    ) -> Result<Option<Response>, WebhookError> {
        let Ok(zoom_request_body) = serde_json::from_str::<ZoomChallengeResponse>(raw_payload)
        else {
            return Ok(None);
        };

        println!("Zoom request body: {:?}", &zoom_request_body);

        if zoom_request_body.event != "endpoint.url_validation" {
            return Ok(None);
        }

        match &authentication_method {
            WebhookAuthenticationMethod::HMAC(hmac) => {
                let hmac_signature = calculate_hmac_signature(
                    HmacAlgorithm::Sha256,
                    &hmac.webhook_signing_secret,
                    &zoom_request_body.payload.plain_token,
                );

                let encoded_hmac_signature = encode_hmac_signature(Encoding::Hex, &hmac_signature);

                let response_body = json!({
                    "plainToken": zoom_request_body.payload.plain_token,
                    "encryptedToken": encoded_hmac_signature
                })
                .to_string();

                let response = Response::builder()
                    .status(200)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(response_body))
                    .unwrap();
                return Ok(Some(response));
            }
            _ => return Ok(None),
        }
    }

    fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
        &self,
        headers: &'header HeaderMap,
        raw_payload: &'payload str,
    ) -> Result<HmacAuthenticationData<'payload, 'header, 'prefix>, WebhookError> {
        let zoom_signature_header = headers.try_get_webhook_header("x-zm-signature")?;
        let zoom_timestamp_header = headers.try_get_webhook_header("x-zm-request-timestamp")?;

        let message = format!("v0:{}:{}", zoom_timestamp_header, raw_payload);

        Ok(HmacAuthenticationData::new(
            Cow::Owned(message),
            zoom_signature_header,
            Some("v0="),
            HmacAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex),
        ))
    }
}
