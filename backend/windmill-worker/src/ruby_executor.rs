use std::{collections::HashMap, process::Stdio};

use anyhow::anyhow;
use const_format::concatcp;
use itertools::Itertools;
use regex::Regex;
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};
use url::Url;
use uuid::Uuid;
use windmill_common::{
    client::AuthedClient,
    error::Error,
    utils::calculate_hash,
    worker::{write_file, Connection, RubyAnnotations},
};
use windmill_parser::Arg;
use windmill_parser_ruby::parse_ruby_signature;
use windmill_queue::{append_logs, CanceledBy, MiniPulledJob};

use crate::{
    common::{
        build_command_with_isolation, create_args_and_out_file, get_reserved_variables,
        read_result, start_child_process, OccupancyMetrics, DEV_CONF_NSJAIL,
    },
    get_proxy_envs_for_lang,
    handle_child::{self},
    universal_pkg_installer::{par_install_language_dependencies_seq, RequiredDependency},
    is_sandboxing_enabled, DISABLE_NUSER, NSJAIL_PATH, PATH_ENV, PROXY_ENVS, RUBY_CACHE_DIR, RUBY_REPOS,
    TRACING_PROXY_CA_CERT_PATH,
};
use windmill_common::scripts::ScriptLang;
lazy_static::lazy_static! {
    static ref RUBY_CONCURRENT_DOWNLOADS: usize = std::env::var("RUBY_CONCURRENT_DOWNLOADS").ok().map(|flag| flag.parse().unwrap_or(20)).unwrap_or(20);
    static ref RUBY_PATH: String = std::env::var("RUBY_PATH").unwrap_or_else(|_| "/usr/bin/ruby".to_string());
    static ref BUNDLE_PATH: String = std::env::var("RUBY_BUNDLE_PATH").unwrap_or_else(|_| "/usr/bin/bundle".to_string());
    static ref GEM_PATH: String = std::env::var("RUBY_GEM_PATH").unwrap_or_else(|_| "/usr/bin/gem".to_string());
    static ref RUBY_PROXY_ENVS: Vec<(String, String)> = {
        PROXY_ENVS
            .clone()
            .into_iter()
            .map(|(k, v)| (k.to_lowercase(), v))
            .collect()
    };
}

const NSJAIL_CONFIG_RUN_RUBY_CONTENT: &str = include_str!("../nsjail/run.ruby.config.proto");
const NSJAIL_CONFIG_DOWNLOAD_RUBY_CONTENT: &str =
    include_str!("../nsjail/download.ruby.config.proto");
const NSJAIL_CONFIG_LOCK_RUBY_CONTENT: &str = include_str!("../nsjail/lock.ruby.config.proto");

#[allow(dead_code)]
pub(crate) struct JobHandlerInput<'a> {
    pub base_internal_url: &'a str,
    pub canceled_by: &'a mut Option<CanceledBy>,
    pub client: &'a AuthedClient,
    pub parent_runnable_path: Option<String>,
    pub conn: &'a Connection,
    pub envs: HashMap<String, String>,
    pub inner_content: &'a str,
    pub job: &'a MiniPulledJob,
    pub job_dir: &'a str,
    pub mem_peak: &'a mut i32,
    pub occupancy_metrics: &'a mut OccupancyMetrics,
    pub requirements_o: Option<&'a String>,
    pub shared_mount: &'a str,
    pub worker_name: &'a str,
}

