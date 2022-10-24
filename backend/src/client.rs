use crate::{error::Error, variables::ListableVariable};

pub async fn get_variable(
    workspace: &str,
    path: &str,
    token: &str,
    base_url: &str,
) -> Result<String, anyhow::Error> {
    let client = reqwest::Client::new();
    let res = client
        .get(format!("{base_url}/api/w/{workspace}/variables/get/{path}"))
        .bearer_auth(token)
        .send()
        .await?;
    if res.status().is_success() {
        let value = res
            .json::<ListableVariable>()
            .await?
            .value
            .unwrap_or_else(|| "".to_string());
        Ok(value)
    } else {
        Err(Error::NotFound(format!("Variable not found at {path}")))?
    }
}

pub async fn get_resource(
    workspace: &str,
    path: &str,
    token: &str,
    base_url: &str,
) -> Result<Option<serde_json::Value>, anyhow::Error> {
    let client = reqwest::Client::new();
    let res = client
        .get(format!(
            "{base_url}/api/w/{workspace}/resources/get_value/{path}"
        ))
        .bearer_auth(token)
        .send()
        .await?;
    if res.status().is_success() {
        let value = res.json::<Option<serde_json::Value>>().await?;
        Ok(value)
    } else {
        Err(Error::NotFound(format!("Resource not found at {path}")))?
    }
}
