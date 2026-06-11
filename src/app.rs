//! Main bMail application — egui app.

use std::sync::{Arc, Mutex};
use crate::config::Config;
use crate::commands::{CommandAction, CommandPalette, CommandContext};
use crate::db::{CachedEmail, CachedFolder, Database};
use crate::imap::{FolderNode, ImapSession};
use crate::smtp::SmtpSender;
use crate::theme;
use crate::html2md;
use crate::ui::folder_pane::FolderSelection;
use crate::ui::compose::ComposeState;
use crate::ui::setup::SetupState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View { Mailbox, EmailView, Compose }

pub struct BmailApp {
    config: Option<Config>,
    setup_state: SetupState,
    theme_name: String,
    view: View,
    folder_tree: Vec<FolderNode>,
    folder_cache: Vec<CachedFolder>,
    selected_folder: Option<FolderSelection>,
    selected_email_uid: Option<u32>,
    emails: Vec<CachedEmail>,
    open_email: Option<CachedEmail>,
    compose_state: Option<ComposeState>,
    command_palette: CommandPalette,
    connected: bool,
    connection_error: Option<String>,
    loading_emails: bool,
    runtime: tokio::runtime::Runtime,
    db: Option<Arc<Mutex<Database>>>,
    smtp: Option<Arc<SmtpSender>>,
    show_debug: bool,
    last_theme_check: std::time::Instant,
    theme_signal_mtime: Option<std::time::SystemTime>,
}

impl BmailApp {
    pub fn new() -> Self {
        let theme_name = theme::detect_current_theme().unwrap_or_else(|| "catppuccin".to_string());
        let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().expect("tokio");
        theme::install_theme_hook().ok();
        let mtime = std::fs::metadata(theme::theme_signal_path()).ok().and_then(|m| m.modified().ok());
        Self {
            config: Config::load(), setup_state: SetupState::default(), theme_name,
            view: View::Mailbox, folder_tree: vec![], folder_cache: vec![],
            selected_folder: None, selected_email_uid: None, emails: vec![],
            open_email: None, compose_state: None,
            command_palette: CommandPalette::new(CommandContext::EmailSelected),
            connected: false, connection_error: None, loading_emails: false,
            runtime, db: None, smtp: None, show_debug: false,
            last_theme_check: std::time::Instant::now(), theme_signal_mtime: mtime,
        }
    }
    fn rt(&self) -> &tokio::runtime::Runtime { &self.runtime }
}

impl eframe::App for BmailApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Theme poll
        if self.last_theme_check.elapsed() > std::time::Duration::from_secs(2) {
            self.last_theme_check = std::time::Instant::now();
            if let Ok(meta) = std::fs::metadata(theme::theme_signal_path()) {
                if let Ok(mtime) = meta.modified() {
                    if self.theme_signal_mtime.is_none_or(|p| mtime > p) {
                        self.theme_signal_mtime = Some(mtime);
                        let new = theme::detect_current_theme().unwrap_or_else(|| self.theme_name.clone());
                        if new != self.theme_name { self.theme_name = new; theme::apply_theme(ctx, &self.theme_name); }
                    }
                }
            }
        }

        // First-run setup
        if self.config.is_none() {
            // Process test connection request (synchronous, ~200ms)
            if self.setup_state.testing {
                self.setup_state.testing = false;
                let imap_cfg = self.setup_state.to_config().imap.clone();
                let result = self.rt().block_on(async {
                    match ImapSession::connect(&imap_cfg).await {
                        Ok(s) => { let _ = s.logout().await; Ok(()) }
                        Err(e) => Err(format!("Connection failed: {e}")),
                    }
                });
                self.setup_state.test_result = Some(result);
            }

            if let Some(config) = crate::ui::setup::render(ctx, &mut self.setup_state) {
                let _ = config.save();
                self.config = Some(config.clone());
                self.db = Database::open().ok().map(|d| Arc::new(Mutex::new(d)));
                self.smtp = SmtpSender::new(&config.smtp).ok().map(Arc::new);
            }
            ctx.request_repaint();
            return;
        }

        // Init
        if self.db.is_none() {
            self.db = Database::open().ok().map(|d| Arc::new(Mutex::new(d)));
            if let Some(ref db) = self.db {
                if let Ok(c) = db.lock().unwrap().get_folders() { self.folder_cache = c; }
            }
        }
        let config = self.config.clone().unwrap();
        if self.smtp.is_none() { self.smtp = SmtpSender::new(&config.smtp).ok().map(Arc::new); }

        // Auto-connect
        if !self.connected && self.folder_tree.is_empty() {
            self.connected = true;
            let imap_cfg = config.imap.clone();
            self.rt().spawn(async move {
                if let Ok(mut s) = ImapSession::connect(&imap_cfg).await {
                    if let Ok(f) = s.list_folders().await { log::info!("{} IMAP folders", f.len()); }
                    let _ = s.logout().await;
                }
            });
        }

        handle_shortcuts(ctx, self);
        render_ui(ctx, self);

        let cmd = crate::ui::command_menu::render(ctx, &mut self.command_palette);
        if let Some(a) = cmd { execute(self, a); }

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if self.connected { ui.label("Connected"); }
                else if let Some(ref e) = self.connection_error { ui.colored_label(ui.visuals().error_fg_color, e); }
                if self.loading_emails { ui.add(egui::Spinner::new()); }
            });
        });
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }
}

