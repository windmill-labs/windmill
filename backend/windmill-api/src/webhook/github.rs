use super::{
    Encoding, HmacAlgorithm, HmacAuthenticationData, HmacAuthenticationDetails,
    TryGetWebhookHeader, WebhookAuthenticationMethod, WebhookError, WebhookHandler,
};
use axum::response::Response;
use http::HeaderMap;
use std::borrow::Cow;

pub struct Github;

impl WebhookHandler for Github {
    fn handle_challenge_request<'header>(
        &self,
        _: &'header HeaderMap,
        _: &WebhookAuthenticationMethod,
        _: &str,
    ) -> Result<Option<Response>, WebhookError> {
        Ok(None)
    }

    fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
        &self,
        headers: &'header HeaderMap,
        raw_payload: &'payload str,
    ) -> Result<HmacAuthenticationData<'payload, 'header, 'prefix>, WebhookError> {
        let github_secret_header = headers.try_get_webhook_header("X-Hub-Signature-256")?;

        let authentication_data = HmacAuthenticationData::new(
            Cow::Borrowed(raw_payload),
            github_secret_header,
            Some("sha256"),
            HmacAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex),
        );

        Ok(authentication_data)
    }
}
