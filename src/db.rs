//! SQLite database for caching IMAP state.
//!
//! Schema:
//! - `folders`: cached folder list with unread/total counts
//! - `emails`: cached email headers + body for offline access
//! - `flags`: per-email flags (seen, deleted, flagged, etc.)
//!
//! Database location: `~/.local/share/bMail/bmail.db`

use rusqlite::{params, Connection};
use std::path::PathBuf;

/// Email metadata cached from IMAP.
#[derive(Debug, Clone)]
pub struct CachedEmail {
    /// IMAP sequence number (1-based within folder).
    pub seq: u32,
    /// IMAP UID (unique across sessions).
    pub uid: u32,
    /// Folder this email belongs to.
    pub folder: String,
    /// Email subject.
    pub subject: String,
    /// Sender display name and address.
    pub from: String,
    /// Recipients (To field).
    pub to: String,
    /// Date as RFC 2822 string.
    pub date: String,
    /// Plain text body (after HTML→Markdown conversion).
    pub body_text: String,
    /// Whether this email has been read.
    pub seen: bool,
    /// Whether this email is flagged/starred.
    pub flagged: bool,
    /// Whether this email is marked for deletion.
    pub deleted: bool,
    /// Whether this email has been answered/replied to.
    pub answered: bool,
    /// Whether the full body has been fetched and cached.
    pub body_fetched: bool,
}

/// Folder metadata cached from IMAP.
#[derive(Debug, Clone)]
pub struct CachedFolder {
    pub name: String,
    pub total: u32,
    pub unread: u32,
}

