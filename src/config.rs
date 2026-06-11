//! Configuration loading, saving, and first-run setup.
//!
//! Config lives at `~/.config/bMail/config.toml` (XDG-compliant).
//! On first run, the app presents a setup screen to collect IMAP/SMTP credentials.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Full application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// IMAP account settings.
    pub imap: ImapConfig,
    /// SMTP account settings (for sending).
    pub smtp: SmtpConfig,
    /// User display name and email address for outgoing mail.
    pub identity: Identity,
    /// UI preferences.
    #[serde(default)]
    pub preferences: Preferences,
}

/// IMAP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImapConfig {
    /// IMAP server hostname (e.g. "imap.gmail.com").
    pub host: String,
    /// IMAP server port (typically 993 for TLS).
    pub port: u16,
    /// Login username (usually the full email address).
    pub username: String,
    /// Login password or app-specific password.
    pub password: String,
}

/// SMTP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    /// SMTP server hostname (e.g. "smtp.gmail.com").
    pub host: String,
    /// SMTP server port (typically 587 for STARTTLS, 465 for TLS).
    pub port: u16,
    /// Login username (often same as IMAP).
    pub username: String,
    /// Login password or app-specific password.
    pub password: String,
}

/// Sender identity for outgoing mail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// Display name (e.g. "Alice Smith").
    pub display_name: String,
    /// Email address (e.g. "alice@gmail.com").
    pub email: String,
}

/// UI preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    /// Number of emails to fetch per folder on first load.
    #[serde(default = "default_fetch_count")]
    pub fetch_count: usize,
    /// Refresh interval in seconds (0 = manual only).
    #[serde(default)]
    pub refresh_interval_secs: u64,
    /// Show unread count badges in folder pane.
    #[serde(default = "default_true")]
    pub show_unread_badges: bool,
    /// Mark email as read after N seconds of viewing (0 = instantly).
    #[serde(default)]
    pub auto_mark_read_secs: u64,
}

fn default_fetch_count() -> usize {
    50
}
fn default_true() -> bool {
    true
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            fetch_count: 50,
            refresh_interval_secs: 300,
            show_unread_badges: true,
            auto_mark_read_secs: 2,
        }
    }
}

impl Config {
    /// Path to the config file: `~/.config/bMail/config.toml`.
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("bMail")
            .join("config.toml")
    }

    /// Load config from disk. Returns `None` if the file doesn't exist
    /// (first run) or if it can't be parsed.
    pub fn load() -> Option<Self> {
        let path = Self::path();
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&path).ok()?;
        toml::from_str(&content).ok()
    }

    /// Save config to disk. Creates parent directories if needed.
    /// Also sets restrictive permissions (0o600) on the file since it
    /// contains plaintext passwords.
    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create config dir: {e}"))?;
        }

        let content =
            toml::to_string_pretty(self).map_err(|e| format!("Cannot serialize config: {e}"))?;

        std::fs::write(&path, &content).map_err(|e| format!("Cannot write config: {e}"))?;

        // Restrict permissions: owner read/write only.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).ok();
        }

        log::info!("Config saved to {:?}", path);
        Ok(())
    }

    /// Check if config exists (i.e., not first run).
    pub fn exists() -> bool {
        Self::path().exists()
    }

    /// Try to auto-detect SMTP settings from IMAP host.
    /// Handles common providers: Gmail, Fastmail, iCloud, Outlook, Yahoo,
    /// ProtonMail Bridge, and generic patterns.
    pub fn suggest_smtp(imap_host: &str) -> Option<(String, u16)> {
        let host = imap_host.to_lowercase();

        // Known providers
        if host.contains("gmail.com") {
            return Some(("smtp.gmail.com".into(), 587));
        }
        if host.contains("fastmail.com") {
            return Some(("smtp.fastmail.com".into(), 587));
        }
        if host.contains("icloud.com") {
            return Some(("smtp.mail.me.com".into(), 587));
        }
        if host.contains("outlook.com") || host.contains("hotmail.com") || host.contains("live.com")
        {
            return Some(("smtp-mail.outlook.com".into(), 587));
        }
        if host.contains("yahoo.com") {
            return Some(("smtp.mail.yahoo.com".into(), 587));
        }
        if host.contains("protonmail") || host.contains("proton.me") {
            // ProtonMail Bridge uses localhost
            return Some(("127.0.0.1".into(), 1025));
        }

        // Generic heuristic: replace "imap" with "smtp" in the hostname
        let smtp_host = host.replace("imap", "smtp");
        if smtp_host != host {
            return Some((smtp_host, 587));
        }

        None
    }

    /// Check for optional environment variable overrides.
    /// BMAIL_IMAP_HOST, BMAIL_IMAP_PORT, BMAIL_IMAP_USER, BMAIL_IMAP_PASS
    /// BMAIL_SMTP_HOST, BMAIL_SMTP_PORT, BMAIL_SMTP_USER, BMAIL_SMTP_PASS
    /// BMAIL_IDENTITY_NAME, BMAIL_IDENTITY_EMAIL
    pub fn env_overrides(&mut self) {
        if let Ok(v) = std::env::var("BMAIL_IMAP_HOST") {
            self.imap.host = v;
        }
        if let Ok(v) = std::env::var("BMAIL_IMAP_PORT") {
            if let Ok(p) = v.parse() {
                self.imap.port = p;
            }
        }
        if let Ok(v) = std::env::var("BMAIL_IMAP_USER") {
            self.imap.username = v;
        }
        if let Ok(v) = std::env::var("BMAIL_IMAP_PASS") {
            self.imap.password = v;
        }
        if let Ok(v) = std::env::var("BMAIL_SMTP_HOST") {
            self.smtp.host = v;
        }
        if let Ok(v) = std::env::var("BMAIL_SMTP_PORT") {
            if let Ok(p) = v.parse() {
                self.smtp.port = p;
            }
        }
        if let Ok(v) = std::env::var("BMAIL_SMTP_USER") {
            self.smtp.username = v;
        }
        if let Ok(v) = std::env::var("BMAIL_SMTP_PASS") {
            self.smtp.password = v;
        }
        if let Ok(v) = std::env::var("BMAIL_IDENTITY_NAME") {
            self.identity.display_name = v;
        }
        if let Ok(v) = std::env::var("BMAIL_IDENTITY_EMAIL") {
            self.identity.email = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smtp_suggestions() {
        assert_eq!(
            Config::suggest_smtp("imap.gmail.com"),
            Some(("smtp.gmail.com".into(), 587))
        );
        assert_eq!(
            Config::suggest_smtp("imap.fastmail.com"),
            Some(("smtp.fastmail.com".into(), 587))
        );
        assert_eq!(
            Config::suggest_smtp("mail.example.com"),
            Some(("mail.example.com".into(), 587)) // generic pattern doesn't match
        );
    }
}
