# 参与 YYPlayer 开发

唯一顶层开发分支与开发起点为 dev，当前 dev 开发版本为 0.3.1；master 仅合入 dev 的已验证改动，用于集成 / 发布，禁止直接编辑或提交源码、版本、脚本与文档。修改前检查当前分支，master 上的合入冲突也先回 dev 处理，再让 master 快进合入。Windows / Linux 共用代码，分支名不决定编译平台。项目自身源码 / 原创 UI 资源采用 GPL-3.0-only，贡献应使用相同许可；不要提交无权授权的代码、用户媒体或凭据，第三方组件保留原许可。

## 开发环境

先读 [README](README.md)、[开发规划](docs/DEVELOPMENT_PLAN.md)、[开发规则](AGENTS.md) 和 [实际状态](docs/STATUS.md)。当前验证平台为 Windows x64 MSVC 与 Ubuntu 26.04 amd64 GNU。Windows 需要 Visual Studio C++ Build Tools / Windows SDK、Rust 和 Git；Linux 先按 [Linux 指南](docs/LINUX.md) 审计原生依赖。Windows / Linux 均已有 Rust / Cargo 1.99.0 记录，MSRV 1.92 尚未专项验证；Slint / slint-build 固定 1.17.1。

```powershell
powershell -ExecutionPolicy Bypass -File scripts/Get-Mpv.ps1
cargo run --locked -p yyplayer-app
```

获取脚本按 manifest 检查运行时 hash。DLL、so、下载 archive、target、用户配置和媒体不得提交。Linux 原生播放 / GPU 和包证据见 STATUS；macOS 尚未编译 / 真机验收。

Ubuntu 构建 / 运行：

```bash
python3 scripts/check-linux-deps.py
cargo build --locked --release -p yyplayer-app --bin yyplayer
./target/release/yyplayer
```

同一条 Cargo build 命令在 Windows 生成 exe、在 Linux 生成 ELF。平台 API 与新增依赖按 target cfg 隔离；共用代码变更需检查两边，Windows Actions 编译打包已由用户确认通过，不等于所有运行场景已回归。

## 变更与检查

改动前先写明目标和验收方式，沿用 Slint 主线程、Engine worker 与 GL presenter 边界。功能变更运行：

```powershell
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
cargo build --locked --release
```

再运行对应场景脚本，结果、未验证条件和下一步写入 STATUS。布局样例不能替代真实播放、USB DAC 或 HDR 验收；测试使用独立配置和自有素材，不修改默认 APPDATA / XDG 用户配置。PR 描述注明触发问题、修改后的行为和实际检查结果。

修改 Windows 打包时，运行 `scripts/Test-WindowsPackaging.ps1 -NsisCompiler <makensis.exe 完整路径>`；本地打包使用 `scripts/Package-Windows.ps1`，由脚本自动执行 Release 编译。依赖缓存完整时可加 `-Offline`。命令、产物目录与失败边界见 [打包记录](docs/validation/windows-packaging.md)。

提交前检查 `git status --short` 和暂存 diff。`.gitignore` 保留 Cargo.lock、runtime manifest、图标、API 示例及验收文档，忽略运行时二进制、编译缓存、发行包和用户配置；不要用 `git add -f` 提交这些生成物。自定义 `CARGO_TARGET_DIR` 放在 `target/` 子目录或仓库外；若放在仓库内其他位置，应添加对应目录规则。

以下清理方式用于 Windows 工作区，不作为 Linux 构建前置。编译缓存较大，完成验证并保存必要证据后，可先运行 `scripts/Clean-Workspace.ps1 -WhatIf` 核对，再清理。脚本保留 Release exe、固定 runtime 和源码，之后完整重新编译是正常现象。

公开源码步骤见 [发布说明](docs/PUBLISHING.md)。不要通过改动此仓库的文档来声称二进制发行许可或硬件资格已经完成。
