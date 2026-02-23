use std::collections::HashMap;

use serde_json::value::RawValue;

use crate::{
    create_nativets_runtime, execute_main, load_client_module, load_user_module, CreatedRuntime,
    ExecuteError, MainArgs, NativeAnnotation,
};

pub struct PrewarmedResult {
    pub result: Result<Box<RawValue>, String>,
    pub logs: String,
}

pub struct ExecutingIsolate {
    result_rx: tokio::sync::oneshot::Receiver<PrewarmedResult>,
    handle: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl ExecutingIsolate {
    pub async fn wait(self) -> anyhow::Result<PrewarmedResult> {
        let result = self
            .result_rx
            .await
            .map_err(|_| anyhow::anyhow!("isolate result channel closed"))?;
        self.handle
            .await
            .map_err(|e| anyhow::anyhow!("isolate thread panicked: {e}"))??;
        Ok(result)
    }
}

pub struct PrewarmedIsolate {
    args_tx: Option<tokio::sync::oneshot::Sender<String>>,
    result_rx: Option<tokio::sync::oneshot::Receiver<PrewarmedResult>>,
    ready_rx: Option<tokio::sync::oneshot::Receiver<()>>,
    handle: Option<tokio::task::JoinHandle<anyhow::Result<()>>>,
}

/// Parse a JSON args object and reorder into positional args matching `arg_names`.
fn args_to_positional(args_json: &str, arg_names: &[String]) -> Vec<Option<Box<RawValue>>> {
    let map: HashMap<String, Box<RawValue>> = serde_json::from_str(args_json).unwrap_or_default();
    arg_names
        .iter()
        .map(|name| map.get(name).cloned())
        .collect()
}

impl PrewarmedIsolate {
    /// Spawn a new isolate on a blocking thread.
    ///
    /// The isolate loads `env_code` + WINDMILL_CLIENT as `windmill.ts`,
    /// then loads `js_code` as `eval.ts`, and waits for args to execute.
    pub fn spawn(
        env_code: String,
        js_code: String,
        ann: NativeAnnotation,
        arg_names: Vec<String>,
    ) -> Self {
        let (args_tx, args_rx) = tokio::sync::oneshot::channel::<String>();
        let (result_tx, result_rx) = tokio::sync::oneshot::channel::<PrewarmedResult>();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<()>();

        let handle = tokio::task::spawn_blocking(move || {
            let CreatedRuntime { mut js_runtime, log_receiver, mut memory_limit_rx } =
                create_nativets_runtime(ann, vec![])?;

            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;

            runtime.block_on(async {
                load_client_module(&mut js_runtime, &env_code).await?;
                load_user_module(&mut js_runtime, format!("{env_code}\n{js_code}")).await?;

                let _ = ready_tx.send(());

                let args_json = match args_rx.await {
                    Ok(a) => a,
                    Err(_) => return Ok(()),
                };

                let positional = args_to_positional(&args_json, &arg_names);
                {
                    let op_state = js_runtime.op_state();
                    let mut op_state = op_state.borrow_mut();
                    op_state.put(MainArgs { args: positional });
                }

                let log_handle = tokio::spawn(async move {
                    let mut log_receiver = log_receiver;
                    let mut logs = String::new();
                    while let Some(log) = log_receiver.recv().await {
                        logs.push_str(&log);
                        logs.push('\n');
                    }
                    logs
                });

                let exec_result = tokio::select! {
                    r = execute_main(&mut js_runtime, None, false, None) => r,
                    _ = memory_limit_rx.recv() => {
                        Err(ExecuteError::Script("Memory limit reached, killing isolate".to_string()))
                    }
                };

                let result = match exec_result {
                    Ok(raw) => Ok(raw),
                    Err(ExecuteError::Script(msg)) => Err(msg),
                    Err(ExecuteError::Js { message, stack, .. }) => {
                        let msg = message.unwrap_or_default();
                        let err = match stack {
                            Some(s) => format!("{msg}\n{s}"),
                            None => msg,
                        };
                        Err(err)
                    }
                };

                drop(js_runtime);
                let logs = log_handle.await.unwrap_or_default();
                let _ = result_tx.send(PrewarmedResult { result, logs });
                Ok(())
            })
        });

        PrewarmedIsolate {
            args_tx: Some(args_tx),
            result_rx: Some(result_rx),
            ready_rx: Some(ready_rx),
            handle: Some(handle),
        }
    }

    /// Wait for the isolate to finish loading modules.
    pub async fn wait_ready(&mut self) -> anyhow::Result<()> {
        if let Some(rx) = self.ready_rx.take() {
            rx.await
                .map_err(|_| anyhow::anyhow!("isolate failed during pre-warm"))?;
        }
        Ok(())
    }

    /// Send args and start execution. Returns an `ExecutingIsolate` that
    /// can be awaited independently, allowing the caller to pre-warm
    /// the next isolate in parallel.
    pub fn start_execution(mut self, args: String) -> ExecutingIsolate {
        let args_tx = self.args_tx.take().expect("start_execution called twice");
        let _ = args_tx.send(args);
        ExecutingIsolate {
            result_rx: self.result_rx.take().expect("result_rx missing"),
            handle: self.handle.take().expect("handle missing"),
        }
    }
}
