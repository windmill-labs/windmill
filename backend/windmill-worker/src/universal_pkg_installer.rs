use std::sync::Arc;

use anyhow::bail;
use itertools::Itertools;
use process_wrap::tokio::TokioChildWrapper;
use tokio::{
    fs::File,
    process::Command,
    sync::{broadcast::Sender, OwnedSemaphorePermit, RwLock, Semaphore},
    task::JoinHandle,
};
use uuid::Uuid;
use windmill_common::{error, worker::Connection};

use crate::{common::start_child_process, is_sandboxing_enabled};

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct RequiredDependency<T: Clone + Send + Sync> {
    /// Expected directory of dependency in cache
    /// For example:
    /// /tmp/windmill/cache/python_311/rich==0.0.0
    /// IMPORTANT!: path should not end with '/'
    pub path: String,
    /// Name to use for S3 tars
    pub _s3_handle: String,
    /// Display name
    /// Name that will be used for console output and logging
    pub display_name: String,
    /// Custom payload can be used for passing information easily. If you wish not to use it just leave '()'
    pub custom_payload: T,
}

#[allow(dead_code)]
pub enum InstallStrategy<T: Clone + Send + Sync> {
    /// Will invoke callback to install single dependency
    Single(Arc<dyn Fn(RequiredDependency<T>) -> Result<Command, error::Error> + Send + Sync>),
    /// Will try to pull S3 first and will invoke closure to install the rest
    AllAtOnce(Arc<RwLock<Vec<RequiredDependency<T>>>>),
}

/// # General
///
/// Languages that compile usually include dependencies in final executable.
/// When dynamic languages do not and runtime dependencies are provided separately.
///
/// This helper assumes that the language is dynamic.
/// Python, Ruby, Java are dynamic and they can use this helper.
///
/// # Features
///
/// This helper will install all specified dependencies in parallel and if it is EE, cache to S3
/// It has atomic success file, allowing to distinguish failed installations from succesfull.
///
/// Besides that it provides console output and does logging.
///
/// # Usage
///
/// Most important arguments in this helper are `deps` and `install_fn`
///
/// In `deps` you specify all dependencies that are needed to be on worker in order to execute script.
/// You don't know which are actually installed and which are not.
///
/// `deps` is a vector of RequiredDependency. Check [RequiredDependency] for more context.
///
/// After `deps` are provided helper will check each dependency and check if it is in cache, if not it will try to pull from S3
/// and if it does not work either, it will invoke `install_fn` closure.
/// Closure arguments has dependency name as well as its expected path in cache.
/// Closure should return Command that will install dependency to asked place.
#[allow(dead_code)]
pub async fn par_install_language_dependencies_all_at_once<
    'a,
    T: Clone + std::marker::Send + Sync + 'a + 'static,