fn render_ui(ctx: &egui::Context, app: &mut BmailApp) {
    match app.view {
        View::Mailbox => {
            egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("bMail"); ui.separator();
                    if ui.button("↻ Refresh").clicked() {
                        if let Some(ref f) = app.selected_folder.clone() { load_folder(app, &f.path, &f.name); }
                    }
                    if ui.button("✎ Compose").clicked() {
                        app.compose_state = Some(ComposeState {
                            to: String::new(), subject: String::new(), body: String::new(),
                            is_reply: false, is_forward: false, in_reply_to: None,
                            sending: false, error: None, sent: false,
                        });
                        app.view = View::Compose;
                    }
                    ui.separator();
                    if ui.button("?").clicked() { app.show_debug = !app.show_debug; }
                    if app.show_debug { ui.label(format!("Theme: {}", app.theme_name)); }
                });
            });

            let sel = app.selected_folder.clone();
            let uid = app.selected_email_uid;

            egui::SidePanel::left("folders").resizable(true).default_width(200.0).min_width(120.0)
                .show(ctx, |ui| {
                    let tree = app.folder_tree.clone();
                    let cache = app.folder_cache.clone();
                    crate::ui::folder_pane::render(ui, &tree, &cache, &sel, &mut |p, n| {
                        load_folder(app, &p, &n);
                    });
                });

            egui::CentralPanel::default().show(ctx, |ui| {
                let emails = app.emails.clone();
                let mut open_target = None;
                crate::ui::email_list::render(ui, &emails, uid,
                    &mut |u| { app.selected_email_uid = Some(u); },
                    &mut |u| { open_target = Some(u); });
                if let Some(u) = open_target {
                    app.selected_email_uid = Some(u);
                    open_email(app, u);
                }
            });
        }
        View::EmailView => {
            let email = app.open_email.clone();
            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(ref e) = email {
                    crate::ui::email_view::render(ui, e, &mut || {
                        app.view = View::Mailbox;
                        app.open_email = None;
                    });
                }
            });
        }
        View::Compose => {
            let smtp = app.smtp.clone();
            let identity = app.config.as_ref().map(|c| c.identity.clone());
            let rt = app.rt().handle().clone();
            let mut editing = app.compose_state.take();

            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(ref mut s) = editing {
                    let mut do_send = false;
                    let mut do_cancel = false;

                    crate::ui::compose::render(ui, s, &mut || { do_send = true; }, &mut || { do_cancel = true; });

                    if do_send {
                        if let (Some(ref smtp), Some(ref id)) = (&smtp, &identity) {
                            let to = s.to.clone();
                            let subject = s.subject.clone();
                            let body = s.body.clone();
                            let name = id.display_name.clone();
                            let email = id.email.clone();
                            let sender = smtp.clone();
                            rt.spawn(async move {
                                let _ = sender.send_reply((&name, &email), &to, &subject, &body).await;
                            });
                        }
                        s.sent = true;
                    } else if do_cancel {
                        // abandon
                    }
                }
            });

            if let Some(s) = editing {
                if !s.sent { app.compose_state = Some(s); }
            }
        }
    }
}

