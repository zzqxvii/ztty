# Ztty PRD

A terminal in Rust, GPUI, Ely components. It draws a terminal as well as Ghostty does, shows every setting on screen, and makes no noise. It is modified from ELYTerminal and built on its component library, and the projects and worktrees column follows Orca.

macOS and Linux are not yet verified, and nothing here claims them: this PRD covers Windows.

Chapters run in build order. `TASKS.md` follows them line by line.

## 1. Product

- One window. Projects and their worktrees on the left, the worktree's tabs in the middle, Git or Files on the right.
- Every setting has a control in the settings window. No setting lives only in a file.
- Silent: no sound, no bell, no notifications but a session's, below, no update checks, no telemetry, no tips, no onboarding.
- Clean: no subtitle, no tagline, no logging options, no status bar, no badges that do not carry state.
- Built-in themes, each shown as a small preview, Catppuccin among them. A theme colors the terminal and the chrome.
- The window background blurs on Windows, through acrylic.

## 2. Foundation

- One binary crate, `ztty`, edition 2024, Rust 1.95.
- Ely from github.com/ELY-Open-Source by pinned rev. gpui, gpui_platform from crates.io `gpui-pre` at the exact version Ely pins.
- `alacritty_terminal` 0.26 for the grid, the parser and the pseudo-terminal, as Ely uses it.
- Logs through `log` to stderr, `RUST_LOG` sets the level. No log setting in the app.
- The window's text, the menu bar and Ely's own components speak English or Simplified Chinese, through Ely's `I18n`, one catalog per language in `src/i18n`; the settings hold the locale, and the first start takes what the system asks for.
- CI on GitHub Actions: Windows builds and tests, and Ubuntu runs `scripts/check.sh` and builds.
- Repo `zzqxvii/ztty` on GitHub, public; it lived on git.zacharyzhang.com until v0.1.7. No credential in any file or commit.

## 3. Settings model and storage

- One JSON file, `settings.json`, in the platform config folder (`dirs::config_dir()/Ztty`).
- Every field has a default. A missing file means defaults. A missing field means its default.
- A file that does not parse, or a value out of range, stops the start with an error naming the file and the field. The app never overwrites a file it could not read.
- Writes go to a temp file in the same folder, then rename. One writer: the app's settings entity.
- `projects` lists the left column's folders as full paths, in the user's order; a relative path, or a path listed twice, stops the start; a folder that is gone is kept, marked in its row.
- Fields: see chapter 10.

## 4. Themes

- Built in: Ely Dark, Ely Light, Catppuccin Mocha, Catppuccin Macchiato, Catppuccin Frappé, Catppuccin Latte, Tokyo Night, Dracula, Nord, Gruvbox Dark, Rosé Pine, Solarized Light, One Light, Kanagawa Wave.
- A theme holds background, foreground, cursor, cursor text, selection background and foreground, and the 16 ANSI colors. Values come from the theme's upstream files (iTerm2-Color-Schemes' Ghostty export, or Ely's palette).
- A theme maps onto Ely's `Palette` through `Theme::set_palette`, so panels, inputs and lists wear the theme.
- Dark mode: one theme, or a second theme for when the system is dark.
- On Omarchy 4 (Linux), an Omarchy theme wears Omarchy's current theme and changes with each switch. It is the default where Omarchy runs; picking another theme stops following.

## 5. Terminal engine

Ghostty is the bar for what the terminal does. Ztty owns its engine on `alacritty_terminal`; Ely's `Terminal` is a gallery widget without mouse reporting, IME, kitty keys or style settings.

