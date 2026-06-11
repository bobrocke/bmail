//! Context-sensitive command system.
//!
//! Commands start with `/` and open a scrollable drop-down. Typing filters
//! commands by prefix. Enter executes the selected command.
//!
//! Three contexts:
//! - `EmailSelected`: when an email is highlighted in the list
//! - `FolderSelected`: when a folder is highlighted in the tree
//! - `EmailOpen`: when viewing a full email

/// The context in which commands are being executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandContext {
    EmailSelected,
    FolderSelected,
    EmailOpen,
}

/// A command definition.
#[derive(Debug, Clone)]
pub struct Command {
    /// The full command name (e.g. "open").
    pub name: &'static str,
    /// Shortcut key (single character, shown in parens after the command).
    pub shortcut: Option<char>,
    /// Help text describing what the command does.
    pub description: &'static str,
    /// The action ID this command triggers.
    pub action: CommandAction,
}

/// Actions that can be triggered by commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandAction {
    // Email actions
    OpenEmail,
    DeleteEmail,
    MarkRead,
    MarkUnread,
    Reply,
    ReplyAll,
    Forward,
    MoveToSpam,

    // Folder actions
    OpenFolder,
    DeleteFolder,
    NewFolder,
    RenameFolder,

    // Navigation
    ExitEmail, // close email view
    QuitApp,
}

impl Command {
    /// Get all commands available for a given context.
    pub fn for_context(context: CommandContext) -> Vec<Command> {
        match context {
            CommandContext::EmailSelected => vec![
                Command {
                    name: "open",
                    shortcut: Some('o'),
                    description: "Open the selected email",
                    action: CommandAction::OpenEmail,
                },
                Command {
                    name: "delete",
                    shortcut: Some('d'),
                    description: "Delete the selected email",
                    action: CommandAction::DeleteEmail,
                },
                Command {
                    name: "mark as read",
                    shortcut: Some('m'),
                    description: "Mark the selected email as read",
                    action: CommandAction::MarkRead,
                },
                Command {
                    name: "mark as unread",
                    shortcut: Some('M'),
                    description: "Mark the selected email as unread",
                    action: CommandAction::MarkUnread,
                },
                Command {
                    name: "reply",
                    shortcut: Some('r'),
                    description: "Reply to the sender",
                    action: CommandAction::Reply,
                },
                Command {
                    name: "reply all",
                    shortcut: Some('R'),
                    description: "Reply to all recipients",
                    action: CommandAction::ReplyAll,
                },
                Command {
                    name: "forward",
                    shortcut: Some('f'),
                    description: "Forward the email",
                    action: CommandAction::Forward,
                },
                Command {
                    name: "move to SPAM",
                    shortcut: Some('s'),
                    description: "Move the email to the SPAM folder",
                    action: CommandAction::MoveToSpam,
                },
                Command {
                    name: "quit",
                    shortcut: Some('q'),
                    description: "Quit bMail",
                    action: CommandAction::QuitApp,
                },
            ],
            CommandContext::FolderSelected => vec![
                Command {
                    name: "open",
                    shortcut: Some('o'),
                    description: "Open the selected folder",
                    action: CommandAction::OpenFolder,
                },
                Command {
                    name: "delete",
                    shortcut: Some('d'),
                    description: "Delete the selected folder",
                    action: CommandAction::DeleteFolder,
                },
                Command {
                    name: "new",
                    shortcut: Some('n'),
                    description: "Create a new folder",
                    action: CommandAction::NewFolder,
                },
                Command {
                    name: "rename",
                    shortcut: Some('r'),
                    description: "Rename the selected folder",
                    action: CommandAction::RenameFolder,
                },
                Command {
                    name: "quit",
                    shortcut: Some('q'),
                    description: "Quit bMail",
                    action: CommandAction::QuitApp,
                },
            ],
            CommandContext::EmailOpen => vec![
                Command {
                    name: "delete",
                    shortcut: Some('d'),
                    description: "Delete this email",
                    action: CommandAction::DeleteEmail,
                },
                Command {
                    name: "reply",
                    shortcut: Some('r'),
                    description: "Reply to the sender",
                    action: CommandAction::Reply,
                },
                Command {
                    name: "reply all",
                    shortcut: Some('R'),
                    description: "Reply to all recipients",
                    action: CommandAction::ReplyAll,
                },
                Command {
                    name: "forward",
                    shortcut: Some('f'),
                    description: "Forward this email",
                    action: CommandAction::Forward,
                },
                Command {
                    name: "move to SPAM",
                    shortcut: Some('s'),
                    description: "Move this email to SPAM",
                    action: CommandAction::MoveToSpam,
                },
                Command {
                    name: "exit",
                    shortcut: Some('e'),
                    description: "Close this email and return to the list",
                    action: CommandAction::ExitEmail,
                },
                Command {
                    name: "quit",
                    shortcut: Some('q'),
                    description: "Quit bMail",
                    action: CommandAction::QuitApp,
                },
            ],
        }
    }

    /// Filter commands by a search string (case-insensitive prefix match).
    pub fn filter<'a>(commands: &'a [Command], query: &str) -> Vec<&'a Command> {
        if query.is_empty() {
            return commands.iter().collect();
        }
        let lower = query.to_lowercase();
        commands
            .iter()
            .filter(|c| c.name.to_lowercase().starts_with(&lower))
            .collect()
    }
}

/// The command palette state.
#[derive(Debug, Clone)]
pub struct CommandPalette {
    pub visible: bool,
    pub query: String,
    pub context: CommandContext,
    pub selected_index: usize,
    pub filtered_commands: Vec<Command>,
}

impl CommandPalette {
    pub fn new(context: CommandContext) -> Self {
        let all = Command::for_context(context);
        Self {
            visible: false,
            query: String::new(),
            context,
            selected_index: 0,
            filtered_commands: all,
        }
    }

    /// Open the command palette.
    pub fn open(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected_index = 0;
        self.filtered_commands = Command::for_context(self.context);
    }

    /// Close the command palette.
    pub fn close(&mut self) {
        self.visible = false;
        self.query.clear();
        self.selected_index = 0;
    }

    /// Update filter based on current query.
    pub fn update_filter(&mut self) {
        let all = Command::for_context(self.context);
        let query = self.query.clone();
        self.filtered_commands = Command::filter(&all, &query).into_iter().cloned().collect();
        if self.selected_index >= self.filtered_commands.len() {
            self.selected_index = self.filtered_commands.len().saturating_sub(1);
        }
    }

    /// Push a character to the query (or start the palette with '/').
    pub fn push_char(&mut self, c: char) {
        if !self.visible {
            if c == '/' {
                self.open();
            }
            return;
        }
        self.query.push(c);
        self.update_filter();
    }

    /// Delete last character from query.
    pub fn backspace(&mut self) {
        if self.query.pop().is_some() {
            self.update_filter();
        }
    }

    /// Move selection up.
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if self.selected_index + 1 < self.filtered_commands.len() {
            self.selected_index += 1;
        }
    }

    /// Get the currently selected command action (if any).
    pub fn selected_action(&self) -> Option<CommandAction> {
        if self.filtered_commands.is_empty() {
            return None;
        }
        // If there's exactly one match and Enter is pressed, select it
        // even if selected_index points to a different one (auto-select
        // when filter narrows to one).
        if self.filtered_commands.len() == 1 {
            return Some(self.filtered_commands[0].action.clone());
        }
        if self.selected_index < self.filtered_commands.len() {
            return Some(self.filtered_commands[self.selected_index].action.clone());
        }
        None
    }
}
