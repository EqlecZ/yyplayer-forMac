# Windows 双击打包入口验收 · 2026-10-12

用户要求把手动 PowerShell 打包再包一层，直接双击运行。修改在 dev 工作区完成，应用保持 0.4.0，基准提交 c089cf735fde4a50144509f26764578bb04d8236；没有提交、推送或合入 master。此前 0.3.1 与本轮前端改动保留。

## 入口与行为

在资源管理器双击根目录 `Package-Windows.cmd`。它只负责打开 pwsh / 自带 Windows PowerShell 5.1、设置当前控制台 UTF-8、调用 PowerShell 流程并在结束后保留窗口；不需要安装 Python或永久改变执行策略。

`scripts/Package-Windows-Interactive.ps1` 根据自身路径定位项目，不依赖调用目录。Rust / Git 不在 Explorer 的 PATH 时尝试正常用户 / 系统安装路径，缺失时明确提示；不自动安装 Rust、Visual Studio 或 Windows SDK。缺失 libmpv 时调用已有固定运行时获取脚本。默认 NSIS 使用锁定的归档，每次校验并解压新副本；高级 `-NsisCompiler` 可指定编译器，`-Offline` 禁止下载并传给 Cargo，`-NoOpen` 关闭目录自动展示。

完成后打开 `dist/windows-<时间>-<随机标识>/`，同时显示本次 `target/windows-packaging-runs/<标识>/packaging.log`。每次独立输出，已有包保护仍保留。底层 Package-Windows 的新 `-OutputDirectory` 参数默认仍是 dist，Windows CI 上传规则不变。全部生成文件已被既有 gitignore 的 target / dist / archive / log 规则覆盖。

便携 NSIS 固定为 3.12，归档 SHA-256 `56581f90db321581c5381193d796fffcf2d24b2f8fed2160a6c6a3baa67f2c4f`，来源与发布页 hash 相符：[官方文件与 hash](https://sourceforge.net/projects/nsis/files/NSIS%203/3.12/nsis-3.12.zip/download)。PowerShell 请求在本机拿到 HTML 下载页，校验拒绝后用 Windows curl 重试同一 URL；最终 archive hash 通过才解压、执行。无未校验下载执行或全局 NSIS 安装。

## 发现并修正的边界

- 原 Get-Mpv 在选择 7-Zip 前无条件执行了一次 tar，现移除；检测 Program Files 中的 7-Zip，支持正常安装但未加入 PATH 的情况。PS5 下载采用 UseBasicParsing。
- 中文项目路径实际复现 Windows 图标资源编译失败。生成的 .rc 为 UTF-8，增加 `#pragma code_page(65001)` 后真实 rc.exe 通过；依据 [Microsoft RC pragma](https://learn.microsoft.com/en-us/windows/win32/menurc/pragma-directives)。
- Explorer / cmd 的旧控制台编码会误解 Cargo UTF-8 JSON 中的中文产物路径。双击入口与高级 PowerShell 流程显式设置当前进程 / 控制台 UTF-8，中文、空格、括号项目名通过实际编译打包。
- 原测试准备阶段在 PS5 遇 Cargo 缓存锁等待进度时误报 NativeCommandError；临时接受原生命令进度并检查真实退出码。启动器测试调用 cmd 采用原始参数的隐藏子进程，避免测试宿主 PS5 转义改变路径；生产 .cmd 自身不使用字符串生成媒体命令。

## 检查与证据

```powershell
cargo fmt --all -- --check
cargo clippy --locked --offline --workspace --all-targets -- -D warnings
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/Test-WindowsPackaging.ps1 -NsisCompiler '<便携 NSIS>/makensis.exe'
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/Test-WindowsPackagingLauncher.ps1 -NsisArchive '<缓存>/nsis-3.12.zip'
```

格式与严格 clippy exit0（后者41.54秒），PowerShell Parser、git diff --check 与相关文档链接通过。原打包7项通过：`target/windows-packaging-qa-de5f950cb68a408b8e56f0e48725db7e/results.json`。新入口10项通过：`target/windows-launcher-qa-9a6bd68a2742438d852715f1fb8eba37/results.json`：

1. 自有小 archive 的下载替身与实际提取（fixture DLL 仅哈希 / 打包，不加载）。
2. archive hash 错误在提取前拒绝。
3. 缺失 / 损坏 NSIS 缓存时离线拒绝。
4. 从其他 cwd 用 PowerShell 5.1 打包中文 / 空格 / 括号路径。
5. 实际 .cmd 入口同版本、同提交重复打包到不同目录，旧包 hash 保留。
6. ZIP 使用本次 Cargo EXE，版本0.4.0、hash匹配。
7. 编译错误不打包旧 EXE、DLL hash 错误拒绝、离线缺失 DLL 拒绝；各次保留日志与非零退出码。

NSIS 首次联网下载 / 重试 / 启动另在独立目录实际验证，pwsh 证据：`target/nsis-download-final-qa-1e6953e160254714b9aae1dfe29af7e4/results.json`；Windows PowerShell 5.1 证据：`target/nsis-download-ps51-qa-20b2911e9a62491a8d6d07c923d00446/results.json`。两者均下载2,362,938字节，hash与manifest一致，CLI为v3.12。测试对生成包不运行安装器，不修改用户 APPDATA。

## 实际 0.4.0 包

第一次直接运行根 .cmd（仅给最终 pause 提供一键输入，测试用 -Offline -NoOpen），Release编译4分08秒，生成 `dist/windows-20261012-145429-22e9bb58/`。第二次用最终 PowerShell 5.1 流程 `-Offline`，Release重新编译2分02秒、生成并实际请求打开 `dist/windows-20261012-150619-a8b04b88/`；均exit0。两次日志在对应 `target/windows-packaging-runs/` 下，旧0.3.1 ZIP / Setup hash与之前记录完全相同。

最终产物：

| 文件 | 字节数 | SHA-256 |
| --- | ---: | --- |
| YYPlayer-0.4.0-c089cf735fde-windows-x64-portable.zip | 58,575,174 | 755eae9ecf07a960d6d6695fc382ede54091991d55b6ab0213b75e396fa51d6c |
| YYPlayer-0.4.0-c089cf735fde-windows-x64-setup.exe | 58,762,181 | a7b09035f9bbf64909525d7eb5ad2afc9395c3ad4950493c0bcc80d846f8fe5d |
| 包内 yyplayer.exe | 29,132,288 | 1d95539694846293fcc6469036b00236b70750d675aa5c1de186e51bd70cdc9c |

包内 EXE FileVersion / ProductVersion为0.4.0，与显式 x64 Cargo 产物、BUILD-INFO hash一致；工作区 modified=True 如实记录。DLL符合原固定manifest。解压后的便携程序从自身目录、独立配置运行4秒并正常退出：exit0、ready=true、Idle、error / render_error为空、renderer初始化0、运行时mpv v0.41.0-1087-ge470f8986。证据为 `target/windows-launcher-package-verification/results.json`、`smoke-results.json`、`diagnostics.json`。

本轮只修改打包 / 资源编译边界，不重复此前前端48单测 / 53阶段或播放设备测试。物理 Explorer 双击、干净机器首次真实 libmpv 网络下载、安装 / 升级 / 卸载未验；实际执行了同一 .cmd / PowerShell入口。Rust / MSVC / SDK / Git与网络仍是构建条件，正式二进制许可闭包、DAC / 位准确 / HDR等既有边界不扩大。下一步在干净Windows构建环境补首次运行时获取与安装验收，按后续授权提交dev与合入master。