- PTY: alacritty's `tty` and event loop. The pty is wrapped so its reader passes bytes through an OSC scanner. The scanner reports the working directory and prompt marks. Output reaches the screen as the program writes it, with no key to bring it.
- Shell integration for zsh, bash, fish and PowerShell, injected at launch as Ghostty does: each prompt sends OSC 7 and a title. On Windows a Shell of `wsl` runs the distribution's bash, zsh or fish with that integration, and its folder reaches Files and Git as Windows names it, git running inside the distribution. Git for Windows' bash, its `sh` too, reports its folder as Windows names it. A shell without integration reports nothing; the right column still shows its worktree's root.
- Prompt marks, OSC 133: the shell integration marks where each prompt starts (A) and ends (B), where a command starts (C), and where it ends with its exit status (D), all but C on macOS's bash 3.2, which has no hook for it; the terminal reads any program's OSC 133. Cmd-Up and Cmd-Down, and Cmd-Shift-Up and Cmd-Shift-Down, on macOS, Ctrl-Shift-Up and Ctrl-Shift-Down elsewhere, scroll to the prompt before or after the view's top, as in Ghostty.
- Program status, OSC 7501 revision 0.4: each terminal keeps the records programs report (state, id, kind, progress, app, title, msg), checked as the spec checks them; a shell's new prompt drops working, blocked and idle records, while a prompt a running program marks itself, as omp does, keeps those it reported; a full reset drops them all; the terminal answers the support query with the states and kinds it takes.
- Progress, OSC 9;4: set, error, indeterminate and pause stand as the root record until a program reports OSC 7501, until a full reset.
- Notifications: OSC 9 (iTerm2), OSC 777 notify (rxvt) and OSC 99 (kitty: title and body in chunks, base64, ids, focus and report on activation, close, expiry, which ones live, and its query; `o` passes by, since a session posts only while unseen, below); ConEmu's other OSC 9 commands are no notifications.
- Inside herdr, which keeps its panes' OSC 7, the session's folder is that of herdr's focused pane, read from herdr's socket while herdr's client runs in the session, on macOS, Linux and Windows.
- Inside herdr, which passes no OSC 7501 on, herdr's agent states (idle, working, blocked, done) are the session's status records.
- Grid: colors (named, 256, truecolor, OSC 4/10/11/12), bold, dim, italic, underline (single, double, curly, dotted, dashed), strikethrough, inverse, hidden, wide characters, combining marks. An erase, insert or delete that splits a wide character clears all of it, as xterm does.
- Box drawing (U+2500–U+257F), block elements (U+2580–U+259F) and powerline shapes (U+E0B0–U+E0B7) painted as geometry, so lines and segments join across cells.
- Fonts: JetBrains Mono by default, any installed font by choice, and fallback families tried in order for a character it lacks, a Chinese font after a Latin one among them; Nerd Font icons the font lacks come from a bundled Nerd Font.
- Cursor: block, bar, underline, hollow block; blinking per setting and per DECSCUSR; hollow when the window is not focused.
- Keys: xterm encoding with modifiers, application cursor mode, Option as Alt on macOS, and natural text editing there when the setting is on: Option-Left and Option-Right go as ESC b and ESC f, Option-Backspace and Option-Delete as ESC DEL and ESC d, Command-Left and Command-Right as Ctrl-A and Ctrl-E, Command-Backspace and Command-Delete as Ctrl-U and Ctrl-K, as iTerm2's preset; the kitty keyboard protocol as the program sets its flags; a key Shift makes a symbol of goes as its own key with Shift, the symbol its alternate, as Ghostty sends it, on macOS and Windows. On Windows keys go as win32-input-mode records while ConPTY asks, as Windows Terminal sends them. Keypad keys type their characters, as Ghostty's do with mode 1035 on by default.
- IME: marked text drawn at the cursor; committed text sent to the program.
- Mouse: select by character, word (double press) and line (triple press); Shift extends; mouse reporting (X10, normal, button, any; SGR and UTF-8); wheel scrolls back, or reports, or sends arrows on the alternate screen.
- Click to move: while the shell reads a line, as its OSC 133 marks say, a plain click on that line, past its prompt, moves the shell's cursor there, as Ghostty's `cursor-click-to-move`: a shell that takes clicks, as fish does, hears the click; others get arrow keys.
- Clipboard: copy, paste (bracketed when asked), copy on select per setting, OSC 52 writes.
- Drop: files dropped on the terminal type their paths at the cursor, as macOS Terminal does, each kept literal for the session's shell and followed by a space.
- Links: OSC 8 and plain URLs; Cmd-press (Ctrl-press off macOS) opens them. Inside herdr, while it takes the mouse, herdr opens them, web links only, as herdr does, whole where its pane wraps them, and underlines them while the key is held; with Shift the terminal opens what it sees.
- Focus reporting, title (OSC 0/2), scrollback limit per setting, find in scrollback (Ely's `FindWidget`).
- A scrollbar over the history, along the terminal's right edge: a drag on its thumb scrolls, whatever the program does with the wheel.
- Bell: no sound. The session row marks it until the session is shown.

## 6. Main window and layout

- Ely's `ColumnShell`: sessions column, terminal column, right column, in either order: a setting puts the projects column on the right and Files and Git on the left, each column's toggle and the window's own buttons following its edge. The window opens with blur when the setting is on.
- The sessions column and the right column toggle, each by a button at its end of the title bar or by key, and keep their state in settings, so the window opens as it was left. A column that hides leaves its button in place: the sessions' follows the window's own controls into the first column's header, before the tabs, and the right column's ends the last column's header, clear of the window's buttons.
- The sessions column and the right column resize by a drag on their border toward the terminal, on macOS, Windows and Linux, from 160 to 960 pixels; the widths are kept in settings once a drag rests, so the window opens with them.
- The window opens at the size it was left at, on macOS, Windows and Linux: the size is kept in settings once it settles; a maximized or full-screen window keeps the size from before; a kept size past the screen shrinks to it; the window is no smaller than 100 by 100, the least the settings keep.
- The title shows the active tab's title, and no other text.
- The pointer over the terminal is the arrow.
- The title bar is transparent, as Ghostty's: the window's background, opacity and blur run under it, so it wears the terminal's color.
- Every control in the title bar takes its own press on every platform, the window buttons Ely draws on Windows and Linux too.

## 7. Projects, worktrees and tabs

- The left column lists projects, not sessions. A project is a folder the user added, kept in `settings.json` in `projects`, in the user's order.
- Under a project, its worktrees: from `git -C <project> worktree list --porcelain`, git's order, the main checkout marked `default` in a project of more than one; a folder git does not know stands as one worktree, itself. Worktrees are made and removed only when the column's own buttons ask.
- A project row is a folder icon, open or closed as its worktrees show, and its name in the bold weight; a press folds or unfolds it, and two buttons on hover leave it alone: one removes it after a question, nothing on disk deleted and its terminals closed, and one, on a repository, makes a worktree of it. No label stands over the rows.
- Making a worktree asks for a branch name, `wt-1` offered and selected; the worktree stands beside the main checkout, named after it and the branch with each slash a dash, on a new branch cut from the branch the project's own folder has checked out. A name git would refuse is held back under the field; git's own refusal shows at the top of the column until dismissed. The new worktree shows with a terminal in it.
- Removing a worktree asks first: its terminals close, `git worktree remove` deletes its folder, and its branch stays. git refuses one that holds changes or a lock, and its words show at the top of the column until dismissed; the main checkout and a folder git does not know take no such button.
- A worktree row is a card stepped in under its project: a dot, or the mark of the terminal waiting hardest, then its branch, or the folder's name while it has none, the `default` badge on the main checkout of a project of several, and a remove button on hover of every card but the main checkout's; under the name a muted line names its folder, and while a program reports, one more says what it is doing. The card shown is bordered and filled; a hidden terminal that rings or starts waiting makes its name strong.
- Add project opens the platform's folder picker; a path that is not a readable folder, or is already listed, is refused with a reason. A project whose folder is gone at start stays in the settings with the reason in its row.
- Selecting a worktree shows its tabs, the one it showed last, and opens a terminal in its root the first time.
- The middle column holds the worktree's tabs: terminals, read-only files and diffs, in the window's top row beside the drag space, the shown one marked by a line under it, each closable, and draggable. A strip too long for its room scrolls with the wheel and brings the shown tab into view; a + after the strip opens a terminal tab, and stays in reach while the strip scrolls.
- New terminal tab opens the Shell setting's program, in home or the shown terminal's folder as Start in says; a worktree's first terminal starts in the worktree's root.
- A file tab shows a UTF-8 text file read-only, in the Files font and size, with Ely's code editor; a binary file says so, a file over 2 MiB shows its first 2 MiB with a line saying so, and a failure says why. Opening the same path shows the tab it has, and every file tab is read again when the window comes back active.
- A diff tab shows a change's diff from the Git panel in Ely's diff viewer, unified or split, and reads it again after each Git action.
- Closing the app's last terminal closes the window and the app.
- Keys: Cmd-T and Cmd-W off macOS Ctrl-Shift-T and Ctrl-Shift-W open and close a tab; Cmd-Shift-] and Cmd-Shift-[ off macOS Ctrl-Shift-] and Ctrl-Shift-[ step through them; Ctrl-Tab and Ctrl-Shift-Tab step through the worktrees; Cmd-1…9 or Alt-1…9 show the first nine.
- The window's title is the active tab's title, else the worktree's name.
- The status marks, their hover, the notifications, Ely's mood and the window title read every terminal of every worktree, as they read every session before.

## 8. Right panel: Files

- Files and Git are two icons at the column's foot, the shown one in the text's color and the other muted, so they stay findable without drawing the eye; Files is the default, and the choice is kept in the settings. Rows and lists sit a step closer than Ely's default: the app wears Ely's compact density.
- Files shows the shown worktree's root as a tree of its own: the root's own entries at the top, no row for the worktree itself, since the projects column already names it; folders first, Git's marks on names, a press folding a folder or opening a file, and each level stepping in 12 pixels.
- The panel follows the worktree, not its tabs: a terminal's `cd`, a file tab and a diff tab move neither the tree nor its marks.
- A press on a folder opens it; a press on a file opens it in a read-only tab in the middle.
- It reloads at each prompt of the shown terminal, after each Git action, and when another worktree is shown.

## 9. Right panel: Git

- Through the `git` command line. No libgit2.
- Shows the branch, staged and unstaged changes, a commit message field and Commit, and recent commits.
- Stage and unstage a file. Press a file to see its diff in Ely's `DiffViewer`, in a tab of the middle, unified or split.
- Discard, which Ely's changes list draws on every unstaged and untracked row, asks first: an edit goes back to the index, a new file is deleted.
- Outside a repository it says so. A failed git command shows its error.
- The column shows one of three views, picked by the icons at its foot: Files, git's changes — the branch, what changed, the commit field — and git's commits, the last 50 on the branch.
- It reloads at each prompt of the shown terminal, and when another worktree is shown.

## 10. Settings window

Cmd-, opens it; on Windows and Linux, which have no menu bar, so does a gear at the foot of the projects column, left of Add project. Eight categories on the left, rows on the right, each side scrolling on its own, Ely's components. Changes apply at once to every session and save.

- General
  - Language: English or 中文; the first start follows the system's language, and the menus, the window's text and Ely's own components change with it at once.
  - Check for updates automatically: a switch, on by default, which checks at start and once a day.
  - Updates: what the last check found, and Check now; a newer release brings Download, which opens its page, and one an automatic check finds posts a notification.
- Appearance
  - Theme: a grid of previews; on Omarchy, the Omarchy theme first.
  - Separate dark theme: a switch; when on, a second grid for dark mode.
- Fonts
  - Terminal: the family, the fallbacks tried in order for a character it lacks, size, line height and ligatures.
  - Files: the family and size the file tabs draw in, apart from the terminal's.
- Cursor: style (block, bar, underline, hollow), blink, with a live preview.
- Terminal
  - Shell: a path; empty runs the login shell, on Windows `pwsh`, else `powershell`, else `cmd`. On Windows a session takes the registry's environment as it starts.
  - Start in: Home, or the active session's folder.
  - Scrollback lines.
- Input
  - Option as Alt: off, left, right, both (macOS only).
  - Natural text editing: a switch, off by default (macOS only); Option and Command edit by word and line, as the shell expects.
  - Copy on select.
- Window
  - Background opacity: a slider. Background blur: a switch.
  - Padding: horizontal and vertical.
  - Layout: Projects on the right, which swaps the side columns.
  - Ely: a switch.
- Shortcuts: every shortcut, as this platform's keys, with a search.

## 11. Background blur

- macOS: gpui's `WindowBackgroundAppearance::Blurred` (NSVisualEffectView).
- Windows: gpui's `Blurred` (acrylic accent). On Windows 10, whose DWM drags an acrylic window seconds behind the pointer, the glass clears while the window moves or sizes and blurs again when it rests; Windows 11 keeps its acrylic throughout.
- Wayland: gpui's `Blurred` speaks `org_kde_kwin_blur`; Ztty adds `ext_background_effect_v1` on the window's surface, which GNOME 51, Plasma 6.7 and Hyprland support. Hyprland honors it only while its own blur is on, which Omarchy 4.0.4 turns off.
- X11: Ztty sets `_KDE_NET_WM_BLUR_BEHIND_REGION` (CARDINAL, 32) on the window, as Ghostty does; removes it when blur turns off.
- A compositor with neither protocol shows transparency without blur. The app logs which path it took.

## 12. Menus and shortcuts

- macOS menu bar: Ztty (Settings, Quit), Shell (New Terminal Tab, Close Terminal Tab), Edit (Copy, Paste, Select All, Find), View (Toggle Sessions, Toggle Right Panel, Bigger, Smaller, Reset Size), Window.
- The same shortcuts on Windows and Linux with Ctrl-Shift for terminal-colliding keys. On Windows Ctrl-V pastes too, as in Windows Terminal.

## 13. Acceptance

- Each chapter's tasks ticked in `TASKS.md` with a passing review in `tasks/reviews.md`.
- `scripts/check.sh` passes: fmt, clippy with `-D warnings`, tests, no file over 500 lines.
- CI green on Ubuntu and Windows.

## 14. Icon and the macOS app

- The icon is Ztty's mark: the terminal's prompt, a chevron and the block cursor on its baseline, phosphor green and white on near-black.
- macOS: the icon on the rounded square macOS gives app icons. Windows and Linux: the same square, so the mark holds on a light taskbar.
- `scripts/mac-app.sh` builds `Ztty.app` from a release build with its icon, signed ad hoc, or for a release with Developer ID and notarized, and a disk image holding it beside a link to Applications.

## 15. Release

- A release on GitHub, tagged `vVERSION`, carries three builds: the macOS disk image (Apple silicon, signed with Developer ID and notarized), a Windows zip (x86_64) and a Linux tarball (x86_64).
- The Windows executable carries the icon. The Linux tarball carries the icon and a desktop entry, and the window names itself after it, `io.github.zzqxvii.ztty`, as the macOS bundle does.
- Each build comes from its platform's script: `scripts/mac-app.sh`, `scripts/windows-zip.ps1`, `scripts/linux-tar.sh`. The Release workflow builds all three — the zip on a Windows runner, the tarball on a Linux one, the disk image on the signing runner — on a `vVERSION` tag or by hand, and puts each on the release of that version.

## Deferred

Moving a project in the list, pinning, search, splits, profiles, key rebinding, kitty graphics, sixel, session restore, push and pull, quick terminal. Renaming a session, duplicating one, reordering sessions by a drag and a session row's menu went with the session rows.
