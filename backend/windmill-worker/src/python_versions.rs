use std::{
    ops::{Deref, DerefMut},
    process::Stdio,
    str::FromStr,
    sync::Arc,
};

use chrono::{DateTime, Duration, Utc};
use itertools::Itertools;
use serde_json::Value;
use tokio::{fs::DirBuilder, process::Command, sync::RwLock};
use uuid::Uuid;
use windmill_common::{
    error::{self, Error},
    worker::{try_parse_locked_python_version_from_requirements, Connection, PyVAlias},
};

use anyhow::{anyhow, bail};
use windmill_queue::append_logs;

use crate::{
    common::{start_child_process, OccupancyMetrics},
    handle_child::handle_child,
    python_executor::{INDEX_CERT, NATIVE_CERT, PYTHON_PATH},
    HOME_ENV, INSTANCE_PYTHON_VERSION, PATH_ENV, PROXY_ENVS, PY_INSTALL_DIR, UV_CACHE_DIR,
    WIN_ENVS,
};
#[cfg(unix)]
use crate::python_executor::UV_PATH;

impl From<PyV> for PyVAlias {
    fn from(value: PyV) -> Self {
        match value.release() {
            [major, minor, ..] => {
                if let Some(alias) = Self::try_from_v1(format!("{}{}", *major, *minor)) {
                    return alias;
                }
            }
            _ => (),
        }

        tracing::warn!(
            "Failed to convert Python Full Version to Alias. Fallback to default ({})",
            *PyV::default()
        );
        Self::default()
    }
}

// To change latest stable version:
// 1. Change placeholder in instanceSettings.ts
// 2. Change LATEST_STABLE_PY in dockerfile
// 3. Change #[default] annotation for PyVersion in backend
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PyV(pub pep440_rs::Version);

impl From<pep440_rs::Version> for PyV {
    fn from(value: pep440_rs::Version) -> Self {
        Self(value)
    }
}

impl From<PyVAlias> for PyV {
    fn from(value: PyVAlias) -> Self {
        Self(value.into())
    }
}

impl Default for PyV {
    fn default() -> Self {
        PyVAlias::default().into()
    }
}

