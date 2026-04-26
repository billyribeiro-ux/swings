//! SMTP transport wrapper used by the FDN-05 notifications pipeline.
//!
//! Historically this module also exposed transactional helpers
//! (`send_password_reset`, `send_welcome`, `send_subscription_confirmation`,
//! `send_subscription_cancelled`) that built their HTML inline from baked-in
//! Tera templates. Those helpers were superseded by the
//! `notifications::send_notification` path (templates live in
//! `notification_templates`, dispatch routes through `EmailChannel`) and were
//! removed in the audit (Phase 7.2 — dead-code sweep). Anything that needs
//! to push a fully-rendered email through SMTP now calls
//! [`EmailService::send_rendered`] via
//! [`crate::notifications::channels::email::LettreProvider`].

use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};

use crate::config::Config;

pub struct EmailService {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
}

impl EmailService {
    pub fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let mailer = if config.smtp_user.is_empty() {
            // Local dev: connect without authentication
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.smtp_host)
                .port(config.smtp_port)
                .build()
        } else {
            let creds = Credentials::new(config.smtp_user.clone(), config.smtp_password.clone());
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)?
                .port(config.smtp_port)
                .credentials(creds)
                .build()
        };

        Ok(Self {
            mailer,
            from: config.smtp_from.clone(),
        })
    }

    async fn send_email(
        &self,
        to_email: &str,
        to_name: &str,
        subject: &str,
        html_body: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let email = Message::builder()
            .from(self.from.parse()?)
            .to(format!("{to_name} <{to_email}>").parse()?)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html_body.to_string())?;

        self.mailer.send(email).await?;
        tracing::info!("Email sent to {to_email}: {subject}");
        Ok(())
    }

    /// Send a fully-rendered e-mail. Used by the FDN-05 notifications
    /// pipeline which resolves templates + preferences + suppression *before*
    /// calling the provider.
    pub async fn send_rendered(
        &self,
        to_email: &str,
        to_name: &str,
        subject: &str,
        html_body: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.send_email(to_email, to_name, subject, html_body).await
    }
}
