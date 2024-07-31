use crate::HTTP_CLIENT;
use futures::{stream::iter, SinkExt, StreamExt};
use mail_parser::MessageParser;
use native_tls::{Identity, TlsAcceptor};
use openssl::asn1::Asn1Time;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::x509::extension::AuthorityKeyIdentifier;
use openssl::x509::extension::BasicConstraints;
use openssl::x509::extension::SubjectKeyIdentifier;
use openssl::x509::X509NameBuilder;
use openssl::x509::X509;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio_native_tls::{TlsAcceptor as TokioTlsAcceptor, TlsStream};
use tokio_util::codec::{Framed, LinesCodec, LinesCodecError};

lazy_static::lazy_static! {
    static ref RE_SMTP_MAIL: regex::Regex = regex::Regex::new(r"(?i)from: ?<(.+)>").unwrap();
    static ref RE_SMTP_RCPT: regex::Regex = regex::Regex::new(r"(?i)to: ?<(.+)>").unwrap();
    static ref RE_EMAIL_PARTS: regex::Regex = regex::Regex::new(r"(?i)^([a-zA-Z0-9_\-]+)\+(?:flow\.([a-zA-Z0-9_\-\.]+)|hash\.([a-zA-Z0-9]+)|([a-zA-Z0-9_\-\.]+))\+([a-zA-Z0-9]+)@").unwrap();

    static ref cert: anyhow::Result<(
        Vec<u8>,
        Vec<u8>
    )> = {
        let rsa = Rsa::generate(4096)?;
        let pkey = PKey::from_rsa(rsa)?;
        let mut name = X509NameBuilder::new()?;
        name.append_entry_by_text("CN", "localhost")?;
        let name = name.build();
        let mut builder = X509::builder()?;
        builder.set_version(2)?;
        builder.set_subject_name(&name)?;
        builder.set_issuer_name(&name)?;
        builder.set_pubkey(&pkey)?;
        let now = Asn1Time::days_from_now(0)?;
        let later = Asn1Time::days_from_now(3650)?;
        builder.set_not_before(now.as_ref())?;
        builder.set_not_after(later.as_ref())?;
        builder.append_extension(BasicConstraints::new().critical().ca().build()?)?;
        builder.append_extension(SubjectKeyIdentifier::new().build(&builder.x509v3_context(None, None))?)?;
        builder.append_extension(AuthorityKeyIdentifier::new().keyid(true).issuer(true).build(&builder.x509v3_context(None, None))?)?;
        builder.sign(&pkey, openssl::hash::MessageDigest::sha256())?;
        let c = builder.build();
        Ok((c.to_pem()?, pkey.private_key_to_pem_pkcs8()?))
    };
}

pub struct SmtpServer;

impl SmtpServer {
    pub async fn start_listener_thread(&mut self, base_internal_url: String) -> anyhow::Result<()> {
        println!("Try to listen to connection");
        let listener = TcpListener::bind("0.0.0.0:2525").await?;
        println!("Connection Opened");

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let mut conn = SmtpConnection::new(base_internal_url.clone());
                        tokio::spawn(async move {
                            if let Err(err) = conn.handle(stream).await {
                                tracing::error!("Error handling SMTP connection: {:?}", err);
                            };
                        });
                    }
                    Err(err) => {
                        tracing::error!("Error establishing SMTP connection: {:?}", err);
                    }
                }
            }
        });

        Ok(())
    }
}

pub struct SmtpConnection {
    base_internal_url: String,
    mailfrom: Option<String>,
    rcpts: Option<Vec<String>>,
    message: String,
    state: SmtpState,
}

#[derive(PartialEq, Debug)]
enum SmtpState {
    Command,
    Data,
    Quit,
}

impl SmtpConnection {
    pub fn new(base_internal_url: String) -> SmtpConnection {
        SmtpConnection {
            base_internal_url,
            mailfrom: None,
            rcpts: None,
            message: String::new(),
            state: SmtpState::Command,
        }
    }

    pub async fn handle(&mut self, stream: TcpStream) -> anyhow::Result<()> {
        self.handle_unsecured_session(stream).await
    }

