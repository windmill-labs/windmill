use crate::webhook::{ParsingRules, PayloadConstruction, SignatureLocation, SignatureParse};

use super::{
    Encoding, HmacAlgorithm, WebhookAuthenticationMethod, WebhookError, WebhookHandler,
    WebhookHmacValidator,
};
use axum::response::Response;
use http::HeaderMap;

lazy_static::lazy_static! {
    pub static ref STRIPE_WEBHOOK_VALIDATOR: WebhookHmacValidator = WebhookHmacValidator {
        prefix: None,
        payload_construction: PayloadConstruction {
            signature_location: SignatureLocation::Header(SignatureParse {
                signature_header_name: "STRIPE-SIGNATURE".to_string(),
                parsing_rules: Some(ParsingRules {
                    separators: vec![",".to_string(), "=".to_string()],
                    signature_key: "v1".to_string(),
                }),
            }),
            payload_format: vec!["#t".to_string()],
            payload_separator: Some(".".to_string()),
            include_raw_body_at_end_of_payload: true,
        },
        signature_encoding: Encoding::Hex,
        algorithm: HmacAlgorithm::Sha256,
    };
}

pub struct Stripe;

impl WebhookHandler for Stripe {
    fn handle_challenge_request<'header>(
        &self,
        _: &'header HeaderMap,
        _: &WebhookAuthenticationMethod,
        _: &str,
    ) -> Result<Option<Response>, WebhookError> {
        Ok(None)
    }
}