pub async fn handle_ruby_job<'a>(
    mut args: JobHandlerInput<'a>,
) -> Result<Box<sqlx::types::JsonRawValue>, Error> {
    // --- Prepare ---
    {
        prepare(&args).await?;
    }
    // --- Gen Lockfile ---

    let lockfile = resolve(
        &args.job.id,
        args.inner_content,
        args.mem_peak,
        args.canceled_by,
        args.job_dir,
        args.conn,
        args.worker_name,
        &args.job.workspace_id,
    )
    .await?;

    // --- Install ---

    let ir = install(&mut args, lockfile).await?;

    // --- Execute ---
    {
        run(&mut args, ir).await?;
    }
    // --- Retrieve results ---
    {
        read_result(&args.job_dir, None).await
    }
}
pub async fn prepare<'a>(
    JobHandlerInput {
        job,
        conn,
        job_dir,
        inner_content,
        client,
        //
        ..
    }: &JobHandlerInput<'a>,
) -> Result<(), Error> {
    create_args_and_out_file(&client, job, job_dir, conn).await?;
    File::create(format!("{}/main.rb", job_dir))
        .await?
        .write_all(&wrap(inner_content)?.into_bytes())
        .await?;

    let mini_wm_path = format!("{RUBY_CACHE_DIR}/gems/windmill-internal/windmill");
    if !std::fs::metadata(&mini_wm_path).is_ok() {
        fs::create_dir_all(&mini_wm_path).await?;

        // Create inline.rb for dependency management (bundler/inline compatibility)
        File::create(format!("{}/inline.rb", &mini_wm_path))
            .await?
            .write_all(&wrap(
        r#"
class GemfileProxy
  def initialize
    @gem_calls = []
  end

  def source(arg = nil, &block)
    if arg.is_a?(String) && block_given?
        # Handle the case where the argument is a string and a block is given
        gemfile(&block)
    elsif arg.is_a?(String)
        # Handle the case where the argument is a string and no block is given
        # Do nothing in this case
    else
        raise ArgumentError, "Invalid argument or block"
    end
  end

  def group(*args, **kwargs)
    # Handle group calls if needed
  end

  def ruby(*args, **kwargs)
    warn "Custom Ruby runtime specifications (engine, engine_version, patchlevel) are not supported in this environment. Using system ruby"
  end

  def gem(name, version = nil, require: nil, **kwargs)
      @gem_calls << { name: name, version: version, require: require, kwargs: kwargs }

      case require
      when false
        # Skip requiring
      when nil, true
        begin
          Kernel.require(name)
        rescue LoadError => e
          warn "WARN: Gem '#{name}' failed to auto-require. Try either:"
          warn "    `:require => false` or specify exact require path"
          warn "Warn details: #{e.class}: #{e.message}"
        end
      else
        begin
          Kernel.require(require.to_s)
        rescue LoadError => e
          warn "WARN: Gem '#{name}' failed to auto-require. Try either:"
          warn "    `:require => false` or specify exact require path"
          warn "Warn details: #{e.class}: #{e.message}"
        end
      end
    end


  def call_block(&block)
    instance_eval(&block) if block_given?
  end
end

def gemfile(&block)
  proxy = GemfileProxy.new
  proxy.call_block(&block)
end

def main
end
"#
            )?.into_bytes())
            .await?;

        // Create mini.rb for Windmill client methods
        File::create(format!("{}/mini.rb", &mini_wm_path))
            .await?
            .write_all(
                &wrap(
                    r##"
require 'net/http'
require 'uri'
require 'json'

# Windmill mini client methods
def get_variable(path)
  base_url = ENV['BASE_INTERNAL_URL']
  workspace = ENV['WM_WORKSPACE']
  token = ENV['WM_TOKEN']

  uri = URI("#{base_url}/api/w/#{workspace}/variables/get_value/#{path}")

  http = Net::HTTP.new(uri.host, uri.port)
  http.use_ssl = uri.scheme == 'https'

  request = Net::HTTP::Get.new(uri)
  request['Authorization'] = "Bearer #{token}"

  response = http.request(request)

  if response.code == '200'
    JSON.parse(response.body)
  else
    raise "Failed to get variable #{path}: #{response.code} #{response.message}"
  end
end

def get_resource(path)
  base_url = ENV['BASE_INTERNAL_URL']
  workspace = ENV['WM_WORKSPACE']
  token = ENV['WM_TOKEN']

  uri = URI("#{base_url}/api/w/#{workspace}/resources/get_value_interpolated/#{path}")

  http = Net::HTTP.new(uri.host, uri.port)
  http.use_ssl = uri.scheme == 'https'

  request = Net::HTTP::Get.new(uri)
  request['Authorization'] = "Bearer #{token}"

  response = http.request(request)

  if response.code == '200'
    JSON.parse(response.body)
  else
    raise "Failed to get resource #{path}: #{response.code} #{response.message}"
  end
end

def main
end
"##,
                )?
                .into_bytes(),
            )
            .await?;
    }
    Ok(())
}