>(
    deps: Vec<RequiredDependency<T>>,
    _language_name: &'a str,
    installer_executable_name: &'a str,
    _platform_agnostic: bool,
    concurrent_downloads: usize,
    stdout_on_err: bool,
    callback: impl Fn(Vec<RequiredDependency<T>>) -> Result<Command, error::Error>
        + Send
        + Sync
        + 'static,
    postinstall_cb: impl AsyncFn(Vec<RequiredDependency<T>>) -> Result<(), error::Error>,
    job_id: &'a Uuid,
    w_id: &'a str,
    worker_name: &'a str,
    jailed: bool,
    conn: &'a Connection,
) -> anyhow::Result<()> {
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    let is_not_pro = !matches!(
        windmill_common::ee_oss::get_license_plan().await,
        windmill_common::ee_oss::LicensePlan::Pro
    );
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    if is_not_pro {
        windmill_queue::append_logs(
            job_id,
            w_id,
            format!("\nLooking for packages on S3:\n"),
            conn,
        )
        .await;
    }
    let total_time = std::time::Instant::now();
    let (missing, name_max_length) = filter_to_missing(deps, job_id, w_id, jailed, conn).await?;
    if missing.is_empty() {
        return Ok(());
    }
    let to_batch_install = Arc::new(RwLock::new(vec![]));
    let handles = spawn_wrapped_installation_threads(
        missing,
        name_max_length,
        InstallStrategy::AllAtOnce(to_batch_install.clone()),
        installer_executable_name,
        concurrent_downloads,
        job_id,
        w_id,
        worker_name,
        conn,
        _language_name,
        _platform_agnostic,
    )
    .await?;
    let installation_res = process_handles(handles, w_id).await;
    if !to_batch_install.read().await.is_empty() {
        let for_all_at_once_copy = to_batch_install.read().await.clone();
        windmill_queue::append_logs(
            job_id,
            w_id,
            format!("\n\nFetching {} packages...\n", for_all_at_once_copy.len()),
            &conn,
        )
        .await;
        let cmd = callback(for_all_at_once_copy.clone())?;
        let child = start_child_process(cmd, &installer_executable_name, false).await?;
        let mut buf = "".to_owned();
        let pipe_stdout = if stdout_on_err { Some(&mut buf) } else { None };

        if let Err(e) = crate::handle_child::handle_child(
            &(if pipe_stdout.is_some() {
                Uuid::nil()
            } else {
                *job_id
            }),
            conn,
            // TODO: Return mem_peak
            &mut 0,
            // TODO: Return canceld_by_ref
            &mut None,
            child,
            is_sandboxing_enabled(),
            &worker_name,
            &w_id,
            &installer_executable_name,
            None,
            false,
            &mut None,
            pipe_stdout,
            None,
        )
        .await
        {
            bail!(format!(
                "\nerror while installing dependencies: {e:?}\n{}",
                buf
            ));
        }
        {
            postinstall_cb(for_all_at_once_copy.clone()).await?;
        }
        for RequiredDependency { path, _s3_handle, .. } in for_all_at_once_copy.into_iter() {
            mark_success(path.clone(), job_id, w_id).await;
            #[cfg(all(feature = "enterprise", feature = "parquet"))]
            {
                if let Some(os) = windmill_object_store::get_object_store().await {
                    let language_name = _language_name.to_owned();
                    tokio::spawn(async move {
                        if let Err(e) = crate::global_cache::build_tar_and_push(
                            os,
                            path,
                            language_name,
                            Some(_s3_handle),
                            _platform_agnostic,
                        )
                        .await
                        {
                            tracing::warn!("failed to build tar and push: {e:?}");
                        }
                    });
                }
            }
        }
    }
    finish_installation(total_time, job_id, w_id, conn).await;
    installation_res
}

/// # General
///
/// Languages that compile usually include dependencies in final executable.
/// When dynamic languages do not and runtime dependencies are provided separately.
///
/// This helper assumes that the language is dynamic.
/// Python, Ruby, Java are dynamic and they can use this helper.
///
/// # Features
///
/// This helper will install all specified dependencies in parallel and if it is EE, cache to S3
/// It has atomic success file, allowing to distinguish failed installations from succesfull.
///
/// Besides that it provides console output and does logging.
///
/// # Usage
///
/// Most important arguments in this helper are `deps` and `install_fn`
///
/// In `deps` you specify all dependencies that are needed to be on worker in order to execute script.
/// You don't know which are actually installed and which are not.
///
/// `deps` is a vector of RequiredDependency. Check [RequiredDependency] for more context.
///
/// After `deps` are provided helper will check each dependency and check if it is in cache, if not it will try to pull from S3
/// and if it does not work either, it will invoke `install_fn` closure.
/// Closure arguments has dependency name as well as it`s expected path in cache.
/// Closure should return Command that will install dependency to asked place.
#[allow(dead_code)]
pub async fn par_install_language_dependencies_seq<
    'a,
    T: Clone + std::marker::Send + Sync + 'a + 'static,
