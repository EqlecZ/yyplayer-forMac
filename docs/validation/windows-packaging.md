# Windows 自动编译打包与 0.3.0 验收

日期：2026-10-12。范围：在现有 Windows 打包入口中自动编译，使用本次实际产物生成 portable ZIP / NSIS Setup，并更新应用版本；不改播放逻辑。验收时工作区为 master，基准提交 `e99d67cb4fd29fb31fdef2bb0496eab291e156f4`，本轮修改尚未提交，BUILD-INFO 标记 `Working tree modified: True`。后续按用户要求将实现、文档与 .gitignore 整理一并在 master 本地提交，未推送；下列包仍保留生成时的真实记录。

## 改动与验收口径

- `scripts/Package-Windows.ps1` 在预检图标、固定 libmpv hash 与 NSIS 后，调用 Cargo metadata 获取实际应用版本 / package ID，再执行 locked x64 MSVC Release build。编译仍使用增量缓存，由本次 Cargo 判断有效性；不强制清理或删除旧 EXE。
- 只接受本次成功构建中 `yyplayer-app` / `yyplayer` bin 的 `compiler-artifact.executable`，同时检查退出码、build-finished、EXE 版本和 staging hash；不拼接固定 target 路径。支持 `CARGO_TARGET_DIR` / `build.target-dir` 与其他调用目录。
- 临时目录按调用生成唯一名称；ZIP 与 Setup 都生成成功后才移入 dist。失败时清理本次目录并恢复 NSIS 环境变量；拒绝覆盖已有同版本 / 提交号的包，删除前校验本次目录位于指定临时根目录并拒绝目录链接。
- 新增 `-Offline`、`-NsisCompiler`；NSIS 也可从 PATH 或标准 Program Files 目录查找。`/VERSION` 记录真实工具版本，应用 hash / 工作区修改状态写入 BUILD-INFO。固定 DLL 校验在编译前及复制后执行。
- workspace 五个 crate / Cargo.lock 为 0.3.0；Windows EXE 的 FileVersion / ProductVersion 由 Cargo 版本生成（数值资源为 0.3.0.0），NSIS 版本必须由脚本传入。Windows workflow 使用同一编译打包入口，移除重复编译，并运行隔离回归；触发分支仍为 master。

