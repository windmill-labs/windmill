// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! A DynamoDB based lock system

use std::borrow::Cow;
use std::collections::HashMap;
use std::future::Future;
use std::time::{Duration, Instant};

use chrono::Utc;
use http::{Method, StatusCode};
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};

use crate::aws::client::S3Client;
use crate::aws::credential::CredentialExt;
use crate::aws::{AwsAuthorizer, AwsCredential};
use crate::client::get::GetClientExt;
use crate::client::retry::RetryExt;
use crate::client::retry::{RequestError, RetryError};
use crate::path::Path;
use crate::{Error, GetOptions, Result};

/// The exception returned by DynamoDB on conflict
const CONFLICT: &str = "ConditionalCheckFailedException";

const STORE: &str = "DynamoDB";

/// A DynamoDB-based commit protocol, used to provide conditional write support for S3
///
/// ## Limitations
///
/// Only conditional operations, e.g. `copy_if_not_exists` will be synchronized, and can
/// therefore race with non-conditional operations, e.g. `put`, `copy`, `delete`, or
/// conditional operations performed by writers not configured to synchronize with DynamoDB.
///
/// Workloads making use of this mechanism **must** ensure:
///
/// * Conditional and non-conditional operations are not performed on the same paths
/// * Conditional operations are only performed via similarly configured clients
///
/// Additionally as the locking mechanism relies on timeouts to detect stale locks,
/// performance will be poor for systems that frequently delete and then create
/// objects at the same path, instead being optimised for systems that primarily create
/// files with paths never used before, or perform conditional updates to existing files
///
/// ## Commit Protocol
///
/// The DynamoDB schema is as follows:
///
/// * A string partition key named `"path"`
/// * A string sort key named `"etag"`
/// * A numeric [TTL] attribute named `"ttl"`
/// * A numeric attribute named `"generation"`
/// * A numeric attribute named `"timeout"`
///
/// An appropriate DynamoDB table can be created with the CLI as follows:
///
/// ```bash
/// $ aws dynamodb create-table --table-name <TABLE_NAME> --key-schema AttributeName=path,KeyType=HASH AttributeName=etag,KeyType=RANGE --attribute-definitions AttributeName=path,AttributeType=S AttributeName=etag,AttributeType=S
/// $ aws dynamodb update-time-to-live --table-name <TABLE_NAME> --time-to-live-specification Enabled=true,AttributeName=ttl
/// ```
///
/// To perform a conditional operation on an object with a given `path` and `etag` (`*` if creating),
/// the commit protocol is as follows:
///
/// 1. Perform HEAD request on `path` and error on precondition mismatch
/// 2. Create record in DynamoDB with given `path` and `etag` with the configured timeout
///     1. On Success: Perform operation with the configured timeout
///     2. On Conflict:
///         1. Periodically re-perform HEAD request on `path` and error on precondition mismatch
///         2. If `timeout * max_skew_rate` passed, replace the record incrementing the `"generation"`
///             1. On Success: GOTO 2.1
///             2. On Conflict: GOTO 2.2
///
/// Provided no writer modifies an object with a given `path` and `etag` without first adding a
/// corresponding record to DynamoDB, we are guaranteed that only one writer will ever commit.
///
/// This is inspired by the [DynamoDB Lock Client] but simplified for the more limited
/// requirements of synchronizing object storage. The major changes are:
///
/// * Uses a monotonic generation count instead of a UUID rvn, as this is:
///     * Cheaper to generate, serialize and compare
///     * Cannot collide
///     * More human readable / interpretable
/// * Relies on [TTL] to eventually clean up old locks
///
/// It also draws inspiration from the DeltaLake [S3 Multi-Cluster] commit protocol, but
/// generalised to not make assumptions about the workload and not rely on first writing
/// to a temporary path.
///
/// [TTL]: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/howitworks-ttl.html
/// [DynamoDB Lock Client]: https://aws.amazon.com/blogs/database/building-distributed-locks-with-the-dynamodb-lock-client/
/// [S3 Multi-Cluster]: https://docs.google.com/document/d/1Gs4ZsTH19lMxth4BSdwlWjUNR-XhKHicDvBjd2RqNd8/edit#heading=h.mjjuxw9mcz9h
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DynamoCommit {
    table_name: String,
    /// The number of milliseconds a lease is valid for
    timeout: u64,
    /// The maximum clock skew rate tolerated by the system
    max_clock_skew_rate: u32,
    /// The length of time a record will be retained in DynamoDB before being cleaned up
    ///
    /// This is purely an optimisation to avoid indefinite growth of the DynamoDB table
    /// and does not impact how long clients may wait to acquire a lock
    ttl: Duration,
    /// The backoff duration before retesting a condition
    test_interval: Duration,
}

