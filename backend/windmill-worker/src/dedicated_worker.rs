use std::collections::HashMap;

use crate::{
    common::{handle_child, read_result, write_file},
    AuthedClientBackgroundTask,
};
use windmill_common::{error, error::Result};

pub fn create_dedicated_worker() {}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn start_worker(
    requirements_o: Option<String>,
    logs: &mut String,
    code: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    job_dir: &str,
    inner_content: &String,
    base_internal_url: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
    shared_mount: &str,
) -> error::Result<serde_json::Value> {
    let _ = write_file(job_dir, "main.ts", inner_content).await?;
    let common_bun_proc_envs: HashMap<String, String> =
        get_common_bun_proc_envs(&base_internal_url);

    if let Some(reqs) = requirements_o {
        let splitted = reqs.split(BUN_LOCKB_SPLIT).collect::<Vec<&str>>();
        if splitted.len() != 2 {
            return Err(error::Error::ExecutionErr(
                format!("Invalid requirements, expectd to find //bun.lockb split pattern in reqs. Found: |{reqs}|")
            ));
        }
        let _ = write_file(job_dir, "package.json", &splitted[0]).await?;
        let lockb = splitted[1];
        if lockb != EMPTY_FILE {
            let _ = write_file_binary(
                job_dir,
                "bun.lockb",
                &base64::engine::general_purpose::STANDARD
                    .decode(&splitted[1])
                    .map_err(|_| {
                        error::Error::InternalErr("Could not decode bun.lockb".to_string())
                    })?,
            )
            .await?;
        }
        install_lockfile(
            logs,
            &job.id,
            &job.workspace_id,
            db,
            job_dir,
            worker_name,
            common_bun_proc_envs.clone(),
        )
        .await?;
    } else if !*DISABLE_NSJAIL {
        logs.push_str("\n\n--- BUN INSTALL ---\n");
        let _ = gen_lockfile(
            logs,
            &job.id,
            &job.workspace_id,
            db,
            &client.get_token().await,
            &job.script_path(),
            job_dir,
            base_internal_url,
            worker_name,
            false,
        )
        .await?;
    }

    let write_wrapper_f = async {
        // let mut start = Instant::now();
        let args = windmill_parser_ts::parse_deno_signature(inner_content, true)?.args;
        let dates = args
            .iter()
            .enumerate()
            .filter_map(|(i, x)| {
                if matches!(x.typ, Typ::Datetime) {
                    Some(i)
                } else {
                    None
                }
            })
            .map(|x| return format!("args[{x}] = args[{x}] ? new Date(args[{x}]) : undefined"))
            .join("\n");

        let spread = args.into_iter().map(|x| x.name).join(",");
        // logs.push_str(format!("infer args: {:?}\n", start.elapsed().as_micros()).as_str());
        // we cannot use Bun.read and Bun.write because it results in an EBADF error on cloud
        let wrapper_content: String = format!(
            r#"
import {{ main }} from "./main.ts";

const fs = require('fs/promises');

const args = await fs.readFile('args.json', {{ encoding: 'utf8' }}).then(JSON.parse)
    .then(({{ {spread} }}) => [ {spread} ])

BigInt.prototype.toJSON = function () {{
    return this.toString();
}};

{dates}
async function run() {{
    let res: any = await main(...args);
    const res_json = JSON.stringify(res ?? null, (key, value) => typeof value === 'undefined' ? null : value);
    await fs.writeFile("result.json", res_json);
    process.exit(0);
}}
run().catch(async (e) => {{
    await fs.writeFile("result.json", JSON.stringify({{ message: e.message, name: e.name, stack: e.stack }}));
    process.exit(1);
}});
    "#,
        );
        write_file(job_dir, "wrapper.ts", &wrapper_content).await?;
        Ok(()) as error::Result<()>
    };

    let reserved_variables_args_out_f = async {
        let client = client.get_authed().await;
        let args_and_out_f = async {
            create_args_and_out_file(&client, job, job_dir).await?;
            Ok(()) as Result<()>
        };
        let reserved_variables_f = async {
            let vars = get_reserved_variables(job, &client.token, db).await?;
            Ok(vars) as Result<HashMap<String, String>>
        };
        let (_, reserved_variables) = tokio::try_join!(args_and_out_f, reserved_variables_f)?;
        Ok(reserved_variables) as error::Result<HashMap<String, String>>
    };

    let (reserved_variables, _) = tokio::try_join!(reserved_variables_args_out_f, write_wrapper_f)?;

    let _ = write_file(
        &job_dir,
        "loader.bun.ts",
        &format!(
            r#"
import {{ plugin }} from "bun";

{}

plugin(p)
"#,
            RELATIVE_BUN_LOADER
                .replace("W_ID", &job.workspace_id)
                .replace("BASE_INTERNAL_URL", base_internal_url)
                .replace("TOKEN", &client.get_token().await)
                .replace("CURRENT_PATH", job.script_path())
        ),
    )
    .await?;

    //do not cache local dependencies
    let child = if !*DISABLE_NSJAIL {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_BUN_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CACHE_DIR}", BUN_CACHE_DIR)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount),
        )
        .await?;

        Command::new(NSJAIL_PATH.as_str())
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .envs(common_bun_proc_envs)
            .env("PATH", PATH_ENV.as_str())
            .args(vec![
                "--config",
                "run.config.proto",
                "--",
                &BUN_PATH,
                "run",
                "-i",
                "--prefer-offline",
                "-r",
                "/tmp/bun/loader.bun.ts",
                "/tmp/bun/wrapper.ts",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        let script_path = format!("{job_dir}/wrapper.ts");
        let args = vec![
            "run",
            "-i",
            "--prefer-offline",
            "-r",
            "./loader.bun.ts",
            &script_path,
        ];
        Command::new(&*BUN_PATH)
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .envs(common_bun_proc_envs)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };

    // logs.push_str(format!("prepare: {:?}\n", start.elapsed().as_micros()).as_str());
    // start = Instant::now();
    handle_child(
        &job.id,
        db,
        logs,
        child,
        false,
        worker_name,
        &job.workspace_id,
        "bun run",
        job.timeout,
    )
    .await?;
    // logs.push_str(format!("execute: {:?}\n", start.elapsed().as_millis()).as_str());
    read_result(job_dir).await
}