产物选择依据 [Cargo JSON 产物协议](https://doc.rust-lang.org/cargo/reference/external-tools.html#artifact-messages)，Windows 版本资源依据 [Microsoft VERSIONINFO](https://learn.microsoft.com/en-us/windows/win32/menurc/versioninfo-resource)。

## 环境与检查

本机 Windows x64，Rust / Cargo 1.99.0，host `x86_64-pc-windows-msvc`，Slint / slint-build 保持 1.17.1。NSIS 未全局安装；使用 [官方 SourceForge NSIS 3.12 ZIP](https://sourceforge.net/projects/nsis/files/NSIS%203/3.12/) 解压到 `target/windows-packaging-tools/`，未改系统设置。工具归档 SHA-256：`56581f90db321581c5381193d796fffcf2d24b2f8fed2160a6c6a3baa67f2c4f`，CLI 返回 `v3.12`。固定 libmpv build：`20260928-git-e470f8986e`。

| 检查 | 实际结果 |
| --- | --- |
| `cargo metadata --locked --offline --no-deps --format-version 1` | 五个 workspace crate 均为 0.3.0，锁定图一致 |
| `cargo fmt --all --check` | exit0 |
| `cargo clippy --locked --offline --workspace --all-targets -- -D warnings` | exit0，33.79秒 |
| `cargo test --locked --offline --workspace` | exit0；core12/mpv9/platform1/UI1/app20，共43通过，既有 HTTPS 集成1项 ignored |
| `scripts/Test-WindowsPackaging.ps1 -NsisCompiler <便携 makensis.exe>` | 最终脚本 PowerShell 7 与 Windows PowerShell 5.1 均7项通过 |
| `scripts/Package-Windows.ps1 -Offline -NsisCompiler <便携 makensis.exe>` | 真实程序自动 Release 编译与两个包生成 exit0；首次编译3分51秒，最终 PowerShell 5.1 复跑使用本次 Cargo 确认的缓存产物 |
| ZIP 内容 / BUILD-INFO / EXE / DLL / Setup 版本校验 | 通过，见下表 |
| ZIP 内程序独立配置启动 / 正常退出 | 4秒后exit0，ready=true、Idle、error / render_error空、render初始化0 |
| PowerShell AST / `git diff --check` | 通过 |

当前沙箱内 Rust 路径解析曾报拒绝访问，允许的本机执行中完成上述 Rust 检查。初次 PowerShell 5.1 回归实际暴露 Cargo / NSIS stderr 会变成终止错误；脚本保留进度输出，并按实际退出码判断。测试脚本初版把不存在的环境变量恢复成空 CARGO_TARGET_DIR，随后 fmt 拒绝；现恢复为原始不存在状态，最终回归与 fmt 通过。旧失败不计为通过证据。

隔离回归使用无依赖小程序和不会加载的文本 DLL fixture，原始默认 target 目录放入旧 EXE，真实 Cargo 产物改到含空格 / 中文的自定义目录；不会运行安装器或修改用户配置。覆盖：runtime hash错误、编译失败时拒绝旧EXE、其他目录调用与自定义target、ZIP的实际EXE hash / 版本、NSIS失败不发布部分包、失败后重试与有效缓存、已有包内容保护。每次同时检查 staging 清理、环境恢复与旧EXE不变。

## 实际产物

目录：`dist/`。以下为最终脚本本机生成的开发包，基准提交号仅用于文件名；本轮源码修改仍在工作区。

| 文件 | 字节数 | SHA-256 |
| --- | ---: | --- |
| `YYPlayer-0.3.0-e99d67cb4fd2-windows-x64-portable.zip` | 58,192,026 | `dab3f76dd688bc251172615d02c77cfd8ddcdd74a5e857103b18f886581ae0bc` |
| `YYPlayer-0.3.0-e99d67cb4fd2-windows-x64-setup.exe` | 58,380,738 | `c52cfff613888ed6afa861630daba63f37728c562d1debf8cdfd07c6a0c19327` |
| Cargo / ZIP 中的 `yyplayer.exe` | 27,524,608 | `2b377333b0b54eddfbb1ba847aaf5a2979e64359424b6b75b35fbad19580f145` |
| ZIP 中的 `runtime/libmpv-2.dll` | 120,816,640 | `0d5b9dbecb73e179ef39dc94314ff8905fab926a80f8d60a2367d824b7254e2e` |

ZIP EXE 与实际 Cargo 产物逐字节 hash 一致；EXE FileVersion / ProductVersion 与 Setup ProductVersion 均为 0.3.0。包内保留 API 示例、LICENSE、依赖记录、runtime manifest 与 BUILD-INFO。独立启动不打开用户媒体，`YYPLAYER_CONFIG` 指向 target 下的测试配置，未启用 runtime hash 绕过。

本地详细日志 / 数据保留在 `target/windows-packaging-{build,final-build,clippy,tests,powershell5}.log`、`target/windows-packaging-smoke/` 及 `target/windows-packaging-qa-*/results.json`；这些生成物不提交。验收不依赖将临时日志纳入源码。

## 限制与下一步

提交整理核对了 Git 历史中的 `5390a3d` / `e99d67c`，更新现行主分支、0.3.0、Rust 1.99.0 与自动打包说明，保留历史记录；Linux workflow 仍只在 dev-linux push 自动触发。`.gitignore` 加入运行时 / 编译二进制、移出 dist 的命名发行包、临时下载与视频索引规则，保留 Cargo.lock、manifest、脚本、图标、API 示例与本文。`git check-ignore --no-index -q` 的26项生成路径 / 17项源码路径断言通过；提交前检查文档相对链接、PowerShell 语法与暂存文件。没有新增应用代码改动或重复硬件资格测试。

未运行远端 Actions，未执行 Setup 安装 / 升级 / 卸载或干净 Windows 验收。本轮 workspace 测试保留原 HTTPS ignored，未重复联网 / UI 点击 / GPU画质 / WASAPI / USB / 位准确 / HDR专项，Linux本轮未运行；播放代码与默认策略未改变。包编译与启动不替代 runtime 内部完整 notices / 对应源码和DLL闭包审计。

下一步提交源码后运行 Windows x64 packages，核对CI包，再在干净Windows x64实测安装、升级、卸载与包依赖；正式发行前继续补齐上述许可与硬件资格。
