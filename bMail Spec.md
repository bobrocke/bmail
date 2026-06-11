# bMail

bMail is a IMAP client for Linux that is optimized for Omarchy

## Interface
- The initial display shows a list of email folders in a scrolling pane to the left and a list of emails from the selected folder in a scrolling pane to the right.
- When an email is opened, the email content replaces the initial display.
- The top of the email display shows the sender, subject, and date of the email.

## UI
- Keybindings are provided for common actions such as opening an email, deleting an email, and navigating between email folders.
- These keybindings are compatible with Omarchy's keybinding system.
- Mouse right-click will open the context command menus.

## Features
- IMAP access to a single account
- Reading email in text format with limited HTML conversion to text
- Keyboard and mouse control
- Fits perfectly into the Omarchy system, suits its style and respect its theme settings

## Commands
- Commands start with / and are context sensitive
- The / opens a drop-down menu listing all available commands in scrollable pane.
- Typing the first few letters, or scrolling to it, selects the command and 'enter' executes it.

### Context: With an email selected
- /open (o)
- /delete (d)
- /mark as read (m)
- /mark as unread (M)
- /reply (r)
- /reply all (R)
- /forward (f)
- /move to SPAM (s)
- /quit (q)

### Context: With a folder selected
- /open (o)
- /delete (d)
- /new (n)
- /rename (r)
- /quit (q)

### Context: With an email open
- /delete (d)
- /reply (r)
- /reply all (R)
- /forward (f)
- /move to SPAM (s)
- /exit (e)
- /quit (q)

## Tech Stack
- Rust
- egui