    async fn handle_unsecured_session(&mut self, mut stream: TcpStream) -> anyhow::Result<()> {
        let (reader, writer) = stream.split();
        let mut reader = BufReader::new(reader);
        let mut writer = BufWriter::new(writer);
        writer.write_all(b"220 Windmill SMTP Receiver\r\n").await?;
        writer.flush().await?;

        let mut is_tls = false;

        let mut line = String::new();

        while reader.read_line(&mut line).await? != 0 {
            tracing::info!("Received unsecure: {}", line);

            let space_pos = line.find(" ").unwrap_or(line.len());
            let (command, _) = line.split_at(space_pos);

            match command.trim().to_uppercase().as_ref() {
                "EHLO" | "HELO" => {
                    writer.write_all(b"250-windmill Hello\r\n").await?;
                    writer.write_all(b"250-STARTTLS\r\n").await?;
                    writer.write_all(b"250 What you've got?\r\n").await?;
                    writer.flush().await?;
                }
                "STARTTLS" => {
                    writer.write_all(b"220 GO ON\r\n").await?;
                    writer.flush().await?;

                    tracing::info!("Starting TLS");
                    is_tls = true;
                    break;
                }
                "QUIT" => {
                    writer.write_all(b"221 Have a nice day!\r\n").await?;
                    writer.flush().await?;
                    break;
                }
                "NOOP" => {
                    writer.write_all(b"250 OK\r\n").await?;
                    writer.flush().await?;
                }
                "MAIL" | "RCPT" | "DATA" | "RSET" => {
                    writer
                        .write_all(b"530 Must issue a STARTTLS command first\r\n")
                        .await?;
                    writer.flush().await?;
                }
                _ => {
                    writer.write_all(b"500 Unknown command\r\n").await?;
                    writer.flush().await?;
                }
            }

            line.clear();
        }

        if is_tls {
            self.handle_starttls(stream).await?;
        }

        Ok(())
    }

    async fn handle_starttls(&mut self, stream: TcpStream) -> anyhow::Result<()> {
        let c = cert
            .as_ref()
            .map_err(|e| anyhow::anyhow!("Could not generate self-signed certificates: {}", e))?;
        let identity = Identity::from_pkcs8(&c.0, &c.1)?;
        let tls_acceptor = TlsAcceptor::builder(identity).build()?;
        let tls_acceptor = TokioTlsAcceptor::from(tls_acceptor);

        match tls_acceptor.accept(stream).await {
            Ok(stream) => {
                self.handle_secured_session(stream).await?;
            }
            Err(e) => {
                println!("Error establishing TLS connection: {:?}", e);
            }
        };

        Ok(())
    }

    async fn send_commands(
        framed: &mut Framed<TlsStream<TcpStream>, LinesCodec>,
        commands: Vec<String>,
    ) -> Result<(), LinesCodecError> {
        // only need to add \r because the codec adds \n
        let messages = iter(commands.into_iter().map(|x| format!("{}\r", x)));
        framed.send_all(&mut messages.map(Ok)).await?;
        Ok(())
    }

