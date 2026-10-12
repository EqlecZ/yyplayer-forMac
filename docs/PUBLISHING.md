# 源码与 GitHub 构建发布

更新：2026-10-12。当前应用版本为 **0.3.0**，源码许可为 **GPL-3.0-only**。正式主分支为 `master`，已包含 Windows/Linux 共用代码、联网搜索、页面返回与歌词主题实现。提交、合并和推送按用户授权执行；本轮已授权合并两端 master，并将本地 / 远端 master 与 dev 同步到同一进度。

## 共用源码与仓库目录

Windows x64 MSVC 与 Linux amd64 GNU 使用同一个 Cargo workspace、Cargo.lock、core、controller 和 Slint 界面；平台 API 与依赖通过 target cfg 选择。在各自系统安装构建依赖后，都可以执行：

```bash
cargo build --locked --release -p yyplayer-app --bin yyplayer
```

默认本机目标的 Windows 产物为 `target/release/yyplayer.exe`，Linux 产物为 `target/release/yyplayer`。分支名不决定编译平台，运行时和安装包仍按系统分别准备，见 [README](../README.md) 与 [Linux 指南](LINUX.md)。

VS Code 打开完整仓库根目录，即包含 `.git`、Cargo.toml、Cargo.lock 和 crates 的目录。Windows 历史工作区为 `E:\SourceFiles\rust\yyplayer`，当前 Linux 工作区为 `/home/yydaniel/sourcefiles/yyplayer`；这些是开发者本机路径，不是编译时必须使用的路径。

## 当前主分支与提交内容

Git 历史确认：Windows/Linux 共用代码在 `5390a3d`（2026-10-03，PR #4）合入 master；`e99d67c`（2026-10-11）包含联网搜索、前端与页面历史栈。0.3.0 的 Windows 打包脚本、版本与文档整理继续在 master 提交，不需要再次合并 dev-linux 或按系统切换分支。

- 用户已确认：使用 **Windows x64 packages** 工作流，可以编译打包 `dev-linux` 内的共用代码。该证据限 Windows 编译与打包，没有提供 run URL / 提交号，也没有新增 Windows 播放、设备或安装回归记录。
- Ubuntu 26.04 amd64 已有本机编译、Wayland 实际播放 / GPU、独立 X11 UI、deb / AppImage 提取启动与 hash 校验记录；干净系统安装、其他发行版和硬件资格仍按 STATUS 保留。
- README、AGENTS 和开发 / 发布指南已按当前主分支、0.3.0 与自动编译打包更新。历史 STATUS / ADR 的分支与验收表述保留当时事实；最新状态见 [STATUS](STATUS.md)，2026-10-03 的准备过程见 [合并准备记录](validation/shared-source-merge-preparation.md)。
- Cargo.lock、LICENSE 和第三方记录保留；不提交 DLL / so、编译产物、下载缓存、用户媒体、用户配置或凭据。

在仓库根目录检查当前分支、工作区与暂存 diff，将源码、脚本、Cargo.lock 和相关文档提交到目标分支。`.gitignore` 排除编译缓存、runtime 二进制、下载 archive、发行包与用户配置；不要强制加入这些文件。自定义 `CARGO_TARGET_DIR` 使用 target 子目录或仓库外目录，避免 Cargo 缓存进入版本控制。开发分支是否保留由用户决定，不要求按 Windows / Linux 分别维护两份应用源码。

只读复查命令：

```bash
git status --short
git diff --cached --stat
git branch -vv
git remote -v
git log -1 --oneline
```

合并状态按 Git 历史确认；编译成功不代表未测试的播放或安装场景已通过。验收包在提交前生成时，BUILD-INFO 中的基准提交号和 working tree modified 状态应保留；需要提交后包时重新运行打包脚本，不只改文件名。

## GitHub 构建与报告

以下为当前仓库工作流的实际触发方式；Windows 编译打包入口于 2026-10-12 更新，触发分支保持不变。