>(
    deps: Vec<RequiredDependency<T>>,
    _language_name: &'a str,
    installer_executable_name: &'a str,
    _platform_agnostic: bool,
    concurrent_downloads: usize,
    callback: impl Fn(RequiredDependency<T>) -> Result<Command, error::Error> + Send + Sync + 'static,
    job_id: &'a Uuid,
    w_id: &'a str,
    worker_name: &'a str,
    jailed: bool,
    conn: &'a Connection,
) -> anyhow::Result<()> {
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    let is_not_pro = !matches!(
        windmill_common::ee_oss::get_license_plan().await,
        windmill_common::ee_oss::LicensePlan::Pro
    );
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    if is_not_pro {
        windmill_queue::append_logs(
            job_id,
            w_id,
            format!("\nLooking for packages on S3:\n"),
            conn,
        )
        .await;
    }
    let total_time = std::time::Instant::now();
    let (missing, name_max_length) = filter_to_missing(deps, job_id, w_id, jailed, conn).await?;
    if missing.is_empty() {
        return Ok(());
    }
    let handles = spawn_wrapped_installation_threads(
        missing,
        name_max_length,
        InstallStrategy::Single(Arc::new(callback)),
        installer_executable_name,
        concurrent_downloads,
        job_id,
        w_id,
        worker_name,
        conn,
        _language_name,
        _platform_agnostic,
    )
    .await?;

    let installation_res = process_handles(handles, w_id).await;
    finish_installation(total_time, job_id, w_id, conn).await;
    installation_res
}

type NameMaxLength = usize;
async fn filter_to_missing<'a, T: Clone + std::marker::Send + Sync + 'a + 'static>(
    mut deps: Vec<RequiredDependency<T>>,
    job_id: &Uuid,
    w_id: &str,
    jailed: bool,
    conn: &Connection,
) -> anyhow::Result<(Vec<RequiredDependency<T>>, NameMaxLength)> {
    // Unique to flatten all same values
    deps = deps.into_iter().unique_by(|rd| rd.path.clone()).collect();
    // Total to install
    let mut missing = vec![];
    // Name max length
    let mut name_ml = 0;
    for rd in deps.into_iter() {
        let display_name = rd.display_name.clone();
        if rd.path.ends_with("/") {
            anyhow::bail!("Internal error: path should not end with '/'")
        }
        {
            // Later will help us align text in log console
            if display_name.len() > name_ml {
                name_ml = rd.display_name.len();
            }
        }
        // Will look like: /tmp/windmill/cache/lang/dependency.valid.windmill
        if tokio::fs::metadata(rd.path.clone() + ".valid.windmill")
            .await
            .is_err()
        {
            missing.push(rd);
        }
    }
    if !missing.is_empty() {
        windmill_queue::append_logs(
            job_id,
            w_id,
            if jailed {
                format!("\n--- ISOLATED INSTALLATION ---\n\nTo be installed:\n\n")
            } else {
                format!("\n--- INSTALLATION ---\n\nTo be installed:\n\n")
            },
            conn,
        )
        .await;
        let to_log = missing
            .iter()
            .map(|rd| format!("- {}", &rd.display_name))
            .join("\n")
            + "\n";

        windmill_queue::append_logs(job_id, w_id, to_log, conn).await;
    }
    Ok((missing, name_ml))
}

