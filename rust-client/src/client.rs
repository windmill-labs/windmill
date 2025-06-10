#[cfg(feature = "async")]
use futures::FutureExt;
use windmill_api::{
    apis::{
        configuration::Configuration,
        job_api,
        resource_api::{self, get_resource_value_interpolated},
        variable_api,
    },
    models::GetCompletedJobResultMaybe200Response,
};

use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    env::{self, var},
    fmt::Debug,
    time::{Duration, Instant},
};

use crate::{maybe_future::maybe_future::MaybeFuture, ret};

pub struct Windmill {
    pub workspace: String,
    pub token: String,
    pub base_internal_url: String,
    pub client_config: Configuration,
}

impl Windmill {
    /// Creates a new `Windmill` instance with default configuration.
    ///
    /// Reads configuration from environment variables:
    /// - `WM_TOKEN` for authentication token
    /// - `WM_WORKSPACE` for workspace name  
    /// - `BASE_INTERNAL_URL` for API base URL (appends `/api` automatically)
    ///
    /// # Errors
    /// Returns `SdkError` if:
    /// - Required environment variables are missing
    /// - Environment variables cannot be read
    pub fn default() -> Result<Self, SdkError> {
        Windmill::new(None, None, None)
    }

    /// Creates a new `Windmill` instance with optional overrides.
    ///
    /// Falls back to environment variables for any `None` parameters:
    /// - `WM_TOKEN` if `token` is `None`
    /// - `WM_WORKSPACE` if `workspace` is `None`  
    /// - `BASE_INTERNAL_URL` + "/api" if `base_internal_url` is `None`
    ///
    /// # Parameters
    /// - `token`: Optional bearer token override
    /// - `workspace`: Optional workspace name override  
    /// - `base_internal_url`: Optional base URL override (without `/api` suffix)
    ///
    /// # Errors  
    /// Returns `SdkError` if:
    /// - Required environment variables are missing when needed
    /// - Environment variables cannot be read
    pub fn new(
        token: Option<String>,
        workspace: Option<String>,
        base_internal_url: Option<String>,
    ) -> Result<Self, SdkError> {
        use env::var;
        let (token, base_internal_url, workspace) = (
            token.or(var("WM_TOKEN").ok()).ok_or(SdkError::BadValue(
                "WM_TOKEN is not set nor was provided in constructor".to_owned(),
            ))?,
            base_internal_url
                .or(var("BASE_INTERNAL_URL").ok())
                .ok_or(SdkError::BadValue(
                    "BASE_INTERNAL_URL is not set nor was provided in constructor".to_owned(),
                ))?,
            workspace
                .or(var("WM_WORKSPACE").ok())
                .ok_or(SdkError::BadValue(
                    "WM_WORKSPACE is not set nor was provided in constructor".to_owned(),
                ))?,
        );

        Ok(Windmill {
            client_config: Configuration {
                // TODO: client: reqwest::blocking::Client::new(), // Use blocking client?
                base_path: base_internal_url.clone(),
                bearer_access_token: Some(token.clone()),
                ..Default::default()
            },
            workspace,
            token,
            base_internal_url,
        })
    }

    /// Retrieves a variable from Windmill, automatically parsing it as JSON/YAML.
    ///
    /// This is the convenience version that attempts to parse the variable value as:
    /// 1. JSON (primary attempt)
    /// 2. YAML (fallback if JSON parsing fails)
    /// 3. Raw string (final fallback if both parsings fail)
    ///
    /// For better performance when you know the format or don't need parsing,
    /// use [`Self::get_variable_raw`] instead.
    ///
    /// # Arguments
    /// * `path` - Variable path (e.g., "u/user/variable_name")
    ///
    /// # Example
    /// ```no_run
    ///
    /// use wmill::Windmill;
    /// use serde_json::json;
    ///
    /// let wm = Windmill::default()?;
    ///
    /// // For a variable containing JSON: {"key": "value"}
    /// let json_var = wm.get_variable("u/admin/config")?;
    ///
    /// // For a variable containing plain text
    /// let text_var = wm.get_variable("u/user/plaintext_note")?;
    ///
    /// # Ok::<(), wmill::SdkError>(())
    /// ```
    ///
    /// See also: [`Self::get_variable_raw`] for the unparsed version
    pub fn get_variable<'a>(&'a self, path: &'a str) -> MaybeFuture<'a, Result<Value, SdkError>> {
        ret!(self.get_variable_inner(path));
    }

