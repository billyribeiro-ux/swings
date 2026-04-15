use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};
use tera::{Context, Tera};

use crate::config::Config;

const BASE_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{{ subject }}</title>
  <style>
    body {
      margin: 0;
      padding: 0;
      background-color: #0a0f1c;
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
      color: #e2e8f0;
      -webkit-font-smoothing: antialiased;
    }
    .wrapper {
      width: 100%;
      background-color: #0a0f1c;
      padding: 40px 0;
    }
    .container {
      max-width: 600px;
      margin: 0 auto;
      background-color: #111827;
      border-radius: 12px;
      overflow: hidden;
      border: 1px solid #1e293b;
    }
    .header {
      background: linear-gradient(135deg, #0a0f1c 0%, #1a1f3c 100%);
      padding: 32px 40px;
      text-align: center;
      border-bottom: 2px solid #0fa4af;
    }
    .logo {
      font-size: 28px;
      font-weight: 800;
      color: #0fa4af;
      letter-spacing: -0.5px;
      text-decoration: none;
    }
    .body {
      padding: 40px;
    }
    .body h1 {
      font-size: 24px;
      font-weight: 700;
      color: #f1f5f9;
      margin: 0 0 16px 0;
    }
    .body p {
      font-size: 16px;
      line-height: 1.6;
      color: #94a3b8;
      margin: 0 0 16px 0;
    }
    .cta-wrapper {
      text-align: center;
      margin: 32px 0;
    }
    .cta-button {
      display: inline-block;
      background-color: #0fa4af;
      color: #ffffff !important;
      text-decoration: none;
      padding: 14px 40px;
      border-radius: 8px;
      font-size: 16px;
      font-weight: 600;
      letter-spacing: 0.3px;
    }
    .note {
      background-color: #0f172a;
      border-left: 3px solid #0fa4af;
      padding: 14px 18px;
      border-radius: 0 6px 6px 0;
      margin: 24px 0;
    }
    .note p {
      font-size: 14px;
      color: #64748b;
      margin: 0;
    }
    .divider {
      height: 1px;
      background-color: #1e293b;
      margin: 24px 0;
    }
    .footer {
      background-color: #0a0f1c;
      padding: 24px 40px;
      text-align: center;
      border-top: 1px solid #1e293b;
    }
    .footer p {
      font-size: 12px;
      color: #475569;
      margin: 0 0 8px 0;
      line-height: 1.5;
    }
    .footer a {
      color: #0fa4af;
      text-decoration: none;
    }
  </style>
</head>
<body>
  <div class="wrapper">
    <div class="container">
      <div class="header">
        <a href="{{ app_url }}" class="logo">Precision Options Signals</a>
      </div>
      <div class="body">
        {% block content %}{% endblock content %}
      </div>
      <div class="footer">
        <p>&copy; {{ year }} Precision Options Signals. All rights reserved.</p>
        <p>If you no longer wish to receive these emails, you can <a href="{{ app_url }}/settings">manage your preferences</a>.</p>
      </div>
    </div>
  </div>
</body>
</html>"#;

const PASSWORD_RESET_TEMPLATE: &str = r#"{% extends "base" %}
{% block content %}
<h1>Reset Your Password</h1>
<p>Hi {{ name }},</p>
<p>We received a request to reset the password for your Precision Options Signals account. Click the button below to choose a new password.</p>
<div class="cta-wrapper">
  <a href="{{ reset_url }}" class="cta-button">Reset Password</a>
</div>
<div class="note">
  <p>This link will expire in <strong style="color: #e2e8f0;">1 hour</strong>. After that, you&#39;ll need to request a new password reset.</p>
</div>
<div class="divider"></div>
<p style="font-size: 14px;">If you didn&#39;t request a password reset, you can safely ignore this email. Your password will remain unchanged.</p>
<p style="font-size: 13px; color: #475569;">If the button above doesn&#39;t work, copy and paste the following URL into your browser:</p>
<p style="font-size: 13px; color: #0fa4af; word-break: break-all;">{{ reset_url }}</p>
{% endblock content %}"#;

const WELCOME_TEMPLATE: &str = r#"{% extends "base" %}
{% block content %}
<h1>Welcome to Precision Options Signals!</h1>
<p>Hi {{ name }},</p>
<p>Thanks for creating your account. We&#39;re excited to have you on board!</p>
<p>Precision Options Signals gives you access to professional swing trading analysis, real-time alerts, and a community of traders dedicated to consistent results.</p>
<div class="cta-wrapper">
  <a href="{{ app_url }}" class="cta-button">Get Started</a>
</div>
<div class="divider"></div>
<p>Here&#39;s what you can do next:</p>
<p style="margin-bottom: 8px;"><strong style="color: #f1f5f9;">1.</strong> Explore our latest market analysis on the blog</p>
<p style="margin-bottom: 8px;"><strong style="color: #f1f5f9;">2.</strong> Subscribe to a plan for full access to trade alerts</p>
<p style="margin-bottom: 8px;"><strong style="color: #f1f5f9;">3.</strong> Set up your profile and notification preferences</p>
<div class="divider"></div>
<p style="font-size: 14px;">Questions? Just reply to this email &mdash; we&#39;d love to hear from you.</p>
{% endblock content %}"#;

const SUBSCRIPTION_CONFIRMATION_TEMPLATE: &str = r#"{% extends "base" %}
{% block content %}
<h1>Subscription Confirmed!</h1>
<p>Hi {{ name }},</p>
<p>Great news &mdash; your <strong style="color: #0fa4af;">{{ plan_name }}</strong> subscription is now active!</p>
<p>You now have full access to all premium features including real-time trade alerts, detailed analysis, and exclusive member content.</p>
<div class="cta-wrapper">
  <a href="{{ app_url }}/member" class="cta-button">Go to Dashboard</a>
</div>
<div class="divider"></div>
<p>Your subscription includes:</p>
<p style="margin-bottom: 8px;"><strong style="color: #f1f5f9;">&#10003;</strong> Real-time swing trade alerts</p>
<p style="margin-bottom: 8px;"><strong style="color: #f1f5f9;">&#10003;</strong> Detailed entry, stop-loss, and target levels</p>
<p style="margin-bottom: 8px;"><strong style="color: #f1f5f9;">&#10003;</strong> Market analysis and commentary</p>
<p style="margin-bottom: 8px;"><strong style="color: #f1f5f9;">&#10003;</strong> Members-only content and resources</p>
<div class="note">
  <p>You can manage your subscription anytime from your <a href="{{ app_url }}/member" style="color: #0fa4af; text-decoration: none;">account dashboard</a>.</p>
</div>
{% endblock content %}"#;

const SUBSCRIPTION_CANCELLED_TEMPLATE: &str = r#"{% extends "base" %}
{% block content %}
<h1>Subscription Cancelled</h1>
<p>Hi {{ name }},</p>
<p>We&#39;re sorry to see you go. Your subscription has been cancelled, but you&#39;ll continue to have access to premium features until <strong style="color: #0fa4af;">{{ end_date }}</strong>.</p>
<p>After that date, your account will revert to the free tier.</p>
<div class="divider"></div>
<p>A few things to keep in mind:</p>
<div class="note">
  <p>You can re-subscribe at any time to regain full access. Your account and preferences will be preserved.</p>
</div>
<div class="cta-wrapper">
  <a href="{{ app_url }}/member" class="cta-button">Resubscribe</a>
</div>
<div class="divider"></div>
<p style="font-size: 14px;">We&#39;d love to know how we can improve. If you have any feedback, simply reply to this email.</p>
{% endblock content %}"#;

pub struct EmailService {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
    templates: Tera,
    app_url: String,
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

        let mut templates = Tera::default();
        templates.add_raw_template("base", BASE_TEMPLATE)?;
        templates.add_raw_template("password_reset", PASSWORD_RESET_TEMPLATE)?;
        templates.add_raw_template("welcome", WELCOME_TEMPLATE)?;
        templates.add_raw_template(
            "subscription_confirmation",
            SUBSCRIPTION_CONFIRMATION_TEMPLATE,
        )?;
        templates.add_raw_template("subscription_cancelled", SUBSCRIPTION_CANCELLED_TEMPLATE)?;

        Ok(Self {
            mailer,
            from: config.smtp_from.clone(),
            templates,
            app_url: config.app_url.clone(),
        })
    }

    fn current_year() -> String {
        chrono::Utc::now().format("%Y").to_string()
    }

    fn base_context(&self) -> Context {
        let mut ctx = Context::new();
        ctx.insert("app_url", &self.app_url);
        ctx.insert("year", &Self::current_year());
        ctx
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

    pub async fn send_password_reset(
        &self,
        to_email: &str,
        to_name: &str,
        reset_token: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let reset_url = format!(
            "{}/admin/reset-password?token={}",
            self.app_url, reset_token
        );

        let mut ctx = self.base_context();
        ctx.insert("subject", "Reset Your Password");
        ctx.insert("name", to_name);
        ctx.insert("reset_url", &reset_url);

        let html = self.templates.render("password_reset", &ctx)?;
        self.send_email(
            to_email,
            to_name,
            "Reset Your Password — Precision Options Signals",
            &html,
        )
        .await
    }

    pub async fn send_welcome(
        &self,
        to_email: &str,
        to_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = self.base_context();
        ctx.insert("subject", "Welcome to Precision Options Signals");
        ctx.insert("name", to_name);

        let html = self.templates.render("welcome", &ctx)?;
        self.send_email(
            to_email,
            to_name,
            "Welcome to Precision Options Signals!",
            &html,
        )
        .await
    }

    pub async fn send_subscription_confirmation(
        &self,
        to_email: &str,
        to_name: &str,
        plan_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = self.base_context();
        ctx.insert("subject", "Subscription Confirmed");
        ctx.insert("name", to_name);
        ctx.insert("plan_name", plan_name);

        let html = self.templates.render("subscription_confirmation", &ctx)?;
        self.send_email(
            to_email,
            to_name,
            "Your Subscription is Active — Precision Options Signals",
            &html,
        )
        .await
    }

    pub async fn send_subscription_cancelled(
        &self,
        to_email: &str,
        to_name: &str,
        end_date: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = self.base_context();
        ctx.insert("subject", "Subscription Cancelled");
        ctx.insert("name", to_name);
        ctx.insert("end_date", end_date);

        let html = self.templates.render("subscription_cancelled", &ctx)?;
        self.send_email(
            to_email,
            to_name,
            "Your Subscription Has Been Cancelled — Precision Options Signals",
            &html,
        )
        .await
    }
}
