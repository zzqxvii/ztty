# 项目与 Worktree 改造设计

- 状态:草案 v2(收敛版),2026-10-24
- 任务:`tasks/projects-worktrees.md`;实施完成后并入 `PRD.md`、`TASKS.md`
- 参考:Orca 只借信息架构——项目 → worktree → worktree 内标签;Orca 其余能力一律不引入

## 1. 需求

- **R1** 左栏的单位从终端会话改为**项目**;项目下面是 **worktree**。
- **R2** 项目没有 worktree 时(非 git 目录),它的默认目录本身就是唯一的一个 worktree。
- **R3** 点开某个 worktree,中栏显示该 worktree 的标签:终端、文件查看、Git diff。
- **R4** 克制:只做以上三件事,不顺带加 Orca 的其他功能。

## 2. 范围

**做**

- 左栏:项目 / worktree 两级列表,添加与移除项目,选中 worktree。
- 中栏:每个 worktree 自己的标签集,终端标签可多个,文件与 diff 只读标签。
- 右栏:保留现有 Files/Git,跟随活动 worktree;点文件开文件标签,点变更开 diff 标签。
- 既有能力(状态点、通知、Ely、快捷键、主题、模糊)在新结构下继续工作,行为不变。

**不做**

- 应用内新建/删除/切换 worktree、分支管理。
- 文件编辑与保存。
- 标签、选中项、折叠状态的持久化。
- 项目排序、固定、休眠、搜索、跳转面板、agent 编排、CLI、PR 集成、多窗口。
- 新增菜单项;不改造右栏本身。

## 3. 模型

```rust
struct Project { path: PathBuf, name: String, worktrees: Vec<Worktree> }

struct Worktree { path: PathBuf, branch: Option<String>, default: bool }

enum Tab { Terminal(session), File(PathBuf), Diff(PathBuf) }
```

- 往 `settings.json` 只加一个字段 `projects: Vec<PathBuf>`(绝对路径,顺序即显示顺序)。
- 启动时按 `projects` 发现 worktree;终端、标签、展开状态都是运行时,进程退出即消失。
- 发现:git 项目运行 `git -C <path> worktree list --porcelain`,git 的顺序,第一条(主检出)标记 default;非 git 目录或 git 失败时,文件夹自身作为唯一 worktree。
- 每一行 worktree 第一次被选中时自动开一个终端,目录 = worktree 根。

## 4. 左栏

- **项目行**:折叠箭头 + 名称;悬停显示完整路径;hover 时右侧出现 `×` 移除(确认后从设置移除,不动磁盘,结束该项目运行中的终端)。
- **worktree 行**:缩进;状态点(该项目下所有终端的聚合,沿用现有 3×3 视觉);分支名,无分支时用文件夹名;主检出带 `⌂`。
- 选中 worktree:切中栏到它上次的活动标签;没有就新开终端。
- 底部 `+ Add project`:文件夹选择器,校验存在/是目录/可读,去重。
- 空列表:启动时或删掉最后一个项目后,`projects` 为空,把 home 作为普通项目加回、写进设置并选中,在其中开一个终端;只有 home 单独存在时它的 `×` 隐藏,最后一个普通项目仍可删,删掉后 home 立刻补回。home 与普通项目一样有 worktree、删除确认和 Files/Git;它被选中时右栏跟随 home,并递归监听 home。
- 左栏宽度、`Cmd-B` 开关、Ely 的 `ColumnShell` 不变。

## 5. 中栏

- 标签条在窗口最上一行,即中栏 header 的 `leading` 位:图标 + 标题,选中标签用底边强调线标记、文字为实色,各带关闭按钮,可拖动排序;标题:终端取程序标题、文件取文件名、diff 取文件名。
- 标签宽度按可用宽度封顶(窗口宽减去两栏与 header 自身控件),超出时横向滚动(滚轮),并自动把当前标签滚进视野。
- 左栏 header 标题为 `Projects`;项目行 = 折叠箭头 + 文件夹图标 + 粗体名称;worktree 行缩进一级,前置状态点或状态标记,名称用弱化色,默认检出在多于一个 worktree 的项目里带 `default` 徽标。
- 同一路径的文件/diff 只开一个标签。
- 关闭:终端标签结束其 PTY;关闭整个应用最后一个终端时关闭窗口(现状)。
- 标签不持久化。
- 快捷键(只新增一组):

