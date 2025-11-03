use anyhow::Context;
use reqwest::{Body, Response};
use serde::de::DeserializeOwned;

use crate::utils::HTTP_CLIENT;

#[derive(Clone)]
pub struct AuthedClient {
    pub base_internal_url: String,
    pub workspace: String,
    pub token: String,
    pub force_client: Option<reqwest::Client>,
}

impl AuthedClient {
    pub fn new(
        base_internal_url: String,
        workspace: String,
        token: String,
        force_client: Option<reqwest::Client>,
    ) -> AuthedClient {
        AuthedClient { base_internal_url, workspace, token, force_client }
    }

    pub async fn get(&self, url: &str, query: Vec<(&str, String)>) -> anyhow::Result<Response> {
        self.force_client
            .as_ref()
            .unwrap_or(&HTTP_CLIENT)
            .get(url)
            .query(&query)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .header(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.token))?,
            )
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Error executing get request from authed http client to {url} with query {query:?}: {e:#?}");
                anyhow::anyhow!("Error executing get request from authed http client to {url} with query {query:?}: {e:#?}")
            })
    }

    pub async fn get_id_token(&self, audience: &str) -> anyhow::Result<String> {
        let url = format!(
            "{}/api/w/{}/oidc/token/{}",
            self.base_internal_url, self.workspace, audience
        );
        make_basic_get_request(self, &url, None, Some("decoding oidc token as json string")).await
    }

    pub async fn get_resource_value<T: DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        let url = format!(
            "{}/api/w/{}/resources/get_value/{}",
            self.base_internal_url, self.workspace, path
        );
        make_basic_get_request(self, &url, None, Some("decoding resource value as json")).await
    }

    pub async fn get_variable_value(&self, path: &str) -> anyhow::Result<String> {
        let url = format!(
            "{}/api/w/{}/variables/get_value/{}",
            self.base_internal_url, self.workspace, path
        );
        make_basic_get_request(self, &url, None, Some("decoding variable value as json")).await
    }

    pub async fn get_resource_value_interpolated<T: DeserializeOwned>(
        &self,
        path: &str,
        job_id: Option<String>,
    ) -> anyhow::Result<T> {
        let url = format!(
            "{}/api/w/{}/resources/get_value_interpolated/{}",
            self.base_internal_url, self.workspace, path
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &job_id {
            query.push(("job_id", v.to_string()));
        }
        let response = self.get(&url, query).await?;
        match response.status().as_u16() {
            200u16 => Ok(response
                .json::<T>()
                .await
                .context("decoding interpolated resource value as json")?),
            _ => Err(anyhow::anyhow!(response.text().await.unwrap_or_default())),
        }
    }

    pub async fn get_completed_job_result<T: DeserializeOwned>(
        &self,
        path: &str,
        json_path: Option<String>,
    ) -> anyhow::Result<T> {
        let url = format!(
            "{}/api/w/{}/jobs_u/completed/get_result/{}",
            self.base_internal_url, self.workspace, path
        );
        let query = query_from_json_path(json_path);
        make_basic_get_request(
            self,
            &url,
            Some(query),
            Some("decoding completed job result as json"),
        )
        .await
    }

    pub async fn get_flow_env_by_flow_job_id<T: DeserializeOwned>(
        &self,
        root_job_id: &str,
        var_name: &str,
        json_path: Option<String>,
    ) -> anyhow::Result<T> {
        let url = format!(
            "{}/api/w/{}/jobs/flow_env_by_flow_job_id/{}/{}",
            self.base_internal_url, self.workspace, root_job_id, var_name
        );
        let query = query_from_json_path(json_path);
        make_basic_get_request(
            self,
            &url,
            Some(query),
            Some("decoding flow env variable as json"),
        )
        .await
    }

    pub async fn get_result_by_id<T: DeserializeOwned>(
        &self,
        flow_job_id: &str,
        node_id: &str,
        json_path: Option<String>,
    ) -> anyhow::Result<T> {
        let url = format!(
            "{}/api/w/{}/jobs/result_by_id/{}/{}",
            self.base_internal_url, self.workspace, flow_job_id, node_id
        );
        let query = query_from_json_path(json_path);
        make_basic_get_request(
            self,
            &url,
            Some(query),
            Some("decoding result by id as json"),
        )
        .await
    }

    pub async fn upload_s3_file<S>(
        &self,
        workspace_id: &str,
        object_key: String,
        storage: Option<String>,
        body: S,
    ) -> anyhow::Result<()>
    where
        S: futures::stream::TryStream + Send + 'static,
        S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        bytes::Bytes: From<S::Ok>,
    {
        let mut query = vec![("file_key", object_key)];
        if let Some(storage) = storage {
            query.push(("storage", storage));
        }
        let response = self
            .force_client
            .as_ref()
            .unwrap_or(&HTTP_CLIENT)
            .post(format!(
                "{}/api/w/{}/job_helpers/upload_s3_file",
                self.base_internal_url, workspace_id
            ))
            .query(&query)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .header(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.token))
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?,
            )
            .body(Body::wrap_stream(body))
            .send()
            .await
            .context(format!("Sent upload_s3_file request",))
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        match response.status().as_u16() {
            200u16 => Ok(()),
            _ => Err(anyhow::anyhow!(response.text().await.unwrap_or_default()))?,
        }
    }

    pub async fn download_s3_file(
        &self,
        workspace_id: &str,
        file_key: &str,
        storage: Option<String>,
    ) -> anyhow::Result<bytes::Bytes> {
        let mut query = vec![("file_key", file_key.to_string())];
        if let Some(storage) = storage {
            query.push(("storage", storage));
        }
        let response = self
            .force_client
            .as_ref()
            .unwrap_or(&HTTP_CLIENT)
            .get(&format!(
                "{}/api/w/{}/job_helpers/download_s3_file",
                self.base_internal_url, workspace_id
            ))
            .query(&query)
            .header(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.token))
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?,
            )
            .send()
            .await
            .context("Failed to send download_s3_file request")
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        match response.status().as_u16() {
            200u16 => Ok(response
                .bytes()
                .await
                .context("Failed to read response bytes")?),
            _ => Err(anyhow::anyhow!(response.text().await.unwrap_or_default())),
        }
    }
}

#[inline]
fn query_from_json_path(json_path: Option<String>) -> Vec<(&'static str, String)> {
    json_path
        .map(|json_path| vec![("json_path", json_path)])
        .unwrap_or_else(|| Vec::new())
}

#[inline]
async fn make_basic_get_request<T: DeserializeOwned>(
    client: &AuthedClient,
    url: &str,
    query: Option<Vec<(&'static str, String)>>,
    context: Option<&'static str>,
) -> anyhow::Result<T> {
    let response = client
        .get(&url, query.unwrap_or_else(|| Vec::new()))
        .await?;

    match response.status().as_u16() {
        200u16 => {
            let json_body = response
                .json::<T>()
                .await
                .context(context.unwrap_or("error decoding body as json"))?;
            Ok(json_body)
        }
        _ => Err(anyhow::anyhow!(response.text().await.unwrap_or_default())),
    }
}