impl DynamoCommit {
    /// Create a new [`DynamoCommit`] with a given table name
    pub fn new(table_name: String) -> Self {
        Self {
            table_name,
            timeout: 20_000,
            max_clock_skew_rate: 3,
            ttl: Duration::from_secs(60 * 60),
            test_interval: Duration::from_millis(100),
        }
    }

    /// Overrides the lock timeout.
    ///
    /// A longer lock timeout reduces the probability of spurious commit failures and multi-writer
    /// races, but will increase the time that writers must wait to reclaim a lock lost. The
    /// default value of 20 seconds should be appropriate for must use-cases.
    pub fn with_timeout(mut self, millis: u64) -> Self {
        self.timeout = millis;
        self
    }

    /// The maximum clock skew rate tolerated by the system.
    ///
    /// An environment in which the clock on the fastest node ticks twice as fast as the slowest
    /// node, would have a clock skew rate of 2. The default value of 3 should be appropriate
    /// for most environments.
    pub fn with_max_clock_skew_rate(mut self, rate: u32) -> Self {
        self.max_clock_skew_rate = rate;
        self
    }

    /// The length of time a record should be retained in DynamoDB before being cleaned up
    ///
    /// This should be significantly larger than the configured lock timeout, with the default
    /// value of 1 hour appropriate for most use-cases.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Parse [`DynamoCommit`] from a string
    pub(crate) fn from_str(value: &str) -> Option<Self> {
        Some(match value.split_once(':') {
            Some((table_name, timeout)) => {
                Self::new(table_name.trim().to_string()).with_timeout(timeout.parse().ok()?)
            }
            None => Self::new(value.trim().to_string()),
        })
    }

    /// Returns the name of the DynamoDB table.
    pub(crate) fn table_name(&self) -> &str {
        &self.table_name
    }

    pub(crate) async fn copy_if_not_exists(
        &self,
        client: &S3Client,
        from: &Path,
        to: &Path,
    ) -> Result<()> {
        self.conditional_op(client, to, None, || async {
            client.copy_request(from, to).send().await?;
            Ok(())
        })
        .await
    }

