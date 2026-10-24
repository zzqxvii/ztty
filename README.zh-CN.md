[English](README.md) | [简体中文](README.zh-CN.md)

# Ztty

用 Rust 写在 GPUI 与 Ely 组件上的终端。左侧是项目及它们的 git worktree，中间是该 worktree 的标签页，右侧是 Git 或 Files。每一项设置都在设置窗口里有控件。

Ztty 修改自 [ELYTerminal](https://github.com/ELY-Open-Source/ELYTerminal)，并使用它的组件库 [Ely GPUI Components](https://github.com/ELY-Open-Source/Ely-GPUI-Components)。左侧的项目与 worktree 栏，以及窗口顶部那一行的标签页，参考了 [Orca](https://github.com/stablyai/orca)。

## 安装

[GitHub 上的 release](https://github.com/zzqxvii/ztty/releases) 提供 x86_64 的 Windows 构建。解压 `Ztty-VERSION-windows-x86_64.zip`，运行 `ztty.exe`。它没有签名，若被 SmartScreen 拦住，点「更多信息」，再点「仍要运行」。

## 构建与运行

Rust 1.95.0，由 `rust-toolchain.toml` 钉住，首次构建时 rustup 会装上；同时需要 Visual Studio 的 C++ 工具，gpui 的构建要用。

```sh
cargo run --release
```

`scripts/windows-zip.ps1` 构建 release，把 `ztty.exe`（图标已嵌入）压成 `target/release` 下的 `Ztty-VERSION-windows-ARCH.zip`。

应用的日志写到 stderr。`RUST_LOG=ztty=debug` 写得更细，`trace` 每帧重绘都记一条。release 构建不开控制台窗口：`ztty.exe 2> log.txt` 可留下日志。

## 功能

- 终端跑在 `alacritty_terminal` 上：256 色与真彩色，粗体、暗色、斜体、五种下划线、宽字符与组合字符。制表符、块元素和 powerline 形状按几何绘制，所以跨单元格的线能接上。按键遵循 xterm 与 kitty 键盘协议，以及 Windows 的 win32-input-mode，与 Windows Terminal 发的一样，所以 herdr 的快捷键可用；输入法在光标处组词；程序可以跟踪鼠标；程序要求时粘贴带括号；OSC 8 链接与普通链接用下面的链接键打开，在 herdr 里则由 herdr 打开，只开网页链接，跨行截断的也整条打开，按住 Shift 则由终端打开它看到的部分；查找搜索回滚历史。
- 终端右边缘的滚动条可以翻历史，即便程序正占用滚轮。
- zsh、bash、fish 与 PowerShell 的 shell 集成，Git for Windows 的 bash 和 `sh` 也在内，Shell 设为 `wsl` 时它启动的 bash、zsh 或 fish 同样如此，于是 Files 与 Git 能跟随 WSL 或 Git Bash 会话：每个提示符报告自己的目录与标题，OSC 133 标出每个提示符、每条命令及其退出状态。
- 会话：新建、关闭与切换。在行上右键可复制该会话（用它跑的 shell 和它最后报告的目录），或关闭它；拖动可以把行移到别处。响铃会在隐藏会话的行上留个标记，但不发声。行的标记显示程序通过 OSC 7501、OSC 9;4 或 herdr 报告的状态：等你响应、出错、进行中及进度、完成或空闲，否则就是上一条命令失败。标记是 3×3 的点阵：进行中的波纹、等你响应的下沉。悬停在标记上会列出每条报告。
- 通知：当窗口在后台或会话被隐藏时，会话开始等待、结束或失败，跑满 5 秒的命令结束，以及程序的 OSC 9、777 或 99，都会发系统通知，每个会话最多 3 秒一条；点一下通知就切换到该会话。
- Ctrl-Shift-Up 与 Ctrl-Shift-Down 跳到视图顶部之前或之后的那个提示符。
- 标题栏两端各有一个按钮，隐藏或显示对应侧栏；拖动侧栏与终端之间的边界可以改宽度；窗口按离开时的尺寸和两侧宽度打开。右侧按钮旁边的齿轮打开设置。
- Files 把 worktree 的根目录显示成带 Git 标记的树，与标签页在做什么无关；点一下折叠文件夹或打开文件。默认显示 Files；Git 与 Files 是侧栏底部的两个图标，选择会被记住。
- Git 显示分支，可以暂存、取消暂存、丢弃、提交、列出最近的提交并打开 diff，全部通过 `git` 命令行完成。
- 主题：Ely Dark、Ely Light、Catppuccin Mocha、Macchiato、Frappé 与 Latte、Tokyo Night、Dracula、Nord、Gruvbox Dark、Rosé Pine、Solarized Light、One Light、Kanagawa Wave。系统处于深色时可以换用第二套主题。
- 界面支持英文与简体中文：在设置 › 通用里选，首次启动取系统的语言，菜单、窗口与所有文案一起换。
- 背景不透明度与模糊：低于 100% 时窗口透出桌面，Background blur 决定是透过 Windows 的亚克力显示，还是直接透出。

## 快捷键

设置 › 快捷键按平台列出全部。

- 新建会话：Ctrl-Shift-T
- 关闭会话：Ctrl-Shift-W
- 第 1 到 8 个会话、最后一个：Alt-1 到 Alt-8、Alt-9
- 下一个、上一个会话：Ctrl-Tab、Ctrl-Shift-Tab
- 会话栏：Ctrl-Shift-B
- 右侧栏：Ctrl-Shift-Alt-B
- 复制、粘贴：Ctrl-Shift-C、Ctrl-Shift-V；Ctrl-V 也能粘贴
- 全选：Ctrl-Shift-A
- 查找：Ctrl-Shift-F
- 放大、缩小、恢复字号：Ctrl-=、Ctrl-减号、Ctrl-0
- 打开链接：Ctrl-点击
- 设置：Ctrl-,
- 退出：Ctrl-Shift-Q

## 设置

设置窗口把每次改动立刻存到 `%APPDATA%\Ztty\settings.json`。

文件解析不了，或某个值超出范围，启动会停下并报出文件名和字段。应用从不覆盖它。

设置 › 通用在启动时、以及之后每天检查一次新版本，点「立即检查」则马上检查；有新版就出现「下载」按钮，打开它的页面。检查通过 `curl` 进行，Windows 自带。

## 开发

`scripts/check.sh` 跑 fmt、clippy、测试和 500 行规则。其余在 `AGENTS.md`：目录结构、决策与工具。

## 致谢

- 修改自 [ELYTerminal](https://github.com/ELY-Open-Source/ELYTerminal)。它的组件库 [Ely GPUI Components](https://github.com/ELY-Open-Source/Ely-GPUI-Components) 绘制了外壳、控件与设置页。
- 左侧的项目与 worktree 栏、标签条、文件标签的字体，以及 Git 与 Files 的切换，参考了 [Orca](https://github.com/stablyai/orca)。
- 制表符绘制与按键编码遵循 Ghostty 的代码，MIT，见 `LICENSE-GHOSTTY`。
- 主题文件来自 iTerm2-Color-Schemes，MIT，许可证在 `assets/themes`。
- JetBrains Mono 及其 Nerd Font 版本使用 SIL Open Font License，见 `assets/fonts`。

## 许可证

MIT，见 `LICENSE`。
