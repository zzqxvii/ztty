[English](README.md) | [简体中文](README.zh-CN.md)

# Ztty

A terminal in Rust on GPUI and Ely components. Projects and their git worktrees sit on the left, the worktree's tabs in the middle, Git or Files on the right. Every setting has a control in the settings window.

Ztty is modified from [ELYTerminal](https://github.com/ELY-Open-Source/ELYTerminal), and builds on its component library, [Ely GPUI Components](https://github.com/ELY-Open-Source/Ely-GPUI-Components). The projects and worktrees column, and the tabs in the window's top row, follow [Orca](https://github.com/stablyai/orca).

## Install

A [release on GitHub](https://github.com/zzqxvii/ztty/releases) carries a Windows build for x86_64. Unzip `Ztty-VERSION-windows-x86_64.zip` and run `ztty.exe`. It is unsigned; if SmartScreen stops it, press More info, then Run anyway.

## Build and run

Rust 1.95.0, which `rust-toolchain.toml` pins; rustup installs it on the first build, beside Visual Studio's C++ tools, which gpui's build needs.

```sh
cargo run --release
```

`scripts/windows-zip.ps1` builds the release and zips `ztty.exe`, its icon inside, as `Ztty-VERSION-windows-ARCH.zip` in `target/release`.

The app writes its log to stderr. `RUST_LOG=ztty=debug` writes more; `trace` logs each paint. A release build opens no console: `ztty.exe 2> log.txt` keeps its log.

## What it does

- The terminal runs on `alacritty_terminal`: 256 colors and truecolor, bold, dim, italic, five underline styles, wide characters and combining marks. Box drawing, block elements and powerline shapes are drawn as geometry, so lines join across cells. Keys follow xterm and the kitty keyboard protocol, and Windows' win32-input-mode, as Windows Terminal sends them, so herdr's shortcuts work; input methods compose at the cursor; programs can track the mouse; pastes are bracketed when a program asks; OSC 8 and plain links open with the link press below; inside herdr, herdr opens them, web links only, whole where its pane wraps them, and with Shift the terminal opens what it sees; find searches the scrollback.
- A scrollbar along the terminal's right edge scrolls its history, even while a program takes the wheel.
- Shell integration for zsh, bash, fish and PowerShell, for Git for Windows' bash and `sh` too, and for the bash, zsh or fish a Shell of `wsl` starts, so Files and Git follow a WSL or Git Bash session: each prompt reports its folder and title, and OSC 133 marks each prompt, each command and its exit status.
- Sessions: new, close and switch. A right press on a row duplicates it, in the shell it runs and the folder it last reported, or closes it, and a drag moves a row to another place. A bell marks a hidden session's row and makes no sound. A row's mark shows what its programs report through OSC 7501, OSC 9;4 or herdr: waiting on you, an error, work with its progress, done or idle, else a failed last command. The mark is 3 by 3 dots: work ripples and waiting dips while they last. Hover over the mark lists each report.
- Notifications: while the window is in the background or the session hidden, a session that starts waiting, finishes or fails, a command that ran 5 seconds or more ending, and a program's OSC 9, 777 or 99, post a system notification, one a session every 3 seconds at most; a press shows the session.
- Ctrl-Shift-Up and Ctrl-Shift-Down jump to the prompt before or after the view's top.
- A button at each end of the title bar hides or shows its side column, and a drag on the border between a side column and the terminal resizes it; the window opens as it was left, at the size and widths it was left at. A gear beside the right button opens Settings.
- Files shows the worktree's root as a tree with Git's marks, whatever the tabs do, a press folding a folder or opening a file. Files opens by default; Git and Files are two icons at the column's foot, and the choice is kept.
- Git shows the branch, stages, unstages, discards, commits, lists recent commits and opens diffs, all through the `git` command line.
- Themes: Ely Dark, Ely Light, Catppuccin Mocha, Macchiato, Frappé and Latte, Tokyo Night, Dracula, Nord, Gruvbox Dark, Rosé Pine, Solarized Light, One Light and Kanagawa Wave. A second theme can take over while the system is dark.
- The interface speaks English or Simplified Chinese: Settings › General picks one, and the first start takes the system's language, so the menus, the windows and every label change at once.
- Background opacity and blur: under 100% the window lets the desktop through, and Background blur says whether it shows it through Windows' acrylic or clear.

## Keys

Settings › Shortcuts lists them all, as the platform binds them.

- New session: Ctrl-Shift-T
- Close session: Ctrl-Shift-W
- Session 1 to 8, last: Alt-1 to Alt-8, Alt-9
- Next, previous session: Ctrl-Tab, Ctrl-Shift-Tab
- Sessions column: Ctrl-Shift-B
- Right column: Ctrl-Shift-Alt-B
- Copy, paste: Ctrl-Shift-C, Ctrl-Shift-V; Ctrl-V pastes too
- Select all: Ctrl-Shift-A
- Find: Ctrl-Shift-F
- Bigger, smaller, reset size: Ctrl-=, Ctrl-minus, Ctrl-0
- Open a link: Ctrl-press
- Settings: Ctrl-,
- Quit: Ctrl-Shift-Q

## Settings

The settings window saves each change at once to `%APPDATA%\Ztty\settings.json`.

A file that does not parse, or holds a value out of range, stops the start with an error naming the file and the field. The app never overwrites it.

Settings › General checks for a newer release at start and once a day, or at once with Check now; a newer one shows a Download button that opens its page. The check runs `curl`, which Windows ships.

## Development

`scripts/check.sh` runs fmt, clippy, the tests and the 500-line rule. `AGENTS.md` lists the rest: the layout, the decisions and the tools.

## Credits

- Modified from [ELYTerminal](https://github.com/ELY-Open-Source/ELYTerminal). Its component library, [Ely GPUI Components](https://github.com/ELY-Open-Source/Ely-GPUI-Components), draws the chrome, the widgets and the settings.
- The projects and worktrees column, the tab strip, the file tabs' font and the Git and Files switch follow [Orca](https://github.com/stablyai/orca).
- Box drawing and key encoding follow Ghostty's code, MIT, in `LICENSE-GHOSTTY`.
- The theme files come from iTerm2-Color-Schemes, MIT, with its license in `assets/themes`.
- JetBrains Mono and its Nerd Font build are under the SIL Open Font License, in `assets/fonts`.

## License

MIT, in `LICENSE`.