fn load_folder(app: &mut BmailApp, path: &str, name: &str) {
    app.selected_folder = Some(FolderSelection { path: path.to_string(), name: name.to_string() });
    app.selected_email_uid = None;
    app.emails.clear();
    app.loading_emails = true;

    if let Some(ref db) = app.db {
        if let Ok(c) = db.lock().unwrap().get_emails(path, 100, 0) {
            if !c.is_empty() { app.emails = c; app.loading_emails = false; }
        }
    }

    if let Some(ref c) = app.config {
        let imap_cfg = c.imap.clone();
        let folder = path.to_string();
        let db = app.db.clone();
        app.rt().spawn(async move {
            if let Ok(mut s) = ImapSession::connect(&imap_cfg).await {
                if s.examine_folder(&folder).await.is_ok() {
                    if let Ok(envs) = s.fetch_envelopes(1, 50).await {
                        let emails: Vec<CachedEmail> = envs.iter().map(|e| CachedEmail {
                            uid: e.uid, folder: folder.clone(), seq: e.seq,
                            subject: e.subject.clone(), from: e.from.clone(),
                            to: e.to.clone(), date: e.date.clone(),
                            body_text: String::new(), seen: e.seen,
                            flagged: e.flagged, deleted: e.deleted,
                            answered: e.answered, body_fetched: false,
                        }).collect();
                        if let Some(ref d) = db { let _ = d.lock().unwrap().upsert_email_headers(&emails); }
                    }
                }
                let _ = s.logout().await;
            }
        });
    }
    app.command_palette = CommandPalette::new(CommandContext::EmailSelected);
}

fn open_email(app: &mut BmailApp, uid: u32) {
    let folder_guard = app.selected_folder.clone();
    let db_guard = app.db.as_ref().map(|d| d.lock().unwrap());
    if let (Some(ref folder), Some(ref db)) = (&folder_guard, &db_guard) {
        if let Ok(Some(email)) = db.get_email(uid, &folder.path) {
            let was_unread = !email.seen;
            let need_body = !email.body_fetched;
            drop(db_guard);

            if was_unread {
                if let Some(ref db) = app.db { let _ = db.lock().unwrap().mark_seen(uid, &folder.path, true); }
            }
            if need_body && app.config.is_some() {
                fetch_body(app, uid, &folder.path);
            }
            app.open_email = Some(email);
            app.view = View::EmailView;
            app.command_palette = CommandPalette::new(CommandContext::EmailOpen);
        }
    }
}

fn fetch_body(app: &mut BmailApp, uid: u32, folder: &str) {
    if let Some(ref c) = app.config {
        let imap_cfg = c.imap.clone();
        let folder = folder.to_string();
        let db = app.db.clone();
        app.rt().spawn(async move {
            if let Ok(mut s) = ImapSession::connect(&imap_cfg).await {
                if s.select_folder(&folder).await.is_ok() {
                    if let Ok(Some(body)) = s.fetch_body(uid).await {
                        let md = html2md::html_to_markdown(&body);
                        if let Some(ref d) = db { let _ = d.lock().unwrap().update_email_body(uid, &folder, &md); }
                    }
                }
                let _ = s.logout().await;
            }
        });
    }
}