enum Action<T: Clone + Send + Sync> {
    Install(Box<dyn TokioChildWrapper + 'static>),
    AddToBulk(Arc<RwLock<Vec<RequiredDependency<T>>>>),
}
struct TaskKiller(tokio::sync::broadcast::Sender<String>);
impl Drop for TaskKiller {
    fn drop(&mut self) {
        // We don't care if it actually stopped anything or not.
        // In some cases it will fire and actually stop
        // In other cases do nothing
        #[allow(unused_must_use)]
        self.0.send("Installation failed".to_owned());
    }
}
// Callback will be invoked only if could not pull from S3
async fn spawn_wrapped_installation_threads<
    'a,
    T: Clone + std::marker::Send + Sync + 'a + 'static,
>(
    missing: Vec<RequiredDependency<T>>,
    name_ml: usize,
    strategy: InstallStrategy<T>,
    installer_executable_name: &str,
    concurrent_downloads: usize,
    job_id: &Uuid,
    w_id: &str,
    worker_name: &str,
    conn: &Connection,
    _language_name: &str,
    _platform_agnostic: bool,
) -> anyhow::Result<(
    Vec<JoinHandle<anyhow::Result<TaskKiller>>>,
    tokio::sync::broadcast::Sender<()>,
)> {
    // Semaphore will panic if value less then 1
    let parallel_limit = concurrent_downloads.clamp(1, 30);
    tracing::info!(
        workspace_id = %w_id,
        "Install parallel limit: {}, job: {}",
        parallel_limit,
        job_id
    );

    let (mut handles, semaphore, total_to_install, counter_arc) = (
        vec![],
        Arc::new(Semaphore::new(parallel_limit)),
        missing.len(),
        Arc::new(tokio::sync::Mutex::new(0)),
    );

    // Pretty sensitive. Single drop will fail installation
    let (kill_tx, ..) = tokio::sync::broadcast::channel::<String>(10);
    //   ^^^^^^^ Original will go into listen_to_cancel
    //
    // But before that it will be used to populate t and r for every thread
    let trxs: Vec<(
        tokio::sync::broadcast::Sender<String>,
        tokio::sync::broadcast::Receiver<String>,
    )> = (0..missing.len())
        .map(|_| (kill_tx.clone(), kill_tx.subscribe()))
        .collect();

    // Listen to cancel events and stop jobs
    // NOTE:
    // once closer is dropped, the listener will be closed
    let closer = listen_to_cancel(job_id.clone(), w_id.to_owned(), conn.clone(), kill_tx).await;

    for (dep, (kill_tx, mut kill_rx)) in missing.into_iter().zip(trxs) {
        let permit = semaphore.clone().acquire_owned().await; // Acquire a permit

        if let Err(_) = permit {
            tracing::error!(
                workspace_id = %w_id,
                "Cannot acquire permit on semaphore, that can only mean that semaphore has been closed."
            );
            break;
        }

        let permit = permit.unwrap();

        let action = match strategy {
            InstallStrategy::Single(ref callback) => Action::Install(
                start_child_process(callback(dep.clone())?, &installer_executable_name, false)
                    .await?,
            ),
            InstallStrategy::AllAtOnce(ref rw_lock) => Action::AddToBulk(Arc::clone(rw_lock)),
        };
        let task_fut = try_install_one_detached(
            dep,
            installer_executable_name.to_owned(),
            job_id.clone(),
            w_id.to_owned(),
            worker_name.to_owned(),
            conn.to_owned(),
            counter_arc.clone(),
            total_to_install,
            name_ml,
            action,
            _language_name.to_owned(),
            _platform_agnostic,
            permit,
            TaskKiller(kill_tx),
        );
        handles.push(tokio::spawn(async move {
            tokio::select! {
                reason = kill_rx.recv() => return Err(anyhow::anyhow!("Installation stopped. Reason: {:?}", reason)),
                r = task_fut => return r
            };
        }));
    }

    Ok((handles, closer))
}
/// Do not drop the returned sender! As soon as you drop it the polling thread will stop
pub async fn listen_to_cancel(
    job_id: Uuid,
    w_id: String,
    conn: Connection,
    killpill: tokio::sync::broadcast::Sender<String>,
) -> tokio::sync::broadcast::Sender<()> {
    let (close_tx, mut close_rx) = tokio::sync::broadcast::channel::<()>(1);
    tokio::spawn(async move {
        'outer: loop {
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                    // TODO: Add memory usage tracking
                    if crate::worker_utils::ping_job_status(&conn, &job_id, None, None).await.map(|r| r.canceled_by.is_some()).unwrap_or(false){
                        tracing::info!(
                            // If there is listener on other side,
                            workspace_id = %w_id,
                            "cancelling installations",
                        );
                        if let Err(ref e) = killpill.send("Job was canceled".to_owned()) {
                            tracing::error!(
                                // If there is listener on other side,
                                workspace_id = %w_id,
                                "failed to send done: Probably receiving end closed too early or have not opened yet\n{}",
                                // If there is no listener, it will be dropped safely
                                e
                            );
                        }
                        // Stop
                        break 'outer;
                    }

                },
                _ = close_rx.recv() => break 'outer,
            }
        }
    });

    // We will return the sender that can stop created thread.
    // It is relatively safe, since if this handle is being dropped from the gecko the thread will stop immediately
    // but it will also chainreaction all installation threads, so be carefull
    close_tx
}