    #[allow(clippy::future_not_send)] // Generics confound this lint
    pub(crate) async fn conditional_op<F, Fut, T>(
        &self,
        client: &S3Client,
        to: &Path,
        etag: Option<&str>,
        op: F,
    ) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, Error>>,
    {
        check_precondition(client, to, etag).await?;

        let mut previous_lease = None;

        loop {
            let existing = previous_lease.as_ref();
            match self.try_lock(client, to.as_ref(), etag, existing).await? {
                TryLockResult::Ok(lease) => {
                    let expiry = lease.acquire + lease.timeout;
                    return match tokio::time::timeout_at(expiry.into(), op()).await {
                        Ok(Ok(v)) => Ok(v),
                        Ok(Err(e)) => Err(e),
                        Err(_) => Err(Error::Generic {
                            store: "DynamoDB",
                            source: format!(
                                "Failed to perform conditional operation in {} milliseconds",
                                self.timeout
                            )
                            .into(),
                        }),
                    };
                }
                TryLockResult::Conflict(conflict) => {
                    let mut interval = tokio::time::interval(self.test_interval);
                    let expiry = conflict.timeout * self.max_clock_skew_rate;
                    loop {
                        interval.tick().await;
                        check_precondition(client, to, etag).await?;
                        if conflict.acquire.elapsed() > expiry {
                            previous_lease = Some(conflict);
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Attempt to acquire a lock, reclaiming an existing lease if provided
    async fn try_lock(
        &self,
        s3: &S3Client,
        path: &str,
        etag: Option<&str>,
        existing: Option<&Lease>,
    ) -> Result<TryLockResult> {
        let attributes;
        let (next_gen, condition_expression, expression_attribute_values) = match existing {
            None => (0_u64, "attribute_not_exists(#pk)", Map(&[])),
            Some(existing) => {
                attributes = [(":g", AttributeValue::Number(existing.generation))];
                (
                    existing.generation.checked_add(1).unwrap(),
                    "attribute_exists(#pk) AND generation = :g",
                    Map(attributes.as_slice()),
                )
            }
        };

        let ttl = (Utc::now() + self.ttl).timestamp();
        let items = [
            ("path", AttributeValue::from(path)),
            ("etag", AttributeValue::from(etag.unwrap_or("*"))),
            ("generation", AttributeValue::Number(next_gen)),
            ("timeout", AttributeValue::Number(self.timeout)),
            ("ttl", AttributeValue::Number(ttl as _)),
        ];
        let names = [("#pk", "path")];

        let req = PutItem {
            table_name: &self.table_name,
            condition_expression,
            expression_attribute_values,
            expression_attribute_names: Map(&names),
            item: Map(&items),
            return_values: None,
            return_values_on_condition_check_failure: Some(ReturnValues::AllOld),
        };

        let credential = s3.config.get_credential().await?;

        let acquire = Instant::now();
        match self
            .request(s3, credential.as_deref(), "DynamoDB_20120810.PutItem", req)
            .await
        {
            Ok(_) => Ok(TryLockResult::Ok(Lease {
                acquire,
                generation: next_gen,
                timeout: Duration::from_millis(self.timeout),
            })),
            Err(e) => match parse_error_response(&e) {
                Some(e) if e.error.ends_with(CONFLICT) => match extract_lease(&e.item) {
                    Some(lease) => Ok(TryLockResult::Conflict(lease)),
                    None => Err(Error::Generic {
                        store: STORE,
                        source: "Failed to extract lease from conflict ReturnValuesOnConditionCheckFailure response".into()
                    }),
                },
                _ => Err(Error::Generic {
                    store: STORE,
                    source: Box::new(e),
                }),
            },
        }
    }

    async fn request<R: Serialize + Send + Sync>(
        &self,
        s3: &S3Client,
        cred: Option<&AwsCredential>,
        target: &str,
        req: R,
    ) -> Result<HttpResponse, RetryError> {
        let region = &s3.config.region;
        let authorizer = cred.map(|x| AwsAuthorizer::new(x, "dynamodb", region));

        let builder = match &s3.config.endpoint {
            Some(e) => s3.client.request(Method::POST, e),
            None => {
                let url = format!("https://dynamodb.{region}.amazonaws.com");
                s3.client.request(Method::POST, url)
            }
        };

        // TODO: Timeout
        builder
            .json(&req)
            .header("X-Amz-Target", target)
            .with_aws_sigv4(authorizer, None)
            .send_retry(&s3.config.retry_config)
            .await
    }
}

#[derive(Debug)]
enum TryLockResult {
    /// Successfully acquired a lease
    Ok(Lease),
    /// An existing lease was found
    Conflict(Lease),
}

/// Validates that `path` has the given `etag` or doesn't exist if `None`
async fn check_precondition(client: &S3Client, path: &Path, etag: Option<&str>) -> Result<()> {
    let options = GetOptions {
        head: true,
        ..Default::default()
    };

    match etag {
        Some(expected) => match client.get_opts(path, options).await {
            Ok(r) => match r.meta.e_tag {
                Some(actual) if expected == actual => Ok(()),
                actual => Err(Error::Precondition {
                    path: path.to_string(),
                    source: format!("{} does not match {expected}", actual.unwrap_or_default())
                        .into(),
                }),
            },
            Err(Error::NotFound { .. }) => Err(Error::Precondition {
                path: path.to_string(),
                source: format!("Object at location {path} not found").into(),
            }),
            Err(e) => Err(e),
        },
        None => match client.get_opts(path, options).await {
            Ok(_) => Err(Error::AlreadyExists {
                path: path.to_string(),
                source: "Already Exists".to_string().into(),
            }),
            Err(Error::NotFound { .. }) => Ok(()),
            Err(e) => Err(e),
        },
    }
}

/// Parses the error response if any
fn parse_error_response(e: &RetryError) -> Option<ErrorResponse<'_>> {
    match e.inner() {
        RequestError::Status {
            status: StatusCode::BAD_REQUEST,
            body: Some(b),
        } => serde_json::from_str(b).ok(),
        _ => None,
    }
}

/// Extracts a lease from `item`, returning `None` on error
fn extract_lease(item: &HashMap<&str, AttributeValue<'_>>) -> Option<Lease> {
    let generation = match item.get("generation") {
        Some(AttributeValue::Number(generation)) => generation,
        _ => return None,
    };

    let timeout = match item.get("timeout") {
        Some(AttributeValue::Number(timeout)) => *timeout,
        _ => return None,
    };

    Some(Lease {
        acquire: Instant::now(),
        generation: *generation,
        timeout: Duration::from_millis(timeout),
    })
}

/// A lock lease
#[derive(Debug, Clone)]
struct Lease {
    acquire: Instant,
    generation: u64,
    timeout: Duration,
}

/// A DynamoDB [PutItem] payload
///
/// [PutItem]: https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_PutItem.html
#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PutItem<'a> {
    /// The table name
    table_name: &'a str,

    /// A condition that must be satisfied in order for a conditional PutItem operation to succeed.
    condition_expression: &'a str,

    /// One or more substitution tokens for attribute names in an expression
    expression_attribute_names: Map<'a, &'a str, &'a str>,

    /// One or more values that can be substituted in an expression
    expression_attribute_values: Map<'a, &'a str, AttributeValue<'a>>,

    /// A map of attribute name/value pairs, one for each attribute
    item: Map<'a, &'a str, AttributeValue<'a>>,

    /// Use ReturnValues if you want to get the item attributes as they appeared
    /// before they were updated with the PutItem request.
    #[serde(skip_serializing_if = "Option::is_none")]
    return_values: Option<ReturnValues>,

    /// An optional parameter that returns the item attributes for a PutItem operation
    /// that failed a condition check.
    #[serde(skip_serializing_if = "Option::is_none")]
    return_values_on_condition_check_failure: Option<ReturnValues>,
}

#[derive(Deserialize)]
struct ErrorResponse<'a> {
    #[serde(rename = "__type")]
    error: &'a str,

    #[serde(borrow, default, rename = "Item")]
    item: HashMap<&'a str, AttributeValue<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum ReturnValues {
    AllOld,
}

/// A collection of key value pairs
///
/// This provides cheap, ordered serialization of maps
struct Map<'a, K, V>(&'a [(K, V)]);

impl<K: Serialize, V: Serialize> Serialize for Map<'_, K, V> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.0.is_empty() {
            return serializer.serialize_none();
        }
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in self.0 {
            map.serialize_entry(k, v)?
        }
        map.end()
    }
}