| 动作 | macOS | Windows/Linux |
|---|---|---|
| 新终端标签 | `Cmd-T`(沿用) | `Ctrl-Shift-T`(沿用) |
| 关闭标签 | `Cmd-W`(沿用) | `Ctrl-Shift-W`(沿用) |
| 下一个/上一个标签 | `Cmd-Shift-]` / `[`(新增) | `Ctrl-Shift-]` / `[` |
| 下一个/上一个 worktree | `Ctrl-Tab`(沿用) | `Ctrl-Tab` |
| 选中第 1–9 个 worktree | `Cmd-1..9`(沿用) | `Alt-1..9`(沿用) |

## 6. 文件标签

- Ely 的 `CodeEditor`,`read_only()`,带行号与语法高亮。
- 文本判定:UTF-8 可解码且不含 NUL;其余显示一行“Binary file”,不做图片预览。
- 超过 2 MiB 只显示前 2 MiB,并在顶部说明截断。
- 打开时读盘;窗口重新激活时重读。读失败显示原因,不崩溃。

## 7. Diff 标签

- Git 面板的行不再弹全屏 Dialog,改为通知 workspace 打开 diff 标签。
- 标签内用 Ely 的 `DiffViewer`,保留 unified/split 切换。
- 每次 Git 操作(`Acted`)后重新取该路径的 diff;文件已无变更时显示说明。
- 二进制文件、未跟踪目录:沿用现有文案。

## 8. 右栏

- 结构与开关不变。
- 跟随:活动标签是终端 → 跟随该终端当前目录(含 herdr/WSL,现状);是文件/diff → 跟随活动 worktree 根目录。
- Files 树单击文件改为打开文件标签,不再调用系统程序。

## 9. 既有能力

- 状态点、悬停记录、80 ms 动画:数据源改为 worktree 内全部终端,聚合优先级 blocked > error > working > done > idle。
- 通知:照旧;按下时定位到对应 worktree 的对应终端标签。
- 窗口标题:活动标签的标题。
- Ely:心情取全部终端聚合,开关不变。

## 10. 设置

- 新字段 `projects`:必须绝对路径、不重复;不存在的路径保留在文件中(启动时记日志并在左栏标错),不因此拒绝启动。
- 左栏的 Add/× 就是该字段的控件,不再单开 Settings 页面。
- `show_sessions` 字段名不改,语义变为“显示项目栏”。

## 11. 代码结构

| 文件 | 职责 |
|---|---|
| `src/projects.rs` / `src/projects/git.rs` | 模型、添加/移除、发现(已实现) |
| `src/worktree.rs` | 单个 worktree 的运行时:终端、标签、状态聚合 |
| `src/workspace/left.rs` | 左栏渲染与交互 |
| `src/workspace/tabs.rs` | 标签条与标签动作 |
| `src/file.rs`、`src/diff.rs` | 两种只读标签 |
| `src/files.rs`、`src/git/panel.rs` | 改为发出“打开文件 / 打开 diff”事件 |
| `src/workspace.rs`、`src/app.rs` | 接入新结构,拆分后仍 ≤ 500 行 |

## 12. 验收

1. 无项目启动 → 左栏只有 Add project,中栏有一个可输入的终端。
2. 添加 git 仓库 → 列出主检出(带 ⌂);添加普通目录 → 只有一行 worktree。
3. 仓库有两个 worktree → 左栏三条与 `git worktree list` 一致;点第二条,中栏新终端开在正确目录。
4. 切回第一条,原终端与输出仍在;两个 worktree 各开两个终端标签不串台。
5. Files 单击文件 → 只读文件标签;再点同一文件只激活;二进制文件显示说明。
6. Git 面板点变更 → diff 标签,unified/split 可切;再次点另一个变更开第二个标签。
7. 关闭最后一个终端 → 窗口关闭、应用退出;关闭单个标签只结束它自己。
8. 通知按下 → 切到对应 worktree 的终端标签;移除项目 → 确认后终端结束、项目从设置消失。

## 13. 决策

- **D1** 只读列出 worktree,不新建/删除(git 只读,避免误改用户仓库)。
- **D2** 文件标签只读。
- **D3** 每个 worktree 可开多个终端标签,状态点在 worktree 行聚合。
- **D4** 保留右栏,作为打开文件与 diff 的入口。
- **D5** 非 git 项目与 git 缺失时,文件夹自身作为唯一 worktree。
- **D6** 只持久化 `projects`;标签、选中项、折叠状态不持久化。
- **D7** 不新增菜单项;`Cmd-T`/`Cmd-W` 语义改为标签,快捷键表只加 `Cmd-Shift-[/]`。