pub async fn resolve<'a>(
    job_id: &Uuid,
    inner_content: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    conn: &Connection,
    worker_name: &str,
    w_id: &str,
) -> Result<String, Error> {
    lazy_static::lazy_static! {
        static ref BUNDLER_RE: Regex = Regex::new(r#"(?m)^\s*require\s*['"]bundler/inline['"]"#).unwrap();
        static ref UNSUPPORTED_SOURCE_RE: Regex = Regex::new(r#"(?m)(?<src>^\s*source\s*['"]\S+://(?<usr>\S+):(?<passwd>\S+)@.*['"]\s*\n)"#).unwrap();
        static ref SOURCES_RE: Regex = Regex::new(r#"\S+://(?<creds>\S+:\S+)@(?<url>\S*)"#).unwrap();
    }

    if BUNDLER_RE.find(&inner_content).is_some() {
        return Err(anyhow!(
            "Detected `require 'bundler/inline'` - please replace with `require 'windmill/inline'`.
Your Gemfile syntax will continue to work as-is."
        )
        .into());
    }

    let gemfile: String = windmill_parser_ruby::parse_ruby_requirements(&inner_content)?
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
        .join("\n");

    if let Some(caps) = UNSUPPORTED_SOURCE_RE.captures(&gemfile) {
        if let (Some(src_m), Some(usr_m), Some(passwd_m)) =
            (caps.name("src"), caps.name("usr"), caps.name("passwd"))
        {
            let cause = src_m
                .as_str()
                .replace(usr_m.as_str(), "REDACTED")
                .replace(passwd_m.as_str(), "REDACTED");

            return Err(anyhow!("!> {cause} - inline source credentials are not supported at the moment. Please set up credentials in instance settings, if you do not have access to it, contact your administrator. If you need this feature please let us know.").into());
        }
    }

    let mut file = File::create(job_dir.to_owned() + "/Gemfile").await?;
    file.write_all(&gemfile.as_bytes()).await?;

    let req_hash = format!("ruby-{}", calculate_hash(&gemfile));
    if let Some(db) = conn.as_sql() {
        if let Some(cached) = sqlx::query_scalar!(
            "SELECT lockfile FROM pip_resolution_cache WHERE hash = $1",
            req_hash
        )
        .fetch_optional(db)
        .await?
        {
            return Ok(cached);
        }
    }
    let lock = {
        append_logs(
            job_id,
            w_id,
            format!("\n--- RESOLVING LOCKFILE ---\n"),
            conn,
        )
        .await;

        let mut cmd = if !cfg!(windows) && is_sandboxing_enabled() {
            let nsjail_proto = format!("{}.lock.config.proto", Uuid::new_v4());
            let _ = write_file(
                &job_dir,
                &nsjail_proto,
                &NSJAIL_CONFIG_LOCK_RUBY_CONTENT
                    .replace("{JOB_DIR}", job_dir)
                    .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                    .replace("{TRACING_PROXY_CA_CERT_PATH}", TRACING_PROXY_CA_CERT_PATH)
                    .replace("#{DEV}", DEV_CONF_NSJAIL), // .replace("{BUILD}", &build_dir),
            )?;
            let mut cmd = Command::new(NSJAIL_PATH.as_str());
            cmd.args(vec!["--config", &nsjail_proto, "--", BUNDLE_PATH.as_str()]);
            cmd
        } else if cfg!(windows) {
            Command::new("bundle.bat")
        } else {
            Command::new(BUNDLE_PATH.as_str())
        };

        cmd.env_clear()
            .current_dir(job_dir.to_owned())
            .envs(vec![
                ("PATH".to_owned(), PATH_ENV.clone()),
                // Make sure there is nothing written to actual home
                // This way we keep everything clean, organized and maintable
                ("HOME".to_owned(), RUBY_CACHE_DIR.to_owned()),
            ])
            .envs(RUBY_PROXY_ENVS.clone());

        for repo in RUBY_REPOS.read().await.clone().unwrap_or_default() {
            if let (Some(url), usr, Some(passwd)) =
                (repo.domain(), repo.username(), repo.password())
            {
                cmd.env(
                    format!(
                        "BUNDLE_{}",
                        url
                            // Align with bundlers' format. Example:
                            // BUNDLE_GEM1__EXAMPLE__COM
                            .replace(".", "__")
                            .to_uppercase()
                            // Remove trailing slashes
                            .replace("/", "")
                    ),
                    // BUNDLE_GEM1__EXAMPLE__COM=admin:123
                    format!("{usr}:{passwd}"),
                );
            }
        }
        cmd.args(&["lock", "--retry", "2"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            cmd.env("SystemRoot", crate::SYSTEM_ROOT.as_str())
                .env("USERPROFILE", crate::USERPROFILE_ENV.as_str())
                .env(
                    "TMP",
                    std::env::var("TMP").unwrap_or_else(|_| String::from("/tmp")),
                );
        }
        let child = start_child_process(cmd, "bundle", false).await?;

        handle_child::handle_child(
            job_id,
            conn,
            mem_peak,
            canceled_by,
            child,
            is_sandboxing_enabled(),
            worker_name,
            w_id,
            "bundle",
            None,
            false,
            &mut None,
            // Some(&mut stdout),
            None,
            None,
        )
        .await?;

        let path_lock = format!("{job_dir}/Gemfile.lock");
        let mut file = File::open(path_lock).await?;
        let mut req_content = "".to_string();
        file.read_to_string(&mut req_content).await?;
        req_content
    };

    if let Some(db) = conn.as_sql() {
        sqlx::query!(
            "INSERT INTO pip_resolution_cache (hash, lockfile, expiration) VALUES ($1, $2, now() + ('3 days')::interval) ON CONFLICT (hash) DO UPDATE SET lockfile = EXCLUDED.lockfile",
            req_hash,
            lock.clone(),
        ).fetch_optional(db).await?;
    }

    append_logs(job_id, w_id, format!("\n{}", &lock), conn).await;
    Ok(lock)
}

struct InstallRes {
    /// "/tmp/windmill/cache/ruby/gems/ab..xz-name-1.0.0/gems/name-1.0.0/lib:/tmp/windmill/..."
    rubylib: String,

    /// ["/tmp/windmill/cache/ruby/gems/ab..xz-name-1.0.0", "/tmp/windmill/..."]
    top_level_paths: Vec<String>,
}

async fn install<'a>(
    JobHandlerInput {
        worker_name,
        job,
        conn,
        job_dir,
        inner_content,
        envs,
        client,
        parent_runnable_path,
        ..
    }: &mut JobHandlerInput<'a>,
    lockfile: String,
) -> Result<InstallRes, Error> {
    #[derive(Debug, Clone)]
    enum CurrentSource {
        GIT { remote: Option<String>, revision: Option<String> },
        GEM { remote: Option<String> },
        INIT,
    }

    #[derive(Debug, Clone)]
    enum FinalSource {
        #[allow(dead_code)]
        GIT {
            remote: String,
            revision: String,
        },
        GEM {
            remote: String,
        },
    }

    #[derive(Clone, Debug)]
    struct CustomPayload {
        source: FinalSource,
        version: String,
        pkg: String,
    }

    let mut deps = vec![];
    let mut last_source = CurrentSource::INIT;
    'lock_lines: for line in lockfile.lines() {
        // Trimed line
        let tl = line.trim_start();
        if tl.starts_with("GIT") {
            last_source = CurrentSource::GIT { remote: None, revision: None };
            continue 'lock_lines;
        } else if tl.starts_with("GEM") {
            last_source = CurrentSource::GEM { remote: None };
            continue 'lock_lines;
        }

        const REMOTE: &str = "remote: ";
        if line.contains(REMOTE) {
            match &mut last_source {
                CurrentSource::GIT { remote, .. } => remote.replace(tl.replace(REMOTE, "")),
                CurrentSource::GEM { remote } => remote.replace(tl.replace(REMOTE, "")),
                CurrentSource::INIT => return Err(Error::InternalErr("BUG on installation stage: detected initial enum variant at the stage it was not supposed to be at. Please report this issue.".to_owned())),
            };
            continue 'lock_lines;
        }

        const REVISION: &str = "revision: ";
        if line.contains(REVISION) {
            if let CurrentSource::GIT { revision, .. } = &mut last_source {
                revision.replace(tl.replace(REVISION, ""));
            }
            continue 'lock_lines;
        }

        // Check on untrimed line
        if !line.starts_with("    ") || line.starts_with("      ") {
            //               "1234"                      "123456"
            // For example:
            //
            // GIT
            //  remote: https://github.com/rails/rails.git
            //  revision: 8cb4f8ceffe375f64052579cd7a580bfdaf36d61
            //  specs:
            //    actioncable (8.1.0.alpha)
            //      actionpack (= 8.1.0.alpha)
            //      activesupport (= 8.1.0.alpha)
            //      nio4r (~> 2.0)
            //      websocket-driver (>= 0.6.1)
            //      zeitwerk (~> 2.6)
            //    actionmailbox (8.1.0.alpha)
            //      actionpack (= 8.1.0.alpha)
            //      activejob (= 8.1.0.alpha)
            //      activerecord (= 8.1.0.alpha)
            //      activestorage (= 8.1.0.alpha)
            //      activesupport (= 8.1.0.alpha)
            //      mail (>= 2.8.0)
            //    actionmailer (8.1.0.alpha)
            //   ...
            //
            // In this snippet we only want actioncable, actionmailbox and actionmailer
            //
            // If it got the hit than that's not it, just skip
            continue 'lock_lines;
        }

        let buf = tl.replace(['(', ')'], "");
        let &[pkg, version, ..] = buf.split_whitespace().collect_vec().as_slice() else {
            return Err(anyhow!("Cannot determine version and package name for: {}", tl).into());
        };

        // Strip platform from version. E.g:
        // 1.18.9-x86_64-darwin -> 1.18.9
        let version = version.split('-').next().unwrap_or(version);

        let custom_payload = CustomPayload {
            version: version.into(),
            pkg: pkg.into(),
            source: match last_source.clone() {
                CurrentSource::GIT { remote, revision } => FinalSource::GIT {
                    remote: remote.unwrap_or_default(),
                    revision: revision.unwrap_or_default(),
                },
                CurrentSource::GEM { remote } => {
                    FinalSource::GEM { remote: remote.unwrap_or_default() }
                },
                CurrentSource::INIT => return Err(Error::InternalErr("BUG on installation stage: detected initial enum variant at the stage it was not supposed to be at. Please report this issue.".to_owned())),
            },
        };

        // Calculate hash based on source
        // Obfuscation is essential because remote can include credentials
        let mut hash = calculate_hash(&format!("{:?}", custom_payload.clone()));

        // Use 32 characters which is 256 bits
        hash.truncate(32);

        // Final format:
        // 123...zx-activesupport-8.0.2
        // ^^^^^^^^ hash based on source and type (GEM or GIT)
        let handle = format!("{}-{}-{}", hash, pkg, version);
        let path = format!("{RUBY_CACHE_DIR}/gems/{}", &handle);

        deps.push(RequiredDependency {
            path,
            _s3_handle: handle,
            display_name: tl.to_owned(),
            custom_payload,
        });
    }

    let job_dir = job_dir.to_owned();
    let jailed = !cfg!(windows) && is_sandboxing_enabled();
    let RubyAnnotations { verbose } = RubyAnnotations::parse(&inner_content);
    let repos = RUBY_REPOS.read().await.clone().unwrap_or_default();
    let (envs, reserved_variables) = (
        envs.clone(),
        get_reserved_variables(job, &client.token, conn, parent_runnable_path.clone()).await?,
    );
    par_install_language_dependencies_seq(
        deps.clone(),
        "ruby",
        "gem",
        false,
        *RUBY_CONCURRENT_DOWNLOADS,
        // true,
        // crate::common::InstallStrategy::Single(Arc::new(move |dependency| {
        move |dependency| {
            let mut cmd = if jailed {
                std::fs::create_dir_all(&dependency.path)?;
                let nsjail_proto = format!("{}.download.config.proto", Uuid::new_v4());
                let _ = write_file(
                    &job_dir,
                    &nsjail_proto,
                    &NSJAIL_CONFIG_DOWNLOAD_RUBY_CONTENT
                        .replace("{TARGET}", &dependency.path)
                        .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                        .replace("{TRACING_PROXY_CA_CERT_PATH}", TRACING_PROXY_CA_CERT_PATH)
                        .replace("#{DEV}", DEV_CONF_NSJAIL), // .replace("{BUILD}", &build_dir),
                )?;
                let mut cmd = Command::new(NSJAIL_PATH.as_str());
                cmd.args(vec!["--config", &nsjail_proto, "--", GEM_PATH.as_str()]);
                cmd
            } else {
                Command::new(if cfg!(windows) {
                    "gem.cmd"
                } else {
                    GEM_PATH.as_str()
                })
            };
            cmd.env_clear()
                .current_dir(&job_dir)
                .envs(vec![
                    ("PATH".to_owned(), PATH_ENV.clone()),
                    // Make sure there is nothing written to actual home
                    // This way we keep everything clean, organized and maintable
                    ("HOME".to_owned(), RUBY_CACHE_DIR.to_owned()),
                ])
                .envs(RUBY_PROXY_ENVS.clone());

            // Figure out source
            let source = {
                let source = match dependency.custom_payload.source {
                    FinalSource::GIT { .. } => return Err(anyhow!("GIT is not supported").into()),
                    FinalSource::GEM { remote } => remote,
                }
                .trim()
                .to_owned();

                let src_url = Url::parse(&source)?;
                repos
                    .iter()
                    .find(|inst_url| {
                        inst_url.domain().is_some()
                            && (inst_url.domain(), inst_url.path())
                                == (src_url.domain(), src_url.path())
                    })
                    .map(Url::to_string)
                    .unwrap_or(source)
            };

            cmd.args(&[
                "install",
                &dependency.custom_payload.pkg,
                "--version",
                &dependency.custom_payload.version,
                "--install-dir",
                &dependency.path,
                // Disable transitive dependencies!
                "--ignore-dependencies",
                "--no-document",
                // Do not use default sources
                "--clear-sources",
                // Specify the one from lockfile
                "--source",
                source.as_str(),
            ])
            .envs(envs.clone())
            .envs(reserved_variables.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

            if !verbose {
                // Make it output minimal info
                cmd.args(&["--quiet", "--silent"]);
            }

            #[cfg(windows)]
            {
                cmd.env("SystemRoot", crate::SYSTEM_ROOT.as_str())
                    .env("USERPROFILE", crate::USERPROFILE_ENV.as_str())
                    .env(
                        "TMP",
                        std::env::var("TMP").unwrap_or_else(|_| String::from("/tmp")),
                    )
                    .env(
                        "SYSTEMDRIVE",
                        std::env::var("SYSTEMDRIVE").unwrap_or_else(|_| String::from("C:")),
                    );
            }

            Ok(cmd)
        },
        // async move |_| Ok(()),
        &job.id,
        &job.workspace_id,
        worker_name,
        jailed,
        conn,
    )
    .await?;

    let mut res = InstallRes {
        rubylib: deps
            .iter()
            .map(|rd| {
                format!(
                    "{}/gems/{}-{}/lib",
                    rd.path, rd.custom_payload.pkg, rd.custom_payload.version
                )
            })
            .join(":"),
        top_level_paths: deps.iter().map(|rd| rd.path.clone()).collect_vec(),
    };
    // Include builtin windmill client
    {
        const WM_INTERNAL: &str = concatcp!(RUBY_CACHE_DIR, "/gems/windmill-internal");
        res.top_level_paths.push(WM_INTERNAL.to_owned());
        res.rubylib += format!(":{WM_INTERNAL}").as_str();
    }
    Ok(res)
}

async fn run<'a>(
    JobHandlerInput {
        occupancy_metrics,
        mem_peak,
        canceled_by,
        worker_name,
        job,
        conn,
        job_dir,
        shared_mount,
        client,
        envs,
        base_internal_url,
        parent_runnable_path,
        ..
    }: &mut JobHandlerInput<'a>,
    InstallRes { rubylib, top_level_paths }: InstallRes,
) -> Result<(), Error> {
    let reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path.clone()).await?;

    let child = if !cfg!(windows) && is_sandboxing_enabled() {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n--- ISOLATED RUBY CODE EXECUTION ---\n"),
            conn,
        )
        .await;
        let shared_deps = top_level_paths
            .into_iter()
            .map(|pp| {
                format!(
                    r#"
mount {{
    src: "{pp}"
    dst: "{pp}"
    is_bind: true
    rw: false
}}
                "#
                )
            })
            .join("\n");

        write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_RUBY_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{SHARED_MOUNT}", &shared_mount)
                .replace("{SHARED_DEPENDENCIES}", &shared_deps)
                .replace("{TRACING_PROXY_CA_CERT_PATH}", TRACING_PROXY_CA_CERT_PATH)
                .replace("#{DEV}", DEV_CONF_NSJAIL)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string()),
        )?;
        let mut cmd = Command::new(NSJAIL_PATH.as_str());
        cmd.env_clear()
            .current_dir(job_dir)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("RUBYLIB", rubylib.as_str())
            .envs(envs)
            .envs(reserved_variables)
            .envs(RUBY_PROXY_ENVS.clone())
            .envs(get_proxy_envs_for_lang(&ScriptLang::Ruby).await?)
            .args(vec![
                "--config",
                "run.config.proto",
                "--",
                RUBY_PATH.as_str(),
                "main.rb",
            ]);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        start_child_process(cmd, NSJAIL_PATH.as_str(), false).await?
    } else {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n--- RUBY CODE EXECUTION ---\n"),
            conn,
        )
        .await;

        let ruby_executable = if cfg!(windows) {
            "ruby.exe"
        } else {
            RUBY_PATH.as_str()
        };

        let args = vec!["main.rb"];
        let mut cmd = build_command_with_isolation(ruby_executable, &args);

        #[cfg(windows)]
        let rubylib = rubylib.replace(":", ";");

        cmd.env_clear()
            .current_dir(job_dir.to_owned())
            .env("PATH", PATH_ENV.as_str())
            .env("RUBYLIB", rubylib.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .envs(reserved_variables)
            .envs(RUBY_PROXY_ENVS.clone())
            .envs(get_proxy_envs_for_lang(&ScriptLang::Ruby).await?)
            .envs(envs);

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            cmd.env("SystemRoot", crate::SYSTEM_ROOT.as_str())
                .env("USERPROFILE", crate::USERPROFILE_ENV.as_str())
                .env(
                    "TMP",
                    std::env::var("TMP").unwrap_or_else(|_| String::from("/tmp")),
                );
        }
        start_child_process(cmd, ruby_executable, false).await?
    };
    handle_child::handle_child(
        &job.id,
        conn,
        mem_peak,
        canceled_by,
        child,
        is_sandboxing_enabled(),
        worker_name,
        &job.workspace_id,
        "ruby",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;
    Ok(())
}
fn wrap(inner_content: &str) -> Result<String, Error> {
    let sig = parse_ruby_signature(inner_content)?;
    let spread = sig
        .args
        .clone()
        .into_iter()
        .map(|Arg { name, .. }| format!("a[\"{}\"]", name))
        .collect_vec()
        .join(",");
    Ok(
        r#"INNER_CONTENT

require 'json'
a = JSON.parse(File.read("args.json"))

begin
    res = main(SPREAD)
    File.open("result.json", "w") do |file|
      file.write(JSON.generate(res))
    end

rescue => e
    error = {
        name: e.class.name,
        stack: e.full_message,
        message: e.message
    }
    File.open("result.json", "w") do |file|
      file.write(JSON.generate(error))
    end
    raise
end
            "#
        .replace("INNER_CONTENT", inner_content)
        .replace("SPREAD", &spread), // .replace("TRANSFORM", transform)
    )
}
