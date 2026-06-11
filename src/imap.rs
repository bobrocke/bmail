//! IMAP client using `async-imap` with TLS.
//!
//! Connects to an IMAP server, lists folders, fetches email headers and bodies,
//! manages flags (seen, deleted, flagged), and supports IDLE for push notifications.

use async_imap::error::Result as ImapResult;
use async_imap::types::{Fetch, Flag};
use async_native_tls::TlsConnector;
use futures::TryStreamExt;
use tokio::net::TcpStream;

use crate::config::ImapConfig;

/// An active IMAP session.
pub struct ImapSession {
    session: async_imap::Session<async_native_tls::TlsStream<TcpStream>>,
    pub config: ImapConfig,
}

/// A lightweight folder summary from IMAP.
#[derive(Debug, Clone)]
pub struct ImapFolder {
    pub name: String,
    pub attributes: Vec<String>,
    /// True if this folder can contain messages (not a namespace-only node).
    pub selectable: bool,
    pub delimiter: Option<String>,
}

/// Raw email envelope data fetched from IMAP.
#[derive(Debug, Clone)]
pub struct EmailEnvelope {
    pub uid: u32,
    pub seq: u32,
    pub subject: String,
    pub from: String,
    pub to: String,
    pub date: String,
    pub seen: bool,
    pub flagged: bool,
    pub deleted: bool,
    pub answered: bool,
    pub size: u32,
}

impl ImapSession {
    /// Connect to an IMAP server and log in.
    pub async fn connect(config: &ImapConfig) -> ImapResult<Self> {
        let addr = format!("{}:{}", config.host, config.port);
        log::info!("IMAP connecting to {addr}...");

        let tcp = TcpStream::connect(&addr).await.map_err(|e| {
            log::error!("IMAP TCP connect failed: {e}");
            e
        })?;
        log::info!("IMAP TCP connected");

        let tls = TlsConnector::new();
        let tls_stream = tls.connect(&config.host, tcp).await
            .map_err(|e| {
                log::error!("IMAP TLS handshake failed: {e}");
                std::io::Error::other(e.to_string())
            })?;
        log::info!("IMAP TLS established");

        let client = async_imap::Client::new(tls_stream);
        let session = client
            .login(&config.username, &config.password)
            .await
            .map_err(|(e, _client)| {
                log::error!("IMAP login failed: {e}");
                e
            })?;
        log::info!("IMAP login successful for {}", config.username);

        Ok(Self {
            session,
            config: config.clone(),
        })
    }

    /// List all mailbox folders.
    pub async fn list_folders(&mut self) -> ImapResult<Vec<ImapFolder>> {
        let mailboxes: Vec<_> = self.session.list(None, Some("*")).await?
            .try_collect().await?;
        let mut folders = Vec::new();

        for mb in mailboxes {
            let attrs: Vec<String> = mb
                .attributes()
                .iter()
                .map(|a| format!("{a:?}"))
                .collect();

            let selectable = !attrs.iter().any(|a| {
                a.contains("Noselect") || a.contains("NoSelect")
            });

            folders.push(ImapFolder {
                name: mb.name().to_string(),
                attributes: attrs,
                selectable,
                delimiter: mb.delimiter().map(|d| d.to_string()),
            });
        }

        Ok(folders)
    }

    /// Select a folder (opens it for reading).
    pub async fn select_folder(&mut self, folder: &str) -> ImapResult<async_imap::types::Mailbox> {
        let mailbox = self.session.select(folder).await?;
        Ok(mailbox)
    }

    /// Examine a folder (read-only open, for getting counts without marking seen).
    pub async fn examine_folder(
        &mut self,
        folder: &str,
    ) -> ImapResult<async_imap::types::Mailbox> {
        let mailbox = self.session.examine(folder).await?;
        Ok(mailbox)
    }