/// Database handle.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) the database at `~/.local/share/bMail/bmail.db`.
    pub fn open() -> Result<Self, String> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Cannot create data dir: {e}"))?;
        }

        let conn = Connection::open(&path).map_err(|e| format!("Cannot open database: {e}"))?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| format!("Cannot set pragmas: {e}"))?;

        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("bMail")
            .join("bmail.db")
    }

    /// Create tables if they don't exist.
    fn migrate(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS folders (
                name        TEXT PRIMARY KEY,
                total       INTEGER NOT NULL DEFAULT 0,
                unread      INTEGER NOT NULL DEFAULT 0,
                updated_at  INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS emails (
                uid         INTEGER NOT NULL,
                folder      TEXT NOT NULL,
                seq         INTEGER NOT NULL DEFAULT 0,
                subject     TEXT NOT NULL DEFAULT '',
                sender      TEXT NOT NULL DEFAULT '',
                recipients  TEXT NOT NULL DEFAULT '',
                date        TEXT NOT NULL DEFAULT '',
                body_text   TEXT NOT NULL DEFAULT '',
                body_fetched INTEGER NOT NULL DEFAULT 0,
                seen        INTEGER NOT NULL DEFAULT 0,
                flagged     INTEGER NOT NULL DEFAULT 0,
                deleted     INTEGER NOT NULL DEFAULT 0,
                answered    INTEGER NOT NULL DEFAULT 0,
                fetched_at  INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (uid, folder)
            );

            CREATE INDEX IF NOT EXISTS idx_emails_folder ON emails(folder);
            CREATE INDEX IF NOT EXISTS idx_emails_date ON emails(folder, date DESC);
            ",
            )
            .map_err(|e| format!("Migration failed: {e}"))
    }

    // ── Folder operations ──

    /// Replace all cached folders with a fresh list from IMAP.
    pub fn sync_folders(&self, folders: &[CachedFolder]) -> Result<(), String> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| format!("Transaction failed: {e}"))?;

        tx.execute("DELETE FROM folders", [])
            .map_err(|e| format!("Delete folders failed: {e}"))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        for f in folders {
            tx.execute(
                "INSERT INTO folders (name, total, unread, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params![f.name, f.total, f.unread, now],
            )
            .map_err(|e| format!("Insert folder failed: {e}"))?;
        }

        tx.commit().map_err(|e| format!("Commit failed: {e}"))?;
        Ok(())
    }

    /// Get all cached folders.
    pub fn get_folders(&self) -> Result<Vec<CachedFolder>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, total, unread FROM folders ORDER BY name")
            .map_err(|e| format!("Prepare failed: {e}"))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(CachedFolder {
                    name: row.get(0)?,
                    total: row.get(1)?,
                    unread: row.get(2)?,
                })
            })
            .map_err(|e| format!("Query failed: {e}"))?;

        let mut folders = Vec::new();
        for row in rows {
            folders.push(row.map_err(|e| format!("Row error: {e}"))?);
        }
        Ok(folders)
    }

    // ── Email operations ──

    /// Upsert an email (insert or update).
    pub fn upsert_email(&self, email: &CachedEmail) -> Result<(), String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        self.conn
            .execute(
                "INSERT INTO emails (uid, folder, seq, subject, sender, recipients, date,
             body_text, body_fetched, seen, flagged, deleted, answered, fetched_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(uid, folder) DO UPDATE SET
                seq = excluded.seq,
                subject = excluded.subject,
                sender = excluded.sender,
                recipients = excluded.recipients,
                date = excluded.date,
                body_text = excluded.body_text,
                body_fetched = excluded.body_fetched,
                seen = excluded.seen,
                flagged = excluded.flagged,
                deleted = excluded.deleted,
                answered = excluded.answered,
                fetched_at = excluded.fetched_at",
                params![
                    email.uid,
                    email.folder,
                    email.seq,
                    email.subject,
                    email.from,
                    email.to,
                    email.date,
                    email.body_text,
                    email.body_fetched as i32,
                    email.seen as i32,
                    email.flagged as i32,
                    email.deleted as i32,
                    email.answered as i32,
                    now,
                ],
            )
            .map_err(|e| format!("Upsert email failed: {e}"))?;
        Ok(())
    }

    /// Bulk upsert email headers (without bodies). Used for fast folder sync.
    pub fn upsert_email_headers(&self, emails: &[CachedEmail]) -> Result<(), String> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| format!("Transaction failed: {e}"))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        for email in emails {
            tx.execute(
                "INSERT INTO emails (uid, folder, seq, subject, sender, recipients, date,
                 body_text, body_fetched, seen, flagged, deleted, answered, fetched_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, '', 0, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT(uid, folder) DO UPDATE SET
                    seq = excluded.seq,
                    subject = excluded.subject,
                    sender = excluded.sender,
                    recipients = excluded.recipients,
                    date = excluded.date,
                    seen = COALESCE(excluded.seen, emails.seen),
                    flagged = COALESCE(excluded.flagged, emails.flagged),
                    deleted = COALESCE(excluded.deleted, emails.deleted),
                    answered = COALESCE(excluded.answered, emails.answered),
                    fetched_at = excluded.fetched_at",
                params![
                    email.uid,
                    email.folder,
                    email.seq,
                    email.subject,
                    email.from,
                    email.to,
                    email.date,
                    email.seen as i32,
                    email.flagged as i32,
                    email.deleted as i32,
                    email.answered as i32,
                    now,
                ],
            )
            .map_err(|e| format!("Bulk insert failed: {e}"))?;
        }

        tx.commit().map_err(|e| format!("Commit failed: {e}"))?;
        Ok(())
    }

    /// Get emails for a folder, ordered by date descending.
    pub fn get_emails(
        &self,
        folder: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<CachedEmail>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT uid, folder, seq, subject, sender, recipients, date,
                    body_text, body_fetched, seen, flagged, deleted, answered
             FROM emails
             WHERE folder = ?1
             ORDER BY date DESC
             LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| format!("Prepare failed: {e}"))?;

        let rows = stmt
            .query_map(params![folder, limit as i64, offset as i64], |row| {
                Ok(CachedEmail {
                    uid: row.get(0)?,
                    folder: row.get(1)?,
                    seq: row.get(2)?,
                    subject: row.get(3)?,
                    from: row.get(4)?,
                    to: row.get(5)?,
                    date: row.get(6)?,
                    body_text: row.get(7)?,
                    body_fetched: row.get::<_, i32>(8)? != 0,
                    seen: row.get::<_, i32>(9)? != 0,
                    flagged: row.get::<_, i32>(10)? != 0,
                    deleted: row.get::<_, i32>(11)? != 0,
                    answered: row.get::<_, i32>(12)? != 0,
                })
            })
            .map_err(|e| format!("Query failed: {e}"))?;

        let mut emails = Vec::new();
        for row in rows {
            emails.push(row.map_err(|e| format!("Row error: {e}"))?);
        }
        Ok(emails)
    }

    /// Get a single email by UID and folder.
    pub fn get_email(&self, uid: u32, folder: &str) -> Result<Option<CachedEmail>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT uid, folder, seq, subject, sender, recipients, date,
                    body_text, body_fetched, seen, flagged, deleted, answered
             FROM emails
             WHERE uid = ?1 AND folder = ?2",
            )
            .map_err(|e| format!("Prepare failed: {e}"))?;

        let mut rows = stmt
            .query_map(params![uid, folder], |row| {
                Ok(CachedEmail {
                    uid: row.get(0)?,
                    folder: row.get(1)?,
                    seq: row.get(2)?,
                    subject: row.get(3)?,
                    from: row.get(4)?,
                    to: row.get(5)?,
                    date: row.get(6)?,
                    body_text: row.get(7)?,
                    body_fetched: row.get::<_, i32>(8)? != 0,
                    seen: row.get::<_, i32>(9)? != 0,
                    flagged: row.get::<_, i32>(10)? != 0,
                    deleted: row.get::<_, i32>(11)? != 0,
                    answered: row.get::<_, i32>(12)? != 0,
                })
            })
            .map_err(|e| format!("Query failed: {e}"))?;

        if let Some(row) = rows.next() {
            Ok(Some(row.map_err(|e| format!("Row error: {e}"))?))
        } else {
            Ok(None)
        }
    }

    /// Update the body of an existing email (lazy body fetch).
    pub fn update_email_body(&self, uid: u32, folder: &str, body_text: &str) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE emails SET body_text = ?1, body_fetched = 1 WHERE uid = ?2 AND folder = ?3",
                params![body_text, uid, folder],
            )
            .map_err(|e| format!("Update body failed: {e}"))?;
        Ok(())
    }

    /// Update the seen flag for an email.
    pub fn mark_seen(&self, uid: u32, folder: &str, seen: bool) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE emails SET seen = ?1 WHERE uid = ?2 AND folder = ?3",
                params![seen as i32, uid, folder],
            )
            .map_err(|e| format!("Mark seen failed: {e}"))?;
        Ok(())
    }

    /// Update the deleted flag for an email.
    pub fn mark_deleted(&self, uid: u32, folder: &str, deleted: bool) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE emails SET deleted = ?1 WHERE uid = ?2 AND folder = ?3",
                params![deleted as i32, uid, folder],
            )
            .map_err(|e| format!("Mark deleted failed: {e}"))?;
        Ok(())
    }

    /// Update the flagged (starred) flag for an email.
    pub fn mark_flagged(&self, uid: u32, folder: &str, flagged: bool) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE emails SET flagged = ?1 WHERE uid = ?2 AND folder = ?3",
                params![flagged as i32, uid, folder],
            )
            .map_err(|e| format!("Mark flagged failed: {e}"))?;
        Ok(())
    }

    /// Get unread count for a folder.
    pub fn unread_count(&self, folder: &str) -> Result<u32, String> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM emails WHERE folder = ?1 AND seen = 0 AND deleted = 0",
                params![folder],
                |row| row.get(0),
            )
            .map_err(|e| format!("Count failed: {e}"))?;
        Ok(count as u32)
    }
}