fn execute(app: &mut BmailApp, action: CommandAction) {
    match action {
        CommandAction::QuitApp => std::process::exit(0),
        CommandAction::ExitEmail => { app.view = View::Mailbox; app.open_email = None; }
        CommandAction::OpenEmail => { if let Some(u) = app.selected_email_uid { open_email(app, u); } }
        CommandAction::DeleteEmail => {
            if let (Some(u), Some(ref f)) = (app.selected_email_uid, &app.selected_folder) {
                if let Some(ref d) = app.db { let _ = d.lock().unwrap().mark_deleted(u, &f.path, true); }
                app.emails.retain(|e| e.uid != u);
                app.selected_email_uid = None;
            }
        }
        CommandAction::MarkRead => {
            if let Some(u) = app.selected_email_uid {
                if let Some(ref f) = app.selected_folder {
                    if let Some(ref d) = app.db { let _ = d.lock().unwrap().mark_seen(u, &f.path, true); }
                }
                for e in &mut app.emails { if e.uid == u { e.seen = true; } }
            }
        }
        CommandAction::MarkUnread => {
            if let Some(u) = app.selected_email_uid {
                if let Some(ref f) = app.selected_folder {
                    if let Some(ref d) = app.db { let _ = d.lock().unwrap().mark_seen(u, &f.path, false); }
                }
                for e in &mut app.emails { if e.uid == u { e.seen = false; } }
            }
        }
        CommandAction::Reply => {
            if let Some(ref e) = app.open_email {
                app.compose_state = Some(ComposeState::reply(&e.from, &e.subject, &e.body_text));
                app.view = View::Compose;
            }
        }
        CommandAction::ReplyAll => {
            if let Some(ref e) = app.open_email {
                let rcpts = if e.to.is_empty() { e.from.clone() } else { format!("{}, {}", e.from, e.to) };
                app.compose_state = Some(ComposeState::reply(&rcpts, &e.subject, &e.body_text));
                app.view = View::Compose;
            }
        }
        CommandAction::Forward => {
            if let Some(ref e) = app.open_email {
                app.compose_state = Some(ComposeState::forward(&e.from, &e.date, &e.subject, &e.body_text));
                app.view = View::Compose;
            }
        }
        _ => {}
    }
}

fn handle_shortcuts(ctx: &egui::Context, app: &mut BmailApp) {
    if app.command_palette.visible || ctx.wants_keyboard_input() { return; }
    ctx.input(|input| {
        for event in &input.events {
            match event {
                egui::Event::Key { key: egui::Key::Escape, pressed: true, .. } => {
                    if app.view == View::EmailView { app.view = View::Mailbox; app.open_email = None; }
                    else if app.view == View::Compose { app.view = View::Mailbox; app.compose_state = None; }
                }
                egui::Event::Key { key: egui::Key::J, pressed: true, .. } if !input.modifiers.ctrl => {
                    if app.view == View::Mailbox && !app.emails.is_empty() {
                        let i = app.selected_email_uid.and_then(|u| app.emails.iter().position(|e| e.uid == u)).unwrap_or(0);
                        app.selected_email_uid = Some(app.emails[(i + 1).min(app.emails.len() - 1)].uid);
                    }
                }
                egui::Event::Key { key: egui::Key::K, pressed: true, .. } if !input.modifiers.ctrl => {
                    if app.view == View::Mailbox && !app.emails.is_empty() {
                        let i = app.selected_email_uid.and_then(|u| app.emails.iter().position(|e| e.uid == u)).unwrap_or(0);
                        app.selected_email_uid = Some(app.emails[i.saturating_sub(1)].uid);
                    }
                }
                egui::Event::Key { key: egui::Key::Enter, pressed: true, .. } if !input.modifiers.shift => {
                    if app.view == View::Mailbox { if let Some(u) = app.selected_email_uid { open_email(app, u); } }
                }
                egui::Event::Text(t) if t == "/"
                    && app.view == View::Mailbox => { app.command_palette.open(); }
                _ => {}
            }
        }
    });
}