    /// Fetch email envelopes (subject, from, date, flags, uid) for a range of
    /// sequence numbers. Returns newest first.
    pub async fn fetch_envelopes(
        &mut self,
        start: u32,
        end: u32,
    ) -> ImapResult<Vec<EmailEnvelope>> {
        let range = format!("{start}:{end}");
        let fetches: Vec<_> = self
            .session
            .fetch(
                &range,
                "(UID FLAGS INTERNALDATE RFC822.SIZE BODY.PEEK[HEADER.FIELDS (SUBJECT FROM TO DATE)])",
            )
            .await?
            .try_collect().await?;

        let mut envelopes = Vec::new();
        for fetch in &fetches {
            if let Some(envelope) = parse_envelope(fetch) {
                envelopes.push(envelope);
            }
        }

        // Reverse: IMAP returns in ascending order, we want newest first.
        envelopes.reverse();
        Ok(envelopes)
    }

    /// Fetch email envelopes by UID range.
    pub async fn fetch_envelopes_by_uid(
        &mut self,
        uid_range: &str,
    ) -> ImapResult<Vec<EmailEnvelope>> {
        let fetches: Vec<_> = self
            .session
            .uid_fetch(
                uid_range,
                "(UID FLAGS INTERNALDATE RFC822.SIZE BODY.PEEK[HEADER.FIELDS (SUBJECT FROM TO DATE)])",
            )
            .await?
            .try_collect().await?;

        let mut envelopes = Vec::new();
        for fetch in &fetches {
            if let Some(envelope) = parse_envelope(fetch) {
                envelopes.push(envelope);
            }
        }

        envelopes.reverse();
        Ok(envelopes)
    }

    /// Fetch the full body of a specific email by UID.
    pub async fn fetch_body(&mut self, uid: u32) -> ImapResult<Option<String>> {
        let uid_str = uid.to_string();
        let fetches: Vec<_> = self
            .session
            .uid_fetch(&uid_str, "(BODY[TEXT])")
            .await?
            .try_collect().await?;

        for fetch in &fetches {
            if let Some(body) = fetch.text() {
                return Ok(Some(String::from_utf8_lossy(body).to_string()));
            }
        }
        Ok(None)
    }

    /// Fetch a specific MIME part by UID and part number.
    pub async fn fetch_mime_part(
        &mut self,
        uid: u32,
        part: &str,
    ) -> ImapResult<Option<Vec<u8>>> {
        let uid_str = uid.to_string();
        let query = format!("BODY[{}]", part);
        let fetches: Vec<_> = self.session.uid_fetch(&uid_str, &query)
            .await?
            .try_collect().await?;

        for fetch in &fetches {
            if let Some(body) = fetch.text() {
                return Ok(Some(body.to_vec()));
            }
        }
        Ok(None)
    }

    /// Set flags on an email by UID.
    pub async fn set_flags(&mut self, uid: u32, flags: &[&str], add: bool) -> ImapResult<()> {
        let uid_str = uid.to_string();
        let flag_str = flags
            .iter()
            .map(|f| format!("\\{f}"))
            .collect::<Vec<_>>()
            .join(" ");

        let operation = if add { "+FLAGS" } else { "-FLAGS" };
        let command = format!("{operation} ({flag_str})");

        self.session.uid_store(&uid_str, &command).await?.try_collect::<Vec<_>>().await?;
        Ok(())
    }

    /// Mark an email as seen.
    pub async fn mark_seen(&mut self, uid: u32) -> ImapResult<()> {
        self.set_flags(uid, &["Seen"], true).await
    }

    /// Mark an email as unseen.
    pub async fn mark_unseen(&mut self, uid: u32) -> ImapResult<()> {
        self.set_flags(uid, &["Seen"], false).await
    }

    /// Mark an email as deleted.
    pub async fn mark_deleted(&mut self, uid: u32) -> ImapResult<()> {
        self.set_flags(uid, &["Deleted"], true).await
    }

    /// Move an email to another folder (copy + delete + expunge).
    pub async fn move_to(&mut self, uid: u32, destination: &str) -> ImapResult<()> {
        let uid_str = uid.to_string();
        self.session.uid_copy(&uid_str, destination).await?;
        self.mark_deleted(uid).await?;
        self.session.expunge().await?.try_collect::<Vec<_>>().await?;
        Ok(())
    }

    /// Expunge deleted messages.
    pub async fn expunge(&mut self) -> ImapResult<()> {
        self.session.expunge().await?.try_collect::<Vec<_>>().await?;
        Ok(())
    }

