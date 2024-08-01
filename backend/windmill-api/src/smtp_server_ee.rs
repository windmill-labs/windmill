pub struct SmtpServer;

impl SmtpServer {
    pub async fn start_listener_thread(
        &mut self,
        _base_internal_url: String,
    ) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("Implementation not open source"))
    }
}