    async fn get_variable_inner<'a>(&'a self, path: &'a str) -> Result<Value, SdkError> {
        let raw = self.get_variable_raw_inner(path).await?;
        Ok(serde_json::from_str(&raw)
            .or(serde_yaml::from_str(&raw))
            .unwrap_or(json!(raw)))
    }

    /// This is the **faster version** when:
    /// - You know the variable contains plain text
    /// - You want to handle parsing yourself
    /// - You need maximum performance
    ///
    /// Performance benefit comes from avoiding:
    /// 1. JSON parsing attempt
    /// 2. YAML parsing fallback
    ///
    /// # Arguments
    /// * `path` - Variable path (e.g., "u/user/variable_name")
    ///
    /// # Example
    /// ```no_run
    /// use wmill::Windmill;
    /// use serde_json::{json, Value};
    ///
    /// let wm = Windmill::default()?;
    ///
    /// // When you need the raw content
    /// let raw_content = wm.get_variable_raw("u/user/custom_format")?;
    ///
    /// // When you know it's JSON and want to parse it differently
    /// let json_value: Value = serde_json::from_str(
    ///     &wm.get_variable_raw("u/admin/config")?
    /// )?;
    ///
    /// # Ok::<(), wmill::SdkError>(())
    /// ```
    ///
    /// See also: [`Self::get_variable`] for the auto-parsed version
    pub fn get_variable_raw<'a>(
        &'a self,
        path: &'a str,
    ) -> MaybeFuture<'a, Result<String, SdkError>> {
        ret!(self.get_variable_raw_inner(path));
    }

    async fn get_variable_raw_inner<'a>(&'a self, path: &'a str) -> Result<String, SdkError> {
        let v =
            variable_api::get_variable_value(&self.client_config, &self.workspace, path).await?;

        v.get(1..v.len() - 1)
            .ok_or(SdkError::InternalError(
                "returned value is incorrect".into(),
            ))
            .map(|s| s.to_owned())
    }

    /// Creates or updates a variable in the workspace.
    ///
    /// This function provides atomic variable management that:
    /// - Creates a new variable if it doesn't exist
    /// - Updates an existing variable if found
    /// - Handles both regular and secret variables
    ///
    /// # Parameters
    /// - `value`: The variable value to set
    /// - `path`: The variable path/identifier
    /// - `is_secret`: Whether to store as a secret (encrypted) variable
    ///
    /// # Errors
    /// Returns `SdkError` if:
    /// - Variable fetch fails for reasons other than "not found"
    /// - Variable creation fails
    /// - Variable update fails
    /// - Underlying API calls fail
    ///
    /// # Notes
    /// - For new variables, defaults to empty description
    /// - Updates only modify the value (preserving other metadata)
    /// - Secret status can only be set during creation
    pub fn set_variable<'a>(
        &'a self,
        value: String,
        path: &'a str,
        is_secret: bool,
    ) -> MaybeFuture<'a, Result<(), SdkError>> {
        ret!(self.set_variable_inner(value, path, is_secret));
    }

    async fn set_variable_inner<'a>(
        &'a self,
        value: String,
        path: &'a str,
        is_secret: bool,
    ) -> Result<(), SdkError> {
        let res =
            variable_api::get_variable(&self.client_config, &self.workspace, path, None, None)
                .await;

        if res.is_err() {
            variable_api::create_variable(
                &self.client_config,
                &self.workspace,
                windmill_api::models::CreateVariable {
                    path: path.to_owned(),
                    value,
                    is_secret,
                    description: "".to_owned(),
                    account: None,
                    is_oauth: None,
                    expires_at: None,
                },
                None,
            )
            .await?;
        } else {
            variable_api::update_variable(
                &self.client_config,
                &self.workspace,
                path,
                windmill_api::models::EditVariable {
                    path: None,
                    value: Some(value),
                    is_secret: None,
                    description: None,
                },
                None,
            )
            .await?;
        }
        Ok(())
    }

    /// Fetches and deserializes a resource into a concrete type.
    ///
    /// This is the recommended way to access resources when you know the expected type.
    /// For raw JSON access or dynamic typing, use [`Self::get_resource_any`] instead.
    ///
    /// # Type Parameters
    /// * `T` - Any type implementing `DeserializeOwned` (most structs with `#[derive(Deserialize)]`)
    ///
    /// # Arguments
    /// * `path` - The resource path (e.g., "u/user/resource_name")
    ///
    /// # Example
    /// ```no_run
    /// use wmill::Windmill;
    /// use serde_json::{json, Value};
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct DbConfig {
    ///     url: String,
    ///     pool_size: Option<u32>,
    /// }
    ///
    /// let wm = Windmill::default()?;
    ///
    /// // Directly deserialize to your type
    /// let config: DbConfig = wm.get_resource("u/admin/db_connection")?;
    /// # Ok::<(), wmill::SdkError>(())
    /// ```
    ///
    /// See also: [`Self::get_resource_any`] for untyped version
    pub fn get_resource<'a, T: serde::de::DeserializeOwned>(
        &'a self,
        path: &'a str,
    ) -> MaybeFuture<'a, Result<T, SdkError>> {
        ret!(self.get_resource_inner(path));
    }

    async fn get_resource_inner<'a, T: serde::de::DeserializeOwned>(
        &'a self,
        path: &'a str,
    ) -> Result<T, SdkError> {
        Ok(serde_json::from_value(
            self.get_resource_any_inner(path).await?,
        )?)
    }

    /// Fetches a raw JSON [`Value`] from Windmill by path.
    ///
    /// Use this when you need the raw JSON structure or don't have a concrete type to deserialize into.
    /// For typed deserialization, prefer [`Self::get_resource`] instead.
    ///
    /// # Arguments
    /// * `path` - The resource path (e.g., "u/user/resource_name")
    ///
    /// # Example
    /// ```no_run
    /// use wmill::Windmill;
    ///
    /// let wm = Windmill::default()?;
    ///
    /// // When you need to inspect the raw structure first
    /// let json = wm.get_resource_any("u/admin/db_connection")?;
    ///
    /// println!("Url is: {}", json["url"]);
    ///
    /// # Ok::<(), wmill::SdkError>(())
    /// ```
    ///
    /// See also: [`Self::get_resource`] for typed version
    pub fn get_resource_any<'a>(
        &'a self,
        path: &'a str,
    ) -> MaybeFuture<'a, Result<Value, SdkError>> {
        ret!(self.get_resource_any_inner(path));
    }

    async fn get_resource_any_inner<'a>(&'a self, path: &'a str) -> Result<Value, SdkError> {
        Ok(
            get_resource_value_interpolated(&self.client_config, &self.workspace, path, None)
                .await?,
        )
    }

    /// Creates or updates a resource in Windmill.
    ///
    /// This function sets a resource's value at the specified path, creating it if it doesn't exist
    /// or updating it if it does. The resource will be of the specified type.
    ///
    /// # Arguments
    /// * `value` - The value to set for the resource. Use `None` to create an empty resource.
    /// * `path` - The ownership path of the resource (e.g., "u/user/resource_name").
    ///            Defines permissions based on Windmill's path prefix system.
    /// * `resource_type` - The type of resource to create (e.g., "postgresql", "smtp").
    ///                     Must be a pre-existing resource type.
    ///
    /// # Examples
    /// ```no_run
    /// use wmill::Windmill;
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let wm = Windmill::default()?;
    /// wm.set_resource(
    ///     Some(serde_json::json!({"host": "localhost", "port": 5432})),
    ///     "u/admin/database",
    ///     "postgresql"
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_resource<'a>(
        &'a self,
        value: Option<Value>,
        path: &'a str,
        resource_type: &'a str,
    ) -> MaybeFuture<'a, Result<(), SdkError>> {
        ret!(self.set_resource_inner(value, path, resource_type));
    }

    async fn set_resource_inner<'a>(
        &'a self,
        value: Option<Value>,
        path: &'a str,
        resource_type: &'a str,
    ) -> Result<(), SdkError> {
        resource_api::create_resource(
            &self.client_config,
            &self.workspace,
            windmill_api::models::CreateResource {
                path: path.to_owned(),
                value,
                description: None,
                resource_type: resource_type.to_owned(),
            },
            Some(true),
        )
        .await?;
        Ok(())
    }

    /// Retrieves and deserializes the current typed state value for a script's execution context.
    ///
    /// This is the typed version of [`Self::get_state_any`], automatically deserializing the state
    /// into the specified type `T` that implements [`serde::de::DeserializeOwned`].
    ///
    /// # Type Parameter
    /// * `T` - The type to deserialize into (must implement `serde::de::DeserializeOwned`)
    ///
    /// # Examples
    /// ```no_run
    /// use wmill::Windmill;
    /// use serde_json::{json, Value};
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct ScriptState {
    ///     counter: i32,
    ///     last_run: String,
    /// }
    ///
    /// let wm = Windmill::default()?;
    ///
    /// // Get typed state
    /// let state: ScriptState = wm.get_state()?;
    ///
    /// println!("Counter: {}, Last run: {}", state.counter, state.last_run);
    ///
    /// # Ok::<(), wmill::SdkError>(())
    /// ```
    ///
    /// # Behavior Details
    /// - Uses same state path resolution as [`Self::get_state_any`] (`WM_STATE_PATH_NEW` → `WM_STATE_PATH` fallback)
    /// - Performs runtime type checking during deserialization
    ///
    /// # When to Use vs [`Self::get_state_any`]
    /// | Use Case               | `get_state<T>`          | `get_state_any`        |
    /// |------------------------|-------------------------|------------------------|
    /// | Known state structure  | ✅ Preferred            | ⚠️ Requires manual parsing |
    /// | Dynamic state          | ❌ Won't compile        | ✅ Works               |
    /// | Type safety            | ✅ Compile-time checks  | ❌ Runtime checks only |
    ///
    /// # Notes
    /// - For complex types, derive `Deserialize` using Serde attributes
    /// - Prefer this over [`Self::get_state_any`] when state schema is stable
    /// - State modifications should use corresponding [`Self::set_state`] with matching type
    ///
    /// # See Also
    /// - [`Self::get_state_any`] - Untyped state access
    /// - [`Self::set_state`] - For updating typed states
    /// - [Windmill State Management](https://www.windmill.dev/docs/core_concepts/persistent_storage/within_windmill#states)
    pub fn get_state<'a, T: serde::de::DeserializeOwned>(
        &'a self,
    ) -> MaybeFuture<'a, Result<T, SdkError>> {
        ret!(self.get_resource_inner(&get_state_path()?));
    }

    /// Retrieves the current state value for a script's execution context.
    ///
    /// States persist data between runs of the same script by the same trigger (schedule or user).
    /// This is the untyped version that returns a generic [`Value`].
    ///
    /// # Examples
    /// ```no_run
    /// use wmill::Windmill;
    /// use serde_json::Value;
    /// use serde::Deserialize;
    ///
    /// let wm = Windmill::default()?;
    ///
    /// // Get state (returns serde_json::Value)
    /// let state: Value = wm.get_state_any()?;
    ///
    /// // Use with default if empty
    /// let counter = state.as_i64().unwrap_or(0);
    /// # Ok::<(), wmill::SdkError>(())
    /// ```
    ///
    /// # Behavior Details
    /// - Automatically uses the script's state path from `WM_STATE_PATH_NEW` (falls back to `WM_STATE_PATH`)
    /// - Returns the full state object stored as a Windmill resource
    /// - State resources are hidden from Workspace view but visible under Resources → States
    ///
    /// # Typical Use Cases
    /// 1. Maintaining counters between runs
    /// 2. Storing last execution timestamps
    /// 3. Keeping reference data (like previous API responses)
    ///
    /// # Notes
    /// - For typed state access, use `get_state<T>` instead
    /// - States are isolated per script and trigger combination
    /// - Maximum state size: 5MB (compressed)
    ///
    /// # See Also
    /// - [`Self::set_state`] - For updating the state
    /// - [Windmill State Management](https://www.windmill.dev/docs/core_concepts/persistent_storage/within_windmill#states)
    pub fn get_state_any<'a>(&'a self) -> MaybeFuture<'a, Result<Value, SdkError>> {
        ret!(self.get_resource_any_inner(&get_state_path()?));
    }

    /// Updates or clears the script's persistent state.
    ///
    /// # Arguments
    /// * `value` - New state value (`Some(Value)`) or `None` to clear state
    ///
    /// # Examples
    /// ```no_run
    /// use wmill::Windmill;
    /// use serde_json::json;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let wm = Windmill::default()?;
    ///
    /// // Set state
    /// wm.set_state(Some(json!({"count": 42})))?;
    ///
    /// // Clear state
    /// wm.set_state(None)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See also: [`Self::get_state`], [`Self::get_state_any`]
    pub fn set_state<'a>(&'a self, value: Option<Value>) -> MaybeFuture<'a, Result<(), SdkError>> {
        ret!(self.set_resource_inner(value, &get_state_path()?, "state"));
    }

    /// Executes a script synchronously and waits for its completion.
    ///
    /// This is a blocking version of `run_script_async` that handles the entire script execution
    /// lifecycle including job scheduling and result waiting.
    ///
    /// # Parameters
    /// - `ident`: Script identifier (either path or hash)
    /// - `ident_is_hash`: Whether the identifier is a hash (true) or path (false)
    /// - `args`: JSON arguments to pass to the script
    /// - `scheduled_in_secs`: Optional delay before execution (in seconds)
    /// - `timeout_secs`: Maximum time to wait for job completion (in seconds)
    /// - `verbose`: Whether to print execution details
    /// - `assert_result_is_not_none`: Whether to fail if the result is None
    ///
    /// # Errors
    /// Returns `SdkError` if:
    /// - Script fails to start
    /// - Job times out
    /// - Result assertion fails
    /// - Underlying API calls fail
    pub fn run_script_sync<'a>(
        &'a self,
        ident: &'a str,
        ident_is_hash: bool,
        args: Value,
        scheduled_in_secs: Option<u32>,
        timeout_secs: Option<u64>,
        verbose: bool,
        assert_result_is_not_none: bool,
    ) -> MaybeFuture<'a, Result<Value, SdkError>> {
        if verbose {
            println!("running `{ident}` synchronously with {:?}", &args);
        }

        ret!(async move {
            let job_id = self
                .run_script_async_inner(ident, ident_is_hash, args, scheduled_in_secs)
                .await?;
            self.wait_job_inner(
                &job_id.to_string(),
                timeout_secs,
                verbose,
                assert_result_is_not_none,
            )
            .await
        });
    }

    /// Asynchronously executes a script in Windmill and returns its job UUID.
    ///
    /// This function runs a script either by path or by hash, with optional:
    /// - Parent job inheritance (when run within a flow)
    /// - Scheduled execution delay
    /// - Argument passing
    ///
    /// # Arguments
    /// * `ident` - Script identifier (path or hash depending on `ident_is_hash`)
    /// * `ident_is_hash` - If true, `ident` is treated as a script hash; if false, as a path
    /// * `args` - JSON arguments to pass to the script (must be an object if using path)
    /// * `scheduled_in_secs` - Optional delay (in seconds) before execution
    ///
    /// # Examples
    /// ```no_run
    /// use wmill::Windmill;
    /// use serde_json::json;
    ///
    /// let wm = Windmill::default()?;
    /// let job_id = wm.run_script_async(
    ///     "u/user/script_path",
    ///     false,
    ///     json!({"param1": "value1"}),
    ///     Some(10) // Run after 10 seconds
    /// )?;
    ///
    /// # Ok::<(), wmill::SdkError>(())
    /// ```
    ///
    /// # Behavior Details
    /// - **Automatic Job Inheritance**:
    ///   - Detects `WM_JOB_ID` env var → sets as `parent_job`
    ///   - Detects `WM_ROOT_FLOW_JOB_ID` env var → sets as `root_job`
    /// - **Scheduled Execution**:
    ///   - When `scheduled_in_secs` is provided, sets `scheduled_in_secs` in args
    /// - **Argument Handling**:
    ///   - For path-based execution (`ident_is_hash=false`), args must be a JSON object
    ///   - For hash-based execution, args can be any valid JSON value
    ///
    /// # Errors
    /// - [`SdkError::BadValue`] if path-based execution receives non-object arguments
    /// - API errors from Windmill's backend
    pub fn run_script_async<'a>(
        &'a self,
        ident: &'a str,
        ident_is_hash: bool,
        args: Value,
        scheduled_in_secs: Option<u32>,
    ) -> MaybeFuture<'a, Result<uuid::Uuid, SdkError>> {
        ret!(self.run_script_async_inner(ident, ident_is_hash, args, scheduled_in_secs));
    }

    async fn run_script_async_inner<'a>(
        &'a self,
        ident: &'a str,
        ident_is_hash: bool,
        mut args: Value,
        scheduled_in_secs: Option<u32>,
    ) -> Result<uuid::Uuid, SdkError> {
        if let Ok(parent_job) = var("WM_JOB_ID") {
            args["parent_job"] = json!(parent_job);
        }

        if let Ok(root_job) = var("WM_ROOT_FLOW_JOB_ID") {
            args["root_job"] = json!(root_job);
        }

        if let Some(scheduled_in_secs) = scheduled_in_secs {
            args["scheduled_in_secs"] = json!(scheduled_in_secs);
        }

        let uuid = if ident_is_hash {
            job_api::run_script_by_hash(
                &self.client_config,
                &self.workspace,
                ident,
                args,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .await?
        } else {
            job_api::run_script_by_path(
                &self.client_config,
                &self.workspace,
                ident,
                args.as_object()
                    .ok_or(SdkError::BadValue(format!(
                        "Args should be Object, but it is: {}",
                        args
                    )))?
                    .clone()
                    .into_iter()
                    .collect(),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .await?
        };
        Ok(uuid)
    }

    /// Waits for a job to complete and returns its result.
    ///
    /// This function provides both synchronous and asynchronous interfaces for waiting
    /// on job completion, with timeout handling and result validation.
    ///
    /// # Parameters
    /// - `job_id`: The ID of the job to wait for
    /// - `timeout_secs`: Maximum time to wait (in seconds) before cancelling the job
    /// - `verbose`: Whether to print progress information
    /// - `assert_result_is_not_none`: Whether to fail if the job returns no result
    ///
    /// # Errors
    /// Returns `SdkError` if:
    /// - Job fails or times out
    /// - Result assertion fails when `assert_result_is_not_none` is true
    /// - Underlying API calls fail
    ///
    /// # Behavior
    /// 1. Polls job status at 500ms intervals
    /// 2. Cancels job if timeout is reached
    /// 3. Validates success status and optional result presence
    /// 4. Returns either the result or appropriate error
    pub fn wait_job<'a>(
        &'a self,
        job_id: &'a str,
        timeout_secs: Option<u64>,
        verbose: bool,
        assert_result_is_not_none: bool,
    ) -> MaybeFuture<'a, Result<Value, SdkError>> {
        ret!(self.wait_job_inner(job_id, timeout_secs, verbose, assert_result_is_not_none));
    }

    async fn wait_job_inner<'a>(
        &'a self,
        job_id: &'a str,
        timeout_secs: Option<u64>,
        verbose: bool,
        assert_result_is_not_none: bool,
    ) -> Result<Value, SdkError> {
        let start = Instant::now();
        loop {
            if let Some(timeout) = timeout_secs {
                if start.elapsed().as_secs() > timeout {
                    println!("WARN: reached timeout for {job_id}. Cancelling the job.");
                    job_api::cancel_queued_job(
                        &self.client_config,
                        &self.workspace,
                        job_id,
                        windmill_api::models::CancelQueuedJobRequest {
                            reason: Some("reached timeout".into()),
                        },
                    )
                    .await?;
                }
            }
            let GetCompletedJobResultMaybe200Response {
                completed,
                result,
                success,
                started,
            } = job_api::get_completed_job_result_maybe(
                &self.client_config,
                &self.workspace,
                job_id,
                Some(true),
            )
            .await?;

            if matches!(started, Some(false)) && verbose {
                println!("INFO: job {job_id} has not started yet");
            }

            if completed {
                if matches!(success, Some(true)) {
                    if result.is_none() && assert_result_is_not_none {
                        return Err(SdkError::ExecutionError("Result was None".into()));
                    }
                    return Ok(result.unwrap_or_default());
                } else {
                    return Err(SdkError::ExecutionError(format!(
                        "Job {job_id} was not successful: {:?}",
                        result
                    )));
                }
            }
            if verbose {
                println!("INFO: sleeping 0.5 seconds for {job_id}");
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    /// Retrieves the current status of a Windmill job by its UUID.
    ///
    /// This function queries the Windmill backend to determine whether a job is:
    /// - Waiting to be executed
    /// - Currently running
    /// - Already completed
    ///
    /// # Arguments
    /// * `job_id` - The UUID of the job to check (format: "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx")
    ///
    /// # Job Lifecycle States
    /// - **Waiting**: Job is queued but hasn't started execution
    /// - **Running**: Job is currently being executed
    /// - **Completed**: Job has finished (successfully or with errors)
    pub fn get_job_status<'a>(
        &'a self,
        job_id: &'a str,
    ) -> MaybeFuture<'a, Result<JobStatus, SdkError>> {
        ret!(self.get_job_status_inner(job_id));
    }

    async fn get_job_status_inner<'a>(&'a self, job_id: &'a str) -> Result<JobStatus, SdkError> {
        let job =
            job_api::get_job(&self.client_config, &self.workspace, job_id, Some(true)).await?;

        Ok(match job {
            windmill_api::models::Job::JobOneOf(..) => JobStatus::Completed,
            windmill_api::models::Job::JobOneOf1(job_one_of1) => {
                if job_one_of1.running {
                    JobStatus::Running
                } else {
                    JobStatus::Waiting
                }
            }
        })
    }

    /// Retrieves the result of a completed job.
    ///
    /// This provides both synchronous and asynchronous interfaces for fetching
    /// the final result of a successfully completed job.
    ///
    /// # Parameters
    /// - `job_id`: The ID of the completed job to fetch results for
    ///
    /// # Errors
    /// Returns `SdkError` if:
    /// - Job is not found
    /// - Job failed to complete successfully
    /// - Underlying API calls fail
    /// - Result cannot be parsed
    ///
    /// # Notes
    /// - Only works for jobs that have already completed
    /// - For pending/running jobs, use `wait_job` instead
    /// - Does not handle timeouts or polling - assumes job is already complete
    pub fn get_result<'a>(&'a self, job_id: &'a str) -> MaybeFuture<'a, Result<Value, SdkError>> {
        ret!(self.get_result_inner(job_id));
    }

    async fn get_result_inner<'a>(&'a self, job_id: &'a str) -> Result<Value, SdkError> {
        Ok(job_api::get_completed_job_result(
            &self.client_config,
            &self.workspace,
            job_id,
            None,
            None,
            None,
            None,
        )
        .await?)
    }

    /// Updates the progress percentage of a running Windmill job.
    ///
    /// This function allows scripts to report their execution progress (0-100%) back to the Windmill UI.
    /// Progress updates are visible in both the jobs dashboard and flow visualizations.
    ///
    /// # Arguments
    /// * `value` - Progress percentage (0-100)
    /// * `job_id` - Optional job UUID. If None, uses current job's ID from `WM_JOB_ID` environment variable
    ///
    /// # Examples
    /// ```no_run
    /// use wmill::Windmill;
    /// # fn main () -> anyhow::Result<()>{
    /// let wm = Windmill::default()?;
    ///
    /// // Report progress for current job
    /// wm.set_progress(25, None)?;
    /// # Ok(())
    /// # }
    ///
    /// ```
    ///
    /// # Behavior Details
    /// - Automatically handles flow context by detecting parent job ID
    /// - Progress updates are reflected in real-time in the Windmill UI
    /// - Typical usage pattern:
    ///   ```ignore
    ///   for (i, item) in items.iter().enumerate() {
    ///       process(item);
    ///       let progress = ((i + 1) * 100 / items.len()) as i32;
    ///       wmill.set_progress(progress, None).await?;
    ///   }
    ///   ```
    ///
    /// # Notes
    /// - Only affects jobs that are currently running
    /// - Progress values outside 0-99 range are clamped by the server
    /// - Progress cannot decrease
    /// - For flows, updates the progress of both the step and parent flow
    ///
    /// # See Also
    /// - [Flow Progress Tracking](https://www.windmill.dev/docs/advanced/explicit_progress)
    /// - [`Self::get_progress`] - For reading job progress
    pub fn set_progress<'a>(
        &'a self,
        value: u32,
        job_id: Option<String>,
    ) -> MaybeFuture<'a, Result<(), SdkError>> {
        let f = async move {
            let job_id = job_id.unwrap_or(var("WM_JOB_ID")?);
            let job =
                job_api::get_job(&self.client_config, &self.workspace, &job_id, Some(true)).await?;

            let flow_id = match job {
                windmill_api::models::Job::JobOneOf(job) => job.parent_job,
                windmill_api::models::Job::JobOneOf1(job) => job.parent_job,
            };

            windmill_api::apis::metrics_api::set_job_progress(
                &self.client_config,
                &self.workspace,
                &job_id,
                windmill_api::models::SetJobProgressRequest {
                    percent: Some(value as i32),
                    flow_job_id: flow_id,
                },
            )
            .await?;
            Ok(())
        };
        ret!(f);
    }

    /// Retrieves the current progress percentage of a Windmill job.
    ///
    /// This function queries the Windmill backend to get the execution progress (0-100%)
    /// of either a specific job or the current job context.
    ///
    /// # Arguments
    /// * `job_id` - Optional job UUID. If `None`, uses current job's ID from `WM_JOB_ID` env var
    ///
    /// # See Also
    /// - [Flow Progress Tracking](https://www.windmill.dev/docs/advanced/explicit_progress)
    /// - [`Self::set_progress`] - For updating job progress
    pub fn get_progress<'a>(
        &'a self,
        job_id: Option<String>,
    ) -> MaybeFuture<'a, Result<u32, SdkError>> {
        let f = async move {
            let job_id = job_id.unwrap_or(var("WM_JOB_ID")?);
            Ok(windmill_api::apis::metrics_api::get_job_progress(
                &self.client_config,
                &self.workspace,
                &job_id,
            )
            .await
            .map(|v| v as u32)?)
        };
        ret!(f);
    }

    /// Executes an API call in either asynchronous or synchronous mode based on compilation context.
    ///
    /// This function serves as a bridge between async and sync code, automatically adapting its behavior:
    /// - With `async` feature: Returns a boxed future for later await
    /// - Without `async` feature: Blocks immediately using the global runtime
    ///
    /// # Examples
    ///
    /// ## Async usage (with `async` feature)
    /// ```ignore
    /// use wmill::Windmill;
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()>{
    /// let wm = Windmill::default()?;
    /// let user = wm.call_api(wmill::apis::admin_api::get_user(&wm.client_config, &wm.workspace, "Bob"))?;
    /// println!("User details: {:?}", user);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Sync usage (without `async` feature)
    /// ```ignore
    /// use wmill::Windmill;
    /// let wm = Windmill::new(Some("<TOKEN>".into()), Some("admins".into()), Some("http://localhost:5000/api".into()))?;
    /// let user = wm.call_api(wmill::apis::admin_api::get_user(&wm.client_config, &wm.workspace, "Bob"));
    /// println!("User details: {:?}", user);
    /// # Ok::<(), wmill::SdkError>(())
    /// ```
    pub fn call_api<'a, T>(&'a self, callback: impl Future<Output = T>) -> MaybeFuture<'a, T> {
        ret!(callback);
    }

    // pub fn get_version() {}
}

pub enum JobStatus {
    Running,
    Waiting,
    Completed,
}

fn get_state_path() -> Result<String, SdkError> {
    Ok(var("WM_STATE_PATH_NEW").unwrap_or(var("WM_STATE_PATH")?))
}

#[derive(thiserror::Error, Debug)]
pub enum SdkError {
    #[error("Error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Having troubles reading Env Variable: {0}")]
    VarError(#[from] std::env::VarError),
    #[error("Api error: {0}")]
    ApiError(String),
    #[error("Internal Error: {0}")]
    InternalError(String),
    #[error("Specified value is incorrect: {0}")]
    BadValue(String),
    #[error("{0}")]
    ExecutionError(String),
}
impl<T: for<'a> Deserialize<'a>> From<windmill_api::apis::Error<T>> for SdkError {
    fn from(value: windmill_api::apis::Error<T>) -> Self {
        Self::ApiError(value.to_string())
    }
}
