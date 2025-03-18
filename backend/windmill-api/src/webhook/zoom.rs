use crate::webhook::{
    calculate_hmac_signature, encode_hmac_signature, PayloadConstruction, SignatureLocation,
    SignatureParse,
};

use super::{
    Encoding, HmacAlgorithm, WebhookAuthenticationMethod, WebhookError, WebhookHandler,
    WebhookHmacValidator,
};
use axum::{body::Body, response::Response};
use http::{header, HeaderMap};
use serde::Deserialize;
use serde_json::json;

lazy_static::lazy_static! {
    pub static ref ZOOM_WEBHOOK_VALIDATOR: WebhookHmacValidator = WebhookHmacValidator {
        prefix: Some("v0=".to_string()),
        payload_construction: PayloadConstruction {
            signature_location: SignatureLocation::Header(SignatureParse {
                signature_header_name: "x-zm-signature".to_string(),
                parsing_rules: None,
            }),
            payload_format: vec!["#v0".to_string(), "x-zm-request-timestamp".to_string()],
            payload_separator: Some(":".to_string()),
            include_raw_body_at_end_of_payload: true,
        },
        signature_encoding: Encoding::Hex,
        algorithm: HmacAlgorithm::Sha256,
    };
}

pub struct Zoom;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct ZoomPayload {
    plain_token: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
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
}
