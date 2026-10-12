# dev 规则与 Windows 0.3.1 本地打包（2026-10-12）

用户要求所有代码修改的顶层开发分支 / 起点只能为 dev，master 仅合入；随后本地重新打包并更新为0.3.1。修改前工作区干净，四个 master / dev 引用均为 `c089cf735fde4a50144509f26764578bb04d8236`；先切换到 dev，再编辑。本轮只在 dev 工作区修改和本地打包，不提交、推送或合入 master。

AGENTS 第0节覆盖源码、UI、脚本、配置、版本及文档；master 合入遇到冲突时，先在 dev 合并最新 master 并处理 / 验证，再让 master 快进合入 dev。旧 STATUS / ADR 分支操作作为历史保留，不能覆盖本轮规则。同步现行开发 / 发布指南，保留既有音视频代码与资格限制。

workspace 五个 crate 与 Cargo.lock 为0.3.1，EXE / NSIS / 包名继续从同一 Cargo 版本生成，打包回归 fixture 更新为0.3.1。沿用已有官方便携 NSIS 3.12（target目录）及固定 hash libmpv，不安装系统工具或修改用户默认配置。

## 本轮检查与产物

Package-Windows 在 dev 使用 `-Offline -NsisCompiler <已有便携 makensis.exe>`，自动执行 locked x64 MSVC Release build 并取本次 compiler-artifact.executable；Windows PowerShell5.1实际编译 / 打包exit0，Release编译4分06秒。包内记录基准提交与 `Working tree modified: True`，不把未提交修改冒充已合入 master。

| 检查 | 实际结果 |
| --- | --- |
| `cargo fmt --all --check` | exit0 |
| `cargo metadata --locked --offline --no-deps --format-version 1` | 五个workspace crate均0.3.1，锁定图一致 |
| `Test-WindowsPackaging.ps1 -NsisCompiler <便携工具>` | 7项隔离回归通过，fixture为0.3.1 |
| 自动Release编译 / ZIP / NSIS Setup | exit0，EXE与Setup的FileVersion / ProductVersion均0.3.1 |
| ZIP内容 / BUILD-INFO / 固定runtime | ZIP EXE与本次Cargo产物SHA一致；BUILD-INFO应用hash / 版本 / dirty标记一致；DLL与manifest SHA一致 |
| 包内程序独立配置启动 | 4秒正常退出exit0，ready=true、Idle、error/render_error空、render初始化0；mpv v0.41.0-1087-ge470f8986 |
| 分支 / 文档 / 语法 / diff | 当前dev；原四个分支 / 远端跟踪引用均未变；master提交版本仍0.3.0；相对链接、PowerShell AST与diff通过 |

最终本地开发包：

| 文件 | 字节数 | SHA-256 |
| --- | ---: | --- |
| `YYPlayer-0.3.1-c089cf735fde-windows-x64-portable.zip` | 58,539,528 | `61206e138769534d484a82432c526e9ac9e56753c30e90a8908625daad0f4ab0` |
| `YYPlayer-0.3.1-c089cf735fde-windows-x64-setup.exe` | 58,727,144 | `6f8a06777c444c5de52e172548d095066671f39820f8f41c5384e47d44646516` |
| Cargo / ZIP中的 `yyplayer.exe` | 28,974,080 | `8b20e7d4ad954cd7e5b0323977622d8ce61e5c3ae5d3419eaab6dc5cf4f58295` |
| ZIP中的 `runtime/libmpv-2.dll` | 120,816,640 | `0d5b9dbecb73e179ef39dc94314ff8905fab926a80f8d60a2367d824b7254e2e` |

包位于dist；编译 / fmt / 打包回归日志在 `target/dev-031-{package,fmt,packaging-test}.log`，包内容及独立启动数据在 `target/dev-031-smoke/`。这些生成物均被.gitignore排除，没有强制加入Git，也未修改master引用。

本轮为分支规则 / 文档 / 版本更新，播放实现不变；以格式、锁定 metadata、现有打包回归、真实编译 / 打包及独立启动核验为范围。没有重新执行严格clippy、完整单测、全面 UI / GPU / 音频 / Linux / macOS / USB / HDR / 安装卸载资格；既有报告保持原结论。下一步按明确授权在dev提交，需要发布时再将已验证dev合入master。