impl Deref for PyV {
    type Target = pep440_rs::Version;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for PyV {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PyV {
    pub async fn resolve(
        version_specifiers: Vec<pep440_rs::VersionSpecifier>,
        job_id: &Uuid,
        w_id: &str,
        select_latest: bool,
        // Needed for logs but optional
        conn: Option<Connection>,
        // Usually for testing
        custom_versions: Option<Vec<PyV>>,
        // For testing
        gravitational_version: Option<PyV>,
    ) -> Result<Self, Error> {
        // Get all versions that can be fetched
        let all_versions = custom_versions.unwrap_or(PyV::list_available_python_versions().await);

        // Narrow down to those that satisfy given version specifiers
        let valid = all_versions
            .clone()
            .into_iter()
            .filter(|v| version_specifiers.iter().all(|vs| (vs).contains(&*v)))
            .collect_vec();

        if !valid.is_empty() {
            let mut result = valid[0].clone();
            // Is there at least one version specifier that has PATCH digit?
            let patch_vs = version_specifiers
                .iter()
                .any(|vs| vs.version().release().get(2).is_some());

            if select_latest {
                return Ok(result.clone());
            }

            // Usually INSTANCE_PYTHON_VERSION
            let gv = gravitational_version
                .unwrap_or(PyV::gravitational_version(job_id, w_id, conn).await);

            // Will be used to determine if picked version matches gravity version
            // Once first match occure, we will stop iterating
            let gravity_matcher = pep440_rs::VersionSpecifier::from_version(
                pep440_rs::Operator::EqualStar,
                (*gv).clone(),
            )
            .map_err(|e| {
                Error::ArgumentErr(format!(
                    "{e}\nLikely means INSTANCE_PYTHON_VERSION is set incorrectly."
                ))
            })?;

            // Reminder of semver: MAJOR.MINOR.PATCH
            //
            // - Go from up to down
            // - We will iterate until find the closest version to target.
            //   - If closest version has the same MINOR version, use it.
            //   - If it differs in MINOR version, take latest PATCH version.

            let [major, minor, ..] = result.release() else {
                return Err(Error::InternalErr(format!("Failed to parse \"{}\". Available python versions are supposed to be in SEMVER format (MAJOR.MINOR)", *result)));
            };
            // This represents newest version with oldest MINOR:
            //
            // I Iterable   Newest in MINOR
            // 1. 3.11.2 -> 3.11.2
            // 2. 3.11.1 -> 3.11.2
            // 3. 3.11.0 -> 3.11.2
            // 4. 3.10.2 -> 3.10.2
            // 5. 3.10.1 -> 3.10.2
            // 6. 3.10.0 -> 3.10.2
            let mut newest_in_minor = (result.clone(), (*major, *minor));

            for v in valid.iter() {
                if v < &gv {
                    // We will not continue if we start looking into versions older than gravity version.
                    break;
                }

                let [major, minor, ..] = v.release() else {
                    return Err(Error::InternalErr(format!("Failed to parse \"{}\". Available python versions are supposed to be in SEMVER format (MAJOR.MINOR)", **v)));
                };

                // Since we go top to down we can assume
                // the first occurence of new minor version contains the latest patch version.
                if newest_in_minor.1 != (*major, *minor) {
                    newest_in_minor = (v.clone(), (*major, *minor));
                }

                if gravity_matcher.contains(v) {
                    // return as soon as gravity matcher has first hit.
                    // Only in case version specifiers do specify PATCH version OR gravity version specify PATCH
                    if patch_vs || gv.release().get(2).is_some() {
                        return Ok(v.clone());
                    } else {
                        let Some(release_numbers) = v.release().get(0..=1) else {
                            return Err(Error::InternalErr(format!(
                                "Failed to get release numbers from: \"{}\". ",
                                **v
                            )));
                        };
                        return Ok(PyV(pep440_rs::Version::new(release_numbers)));
                    }
                }
                // If we are still in the loop, it means that we are getting closer to gravity version
                else {
                    result = v.clone();
                }
            }

            let [gravity_major, gravity_minor, ..] = gv.release() else {
                return Err(Error::internal_err(format!("Cannot get MAJOR nor MINOR version of python gravity version ({}). Something might be wrong with INSTANCE_PYTHON_VERSION.", &*gv)));
            };

            if (*gravity_major, *gravity_minor) != newest_in_minor.1 {
                // Return full version only if there is PATCH versions in version specifiers
                if patch_vs {
                    return Ok(newest_in_minor.0);
                } else {
                    let mm = newest_in_minor.1;
                    return Ok(PyV(pep440_rs::Version::new([mm.0, mm.1])));
                }
            }

            Ok(result)
        } else {
            Err(anyhow!(
                "
  × No solution found when resolving python:
  ╰─▶ Because you require python {}, we can conclude that your requirements are unsatisfiable.

  All versions: \n{}
                \n",
                version_specifiers.iter().map(|s| s.to_string()).join(", "),
                all_versions
                    .iter()
                    .enumerate()
                    .map(|(i, v)| format!(
                        "{}{}",
                        windmill_common::worker::pad_string(&v.0.to_string(), 11),
                        if (i + 1) % 5 == 0 { "\n" } else { "" }
                    ))
                    .collect::<String>()
            )
            .into())
        }
    }
    /// e.g.: `/tmp/windmill/cache/python_3_x_y`
    pub(crate) fn to_cache_dir(&self, ignore_patch: bool) -> String {
        use windmill_common::worker::ROOT_CACHE_DIR;
        format!(
            "{ROOT_CACHE_DIR}{}",
            self.to_cache_dir_top_level(ignore_patch)
        )
    }

    /// e.g.: `python_3_x_y`
    pub fn to_cache_dir_top_level(&self, ignore_patch: bool) -> String {
        if ignore_patch {
            if let [major, minor, ..] = self.release() {
                return format!("python_{major}_{minor}");
            }

            tracing::warn!("failed to parse python's ({}) top level directory with no patch digit, fallback to full version.", self.to_string());
        }
        format!("python_{}", self.to_string().replace(".", "_"))
    }

    pub async fn gravitational_version(
        job_id: &Uuid,
        w_id: &str,
        conn: Option<Connection>,
    ) -> Self {
        let mut err = None;
        let pyv = match INSTANCE_PYTHON_VERSION.read().await.clone() {
            Some(v) if &v == "default" => PyVAlias::default().into(),
            Some(v) => pep440_rs::Version::from_str(&v).unwrap_or_else(|_| {
                let v = PyVAlias::default().into();
                err = Some(format!("\nCannot parse INSTANCE_PYTHON_VERSION ({:?}), fallback to latest_stable ({v:?})", *INSTANCE_PYTHON_VERSION));
                v
            }),
            // Use latest stable
            None => PyVAlias::default().into(),
        };

        if let Some(msg) = err {
            if let Some(conn) = conn {
                append_logs(job_id, w_id, &msg, &conn).await;
            }
            tracing::error!(msg);
        }
        pyv.into()
    }

    pub async fn list_available_python_versions() -> Vec<Self> {
        match Self::list_available_python_versions_inner().await {
            Ok(pyvs) => pyvs,
            Err(e) => {
                tracing::error!(
                    "Fallback to preconfigured aliases. Cannot list python versions due to this error: {e}"
                );
                PyVAlias::all()
            }
        }
    }
    async fn list_available_python_versions_inner() -> anyhow::Result<Vec<Self>> {
        lazy_static::lazy_static! {
            static ref CACHED_VERSIONS: Arc<RwLock<Option<Vec<PyV>>>> = Arc::new(RwLock::new(None));
            static ref LAST_CHECKED: Arc<RwLock<DateTime<Utc>>> = Arc::new(RwLock::new(Utc::now()));
        }
        match (
            Utc::now().signed_duration_since(*LAST_CHECKED.read().await) > Duration::minutes(30),
            CACHED_VERSIONS.read().await.clone(),
        ) {
            (false, Some(vs)) => return Ok(vs),
            _ => {}
        };

        let output = {
            #[cfg(windows)]
            let uv_cmd = "uv";

            #[cfg(unix)]
            let uv_cmd = UV_PATH.as_str();

            Command::new(uv_cmd)
                .env_clear()
                .envs(WIN_ENVS.to_vec())
                .env("UV_CACHE_DIR", UV_CACHE_DIR)
                .args([
                    "python",
                    "list",
                    "--all-versions",
                    "--output-format",
                    "json",
                ])
                .stderr(Stdio::piped())
                .output()
                .await?
        };

        // We want to skip all versions smaller then 3.10
        // Windmill is incompatible with 3.9 and older
        let filter = pep440_rs::VersionSpecifier::from_version(
            pep440_rs::Operator::GreaterThanEqual,
            PyVAlias::Py310.into(),
        )?;

        if output.status.success() {
            let res = String::from_utf8(output.stdout)?;
            let list = serde_json::from_str::<Vec<serde_json::Map<String, Value>>>(&res)?
                .into_iter()
                .filter_map(|e| {
                    if e.get("implementation").and_then(Value::as_str) == Some("pypy") {
                        None
                    } else {
                        Some(
                            e.get("version")
                                .and_then(Value::as_str)
                                .and_then(|s| pep440_rs::Version::from_str(s).ok())
                                .map(PyV::from)
                                .ok_or(Error::internal_err("version is None")),
                        )
                    }
                })
                .collect::<Result<Vec<PyV>, Error>>()?
                .into_iter()
                .unique()
                .sorted()
                .filter(|pyv| filter.contains(&*pyv))
                .rev()
                .collect_vec();

            *LAST_CHECKED.write().await = Utc::now();
            CACHED_VERSIONS.write().await.replace(list.clone());

            Ok(list)
        } else {
            // If the command failed, print the error
            let stderr = String::from_utf8(output.stderr)?;
            bail!(
                "Cannot list python versions, is uv (0.5.19 and newer) installed? Err:\n{}",
                stderr
            );
        }
    }

    /// Parse lockfile for assigned python version.
    /// If not found returns 3.11
    pub fn parse_from_requirements<S: AsRef<str>>(requirements_lines: &[S]) -> Self {
        Self::try_parse_from_requirements(requirements_lines).unwrap_or(
            // If there is no assigned version in lockfile we automatically fallback to 3.11
            // In this case we have dependencies or other metadata, but no associated python version
            // This is the case for old deployed scripts
            PyVAlias::default().into(),
        )
    }

    /// Parse lockfile for assigned python version.
    /// If not found returns None
    pub fn try_parse_from_requirements<S: AsRef<str>>(requirements_lines: &[S]) -> Option<Self> {
        try_parse_locked_python_version_from_requirements(requirements_lines).map(PyV::from)
    }

    pub async fn get_python(
        &self,
        worker_name: &str,
        job_id: &Uuid,
        w_id: &str,
        mem_peak: &mut i32,
        conn: &Connection,
        occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
    ) -> windmill_common::error::Result<String> {
        let python_path = if let Some(python_path) = PYTHON_PATH.clone() {
            python_path
        } else if let Some(python_path) = self
            .try_get_python(
                &job_id,
                mem_peak,
                conn,
                worker_name,
                w_id,
                occupancy_metrics,
            )
            .await?
        {
            python_path
        } else {
            return Err(Error::ExecutionErr(format!(
                "uv could not manage python path. Please manage it manually by setting PYTHON_PATH environment variable to your python binary path"
            )));
        };
        Ok(python_path)
    }

    pub async fn try_get_python(
        &self,
        job_id: &Uuid,
        mem_peak: &mut i32,
        // canceled_by: &mut Option<CanceledBy>,
        conn: &Connection,
        worker_name: &str,
        w_id: &str,
        occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
    ) -> error::Result<Option<String>> {
        // lazy_static::lazy_static! {
        //     static ref PYTHON_PATHS: Arc<RwLock<HashMap<PyVersion, String>>> = Arc::new(RwLock::new(HashMap::new()));
        // }

        let res = self
            .get_python_inner(job_id, mem_peak, conn, worker_name, w_id, occupancy_metrics)
            .await;

        if let Err(ref e) = res {
            tracing::error!(
                "worker_name: {worker_name}, w_id: {w_id}, job_id: {job_id}\n
                Error while getting python from uv, falling back to system python: {e:?}"
            );
            append_logs(
                job_id,
                w_id,
                format!(
                    "\nError while getting python from uv, falling back to system python: {e:?}"
                ),
                conn,
            )
            .await;
        }
        res
    }

    async fn get_python_inner(
        &self,
        job_id: &Uuid,
        mem_peak: &mut i32,
        // canceled_by: &mut Option<CanceledBy>,
        conn: &Connection,
        worker_name: &str,
        w_id: &str,
        occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
    ) -> error::Result<Option<String>> {
        let py_path = self.find_python().await;

        // Runtime is not installed
        if let Err(py_err) = py_path {
            // Install it
            if let Err(err) = self
                .install_python(job_id, mem_peak, conn, worker_name, w_id, occupancy_metrics)
                .await
            {
                tracing::error!(
                    "Cannot install python: {err}, after runtime wasn't found: {py_err}"
                );
                return Err(err);
            } else {
                // Try to find one more time
                let py_path = self.find_python().await;

                if let Err(err) = py_path {
                    tracing::error!(
                        "Cannot find python version {err} after runtime wasn't found: {py_err}"
                    );
                    return Err(err);
                }

                // TODO: Cache the result
                py_path
            }
        } else {
            py_path
        }
    }
    async fn install_python(
        &self,
        job_id: &Uuid,
        mem_peak: &mut i32,
        // canceled_by: &mut Option<CanceledBy>,
        conn: &Connection,
        worker_name: &str,
        w_id: &str,
        occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
    ) -> error::Result<()> {
        let v = self.to_string();
        append_logs(job_id, w_id, format!("\nINSTALLING PYTHON ({})", v), conn).await;
        // Create dirs for newly installed python
        // If we dont do this, NSJAIL will not be able to mount cache
        // For the default version directory created during startup (main.rs)
        DirBuilder::new()
            .recursive(true)
            .create(self.to_cache_dir(false))
            .await
            .expect("could not create initial worker dir");

        let logs = String::new();

        #[cfg(windows)]
        let uv_cmd = "uv";

        #[cfg(unix)]
        let uv_cmd = UV_PATH.as_str();

        let mut child_cmd = Command::new(uv_cmd);
        child_cmd
            .env_clear()
            .env("HOME", HOME_ENV.to_string())
            .env("PATH", PATH_ENV.to_string())
            .envs(PROXY_ENVS.clone())
            .args([
                "python",
                "install",
                &v,
                "--python-preference=only-managed",
                "--no-bin",
            ])
            // TODO: Do we need these?
            .envs([
                ("UV_PYTHON_INSTALL_DIR", PY_INSTALL_DIR),
                ("UV_CACHE_DIR", UV_CACHE_DIR),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            child_cmd
                .env("SystemRoot", crate::SYSTEM_ROOT.as_str())
                .env("USERPROFILE", crate::USERPROFILE_ENV.as_str())
                .env(
                    "TMP",
                    std::env::var("TMP").unwrap_or_else(|_| String::from("/tmp")),
                )
                .env(
                    "LOCALAPPDATA",
                    std::env::var("LOCALAPPDATA")
                        .unwrap_or_else(|_| format!("{}\\AppData\\Local", HOME_ENV.as_str())),
                );
        }

        let child_process = start_child_process(child_cmd, "uv", false).await?;

        append_logs(&job_id, &w_id, logs, conn).await;
        handle_child(
            job_id,
            conn,
            mem_peak,
            &mut None,
            child_process,
            false,
            worker_name,
            &w_id,
            "uv",
            None,
            false,
            occupancy_metrics,
            None,
            None,
        )
        .await?;
        Ok(())
    }
    async fn find_python(&self) -> error::Result<Option<String>> {
        #[cfg(windows)]
        let uv_cmd = "uv";

        #[cfg(unix)]
        let uv_cmd = UV_PATH.as_str();

        let mut child_cmd = Command::new(uv_cmd);

        child_cmd.env_clear();

        #[cfg(windows)]
        {
            child_cmd
                .env("SystemRoot", crate::SYSTEM_ROOT.as_str())
                .env("USERPROFILE", crate::USERPROFILE_ENV.as_str())
                .env(
                    "TMP",
                    std::env::var("TMP").unwrap_or_else(|_| String::from("/tmp")),
                )
                .env(
                    "LOCALAPPDATA",
                    std::env::var("LOCALAPPDATA")
                        .unwrap_or_else(|_| format!("{}\\AppData\\Local", HOME_ENV.as_str())),
                );
        }

        let mut vars: Vec<(&str, &str)> = vec![];
        if let Some(cert_path) = INDEX_CERT.as_ref() {
            vars.push(("SSL_CERT_FILE", cert_path));
        }

        if *NATIVE_CERT {
            vars.push(("UV_NATIVE_TLS", "true"));
        }
        let output = child_cmd
            // .current_dir(job_dir)
            .env("HOME", HOME_ENV.to_string())
            .env("PATH", PATH_ENV.to_string())
            .envs(vars)
            .args([
                "python",
                "find",
                &self.to_string(),
                "--system",
                "--python-preference=only-managed",
            ])
            .envs([
                ("UV_PYTHON_INSTALL_DIR", PY_INSTALL_DIR),
                ("UV_PYTHON_PREFERENCE", "only-managed"),
                ("UV_CACHE_DIR", UV_CACHE_DIR),
            ])
            // .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        // Check if the command was successful
        if output.status.success() {
            // Convert the output to a String
            let stdout =
                String::from_utf8(output.stdout).expect("Failed to convert output to String");
            return Ok(Some(stdout.replace('\n', "")));
        } else {
            // If the command failed, print the error
            let stderr =
                String::from_utf8(output.stderr).expect("Failed to convert error output to String");
            return Err(error::Error::FindPythonError(stderr));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unsafe helper for testing
    fn pyv(value: &str) -> PyV {
        pep440_rs::Version::from_str(value).unwrap().into()
    }

    async fn assert_resolution(
        instance_version: &str,
        select_highest: bool,
        specifiers: Vec<&str>,
        available: Vec<PyV>,
        expected: PyV,
    ) {
        let resolved = PyV::resolve(
            specifiers
                .into_iter()
                .map(|s| pep440_rs::VersionSpecifier::from_str(s).unwrap())
                .collect_vec(),
            &Uuid::nil(),
            "",
            select_highest,
            None,
            Some(available),
            Some(pyv(instance_version)),
        )
        .await
        .unwrap();
        assert_eq!(expected, resolved);
    }

    #[tokio::test]
    async fn test_python_resolution_1() {
        assert_resolution(
            "1.0",
            false,
            vec![],
            vec![
                pyv("1.2.0"),
                pyv("1.1.0"),
                pyv("1.0.0"),
                pyv("0.9.0"), //
            ],
            pyv("1.0.0"), //
        )
        .await;
    }
    #[tokio::test]
    async fn test_python_resolution_2() {
        assert_resolution(
            "1.0.0",
            false,
            vec!["!=1.*"],
            vec![
                pyv("1.2"),
                pyv("1.1"),
                pyv("1.0.2"),
                pyv("1.0.1"),
                pyv("1.0.0"),
                pyv("0.9.4"),
                pyv("0.9.3"),
                pyv("0.9.2"),
            ],
            pyv("0.9"), //
        )
        .await;
    }
    #[tokio::test]
    async fn test_python_resolution_3() {
        assert_resolution(
            "0.9",
            false,
            vec!["!=0.9.*"],
            vec![
                pyv("1.2"),
                pyv("1.1"),
                pyv("1.0.2"),
                pyv("1.0.1"),
                pyv("1.0.0"),
                pyv("0.9.4"),
                pyv("0.9.3"),
                pyv("0.9.2"),
                pyv("0.8.2"),
                pyv("0.8.1"),
                pyv("0.8.0"),
            ],
            pyv("1.0"), //
        )
        .await;
    }
    #[tokio::test]
    async fn test_python_resolution_4() {
        assert_resolution(
            "0.9",
            false,
            vec!["<=0.8.1"],
            vec![pyv("1.0.0"), pyv("0.9.0"), pyv("0.8.1"), pyv("0.8.0")],
            pyv("0.8.1"), //
        )
        .await;
    }
    #[tokio::test]
    async fn test_python_resolution_5() {
        assert_resolution(
            "0.0.1",
            false,
            vec!["!=0.1.0"],
            vec![pyv("2.1.0"), pyv("1.1.0"), pyv("0.1.0")],
            pyv("1.1.0"),
        )
        .await;
    }
    #[tokio::test]
    async fn test_python_resolution_6() {
        assert_resolution(
            "1.1.1",
            false,
            vec![],
            vec![
                pyv("3.0.1"),
                pyv("3.0.0"),
                pyv("2.2.2"),
                pyv("2.2.1"),
                pyv("2.2.0"),
            ],
            pyv("2.2"),
        )
        .await;
    }
    #[tokio::test]
    async fn test_python_resolution_7() {
        assert_resolution(
            "2.2.1",
            true,
            vec![],
            vec![
                pyv("3.0.1"),
                pyv("3.0.0"),
                pyv("2.2.2"),
                pyv("2.2.1"),
                pyv("2.2.0"),
            ],
            pyv("3.0.1"),
        )
        .await;
    }
    #[tokio::test]
    async fn test_python_resolution_8() {
        assert_resolution(
            "2.2",
            false,
            vec![">2.2", ">=2.4", "<2.4.1"],
            vec![
                pyv("2.4.1"),
                pyv("2.4.0"),
                pyv("2.3.1"),
                pyv("2.3.0"),
                pyv("2.2.1"),
                pyv("2.2.0"),
                pyv("2.1.1"),
                pyv("2.1.0"),
            ],
            pyv("2.4.0"),
        )
        .await;
    }
    #[tokio::test]
    async fn test_python_resolution_9() {
        assert_resolution(
            "2.2",
            true,
            vec![">2.2", ">2.3", "<2.4.1"],
            vec![
                pyv("2.4.1"),
                pyv("2.4.0"),
                pyv("2.3.1"),
                pyv("2.3.0"),
                pyv("2.2.1"),
                pyv("2.2.0"),
                pyv("2.1.1"),
                pyv("2.1.0"),
            ],
            pyv("2.4.0"),
        )
        .await;
    }
    #[tokio::test]
    async fn test_python_resolution_10() {
        assert_resolution(
            "2.2",
            false,
            vec![">2.2", ">=2.4"],
            vec![
                pyv("2.4.1"),
                pyv("2.4.0"),
                pyv("2.3.1"),
                pyv("2.3.0"),
                pyv("2.2.1"),
                pyv("2.2.0"),
                pyv("2.1.1"),
                pyv("2.1.0"),
            ],
            // vec![pyv("2.4.1"), pyv("2.3"), pyv("2.2"), pyv("2.1")],
            pyv("2.4"),
        )
        .await;
    }
    #[tokio::test]
    async fn test_python_resolution_11() {
        assert_resolution(
            "2.4.1",
            false,
            vec![">2.2", ">=2.3", "<2.4"],
            vec![
                pyv("2.4.1"),
                pyv("2.4.0"),
                pyv("2.3.1"),
                pyv("2.3.0"),
                pyv("2.2.1"),
                pyv("2.2.0"),
                pyv("2.1.1"),
                pyv("2.1.0"),
            ],
            // vec![pyv("2.4.1"), pyv("2.3"), pyv("2.2"), pyv("2.1")],
            pyv("2.3"),
        )
        .await;
    }
    #[tokio::test]
    async fn test_python_resolution_12() {
        assert_resolution(
            "2.3",
            false,
            vec![">2.2", ">=2.3", "<2.4"],
            vec![
                pyv("2.4.1"),
                pyv("2.4.0"),
                pyv("2.3.1"),
                pyv("2.3.0"),
                pyv("2.2.1"),
                pyv("2.2.0"),
                pyv("2.1.1"),
                pyv("2.1.0"),
            ],
            // vec![pyv("2.4.1"), pyv("2.3"), pyv("2.2"), pyv("2.1")],
            pyv("2.3"),
        )
        .await;
    }
    #[tokio::test]
    async fn test_python_resolution_13() {
        assert_resolution(
            "2.3.1",
            false,
            vec![">2.2", ">=2.3", "<2.4"],
            vec![
                pyv("2.4.1"),
                pyv("2.4.0"),
                pyv("2.3.1"),
                pyv("2.3.0"),
                pyv("2.2.1"),
                pyv("2.2.0"),
                pyv("2.1.1"),
                pyv("2.1.0"),
            ],
            // vec![pyv("2.4.1"), pyv("2.3"), pyv("2.2"), pyv("2.1")],
            pyv("2.3.1"),
        )
        .await;
    }
    #[tokio::test]
    async fn test_python_resolution_14() {
        assert_resolution(
            "2.3.0",
            false,
            vec![],
            vec![pyv("2.4.1"), pyv("2.4.0"), pyv("2.3.1")],
            pyv("2.3.1"),
        )
        .await;
    }

    #[tokio::test]
    async fn test_python_resolution_16() {
        assert_resolution(
            "2.3",
            false,
            vec![],
            vec![pyv("2.4.1"), pyv("2.4.0"), pyv("2.3.1")],
            pyv("2.3"),
        )
        .await;
    }
}
