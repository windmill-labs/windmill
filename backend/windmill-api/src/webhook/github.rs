use http::HeaderMap;

use super::{HmacAuthenticationData, TryGetWebhookHeader, WebhookAuthenticationData, WebhookError};

pub struct Github;

impl WebhookAuthenticationData for Github {
    fn get_authentication_data<'header>(
        &self,
        headers: &'header HeaderMap,
        raw_payload: &str,
    ) -> Result<HmacAuthenticationData<'header>, WebhookError> {
        let github_secret_header = headers.try_get_webhook_header("X-Hub-Signature-256")?;

        Ok(HmacAuthenticationData::new(
            raw_payload.to_string(),
            github_secret_header,
            Some("sha256"),
        ))
    }
}
