//! SMTP client for sending email (reply, forward, new message).
//!
//! Uses `lettre` with tokio async transport and native-tls for STARTTLS/TLS.

use lettre::{
    AsyncTransport, Message,
    message::{Mailbox, header},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, Tokio1Executor,
};
use lettre::transport::smtp::client::{Tls, TlsParameters};

use crate::config::SmtpConfig;

/// An SMTP sender session.
pub struct SmtpSender {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    config: SmtpConfig,
}

impl SmtpSender {
    /// Create a new SMTP sender from config.
    pub fn new(config: &SmtpConfig) -> Result<Self, String> {
        let tls_params = TlsParameters::builder(config.host.clone())
            .dangerous_accept_invalid_certs(false)
            .build()
            .map_err(|e| format!("TLS config error: {e}"))?;

        let tls = if config.port == 465 {
            Tls::Wrapper(tls_params)
        } else {
            Tls::Required(tls_params)
        };

        let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
            .map_err(|e| format!("SMTP relay config error: {e}"))?
            .port(config.port)
            .credentials(Credentials::new(
                config.username.clone(),
                config.password.clone(),
            ))
            .tls(tls)
            .build();

        Ok(Self {
            transport,
            config: config.clone(),
        })
    }

    /// Send an email message.
    pub async fn send(&self, message: Message) -> Result<(), String> {
        self.transport
            .send(message)
            .await
            .map_err(|e| format!("SMTP send error: {e}"))?;
        Ok(())
    }

    /// Build and send a reply message.
    pub async fn send_reply(
        &self,
        from: (&str, &str),       // (display_name, email)
        to: &str,                  // recipient email
        subject: &str,
        body: &str,
    ) -> Result<(), String> {
        let from_mbox = Mailbox::new(
            Some(from.0.to_string()),
            from.1.parse().map_err(|e| format!("Invalid from address: {e}"))?,
        );

        let to_mbox = Mailbox::new(
            None,
            to.parse().map_err(|e| format!("Invalid to address: {e}"))?,
        );

        let email = Message::builder()
            .from(from_mbox)
            .to(to_mbox)
            .subject(subject)
            .header(header::ContentType::TEXT_PLAIN)
            .body(body.to_string())
            .map_err(|e| format!("Build message error: {e}"))?;

        self.send(email).await
    }

    /// Build and send a reply-all message (to original sender + all other
    /// To/CC recipients except ourselves).
    pub async fn send_reply_all(
        &self,
        from: (&str, &str),       // (display_name, email)
        to_addresses: &[&str],    // all recipients
        subject: &str,
        body: &str,
    ) -> Result<(), String> {
        let from_mbox = Mailbox::new(
            Some(from.0.to_string()),
            from.1.parse().map_err(|e| format!("Invalid from address: {e}"))?,
        );

        let to_mboxes: Result<Vec<Mailbox>, String> = to_addresses
            .iter()
            .map(|addr| {
                addr.parse::<lettre::Address>()
                    .map(|a| Mailbox::new(None, a))
                    .map_err(|e| format!("Invalid address '{addr}': {e}"))
            })
            .collect();
        let to_mboxes = to_mboxes?;

        let mut builder = Message::builder()
            .from(from_mbox)
            .subject(subject)
            .header(header::ContentType::TEXT_PLAIN);

        // Add To recipients
        for mbox in &to_mboxes {
            builder = builder.to(mbox.clone());
        }

        let email = builder
            .body(body.to_string())
            .map_err(|e| format!("Build message error: {e}"))?;

        self.send(email).await
    }

    /// Build and send a forward message (with quoted original content).
    pub async fn send_forward(
        &self,
        from: (&str, &str),
        to: &str,
        original_subject: &str,
        original_from: &str,
        original_date: &str,
        original_body: &str,
    ) -> Result<(), String> {
        let from_mbox = Mailbox::new(
            Some(from.0.to_string()),
            from.1.parse().map_err(|e| format!("Invalid from address: {e}"))?,
        );

        let to_mbox = Mailbox::new(
            None,
            to.parse().map_err(|e| format!("Invalid to address: {e}"))?,
        );

        let subject = if original_subject.to_lowercase().starts_with("fwd:") {
            original_subject.to_string()
        } else {
            format!("Fwd: {original_subject}")
        };

        let body = format!(
            "\n---------- Forwarded message ----------\n\
             From: {original_from}\n\
             Date: {original_date}\n\
             Subject: {original_subject}\n\n\
             {original_body}"
        );

        let email = Message::builder()
            .from(from_mbox)
            .to(to_mbox)
            .subject(subject)
            .header(header::ContentType::TEXT_PLAIN)
            .body(body)
            .map_err(|e| format!("Build message error: {e}"))?;

        self.send(email).await
    }
}