    pub async fn handle_secured_session(
        &mut self,
        stream: TlsStream<TcpStream>,
    ) -> anyhow::Result<()> {
        let mut framed = Framed::new(stream, LinesCodec::new());

        while let Some(line_str) = framed.next().await {
            let line = line_str?;
            tracing::info!("Received through TLS: {}", line);
            match self.state {
                SmtpState::Command => {
                    let space_pos = line.find(" ").unwrap_or(line.len());
                    let (command, arg) = line.split_at(space_pos);
                    let arg = arg.trim();
                    match &*command.trim().to_uppercase() {
                        "HELO" | "EHLO" => {
                            Self::send_commands(&mut framed, vec!["250 Hello again".to_string()])
                                .await?;
                        }
                        "MAIL" => {
                            // Syntax MAIL From: <user@example.com>
                            if let Some(address) = RE_SMTP_MAIL
                                .captures(arg)
                                .and_then(|cap| cap.get(1))
                                .map(|m| m.as_str())
                            {
                                self.mailfrom = Some(address.to_string());
                                Self::send_commands(&mut framed, vec!["250 OK".to_string()])
                                    .await?;
                            } else {
                                Self::send_commands(
                                    &mut framed,
                                    vec!["501 Syntax: MAIL From: <address>".to_string()],
                                )
                                .await?;
                            }
                        }
                        "RCPT" => {
                            // Syntax RCPT To: <user@example.com>
                            match self.mailfrom {
                                Some(_) => {
                                    if let Some(address) = RE_SMTP_RCPT
                                        .captures(arg)
                                        .and_then(|cap| cap.get(1))
                                        .map(|m| m.as_str())
                                    {
                                        if let Some(rcpts) = &mut self.rcpts {
                                            rcpts.push(address.to_string());
                                        } else {
                                            self.rcpts = Some(vec![address.to_string()]);
                                        }

                                        Self::send_commands(
                                            &mut framed,
                                            vec!["250 OK".to_string()],
                                        )
                                        .await?;
                                    } else {
                                        Self::send_commands(
                                            &mut framed,
                                            vec!["501 Syntax: RCPT To: <address>".to_string()],
                                        )
                                        .await?;
                                    }
                                }
                                None => {
                                    Self::send_commands(
                                        &mut framed,
                                        vec!["503 Error: Send MAIL first".to_string()],
                                    )
                                    .await?;
                                }
                            }
                        }
                        "DATA" => {
                            if self.rcpts.is_none() {
                                Self::send_commands(
                                    &mut framed,
                                    vec!["503 Error: Send RCPT first".to_string()],
                                )
                                .await?;
                                return Ok(());
                            }

                            self.state = SmtpState::Data;

                            Self::send_commands(
                                &mut framed,
                                vec!["354 End data with <CRLF>.<CRLF>".to_string()],
                            )
                            .await?;
                        }
                        "NOOP" => {
                            Self::send_commands(&mut framed, vec!["250 OK".to_string()]).await?;
                        }
                        "RSET" => {
                            self.mailfrom = None;
                            self.rcpts = None;
                            self.message = String::new();
                            Self::send_commands(&mut framed, vec!["250 OK".to_string()]).await?;
                        }
                        "QUIT" => {
                            Self::send_commands(
                                &mut framed,
                                vec!["221 Have a nice day!".to_string()],
                            )
                            .await?;
                            self.state = SmtpState::Quit;
                        }
                        _ => {
                            Self::send_commands(
                                &mut framed,
                                vec!["500 Unknown command".to_string()],
                            )
                            .await?;
                        }
                    }
                }
                SmtpState::Data => {
                    if line.trim() == "." {
                        Self::send_commands(&mut framed, vec!["250 OK".to_string()]).await?;
                        tracing::info!("Received message: {}", self.message);

                        self.handle_message().await;

                        self.mailfrom = None;
                        self.rcpts = None;
                        self.message = String::new();
                        self.state = SmtpState::Command;
                    } else {
                        self.message.push_str(&line);
                        self.message.push_str("\n");
                    }
                }
                SmtpState::Quit => {
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_message(&self) {
        if let Some(rcpts) = self.rcpts.as_ref() {
            for rcpt in rcpts {
                if let Some(cap) = RE_EMAIL_PARTS.captures(rcpt) {
                    let (w_id, path, token, job_kind_path) = {
                        (
                            cap.get(1).unwrap().as_str(),
                            cap.get(2)
                                .unwrap_or_else(|| {
                                    cap.get(3).unwrap_or_else(|| cap.get(4).unwrap())
                                })
                                .as_str()
                                .replace(".", "/"),
                            cap.get(5).unwrap().as_str(),
                            if cap.get(2).is_some() {
                                "f"
                            } else if cap.get(3).is_some() {
                                "h"
                            } else {
                                "p"
                            },
                        )
                    };

                    let email = MessageParser::default().parse(&self.message);

                    let body = serde_json::json!({
                        "parsed_email": email,
                        "raw_email": self.message
                    });

                    match HTTP_CLIENT
                        .post(&format!(
                            "{}/api/w/{}/jobs/run/{}/{}",
                            self.base_internal_url, w_id, job_kind_path, path
                        ))
                        .bearer_auth(token)
                        .json(&body)
                        .send()
                        .await
                    {
                        Ok(response) => {
                            if let Err(err) = response.error_for_status() {
                                tracing::error!(
                                    "Error triggering job by email (rcpt: {}): {:?}",
                                    rcpt,
                                    err
                                )
                            }
                        }
                        Err(err) => {
                            tracing::error!(
                                "Error triggering job by email (rcpt: {}): {:?}",
                                rcpt,
                                err
                            )
                        }
                    }
                }
            }
        }
    }
}