    /// Get the raw session for advanced operations.
    pub fn raw_session(
        &mut self,
    ) -> &mut async_imap::Session<async_native_tls::TlsStream<TcpStream>> {
        &mut self.session
    }

    /// Log out and close the connection.
    pub async fn logout(mut self) -> ImapResult<()> {
        self.session.logout().await?;
        Ok(())
    }
}

/// Parse an IMAP FETCH response into an EmailEnvelope.
fn parse_envelope(fetch: &Fetch) -> Option<EmailEnvelope> {
    let uid = fetch.uid?;
    let seq = fetch.message;
    let flags: Vec<_> = fetch.flags().collect();
    let seen = flags.contains(&Flag::Seen);
    let flagged = flags.contains(&Flag::Flagged);
    let deleted = flags.contains(&Flag::Deleted);
    let answered = flags.contains(&Flag::Answered);

    // Parse header fields from the fetch
    let header = fetch.header()?;
    let header_str = String::from_utf8_lossy(header);

    let subject = extract_header(&header_str, "Subject").unwrap_or_default();
    let from = extract_header(&header_str, "From").unwrap_or_default();
    let to = extract_header(&header_str, "To").unwrap_or_default();
    let date = extract_header(&header_str, "Date").unwrap_or_default();

    let size = fetch.size.unwrap_or(0);

    Some(EmailEnvelope {
        uid,
        seq,
        subject: decode_rfc2047(&subject),
        from: decode_rfc2047(&from),
        to: decode_rfc2047(&to),
        date,
        seen,
        flagged,
        deleted,
        answered,
        size,
    })
}

/// Extract a header value from raw header text.
fn extract_header(headers: &str, name: &str) -> Option<String> {
    let search = format!("\n{name}: ");
    let alt_search = format!("{name}: ");

    // Try with leading newline first (matches after first header)
    if let Some(pos) = headers.find(&search) {
        let start = pos + search.len();
        let end = headers[start..]
            .find('\n')
            .map(|p| start + p)
            .unwrap_or(headers.len());
        let value = headers[start..end].trim();
        // Unfold continuation lines
        let value = value.replace("\r\n ", " ").replace("\n ", " ");
        return Some(value.to_string());
    }

    // Try at the start of the string
    if headers.starts_with(&alt_search) {
        let start = alt_search.len();
        let end = headers[start..]
            .find('\n')
            .map(|p| start + p)
            .unwrap_or(headers.len());
        let value = headers[start..end].trim();
        let value = value.replace("\r\n ", " ").replace("\n ", " ");
        return Some(value.to_string());
    }

    None
}

/// Decode RFC 2047 encoded words (e.g. `=?UTF-8?B?...?=` or `=?UTF-8?Q?...?=`).
fn decode_rfc2047(input: &str) -> String {
    // Quick check: if no encoded words, return as-is.
    if !input.contains("=?") || !input.contains("?=") {
        return input.to_string();
    }

    let mut result = String::new();
    let mut remaining = input;

    while let Some(start) = remaining.find("=?") {
        // Add text before the encoded word
        result.push_str(&remaining[..start]);
        remaining = &remaining[start..];

        // Find the end of the encoded word
        if let Some(end) = remaining.find("?=") {
            let encoded = &remaining[..end + 2];
            remaining = &remaining[end + 2..];

            // Parse: =?charset?encoding?text?=
            let parts: Vec<&str> = encoded
                .trim_start_matches("=?")
                .trim_end_matches("?=")
                .splitn(3, '?')
                .collect();

            if parts.len() == 3 {
                let encoding = parts[1].to_uppercase();
                let text = parts[2];

                match encoding.as_str() {
                    "B" | "b" => {
                        // Base64 decode
                        if let Ok(decoded) = base64_decode(text) {
                            result.push_str(&decoded);
                        } else {
                            result.push_str(text);
                        }
                    }
                    "Q" | "q" => {
                        // Quoted-printable decode
                        let decoded = quoted_printable_decode(text);
                        result.push_str(&decoded);
                    }
                    _ => {
                        result.push_str(text);
                    }
                }
            }
        } else {
            result.push_str(remaining);
            break;
        }
    }

    result.push_str(remaining);
    result
}

