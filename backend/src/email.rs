use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
    Tokio1Executor,
};

use crate::error::{Error, Result};

pub struct EmailSender {
    pub from: String,
    pub server: String,
    pub password: String,
}

impl EmailSender {
    pub async fn send_email(&self, email: Message) -> Result<()> {
        let creds = Credentials::new(self.from.to_string(), self.password.to_string());

        // Open a remote connection to gmail
        let mailer: AsyncSmtpTransport<Tokio1Executor> =
            AsyncSmtpTransport::<Tokio1Executor>::relay(&self.server)
                .unwrap()
                .credentials(creds)
                .build();

        mailer
            .send(email)
            .await
            .map_err(|x| Error::InternalErr(format!("Impossible to send email {x}")))?;
        Ok(())
    }
}