async fn try_install_one_detached<'a, T: Clone + std::marker::Send + Sync + 'a + 'static>(
    dep: RequiredDependency<T>,
    installer_executable_name: String,
    job_id: Uuid,
    w_id: String,
    worker_name: String,
    conn: Connection,
    counter_arc: Arc<tokio::sync::Mutex<usize>>,
    total_to_install: usize,
    name_ml: usize,
    action: Action<T>,
    _language_name: String,
    _platform_agnostic: bool,
    _permit: OwnedSemaphorePermit,
    // If dropped the entire installation fails and all installation threads are being stopped
    // That's why we just pass it to return so it is not being dropped
    kill_all_tasks: TaskKiller,
) -> anyhow::Result<TaskKiller> {
    let start = std::time::Instant::now();

    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    let is_not_pro = !matches!(
        windmill_common::ee_oss::get_license_plan().await,
        windmill_common::ee_oss::LicensePlan::Pro
    );

    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    let s3_pull_future = if is_not_pro {
        if let Some(os) = windmill_object_store::get_object_store().await {
            Some(crate::global_cache::pull_from_tar(
                os,
                dep.path.clone(),
                _language_name.to_owned(),
                Some(dep._s3_handle.clone()),
                _platform_agnostic,
            ))
        } else {
            None
        }
    } else {
        None
    };

    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    if let Some(s3_pull_future) = s3_pull_future {
        if let Err(e) = s3_pull_future.await {
            tracing::info!(
                workspace_id = %w_id,
                "No tarball was found for {:?} on S3 or different problem occurred {job_id}:\n{e}",
                &dep._s3_handle.clone()
            );
        } else {
            mark_success(dep.path, &job_id, &w_id).await;
            print_success(
                true,
                false,
                &job_id,
                &w_id,
                &dep.display_name,
                name_ml,
                counter_arc,
                total_to_install,
                start,
                &conn,
            )
            .await;
            return Ok(kill_all_tasks);
        }
    }
    let child = match action {
        Action::Install(child) => child,
        Action::AddToBulk(rw_lock) => {
            rw_lock.write().await.push(dep.clone());
            return Ok(kill_all_tasks);
        }
    };
    if let Err(e) = crate::handle_child::handle_child(
        &job_id,
        &conn,
        // TODO: Return mem_peak
        &mut 0,
        // TODO: Return canceld_by_ref
        &mut None,
        child,
        is_sandboxing_enabled(),
        &worker_name,
        &w_id,
        &installer_executable_name,
        None,
        false,
        &mut None,
        None,
        None,
    )
    .await
    {
        windmill_queue::append_logs(
            &job_id,
            &w_id,
            format!(
                "\n[x] - error while installing {}: {e:?}",
                &dep.display_name
            ),
            &conn,
        )
        .await;
        bail!(format!(
            "\n error while installing dependencies: {e:?}\n{}",
            &dep.display_name
        ));
    } else {
        mark_success(dep.path.clone(), &job_id, &w_id).await;
        print_success(
            false,
            true,
            &job_id,
            &w_id,
            &dep.display_name,
            name_ml,
            counter_arc,
            total_to_install,
            start,
            &conn,
        )
        .await;

        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        {
            if let Some(os) = windmill_object_store::get_object_store().await {
                let language_name = _language_name.to_string();
                let platform_agnostic = _platform_agnostic;
                let path = dep.path.clone();
                let handle = dep._s3_handle.clone();
                tokio::spawn(async move {
                    if let Err(e) = crate::global_cache::build_tar_and_push(
                        os,
                        path,
                        language_name,
                        Some(handle),
                        platform_agnostic,
                    )
                    .await
                    {
                        tracing::warn!("failed to build tar and push: {e:?}");
                    }
                });
            }
        }
    }

    Ok(kill_all_tasks)
}