/// Simple base64 decode for RFC 2047 B-encoding.
fn base64_decode(input: &str) -> Result<String, ()> {
    use std::collections::HashMap;

    let charset: HashMap<char, u8> = ('A'..='Z')
        .enumerate()
        .map(|(i, c)| (c, i as u8))
        .chain(('a'..='z').enumerate().map(|(i, c)| (c, 26 + i as u8)))
        .chain(('0'..='9').enumerate().map(|(i, c)| (c, 52 + i as u8)))
        .chain([('+', 62), ('/', 63)])
        .collect();

    let input = input.replace(['\n', '\r', ' '], "");
    let mut output = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;

    for c in input.chars() {
        if c == '=' {
            break; // Padding
        }
        if let Some(&val) = charset.get(&c) {
            buffer = (buffer << 6) | val as u32;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                output.push((buffer >> bits) as u8);
                buffer &= (1 << bits) - 1;
            }
        }
    }

    String::from_utf8(output).map_err(|_| ())
}

/// Simple quoted-printable decode for RFC 2047 Q-encoding.
fn quoted_printable_decode(input: &str) -> String {
    let input = input.replace('_', " ");
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '=' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                } else {
                    result.push('=');
                    result.push_str(&hex);
                }
            } else {
                result.push('=');
                result.push_str(&hex);
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Convert an ImapFolder list into a hierarchical tree for the folder pane.
#[derive(Debug, Clone)]
pub struct FolderNode {
    pub name: String,
    pub full_path: String,
    pub selectable: bool,
    pub children: Vec<FolderNode>,
}

/// Build a nested folder tree from a flat list of IMAP folders.
pub fn build_folder_tree(folders: &[ImapFolder]) -> Vec<FolderNode> {
    let mut roots: Vec<FolderNode> = Vec::new();

    for folder in folders {
        let delimiter = folder
            .delimiter
            .as_deref()
            .unwrap_or("/");

        let parts: Vec<&str> = folder.name.split(delimiter).collect();
        if parts.is_empty() {
            continue;
        }

        // Insert into tree
        insert_into_tree(
            &mut roots,
            &parts,
            &folder.name,
            folder.selectable,
            0,
        );
    }

    // Sort: INBOX first, then alphabetically
    sort_tree(&mut roots);

    roots
}

fn insert_into_tree(
    nodes: &mut Vec<FolderNode>,
    parts: &[&str],
    full_path: &str,
    selectable: bool,
    depth: usize,
) {
    if depth >= parts.len() {
        return;
    }

    let name = parts[depth].to_string();
    let is_leaf = depth == parts.len() - 1;

    // Find or create this node
    let pos = nodes.iter().position(|n| n.name == name);
    if let Some(idx) = pos {
        if is_leaf {
            nodes[idx].selectable = selectable;
        }
        if !is_leaf {
            insert_into_tree(
                &mut nodes[idx].children,
                parts,
                full_path,
                selectable,
                depth + 1,
            );
        }
    } else {
        let mut node = FolderNode {
            name: name.clone(),
            full_path: if is_leaf {
                full_path.to_string()
            } else {
                // For intermediate nodes, reconstruct the partial path
                parts[..=depth].join("/")
            },
            selectable: is_leaf && selectable,
            children: Vec::new(),
        };

        if !is_leaf {
            insert_into_tree(
                &mut node.children,
                parts,
                full_path,
                selectable,
                depth + 1,
            );
        }

        nodes.push(node);
    }
}

fn sort_tree(nodes: &mut Vec<FolderNode>) {
    nodes.sort_by(|a, b| {
        // INBOX always first
        if a.name.eq_ignore_ascii_case("INBOX") {
            return std::cmp::Ordering::Less;
        }
        if b.name.eq_ignore_ascii_case("INBOX") {
            return std::cmp::Ordering::Greater;
        }
        a.name.to_lowercase().cmp(&b.name.to_lowercase())
    });

    for node in nodes.iter_mut() {
        sort_tree(&mut node.children);
    }
}
