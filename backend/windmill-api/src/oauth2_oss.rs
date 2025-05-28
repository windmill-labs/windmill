use crate::db::DB;
use axum::Router;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use windmill_common::error;

#[derive(Serialize, Deserialize)]
pub struct ClientWithScopes;

pub type BasicClientsMap = HashMap<String, ClientWithScopes>;

#[derive(Serialize, Deserialize)]
pub struct OAuthConfig;

#[derive(Serialize, Deserialize)]
pub struct OAuthClient;

#[derive(Serialize, Deserialize)]
pub struct AllClients;

#[derive(Serialize, Deserialize)]
pub struct TokenResponse;

pub struct SlackVerifier;

impl SlackVerifier {
    pub fn new<S: AsRef<[u8]>>(secret: S) -> anyhow::Result<SlackVerifier> {
        crate::oauth2_ee::SlackVerifier::new(secret)
    }
}

pub fn global_service() -> Router {
    crate::oauth2_ee::global_service()
}

pub fn workspaced_service() -> Router {
    crate::oauth2_ee::workspaced_service()
}

pub async fn build_oauth_clients(
    _db: &DB,
    _w_id: &str,
    _base_url: &str,
) -> anyhow::Result<AllClients> {
    crate::oauth2_ee::build_oauth_clients(_db, _w_id, _base_url).await
}

pub async fn _refresh_token(
    _workspace_id: &str,
    _path: &str,
    _db: &DB,
    _token: &str,
    _http_client: &reqwest::Client,
) -> error::Result<String> {
    crate::oauth2_ee::_refresh_token(_workspace_id, _path, _db, _token, _http_client).await
}

pub async fn check_nb_of_user(db: &DB) -> error::Result<()> {
    crate::oauth2_ee::check_nb_of_user(db).await
}