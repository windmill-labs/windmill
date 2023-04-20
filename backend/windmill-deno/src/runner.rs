use anyhow::anyhow;

pub struct RunnerManager;

#[async_trait::async_trait]
impl deadpool::managed::Manager for RunnerManager {
    type Type = DenoRunner;
    type Error = anyhow::Error;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        Ok(DenoRunner::new())
    }

    async fn recycle(
        &self,
        runner: &mut Self::Type,
    ) -> deadpool::managed::RecycleResult<Self::Error> {
        if !runner.is_up() {
            return Err(deadpool::managed::RecycleError::StaticMessage(
                "Runner is down",
            ));
        }

        runner.recycle().await;

        if !runner.is_up() {
            return Err(deadpool::managed::RecycleError::StaticMessage(
                "Runner is down",
            ));
        }

        Ok(())
    }
}

pub struct DenoRunnerPool(deadpool::managed::Pool<RunnerManager>);

impl DenoRunnerPool {
    pub async fn run_job(
        &self,
        args: Vec<String>,
        job_dir: String,
        cache_dir: String,
        stdio: deno_runtime::deno_io::Stdio,
    ) -> Result<
        (
            tokio::sync::oneshot::Receiver<Result<i32, anyhow::Error>>,
            tokio::sync::oneshot::Sender<()>,
        ),
        anyhow::Error,
    > {
        let runner = self.0.get().await.map_err(|e| anyhow!(e))?;

        Ok(runner.run_job(args, job_dir, cache_dir, stdio))
    }

    pub fn new() -> Self {
        let pool = deadpool::managed::Pool::builder(RunnerManager)
            .build()
            .unwrap();

        DenoRunnerPool(pool)
    }
}

pub struct DenoRunner {
    job_sender: tokio::sync::mpsc::UnboundedSender<Message>,
    handle: std::thread::JoinHandle<()>,
}

struct DenoJob {
    completion_sender: tokio::sync::oneshot::Sender<Result<i32, anyhow::Error>>,
    cancellation_receiver: tokio::sync::oneshot::Receiver<()>,
    args: Vec<String>,
    job_dir: String,
    cache_dir: String,
    stdio: deno_runtime::deno_io::Stdio,
}

enum Message {
    Job(DenoJob),
    Recycle,
}

impl DenoRunner {
    fn new() -> Self {
        let (job_sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let handle = tokio::runtime::Handle::current();
        let handle = std::thread::Builder::new()
            .name("Deno Worker".to_owned())
            .spawn(move || {
                let mut receiver: tokio::sync::mpsc::UnboundedReceiver<Message> = receiver;
                let handle = handle;

                loop {
                    let Some(Message::Job(job)) = receiver.blocking_recv() else {
                        break;
                    };
                    // Run deno job
                    handle.block_on(async move {
                        async fn inner_run(
                            mut cancellation_receiver: tokio::sync::oneshot::Receiver<()>,
                            args: Vec<String>,
                            job_dir: String,
                            cache_dir: String,
                            stdio: deno_runtime::deno_io::Stdio,
                        ) -> anyhow::Result<i32> {
                            let (main_module, mut worker, ps) =
                                crate::prepare_run(args, &job_dir, &cache_dir, stdio).await?;

                            if let Ok(()) = cancellation_receiver.try_recv() {
                                return Err(anyhow::anyhow!("Job has been cancelled!"));
                            }

                            crate::pre_run(&main_module, &mut worker, &ps).await?;

                            loop {
                                if let Ok(()) = cancellation_receiver.try_recv() {
                                    return Err(anyhow::anyhow!("Job has been cancelled!"));
                                }
                                if !crate::run_once(&mut worker).await? {
                                    break;
                                }
                            }

                            // At this point, cancelling would be stupid. Just finish the job
                            let res = crate::post_run(&mut worker).await?;

                            Ok(res)
                        }
                        job.completion_sender
                            .send(
                                inner_run(
                                    job.cancellation_receiver,
                                    job.args,
                                    job.job_dir,
                                    job.cache_dir,
                                    job.stdio,
                                )
                                .await,
                            )
                            .unwrap()
                    });
                    let Some(Message::Recycle) = receiver.blocking_recv() else {
                        break;
                    };
                    // TODO: Do recylcing
                }
            })
            .unwrap();

        Self { job_sender, handle }
    }

    fn run_job(
        &self,
        args: Vec<String>,
        job_dir: String,
        cache_dir: String,
        stdio: deno_runtime::deno_io::Stdio,
    ) -> (
        tokio::sync::oneshot::Receiver<Result<i32, anyhow::Error>>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        let (completion_sender, completion_receiver) = tokio::sync::oneshot::channel();
        let (cancellation_sender, cancellation_receiver) = tokio::sync::oneshot::channel();

        self.job_sender
            .send(Message::Job(DenoJob {
                completion_sender,
                cancellation_receiver,
                args,
                job_dir,
                cache_dir,
                stdio,
            }))
            .map_err(|_| ())
            .unwrap();

        (completion_receiver, cancellation_sender)
    }

    async fn recycle(&self) {
        self.job_sender
            .send(Message::Recycle)
            .map_err(|_| ())
            .unwrap();
    }

    fn is_up(&self) -> bool {
        !self.handle.is_finished()
    }
}
