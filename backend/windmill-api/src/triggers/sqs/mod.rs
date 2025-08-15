use crate::{db::ApiAuthed, resources::try_get_resource_from_db_as};
use aws_config::{BehaviorVersion, Region};
use aws_sdk_sqs::{config::Credentials, error::SdkError, Client};
use aws_sdk_sts::operation::assume_role_with_web_identity::AssumeRoleWithWebIdentityOutput;
use backon::{ExponentialBuilder, Retryable};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use windmill_common::{
    auth::{
        aws::{
            get_assume_role_with_web_identity_fluent_builder, AWSAuthConfig, AwsAuthResourceType,
            GetAuthenticationOutput, OidcAuth, AWS_OIDC_AUDIENCE,
        },
        IdToken,
    },
    db::UserDB,
    error::{to_anyhow, Error, Result},
    oidc_ee::WorkspaceClaim,
    DB,
};

mod handler_ee;
pub mod handler_oss;

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct SqsConfig {
    pub queue_url: String,
    pub aws_resource_path: String,
    pub message_attributes: Vec<String>,
    pub aws_auth_resource_type: AwsAuthResourceType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSqsConfig {
    pub queue_url: String,
    pub aws_resource_path: String,
    pub message_attributes: Vec<String>,
    pub aws_auth_resource_type: AwsAuthResourceType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditSqsConfig {
    pub queue_url: String,
    pub aws_resource_path: String,
    pub message_attributes: Vec<String>,
    pub aws_auth_resource_type: AwsAuthResourceType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSqsConfig {
    pub aws_resource_path: String,
    pub queue_url: String,
}

pub fn validate_queue_url(url: &str) -> windmill_common::error::Result<()> {
    if url.trim().is_empty() {
        return Err(windmill_common::error::Error::BadRequest(
            "Queue URL cannot be empty".to_string(),
        ));
    }

    Ok(())
}

pub fn validate_aws_resource_path(path: &str) -> windmill_common::error::Result<()> {
    if path.trim().is_empty() {
        return Err(windmill_common::error::Error::BadRequest(
            "AWS resource path cannot be empty".to_string(),
        ));
    }
    Ok(())
}

#[allow(unused)]
struct AuthManager {
    credentials_expiration: DateTime<Utc>,
    id_token: IdToken,
    oidc: OidcAuth,
}

async fn get_sqs_client(
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
    region: Option<String>,
) -> Client {
    let credentials = Credentials::new(
        access_key_id,
        secret_access_key,
        session_token,
        None,
        "UserInput",
    );

    let config_loader =
        aws_config::defaults(BehaviorVersion::latest()).credentials_provider(credentials);

    let config_loader = match region {
        Some(region) => config_loader.region(Region::new(region)),
        _ => config_loader,
    };

    let sdk_config = config_loader.load().await;

    let client = Client::new(&sdk_config);

    client
}

async fn get_oidc_authentication_data(
    oidc: &OidcAuth,
    id_token: &IdToken,
) -> Result<AssumeRoleWithWebIdentityOutput> {
    let assume_role_with_web_identity_fluent_builder =
        get_assume_role_with_web_identity_fluent_builder(
            oidc,
            id_token.to_string(),
            Some("windmill-fetch-credentials"),
        )
        .await
        .map_err(to_anyhow)?;

    let assume_role_with_web_identity_output = (async || {
        assume_role_with_web_identity_fluent_builder
            .clone()
            .send()
            .await
    })
    .retry(ExponentialBuilder::default())
    .when(|sdk_err| match sdk_err {
        SdkError::ServiceError(service_err) => {
            service_err.err().is_idp_communication_error_exception()
        }
        _ => true,
    })
    .notify(|err, duration| {
        tracing::warn!(
            "Error while trying to assume role with web identity: {:?}, retrying in {:?}",
            err,
            duration
        );
    })
    .await
    .map_err(to_anyhow)?;

    Ok(assume_role_with_web_identity_output)
}

async fn get_sqs_auth_data(
    authed: &ApiAuthed,
    db: &DB,
    user_db: UserDB,
    aws_resource_path: &str,
    w_id: &str,
) -> Result<(Client, Option<AuthManager>)> {
    let aws_auth_config = try_get_resource_from_db_as::<AWSAuthConfig>(
        authed,
        Some(user_db),
        &db,
        aws_resource_path,
        w_id,
    )
    .await?;

    let (access_key_id, secret_access_key, region, session_token, auth_manager) =
        match aws_auth_config {
            AWSAuthConfig::Credentials(credentials) => (
                credentials.aws_access_key_id,
                credentials.aws_secret_access_key,
                credentials.region,
                None,
                None,
            ),
            AWSAuthConfig::Oidc(oidc) => {
                let id_token = windmill_common::oidc_ee::generate_id_token(
                    Some(db),
                    WorkspaceClaim { workspace: w_id.to_string() },
                    AWS_OIDC_AUDIENCE,
                    w_id.to_string(),
                    None,
                )
                .await?;

                let region = oidc.region.clone();

                let assume_role_with_web_identity_output =
                    get_oidc_authentication_data(&oidc, &id_token).await?;

                let credentials = assume_role_with_web_identity_output.get_credentials()?;

                let auth_manager = AuthManager {
                    credentials_expiration: DateTime::from_timestamp(
                        credentials.expiration.secs(),
                        credentials.expiration.subsec_nanos(),
                    )
                    .ok_or(Error::Anyhow {
                        error: anyhow::anyhow!("Invalid timestamp"),
                        location: "sqs_triggers_ee.rs".to_string(),
                    })?,
                    id_token,
                    oidc,
                };
                (
                    credentials.access_key_id.clone(),
                    credentials.secret_access_key.clone(),
                    region,
                    Some(credentials.session_token.clone()),
                    Some(auth_manager),
                )
            }
        };

    let aws_sqs_client =
        get_sqs_client(access_key_id, secret_access_key, session_token, region).await;

    Ok((aws_sqs_client, auth_manager))
}
