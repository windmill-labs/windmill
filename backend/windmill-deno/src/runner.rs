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
    ) -> Result<i32, anyhow::Error> {
        let runner = self.0.get().await.map_err(|e| anyhow!(e))?;

        runner.run_job(args, job_dir, cache_dir).await
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

#[derive(Debug)]
struct DenoJob {
    notification: tokio::sync::oneshot::Sender<Result<i32, anyhow::Error>>,
    args: Vec<String>,
    job_dir: String,
    cache_dir: String,
}

#[derive(Debug)]
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
                        let res = crate::run_deno_cli(job.args, &job.job_dir, &job.cache_dir).await;
                        job.notification.send(res).unwrap()
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

    async fn run_job(
        &self,
        args: Vec<String>,
        job_dir: String,
        cache_dir: String,
    ) -> Result<i32, anyhow::Error> {
        let (sender, receiver) = tokio::sync::oneshot::channel();

        self.job_sender
            .send(Message::Job(DenoJob {
                notification: sender,
                args,
                job_dir,
                cache_dir,
            }))
            .unwrap();

        receiver.await.unwrap()
    }

    async fn recycle(&self) {
        self.job_sender.send(Message::Recycle).unwrap();
    }

    fn is_up(&self) -> bool {
        !self.handle.is_finished()
    }
}