/// A DynamoDB [AttributeValue]
///
/// [AttributeValue]: https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_AttributeValue.html
#[derive(Debug, Serialize, Deserialize)]
enum AttributeValue<'a> {
    #[serde(rename = "S")]
    String(Cow<'a, str>),
    #[serde(rename = "N", with = "number")]
    Number(u64),
}

impl<'a> From<&'a str> for AttributeValue<'a> {
    fn from(value: &'a str) -> Self {
        Self::String(Cow::Borrowed(value))
    }
}

/// Numbers are serialized as strings
mod number {
    use serde::{Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S: Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&v.to_string())
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let v: &str = Deserialize::deserialize(d)?;
        v.parse().map_err(serde::de::Error::custom)
    }
}

use crate::client::HttpResponse;
/// Re-export integration_test to be called by s3_test
#[cfg(test)]
pub(crate) use tests::integration_test;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::AmazonS3;
    use crate::ObjectStore;
    use rand::distr::Alphanumeric;
    use rand::{rng, Rng};

    #[test]
    fn test_attribute_serde() {
        let serde = serde_json::to_string(&AttributeValue::Number(23)).unwrap();
        assert_eq!(serde, "{\"N\":\"23\"}");
        let back: AttributeValue<'_> = serde_json::from_str(&serde).unwrap();
        assert!(matches!(back, AttributeValue::Number(23)));
    }

    /// An integration test for DynamoDB
    ///
    /// This is a function called by s3_test to avoid test concurrency issues
    pub(crate) async fn integration_test(integration: &AmazonS3, d: &DynamoCommit) {
        let client = integration.client.as_ref();

        let src = Path::from("dynamo_path_src");
        integration.put(&src, "asd".into()).await.unwrap();

        let dst = Path::from("dynamo_path");
        let _ = integration.delete(&dst).await; // Delete if present

        // Create a lock if not already exists
        let existing = match d.try_lock(client, dst.as_ref(), None, None).await.unwrap() {
            TryLockResult::Conflict(l) => l,
            TryLockResult::Ok(l) => l,
        };

        // Should not be able to acquire a lock again
        let r = d.try_lock(client, dst.as_ref(), None, None).await;
        assert!(matches!(r, Ok(TryLockResult::Conflict(_))));

        // But should still be able to reclaim lock and perform copy
        d.copy_if_not_exists(client, &src, &dst).await.unwrap();

        match d.try_lock(client, dst.as_ref(), None, None).await.unwrap() {
            TryLockResult::Conflict(new) => {
                // Should have incremented generation to do so
                assert_eq!(new.generation, existing.generation + 1);
            }
            _ => panic!("Should conflict"),
        }

        let rng = rng();
        let etag = String::from_utf8(rng.sample_iter(Alphanumeric).take(32).collect()).unwrap();
        let t = Some(etag.as_str());

        let l = match d.try_lock(client, dst.as_ref(), t, None).await.unwrap() {
            TryLockResult::Ok(l) => l,
            _ => panic!("should not conflict"),
        };

        match d.try_lock(client, dst.as_ref(), t, None).await.unwrap() {
            TryLockResult::Conflict(c) => assert_eq!(l.generation, c.generation),
            _ => panic!("should conflict"),
        }

        match d.try_lock(client, dst.as_ref(), t, Some(&l)).await.unwrap() {
            TryLockResult::Ok(new) => assert_eq!(new.generation, l.generation + 1),
            _ => panic!("should not conflict"),
        }
    }
}