// Create a file to indicate that installation was successfull
async fn mark_success(path: String, job_id: &Uuid, w_id: &str) {
    let valid_path = path + ".valid.windmill";
    // This is atomic operation, meaning, that it either completes and dependency is valid,
    // or it does not and dependency is invalid and will be reinstalled next run
    if let Err(e) = File::create(&valid_path).await {
        tracing::error!(
            workspace_id = %w_id,
            job_id = %job_id,
            "Failed to create {}!\n{e}\n
                This file needed for jobs to function",
            valid_path
        );
    };
}
// Append logs with line like this:
// [9/21]   +  requests==2.32.3            << (S3) |  in 57ms
#[allow(unused_assignments)]
async fn print_success(
    mut s3_pull: bool,
    mut s3_push: bool,
    job_id: &Uuid,
    w_id: &str,
    req: &str,
    req_tl: usize,
    counter_arc: Arc<tokio::sync::Mutex<usize>>,
    total_to_install: usize,
    instant: std::time::Instant,
    conn: &Connection,
) {
    #[cfg(not(all(feature = "enterprise", feature = "parquet")))]
    {
        (s3_pull, s3_push) = (false, false);
    }

    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    if windmill_object_store::OBJECT_STORE_SETTINGS
        .read()
        .await
        .is_none()
    {
        (s3_pull, s3_push) = (false, false);
    }

    let mut counter = counter_arc.lock().await;
    *counter += 1;

    windmill_queue::append_logs(
        job_id,
        w_id,
        format!(
            "\n{}+  {}{}{}|  in {}ms",
            windmill_common::worker::pad_string(&format!("[{}/{total_to_install}]", counter), 9),
            // Because we want to align to max len [999/999] we take 9
            //                                     123456789
            windmill_common::worker::pad_string(&req, req_tl + 1),
            // Margin to the right    ^
            if s3_pull { "<< (S3) " } else { "" },
            if s3_push { " > (S3) " } else { "" },
            instant.elapsed().as_millis(),
        ),
        conn,
    )
    .await;
    // Drop lock, so next print success can fire
}

async fn process_handles(
    (handles, closer): (Vec<JoinHandle<anyhow::Result<TaskKiller>>>, Sender<()>),
    w_id: &str,
) -> anyhow::Result<()> {
    let mut killers = vec![];
    for handle in handles {
        match handle
            .await
            .unwrap_or(Err(anyhow::anyhow!("Problem by joining handle")))
        {
            // __________< Cannot drop just yet.
            Ok(kill_tasks) => killers.push(kill_tasks),
            Err(e) => {
                tracing::warn!(
                    workspace_id = %w_id,
                    "Env installation failed: {:?}",
                    e
                );
                // We can safely return,
                // it will drop `closer`
                // which will drop first kill_tx for all installation threads
                // which will trigger chain reaction and kill every installation
                return Err(e);
            }
        }
    }

    // Only now we can be sure there is no installation thread running
    // we can safely stop poller
    drop(closer);
    // Drop ONLY after we processed all handles
    drop(killers);
    Ok(())
}
async fn finish_installation(
    total_time: std::time::Instant,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
) {
    let total_time = total_time.elapsed().as_millis();
    windmill_queue::append_logs(
        &job_id,
        w_id,
        format!(
            "\nDone. Time spent on installation phase: {}ms\n",
            total_time
        ),
        conn,
    )
    .await;
}
