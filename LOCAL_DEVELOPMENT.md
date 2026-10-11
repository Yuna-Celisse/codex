# Better Codex：官方 TUI 基线

本工作区基于 OpenAI Codex 官方源码，固定于 `rust-v0.162.0`，提交 `c1382380de69521303b416720a52f42d51af6248`。当前自定义代码仅涉及原生 TUI 的响应式 Context 侧栏；App Server 实现、上游 README、LICENSE 和 NOTICE 保持原样。

目前在该基线上加入响应式 Context 侧栏：终端宽度达到 120 列时显示右侧 32 列面板和竖线；低于 120 列时自动隐藏，聊天区恢复完整宽度。面板显示最近请求的上下文 token、窗口占比、输入/缓存/输出统计、累计会话用量与工作目录。缩放不会清空输入草稿或改变会话。

本地启动默认使用官方内置的 full-screen transcript，以保持侧栏固定；只传启动参数，不修改全局配置。源代码有改动时，启动器不会回退到未包含这些改动的官方发行版。

当前本地分支为 `better-codex`，上游远程为 `upstream`。之前的 TypeScript / Ink 实现和安装依赖完整保存在 `.legacy/ink-tui-2026-10-11`，并通过本机 Git exclude 排除。本次侧栏是原生 Rust 实现，没有导入旧 TypeScript 客户端。

## 启动

在仓库根目录运行 `start.cmd`，或双击该文件。

启动入口优先运行本工作区编译出的 `codex-rs/target/debug/codex.exe`（其次 release）。没有构建产物时，它只允许使用本机同版本的官方 `codex-cli 0.162.0`，并明确提示这是发行版，不会把源码变更伪装为已构建生效。

```powershell
.\start.cmd --help
.\start.cmd
```

官方 TUI 也支持连接独立 App Server：

```powershell
# 另开一个终端
codex app-server --listen ws://127.0.0.1:4500

# 本工作区
.\start.cmd --remote ws://127.0.0.1:4500
```

参数按官方 CLI 的格式使用：例如 `-m` / `--model`、`-C` / `--cd`、`--remote`。根目录 `package.json` 是上游维护工具配置，`npm start` 不再作为 TUI 启动入口。

## 修改与构建

官方 TUI 源码位于 `codex-rs/tui`；CLI 入口位于 `codex-rs/cli`；App Server 位于 `codex-rs/app-server`。后续个性化修改从这些原生 Rust 文件开始。

Windows 构建需要 Rust、Visual Studio C++ Build Tools 和 Windows SDK。官方锁定 Rust 1.95.0；本地构建脚本会初始化 MSVC 环境，并保持 Cargo.lock 不变：

```powershell
.\local\build.ps1
.\local\build.ps1 -Release

# 如需使用本机已安装的 stable 工具链
.\local\build.ps1 -Toolchain stable
```

构建后重新运行 `start.cmd`，会优先使用工作区产物。完整上游说明见 `docs/install.md` 和 `codex-rs/scripts/setup-windows.ps1`。

查看相对官方 TUI 基线的改动：

```powershell
git diff rust-v0.162.0 -- codex-rs/tui
git status --short
```

版本来源记录在 `local/upstream.json`。发行标签的 Cargo.lock 仍将本地工作区包锁为 0.0.0，而 Cargo.toml 声明 0.162.0；为使 `--locked` 构建可用，已只同步这些本地包的版本号，外部依赖版本和校验和未改变。

## 本次验证

- 工作区原生 `codex.exe` 构建成功（本机 stable / Rust 1.98.1；官方指定的 Rust 1.95.0 也已安装）。当前启动器使用工作区产物。
- `cargo +stable test --locked -p codex-tui context_sidebar --lib`：3 项通过，覆盖 119/120 列阈值、反复缩放保留草稿、当前用量与累计用量区分、全高竖线及光标/聊天区布局。
- 实际启动工作区 TUI：140 列显示侧栏，95 列隐藏侧栏；App Server、core、LICENSE 和 NOTICE 未修改。
- `start.cmd -c check_for_update_on_startup=false --version` 参数转发验证通过。启动脚本使用未绑定的 `$args`，避免 PowerShell 将 `-c` 当作脚本参数而丢失标志。
- 首次终端验证暴露的参数转发错误导致全局更新检查设置被误改；该设置已从运行前备份按字节完整还原。备份哈希与当前全局配置相同，没有保留该误改。
