# 项目与 Worktree 改造任务

设计见 `docs/design-projects-worktrees.md`(v2 收敛版)。只做设计里写明的内容,不加 Orca 的其他功能。

状态:`[ ]` 未开始 · `[~]` 进行中 · `[x]` 完成 · `[!]` 阻塞

完成定义:代码 + 测试 + `scripts/check.sh` 通过 + 对应验收场景走通。一次一条,按顺序推进。

## A. 模型与发现

- [x] **T73 设置字段与项目模型**
  - 内容:`Settings.projects: Vec<PathBuf>`(默认空);`src/projects.rs` 的 `Project`/`Worktree`;添加校验(绝对路径、存在、是目录、可读、去重);移除;不存在路径保留并标错。
  - 测试:序列化往返、缺字段默认、相对路径拒绝、重复拒绝、平台大小写比较。

- [x] **T74 worktree 发现**
  - 内容:`src/projects/git.rs` 调用 `git -C <path> worktree list --porcelain`;解析 worktree 与 branch(HEAD 行忽略);C 转义路径;非 git 目录与 git 失败降级为单 worktree;主检出标记 default。
  - 测试:临时仓库、第二个 worktree、detached、普通目录、带空格/非 ASCII 的路径解析。

## B. 左栏与切换

- [x] **T75 左栏:项目 → worktree**
  - 依赖:T73、T74
  - 内容:左栏 header 标题 `Projects`;项目行(折叠箭头 + 文件夹图标 + 粗体名称、悬停路径、hover 的 × 移除并确认);worktree 行(缩进一级、状态点或状态标记、分支名弱化、多个 worktree 的项目里主检出带 `default` 徽标);底部 Add project(文件夹选择器);空状态;选中 worktree。
  - 验收:验收场景 1、2、3 的左栏部分。

- [x] **T76 worktree 切换、标签条与多终端**
  - 依赖:T75
  - 内容:`src/worktree.rs` 运行时;选中 worktree 切中栏并恢复其活动标签,首次选中自动开终端;标签条在窗口最上一行(中栏 header 的 leading 位,自绘,选中标签底边强调线);终端标签新建/关闭/切换,关闭结束对应 PTY,关掉应用最后一个终端才关窗口;`Cmd-T`/`Cmd-W`/`Cmd-1..9`/`Ctrl-Tab` 改为新语义,新增 `Cmd-Shift-[]` 切标签;中栏头部与窗口标题。
  - 验收:验收场景 3、4、7。

## C. 标签

- [x] **T77 只读文件标签**
  - 依赖:T76
  - 内容:Ely `CodeEditor::read_only`;文本判定(UTF-8 且无 NUL);二进制说明;>2 MiB 截断;同一路径单标签;窗口激活时重读;读失败提示。
  - 验收:验收场景 5。

- [x] **T78 diff 标签**
  - 依赖:T76
  - 内容:Git 面板从全屏 Dialog 改为发出打开事件;Ely `DiffViewer` + unified/split;`Acted` 后刷新;变更消失/二进制/未跟踪目录的说明;同一路径单标签。
  - 验收:验收场景 6。

## D. 接入与收尾

- [x] **T79 右栏跟随与既有能力接入**
  - 依赖:T76、T77、T78
  - 内容:Files 单击改为打开文件标签;右栏根目录按活动标签跟随;状态点/通知/Ely/窗口标题改为遍历所有 worktree 的终端,行为与现在一致。
  - 验收:验收场景 8。

- [x] **T81 顶栏重设计(业主反馈)**
  - 依赖:T75、T76
  - 内容:标签条从独立一行移进窗口最上一行(中栏 header),按可用宽度封顶并在其内滚动(滚轮),当前标签变化时自动滚入视野;左栏 header 写 `Projects`,项目行与 worktree 行按字重与缩进区分,主检出徽标只在多 worktree 的项目里出现。
  - 验收:Windows 上两项目、单标签、再到 8 个标签:标签条被限宽、滚轮可滚、新开标签自动可见。

- [~] **T80 测试、验收与文档**
  - 依赖:T73–T79
  - 内容:补齐验收场景 1–8 的证据;更新 `PRD.md`、`AGENTS.md`、`README.md`;`TASKS.md` 收录本阶段;`scripts/check.sh` 全绿。
  - 验收:全部场景有证据,文档与实现一致。

## 实现中的偏离(已写进 PRD 与设计)

- 关闭某个 worktree 的最后一个标签后中栏留空,再点该 worktree 会重新开终端;只有应用最后一个终端关闭才关窗口。
- 项目行按一下折叠/展开;悬停出现的 × 与行点击会同时触发折叠(移除前的那次折叠不可见)。
- 文件/diff 标签模块落在 `src/file.rs`、`src/diff.rs`(设计表原写 `src/tabs/`)。
- Windows 上只验证了:构建、启动、按 `projects` 发现 worktree、终端开在 worktree 根、Files/Git 跟随、顶栏布局与标签滚动;鼠标点击路径未手动走过(`shot` 仅 macOS)。

## 备注

- `workspace.rs` 拆分后仍需 ≤ 500 行;超出就先拆模块再继续。
- 标签与终端以 `EntityId` 为键,不用索引。
- 每次切换 worktree 都要重建右栏根目录;herdr 迟到的答复仍按现有规则丢弃。