| 工作流 | 自动触发 | 手动用途 |
| --- | --- | --- |
| Windows x64 packages | `master` push | Run workflow 选择 master 或含工作流的功能分支；本地 0.3.0 编译打包证据见 STATUS |
| Ubuntu 26.04 packages | `dev-linux` push | 可选择 `master` 构建 deb / AppImage；当前没有 `master` push 自动触发 |
| Linux compatibility compile matrix | 无 | 选择 `master` 或含工作流的功能分支，测试 Ubuntu / Debian 各版本源码编译下限 |

最低版本矩阵需要工作流在 GitHub 默认分支注册；真实报告尚未提供，当前不能填写实测最低源码编译版本。操作与报告边界见 [矩阵指南](LINUX_COMPATIBILITY_CI.md)。

### Windows x64 产物

`.github/workflows/windows-release.yml` 使用 `windows-2022` x64 MSVC runner，校验固定 libmpv archive / DLL hash，生成：

- `YYPlayer-<版本>-<提交号>-windows-x64-portable.zip`：解压后从 `yyplayer.exe` 启动，运行时 DLL 位于 `runtime/`，配置在 `%APPDATA%`。
- `YYPlayer-<版本>-<提交号>-windows-x64-setup.exe`：NSIS 安装器，限 x64 Windows，需要管理员权限，安装到 64 位 Program Files，创建快捷方式和卸载入口；卸载不删除用户配置。

文件位于该次 Actions run 的 `YYPlayer-windows-x64-<commit>` artifact，保留 30 天。7-Zip / NSIS 缺失时工作流明确失败。

本地与 CI 共用 `scripts/Package-Windows.ps1`：脚本先执行 `cargo build --locked --release --target x86_64-pc-windows-msvc -p yyplayer-app --bin yyplayer`（另加 JSON 产物输出），成功后从本次 `compiler-artifact.executable` 取得实际 EXE，不再读取固定 target 路径。Cargo 的自定义产物目录继续生效，CI 已移除单独编译步骤，并运行隔离打包回归。

本地用 `./scripts/Package-Windows.ps1`，依赖缓存完整时可加 `-Offline`；NSIS 不在 PATH / 标准安装目录时，用 `-NsisCompiler 'C:\Tools\NSIS\makensis.exe'`。提前准备固定 libmpv，脚本在编译前核验 DLL hash 与打包工具。ZIP 与 Setup 全部生成成功后移入 `dist/`，临时目录在成功 / 失败时清理，保留已有同版本 / 提交号的包。0.3.0 的 workspace、EXE 版本资源和安装器版本一致，`BUILD-INFO.txt` 记录 EXE hash 与工作区修改状态。命令、失败边界及安装验收限制见 [Windows 打包记录](validation/windows-packaging.md)。

### Linux 产物

`.github/workflows/linux-packages.yml` 在 Ubuntu 26.04 容器中检查格式、lint、测试和独立 X11 UI，随后生成 deb / AppImage、对应源码归档、Rust 源码清单、runtime manifest、构建信息和校验和。产物位于 `YYPlayer-ubuntu-26.04-amd64-<commit>` artifact，保留 30 天。完整目录与运行方式见 [Linux 指南](LINUX.md)。

手动运行这个工作流即可验证 `master` 的 Linux 包；若希望 `master` push 同时自动构建 Linux，需要另行修改它的 branches 触发条件。

## 正式二进制发行边界

工作流生成逐提交 CI artifact，不会自动创建 GitHub Release。Windows 的完整 libmpv 内部组件 notices / 对应源码安排与 DLL 闭包、Linux 上游 AppImage runtime 的完整静态闭包 / 可重链接安排仍待补证。构建通过不代表正式二进制发行审计、干净系统安装 / 升级 / 卸载、USB 位准确或 HDR 资格完成。

Windows 运行时见 [libmpv 说明](../third_party/mpv/README.md)，Linux 发行剩余事项见 [runtime 记录](../packaging/linux/runtime-notices/README.md)。
