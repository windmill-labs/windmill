use crate::webhook::{PayloadConstruction, SignatureLocation, SignatureParse};

use super::{
    Encoding, HmacAlgorithm, WebhookAuthenticationMethod, WebhookError, WebhookHandler,
    WebhookHmacValidator,
};
use axum::response::Response;
use http::HeaderMap;

lazy_static::lazy_static! {
    pub static ref SLACK_WEBHOOK_VALIDATOR: WebhookHmacValidator = WebhookHmacValidator {
        prefix: Some("v0=".to_string()),
        payload_construction: PayloadConstruction {
            signature_location: SignatureLocation::Header(SignatureParse {
                signature_header_name: "X-Slack-Signature".to_string(),
                parsing_rules: None,
            }),
            payload_format: vec!["#v0".to_string(), "X-Slack-Request-Timestamp".to_string()],
            payload_separator: Some(":".to_string()),
            include_raw_body_at_end_of_payload: true,
        },
        signature_encoding: Encoding::Hex,
        algorithm: HmacAlgorithm::Sha256,
    };
}

pub struct Slack;

impl WebhookHandler for Slack {
    fn handle_challenge_request<'header>(
        &self,
        _: &'header HeaderMap,
        _: &WebhookAuthenticationMethod,
        _: &str,
    ) -> Result<Option<Response>, WebhookError> {
        Ok(None)
    }
}
